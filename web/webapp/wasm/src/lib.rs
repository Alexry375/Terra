//! La ligne de bilan compte plus de 60 clés : `serde_json::json!` est une macro
//! récursive, il lui faut la marge (même raison que `engine/src/bin/simulate.rs`).
#![recursion_limit = "512"]
//! # Pont WebAssembly vers le moteur Rust
//!
//! Ce crate n'implémente **aucune règle du jeu**. Il expose au navigateur (et à
//! Node) exactement les services que le binaire natif `simulate` expose, tous
//! rendus par le moteur `engine` :
//!
//! - `dump_deck`, `dump_corporations`, `probe`, `probe_action`, `dump_state` :
//!   les **interrogations** du moteur. Elles ne tirent rien au hasard et sont
//!   construites par le MÊME code de sérialisation que
//!   `engine/src/bin/simulate.rs` — d'où l'égalité au caractère près.
//! - `bilan` : `engine::sim::run_simulation` avec `RandomPolicy`, comme le
//!   binaire natif. (Les empreintes de parties ne sont PAS comparables au natif :
//!   `usize` fait 32 bits ici et 64 bits là-bas, la consommation du générateur
//!   aléatoire diverge donc légitimement — voir le journal.)
//! - `pas` : la partie **pas-à-pas**. On rejoue la partie depuis la graine avec
//!   la liste des décisions déjà prises ; à la première décision non
//!   enregistrée, le pont rend « quel joueur, quelle décision, quelles options »
//!   et l'état **vivant** reçu par `Policy::observe` juste avant cette décision,
//!   rendu par `engine::observe::state_view`.
//!
//! Interface : une seule fonction `terra_call(ptr, len) -> i32` qui reçoit une
//! requête JSON et laisse la réponse JSON dans un tampon global
//! (`terra_result_ptr`). Pas de wasm-bindgen : « interface C minimale », comme
//! le contrat l'autorise.
//!
//! Cible : `wasm32-wasip1`. Le moteur charge `cards.json` par `std::fs`
//! (`CardsDb::load_boites`) et `../../engine/` ne bouge pas d'une ligne : il
//! faut donc un système de fichiers, que le shim WASI de la page fournit.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Tag};
use engine::choice::{
    action_res_label, action_res_quantity, describe_branch, describe_selector_grant,
    spend_amount_quantity, tag_label, ChoiceContext,
};
use engine::joueur;
use engine::flow::{
    play_round, score_parts, setup_game, ActionSource, SelectorBonus, UpgradeSource,
};
use engine::observe::{state_view, ObservingPolicy};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{
    run_probe_action_target, run_probe_seq_corp, ProbeCorp, ProbeDelta, ProbeOptions, ProbeRes,
    ProbeScript,
};
use engine::sim::{run_simulation, MAX_GENERATIONS};
use engine::state::GameState;
use rand::rngs::StdRng;
use serde_json::{json, Value};
use std::cell::RefCell;

// ---------------------------------------------------------------- tampon global

thread_local! {
    /// Réponse du dernier `terra_call`, lue par l'hôte à `terra_result_ptr()`.
    static RESULTAT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Adresse du tampon de réponse (valide jusqu'au prochain `terra_call`).
#[no_mangle]
pub extern "C" fn terra_result_ptr() -> *const u8 {
    RESULTAT.with(|r| r.borrow().as_ptr())
}

/// Réserve `len` octets et rend leur adresse : l'hôte y écrit la requête JSON.
#[no_mangle]
pub extern "C" fn terra_alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len);
    v.resize(len, 0);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Rend la mémoire réservée par `terra_alloc`.
#[no_mangle]
pub extern "C" fn terra_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe { drop(Vec::from_raw_parts(ptr, len, len)) }
    }
}

/// Point d'entrée unique. Rend la longueur de la réponse JSON, ou -1 si la
/// requête elle-même est illisible.
///
/// # Safety
/// `ptr`/`len` doivent désigner une zone lisible de la mémoire linéaire, telle
/// que celle rendue par `terra_alloc`.
#[no_mangle]
pub extern "C" fn terra_call(ptr: *const u8, len: usize) -> i32 {
    let req = unsafe { std::slice::from_raw_parts(ptr, len) };
    let out = match serde_json::from_slice::<Value>(req) {
        Ok(v) => repondre(&v),
        Err(e) => erreur(format!("requete illisible: {e}")),
    };
    let bytes = serde_json::to_vec(&out).unwrap_or_else(|e| {
        format!("{{\"ok\":false,\"erreur\":\"reponse illisible: {e}\"}}").into_bytes()
    });
    let n = bytes.len() as i32;
    RESULTAT.with(|r| *r.borrow_mut() = bytes);
    n
}

fn erreur(msg: String) -> Value {
    json!({ "ok": false, "erreur": msg })
}

/// Réponse « lignes de sortie standard » : les interrogations du moteur rendent
/// des LIGNES, que l'hôte imprime telles quelles. C'est ce qui garantit l'égalité
/// au caractère près avec `simulate` (même `serde_json`, même construction).
fn lignes(v: Vec<String>) -> Value {
    json!({ "ok": true, "lignes": v })
}

// ------------------------------------------------------------- base de cartes

/// Base de cartes chargée une fois par configuration (`chemin` + `boites` +
/// `effets`) : le pas-à-pas rejoue la partie à chaque coup, il ne faut pas relire
/// le fichier 500 fois. La base est celle du moteur, chargée par le moteur.
type CleBase = (String, String, bool);

thread_local! {
    static CACHE: RefCell<Option<(CleBase, &'static CardsDb)>> = const { RefCell::new(None) };
}

fn base(chemin: &str, boites_txt: &str, effets: bool) -> Result<&'static CardsDb, String> {
    let cle: CleBase = (chemin.to_string(), boites_txt.to_string(), effets);
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let frais = match c.as_ref() {
            Some((k, _)) => *k != cle,
            None => true,
        };
        if frais {
            let boites = BoiteSet::parse(boites_txt)?;
            let mut db = CardsDb::load_boites(chemin, boites)?;
            db.effects_on = effets;
            // `Box::leak` : la base vit aussi longtemps que le module (au plus
            // quelques configurations par session), ce qui évite tout
            // `static mut` et tout `unsafe` ici.
            let db: &'static CardsDb = Box::leak(Box::new(db));
            *c = Some((cle, db));
        }
        match c.as_ref() {
            Some((_, db)) => Ok(*db),
            None => Err("cache de base de cartes vide".to_string()),
        }
    })
}

fn chaine(v: &Value, cle: &str, defaut: &str) -> String {
    v.get(cle)
        .and_then(|x| x.as_str())
        .unwrap_or(defaut)
        .to_string()
}

fn drapeau(v: &Value, cle: &str) -> bool {
    v.get(cle).and_then(|x| x.as_bool()).unwrap_or(false)
}

/// Un entier de 64 bits NON SIGNÉ, accepté sous forme de nombre JSON **ou de
/// chaîne de chiffres**.
///
/// La forme chaîne existe pour une raison : un nombre JSON est un flottant côté
/// hôte, et au-delà de 2^53 il ne représente plus exactement une graine. Sans
/// elle, `--seed 18446744073709551615` arrivait ici comme un flottant,
/// `as_u64()` échouait, et le pont retombait **silencieusement** sur 0 — il
/// rendait donc la partie d'une AUTRE graine en la présentant comme la bonne.
/// Une valeur présente mais illisible est désormais refusée, jamais devinée.
fn nombre_u64(v: &Value, cle: &str, defaut: u64) -> Result<u64, String> {
    match v.get(cle) {
        None | Some(Value::Null) => Ok(defaut),
        Some(Value::String(s)) => s
            .parse::<u64>()
            .map_err(|_| format!("{cle} invalide: « {s} » (entier de 0 à 2^64-1 attendu)")),
        Some(x) => match x.as_u64() {
            Some(n) => Ok(n),
            None => Err(format!("{cle} invalide: {x} (entier de 0 à 2^64-1 attendu)")),
        },
    }
}

/// Un entier NON SIGNÉ de taille machine, refusé s'il est absent de l'intervalle.
fn nombre_usize(v: &Value, cle: &str, defaut: usize) -> Result<usize, String> {
    match v.get(cle) {
        None | Some(Value::Null) => Ok(defaut),
        Some(x) => match x.as_u64() {
            Some(n) if n <= usize::MAX as u64 => Ok(n as usize),
            _ => Err(format!("{cle} invalide: {x} (entier positif attendu)")),
        },
    }
}

/// Une chaîne, refusée si la clef est présente sous une autre forme.
fn chaine_opt(v: &Value, cle: &str) -> Result<Option<String>, String> {
    match v.get(cle) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(x) => Err(format!("{cle} invalide: {x} (chaîne attendue)")),
    }
}

/// Plafond de parties par appel. Le pont tourne en 32 bits : `--games` absurde
/// faisait déborder une capacité de vecteur, et une panique en WebAssembly est
/// un déroutement IRRATTRAPABLE (le contrat le rappelle) — l'instance mourait,
/// la page avec. On refuse donc avant d'appeler le moteur, plutôt que d'être
/// tué par lui. Le binaire natif n'a pas cette limite ; aucun usage réel ne
/// l'atteint (le check en demande 30, le moteur en joue 2000 en 274 ms).
const MAX_PARTIES_PAR_APPEL: u64 = 1_000_000;

fn repondre(v: &Value) -> Value {
    let op = chaine(v, "op", "");
    let cards = chaine(v, "cards", "assets/cards.json");
    let boites = chaine(v, "boites", "base");
    let effets = v.get("effects").and_then(|x| x.as_bool()).unwrap_or(true);
    let db = match base(&cards, &boites, effets) {
        Ok(db) => db,
        Err(e) => return erreur(e),
    };
    let mut out = repondre_op(db, v, &op);
    // Ce que la composition des boîtes a eu à signaler sans le corriger. Le
    // binaire natif l'écrit sur stderr ; l'hôte en fait autant.
    if out.get("ok").and_then(|x| x.as_bool()) == Some(true) && !db.avertissements.is_empty() {
        out["avertissements"] = json!(db.avertissements);
    }
    out
}

fn repondre_op(db: &'static CardsDb, v: &Value, op: &str) -> Value {
    match repondre_op_verif(db, v, op) {
        Ok(x) => x,
        Err(e) => erreur(e),
    }
}

/// Toute la validation est ici, AVANT le moindre appel au moteur : en
/// WebAssembly une panique tue l'instance sans retour possible, le pont ne peut
/// donc pas se contenter de « laisser le moteur refuser ».
fn repondre_op_verif(db: &'static CardsDb, v: &Value, op: &str) -> Result<Value, String> {
    Ok(match op {
        "dump_deck" => dump_deck(db),
        "dump_corporations" => dump_corporations(db),
        "dump_state" => dump_state(db, nombre_u64(v, "seed", 0)?),
        "probe" => probe(db, v)?,
        "bilan" => {
            let games = nombre_u64(v, "games", 1000)?;
            if games > MAX_PARTIES_PAR_APPEL {
                return Err(format!(
                    "games invalide: {games} (au plus {MAX_PARTIES_PAR_APPEL} par appel)"
                ));
            }
            bilan(
                db,
                games,
                nombre_u64(v, "seed", 0)?,
                drapeau(v, "observe"),
                drapeau(v, "observe_state"),
                drapeau(v, "dump_turn_order"),
            )
        }
        "pas" => {
            let seed = nombre_u64(v, "seed", 0)?;
            // Une liste de décisions illisible n'est pas « aucune décision » :
            // le pont rendrait la décision n°0 comme si de rien n'était.
            let decisions: Vec<Value> = match v.get("decisions") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(a)) => a.clone(),
                Some(x) => return Err(format!("decisions invalide: {x} (liste attendue)")),
            };
            pas(db, seed, decisions, essais(v)?)
        }
        autre => return Err(format!("op inconnue: {autre}")),
    })
}

/// Réponse « lignes » enrichie d'une valeur informative (jamais comparée au
/// natif sur la sortie standard : elle part sur stderr chez lui aussi).
fn lignes_et(v: Vec<String>, cle: &str, valeur: Value) -> Value {
    let mut o = lignes(v);
    o[cle] = valeur;
    o
}

// ------------------------------------------------- interrogations du moteur
//
// Les trois blocs qui suivent recopient, ligne pour ligne, la sérialisation de
// `engine/src/bin/simulate.rs`. C'est délibéré : le check 02 compare les deux
// sorties au caractère près, et la seule façon honnête d'y arriver est de
// construire le MÊME objet JSON à partir des MÊMES champs du moteur.

fn dump_deck(db: &CardsDb) -> Value {
    let mut out = Vec::new();
    for c in db.recensement() {
        let line = json!({
            "name": c.name,
            "kind": c.kind.as_str(),
            "boite": c.boite.as_str(),
            "planche": c.planche,
            "couleur": c.couleur,
            "effets_geres": c.effets_geres,
        });
        out.push(line.to_string());
    }
    lignes(out)
}

fn dump_corporations(db: &CardsDb) -> Value {
    let mut out = Vec::new();
    for c in &db.corporations {
        let line = json!({
            "name": c.name,
            "starting_mc": c.starting_mc,
            "tags": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "encoded": c.effect.is_some(),
        });
        out.push(line.to_string());
    }
    lignes(out)
}

/// La vue de l'état de DÉPART, avec la graine que `--games N --seed S` emploie
/// pour sa première partie (`run_simulation` tire la graine de chaque partie
/// d'un RNG maître seedé par `--seed`). Même dérivation que le binaire natif.
fn dump_state(db: &CardsDb, seed: u64) -> Value {
    use rand::{RngCore, SeedableRng};
    let mut master = StdRng::seed_from_u64(seed);
    let game_seed = master.next_u64();
    let mut policy = RandomPolicy;
    let game = setup_game(db, game_seed, &mut policy);
    lignes(vec![state_view(&game, db).to_string()])
}

// ------------------------------------------------------------ sérialisations
// (identiques à `engine/src/bin/simulate.rs`)

fn delta_json(d: &ProbeDelta) -> Value {
    json!({
        "mc": d.mc,
        "heat": d.heat,
        "plants": d.plants,
        "hand": d.hand,
        "mc_prod": d.mc_prod,
        "heat_prod": d.heat_prod,
        "plant_prod": d.plant_prod,
        "card_prod": d.card_prod,
        "tr": d.tr,
        "temperature": d.temperature,
        "oxygen": d.oxygen,
        "oceans": d.oceans,
        "forests": d.forests,
    })
}

fn resources_json(rs: &[ProbeRes]) -> Value {
    Value::Array(
        rs.iter()
            .map(|r| json!({ "card": r.card, "kind": r.kind, "n": r.n }))
            .collect(),
    )
}

fn corp_json(c: &ProbeCorp) -> Value {
    json!({
        "name": c.name,
        "found": c.found,
        "encoded": c.encoded,
        "starting_mc": c.starting_mc,
        "start_prod": {
            "mc": c.start_prod.0,
            "heat": c.start_prod.1,
            "plants": c.start_prod.2,
        },
        "upgrades": c.upgrades,
        "start_heat": c.start_heat,
        "discard_rate": c.discard_rate,
        "has_action": c.has_action,
    })
}

fn selector_bonus_json(b: &SelectorBonus) -> Value {
    json!({
        "phase": b.phase,
        "upgraded": b.upgraded.map(|v| v.label()),
        "mc_discount": b.mc_discount,
        "mc": b.mc,
        "draw": b.draw,
        "extra_activations": b.extra_activations,
        "extra_builds": b.extra_builds,
        "research_draw": b.research_draw,
        "research_keep": b.research_keep,
        "alternative": b.alternative,
        "card": b.spec.name,
    })
}

// -------------------------------------------------------------------- op probe

/// Construit les options de sonde depuis la requête, en REFUSANT (jamais en
/// ignorant) un argument mal formé — comme le binaire natif, qui appelle `die`.
/// Toute clef présente sous une forme inattendue est une erreur : un argument
/// silencieusement ignoré produirait une sonde qui répond à une AUTRE question
/// que celle posée.
fn options_sonde(v: &Value) -> Result<(ProbeOptions, ProbeScript), String> {
    let mut opts = ProbeOptions::default();
    let mut script = ProbeScript::default();
    let entier_i64 = |cle: &str| -> Result<Option<i64>, String> {
        match v.get(cle) {
            None | Some(Value::Null) => Ok(None),
            Some(x) => x
                .as_i64()
                .map(Some)
                .ok_or_else(|| format!("{cle} invalide: {x} (entier attendu)")),
        }
    };
    if let Some(x) = entier_i64("probe_mc")? {
        opts.mc = x;
    }
    if let Some(x) = entier_i64("probe_plants")? {
        opts.plants = x;
    }
    opts.filler = nombre_usize(v, "probe_filler", opts.filler)?;
    opts.strict = drapeau(v, "probe_strict");
    if let Some(x) = v.get("probe_phase") {
        let n = x
            .as_u64()
            .ok_or_else(|| format!("probe_phase invalide: {x} (entier 1..5 attendu)"))?;
        if !(1..=5).contains(&n) {
            return Err("--probe-phase hors bornes (1..5)".to_string());
        }
        opts.phase = n as u8;
    }
    if let Some(x) = v.get("probe_upgrade") {
        let a = x
            .as_array()
            .ok_or_else(|| format!("probe_upgrade invalide: {x} (liste attendue)"))?;
        for u in a {
            let s = u.as_str().unwrap_or("");
            let Some((phase, variant)) = engine::state::parse_phase_upgrade(s) else {
                return Err(format!(
                    "--probe-upgrade invalide: « {s} » \
                     (attendu <phase 1..5><variante A|B>, par exemple 1B)"
                ));
            };
            opts.upgrades[phase as usize - 1] = Some(variant);
        }
    }
    if let Some(s) = chaine_opt(v, "probe_objectif")? {
        let s = s.as_str();
        let Some(kind) = engine::state::MilestoneKind::from_name(s) else {
            let noms: Vec<&str> = engine::state::MILESTONE_POOL
                .iter()
                .map(|k| k.name())
                .collect();
            return Err(format!(
                "--probe-objectif invalide: « {s} » (Objectifs du moteur : {})",
                noms.join(", ")
            ));
        };
        opts.objectif = Some(kind);
    }
    if let Some(x) = v.get("probe_choice") {
        let a = x
            .as_array()
            .ok_or_else(|| format!("probe_choice invalide: {x} (liste attendue)"))?;
        let mut out = Vec::with_capacity(a.len());
        for y in a {
            out.push(
                y.as_u64()
                    .ok_or_else(|| format!("probe_choice invalide: {y} (entier positif attendu)"))?
                    as usize,
            );
        }
        script.choices = out;
    }
    if let Some(x) = v.get("probe_target") {
        let a = x
            .as_array()
            .ok_or_else(|| format!("probe_target invalide: {x} (liste attendue)"))?;
        let mut out = Vec::with_capacity(a.len());
        for y in a {
            out.push(
                y.as_str()
                    .ok_or_else(|| format!("probe_target invalide: {y} (chaîne attendue)"))?
                    .to_string(),
            );
        }
        script.targets = out;
    }
    if let Some(s) = chaine_opt(v, "probe_joker_tag")? {
        let s = s.as_str();
        let Some(t) = Tag::parse_joker_choice(s) else {
            let noms: Vec<&str> = engine::cards::JOKER_TAG_CHOICES
                .iter()
                .map(|t| t.as_str())
                .collect();
            return Err(format!(
                "--probe-joker-tag invalide: « {s} » (badges choisissables : {})",
                noms.join(", ")
            ));
        };
        script.joker_tag = Some(t);
    }
    Ok((opts, script))
}

fn probe(db: &CardsDb, v: &Value) -> Result<Value, String> {
    let (opts, script) = options_sonde(v)?;
    let corp_txt = chaine_opt(v, "probe_corp")?;
    let nom_txt = chaine_opt(v, "probe")?;
    let action_txt = chaine_opt(v, "probe_action")?;
    let probe_corp = corp_txt.as_deref();
    let nom = nom_txt.as_deref();
    let action = action_txt.as_deref();
    let produce = drapeau(v, "probe_produce");

    // `--probe-action` PREND LA MAIN quand il est donné (même arbitrage que le
    // binaire natif).
    if let Some(aname) = action {
        let sequence = nom.unwrap_or(aname).to_string();
        let names: Vec<&str> = sequence.split(';').map(|s| s.trim()).collect();
        let target: Option<&str> = if nom.is_some() { Some(aname) } else { None };
        let r = run_probe_action_target(db, &names, &script, probe_corp, opts, target);
        let mut line = json!({
            "card": r.card,
            "found": r.found,
            "in_lot": r.in_lot,
            "has_action": r.has_action,
            "action_applied": r.action_applied,
            "upgrades": r.upgrades,
            "delta": delta_json(&r.delta),
            "resources": resources_json(&r.resources),
            "target_error": r.target_error,
        });
        if let Some(c) = &r.corp {
            line["corp"] = corp_json(c);
        }
        return Ok(lignes(vec![line.to_string()]));
    }

    let corp_only = nom.is_none() && probe_corp.is_some();
    let nom = nom.unwrap_or_default().to_string();
    let names: Vec<&str> = if corp_only {
        Vec::new()
    } else {
        nom.split(';').map(|s| s.trim()).collect()
    };
    let r = run_probe_seq_corp(db, &names, opts, &script, produce, probe_corp);
    let mut line = json!({
        "card": r.card,
        "found": r.found,
        "in_lot": r.in_lot,
        "prereq_ok": r.prereq_ok,
        "prereq_ok_now": r.prereq_ok_now,
        "played": r.played,
        "delta": delta_json(&r.delta),
        "vp": r.vp,
        "paid": r.paid,
        "discarded": r.discarded,
        "resources": resources_json(&r.resources),
        "target_error": r.target_error,
        "produced": r.produced,
        "derived_prod": {
            "mc": r.derived_prod.0,
            "heat": r.derived_prod.1,
            "plants": r.derived_prod.2,
        },
        "vp_total": r.vp_total,
        "steel": r.steel,
        "titanium": r.titanium,
        "research": {
            "draw": r.research.0,
            "keep": r.research.1,
        },
        "upgrades": r.upgrades,
        "selector_bonus": selector_bonus_json(&r.selector_bonus),
        "joker_tag": r.joker_tag,
    });
    if let Some(c) = &r.corp {
        line["corp"] = corp_json(c);
    }
    Ok(lignes(vec![line.to_string()]))
}

// ------------------------------------------------------------------- op bilan

/// Même appel, mêmes clés, même ordre que `engine/src/bin/simulate.rs`.
/// `observe` enveloppe la politique dans `ObservingPolicy`, qui délègue TOUTES
/// ses réponses : le déroulement est bit à bit le même. Les observations partent
/// sur la sortie standard (le shim WASI de l'hôte les relaie), comme en natif.
fn bilan(
    db: &CardsDb,
    games: u64,
    seed: u64,
    observe: bool,
    observe_state: bool,
    dump_turn_order: bool,
) -> Value {
    let mut base_pol = RandomPolicy;
    let s = if observe {
        let mut policy = ObservingPolicy::new(db, RandomPolicy)
            .emitting(true)
            .with_full_state(observe_state)
            .keeping(false);
        run_simulation(db, games, seed, &mut policy)
    } else {
        run_simulation(db, games, seed, &mut base_pol)
    };
    let mut out: Vec<String> = Vec::new();
    if dump_turn_order {
        for order in &s.turn_orders {
            let seq: Vec<String> = order.iter().map(|p| p.to_string()).collect();
            out.push(format!("turn_order:{}", seq.join(",")));
        }
    }
    let line = json!({
        "games": s.games,
        "completed": s.completed,
        "truncated": s.truncated,
        "invariant_violations": s.invariant_violations,
        "avg_generations": s.avg_generations,
        "avg_score_p1": s.avg_score_p1,
        "avg_score_p2": s.avg_score_p2,
        "state_hash": format!("{:016x}", s.state_hash),
        "blue_actions": s.blue_actions,
        "prereq_snapshot_blocks": s.prereq_snapshot_blocks,
        "draw_before_build": s.draw_before_build,
        "draw_after_build": s.draw_after_build,
        "discard_payments": s.discard_payments,
        "draws": s.draws,
        "turn_order_switches": s.turn_order_switches,
        "res_added": s.res_added,
        "res_removed": s.res_removed,
        "res_targets_missing": s.res_targets_missing,
        "phase_upgrades_skipped": s.phase_upgrades_skipped,
        "phase_upgrades_granted": s.phase_upgrades_granted,
        "phase_upgrades_reupgraded": s.phase_upgrades_reupgraded,
        "upgraded_bonus_applied": s.upgraded_bonus_applied,
        "upgraded_extra_builds": s.upgraded_extra_builds,
        "phase_upgrades_targeted": s.phase_upgrades_targeted,
        "phase_upgrades_by_action": s.phase_upgrades_by_action,
        "upgraded_reveal_bonuses": s.upgraded_reveal_bonuses,
        "objective_condition_hits": s.objective_condition_hits,
        "draw_then_discard_uses": s.draw_then_discard_uses,
        "visionary_award_points": s.visionary_award_points,
        "cards_effects_unhandled": s.cards_effects_unhandled,
        "vp_from_resources": s.vp_from_resources,
        "derived_mc": s.derived_mc,
        "derived_heat": s.derived_heat,
        "derived_plants": s.derived_plants,
        "tr_from_tags": s.tr_from_tags,
        "research_extra_draws": s.research_extra_draws,
        "extra_builds_granted": s.extra_builds_granted,
        "extra_builds_used": s.extra_builds_used,
        "free_builds": s.free_builds,
        "next_card_mods_armed": s.next_card_mods_armed,
        "next_card_mods_used": s.next_card_mods_used,
        "corp_heat_as_mc": s.corp_heat_as_mc,
        "corp_forest_rebates": s.corp_forest_rebates,
        "corp_tr_boosts": s.corp_tr_boosts,
        "corp_trigger_tr": s.corp_trigger_tr,
        "action_phase_bonuses": s.action_phase_bonuses,
        "action_discard_costs": s.action_discard_costs,
        "draw_discard_discards": s.draw_discard_discards,
        "cards_revealed": s.cards_revealed,
        "standard_action_discounts": s.standard_action_discounts,
        "action_mc_bonuses": s.action_mc_bonuses,
        "joker_tag_choices": s.joker_tag_choices,
        "joker_tag_hits": s.joker_tag_hits,
        "corp_phase_upgrades_at_setup": s.corp_phase_upgrades_at_setup,
        "discard_bonus_mc": s.discard_bonus_mc,
        "action_phase_self_bonus": s.action_phase_self_bonus,
    });
    out.push(line.to_string());
    // `games_per_sec` n'est pas déterministe : le binaire natif l'envoie sur
    // stderr, hors de la ligne de bilan. Même traitement ici.
    lignes_et(out, "games_per_sec", json!(s.games_per_sec))
}

// ---------------------------------------------------- descripteurs de décision
//
// Ces objets décrivent les OPTIONS que le moteur soumet à la politique. Ils ne
// décrivent pas l'état de la partie — celui-là vient de `observe::state_view`,
// et de nulle part ailleurs.

// LA SORTE, ET POURQUOI ELLE EST INDISPENSABLE.
//
// Le moteur range ses cartes dans DEUX tables distinctes : `db.projects` et
// `db.corporations`. L'identifiant publié ici est un INDICE dans l'une ou dans
// l'autre — jamais un identifiant unique sur l'ensemble du jeu. Les projets vont
// de 0 à ~330, les corporations de 0 à ~15 : les deux plages se recouvrent, donc
// le numéro 7 désigne à la fois la carte projet « Arctic Algae » et la
// corporation « Inventrix ».
//
// Tant que la sorte n'était pas publiée, rien ne permettait de les distinguer à
// l'arrivée. Mesuré le 02-08 sur 70 graines : 3 fois sur 70, l'écran écartait une
// corporation comme doublon d'une carte projet portant le même numéro, et
// reportait le choix « joue cette corporation » sur cette carte projet. Cliquer
// une carte de sa main jouait alors une corporation.
//
// On publie donc la sorte à côté du numéro. C'est le COUPLE (sorte, numéro) qui
// désigne une carte, et rien d'autre.
fn carte_json(db: &CardsDb, id: u16) -> Value {
    match db.projects.get(id as usize) {
        Some(c) => json!({
            "sorte": "projet",
            "id": id,
            "nom": c.name,
            "prix": c.price,
            "couleur": c.color.nom_fr(),
            "badges": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "pv": c.vp,
        }),
        None => json!({ "sorte": "projet", "id": id, "nom": format!("carte inconnue {id}") }),
    }
}

fn corpo_json(db: &CardsDb, id: u16) -> Value {
    match db.corporations.get(id as usize) {
        Some(c) => json!({
            "sorte": "corporation",
            "id": id,
            "nom": c.name,
            "mc_depart": c.starting_mc,
            "badges": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        }),
        None => json!({
            "sorte": "corporation",
            "id": id,
            "nom": format!("corporation inconnue {id}"),
        }),
    }
}

fn nom_phase(n: u8) -> &'static str {
    match n {
        1 => "I — Développement",
        2 => "II — Construction",
        3 => "III — Action",
        4 => "IV — Production",
        5 => "V — Recherche",
        _ => "?",
    }
}

fn nom_action(db: &CardsDb, o: &ActionOpt) -> String {
    match o {
        ActionOpt::ForestWithPlants => "Forêt (plantes)".to_string(),
        ActionOpt::ForestWithMc => "Forêt (MC)".to_string(),
        ActionOpt::TemperatureWithHeat => "Température (chaleur)".to_string(),
        ActionOpt::TemperatureWithMc => "Température (MC)".to_string(),
        ActionOpt::OceanWithMc => "Océan (MC)".to_string(),
        // (moteur-questions-manquantes) « Défausser 1 carte pour du MC » N'EST
        // PLUS une action de la phase Action : le moteur ne produit plus ce
        // variant (voir `engine::policy::ActionOpt`). Vendre passe par
        // l'occasion libre, qui ne coûte pas d'échange — voir
        // `Harnais::vendre_librement`.
        ActionOpt::BlueAction(i) => match db.projects.get(*i as usize) {
            Some(c) => format!("Action de {}", c.name),
            None => format!("Action de la carte bleue {i}"),
        },
        ActionOpt::CorpAction => "Action de la corporation".to_string(),
    }
}

// ------------------------------------------------------- politique du harnais

/// Politique « rejeu » : elle répond la liste des décisions déjà prises, puis
/// signale la PREMIÈRE décision non encore prise. Elle ne décide rien elle-même
/// (les réponses par défaut qui suivent la décision en attente ne servent qu'à
/// faire retourner `play_round` : leur partie est jetée).
///
/// **`observe` est le cœur du dispositif** : le moteur l'appelle juste avant
/// CHAQUE décision avec l'état vivant. Dès que toutes les réponses enregistrées
/// sont consommées (`curseur == reponses.len()`), chaque observation écrase la
/// précédente ; la dernière retenue est donc celle qui précède immédiatement la
/// décision en attente. L'état rendu à la page est exactement celui que le
/// moteur a sous les yeux au moment du choix — ni reconstitué, ni daté d'un
/// début de manche.
///
/// **Pourquoi écraser plutôt que compter les observations** : le moteur observe
/// AUSSI les points de décision qu'il finit par ne pas poser (liste d'options
/// vide — aucune carte finançable, aucune action disponible). Un compteur
/// d'observations se désynchroniserait alors du compteur de réponses, et l'état
/// rendu serait celui d'une autre décision. Mesuré : 14 discordances sur une
/// partie de 383 décisions avant cette correction, 0 après.
struct Harnais<'a> {
    db: &'a CardsDb,
    reponses: Vec<Value>,
    curseur: usize,
    attente: Option<Value>,
    /// `state_view` de l'état vivant précédant la décision en attente.
    vue: Option<Value>,
    /// Mains des deux joueurs au moment de cette même observation : elles
    /// donnent leur sens aux indices que le moteur soumet (`choose_build`).
    mains: [Vec<u16>; 2],
    erreur: Option<String>,
    defaut: RandomPolicy,

    // ------------------------------------------------- (le-pont-ne-triche-plus)
    /// **Le compteur d'occasions de vente**, jumeau de `Joueur::occasions_partie`
    /// et de `Rejeu::occasions`. Il compte un rang dans la partie, pas un nombre
    /// de ventes : toute occasion ouverte l'incrémente, honorée ou non. C'est ce
    /// numéro qui permet de refuser une vente décidée à une occasion PLUS TARD
    /// que celle où le rejeu la rencontre.
    occasions: u64,
    /// **Le compte d'occasions AU POINT D'ARRÊT**, et c'est le seul que la page
    /// puisse interpréter. Passé le point d'attente, le moteur continue de
    /// tourner jusqu'à la fin de sa manche avec des réponses par défaut qui
    /// seront jetées — et il ouvre des occasions au passage, que personne ne
    /// verra jamais. Publier `occasions` brut ferait donc RECULER le compteur
    /// d'un coup à l'autre, selon la longueur de cette queue jetée.
    occasions_a_l_arret: u64,
    /// Les occasions ouvertes que personne n'a encore saisies, à l'instant de la
    /// décision en attente. La page les reçoit dans la réponse et peut y glisser
    /// une entrée `vendre` — le moteur, lui, ne pose aucune question.
    occasions_ouvertes: Vec<Value>,

    /// Passe 1 d'un rejeu d'essai : on cherche le MOMENT de l'essai (le point de
    /// la vraie partie où la décision essayée se prend). Faux en jeu normal.
    cherche_moment: bool,
    /// Rang de la décision essayée (= `Joueur::journal.len()` au moment du choix).
    essai_rang: usize,
    /// Numéro de l'occasion de vente à laquelle l'essai a lieu, s'il y a lieu.
    essai_occasion: Option<u64>,
    /// Vrai dès que le moment est atteint : plus rien n'est relevé après lui.
    moment: bool,
    /// Ce que le joueur avait sous les yeux à ce moment-là (voir `joueur.rs`).
    occasions_moment: u64,
    deck_vu: usize,
    oceans_vus: usize,
    corpos_vues: usize,
    main_connue: Vec<u16>,
    main_connue_siege: usize,
    siege_moment: usize,
    /// Vrai tant que la mise en place n'est pas finie (passe 1 seulement).
    en_mise_en_place: bool,
    /// Passe 2 depuis la mise en place : `ecarter_les_cartes_du_futur` a besoin
    /// de l'état lui-même, pas de sa sérialisation. Un clone par observation,
    /// et seulement là.
    cloner_etat: bool,
    etat_vu: Option<GameState>,
}

impl<'a> Harnais<'a> {
    fn new(db: &'a CardsDb, reponses: Vec<Value>) -> Harnais<'a> {
        Harnais {
            db,
            reponses,
            curseur: 0,
            attente: None,
            vue: None,
            mains: [Vec::new(), Vec::new()],
            erreur: None,
            defaut: RandomPolicy,
            occasions: 0,
            occasions_a_l_arret: 0,
            occasions_ouvertes: Vec::new(),
            cherche_moment: false,
            essai_rang: 0,
            essai_occasion: None,
            moment: false,
            occasions_moment: 0,
            deck_vu: 0,
            oceans_vus: 0,
            corpos_vues: 0,
            main_connue: Vec::new(),
            main_connue_siege: 0,
            siege_moment: 0,
            en_mise_en_place: false,
            cloner_etat: false,
            etat_vu: None,
        }
    }

    /// **Le moment de l'essai est atteint.** On gèle ce qu'on a relevé et l'on
    /// arrête la passe 1 : la suite sera rejouée depuis le point de reprise, avec
    /// l'avenir rebattu. `attente` sert d'interrupteur — `play_round` ne sait pas
    /// s'interrompre, mais une politique qui n'attend plus rien répond par défaut
    /// et la manche se termine à vide.
    fn atteindre_le_moment(&mut self, siege: usize) {
        self.moment = true;
        self.occasions_moment = self.occasions;
        self.siege_moment = siege;
        if self.attente.is_none() {
            self.attente = Some(Value::Null);
        }
    }

    /// Rend la réponse enregistrée pour cette décision, ou `None` s'il faut la
    /// demander (le descripteur est alors mémorisé, une seule fois).
    fn prendre(&mut self, desc: Value) -> Option<Value> {
        if self.attente.is_some() {
            return None; // décisions suivantes : réponse par défaut, jetée
        }
        if self.curseur < self.reponses.len() {
            // (regles-de-la-vente) Une vente est une entrée d'OCCASION : elle se
            // consomme à un point d'occasion, jamais comme réponse à une
            // question. Si elle arrive jusqu'ici, c'est que la page l'a inscrite
            // à un endroit où le moteur n'offrait pas de vente — la faute est
            // DÉCLARÉE, et la page retire l'entrée, plutôt que de la voir
            // interprétée comme un indice de choix et d'empoisonner le rejeu.
            if self.reponses[self.curseur].get("vendre").is_some() {
                self.faute_vente(
                    "une vente est proposée là où le moteur attend une réponse : \
                     aucune occasion de vendre n'est ouverte à ce point"
                        .to_string(),
                );
                return None;
            }
            let r = self.reponses[self.curseur].clone();
            let siege = self.siege_moment;
            self.curseur += 1;
            // (le-pont-ne-triche-plus) La décision essayée vient d'être prise :
            // c'est ici le moment de l'essai, quand il ne s'agit pas d'une vente.
            if self.cherche_moment
                && !self.moment
                && self.essai_occasion.is_none()
                && self.curseur == self.essai_rang + 1
            {
                self.atteindre_le_moment(siege);
            }
            return Some(r);
        }
        let mut d = desc;
        d["rang"] = json!(self.curseur);
        let siege = self.siege_moment;
        self.attente = Some(d);
        if self.cherche_moment
            && !self.moment
            && self.essai_occasion.is_none()
            && self.curseur == self.essai_rang
        {
            self.atteindre_le_moment(siege);
        }
        None
    }

    fn faute(&mut self, quoi: String) {
        if self.erreur.is_none() {
            self.erreur = Some(format!("décision n°{} : {}", self.curseur - 1, quoi));
        }
    }

    /// (regles-de-la-vente) Faute sur une ENTRÉE de vente. Elle porte son propre
    /// libellé parce qu'une vente n'est pas une décision numérotée : la page a
    /// besoin de savoir laquelle retirer, et `self.curseur` peut valoir 0 —
    /// `faute` y soustrairait 1 et déborderait.
    fn faute_vente(&mut self, quoi: String) {
        if self.erreur.is_none() {
            self.erreur = Some(format!("entrée n°{} (vente) : {}", self.curseur, quoi));
        }
    }

    /// Réponse entière bornée à `0..n` (n exclu) ; hors bornes = faute déclarée.
    fn indice(&mut self, r: &Value, n: usize) -> Option<usize> {
        match r.as_u64() {
            Some(i) if (i as usize) < n => Some(i as usize),
            _ => {
                self.faute(format!("indice {r} hors de 0..{n}"));
                None
            }
        }
    }

    /// Comme `liste`, mais SANS nombre imposé : de zéro à `n` indices, tous
    /// distincts et dans les bornes. Sert au mulligan projets.
    fn liste_libre(&mut self, r: &Value, n: usize) -> Option<Vec<usize>> {
        let Some(a) = r.as_array() else {
            self.faute(format!("liste attendue, reçu {r}"));
            return None;
        };
        let mut v: Vec<usize> = Vec::with_capacity(a.len());
        for x in a {
            match x.as_u64() {
                Some(i) if (i as usize) < n && !v.contains(&(i as usize)) => v.push(i as usize),
                _ => {
                    self.faute(format!("indice {x} invalide ou en double (0..{n})"));
                    return None;
                }
            }
        }
        Some(v)
    }

    fn liste(&mut self, r: &Value, n: usize, attendu: usize) -> Option<Vec<usize>> {
        let Some(a) = r.as_array() else {
            self.faute(format!("liste attendue, reçu {r}"));
            return None;
        };
        let mut v: Vec<usize> = Vec::with_capacity(a.len());
        for x in a {
            match x.as_u64() {
                Some(i) if (i as usize) < n && !v.contains(&(i as usize)) => v.push(i as usize),
                _ => {
                    self.faute(format!("indice {x} invalide ou en double (0..{n})"));
                    return None;
                }
            }
        }
        if v.len() != attendu {
            self.faute(format!("{} indices donnés, {attendu} attendus", v.len()));
            return None;
        }
        Some(v)
    }

    fn cartes(&self, ids: &[u16]) -> Vec<Value> {
        ids.iter().map(|c| carte_json(self.db, *c)).collect()
    }

    /// La carte d'indice `i` dans la main OBSERVÉE du joueur, s'il y en a une.
    fn carte_de_main(&self, joueur: usize, i: usize) -> Value {
        match self.mains.get(joueur).and_then(|m| m.get(i)) {
            Some(id) => carte_json(self.db, *id),
            None => Value::Null,
        }
    }

    /// Nom imprimé d'une carte projet, pour rédiger une question.
    fn nom_carte(&self, id: u16) -> String {
        match self.db.projects.get(id as usize) {
            Some(c) => c.name.clone(),
            None => format!("carte {id}"),
        }
    }

    /// **(choix-parlants) La question et les options, pour chacune des onze
    /// natures de choix.**
    ///
    /// Une par variante de [`ChoiceContext`], aucune rédaction générique : c'est
    /// ce qui distingue « dire de quoi on parle » de « renuméroter des boutons ».
    /// Tout ce qui est affiché est lu sur le contexte que le moteur a construit.
    fn decrire_choix(&self, ctx: &ChoiceContext) -> Value {
        match ctx {
            ChoiceContext::CorpTrBoost {
                corporation,
                cost_mc,
                steps,
            } => {
                let corpo = corporation.map(|c| corpo_json(self.db, c));
                json!({
                    "question": format!(
                        "Votre corporation vous propose de payer {cost_mc} MC pour {steps} pas de NT \
                         supplémentaire(s). Payer ?"
                    ),
                    "options": [
                        { "libelle": format!("Payer {cost_mc} MC et gagner {steps} NT"),
                          "cout_mc": cost_mc, "pas_nt": steps },
                        { "libelle": "Ne pas payer", "cout_mc": 0, "pas_nt": 0 },
                    ],
                    "corporation": corpo,
                })
            }

            ChoiceContext::PhaseUpgrade {
                candidates,
                imposed_phase,
                source,
            } => json!({
                "question": match imposed_phase {
                    Some(ph) => format!(
                        "Améliorez votre carte Phase {} : quelle variante ?", nom_phase(*ph)
                    ),
                    None => "Améliorez une carte Phase : laquelle, et en quelle variante ?"
                        .to_string(),
                },
                // Phase ET variante par option : c'est le couple qui désigne le
                // visuel de la carte améliorée à afficher.
                "options": candidates.iter().map(|c| json!({
                    "libelle": format!(
                        "{} — variante {} : {}", nom_phase(c.phase), c.variant.label(), c.name
                    ),
                    "phase": c.phase,
                    "variante": c.variant.label(),
                    "nom": c.name,
                })).collect::<Vec<_>>(),
                "phase_imposee": imposed_phase,
                "origine": nom_origine_amelioration(*source),
            }),

            ChoiceContext::CardAlternative {
                card,
                source,
                branches,
            } => json!({
                "question": format!(
                    "« {} » vous laisse le choix : quelle proposition appliquez-vous ?",
                    self.nom_carte(*card)
                ),
                "options": branches.iter().map(|b| json!({
                    "libelle": describe_branch(b.effects),
                    "rang_imprime": b.printed_rank,
                })).collect::<Vec<_>>(),
                "carte": carte_json(self.db, *card),
                "origine": nom_origine_amelioration(*source),
            }),

            ChoiceContext::ActionAlternative { card, branches } => json!({
                "question": format!(
                    "Action de « {} » : quelle proposition appliquez-vous ?",
                    self.nom_carte(*card)
                ),
                "options": branches.iter().map(|b| json!({
                    "libelle": describe_branch(b.effects),
                    "rang_imprime": b.printed_rank,
                })).collect::<Vec<_>>(),
                "carte": carte_json(self.db, *card),
            }),

            ChoiceContext::MicrobeDiscount {
                card,
                holder,
                count,
                amount,
            } => json!({
                "question": format!(
                    "Dépenser {count} microbe(s) de « {} » pour payer {amount} MC de moins sur \
                     « {} » ?",
                    self.nom_carte(*holder), self.nom_carte(*card)
                ),
                "options": [
                    { "libelle": format!("Oui : −{amount} MC contre {count} microbe(s)"),
                      "microbes": count, "reduction_mc": amount },
                    { "libelle": "Non : garder les microbes et payer le prix plein",
                      "microbes": 0, "reduction_mc": 0 },
                ],
                "carte": carte_json(self.db, *card),
                "carte_porteuse": carte_json(self.db, *holder),
            }),

            ChoiceContext::PlantDiscount {
                card,
                plants,
                amount,
            } => json!({
                "question": format!(
                    "Dépenser {plants} plante(s) pour payer {amount} MC de moins sur « {} » ?",
                    self.nom_carte(*card)
                ),
                "options": [
                    { "libelle": format!("Oui : −{amount} MC contre {plants} plante(s)"),
                      "plantes": plants, "reduction_mc": amount },
                    { "libelle": "Non : garder les plantes et payer le prix plein",
                      "plantes": 0, "reduction_mc": 0 },
                ],
                "carte": carte_json(self.db, *card),
            }),

            ChoiceContext::HeatAsMc { card, cost } => json!({
                "question": format!(
                    "« {} » coûte {cost} MC : convertir votre chaleur en MC pour la payer ?",
                    self.nom_carte(*card)
                ),
                "options": [
                    { "libelle": "Oui : payer en convertissant de la chaleur" },
                    { "libelle": "Non : payer en défaussant des cartes" },
                ],
                "carte": carte_json(self.db, *card),
                "cout": cost,
            }),

            ChoiceContext::DiscardToDraw {
                card,
                tag,
                draw_if,
                draw_else,
            } => json!({
                "question": format!(
                    "{} : défausser une carte pour en piocher {draw_else} — ou {draw_if} si elle \
                     porte un badge {} ?",
                    match card {
                        Some(c) => format!("« {} »", self.nom_carte(*c)),
                        None => "Votre corporation".to_string(),
                    },
                    tag_label(*tag)
                ),
                "options": [
                    { "libelle": format!(
                        "Défausser une carte (piocher {draw_if} avec un badge {}, {draw_else} \
                         sinon)", tag_label(*tag)) },
                    { "libelle": "Ne rien défausser" },
                ],
                "carte": card.map(|c| carte_json(self.db, c)),
                "badge": tag_label(*tag),
            }),

            ChoiceContext::SpendAmount {
                source,
                spend,
                gain,
                max,
            } => json!({
                "question": format!(
                    "Combien de {} dépenser (1 à {max}) pour gagner autant de {} ?",
                    action_res_label(*spend), action_res_label(*gain)
                ),
                // Quantités CROISSANTES : l'option k vaut k+1 unités. La clé
                // `quantite` le dit à l'écran, qui peut donc offrir un curseur
                // plutôt qu'une rangée de boutons.
                // La quantité de l'option k est celle du moteur
                // (`choice::spend_amount_quantity`) : le pont ne réécrit pas la
                // correspondance « option k = k+1 unités ».
                "options": (0..*max as usize).map(|k| {
                    let q = spend_amount_quantity(k);
                    json!({
                        "libelle": format!(
                            "Dépenser {} pour gagner {}",
                            action_res_quantity(*spend, q), action_res_quantity(*gain, q)
                        ),
                        "quantite": q,
                    })
                }).collect::<Vec<_>>(),
                "quantites_croissantes": true,
                "minimum": 1,
                "maximum": max,
                "carte": match source {
                    ActionSource::Card(c) => carte_json(self.db, *c),
                    ActionSource::Corp => Value::Null,
                },
            }),

            ChoiceContext::SelectorBonus {
                phase,
                variant,
                card_name,
                branches,
            } => json!({
                "question": format!(
                    "Bonus du sélectionneur de « {card_name} » ({}) : lequel prenez-vous ?",
                    nom_phase(*phase)
                ),
                "options": branches.iter().map(|g| json!({
                    "libelle": describe_selector_grant(g),
                })).collect::<Vec<_>>(),
                "phase": phase,
                "variante": variant.map(|v| v.label()),
                "nom": card_name,
            }),

            ChoiceContext::ReplayProduction { candidates } => json!({
                "question": "Quelle carte verte rejoue son effet de production ?",
                "options": candidates.iter().map(|c| {
                    let mut o = carte_json(self.db, c.card);
                    let nom = o["nom"].as_str().unwrap_or("?").to_string();
                    // Ce que le rejeu RAPPORTE, tel que le moteur l'a mesuré.
                    let mut gains: Vec<String> = Vec::new();
                    if c.mc != 0 { gains.push(format!("{} MC", c.mc)); }
                    if c.heat != 0 { gains.push(format!("{} chaleur", c.heat)); }
                    if c.plants != 0 { gains.push(format!("{} plantes", c.plants)); }
                    if c.cards != 0 { gains.push(format!("{} carte(s) piochée(s)", c.cards)); }
                    o["libelle"] = json!(if gains.is_empty() {
                        nom
                    } else {
                        format!("{nom} — {}", gains.join(", "))
                    });
                    o["production"] = json!({
                        "mc": c.mc, "chaleur": c.heat, "plantes": c.plants, "cartes": c.cards,
                    });
                    o
                }).collect::<Vec<_>>(),
            }),
        }
    }
}

/// D'où vient une amélioration de carte Phase, en français.
fn nom_origine_amelioration(src: UpgradeSource) -> &'static str {
    match src {
        UpgradeSource::Build => "pose d'une carte",
        UpgradeSource::Action => "action d'une carte",
        UpgradeSource::Setup => "mise en place de la corporation",
    }
}

impl Policy for Harnais<'_> {
    fn observe(&mut self, game: &GameState, player: usize) {
        if self.attente.is_none() && self.curseur == self.reponses.len() {
            self.vue = Some(state_view(game, self.db));
            self.mains = [game.players[0].hand.clone(), game.players[1].hand.clone()];
            if self.cloner_etat {
                self.etat_vu = Some(game.clone());
            }
        }
        // **(le-pont-ne-triche-plus) CE QUE LE JOUEUR A DÉJÀ VU.** Recopié de
        // `Joueur::observe` : à chaque observation, l'état des trois tas cachés et
        // — pendant la seule mise en place — la main qu'il a sous les yeux. La
        // dernière valeur retenue avant le moment de l'essai est celle qui borne
        // le rebattage : on ne rebat jamais ce que le joueur a déjà vu sortir.
        if self.cherche_moment && !self.moment {
            self.deck_vu = game.deck.len();
            self.oceans_vus = game.oceans_revealed as usize;
            self.corpos_vues = game.corp_deck.len();
            self.siege_moment = player;
            if self.en_mise_en_place {
                self.main_connue.clear();
                self.main_connue.extend_from_slice(&game.players[player].hand);
                self.main_connue_siege = player;
            }
        }
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        let desc = json!({
            "type": "corp_mulligan",
            "joueur": player,
            "question": "Remplacer vos 2 corporations par 2 nouvelles ?",
            "options": [ { "libelle": "Garder" }, { "libelle": "Remplacer les 2" } ],
            "corporations": corps.iter().map(|c| corpo_json(self.db, *c)).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, 2).map(|i| i == 1).unwrap_or(false),
            None => self.defaut.corp_mulligan(rng, player, corps),
        }
    }

    /// Le mulligan projets n'est PAS du tout ou rien : le joueur coche entre 0
    /// et 8 cartes. `a_choisir` est absent — c'est ce qui signale à l'écran un
    /// nombre libre, là où `discard_down` en impose un.
    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        let desc = json!({
            "type": "project_mulligan",
            "joueur": player,
            "question": "Quelles cartes projets remplacez-vous ? (de 0 à 8)",
            "options": hand.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
            "multiple": true,
            "main": self.cartes(hand),
        });
        match self.prendre(desc) {
            Some(r) => match self.liste_libre(&r, hand.len()) {
                Some(v) => v,
                None => self.defaut.project_mulligan(rng, player, hand),
            },
            None => self.defaut.project_mulligan(rng, player, hand),
        }
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        let desc = json!({
            "type": "pick_corporation",
            "joueur": player,
            "question": "Choisissez votre corporation",
            "options": corps.iter().map(|c| {
                let mut o = corpo_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, corps.len()).unwrap_or(0),
            None => self.defaut.pick_corporation(rng, player, corps),
        }
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        let desc = json!({
            "type": "pick_phase",
            "joueur": player,
            "question": "Choisissez votre carte Phase",
            "options": allowed.iter().map(|n| json!({
                "libelle": nom_phase(*n), "phase": n,
            })).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => match self.indice(&r, allowed.len()) {
                Some(i) => allowed[i],
                None => allowed[0],
            },
            None => self.defaut.pick_phase(rng, player, allowed),
        }
    }

    /// **(moteur-questions-manquantes) La question est posée MÊME quand aucune
    /// carte n'est payable.**
    ///
    /// Le pont escamotait le point de décision (`if affordable.is_empty() {
    /// return None }`) : la page n'avait alors l'occasion ni de poser la
    /// question, ni d'offrir la vente que le moteur venait pourtant d'ouvrir
    /// juste au-dessus (`flow::occasion_de_vendre`, hoistée au-dessus de
    /// l'énumération des cartes payables). Vécu le 04-08, partie `mars2`,
    /// graine 210055 : 8 MC en poche, dix cartes en main, trois bleues ou rouges
    /// à 15, 22 et 35 MC — vendre trois cartes en rapportait 9, de quoi poser la
    /// première. La question n'a jamais été posée, et la phase s'est arrêtée
    /// sans un mot.
    ///
    /// Le point de décision existe donc toujours. Sans option, il ne porte qu'un
    /// « passer » — et une phrase qui DIT pourquoi, en anglais comme le reste de
    /// l'écran de jeu. Vendre reste possible tant qu'il est ouvert : c'est une
    /// entrée d'occasion (`vendre_librement`), pas une réponse, et l'énumération
    /// est refaite sur la main d'après la vente.
    ///
    /// Aucune option nouvelle n'est ajoutée pour autant : « passer » est la
    /// seule issue, exactement l'issue que le moteur prenait tout seul.
    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        let desc = json!({
            "type": "choose_build",
            "joueur": player,
            "question": if affordable.is_empty() {
                "No card can be built this phase. You may still sell cards from your hand."
            } else {
                "Quelle carte poser ?"
            },
            "options": affordable.iter().map(|i| json!({
                "libelle": match self.carte_de_main(player, *i) {
                    Value::Null => format!("poser (main n°{i})"),
                    c => format!("poser {}", c["nom"].as_str().unwrap_or("?")),
                },
                "indice_main": i,
                "carte": self.carte_de_main(player, *i),
            })).collect::<Vec<_>>(),
            "passer": true,
        });
        match self.prendre(desc) {
            Some(r) => {
                let n = affordable.len();
                match self.indice(&r, n + 1) {
                    Some(i) if i < n => Some(affordable[i]),
                    Some(_) => None, // indice n = passer
                    None => None,
                }
            }
            None => self.defaut.choose_build(rng, player, affordable),
        }
    }

    /// **(MOT-3) Le bonus de Construction n'est plus une question, mais DEUX.**
    ///
    /// Cette méthode-ci n'est plus appelée par le déroulement : elle reste le
    /// choix de fond de la politique, et le pont la sert par les deux temps.
    /// Le corps est conservé pour les chemins qui n'ont pas de moment (la
    /// sonde, les tests) et rend la même chose qu'avant.
    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let desc = json!({
            "type": "construction_bonus",
            "joueur": player,
            "question": "Bonus du sélectionneur de la phase Construction",
            "options": [
                { "libelle": "Piocher 1 carte AVANT de poser" },
                { "libelle": "Piocher 1 carte APRÈS avoir posé" },
                { "libelle": "Poser une carte bleue/rouge supplémentaire" },
            ],
        });
        match self.prendre(desc) {
            Some(r) => match self.indice(&r, 3) {
                Some(0) => ConstructionBonus::DrawCardBefore,
                Some(1) => ConstructionBonus::DrawCard,
                Some(_) => ConstructionBonus::SecondBuild,
                None => ConstructionBonus::DrawCard,
            },
            None => self.defaut.construction_bonus(rng, player),
        }
    }

    /// (MOT-3) PREMIER TEMPS — la seule moitié qui doit se trancher avant la
    /// pose. Deux issues, pas trois : la question réduite du livret.
    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        let desc = json!({
            "type": "construction_bonus",
            "joueur": player,
            "temps": "avant",
            "question": "Piocher 1 carte tout de suite, avant de poser ?",
            "options": [
                { "libelle": "Piocher 1 carte tout de suite" },
                { "libelle": "Décider après avoir posé" },
            ],
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, 2).map(|i| i == 0).unwrap_or(false),
            None => self.defaut.construction_bonus_avant(rng, player),
        }
    }

    /// (MOT-3) SECOND TEMPS — la carte posée, le joueur sait enfin ce qu'il a
    /// pu poser. Deux issues : piocher, ou poser une seconde carte.
    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let desc = json!({
            "type": "construction_bonus",
            "joueur": player,
            "temps": "apres",
            "question": "Bonus du sélectionneur : piocher, ou poser une seconde carte ?",
            "options": [
                { "libelle": "Piocher 1 carte" },
                { "libelle": "Poser une carte bleue/rouge supplémentaire" },
            ],
        });
        match self.prendre(desc) {
            Some(r) => match self.indice(&r, 2) {
                Some(1) => ConstructionBonus::SecondBuild,
                _ => ConstructionBonus::DrawCard,
            },
            None => self.defaut.construction_bonus_apres(rng, player),
        }
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None;
        }
        let desc = json!({
            "type": "action_choice",
            "joueur": player,
            "question": "Action à activer",
            "options": options.iter().map(|o| json!({ "libelle": nom_action(self.db, o) }))
                .collect::<Vec<_>>(),
            "passer": true,
        });
        match self.prendre(desc) {
            Some(r) => {
                let n = options.len();
                match self.indice(&r, n + 1) {
                    Some(i) if i < n => Some(i),
                    _ => None,
                }
            }
            None => self.defaut.action_choice(rng, player, options),
        }
    }

    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        let desc = json!({
            "type": "action_amount",
            "joueur": player,
            "question": format!("Quel montant dépenser (0 à {max}) ?"),
            "minimum": 0,
            "maximum": max,
            "montant": true,
        });
        match self.prendre(desc) {
            Some(r) => match r.as_i64() {
                Some(x) if x >= 0 && x <= max => x,
                _ => {
                    self.faute(format!("montant {r} hors de 0..={max}"));
                    0
                }
            },
            None => self.defaut.action_amount(rng, player, max),
        }
    }

    /// **(regles-de-la-vente) La vente libre : une ENTRÉE de la liste de
    /// décisions, jamais une réponse à une question.**
    ///
    /// Le moteur n'interroge pas le joueur (« voulez-vous vendre ? ») : il fait
    /// savoir, avant chacun de ses points de décision, qu'ici une vente est
    /// recevable (`flow::occasion_de_vendre`). Ce point d'occasion ne peut donc
    /// pas mettre la partie en attente comme le fait [`Harnais::prendre`] — il
    /// n'y a pas de question à poser à la page.
    ///
    /// La page inscrit son geste dans la liste des décisions, sous la forme :
    ///
    /// ```json
    /// {"vendre": {"joueur": 0, "cartes": [3]}}
    /// ```
    ///
    /// et l'entrée est consommée à la PREMIÈRE occasion offerte au joueur
    /// nommé. Comme la page l'ajoute au moment où une décision l'attend, cette
    /// occasion est exactement celle qui précède cette décision-là : le rejeu
    /// replace la vente à l'instant où le joueur l'a faite, et l'énumération qui
    /// suit (cartes payables, contour vert) est refaite sur la main d'après.
    ///
    /// Une entrée qui n'est pas une vente n'est PAS consommée : c'est la réponse
    /// à la décision qui vient, et elle sera lue par `prendre`.
    fn vendre_librement(&mut self, _rng: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        // **LE NUMÉRO DE CETTE OCCASION.** Compté avant tout le reste et quoi
        // qu'il arrive : c'est un rang dans la partie, pas un compteur de ventes.
        // Jumeau exact de `Joueur::vendre_librement` et de `Rejeu::vendre_librement`.
        let numero = self.occasions;
        self.occasions += 1;
        if self.attente.is_none() {
            self.occasions_a_l_arret = self.occasions;
        }
        // (le-pont-ne-triche-plus) Le moment de l'essai, quand l'essai porte sur
        // une occasion de vente : il précède l'observation de la décision qui
        // suit, exactement comme dans `flow::avant_decision`.
        if self.cherche_moment && !self.moment && self.essai_occasion == Some(numero) {
            self.atteindre_le_moment(joueur);
            return Vec::new();
        }
        // Passé le point d'attente, plus rien n'est décidé : les décisions
        // suivantes prennent leur réponse par défaut et sont jetées.
        if self.attente.is_some() {
            return Vec::new();
        }
        if self.curseur >= self.reponses.len() {
            // **UNE OCCASION OUVERTE QUE PERSONNE N'A SAISIE.** Le moteur ne pose
            // pas la question ; c'est la réponse du pont qui la porte, et la page
            // y glisse — ou non — une entrée `vendre` numérotée.
            self.occasions_ouvertes.push(json!({
                "numero": numero,
                "joueur": joueur,
                "main": self.cartes(main),
            }));
            return Vec::new();
        }
        let Some(vente) = self.reponses[self.curseur].get("vendre").cloned() else {
            return Vec::new(); // réponse ordinaire : elle appartient à `prendre`
        };
        // Une vente adressée à l'AUTRE joueur attend son occasion à lui.
        if vente.get("joueur").and_then(Value::as_u64) != Some(joueur as u64) {
            return Vec::new();
        }
        // **(le-pont-ne-triche-plus, critère E) JAMAIS AVANT SON HEURE.** Une
        // vente décidée à l'occasion n ne se consomme pas à l'occasion n-1 : sans
        // ce refus, une vente décidée après coup remonterait le temps et
        // s'appliquerait à une main que le joueur n'avait pas encore. La règle est
        // donc « jamais avant son numéro, au plus tard à la première occasion
        // suivante du même siège ». Une entrée SANS numéro reste acceptée telle
        // quelle : c'est le format d'avant, et l'écran de jeu l'écrit encore.
        //
        // **UN NUMÉRO MAL FORMÉ EST REFUSÉ, PAS IGNORÉ.** `as_u64` rend « rien »
        // sur `"3"`, `1.5`, `-1`, `true`, `null`, une liste ou un objet. Se
        // contenter de `and_then(as_u64)` sauterait alors la garde, et la vente
        // retomberait à la première occasion du siège — c'est-à-dire très
        // exactement le défaut V2 que ce lot ferme, rouvert par une valeur qui a
        // transité par un relais ou une concaténation. On distingue donc « clef
        // absente » (le format d'avant, accepté) de « clef présente et illisible »
        // (une faute déclarée).
        match vente.get("occasion") {
            None | Some(Value::Null) => {}
            Some(v) => match v.as_u64() {
                Some(n) => {
                    if numero < n {
                        return Vec::new();
                    }
                }
                None => {
                    self.faute_vente(format!(
                        "« occasion » doit être un entier positif, reçu {v}"
                    ));
                    return Vec::new();
                }
            },
        }
        // L'entrée est VALIDÉE avant d'avancer le curseur : une entrée
        // malformée qui aurait quand même consommé sa place décalerait toutes
        // les réponses suivantes d'un cran, et le rejeu répondrait à côté de
        // chaque question. La faute est déclarée, la page retire l'entrée, et
        // rien n'a bougé entre-temps.
        let Some(cartes) = vente.get("cartes").and_then(Value::as_array) else {
            self.faute_vente("« cartes » attendu : une liste d'indices de main".to_string());
            return Vec::new();
        };
        let mut idx: Vec<usize> = Vec::with_capacity(cartes.len());
        for x in cartes {
            match x.as_u64() {
                Some(i) if (i as usize) < main.len() && !idx.contains(&(i as usize)) => {
                    idx.push(i as usize)
                }
                _ => {
                    self.faute_vente(format!(
                        "indice de vente {x} invalide ou en double (0..{})",
                        main.len()
                    ));
                    return Vec::new();
                }
            }
        }
        self.curseur += 1;
        idx
    }

    /// **(choix-parlants) La voie anonyme, rendue BRUYANTE.**
    ///
    /// Plus aucun site du moteur ne l'emprunte : tous passent par
    /// `choose_option_ctx`. Elle n'est pas supprimée pour autant, et c'est
    /// délibéré — sans elle, un site qui reviendrait à l'ancienne voie ferait
    /// décider le corps par défaut du trait, c'est-à-dire un tirage aléatoire
    /// **sans jamais interroger le navigateur**, silencieusement. Ici la faute
    /// est déclarée : elle remonte à la page dans le champ `erreur`.
    fn choose_option(&mut self, rng: &mut StdRng, player: usize, n: usize) -> usize {
        if self.erreur.is_none() {
            // Écrit sans passer par `faute`, qui numérote la décision à partir
            // du curseur : il n'y a pas eu de décision soumise ici, il n'y a
            // donc pas de numéro à citer.
            self.erreur = Some(format!(
                "le moteur a demandé un choix parmi {n} sans dire de quoi il s'agit \
                 (voie anonyme `choose_option`) : la page n'a pas pu poser la question"
            ));
        }
        self.defaut.choose_option(rng, player, n)
    }

    /// **(choix-parlants) La voie enrichie, et la seule que le moteur emprunte.**
    ///
    /// `choose_option` a disparu de ce pont : les onze points d'alternative de
    /// `flow.rs` passent tous par ici, et chacun sait dire de quoi il parle. Le
    /// descripteur porte donc un `type` propre à la NATURE du choix (celui du
    /// moteur, `ChoiceContext::kind`), une question rédigée pour un joueur, et
    /// des options qui portent de quoi être affichées — jamais un numéro nu.
    ///
    /// **Ce pont ne recalcule aucune règle.** Les couples (phase, variante)
    /// d'une amélioration de carte Phase, le nom de la carte améliorée
    /// correspondante et la description de chaque branche viennent tels quels du
    /// moteur (`engine::choice`). Reconstruire ici la liste « 5 phases × 2
    /// variantes moins celles déjà en place » serait une seconde implémentation
    /// de la même règle, qui divergerait au premier changement.
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        if n == 0 {
            return 0;
        }
        let mut desc = self.decrire_choix(ctx);
        desc["type"] = json!(ctx.kind());
        desc["joueur"] = json!(player);
        match self.prendre(desc) {
            Some(r) => self.indice(&r, n).unwrap_or(0),
            None => self.defaut.choose_option_ctx(rng, player, ctx),
        }
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let desc = json!({
            "type": "choose_res_target",
            "joueur": player,
            "question": "Sur quelle carte poser la ressource ?",
            "options": candidates.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, candidates.len()).unwrap_or(0),
            None => self.defaut.choose_res_target(rng, player, candidates),
        }
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let desc = json!({
            "type": "choose_res_source",
            "joueur": player,
            "question": "Sur quelle carte retirer une ressource ?",
            "options": candidates.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, candidates.len()).unwrap_or(0),
            None => self.defaut.choose_res_source(rng, player, candidates),
        }
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        card: u16,
        tag_counts: &[u32],
    ) -> usize {
        let n = engine::cards::JOKER_TAG_CHOICES.len();
        let desc = json!({
            "type": "pick_joker_tag",
            "joueur": player,
            "question": "Choisissez le badge à ajouter à cette carte",
            "carte": carte_json(self.db, card),
            "options": engine::cards::JOKER_TAG_CHOICES.iter().enumerate().map(|(i, t)| json!({
                "libelle": format!("{} (vous en avez {})", t.as_str(),
                    tag_counts.get(i).copied().unwrap_or(0)),
                "badge": t.as_str(),
            })).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, n).unwrap_or(0),
            None => self.defaut.pick_joker_tag(rng, player, card, tag_counts),
        }
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        let desc = json!({
            "type": "research_keep",
            "joueur": player,
            "question": format!("Gardez {keep} carte(s) parmi {}", drawn.len()),
            "options": drawn.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
            "a_choisir": keep,
            "multiple": true,
        });
        match self.prendre(desc) {
            Some(r) => match self.liste(&r, drawn.len(), keep) {
                Some(v) => v,
                None => self.defaut.research_keep(rng, player, drawn, keep),
            },
            None => self.defaut.research_keep(rng, player, drawn, keep),
        }
    }

    /// **La révélation du dessus de la pioche, MONTRÉE.**
    ///
    /// Le moteur retourne trois cartes face visible ; la page doit les voir,
    /// toutes les trois, à chaque fois — même quand aucune n'est prenable et
    /// qu'il n'y a rien à décider. Le descripteur porte donc DEUX listes, et la
    /// distinction entre elles est tout le sujet :
    ///
    /// - `revelees` : les trois cartes retournées, chacune marquée `prenable`
    ///   (le filtre imprimé de la carte Phase : « une carte bleue ou rouge »).
    ///   C'est de l'INFORMATION, pas un choix — aucune n'est cliquable à ce
    ///   titre. Rien n'y entre que le joueur n'ait le droit de voir : ces trois
    ///   cartes-là sont posées sur la table par la règle, et le reste de la
    ///   pioche n'est pas nommé.
    /// - `options` : les seules cartes PRENABLES, dans l'ordre du moteur. Les
    ///   indices de la réponse sont les leurs, exactement comme avant ce
    ///   chantier (`research_keep` recevait déjà les seules candidates) : aucun
    ///   fournisseur de décisions existant n'a à changer d'un signe, et aucun ne
    ///   peut désigner une carte qu'il n'a pas le droit de prendre.
    ///
    /// Quand rien n'est prenable, `options` est vide et `a_choisir` vaut 0 : la
    /// réponse attendue est la liste vide. Un fournisseur qui suit le contrat
    /// (« un tableau de `a_choisir` indices distincts ») la produit sans rien
    /// savoir de cette carte Phase.
    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        revelees: &[u16],
        candidates: &[u16],
        keep: usize,
        filtre: engine::effects::RevealFilter,
    ) -> Vec<usize> {
        // POURQUOI une carte n'est pas prenable : le filtre imprimé, rendu en
        // CLEFS et en valeurs du moteur (couleur, badges) — jamais en phrase.
        // La page en fait une phrase anglaise, comme pour tout le reste.
        let filtre_json = match filtre {
            engine::effects::RevealFilter::ColorIsNot(c) => json!({
                "sorte": "couleur_sauf",
                "couleur": c.nom_fr(),
            }),
            engine::effects::RevealFilter::AnyOfTags(t) => json!({
                "sorte": "badges",
                "badges": t.iter().map(|x| x.as_str()).collect::<Vec<_>>(),
            }),
        };
        let desc = json!({
            "type": "revelation_pioche",
            "joueur": player,
            "question": if keep == 0 {
                format!(
                    "Révélation : aucune des {} cartes du dessus n'est prenable",
                    revelees.len()
                )
            } else {
                format!(
                    "Révélation : ajoutez {keep} carte(s) à votre main parmi les {} révélées",
                    revelees.len()
                )
            },
            "revelees": revelees.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["prenable"] = json!(candidates.contains(c));
                o
            }).collect::<Vec<_>>(),
            "options": candidates.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
            "filtre": filtre_json,
            "a_choisir": keep,
            "multiple": true,
        });
        match self.prendre(desc) {
            Some(r) => match self.liste(&r, candidates.len(), keep) {
                Some(v) => v,
                None => self
                    .defaut
                    .reveal_pick(rng, player, revelees, candidates, keep, filtre),
            },
            None => self
                .defaut
                .reveal_pick(rng, player, revelees, candidates, keep, filtre),
        }
    }

    // (moteur-questions-manquantes) LA QUESTION « quelle carte vendez-vous
    // pour 3 MC ? » N'EXISTE PLUS : elle était la seconde moitié de l'action
    // standard retirée du moteur (81 décisions sur la seule graine 4242, autant
    // de tours de jeu perdus). La vente se dit maintenant par une ENTRÉE
    // `{"vendre": …}`, lue par `vendre_librement` ci-dessus, qui ne consomme
    // aucun échange et prend autant de cartes qu'on veut.

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        let desc = json!({
            "type": "discard_down",
            "joueur": player,
            "question": format!("Limite de main : défaussez {n} carte(s)"),
            "options": hand.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
            "a_choisir": n,
            "multiple": true,
        });
        match self.prendre(desc) {
            Some(r) => match self.liste(&r, hand.len(), n) {
                Some(v) => v,
                None => self.defaut.discard_down(rng, player, hand, n),
            },
            None => self.defaut.discard_down(rng, player, hand, n),
        }
    }
}

// ---------------------------------------------------------------------- op pas

/// **(le-pont-ne-triche-plus, critère A) LA GRAINE DES REJEUX D'ESSAI, ET LE
/// MOMENT AUQUEL L'ESSAI A LIEU.**
///
/// Trois nombres, et il en faut trois. `joueur.rs` ne dérive pas la graine d'un
/// essai de la seule graine d'essais : il y mêle le RANG de la décision et le
/// COMPTE DES OCCASIONS DE VENTE déjà ouvertes, pour que deux décisions
/// successives — et deux occasions déclinées de suite — n'explorent pas le même
/// avenir imaginaire. Et le rebattage lui-même ne peut pas toucher ce que le
/// joueur a déjà vu sortir : il faut donc savoir à quel INSTANT de la partie
/// l'essai se place. Un nombre seul ne le dirait pas.
///
/// - `graine` : `--graine-essais` du natif. **Zéro est une valeur**, pas une
///   absence : c'est `joueur::GRAINE_ESSAIS_DEFAUT`. C'est la PRÉSENCE de la clef
///   `graine_essais` qui allume le rebattage, jamais sa valeur.
/// - `rang` : le nombre de décisions déjà inscrites au journal, c'est-à-dire
///   l'indice de l'entrée essayée.
/// - `occasion` : le numéro de l'occasion de vente essayée, quand l'essai porte
///   sur une vente. Absent pour une décision ordinaire.
struct Essais {
    graine: u64,
    rang: usize,
    occasion: Option<u64>,
}

/// Lit le descripteur d'essai de la requête. Absent = jeu normal, et le pont se
/// comporte alors **exactement** comme avant ce lot : aucun rebattage.
fn essais(v: &Value) -> Result<Option<Essais>, String> {
    match v.get("graine_essais") {
        None | Some(Value::Null) => Ok(None),
        _ => {
            let graine = nombre_u64(v, "graine_essais", 0)?;
            // Le rang n'est pas devinable : une valeur par défaut ferait juger
            // toutes les décisions sur le même avenir imaginaire, en silence.
            if v.get("rang_essais").is_none() {
                return Err(
                    "rang_essais manquant : une graine d'essais sans rang ne dit pas \
                     À QUELLE décision l'essai se place"
                        .to_string(),
                );
            }
            let rang = nombre_usize(v, "rang_essais", 0)?;
            let occasion = match v.get("occasion_essais") {
                None | Some(Value::Null) => None,
                _ => Some(nombre_u64(v, "occasion_essais", 0)?),
            };
            Ok(Some(Essais {
                graine,
                rang,
                occasion,
            }))
        }
    }
}

/// Le point où un rejeu d'essai reprend la partie : le clone de l'état au début
/// de la manche en cours, ou bien la mise en place. Jumeau de `joueur::Reprise`.
struct Reprise {
    base: Option<GameState>,
    curseur: usize,
    occasions: u64,
}

/// Rejoue la partie `seed` avec les décisions déjà prises et s'arrête à la
/// première décision non prise. Le moteur fait tout : `setup_game` puis
/// `play_round`, exactement comme `engine::sim::play_game`.
///
/// Pourquoi le rejeu et pas une suspension : `catch_unwind` ne rattrape rien en
/// WebAssembly (une panique y devient un déroutement irrattrapable), le pont ne
/// peut donc pas s'arrêter au milieu d'une manche. C'est le chemin que le
/// contrat recommande — le moteur joue 2000 parties en 274 ms.
///
/// **(le-pont-ne-triche-plus) AVEC UNE GRAINE D'ESSAIS, LE PONT NE MONTRE PLUS
/// L'AVENIR.** Sans elle, rien ne change. Avec elle, l'appel se fait en DEUX
/// PASSES :
///
/// 1. la première rejoue la vraie partie jusqu'au *moment de l'essai* — la
///    décision de rang `rang`, ou l'occasion de vente numéro `occasion`. Elle en
///    rapporte le point de reprise (le début de la manche en cours), le compte
///    des occasions à ce point, et ce que le joueur avait déjà vu sortir des
///    trois tas cachés ;
/// 2. la seconde repart de ce point, **rebat ce qui reste caché** par
///    `joueur::rebattre_le_reste` — le même code que l'entraînement, appelé, pas
///    recopié — et rejoue la liste des décisions.
///
/// Pourquoi deux passes et non un rebattage unique juste après `setup_game` : le
/// rebattage ré-ensemence aussi le générateur de la partie. Le faire à la mise en
/// place changerait les manches déjà jouées (recharge du paquet, tirage des
/// jalons et des récompenses) et le pont ne rendrait plus ce que rend le natif.
fn pas(db: &CardsDb, seed: u64, decisions: Vec<Value>, essais: Option<Essais>) -> Value {
    let donnees = decisions.len();
    let Some(e) = essais else {
        let mut pol = Harnais::new(db, decisions);
        let mut game = setup_game(db, seed, &mut pol);
        while pol.attente.is_none() && !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, db, &mut pol);
        }
        return rendre(db, seed, donnees, pol, game, None);
    };

    // ---- passe 1 : où l'essai se place, et ce que le joueur a déjà vu
    let mut p1 = Harnais::new(db, decisions.clone());
    p1.cherche_moment = true;
    p1.essai_rang = e.rang;
    p1.essai_occasion = e.occasion;
    p1.en_mise_en_place = true;
    let mut g1 = setup_game(db, seed, &mut p1);
    let mut reprise = Reprise {
        base: None,
        curseur: 0,
        occasions: 0,
    };
    if !p1.moment {
        p1.en_mise_en_place = false;
        while !p1.moment && !g1.game_over && g1.generation <= MAX_GENERATIONS {
            // Le clone du DÉBUT de manche, comme `Joueur::debut_manche`.
            reprise = Reprise {
                base: Some(g1.clone()),
                curseur: p1.curseur,
                occasions: p1.occasions,
            };
            play_round(&mut g1, db, &mut p1);
        }
    }
    if let Some(err) = p1.erreur.clone() {
        return erreur(err);
    }
    // **UN MOMENT D'ESSAI INATTEIGNABLE EST UNE FAUTE, PAS UN SILENCE.** Si la
    // passe 1 n'a jamais atteint le moment demandé — rang au-delà des décisions
    // que la partie pose, rang qui tombe sur une entrée `vendre` (elle se
    // consomme à une occasion, pas à une question), ou numéro d'occasion jamais
    // ouvert — alors `reprise` porte la DERNIÈRE manche jouée et rien d'autre.
    // Rebattre là rendrait un écran sans rapport avec l'essai demandé, et
    // personne ne le saurait : c'est très exactement la classe de défaut que ce
    // lot ferme. On le déclare.
    if !p1.moment {
        return erreur(match e.occasion {
            Some(n) => format!(
                "essai impossible : l'occasion de vente n°{n} ne s'ouvre jamais \
                 dans cette partie"
            ),
            None => format!(
                "essai impossible : la décision de rang {} n'est jamais prise \
                 dans cette partie (rang au-delà de la partie, ou entrée « vendre » \
                 à ce rang)",
                e.rang
            ),
        });
    }

    // ---- la graine dérivée : `joueur::graine_du_rejeu`, terme pour terme
    let compte_occasions = if p1.moment {
        p1.occasions_moment
    } else {
        p1.occasions
    };
    let graine = joueur::brasser(e.graine)
        ^ joueur::brasser(seed ^ 0xA5A5_A5A5_A5A5_A5A5)
        ^ joueur::brasser(e.rang as u64)
        ^ joueur::brasser(compte_occasions.wrapping_mul(0x1000_0001));

    // ---- passe 2 : depuis le point de reprise, l'avenir rebattu
    let mise_en_place = reprise.base.is_none();
    let mut p2 = Harnais::new(db, decisions);
    p2.curseur = reprise.curseur;
    p2.occasions = reprise.occasions;
    // **LE COMPTE PUBLIÉ REPART LUI AUSSI DU POINT DE REPRISE.** `occasions` est
    // le rang courant, `occasions_a_l_arret` est ce que la page lira ; les deux
    // sont repris ensemble. Ne recaler que le premier laisserait le second à
    // zéro quand la passe 2 s'arrête sans avoir ouvert la moindre occasion — la
    // page verrait le compteur retomber à 0 au milieu d'une partie.
    p2.occasions_a_l_arret = reprise.occasions;
    p2.cloner_etat = mise_en_place;
    let mut g2 = match reprise.base {
        None => {
            // La mise en place repasse par la VRAIE graine : c'est là que sont la
            // main et les corporations que le joueur a sous les yeux. Le paquet
            // n'est rebattu qu'ensuite, pour les manches qui suivent ; la voyance
            // du mulligan, elle, est retirée plus bas par
            // `joueur::ecarter_les_cartes_du_futur`.
            let mut g = setup_game(db, seed, &mut p2);
            joueur::rebattre_l_avenir(&mut g, graine);
            g
        }
        Some(base) => {
            let mut g = base.clone();
            // Pioche rechargée depuis la défausse : le paquet du début de manche
            // est plus court que ce que le joueur a vu, on ne rebat alors rien.
            let recharge = p1.deck_vu > base.deck.len();
            let vu = joueur::DejaVu {
                cartes: if recharge {
                    base.deck.len()
                } else {
                    base.deck.len() - p1.deck_vu
                },
                oceans: p1.oceans_vus,
                corpos: base.corp_deck.len().saturating_sub(p1.corpos_vues),
            };
            joueur::rebattre_le_reste(&mut g, graine, vu);
            g
        }
    };
    while p2.attente.is_none() && !g2.game_over && g2.generation <= MAX_GENERATIONS {
        play_round(&mut g2, db, &mut p2);
    }
    // Et l'on retire de l'état atteint les cartes que le mulligan a fait
    // repiocher dans le paquet de la VRAIE partie (défaut V1, `joueur.rs`).
    let amender = if mise_en_place && p1.main_connue_siege == p1.siege_moment {
        Some((p1.main_connue.clone(), p1.siege_moment, graine))
    } else {
        None
    };
    rendre(db, seed, donnees, p2, g2, amender)
}

/// La réponse de l'op `pas`, une fois le rejeu fini. Séparée pour que le jeu
/// normal et le rejeu d'essai la construisent par le même chemin.
fn rendre(
    db: &CardsDb,
    seed: u64,
    donnees: usize,
    mut pol: Harnais,
    game: GameState,
    amender: Option<(Vec<u16>, usize, u64)>,
) -> Value {
    if let Some(e) = pol.erreur {
        return erreur(e);
    }
    let termine = pol.attente.is_none();
    let fin = if termine {
        let (scores, _, _) = score_parts(&game, db);
        Some((scores[0], scores[1], game.turn_order.len(), game.game_over))
    } else {
        None
    };
    // L'état rendu : celui que `Policy::observe` a reçu juste avant la décision
    // en attente (le point de vue du moteur, à l'instant du choix), ou l'état
    // final si la partie est finie. Jumeau de `joueur::etat_atteint`.
    let etat = match amender {
        Some((main_connue, siege, graine)) => {
            let mut g = if termine {
                game
            } else {
                pol.etat_vu.take().unwrap_or(game)
            };
            joueur::ecarter_les_cartes_du_futur(&main_connue, &mut g, siege, graine);
            state_view(&g, db)
        }
        None => {
            if termine {
                state_view(&game, db)
            } else {
                pol.vue.clone().unwrap_or_else(|| state_view(&game, db))
            }
        }
    };
    let mut out = json!({
        "ok": true,
        "graine": seed,
        "decisions_prises": donnees,
        "termine": termine,
        "etat": etat,
        "decision": pol.attente,
        // (le-pont-ne-triche-plus) Le compte des occasions de vente ouvertes
        // depuis le début de la partie, et celles que personne n'a saisies à
        // l'instant de la décision en attente.
        "occasions": pol.occasions_a_l_arret,
        "occasions_ouvertes": pol.occasions_ouvertes,
    });
    if let Some((a, b, manches, complete)) = fin {
        // `generation` est incrémentée en fin de manche : le nombre de manches
        // JOUÉES est la longueur de l'ordre du tour relevé par le moteur.
        out["scores"] = json!([a, b]);
        out["manches"] = json!(manches);
        out["partie_complete"] = json!(complete);
    }
    out
}

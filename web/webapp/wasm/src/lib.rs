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
use engine::flow::{play_round, score_parts, setup_game, SelectorBonus};
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
            pas(db, seed, decisions)
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

fn carte_json(db: &CardsDb, id: u16) -> Value {
    match db.projects.get(id as usize) {
        Some(c) => json!({
            "id": id,
            "nom": c.name,
            "prix": c.price,
            "couleur": c.color.nom_fr(),
            "badges": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "pv": c.vp,
        }),
        None => json!({ "id": id, "nom": format!("carte inconnue {id}") }),
    }
}

fn corpo_json(db: &CardsDb, id: u16) -> Value {
    match db.corporations.get(id as usize) {
        Some(c) => json!({
            "id": id,
            "nom": c.name,
            "mc_depart": c.starting_mc,
            "badges": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        }),
        None => json!({ "id": id, "nom": format!("corporation inconnue {id}") }),
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
        ActionOpt::SellCard => "Défausser 1 carte pour du MC".to_string(),
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
        }
    }

    /// Rend la réponse enregistrée pour cette décision, ou `None` s'il faut la
    /// demander (le descripteur est alors mémorisé, une seule fois).
    fn prendre(&mut self, desc: Value) -> Option<Value> {
        if self.attente.is_some() {
            return None; // décisions suivantes : réponse par défaut, jetée
        }
        if self.curseur < self.reponses.len() {
            let r = self.reponses[self.curseur].clone();
            self.curseur += 1;
            return Some(r);
        }
        let mut d = desc;
        d["rang"] = json!(self.curseur);
        self.attente = Some(d);
        None
    }

    fn faute(&mut self, quoi: String) {
        if self.erreur.is_none() {
            self.erreur = Some(format!("décision n°{} : {}", self.curseur - 1, quoi));
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
}

impl Policy for Harnais<'_> {
    fn observe(&mut self, game: &GameState, _player: usize) {
        if self.attente.is_none() && self.curseur == self.reponses.len() {
            self.vue = Some(state_view(game, self.db));
            self.mains = [game.players[0].hand.clone(), game.players[1].hand.clone()];
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

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        if affordable.is_empty() {
            return None;
        }
        let desc = json!({
            "type": "choose_build",
            "joueur": player,
            "question": "Quelle carte poser ?",
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

    fn discard_payment_count(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        mc: i64,
        cost: i64,
        hand: &[u16],
        rate: i64,
    ) -> usize {
        // Minimum imposé par le moteur : de quoi couvrir le coût. Ce n'est pas
        // une règle réécrite — c'est le corps par défaut du trait `Policy`, que
        // le moteur applique lui-même quand la politique ne répond pas.
        let taux = rate.max(1);
        let manque = cost - mc;
        let mini = if manque <= 0 {
            0
        } else {
            (((manque + taux - 1) / taux) as usize).min(hand.len())
        };
        let desc = json!({
            "type": "discard_payment_count",
            "joueur": player,
            "question": format!(
                "Coût {cost} MC, vous avez {mc} MC : combien de cartes défausser ({taux} MC par carte) ?"
            ),
            "mc": mc, "cout": cost, "taux": taux,
            "minimum": mini, "maximum": hand.len(),
            "main": self.cartes(hand),
            "montant": true,
        });
        match self.prendre(desc) {
            Some(r) => match r.as_u64() {
                Some(x) if (x as usize) >= mini && (x as usize) <= hand.len() => x as usize,
                _ => {
                    self.faute(format!("nombre de cartes {r} hors de {mini}..={}", hand.len()));
                    mini
                }
            },
            None => self
                .defaut
                .discard_payment_count(rng, player, mc, cost, hand, rate),
        }
    }

    fn choose_option(&mut self, rng: &mut StdRng, player: usize, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        // Le moteur ne donne QUE le nombre de branches jouables (dans l'ordre du
        // texte imprimé) : il n'y a pas de libellé à afficher, la page numérote.
        let desc = json!({
            "type": "choose_option",
            "joueur": player,
            "question": "Choisissez une branche du texte de la carte (ordre du texte imprimé)",
            "options": (0..n).map(|i| json!({ "libelle": format!("branche {}", i + 1) }))
                .collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self.indice(&r, n).unwrap_or(0),
            None => self.defaut.choose_option(rng, player, n),
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

    /// Vente d'une carte pour 3 MC : c'est le JOUEUR qui désigne laquelle.
    fn sell_card(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> usize {
        let desc = json!({
            "type": "sell_card",
            "joueur": player,
            "question": "Quelle carte vendez-vous pour 3 MC ?",
            "options": hand.iter().map(|c| {
                let mut o = carte_json(self.db, *c);
                o["libelle"] = json!(o["nom"].as_str().unwrap_or("?"));
                o
            }).collect::<Vec<_>>(),
        });
        match self.prendre(desc) {
            Some(r) => self
                .indice(&r, hand.len())
                .unwrap_or_else(|| self.defaut.sell_card(rng, player, hand)),
            None => self.defaut.sell_card(rng, player, hand),
        }
    }

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

/// Rejoue la partie `seed` avec les décisions déjà prises et s'arrête à la
/// première décision non prise. Le moteur fait tout : `setup_game` puis
/// `play_round`, exactement comme `engine::sim::play_game`.
///
/// Pourquoi le rejeu et pas une suspension : `catch_unwind` ne rattrape rien en
/// WebAssembly (une panique y devient un déroutement irrattrapable), le pont ne
/// peut donc pas s'arrêter au milieu d'une manche. C'est le chemin que le
/// contrat recommande — le moteur joue 2000 parties en 274 ms.
fn pas(db: &CardsDb, seed: u64, decisions: Vec<Value>) -> Value {
    let donnees = decisions.len();
    let mut pol = Harnais::new(db, decisions);
    let mut game = setup_game(db, seed, &mut pol);
    while pol.attente.is_none() && !game.game_over && game.generation <= MAX_GENERATIONS {
        play_round(&mut game, db, &mut pol);
    }
    if let Some(e) = pol.erreur {
        return erreur(e);
    }
    let termine = pol.attente.is_none();
    // L'état rendu : celui que `Policy::observe` a reçu juste avant la décision
    // en attente (le point de vue du moteur, à l'instant du choix), ou l'état
    // final si la partie est finie.
    let etat = if termine {
        state_view(&game, db)
    } else {
        pol.vue.clone().unwrap_or_else(|| state_view(&game, db))
    };
    let mut out = json!({
        "ok": true,
        "graine": seed,
        "decisions_prises": donnees,
        "termine": termine,
        "etat": etat,
        "decision": pol.attente,
    });
    if termine {
        let (scores, _, _) = score_parts(&game, db);
        // `generation` est incrémentée en fin de manche : le nombre de manches
        // JOUÉES est la longueur de l'ordre du tour relevé par le moteur.
        out["scores"] = json!([scores[0], scores[1]]);
        out["manches"] = json!(game.turn_order.len());
        out["partie_complete"] = json!(game.game_over);
    }
    out
}

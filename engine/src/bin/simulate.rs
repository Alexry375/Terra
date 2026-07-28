//! Binaire `simulate` : joue N parties aléatoires et écrit UNE ligne JSON
//! finale sur stdout ; mode sonde : joue UNE carte depuis l'état fixe d'audit.
//!
//! Usage : simulate --games N --seed S [--cards chemin/cards.json]
//!                  [--effects on|off] [--probe "<nom exact de carte>"]
//!                  [--boites base[,promo][,decouverte]] [--dump-deck]
//!
//! (boites-1) `--boites` choisit les boîtes physiques dont les cartes composent
//! les pioches ; son défaut est `base` (I3). `--dump-deck` recense la pioche
//! ainsi composée, un objet JSON par ligne et par carte retenue.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::policy::RandomPolicy;
use engine::probe::{
    run_probe_action_corp, run_probe_seq_corp, ProbeCorp, ProbeDelta, ProbeOptions, ProbeRes,
    ProbeScript,
};
use engine::sim::run_simulation;

/// Sérialise un `ProbeDelta` en objet JSON (schéma commun aux deux sondes).
fn delta_json(d: &ProbeDelta) -> serde_json::Value {
    serde_json::json!({
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

/// Sérialise le champ `resources` (trié par nom de carte par la sonde).
fn resources_json(rs: &[ProbeRes]) -> serde_json::Value {
    serde_json::Value::Array(
        rs.iter()
            .map(|r| serde_json::json!({ "card": r.card, "kind": r.kind, "n": r.n }))
            .collect(),
    )
}

/// (corpo-1) Sérialise l'objet `corp` — émis UNIQUEMENT quand `--probe-corp`
/// est donné, pour que les sondes existantes gardent exactement leur sortie
/// d'aujourd'hui (journal D10).
fn corp_json(c: &ProbeCorp) -> serde_json::Value {
    serde_json::json!({
        "name": c.name,
        "found": c.found,
        "encoded": c.encoded,
        "starting_mc": c.starting_mc,
        "start_prod": {
            "mc": c.start_prod.0,
            "heat": c.start_prod.1,
            "plants": c.start_prod.2,
        },
    })
}

fn die(msg: &str) -> ! {
    eprintln!("simulate: {msg}");
    std::process::exit(2);
}

fn main() {
    let mut games: u64 = 1000;
    let mut seed: u64 = 0;
    let mut cards_path = String::from("data/cards.json");
    let mut effects_on = true;
    let mut probe: Option<String> = None;
    let mut probe_action: Option<String> = None;
    let mut probe_opts = ProbeOptions::default();
    let mut probe_script = ProbeScript::default();
    // (lot 4) `--probe-produce` : exécuter la vraie phase IV après la séquence.
    let mut probe_produce = false;
    let mut dump_turn_order = false;
    // (corpo-1) Corporation imposée à la sonde, et vidage de la pioche.
    let mut probe_corp: Option<String> = None;
    let mut dump_corporations = false;
    // (boites-1) Boîtes actives et recensement de la pioche composée.
    let mut boites = BoiteSet::default();
    let mut dump_deck = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let value = |i: usize| -> &str {
            args.get(i + 1)
                .unwrap_or_else(|| die(&format!("valeur manquante pour {}", args[i])))
        };
        match args[i].as_str() {
            "--games" => {
                games = value(i).parse().unwrap_or_else(|_| die("--games invalide"));
                i += 2;
            }
            "--seed" => {
                seed = value(i).parse().unwrap_or_else(|_| die("--seed invalide"));
                i += 2;
            }
            "--cards" => {
                cards_path = value(i).to_string();
                i += 2;
            }
            "--effects" => {
                effects_on = match value(i) {
                    "on" => true,
                    "off" => false,
                    other => die(&format!("--effects invalide: {other} (on|off)")),
                };
                i += 2;
            }
            "--probe" => {
                probe = Some(value(i).to_string());
                i += 2;
            }
            "--probe-action" => {
                probe_action = Some(value(i).to_string());
                i += 2;
            }
            // Sonde étendue (lot 3).
            // Accepte les deux formes : `--probe-strict "<séquence>"` et
            // `--probe-strict` combiné à `--probe "<séquence>"`.
            "--probe-strict" => {
                probe_opts.strict = true;
                match args.get(i + 1) {
                    Some(v) if !v.starts_with("--") => {
                        probe = Some(v.clone());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            // (lot 4) Exécute la VRAIE phase IV après la séquence.
            "--probe-produce" => {
                probe_produce = true;
                i += 1;
            }
            "--probe-mc" => {
                probe_opts.mc = value(i).parse().unwrap_or_else(|_| die("--probe-mc invalide"));
                i += 2;
            }
            "--probe-filler" => {
                probe_opts.filler = value(i)
                    .parse()
                    .unwrap_or_else(|_| die("--probe-filler invalide"));
                i += 2;
            }
            // Sonde étendue (lot 3, ressources) : réponses imposées à la
            // POLITIQUE — mêmes points de décision que `simulate`.
            "--probe-choice" => {
                probe_script.choices = value(i)
                    .split(',')
                    .map(|x| x.trim())
                    .filter(|x| !x.is_empty())
                    .map(|x| x.parse().unwrap_or_else(|_| die("--probe-choice invalide")))
                    .collect();
                i += 2;
            }
            "--probe-target" => {
                probe_script.targets = value(i)
                    .split(';')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect();
                i += 2;
            }
            // (corpo-1) Corporation imposée au joueur sondé, à la place du tirage.
            "--probe-corp" => {
                probe_corp = Some(value(i).to_string());
                i += 2;
            }
            // (corpo-1) Une ligne JSON par corporation de la pioche d'une partie.
            "--dump-corporations" => {
                dump_corporations = true;
                i += 1;
            }
            "--boites" => {
                boites = BoiteSet::parse(value(i)).unwrap_or_else(|e| die(&e));
                i += 2;
            }
            "--dump-deck" => {
                dump_deck = true;
                i += 1;
            }
            "--dump-turn-order" => {
                dump_turn_order = true;
                i += 1;
            }
            other => die(&format!("argument inconnu: {other}")),
        }
    }

    let mut db = CardsDb::load_boites(&cards_path, boites).unwrap_or_else(|e| die(&e));
    db.effects_on = effects_on;

    // (boites-1) Ce que la composition a eu à signaler sans le corriger — sur
    // stderr, pour que stdout reste strictement déterministe.
    for a in &db.avertissements {
        eprintln!("boites: {a}");
    }

    // (boites-1) Recensement de la pioche RÉELLE de la configuration courante :
    // `db.recensement()` lit les mêmes champs que `setup_game` distribue, il
    // n'y a pas de seconde composition. Un objet JSON par ligne.
    if dump_deck {
        for c in db.recensement() {
            let line = serde_json::json!({
                "name": c.name,
                "kind": c.kind.as_str(),
                "boite": c.boite.as_str(),
                "planche": c.planche,
                "effets_geres": c.effets_geres,
            });
            println!("{line}");
        }
        return;
    }

    // (corpo-1) La pioche de corporations RÉELLE d'une partie, dans l'ordre de
    // chargement : `db.corporations` est exactement ce que `setup_game` distribue.
    if dump_corporations {
        for c in &db.corporations {
            let line = serde_json::json!({
                "name": c.name,
                "starting_mc": c.starting_mc,
                "tags": c.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
                "encoded": c.effect.is_some(),
            });
            println!("{line}");
        }
        return;
    }

    // (corpo-1) `--probe-corp` employé SANS `--probe` : la sonde se déroule sur
    // une séquence de cartes VIDE (mise en place de la corporation, puis
    // `--probe-produce` s'il est demandé). On passe une tranche vide, jamais un
    // nom vide : `--probe ""` doit continuer de rendre `found:false`.
    let corp_only = probe.is_none() && probe_corp.is_some() && probe_action.is_none();

    if probe.is_some() || corp_only {
        // Séquence : cartes séparées par « ; » (rétro-compatible : 1 carte).
        let name = probe.unwrap_or_default();
        let names: Vec<&str> = if corp_only {
            Vec::new()
        } else {
            name.split(';').map(|s| s.trim()).collect()
        };
        let r = run_probe_seq_corp(
            &db,
            &names,
            probe_opts,
            &probe_script,
            probe_produce,
            probe_corp.as_deref(),
        );
        let mut line = serde_json::json!({
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
            // (lot 4) Production dérivée réellement créditée par la phase IV.
            "produced": r.produced,
            "derived_prod": {
                "mc": r.derived_prod.0,
                "heat": r.derived_prod.1,
                "plants": r.derived_prod.2,
            },
            "vp_total": r.vp_total,
        });
        if let Some(c) = &r.corp {
            line["corp"] = corp_json(c);
        }
        println!("{line}");
        return;
    }

    if let Some(name) = probe_action {
        let r = run_probe_action_corp(&db, &name, &probe_script, probe_corp.as_deref());
        let mut line = serde_json::json!({
            "card": r.card,
            "found": r.found,
            "in_lot": r.in_lot,
            "has_action": r.has_action,
            "action_applied": r.action_applied,
            "delta": delta_json(&r.delta),
            "resources": resources_json(&r.resources),
            "target_error": r.target_error,
        });
        if let Some(c) = &r.corp {
            line["corp"] = corp_json(c);
        }
        println!("{line}");
        return;
    }

    let mut policy = RandomPolicy;
    let s = run_simulation(&db, games, seed, &mut policy);

    // games_per_sec est informatif mais non déterministe : il irait à
    // l'encontre du critère « même graine → sortie strictement identique »
    // s'il figurait dans la ligne JSON. Il part donc sur stderr (D17).
    eprintln!("games_per_sec: {:.1}", s.games_per_sec);

    // (C4) Ordre du tour RÉELLEMENT emprunté par la boucle de jeu, une ligne
    // par partie, avant la ligne JSON finale (qui reste la dernière ligne).
    if dump_turn_order {
        for order in &s.turn_orders {
            let seq: Vec<String> = order.iter().map(|p| p.to_string()).collect();
            println!("turn_order:{}", seq.join(","));
        }
    }

    // Une seule ligne JSON finale, déterministe (format du prompt).
    let line = serde_json::json!({
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
        // (lot 3) ressources posées sur les cartes.
        "res_added": s.res_added,
        "res_removed": s.res_removed,
        "res_targets_missing": s.res_targets_missing,
        "phase_upgrades_skipped": s.phase_upgrades_skipped,
        // (boites-1) I4 : cartes à effet non géré réellement JOUÉES.
        "cards_effects_unhandled": s.cards_effects_unhandled,
        "vp_from_resources": s.vp_from_resources,
        // (lot 4) productions dérivées, NT par badge, bonus de recherche.
        "derived_mc": s.derived_mc,
        "derived_heat": s.derived_heat,
        "derived_plants": s.derived_plants,
        "tr_from_tags": s.tr_from_tags,
        "research_extra_draws": s.research_extra_draws,
        // (lot corporations) effets de corporation observés en partie réelle.
        "corp_heat_as_mc": s.corp_heat_as_mc,
        "corp_forest_rebates": s.corp_forest_rebates,
        "corp_tr_boosts": s.corp_tr_boosts,
        "corp_trigger_tr": s.corp_trigger_tr,
    });
    println!("{line}");
}

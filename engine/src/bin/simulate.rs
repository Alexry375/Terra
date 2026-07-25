//! Binaire `simulate` : joue N parties aléatoires et écrit UNE ligne JSON
//! finale sur stdout ; mode sonde : joue UNE carte depuis l'état fixe d'audit.
//!
//! Usage : simulate --games N --seed S [--cards chemin/cards.json]
//!                  [--effects on|off] [--probe "<nom exact de carte>"]

use engine::cards::CardsDb;
use engine::policy::RandomPolicy;
use engine::probe::{run_probe_action, run_probe_seq_opts, ProbeDelta, ProbeOptions};
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
    let mut dump_turn_order = false;

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
            "--dump-turn-order" => {
                dump_turn_order = true;
                i += 1;
            }
            other => die(&format!("argument inconnu: {other}")),
        }
    }

    let mut db = CardsDb::load(&cards_path).unwrap_or_else(|e| die(&e));
    db.effects_on = effects_on;

    if let Some(name) = probe {
        // Séquence : cartes séparées par « ; » (rétro-compatible : 1 carte).
        let names: Vec<&str> = name.split(';').map(|s| s.trim()).collect();
        let r = run_probe_seq_opts(&db, &names, probe_opts);
        let line = serde_json::json!({
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
        });
        println!("{line}");
        return;
    }

    if let Some(name) = probe_action {
        let r = run_probe_action(&db, &name);
        let line = serde_json::json!({
            "card": r.card,
            "found": r.found,
            "in_lot": r.in_lot,
            "has_action": r.has_action,
            "action_applied": r.action_applied,
            "delta": delta_json(&r.delta),
        });
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
    });
    println!("{line}");
}

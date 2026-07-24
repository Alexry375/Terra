//! Binaire `simulate` : joue N parties aléatoires et écrit UNE ligne JSON
//! finale sur stdout ; mode sonde : joue UNE carte depuis l'état fixe d'audit.
//!
//! Usage : simulate --games N --seed S [--cards chemin/cards.json]
//!                  [--effects on|off] [--probe "<nom exact de carte>"]

use engine::cards::CardsDb;
use engine::policy::RandomPolicy;
use engine::probe::run_probe;
use engine::sim::run_simulation;

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
            other => die(&format!("argument inconnu: {other}")),
        }
    }

    let mut db = CardsDb::load(&cards_path).unwrap_or_else(|e| die(&e));
    db.effects_on = effects_on;

    if let Some(name) = probe {
        let r = run_probe(&db, &name);
        let line = serde_json::json!({
            "card": r.card,
            "found": r.found,
            "in_lot": r.in_lot,
            "prereq_ok": r.prereq_ok,
            "played": r.played,
            "delta": {
                "mc": r.delta.mc,
                "heat": r.delta.heat,
                "plants": r.delta.plants,
                "hand": r.delta.hand,
                "mc_prod": r.delta.mc_prod,
                "heat_prod": r.delta.heat_prod,
                "plant_prod": r.delta.plant_prod,
                "card_prod": r.delta.card_prod,
                "tr": r.delta.tr,
                "temperature": r.delta.temperature,
                "oxygen": r.delta.oxygen,
                "oceans": r.delta.oceans,
                "forests": r.delta.forests,
            },
            "vp": r.vp,
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
    });
    println!("{line}");
}

//! Binaire `simulate` : joue N parties aléatoires et écrit UNE ligne JSON
//! finale sur stdout.
//!
//! Usage : simulate --games N --seed S [--cards chemin/cards.json]

use engine::cards::CardsDb;
use engine::policy::RandomPolicy;
use engine::sim::run_simulation;

fn die(msg: &str) -> ! {
    eprintln!("simulate: {msg}");
    std::process::exit(2);
}

fn main() {
    let mut games: u64 = 1000;
    let mut seed: u64 = 0;
    let mut cards_path = String::from("inputs/cards.json");

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
            other => die(&format!("argument inconnu: {other}")),
        }
    }

    let db = CardsDb::load(&cards_path).unwrap_or_else(|e| die(&e));

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

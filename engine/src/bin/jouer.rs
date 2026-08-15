//! **(le-juge-apprend) Le joueur Rust joue une partie entière, et dit ce qu'il a
//! répondu.**
//!
//!     jouer --graine G --poids data/poids/apprenti.txt [--boites …] [--sieges 0,1]
//!         → {"decisions": [ … ], "scores": [a, b], "generations": n}
//!
//! Il sert au banc du §4 : « le joueur Rust et le joueur JavaScript doivent
//! choisir la même option dans la même situation ». Le banc
//! `web/webapp/verif/juge-meme-option.mjs` fait jouer la même graine au joueur
//! JavaScript et compare les deux listes de réponses, décision par décision. Si
//! les deux listes sont égales, les deux joueurs ont choisi la même option à
//! chacune des décisions de la partie — c'est le critère le plus fort possible,
//! et il ne demande aucune situation fabriquée.
//!
//! Ni exploration ni apprentissage : on mesure une force, et un joueur qui
//! explore pendant sa propre notation se sabote (§5).

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score_parts, setup_game};
use engine::sim::MAX_GENERATIONS;
use serde_json::json;

#[path = "../description.rs"]
mod description;
#[path = "../joueur.rs"]
mod joueur;
#[path = "../rejeu.rs"]
mod rejeu;
#[path = "../reseau.rs"]
mod reseau;

use description::Description;
use joueur::Joueur;
use reseau::{Pile, Reseau};

fn mourir(msg: &str) -> ! {
    eprintln!("jouer: {msg}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut graine: u64 = 1;
    let mut poids = String::from("data/poids/apprenti.txt");
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut tracer: i64 = -1;
    let mut i = 1;
    while i < args.len() {
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--graine" => graine = val(i).parse().unwrap_or_else(|_| mourir("--graine")),
            "--poids" => poids = val(i),
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            "--tracer-rang" => tracer = val(i).parse().unwrap_or(-1),
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += 2;
    }
    let boites = match BoiteSet::parse(&boites_txt) {
        Ok(b) => b,
        Err(e) => mourir(&e),
    };
    let chemin = if std::path::Path::new(&cartes).exists() {
        cartes.clone()
    } else {
        format!("../{cartes}")
    };
    let db = match CardsDb::load_boites(&chemin, boites) {
        Ok(db) => db,
        Err(e) => mourir(&e),
    };
    let desc = Description::new(&db);
    let noms = desc.noms_avec(&db);
    let mut reseau = match Reseau::lire(&poids, &noms) {
        Ok(r) => r,
        Err(e) => mourir(&e),
    };
    let mut pile = Pile::new(desc.taille);
    let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, graine);
    j.exploration = 0.0;
    j.tracer_rang = tracer;
    j.apprendre = false;
    j.nouvelle_partie(graine);

    let mut game = setup_game(&db, graine, &mut j);
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        j.debut_manche(&game);
        play_round(&mut game, &db, &mut j);
    }
    let (scores, _, _) = score_parts(&game, &db);
    // **L'écart d'évaluation entre les options d'une même décision** (§2.2) : au
    // round 1 il valait 0,016, le niveau du bruit. C'est la mesure qui dit si le
    // réseau départage vraiment, et non s'il prédit bien.
    let ecart = if j.compte_ecart > 0 {
        j.somme_ecart / j.compte_ecart as f64
    } else {
        0.0
    };
    eprintln!(
        "écart moyen entre la meilleure et la pire option : {ecart:.4} ({} décisions à plusieurs options)",
        j.compte_ecart
    );
    eprintln!(
        "avance vers le repère (§4.1) : {} pas, plafond atteint {} fois",
        j.pas_avance, j.plafonds
    );
    println!(
        "{}",
        json!({
            "decisions": j.journal,
            "scores": [scores[0], scores[1]],
            "generations": game.generation,
            "partie_complete": game.game_over,
            "ecart_options": ecart,
            "decisions_multiples": j.compte_ecart,
        })
    );
}

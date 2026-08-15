//! **(le-juge-apprend) Où passe le temps d'un essai d'option.**
//!
//! Le §9 pose un garde-fou : « si un entraînement de 10 000 parties dépasse un
//! quart d'heure, l'entraînement final en dépassera cinq ». Quand il est dépassé,
//! il faut savoir CE QUI coûte, et non deviner. Ce binaire chronomètre les quatre
//! gestes d'un essai, séparément.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, setup_game, score_breakdown, main_payable};
use engine::policy::RandomPolicy;
use engine::state::GameState;
use std::time::Instant;

#[path = "../description.rs"]
mod description;
#[path = "../rejeu.rs"]
mod rejeu;
#[path = "../reseau.rs"]
mod reseau;

use description::{Description, Tampons};
use reseau::Reseau;

fn main() {
    let db = CardsDb::load_boites("data/cards.json", BoiteSet::parse("base,decouverte").unwrap())
        .expect("cartes");
    let desc = Description::new(&db);
    let mut reseau = Reseau::neuf(desc.taille);
    let mut t = Tampons::new(&desc);
    let mut x: Vec<f64> = Vec::new();

    // Un état de milieu de partie, pris sur une vraie partie.
    let mut pol = RandomPolicy;
    let mut game: GameState = setup_game(&db, 100000, &mut pol);
    for _ in 0..20 {
        if game.game_over {
            break;
        }
        play_round(&mut game, &db, &mut pol);
    }
    let n = 2000;

    let t0 = Instant::now();
    for _ in 0..n {
        let g2 = game.clone();
        std::hint::black_box(&g2);
    }
    println!("clone d'état        : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    let t0 = Instant::now();
    for _ in 0..n {
        let s = score_breakdown(&game, &db);
        std::hint::black_box(&s);
    }
    println!("score_breakdown     : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    let t0 = Instant::now();
    for _ in 0..n {
        let s = main_payable(&game, &db, 0);
        std::hint::black_box(&s);
    }
    println!("main_payable        : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    let t0 = Instant::now();
    for _ in 0..n {
        desc.decrire(&game, &db, 0, &mut x, &mut t);
    }
    println!("decrire (complet)   : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    reseau.oublier();
    let t0 = Instant::now();
    for _ in 0..n {
        reseau.evaluer(&x);
    }
    println!("evaluer (differentiel) : {:5.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    reseau.sans_optimisation = true;
    let t0 = Instant::now();
    for _ in 0..n {
        reseau.evaluer(&x);
    }
    println!("evaluer (complet)   : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);
    reseau.sans_optimisation = false;

    // Une manche rejouée depuis un clone, comme le fait un essai.
    let t0 = Instant::now();
    for _ in 0..n {
        let mut g2 = game.clone();
        let mut r = rejeu::Rejeu::new(Vec::new());
        while r.attente.is_none() && !g2.game_over && g2.generation <= engine::sim::MAX_GENERATIONS {
            play_round(&mut g2, &db, &mut r);
        }
        std::hint::black_box(&g2);
    }
    println!("rejeu d'une manche  : {:8.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);

    let t0 = Instant::now();
    for _ in 0..n {
        reseau.oublier();
        reseau.evaluer(&x);
        reseau.accumuler(&x, [0.6, 0.4], 1.0, 0.0001);
        reseau.appliquer();
    }
    println!("une passe d'apprentissage : {:3.2} µs", 1e6 * t0.elapsed().as_secs_f64() / n as f64);
}

//! **(le-juge-apprend) LE RÉSEAU VAUT-IL MIEUX QUE TROIS CHAMPS DE L'ÉTAT ?**
//!
//!     predire --parties N --graine-debut G --poids chemin [--boites …]
//!
//! Le §3.0 du contrat pose le repère de tout ce chantier : **120 parties entre
//! deux `reflechi`, arrêtées à la moitié de leurs générations, et une règle
//! triviale appliquée à trois champs** désignent le vainqueur 82,5 fois sur 100 —
//! « un réseau qui en voit mille doit faire mieux ». Ce banc pose la question
//! au réseau livré, sur les MÊMES parties et au MÊME instant que la règle
//! triviale, ce qui est la seule façon honnête de les comparer :
//!
//! - on joue N parties de l'apprenti contre lui-même, sans exploration ;
//! - à la moitié exacte des générations de chaque partie, on relève d'un côté la
//!   probabilité que le réseau accorde au siège 0, de l'autre les trois règles
//!   triviales du §3.0 (score seul ; score + 2 × production ; score + 2 ×
//!   production + cartes posées) ;
//! - on compare chacun au vainqueur réel, parties nulles écartées.
//!
//! Ce n'est pas une mesure de force — c'est une mesure de la QUALITÉ DU JUGEMENT,
//! et elle dit lequel des deux étages du joueur est en cause quand la force
//! plafonne.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score_parts, setup_game};
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;

#[path = "../description.rs"]
mod description;
#[path = "../joueur.rs"]
mod joueur;
#[path = "../rejeu.rs"]
mod rejeu;
#[path = "../reseau.rs"]
mod reseau;

use description::{Description, Tampons};
use joueur::Joueur;
use reseau::{Pile, Reseau};

fn mourir(msg: &str) -> ! {
    eprintln!("predire: {msg}");
    std::process::exit(2);
}

/// Les trois champs du §3.0, pour un siège : score acquis, production totale,
/// cartes posées.
fn champs(g: &GameState, db: &CardsDb, siege: usize) -> (f64, f64, f64) {
    let (scores, _, _) = score_parts(g, db);
    let p = &g.players[siege];
    let prod = (p.mc_prod + p.heat_prod + p.plant_prod + p.card_prod) as f64;
    (scores[siege] as f64, prod, p.played.len() as f64)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parties: u64 = 120;
    let mut graine_debut: u64 = 100_000;
    let mut poids = String::from("data/poids/apprenti.txt");
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut i = 1;
    while i < args.len() {
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--parties" => parties = val(i).parse().unwrap_or_else(|_| mourir("--parties")),
            "--graine-debut" => {
                graine_debut = val(i).parse().unwrap_or_else(|_| mourir("--graine-debut"))
            }
            "--poids" => poids = val(i),
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += 2;
    }
    if graine_debut < 100_000 {
        mourir("mesure confinée aux graines 100000 et au-delà");
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
    let mut tampons = Tampons::new(&desc);
    let mut vecteur: Vec<f64> = Vec::new();

    let mut decisives = 0u64;
    let (mut j_reseau, mut j_score, mut j_sp, mut j_spc) = (0u64, 0u64, 0u64, 0u64);
    let mut somme_p = 0.0f64;
    let mut somme_ecart_p = 0.0f64;

    for n in 0..parties {
        let seed = graine_debut + n;
        // --- première passe : combien de générations dure cette partie ?
        let total = {
            let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, seed);
            j.exploration = 0.0;
            j.apprendre = false;
            j.nouvelle_partie(seed);
            let mut g = setup_game(&db, seed, &mut j);
            while !g.game_over && g.generation <= MAX_GENERATIONS {
                j.debut_manche(&g);
                play_round(&mut g, &db, &mut j);
            }
            g.generation
        };
        let moitie = (total / 2).max(1);

        // --- seconde passe : on s'arrête à la moitié, on relève, on finit.
        let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, seed);
        j.exploration = 0.0;
        j.apprendre = false;
        j.nouvelle_partie(seed);
        let mut g = setup_game(&db, seed, &mut j);
        let mut releve: Option<(f64, f64, f64, f64)> = None;
        while !g.game_over && g.generation <= MAX_GENERATIONS {
            if releve.is_none() && g.generation >= moitie {
                let (s0, p0, c0) = champs(&g, &db, 0);
                let (s1, p1, c1) = champs(&g, &db, 1);
                j.desc
                    .decrire(&g, &db, 0, &mut vecteur, &mut tampons);
                j.reseau.oublier();
                let p = j.reseau.evaluer(&vecteur);
                releve = Some((p[0], s0 - s1, (s0 + 2.0 * p0) - (s1 + 2.0 * p1), (s0 + 2.0 * p0 + c0) - (s1 + 2.0 * p1 + c1)));
            }
            j.debut_manche(&g);
            play_round(&mut g, &db, &mut j);
        }
        let (scores, _, _) = score_parts(&g, &db);
        let Some((p0, d_score, d_sp, d_spc)) = releve else {
            continue;
        };
        if scores[0] == scores[1] {
            continue;
        }
        decisives += 1;
        let gagne0 = scores[0] > scores[1];
        somme_p += p0;
        somme_ecart_p += (p0 - 0.5).abs();
        if (p0 > 0.5) == gagne0 {
            j_reseau += 1;
        }
        if (d_score > 0.0) == gagne0 {
            j_score += 1;
        }
        if (d_sp > 0.0) == gagne0 {
            j_sp += 1;
        }
        if (d_spc > 0.0) == gagne0 {
            j_spc += 1;
        }
    }

    if decisives == 0 {
        mourir("aucune partie décisive");
    }
    let pc = |x: u64| 100.0 * x as f64 / decisives as f64;
    println!("poids : {poids}");
    println!("{decisives} parties décisives sur {parties}, vainqueur désigné à mi-partie :");
    println!("  le RÉSEAU (1472 entrées)                    : {:.1} %", pc(j_reseau));
    println!("  le score acquis seul                        : {:.1} %", pc(j_score));
    println!("  score + 2 × production                      : {:.1} %", pc(j_sp));
    println!("  score + 2 × production + cartes posées      : {:.1} %", pc(j_spc));
    println!(
        "  amplitude du réseau : sortie moyenne {:.3}, écart moyen à 0,5 : {:.4}",
        somme_p / decisives as f64,
        somme_ecart_p / decisives as f64
    );
}

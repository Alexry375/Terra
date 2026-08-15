//! **(le-juge-apprend) Point d'accroche n°3 — l'entraînement.**
//!
//!     entraine --parties N --graine-debut G --sortie chemin
//!              [--exploration x] [--boites …] [--instantanes "10000,50000"]
//!              [--sans-optimisation]
//!
//! Il joue N parties du réseau **contre lui-même** (les deux sièges partagent les
//! mêmes poids et apprennent de la même partie), écrit le fichier de poids du §7,
//! et imprime une ligne JSON par tranche :
//!
//!     {"parties": n, "erreur": x, "justes": y}
//!
//! `erreur` est l'écart moyen de prédiction (racine de la moyenne des carrés des
//! erreurs accumulées sur la tranche) ; `justes` la proportion de parties dont le
//! vainqueur était bien celui que le réseau donnait gagnant **à mi-partie** —
//! exactement le protocole du §3.0, où trois champs bien choisis désignent le
//! vainqueur 82,5 fois sur 100.
//!
//! **Tout est semé.** Deux entraînements lancés avec les mêmes arguments
//! produisent le même fichier, octet pour octet : les poids de départ viennent
//! d'un générateur de graine fixe, l'amorçage aussi, et chaque partie tire sa
//! graine de `--graine-debut + rang`. Aucune horloge n'entre dans le calcul.
//!
//! **Les graines restent au-dessus de 100000** : le binaire refuse de descendre
//! plus bas, parce que la balance du dépôt joue les graines 1 à N et qu'un
//! apprentissage qui les aurait vues serait une récitation.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score_parts, setup_game};
use engine::sim::MAX_GENERATIONS;
use std::time::Instant;

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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use reseau::{Pile, Reseau, AMORCAGE_FACTEUR, AMORCAGE_PARTIES, AMORCAGE_SCORE_MAX, TAUX};

fn mourir(msg: &str) -> ! {
    eprintln!("entraine: {msg}");
    std::process::exit(2);
}

/// **L'amorçage du §2.7** : cinq mille fins de partie FABRIQUÉES — on ne joue
/// pas, on part d'un état vide marqué « partie finie » et on donne à chaque
/// joueur un score tiré au hasard entre 0 et 49 — entraînées vers la cible du
/// §2.3, taux multiplié par dix.
///
/// Un réseau tiré au hasard ne sait même pas que « plus de points, c'est
/// mieux » ; sans cet amorçage il met très longtemps à le découvrir seul.
fn amorcer(reseau: &mut Reseau, noms: &[String], graine: u64) {
    let n = noms.len();
    let rang_fin = noms
        .iter()
        .position(|x| x == "global_fin_de_partie")
        .unwrap_or_else(|| mourir("entrée « global_fin_de_partie » introuvable"));
    let debut_moi = noms
        .iter()
        .position(|x| x.starts_with("moi_score_acquis>"))
        .unwrap_or_else(|| mourir("entrées « moi_score_acquis> » introuvables"));
    let debut_adv = noms
        .iter()
        .position(|x| x.starts_with("adv_score_acquis>"))
        .unwrap_or_else(|| mourir("entrées « adv_score_acquis> » introuvables"));
    let seuils: Vec<i64> = noms
        .iter()
        .filter(|x| x.starts_with("moi_score_acquis>"))
        .map(|x| x["moi_score_acquis>".len()..].parse::<i64>().unwrap())
        .collect();

    let mut rng = StdRng::seed_from_u64(graine);
    let mut x = vec![-1.0f64; n];
    for _ in 0..AMORCAGE_PARTIES {
        let s_moi = rng.gen_range(0..=AMORCAGE_SCORE_MAX);
        let s_adv = rng.gen_range(0..=AMORCAGE_SCORE_MAX);
        x.fill(-1.0);
        x[rang_fin] = 1.0;
        for (k, s) in seuils.iter().enumerate() {
            x[debut_moi + k] = if s_moi > *s { 1.0 } else { -1.0 };
            x[debut_adv + k] = if s_adv > *s { 1.0 } else { -1.0 };
        }
        let cible = Reseau::cible_finale(s_moi, s_adv);
        reseau.entrainer_une(&x, cible, TAUX * AMORCAGE_FACTEUR);
    }
    reseau.raz_stats();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parties: u64 = 10_000;
    let mut graine_debut: u64 = 100_000;
    let mut sortie = String::new();
    let mut exploration: f64 = 0.05;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut instantanes: Vec<u64> = Vec::new();
    let mut sans_optimisation = false;
    let mut lambda = reseau::LAMBDA;
    let mut i = 1;
    while i < args.len() {
        let mut avance = 2;
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
            "--sortie" => sortie = val(i),
            "--exploration" => {
                exploration = val(i).parse().unwrap_or_else(|_| mourir("--exploration"))
            }
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            "--instantanes" => {
                instantanes = val(i)
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().parse().unwrap_or_else(|_| mourir("--instantanes")))
                    .collect()
            }
            "--lambda" => lambda = val(i).parse().unwrap_or_else(|_| mourir("--lambda")),
            "--sans-optimisation" => {
                sans_optimisation = true;
                avance = 1;
            }
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += avance;
    }
    if sortie.is_empty() {
        mourir("--sortie est obligatoire");
    }
    if graine_debut < 100_000 {
        mourir("l'entraînement est confiné aux graines 100000 et au-delà (prompt § « Où tu as le droit de régler »)");
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
    let mut reseau = Reseau::neuf(desc.taille);
    reseau.sans_optimisation = sans_optimisation;
    reseau.lambda = lambda;
    let mut pile = Pile::new(desc.taille);

    let t0 = Instant::now();
    amorcer(&mut reseau, &noms, reseau::GRAINE_POIDS);
    eprintln!("amorçage : {AMORCAGE_PARTIES} fins de partie fabriquées en {:.1} s", t0.elapsed().as_secs_f64());

    let tranche = (parties / 20).max(1);
    let mut justes_tranche = 0u64;
    let mut decisives_tranche = 0u64;
    let mut instantanes_faits: Vec<u64> = Vec::new();

    let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, graine_debut);
    j.exploration = exploration;
    j.apprendre = true;

    for g in 0..parties {
        let seed = graine_debut + g;
        j.nouvelle_partie(seed);
        let mut game = setup_game(&db, seed, &mut j);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            j.debut_manche(&game);
            play_round(&mut game, &db, &mut j);
        }
        let (scores, _, _) = score_parts(&game, &db);

        // ---- la correction de fin de partie (§2.3), pour les deux joueurs
        let mut fin = Vec::new();
        let mut tampons = description::Tampons::new(&desc);
        for p in 0..2 {
            desc.decrire(&game, &db, p, &mut fin, &mut tampons);
            j.pile.empiler(&fin, p);
        }
        for p in 0..2 {
            let cible = Reseau::cible_finale(scores[p], scores[1 - p]);
            j.reseau.corriger(j.pile, p, cible, j.taux);
        }

        // ---- le vainqueur était-il celui qu'on donnait gagnant à mi-partie ?
        if scores[0] != scores[1] && !j.predictions.is_empty() {
            let milieu = j.predictions.len() / 2;
            let predit_0 = j.predictions[milieu] > 0.5;
            decisives_tranche += 1;
            if predit_0 == (scores[0] > scores[1]) {
                justes_tranche += 1;
            }
        }
        j.reseau.parties = g + 1;

        if (g + 1) % tranche == 0 || g + 1 == parties {
            let justes = if decisives_tranche > 0 {
                justes_tranche as f64 / decisives_tranche as f64
            } else {
                0.0
            };
            println!(
                "{{\"parties\": {}, \"erreur\": {:.6}, \"justes\": {:.4}}}",
                g + 1,
                j.reseau.erreur_moyenne(),
                justes
            );
            use std::io::Write;
            let _ = std::io::stdout().flush();
            j.reseau.raz_stats();
            justes_tranche = 0;
            decisives_tranche = 0;
        }

        // ---- les instantanés de la courbe de force (§6)
        if instantanes.contains(&(g + 1)) && !instantanes_faits.contains(&(g + 1)) {
            instantanes_faits.push(g + 1);
            let chemin = format!("{sortie}.{}", g + 1);
            if let Err(e) = j.reseau.ecrire(&chemin, &noms) {
                eprintln!("entraine: instantané {chemin} non écrit : {e}");
            } else {
                eprintln!("instantané : {chemin} ({} parties, {:.1} s)", g + 1, t0.elapsed().as_secs_f64());
            }
        }
    }

    let essais = j.essais;
    let t_essais = j.t_essais;
    let t_apprentissage = j.t_apprentissage;
    let passes = j.passes;
    if let Err(e) = reseau.ecrire(&sortie, &noms) {
        mourir(&format!("écriture de {sortie} : {e}"));
    }
    eprintln!(
        "fini : {parties} parties, {essais} essais d'option, {:.1} s ({:.1} ms par partie)",
        t0.elapsed().as_secs_f64(),
        1000.0 * t0.elapsed().as_secs_f64() / parties as f64
    );
    eprintln!(
        "  dont essais {:.1} s, apprentissage {:.1} s ({} passes)",
        t_essais, t_apprentissage, passes
    );
}

//! **(le-juge-apprend) La distribution réelle des quantités du jeu.**
//!
//! Le prompt §3.5 refuse des seuils de thermomètre choisis au doigt mouillé :
//! « joue mille parties au hasard, relève la distribution réelle de chaque
//! quantité, et place tes seuils de sorte qu'aucun ne soit atteint dans moins de
//! deux pour cent des situations ni dans plus de quatre-vingt-dix-huit pour
//! cent ». Ce binaire fait exactement cela, et rien d'autre : il ne décide rien,
//! il MESURE, et il propose les seuils que la mesure autorise.
//!
//!     mesures --parties 1000 --graine-debut 100000 --boites base,decouverte
//!
//! Une observation = un point de décision du moteur, relevé pour LES DEUX
//! joueurs (la description du §3 est écrite deux fois, « moi » puis « l'autre » :
//! les deux sièges alimentent donc la même distribution).
//!
//! Les graines restent au-dessus de 100000 : le prompt interdit de régler quoi
//! que ce soit sur les graines de mesure, et un seuil de thermomètre est un
//! réglage.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, JOKER_TAG_CHOICES};
use engine::flow::{play_round, setup_game};
use engine::policy::RandomPolicy;
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;
use std::collections::BTreeMap;

#[path = "../espion.rs"]
mod espion;
use espion::Espion;

/// Les quantités relevées, dans l'ordre où elles seront imprimées. Ce sont
/// exactement celles que le §3.3 met en thermomètre.
fn quantites(game: &GameState, db: &CardsDb, p: usize, out: &mut Vec<(&'static str, i64)>) {
    let pl = &game.players[p];
    out.clear();
    out.push(("generation", game.generation as i64));
    out.push(("temperature", game.temperature as i64));
    out.push(("oxygene", game.oxygen as i64));
    out.push(("oceans", game.oceans_revealed as i64));
    out.push(("pioche", game.deck.len() as i64));
    out.push(("defausse", game.discard.len() as i64));
    out.push(("mc", pl.mc));
    out.push(("chaleur", pl.heat));
    out.push(("plantes", pl.plants));
    out.push(("prod_mc", pl.mc_prod));
    out.push(("prod_chaleur", pl.heat_prod));
    out.push(("prod_plantes", pl.plant_prod));
    out.push(("prod_cartes", pl.card_prod));
    out.push(("nt", pl.tr));
    out.push(("forets", pl.forests));
    out.push(("score_acquis", {
        let (parts, _, _) = engine::flow::score_breakdown(game, db);
        parts[p].acquis()
    }));
    out.push(("main", pl.hand.len() as i64));
    out.push(("posees", pl.played.len() as i64));
    out.push(("acier", pl.steel_capacity));
    out.push(("titane", pl.titanium_capacity));
    out.push((
        "reperes_atteints",
        game.milestones.iter().filter(|m| m.achieved_by[p]).count() as i64,
    ));
    let payable = engine::flow::main_payable(game, db, p);
    out.push(("main_payable", payable.iter().filter(|x| **x).count() as i64));
    for (nom, coul) in [("verte", "verte"), ("bleue", "bleue"), ("rouge", "rouge")] {
        let n = pl
            .hand
            .iter()
            .enumerate()
            .filter(|(i, id)| {
                payable.get(*i).copied().unwrap_or(false)
                    && db.projects[**id as usize].color.nom_fr() == coul
            })
            .count() as i64;
        out.push((
            match nom {
                "verte" => "payable_verte",
                "bleue" => "payable_bleue",
                _ => "payable_rouge",
            },
            n,
        ));
    }
    for (i, t) in JOKER_TAG_CHOICES.iter().enumerate() {
        out.push((
            match t.as_str() {
                "BUILDING" => "badge_BUILDING",
                "SPACE" => "badge_SPACE",
                "SCIENCE" => "badge_SCIENCE",
                "PLANT" => "badge_PLANT",
                "MICROBE" => "badge_MICROBE",
                "ANIMAL" => "badge_ANIMAL",
                "EARTH" => "badge_EARTH",
                "JUPITER" => "badge_JUPITER",
                "ENERGY" => "badge_ENERGY",
                "EVENT" => "badge_EVENT",
                _ => "badge_AUTRE",
            },
            pl.tag_counts[i] as i64,
        ));
    }
}

/// Les seuils qu'une distribution autorise : ceux dont la fraction « q > seuil »
/// tombe entre 2 % et 98 %, choisis pour être régulièrement espacés EN
/// PROBABILITÉ (et non en valeur) — c'est ce qui donne des entrées également
/// informatives.
fn seuils(hist: &BTreeMap<i64, u64>, total: u64, k: usize) -> Vec<i64> {
    let mut candidats: Vec<(i64, f64)> = Vec::new();
    let mut cumul = 0u64; // nombre de valeurs <= s
    for (&v, &n) in hist {
        cumul += n;
        let au_dessus = (total - cumul) as f64 / total as f64;
        if (0.02..=0.98).contains(&au_dessus) {
            candidats.push((v, au_dessus));
        }
    }
    if candidats.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<i64> = Vec::new();
    for i in 0..k {
        // Cibles décroissantes : 0,98 … 0,02.
        let cible = 0.98 - (0.96 * i as f64) / (k.max(2) - 1) as f64;
        let meilleur = candidats
            .iter()
            .min_by(|a, b| {
                (a.1 - cible)
                    .abs()
                    .partial_cmp(&(b.1 - cible).abs())
                    .unwrap()
            })
            .unwrap()
            .0;
        if !out.contains(&meilleur) {
            out.push(meilleur);
        }
    }
    out.sort_unstable();
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parties: u64 = 1000;
    let mut graine_debut: u64 = 100_000;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut k = 8usize;
    let mut i = 1;
    while i < args.len() {
        let val = || args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--parties" => parties = val().parse().expect("--parties"),
            "--graine-debut" => graine_debut = val().parse().expect("--graine-debut"),
            "--boites" => boites_txt = val(),
            "--cards" => cartes = val(),
            "--seuils" => k = val().parse().expect("--seuils"),
            autre => {
                eprintln!("mesures: argument inconnu {autre}");
                std::process::exit(2);
            }
        }
        i += 2;
    }
    assert!(
        graine_debut >= 100_000,
        "le prompt interdit toute mesure de réglage sous la graine 100000"
    );

    let boites = BoiteSet::parse(&boites_txt).expect("boites");
    let db = CardsDb::load_boites(&cartes, boites).expect("base de cartes");

    let mut hists: Vec<(String, BTreeMap<i64, u64>)> = Vec::new();
    let mut total = 0u64;

    for g in 0..parties {
        let seed = graine_debut + g;
        // Le relevé se fait dans la fermeture : elle ne peut pas infléchir la
        // partie, elle ne fait que lire.
        let mut lignes: Vec<(&'static str, i64)> = Vec::new();
        let mut lot: Vec<(String, i64)> = Vec::new();
        {
            let mut espion = Espion::new(RandomPolicy, |game: &GameState, _joueur: usize| {
                for p in 0..2 {
                    quantites(game, &db, p, &mut lignes);
                    for (nom, v) in lignes.iter() {
                        lot.push((nom.to_string(), *v));
                    }
                }
            });
            let mut game = setup_game(&db, seed, &mut espion);
            while !game.game_over && game.generation <= MAX_GENERATIONS {
                play_round(&mut game, &db, &mut espion);
            }
        }
        // Rangement après la partie : la fermeture ne peut pas emprunter `hists`
        // en même temps que `lot`.
        for (nom, v) in lot {
            if let Some((_, h)) = hists.iter_mut().find(|(n, _)| *n == nom) {
                *h.entry(v).or_insert(0) += 1;
            } else {
                let mut h = BTreeMap::new();
                h.insert(v, 1u64);
                hists.push((nom, h));
            }
        }
        if g % 100 == 0 {
            eprintln!("… {g} parties");
        }
    }
    if let Some((_, h)) = hists.first() {
        total = h.values().sum();
    }

    println!("# distribution relevée sur {parties} parties (graines {graine_debut}..{}), {total} observations", graine_debut + parties - 1);
    for (nom, h) in &hists {
        let n: u64 = h.values().sum();
        let min = *h.keys().next().unwrap_or(&0);
        let max = *h.keys().next_back().unwrap_or(&0);
        let moy: f64 = h.iter().map(|(v, c)| *v as f64 * *c as f64).sum::<f64>() / n as f64;
        let s = seuils(h, n, k);
        println!(
            "{nom}\tmin={min}\tmax={max}\tmoy={moy:.2}\tseuils={:?}",
            s
        );
    }
}

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
//!
//! # Deux ajouts du lot L3
//!
//! 1. **`--poids <fichier>`** — les parties sont alors jouées par le RÉSEAU et
//!    non au hasard. Le §3.5 l'exige pour les seuils de ce lot : « les grandeurs
//!    de fin de partie n'ont pas la même distribution ». Sans l'option, tout se
//!    passe exactement comme avant, politique de hasard comprise.
//!
//! 2. **`--fiche`** — un mode neuf, qui ne relève plus des quantités mais LA
//!    FICHE elle-même, situation par situation :
//!
//!    ```text
//!    mesures --fiche --parties N --graine-debut G \
//!        --poids data/poids/apprenti-L3-amorce.txt --boites base,decouverte
//!    ```
//!
//!    Il écrit un seul objet JSON sur la sortie standard. `cases_figees` compte
//!    les cases qui n'ont JAMAIS changé de valeur d'une situation à l'autre :
//!    c'est la mesure qui prouve le défaut 2.12 et qui protège contre l'ajout
//!    d'entrées mortes. `situations_indiscernables` compte les situations où
//!    l'écart réel de score atteint 8 points et où les cases de score des deux
//!    joueurs sont pourtant identiques : c'est la mesure qui prouve le défaut
//!    2.9.
//!
//! Une situation = un point de décision VU D'UN SIÈGE. Chaque décision en donne
//! donc deux, une par siège — la même convention que le relevé de quantités
//! ci-dessus, et la seule qui couvre les deux moitiés « moi » / « adversaire »
//! de la fiche.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, JOKER_TAG_CHOICES};
use engine::description::{Description, Tampons};
use engine::flow::{play_round, setup_game};
use engine::joueur::Joueur;
use engine::policy::RandomPolicy;
use engine::reseau::{Pile, Reseau};
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;
use std::collections::BTreeMap;

use engine::espion;
use espion::Espion;

/// **Le prix d'une main vide.** `prix_min` répond « la carte la moins chère de
/// ma main » ; une main vide n'en a aucune, et répondre 0 dirait « j'ai une
/// carte gratuite », le contraire de la vérité. On répond donc au-dessus de tous
/// les prix imprimés du jeu — « rien de bon marché ici ». La main n'est vide
/// qu'à la mise en place (une poignée de situations par partie).
pub const PRIX_MAIN_VIDE: i64 = 99;

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

    // ------------------------------------------------------------------
    // (L3) LES GRANDEURS QUE LE LOT AJOUTE À LA FICHE.
    //
    // Le §3.5 veut un seuil MESURÉ pour chaque thermomètre neuf ; il faut donc
    // que ce binaire relève les grandeurs neuves, sinon aucun de leurs seuils
    // ne peut être posé autrement qu'à la préférence. Elles sont lues par les
    // MÊMES fonctions que la fiche (`description::resume_main`,
    // `description::ecarts`, `description::ressources_posees`) : un seuil ne
    // peut donc pas être mesuré sur une grandeur qui n'est pas celle qu'on
    // publie.
    // ------------------------------------------------------------------
    let autre = (p + 1) % 2;
    let adv = &game.players[autre];
    let (parts, _, _) = engine::flow::score_breakdown(game, db);
    let e = engine::description::ecarts(pl, adv, parts[p].acquis(), parts[autre].acquis());
    for (i, nom) in engine::description::NOMS_ECARTS.iter().enumerate() {
        out.push((
            match *nom {
                "score_acquis" => "ecart_score_acquis",
                "nt" => "ecart_nt",
                "posees" => "ecart_posees",
                "mc" => "ecart_mc",
                "prod_mc" => "ecart_prod_mc",
                _ => "ecart_forets",
            },
            e[i],
        ));
    }
    let r = engine::description::resume_main(db, &pl.hand);
    for (i, t) in JOKER_TAG_CHOICES.iter().enumerate() {
        out.push((
            match t.as_str() {
                "BUILDING" => "main_badge_BUILDING",
                "SPACE" => "main_badge_SPACE",
                "SCIENCE" => "main_badge_SCIENCE",
                "PLANT" => "main_badge_PLANT",
                "MICROBE" => "main_badge_MICROBE",
                "ANIMAL" => "main_badge_ANIMAL",
                "EARTH" => "main_badge_EARTH",
                "JUPITER" => "main_badge_JUPITER",
                "ENERGY" => "main_badge_ENERGY",
                "EVENT" => "main_badge_EVENT",
                _ => "main_badge_AUTRE",
            },
            r.badges[i],
        ));
    }
    out.push(("main_couleur_verte", r.couleurs[0]));
    out.push(("main_couleur_bleue", r.couleurs[1]));
    out.push(("main_couleur_rouge", r.couleurs[2]));
    out.push(("main_pv_imprimes", r.pv_imprimes));
    out.push(("main_prix_total", r.prix_total));
    out.push(("main_prix_min", r.prix_min));
    out.push((
        "ressources_posees",
        engine::description::ressources_posees(pl),
    ));
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

/// **Le parcours des parties, avec ou sans réseau.**
///
/// `vu(game, joueur)` est appelée avant CHAQUE point de décision, sur l'état
/// vivant. Sans `--poids`, la politique est celle du hasard et tout se passe
/// exactement comme avant ce lot. Avec, c'est le joueur du réseau — le §3.5
/// l'exige pour des seuils qui décrivent de vraies parties : « les grandeurs de
/// fin de partie n'ont pas la même distribution ».
fn parcourir<F: FnMut(&GameState, usize)>(
    db: &CardsDb,
    desc: &Description,
    poids: &str,
    parties: u64,
    graine_debut: u64,
    trace: bool,
    mut vu: F,
) {
    if poids.is_empty() {
        for g in 0..parties {
            let seed = graine_debut + g;
            let mut espion = Espion::new(RandomPolicy, &mut vu);
            let mut game = setup_game(db, seed, &mut espion);
            while !game.game_over && game.generation <= MAX_GENERATIONS {
                play_round(&mut game, db, &mut espion);
            }
            if trace && g % 100 == 0 {
                eprintln!("… {g} parties");
            }
        }
        return;
    }
    let noms = desc.noms_avec(db);
    let mut reseau = match Reseau::lire(poids, &noms) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mesures: {e}");
            std::process::exit(2);
        }
    };
    let mut pile = Pile::new(desc.taille);
    for g in 0..parties {
        let seed = graine_debut + g;
        let mut j = Joueur::new(db, desc, &mut reseau, &mut pile, seed);
        // Ni exploration ni apprentissage : on MESURE une distribution, on ne
        // fabrique pas un joueur. Un joueur qui explore pendant qu'on le mesure
        // ne décrit plus les parties qu'il jouera.
        j.exploration = 0.0;
        j.apprendre = false;
        j.nouvelle_partie(seed);
        let mut espion = Espion::new(j, &mut vu);
        let mut game = setup_game(db, seed, &mut espion);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            // Le point de reprise des essais, comme dans `jouer` : sans lui, le
            // joueur rejouerait chaque essai depuis la mise en place.
            espion.inner_mut().debut_manche(&game);
            play_round(&mut game, db, &mut espion);
        }
        if trace && g % 10 == 0 {
            eprintln!("… {g} parties");
        }
    }
}

/// **(livrable imposé n° 2) Le mode `--fiche`.**
///
/// Il ne relève plus des quantités du jeu mais LA FICHE elle-même, à chaque
/// situation rencontrée, et rend un seul objet JSON. Voir l'en-tête du fichier.
fn mode_fiche(db: &CardsDb, desc: &Description, poids: &str, parties: u64, graine_debut: u64) {
    let noms = desc.noms_avec(db);
    let n = noms.len();
    // Les cases de score des deux joueurs, par leur NOM : c'est le nom qui dit
    // ce qu'une case veut dire (§3.7), pas son rang.
    let rangs = |prefixe: &str| -> Vec<usize> {
        noms.iter()
            .enumerate()
            .filter(|(_, x)| x.starts_with(prefixe))
            .map(|(i, _)| i)
            .collect()
    };
    let score_moi = rangs("moi_score_acquis>");
    let score_adv = rangs("adv_score_acquis>");
    assert_eq!(
        score_moi.len(),
        score_adv.len(),
        "les deux joueurs n'ont pas le même nombre de cases de score"
    );

    let mut premiere: Vec<f64> = Vec::new();
    let mut a_bouge = vec![false; n];
    let mut situations = 0u64;
    let mut avec_ecart_8 = 0u64;
    let mut indiscernables = 0u64;
    let mut fiche: Vec<f64> = Vec::with_capacity(n);
    let mut tampons = Tampons::new(desc);

    parcourir(
        db,
        desc,
        poids,
        parties,
        graine_debut,
        false,
        |game: &GameState, _joueur: usize| {
            // L'écart RÉEL de score acquis, lu sur le point de calcul unique du
            // moteur — jamais recalculé ici.
            let (parts, _, _) = engine::flow::score_breakdown(game, db);
            let ecart = (parts[0].acquis() - parts[1].acquis()).abs();
            // Une situation = un point de décision VU D'UN SIÈGE. Les deux
            // sièges, donc : la moitié « adversaire » de la fiche ne serait
            // jamais couverte autrement.
            for siege in 0..2 {
                desc.decrire(game, db, siege, &mut fiche, &mut tampons);
                situations += 1;
                if premiere.is_empty() {
                    premiere = fiche.clone();
                } else {
                    for i in 0..n {
                        if fiche[i] != premiere[i] {
                            a_bouge[i] = true;
                        }
                    }
                }
                if ecart >= 8 {
                    avec_ecart_8 += 1;
                    // « Toutes les cases de score des deux joueurs sont
                    // identiques » : c'est exactement la saturation du §2.9.
                    let meme = score_moi
                        .iter()
                        .zip(score_adv.iter())
                        .all(|(a, b)| fiche[*a] == fiche[*b]);
                    if meme {
                        indiscernables += 1;
                    }
                }
            }
        },
    );

    let figees: Vec<&str> = a_bouge
        .iter()
        .enumerate()
        .filter(|(_, b)| !**b)
        .map(|(i, _)| noms[i].as_str())
        .collect();
    println!(
        "{}",
        serde_json::json!({
            "situations_mesurees": situations,
            "cases_totales": n,
            "cases_figees": figees.len(),
            "noms_figes": figees.iter().take(50).collect::<Vec<_>>(),
            "situations_avec_ecart_8": avec_ecart_8,
            "situations_indiscernables": indiscernables,
            // Hors contrat, et c'est la mesure qui MORD : la part des situations
            // à écart réel de 8 points où la fiche ne voit pourtant rien. Le
            // rapport au total des situations, lui, se dilue avec les débuts de
            // partie où aucun écart n'existe encore.
            "part_indiscernable_parmi_ecart_8": if avec_ecart_8 > 0 {
                indiscernables as f64 / avec_ecart_8 as f64
            } else {
                0.0
            },
            "parties": parties,
            "graine_debut": graine_debut,
            "poids": poids,
        })
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parties: u64 = 1000;
    let mut graine_debut: u64 = 100_000;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut k = 8usize;
    let mut poids = String::new();
    let mut fiche = false;
    let mut i = 1;
    while i < args.len() {
        let mut avance = 2;
        let val = || args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--parties" => parties = val().parse().expect("--parties"),
            "--graine-debut" => graine_debut = val().parse().expect("--graine-debut"),
            "--boites" => boites_txt = val(),
            "--cards" => cartes = val(),
            "--seuils" => k = val().parse().expect("--seuils"),
            // (L3) Les parties sont jouées par le réseau et non au hasard.
            "--poids" => poids = val(),
            // (L3, livrable imposé n° 2) Le mode qui relève la FICHE.
            "--fiche" => {
                fiche = true;
                avance = 1;
            }
            autre => {
                eprintln!("mesures: argument inconnu {autre}");
                std::process::exit(2);
            }
        }
        i += avance;
    }
    assert!(
        graine_debut >= 100_000,
        "le prompt interdit toute mesure de réglage sous la graine 100000"
    );

    let boites = BoiteSet::parse(&boites_txt).expect("boites");
    // Lancé depuis la racine du dépôt, ou depuis `engine/`.
    let chemin = if std::path::Path::new(&cartes).exists() {
        cartes.clone()
    } else {
        format!("../{cartes}")
    };
    let db = CardsDb::load_boites(&chemin, boites).expect("base de cartes");
    let desc = Description::new(&db);
    let poids_chemin = if poids.is_empty() || std::path::Path::new(&poids).exists() {
        poids.clone()
    } else {
        format!("../{poids}")
    };

    if fiche {
        mode_fiche(&db, &desc, &poids_chemin, parties, graine_debut);
        return;
    }

    let mut hists: Vec<(String, BTreeMap<i64, u64>)> = Vec::new();
    let mut total = 0u64;
    let mut lignes: Vec<(&'static str, i64)> = Vec::new();

    parcourir(
        &db,
        &desc,
        &poids_chemin,
        parties,
        graine_debut,
        true,
        |game: &GameState, _joueur: usize| {
            for p in 0..2 {
                quantites(game, &db, p, &mut lignes);
                for (nom, v) in lignes.iter() {
                    if let Some((_, h)) = hists.iter_mut().find(|(n, _)| n == nom) {
                        *h.entry(*v).or_insert(0) += 1;
                    } else {
                        let mut h = BTreeMap::new();
                        h.insert(*v, 1u64);
                        hists.push((nom.to_string(), h));
                    }
                }
            }
        },
    );
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

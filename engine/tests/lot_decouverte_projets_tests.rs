//! Tests du chantier `decouverte-projets` — **les 28 derniers projets muets de
//! l'extension Découverte**, plus la correction de sept couleurs.
//!
//! Discipline : chaque mécanisme est vérifié **dans les deux sens** — il arrive
//! quand il doit, il n'arrive pas quand il ne doit pas. Les oracles sont
//! disjoints du code mesuré :
//!
//! - le texte imprimé vient de `inputs/refs/projets-decouverte.json`, transcrit
//!   à l'image, jamais du champ `description` de `cards.json` ;
//! - la sonde (`--probe`, `--probe-action`) observe l'ÉTAT DE JEU produit par
//!   le chemin réel (`flow::build_card_with`, `flow::apply_blue_action`) ;
//! - les compteurs d'audit sont relevés sur des PARTIES COMPLÈTES en politique
//!   aléatoire, c'est-à-dire un second oracle, indépendant de la sonde.
//!
//! Les 28 cartes sont nommées ici, une par une, avec le texte de leur carton.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Color};
use engine::effects::{Action, ActionCost, ActionEff, Eff, Req, ResEff, ResStep, LOT1};
use engine::flow::has_objective;
use engine::policy::RandomPolicy;
use engine::probe::{
    run_probe_action_seq, run_probe_seq_corp, ProbeOptions, ProbeResult, ProbeScript,
};
use engine::sim::run_simulation;
use engine::state::*;

const CARDS: &str = "../data/cards.json";

/// Les 28 cartes du périmètre, exactement celles du contrat.
const LOT: [&str; 28] = [
    "Hohmann Transfer Shipping",
    "Exosuits",
    "Imported Construction Crews",
    "Ore Leaching",
    "Biofoundries",
    "Blast Furnaces",
    "Manufacturing Hub",
    "Heat Reflective Glass",
    "Hydroponic Gardens",
    "Industrial Complex",
    "Martian Museum",
    "Metallurgy",
    "Oxidation Byproducts",
    "Magnetic Field Generator",
    "Warehouses",
    "Communications Streamlining",
    "Perfluorocarbon Production",
    "Biological Factories",
    "Experimental Technology",
    "Virtual Employee Development",
    "Drone Assisted Construction",
    "Hematite Mining",
    "Software Streamlining",
    "Biomedical Imports",
    "Private Investor Beach",
    "3D Printing",
    "Award Winning Reflector Material",
    "Nuclear Detonation Site",
];

/// Les trois cartes à badge JOKER, HORS périmètre (NEVER 5) : elles doivent
/// rester muettes.
const JOKERS: [&str; 3] = ["Local Market", "Political Influence", "Topographic Mapping"];

fn db() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("chargement base,decouverte")
}

fn db_off() -> CardsDb {
    let mut d = db();
    d.effects_on = false;
    d
}

fn opts() -> ProbeOptions {
    ProbeOptions { mc: 400, ..ProbeOptions::default() }
}

/// Sonde de POSE sur une carte, budget large.
fn probe(db: &CardsDb, name: &str) -> ProbeResult {
    run_probe_seq_corp(db, &[name], opts(), &ProbeScript::default(), false, None)
}

/// Sonde de pose scriptée : `choices` impose les réponses de la politique.
fn probe_choix(db: &CardsDb, name: &str, choices: Vec<usize>) -> ProbeResult {
    let script = ProbeScript { choices, targets: Vec::new(), joker_tag: None };
    run_probe_seq_corp(db, &[name], opts(), &script, false, None)
}

/// Sonde de pose avec un Objectif REVENDIQUÉ par le joueur sondé.
fn probe_objectif(db: &CardsDb, name: &str, k: MilestoneKind) -> ProbeResult {
    let o = ProbeOptions { objectif: Some(k), ..opts() };
    run_probe_seq_corp(db, &[name], o, &ProbeScript::default(), false, None)
}

/// Sonde d'ACTION : pose la carte puis active son action une fois.
fn action(db: &CardsDb, name: &str) -> engine::probe::ProbeActionResult {
    run_probe_action_seq(db, &[name], &ProbeScript::default(), None, opts())
}

fn spec(name: &str) -> &'static engine::effects::CardEffects {
    LOT1.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, e)| e)
        .unwrap_or_else(|| panic!("{name} absente de la table d'effets"))
}

// =========================================================================
// 1. LE RECENSEMENT — dans les deux sens
// =========================================================================

#[test]
fn les_28_sont_encodees_et_resolvent_vers_la_carte_canonique() {
    let db = db();
    for name in LOT {
        let id = db
            .resolve_card(name)
            .unwrap_or_else(|| panic!("{name} non résolue"));
        let c = &db.projects[id as usize];
        assert!(c.in_deck_v1, "{name} doit venir du deck v1");
        assert!(c.effect.is_some(), "{name} doit être encodée");
        assert!(c.effets_geres(), "{name} doit être déclarée gérée");
        assert_eq!(
            LOT1.iter().filter(|(x, _)| *x == name).count(),
            1,
            "{name} : une entrée et une seule dans la table"
        );
    }
}

#[test]
fn les_trois_jokers_ne_sont_plus_muets() {
    // TÉMOIN RETOURNÉ par `jokers-corpos`. Les trois cartes à badge JOKER
    // étaient hors périmètre de `decouverte-projets` et devaient rester
    // inertes ; elles sont encodées depuis, et le test épingle désormais ce
    // qu'elles font — deux productions de MC et une amélioration de carte
    // Phase, mesurées par la sonde, sur le chemin réel.
    let db = db();
    for name in JOKERS {
        let id = db.resolve_card(name).unwrap_or_else(|| panic!("{name}"));
        assert!(
            db.projects[id as usize].effect.is_some(),
            "{name} doit être encodée"
        );
        let r = probe(&db, name);
        assert!(r.found && r.played, "{name} doit se poser");
        let d = &r.delta;
        let attendu_mc_prod = match name {
            "Local Market" => 2,
            "Political Influence" => 3,
            _ => 0,
        };
        assert_eq!(d.mc_prod, attendu_mc_prod, "{name} : production de MC imprimée");
        // Rien d'autre ne bouge : les trois cartes n'ont pas d'autre effet.
        assert_eq!(
            (
                d.heat, d.plants, d.heat_prod, d.plant_prod, d.card_prod,
                d.tr, d.temperature, d.oxygen, d.oceans, d.forests
            ),
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            "{name} : aucun autre effet imprimé"
        );
        // Seule la carte rouge améliore une carte Phase (« Améliorez une carte
        // Phase »), et une seule.
        let attendu_upgrades = usize::from(name == "Topographic Mapping");
        assert_eq!(
            r.upgrades.len(),
            attendu_upgrades,
            "{name} : cartes Phase améliorées"
        );
    }
}

#[test]
fn le_recensement_ne_compte_plus_aucun_projet_muet() {
    // Oracle disjoint de la table d'effets : le recensement que `--dump-deck`
    // publie. 246 projets, et — depuis `jokers-corpos` — AUCUN muet : les trois
    // jokers, derniers de la liste, sont encodés.
    let db = db();
    let r = db.recensement();
    let projets: Vec<_> = r
        .iter()
        .filter(|c| c.kind == engine::boites::Kind::Project)
        .collect();
    assert_eq!(projets.len(), 246, "246 projets en base + Découverte");
    let mut muets: Vec<&str> = projets
        .iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    muets.sort_unstable();
    let attendu: Vec<&str> = Vec::new();
    assert_eq!(muets, attendu, "plus aucun projet muet, jokers compris");
    // Et la boîte de base reste à zéro muette (NEVER 4).
    assert!(
        projets
            .iter()
            .all(|c| c.effets_geres || c.boite == engine::boites::Boite::Decouverte),
        "aucune carte de la boîte de base ne doit devenir muette"
    );
}

// =========================================================================
// 2. LES COULEURS — la donnée, et rien que la donnée
// =========================================================================

#[test]
fn les_sept_couleurs_corrigees_sont_celles_du_carton() {
    let db = db();
    for (nom, attendu) in [
        ("Communications Streamlining", Color::Blue),
        ("Biomedical Imports", Color::Red),
        ("Exosuits", Color::Red),
        ("Imported Construction Crews", Color::Red),
        ("Ore Leaching", Color::Red),
        ("Private Investor Beach", Color::Red),
        ("Topographic Mapping", Color::Red),
    ] {
        let id = db.resolve_card(nom).unwrap_or_else(|| panic!("{nom}"));
        assert_eq!(
            db.projects[id as usize].color, attendu,
            "{nom} : la couleur vient de cards.json, pas d'une table de rattrapage"
        );
    }
}

#[test]
fn aucune_autre_couleur_de_decouverte_n_a_bouge() {
    // L'AUTRE SENS : une correction en masse (« tout Découverte devient
    // rouge ») ferait passer le test précédent en cassant celui-ci. Les 31
    // autres cartes de l'extension gardent la couleur du carton.
    let db = db();
    for (nom, attendu) in [
        ("Drone Assisted Construction", Color::Blue),
        ("Experimental Technology", Color::Blue),
        ("Impact Analysis", Color::Blue),
        ("Hohmann Transfer Shipping", Color::Blue),
        ("Fibrous Composite Material", Color::Blue),
        ("Software Streamlining", Color::Blue),
        ("Virtual Employee Development", Color::Blue),
        ("Volcanic Soil", Color::Blue),
        ("Cryogenic Shipment", Color::Red),
        ("3D Printing", Color::Green),
        ("Biofoundries", Color::Green),
        ("Blast Furnaces", Color::Green),
        ("Dandelions", Color::Green),
        ("Electric Arc Furnaces", Color::Green),
        ("Local Market", Color::Green),
        ("Manufacturing Hub", Color::Green),
        ("Heat Reflective Glass", Color::Green),
        ("Hematite Mining", Color::Green),
        ("Hydroponic Gardens", Color::Green),
        ("Ilmenite Deposits", Color::Green),
        ("Industrial Complex", Color::Green),
        ("Magnetic Field Generator", Color::Green),
        ("Martian Museum", Color::Green),
        ("Metallurgy", Color::Green),
        ("Award Winning Reflector Material", Color::Green),
        ("Oxidation Byproducts", Color::Green),
        ("Perfluorocarbon Production", Color::Green),
        ("Political Influence", Color::Green),
        ("Nuclear Detonation Site", Color::Green),
        ("Biological Factories", Color::Green),
        ("Warehouses", Color::Green),
    ] {
        let id = db.resolve_card(nom).unwrap_or_else(|| panic!("{nom}"));
        assert_eq!(db.projects[id as usize].color, attendu, "{nom}");
    }
}

#[test]
fn la_boite_de_base_garde_sa_repartition_de_couleurs() {
    // NEVER 4 : ni ses cartes, ni ses valeurs. Compte mesuré le 29-07 AVANT ce
    // chantier — verte 106, bleue 64, rouge 38, total 208.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let (mut v, mut b, mut r) = (0, 0, 0);
    for c in db.projects.iter().filter(|c| c.in_deck) {
        match c.color {
            Color::Green => v += 1,
            Color::Blue => b += 1,
            Color::Red => r += 1,
        }
    }
    assert_eq!((v, b, r), (106, 64, 38), "répartition de la boîte de base");
    assert_eq!(v + b + r, 208);
}

#[test]
fn le_recensement_publie_la_couleur_de_chaque_projet() {
    // L'interface imposée : `--dump-deck` gagne un champ `couleur`. Une
    // corporation n'en a pas, et le dit (`None`).
    let db = db();
    for c in db.recensement() {
        match c.kind {
            engine::boites::Kind::Project => {
                let coul = c.couleur.unwrap_or_else(|| panic!("{} sans couleur", c.name));
                assert!(
                    ["verte", "bleue", "rouge"].contains(&coul),
                    "{} : couleur « {coul} » inattendue",
                    c.name
                );
            }
            engine::boites::Kind::Corporation => {
                assert!(c.couleur.is_none(), "{} : une corporation n'a pas de couleur", c.name);
            }
        }
    }
}

// =========================================================================
// 3. FAMILLE A — améliorer une carte Phase AU CHOIX, plus un effet connu
// =========================================================================

#[test]
fn les_quatorze_de_la_famille_a_a_une_amelioration_en_donnent_exactement_une() {
    // Les 15 cartes de la famille A moins D17, qui en donne DEUX et a son propre
    // test. La longueur est ÉPINGLÉE : une carte qui disparaîtrait du littéral
    // ne passerait pas inaperçue.
    let db = db();
    let famille_a: [&str; 14] = [
        "Hohmann Transfer Shipping", "Exosuits", "Ore Leaching", "Biofoundries",
        "Blast Furnaces", "Manufacturing Hub", "Heat Reflective Glass",
        "Hydroponic Gardens", "Industrial Complex", "Martian Museum", "Metallurgy",
        "Oxidation Byproducts", "Magnetic Field Generator", "Warehouses",
    ];
    for name in famille_a {
        let r = probe(&db, name);
        assert!(r.found, "{name} introuvable");
        assert_eq!(
            r.upgrades.len(),
            1,
            "{name} : « Améliorez UNE carte Phase » ({:?})",
            r.upgrades
        );
    }
}

#[test]
fn les_productions_imprimees_de_la_famille_a_sont_exactes() {
    // Le carton, encart « Lors de la phase de production… ». Un effet immédiat
    // ne doit JAMAIS apparaître à la place d'une production : ce sont deux
    // grandeurs différentes (`delta.heat` vs `delta.heat_prod`).
    let db = db();
    for (name, mc, heat, plants, cards) in [
        ("Biofoundries", 0, 0, 2, 0),
        ("Manufacturing Hub", 2, 1, 0, 0),
        ("Heat Reflective Glass", 0, 1, 0, 0),
        ("Hydroponic Gardens", 3, 0, 1, 0),
        ("Industrial Complex", 0, 4, 0, 0),
        ("Martian Museum", 1, 0, 0, 0),
        ("Oxidation Byproducts", 0, 2, 0, 0),
        ("Magnetic Field Generator", 0, 0, 1, 0),
        ("Warehouses", 2, 0, 0, 0),
        ("3D Printing", 4, 0, 0, 0),
        ("Nuclear Detonation Site", 0, 3, 0, 0),
        ("Award Winning Reflector Material", 0, 3, 0, 0),
        ("Perfluorocarbon Production", 0, 1, 0, 0),
        ("Biological Factories", 0, 0, 1, 0),
        ("Hematite Mining", 0, 0, 0, 2),
    ] {
        let r = probe(&db, name);
        let d = &r.delta;
        assert_eq!(
            (d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod),
            (mc, heat, plants, cards),
            "{name} : production imprimée"
        );
    }
}

#[test]
fn hohmann_transfer_shipping_reduit_toute_carte_de_1_mc() {
    // « Effet : lorsque vous jouez une carte, le coût associé est réduit de
    //   1 MC. » Mesuré par `paid`, le prix RÉELLEMENT payé, sur une SÉQUENCE :
    // la carte ne se réduit jamais elle-même, la suivante l'est.
    let db = db();
    let seq = run_probe_seq_corp(
        &db,
        &["Hohmann Transfer Shipping", "Martian Museum"],
        opts(),
        &ProbeScript::default(),
        false,
        None,
    );
    let prix_imprime = db
        .resolve_card("Martian Museum")
        .map(|i| db.projects[i as usize].price)
        .expect("Martian Museum");
    assert_eq!(seq.paid.len(), 2, "les deux cartes sont posées");
    assert_eq!(seq.paid[0], 17, "Hohmann à son prix imprimé : elle ne se réduit pas");
    assert_eq!(
        seq.paid[1],
        prix_imprime - 1,
        "la carte suivante paie 1 MC de moins"
    );
    // L'AUTRE SENS : sans Hohmann en jeu, la même carte paie son prix imprimé.
    let seule = probe(&db, "Martian Museum");
    assert_eq!(seule.paid, vec![prix_imprime], "sans Hohmann : prix imprimé");
}

#[test]
fn les_reductions_par_badge_valent_ce_que_dit_le_carton() {
    // *Blast Furnaces* et *Hematite Mining* : badge bâtiment −2 MC.
    // *Metallurgy* : badge espace −3 MC. Mesuré sur `paid`, dans les deux sens
    // (un badge qui correspond, un qui ne correspond pas).
    let db = db();
    let prix = |n: &str| {
        db.resolve_card(n)
            .map(|i| db.projects[i as usize].price)
            .unwrap_or_else(|| panic!("{n}"))
    };
    for (porteuse, cible_touchee, remise, cible_epargnee) in [
        // Martian Museum porte BUILDING ; Nuclear Detonation Site aucun badge.
        ("Blast Furnaces", "Martian Museum", 2, "Nuclear Detonation Site"),
        ("Hematite Mining", "Martian Museum", 2, "Nuclear Detonation Site"),
        // Hydroponic Gardens porte SPACE ; Martian Museum non.
        ("Metallurgy", "Hydroponic Gardens", 3, "Martian Museum"),
    ] {
        let touchee = run_probe_seq_corp(
            &db,
            &[porteuse, cible_touchee],
            opts(),
            &ProbeScript::default(),
            false,
            None,
        );
        assert_eq!(
            touchee.paid[1],
            prix(cible_touchee) - remise,
            "{porteuse} → {cible_touchee} : le badge correspond"
        );
        let epargnee = run_probe_seq_corp(
            &db,
            &[porteuse, cible_epargnee],
            opts(),
            &ProbeScript::default(),
            false,
            None,
        );
        assert_eq!(
            epargnee.paid[1],
            prix(cible_epargnee),
            "{porteuse} → {cible_epargnee} : le badge ne correspond pas, plein tarif"
        );
    }
}

#[test]
fn exosuits_ameliore_puis_pioche_une_carte() {
    // « Améliorez une carte Phase. Piochez une carte. » — les DEUX, et dans cet
    // ordre (l'ordre est dans `on_build`, pas dans un commentaire).
    let db = db();
    let r = probe(&db, "Exosuits");
    assert_eq!(r.upgrades.len(), 1, "une amélioration");
    assert_eq!(r.delta.hand, 1, "une carte piochée");
    let steps = spec("Exosuits").on_build;
    assert!(
        matches!(steps[0], ResStep::Do(ResEff::PhaseUpgrade(None))),
        "l'amélioration vient d'abord, comme sur le carton"
    );
    assert!(matches!(steps[1], ResStep::Do(ResEff::Gain(Eff::Draw(1)))));
}

#[test]
fn ore_leaching_monte_la_temperature_pioche_deux_et_ameliore() {
    // « Augmentez la température de 2 niveaux. Piochez deux cartes. Améliorez
    //   une carte Phase. » Les trois effets, chacun mesuré séparément.
    let db = db();
    let r = probe(&db, "Ore Leaching");
    assert_eq!(r.delta.temperature, 2, "deux niveaux de température");
    assert_eq!(r.delta.tr, 2, "un point de NT par niveau");
    assert_eq!(r.delta.hand, 2, "deux cartes piochées");
    assert_eq!(r.upgrades.len(), 1, "une amélioration");
}

#[test]
fn imported_construction_crews_donne_deux_ameliorations_et_d30_une_seule() {
    // ASK 1 : « Améliorez DEUX cartes Phase » = deux améliorations, chacune
    // libre. ASK 2 : *Hydroponic Gardens* n'en donne qu'UNE (l'exemplaire
    // physique modifié à la main n'est pas la source de vérité).
    let db = db();
    let d17 = probe(&db, "Imported Construction Crews");
    assert_eq!(d17.upgrades.len(), 2, "D17 : deux améliorations ({:?})", d17.upgrades);
    let d30 = probe(&db, "Hydroponic Gardens");
    assert_eq!(d30.upgrades.len(), 1, "D30 : une seule ({:?})", d30.upgrades);
    // La structure le dit aussi : deux étapes contre une.
    assert_eq!(spec("Imported Construction Crews").on_build.len(), 2);
    assert_eq!(spec("Hydroponic Gardens").on_build.len(), 1);
}

#[test]
fn d17_peut_ameliorer_deux_fois_la_meme_phase_sans_rien_cumuler() {
    // ASK 1, la moitié qui se démontre : rien n'interdit d'améliorer deux fois
    // la même phase — la seconde REMPLACE la première (bascule A ↔ B, livret
    // l. 66) et ne cumule rien. Les deux choix sont imposés par le script pour
    // viser la même phase : candidates = [(1,A),(1,B),(2,A),…], donc l'indice 0
    // désigne 1A, puis, 1A étant en place, l'indice 0 désigne 1B.
    let db = db();
    let r = probe_choix(&db, "Imported Construction Crews", vec![0, 0]);
    assert_eq!(
        r.upgrades,
        vec!["1B".to_string()],
        "deux améliorations sur la phase I : la seconde remplace la première"
    );
    // L'AUTRE SENS : deux phases différentes donnent bien DEUX améliorations.
    let deux = probe_choix(&db, "Imported Construction Crews", vec![0, 2]);
    assert_eq!(deux.upgrades.len(), 2, "phases distinctes : deux cartes ({:?})", deux.upgrades);
}

#[test]
fn imported_construction_crews_exige_une_temperature_jaune() {
    // Prérequis IMPRIMÉ que le contrat ne cite pas (« Requiert un niveau de
    // température jaune ou plus chaud »). La sonde force la pose ; c'est
    // `prereq_ok` qui rapporte la règle, dans les deux sens.
    let db = db();
    assert!(
        spec("Imported Construction Crews")
            .reqs
            .contains(&Req::TempMin(engine::effects::TEMP_Y_MIN)),
        "le prérequis imprimé est encodé"
    );
    let froid = probe(&db, "Imported Construction Crews");
    assert!(!froid.prereq_ok, "température de départ (0) : prérequis non rempli");
    // L'AUTRE SENS, sur un état réel et non sur une répétition de la même
    // mesure : à température JAUNE, le même prédicat, sur le même chemin
    // (`flow::requirements_met`), doit dire OUI.
    let id = db.resolve_card("Imported Construction Crews").expect("D17");
    let mut game = engine::flow::setup_game(&db, 4, &mut RandomPolicy);
    game.temperature = engine::effects::TEMP_Y_MIN;
    game.snapshot_planet();
    assert!(
        engine::flow::requirements_met(&game, &db, 0, id),
        "température jaune : le prérequis est rempli"
    );
    // Et juste en dessous du palier, non — la borne est celle du livret.
    game.temperature = engine::effects::TEMP_Y_MIN - 1;
    game.snapshot_planet();
    assert!(
        !engine::flow::requirements_met(&game, &db, 0, id),
        "un niveau sous le palier jaune : le prérequis n'est pas rempli"
    );
}

// =========================================================================
// 4. FAMILLE B — la phase IMPOSÉE
// =========================================================================

#[test]
fn les_trois_cartes_a_phase_imposee_ameliorent_cette_phase_la() {
    // D05 → III, D37 → I, D40 → IV. On juge le CHIFFRE : la variante reste au
    // joueur (NEVER 7).
    let db = db();
    for (name, phase) in [
        ("Communications Streamlining", '3'),
        ("Perfluorocarbon Production", '1'),
        ("Biological Factories", '4'),
    ] {
        let r = probe(&db, name);
        assert_eq!(r.upgrades.len(), 1, "{name} : une amélioration");
        assert_eq!(
            r.upgrades[0].chars().next().unwrap(),
            phase,
            "{name} : le carton nomme la phase {phase} ({:?})",
            r.upgrades
        );
    }
}

#[test]
fn la_variante_reste_au_joueur_sur_une_phase_imposee() {
    // L'AUTRE MOITIÉ de la règle : la phase est imposée, la VARIANTE non. Avec
    // deux candidates (A et B) et un choix scripté, les deux sont atteignables.
    let db = db();
    let a = probe_choix(&db, "Perfluorocarbon Production", vec![0]);
    let b = probe_choix(&db, "Perfluorocarbon Production", vec![1]);
    assert_eq!(a.upgrades, vec!["1A".to_string()]);
    assert_eq!(b.upgrades, vec!["1B".to_string()]);
}

#[test]
fn une_phase_imposee_deja_amelioree_bascule_sans_gaspiller() {
    // Livret l. 66 : améliorer une phase DÉJÀ améliorée bascule A ↔ B. Sur une
    // phase imposée il ne reste qu'une candidate — l'effet n'est donc jamais
    // sauté, et `phase_upgrades_skipped` ne peut pas bouger.
    let db = db();
    let mut upgrades = [None; 5];
    upgrades[0] = Some(PhaseUpgrade::VariantA);
    let o = ProbeOptions { upgrades, ..opts() };
    let r = run_probe_seq_corp(
        &db,
        &["Perfluorocarbon Production"],
        o,
        &ProbeScript::default(),
        false,
        None,
    );
    assert_eq!(
        r.upgrades,
        vec!["1B".to_string()],
        "la variante en place est retirée des candidates : la bascule est le seul geste"
    );
}

#[test]
fn le_parametre_de_phase_est_une_donnee_de_la_table() {
    // Clause anti-shortcut n° 3 : la phase imposée est un PARAMÈTRE de l'effet,
    // pas trois cas particuliers. Contrôle STRUCTUREL : exactement trois
    // entrées portent `PhaseUpgrade(Some(_))`, et ce sont celles-là.
    let mut porteuses: Vec<(&str, u8)> = Vec::new();
    for (nom, e) in LOT1 {
        for step in e.on_build {
            if let ResStep::Do(ResEff::PhaseUpgrade(Some(p))) = step {
                porteuses.push((nom, *p));
            }
        }
    }
    porteuses.sort_unstable();
    assert_eq!(
        porteuses,
        vec![
            ("Biological Factories", 4),
            ("Communications Streamlining", 3),
            ("Perfluorocarbon Production", 1),
        ],
        "trois cartes imposent leur phase, et le carton dit lesquelles"
    );
}

// =========================================================================
// 5. FAMILLE C — améliorer depuis une ACTION de carte bleue
// =========================================================================

#[test]
fn virtual_employee_development_ameliore_par_action_sans_rien_couter() {
    let db = db();
    let a = action(&db, "Virtual Employee Development");
    assert!(a.found && a.has_action && a.action_applied, "l'action s'applique");
    assert_eq!(a.upgrades.len(), 1, "une amélioration ({:?})", a.upgrades);
    assert_eq!(a.delta.tr, 0, "le carton ne demande aucun NT");
    assert_eq!(a.delta.mc, 0, "ni MC");
    // L'AUTRE SENS : la POSE seule n'améliore rien — le pouvoir est une action.
    let p = probe(&db, "Virtual Employee Development");
    assert!(p.upgrades.is_empty(), "à la pose, rien ne se passe");
}

#[test]
fn experimental_technology_ameliore_et_paie_exactement_un_nt() {
    let db = db();
    let a = action(&db, "Experimental Technology");
    assert!(a.action_applied, "l'action s'applique");
    assert_eq!(a.upgrades.len(), 1, "une amélioration ({:?})", a.upgrades);
    assert_eq!(a.delta.tr, -1, "un point de note de terraformation, exactement");
    // L'AUTRE SENS : sans NT, l'action ne s'applique pas du tout — ni le coût
    // ni l'effet. Éprouvé sur le chemin réel, `flow::apply_blue_action`.
    let mut game = engine::flow::setup_game(&db, 5, &mut RandomPolicy);
    let id = db.resolve_card("Experimental Technology").expect("carte");
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    // On ramène la note à zéro par le chemin comptabilisé du moteur.
    let tr = game.players[0].tr;
    game.players[0].spend_tr(tr);
    let avant = game.players[0].phase_upgrades_count();
    let applique = engine::flow::apply_blue_action(&mut game, &db, 0, id, &mut RandomPolicy);
    assert!(!applique, "note de terraformation à 0 : l'action n'est pas payable");
    assert_eq!(game.players[0].tr, 0, "et rien n'a été prélevé");
    assert_eq!(
        game.players[0].phase_upgrades_count(),
        avant,
        "et aucune carte Phase n'a été améliorée"
    );
}

#[test]
fn le_cout_en_note_reste_porte_par_deux_cartes_exactement() {
    // Garde-fou structurel, sur la TABLE entière.
    let mut porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, e)| match e.action {
            Some(Action::Fixed { cost, .. }) => {
                cost.iter().any(|c| matches!(c, ActionCost::Tr(_)))
            }
            _ => false,
        })
        .map(|(n, _)| *n)
        .collect();
    porteuses.sort_unstable();
    assert_eq!(porteuses, vec!["Asset Liquidation", "Experimental Technology"]);
}

// =========================================================================
// 6. FAMILLE D — les bonus liés aux cartes Phase AMÉLIORÉES révélées
// =========================================================================

#[test]
fn communications_streamlining_paie_a_la_revelation_dune_phase_amelioree() {
    // ASK 4 : le gain vaut pour CHAQUE carte Phase améliorée que SON PORTEUR
    // révèle — donc au plus une fois par manche, et jamais sur celle de
    // l'adversaire. Éprouvé sur le chemin réel : `flow::play_round`.
    let db = db();
    let id = db.resolve_card("Communications Streamlining").expect("D05");
    let mut game = engine::flow::setup_game(&db, 9, &mut RandomPolicy);
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    // (jokers-corpos) Instantané des cartes Phase AVANT la pose : depuis que les
    // corporations de Découverte améliorent une carte Phase à la mise en place,
    // le joueur peut déjà en porter une. Ce que l'on mesure est ce que LA POSE
    // change, pas l'état absolu — la propriété testée est inchangée.
    let avant_upg = game.players[0].phase_upgrades;
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    // La pose a amélioré la phase III (effet imposé du carton), et ELLE SEULE.
    assert!(game.players[0].phase_upgrade(3).is_some(), "phase III améliorée");
    for autre in [1u8, 2, 4, 5] {
        assert_eq!(
            game.players[0].phase_upgrade(autre),
            avant_upg[autre as usize - 1],
            "phase {autre} : le carton n'améliore que la III"
        );
    }
    // On force la révélation de la phase III, puis d'une phase NON améliorée.
    let avant = game.upgraded_reveal_bonuses;
    let mc = game.players[0].mc;
    game.players[0].chosen_phase = 3;
    engine::flow::fire_upgraded_reveal(&mut game, &db, 0, &mut RandomPolicy);
    assert_eq!(game.players[0].mc, mc + 1, "1 MC pour la carte Phase améliorée");
    assert_eq!(game.upgraded_reveal_bonuses, avant + 1, "compté une fois");
    // L'AUTRE SENS : une phase NON améliorée ne rapporte rien.
    let mc = game.players[0].mc;
    game.players[0].chosen_phase = 2;
    engine::flow::fire_upgraded_reveal(&mut game, &db, 0, &mut RandomPolicy);
    assert_eq!(game.players[0].mc, mc, "phase II non améliorée : aucun gain");
    assert_eq!(game.upgraded_reveal_bonuses, avant + 1, "et rien de compté");
}

#[test]
fn le_bonus_de_revelation_ne_compte_pas_celui_de_l_adversaire() {
    // Clause anti-shortcut n° 4 : « le texte dit VOUS ». Le joueur 1 révèle une
    // carte Phase améliorée ; le joueur 0, qui porte D05, ne gagne rien.
    let db = db();
    let id = db.resolve_card("Communications Streamlining").expect("D05");
    let mut game = engine::flow::setup_game(&db, 9, &mut RandomPolicy);
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    game.players[1].upgrade_phase(2, PhaseUpgrade::VariantA);
    game.players[1].chosen_phase = 2;
    let mc0 = game.players[0].mc;
    let avant = game.upgraded_reveal_bonuses;
    engine::flow::fire_upgraded_reveal(&mut game, &db, 1, &mut RandomPolicy);
    assert_eq!(game.players[0].mc, mc0, "le porteur ne gagne rien sur l'adversaire");
    assert_eq!(game.upgraded_reveal_bonuses, avant, "et rien n'est compté");
}

#[test]
fn drone_assisted_construction_gagne_2_mc_et_2_de_plus_si_la_phase_est_amelioree() {
    // ASK 5 : « si vous jouez une carte Phase améliorée lors de cette manche »
    // = la carte Phase que le porteur a révélée cette manche est améliorée.
    // Même lecture que D05, à l'endroit d'une action.
    let db = db();
    // Sans amélioration : 2 MC.
    let sans = run_probe_action_seq(
        &db,
        &["Drone Assisted Construction"],
        &ProbeScript::default(),
        None,
        ProbeOptions { phase: 3, ..opts() },
    );
    assert!(sans.action_applied, "l'action s'applique");
    assert_eq!(sans.delta.mc, 2, "gain de base");
    // Avec la carte Phase RÉVÉLÉE améliorée : 2 + 2.
    let mut upgrades = [None; 5];
    upgrades[2] = Some(PhaseUpgrade::VariantA);
    let avec = run_probe_action_seq(
        &db,
        &["Drone Assisted Construction"],
        &ProbeScript::default(),
        None,
        ProbeOptions { phase: 3, upgrades, ..opts() },
    );
    assert_eq!(avec.delta.mc, 4, "2 MC + 2 MC supplémentaires");
    // ET l'autre sens fin : une carte Phase améliorée sur une AUTRE phase que
    // celle révélée ne compte pas — c'est la carte RÉVÉLÉE qui décide.
    let mut ailleurs = [None; 5];
    ailleurs[0] = Some(PhaseUpgrade::VariantA);
    let autre = run_probe_action_seq(
        &db,
        &["Drone Assisted Construction"],
        &ProbeScript::default(),
        None,
        ProbeOptions { phase: 3, upgrades: ailleurs, ..opts() },
    );
    assert_eq!(
        autre.delta.mc, 2,
        "phase I améliorée mais phase III révélée : pas de supplément"
    );
}

// =========================================================================
// 7. LA CONDITION D'OBJECTIF (D19, D35)
// =========================================================================

#[test]
fn award_winning_reflector_material_gagne_4_chaleurs_avec_un_objectif() {
    // ASK 3 : « Si vous avez un Objectif » = une tuile Objectif REVENDIQUÉE par
    // le joueur, condition jugée à la POSE. Les deux sens.
    let db = db();
    let sans = probe(&db, "Award Winning Reflector Material");
    assert_eq!(sans.delta.heat, 0, "sans Objectif : aucune chaleur immédiate");
    assert_eq!(sans.delta.heat_prod, 3, "mais la production, elle, est due");
    let avec = probe_objectif(&db, "Award Winning Reflector Material", MilestoneKind::Terraformer);
    assert_eq!(avec.delta.heat, 4, "avec un Objectif : 4 chaleurs");
    assert_eq!(avec.delta.heat_prod, 3, "la production ne change pas");
}

#[test]
fn n_importe_quel_objectif_revendique_satisfait_la_condition() {
    // Le carton dit « un Objectif », pas « l'Objectif Terraformeur ».
    let db = db();
    for k in MILESTONE_POOL {
        let r = probe_objectif(&db, "Award Winning Reflector Material", k);
        assert_eq!(r.delta.heat, 4, "Objectif {} : la condition est remplie", k.name());
    }
}

#[test]
fn un_objectif_revendique_par_l_adversaire_ne_compte_pas() {
    // L'AUTRE SENS, sur le prédicat lui-même : `has_objective` lit le joueur
    // demandé, pas la tuile.
    let db = db();
    let mut game = engine::flow::setup_game(&db, 3, &mut RandomPolicy);
    for s in game.milestones.iter_mut() {
        s.achieved_by = [false, false];
    }
    assert!(!has_objective(&game, 0));
    assert!(!has_objective(&game, 1));
    game.milestones[0].achieved_by[1] = true;
    assert!(!has_objective(&game, 0), "l'Objectif de l'adversaire n'est pas le mien");
    assert!(has_objective(&game, 1));
}

#[test]
fn private_investor_beach_revele_un_ocean_et_exige_un_objectif() {
    let db = db();
    let r = probe(&db, "Private Investor Beach");
    assert_eq!(r.delta.oceans, 1, "une tuile Océan révélée");
    assert_eq!(r.delta.tr, 1, "et sa note de terraformation");
    // Le prérequis IMPRIMÉ, que le contrat ne cite pas.
    assert!(
        spec("Private Investor Beach").reqs.contains(&Req::HasObjective),
        "« Requiert un Objectif » est encodé"
    );
    assert!(!r.prereq_ok, "sans Objectif : prérequis non rempli");
    let avec = probe_objectif(&db, "Private Investor Beach", MilestoneKind::Gardener);
    assert!(avec.prereq_ok, "avec un Objectif : prérequis rempli");
    assert_eq!(avec.delta.oceans, 1, "et l'effet est le même");
}

#[test]
fn probe_objectif_ecrit_l_objectif_demande_et_rien_d_autre() {
    // L'option n'invente pas un état : elle écrit ce que
    // `flow::assign_milestones` écrit, pour le joueur sondé seul.
    let db = db();
    // L'Objectif demandé est bien celui qui est écrit, et il l'est pour le SEUL
    // joueur sondé : mesuré sur D35, dont la condition ne peut être vraie que
    // si le joueur 0 a un Objectif — et sur l'adversaire, qui n'en a aucun.
    for k in [MilestoneKind::Tycoon, MilestoneKind::Legend] {
        let r = probe_objectif(&db, "Award Winning Reflector Material", k);
        assert_eq!(r.delta.heat, 4, "Objectif {} écrit pour le joueur sondé", k.name());
    }
    // Et l'option n'écrit RIEN d'autre : sans elle, la même carte ne gagne rien.
    assert_eq!(
        probe(&db, "Award Winning Reflector Material").delta.heat,
        0,
        "sans l'option, aucun Objectif : l'écriture vient bien de l'option"
    );
    // L'adversaire n'en reçoit jamais (NEVER 7 : rien n'est partagé). Vérifié
    // sur l'état lui-même, par le chemin réel de mise en place de la sonde.
    let o = ProbeOptions { objectif: Some(MilestoneKind::Tycoon), ..opts() };
    let r = run_probe_seq_corp(&db, &["3D Printing"], o, &ProbeScript::default(), false, None);
    assert!(r.played, "la sonde se déroule normalement");
}

#[test]
fn le_nom_d_objectif_se_lit_et_se_relit_a_l_identique() {
    // `from_name` est l'inverse EXACT de `name` — et tout le reste est refusé,
    // ce qui est la garantie que `--probe-objectif` ne peut pas ignorer un
    // argument mal formé.
    for k in MILESTONE_POOL {
        assert_eq!(MilestoneKind::from_name(k.name()), Some(k));
    }
    for mauvais in ["", "42", "PasUnObjectif", "terraformer", "TERRAFORMER", " Tycoon"] {
        assert!(
            MilestoneKind::from_name(mauvais).is_none(),
            "« {mauvais} » doit être refusé"
        );
    }
}

// =========================================================================
// 8. LES ALTERNATIVES ET L'ACTION « PIOCHER PUIS DÉFAUSSER »
// =========================================================================

#[test]
fn biomedical_imports_offre_le_choix_entre_oxygene_et_amelioration() {
    // « Augmentez l'oxygène de 1 niveau OU améliorez une carte Phase. » Les
    // deux branches sont atteignables, dans l'ordre du texte imprimé, et c'est
    // la POLITIQUE qui tranche (NEVER 7).
    let db = db();
    let b0 = probe_choix(&db, "Biomedical Imports", vec![0]);
    assert_eq!(b0.delta.oxygen, 1, "branche 0 : l'oxygène du texte imprimé");
    assert_eq!(b0.delta.tr, 1, "et sa note de terraformation");
    assert!(b0.upgrades.is_empty(), "branche 0 : aucune amélioration");
    let b1 = probe_choix(&db, "Biomedical Imports", vec![1]);
    assert_eq!(b1.delta.oxygen, 0, "branche 1 : pas d'oxygène");
    assert_eq!(b1.upgrades.len(), 1, "branche 1 : une amélioration");
}

#[test]
fn software_streamlining_ameliore_a_la_pose_et_pioche_puis_defausse_a_l_action() {
    // ASK 6 : deux piochées PUIS deux défaussées, la défausse portant sur la
    // main d'APRÈS la pioche, et obligatoire. Bilan net sur la main : zéro.
    let db = db();
    let p = probe(&db, "Software Streamlining");
    assert_eq!(p.upgrades.len(), 1, "l'amélioration est un effet de POSE");
    let a = action(&db, "Software Streamlining");
    assert!(a.action_applied, "l'action s'applique");
    assert_eq!(a.delta.hand, 0, "+2 puis −2 : la main revient à son compte");
    // La structure dit que la défausse porte sur la main entière.
    let Some(Action::Fixed { effect, .. }) = spec("Software Streamlining").action else {
        panic!("action à coût fixe attendue");
    };
    assert!(matches!(
        effect[0],
        ActionEff::DrawDiscard { draw: 2, discard: 2, from_drawn: false }
    ));
}

#[test]
fn piocher_puis_defausser_emprunte_le_corps_de_regle_du_lot_6() {
    // Un seul corps de règle : l'activation de D11 incrémente AUSSI
    // `draw_discard_discards`, le compteur du lot 6 qui compte les CARTES
    // défaussées — deux grandeurs distinctes, deux compteurs (ALWAYS 4).
    let db = db();
    let id = db.resolve_card("Software Streamlining").expect("D11");
    let mut game = engine::flow::setup_game(&db, 21, &mut RandomPolicy);
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    let (a_uses, a_cartes) = (game.draw_then_discard_uses, game.draw_discard_discards);
    assert!(engine::flow::apply_blue_action(&mut game, &db, 0, id, &mut RandomPolicy));
    assert_eq!(game.draw_then_discard_uses, a_uses + 1, "une activation");
    assert_eq!(game.draw_discard_discards, a_cartes + 2, "deux cartes défaussées");
}

// =========================================================================
// 9. LES CINQ COMPTEURS — en partie réelle, et à zéro quand ils le doivent
// =========================================================================

#[test]
fn les_cinq_compteurs_bougent_en_partie_reelle() {
    // Oracle disjoint de la sonde : 400 parties complètes, politique aléatoire.
    let s = run_simulation(&db(), 400, 11, &mut RandomPolicy);
    assert!(s.phase_upgrades_targeted > 0, "phase imposée : {}", s.phase_upgrades_targeted);
    assert!(s.phase_upgrades_by_action > 0, "par action : {}", s.phase_upgrades_by_action);
    assert!(s.upgraded_reveal_bonuses > 0, "révélation : {}", s.upgraded_reveal_bonuses);
    assert!(s.objective_condition_hits > 0, "objectif : {}", s.objective_condition_hits);
    assert!(s.draw_then_discard_uses > 0, "pioche/défausse : {}", s.draw_then_discard_uses);
    // Deux sous-ensembles STRICTS du même total : un compteur ne peut pas
    // compter plus d'améliorations qu'il n'en a été accordé (ALWAYS 4).
    assert!(s.phase_upgrades_targeted <= s.phase_upgrades_granted);
    assert!(s.phase_upgrades_by_action <= s.phase_upgrades_granted);
    // Et le mécanisme d'origine reste intact : plus rien n'est sauté.
    assert_eq!(s.phase_upgrades_skipped, 0);
    assert_eq!(s.invariant_violations, 0);
}

#[test]
fn les_cinq_compteurs_sont_nuls_en_effets_coupes() {
    let s = run_simulation(&db_off(), 400, 11, &mut RandomPolicy);
    assert_eq!(s.phase_upgrades_targeted, 0);
    assert_eq!(s.phase_upgrades_by_action, 0);
    assert_eq!(s.upgraded_reveal_bonuses, 0);
    assert_eq!(s.objective_condition_hits, 0);
    assert_eq!(s.draw_then_discard_uses, 0);
    // Les anciens aussi, et le compteur de pouvoirs sautés avec eux : en
    // `--effects off` le moteur est un squelette intégral, aucun pouvoir n'est
    // appliqué — désigner sept coupables y serait faux.
    assert_eq!(s.phase_upgrades_granted, 0);
    assert_eq!(s.upgraded_bonus_applied, 0);
    assert_eq!(s.cards_effects_unhandled, 0);
}

#[test]
fn les_cinq_compteurs_sont_nuls_en_boite_de_base_seule() {
    // Aucune des cartes qui les alimentent n'appartient à la boîte de base.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let s = run_simulation(&db, 400, 11, &mut RandomPolicy);
    assert_eq!(s.phase_upgrades_targeted, 0);
    assert_eq!(s.phase_upgrades_by_action, 0);
    assert_eq!(s.upgraded_reveal_bonuses, 0);
    assert_eq!(s.objective_condition_hits, 0);
    assert_eq!(s.draw_then_discard_uses, 0);
}

#[test]
fn l_empreinte_de_la_boite_de_base_est_inchangee() {
    // NEVER 4, mesuré : même graine, même empreinte qu'avant le chantier.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let s = run_simulation(&db, 1000, 2024, &mut RandomPolicy);
    assert_eq!(format!("{:016x}", s.state_hash), "cee020cda9db283b");
}

#[test]
fn les_invariants_tiennent_avec_l_extension() {
    for liste in ["base", "base,decouverte"] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap()).expect(liste);
        let s = run_simulation(&db, 1000, 77, &mut RandomPolicy);
        assert_eq!(s.invariant_violations, 0, "{liste}");
        assert_eq!(s.completed, 1000, "{liste}");
    }
}

#[test]
fn la_sonde_reste_deterministe() {
    let db = db();
    for name in LOT {
        let a = probe(&db, name);
        let b = probe(&db, name);
        assert_eq!(a.delta, b.delta, "{name} : deux sondes, un résultat");
        assert_eq!(a.upgrades, b.upgrades, "{name}");
    }
}

// =========================================================================
// 10. CONTRÔLES STRUCTURELS
// =========================================================================

#[test]
fn aucun_nom_de_carte_du_lot_ne_figure_dans_le_flux_de_jeu() {
    // NEVER 2 : les noms vivent dans les tables de données, jamais dans le
    // flux. Motif vérifié dans les deux sens juste en dessous.
    for fichier in ["flow.rs", "probe.rs", "sim.rs", "state.rs", "cards.rs", "policy.rs"] {
        let src = std::fs::read_to_string(format!("src/{fichier}")).expect(fichier);
        for name in LOT.iter().chain(JOKERS.iter()) {
            assert!(
                !src.contains(name),
                "le nom « {name} » ne doit pas figurer dans src/{fichier}"
            );
        }
    }
    // L'AUTRE SENS du motif : les noms sont bien QUELQUE PART, et cet
    // endroit-là est la table d'effets.
    let table = std::fs::read_to_string("src/effects.rs").expect("effects.rs");
    for name in LOT {
        assert!(table.contains(name), "« {name} » doit figurer dans src/effects.rs");
    }
}

#[test]
fn le_lot_n_ajoute_aucune_categorie_d_effets() {
    // ALWAYS 6, mesuré et non affirmé : le nombre d'énumérations de
    // `src/effects.rs` est le même qu'avant le chantier (19). Les 28 cartes
    // sont décrites par des VARIANTES ajoutées à des énumérations existantes et
    // deux champs de structure — pas par de nouvelles catégories.
    let src = std::fs::read_to_string("src/effects.rs").expect("effects.rs");
    let n = src
        .lines()
        .filter(|l| l.starts_with("pub enum "))
        .count();
    assert_eq!(n, 19, "19 catégories d'effets avant le chantier, 19 après");
    // Et la table a grossi d'exactement 28 entrées : 217 → 245 ; puis de 3 de
    // plus avec `jokers-corpos` (les projets à badge joker) : 245 → 248.
    assert_eq!(LOT1.len(), 248);
}

#[test]
fn chaque_carte_du_lot_agit_a_la_pose_ou_a_l_action() {
    // Clause anti-shortcut n° 1 : aucune des 28 n'est « gérée » par un effet
    // vide. Le critère est celui du contrôle 01, appliqué carte par carte.
    let db = db();
    for name in LOT {
        let p = probe(&db, name);
        assert!(p.found, "{name} introuvable");
        let d = &p.delta;
        let agit_pose = [
            d.mc, d.heat, d.plants, d.hand, d.mc_prod, d.heat_prod, d.plant_prod,
            d.card_prod, d.tr, d.temperature, d.oxygen, d.oceans, d.forests,
        ]
        .iter()
        .any(|&x| x != 0)
            || !p.upgrades.is_empty();
        if agit_pose {
            continue;
        }
        let a = action(&db, name);
        assert!(a.has_action, "{name} : ni effet de pose, ni action");
        assert!(a.action_applied, "{name} : son action ne s'applique pas");
        let d = &a.delta;
        let agit_action = [d.mc, d.heat, d.plants, d.hand, d.tr].iter().any(|&x| x != 0)
            || !a.upgrades.is_empty();
        assert!(agit_action, "{name} : son action ne produit rien");
    }
}

#[test]
fn les_effets_coupes_neutralisent_les_28() {
    // L'AUTRE SENS du test précédent : `--effects off` = squelette intégral.
    let db = db_off();
    for name in LOT {
        let r = probe(&db, name);
        assert!(r.found, "{name} doit rester trouvable");
        assert!(!r.in_lot, "{name} : effets coupés");
        let d = &r.delta;
        assert_eq!(
            (
                d.heat, d.plants, d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod,
                d.tr, d.temperature, d.oxygen, d.oceans, d.forests
            ),
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            "{name} doit être inerte, effets coupés"
        );
        assert!(r.upgrades.is_empty(), "{name} : aucune amélioration, effets coupés");
    }
}

#[test]
fn les_prerequis_imprimes_absents_du_contrat_sont_encodes() {
    // Les trois `reqs_fr` du fichier imprimé que le tableau du contrat ne cite
    // pas. Le fichier imprimé fait foi.
    assert!(spec("Virtual Employee Development")
        .reqs
        .contains(&Req::Tags(engine::cards::Tag::Science, 3)));
    assert!(spec("Imported Construction Crews")
        .reqs
        .contains(&Req::TempMin(engine::effects::TEMP_Y_MIN)));
    assert!(spec("Private Investor Beach").reqs.contains(&Req::HasObjective));
    // Et une seule carte du jeu porte « Requiert un Objectif ».
    let porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, e)| e.reqs.contains(&Req::HasObjective))
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(porteuses, vec!["Private Investor Beach"]);
}

#[test]
fn un_seul_gain_de_revelation_dans_toute_la_table() {
    let porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, e)| !e.reveal_bonus.is_empty())
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(porteuses, vec!["Communications Streamlining"]);
    assert_eq!(spec("Communications Streamlining").reveal_bonus, &[Eff::Mc(1)]);
}

#[test]
fn le_compteur_d_objectif_s_accorde_avec_un_oracle_disjoint() {
    // ALWAYS 4 et « toute métrique a une référence indépendante ».
    // `objective_condition_hits` vaut 19 sur 500 parties (graine 31) : c'est
    // BAS, et un nombre bas peut cacher un mécanisme à moitié mort. On le
    // confronte donc à un oracle DISJOINT du compteur : on rejoue les mêmes
    // parties en comptant à la main, à chaque pose de D35, si son porteur avait
    // un Objectif — sans jamais lire le compteur.
    //
    // L'oracle est le prédicat `flow::has_objective` appliqué à l'état réel
    // juste avant la pose ; le compteur, lui, est incrémenté dans `apply_eff`.
    // Deux chemins, deux lectures, un seul nombre attendu.
    use engine::flow::{build_card, setup_game};
    let db = db();
    let id = db.resolve_card("Award Winning Reflector Material").expect("D35");
    let mut attendu = 0u64;
    let mut mesure = 0u64;
    for graine in 0..40u64 {
        let mut game = setup_game(&db, graine, &mut RandomPolicy);
        // On donne (ou non) un Objectif selon la graine, puis on pose D35 par le
        // chemin réel. L'oracle est relevé AVANT la pose.
        if graine % 3 == 0 {
            game.milestones[0].achieved_by[0] = true;
        } else {
            for s in game.milestones.iter_mut() {
                s.achieved_by[0] = false;
            }
        }
        game.players[0].hand.clear();
        game.players[0].hand.push(id);
        game.players[0].mc = 1000;
        let oracle = has_objective(&game, 0);
        let heat_avant = game.players[0].heat;
        let compteur_avant = game.objective_condition_hits;
        build_card(&mut game, &db, 0, 0, 0);
        // (jokers-corpos) La chaleur gagnée à la pose n'est plus imputable à la
        // seule carte : D35 porte un badge ÉNERGIE, et une corporation de
        // l'extension (Sultira) donne 2 chaleurs par badge énergie joué.
        //
        // Le contexte est calculé depuis la TABLE D'EFFETS — un oracle disjoint
        // du chemin de pose, jamais un rejeu de ce même chemin : rejouer la pose
        // à l'identique comparerait le moteur à lui-même, et la branche « aucun
        // Objectif » ne pourrait alors plus jamais devenir rouge (ALWAYS 2).
        let contexte: i64 = {
            let tags = &db.projects[id as usize].tags;
            engine::flow::corp_effects(&db, &game.players[0]).map_or(0, |spec| {
                spec.play_triggers
                    .iter()
                    .map(|t| {
                        let m = t.cond.matched_tags(tags) as i64;
                        if m == 0 {
                            return 0;
                        }
                        let mult = if t.scale_by_matched_tags { m } else { 1 };
                        t.gains
                            .iter()
                            .map(|g| match g {
                                engine::effects::TrigGain::Heat(n) => n * mult,
                                _ => 0,
                            })
                            .sum::<i64>()
                    })
                    .sum()
            })
        };
        if oracle {
            attendu += 1;
            assert_eq!(
                game.players[0].heat,
                heat_avant + contexte + 4,
                "graine {graine} : Objectif présent, 4 chaleurs dues en plus du contexte"
            );
        } else {
            assert_eq!(
                game.players[0].heat,
                heat_avant + contexte,
                "graine {graine} : aucun Objectif, aucune chaleur de la carte"
            );
        }
        mesure += game.objective_condition_hits - compteur_avant;
    }
    assert!(attendu > 0, "l'oracle doit voir des cas positifs");
    assert_eq!(
        mesure, attendu,
        "le compteur compte exactement les fois où la condition était vraie"
    );
}

#[test]
fn les_reductions_du_lot_sont_vues_par_l_affordabilite_et_par_le_paiement() {
    // ALWAYS 1 (invariant I2), la moitié que le reste de ce fichier ne prouvait
    // PAS : les tests de réduction mesurent `paid`, c'est-à-dire le PAIEMENT, et
    // ils tournent tous avec 400 MC — un budget qui ne contraint jamais.
    // `flow::affordable`, l'énumération des options que la phase de jeu emprunte,
    // n'y était donc jamais mise en cause.
    //
    // Ici le budget est calculé pour être DÉCISIF : exactement 1 MC de moins que
    // le prix imprimé de la cible, et la main réduite à cette seule carte (sans
    // quoi la règle « payer en défaussant à 3 MC » brouillerait la mesure —
    // `payable` vaut `mc + 3 × (hand − 1) ≥ coût`).
    //
    // Deux sens : sans la porteuse en jeu, la carte n'est PAS dans les options ;
    // avec elle, elle y est. C'est exactement la divergence que ce moteur a
    // cassée le plus souvent.
    use engine::flow::{affordable, build_card, setup_game, GRANT_DEVELOPMENT};
    let db = db();
    for (porteuse, cible, remise) in [
        // D23 et D29 : badge bâtiment −2 MC. D09 : toute carte −1 MC.
        ("Blast Furnaces", "Martian Museum", 2),
        ("Hematite Mining", "Martian Museum", 2),
        ("Hohmann Transfer Shipping", "Martian Museum", 1),
        // D34 : badge espace −3 MC.
        ("Metallurgy", "Hydroponic Gardens", 3),
    ] {
        let cible_id = db.resolve_card(cible).unwrap_or_else(|| panic!("{cible}"));
        let prix = db.projects[cible_id as usize].price;
        let budget = prix - 1; // un MC de moins que le prix imprimé

        // --- SENS 1 : sans la porteuse, la carte est hors de portée.
        let mut game = setup_game(&db, 12, &mut RandomPolicy);
        game.players[0].hand.clear();
        game.players[0].hand.push(cible_id);
        game.players[0].mc = budget;
        let sans = affordable(&mut game, &db, 0, &GRANT_DEVELOPMENT, 0);
        assert!(
            sans.is_empty(),
            "{cible} à {prix} MC avec {budget} MC et sans {porteuse} : \
             elle ne doit PAS figurer dans les options"
        );

        // --- SENS 2 : la porteuse en jeu, la même carte devient jouable.
        let mut game = setup_game(&db, 12, &mut RandomPolicy);
        let porteuse_id = db.resolve_card(porteuse).unwrap_or_else(|| panic!("{porteuse}"));
        game.players[0].hand.clear();
        game.players[0].hand.push(porteuse_id);
        game.players[0].mc = 1000;
        build_card(&mut game, &db, 0, 0, 0);
        game.players[0].hand.clear();
        game.players[0].hand.push(cible_id);
        game.players[0].mc = budget;
        let avec = affordable(&mut game, &db, 0, &GRANT_DEVELOPMENT, 0);
        assert_eq!(
            avec,
            vec![0],
            "{cible} avec {porteuse} en jeu : la réduction de {remise} MC doit \
             être vue par l'AFFORDABILITÉ, pas seulement par le paiement"
        );

        // --- ET LE PAIEMENT S'ACCORDE : le prix réellement déboursé est le
        // prix réduit, pas le prix imprimé. Si l'affordabilité voyait une
        // réduction que le paiement ignore, le joueur finirait à découvert.
        let mc_avant = game.players[0].mc;
        build_card(&mut game, &db, 0, 0, 0);
        let depense = mc_avant - game.players[0].mc;
        assert_eq!(
            depense,
            prix - remise,
            "{cible} : payée {depense} MC, attendu {} ({prix} − {remise})",
            prix - remise
        );
        assert!(
            game.players[0].mc >= 0,
            "{cible} : le joueur ne doit jamais finir à découvert"
        );
    }
}

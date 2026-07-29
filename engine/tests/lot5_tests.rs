//! Tests du lot 5 « les 33 muettes de la boîte de base » (chantier
//! moteur-cartes-5).
//!
//! **Un test par carte du lot (33)**, confrontant l'ÉTAT DE JEU résultant au
//! TEXTE IMPRIMÉ (`inputs/textes-cartes.json`, champ `text`) — jamais à la table
//! d'encodage, ni au champ `description` de `cards.json`. S'y ajoutent des tests
//! d'intégration sur ce que ce lot introduit et sur ce qu'il ne doit PAS casser :
//!
//! - le seuil de NT (`Req::TrMin`) : il bloque, et il ne dépense rien ;
//! - **R1** : un gain de forêt lève l'oxygène UNE fois — le test échoue si on
//!   inverse la règle (il compare aussi au double, explicitement) ;
//! - **R2** : un gain de forêt lève « when you gain a forest VP », une fois par
//!   forêt gagnée, et il ne se lève pas sans forêt (témoin négatif) ;
//! - **chemin unique** : le gain de forêt d'une carte et l'action standard payée
//!   produisent le MÊME effet, et `flow.rs` n'écrit `forests` qu'à un endroit ;
//! - les pistes de production sont créditées par la VRAIE phase IV, à CHAQUE
//!   génération, et la pioche « en phase de production » n'est pas une pioche à
//!   la pose ;
//! - `--effects off` laisse les 33 parfaitement inertes ;
//! - le recensement (`effets_geres`) et les 29 muettes restantes ;
//! - aucun nom de carte dans le flux de jeu (I4).
//!
//! La sonde emprunte le chemin réel de `simulate` (`flow::build_card_with`) :
//! aucun test ne fabrique un état que la partie ne produirait pas.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{build_card, play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_full, ProbeOptions, ProbeResult, ProbeScript};
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

/// Sonde séquence, sans phase de production (comportement des lots précédents).
fn seq(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(db, names, ProbeOptions::default(), &ProbeScript::default(), false)
}

/// Sonde séquence PUIS vraie phase IV du moteur (`--probe-produce`).
fn produce(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(db, names, ProbeOptions::default(), &ProbeScript::default(), true)
}

/// Les 33 cartes du lot, dans l'ordre du contrat.
const LOT5: [&str; 33] = [
    "Beam from a Thorium Asteroid", "Callisto Penal Mines", "Energy Storage",
    "Giant Space Mirror", "Import of Advanced GHG", "Low-Atmo Shields",
    "Methane from Titan", "Natural Preserve", "New Portfolios",
    "Nitropholic Moss", "Nuclear Plants", "Power Plant",
    "Power Supply Consortium", "Quantum Extractor", "Rad Suits",
    "Slash and Burn Agriculture", "Solar Power", "Soletta",
    "Tectonic Stress Power", "Undersea Vents", "Lagrange Observatory",
    "Ice Cap Melting", "Permafrost Extraction", "Lake Mariners",
    "Technology Demonstration", "Trapped Heat", "Phobos Falls", "Trading Post",
    "Noctis Farming", "Mangrove", "Plantation", "Protected Valley",
    "Biothermal Power",
];

/// Les 4 cartes du groupe C (gain de forêt), avec le nombre de forêts imprimé.
const GROUPE_C: [(&str, i64); 4] = [
    ("Mangrove", 1), ("Plantation", 2), ("Protected Valley", 1),
    ("Biothermal Power", 1),
];

/// Productions inscrites sur les pistes : `(mc, chaleur, plantes, cartes)`.
fn pistes(r: &ProbeResult) -> (i64, i64, i64, i64) {
    (r.delta.mc_prod, r.delta.heat_prod, r.delta.plant_prod, r.delta.card_prod)
}

/// Tout ce qui se gagne À LA POSE :
/// `(mc, chaleur, plantes, main, NT, température, oxygène, océans, forêts)`.
fn immediat(r: &ProbeResult) -> (i64, i64, i64, i64, i64, i64, i64, i64, i64) {
    let d = &r.delta;
    (d.mc, d.heat, d.plants, d.hand, d.tr, d.temperature, d.oxygen, d.oceans, d.forests)
}

/// Carte de production PURE : les pistes attendues, et **rien** d'immédiat.
/// C'est le contrôle qui empêche de confondre « produit 3 chaleurs » (piste,
/// encaissée à chaque phase IV) avec « gagnez 3 chaleurs » (une seule fois).
fn production_seule(db: &CardsDb, name: &str, attendu: (i64, i64, i64, i64)) {
    let r = seq(db, &[name]);
    assert!(r.played, "{name} doit être posée");
    assert_eq!(pistes(&r), attendu, "{name} : pistes de production");
    assert_eq!(
        immediat(&r),
        (0, 0, 0, 0, 0, 0, 0, 0, 0),
        "{name} : une production n'est PAS un gain immédiat"
    );
}

// =========================================================================
// Groupe A — production seule (20 cartes)
//
// Attendu au format (mc_prod, heat_prod, plant_prod, card_prod).
// =========================================================================

#[test]
fn beam_from_a_thorium_asteroid_produces_one_plant_and_three_heat() {
    // « Requires a [jupiter]. During the production phase, this produces 1 plant
    //   and 3 heat. »
    let db = db();
    production_seule(&db, "Beam from a Thorium Asteroid", (0, 3, 1, 0));
    // Le prérequis existe : à l'état de départ de la sonde (aucun badge), il
    // n'est PAS rempli.
    assert!(!seq(&db, &["Beam from a Thorium Asteroid"]).prereq_ok, "1 badge Jupiter exigé");
}

#[test]
fn callisto_penal_mines_draws_a_card_in_the_production_phase() {
    // « During the production phase, draw a card. » → piste `card_prod`, pas
    // une pioche à la pose.
    let db = db();
    production_seule(&db, "Callisto Penal Mines", (0, 0, 0, 1));
}

#[test]
fn energy_storage_draws_two_cards_in_the_production_phase() {
    // « Requires you to have 7 or more TR. During the production phase, draw two
    //   cards. »
    let db = db();
    production_seule(&db, "Energy Storage", (0, 0, 0, 2));
}

#[test]
fn giant_space_mirror_produces_three_heat() {
    // « During the production phase, this produces 3 heat. »
    let db = db();
    production_seule(&db, "Giant Space Mirror", (0, 3, 0, 0));
}

#[test]
fn import_of_advanced_ghg_produces_two_heat() {
    // « During the production phase, this produces 2 heat. »
    let db = db();
    production_seule(&db, "Import of Advanced GHG", (0, 2, 0, 0));
}

#[test]
fn low_atmo_shields_produces_one_mc_and_two_heat() {
    // « Requires red oxygen or higher. …produces 1 MC and 2 heat. »
    let db = db();
    production_seule(&db, "Low-Atmo Shields", (1, 2, 0, 0));
    assert!(!seq(&db, &["Low-Atmo Shields"]).prereq_ok, "oxygène rouge exigé");
}

#[test]
fn methane_from_titan_produces_two_plants_and_two_heat() {
    // « Requires red oxygen or higher. …produces 2 plants and 2 heat. »
    let db = db();
    production_seule(&db, "Methane from Titan", (0, 2, 2, 0));
    assert!(!seq(&db, &["Methane from Titan"]).prereq_ok, "oxygène rouge exigé");
}

#[test]
fn natural_preserve_produces_two_mc() {
    // « Requires red oxygen or higher. …produces 2 MC. »
    let db = db();
    production_seule(&db, "Natural Preserve", (2, 0, 0, 0));
    assert!(!seq(&db, &["Natural Preserve"]).prereq_ok, "oxygène rouge exigé");
}

#[test]
fn new_portfolios_produces_one_mc_one_plant_and_one_heat() {
    // « …produces 1 MC, 1 plant, and 1 heat. »
    let db = db();
    production_seule(&db, "New Portfolios", (1, 1, 1, 0));
}

#[test]
fn nitropholic_moss_produces_two_plants() {
    // « …produces 2 plants. »
    let db = db();
    production_seule(&db, "Nitropholic Moss", (0, 0, 2, 0));
}

#[test]
fn nuclear_plants_produces_one_mc_and_three_heat() {
    // « …produces 1 MC and 3 heat. » Le −1 PV imprimé est une donnée de
    // `cards.json`, comptée au score : il ne s'encode pas ici.
    let db = db();
    production_seule(&db, "Nuclear Plants", (1, 3, 0, 0));
    assert_eq!(seq(&db, &["Nuclear Plants"]).vp, -1, "PV imprimé −1");
}

#[test]
fn power_plant_produces_one_heat() {
    // « During the production phase, this produces 1 heat. »
    let db = db();
    production_seule(&db, "Power Plant", (0, 1, 0, 0));
}

#[test]
fn power_supply_consortium_produces_two_mc_and_one_heat() {
    // « …produces 2 MC and 1 heat. » (l'encart imprimé montre « 2 » puis
    //  l'icône chaleur sans chiffre = 1.)
    let db = db();
    production_seule(&db, "Power Supply Consortium", (2, 1, 0, 0));
}

#[test]
fn quantum_extractor_produces_three_heat() {
    // « Requires 3 [science]. …produces 3 heat. »
    let db = db();
    production_seule(&db, "Quantum Extractor", (0, 3, 0, 0));
    assert!(!seq(&db, &["Quantum Extractor"]).prereq_ok, "3 badges Science exigés");
}

#[test]
fn rad_suits_produces_two_mc() {
    // « Requires 2 ocean tiles to be flipped. …produces 2 MC. »
    let db = db();
    production_seule(&db, "Rad Suits", (2, 0, 0, 0));
    assert!(!seq(&db, &["Rad Suits"]).prereq_ok, "2 océans exigés");
    // Deux océans révélés d'abord (Ice Asteroid) : le prérequis passe à l'état
    // courant. C'est le prérequis, pas un décor.
    let r = seq(&db, &["Ice Asteroid", "Rad Suits"]);
    assert_eq!(r.delta.oceans, 2);
    assert!(r.prereq_ok_now, "2 océans révélés : Rad Suits devient jouable");
}

#[test]
fn slash_and_burn_agriculture_produces_two_plants() {
    // « …produces 2 plants. » (PV imprimé −1.)
    let db = db();
    production_seule(&db, "Slash and Burn Agriculture", (0, 0, 2, 0));
    assert_eq!(seq(&db, &["Slash and Burn Agriculture"]).vp, -1, "PV imprimé −1");
}

#[test]
fn solar_power_produces_one_heat() {
    // « …produces 1 heat. »
    let db = db();
    production_seule(&db, "Solar Power", (0, 1, 0, 0));
}

#[test]
fn soletta_produces_five_heat() {
    // « …produces 5 heat. » — la plus grosse production de chaleur du lot.
    let db = db();
    production_seule(&db, "Soletta", (0, 5, 0, 0));
}

#[test]
fn tectonic_stress_power_produces_three_heat() {
    // « …produces 3 heat. »
    let db = db();
    production_seule(&db, "Tectonic Stress Power", (0, 3, 0, 0));
}

#[test]
fn undersea_vents_draws_a_card_and_produces_four_heat() {
    // « During the production phase, you draw a card and this produces 4 heat. »
    // La pioche est DANS la phase de production → `card_prod`, pas `Draw`.
    let db = db();
    production_seule(&db, "Undersea Vents", (0, 4, 0, 1));
}

// =========================================================================
// Groupe B — effet immédiat, éventuellement suivi d'une production (9 cartes)
//
// L'état de départ de la sonde révèle les tuiles océan NON mélangées :
// 1re = +2 plantes, 2e = +4 MC (ARCHITECTURE.md §Sonde). Un océan donne aussi
// +1 NT. Ces valeurs sont celles du moteur, pas du lot : elles servent
// seulement à lire le résultat.
// =========================================================================

#[test]
fn lagrange_observatory_draws_a_card_at_once() {
    // « [effect] Draw a card. » — une pioche IMMÉDIATE, pas une production.
    let db = db();
    let r = seq(&db, &["Lagrange Observatory"]);
    assert_eq!(r.delta.hand, 1, "une carte piochée à la pose");
    assert_eq!(pistes(&r), (0, 0, 0, 0), "aucune piste de production");
}

#[test]
fn ice_cap_melting_flips_one_ocean() {
    // « Requires white temperature. [effect] Flip an ocean tile. »
    let db = db();
    let r = seq(&db, &["Ice Cap Melting"]);
    assert_eq!(r.delta.oceans, 1, "un océan révélé");
    assert_eq!(r.delta.tr, 1, "un océan = 1 NT");
    assert_eq!(r.delta.plants, 2, "bonus de la 1re tuile de la sonde");
    assert_eq!(pistes(&r), (0, 0, 0, 0));
    assert!(!r.prereq_ok, "température blanche exigée");
}

#[test]
fn permafrost_extraction_flips_one_ocean() {
    // « Requires yellow temperature or warmer. [effect] Flip an ocean tile. »
    let db = db();
    let r = seq(&db, &["Permafrost Extraction"]);
    assert_eq!((r.delta.oceans, r.delta.tr), (1, 1));
    assert_eq!(pistes(&r), (0, 0, 0, 0));
    assert!(!r.prereq_ok, "température jaune ou plus exigée");
}

#[test]
fn lake_mariners_flips_two_oceans() {
    // « Requires yellow temperature or warmer. [effect] Flip 2 ocean tiles. »
    let db = db();
    let r = seq(&db, &["Lake Mariners"]);
    assert_eq!(r.delta.oceans, 2, "DEUX océans, pas un");
    assert_eq!(r.delta.tr, 2, "un NT par océan");
    // Bonus des deux premières tuiles de la sonde : +2 plantes puis +4 MC.
    assert_eq!((r.delta.plants, r.delta.mc), (2, 4));
    assert!(!r.prereq_ok, "température jaune ou plus exigée");
}

#[test]
fn technology_demonstration_flips_an_ocean_then_draws_two() {
    // « [effect] Flip an ocean tile. [effect] Draw two cards. »
    let db = db();
    let r = seq(&db, &["Technology Demonstration"]);
    assert_eq!((r.delta.oceans, r.delta.tr), (1, 1));
    assert_eq!(r.delta.hand, 2, "deux cartes piochées");
    assert_eq!(pistes(&r), (0, 0, 0, 0), "aucune production");
}

#[test]
fn trapped_heat_flips_an_ocean_and_then_produces_two_heat() {
    // « Requires red temperature or warmer. [effect] Flip an ocean tile. During
    //   the production phase, this produces 2 heat. » — les deux, pas l'un ou
    //   l'autre.
    let db = db();
    let r = seq(&db, &["Trapped Heat"]);
    assert_eq!((r.delta.oceans, r.delta.tr), (1, 1), "l'océan est immédiat");
    assert_eq!(pistes(&r), (0, 2, 0, 0), "la chaleur est une PRODUCTION");
    assert_eq!(r.delta.heat, 0, "aucune chaleur gagnée à la pose");
    assert!(!r.prereq_ok, "température rouge ou plus exigée");
}

#[test]
fn phobos_falls_raises_temperature_flips_an_ocean_and_draws_two() {
    // « [effect] Raise the temperature 1 step. [effect] Flip an ocean tile.
    //   [effect] Draw two cards. » — trois effets, dans cet ordre.
    let db = db();
    let r = seq(&db, &["Phobos Falls"]);
    assert_eq!(r.delta.temperature, 1);
    assert_eq!(r.delta.oceans, 1);
    assert_eq!(r.delta.hand, 2);
    assert_eq!(r.delta.tr, 2, "1 NT pour la température + 1 pour l'océan");
    assert_eq!(pistes(&r), (0, 0, 0, 0));
}

#[test]
fn trading_post_gains_three_plants_then_produces_two_mc() {
    // « [effect] Gain 3 plants. During the production phase, this produces 2 MC. »
    let db = db();
    let r = seq(&db, &["Trading Post"]);
    assert_eq!(r.delta.plants, 3, "3 plantes GAGNÉES, une seule fois");
    assert_eq!(pistes(&r), (2, 0, 0, 0), "et 2 MC de PRODUCTION");
    assert_eq!(r.delta.plant_prod, 0, "les 3 plantes ne sont pas une production");
}

#[test]
fn noctis_farming_gains_two_plants_then_produces_one_plant() {
    // « Requires red temperature or warmer. [effect] Gain 2 plants. During the
    //   production phase, this produces 1 plant. » — le piège du lot : deux
    //   plantes en gain, UNE en production. Ce ne sont pas les mêmes plantes.
    let db = db();
    let r = seq(&db, &["Noctis Farming"]);
    assert_eq!(r.delta.plants, 2, "gain immédiat de 2 plantes");
    assert_eq!(pistes(&r), (0, 0, 1, 0), "production de 1 plante");
    assert!(!r.prereq_ok, "température rouge ou plus exigée");
}

// =========================================================================
// Groupe C — gain de forêt (4 cartes)
// =========================================================================

#[test]
fn mangrove_gains_one_forest_and_one_oxygen_step() {
    // « Requires white temperature. [effect] Gain a forest VP and raise oxygen
    //   1 step. »
    let db = db();
    let r = seq(&db, &["Mangrove"]);
    assert_eq!(r.delta.forests, 1);
    assert_eq!(r.delta.oxygen, 1, "UN pas d'oxygène (R1)");
    assert_eq!(r.delta.tr, 1, "la hausse d'oxygène rapporte 1 NT");
    assert_eq!(pistes(&r), (0, 0, 0, 0), "aucune production");
    assert!(!r.prereq_ok, "température blanche exigée");
}

#[test]
fn plantation_gains_two_forests_and_two_oxygen_steps() {
    // « Requires 4 [science]. [effect] Gain 2 forest VPs and raise oxygen
    //   2 steps. » — 2 et 2. Une lecture cumulative donnerait 4 pas d'oxygène.
    let db = db();
    let r = seq(&db, &["Plantation"]);
    assert_eq!(r.delta.forests, 2);
    assert_eq!(r.delta.oxygen, 2, "DEUX pas d'oxygène");
    assert_ne!(r.delta.oxygen, 4, "JAMAIS le double (R1)");
    assert_eq!(r.delta.tr, 2);
    assert!(!r.prereq_ok, "4 badges Science exigés");
}

#[test]
fn protected_valley_gains_a_forest_then_produces_two_mc() {
    // « [effect] Gain a forest VP and raise oxygen 1 step. During the production
    //   phase, this produces 2 MC. »
    let db = db();
    let r = seq(&db, &["Protected Valley"]);
    assert_eq!((r.delta.forests, r.delta.oxygen), (1, 1));
    assert_eq!(pistes(&r), (2, 0, 0, 0));
}

#[test]
fn biothermal_power_gains_a_forest_then_produces_one_heat() {
    // « [effect] Gain a forest VP and raise oxygen 1 step. During the production
    //   phase, this produces 1 heat. »
    let db = db();
    let r = seq(&db, &["Biothermal Power"]);
    assert_eq!((r.delta.forests, r.delta.oxygen), (1, 1));
    assert_eq!(pistes(&r), (0, 1, 0, 0));
    assert_eq!(r.delta.heat, 0, "la chaleur est produite, pas gagnée");
}

// =========================================================================
// R1 — le gain de forêt lève l'oxygène UNE fois, pas deux
// =========================================================================

#[test]
fn r1_oxygen_steps_equal_forests_gained_never_double() {
    // Le rapport est de 1 pour 1 sur les QUATRE cartes du groupe C. Ce test
    // ÉCHOUE si le gain de forêt et la hausse d'oxygène sont lus comme deux
    // effets qui s'additionnent (Mangrove donnerait 2, Plantation 4).
    let db = db();
    for (name, forets) in GROUPE_C {
        let r = seq(&db, &[name]);
        assert_eq!(r.delta.forests, forets, "{name} : forêts");
        assert_eq!(r.delta.oxygen, forets, "{name} : UN pas d'oxygène par forêt");
        assert_ne!(r.delta.oxygen, 2 * forets, "{name} : DOUBLE COMPTAGE de l'oxygène");
    }
}

#[test]
fn r1_the_standard_forest_action_also_raises_oxygen_once() {
    // Le témoin de R1 : l'action standard payée, qui imprime la MÊME formule au
    // livret (p. 14 : « Dépenser 8 plantes pour gagner un PV Forêt et augmenter
    // l'oxygène d'un niveau »), lève l'oxygène une seule fois. La carte doit
    // faire exactement pareil — c'est tout l'argument de R1.
    let db = db();
    let mut pol = ActionForcer::forest_with_plants();
    let mut game = setup_game(&db, 4242, &mut pol);
    for p in 0..2 {
        game.players[p].plants = 8;
    }
    let (f0, o0, tr0) = (game.players[0].forests, game.oxygen as i64, game.players[0].tr);
    play_round(&mut game, &db, &mut pol);
    assert!(pol.forests_taken >= 1, "l'action forêt doit avoir été jouée au moins une fois");
    let df = game.players[0].forests - f0;
    assert!(df >= 1, "le joueur 0 a gagné au moins une forêt");
    // L'oxygène total monté par la phase est au plus le nombre de forêts des
    // DEUX joueurs — jamais le double. Comparaison volontairement large : le
    // point mesuré est l'absence de double comptage.
    let forets_totales = df + (game.players[1].forests);
    assert!(
        (game.oxygen as i64 - o0) <= forets_totales,
        "l'oxygène monte au plus d'un pas par forêt gagnée"
    );
    assert!(game.players[0].tr - tr0 >= df, "chaque pas d'oxygène rapporte 1 NT");
}

// =========================================================================
// R2 — le gain de forêt DÉCLENCHE « when you gain a forest VP »
// =========================================================================

/// Nombre d'animaux posés sur `Small Animals` après la séquence.
fn animaux(r: &ProbeResult) -> u32 {
    r.resources
        .iter()
        .find(|x| x.card == "Small Animals")
        .map(|x| x.n)
        .expect("Small Animals doit être en jeu")
}

#[test]
fn r2_a_card_forest_fires_the_gain_forest_vp_trigger() {
    // *Small Animals* imprime « Effect: When you gain a forest VP, add 1 animal
    // to this card. » — mot pour mot la formule du groupe C.
    let db = db();
    assert_eq!(animaux(&seq(&db, &["Small Animals", "Mangrove"])), 1);
}

#[test]
fn r2_the_trigger_fires_once_per_forest_gained() {
    // Livret l. 106 : « Si la condition d'un effet est remplie plusieurs fois …
    // résolvez l'effet correspondant plusieurs fois. » Plantation = 2 forêts.
    let db = db();
    assert_eq!(animaux(&seq(&db, &["Small Animals", "Plantation"])), 2);
    // Les deux autres cartes du groupe C, une forêt chacune.
    assert_eq!(animaux(&seq(&db, &["Small Animals", "Protected Valley"])), 1);
    assert_eq!(animaux(&seq(&db, &["Small Animals", "Biothermal Power"])), 1);
}

#[test]
fn r2_negative_witness_a_card_without_forest_fires_nothing() {
    // Témoin négatif : sans forêt, pas d'animal. Sans lui, un déclencheur qui
    // se lèverait à CHAQUE pose passerait les deux tests précédents.
    let db = db();
    for name in ["Power Plant", "Soletta", "Lagrange Observatory", "Trading Post"] {
        let r = seq(&db, &["Small Animals", name]);
        assert_eq!(animaux(&r), 0, "{name} ne gagne aucune forêt");
        assert_eq!(r.delta.forests, 0, "{name}");
    }
}

#[test]
fn r2_forest_animals_are_worth_victory_points() {
    // *Small Animals* : « 1 VP per 2 animals on this card. » Plantation en pose
    // 2 → 1 PV réel, lu sur `flow::card_points` (le chemin du score de partie).
    let db = db();
    let sans = seq(&db, &["Small Animals"]);
    let avec = seq(&db, &["Small Animals", "Plantation"]);
    assert_eq!(avec.vp_total - sans.vp_total, 1, "2 animaux = 1 PV");
}

#[test]
fn r2_the_oxygen_step_of_a_card_forest_also_fires_its_own_trigger() {
    // *Herbivores* : « When you raise oxygen, flip an ocean tile, or raise the
    // temperature, add 1 animal to this card. » La hausse d'oxygène d'un gain de
    // forêt est une hausse d'oxygène comme une autre — Plantation en fait deux.
    let db = db();
    let n = |r: &ProbeResult| {
        r.resources.iter().find(|x| x.card == "Herbivores").map(|x| x.n).expect("Herbivores")
    };
    assert_eq!(n(&seq(&db, &["Herbivores", "Mangrove"])), 1);
    assert_eq!(n(&seq(&db, &["Herbivores", "Plantation"])), 2);
}

// =========================================================================
// I2 — un SEUL chemin de gain de forêt
// =========================================================================

#[test]
fn i2_flow_writes_the_forest_counter_in_exactly_one_place() {
    // Contrôle STRUCTUREL : `flow.rs` ne doit contenir qu'une seule écriture de
    // `players[p].forests`. Un second chemin parallèle (qui ne lèverait ni
    // l'oxygène ni le déclencheur) se verrait ici, même s'il passait tous les
    // tests de comportement.
    let src = include_str!("../src/flow.rs");
    let n = src.matches("forests += 1").count();
    assert_eq!(n, 1, "un seul incrément de `forests` dans flow.rs, trouvé {n}");
    // Et cet incrément est bien dans `gain_forest`.
    let idx = src.find("forests += 1").expect("incrément trouvé");
    let avant = &src[..idx];
    let derniere_fn = avant.rfind("fn ").expect("une fonction englobante");
    assert!(
        avant[derniere_fn..].starts_with("fn gain_forest"),
        "l'incrément de `forests` doit vivre dans gain_forest"
    );
}

#[test]
fn i2_a_card_forest_and_the_paid_forest_have_the_same_shape() {
    // Une forêt de carte et une forêt d'action standard produisent le même
    // triplet (forêt, oxygène, NT) : c'est la signature du chemin unique.
    let db = db();
    let carte = seq(&db, &["Mangrove"]);
    assert_eq!(
        (carte.delta.forests, carte.delta.oxygen, carte.delta.tr),
        (1, 1, 1),
        "gain de forêt par carte"
    );
    // Le pendant payé, mesuré dans une partie réelle par l'action standard.
    let mut pol = ActionForcer::forest_with_plants();
    let mut game = setup_game(&db, 909, &mut pol);
    game.players[0].plants = 8;
    game.players[1].plants = 0;
    let avant = (game.players[0].forests, game.oxygen as i64, game.players[0].tr);
    play_round(&mut game, &db, &mut pol);
    let apres = (game.players[0].forests, game.oxygen as i64, game.players[0].tr);
    assert_eq!(apres.0 - avant.0, 1, "une forêt payée");
    assert_eq!(apres.1 - avant.1, 1, "un pas d'oxygène, comme la carte");
    assert_eq!(apres.2 - avant.2, 1, "un NT, comme la carte");
}

#[test]
fn i2_a_card_forest_costs_no_plants_and_gets_no_rebate() {
    // La remise d'Ecoline porte sur « lorsque vous DÉPENSEZ DES PLANTES pour
    // gagner un jeton PV Forêt » : une forêt offerte par une carte ne dépense
    // rien, donc ne remise rien. Le paiement reste hors de `gain_forest`.
    let db = db();
    let r = seq(&db, &["Mangrove"]);
    assert_eq!(r.delta.plants, 0, "aucune plante dépensée ni gagnée");
    assert_eq!(r.delta.forests, 1);
    assert_eq!(r.paid, vec![12], "seul le prix imprimé de la carte est payé");
}

// =========================================================================
// Req::TrMin — le seuil de NT (divergence déclarée, journal D1)
// =========================================================================

#[test]
fn trmin_blocks_the_card_below_the_threshold() {
    // « Requires you to have 7 or more TR. » L'état de départ de la sonde donne
    // 5 NT : le prérequis n'est PAS rempli.
    let db = db();
    let r = seq(&db, &["Energy Storage"]);
    assert!(!r.prereq_ok, "5 NT < 7 : le prérequis doit bloquer");
    assert!(!r.prereq_ok_now, "et à l'état courant aussi");
}

#[test]
fn trmin_is_satisfied_once_the_tr_threshold_is_reached() {
    // Deux cartes qui montent le NT de 2 chacune (« Raise your TR 2 steps ») :
    // 5 → 7. Le seuil se juge à l'ÉTAT COURANT, pas sur l'instantané de début
    // de phase — le NT n'est pas un paramètre planétaire.
    let db = db();
    let r = seq(&db, &["Release of Inert Gases", "Energy Storage"]);
    assert_eq!(r.delta.tr, 2, "le NT est monté à 7");
    assert!(r.prereq_ok_now, "7 NT : Energy Storage devient jouable");
    // (lot cartes-7, journal D2) La dernière assertion disait « …mais pas à
    // l'état de DÉPART de la sonde (5 NT) ». Elle était vraie pour une raison
    // de SONDE, pas de règle : `prereq_ok` était relevé avant la pose de la
    // première carte de la séquence. Il l'est désormais juste avant la pose de
    // la DERNIÈRE, comme `flow::affordable` le fait en partie réelle.
    //
    // L'assertion n'est pas supprimée, elle est RETOURNÉE et devient plus
    // exigeante : `TrMin` est une ressource de JOUEUR, jamais un paramètre
    // planétaire, donc les deux lectures — instantané et état courant — doivent
    // désormais TOMBER D'ACCORD. Si `TrMin` était (à tort) jugé sur
    // l'instantané de début de phase, celle-ci échouerait.
    assert!(
        r.prereq_ok,
        "TrMin se juge à l'état courant : les deux lectures doivent s'accorder"
    );
}

#[test]
fn trmin_never_spends_the_tr_it_requires() {
    // Le piège : `SpendTr` DÉPENSE le NT. `TrMin` ne fait que le tester. La pose
    // d'Energy Storage ne doit rien retirer.
    let db = db();
    let r = seq(&db, &["Energy Storage"]);
    assert_eq!(r.delta.tr, 0, "Requires ≠ Spend : aucun NT dépensé");
    // Contre-témoin : une carte qui, elle, dépense vraiment 1 NT.
    let spend = seq(&db, &["Investment Loan"]);
    assert_eq!(spend.delta.tr, -1, "SpendTr retire bien 1 NT");
}

#[test]
fn trmin_actually_gates_the_card_in_strict_mode() {
    // En mode strict la sonde ne force plus la pose : Energy Storage est
    // refusée à 5 NT. C'est le prérequis qui agit, pas un affichage.
    let db = db();
    let opts = ProbeOptions { strict: true, ..ProbeOptions::default() };
    let r = run_probe_seq_full(&db, &["Energy Storage"], opts, &ProbeScript::default(), false);
    assert!(!r.played, "prérequis non rempli : la carte n'est pas posée");
    assert_eq!(r.delta.card_prod, 0, "et sa production n'est pas inscrite");
    // La même carte précédée de deux hausses de NT passe.
    let r2 = run_probe_seq_full(
        &db,
        &["Release of Inert Gases", "Bribed Comittee", "Energy Storage"],
        opts,
        &ProbeScript::default(),
        false,
    );
    assert!(r2.played, "9 NT : la carte passe en mode strict");
    assert_eq!(r2.delta.card_prod, 2, "et sa production est inscrite");
}

// =========================================================================
// Les productions sont ENCAISSÉES par la vraie phase IV, à chaque génération
// =========================================================================

#[test]
fn the_production_tracks_are_credited_by_the_real_production_phase() {
    // `--probe-produce` exécute la VRAIE phase IV (`flow::phase_production`).
    // La chaleur et les plantes n'ont aucune autre source dans cette phase.
    let db = db();
    for (name, heat, plants) in [
        ("Soletta", 5, 0), ("Power Plant", 1, 0), ("Nitropholic Moss", 0, 2),
        ("Methane from Titan", 2, 2), ("New Portfolios", 1, 1),
    ] {
        let r = produce(&db, &[name]);
        assert!(r.produced, "{name} : la phase IV doit avoir tourné");
        assert_eq!(r.delta.heat, heat, "{name} : chaleur encaissée");
        assert_eq!(
            r.delta.plants - seq(&db, &[name]).delta.plants,
            plants,
            "{name} : plantes encaissées par la production"
        );
    }
}

#[test]
fn card_production_draws_only_during_the_production_phase() {
    // « During the production phase, draw a card / two cards » : rien à la pose,
    // tout à la production. C'est la différence avec `Eff::Draw`.
    let db = db();
    for (name, n) in [("Callisto Penal Mines", 1), ("Energy Storage", 2), ("Undersea Vents", 1)] {
        assert_eq!(seq(&db, &[name]).delta.hand, 0, "{name} : rien piochée à la pose");
        assert_eq!(produce(&db, &[name]).delta.hand, n, "{name} : {n} carte(s) en production");
    }
    // Le contraste : Lagrange Observatory pioche à la POSE, et sa main ne
    // grossit pas davantage à la production.
    assert_eq!(seq(&db, &["Lagrange Observatory"]).delta.hand, 1);
    assert_eq!(produce(&db, &["Lagrange Observatory"]).delta.hand, 1);
}

#[test]
fn the_production_track_pays_out_at_every_generation_not_once() {
    // Partie RÉELLE, deux phases IV séparées par une phase V (le livret interdit
    // de rejouer la même phase deux manches de suite). Une production inscrite
    // sur la piste doit payer AUTANT la seconde fois que la première.
    let db = db();
    let mut pol = PhaseForcer::new(&[4, 5, 4]);
    let mut game = setup_game(&db, 909, &mut pol);
    let soletta = db.resolve_card("Soletta").expect("Soletta résolue");
    game.players[0].hand.clear();
    game.players[0].hand.push(soletta);
    game.players[0].mc = 1000;
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.players[0].heat_prod, 5, "la piste porte 5 chaleurs");

    let h0 = game.players[0].heat;
    play_round(&mut game, &db, &mut pol);
    let h1 = game.players[0].heat;
    assert_eq!(h1 - h0, 5, "1re phase IV : +5 chaleurs");
    play_round(&mut game, &db, &mut pol); // phase V : aucune production
    assert_eq!(game.players[0].heat, h1, "une manche sans phase IV ne produit rien");
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].heat - h1, 5, "2e phase IV : +5 encore, ni gel ni cumul");
}

// =========================================================================
// Non-régression, recensement, cadrage
// =========================================================================

#[test]
fn the_thirty_three_cards_resolve_to_the_base_box_deck() {
    let db = db();
    for name in LOT5 {
        let id = db.resolve_card(name).unwrap_or_else(|| panic!("{name} non résolue"));
        let card = &db.projects[id as usize];
        assert!(card.in_deck_v1, "{name} doit venir du deck v1");
        assert!(card.effect.is_some(), "{name} doit être encodée");
        assert!(card.effets_geres(), "{name} doit être déclarée gérée");
    }
    // 110 (lots 1-2) + 28 (ressources) + 17 (lot 4) + 33 (lot 5) + 11 (lot 6)
    // + 4 (lot acier-titane) + 9 (lot cartes-7) + 5 (lot cartes-8)
    // + 28 (decouverte-projets) + 3 (jokers-corpos) = 248. ATTENTE MISE À JOUR
    // par le lot 6, le lot acier-titane, le lot cartes-7, le lot cartes-8,
    // `decouverte-projets` puis `jokers-corpos` : taille EXACTE toujours
    // épinglée.
    assert_eq!(engine::effects::LOT1.len(), 248);
}

#[test]
fn the_thirty_three_prices_match_the_printed_cost() {
    // Contrôle croisé indépendant de l'encodage : le prix réellement payé par la
    // sonde doit être le coût IMPRIMÉ relevé sur les cartons
    // (`textes-cartes.json`, champ `cost`). Si une entrée résolvait vers un
    // homonyme (piège « Buffed »), le prix trahirait la substitution.
    let db = db();
    for (name, cout) in [
        ("Beam from a Thorium Asteroid", 23), ("Callisto Penal Mines", 20),
        ("Energy Storage", 18), ("Giant Space Mirror", 13),
        ("Import of Advanced GHG", 8), ("Low-Atmo Shields", 9),
        ("Methane from Titan", 35), ("Natural Preserve", 12),
        ("New Portfolios", 14), ("Nitropholic Moss", 14), ("Nuclear Plants", 10),
        ("Power Plant", 3), ("Power Supply Consortium", 12),
        ("Quantum Extractor", 16), ("Rad Suits", 4),
        ("Slash and Burn Agriculture", 8), ("Solar Power", 10), ("Soletta", 30),
        ("Tectonic Stress Power", 20), ("Undersea Vents", 31),
        ("Lagrange Observatory", 7), ("Ice Cap Melting", 4),
        ("Permafrost Extraction", 8), ("Lake Mariners", 17),
        ("Technology Demonstration", 17), ("Trapped Heat", 20),
        ("Phobos Falls", 32), ("Trading Post", 11), ("Noctis Farming", 13),
        ("Mangrove", 12), ("Plantation", 22), ("Protected Valley", 22),
        ("Biothermal Power", 18),
    ] {
        assert_eq!(seq(&db, &[name]).paid, vec![cout], "{name} : coût imprimé");
    }
}

#[test]
fn effects_off_leaves_the_thirty_three_completely_inert() {
    // Interrupteur `--effects off` : squelette intégral. Aucune des 33 ne doit
    // changer quoi que ce soit à l'état.
    let mut db = CardsDb::load(CARDS).expect("chargement");
    db.effects_on = false;
    for name in LOT5 {
        let r = seq(&db, &[name]);
        assert!(!r.in_lot, "{name} : hors lot quand les effets sont coupés");
        assert_eq!(
            (pistes(&r), immediat(&r)),
            ((0, 0, 0, 0), (0, 0, 0, 0, 0, 0, 0, 0, 0)),
            "{name} change l'état alors que les effets sont coupés"
        );
    }
}

#[test]
fn no_base_card_remains_unhandled() {
    // D2 vu du moteur : le champ `effets_geres` est DÉRIVÉ de l'encodage. Les
    // restantes sont celles qui réclament des mécanismes absents.
    //
    // ATTENTE MISE À JOUR par le lot 6 (29 → 18) : les 11 cartes du lot 6 ont
    // été encodées, elles ne sont donc plus muettes. Le test n'est ni supprimé
    // ni assoupli — il continue d'épingler le nombre EXACT et la liste EXACTE.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let muettes: std::collections::BTreeSet<&str> = db
        .recensement()
        .into_iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    // ATTENTE MISE À JOUR par le lot acier-titane (18 → 14), par le lot
    // cartes-7 (14 → 5), puis par le lot cartes-8 (5 → 0) : les cinq dernières
    // — les poses supplémentaires — sont encodées. **La boîte de base est
    // intégralement encodée.**
    //
    // Le test n'est pas vidé de sa substance : l'assertion « exactement zéro »
    // est la plus forte qu'il puisse porter, et la MOINDRE régression qui
    // rendrait une carte muette la fait échouer en la nommant.
    assert!(
        muettes.is_empty(),
        "la boîte de base doit être intégralement encodée ; encore muettes : {muettes:?}"
    );
    // Et aucune des 33 n'y figure.
    for name in LOT5 {
        assert!(!muettes.contains(name), "{name} ne doit plus être muette");
    }
}

#[test]
fn the_unhandled_counter_drops_in_real_games() {
    // Le compteur `cards_effects_unhandled` est incrémenté à la POSE d'une carte
    // sans encodage (`flow::build_card_with`), pas dérivé du recensement : c'est
    // un oracle disjoint. Une partie réelle en pose forcément moins qu'avant.
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 2024, &mut pol);
    for _ in 0..12 {
        play_round(&mut game, &db, &mut pol);
    }
    let poses: u64 = (0..2).map(|p| game.players[p].played.len() as u64).sum();
    assert!(poses > 0, "des cartes ont été posées");
    assert!(
        (game.cards_effects_unhandled as u64) < poses,
        "moins d'un pouvoir sauté par carte posée ({} sautés pour {poses} poses)",
        game.cards_effects_unhandled
    );
}

#[test]
fn i4_no_card_name_of_this_lot_appears_in_the_game_flow() {
    // « Ne jamais écrire un nom de carte dans le flux de jeu. » Les noms vivent
    // dans la table de données `effects::LOT1`, nulle part ailleurs.
    for (fichier, src) in [
        ("flow.rs", include_str!("../src/flow.rs")),
        ("cards.rs", include_str!("../src/cards.rs")),
        ("state.rs", include_str!("../src/state.rs")),
        ("policy.rs", include_str!("../src/policy.rs")),
        ("sim.rs", include_str!("../src/sim.rs")),
        ("probe.rs", include_str!("../src/probe.rs")),
    ] {
        for name in LOT5 {
            assert!(
                !src.contains(name),
                "le nom « {name} » ne doit pas figurer dans src/{fichier}"
            );
        }
    }
}

#[test]
fn the_new_encodings_are_deterministic() {
    // Deux sondes identiques donnent exactement le même résultat : les gains de
    // forêt, les océans et les pioches n'introduisent aucun aléa non maîtrisé.
    let db = db();
    for names in [
        &["Plantation"][..],
        &["Small Animals", "Plantation"][..],
        &["Phobos Falls"][..],
        &["Lake Mariners"][..],
    ] {
        let a = seq(&db, names);
        let b = seq(&db, names);
        assert_eq!(a.delta, b.delta, "{names:?}");
        assert_eq!(a.paid, b.paid, "{names:?}");
        assert_eq!(a.resources, b.resources, "{names:?}");
    }
}

#[test]
fn the_lot_did_not_touch_the_boxes_composition() {
    // Encoder n'est pas composer : les effectifs de pioche sont inchangés.
    for (liste, projets, corps) in [
        ("base", 208, 12),
        ("base,promo", 219, 12),
        ("base,decouverte", 246, 16),
        ("base,promo,decouverte", 257, 16),
    ] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap()).expect(liste);
        assert_eq!(db.deck_project_count, projets, "projets de --boites {liste}");
        assert_eq!(db.corporations.len(), corps, "corporations de --boites {liste}");
    }
}

#[test]
fn only_discovery_cards_remain_unhandled() {
    // Le décompte porte
    // sur le RECENSEMENT complet : les 37 de Découverte sont 33 projets plus
    // 4 corporations sans encodage — d'où l'écart avec le contrôle
    // `02-les-14-restantes.sh`, qui ne compte que les projets (47).
    // ATTENTE MISE À JOUR par le lot 6 (66 → 55), par le lot acier-titane
    // (55 → 51), par le lot cartes-7 (51 → 42) puis par le lot cartes-8
    // (42 → 37) : les 5 dernières cartes encodées sont toutes de la boîte de
    // base, Découverte n'a pas bougé. Il ne reste donc QUE Découverte —
    // 33 projets + 4 corporations = 37, et plus une seule carte de base.
    //
    // TÉMOIN RETOURNÉ par le chantier `decouverte-phases` (37 → 35), sans
    // qu'une seule carte ait été encodée : *Cryogenic Shipment* et *Fibrous
    // Composite Material* étaient comptées muettes parce que leur encodage
    // portait un `ResEff::PhaseUpgrade` que le moteur reconnaissait SAUTER
    // (`cards::encodage_integral`). Le mécanisme existe désormais, l'effet
    // n'est plus sauté : leur texte imprimé est appliqué en entier, elles ne
    // sont plus muettes. Reste 31 projets + 4 corporations = 35.
    //
    // TÉMOIN RETOURNÉ une seconde fois par `decouverte-projets` (35 → 7) : les
    // 28 derniers projets muets de l'extension sont encodés. Il ne restait que
    // les 3 projets à badge JOKER (*Local Market*, *Political Influence*,
    // *Topographic Mapping*) et les 4 corporations de Découverte — 3 + 4 = 7.
    //
    // TÉMOIN RETOURNÉ une TROISIÈME fois par `jokers-corpos` (7 → 0) : ces
    // sept-là sont encodées à leur tour. Le NOM du test est devenu faux — plus
    // aucune carte de Découverte n'est muette — mais il garde tout son objet :
    // il épingle un nombre EXACT, et ce nombre est désormais ZÉRO. Il n'est ni
    // supprimé ni assoupli ; il devient au contraire plus strict, et il nomme
    // toujours les sept dernières pour dire qu'elles sont, elles aussi, gérées.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte");
    let muettes: Vec<&str> = db
        .recensement()
        .into_iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    assert_eq!(muettes.len(), 0, "muettes en base + Découverte : {muettes:?}");
    for nom in [
        "Local Market",
        "Political Influence",
        "Topographic Mapping",
        "Apollo Industries",
        "Exocorp",
        "Hyperion Systems",
        "Sultira",
    ] {
        let c = db
            .recensement()
            .into_iter()
            .find(|c| c.name == nom)
            .unwrap_or_else(|| panic!("{nom} absente du recensement"));
        assert!(c.effets_geres, "{nom} : son texte imprimé est appliqué");
    }
    // Et la raison exacte : les deux cartes à « améliorez une carte Phase »
    // sont désormais intégralement gérées.
    for nom in ["Cryogenic Shipment", "Fibrous Composite Material"] {
        let c = db
            .recensement()
            .into_iter()
            .find(|c| c.name == nom)
            .unwrap_or_else(|| panic!("{nom} absente du recensement"));
        assert!(c.effets_geres, "{nom} : son amélioration de phase est appliquée");
    }
}

#[test]
fn a_thousand_games_still_run_clean_with_the_new_encodings() {
    // Non-régression de bout en bout : les gains de forêt de carte ne cassent ni
    // les invariants (dont la comptabilité du NT), ni la terminaison.
    let db = db();
    let mut pol = RandomPolicy;
    let out = engine::sim::run_simulation(&db, 300, 2024, &mut pol);
    assert_eq!(out.invariant_violations, 0, "aucune violation d'invariant");
    assert_eq!(out.truncated, 0, "aucune partie tronquée");
    assert_eq!(out.completed, 300, "toutes les parties achevées");
}

// =========================================================================
// Politiques de test — le flux reste celui du moteur
// =========================================================================

/// Politique qui force une phase donnée pour les deux joueurs, manche par
/// manche. Tout le reste est délégué à `RandomPolicy`.
struct PhaseForcer {
    inner: RandomPolicy,
    phases: Vec<u8>,
    calls: usize,
}

impl PhaseForcer {
    fn new(phases: &[u8]) -> PhaseForcer {
        PhaseForcer { inner: RandomPolicy, phases: phases.to_vec(), calls: 0 }
    }
}

impl Policy for PhaseForcer {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        let want = self.phases[(self.calls / 2) % self.phases.len()];
        self.calls += 1;
        if allowed.contains(&want) {
            want
        } else {
            self.inner.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.inner.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.inner.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

/// Politique qui force la phase III et, dans cette phase, choisit l'action forêt
/// payée en plantes chaque fois qu'elle est offerte — puis passe. Elle observe
/// l'action standard telle que le moteur la propose : rien n'est fabriqué.
struct ActionForcer {
    inner: RandomPolicy,
    forests_taken: usize,
}

impl ActionForcer {
    fn forest_with_plants() -> ActionForcer {
        ActionForcer { inner: RandomPolicy, forests_taken: 0 }
    }
}

impl Policy for ActionForcer {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&3) {
            3
        } else {
            self.inner.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, o: &[ActionOpt]) -> Option<usize> {
        // L'action forêt payée en plantes si elle est offerte, sinon passer.
        match o.iter().position(|a| matches!(a, ActionOpt::ForestWithPlants)) {
            Some(i) => {
                self.forests_taken += 1;
                Some(i)
            }
            None => None,
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

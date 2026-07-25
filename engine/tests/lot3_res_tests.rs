//! Tests du lot 3 « ressources posées sur les cartes » (chantier cartes-3).
//!
//! Un test par carte du lot (28) vérifiant l'ÉTAT DE JEU résultant contre le
//! TEXTE IMPRIMÉ — pas contre la table d'encodage — plus des tests
//! d'intégration : service unique, tri et complétude du champ `resources`,
//! choix passés à la politique, cible imposée absente, absence de cible,
//! points de victoire réels au SCORE de partie, compteurs d'audit en flux
//! réel, interrupteur `--effects off`, déterminisme, et piège des classes
//! « Buffed ».
//!
//! Sondes utilisées : `run_probe_seq_scripted` (pose forcée d'une séquence,
//! réponses de politique imposées) et `run_probe_action_scripted` (pose puis
//! activation de l'action). Les deux empruntent le chemin de pose de
//! `simulate` (`flow::build_card_with`).

use engine::cards::CardsDb;
use engine::flow::{build_card, play_round, score, setup_game};
use engine::policy::{ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{
    run_probe_action_scripted, run_probe_seq_scripted, ProbeOptions, ProbeResult, ProbeScript,
};
use engine::sim::run_simulation;
use rand::rngs::StdRng;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

fn script(choices: &[usize], targets: &[&str]) -> ProbeScript {
    ProbeScript {
        choices: choices.to_vec(),
        targets: targets.iter().map(|s| s.to_string()).collect(),
    }
}

/// Sonde séquence scriptée.
fn seq(db: &CardsDb, names: &[&str], choices: &[usize], targets: &[&str]) -> ProbeResult {
    run_probe_seq_scripted(db, names, ProbeOptions::default(), &script(choices, targets))
}

/// Ressources posées sur `card` après la sonde ; `None` si la carte n'est pas
/// une porteuse en jeu.
fn res(r: &ProbeResult, card: &str) -> Option<u32> {
    r.resources.iter().find(|x| x.card == card).map(|x| x.n)
}

fn kind_of(r: &ProbeResult, card: &str) -> Option<&'static str> {
    r.resources.iter().find(|x| x.card == card).map(|x| x.kind)
}

// =========================================================== A. les conteneurs

#[test]
fn tardigrades_action_adds_one_microbe() {
    let db = db();
    // « Action: Add 1 microbe to this card. »
    let r = run_probe_action_scripted(&db, "Tardigrades", &ProbeScript::default());
    assert!(r.in_lot && r.has_action && r.action_applied);
    assert_eq!(r.resources.len(), 1);
    assert_eq!((r.resources[0].kind, r.resources[0].n), ("microbe", 1));
}

#[test]
fn tardigrades_vp_is_one_per_three_microbes_integer_division() {
    let db = db();
    // « 1 VP per 3 microbes on this card. » — division ENTIÈRE.
    // CEO's Favorite Project pose 2 ressources : 2/3 = 0 VP.
    let two = seq(&db, &["Tardigrades", "CEO's Favorite Project"], &[], &["Tardigrades"]);
    assert_eq!(res(&two, "Tardigrades"), Some(2));
    assert_eq!(two.vp, 0);
    // Imported Nitrogen pose 3 microbes : 3/3 = 1 VP.
    let three = seq(&db, &["Tardigrades", "Imported Nitrogen"], &[], &["Tardigrades"]);
    assert_eq!(res(&three, "Tardigrades"), Some(3));
    assert_eq!(three.vp, 1);
}

#[test]
fn birds_requires_white_oxygen_and_its_action_adds_an_animal() {
    let db = db();
    // « Requires white oxygen. » — faux à l'oxygène de départ (0 %).
    let posed = seq(&db, &["Birds"], &[], &[]);
    assert!(!posed.prereq_ok, "Birds exige l'oxygène BLANC");
    // Pose : 0 animal (le Java n'appelle que initResources) ; l'ajout est l'action.
    assert_eq!(res(&posed, "Birds"), Some(0));
    let r = run_probe_action_scripted(&db, "Birds", &ProbeScript::default());
    assert!(r.action_applied);
    assert_eq!(r.resources[0].n, 1);
    // « 1 VP per animal on this card. »
    let two = seq(&db, &["Birds", "CEO's Favorite Project"], &[], &["Birds"]);
    assert_eq!((res(&two, "Birds"), two.vp), (Some(2), 2));
}

#[test]
fn nitrite_reducting_bacteria_places_3_then_action_branches() {
    let db = db();
    // « Add 3 microbes to this card. »
    assert_eq!(res(&seq(&db, &["Nitrite Reducting Bacteria"], &[], &[]),
                   "Nitrite Reducting Bacteria"), Some(3));
    // Branche 0 : « Add 1 microbe to this card. »
    let add = run_probe_action_scripted(&db, "Nitrite Reducting Bacteria", &script(&[0], &[]));
    assert!(add.action_applied);
    assert_eq!(add.resources[0].n, 4);
    assert_eq!(add.delta.oceans, 0);
    // Branche 1 : « remove 3 microbes to flip an ocean tile. »
    let flip = run_probe_action_scripted(&db, "Nitrite Reducting Bacteria", &script(&[1], &[]));
    assert!(flip.action_applied);
    assert_eq!(flip.resources[0].n, 0);
    assert_eq!(flip.delta.oceans, 1);
}

#[test]
fn fibrous_composite_material_holds_science_and_skips_phase_upgrade() {
    let db = db();
    // « Add 3 science resources to this card. »
    let posed = seq(&db, &["Fibrous Composite Material"], &[], &[]);
    assert_eq!(kind_of(&posed, "Fibrous Composite Material"), Some("science"));
    assert_eq!(res(&posed, "Fibrous Composite Material"), Some(3));
    // Branche 0 : +1 science.
    let add = run_probe_action_scripted(&db, "Fibrous Composite Material", &script(&[0], &[]));
    assert_eq!(add.resources[0].n, 4);
    // Branche 1 : −3 science pour améliorer une phase — mécanisme non géré,
    // l'amélioration est PERDUE, sans la moindre compensation (NEVER 5).
    let up = run_probe_action_scripted(&db, "Fibrous Composite Material", &script(&[1], &[]));
    assert!(up.action_applied);
    assert_eq!(up.resources[0].n, 0);
    assert_eq!(
        (up.delta.mc, up.delta.hand, up.delta.tr, up.delta.plants, up.delta.heat),
        (0, 0, 0, 0, 0),
        "aucune compensation de l'amélioration de phase non gérée"
    );
}

#[test]
fn decomposing_fungus_places_2_and_action_trades_a_resource_for_plants() {
    let db = db();
    assert_eq!(res(&seq(&db, &["Decomposing Fungus"], &[], &[]), "Decomposing Fungus"),
               Some(2));
    // « Action: Remove 1 animal or 1 microbe from one of your cards to gain
    //   3 plants. » (source choisie par la politique : ici la carte elle-même,
    //   seule porteuse.)
    let r = run_probe_action_scripted(&db, "Decomposing Fungus", &ProbeScript::default());
    assert!(r.action_applied);
    assert_eq!(r.resources[0].n, 1);
    assert_eq!(r.delta.plants, 3);
}

#[test]
fn ghg_production_bacteria_requires_red_oxygen_and_filters_unplayable_branch() {
    let db = db();
    let posed = seq(&db, &["GHG Production Bacteria"], &[], &[]);
    assert!(!posed.prereq_ok, "exige l'oxygène rouge ou plus");
    assert_eq!(res(&posed, "GHG Production Bacteria"), Some(0));
    // À 0 microbe, la branche « remove 2 microbes » est INJOUABLE : elle est
    // filtrée avant le choix, la branche 0 s'applique même si on impose « 1 ».
    let r = run_probe_action_scripted(&db, "GHG Production Bacteria", &script(&[1], &[]));
    assert!(r.action_applied);
    assert_eq!(r.resources[0].n, 1);
    assert_eq!(r.delta.temperature, 0);
}

#[test]
fn regolith_eaters_requires_red_temperature_and_action_adds_microbe() {
    let db = db();
    let posed = seq(&db, &["Regolith Eaters"], &[], &[]);
    assert!(!posed.prereq_ok, "exige la température rouge ou plus");
    let r = run_probe_action_scripted(&db, "Regolith Eaters", &ProbeScript::default());
    assert!(r.action_applied);
    assert_eq!(r.resources[0].n, 1);
}

#[test]
fn fish_gains_an_animal_per_ocean_flipped() {
    let db = db();
    // « When you flip an ocean tile, add 1 animal to this card. »
    // Ice Asteroid révèle 2 océans.
    let r = seq(&db, &["Fish", "Ice Asteroid"], &[], &[]);
    assert_eq!(r.delta.oceans, 2);
    assert_eq!(res(&r, "Fish"), Some(2));
    // « 1 VP per animal on this card. »
    assert_eq!(r.vp, 2);
}

#[test]
fn livestock_gains_an_animal_per_temperature_step() {
    let db = db();
    // « When you raise the temperature, add 1 animal to this card. »
    // Lava Flows : température +2 pas.
    let r = seq(&db, &["Livestock", "Lava Flows"], &[], &[]);
    assert_eq!(r.delta.temperature, 2);
    assert_eq!(res(&r, "Livestock"), Some(2));
}

#[test]
fn small_animals_gains_an_animal_per_forest_built() {
    let db = db();
    // « When you build a forest, add 1 animal to this card. » La forêt n'est
    // pas un effet de carte : ce test passe par le FLUX RÉEL (phase III et sa
    // conversion obligatoire de fin de phase).
    let mut pol = PhaseOnly::new(3);
    let mut game = setup_game(&db, 11, &mut pol);
    let sa = db.resolve_card("Small Animals").unwrap();
    game.deck.retain(|&c| c != sa);
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    game.players[0].hand.push(sa);
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.players[0].resources_on(sa), 0, "posée vide");
    // 8 plantes : la règle de conversion obligatoire construira une forêt.
    game.players[0].plants = 8;
    game.players[1].plants = 0;
    game.players[1].heat = 0;
    play_round(&mut game, &db, &mut pol);
    assert!(game.players[0].forests >= 1, "une forêt a bien été construite");
    assert_eq!(
        game.players[0].resources_on(sa) as i64,
        game.players[0].forests,
        "1 animal par forêt construite"
    );
}

#[test]
fn herbivores_gains_on_oxygen_ocean_and_temperature() {
    let db = db();
    // « Requires 5 oceans to be flipped. »
    assert!(!seq(&db, &["Herbivores"], &[], &[]).prereq_ok);
    // Towing a Comet : oxygène +1, océan +1 → 2 animaux.
    let a = seq(&db, &["Herbivores", "Towing a Comet"], &[], &[]);
    assert_eq!(res(&a, "Herbivores"), Some(2));
    // Lava Flows : température +2 → 2 animaux.
    let b = seq(&db, &["Herbivores", "Lava Flows"], &[], &[]);
    assert_eq!(res(&b, "Herbivores"), Some(2));
    // « 1 VP per 2 animals » : 2/2 = 1.
    assert_eq!(b.vp, 1);
}

#[test]
fn physics_complex_gains_science_per_temperature_step() {
    let db = db();
    assert!(!seq(&db, &["Physics Complex"], &[], &[]).prereq_ok, "4 tags Science");
    let r = seq(&db, &["Physics Complex", "Lava Flows"], &[], &[]);
    assert_eq!(kind_of(&r, "Physics Complex"), Some("science"));
    assert_eq!(res(&r, "Physics Complex"), Some(2));
    assert_eq!(r.vp, 1, "1 VP par 2 ressources science");
}

#[test]
fn ecological_zone_gains_one_animal_per_animal_or_plant_tag() {
    let db = db();
    // « When you play a Animal or Plant, including these » — ses propres tags
    // PLANT + ANIMAL = 2 tags concernés → 2 animaux à sa pose.
    let alone = seq(&db, &["Ecological Zone"], &[], &[]);
    assert_eq!(res(&alone, "Ecological Zone"), Some(2));
    // Lichen (1 tag PLANT) → +1.
    let then = seq(&db, &["Ecological Zone", "Lichen"], &[], &[]);
    assert_eq!(res(&then, "Ecological Zone"), Some(3));
    // Lava Flows (EVENT seul) → aucun gain.
    let none = seq(&db, &["Ecological Zone", "Lava Flows"], &[], &[]);
    assert_eq!(res(&none, "Ecological Zone"), Some(2));
}

#[test]
fn anaerobic_microorganisms_gains_one_microbe_per_animal_microbe_plant_tag() {
    let db = db();
    // Son propre tag MICROBE → 1 microbe à la pose.
    let alone = seq(&db, &["Anaerobic Microorganisms"], &[], &[]);
    assert_eq!(res(&alone, "Anaerobic Microorganisms"), Some(1));
    // Conserved Biome : tags BUILDING + MICROBE + ANIMAL → 2 tags concernés.
    // (round 2 : sa propre pose n'ajoute plus rien, c'est devenu une action.)
    let more = seq(&db, &["Anaerobic Microorganisms", "Conserved Biome"], &[], &[]);
    assert_eq!(res(&more, "Anaerobic Microorganisms"), Some(1 + 2),
               "1 par son propre tag Microbe + 2 par les tags de Conserved Biome");
}

// ============================== B. cartes qui posent des ressources ailleurs

/// (round 2) « Action: Add a microbe to ANOTHER* card. » — c'est une ACTION
/// depuis l'ADDENDUM (scan de la carte imprimée), donc observable seulement en
/// phase III. Test en FLUX RÉEL : pose par `build_card`, activation par la
/// boucle de jeu.
#[test]
fn symbiotic_fungus_action_adds_a_microbe_to_another_card() {
    let db = db();
    let (game, ids) = activate(&db, 3, &["Tardigrades", "Symbiotic Fungus"],
                               "Symbiotic Fungus", None, 1);
    assert_eq!(game.players[0].resources_on(ids["Tardigrades"]), 1);
    // Symbiotic Fungus ne porte rien : elle n'est jamais un réceptacle.
    assert_eq!(game.players[0].card_resources.len(), 1);
    assert!(!game.players[0].card_resources.contains_key(&ids["Symbiotic Fungus"]));
}

/// (round 2) L'action est RÉPÉTABLE : le sélectionneur de la phase III a droit
/// à une activation supplémentaire, et la carte pose alors 2 microbes.
#[test]
fn symbiotic_fungus_action_is_repeatable_within_the_phase() {
    let db = db();
    let (game, ids) = activate(&db, 3, &["Tardigrades", "Symbiotic Fungus"],
                               "Symbiotic Fungus", None, 2);
    assert_eq!(game.players[0].resources_on(ids["Tardigrades"]), 2);
}

/// (round 2) Les trois cartes reclassées n'ont plus AUCUN effet à la pose.
#[test]
fn the_three_rebranded_cards_have_no_build_effect_at_all() {
    let db = db();
    for name in ["Symbiotic Fungus", "Extreme-Cold Fungus", "Conserved Biome"] {
        // Posées seules : rien.
        let alone = seq(&db, &[name], &[], &[]);
        assert!(alone.played, "{name} se pose");
        assert!(alone.resources.is_empty(), "{name} ne porte rien");
        assert_eq!(
            (alone.delta.mc, alone.delta.hand, alone.delta.tr, alone.delta.plants),
            (0, 0, 0, 0),
            "{name} : aucun effet de pose"
        );
        // Posées APRÈS une porteuse : elles ne lui ajoutent toujours rien.
        let after = seq(&db, &["Tardigrades", name], &[], &[]);
        assert_eq!(res(&after, "Tardigrades"), Some(0), "{name} : rien posé à la pose");
        assert_eq!(after.delta.plants, 0, "{name} : aucune plante à la pose");
    }
}

/// (round 2) « Action: Gain 1 plant OR add a microbe to ANOTHER* card. »
/// Branche 0 = plantes, branche 1 = microbe (ordre du texte imprimé).
#[test]
fn extreme_cold_fungus_action_branches_in_printed_order() {
    let db = db();
    let cards = ["Tardigrades", "Extreme-Cold Fungus"];
    let (a, ids) = activate(&db, 4, &cards, "Extreme-Cold Fungus", Some(0), 1);
    assert_eq!(a.players[0].plants, 1, "branche 0 : 1 plante");
    assert_eq!(a.players[0].resources_on(ids["Tardigrades"]), 0);
    let (b, ids) = activate(&db, 4, &cards, "Extreme-Cold Fungus", Some(1), 1);
    assert_eq!(b.players[0].plants, 0, "branche 1 : aucune plante");
    assert_eq!(b.players[0].resources_on(ids["Tardigrades"]), 1);
}

#[test]
fn extreme_cold_fungus_is_the_official_card_not_the_buffed_twin() {
    let db = db();
    // Piège des classes « Buffed » : l'homonyme hors pioche v1 coûte 6 et
    // figure AVANT la vraie carte dans cards.json.
    let id = db.resolve_card("Extreme-Cold Fungus").expect("carte résolue");
    let card = &db.projects[id as usize];
    assert!(card.in_deck_v1, "la carte du deck v1 est la carte canonique");
    assert_eq!(card.price, 10, "prix imprimé de la vraie Extreme-Cold Fungus");
    assert!(card.effect.is_some(), "l'effet est rattaché à la carte canonique");
    // Le jumeau Buffed, lui, reste un stub neutre.
    let buffed: Vec<usize> = db
        .projects
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name == "Extreme-Cold Fungus" && !c.in_deck_v1)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(buffed.len(), 1);
    assert!(db.projects[buffed[0]].effect.is_none(), "le jumeau Buffed reste un stub");
}

/// (round 2) « Action: Add a microbe to ANOTHER* card OR add an animal to
/// ANOTHER* card. » Branche 0 = microbe, branche 1 = animal.
#[test]
fn conserved_biome_action_offers_microbe_then_animal() {
    let db = db();
    let cards = ["Tardigrades", "Birds", "Conserved Biome"];
    let (m, ids) = activate(&db, 6, &cards, "Conserved Biome", Some(0), 1);
    assert_eq!(
        (m.players[0].resources_on(ids["Tardigrades"]), m.players[0].resources_on(ids["Birds"])),
        (1, 0)
    );
    let (a, ids) = activate(&db, 6, &cards, "Conserved Biome", Some(1), 1);
    assert_eq!(
        (a.players[0].resources_on(ids["Tardigrades"]), a.players[0].resources_on(ids["Birds"])),
        (0, 1)
    );
}

#[test]
fn viral_enhancers_is_a_flat_one_not_multiplied_by_tags() {
    let db = db();
    // Ses propres tags MICROBE + PLANT déclenchent l'effet, mais le gain est
    // FORFAITAIRE : 1 plante, pas 2.
    let alone = seq(&db, &["Viral Enhancers"], &[0], &[]);
    assert_eq!(alone.delta.plants, 1);
    // Branche « poser sur une AUTRE carte » : 1 ressource, pas 2.
    let put = seq(&db, &["Tardigrades", "Viral Enhancers"], &[1], &["Tardigrades"]);
    assert_eq!(res(&put, "Tardigrades"), Some(1));
    assert_eq!(put.delta.plants, 0);
}

#[test]
fn decomposers_adds_a_microbe_or_trades_one_for_a_card() {
    let db = db();
    // À sa pose, la carte est vide : la branche « retirer un microbe » est
    // injouable, seule l'addition reste → 1 microbe.
    let alone = seq(&db, &["Decomposers"], &[1], &[]);
    assert_eq!(res(&alone, "Decomposers"), Some(1));
    assert_eq!(alone.vp, 1, "1 VP fixe imprimé");
    // Sur la carte suivante à tag Microbe, les deux branches sont jouables.
    let add = seq(&db, &["Decomposers", "Tardigrades"], &[0], &[]);
    assert_eq!(res(&add, "Decomposers"), Some(2));
    let draw = seq(&db, &["Decomposers", "Tardigrades"], &[1], &[]);
    assert_eq!(res(&draw, "Decomposers"), Some(0));
    assert_eq!(draw.delta.hand, 1, "une carte piochée contre le microbe");
}

#[test]
fn astrofarm_puts_two_microbes_elsewhere_and_produces() {
    let db = db();
    let r = seq(&db, &["Tardigrades", "Astrofarm"], &[], &["Tardigrades"]);
    assert_eq!(res(&r, "Tardigrades"), Some(2));
    assert_eq!((r.delta.plant_prod, r.delta.heat_prod), (1, 3));
}

#[test]
fn eos_chasma_puts_an_animal_elsewhere_gains_plants_and_produces() {
    let db = db();
    let r = seq(&db, &["Birds", "Eos Chasma National Park"], &[], &["Birds"]);
    assert!(!r.prereq_ok, "exige la température rouge ou plus");
    assert_eq!(res(&r, "Birds"), Some(1));
    assert_eq!((r.delta.plants, r.delta.mc_prod), (3, 2));
    assert_eq!(r.vp, 1 + 1, "1 VP fixe + 1 VP par animal sur Birds");
}

#[test]
fn ceos_favorite_project_adds_two_resources_of_any_kind() {
    let db = db();
    for (carrier, kind) in [
        ("Tardigrades", "microbe"),
        ("Birds", "animal"),
        ("Physics Complex", "science"),
    ] {
        let r = seq(&db, &[carrier, "CEO's Favorite Project"], &[], &[carrier]);
        assert_eq!(res(&r, carrier), Some(2), "{carrier}");
        assert_eq!(kind_of(&r, carrier), Some(kind), "{carrier}");
    }
}

#[test]
fn local_heat_trapping_spends_heat_gains_plants_and_places_two() {
    let db = db();
    let r = seq(&db, &["Tardigrades", "Local Heat Trapping"], &[], &["Tardigrades"]);
    assert_eq!((r.delta.heat, r.delta.plants), (-3, 4));
    assert_eq!(res(&r, "Tardigrades"), Some(2));
}

#[test]
fn imported_nitrogen_asks_two_targets_in_printed_order() {
    let db = db();
    // « Add 2 animals to ANOTHER card. Add 3 microbes to ANOTHER card. »
    let r = seq(&db, &["Birds", "Tardigrades", "Imported Nitrogen"], &[],
                &["Birds", "Tardigrades"]);
    assert_eq!((r.delta.tr, r.delta.plants), (1, 4));
    assert_eq!(res(&r, "Birds"), Some(2));
    assert_eq!(res(&r, "Tardigrades"), Some(3));
    assert_eq!(r.vp, 3, "2 VP Birds + 1 VP Tardigrades");
}

#[test]
fn imported_hydrogen_amount_depends_on_the_target_kind() {
    let db = db();
    // « Gain 3 plants, or add 3 microbes or 2 animals to ANOTHER card. »
    let plants = seq(&db, &["Tardigrades", "Imported Hydrogen"], &[0], &[]);
    // 3 plantes de la carte + 2 de la 1re tuile océan (sonde : tuiles non
    // mélangées, la première donne 2 plantes).
    assert_eq!(plants.delta.plants, 3 + 2);
    assert_eq!(res(&plants, "Tardigrades"), Some(0));
    let micro = seq(&db, &["Tardigrades", "Imported Hydrogen"], &[1], &["Tardigrades"]);
    assert_eq!(res(&micro, "Tardigrades"), Some(3));
    let animal = seq(&db, &["Birds", "Imported Hydrogen"], &[1], &["Birds"]);
    assert_eq!(res(&animal, "Birds"), Some(2));
    // L'océan de la carte est révélé dans les deux cas.
    assert_eq!(plants.delta.oceans, 1);
}

#[test]
fn large_convoy_targets_another_card_and_offers_plants() {
    let db = db();
    // « Flip an ocean tile. Draw two cards. Gain 5 plants or add 3 animals to
    //   ANOTHER card. » (round 2 : le scan dit ANOTHER, pas ANY.)
    let plants = seq(&db, &["Birds", "Large Convoy"], &[0], &[]);
    assert_eq!(plants.delta.plants, 5 + 2, "5 de la carte + 2 de la tuile océan");
    assert_eq!(plants.delta.oceans, 1);
    assert_eq!(plants.delta.hand, 2);
    assert_eq!(plants.vp, 2, "2 VP fixes");
    let animals = seq(&db, &["Birds", "Large Convoy"], &[1], &["Birds"]);
    assert_eq!(res(&animals, "Birds"), Some(3));
    assert_eq!(animals.delta.plants, 2, "seulement la tuile océan");
    assert_eq!(animals.vp, 2 + 3);
}

#[test]
fn cryogenic_shipment_places_resources_and_loses_the_phase_upgrade() {
    let db = db();
    let micro = seq(&db, &["Tardigrades", "Cryogenic Shipment"], &[], &["Tardigrades"]);
    assert_eq!(res(&micro, "Tardigrades"), Some(3));
    // L'amélioration de phase non gérée n'est compensée par RIEN.
    assert_eq!(
        (micro.delta.mc, micro.delta.hand, micro.delta.tr, micro.delta.plants),
        (0, 0, 0, 0)
    );
    let animal = seq(&db, &["Birds", "Cryogenic Shipment"], &[], &["Birds"]);
    assert_eq!(res(&animal, "Birds"), Some(2));
}

#[test]
fn advanced_ecosystems_requires_three_tags_and_scores_three_fixed_vp() {
    let db = db();
    let r = seq(&db, &["Advanced Ecosystems"], &[], &[]);
    assert!(r.in_lot);
    assert!(!r.prereq_ok, "exige 1 tag Animal, 1 Microbe et 1 Plante");
    assert_eq!(r.vp, 3);
    assert!(r.resources.is_empty(), "ne porte aucune ressource");
    // Avec les trois tags en jeu, le prérequis est satisfait.
    let ok = seq(&db, &["Ecological Zone", "Tardigrades", "Advanced Ecosystems"], &[], &[]);
    assert!(ok.prereq_ok_now, "Ecological Zone (Plant+Animal) + Tardigrades (Microbe)");
}

// =============================================================== intégration

#[test]
fn resources_lists_every_carrier_sorted_by_name_including_empty_ones() {
    let db = db();
    let r = seq(&db, &["Tardigrades", "Birds", "Fish"], &[], &[]);
    let names: Vec<&str> = r.resources.iter().map(|x| x.card.as_str()).collect();
    assert_eq!(names, vec!["Birds", "Fish", "Tardigrades"], "trié par nom de carte");
    assert!(r.resources.iter().all(|x| x.n == 0), "les porteuses vides sont listées");
}

#[test]
fn non_carrier_cards_are_never_receptacles() {
    let db = db();
    // Lichen porte un vp_dynamic ? non — mais surtout, aucune carte hors lot
    // n'entre dans `resources`, sans quoi le cas « aucune cible » n'existerait
    // jamais (clause anti-shortcut n° 5).
    let r = seq(&db, &["Lichen", "Comet", "Grass"], &[], &[]);
    assert!(r.resources.is_empty());
}

#[test]
fn an_imposed_target_absent_from_candidates_is_reported_not_silently_replaced() {
    let db = db();
    // (round 2 : Symbiotic Fungus est devenue une action ; on prend Astrofarm,
    // dont la POSE demande une cible pour ses 2 microbes.)
    let r = seq(&db, &["Tardigrades", "Astrofarm"], &[], &["Birds"]);
    assert!(r.target_error.is_some(), "erreur de cible signalée");
    assert_eq!(res(&r, "Tardigrades"), Some(0), "aucun repli silencieux");
    // Nom totalement inconnu : erreur aussi.
    let unknown = seq(&db, &["Tardigrades", "Astrofarm"], &[], &["Carte Inexistante"]);
    assert!(unknown.target_error.is_some());
    assert_eq!(res(&unknown, "Tardigrades"), Some(0));
}

#[test]
fn a_name_with_no_v1_twin_stays_resolvable_as_before() {
    let db = db();
    // Filter Feeders et Genetically Modified Vegetables ont DEUX entrées, aucune
    // dans le deck v1. La résolution canonique ne doit pas les faire
    // disparaître : à défaut de carte du deck v1, on garde la première, c'est-à-
    // dire le comportement historique de la sonde (rétro-compatibilité stricte
    // de `--probe`).
    for name in ["Filter Feeders", "Genetically Modified Vegetables"] {
        let id = db.resolve_card(name).unwrap_or_else(|| panic!("{name} introuvable"));
        let first = db
            .projects
            .iter()
            .position(|c| c.name == name)
            .expect(name) as u16;
        assert_eq!(id, first, "{name}: première entrée conservée");
        assert!(seq(&db, &[name], &[], &[]).found, "{name}: trouvée par la sonde");
    }
    // Un nom inconnu reste introuvable.
    assert!(db.resolve_card("Carte Inexistante").is_none());
}

#[test]
fn the_microbe_discount_checks_the_declared_resource_kind() {
    let db = db();
    let anaerobic = db.resolve_card("Anaerobic Microorganisms").unwrap();
    // La réduction déclare `kind: Microbe` ; la carte porte bien des microbes.
    assert_eq!(
        db.projects[anaerobic as usize].holds(),
        Some(engine::effects::ResKind::Microbe),
        "le type déclaré par la réduction doit être celui que la carte porte"
    );
    // Aucune autre carte du moteur ne déclare de réduction payée en ressources.
    let payers: Vec<&str> = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| {
            e.reductions
                .iter()
                .any(|r| matches!(r, engine::effects::Reduction::PayResources { .. }))
        })
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(payers, vec!["Anaerobic Microorganisms"]);
}

#[test]
fn probe_without_new_options_is_unchanged_and_reports_no_error() {
    let db = db();
    let a = seq(&db, &["Media Group", "Lichen"], &[], &[]);
    let b = run_probe_seq_scripted(&db, &["Media Group", "Lichen"], ProbeOptions::default(),
                                   &ProbeScript::default());
    assert_eq!((a.paid.clone(), a.vp), (b.paid.clone(), b.vp));
    assert_eq!(a.paid, vec![11, 5], "prix du lot 2 inchangés");
    assert!(a.target_error.is_none());
    assert!(a.resources.is_empty());
}

#[test]
fn probe_is_deterministic_with_and_without_a_script() {
    let db = db();
    let names = ["Tardigrades", "Birds", "Conserved Biome"];
    for sc in [ProbeScript::default(), script(&[1], &["Birds"])] {
        let a = run_probe_seq_scripted(&db, &names, ProbeOptions::default(), &sc);
        let b = run_probe_seq_scripted(&db, &names, ProbeOptions::default(), &sc);
        assert_eq!(a.resources, b.resources);
        assert_eq!((a.vp, a.paid.clone()), (b.vp, b.paid.clone()));
    }
}

#[test]
fn effects_off_disables_resources_entirely() {
    let mut db = db();
    db.effects_on = false;
    let r = seq(&db, &["Nitrite Reducting Bacteria"], &[], &[]);
    assert!(!r.in_lot);
    assert!(r.resources.is_empty(), "aucune ressource sans couche d'effets");
    assert_eq!(r.vp, 0);
}

#[test]
fn resource_victory_points_count_in_the_real_game_score() {
    let db = db();
    // Le score de partie et la sonde passent par le MÊME `flow::card_points`.
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 5, &mut pol);
    let birds = db.resolve_card("Birds").unwrap();
    game.deck.retain(|&c| c != birds);
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    game.players[0].hand.push(birds);
    let before = score(&game, &db)[0];
    build_card(&mut game, &db, 0, 0, 0);
    let empty = score(&game, &db)[0];
    // Birds posée vide : 0 VP de ressources (mais la carte compte pour les
    // awards/milestones, d'où une comparaison relative sur les seules
    // ressources ci-dessous).
    engine::flow::add_resources(&mut game, &db, 0, birds, 3);
    let full = score(&game, &db)[0];
    assert_eq!(full - empty, 3, "1 VP par animal posé sur Birds");
    let _ = before;
}

#[test]
fn audit_counters_are_real_and_neutral_with_effects_off() {
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(s.invariant_violations, 0);
    assert!(s.res_added > 0, "des ressources sont posées en flux réel");
    assert!(s.res_removed > 0, "et retirées");
    assert!(s.res_removed <= s.res_added, "jamais plus retiré qu'ajouté");
    assert!(s.vp_from_resources > 0, "elles rapportent des points");

    // Le compteur suit la taille de l'échantillon (il est incrémenté à
    // l'endroit réel, pas calculé en fin de partie).
    let big = run_simulation(&db, 600, 2024, &mut pol);
    assert!(big.res_added > s.res_added);

    let mut off = db;
    off.effects_on = false;
    let n = run_simulation(&off, 200, 2024, &mut pol);
    assert_eq!(
        (n.res_added, n.res_removed, n.res_targets_missing,
         n.phase_upgrades_skipped, n.vp_from_resources),
        (0, 0, 0, 0, 0)
    );
}

#[test]
fn anaerobic_microorganisms_discount_costs_two_microbes_and_is_a_choice() {
    let db = db();
    let anaerobic = db.resolve_card("Anaerobic Microorganisms").unwrap();
    // Cible sans tag Animal/Microbe/Plante, pour que le déclencheur de la
    // carte ne vienne pas repeupler ses microbes pendant le test.
    let target = db.resolve_card("Lava Flows").unwrap(); // rouge, prix 17, EVENT
    // Branche 0 = utiliser la réduction (2 microbes → −10 MC) ; 1 = y renoncer.
    for (choice, expect_microbes, expect_mc) in [(0usize, 0u32, 7i64), (1, 2, 17)] {
        let mut pol = ForceChoice { base: RandomPolicy, choice };
        let mut game = setup_game(&db, 21, &mut pol);
        game.deck.retain(|&c| c != anaerobic && c != target);
        let old: Vec<u16> = game.players[0].hand.drain(..).collect();
        game.deck.extend(old);
        // Anaerobic posée d'abord : elle prend 1 microbe (son propre tag), un
        // deuxième via une carte à tag Microbe.
        game.players[0].hand.push(anaerobic);
        engine::flow::build_card(&mut game, &db, 0, 0, 0);
        engine::flow::add_resources(&mut game, &db, 0, anaerobic, 1);
        assert_eq!(game.players[0].resources_on(anaerobic), 2);

        game.players[0].hand.push(target);
        game.players[0].mc = 100;
        let mc_before = game.players[0].mc;
        engine::flow::build_card_with(&mut game, &db, 0, 0, 0, &mut pol);
        assert_eq!(
            mc_before - game.players[0].mc,
            expect_mc,
            "choix {choice}: prix payé"
        );
        assert_eq!(
            game.players[0].resources_on(anaerobic),
            expect_microbes,
            "choix {choice}: microbes consommés"
        );
    }
}

#[test]
fn anaerobic_discount_counts_in_affordability() {
    let db = db();
    let anaerobic = db.resolve_card("Anaerobic Microorganisms").unwrap();
    // Carte chère : payable seulement grâce aux 10 MC de réduction.
    let target = db.resolve_card("Lava Flows").unwrap(); // rouge, prix 17
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 33, &mut pol);
    game.deck.retain(|&c| c != anaerobic && c != target);
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    game.players[0].hand.push(anaerobic);
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    engine::flow::add_resources(&mut game, &db, 0, anaerobic, 1);
    assert_eq!(game.players[0].resources_on(anaerobic), 2);
    // Le service annonce bien la réduction potentielle.
    let d = engine::flow::microbe_discount(&game, &db, 0);
    assert_eq!(d.map(|(c, n, a)| (c, n, a)), Some((anaerobic, 2, 10)));
    // Sans les microbes, plus rien.
    engine::flow::remove_resources(&mut game, &db, 0, anaerobic, 2);
    assert!(engine::flow::microbe_discount(&game, &db, 0).is_none());
    let _ = target;
}

#[test]
fn every_lot3_card_is_encoded_and_resolves_to_the_v1_deck() {
    let db = db();
    const LOT3: [&str; 28] = [
        "Tardigrades", "Birds", "Nitrite Reducting Bacteria",
        "Fibrous Composite Material", "Decomposing Fungus",
        "GHG Production Bacteria", "Regolith Eaters", "Fish", "Livestock",
        "Small Animals", "Herbivores", "Physics Complex", "Ecological Zone",
        "Anaerobic Microorganisms", "Symbiotic Fungus", "Extreme-Cold Fungus",
        "Conserved Biome", "Viral Enhancers", "Decomposers", "Astrofarm",
        "Eos Chasma National Park", "CEO's Favorite Project",
        "Local Heat Trapping", "Imported Nitrogen", "Imported Hydrogen",
        "Large Convoy", "Cryogenic Shipment", "Advanced Ecosystems",
    ];
    for name in LOT3 {
        let id = db.resolve_card(name).unwrap_or_else(|| panic!("{name} non résolue"));
        let card = &db.projects[id as usize];
        assert!(card.in_deck_v1, "{name} doit venir du deck v1");
        assert!(card.effect.is_some(), "{name} doit être encodée");
    }
    // 110 cartes des lots 1-2 + 28 (lot ressources) + 17 (lot 4 : productions
    // dérivées, NT par badge, bonus de recherche) = 155 entrées.
    assert_eq!(engine::effects::LOT1.len(), 155);
}

#[test]
fn the_fourteen_carriers_declare_the_printed_resource_kind() {
    let db = db();
    for (name, kind) in [
        ("Tardigrades", "microbe"), ("Birds", "animal"),
        ("Nitrite Reducting Bacteria", "microbe"),
        ("Fibrous Composite Material", "science"),
        ("Decomposing Fungus", "microbe"), ("GHG Production Bacteria", "microbe"),
        ("Regolith Eaters", "microbe"), ("Fish", "animal"),
        ("Livestock", "animal"), ("Small Animals", "animal"),
        ("Herbivores", "animal"), ("Physics Complex", "science"),
        ("Ecological Zone", "animal"), ("Anaerobic Microorganisms", "microbe"),
        ("Decomposers", "microbe"),
    ] {
        let r = seq(&db, &[name], &[], &[]);
        assert_eq!(kind_of(&r, name), Some(kind), "{name}");
    }
}

// ============================ activation d'une action en FLUX RÉEL (round 2)

/// Pose `cards` dans l'ordre (chemin réel `flow::build_card`), puis joue une
/// manche de phase III où le joueur 0 active `times` fois l'action de
/// `action_card`, en imposant éventuellement la branche `choice`.
///
/// Aucune boucle de test parallèle : la pose passe par `build_card` et
/// l'activation par `play_round`, comme une vraie partie. Les deux joueurs
/// choisissent la phase III, donc le joueur 0 est sélectionneur et dispose de
/// l'activation supplémentaire — ce qui permet d'observer qu'une action est
/// bien RÉPÉTABLE.
fn activate(
    db: &CardsDb,
    seed: u64,
    cards: &[&str],
    action_card: &str,
    choice: Option<usize>,
    times: usize,
) -> (engine::state::GameState, std::collections::BTreeMap<String, u16>) {
    let mut setup = RandomPolicy;
    let mut game = setup_game(db, seed, &mut setup);
    let mut ids = std::collections::BTreeMap::new();
    for name in cards {
        ids.insert(name.to_string(), db.resolve_card(name).expect(name));
    }
    // Main du joueur 0 = uniquement les cartes voulues, dans l'ordre.
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    let wanted: Vec<u16> = ids.values().copied().collect();
    game.deck.retain(|c| !wanted.contains(c));
    // De quoi payer comptant : `build_card` est une pose FORCÉE, elle assère le
    // paiement. Le budget ne fait pas partie de ce qu'on mesure ici.
    game.players[0].mc = 1000;
    for name in cards {
        game.players[0].hand.push(ids[*name]);
        build_card(&mut game, db, 0, 0, 0);
    }
    // Plantes et chaleur à 0 : la conversion obligatoire de fin de phase III ne
    // doit pas polluer la mesure (forêts, température et leurs déclencheurs).
    for p in 0..2 {
        game.players[p].plants = 0;
        game.players[p].heat = 0;
    }
    let mut pol = ActivateBlue {
        base: RandomPolicy,
        target: ids[action_card],
        remaining: times,
        choice,
    };
    play_round(&mut game, db, &mut pol);
    assert_eq!(pol.remaining, 0, "l'action n'a pas pu être activée {times} fois");
    (game, ids)
}

/// Politique de test : choisit la phase III, ne construit rien, et active
/// `remaining` fois l'action de la carte bleue `target`, puis passe.
struct ActivateBlue {
    base: RandomPolicy,
    target: u16,
    remaining: usize,
    choice: Option<usize>,
}
impl Policy for ActivateBlue {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&3) {
            3
        } else {
            self.base.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        _r: &mut StdRng,
        _p: usize,
        options: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        if self.remaining == 0 {
            return None;
        }
        // Le joueur 1 n'a pas la carte : l'option n'existe pas chez lui, il passe.
        let i = options
            .iter()
            .position(|o| *o == engine::policy::ActionOpt::BlueAction(self.target))?;
        self.remaining -= 1;
        Some(i)
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        match self.choice {
            Some(c) => c,
            None => self.base.choose_option(r, p, n),
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

// ================================================== politiques de test locales

/// Choisit toujours la même phase et ne fait rien d'autre que le défaut.
struct PhaseOnly {
    base: RandomPolicy,
    phase: u8,
}
impl PhaseOnly {
    fn new(phase: u8) -> PhaseOnly {
        PhaseOnly { base: RandomPolicy, phase }
    }
}
impl Policy for PhaseOnly {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) {
            self.phase
        } else {
            self.base.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        _r: &mut StdRng,
        _p: usize,
        _o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        None // tout le monde passe : seule la conversion obligatoire agit
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// Impose une réponse fixe à `choose_option` (branches d'alternative).
struct ForceChoice {
    base: RandomPolicy,
    choice: usize,
}
impl Policy for ForceChoice {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn choose_option(&mut self, _r: &mut StdRng, _p: usize, _n: usize) -> usize {
        self.choice
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

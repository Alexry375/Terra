//! Tests du lot 1 — un test par carte du lot (63), plus tests d'intégration
//! de la couche d'effets (prérequis dans le flux réel, interrupteur, score).
//!
//! Chaque test par carte joue la carte par la sonde (`probe::run_probe`), qui
//! passe par `flow::build_card` — le MÊME chemin de pose que `simulate` — et
//! vérifie l'ÉTAT DE JEU résultant contre des valeurs écrites À LA MAIN
//! depuis le texte imprimé de la carte (champ `description` de cards.json),
//! PAS depuis la table d'encodage (clause anti-shortcut du prompt).
//!
//! État de départ de la sonde (fixe, documenté dans src/probe.rs) :
//! 100 MC, 20 chaleur, 20 plantes, prods 0, TR 5, température niveau 0
//! (−30 °C), oxygène 0, 0 océan, aucune corporation ni tag en jeu.
//! Tuiles océan non mélangées : 1re = +2 plantes, 2e = +4 MC.
//! Rappels : chaque hausse de température/oxygène/océan donne +1 TR ;
//! `delta.mc` exclut le prix payé ; `delta.hand` exclut la carte jouée.

use engine::cards::CardsDb;
use engine::probe::run_probe;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

/// delta attendu : [mc, heat, plants, hand, mc_prod, heat_prod, plant_prod,
/// card_prod, tr, temperature, oxygen, oceans, forests]
macro_rules! probe_case {
    ($fn_name:ident, $card:literal, prereq: $pr:literal, vp: $vp:literal,
     [$mc:literal, $heat:literal, $plants:literal, $hand:literal,
      $mcp:literal, $hp:literal, $pp:literal, $cp:literal,
      $tr:literal, $temp:literal, $oxy:literal, $oc:literal, $fo:literal]) => {
        #[test]
        fn $fn_name() {
            let db = db();
            let r = run_probe(&db, $card);
            assert!(r.found, "{}: found", $card);
            assert!(r.in_lot, "{}: in_lot", $card);
            assert!(r.played, "{}: played", $card);
            assert_eq!(r.prereq_ok, $pr, "{}: prereq_ok", $card);
            assert_eq!(r.vp, $vp, "{}: vp fixes", $card);
            let d = r.delta;
            assert_eq!(
                (d.mc, d.heat, d.plants, d.hand),
                ($mc, $heat, $plants, $hand),
                "{}: delta ressources/main", $card
            );
            assert_eq!(
                (d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod),
                ($mcp, $hp, $pp, $cp),
                "{}: delta productions", $card
            );
            assert_eq!(
                (d.tr, d.temperature, d.oxygen, d.oceans, d.forests),
                ($tr, $temp, $oxy, $oc, $fo),
                "{}: delta TR/planète", $card
            );
        }
    };
}

// ------------------------------------------------------ les 10 cartes imposées

// « Raise the temperature 1 step. Flip an ocean tile. » : +1 temp (+1 TR),
// 1 océan (+1 TR, tuile 1 = +2 plantes).
probe_case!(probe_comet, "Comet", prereq: true, vp: 0,
    [0, 0, 2, 0, 0, 0, 0, 0, 2, 1, 0, 1, 0]);
// « Requires white temperature. Gain 2 plants. …produces 2 MC and 2 plants. »
// Température −30 = palier violet → prérequis NON satisfait, pose forcée.
probe_case!(probe_farming, "Farming", prereq: false, vp: 2,
    [0, 0, 2, 0, 2, 0, 2, 0, 0, 0, 0, 0, 0]);
// « During the production phase, this produces 1 plant. »
probe_case!(probe_lichen, "Lichen", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Raise the temperature 1 step. …produces 1 heat. »
probe_case!(probe_deep_well_heating, "Deep Well Heating", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0]);
// « Requires 4 Science tag. » — 0 tag science en jeu → prérequis non
// satisfait ; aucun effet, 4 VP imprimés.
probe_case!(probe_interstellar_colony_ship, "Interstellar Colony Ship",
    prereq: false, vp: 4,
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires 5 ocean tiles to be flipped. …produces 2 plants. »
probe_case!(probe_algae, "Algae", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0]);
// « Requires red temperature or warmer. Gain 2 plants. …produces 2 plants. »
probe_case!(probe_bushes, "Bushes", prereq: false, vp: 0,
    [0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0]);
// « During the production phase, draw a card. »
probe_case!(probe_acquired_company, "Acquired Company", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
// « Requires you to spend 1 TR. …produces 4 heat. » TR 5 → dépense possible.
probe_case!(probe_lunar_beam, "Lunar Beam", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 4, 0, 0, -1, 0, 0, 0, 0]);
// « Raise infrastructure 2 steps. Gain 4 plants. » Chaque pas
// d'infrastructure : +1 TR et pioche 1 carte (sémantique Java, journal B2).
probe_case!(probe_grain_silos, "Grain Silos", prereq: true, vp: 0,
    [0, 0, 4, 2, 0, 0, 0, 0, 2, 0, 0, 0, 0]);

// ------------------------------------------------------------- vertes (37)

// « During the production phase this produces 1 plant. »
probe_case!(probe_adapted_lichen, "Adapted Lichen", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Requires red oxygen or higher. …you draw a card and this produces 2 heat. »
probe_case!(probe_aerated_magma, "Aerated Magma", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 2, 0, 1, 0, 0, 0, 0, 0]);
// « Requires red oxygen or higher. Raise oxygen 1 step. …produces 2 heat. »
probe_case!(probe_airborne_radiation, "Airborne Radiation", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 2, 0, 0, 1, 0, 1, 0, 0]);
// « Requires purple temperature. …produces 1 plant. » −30 °C = violet → OK.
probe_case!(probe_archaebacteria, "Archaebacteria", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « …produces 1 plant and 1 heat. »
probe_case!(probe_artificial_photosynthesis, "Artificial Photosynthesis",
    prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0]);
// « Requires you to spend 1 TR. …produces 3 MC. » 1 VP imprimé.
probe_case!(probe_balanced_portfolios, "Balanced Portfolios", prereq: true, vp: 1,
    [0, 0, 0, 0, 3, 0, 0, 0, -1, 0, 0, 0, 0]);
// « Requires you to spend 2 plants. …produces 5 heat. »
probe_case!(probe_biomass_combustors, "Biomass Combustors", prereq: true, vp: 0,
    [0, 0, -2, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
// « …you draw a card and this produces 2 MC. »
probe_case!(probe_blueprints, "Blueprints", prereq: true, vp: 0,
    [0, 0, 0, 0, 2, 0, 0, 1, 0, 0, 0, 0, 0]);
// « …produces 3 heat. »
probe_case!(probe_coal_imports, "Coal Imports", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]);
// « …produces 4 MC. » 2 VP imprimés.
probe_case!(probe_commercial_district, "Commercial District", prereq: true, vp: 2,
    [0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires red temperature or warmer. …draw a card and …1 plant. »
probe_case!(probe_dandelions, "Dandelions", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0]);
// « Requires red temperature or colder. …produces 2 plants. » −30 °C ≤ palier
// rouge → prérequis satisfait.
probe_case!(probe_designed_microorganisms, "Designed Microorganisms",
    prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0]);
// « Gain 3 plants and 3 heat. …produces 1 plant. »
probe_case!(probe_diversified_interests, "Diversified Interests",
    prereq: true, vp: 0,
    [0, 3, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « …produces 3 MC. »
probe_case!(probe_economic_growth, "Economic Growth", prereq: true, vp: 0,
    [0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires you to spend 2 plants. …produces 4 MC. »
probe_case!(probe_food_factory, "Food Factory", prereq: true, vp: 0,
    [0, 0, -2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires you to spend 1 TR. …produces 2 heat. » 1 VP imprimé.
probe_case!(probe_fueled_generators, "Fueled Generators", prereq: true, vp: 1,
    [0, 0, 0, 0, 0, 2, 0, 0, -1, 0, 0, 0, 0]);
// « Requires 2 Energy tags. During the production phase, draw a card. »
// 0 tag énergie en jeu (les tags de la carte ne comptent pas avant sa pose).
probe_case!(probe_fusion_power, "Fusion Power", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
// « Requires 3 Science tags. …produces 2 MC. » 2 VP imprimés.
probe_case!(probe_gene_repair, "Gene Repair", prereq: false, vp: 2,
    [0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
// « …produces 2 heat. »
probe_case!(probe_geothermal_power, "Geothermal Power", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
// « Requires red temperature or warmer. Gain 3 plants. …produces 1 plant. »
probe_case!(probe_grass, "Grass", prereq: false, vp: 0,
    [0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Requires 2 ocean tiles to be flipped. …produces 2 heat. » 1 VP imprimé.
probe_case!(probe_great_dam, "Great Dam", prereq: false, vp: 1,
    [0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
// « Gain 1 plant. …produces 1 plant. » 1 VP imprimé.
probe_case!(probe_heather, "Heather", prereq: true, vp: 1,
    [0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Gain 5 heat. …produces 1 heat. »
probe_case!(probe_imported_ghg, "Imported GHG", prereq: true, vp: 0,
    [0, 5, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
// « …produces 1 MC and 2 plants. »
probe_case!(probe_industrial_farming, "Industrial Farming", prereq: true, vp: 0,
    [0, 0, 0, 0, 1, 0, 2, 0, 0, 0, 0, 0, 0]);
// « Requires 6 ocean tiles… Gain 2 plants. …produces 2 MC and 3 plants. »
probe_case!(probe_kelp_farming, "Kelp Farming", prereq: false, vp: 1,
    [0, 0, 2, 0, 2, 0, 3, 0, 0, 0, 0, 0, 0]);
// « …produces 4 heat. »
probe_case!(probe_mohole_area, "Mohole Area", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
// « Requires you to spend 1 TR. …produces 2 plants. »
probe_case!(probe_monocultures, "Monocultures", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 2, 0, -1, 0, 0, 0, 0]);
// « Requires 3 ocean tiles to be flipped and for you to spend 1 plant.
// …produces 1 plant. » Océans insuffisants (prereq faux), la dépense de
// 1 plante est bien payée à la pose forcée.
probe_case!(probe_moss, "Moss", prereq: false, vp: 0,
    [0, 0, -1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Draw 2 cards. …produces 5 heat. »
probe_case!(probe_smelting, "Smelting", prereq: true, vp: 0,
    [0, 0, 0, 2, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
// « Raise the temperature 1 step. …produces 2 plants. »
probe_case!(probe_soil_warming, "Soil Warming", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 2, 0, 1, 1, 0, 0, 0]);
// « Draw a card and gain 3 heat. …produces 1 heat. »
probe_case!(probe_solar_trapping, "Solar Trapping", prereq: true, vp: 0,
    [0, 3, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
// « Draw a card. …produces 2 heat. »
probe_case!(probe_space_heater, "Space Heater", prereq: true, vp: 0,
    [0, 0, 0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
// « …produces 2 MC. »
probe_case!(probe_sponsors, "Sponsors", prereq: true, vp: 0,
    [0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires yellow temperature or warmer. Gain 1 plant. …produces 3 plants. »
probe_case!(probe_trees, "Trees", prereq: false, vp: 1,
    [0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0]);
// « Requires you to spend 5 heat. …produces 4 MC. » 2 VP imprimés.
probe_case!(probe_tropical_resort, "Tropical Resort", prereq: true, vp: 2,
    [0, -5, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Requires yellow temperature or warmer. Gain 1 plant. …2 MC and 1 plant. »
probe_case!(probe_tundra_farming, "Tundra Farming", prereq: false, vp: 1,
    [0, 0, 1, 0, 2, 0, 1, 0, 0, 0, 0, 0, 0]);
// « Requires 3 ocean tiles to be flipped. …produces 3 heat. »
probe_case!(probe_wave_power, "Wave Power", prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0]);

// ------------------------------------------------------------- rouges (16)

// « Requires yellow temperature or warmer. Flip an ocean tile. » 1 VP.
// Océan 1 = +2 plantes, +1 TR.
probe_case!(probe_artificial_lake, "Artificial Lake", prereq: false, vp: 1,
    [0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0]);
// « Requires 2 Science tags. Raise oxygen 1 step. »
probe_case!(probe_atmosphere_filtering, "Atmosphere Filtering",
    prereq: false, vp: 0,
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0]);
// « Requires yellow oxygen or higher. » Aucun effet, 2 VP imprimés.
probe_case!(probe_breathing_filters, "Breathing Filters", prereq: false, vp: 2,
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Raise your TR 2 steps. » −2 VP imprimés.
probe_case!(probe_bribed_comittee, "Bribed Comittee", prereq: true, vp: -2,
    [0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0]);
// « Draw a card. Flip an ocean tile. »
probe_case!(probe_convoy_from_europa, "Convoy from Europa", prereq: true, vp: 0,
    [0, 0, 2, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0]);
// « Requires 3 EVT. Flip an ocean tile. » 0 tag événement en jeu.
probe_case!(probe_crater, "Crater", prereq: false, vp: 0,
    [0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0]);
// « Raise the temperature 3 steps. Gain 7 MC. »
probe_case!(probe_deimos_down, "Deimos Down", prereq: true, vp: 0,
    [7, 0, 0, 0, 0, 0, 0, 0, 3, 3, 0, 0, 0]);
// « Raise the temperature 2 steps. Flip 2 ocean tiles. »
// Océans 1-2 : +2 plantes, +4 MC ; TR = 2 temp + 2 océans.
probe_case!(probe_giant_ice_asteroid, "Giant Ice Asteroid", prereq: true, vp: 0,
    [4, 0, 2, 0, 0, 0, 0, 0, 4, 2, 0, 2, 0]);
// « Flip 2 ocean tiles. »
probe_case!(probe_ice_asteroid, "Ice Asteroid", prereq: true, vp: 0,
    [4, 0, 2, 0, 0, 0, 0, 0, 2, 0, 0, 2, 0]);
// « Requires you to spend 1 TR. Gain 10 MC. » 1 VP imprimé.
probe_case!(probe_investment_loan, "Investment Loan", prereq: true, vp: 1,
    [10, 0, 0, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0]);
// « Raise the temperature 2 steps. »
probe_case!(probe_lava_flows, "Lava Flows", prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0]);
// « Raise your TR 2 steps. Raise the temperature 1 step. Gain 2 plants.
// If you have 3 or more Plant, gain 4 additional plants. » — 0 tag plante en
// jeu : pas de bonus. TR = +2 (direct) +1 (température).
probe_case!(probe_nitrogen_rich_asteroid, "Nitrogen-Rich Asteroid",
    prereq: true, vp: 0,
    [0, 0, 2, 0, 0, 0, 0, 0, 3, 1, 0, 0, 0]);
// « Raise your TR 2 steps. »
probe_case!(probe_release_of_inert_gases, "Release of Inert Gases",
    prereq: true, vp: 0,
    [0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0]);
// « Draw 2 cards. »
probe_case!(probe_research, "Research", prereq: true, vp: 0,
    [0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
// « Flip an ocean tile. »
probe_case!(probe_subterranean_reservoir, "Subterranean Reservoir",
    prereq: true, vp: 0,
    [0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0]);
// « Raise oxygen 1 step. Flip an ocean tile. Gain 2 plants » — plantes :
// 2 (texte) + 2 (tuile océan 1) ; TR : oxygène + océan.
probe_case!(probe_towing_a_comet, "Towing a Comet", prereq: true, vp: 0,
    [0, 0, 4, 0, 0, 0, 0, 0, 2, 0, 1, 1, 0]);

// ================================================= intégration couche d'effets

use engine::flow::{play_round, requirements_met, score, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::sim::{check_invariants, InvariantTracker};
use rand::rngs::StdRng;
use std::collections::VecDeque;

/// Politique minimale : phases scriptées, enregistre les options de
/// construction proposées (`affordable`), ne construit jamais.
struct RecordingPolicy {
    base: RandomPolicy,
    phase_script: VecDeque<u8>,
    build_offers: Vec<Vec<usize>>,
}

impl Policy for RecordingPolicy {
    fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, rng: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        match self.phase_script.pop_front() {
            Some(ph) => ph,
            None => self.base.pick_phase(rng, p, allowed),
        }
    }
    fn choose_build(&mut self, _: &mut StdRng, _: usize, a: &[usize]) -> Option<usize> {
        self.build_offers.push(a.to_vec());
        None
    }
    fn construction_bonus(&mut self, _: &mut StdRng, _: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _: &mut StdRng, _: usize, _: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn research_keep(&mut self, _: &mut StdRng, _: usize, _: &[u16], k: usize) -> Vec<usize> {
        (0..k).collect()
    }
    fn discard_down(&mut self, _: &mut StdRng, _: usize, _: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

fn card_id(db: &CardsDb, name: &str) -> u16 {
    db.projects.iter().position(|c| c.name == name).unwrap() as u16
}

/// Prérequis vérifiés AVANT de jouer, dans le flux réel : Bushes (« Requires
/// red temperature or warmer ») n'est pas proposée à la construction à
/// −30 °C, elle l'est au palier rouge.
#[test]
fn requirements_gate_construction_in_real_flow() {
    let db = db();
    let bushes = card_id(&db, "Bushes");
    for (temp_level, expect_offered) in [(0u8, false), (6u8, true)] {
        let mut pol = RecordingPolicy {
            base: RandomPolicy,
            phase_script: VecDeque::from(vec![1, 4]),
            build_offers: Vec::new(),
        };
        let mut game = setup_game(&db, 7, &mut pol);
        // Main de p0 contrôlée : uniquement Bushes (les autres cartes
        // retournent à la pioche), 100 MC pour exclure la contrainte de prix.
        let old: Vec<u16> = game.players[0].hand.drain(..).collect();
        game.deck.extend(old);
        game.deck.retain(|&c| c != bushes);
        game.players[0].hand.push(bushes);
        game.players[0].mc = 100;
        game.temperature = temp_level;

        play_round(&mut game, &db, &mut pol);
        let p0_offer = &pol.build_offers[0];
        assert_eq!(
            !p0_offer.is_empty(),
            expect_offered,
            "temp niveau {temp_level}: Bushes proposée = {expect_offered}"
        );
    }
}

/// La même vérification, par le prédicat public : paliers de température.
#[test]
fn requirement_thresholds_match_color_zones() {
    let db = db();
    let mut pol = RecordingPolicy {
        base: RandomPolicy,
        phase_script: VecDeque::new(),
        build_offers: Vec::new(),
    };
    let mut game = setup_game(&db, 8, &mut pol);
    let farming = card_id(&db, "Farming"); // white temperature = niveau 16+
    let trees = card_id(&db, "Trees"); // yellow or warmer = niveau 11+
    let archae = card_id(&db, "Archaebacteria"); // purple = niveau <= 5
    for lvl in 0u8..=19 {
        game.temperature = lvl;
        // Lot 3 / C1 : les prérequis de paramètres sont jugés sur l'INSTANTANÉ
        // de début de phase (livret p.13). Le test écrivait la valeur courante
        // hors de tout flux de phase ; il prend maintenant l'instantané, ce que
        // `play_round` fait au début de chaque phase. Paliers inchangés.
        game.snapshot_planet();
        assert_eq!(requirements_met(&game, &db, 0, farming), lvl >= 16);
        assert_eq!(requirements_met(&game, &db, 0, trees), lvl >= 11);
        assert_eq!(requirements_met(&game, &db, 0, archae), lvl <= 5);
    }
}

/// « Spend 1 TR » : après pose par le chemin réel, l'état reste cohérent pour
/// TOUS les invariants (TR comptabilisé par tr_decrements, conservation…).
#[test]
fn spend_tr_keeps_invariants_consistent() {
    let db = db();
    let r = run_probe(&db, "Lunar Beam");
    assert_eq!(r.delta.tr, -1);
    // Rejoue l'état de la sonde à la main pour inspecter les invariants.
    let mut pol = RecordingPolicy {
        base: RandomPolicy,
        phase_script: VecDeque::new(),
        build_offers: Vec::new(),
    };
    let mut game = setup_game(&db, 9, &mut pol);
    let lunar = card_id(&db, "Lunar Beam");
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    game.deck.retain(|&c| c != lunar);
    game.players[0].hand.push(lunar);
    game.players[0].mc = 100;
    engine::flow::build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.players[0].tr, 4, "TR 5 - 1 dépensé");
    assert_eq!(game.players[0].tr_decrements, 1);
    let mut tracker = InvariantTracker::new(&game);
    assert!(check_invariants(&game, &db, &mut tracker).is_ok());
}

/// `--effects off` : la sonde ne trouve aucune carte « en lot », la pose est
/// neutre (delta nul hors prix) — non-régression du squelette.
#[test]
fn effects_off_makes_probe_neutral() {
    let mut db = db();
    db.effects_on = false;
    let r = run_probe(&db, "Comet");
    assert!(r.found && r.played);
    assert!(!r.in_lot, "effets coupés: hors lot");
    assert!(r.prereq_ok, "effets coupés: aucun prérequis");
    assert_eq!(r.delta, engine::probe::ProbeDelta::default(), "delta nul hors prix");
}

/// Une carte hors lot reste un stub neutre jouable même avec les effets ON.
///
/// (lot cartes-7) Ce test nommait *Adaptation Technology* comme exemple de
/// carte sans encodage. Elle est encodée depuis ce lot : le témoin était devenu
/// faux, non par régression mais parce que le travail demandé l'a rendu faux —
/// déclaré au journal (§D8) et dans `result.md`. **L'assertion n'est pas
/// affaiblie** : elle est rendue indépendante d'un nom de carte, en cherchant
/// dans la pioche réelle la première carte que le moteur n'encode pas. Elle
/// restera vraie au lot suivant, quelle que soit la carte encodée.
///
/// (lot cartes-8) Le lot précédent le rendait indépendant d'un nom de carte ;
/// celui-ci lui retire son dernier sujet en boîte de BASE — les 208 projets y
/// sont désormais encodés, il n'existe plus une seule carte à observer. Le test
/// est donc **retourné et rendu plus exigeant**, jamais neutralisé :
///
/// 1. il ÉPINGLE le fait que la boîte de base est intégralement encodée — une
///    régression qui rendrait une carte muette le ferait échouer ;
/// 2. il continue de vérifier le comportement de stub neutre, sur la
///    configuration `base,decouverte` où 33 cartes restent sans encodage.
///
/// Le jour où Découverte sera encodée à son tour, la seconde partie n'aura plus
/// de sujet non plus : elle deviendra alors le même épinglage que la première.
#[test]
fn out_of_lot_card_stays_neutral_stub() {
    let db = db();
    assert!(
        !db.projects.iter().any(|c| c.in_deck && c.effect.is_none()),
        "la boîte de base doit rester intégralement encodée"
    );

    // (jokers-corpos) Ce jour-là est arrivé : les trois dernières cartes muettes
    // (les projets à badge joker) sont encodées, la pioche `base,decouverte` ne
    // contient donc plus AUCUN stub. La seconde moitié du test devient ce que sa
    // propre documentation annonçait — le même épinglage que la première, sur la
    // configuration des deux boîtes. Le témoin « une carte sans encodage se
    // comporte en stub neutre » n'a plus de sujet : il ne peut plus être écrit
    // sans fabriquer une carte que le jeu ne contient pas.
    let db = CardsDb::load_boites(
        "../data/cards.json",
        engine::boites::BoiteSet::parse("base,decouverte").expect("configuration valide"),
    )
    .expect("cards.json doit se charger");
    let nues: Vec<&str> = db
        .projects
        .iter()
        .filter(|c| c.in_deck && c.effect.is_none())
        .map(|c| c.name.as_str())
        .collect();
    assert!(
        nues.is_empty(),
        "base + Découverte doit rester intégralement encodée, or : {nues:?}"
    );
}

/// Le score compte les VP fixes des cartes jouées avec les effets ON,
/// et les ignore avec `--effects off` (squelette intégral).
#[test]
fn score_counts_fixed_card_vp_only_with_effects_on() {
    let mut db = db();
    let mut pol = RecordingPolicy {
        base: RandomPolicy,
        phase_script: VecDeque::new(),
        build_offers: Vec::new(),
    };
    let mut game = setup_game(&db, 10, &mut pol);
    // Awards neutralisés : ProjectManager (cartes jouées) fausserait le delta.
    game.awards = [engine::state::AwardKind::Celebrity; 3];
    let cd = card_id(&db, "Commercial District"); // 2 VP imprimés
    game.deck.retain(|&c| c != cd);
    game.players[0].put_in_play(cd, &db);

    let s_on = score(&game, &db);
    db.effects_on = false;
    let s_off = score(&game, &db);
    assert_eq!(s_on[0] - s_off[0], 2, "2 VP imprimés comptés avec effets ON");
    assert_eq!(s_on[1], s_off[1]);
}

/// VP dynamiques : Io Mining Industries vaut « 1 VP per Jupiter tag you
/// have » — 2 tags Jupiter en jeu (elle-même + Ganymede Shipyard) = 2 VP.
#[test]
fn score_counts_dynamic_jupiter_vp() {
    let db = db();
    let mut pol = RecordingPolicy {
        base: RandomPolicy,
        phase_script: VecDeque::new(),
        build_offers: Vec::new(),
    };
    let mut game = setup_game(&db, 11, &mut pol);
    // Awards neutralisés : ProjectManager (cartes jouées) fausserait le delta.
    game.awards = [engine::state::AwardKind::Celebrity; 3];
    let base = score(&game, &db)[0];
    let io = card_id(&db, "Io Mining Industries"); // JUPITER 1/1, 0 VP fixe
    let gany = card_id(&db, "Ganymede Shipyard"); // tag JUPITER, 0 VP
    game.deck.retain(|&c| c != io && c != gany);
    game.players[0].put_in_play(io, &db);
    game.players[0].put_in_play(gany, &db);
    let s = score(&game, &db)[0];
    assert_eq!(s - base, 2, "2 tags Jupiter x 1 VP");
}

/// La sonde est déterministe : deux exécutions identiques.
#[test]
fn probe_is_deterministic() {
    let db = db();
    for name in ["Comet", "Smelting", "Grain Silos"] {
        let a = run_probe(&db, name);
        let b = run_probe(&db, name);
        assert_eq!(a.delta, b.delta, "{name}");
        assert_eq!(
            (a.found, a.in_lot, a.prereq_ok, a.played, a.vp),
            (b.found, b.in_lot, b.prereq_ok, b.played, b.vp)
        );
    }
}

/// Intégrité du lot : >= 50 cartes, les 10 imposées présentes, chaque entrée
/// résolue vers une carte unique de la base (le chargement échoue sinon).
#[test]
fn lot_table_integrity() {
    let db = db();
    assert!(engine::effects::LOT1.len() >= 50, "lot >= 50 cartes");
    for name in [
        "Comet", "Farming", "Lichen", "Deep Well Heating",
        "Interstellar Colony Ship", "Algae", "Bushes", "Acquired Company",
        "Lunar Beam", "Grain Silos",
    ] {
        assert!(
            engine::effects::lookup(name).is_some(),
            "carte imposée absente du lot: {name}"
        );
        assert!(
            db.projects.iter().any(|c| c.name == name),
            "carte imposée absente de la base: {name}"
        );
    }
    // Toutes les cartes du lot sauf Grain Silos (imposée, hors pioche —
    // journal B2) viennent de la pioche v1.
    for (name, _) in engine::effects::LOT1 {
        // (lot 3) La recherche par premier nom identique ne suffit plus : quatre
        // cartes du lot 3 ont un homonyme « Buffed » hors pioche v1 placé AVANT
        // elles dans cards.json. On emprunte la résolution CANONIQUE du moteur —
        // celle qui rattache les effets. L'assertion, elle, est inchangée.
        let card = &db.projects[db.resolve_card(name).expect("carte du lot non résolue") as usize];
        assert!(
            card.in_deck_v1 || *name == "Grain Silos",
            "carte du lot hors pioche v1: {name}"
        );
    }
}

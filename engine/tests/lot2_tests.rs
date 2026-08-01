//! Tests du lot 2 — un test par carte du lot (47) vérifiant l'ÉTAT DE JEU
//! résultant contre le TEXTE IMPRIMÉ (pas contre la table d'encodage), plus des
//! tests d'intégration (réductions dans l'affordabilité, compteur blue_actions,
//! interrupteur --effects off, intégrité du lot).
//!
//! Mécanismes et sondes utilisées :
//! - (A) réductions : `--probe "reducteur;cible"` → `paid[1]` = prix de la cible
//!   après réduction. La cible est une carte hors-lot (stub) de tag connu ; le
//!   montant de réduction est écrit à la main depuis le texte imprimé, le prix
//!   de la cible est lu dans la base (fait de jeu indépendant).
//! - (B) déclencheurs : `--probe "declencheur;cible"` (ou `--probe` simple pour
//!   « including this ») → delta d'état.
//! - (C) actions : `--probe-action "carte"` → delta isolant l'action.

use engine::cards::{CardsDb, Color};
use engine::flow::{build_card, play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe, run_probe_action, run_probe_seq};
use rand::rngs::StdRng;
use std::collections::VecDeque;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

// (lot 3) Les deux accesseurs empruntent la résolution CANONIQUE du moteur
// (`CardsDb::resolve_card`), la même que la sonde. La recherche par premier nom
// identique ne convient plus : cards.json contient des homonymes « Buffed »
// hors pioche v1, parfois AVANT la carte officielle et à un prix différent
// (Drone Assisted Construction : 7 pour la Buffed, 15 pour la vraie). Les
// assertions des tests sont inchangées ; c'est la carte visée qui est
// désormais la bonne.
fn price(db: &CardsDb, name: &str) -> i64 {
    db.projects[card_id(db, name) as usize].price
}

fn card_id(db: &CardsDb, name: &str) -> u16 {
    db.resolve_card(name).expect(name)
}

/// Prix payé pour `target` quand `reducer` est déjà en jeu (pose forcée séquence).
fn paid_target(db: &CardsDb, reducer: &str, target: &str) -> i64 {
    let r = run_probe_seq(db, &[reducer, target]);
    assert_eq!(r.paid.len(), 2, "{reducer};{target}: paid attendu de longueur 2");
    r.paid[1]
}

// Cibles hors-lot (stubs neutres) de tag connu, pour observer les réductions.
const T_ANY: &str = "Bribed Comittee"; // red, prix 5 (EARTH, EVENT)
const T_BUILDING: &str = "Microorganism Industry"; // blue, prix 5 (BUILDING)
const T_SPACE: &str = "Drone Assisted Construction"; // blue, prix 15 (SPACE)
const T_EVENT: &str = "Lava Flows"; // red, prix 17 (EVENT)
const T_ENERGY: &str = "Fusion Power"; // green, prix 7 (ENERGY)
const T_EARTH: &str = "Advertising"; // green, prix 4 (EARTH)
const T_JUPITER: &str = "Colonizer Training Camp"; // red, prix 10 (JUPITER, pas EARTH)
const T_STUB_EVENT: &str = "Synthetic Catastrophe"; // red, prix 0 (EVENT, stub)

// ============================================================ 10 imposées (A/B/C)

#[test]
fn earth_catapult_reduces_any_card_2() {
    let db = db();
    let r = run_probe(&db, "Earth Catapult");
    assert!(r.in_lot && r.played);
    // « When you play a card, you pay 2 MC less for it. »
    assert_eq!(paid_target(&db, "Earth Catapult", T_ANY), price(&db, T_ANY) - 2);
    assert_eq!(paid_target(&db, "Earth Catapult", T_SPACE), price(&db, T_SPACE) - 2);
}

#[test]
fn research_outpost_reduces_any_card_1() {
    let db = db();
    assert!(run_probe(&db, "Research Outpost").in_lot);
    assert_eq!(paid_target(&db, "Research Outpost", T_ANY), price(&db, T_ANY) - 1);
}

#[test]
fn media_group_reduces_events_5_only() {
    let db = db();
    // « When you play an Event, you pay 5 MC less for it. »
    assert_eq!(paid_target(&db, "Media Group", T_EVENT), price(&db, T_EVENT) - 5);
    // Pas de réduction sur une carte sans tag Event.
    assert_eq!(paid_target(&db, "Media Group", T_BUILDING), price(&db, T_BUILDING));
}

#[test]
fn asteroid_mining_reduces_space_6() {
    let db = db();
    // Texte imprimé « pay 6 MC less on Space » (Java = titane 2, conflit lot2.md).
    let r = run_probe(&db, "Asteroid Mining");
    assert!(r.in_lot && r.played);
    assert_eq!(paid_target(&db, "Asteroid Mining", T_SPACE), price(&db, T_SPACE) - 6);
    // Aucune réduction sur une carte sans tag Space.
    assert_eq!(paid_target(&db, "Asteroid Mining", T_ANY), price(&db, T_ANY));
}

#[test]
fn energy_subsidies_reduces_energy_4_and_draws() {
    let db = db();
    // « pay 4 MC less for it and you draw a card » (par tag Energy).
    assert_eq!(paid_target(&db, "Energy Subsidies", T_ENERGY), price(&db, T_ENERGY) - 4);
    // Fusion Power (1 tag Energy) : Energy Subsidies pioche 1 carte.
    let r = run_probe_seq(&db, &["Energy Subsidies", T_ENERGY]);
    // hand : +1 (déclencheur pioche) ; Fusion Power (lot 1) ajoute card_prod +1.
    assert_eq!(r.delta.hand, 1, "Energy Subsidies pioche 1 sur tag Energy");
}

#[test]
fn development_center_action_spends_2_heat_draws() {
    let db = db();
    let r = run_probe_action(&db, "Development Center");
    assert!(r.has_action && r.action_applied);
    assert_eq!((r.delta.heat, r.delta.hand), (-2, 1));
}

#[test]
fn farmers_market_action_spends_1_mc_gains_2_plants() {
    let db = db();
    let r = run_probe_action(&db, "Farmers Market");
    assert!(r.action_applied);
    assert_eq!((r.delta.mc, r.delta.plants), (-1, 2));
}

#[test]
fn caretaker_contract_action_spends_8_heat_raises_tr() {
    let db = db();
    // Prérequis de POSE : jaune ou plus chaud → faux à −30 (pose forcée).
    assert!(!run_probe(&db, "Caretaker Contract").prereq_ok);
    let r = run_probe_action(&db, "Caretaker Contract");
    assert!(r.action_applied);
    assert_eq!((r.delta.heat, r.delta.tr), (-8, 1));
}

#[test]
fn optimal_aerobraking_event_gains_2_heat_2_plants() {
    let db = db();
    // « When you play an Event tag, you gain 2 heat and 2 plants. »
    let r = run_probe_seq(&db, &["Optimal Aerobraking", T_STUB_EVENT]);
    assert_eq!((r.delta.heat, r.delta.plants), (2, 2));
    // Pose seule (tag Space, pas Event) : rien.
    assert_eq!(run_probe(&db, "Optimal Aerobraking").delta.heat, 0);
}

#[test]
fn olympus_conference_science_draws_including_self() {
    let db = db();
    // « When you play a Science tag, including this, draw a card. »
    // Pose seule : Olympus a 1 tag Science → se déclenche sur elle-même → +1.
    assert_eq!(run_probe(&db, "Olympus Conference").delta.hand, 1);
    // Puis sur une autre carte à tag Science (Fusion Power) : +1 de plus.
    let r = run_probe_seq(&db, &["Olympus Conference", T_ENERGY]);
    assert_eq!(r.delta.hand, 2, "self + Fusion Power (1 science)");
}

// ============================================================ A : réductions

#[test]
fn asteroid_mining_consortium_space_3() {
    let db = db();
    assert_eq!(paid_target(&db, "Asteroid Mining Consortium", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn electric_arc_furnaces_building_4() {
    let db = db();
    assert_eq!(paid_target(&db, "Electric Arc Furnaces", T_BUILDING), price(&db, T_BUILDING) - 4);
}
#[test]
fn great_escarpment_consortium_building_2() {
    let db = db();
    assert_eq!(paid_target(&db, "Great Escarpment Consortium", T_BUILDING), price(&db, T_BUILDING) - 2);
}
#[test]
fn mine_building_4() {
    let db = db();
    assert_eq!(paid_target(&db, "Mine", T_BUILDING), price(&db, T_BUILDING) - 4);
}
#[test]
fn space_station_space_3() {
    let db = db();
    assert_eq!(paid_target(&db, "Space Station", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn titanium_mine_space_3() {
    let db = db();
    assert_eq!(paid_target(&db, "Titanium Mine", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn vesta_shipyard_space_3() {
    let db = db();
    assert_eq!(paid_target(&db, "Vesta Shipyard", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn ganymede_shipyard_space_6() {
    let db = db();
    assert_eq!(paid_target(&db, "Ganymede Shipyard", T_SPACE), price(&db, T_SPACE) - 6);
}
#[test]
fn ilmenite_deposits_space_6() {
    let db = db();
    assert_eq!(paid_target(&db, "Ilmenite Deposits", T_SPACE), price(&db, T_SPACE) - 6);
}
#[test]
fn surface_mines_building_2_and_space_3() {
    let db = db();
    assert_eq!(paid_target(&db, "Surface Mines", T_BUILDING), price(&db, T_BUILDING) - 2);
    assert_eq!(paid_target(&db, "Surface Mines", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn industrial_center_prod3_and_building_2() {
    let db = db();
    assert_eq!(run_probe(&db, "Industrial Center").delta.mc_prod, 3);
    assert_eq!(paid_target(&db, "Industrial Center", T_BUILDING), price(&db, T_BUILDING) - 2);
}
#[test]
fn industrial_microbes_heatprod1_and_building_2() {
    let db = db();
    assert_eq!(run_probe(&db, "Industrial Microbes").delta.heat_prod, 1);
    assert_eq!(paid_target(&db, "Industrial Microbes", T_BUILDING), price(&db, T_BUILDING) - 2);
}
#[test]
fn underground_city_mcprod1_and_building_2() {
    let db = db();
    assert_eq!(run_probe(&db, "Underground City").delta.mc_prod, 1);
    assert_eq!(paid_target(&db, "Underground City", T_BUILDING), price(&db, T_BUILDING) - 2);
}
#[test]
fn micro_mills_heatprod1_and_building_2() {
    let db = db();
    assert_eq!(run_probe(&db, "Micro-Mills").delta.heat_prod, 1);
    assert_eq!(paid_target(&db, "Micro-Mills", T_BUILDING), price(&db, T_BUILDING) - 2);
}
#[test]
fn building_industries_spend4heat_and_building_4() {
    let db = db();
    // « Requires you to spend 4 heat. »
    assert_eq!(run_probe(&db, "Building Industries").delta.heat, -4);
    assert_eq!(paid_target(&db, "Building Industries", T_BUILDING), price(&db, T_BUILDING) - 4);
}
#[test]
fn fuel_factory_spend3heat_prod1_and_space_3() {
    let db = db();
    let r = run_probe(&db, "Fuel Factory");
    assert_eq!((r.delta.heat, r.delta.mc_prod), (-3, 1));
    assert_eq!(paid_target(&db, "Fuel Factory", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn strip_mine_spend1tr_building_4_space_3() {
    let db = db();
    // « Requires you to spend 1 TR. »
    assert_eq!(run_probe(&db, "Strip Mine").delta.tr, -1);
    assert_eq!(paid_target(&db, "Strip Mine", T_BUILDING), price(&db, T_BUILDING) - 4);
    assert_eq!(paid_target(&db, "Strip Mine", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn io_mining_industries_prod2_space_6_vp_dynamic() {
    let db = db();
    let r = run_probe(&db, "Io Mining Industries");
    assert_eq!(r.delta.mc_prod, 2);
    assert_eq!(r.vp, 0, "0 VP fixe (les VP viennent du dynamique JUPITER)");
    assert_eq!(paid_target(&db, "Io Mining Industries", T_SPACE), price(&db, T_SPACE) - 6);
}
#[test]
fn mass_converter_req4sci_heatprod3_space_3() {
    let db = db();
    let r = run_probe(&db, "Mass Converter");
    // « Requires 4 Science tags. » — 0 en jeu → prérequis faux.
    assert!(!r.prereq_ok);
    assert_eq!(r.delta.heat_prod, 3);
    assert_eq!(paid_target(&db, "Mass Converter", T_SPACE), price(&db, T_SPACE) - 3);
}
#[test]
fn dusty_quarry_ocean_max_and_building_2() {
    let db = db();
    // « Requires 3 or fewer ocean tiles » — 0 océan → prérequis vrai.
    assert!(run_probe(&db, "Dusty Quarry").prereq_ok);
    assert_eq!(paid_target(&db, "Dusty Quarry", T_BUILDING), price(&db, T_BUILDING) - 2);
}

// ============================================================ B : déclencheurs

#[test]
fn interplanetary_conference_earth_jupiter_3_and_draw() {
    let db = db();
    // « Earth or Jupiter tag, excluding this, pay 3 MC less and draw a card. »
    assert_eq!(paid_target(&db, "Interplanetary Conference", T_EARTH), price(&db, T_EARTH) - 3);
    assert_eq!(paid_target(&db, "Interplanetary Conference", T_JUPITER), price(&db, T_JUPITER) - 3);
    // Pioche 1 carte sur une cible à tag Earth/Jupiter.
    assert_eq!(run_probe_seq(&db, &["Interplanetary Conference", T_EARTH]).delta.hand, 1);
    // « excluding this » : pose seule (tag Earth) ne se déclenche pas.
    assert_eq!(run_probe(&db, "Interplanetary Conference").delta.hand, 0);
}
#[test]
fn anti_gravity_technology_any_card_2heat_2plants() {
    let db = db();
    // « Requires 5 Science. » (pose forcée) « When you play a card, gain 2 heat and 2 plants. »
    assert!(!run_probe(&db, "Anti-Gravity Technology").prereq_ok);
    let r = run_probe_seq(&db, &["Anti-Gravity Technology", T_STUB_EVENT]);
    assert_eq!((r.delta.heat, r.delta.plants), (2, 2));
}
#[test]
fn impact_analysis_event_draws_1() {
    let db = db();
    assert_eq!(run_probe_seq(&db, &["Impact Analysis", T_STUB_EVENT]).delta.hand, 1);
    // Pas d'événement → pas de pioche.
    assert_eq!(run_probe_seq(&db, &["Impact Analysis", T_BUILDING]).delta.hand, 0);
}
#[test]
fn recycled_detritus_event_draws_2() {
    let db = db();
    assert_eq!(run_probe_seq(&db, &["Recycled Detritus", T_STUB_EVENT]).delta.hand, 2);
}
#[test]
fn volcanic_soil_gains_2_plants_per_temperature_step() {
    let db = db();
    // « When you raise the temperature, gain 2 plants. » Lava Flows = 2 pas → +4.
    let r = run_probe_seq(&db, &["Volcanic Soil", "Lava Flows"]);
    assert_eq!((r.delta.plants, r.delta.temperature), (4, 2));
    // Pose seule : aucune hausse → 0 plante.
    assert_eq!(run_probe(&db, "Volcanic Soil").delta.plants, 0);
}
#[test]
fn arctic_algae_gains_4_plants_per_ocean() {
    let db = db();
    // « When you flip an ocean tile, gain 4 plants. » Subterranean = 1 océan.
    // +4 (déclencheur) + 2 (tuile océan 1) = 6 plantes.
    let r = run_probe_seq(&db, &["Arctic Algae", "Subterranean Reservoir"]);
    assert_eq!((r.delta.plants, r.delta.oceans), (6, 1));
}

// ============================================================ C : actions

#[test]
fn circuit_board_factory_action_draws_1() {
    let db = db();
    let r = run_probe_action(&db, "Circuit Board Factory");
    assert!(r.action_applied);
    assert_eq!(r.delta.hand, 1);
}
#[test]
fn matter_manufactoring_action_spend1mc_draw1() {
    let db = db();
    let r = run_probe_action(&db, "Matter Manufactoring");
    assert_eq!((r.delta.mc, r.delta.hand), (-1, 1));
}
#[test]
fn artificial_jungle_action_spend1plant_draw1() {
    let db = db();
    let r = run_probe_action(&db, "Artificial Jungle");
    assert_eq!((r.delta.plants, r.delta.hand), (-1, 1));
}
#[test]
fn ironworks_action_spend4heat_oxygen1() {
    let db = db();
    let r = run_probe_action(&db, "Ironworks");
    assert_eq!((r.delta.heat, r.delta.oxygen, r.delta.tr), (-4, 1, 1));
}
#[test]
fn steelworks_action_spend6heat_2mc_oxygen1() {
    let db = db();
    let r = run_probe_action(&db, "Steelworks");
    assert_eq!((r.delta.heat, r.delta.mc, r.delta.oxygen, r.delta.tr), (-6, 2, 1, 1));
}
#[test]
fn ai_central_action_draws_2() {
    let db = db();
    // « Requires 5 Science. » (pose forcée) « Action: Draw 2 cards. »
    assert!(!run_probe(&db, "Ai Central").prereq_ok);
    let r = run_probe_action(&db, "Ai Central");
    assert!(r.action_applied);
    assert_eq!(r.delta.hand, 2);
}
#[test]
fn think_tank_action_spend2mc_draw1_vp_dynamic() {
    let db = db();
    assert_eq!(run_probe(&db, "Think Tank").vp, 0, "VP dynamiques BLUE_CARD, 0 fixe");
    let r = run_probe_action(&db, "Think Tank");
    assert_eq!((r.delta.mc, r.delta.hand), (-2, 1));
}
#[test]
fn power_infrastructure_action_heat_to_mc() {
    let db = db();
    // « Spend any amount of heat to gain that amount of MC. »
    let r = run_probe_action(&db, "Power Infrastructure");
    assert!(r.has_action);
    assert_eq!(r.delta.mc, -r.delta.heat, "MC gagné = chaleur dépensée");
    assert!(r.delta.heat <= 0 && r.delta.mc >= 0);
}
#[test]
fn volcanic_pools_action_spend_reduced_mc_flip_ocean() {
    let db = db();
    // Aucun tag Energy en jeu → coût plein 12 ; flip océan (tuile 1 = +2 plantes).
    let r = run_probe_action(&db, "Volcanic Pools");
    assert!(r.action_applied);
    assert_eq!((r.delta.mc, r.delta.oceans, r.delta.plants, r.delta.tr), (-12, 1, 2, 1));
}
#[test]
fn developed_infrastructure_action_spend_mc_raise_temp() {
    let db = db();
    // 1 carte bleue en jeu (elle-même) < 5 → coût plein 10 ; +1 température.
    let r = run_probe_action(&db, "Developed Infrastructure");
    assert!(r.action_applied);
    assert_eq!((r.delta.mc, r.delta.temperature, r.delta.tr), (-10, 1, 1));
}
#[test]
fn redrafted_contracts_action_discards_and_draws_equal() {
    // « Discard up to three cards in hand. Draw that many cards. »
    // Sonde-action depuis la main vide ne montre rien ; test en flux réel avec
    // une main garnie et un montant scripté (2).
    let db = db();
    let mut pol = ActionScript::phase3_action("Redrafted Contracts", 2);
    let mut game = setup_game(&db, 5, &mut pol);
    let rc = card_id(&db, "Redrafted Contracts");
    game.deck.retain(|&c| c != rc);
    game.players[0].put_in_play(rc, &db);
    game.players[0].mc = 0;
    // Main de p0 : 8 cartes de départ (garanti ≥ 2).
    let hand_before = game.players[0].hand.len();
    let cartes_de_p0: Vec<u16> = game.players[0].hand.clone();
    play_round(&mut game, &db, &mut pol);
    // Défausse 2, pioche 2 : main inchangée, action comptée.
    assert_eq!(game.players[0].hand.len(), hand_before, "défausse 2 / pioche 2");
    // (boites-1) ATTENTE MISE À JOUR — la comparaison portait sur la TAILLE de
    // la défausse commune (`discard_before + 2`), qui mélange les défausses des
    // deux joueurs. Avec la pioche réelle (208 cartes au lieu de 248), la
    // graine 5 donne à p1 une autre corporation et p1 défausse une carte de
    // plus dans le même tour : le total passe à 3 sans que l'action testée ait
    // changé. On compte donc désormais les cartes DE P0 arrivées à la défausse
    // — attribution exacte, insensible à ce que fait l'autre joueur.
    let de_p0_defaussees = cartes_de_p0
        .iter()
        .filter(|c| game.discard.contains(c))
        .count();
    assert_eq!(de_p0_defaussees, 2, "les 2 cartes défaussées sont celles de p0");
    assert_eq!(game.blue_actions, 1);
}

// ==================================================== intégration (A/B/C + off)

/// Politique scriptée : phase 3 (action) pour p0, action bleue nommée choisie une
/// fois avec un montant fixe, puis stop. p1 joue en recherche.
struct ActionScript {
    base: RandomPolicy,
    phase_script: VecDeque<u8>,
    target: Option<(u16, bool)>, // (id, déjà joué ?)
    amount: i64,
    resolved_target: String,
}
impl ActionScript {
    fn phase3_action(target: &str, amount: i64) -> ActionScript {
        ActionScript {
            base: RandomPolicy,
            // Phase 3 (action) pour p0, phase 4 (production, sans pioche) pour p1
            // afin que la main de p0 ne bouge QUE par l'action testée.
            phase_script: VecDeque::from(vec![3, 4]),
            target: None,
            amount,
            resolved_target: target.to_string(),
        }
    }
}
impl Policy for ActionScript {
    fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        match self.phase_script.pop_front() {
            Some(ph) if allowed.contains(&ph) => ph,
            _ => self.base.pick_phase(r, p, allowed),
        }
    }
    fn choose_build(&mut self, _: &mut StdRng, _: usize, _: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, _: &mut StdRng, _: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _: &mut StdRng, p: usize, options: &[ActionOpt]) -> Option<usize> {
        if p != 0 {
            return None;
        }
        // Cherche l'option BlueAction de la carte cible, une seule fois.
        for (i, o) in options.iter().enumerate() {
            if let ActionOpt::BlueAction(_) = o {
                if self.target.map(|(_, done)| !done).unwrap_or(true) {
                    self.target = Some((0, true));
                    return Some(i);
                }
            }
        }
        None
    }
    fn action_amount(&mut self, _: &mut StdRng, _: usize, max: i64) -> i64 {
        self.amount.min(max).max(0)
    }
    fn research_keep(&mut self, _: &mut StdRng, _: usize, _: &[u16], k: usize) -> Vec<usize> {
        (0..k).collect()
    }
    fn discard_down(&mut self, _: &mut StdRng, _: usize, _: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

#[test]
fn blue_action_counter_increments_in_real_flow_and_applies_effect() {
    // Circuit Board Factory (action gratuite : pioche 1) posée, activée en phase
    // III réelle : blue_actions passe à 1 ET la main gagne 1 carte.
    let db = db();
    let mut pol = ActionScript::phase3_action("Circuit Board Factory", 0);
    let mut game = setup_game(&db, 3, &mut pol);
    let cbf = card_id(&db, "Circuit Board Factory");
    game.deck.retain(|&c| c != cbf);
    game.players[0].put_in_play(cbf, &db);
    let hand_before = game.players[0].hand.len();
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.blue_actions, 1, "une action bleue appliquée");
    assert!(
        game.players[0].hand.len() > hand_before,
        "l'action a réellement pioché"
    );
}

#[test]
fn reduction_gates_affordability_in_real_flow() {
    // Une carte devient constructible UNIQUEMENT grâce à une réduction en jeu.
    // Cible : Drone Assisted Construction (Space, prix 15). Réducteur : Asteroid
    // Mining (Space −6 → prix 9). Avec 9 MC : inabordable sans, abordable avec.
    //
    // (lot 3) Le montant de MC est passé de 3 à 9 : la sonde et les tests
    // résolvent désormais la VRAIE Drone Assisted Construction (prix 15) et non
    // son homonyme « Buffed » hors pioche v1 (prix 7), qui était celui atteint
    // par la recherche du premier nom identique. L'assertion — « la carte n'est
    // proposée QUE grâce à la réduction » — est inchangée.
    let db = db();
    let target = card_id(&db, T_SPACE);
    let reducer = card_id(&db, "Asteroid Mining");
    for (with_reducer, expect) in [(false, false), (true, true)] {
        let mut pol = RecordBuild::new(vec![2, 5]);
        let mut game = setup_game(&db, 7, &mut pol);
        // Main de p0 = uniquement la cible ; 3 MC.
        let old: Vec<u16> = game.players[0].hand.drain(..).collect();
        game.deck.extend(old);
        game.deck.retain(|&c| c != target && c != reducer);
        game.players[0].hand.push(target);
        game.players[0].mc = 9;
        if with_reducer {
            game.players[0].put_in_play(reducer, &db);
        }
        play_round(&mut game, &db, &mut pol);
        let offered = !pol.offers.is_empty() && !pol.offers[0].is_empty();
        assert_eq!(offered, expect, "réduction en jeu = {with_reducer}");
    }
}

/// Politique qui enregistre les options de construction et ne construit rien.
struct RecordBuild {
    base: RandomPolicy,
    phase_script: VecDeque<u8>,
    offers: Vec<Vec<usize>>,
}
impl RecordBuild {
    fn new(phases: Vec<u8>) -> RecordBuild {
        RecordBuild {
            base: RandomPolicy,
            phase_script: VecDeque::from(phases),
            offers: Vec::new(),
        }
    }
}
impl Policy for RecordBuild {
    fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        match self.phase_script.pop_front() {
            Some(ph) if allowed.contains(&ph) => ph,
            _ => self.base.pick_phase(r, p, allowed),
        }
    }
    fn choose_build(&mut self, _: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        if p == 0 {
            self.offers.push(a.to_vec());
        }
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

#[test]
fn effects_off_neutralizes_reductions_triggers_and_actions() {
    let mut db = db();
    db.effects_on = false;
    // Réduction inopérante : prix plein.
    assert_eq!(paid_target(&db, "Earth Catapult", T_ANY), price(&db, T_ANY));
    // Déclencheur inopérant : Olympus ne pioche pas.
    assert_eq!(run_probe(&db, "Olympus Conference").delta.hand, 0);
    // Action inopérante : sonde-action neutre, hors lot.
    let a = run_probe_action(&db, "Development Center");
    assert!(!a.has_action && !a.action_applied);
    assert_eq!(a.delta, engine::probe::ProbeDelta::default());
    assert!(!run_probe(&db, "Earth Catapult").in_lot);
}

#[test]
fn build_card_never_charges_negative_price() {
    // Réductions cumulées > prix : payé plafonné à 0 (assert paid>=0 dans build_card).
    let db = db();
    // Great Escarpment (Building −2) + Mine (Building −4) = −6 sur Landfill (prix 2).
    let mut game = setup_game(&db, 9, &mut RandomPolicy);
    let mine = card_id(&db, "Mine");
    let esc = card_id(&db, "Great Escarpment Consortium");
    let landfill = card_id(&db, "Landfill");
    game.players[0].put_in_play(mine, &db);
    game.players[0].put_in_play(esc, &db);
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    game.deck.retain(|&c| c != landfill);
    game.players[0].hand.push(landfill);
    let mc_before = game.players[0].mc;
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.players[0].mc, mc_before, "prix payé plafonné à 0");
}

#[test]
fn lot2_table_integrity_and_imposed_present() {
    let db = db();
    // Lot 1 (63) + lot 2 (47) = 110 entrées.
    assert!(engine::effects::LOT1.len() >= 108, "lot 1 + lot 2");
    for name in [
        "Earth Catapult", "Research Outpost", "Media Group", "Asteroid Mining",
        "Energy Subsidies", "Development Center", "Farmers Market",
        "Caretaker Contract", "Optimal Aerobraking", "Olympus Conference",
    ] {
        let r = run_probe(&db, name);
        assert!(r.found && r.in_lot && r.played, "imposée jouable : {name}");
    }
}

#[test]
fn sequence_probe_is_deterministic() {
    let db = db();
    let a = run_probe_seq(&db, &["Earth Catapult", "Research Outpost", "Bribed Comittee"]);
    let b = run_probe_seq(&db, &["Earth Catapult", "Research Outpost", "Bribed Comittee"]);
    assert_eq!(a.paid, b.paid);
    assert_eq!(a.delta, b.delta);
}

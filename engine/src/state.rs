//! État de jeu : joueurs, planète, pioches, milestones/awards.
//!
//! Sources des constantes :
//! - Température -30..+8 par pas de 2 (20 niveaux), oxygène 0..14 (15 niveaux),
//!   9 océans : livret de base (aperçu p.2, plateau p.4) et
//!   `PlanetFactory.createMars` du moteur Java.
//! - Bonus des tuiles océan : `PlanetFactory.generateOceans` (Java).
//! - TR de départ 5, main de départ 8, limite de main 10, défausse 3 MC :
//!   `Constants.java` (STARTING_RT, DEFAULT_START_HAND_SIZE,
//!   MAX_HAND_SIZE_LAST_ROUND) + livret (avslutningssteget p.16).

use crate::cards::{CardsDb, Color, TAG_COUNT};
use rand::rngs::StdRng;

pub const NUM_PLAYERS: usize = 2;
/// Niveau max de température (index 19 == +8 °C).
pub const TEMPERATURE_MAX: u8 = 19;
/// Niveau max d'oxygène (14 %).
pub const OXYGEN_MAX: u8 = 14;
/// Nombre de tuiles océan.
pub const NUM_OCEANS: u8 = 9;

pub const STARTING_TR: i64 = 5;
pub const STARTING_HAND: usize = 8;
pub const HAND_LIMIT: usize = 10;
pub const SELL_CARD_MC: i64 = 3;

// Actions standard (livret p.14 + Constants.java).
pub const FOREST_PLANT_COST: i64 = 8;
pub const FOREST_MC_COST: i64 = 20;
pub const TEMPERATURE_HEAT_COST: i64 = 8;
pub const TEMPERATURE_MC_COST: i64 = 14;
pub const OCEAN_MC_COST: i64 = 15;

// Bonus du sélectionneur de phase (faskort du livret p.11-15).
pub const DEV_SELECTOR_DISCOUNT: i64 = 3;
pub const PRODUCTION_SELECTOR_MC: i64 = 4;

/// Bonus d'une tuile océan (cartes, MC, plantes) — `PlanetFactory` Java.
#[derive(Debug, Clone, Copy)]
pub struct OceanTile {
    pub cards: u8,
    pub mc: i64,
    pub plants: i64,
}

/// Les 9 tuiles océan du jeu de base (ordre avant mélange).
pub const OCEAN_TILES: [OceanTile; 9] = [
    OceanTile { cards: 0, mc: 0, plants: 2 },
    OceanTile { cards: 0, mc: 4, plants: 0 },
    OceanTile { cards: 1, mc: 1, plants: 0 },
    OceanTile { cards: 0, mc: 2, plants: 1 },
    OceanTile { cards: 1, mc: 0, plants: 1 },
    OceanTile { cards: 1, mc: 0, plants: 0 },
    OceanTile { cards: 0, mc: 1, plants: 1 },
    OceanTile { cards: 1, mc: 0, plants: 0 },
    OceanTile { cards: 0, mc: 0, plants: 2 },
];

/// Améliorations de phase (Discovery) — STRUCTURE seulement, effets neutres v1 (D14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseUpgrade {
    VariantA,
    VariantB,
}

/// Milestones (pool du moteur Java, base + Discovery). 3 en jeu par partie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneKind {
    /// 8 tags bâtiment.
    Builder,
    /// 9 tags différents.
    Diversifier,
    /// 10 de production de chaleur.
    Energizer,
    /// 5 de production de plantes.
    Farmer,
    /// 6 cartes rouges.
    Legend,
    /// 8 cartes vertes.
    Magnate,
    /// 12 cartes jouées.
    Planner,
    /// 7 tags espace.
    SpaceBaron,
    /// TR >= 15.
    Terraformer,
    /// 6 cartes bleues.
    Tycoon,
    /// 3 forêts.
    Gardener,
}

pub const MILESTONE_POOL: [MilestoneKind; 11] = [
    MilestoneKind::Builder,
    MilestoneKind::Diversifier,
    MilestoneKind::Energizer,
    MilestoneKind::Farmer,
    MilestoneKind::Legend,
    MilestoneKind::Magnate,
    MilestoneKind::Planner,
    MilestoneKind::SpaceBaron,
    MilestoneKind::Terraformer,
    MilestoneKind::Tycoon,
    MilestoneKind::Gardener,
];

/// Awards (pool du moteur Java ; le livret Discovery annonce 7 tuiles mais le
/// moteur de référence n'en implémente que 6 — conflit noté, D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwardKind {
    /// Production de MC.
    Celebrity,
    /// Ressources posées sur cartes (toujours 0 en v1 — stub).
    Collector,
    /// Production de chaleur.
    Generator,
    /// Capacités acier+titane (toujours 0 en v1 — stub).
    Industrialist,
    /// Nombre de cartes jouées.
    ProjectManager,
    /// Tags science.
    Researcher,
}

pub const AWARD_POOL: [AwardKind; 6] = [
    AwardKind::Celebrity,
    AwardKind::Collector,
    AwardKind::Generator,
    AwardKind::Industrialist,
    AwardKind::ProjectManager,
    AwardKind::Researcher,
];

/// Un milestone en jeu + qui l'a revendiqué (revendication simplifiée D8).
#[derive(Debug, Clone, Copy)]
pub struct MilestoneSlot {
    pub kind: MilestoneKind,
    pub achieved_by: [bool; NUM_PLAYERS],
}

/// État d'un joueur.
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    pub tr: i64,
    pub forests: i64,
    // Productions (toujours 0 en v1 — les cartes stub n'en donnent pas ;
    // structure présente pour les chantiers cartes).
    pub mc_prod: i64,
    pub heat_prod: i64,
    pub plant_prod: i64,
    pub card_prod: i64,
    // Capacités acier/titane (stub v1, structure pour chantiers suivants).
    pub steel_capacity: i64,
    pub titanium_capacity: i64,
    /// Main (indices dans CardsDb.projects).
    pub hand: Vec<u16>,
    /// Cartes jouées.
    pub played: Vec<u16>,
    /// Corporation choisie (indice dans CardsDb.corporations).
    pub corporation: Option<u16>,
    /// Compteurs de tags en jeu (corporation incluse).
    pub tag_counts: [u32; TAG_COUNT],
    /// Compteurs de couleurs jouées (vert/bleu/rouge).
    pub color_counts: [u32; 3],
    /// Phase choisie cette ronde (1-5), 0 = pas encore choisie.
    pub chosen_phase: u8,
    /// Phase choisie à la ronde précédente (interdite cette ronde).
    pub previous_phase: Option<u8>,
    /// Activations bonus de la phase action (sélectionneur : +1).
    pub extra_blue_activations: u8,
    /// Améliorations de phase Discovery — structure stub, toujours None (D14).
    pub phase_upgrades: [Option<PhaseUpgrade>; 5],
    /// Compteur d'audit : nombre d'incréments de TR accordés (invariant TR).
    pub tr_increments: u64,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            mc: 0,
            heat: 0,
            plants: 0,
            tr: STARTING_TR,
            forests: 0,
            mc_prod: 0,
            heat_prod: 0,
            plant_prod: 0,
            card_prod: 0,
            steel_capacity: 0,
            titanium_capacity: 0,
            hand: Vec::new(),
            played: Vec::new(),
            corporation: None,
            tag_counts: [0; TAG_COUNT],
            color_counts: [0; 3],
            chosen_phase: 0,
            previous_phase: None,
            extra_blue_activations: 0,
            phase_upgrades: [None; 5],
            tr_increments: 0,
        }
    }

    /// Fait entrer une carte en jeu (tags + couleur) — effet unique : aucun (stub).
    pub fn put_in_play(&mut self, card_id: u16, db: &CardsDb) {
        let card = &db.projects[card_id as usize];
        for t in &card.tags {
            if let Some(i) = t.index() {
                self.tag_counts[i] += 1;
            }
        }
        self.color_counts[card.color.index()] += 1;
        self.played.push(card_id);
    }

    pub fn played_count(&self, color: Color) -> u32 {
        self.color_counts[color.index()]
    }

    pub fn unique_tags(&self) -> u32 {
        self.tag_counts.iter().filter(|&&c| c > 0).count() as u32
    }

    /// Incrémente le TR (comptabilisé pour l'invariant de cohérence).
    pub fn gain_tr(&mut self) {
        self.tr += 1;
        self.tr_increments += 1;
    }
}

/// État complet d'une partie.
pub struct GameState {
    pub rng: StdRng,
    /// Pioche projets (le dessus = fin du Vec).
    pub deck: Vec<u16>,
    pub discard: Vec<u16>,
    /// Paquet corporations restant.
    pub corp_deck: Vec<u16>,
    /// Corporations écartées (mulligan, non choisies).
    pub corp_discard: Vec<u16>,
    pub oceans: [OceanTile; 9],
    pub oceans_revealed: u8,
    /// Niveau de température (0..=19).
    pub temperature: u8,
    /// Niveau d'oxygène (0..=14).
    pub oxygen: u8,
    pub players: [PlayerState; NUM_PLAYERS],
    pub generation: u32,
    pub milestones: [MilestoneSlot; 3],
    pub awards: [AwardKind; 3],
    pub game_over: bool,
    // Instantané planétaire au début de la phase en cours (D6).
    pub snap_temperature: u8,
    pub snap_oxygen: u8,
    pub snap_oceans: u8,
}

impl GameState {
    pub fn all_parameters_maxed(&self) -> bool {
        self.temperature == TEMPERATURE_MAX
            && self.oxygen == OXYGEN_MAX
            && self.oceans_revealed == NUM_OCEANS
    }

    /// Prend l'instantané planétaire de début de phase.
    pub fn snapshot_planet(&mut self) {
        self.snap_temperature = self.temperature;
        self.snap_oxygen = self.oxygen;
        self.snap_oceans = self.oceans_revealed;
    }
}

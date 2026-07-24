//! Sonde d'audit `simulate --probe "<nom>"` : joue UNE carte depuis un état
//! de départ fixe et documenté, par le MÊME chemin de pose que `simulate`
//! (`flow::build_card`), et rapporte le delta d'état.
//!
//! État de départ (prompt §Sonde) : joueur 1 sans corporation, 100 MC,
//! 20 chaleur, 20 plantes, productions 0, TR 5, paramètres globaux au départ
//! (température −30, oxygène 0, 0 océan, infrastructure 0), la carte nommée
//! seule en main. Déterminé de bout en bout :
//! - pioche = toutes les cartes projets v1 sauf la carte sondée, en ordre
//!   d'index croissant (les pioches servent le plus grand index d'abord) ;
//! - tuiles océan dans l'ordre imprimé de `OCEAN_TILES` (non mélangées :
//!   1re tuile = 2 plantes, 2e = 4 MC, …).
//!
//! La sonde FORCE la pose : les prérequis non satisfaits (paramètres globaux,
//! tags) ne bloquent pas, mais les dépenses « spend » sont réellement payées
//! (toujours possibles depuis l'état généreux). `prereq_ok` = prérequis
//! satisfaits dans l'état de départ ; `delta.mc` exclut le prix payé ;
//! `delta.hand` exclut la carte jouée elle-même (journal B6).

use crate::cards::CardsDb;
use crate::flow::{build_card, requirements_met};
use crate::state::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Delta d'état après − avant (hors prix payé, hors carte jouée pour `hand`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProbeDelta {
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    pub hand: i64,
    pub mc_prod: i64,
    pub heat_prod: i64,
    pub plant_prod: i64,
    pub card_prod: i64,
    pub tr: i64,
    pub temperature: i64,
    pub oxygen: i64,
    pub oceans: i64,
    pub forests: i64,
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub card: String,
    pub found: bool,
    pub in_lot: bool,
    pub prereq_ok: bool,
    pub played: bool,
    pub delta: ProbeDelta,
    pub vp: i64,
}

/// Construit l'état de départ fixe de la sonde, la carte `card_id` en main
/// du joueur 0.
fn probe_state(db: &CardsDb, card_id: u16) -> GameState {
    let deck: Vec<u16> = (0..db.projects.len() as u16)
        .filter(|&c| c != card_id && db.projects[c as usize].in_deck_v1)
        .collect();
    let mut players = [PlayerState::new(), PlayerState::new()];
    players[0].mc = 100;
    players[0].heat = 20;
    players[0].plants = 20;
    players[0].hand.push(card_id);

    let mut game = GameState {
        rng: StdRng::seed_from_u64(0),
        deck,
        discard: Vec::new(),
        corp_deck: (0..db.corporations.len() as u16).collect(),
        corp_discard: Vec::new(),
        oceans: OCEAN_TILES,
        oceans_revealed: 0,
        temperature: 0,
        oxygen: 0,
        infrastructure: 0,
        players,
        generation: 1,
        milestones: [MilestoneSlot {
            kind: MilestoneKind::Builder,
            achieved_by: [false; NUM_PLAYERS],
        }; 3],
        awards: [AwardKind::Celebrity; 3],
        game_over: false,
        snap_temperature: 0,
        snap_oxygen: 0,
        snap_oceans: 0,
        snap_infrastructure: 0,
    };
    game.snapshot_planet();
    game
}

/// Joue la carte nommée depuis l'état fixe et rapporte le delta.
pub fn run_probe(db: &CardsDb, name: &str) -> ProbeResult {
    let Some(card_id) = db
        .projects
        .iter()
        .position(|c| c.name == name)
        .map(|i| i as u16)
    else {
        return ProbeResult {
            card: name.to_string(),
            found: false,
            in_lot: false,
            prereq_ok: false,
            played: false,
            delta: ProbeDelta::default(),
            vp: 0,
        };
    };
    let card = &db.projects[card_id as usize];
    let price = card.price;
    let in_lot = db.effects_on && card.effect.is_some();

    let mut game = probe_state(db, card_id);
    let prereq_ok = requirements_met(&game, db, 0, card_id);

    // Instantané avant pose. `hand` : la carte sondée est exclue du compte
    // (elle quitte la main en étant jouée — B6).
    let before = &game.players[0];
    let (mc0, heat0, plants0) = (before.mc, before.heat, before.plants);
    let hand0 = (before.hand.len() - 1) as i64;
    let (mcp0, hp0, pp0, cp0) = (
        before.mc_prod,
        before.heat_prod,
        before.plant_prod,
        before.card_prod,
    );
    let (tr0, forests0) = (before.tr, before.forests);
    let (temp0, oxy0, oc0) = (game.temperature, game.oxygen, game.oceans_revealed);

    // Pose FORCÉE par le chemin réel : la carte est à l'indice 0 de la main.
    // (`affordable` est contourné volontairement — c'est le forçage de la
    // sonde ; `build_card` est le même code que celui de `simulate`.)
    build_card(&mut game, db, 0, 0, 0);
    let played = game.players[0].played.contains(&card_id);

    let after = &game.players[0];
    ProbeResult {
        card: name.to_string(),
        found: true,
        in_lot,
        prereq_ok,
        played,
        delta: ProbeDelta {
            // Hors prix payé : on réintègre le prix déboursé par build_card.
            mc: after.mc + price - mc0,
            heat: after.heat - heat0,
            plants: after.plants - plants0,
            hand: after.hand.len() as i64 - hand0,
            mc_prod: after.mc_prod - mcp0,
            heat_prod: after.heat_prod - hp0,
            plant_prod: after.plant_prod - pp0,
            card_prod: after.card_prod - cp0,
            tr: after.tr - tr0,
            temperature: game.temperature as i64 - temp0 as i64,
            oxygen: game.oxygen as i64 - oxy0 as i64,
            oceans: game.oceans_revealed as i64 - oc0 as i64,
            forests: after.forests - forests0,
        },
        vp: db.projects[card_id as usize].vp,
    }
}

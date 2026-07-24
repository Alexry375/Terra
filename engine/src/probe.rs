//! Sonde d'audit. Deux modes, tous deux par le MÊME chemin de pose que
//! `simulate` (`flow::build_card`) depuis un état de départ fixe et documenté :
//!
//! - `--probe "<A>;<B>;…"` : pose FORCÉE des cartes DANS L'ORDRE. Rapporte le
//!   delta d'état CUMULÉ (hors prix payés) et `paid[]` = prix réellement payé de
//!   chaque carte (après réductions, ≥ 0). Rétro-compatible : une seule carte =
//!   comportement du lot 1. Les réductions et déclencheurs des cartes posées
//!   plus tôt s'appliquent aux poses suivantes (c'est le but de la séquence).
//! - `--probe-action "<nom>"` : pose la carte puis active son action UNE fois si
//!   elle est payable ; le delta isole L'ACTION SEULE (état après pose → après
//!   action, dépenses de l'action incluses).
//!
//! État de départ (prompt §Sonde) : joueur 1 sans corporation, 100 MC,
//! 20 chaleur, 20 plantes, productions 0, TR 5, paramètres globaux au départ,
//! les cartes nommées seules en main (dans l'ordre) ; pioche = cartes v1
//! restantes en ordre d'index ; tuiles océan NON mélangées.

use crate::cards::CardsDb;
use crate::flow::{apply_blue_action, build_card, card_discount, requirements_met};
use crate::policy::RandomPolicy;
use crate::state::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Delta d'état après − avant. Pour `--probe`, cumulé et hors prix payés (le
/// prix réel est réintégré via `paid`) ; pour `--probe-action`, brut (les
/// dépenses de l'action sont comprises).
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
    /// Dernière carte de la séquence.
    pub card: String,
    pub found: bool,
    pub in_lot: bool,
    pub prereq_ok: bool,
    pub played: bool,
    pub delta: ProbeDelta,
    pub vp: i64,
    /// Prix effectivement payé de chaque carte posée, dans l'ordre (≥ 0).
    pub paid: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ProbeActionResult {
    pub card: String,
    pub found: bool,
    pub in_lot: bool,
    pub has_action: bool,
    pub action_applied: bool,
    /// Delta isolant l'action seule (post-pose → post-action).
    pub delta: ProbeDelta,
}

/// Construit l'état de départ fixe de la sonde, `ids` en main du joueur 0 dans
/// l'ordre donné (le reste de la pioche v1 en ordre d'index croissant).
fn probe_state(db: &CardsDb, ids: &[u16]) -> GameState {
    let deck: Vec<u16> = (0..db.projects.len() as u16)
        .filter(|&c| !ids.contains(&c) && db.projects[c as usize].in_deck_v1)
        .collect();
    let mut players = [PlayerState::new(), PlayerState::new()];
    players[0].mc = 100;
    players[0].heat = 20;
    players[0].plants = 20;
    players[0].hand.extend_from_slice(ids);

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
        blue_actions: 0,
        snap_temperature: 0,
        snap_oxygen: 0,
        snap_oceans: 0,
        snap_infrastructure: 0,
    };
    game.snapshot_planet();
    game
}

fn resolve(db: &CardsDb, name: &str) -> Option<u16> {
    db.projects
        .iter()
        .position(|c| c.name == name)
        .map(|i| i as u16)
}

/// Capture des champs suivis pour le delta (ordre : mc, heat, plants, mc_prod,
/// heat_prod, plant_prod, card_prod, tr, forests, temperature, oxygen, oceans).
fn snap(game: &GameState) -> [i64; 12] {
    let pl = &game.players[0];
    [
        pl.mc,
        pl.heat,
        pl.plants,
        pl.mc_prod,
        pl.heat_prod,
        pl.plant_prod,
        pl.card_prod,
        pl.tr,
        pl.forests,
        game.temperature as i64,
        game.oxygen as i64,
        game.oceans_revealed as i64,
    ]
}

fn make_delta(before: &[i64; 12], after: &[i64; 12], hand: i64, mc_extra: i64) -> ProbeDelta {
    ProbeDelta {
        mc: after[0] - before[0] + mc_extra,
        heat: after[1] - before[1],
        plants: after[2] - before[2],
        hand,
        mc_prod: after[3] - before[3],
        heat_prod: after[4] - before[4],
        plant_prod: after[5] - before[5],
        card_prod: after[6] - before[6],
        tr: after[7] - before[7],
        temperature: after[9] - before[9],
        oxygen: after[10] - before[10],
        oceans: after[11] - before[11],
        forests: after[8] - before[8],
    }
}

/// Sonde simple (rétro-compatible) : une carte.
pub fn run_probe(db: &CardsDb, name: &str) -> ProbeResult {
    run_probe_seq(db, &[name])
}

/// Sonde séquence : pose forcée des cartes DANS L'ORDRE, delta cumulé, `paid[]`.
pub fn run_probe_seq(db: &CardsDb, names: &[&str]) -> ProbeResult {
    let last = *names.last().unwrap_or(&"");

    let Some(last_id) = resolve(db, last) else {
        // Dernière carte introuvable : résultat « non trouvée » (comme lot 1).
        return ProbeResult {
            card: last.to_string(),
            found: false,
            in_lot: false,
            prereq_ok: false,
            played: false,
            delta: ProbeDelta::default(),
            vp: 0,
            paid: Vec::new(),
        };
    };

    // Cartes résolues, dans l'ordre (les noms inconnus sont ignorés à la pose).
    let ids: Vec<u16> = names.iter().filter_map(|n| resolve(db, n)).collect();

    let mut game = probe_state(db, &ids);
    // prérequis de la DERNIÈRE carte, dans l'état de départ (comme lot 1).
    let prereq_ok = requirements_met(&game, db, 0, last_id);

    let n = ids.len();
    let hand0 = (game.players[0].hand.len() - n) as i64;
    let before = snap(&game);

    // Pose de chaque carte dans l'ordre (toujours à l'indice 0 de la main : les
    // cartes de la séquence sont en tête, les pioches s'ajoutent en fin).
    let mut paid = Vec::with_capacity(n);
    for &id in &ids {
        let price = db.projects[id as usize].price;
        let disc = card_discount(&game, db, 0, id);
        paid.push((price - disc).max(0));
        build_card(&mut game, db, 0, 0, 0);
    }

    let after = snap(&game);
    let total_paid: i64 = paid.iter().sum();
    let played = game.players[0].played.contains(&last_id);
    let last_card = &db.projects[last_id as usize];
    let hand_delta = game.players[0].hand.len() as i64 - hand0;

    ProbeResult {
        card: last.to_string(),
        found: true,
        in_lot: db.effects_on && last_card.effect.is_some(),
        prereq_ok,
        played,
        delta: make_delta(&before, &after, hand_delta, total_paid),
        vp: last_card.vp,
        paid,
    }
}

/// Sonde action : pose la carte puis active son action une fois si payable ;
/// le delta isole l'action (état après pose → après action).
pub fn run_probe_action(db: &CardsDb, name: &str) -> ProbeActionResult {
    let Some(card_id) = resolve(db, name) else {
        return ProbeActionResult {
            card: name.to_string(),
            found: false,
            in_lot: false,
            has_action: false,
            action_applied: false,
            delta: ProbeDelta::default(),
        };
    };

    let card = &db.projects[card_id as usize];
    let in_lot = db.effects_on && card.effect.is_some();
    let has_action = in_lot && card.effect.and_then(|e| e.action).is_some();

    let mut game = probe_state(db, &[card_id]);
    // Pose (état de référence du delta d'action).
    build_card(&mut game, db, 0, 0, 0);
    let hand_after_pose = game.players[0].hand.len() as i64;
    let before = snap(&game);

    let action_applied = if has_action {
        // Les actions variables tirent leur montant via RandomPolicy sur le RNG
        // déterministe (graine 0) de l'état de sonde.
        let mut pol = RandomPolicy;
        apply_blue_action(&mut game, db, 0, card_id, &mut pol)
    } else {
        false
    };

    let after = snap(&game);
    let hand_delta = game.players[0].hand.len() as i64 - hand_after_pose;
    ProbeActionResult {
        card: name.to_string(),
        found: true,
        in_lot,
        has_action,
        action_applied,
        delta: make_delta(&before, &after, hand_delta, 0),
    }
}

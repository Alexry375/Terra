//! Simulation : parties complètes en politique aléatoire, invariants vérifiés
//! à chaque ronde, empreinte déterministe des états finaux.

use crate::cards::CardsDb;
use crate::flow::{play_round, score, setup_game};
use crate::policy::Policy;
use crate::state::*;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

/// Plafond de sécurité : une partie qui l'atteint est `truncated`, jamais
/// `completed` (clause anti-shortcut du prompt, D12).
pub const MAX_GENERATIONS: u32 = 1000;

/// FNV-1a 64 bits (maison, zéro dépendance — D15).
#[derive(Clone, Copy)]
pub struct Fnv1a(pub u64);

impl Fnv1a {
    pub fn new() -> Fnv1a {
        Fnv1a(0xcbf29ce484222325)
    }
    pub fn write_u8(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.write_u8(b);
        }
    }
    pub fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }
}

impl Default for Fnv1a {
    fn default() -> Self {
        Self::new()
    }
}

/// Empreinte canonique d'un état final (dépend réellement de l'état : ronde,
/// paramètres globaux, ressources/TR/forêts/score, cartes en jeu et en main).
pub fn final_state_hash(game: &GameState, scores: &[i64; NUM_PLAYERS]) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(game.generation as u64);
    h.write_u8(game.temperature);
    h.write_u8(game.oxygen);
    h.write_u8(game.oceans_revealed);
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        h.write_i64(pl.tr);
        h.write_i64(pl.mc);
        h.write_i64(pl.heat);
        h.write_i64(pl.plants);
        h.write_i64(pl.forests);
        h.write_i64(scores[p]);
        let mut played = pl.played.clone();
        played.sort_unstable();
        h.write_u64(played.len() as u64);
        for c in played {
            h.write_u64(c as u64);
        }
        let mut hand = pl.hand.clone();
        hand.sort_unstable();
        h.write_u64(hand.len() as u64);
        for c in hand {
            h.write_u64(c as u64);
        }
    }
    h.0
}

/// Suivi de monotonie entre rondes (paramètres globaux et TR).
pub struct InvariantTracker {
    prev_temperature: u8,
    prev_oxygen: u8,
    prev_oceans: u8,
    prev_tr_increments: [u64; NUM_PLAYERS],
    prev_tr_decrements: [u64; NUM_PLAYERS],
}

impl InvariantTracker {
    pub fn new(game: &GameState) -> InvariantTracker {
        InvariantTracker {
            prev_temperature: game.temperature,
            prev_oxygen: game.oxygen,
            prev_oceans: game.oceans_revealed,
            prev_tr_increments: [
                game.players[0].tr_increments,
                game.players[1].tr_increments,
            ],
            prev_tr_decrements: [
                game.players[0].tr_decrements,
                game.players[1].tr_decrements,
            ],
        }
    }
}

/// Invariants vérifiés à chaque ronde de chaque partie (prompt §ALWAYS) :
/// ressources jamais négatives ; paramètres globaux dans leurs bornes et
/// monotones croissants ; TR cohérent (== 5 + incréments comptés, monotone) ;
/// conservation des cartes (pioche+défausse+mains+en-jeu constante).
pub fn check_invariants(
    game: &GameState,
    db: &CardsDb,
    tracker: &mut InvariantTracker,
) -> Result<(), String> {
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        if pl.mc < 0 || pl.heat < 0 || pl.plants < 0 {
            return Err(format!(
                "joueur {p}: ressource négative (mc={} heat={} plants={})",
                pl.mc, pl.heat, pl.plants
            ));
        }
        // TR : cohérence comptable (les seules baisses licites sont les
        // dépenses « spend n TR » comptées dans tr_decrements) et compteurs
        // monotones — remplace la monotonie brute du TR depuis que la couche
        // d'effets autorise « Requires you to spend 1 TR » (journal B3).
        if pl.tr != STARTING_TR + pl.tr_increments as i64 - pl.tr_decrements as i64 {
            return Err(format!(
                "joueur {p}: TR incohérent ({} != 5 + {} - {})",
                pl.tr, pl.tr_increments, pl.tr_decrements
            ));
        }
        if pl.tr < 0 {
            return Err(format!("joueur {p}: TR négatif"));
        }
        if pl.tr_increments < tracker.prev_tr_increments[p]
            || pl.tr_decrements < tracker.prev_tr_decrements[p]
        {
            return Err(format!("joueur {p}: compteur de TR décroissant"));
        }
    }
    if game.temperature > TEMPERATURE_MAX
        || game.oxygen > OXYGEN_MAX
        || game.oceans_revealed > NUM_OCEANS
    {
        return Err("paramètre global hors bornes".to_string());
    }
    if game.temperature < tracker.prev_temperature
        || game.oxygen < tracker.prev_oxygen
        || game.oceans_revealed < tracker.prev_oceans
    {
        return Err("paramètre global décroissant".to_string());
    }
    let total = game.deck.len()
        + game.discard.len()
        + game.players.iter().map(|p| p.hand.len() + p.played.len()).sum::<usize>();
    if total != db.v1_project_count {
        return Err(format!(
            "conservation des cartes violée: {} != {}",
            total, db.v1_project_count
        ));
    }
    let corps = game.corp_deck.len()
        + game.corp_discard.len()
        + game.players.iter().filter(|p| p.corporation.is_some()).count();
    if corps != db.corporations.len() {
        return Err(format!(
            "conservation des corporations violée: {} != {}",
            corps,
            db.corporations.len()
        ));
    }

    tracker.prev_temperature = game.temperature;
    tracker.prev_oxygen = game.oxygen;
    tracker.prev_oceans = game.oceans_revealed;
    tracker.prev_tr_increments = [
        game.players[0].tr_increments,
        game.players[1].tr_increments,
    ];
    tracker.prev_tr_decrements = [
        game.players[0].tr_decrements,
        game.players[1].tr_decrements,
    ];
    Ok(())
}

pub struct GameOutcome {
    pub completed: bool,
    pub generations: u32,
    pub scores: [i64; NUM_PLAYERS],
    pub violations: u64,
    pub state_hash: u64,
    /// Activations d'actions bleues ayant appliqué un effet (lot 2).
    pub blue_actions: u64,
    /// (C4) Premier joueur de chaque manche réellement jouée, lu sur l'état de
    /// la partie (`GameState::turn_order`).
    pub turn_order: Vec<u8>,
    /// (C4) Alternances observées dans `turn_order`.
    pub turn_order_switches: u64,
    /// (C1) Cartes exclues par l'instantané de début de phase.
    pub prereq_snapshot_blocks: u64,
    /// (C2) Pioches du bonus construction prises avant / après la pose.
    pub draw_before_build: u64,
    pub draw_after_build: u64,
    /// (C3) Cartes défaussées pour payer des cartes Projet.
    pub discard_payments: u64,
    /// (C5) Partie terminée sur une égalité de PV (aucun départage : règle
    /// maison — une égalité reste une égalité).
    pub draw: bool,
}

/// Joue une partie complète (politique fournie), invariants vérifiés à chaque
/// ronde. Une partie arrêtée par le plafond n'est PAS complétée.
pub fn play_game(db: &CardsDb, seed: u64, policy: &mut dyn Policy) -> GameOutcome {
    let mut game = setup_game(db, seed, policy);
    let mut tracker = InvariantTracker::new(&game);
    let mut violations = 0u64;

    while !game.game_over && game.generation <= MAX_GENERATIONS {
        play_round(&mut game, db, policy);
        if let Err(_e) = check_invariants(&game, db, &mut tracker) {
            violations += 1;
        }
    }

    let scores = score(&game, db);
    GameOutcome {
        completed: game.game_over,
        generations: game.generation,
        scores,
        violations,
        state_hash: final_state_hash(&game, &scores),
        blue_actions: game.blue_actions,
        turn_order_switches: game.turn_order_switches(),
        turn_order: game.turn_order.clone(),
        prereq_snapshot_blocks: game.prereq_snapshot_blocks,
        draw_before_build: game.draw_before_build,
        draw_after_build: game.draw_after_build,
        discard_payments: game.discard_payments,
        // (C5) Aucun départage n'est appliqué : deux scores égaux = une égalité.
        draw: scores[0] == scores[1],
    }
}

pub struct SimSummary {
    pub games: u64,
    pub completed: u64,
    pub truncated: u64,
    pub invariant_violations: u64,
    pub avg_generations: f64,
    pub avg_score_p1: f64,
    pub avg_score_p2: f64,
    pub state_hash: u64,
    pub games_per_sec: f64,
    /// Total des activations d'actions bleues sur toutes les parties (lot 2).
    pub blue_actions: u64,
    // ------------------------------------------------- lot 3 (conformité)
    /// (C4) Ordre du tour réellement joué, une liste par partie, dans l'ordre
    /// des parties (`--dump-turn-order` en imprime une ligne par partie).
    pub turn_orders: Vec<Vec<u8>>,
    /// (C4) Somme des alternances d'ordre du tour sur toutes les parties.
    pub turn_order_switches: u64,
    /// (C1) Total des cartes exclues par l'instantané de début de phase.
    pub prereq_snapshot_blocks: u64,
    /// (C2) Totaux des deux moments de pioche du bonus de construction.
    pub draw_before_build: u64,
    pub draw_after_build: u64,
    /// (C3) Total des cartes défaussées pour payer des cartes Projet.
    pub discard_payments: u64,
    /// (C5) Parties terminées sur une égalité de PV.
    pub draws: u64,
}

/// Lance `games` parties aléatoires. Graine unique : un RNG maître seedé par
/// `seed` produit la graine de chaque partie (D11).
pub fn run_simulation(
    db: &CardsDb,
    games: u64,
    seed: u64,
    policy: &mut dyn Policy,
) -> SimSummary {
    let mut master = StdRng::seed_from_u64(seed);
    let mut agg = Fnv1a::new();
    let mut completed = 0u64;
    let mut truncated = 0u64;
    let mut violations = 0u64;
    let mut sum_gens = 0u64;
    let mut sum_p1 = 0i64;
    let mut sum_p2 = 0i64;
    let mut blue_actions = 0u64;
    let mut turn_orders: Vec<Vec<u8>> = Vec::with_capacity(games as usize);
    let mut turn_order_switches = 0u64;
    let mut prereq_snapshot_blocks = 0u64;
    let mut draw_before_build = 0u64;
    let mut draw_after_build = 0u64;
    let mut discard_payments = 0u64;
    let mut draws = 0u64;

    let t0 = std::time::Instant::now();
    for _ in 0..games {
        let game_seed = master.next_u64();
        let out = play_game(db, game_seed, policy);
        if out.completed {
            completed += 1;
        } else {
            truncated += 1;
        }
        violations += out.violations;
        sum_gens += out.generations as u64;
        sum_p1 += out.scores[0];
        sum_p2 += out.scores[1];
        blue_actions += out.blue_actions;
        turn_order_switches += out.turn_order_switches;
        prereq_snapshot_blocks += out.prereq_snapshot_blocks;
        draw_before_build += out.draw_before_build;
        draw_after_build += out.draw_after_build;
        discard_payments += out.discard_payments;
        if out.draw {
            draws += 1;
        }
        turn_orders.push(out.turn_order);
        agg.write_u64(out.state_hash);
    }
    let elapsed = t0.elapsed().as_secs_f64();

    let n = games.max(1) as f64;
    SimSummary {
        games,
        completed,
        truncated,
        invariant_violations: violations,
        avg_generations: sum_gens as f64 / n,
        avg_score_p1: sum_p1 as f64 / n,
        avg_score_p2: sum_p2 as f64 / n,
        state_hash: agg.0,
        games_per_sec: if elapsed > 0.0 { games as f64 / elapsed } else { 0.0 },
        blue_actions,
        turn_orders,
        turn_order_switches,
        prereq_snapshot_blocks,
        draw_before_build,
        draw_after_build,
        discard_payments,
        draws,
    }
}

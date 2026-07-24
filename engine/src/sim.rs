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
    prev_tr: [i64; NUM_PLAYERS],
}

impl InvariantTracker {
    pub fn new(game: &GameState) -> InvariantTracker {
        InvariantTracker {
            prev_temperature: game.temperature,
            prev_oxygen: game.oxygen,
            prev_oceans: game.oceans_revealed,
            prev_tr: [game.players[0].tr, game.players[1].tr],
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
        if pl.tr != STARTING_TR + pl.tr_increments as i64 {
            return Err(format!(
                "joueur {p}: TR incohérent ({} != 5 + {})",
                pl.tr, pl.tr_increments
            ));
        }
        if pl.tr < tracker.prev_tr[p] {
            return Err(format!("joueur {p}: TR décroissant"));
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
    if total != db.projects.len() {
        return Err(format!(
            "conservation des cartes violée: {} != {}",
            total,
            db.projects.len()
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
    tracker.prev_tr = [game.players[0].tr, game.players[1].tr];
    Ok(())
}

pub struct GameOutcome {
    pub completed: bool,
    pub generations: u32,
    pub scores: [i64; NUM_PLAYERS],
    pub violations: u64,
    pub state_hash: u64,
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

    let scores = score(&game);
    GameOutcome {
        completed: game.game_over,
        generations: game.generation,
        scores,
        violations,
        state_hash: final_state_hash(&game, &scores),
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
    }
}

//! Simulation : parties complètes en politique aléatoire, invariants vérifiés
//! à chaque ronde, empreinte déterministe des états finaux.

use crate::cards::CardsDb;
use crate::flow::{play_round, score_parts, setup_game};
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
    /// (lot acier-titane) Comptes de savoir-faire de la manche précédente : ils
    /// ne décroissent jamais (on ne dépense pas un acier).
    prev_steel: [i64; NUM_PLAYERS],
    prev_titanium: [i64; NUM_PLAYERS],
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
            prev_steel: [
                game.players[0].steel_capacity,
                game.players[1].steel_capacity,
            ],
            prev_titanium: [
                game.players[0].titanium_capacity,
                game.players[1].titanium_capacity,
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
        // (lot acier-titane) Les deux champs de compte sont un CACHE : leur
        // seule écriture est `flow::refresh_capacities`, qui recopie la
        // dérivation. Un cache qui dérive en silence serait une seconde source
        // de vérité — exactement ce que I2 interdit. On le recompare donc à la
        // dérivation à CHAQUE manche de CHAQUE partie : 2 × 1000 parties sans
        // violation, c'est la preuve que les deux ne peuvent pas diverger.
        let derived = crate::flow::capacities(db, pl);
        if pl.steel_capacity != derived.steel || pl.titanium_capacity != derived.titanium {
            return Err(format!(
                "joueur {p}: compte de savoir-faire divergent (acier {} != {}, \
                 titane {} != {})",
                pl.steel_capacity, derived.steel, pl.titanium_capacity, derived.titanium
            ));
        }
        // Un savoir-faire est PERMANENT : il ne se dépense pas (NEVER 8), donc
        // le compte ne décroît jamais.
        if pl.steel_capacity < tracker.prev_steel[p] || pl.titanium_capacity < tracker.prev_titanium[p]
        {
            return Err(format!("joueur {p}: compte de savoir-faire décroissant"));
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
    if total != db.deck_project_count {
        return Err(format!(
            "conservation des cartes violée: {} != {}",
            total, db.deck_project_count
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
    tracker.prev_steel = [
        game.players[0].steel_capacity,
        game.players[1].steel_capacity,
    ];
    tracker.prev_titanium = [
        game.players[0].titanium_capacity,
        game.players[1].titanium_capacity,
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
    // ----------------------------------- lot 3 (ressources sur les cartes)
    /// Ressources ajoutées / retirées sur des cartes (unités).
    pub res_added: u64,
    pub res_removed: u64,
    /// Poses de ressources sautées faute de cible valide.
    pub res_targets_missing: u64,
    /// Améliorations de carte Phase demandées et non gérées (0 depuis le
    /// chantier `decouverte-phases`).
    pub phase_upgrades_skipped: u64,
    /// (Découverte) Améliorations de carte Phase accordées, dont bascules A ↔ B.
    pub phase_upgrades_granted: u64,
    pub phase_upgrades_reupgraded: u64,
    /// (Découverte) Bonus de sélectionneur AMÉLIORÉS réellement lus, et
    /// permissions de pose qu'ils ont versées.
    pub upgraded_bonus_applied: u64,
    pub upgraded_extra_builds: u64,
    /// (Découverte) Points distribués par la seule tuile VISIONNAIRE, les deux
    /// joueurs cumulés — lus sur `flow::award_points_split`, le parcours qui
    /// les a réellement distribués.
    pub visionary_award_points: i64,
    /// (boites-1) Cartes sans encodage entrées en jeu.
    pub cards_effects_unhandled: u64,
    /// Points de victoire venant des ressources posées, les deux joueurs.
    pub vp_from_resources: i64,
    // ------------------------------------------- lot 4 (productions dérivées)
    /// Ressources créditées par la PRODUCTION DÉRIVÉE (`flow::phase_production`).
    pub derived_mc: u64,
    pub derived_heat: u64,
    pub derived_plants: u64,
    /// Pas de NT gagnés par `Eff::TrPerTag`.
    pub tr_from_tags: u64,
    /// Cartes supplémentaires piochées en phase Recherche grâce au bonus
    /// permanent (`flow::phase_research`).
    pub research_extra_draws: u64,
    pub extra_builds_granted: u64,
    pub extra_builds_used: u64,
    pub free_builds: u64,
    pub next_card_mods_armed: u64,
    pub next_card_mods_used: u64,
    /// (corpo-1) Chaleur convertie en MC par Helion.
    pub corp_heat_as_mc: u64,
    /// (corpo-1) Forêts payées à prix réduit par Ecoline.
    pub corp_forest_rebates: u64,
    /// (corpo-1) Pas de NT achetés 6 MC par Unmi.
    pub corp_tr_boosts: u64,
    /// (corpo-1) Pas de NT accordés par un déclencheur de corporation.
    pub corp_trigger_tr: u64,
    pub action_phase_bonuses: u64,
    pub action_discard_costs: u64,
    pub draw_discard_discards: u64,
    pub cards_revealed: u64,
    /// (lot cartes-7) Actions standard payées moins cher (Standard Technology).
    pub standard_action_discounts: u64,
    /// (lot cartes-7) MC gagnés par Assembly Lines sur une action de carte.
    pub action_mc_bonuses: u64,
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

    // Le score et ses deux compteurs d'audit sortent d'un SEUL parcours : la
    // part de VISIONNAIRE rapportée ici est celle que ce parcours-là a
    // réellement distribuée, pas un second calcul.
    let (scores, vp_from_resources, visionary_award_points) = score_parts(&game, db);
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
        res_added: game.res_added,
        res_removed: game.res_removed,
        res_targets_missing: game.res_targets_missing,
        phase_upgrades_skipped: game.phase_upgrades_skipped,
        phase_upgrades_granted: game.phase_upgrades_granted,
        phase_upgrades_reupgraded: game.phase_upgrades_reupgraded,
        upgraded_bonus_applied: game.upgraded_bonus_applied,
        upgraded_extra_builds: game.upgraded_extra_builds,
        visionary_award_points,
        cards_effects_unhandled: game.cards_effects_unhandled,
        vp_from_resources,
        derived_mc: game.derived_mc,
        derived_heat: game.derived_heat,
        derived_plants: game.derived_plants,
        tr_from_tags: game.tr_from_tags,
        research_extra_draws: game.research_extra_draws,
        extra_builds_granted: game.extra_builds_granted,
        extra_builds_used: game.extra_builds_used,
        free_builds: game.free_builds,
        next_card_mods_armed: game.next_card_mods_armed,
        next_card_mods_used: game.next_card_mods_used,
        corp_heat_as_mc: game.corp_heat_as_mc,
        corp_forest_rebates: game.corp_forest_rebates,
        corp_tr_boosts: game.corp_tr_boosts,
        corp_trigger_tr: game.corp_trigger_tr,
        action_phase_bonuses: game.action_phase_bonuses,
        action_discard_costs: game.action_discard_costs,
        draw_discard_discards: game.draw_discard_discards,
        cards_revealed: game.cards_revealed,
        standard_action_discounts: game.standard_action_discounts,
        action_mc_bonuses: game.action_mc_bonuses,
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
    // ----------------------------------- lot 3 (ressources sur les cartes)
    /// Totaux sur toutes les parties, agrégés depuis `GameOutcome` — donc
    /// depuis les compteurs incrémentés dans les services d'ajout/retrait au
    /// moment réel de l'opération, jamais recalculés ici.
    pub res_added: u64,
    pub res_removed: u64,
    pub res_targets_missing: u64,
    pub phase_upgrades_skipped: u64,
    /// (Découverte) Totaux du mécanisme des cartes Phase améliorées, agrégés
    /// depuis `GameOutcome` — donc depuis les compteurs incrémentés à l'endroit
    /// exact du mécanisme, jamais recalculés ici.
    pub phase_upgrades_granted: u64,
    pub phase_upgrades_reupgraded: u64,
    pub upgraded_bonus_applied: u64,
    pub upgraded_extra_builds: u64,
    pub visionary_award_points: i64,
    /// (boites-1) Cartes sans encodage entrées en jeu, toutes parties cumulées.
    pub cards_effects_unhandled: u64,
    pub vp_from_resources: i64,
    // ------------------------------------------- lot 4 (productions dérivées)
    /// Ressources créditées par la PRODUCTION DÉRIVÉE (`flow::phase_production`).
    pub derived_mc: u64,
    pub derived_heat: u64,
    pub derived_plants: u64,
    /// Pas de NT gagnés par `Eff::TrPerTag`.
    pub tr_from_tags: u64,
    /// Cartes supplémentaires piochées en phase Recherche grâce au bonus
    /// permanent (`flow::phase_research`).
    pub research_extra_draws: u64,
    pub extra_builds_granted: u64,
    pub extra_builds_used: u64,
    pub free_builds: u64,
    pub next_card_mods_armed: u64,
    pub next_card_mods_used: u64,
    /// (corpo-1) Chaleur convertie en MC par Helion.
    pub corp_heat_as_mc: u64,
    /// (corpo-1) Forêts payées à prix réduit par Ecoline.
    pub corp_forest_rebates: u64,
    /// (corpo-1) Pas de NT achetés 6 MC par Unmi.
    pub corp_tr_boosts: u64,
    /// (corpo-1) Pas de NT accordés par un déclencheur de corporation.
    pub corp_trigger_tr: u64,
    pub action_phase_bonuses: u64,
    pub action_discard_costs: u64,
    pub draw_discard_discards: u64,
    pub cards_revealed: u64,
    /// (lot cartes-7) Actions standard payées moins cher (Standard Technology).
    pub standard_action_discounts: u64,
    /// (lot cartes-7) MC gagnés par Assembly Lines sur une action de carte.
    pub action_mc_bonuses: u64,
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
    let mut res_added = 0u64;
    let mut res_removed = 0u64;
    let mut res_targets_missing = 0u64;
    let mut phase_upgrades_skipped = 0u64;
    let mut phase_upgrades_granted = 0u64;
    let mut phase_upgrades_reupgraded = 0u64;
    let mut upgraded_bonus_applied = 0u64;
    let mut upgraded_extra_builds = 0u64;
    let mut visionary_award_points = 0i64;
    let mut cards_effects_unhandled = 0u64;
    let mut vp_from_resources = 0i64;
    let mut derived_mc = 0u64;
    let mut derived_heat = 0u64;
    let mut derived_plants = 0u64;
    let mut tr_from_tags = 0u64;
    let mut research_extra_draws = 0u64;
    let mut extra_builds_granted = 0u64;
    let mut extra_builds_used = 0u64;
    let mut free_builds = 0u64;
    let mut next_card_mods_armed = 0u64;
    let mut next_card_mods_used = 0u64;
    let mut corp_heat_as_mc = 0u64;
    let mut corp_forest_rebates = 0u64;
    let mut corp_tr_boosts = 0u64;
    let mut corp_trigger_tr = 0u64;
    let mut action_phase_bonuses = 0u64;
    let mut action_discard_costs = 0u64;
    let mut draw_discard_discards = 0u64;
    let mut cards_revealed = 0u64;
    let mut standard_action_discounts = 0u64;
    let mut action_mc_bonuses = 0u64;

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
        res_added += out.res_added;
        res_removed += out.res_removed;
        res_targets_missing += out.res_targets_missing;
        phase_upgrades_skipped += out.phase_upgrades_skipped;
        phase_upgrades_granted += out.phase_upgrades_granted;
        phase_upgrades_reupgraded += out.phase_upgrades_reupgraded;
        upgraded_bonus_applied += out.upgraded_bonus_applied;
        upgraded_extra_builds += out.upgraded_extra_builds;
        visionary_award_points += out.visionary_award_points;
        cards_effects_unhandled += out.cards_effects_unhandled;
        vp_from_resources += out.vp_from_resources;
        derived_mc += out.derived_mc;
        derived_heat += out.derived_heat;
        derived_plants += out.derived_plants;
        tr_from_tags += out.tr_from_tags;
        research_extra_draws += out.research_extra_draws;
        extra_builds_granted += out.extra_builds_granted;
        extra_builds_used += out.extra_builds_used;
        free_builds += out.free_builds;
        next_card_mods_armed += out.next_card_mods_armed;
        next_card_mods_used += out.next_card_mods_used;
        corp_heat_as_mc += out.corp_heat_as_mc;
        corp_forest_rebates += out.corp_forest_rebates;
        corp_tr_boosts += out.corp_tr_boosts;
        corp_trigger_tr += out.corp_trigger_tr;
        action_phase_bonuses += out.action_phase_bonuses;
        action_discard_costs += out.action_discard_costs;
        draw_discard_discards += out.draw_discard_discards;
        cards_revealed += out.cards_revealed;
        standard_action_discounts += out.standard_action_discounts;
        action_mc_bonuses += out.action_mc_bonuses;
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
        res_added,
        res_removed,
        res_targets_missing,
        phase_upgrades_skipped,
        phase_upgrades_granted,
        phase_upgrades_reupgraded,
        upgraded_bonus_applied,
        upgraded_extra_builds,
        visionary_award_points,
        cards_effects_unhandled,
        vp_from_resources,
        derived_mc,
        derived_heat,
        derived_plants,
        tr_from_tags,
        research_extra_draws,
        extra_builds_granted,
        extra_builds_used,
        free_builds,
        next_card_mods_armed,
        next_card_mods_used,
        corp_heat_as_mc,
        corp_forest_rebates,
        corp_tr_boosts,
        corp_trigger_tr,
        action_phase_bonuses,
        action_discard_costs,
        draw_discard_discards,
        cards_revealed,
        standard_action_discounts,
        action_mc_bonuses,
    }
}

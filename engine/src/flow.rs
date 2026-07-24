//! Flux de jeu : mise en place (avec les deux mulligans maison), boucle de
//! ronde (planification, phases I-V, étape de fin), fin de partie et score.
//!
//! Sources : livret de base (planeringssteget p.10, faserna p.11-15,
//! avslutningssteget p.16, spelets slut p.16-17), livret Discovery p.3
//! (milestones/awards), moteur Java (`StateTransitionService`,
//! `CollectIncomeTurnProcessor`, `DraftCardsTurnProcessor`,
//! `PickPhaseProcessor`, `TerraformingService`, `MarsGame.assignMilestones`).

use crate::cards::{CardsDb, Color, Tag, VpKind};
use crate::effects::{
    Action, ActionCost, ActionEff, Eff, GlobalTrigger, Req, TrigGain,
};
use crate::policy::{ActionOpt, ConstructionBonus, Policy};
use crate::state::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Mélange de Fisher-Yates (in place) via le RNG de la partie.
fn shuffle<T>(v: &mut [T], rng: &mut StdRng) {
    for i in (1..v.len()).rev() {
        let j = rng.gen_range(0..=i);
        v.swap(i, j);
    }
}

/// Pioche une carte ; remélange la défausse si la pioche est vide (livret p.15).
pub fn draw_card(game: &mut GameState) -> Option<u16> {
    if game.deck.is_empty() && !game.discard.is_empty() {
        std::mem::swap(&mut game.deck, &mut game.discard);
        let mut deck = std::mem::take(&mut game.deck);
        shuffle(&mut deck, &mut game.rng);
        game.deck = deck;
    }
    game.deck.pop()
}

fn draw_n(game: &mut GameState, n: usize, out: &mut Vec<u16>) {
    out.clear();
    for _ in 0..n {
        match draw_card(game) {
            Some(c) => out.push(c),
            None => break,
        }
    }
}

/// Mise en place complète d'une partie, règles maison incluses.
///
/// Ordre (règles maison d'Alexis, prompt) :
/// 1. 2 corporations données à chaque joueur ;
/// 2. mulligan corporations (les 2 ou aucune) AVANT les cartes projets ;
/// 3. 8 cartes projets chacun ;
/// 4. mulligan projets (les 8 ou aucune, en une fois) ;
/// 5. choix final de corporation (1 parmi 2), cartes projets en main.
pub fn setup_game(db: &CardsDb, seed: u64, policy: &mut dyn Policy) -> GameState {
    let mut rng = StdRng::seed_from_u64(seed);

    // Pioche v1 uniquement : les cartes hors pioche (in_deck_v1 == false)
    // restent accessibles à la sonde/aux tests mais jamais distribuées.
    let mut deck: Vec<u16> = (0..db.projects.len() as u16)
        .filter(|&c| db.projects[c as usize].in_deck_v1)
        .collect();
    shuffle(&mut deck, &mut rng);
    let mut corp_deck: Vec<u16> = (0..db.corporations.len() as u16).collect();
    shuffle(&mut corp_deck, &mut rng);

    let mut oceans = OCEAN_TILES;
    shuffle(&mut oceans, &mut rng);

    let mut game = GameState {
        rng,
        deck,
        discard: Vec::new(),
        corp_deck,
        corp_discard: Vec::new(),
        oceans,
        oceans_revealed: 0,
        temperature: 0,
        oxygen: 0,
        infrastructure: 0,
        players: [PlayerState::new(), PlayerState::new()],
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

    // Milestones/awards : 3 + 3 tirés des pools (Discovery p.2 « reveal three »).
    let mut mpool = MILESTONE_POOL;
    shuffle(&mut mpool, &mut game.rng);
    for i in 0..3 {
        game.milestones[i] = MilestoneSlot {
            kind: mpool[i],
            achieved_by: [false; NUM_PLAYERS],
        };
    }
    let mut apool = AWARD_POOL;
    shuffle(&mut apool, &mut game.rng);
    for i in 0..3 {
        game.awards[i] = apool[i];
    }

    // 1. Deux corporations chacun.
    let mut corps: [Vec<u16>; NUM_PLAYERS] = [Vec::new(), Vec::new()];
    for p in 0..NUM_PLAYERS {
        for _ in 0..2 {
            corps[p].push(game.corp_deck.pop().expect("paquet corporations épuisé"));
        }
    }

    // 2. Mulligan corporations — règle maison n°1 (avant les cartes projets).
    for p in 0..NUM_PLAYERS {
        if policy.corp_mulligan(&mut game.rng, p, &corps[p]) {
            for c in corps[p].drain(..) {
                game.corp_discard.push(c);
            }
            for _ in 0..2 {
                corps[p].push(game.corp_deck.pop().expect("paquet corporations épuisé"));
            }
        }
    }

    // 3. Huit cartes projets chacun (livret setup + Constants.DEFAULT_START_HAND_SIZE).
    let mut buf = Vec::with_capacity(STARTING_HAND);
    for p in 0..NUM_PLAYERS {
        draw_n(&mut game, STARTING_HAND, &mut buf);
        game.players[p].hand.append(&mut buf);
    }

    // 4. Mulligan projets — règle maison n°2 (les 8 ou aucune, en une fois).
    for p in 0..NUM_PLAYERS {
        let hand_snapshot = game.players[p].hand.clone();
        if policy.project_mulligan(&mut game.rng, p, &hand_snapshot) {
            let old: Vec<u16> = game.players[p].hand.drain(..).collect();
            game.discard.extend(old);
            draw_n(&mut game, STARTING_HAND, &mut buf);
            game.players[p].hand.append(&mut buf);
        }
    }

    // 5. Choix final de corporation, cartes projets en main.
    for p in 0..NUM_PLAYERS {
        let pick = policy.pick_corporation(&mut game.rng, p, &corps[p]);
        assert!(pick < corps[p].len(), "choix de corporation hors bornes");
        let chosen = corps[p].remove(pick);
        for other in corps[p].drain(..) {
            game.corp_discard.push(other);
        }
        let corp = &db.corporations[chosen as usize];
        game.players[p].corporation = Some(chosen);
        game.players[p].mc = corp.starting_mc; // stub D3 : MC de départ = price
        for t in &corp.tags {
            if let Some(i) = t.index() {
                game.players[p].tag_counts[i] += 1;
            }
        }
    }

    game
}

/// Phases autorisées cette ronde pour un joueur : 1-5 moins la phase de la
/// ronde précédente (livret p.10 : « inte välja samma faskort två gånger på
/// raken » ; Java `TurnService` ligne 80).
pub fn allowed_phases(player: &PlayerState) -> Vec<u8> {
    (1u8..=5)
        .filter(|ph| player.previous_phase != Some(*ph))
        .collect()
}

fn effective_cost(price: i64, discount: i64) -> i64 {
    (price - discount).max(0)
}

/// (A) Réduction de coût applicable à une carte donnée pour un joueur donné :
/// somme des réductions de TOUTES ses cartes persistantes déjà en jeu (lot 2).
/// Service UNIQUE consommé par `affordable` (affordabilité) ET `build_card`
/// (paiement) — jamais deux logiques parallèles. Calculée avant la mise en jeu
/// de la carte, donc une carte ne se réduit jamais elle-même. 0 si effets coupés.
pub fn card_discount(game: &GameState, db: &CardsDb, p: usize, card_id: u16) -> i64 {
    if !db.effects_on {
        return 0;
    }
    let tags = &db.projects[card_id as usize].tags;
    let mut d = 0;
    for &owned in &game.players[p].played {
        if let Some(spec) = db.projects[owned as usize].effect {
            for r in spec.reductions {
                d += r.amount_for(tags);
            }
        }
    }
    d
}

/// Les prérequis de la carte sont-ils satisfaits dans l'état courant ?
/// (Paramètres globaux au niveau COURANT, tags en jeu, capacité de payer les
/// dépenses « spend ».) Carte hors lot ou effets coupés : toujours vrai.
pub fn requirements_met(game: &GameState, db: &CardsDb, p: usize, card_id: u16) -> bool {
    if !db.effects_on {
        return true;
    }
    let Some(spec) = db.projects[card_id as usize].effect else {
        return true;
    };
    let pl = &game.players[p];
    spec.reqs.iter().all(|req| match *req {
        Req::TempMin(n) => game.temperature >= n,
        Req::TempMax(n) => game.temperature <= n,
        Req::OxyMin(n) => game.oxygen >= n,
        Req::OceanMin(n) => game.oceans_revealed >= n,
        Req::OceanMax(n) => game.oceans_revealed <= n,
        Req::Tags(tag, n) => {
            tag.index().map_or(false, |i| pl.tag_counts[i] >= n as u32)
        }
        Req::SpendHeat(n) => pl.heat >= n,
        Req::SpendPlants(n) => pl.plants >= n,
        Req::SpendTr(n) => pl.tr >= n,
    })
}

/// Indices de main constructibles pour une couleur donnée : paiement MC (D9)
/// ET prérequis de la couche d'effets satisfaits.
fn affordable(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    colors: &[Color],
    discount: i64,
) -> Vec<usize> {
    let player = &game.players[p];
    player
        .hand
        .iter()
        .enumerate()
        .filter(|(_, &c)| {
            let card = &db.projects[c as usize];
            colors.contains(&card.color)
                && effective_cost(card.price, discount + card_discount(game, db, p, c))
                    <= player.mc
                && requirements_met(game, db, p, c)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Applique les dépenses de prérequis puis les effets de pose d'une carte du
/// lot. Appelé uniquement depuis `build_card` (même chemin pour `simulate`,
/// la sonde et les tests). Les hausses de paramètres réutilisent les
/// fonctions du squelette (TR + caps sur l'instantané de phase).
fn apply_card_effects(game: &mut GameState, db: &CardsDb, p: usize, card_id: u16) {
    let Some(spec) = db.projects[card_id as usize].effect else {
        return;
    };
    // 1. Dépenses de pose (« Requires you to spend … »).
    for req in spec.reqs {
        match *req {
            Req::SpendHeat(n) => {
                assert!(game.players[p].heat >= n, "pose sans la chaleur à dépenser");
                game.players[p].heat -= n;
            }
            Req::SpendPlants(n) => {
                assert!(game.players[p].plants >= n, "pose sans les plantes à dépenser");
                game.players[p].plants -= n;
            }
            Req::SpendTr(n) => game.players[p].spend_tr(n),
            _ => {}
        }
    }
    // 2. Effets.
    for eff in spec.effects {
        match *eff {
            Eff::Mc(n) => game.players[p].mc += n,
            Eff::Heat(n) => game.players[p].heat += n,
            Eff::Plants(n) => game.players[p].plants += n,
            Eff::Draw(n) => {
                for _ in 0..n {
                    if let Some(c) = draw_card(game) {
                        game.players[p].hand.push(c);
                    }
                }
            }
            Eff::McProd(n) => game.players[p].mc_prod += n,
            Eff::HeatProd(n) => game.players[p].heat_prod += n,
            Eff::PlantProd(n) => game.players[p].plant_prod += n,
            Eff::CardProd(n) => game.players[p].card_prod += n,
            Eff::Temperature(n) => {
                for _ in 0..n {
                    raise_temperature(game, db, p);
                }
            }
            Eff::Oxygen(n) => {
                for _ in 0..n {
                    raise_oxygen(game, p);
                }
            }
            Eff::Ocean(n) => {
                for _ in 0..n {
                    reveal_ocean(game, db, p);
                }
            }
            Eff::Tr(n) => {
                for _ in 0..n {
                    game.players[p].gain_tr();
                }
            }
            Eff::Infrastructure(n) => {
                for _ in 0..n {
                    raise_infrastructure(game, p);
                }
            }
            Eff::PlantsIfTags(tag, min, gain) => {
                let i = tag.index().expect("tag conditionnel non compté");
                if game.players[p].tag_counts[i] >= min as u32 {
                    game.players[p].plants += gain;
                }
            }
        }
    }
}

/// Construit la carte d'indice de main `idx` : paie le prix, entre en jeu
/// (tags/couleur), puis applique dépenses + effets du lot si les effets sont
/// activés. Chemin UNIQUE de pose (simulate, sonde, tests).
pub fn build_card(game: &mut GameState, db: &CardsDb, p: usize, idx: usize, discount: i64) {
    let card_id = game.players[p].hand.remove(idx);
    // Réduction totale = remise de phase (sélectionneur) + réductions des cartes
    // en jeu (service unique `card_discount`, calculé AVANT la mise en jeu).
    let total_discount = discount + card_discount(game, db, p, card_id);
    let cost = effective_cost(db.projects[card_id as usize].price, total_discount);
    assert!(cost >= 0, "prix payé négatif (réduction non plafonnée)");
    assert!(game.players[p].mc >= cost, "construction sans les MC requis");
    game.players[p].mc -= cost;
    game.players[p].put_in_play(card_id, db);
    if db.effects_on {
        // Effet propre de la carte, puis déclencheurs « When you play … » de
        // toutes les cartes persistantes en jeu (la carte incluse si applicable).
        apply_card_effects(game, db, p, card_id);
        fire_play_triggers(game, db, p, card_id);
    }
}

/// (B) Déclencheurs de pose : évalués à la pose de `played_id`, sur les tags de
/// la carte posée, pour toutes les cartes persistantes en jeu du joueur `p`
/// (la carte elle-même incluse ssi son déclencheur porte `include_self`).
/// Chemin unique `build_card` (simulate, sonde, tests).
fn fire_play_triggers(game: &mut GameState, db: &CardsDb, p: usize, played_id: u16) {
    let played_tags = db.projects[played_id as usize].tags.clone();
    let sources = game.players[p].played.clone();
    for src in sources {
        let Some(spec) = db.projects[src as usize].effect else {
            continue;
        };
        for trig in spec.play_triggers {
            if src == played_id && !trig.include_self {
                continue;
            }
            let matched = trig.cond.matched_tags(&played_tags);
            if matched == 0 {
                continue;
            }
            let mult = if trig.scale_by_matched_tags {
                matched as i64
            } else {
                1
            };
            for g in trig.gains {
                apply_trig_gain(game, p, *g, mult);
            }
        }
    }
}

/// Applique un gain de déclencheur `mult` fois (facteur = nb de tags satisfaits
/// pour les déclencheurs « par tag », 1 sinon).
fn apply_trig_gain(game: &mut GameState, p: usize, g: TrigGain, mult: i64) {
    match g {
        TrigGain::Heat(n) => game.players[p].heat += n * mult,
        TrigGain::Plants(n) => game.players[p].plants += n * mult,
        TrigGain::Draw(n) => {
            for _ in 0..(n as i64 * mult) {
                if let Some(c) = draw_card(game) {
                    game.players[p].hand.push(c);
                }
            }
        }
    }
}

/// (B) Déclencheurs globaux du joueur agissant, fixés à une hausse effective de
/// paramètre (Volcanic Soil sur température, Arctic Algae sur océan). Java itère
/// `player.getPlayed()` du joueur qui provoque la hausse.
fn fire_global_trigger(game: &mut GameState, db: &CardsDb, p: usize, is_temperature: bool) {
    if !db.effects_on {
        return;
    }
    // Collecte d'abord (lecture seule), applique ensuite (aucune allocation si
    // le joueur n'a aucune carte à déclencheur global — cas courant).
    let mut pending: Vec<TrigGain> = Vec::new();
    for &src in &game.players[p].played {
        if let Some(spec) = db.projects[src as usize].effect {
            for g in spec.global_triggers {
                match *g {
                    GlobalTrigger::OnRaiseTemperature(gs) if is_temperature => {
                        pending.extend_from_slice(gs)
                    }
                    GlobalTrigger::OnFlipOcean(gs) if !is_temperature => {
                        pending.extend_from_slice(gs)
                    }
                    _ => {}
                }
            }
        }
    }
    for g in pending {
        apply_trig_gain(game, p, g, 1);
    }
}

/// Hausse d'oxygène : cap sur l'instantané de début de phase (D6). TR accordé
/// si l'instantané le permet, niveau réel saturé au max.
fn raise_oxygen(game: &mut GameState, p: usize) {
    if game.snap_oxygen >= OXYGEN_MAX {
        return;
    }
    if game.oxygen < OXYGEN_MAX {
        game.oxygen += 1;
    }
    game.players[p].gain_tr();
}

/// Hausse d'infrastructure (extension Grain Silos, journal B2) : par pas,
/// +1 TR et pioche 1 carte (sémantique Java `increaseInfrastructure`),
/// cap sur l'instantané de phase comme les autres paramètres.
fn raise_infrastructure(game: &mut GameState, p: usize) {
    if game.snap_infrastructure >= INFRASTRUCTURE_MAX {
        return;
    }
    if game.infrastructure < INFRASTRUCTURE_MAX {
        game.infrastructure += 1;
    }
    game.players[p].gain_tr();
    if let Some(c) = draw_card(game) {
        game.players[p].hand.push(c);
    }
}

fn raise_temperature(game: &mut GameState, db: &CardsDb, p: usize) {
    if game.snap_temperature >= TEMPERATURE_MAX {
        return;
    }
    if game.temperature < TEMPERATURE_MAX {
        game.temperature += 1;
    }
    game.players[p].gain_tr();
    // (B) déclencheurs « When you raise the temperature » du joueur agissant.
    fire_global_trigger(game, db, p, true);
}

/// Révèle un océan : bonus de la tuile + TR. Au-delà du 9e dans la phase du
/// max : bonus de la dernière tuile révélée (livret p.14, fallback Java).
fn reveal_ocean(game: &mut GameState, db: &CardsDb, p: usize) {
    if game.snap_oceans >= NUM_OCEANS {
        return;
    }
    let tile = if game.oceans_revealed < NUM_OCEANS {
        let t = game.oceans[game.oceans_revealed as usize];
        game.oceans_revealed += 1;
        t
    } else {
        game.oceans[(NUM_OCEANS - 1) as usize]
    };
    game.players[p].mc += tile.mc;
    game.players[p].plants += tile.plants;
    for _ in 0..tile.cards {
        if let Some(c) = draw_card(game) {
            game.players[p].hand.push(c);
        }
    }
    game.players[p].gain_tr();
    // (B) déclencheurs « When you flip an ocean tile » du joueur agissant.
    fire_global_trigger(game, db, p, false);
}

/// Forêt : 8 plantes ou 20 MC → +1 forêt (VP), oxygène +1 si l'instantané le
/// permet (livret p.14 ; Java `buildForest`).
fn build_forest(game: &mut GameState, p: usize, with_plants: bool) {
    if with_plants {
        assert!(game.players[p].plants >= FOREST_PLANT_COST);
        game.players[p].plants -= FOREST_PLANT_COST;
    } else {
        assert!(game.players[p].mc >= FOREST_MC_COST);
        game.players[p].mc -= FOREST_MC_COST;
    }
    game.players[p].forests += 1;
    raise_oxygen(game, p);
}

fn action_options(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    remaining_blue: &[u16],
    out: &mut Vec<ActionOpt>,
) {
    out.clear();
    let pl = &game.players[p];
    for &c in remaining_blue {
        out.push(ActionOpt::BlueAction(c));
    }
    if pl.plants >= FOREST_PLANT_COST {
        out.push(ActionOpt::ForestWithPlants);
    }
    if pl.mc >= FOREST_MC_COST {
        out.push(ActionOpt::ForestWithMc);
    }
    if pl.heat >= TEMPERATURE_HEAT_COST && game.snap_temperature < TEMPERATURE_MAX {
        out.push(ActionOpt::TemperatureWithHeat);
    }
    if pl.mc >= TEMPERATURE_MC_COST && game.snap_temperature < TEMPERATURE_MAX {
        out.push(ActionOpt::TemperatureWithMc);
    }
    if pl.mc >= OCEAN_MC_COST && game.snap_oceans < NUM_OCEANS {
        out.push(ActionOpt::OceanWithMc);
    }
    if !pl.hand.is_empty() {
        out.push(ActionOpt::SellCard);
    }
    let _ = db;
}

/// (C) Applique l'action réelle d'une carte bleue en jeu (lot 2). Renvoie `true`
/// si un effet a réellement été appliqué (coût payé / effet produit) — seul cas
/// où le compteur `blue_actions` est incrémenté. Renvoie `false` si la carte n'a
/// pas d'action, si le coût fixe n'est pas payable, ou si une action variable
/// tire un montant nul. Les montants « up to X » sont tirés par la politique.
pub(crate) fn apply_blue_action(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    policy: &mut dyn Policy,
) -> bool {
    let Some(action) = db.projects[card_id as usize].effect.and_then(|e| e.action) else {
        return false;
    };
    match action {
        Action::Fixed { cost, effect } => {
            // Payabilité (Java : *ActionValidator).
            for c in cost {
                let ok = match *c {
                    ActionCost::Heat(n) => game.players[p].heat >= n,
                    ActionCost::Mc(n) => game.players[p].mc >= n,
                    ActionCost::Plants(n) => game.players[p].plants >= n,
                };
                if !ok {
                    return false;
                }
            }
            for c in cost {
                match *c {
                    ActionCost::Heat(n) => game.players[p].heat -= n,
                    ActionCost::Mc(n) => game.players[p].mc -= n,
                    ActionCost::Plants(n) => game.players[p].plants -= n,
                }
            }
            for e in effect {
                match *e {
                    ActionEff::Draw(n) => {
                        for _ in 0..n {
                            if let Some(c) = draw_card(game) {
                                game.players[p].hand.push(c);
                            }
                        }
                    }
                    ActionEff::Plants(n) => game.players[p].plants += n,
                    ActionEff::Mc(n) => game.players[p].mc += n,
                    ActionEff::Tr(n) => {
                        for _ in 0..n {
                            game.players[p].gain_tr();
                        }
                    }
                    ActionEff::Oxygen(n) => {
                        for _ in 0..n {
                            raise_oxygen(game, p);
                        }
                    }
                }
            }
            true
        }
        // « Spend any amount of heat to gain that amount of MC. »
        Action::HeatToMc => {
            let max = game.players[p].heat;
            let amt = policy.action_amount(&mut game.rng, p, max).clamp(0, max);
            if amt <= 0 {
                return false;
            }
            game.players[p].heat -= amt;
            game.players[p].mc += amt;
            true
        }
        // « Spend max(0, base − nb tags per_tag) MC → flip un océan. »
        Action::FlipOceanTagDiscount { base, per_tag } => {
            if game.snap_oceans >= NUM_OCEANS {
                return false;
            }
            let n = per_tag
                .index()
                .map_or(0, |i| game.players[p].tag_counts[i] as i64);
            let cost = (base - n).max(0);
            if game.players[p].mc < cost {
                return false;
            }
            game.players[p].mc -= cost;
            reveal_ocean(game, db, p);
            true
        }
        // « Spend base − (reduction si ≥ threshold cartes bleues) MC → +1 temp. »
        Action::RaiseTempBlueDiscount {
            base,
            threshold,
            reduction,
        } => {
            if game.snap_temperature >= TEMPERATURE_MAX {
                return false;
            }
            let blue = game.players[p].played_count(Color::Blue);
            let cost = base - if blue >= threshold { reduction } else { 0 };
            if game.players[p].mc < cost {
                return false;
            }
            game.players[p].mc -= cost;
            raise_temperature(game, db, p);
            true
        }
        // « Discard up to `cap` cards, draw that many. »
        Action::DiscardDraw(cap) => {
            let max = (game.players[p].hand.len() as i64).min(cap);
            let amt = policy.action_amount(&mut game.rng, p, max).clamp(0, max);
            if amt <= 0 {
                return false;
            }
            for _ in 0..amt {
                let n = game.players[p].hand.len();
                let i = game.rng.gen_range(0..n);
                let card = game.players[p].hand.remove(i);
                game.discard.push(card);
            }
            for _ in 0..amt {
                if let Some(c) = draw_card(game) {
                    game.players[p].hand.push(c);
                }
            }
            true
        }
    }
}

/// Phase I — Développement (livret p.11) : chacun peut jouer 1 carte verte ;
/// sélectionneur : -3 MC.
fn phase_development(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    for p in 0..NUM_PLAYERS {
        let discount = if game.players[p].chosen_phase == 1 {
            DEV_SELECTOR_DISCOUNT
        } else {
            0
        };
        let opts = affordable(game, db, p, &[Color::Green], discount);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card(game, db, p, idx, discount);
        }
    }
}

/// Phase II — Construction (livret p.12) : chacun peut jouer 1 carte
/// bleue/rouge ; sélectionneur : piocher 1 carte OU en jouer une 2e.
fn phase_construction(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    const BR: [Color; 2] = [Color::Blue, Color::Red];
    for p in 0..NUM_PLAYERS {
        let opts = affordable(game, db, p, &BR, 0);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card(game, db, p, idx, 0);
        }
        if game.players[p].chosen_phase == 2 {
            match policy.construction_bonus(&mut game.rng, p) {
                ConstructionBonus::DrawCard => {
                    if let Some(c) = draw_card(game) {
                        game.players[p].hand.push(c);
                    }
                }
                ConstructionBonus::SecondBuild => {
                    let opts = affordable(game, db, p, &BR, 0);
                    if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
                        assert!(opts.contains(&idx), "choix de construction hors options");
                        build_card(game, db, p, idx, 0);
                    }
                }
            }
        }
    }
}

/// Phase III — Action (livret p.14) : actions bleues (stubs neutres, une fois
/// chacune ; sélectionneur : une répétition), actions standard à volonté,
/// puis conversions OBLIGATOIRES de fin de phase (D7).
fn phase_action(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    let mut options: Vec<ActionOpt> = Vec::with_capacity(16);
    for p in 0..NUM_PLAYERS {
        // Chaque carte bleue jouée offre son action une fois par phase.
        let mut remaining_blue: Vec<u16> = game.players[p]
            .played
            .iter()
            .copied()
            .filter(|&c| db.projects[c as usize].color == Color::Blue)
            .collect();
        let mut extra = if game.players[p].chosen_phase == 3 {
            game.players[p].extra_blue_activations
        } else {
            0
        };

        loop {
            action_options(game, db, p, &remaining_blue, &mut options);
            let Some(choice) = policy.action_choice(&mut game.rng, p, &options) else {
                break;
            };
            assert!(choice < options.len(), "choix d'action hors options");
            match options[choice] {
                ActionOpt::BlueAction(card) => {
                    // (C) Effets ON : l'action réelle de la carte bleue est
                    // appliquée si elle est définie et payable ; le compteur
                    // d'audit n'est incrémenté que si un effet a réellement eu
                    // lieu. Effets OFF : no-op (squelette « à blanc »).
                    if db.effects_on && apply_blue_action(game, db, p, card, policy) {
                        game.blue_actions += 1;
                    }
                    // L'activation est consommée dans tous les cas.
                    if let Some(pos) = remaining_blue.iter().position(|&c| c == card) {
                        remaining_blue.remove(pos);
                    }
                    // Bonus du sélectionneur : une activation supplémentaire.
                    if extra > 0 {
                        extra -= 1;
                        remaining_blue.push(card);
                    }
                }
                ActionOpt::ForestWithPlants => build_forest(game, p, true),
                ActionOpt::ForestWithMc => build_forest(game, p, false),
                ActionOpt::TemperatureWithHeat => {
                    game.players[p].heat -= TEMPERATURE_HEAT_COST;
                    raise_temperature(game, db, p);
                }
                ActionOpt::TemperatureWithMc => {
                    game.players[p].mc -= TEMPERATURE_MC_COST;
                    raise_temperature(game, db, p);
                }
                ActionOpt::OceanWithMc => {
                    game.players[p].mc -= OCEAN_MC_COST;
                    reveal_ocean(game, db, p);
                }
                ActionOpt::SellCard => {
                    let n = game.players[p].hand.len();
                    let i = game.rng.gen_range(0..n);
                    let card = game.players[p].hand.remove(i);
                    game.discard.push(card);
                    game.players[p].mc += SELL_CARD_MC;
                }
            }
        }
    }

    // « Viktig regel » (livret p.14) : en fin de phase d'action, on DOIT payer
    // plantes et chaleur pour hausser oxygène et température si possible,
    // sauf paramètre déjà au max.
    for p in 0..NUM_PLAYERS {
        while game.players[p].plants >= FOREST_PLANT_COST && game.oxygen < OXYGEN_MAX {
            build_forest(game, p, true);
        }
        while game.players[p].heat >= TEMPERATURE_HEAT_COST && game.temperature < TEMPERATURE_MAX
        {
            game.players[p].heat -= TEMPERATURE_HEAT_COST;
            raise_temperature(game, db, p);
        }
    }
}

/// Phase IV — Production (livret p.15, `CollectIncomeTurnProcessor` Java) :
/// MC = production MC + TR (+4 sélectionneur) ; chaleur, plantes, cartes
/// selon production.
fn phase_production(game: &mut GameState, _db: &CardsDb, _policy: &mut dyn Policy) {
    for p in 0..NUM_PLAYERS {
        let bonus = if game.players[p].chosen_phase == 4 {
            PRODUCTION_SELECTOR_MC
        } else {
            0
        };
        let pl = &mut game.players[p];
        pl.mc += pl.mc_prod + pl.tr + bonus;
        pl.heat += pl.heat_prod;
        pl.plants += pl.plant_prod;
        let n = game.players[p].card_prod;
        for _ in 0..n {
            if let Some(c) = draw_card(game) {
                game.players[p].hand.push(c);
            }
        }
    }
}

/// Phase V — Recherche (livret p.15) : 2 piochées / 1 gardée ;
/// sélectionneur : 5 piochées / 2 gardées.
fn phase_research(game: &mut GameState, _db: &CardsDb, policy: &mut dyn Policy) {
    let mut drawn = Vec::with_capacity(5);
    for p in 0..NUM_PLAYERS {
        let (n, keep) = if game.players[p].chosen_phase == 5 {
            (5usize, 2usize)
        } else {
            (2usize, 1usize)
        };
        draw_n(game, n, &mut drawn);
        let keep = keep.min(drawn.len());
        let kept_idx = policy.research_keep(&mut game.rng, p, &drawn, keep);
        assert_eq!(kept_idx.len(), keep, "recherche: mauvais nombre de cartes gardées");
        let mut kept_flags = vec![false; drawn.len()];
        for &i in &kept_idx {
            assert!(i < drawn.len() && !kept_flags[i], "recherche: indice invalide");
            kept_flags[i] = true;
        }
        for (i, &c) in drawn.iter().enumerate() {
            if kept_flags[i] {
                game.players[p].hand.push(c);
            } else {
                game.discard.push(c);
            }
        }
    }
}

/// Valeur courante d'un joueur pour un objectif de milestone.
fn milestone_value(kind: MilestoneKind, pl: &PlayerState) -> i64 {
    match kind {
        MilestoneKind::Builder => pl.tag_counts[Tag::Building.index().unwrap()] as i64,
        MilestoneKind::Diversifier => pl.unique_tags() as i64,
        MilestoneKind::Energizer => pl.heat_prod,
        MilestoneKind::Farmer => pl.plant_prod,
        MilestoneKind::Legend => pl.played_count(Color::Red) as i64,
        MilestoneKind::Magnate => pl.played_count(Color::Green) as i64,
        MilestoneKind::Planner => pl.played.len() as i64,
        MilestoneKind::SpaceBaron => pl.tag_counts[Tag::Space.index().unwrap()] as i64,
        MilestoneKind::Terraformer => pl.tr,
        MilestoneKind::Tycoon => pl.played_count(Color::Blue) as i64,
        MilestoneKind::Gardener => pl.forests,
    }
}

fn milestone_goal(kind: MilestoneKind) -> i64 {
    match kind {
        MilestoneKind::Builder => 8,
        MilestoneKind::Diversifier => 9,
        MilestoneKind::Energizer => 10,
        MilestoneKind::Farmer => 5,
        MilestoneKind::Legend => 6,
        MilestoneKind::Magnate => 8,
        MilestoneKind::Planner => 12,
        MilestoneKind::SpaceBaron => 7,
        MilestoneKind::Terraformer => 15,
        MilestoneKind::Tycoon => 6,
        MilestoneKind::Gardener => 3,
    }
}

/// Revendication simplifiée (D8) : à chaque transition de phase, un milestone
/// non revendiqué est acquis par tout joueur remplissant l'objectif (les
/// revendications simultanées scorent toutes 3 VP — Discovery p.3).
pub fn assign_milestones(game: &mut GameState) {
    for slot in game.milestones.iter_mut() {
        if slot.achieved_by.iter().any(|&b| b) {
            continue;
        }
        for p in 0..NUM_PLAYERS {
            if milestone_value(slot.kind, &game.players[p]) >= milestone_goal(slot.kind) {
                slot.achieved_by[p] = true;
            }
        }
    }
}

fn award_value(kind: AwardKind, pl: &PlayerState) -> i64 {
    match kind {
        AwardKind::Celebrity => pl.mc_prod,
        AwardKind::Collector => 0, // ressources sur cartes : stub v1
        AwardKind::Generator => pl.heat_prod,
        AwardKind::Industrialist => pl.steel_capacity + pl.titanium_capacity,
        AwardKind::ProjectManager => pl.played.len() as i64,
        AwardKind::Researcher => pl.tag_counts[Tag::Science.index().unwrap()] as i64,
    }
}

/// Points d'awards par joueur : 1er = 5 VP, 2e = 2 VP ; égalité au 1er rang :
/// 4 VP chacun et pas de 2e (Discovery p.3). À 2 joueurs, pas d'égalité
/// possible au 2e rang.
pub fn award_points(game: &GameState) -> [i64; NUM_PLAYERS] {
    let mut pts = [0i64; NUM_PLAYERS];
    for &award in &game.awards {
        let v0 = award_value(award, &game.players[0]);
        let v1 = award_value(award, &game.players[1]);
        if v0 == v1 {
            pts[0] += 4;
            pts[1] += 4;
        } else if v0 > v1 {
            pts[0] += 5;
            pts[1] += 2;
        } else {
            pts[0] += 2;
            pts[1] += 5;
        }
    }
    pts
}

/// VP d'une carte jouée : VP fixes + VP dynamiques calculables avec l'état v1
/// (JUPITER = tags Jupiter, EARTH = tags Terre, FOREST = forêts, BLUE_CARD =
/// cartes bleues jouées, ANY_CARD = toutes cartes jouées ; ressources posées
/// sur les cartes = 0 en v1). Formule Java `WinPointsService.getWinPoints` :
/// floor(n / resources) * points.
fn card_points(db: &CardsDb, pl: &PlayerState, card_id: u16) -> i64 {
    let card = &db.projects[card_id as usize];
    let mut s = card.vp;
    if let Some(dynv) = card.vp_dynamic {
        let n = match dynv.kind {
            VpKind::Jupiter => pl.tag_counts[Tag::Jupiter.index().unwrap()] as i64,
            VpKind::Earth => pl.tag_counts[Tag::Earth.index().unwrap()] as i64,
            VpKind::Forest => pl.forests,
            VpKind::BlueCard => pl.played_count(Color::Blue) as i64,
            VpKind::AnyCard => pl.played.len() as i64,
            VpKind::Unsupported => 0,
        };
        if dynv.resources > 0 {
            s += (n / dynv.resources) * dynv.points;
        }
    }
    s
}

/// Score final (livret p.16-17 + Discovery p.3) : TR + 1 VP par forêt +
/// VP des cartes jouées (fixes + dynamiques, effets ON uniquement — `--effects
/// off` reproduit le squelette) + 3 VP par milestone + awards.
pub fn score(game: &GameState, db: &CardsDb) -> [i64; NUM_PLAYERS] {
    let awards = award_points(game);
    let mut out = [0i64; NUM_PLAYERS];
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        let mut s = pl.tr + pl.forests;
        if db.effects_on {
            for &c in &pl.played {
                s += card_points(db, pl, c);
            }
        }
        for slot in &game.milestones {
            if slot.achieved_by[p] {
                s += 3;
            }
        }
        s += awards[p];
        out[p] = s;
    }
    out
}

/// Joue une ronde complète. Fin de partie testée après chaque phase : quand
/// les 3 paramètres sont au max, on finit la phase en cours et on saute le
/// reste de la ronde (livret « spelets slut », D5).
pub fn play_round(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    assert!(!game.game_over, "play_round sur une partie terminée");

    // A. Planification (simultanée et secrète dans le jeu réel ; l'ordre
    // d'appel n'influe pas sur l'information disponible en politique v1).
    let mut picked = [false; 6];
    for p in 0..NUM_PLAYERS {
        let allowed = allowed_phases(&game.players[p]);
        let phase = policy.pick_phase(&mut game.rng, p, &allowed);
        assert!(
            allowed.contains(&phase),
            "phase {} interdite (précédente : {:?})",
            phase,
            game.players[p].previous_phase
        );
        game.players[p].chosen_phase = phase;
        game.players[p].previous_phase = Some(phase);
        // Bonus du sélectionneur de la phase action (`PickPhaseProcessor` Java).
        game.players[p].extra_blue_activations = if phase == 3 { 1 } else { 0 };
        picked[phase as usize] = true;
    }

    // B. Exécution : seules les phases choisies, dans l'ordre I..V.
    for phase in 1u8..=5 {
        if !picked[phase as usize] {
            continue;
        }
        game.snapshot_planet();
        match phase {
            1 => phase_development(game, db, policy),
            2 => phase_construction(game, db, policy),
            3 => phase_action(game, db, policy),
            4 => phase_production(game, db, policy),
            _ => phase_research(game, db, policy),
        }
        assign_milestones(game);
        if game.all_parameters_maxed() {
            game.game_over = true;
            return;
        }
    }

    // C. Étape de fin : limite de main 10, 3 MC par carte défaussée
    // (livret « avslutningssteget » p.16).
    for p in 0..NUM_PLAYERS {
        let over = game.players[p].hand.len().saturating_sub(HAND_LIMIT);
        if over > 0 {
            let hand_snapshot = game.players[p].hand.clone();
            let mut idx = policy.discard_down(&mut game.rng, p, &hand_snapshot, over);
            assert_eq!(idx.len(), over, "défausse de fin de ronde: mauvais nombre");
            idx.sort_unstable();
            idx.dedup();
            assert_eq!(idx.len(), over, "défausse de fin de ronde: doublons");
            for &i in idx.iter().rev() {
                let card = game.players[p].hand.remove(i);
                game.discard.push(card);
                game.players[p].mc += SELL_CARD_MC;
            }
        }
    }

    game.generation += 1;
}

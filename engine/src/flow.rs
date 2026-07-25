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
    Action, ActionCost, ActionEff, Eff, GlobalTrigger, ProdCount, ProdRes, Reduction, Req,
    ResAmount, ResEff, ResKind, ResPut, ResStep, ResTarget, TrigGain,
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
        // (C4) Règle maison : la manche 1 commence par le joueur 0.
        first_player: 0,
        turn_order: Vec::new(),
        prereq_snapshot_blocks: 0,
        draw_before_build: 0,
        draw_after_build: 0,
        discard_payments: 0,
        res_added: 0,
        res_removed: 0,
        res_targets_missing: 0,
        phase_upgrades_skipped: 0,
        derived_mc: 0,
        derived_heat: 0,
        derived_plants: 0,
        tr_from_tags: 0,
        research_extra_draws: 0,
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

// =============================================================================
// (lot 3) Ressources posées sur les cartes — service unique + interprète du
// vocabulaire déclaratif de `effects.rs`.
//
// TOUT ajout et TOUT retrait passe par `add_resources` / `remove_resources` :
// pose, déclencheur de pose, déclencheur global, action de carte, sonde. Il n'y
// a aucune écriture directe de `PlayerState::card_resources` ailleurs — c'est
// la même discipline que `card_discount` au lot 2.
// =============================================================================

/// Fait entrer une carte PORTEUSE dans la table des ressources du joueur, à 0.
/// Règle du jeu (et oracle Java `Player.initResources`) : une carte porteuse
/// vide est déjà une cible valide. Appelé une seule fois, à la pose, depuis
/// `build_card_with`. Une carte non porteuse n'y entre JAMAIS (NEVER 8).
fn init_card_resources(game: &mut GameState, db: &CardsDb, p: usize, card_id: u16) {
    if db.projects[card_id as usize].holds().is_some() {
        game.players[p].card_resources.insert(card_id, 0);
    }
}

/// SERVICE UNIQUE d'ajout de ressources sur une carte en jeu. Incrémente
/// `res_added` (en unités) au moment EXACT de l'ajout.
///
/// Panique si la carte n'est pas une porteuse en jeu du joueur : un ajout hors
/// de ce cadre est un bug d'encodage, pas un cas de jeu (NEVER 7).
pub fn add_resources(game: &mut GameState, db: &CardsDb, p: usize, card_id: u16, n: u32) {
    if n == 0 {
        return;
    }
    assert!(
        db.projects[card_id as usize].holds().is_some(),
        "ajout de ressource sur une carte qui n'en porte pas: {}",
        db.projects[card_id as usize].name
    );
    let slot = game.players[p]
        .card_resources
        .get_mut(&card_id)
        .expect("ajout de ressource sur une carte qui n'est pas en jeu chez ce joueur");
    *slot += n;
    game.res_added += n as u64;
}

/// SERVICE UNIQUE de retrait. Incrémente `res_removed` au moment du retrait.
pub fn remove_resources(game: &mut GameState, db: &CardsDb, p: usize, card_id: u16, n: u32) {
    if n == 0 {
        return;
    }
    let _ = db;
    let slot = game.players[p]
        .card_resources
        .get_mut(&card_id)
        .expect("retrait de ressource sur une carte qui n'est pas en jeu chez ce joueur");
    assert!(*slot >= n, "retrait de plus de ressources que la carte n'en porte");
    *slot -= n;
    game.res_removed += n as u64;
}

/// Cartes porteuses en jeu du joueur acceptant l'un des types `kinds`, hors
/// `exclude`. L'ordre vient de `card_resources` (`BTreeMap`) : croissant par
/// identifiant de carte, donc TOTALEMENT déterministe — c'est l'ordre dans
/// lequel les candidats sont présentés à la politique (contrat).
fn res_targets(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    kinds: &[ResKind],
    exclude: Option<u16>,
) -> Vec<u16> {
    game.players[p]
        .card_resources
        .keys()
        .copied()
        .filter(|c| Some(*c) != exclude)
        .filter(|c| {
            db.projects[*c as usize]
                .holds()
                .map_or(false, |k| kinds.contains(&k))
        })
        .collect()
}

/// Cartes porteuses du joueur sur lesquelles on peut RETIRER `n` ressources de
/// l'un des types `kinds` (Decomposing Fungus). Même ordre déterministe.
fn res_sources(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    kinds: &[ResKind],
    n: u32,
) -> Vec<u16> {
    res_targets(game, db, p, kinds, None)
        .into_iter()
        .filter(|c| game.players[p].resources_on(*c) >= n)
        .collect()
}

/// Candidats d'une pose donnée, `self_card` étant la carte qui porte l'effet.
fn put_targets(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    self_card: u16,
    put: &ResPut,
) -> Vec<u16> {
    match put.target {
        ResTarget::SelfCard => {
            if game.players[p].card_resources.contains_key(&self_card) {
                vec![self_card]
            } else {
                Vec::new()
            }
        }
        // « ANOTHER card » = une autre carte que celle qui porte l'effet.
        ResTarget::Another => res_targets(game, db, p, put.kinds, Some(self_card)),
        // « ANY card » (Large Convoy, CEO's Favorite Project) : aucune exclusion.
        ResTarget::Any => res_targets(game, db, p, put.kinds, None),
    }
}

/// Une branche d'alternative est-elle jouable ? Les branches impossibles sont
/// filtrées AVANT d'être présentées à la politique (contrat).
fn branch_playable(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    self_card: u16,
    branch: &[ResEff],
) -> bool {
    branch.iter().all(|e| match e {
        ResEff::Gain(_) | ResEff::PhaseUpgrade => true,
        ResEff::Put(put) => !put_targets(game, db, p, self_card, put).is_empty(),
        ResEff::RemoveSelf(n) => game.players[p].resources_on(self_card) >= *n,
        ResEff::RemoveAny(kinds, n) => !res_sources(game, db, p, kinds, *n).is_empty(),
    })
}

/// Applique UN effet à ressources. `self_card` = carte qui porte l'effet (celle
/// qu'on pose, ou la source du déclencheur, ou la carte dont on active
/// l'action).
fn apply_res_eff(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    self_card: u16,
    e: &ResEff,
    policy: &mut dyn Policy,
) {
    match e {
        ResEff::Gain(eff) => apply_eff(game, db, p, *eff, policy),
        // Amélioration de carte Phase : mécanisme d'un lot ultérieur. L'effet
        // est perdu, SANS compensation d'aucune sorte, et compté.
        ResEff::PhaseUpgrade => game.phase_upgrades_skipped += 1,
        ResEff::Put(put) => {
            let cands = put_targets(game, db, p, self_card, put);
            if cands.is_empty() {
                // Aucune cible : l'effet est sauté, sans compensation.
                game.res_targets_missing += 1;
                return;
            }
            let target = if put.target == ResTarget::SelfCard {
                self_card
            } else {
                let i = policy.choose_res_target(&mut game.rng, p, &cands);
                if i >= cands.len() {
                    return; // renoncement explicite (journal D4)
                }
                cands[i]
            };
            let n = match put.amount {
                ResAmount::Fixed(n) => n,
                // « 3 microbes ou 2 animaux » : la quantité dépend du type
                // porté par la carte CIBLE (Java `ImportedHydrogen`).
                ResAmount::ByKind { microbe, other } => {
                    if db.projects[target as usize].holds() == Some(ResKind::Microbe) {
                        microbe
                    } else {
                        other
                    }
                }
            };
            add_resources(game, db, p, target, n);
        }
        ResEff::RemoveSelf(n) => {
            if game.players[p].resources_on(self_card) >= *n {
                remove_resources(game, db, p, self_card, *n);
            }
        }
        ResEff::RemoveAny(kinds, n) => {
            let cands = res_sources(game, db, p, kinds, *n);
            if cands.is_empty() {
                return;
            }
            let i = policy.choose_res_source(&mut game.rng, p, &cands);
            if i >= cands.len() {
                return; // renoncement explicite (journal D4)
            }
            remove_resources(game, db, p, cands[i], *n);
        }
    }
}

/// Alternative « … ou … » : filtre les branches injouables, demande la branche
/// à la politique s'il en reste au moins deux, applique la branche retenue.
/// Aucune branche jouable = effet entier sauté (contrat).
fn apply_choice(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    self_card: u16,
    branches: &'static [&'static [ResEff]],
    policy: &mut dyn Policy,
) {
    let playable: Vec<usize> = (0..branches.len())
        .filter(|&i| branch_playable(game, db, p, self_card, branches[i]))
        .collect();
    if playable.is_empty() {
        // Si l'alternative proposait une pose, c'est bien une pose perdue
        // faute de cible : elle est comptée une fois (journal D5).
        if branches
            .iter()
            .any(|b| b.iter().any(|e| matches!(e, ResEff::Put(_))))
        {
            game.res_targets_missing += 1;
        }
        return;
    }
    // Une seule branche jouable : il n'y a plus d'alternative (journal D3).
    let k = if playable.len() == 1 {
        0
    } else {
        let c = policy.choose_option(&mut game.rng, p, playable.len());
        if c >= playable.len() {
            return; // renoncement explicite (journal D4)
        }
        c
    };
    for e in branches[playable[k]] {
        apply_res_eff(game, db, p, self_card, e, policy);
    }
}

/// Exécute les étapes `on_build` d'une carte, DANS L'ORDRE DU TEXTE IMPRIMÉ
/// (plusieurs cibles = plusieurs demandes successives à la politique).
fn apply_res_steps(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    self_card: u16,
    steps: &'static [ResStep],
    policy: &mut dyn Policy,
) {
    for step in steps {
        match step {
            ResStep::Do(e) => apply_res_eff(game, db, p, self_card, e, policy),
            ResStep::Choose(branches) => {
                apply_choice(game, db, p, self_card, branches, policy)
            }
        }
    }
}

/// (lot 3) Réduction CONDITIONNELLE payée en ressources posées : Anaerobic
/// Microorganisms, « you may remove 2 microbes from this card to pay 10 MC
/// less ». Renvoie `(carte source, ressources à retirer, montant)` si une carte
/// en jeu du joueur porte cette réduction ET porte assez de ressources.
///
/// Elle ne passe PAS par `card_discount` (qui somme les réductions fixes,
/// inconditionnelles) : celle-ci est payante et soumise à une décision du
/// joueur. Les deux sont consommées par les MÊMES deux appelants — `affordable`
/// (montant potentiel, pour ne pas juger la carte hors budget) et
/// `build_card_with` (décision, paiement, retrait effectif).
pub fn microbe_discount(game: &GameState, db: &CardsDb, p: usize) -> Option<(u16, u32, i64)> {
    if !db.effects_on {
        return None;
    }
    for &owned in &game.players[p].played {
        if let Some(spec) = db.projects[owned as usize].effect {
            for r in spec.reductions {
                if let Reduction::PayResources { kind, count, amount } = *r {
                    // Le type déclaré doit être celui que la carte porte
                    // réellement : on ne paie jamais avec une ressource d'un
                    // autre type que celui annoncé par le texte imprimé.
                    if db.projects[owned as usize].holds() == Some(kind)
                        && game.players[p].resources_on(owned) >= count
                    {
                        return Some((owned, count, amount));
                    }
                }
            }
        }
    }
    None
}

/// Prédicat commun aux deux lectures de prérequis. `param` fournit les valeurs
/// (température, oxygène, océans) contre lesquelles les prérequis de PARAMÈTRES
/// sont jugés ; les prérequis de tags et de dépenses sont toujours jugés à
/// l'état COURANT (le livret ne les mentionne pas dans la règle de l'instantané).
fn reqs_satisfied(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    param: (u8, u8, u8),
) -> bool {
    if !db.effects_on {
        return true;
    }
    let Some(spec) = db.projects[card_id as usize].effect else {
        return true;
    };
    let (temperature, oxygen, oceans) = param;
    let pl = &game.players[p];
    spec.reqs.iter().all(|req| match *req {
        Req::TempMin(n) => temperature >= n,
        Req::TempMax(n) => temperature <= n,
        Req::OxyMin(n) => oxygen >= n,
        Req::OceanMin(n) => oceans >= n,
        Req::OceanMax(n) => oceans <= n,
        Req::Tags(tag, n) => {
            tag.index().map_or(false, |i| pl.tag_counts[i] >= n as u32)
        }
        Req::SpendHeat(n) => pl.heat >= n,
        Req::SpendPlants(n) => pl.plants >= n,
        Req::SpendTr(n) => pl.tr >= n,
    })
}

/// (C1) Les prérequis de la carte sont-ils satisfaits ? RÈGLE DU JEU : les
/// prérequis de PARAMÈTRES (océans, oxygène, température) sont jugés sur
/// l'INSTANTANÉ de début de phase (`snap_*`) — livret p.13, l.352 : « ce
/// prérequis doit être rempli **au début de la phase** ». Les prérequis de tags
/// et de dépenses (`Tags`/`Spend*`) restent jugés à l'état COURANT.
/// Carte hors lot ou effets coupés : toujours vrai.
pub fn requirements_met(game: &GameState, db: &CardsDb, p: usize, card_id: u16) -> bool {
    reqs_satisfied(
        game,
        db,
        p,
        card_id,
        (game.snap_temperature, game.snap_oxygen, game.snap_oceans),
    )
}

/// (C1) Même prédicat, mais les prérequis de paramètres jugés à l'état COURANT.
/// N'est PAS la règle du jeu : sert à observer l'écart que `requirements_met`
/// corrige (compteur `prereq_snapshot_blocks`, champ de sonde `prereq_ok_now`).
pub fn requirements_met_now(game: &GameState, db: &CardsDb, p: usize, card_id: u16) -> bool {
    reqs_satisfied(
        game,
        db,
        p,
        card_id,
        (game.temperature, game.oxygen, game.oceans_revealed),
    )
}

/// (C3) Une carte de coût effectif `cost` est-elle payable par un joueur qui a
/// `mc` MC et `hand_len` cartes en main (la carte à poser comprise) ? Livret
/// p.13, l.348 : MC **et/ou** défausse de cartes à 3 MC/carte. La carte posée ne
/// pouvant pas se payer elle-même, la monnaie disponible est `hand_len - 1`.
/// Prédicat UNIQUE d'affordabilité : consommé par `affordable` (énumération des
/// options du flux réel) et par la sonde. `build_card_with` en est la
/// contrepartie exacte — il paie de la même façon et assère le résultat — de
/// sorte que les deux ne peuvent pas diverger.
pub fn payable(mc: i64, hand_len: usize, cost: i64) -> bool {
    mc + SELL_CARD_MC * (hand_len as i64 - 1).max(0) >= cost
}

/// Indices de main constructibles pour une couleur donnée : paiement (MC et/ou
/// défausse, C3) ET prérequis de la couche d'effets satisfaits (sur
/// l'instantané, C1).
///
/// Prend `&mut GameState` pour alimenter le compteur d'audit
/// `prereq_snapshot_blocks` à l'endroit EXACT où l'exclusion a lieu.
fn affordable(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    colors: &[Color],
    discount: i64,
) -> Vec<usize> {
    let hand_len = game.players[p].hand.len();
    let mc = game.players[p].mc;
    let mut out = Vec::new();
    let mut blocked = 0u64;
    // (lot 3) Réduction payable en microbes : elle compte dans l'affordabilité,
    // sinon une carte jouable serait jugée hors budget (contrat). Calculée une
    // fois par énumération : elle ne dépend pas de la carte examinée.
    let payable_disc = microbe_discount(game, db, p).map_or(0, |(_, _, a)| a);
    for i in 0..hand_len {
        let c = game.players[p].hand[i];
        let card = &db.projects[c as usize];
        if !colors.contains(&card.color) {
            continue;
        }
        let cost = effective_cost(
            card.price,
            discount + card_discount(game, db, p, c) + payable_disc,
        );
        if !payable(mc, hand_len, cost) {
            continue;
        }
        if requirements_met(game, db, p, c) {
            out.push(i);
        } else if requirements_met_now(game, db, p, c) {
            // Carte payable, autorisée par l'état courant, refusée par
            // l'instantané de début de phase : c'est exactement l'écart E6.
            blocked += 1;
        }
    }
    game.prereq_snapshot_blocks += blocked;
    out
}

/// Applique les dépenses de prérequis puis les effets de pose d'une carte du
/// lot. Appelé uniquement depuis `build_card` (même chemin pour `simulate`,
/// la sonde et les tests). Les hausses de paramètres réutilisent les
/// fonctions du squelette (TR + caps sur l'instantané de phase).
fn apply_card_effects(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    policy: &mut dyn Policy,
) {
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
    // 2. Effets simples, puis (lot 3) effets à ressources dans l'ordre du texte.
    for eff in spec.effects {
        apply_eff(game, db, p, *eff, policy);
    }
    apply_res_steps(game, db, p, card_id, spec.on_build, policy);
}

/// Applique UN effet du vocabulaire lot 1. Extrait de `apply_card_effects` pour
/// que les branches d'alternative du lot 3 (`ResEff::Gain`) empruntent
/// exactement le même code — un seul chemin par effet.
fn apply_eff(game: &mut GameState, db: &CardsDb, p: usize, eff: Eff, policy: &mut dyn Policy) {
    match eff {
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
                raise_temperature(game, db, p, policy);
            }
        }
        Eff::Oxygen(n) => {
            for _ in 0..n {
                raise_oxygen(game, db, p, policy);
            }
        }
        Eff::Ocean(n) => {
            for _ in 0..n {
                reveal_ocean(game, db, p, policy);
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
        // (lot 4) Hausse de NT d'un pas PAR BADGE, lue à l'instant de
        // l'application. La carte est déjà en jeu (`put_in_play` précède
        // `apply_card_effects`, voir `build_card_with`) : son propre badge est
        // compté sans traitement particulier — c'est cela, « including this ».
        // Chaque pas passe par `gain_tr`, le chemin de hausse de NT existant.
        Eff::TrPerTag(tag) => {
            let steps = tag
                .index()
                .map_or(0, |i| game.players[p].tag_counts[i]);
            for _ in 0..steps {
                game.players[p].gain_tr();
            }
            game.tr_from_tags += steps as u64;
        }
    }
}

/// (lot 4) **Production dérivée totale** d'un joueur : `(MC, chaleur, plantes)`.
///
/// Somme, sur les cartes EN JEU du joueur, des `DerivedProd` de la table
/// d'effets. Le compteur (badges d'un type, ou jetons Forêt) est lu à l'instant
/// de l'appel — c'est ce qui fait que la production « suit » les badges gagnés
/// APRÈS la pose (livret FR p.13 l.180). La division est ENTIÈRE.
///
/// C'est l'UNIQUE chemin de calcul : la phase IV et la sonde le consomment tous
/// deux, il n'existe pas de seconde implémentation (NEVER 2). Renvoie `(0,0,0)`
/// si les effets sont coupés (`--effects off`).
pub fn derived_production(db: &CardsDb, pl: &PlayerState) -> (i64, i64, i64) {
    if !db.effects_on {
        return (0, 0, 0);
    }
    let (mut mc, mut heat, mut plants) = (0i64, 0i64, 0i64);
    for &c in &pl.played {
        let Some(spec) = db.projects[c as usize].effect else {
            continue;
        };
        let Some(prod) = spec.prod else {
            continue;
        };
        if prod.per == 0 {
            continue;
        }
        let counted = match prod.count {
            ProdCount::Tag(t) => t.index().map_or(0, |i| pl.tag_counts[i] as i64),
            ProdCount::Forests => pl.forests,
        };
        let gained = counted / prod.per as i64;
        match prod.res {
            ProdRes::Mc => mc += gained,
            ProdRes::Heat => heat += gained,
            ProdRes::Plants => plants += gained,
        }
    }
    (mc, heat, plants)
}

/// (lot 4) **Bonus permanent de phase Recherche** d'un joueur :
/// `(cartes piochées en plus, cartes gardées en plus)`.
///
/// Cumulé sur les cartes EN JEU (deux exemplaires du même effet ajouteraient
/// 2/2). Consommé par la SEULE phase V — jamais par la mise en place, jamais par
/// la production de cartes (`card_prod`), jamais par une pioche d'effet de
/// carte. Unique implémentation (NEVER 2). `(0, 0)` si les effets sont coupés.
pub fn research_extra(db: &CardsDb, pl: &PlayerState) -> (usize, usize) {
    if !db.effects_on {
        return (0, 0);
    }
    let (mut draw, mut keep) = (0usize, 0usize);
    for &c in &pl.played {
        if let Some(spec) = db.projects[c as usize].effect {
            if let Some(bonus) = spec.research {
                draw += bonus.draw;
                keep += bonus.keep;
            }
        }
    }
    (draw, keep)
}

/// (lot 4) Base du livret pour la phase V (p.15) : 2 cartes piochées / 1 gardée ;
/// sélectionneur de la phase 5 : 5 piochées / 2 gardées.
pub fn research_base(pl: &PlayerState) -> (usize, usize) {
    if pl.chosen_phase == 5 {
        (5, 2)
    } else {
        (2, 1)
    }
}

/// (lot 4) Cartes piochées / gardées en phase V par un joueur : base du livret
/// (`research_base`) + bonus permanent de ses cartes en jeu (`research_extra`).
/// Joueur ordinaire 2/1 → 3/2 ; sélectionneur 5/2 → 6/3. Chemin unique,
/// consommé par `phase_research`.
pub fn research_draw_keep(db: &CardsDb, pl: &PlayerState) -> (usize, usize) {
    let (base_n, base_keep) = research_base(pl);
    let (extra_n, extra_keep) = research_extra(db, pl);
    (base_n + extra_n, base_keep + extra_keep)
}

/// Construit la carte d'indice de main `idx` en appliquant la règle de paiement
/// **par défaut** du trait `Policy` (façade historique du lot 2 : même
/// signature, appelée par la sonde et les tests). Délègue à
/// [`build_card_with`] : il n'existe pas de seconde logique de pose ni de
/// paiement. La boucle de jeu, elle, passe toujours par `build_card_with` avec
/// la politique réelle de la partie — c'est elle qui décide alors du nombre de
/// cartes défaussées. Renvoie le nombre de cartes défaussées pour payer.
pub fn build_card(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    idx: usize,
    discount: i64,
) -> usize {
    let mut default = crate::policy::RandomPolicy;
    build_card_with(game, db, p, idx, discount, &mut default)
}

/// Construit la carte d'indice de main `idx` : paie le coût effectif en MC
/// et/ou en défaussant des cartes (C3, livret p.13 l.348 — 3 MC par carte,
/// surplus rendu), entre en jeu (tags/couleur), puis applique dépenses + effets
/// du lot si les effets sont activés.
///
/// Renvoie le nombre de cartes défaussées pour payer CETTE carte (0 si les MC
/// suffisaient). La carte posée est retirée de la main AVANT le choix des cartes
/// à défausser : elle ne peut donc jamais se payer elle-même.
pub fn build_card_with(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    idx: usize,
    discount: i64,
    policy: &mut dyn Policy,
) -> usize {
    let hand_len_before = game.players[p].hand.len();
    let card_id = game.players[p].hand.remove(idx);
    // Réduction totale = remise de phase (sélectionneur) + réductions des cartes
    // en jeu (service unique `card_discount`, calculé AVANT la mise en jeu).
    let fixed_discount = discount + card_discount(game, db, p, card_id);
    let price = db.projects[card_id as usize].price;

    // (lot 3) Réduction payée en microbes (Anaerobic Microorganisms) : c'est un
    // CHOIX du joueur, pas un automatisme. La branche « y renoncer » n'est
    // proposée que si elle est jouable, c'est-à-dire si la carte reste payable
    // sans la réduction (règle générale de filtrage des branches — journal D7).
    let mut pay_with_resources: Option<(u16, u32)> = None;
    let mut total_discount = fixed_discount;
    if let Some((src, count, amount)) = microbe_discount(game, db, p) {
        let cost_without = effective_cost(price, fixed_discount);
        let can_decline = payable(game.players[p].mc, hand_len_before, cost_without);
        // Branche 0 = utiliser la réduction (l'option imprimée) ; branche 1 = y
        // renoncer.
        let use_it = if can_decline {
            policy.choose_option(&mut game.rng, p, 2) == 0
        } else {
            true
        };
        if use_it {
            pay_with_resources = Some((src, count));
            total_discount += amount;
        }
    }

    let cost = effective_cost(price, total_discount);
    assert!(cost >= 0, "prix payé négatif (réduction non plafonnée)");

    // (C3) Paiement : d'abord les MC, puis la défausse pour le reste. Le
    // nombre de cartes vient de la politique (défaut du trait = minimum).
    let mut discarded = 0usize;
    if game.players[p].mc < cost {
        let hand = game.players[p].hand.clone();
        let n = policy.discard_payment_count(&mut game.rng, p, game.players[p].mc, cost, &hand);
        assert!(n <= game.players[p].hand.len(), "défausse-paiement hors main");
        // Quelles cartes : les DERNIÈRES de la main. Le livret laisse le choix
        // libre ; prendre par la fin est déterministe, en O(1), et préserve la
        // tête de main — ce dont dépend la sonde séquence, qui pose toujours à
        // l'indice 0.
        for _ in 0..n {
            let card = game.players[p].hand.pop().expect("défausse-paiement hors main");
            game.discard.push(card);
            game.players[p].mc += SELL_CARD_MC;
        }
        discarded = n;
        game.discard_payments += n as u64;
    }
    assert!(
        game.players[p].mc >= cost,
        "construction sans le paiement requis (MC + défausse)"
    );
    // Le surplus reste au joueur : « la différence vous est rendue » (p.13).
    game.players[p].mc -= cost;
    // (lot 3) Les ressources de la réduction ne sont consommées QUE maintenant :
    // la carte est effectivement posée, aucun microbe n'est perdu sur une pose
    // annulée. Service unique de retrait.
    if let Some((src, count)) = pay_with_resources {
        remove_resources(game, db, p, src, count);
    }
    game.players[p].put_in_play(card_id, db);
    if db.effects_on {
        // (lot 3) Une carte porteuse entre en jeu avec 0 ressource : elle est
        // déjà une cible valide pour son propre effet de pose et pour ses
        // déclencheurs (`Player.initResources` du moteur Java).
        init_card_resources(game, db, p, card_id);
        // Effet propre de la carte, puis déclencheurs « When you play … » de
        // toutes les cartes persistantes en jeu (la carte incluse si applicable).
        apply_card_effects(game, db, p, card_id, policy);
        fire_play_triggers(game, db, p, card_id, policy);
    }
    discarded
}

/// (B) Déclencheurs de pose : évalués à la pose de `played_id`, sur les tags de
/// la carte posée, pour toutes les cartes persistantes en jeu du joueur `p`
/// (la carte elle-même incluse ssi son déclencheur porte `include_self`).
/// Chemin unique `build_card` (simulate, sonde, tests).
fn fire_play_triggers(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    played_id: u16,
    policy: &mut dyn Policy,
) {
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
                apply_trig_gain(game, db, p, src, *g, mult, policy);
            }
        }
    }
}

/// Applique un gain de déclencheur `mult` fois (facteur = nb de tags satisfaits
/// pour les déclencheurs « par tag », 1 sinon). `src` = carte qui porte le
/// déclencheur : c'est elle qui reçoit les ressources de `ResSelf` et qui sert
/// de référence à « ANOTHER card » dans une alternative.
fn apply_trig_gain(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    src: u16,
    g: TrigGain,
    mult: i64,
    policy: &mut dyn Policy,
) {
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
        // (lot 3) Ressources sur la carte qui porte le déclencheur (Ecological
        // Zone / Anaerobic : `mult` = nb de tags concernés, Java countCardTags).
        TrigGain::ResSelf(n) => {
            add_resources(game, db, p, src, n * mult.max(0) as u32)
        }
        // (lot 3) Alternative : appliquée UNE fois par déclenchement — les deux
        // cartes concernées (Viral Enhancers, Decomposers) sont au forfait.
        TrigGain::Choose(branches) => apply_choice(game, db, p, src, branches, policy),
    }
}

/// (B) Déclencheurs globaux du joueur agissant, fixés à une hausse effective de
/// paramètre (Volcanic Soil sur température, Arctic Algae sur océan). Java itère
/// `player.getPlayed()` du joueur qui provoque la hausse.
/// Événement global auquel un déclencheur peut réagir (lot 2 : température,
/// océan ; lot 3 : oxygène, forêt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlobalEvent {
    Temperature,
    Ocean,
    Oxygen,
    Forest,
}

fn fire_global_trigger(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    ev: GlobalEvent,
    policy: &mut dyn Policy,
) {
    if !db.effects_on {
        return;
    }
    // Collecte d'abord (lecture seule), applique ensuite (aucune allocation si
    // le joueur n'a aucune carte à déclencheur global — cas courant).
    let mut pending: Vec<(u16, TrigGain)> = Vec::new();
    for &src in &game.players[p].played {
        if let Some(spec) = db.projects[src as usize].effect {
            for g in spec.global_triggers {
                let gains = match *g {
                    GlobalTrigger::OnRaiseTemperature(gs)
                        if ev == GlobalEvent::Temperature =>
                    {
                        Some(gs)
                    }
                    GlobalTrigger::OnFlipOcean(gs) if ev == GlobalEvent::Ocean => Some(gs),
                    GlobalTrigger::OnRaiseOxygen(gs) if ev == GlobalEvent::Oxygen => Some(gs),
                    GlobalTrigger::OnBuildForest(gs) if ev == GlobalEvent::Forest => Some(gs),
                    _ => None,
                };
                if let Some(gs) = gains {
                    for x in gs {
                        pending.push((src, *x));
                    }
                }
            }
        }
    }
    for (src, g) in pending {
        apply_trig_gain(game, db, p, src, g, 1, policy);
    }
}

/// Hausse d'oxygène : cap sur l'instantané de début de phase (D6). TR accordé
/// si l'instantané le permet, niveau réel saturé au max.
fn raise_oxygen(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
    if game.snap_oxygen >= OXYGEN_MAX {
        return;
    }
    if game.oxygen < OXYGEN_MAX {
        game.oxygen += 1;
    }
    game.players[p].gain_tr();
    // (lot 3) « When you raise oxygen » du joueur agissant (Herbivores).
    fire_global_trigger(game, db, p, GlobalEvent::Oxygen, policy);
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

fn raise_temperature(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
    if game.snap_temperature >= TEMPERATURE_MAX {
        return;
    }
    if game.temperature < TEMPERATURE_MAX {
        game.temperature += 1;
    }
    game.players[p].gain_tr();
    // (B) déclencheurs « When you raise the temperature » du joueur agissant.
    fire_global_trigger(game, db, p, GlobalEvent::Temperature, policy);
}

/// Révèle un océan : bonus de la tuile + TR. Au-delà du 9e dans la phase du
/// max : bonus de la dernière tuile révélée (livret p.14, fallback Java).
fn reveal_ocean(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
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
    fire_global_trigger(game, db, p, GlobalEvent::Ocean, policy);
}

/// Forêt : 8 plantes ou 20 MC → +1 forêt (VP), oxygène +1 si l'instantané le
/// permet (livret p.14 ; Java `buildForest`).
fn build_forest(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    with_plants: bool,
    policy: &mut dyn Policy,
) {
    if with_plants {
        assert!(game.players[p].plants >= FOREST_PLANT_COST);
        game.players[p].plants -= FOREST_PLANT_COST;
    } else {
        assert!(game.players[p].mc >= FOREST_MC_COST);
        game.players[p].mc -= FOREST_MC_COST;
    }
    game.players[p].forests += 1;
    raise_oxygen(game, db, p, policy);
    // (lot 3) « When you build a forest » du joueur agissant (Small Animals).
    fire_global_trigger(game, db, p, GlobalEvent::Forest, policy);
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
                            raise_oxygen(game, db, p, policy);
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
            reveal_ocean(game, db, p, policy);
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
            raise_temperature(game, db, p, policy);
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
        // (lot 3) Action à ressources : alternative dont les branches sont dans
        // l'ordre du texte imprimé. Filtrage des branches injouables puis choix
        // du joueur ; aucune branche jouable = l'action ne s'applique pas
        // (`action_applied` faux, activation tout de même consommée par la
        // phase III, comme pour un coût impayable).
        Action::Res(branches) => {
            let playable: Vec<usize> = (0..branches.len())
                .filter(|&i| branch_playable(game, db, p, card_id, branches[i]))
                .collect();
            if playable.is_empty() {
                return false;
            }
            let k = if playable.len() == 1 {
                0
            } else {
                let c = policy.choose_option(&mut game.rng, p, playable.len());
                if c >= playable.len() {
                    return false; // renoncement explicite (journal D4)
                }
                c
            };
            for e in branches[playable[k]] {
                apply_res_eff(game, db, p, card_id, e, policy);
            }
            true
        }
    }
}

/// Phase I — Développement (livret p.11) : chacun peut jouer 1 carte verte ;
/// sélectionneur : -3 MC. Un passage chacun, dans l'ordre du tour (C4).
fn phase_development(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    for p in game.players_in_turn_order() {
        let discount = if game.players[p].chosen_phase == 1 {
            DEV_SELECTOR_DISCOUNT
        } else {
            0
        };
        let opts = affordable(game, db, p, &[Color::Green], discount);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card_with(game, db, p, idx, discount, policy);
        }
    }
}

/// Phase II — Construction (livret p.12) : chacun peut jouer 1 carte
/// bleue/rouge ; sélectionneur : piocher 1 carte AVANT ou APRÈS avoir joué
/// (C2), OU en jouer une 2e. Un passage chacun, dans l'ordre du tour (C4).
fn phase_construction(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    const BR: [Color; 2] = [Color::Blue, Color::Red];
    for p in game.players_in_turn_order() {
        // Le bonus est décidé AVANT la pose : c'est la seule façon d'ouvrir
        // le moment « pioche avant » du livret (l.336).
        let bonus = if game.players[p].chosen_phase == 2 {
            Some(policy.construction_bonus(&mut game.rng, p))
        } else {
            None
        };

        // (C2) Pioche AVANT : la carte piochée entre en main avant le calcul
        // d'affordabilité, elle peut donc être posée dans la foulée.
        if bonus == Some(ConstructionBonus::DrawCardBefore) {
            if let Some(c) = draw_card(game) {
                game.players[p].hand.push(c);
                game.draw_before_build += 1;
            }
        }

        let opts = affordable(game, db, p, &BR, 0);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card_with(game, db, p, idx, 0, policy);
        }

        match bonus {
            // (C2) Pioche APRÈS la pose.
            Some(ConstructionBonus::DrawCard) => {
                if let Some(c) = draw_card(game) {
                    game.players[p].hand.push(c);
                    game.draw_after_build += 1;
                }
            }
            Some(ConstructionBonus::SecondBuild) => {
                let opts = affordable(game, db, p, &BR, 0);
                if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
                    assert!(opts.contains(&idx), "choix de construction hors options");
                    build_card_with(game, db, p, idx, 0, policy);
                }
            }
            // Déjà résolu avant la pose, ou pas de bonus.
            Some(ConstructionBonus::DrawCardBefore) | None => {}
        }
    }
}

/// Phase III — Action (livret p.14) : actions bleues (stubs neutres, une fois
/// chacune ; sélectionneur : une répétition), actions standard à volonté,
/// puis conversions OBLIGATOIRES de fin de phase (D7).
fn phase_action(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    let mut options: Vec<ActionOpt> = Vec::with_capacity(16);
    let order = game.players_in_turn_order();

    // Chaque carte bleue jouée offre son action une fois par phase.
    let mut remaining_blue: [Vec<u16>; NUM_PLAYERS] = Default::default();
    let mut extra = [0u8; NUM_PLAYERS];
    let mut passed = [false; NUM_PLAYERS];
    for p in 0..NUM_PLAYERS {
        remaining_blue[p] = game.players[p]
            .played
            .iter()
            .copied()
            .filter(|&c| db.projects[c as usize].color == Color::Blue)
            .collect();
        extra[p] = if game.players[p].chosen_phase == 3 {
            game.players[p].extra_blue_activations
        } else {
            0
        };
    }

    // (C4, règle maison) Alternance ACTION PAR ACTION : chaque joueur fait UNE
    // action à son tour, en commençant par le premier joueur de la manche ; un
    // joueur qui passe est retiré du tour ; la phase s'arrête quand tous ont
    // passé.
    while !passed.iter().all(|&b| b) {
        for p in order {
            if passed[p] {
                continue;
            }
            action_options(game, db, p, &remaining_blue[p], &mut options);
            let Some(choice) = policy.action_choice(&mut game.rng, p, &options) else {
                passed[p] = true;
                continue;
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
                    if let Some(pos) = remaining_blue[p].iter().position(|&c| c == card) {
                        remaining_blue[p].remove(pos);
                    }
                    // Bonus du sélectionneur : une activation supplémentaire.
                    if extra[p] > 0 {
                        extra[p] -= 1;
                        remaining_blue[p].push(card);
                    }
                }
                ActionOpt::ForestWithPlants => build_forest(game, db, p, true, policy),
                ActionOpt::ForestWithMc => build_forest(game, db, p, false, policy),
                ActionOpt::TemperatureWithHeat => {
                    game.players[p].heat -= TEMPERATURE_HEAT_COST;
                    raise_temperature(game, db, p, policy);
                }
                ActionOpt::TemperatureWithMc => {
                    game.players[p].mc -= TEMPERATURE_MC_COST;
                    raise_temperature(game, db, p, policy);
                }
                ActionOpt::OceanWithMc => {
                    game.players[p].mc -= OCEAN_MC_COST;
                    reveal_ocean(game, db, p, policy);
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
    // sauf paramètre déjà au max. (C5) Le max est jugé sur l'INSTANTANÉ de
    // début de phase, comme les hausses individuelles (`raise_*`) — sinon un
    // paramètre atteint pendant CETTE phase couperait l'obligation en cours de
    // route alors que la phase l'autorise encore. Reste après la boucle.
    for p in order {
        while game.players[p].plants >= FOREST_PLANT_COST && game.snap_oxygen < OXYGEN_MAX {
            build_forest(game, db, p, true, policy);
        }
        while game.players[p].heat >= TEMPERATURE_HEAT_COST
            && game.snap_temperature < TEMPERATURE_MAX
        {
            game.players[p].heat -= TEMPERATURE_HEAT_COST;
            raise_temperature(game, db, p, policy);
        }
    }
}

/// Phase IV — Production (livret p.15, `CollectIncomeTurnProcessor` Java) :
/// MC = production MC + TR (+4 sélectionneur) ; chaleur, plantes, cartes
/// selon production.
pub(crate) fn phase_production(game: &mut GameState, db: &CardsDb, _policy: &mut dyn Policy) {
    for p in 0..NUM_PLAYERS {
        let bonus = if game.players[p].chosen_phase == 4 {
            PRODUCTION_SELECTOR_MC
        } else {
            0
        };
        // (lot 4) Production DÉRIVÉE : recalculée ICI, à chaque phase, à partir
        // des cartes en jeu et des badges du moment — jamais figée à la pose,
        // jamais inscrite sur les pistes `*_prod` (celles-ci restent réservées
        // aux productions FIXES). Service unique, partagé avec la sonde.
        let (d_mc, d_heat, d_plants) = derived_production(db, &game.players[p]);
        let pl = &mut game.players[p];
        pl.mc += pl.mc_prod + pl.tr + bonus + d_mc;
        pl.heat += pl.heat_prod + d_heat;
        pl.plants += pl.plant_prod + d_plants;
        // Compteurs d'audit incrémentés à l'endroit EXACT du crédit : c'est
        // aussi ce que la sonde `--probe-produce` relève (jamais recalculé).
        game.derived_mc += d_mc as u64;
        game.derived_heat += d_heat as u64;
        game.derived_plants += d_plants as u64;
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
fn phase_research(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    let mut drawn = Vec::with_capacity(8);
    // Un passage chacun, dans l'ordre du tour (C4).
    for p in game.players_in_turn_order() {
        let (base_n, _) = research_base(&game.players[p]);
        // (lot 4) Base du livret + bonus PERMANENT des cartes en jeu, cumulés
        // par le service unique (2/1 → 3/2 ; sélectionneur 5/2 → 6/3).
        let (n, keep) = research_draw_keep(db, &game.players[p]);
        draw_n(game, n, &mut drawn);
        // Cartes RÉELLEMENT piochées en plus grâce au bonus permanent (une
        // pioche épuisée en donnerait moins) — relevé au site de pioche.
        game.research_extra_draws += drawn.len().saturating_sub(base_n) as u64;
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

/// VP d'une carte jouée : VP fixes + VP dynamiques (JUPITER = tags Jupiter,
/// EARTH = tags Terre, FOREST = forêts, BLUE_CARD = cartes bleues jouées,
/// ANY_CARD = toutes cartes jouées ; ANIMAL/MICROBE/SCIENCE = ressources
/// posées sur CETTE carte — lot 3). Formule Java `WinPointsService` :
/// floor(n / resources) * points, en division ENTIÈRE.
///
/// Renvoie `(total, part venant des ressources posées)`. C'est l'UNIQUE endroit
/// où les points de victoire d'une carte sont calculés : le score de partie et
/// la sonde consomment tous deux cette fonction, il n'existe pas de second
/// chemin (NEVER 3).
pub fn card_points(db: &CardsDb, pl: &PlayerState, card_id: u16) -> (i64, i64) {
    let card = &db.projects[card_id as usize];
    let mut total = card.vp;
    let mut from_resources = 0i64;
    if let Some(dynv) = card.vp_dynamic {
        // `on_resources` : le décompte porte sur les ressources posées sur la
        // carte, pas sur un état global du joueur.
        let (n, on_resources) = match dynv.kind {
            VpKind::Jupiter => (pl.tag_counts[Tag::Jupiter.index().unwrap()] as i64, false),
            VpKind::Earth => (pl.tag_counts[Tag::Earth.index().unwrap()] as i64, false),
            VpKind::Forest => (pl.forests, false),
            VpKind::BlueCard => (pl.played_count(Color::Blue) as i64, false),
            VpKind::AnyCard => (pl.played.len() as i64, false),
            VpKind::Animal | VpKind::Microbe | VpKind::Science => {
                (pl.resources_on(card_id) as i64, true)
            }
            VpKind::Unsupported => (0, false),
        };
        if dynv.resources > 0 {
            let pts = (n / dynv.resources) * dynv.points;
            total += pts;
            if on_resources {
                from_resources = pts;
            }
        }
    }
    (total, from_resources)
}


/// Score final (livret p.16-17 + Discovery p.3) : TR + 1 VP par forêt +
/// VP des cartes jouées (fixes + dynamiques, effets ON uniquement — `--effects
/// off` reproduit le squelette) + 3 VP par milestone + awards.
pub fn score(game: &GameState, db: &CardsDb) -> [i64; NUM_PLAYERS] {
    score_parts(game, db).0
}

/// Score final + total des points de victoire venant des RESSOURCES posées sur
/// les cartes, tous joueurs confondus (compteur d'audit `vp_from_resources`).
/// Les deux sortent du même parcours et du même calcul par carte
/// (`card_points`) : la valeur rapportée est celle qui compte réellement
/// au score, pas un recalcul parallèle.
pub fn score_parts(game: &GameState, db: &CardsDb) -> ([i64; NUM_PLAYERS], i64) {
    let awards = award_points(game);
    let mut out = [0i64; NUM_PLAYERS];
    let mut vp_from_resources = 0i64;
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        let mut s = pl.tr + pl.forests;
        if db.effects_on {
            for &c in &pl.played {
                let (total, from_res) = card_points(db, pl, c);
                s += total;
                vp_from_resources += from_res;
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
    (out, vp_from_resources)
}

/// Joue une ronde complète. Fin de partie testée après chaque phase : quand
/// les 3 paramètres sont au max, on finit la phase en cours et on saute le
/// reste de la ronde (livret « spelets slut », D5).
pub fn play_round(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    assert!(!game.game_over, "play_round sur une partie terminée");

    // (C4, règle maison) Ordre du tour de CETTE manche, enregistré tel qu'il va
    // être emprunté par les phases ci-dessous (`players_in_turn_order` lit le
    // même champ). Une entrée par manche réellement jouée.
    game.turn_order.push(game.first_player as u8);

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

    // (C4, règle maison) La manche est allée à son terme : le premier joueur
    // alterne pour la suivante.
    game.first_player = (game.first_player + 1) % NUM_PLAYERS;
    game.generation += 1;
}

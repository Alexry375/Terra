//! Flux de jeu : mise en place (avec les deux mulligans maison), boucle de
//! ronde (planification, phases I-V, étape de fin), fin de partie et score.
//!
//! Sources : livret de base (planeringssteget p.10, faserna p.11-15,
//! avslutningssteget p.16, spelets slut p.16-17), livret Discovery p.3
//! (milestones/awards), moteur Java (`StateTransitionService`,
//! `CollectIncomeTurnProcessor`, `DraftCardsTurnProcessor`,
//! `PickPhaseProcessor`, `TerraformingService`, `MarsGame.assignMilestones`).

use crate::cards::{CardsDb, Color, Tag, VpKind, JOKER_TAG_CHOICES};
use crate::boites::Boite;
use crate::choice::{BranchOption, ChoiceContext, PhaseUpgradeOption, ProductionOption};
use crate::effects::{
    self, Action, ActionCost, ActionEff, ActionRes, BuildGrant, Capacity, CorpEffects, Eff,
    GlobalTrigger,
    PhaseBonus, ProdCount, ProdRes, Reduction, Req, ResAmount, ResEff, ResKind, ResPut, ResStep,
    ResTarget, Reveal, RevealFilter, SelectorGrant, SelectorSpec, TrigGain,
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
/// 4. mulligan projets (entre 0 et 8 cartes, au choix carte par carte) ;
/// 5. choix final de corporation (1 parmi 2), cartes projets en main.
pub fn setup_game(db: &CardsDb, seed: u64, policy: &mut dyn Policy) -> GameState {
    let mut rng = StdRng::seed_from_u64(seed);

    // (boites-1) La pioche = les cartes des boîtes demandées par `--boites`,
    // marquées `in_deck` par le point de composition unique (`boites::composer`).
    // Les autres cartes du fichier restent accessibles à la sonde et aux tests,
    // mais ne sont jamais distribuées.
    let mut deck: Vec<u16> = (0..db.projects.len() as u16)
        .filter(|&c| db.projects[c as usize].in_deck)
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
        phase_upgrades_granted: 0,
        phase_upgrades_reupgraded: 0,
        upgraded_bonus_applied: 0,
        phase_upgrades_targeted: 0,
        phase_upgrades_by_action: 0,
        upgraded_reveal_bonuses: 0,
        objective_condition_hits: 0,
        draw_then_discard_uses: 0,
        upgraded_extra_builds: 0,
        cards_effects_unhandled: 0,
        derived_mc: 0,
        derived_heat: 0,
        derived_plants: 0,
        tr_from_tags: 0,
        research_extra_draws: 0,
        extra_builds_granted: 0,
        extra_builds_used: 0,
        free_builds: 0,
        next_card_mods_armed: 0,
        next_card_mods_used: 0,
        corp_heat_as_mc: 0,
        corp_forest_rebates: 0,
        corp_tr_boosts: 0,
        corp_trigger_tr: 0,
        action_phase_bonuses: 0,
        action_discard_costs: 0,
        draw_discard_discards: 0,
        cards_revealed: 0,
        standard_action_discounts: 0,
        action_mc_bonuses: 0,
        joker_tag_choices: 0,
        joker_tag_hits: 0,
        corp_phase_upgrades_at_setup: 0,
        discard_bonus_mc: 0,
        action_phase_self_bonus: 0,
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
    let mut apool = award_pool(db);
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
        policy.observe(&game, p);
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

    // 4. Mulligan projets — règle maison n°2 : le joueur désigne CARTE PAR
    //    CARTE celles qu'il remplace, de zéro à huit. Contrairement au mulligan
    //    corporations, ce n'est PAS du tout ou rien.
    for p in 0..NUM_PLAYERS {
        let hand_snapshot = game.players[p].hand.clone();
        policy.observe(&game, p);
        let mut idx = policy.project_mulligan(&mut game.rng, p, &hand_snapshot);
        // Une politique peut rendre n'importe quoi : on assainit sans jamais
        // défausser deux fois la même carte ni sortir de la main.
        idx.retain(|&i| i < hand_snapshot.len());
        idx.sort_unstable();
        idx.dedup();
        if idx.is_empty() {
            continue;
        }
        // Retrait par indices DÉCROISSANTS : les indices restants ne bougent
        // pas au fur et à mesure des suppressions.
        for &i in idx.iter().rev() {
            let c = game.players[p].hand.remove(i);
            game.discard.push(c);
        }
        // On repioche exactement autant de cartes qu'on en a rendues, et elles
        // rejoignent la main derrière celles qu'on a gardées.
        draw_n(&mut game, idx.len(), &mut buf);
        game.players[p].hand.append(&mut buf);
    }

    // 5. Choix final de corporation, cartes projets en main.
    for p in 0..NUM_PLAYERS {
        policy.observe(&game, p);
        let pick = policy.pick_corporation(&mut game.rng, p, &corps[p]);
        assert!(pick < corps[p].len(), "choix de corporation hors bornes");
        let chosen = corps[p].remove(pick);
        for other in corps[p].drain(..) {
            game.corp_discard.push(other);
        }
        install_corporation_with(&mut game, db, p, chosen, policy);
    }

    game
}

/// (corpo-1) **Service UNIQUE de mise en place d'une corporation** : MC de
/// départ, badges, production de départ, pioche de départ. Emprunté par
/// `setup_game` ET par la sonde (`--probe-corp`) — il n'existe pas de second
/// chemin d'installation.
///
/// Les productions de départ sont inscrites sur les pistes FIXES
/// (`mc_prod`/`heat_prod`/`plant_prod`), que `phase_production` consomme à
/// chaque génération : la production se répète, elle n'est pas un gain unique.
///
/// Comme tout effet de carte, les effets de corporation sont coupés par
/// `--effects off` (journal D5) ; le MC de départ et les badges, eux, sont la
/// planche elle-même et restent dans les deux modes (comportement historique).
/// Façade historique (signature du lot corpo-1) : mise en place avec la règle de
/// décision par défaut. Délègue à [`install_corporation_with`] — il n'existe pas
/// de second chemin d'installation.
pub fn install_corporation(game: &mut GameState, db: &CardsDb, p: usize, corp_id: u16) {
    let mut default = crate::policy::RandomPolicy;
    install_corporation_with(game, db, p, corp_id, &mut default);
}

/// (jokers-corpos) La mise en place complète, politique comprise : « Améliorez
/// votre carte Phase n » laisse au joueur le choix de la VARIANTE (A ou B), qui
/// est une décision comme une autre (`Policy::choose_option`, via
/// `apply_phase_upgrade`).
pub fn install_corporation_with(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    corp_id: u16,
    policy: &mut dyn Policy,
) {
    let corp = &db.corporations[corp_id as usize];
    let starting_mc = corp.starting_mc;
    let tags = corp.tags.clone();
    let spec = corp.effect;

    game.players[p].corporation = Some(corp_id);
    game.players[p].mc = starting_mc;
    // (boites-1) I4 — corporation sans encodage (les 4 de Découverte) : son
    // pouvoir imprimé ne sera jamais appliqué de la partie, on le compte.
    //
    // (decouverte-projets) …mais SEULEMENT quand la couche d'effets est active.
    // En `--effects off` le moteur est un squelette intégral : AUCUN pouvoir
    // imprimé n'est appliqué, ni celui des cartes encodées ni celui des autres.
    // Compter les seules cartes sans encodage y désignerait sept coupables dans
    // une partie où les 388 sont muettes — un compteur qui ne compte pas la
    // grandeur qu'il annonce (ALWAYS 4). Voir `result.md`, § Où je vous
    // contredis : le commentaire d'origine affirmait l'inverse.
    if spec.is_none() && db.effects_on {
        game.cards_effects_unhandled += 1;
    }
    for t in &tags {
        if let Some(i) = t.index() {
            game.players[p].tag_counts[i] += 1;
        }
    }
    // (lot acier-titane) La planche peut porter un savoir-faire (encart gris :
    // Mining Guild et Interplanetary Cinematics, un acier ; PhoboLog et Saturn
    // Systems, un titane). Le compte est rafraîchi ici, à l'endroit exact de la
    // mise en place — avant le `return` d'`--effects off`, où la dérivation rend
    // (0, 0) d'elle-même : les deux modes passent par le même appel.
    refresh_capacities(game, db, p);
    if !db.effects_on {
        return;
    }
    let Some(spec) = spec else { return };
    game.players[p].mc_prod += spec.start_prod.mc;
    game.players[p].heat_prod += spec.start_prod.heat;
    game.players[p].plant_prod += spec.start_prod.plants;
    for _ in 0..spec.start_draw {
        if let Some(c) = draw_card(game) {
            game.players[p].hand.push(c);
        }
    }
    // (jokers-corpos) « Améliorez votre carte Phase n » : le chemin d'octroi
    // UNIQUE du moteur, avec la phase IMPOSÉE par le carton. Le déroulement ne
    // connaît aucune corporation par son nom — il lit la table.
    for e in spec.setup {
        match *e {
            ResEff::PhaseUpgrade(t) => {
                apply_phase_upgrade(game, p, policy, t, UpgradeSource::Setup)
            }
            ResEff::Gain(g) => apply_eff(game, db, p, g, policy),
            // Les variantes à ressources exigent une carte réceptacle, qu'une
            // planche n'est pas : un test structurel du lot interdit de les
            // déclarer ici, elles ne peuvent donc pas arriver.
            other => unreachable!(
                "effet de mise en place non exprimable pour une corporation : {other:?}"
            ),
        }
    }
    // (jokers-corpos) « … y compris celui-ci » : les badges de la PLANCHE
    // déclenchent ses propres déclencheurs de pose marqués `include_self`.
    fire_corp_self_triggers(game, db, p, policy);
}

/// **(jokers-corpos) « Chaque fois que vous jouez un badge [énergie], Y COMPRIS
/// CELUI-CI, gagnez 2 chaleurs »** — Sultira.
///
/// Le badge de la planche elle-même est un badge « joué » au sens du carton :
/// les déclencheurs de la corporation qui portent `include_self` sont donc levés
/// contre ses PROPRES badges, à la mise en place. Rien n'est écrit en dur : ce
/// sont les mêmes `TrigGain`, appliqués par le même `apply_trig_gain`, que ceux
/// que lève la pose d'une carte.
///
/// Les onze autres planches encodées ne portent aucun déclencheur
/// `include_self` — Saturn Systems dit au contraire « excluding this » — et rien
/// ne se produit pour elles (contre-témoin du contrôle 05).
fn fire_corp_self_triggers(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) {
    let Some(spec) = corp_effects(db, &game.players[p]) else {
        return;
    };
    let Some(cid) = game.players[p].corporation else {
        return;
    };
    let own_tags = db.corporations[cid as usize].tags.clone();
    let triggers = spec.play_triggers;
    for trig in triggers {
        if !trig.include_self {
            continue;
        }
        let matched = trig.cond.matched_tags(&own_tags);
        if matched == 0 {
            continue;
        }
        let mult = if trig.scale_by_matched_tags {
            matched as i64
        } else {
            1
        };
        for g in trig.gains {
            apply_trig_gain(game, db, p, None, *g, mult, policy);
        }
    }
}

/// (corpo-1) Encodage de la corporation d'un joueur, ou `None` si les effets
/// sont coupés / le joueur n'a pas de corporation. Point de lecture UNIQUE :
/// tous les services de corporation passent par lui, aucun ne relit
/// `PlayerState::corporation` directement.
pub fn corp_effects<'a>(db: &'a CardsDb, pl: &PlayerState) -> Option<&'a CorpEffects> {
    if !db.effects_on {
        return None;
    }
    pl.corporation
        .and_then(|c| db.corporations[c as usize].effect)
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

// =============================================================================
// (jokers-corpos) LE BADGE JOKER — « Choisissez un badge et ajoutez-le à cette
// carte. »
//
// Livret Découverte : « Dès qu'une carte indiquant un badge joker est révélée,
// le joueur qui l'a révélée choisit à quel badge équivaut le joker. Lorsque vous
// jouez une carte disposant de ce badge, vous devez prendre un jeton Badge
// correspondant au badge choisi et le placer sur le badge joker. Désormais, il
// déclenchera les effets relatifs à ce badge. Par exemple, si vous choisissez le
// badge Espace, les savoir-faire Titanium réduiront le coût en MC pour jouer la
// carte. »
//
// Le moteur n'a pas de notion de RÉVÉLATION publique. Le jeton est donc posé au
// dernier moment où le livret l'autorise encore et où la conséquence imprimée
// reste vraie : AVANT que le prix de la carte ne soit calculé. Concrètement, les
// jokers de la main reçoivent leur jeton juste avant l'énumération
// d'abordabilité (`resolve_hand_jokers`, aux quatre sites qui appellent
// `affordable`), et `build_card_granted` en repasse une couche en garde-fou
// avant tout calcul de prix. L'abordabilité et le paiement voient donc
// exactement le même badge (I2), et l'exemple du livret tient.
//
// Le CHOIX lui-même n'est pas dans le déroulement : il est demandé à
// `Policy::pick_joker_tag`, au même titre que `pick_phase` (NEVER 4).
//
// Une entrée PAR CARTE (`PlayerState::joker_tags`), écrite une seule fois : le
// badge est définitif, et deux cartes joker déclarées Terre valent deux badges
// Terre.
// =============================================================================

/// (jokers-corpos) La carte `card_id` porte-t-elle un badge joker ?
pub fn has_joker_tag(db: &CardsDb, card_id: u16) -> bool {
    db.projects[card_id as usize].tags.iter().any(|t| t.is_joker())
}

/// **(jokers-corpos) Pose le jeton Badge sur le badge joker de `card_id`**, si
/// la carte en porte un et n'en a pas encore reçu. Écriture UNIQUE de
/// `PlayerState::joker_tags` : rien d'autre dans le moteur n'y touche, ce qui
/// rend le choix définitif par construction.
///
/// `--effects off` : aucun choix n'est fait — le moteur y est un squelette où
/// aucun pouvoir imprimé n'est appliqué, et le badge reste indéterminé, donc
/// hors décompte (`Tag::index()` rend `None` pour `Tag::Dynamic`). C'est ce qui
/// laisse `joker_tag_choices` à zéro dans ce mode.
pub fn ensure_joker_tag(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    policy: &mut dyn Policy,
) {
    if !db.effects_on || !has_joker_tag(db, card_id) {
        return;
    }
    if game.players[p].joker_tags.contains_key(&card_id) {
        return; // le badge est DÉFINITIF : jamais réécrit.
    }
    let counts = game.players[p].tag_counts;
    policy.observe(&game, p);
    let i = policy.pick_joker_tag(&mut game.rng, p, card_id, &counts);
    // Un indice hors bornes est un manquement au contrat de `Policy`, pas un cas
    // de jeu : le moteur le SIGNALE au lieu de le raboter en silence sur EVENT.
    // Même discipline que `choose_build` (« choix de pose hors options ») et que
    // le coût en cartes d'une action (« la politique doit rendre n indices »).
    // Sans elle, une intelligence artificielle fautive obtiendrait un badge
    // arbitraire sans que rien ne le dise (défaut trouvé en relecture
    // adversariale).
    assert!(
        i < JOKER_TAG_CHOICES.len(),
        "badge joker hors bornes : la politique doit rendre un indice dans \
         0..{} (reçu {i})",
        JOKER_TAG_CHOICES.len()
    );
    let tag = JOKER_TAG_CHOICES[i];
    game.players[p].joker_tags.insert(card_id, tag);
    game.joker_tag_choices += 1;
}

/// **(jokers-corpos) Pose le jeton sur tous les badges jokers de la MAIN** du
/// joueur `p`. Appelée juste avant chaque énumération d'abordabilité : c'est ce
/// qui garantit qu'`affordable` juge une carte joker sur son badge réel, et donc
/// qu'elle ne refuse jamais une carte que le paiement, lui, saurait poser (I2).
pub fn resolve_hand_jokers(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) {
    if !db.effects_on {
        return;
    }
    let hand = game.players[p].hand.clone();
    for c in hand {
        ensure_joker_tag(game, db, p, c, policy);
    }
}

// =============================================================================
// (lot acier-titane) LE COMPTE D'ACIERS ET DE TITANES
//
// Aucune de nos sources ne dit « cette carte donne 2 aciers ». Elle n'en a pas
// besoin : le moteur encode déjà l'EFFET NET de chaque savoir-faire sous forme
// de `Reduction::Tag(Building|Space, n)`, et le livret fixe le taux (2 MC par
// acier, 3 MC par titane). Le compte se DÉRIVE donc de ce qui est déjà là,
// plutôt que d'être ressaisi dans une seconde table qui se désynchroniserait au
// premier savoir-faire ajouté (I2).
//
// Critère complet, et il tient en deux lignes :
//   Reduction::Tag(Building, n) portée par une carte VERTE ou une corporation
//                                                              → n / 2 aciers
//   Reduction::Tag(Space, n)    idem                           → n / 3 titanes
//
// La COULEUR fait partie du critère (I4). Une réduction bâtiment ou espace
// portée par une carte BLEUE n'est pas un savoir-faire : la carte du lot qui
// amplifie les savoir-faire des autres est bleue, et n'en est pas un. Aujourd'hui
// aucune carte bleue ne porte de `Reduction::Tag(Building|Space, …)` — la garde
// est là pour Découverte et pour la suite.
//
// La CORPORATION compte, sans condition de couleur : une planche n'a pas de
// couleur de carte projet, et quatre d'entre elles portent un savoir-faire dans
// leur encart gris (Mining Guild et Interplanetary Cinematics : un acier ;
// PhoboLog et Saturn Systems : un titane). Vérifié à l'image le 28-07, et
// recoupé par la transcription : *Ganymede Shipyard* note « l'encart gris à deux
// étoiles jaunes est un savoir-faire de 2 TITANE (2 × 3 MC = les 6 MC de
// réduction [space] du texte) ».
// =============================================================================

/// (lot acier-titane) Le compte d'aciers et de titanes d'un joueur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capacities {
    pub steel: i64,
    pub titanium: i64,
}

impl Capacities {
    /// Unités du savoir-faire demandé.
    pub fn get(self, cap: Capacity) -> i64 {
        match cap {
            Capacity::Steel => self.steel,
            Capacity::Titanium => self.titanium,
        }
    }

    fn add(&mut self, cap: Capacity, n: i64) {
        match cap {
            Capacity::Steel => self.steel += n,
            Capacity::Titanium => self.titanium += n,
        }
    }
}

/// **(lot acier-titane) Point de calcul UNIQUE du compte d'aciers et de titanes**
/// (I1). Aucun autre endroit du moteur ne recalcule ce nombre : les champs
/// `PlayerState::steel_capacity` / `titanium_capacity` sont écrits par
/// `refresh_capacities`, qui appelle cette fonction et rien d'autre.
///
/// Dérivé, jamais ressaisi (I2) : lit les réductions déjà encodées des cartes
/// VERTES en jeu du joueur (I4) et de sa corporation, et les divise par le taux
/// du livret porté par `Capacity`.
///
/// Ne dépend QUE de l'état du joueur `pl` — jamais de son adversaire (NEVER 9) :
/// la signature ne reçoit même pas l'autre joueur.
///
/// `--effects off` : (0, 0), comme toute la couche d'effets. Les réductions n'y
/// sont pas appliquées, les savoir-faire qui les portent n'existent pas non plus.
pub fn capacities(db: &CardsDb, pl: &PlayerState) -> Capacities {
    let mut out = Capacities::default();
    if !db.effects_on {
        return out;
    }
    for &owned in &pl.played {
        let card = &db.projects[owned as usize];
        // I4 — la couleur fait partie du critère du savoir-faire.
        if card.color != Color::Green {
            continue;
        }
        let Some(spec) = card.effect else { continue };
        for r in spec.reductions {
            if let Some((cap, n)) = r.capacity_units() {
                out.add(cap, n);
            }
        }
    }
    if let Some(spec) = corp_effects(db, pl) {
        for r in spec.reductions {
            if let Some((cap, n)) = r.capacity_units() {
                out.add(cap, n);
            }
        }
    }
    out
}

/// (lot acier-titane) Recopie le compte dérivé dans l'état du joueur. **Seule
/// écriture** de `steel_capacity` / `titanium_capacity` du moteur : appelée à
/// chaque mise en jeu (pose d'une carte, installation d'une corporation), les
/// deux seuls événements qui peuvent changer le compte.
///
/// Les deux champs sont donc un CACHE, pas une seconde vérité — et pour qu'ils
/// ne puissent pas diverger en silence, `sim::check_invariants` les recompare à
/// `capacities` à chaque manche de chaque partie.
pub fn refresh_capacities(game: &mut GameState, db: &CardsDb, p: usize) {
    let c = capacities(db, &game.players[p]);
    game.players[p].steel_capacity = c.steel;
    game.players[p].titanium_capacity = c.titanium;
}

/// (lot acier-titane) Le compte tel qu'il est écrit dans l'état du joueur.
/// Lecture unique des deux champs par le reste du moteur.
pub fn player_capacities(pl: &PlayerState) -> Capacities {
    Capacities {
        steel: pl.steel_capacity,
        titanium: pl.titanium_capacity,
    }
}

/// (A) Réduction de coût applicable à une carte donnée pour un joueur donné :
/// somme des réductions de TOUTES ses cartes persistantes déjà en jeu (lot 2).
/// Service UNIQUE consommé par `affordable` (affordabilité) ET `build_card`
/// (paiement) — jamais deux logiques parallèles. Calculée avant la mise en jeu
/// de la carte, donc une carte ne se réduit jamais elle-même. 0 si effets coupés.
/// (corpo-1) La CORPORATION du joueur contribue à cette même somme : sa
/// réduction n'a pas de second chemin de calcul. `Reduction::MinPrice`
/// (Credicor) est jugée sur le prix IMPRIMÉ de la carte, jamais sur un coût
/// déjà réduit.
pub fn card_discount(game: &GameState, db: &CardsDb, p: usize, card_id: u16) -> i64 {
    if !db.effects_on {
        return 0;
    }
    let card = &db.projects[card_id as usize];
    // (jokers-corpos) Les badges de la carte VUS PAR CE JOUEUR : le badge joker
    // y est déjà remplacé par le jeton posé dessus. C'est ici que se joue
    // l'exemple du livret — « si vous choisissez le badge Espace, les
    // savoir-faire Titanium réduiront le coût en MC pour jouer LA CARTE ».
    // `card_discount` étant le service unique de réduction, consommé par
    // l'abordabilité comme par le paiement, les deux voient le même badge (I2).
    let tags = game.players[p].tags_of(db, card_id);
    let (tags, price) = (&tags, card.price);
    // (lot acier-titane) Le compte du joueur À CET INSTANT : `Reduction::
    // PerCapacity` en dépend, et rien n'est figé à
    // la pose (I7). Lu sur l'état du joueur, jamais recalculé ici.
    let caps = player_capacities(&game.players[p]);
    let mut d = 0;
    for &owned in &game.players[p].played {
        if let Some(spec) = db.projects[owned as usize].effect {
            for r in spec.reductions {
                d += r.amount_for(tags, price) + per_capacity_amount(*r, tags, caps);
            }
        }
    }
    if let Some(spec) = corp_effects(db, &game.players[p]) {
        for r in spec.reductions {
            d += r.amount_for(tags, price) + per_capacity_amount(*r, tags, caps);
        }
    }
    d
}

/// (lot acier-titane) Montant d'une `Reduction::PerCapacity` pour une carte de
/// tags donnés et un compte de savoir-faire donné. 0 pour toute autre réduction.
///
/// Séparé de `Reduction::amount_for` parce que celle-ci ne connaît que la CARTE
/// VISÉE, quand ce montant-ci dépend de l'ÉTAT DU JOUEUR — même partage que
/// `Reduction::PayResources`, servie par `microbe_discount`.
fn per_capacity_amount(r: Reduction, tags: &[Tag], caps: Capacities) -> i64 {
    match r {
        Reduction::PerCapacity { tag, cap, per } if tags.contains(&tag) => per * caps.get(cap),
        _ => 0,
    }
}

// =============================================================================
// (corpo-1) La chaleur employée comme des MC — Helion Corporation, « You may use
// heat as MC. You may not use MC as heat. »
//
// Deux fonctions, un seul mécanisme : `spendable_mc` répond « de quoi ce joueur
// dispose-t-il pour payer ? » (affordabilité), `top_up_mc_with_heat` convertit
// effectivement la chaleur en MC juste avant la dépense. TOUS les sites qui
// dépensent des MC les empruntent — pose de carte, actions standard de la phase
// III, actions de cartes bleues, pas de NT acheté par Unmi — il n'existe donc
// pas de dépense de MC qui ignorerait Helion.
//
// Le « may » du texte imprimé est OFFERT AU JOUEUR par `Policy::choose_option`
// à la pose d'une carte (voir `build_card_with`), seul site où le livret
// propose une alternative — payer en défaussant des cartes à 3 MC. Partout
// ailleurs (actions standard, actions de cartes bleues, pas de NT d'Unmi),
// renoncer à la chaleur reviendrait à renoncer à l'action : la chaleur comble
// alors ce qui manque sans question posée. Dans tous les cas elle ne sert que
// de complément : jamais de chaleur brûlée quand les MC suffisent.
//
// (Le journal D6 décrivait une convention en dur ; D15 l'a remplacée par ce
// choix après relecture adversariale. Ce commentaire suit le code, pas D6.)
// =============================================================================

/// La corporation du joueur autorise-t-elle à dépenser la chaleur comme des MC ?
fn heat_as_mc(db: &CardsDb, pl: &PlayerState) -> bool {
    corp_effects(db, pl).map_or(false, |s| s.heat_as_mc)
}

/// **Chaleur RÉSERVÉE** par une carte : celle que son prérequis « Requires you
/// to spend N heat » l'engage à dépenser à la pose. Cette chaleur-là n'est pas
/// de la monnaie : Helion ne peut pas la convertir en MC pour payer le prix de
/// la carte, sinon la dépense de pose serait impayable. Lue sur la table
/// d'effets, jamais recalculée ailleurs.
pub fn heat_reserved_by(db: &CardsDb, card_id: u16) -> i64 {
    if !db.effects_on {
        return 0;
    }
    db.projects[card_id as usize].effect.map_or(0, |spec| {
        spec.reqs
            .iter()
            .map(|r| match *r {
                Req::SpendHeat(n) => n,
                _ => 0,
            })
            .sum()
    })
}

/// Ce qu'un joueur peut réellement engager en « MC » : ses MC, plus sa chaleur
/// si sa corporation le permet. Prédicat d'affordabilité UNIQUE — consommé par
/// `affordable`, `action_options`, `apply_blue_action` et la sonde.
pub fn spendable_mc(db: &CardsDb, pl: &PlayerState) -> i64 {
    spendable_mc_reserving(db, pl, 0)
}

/// Idem, `reserved` unités de chaleur mises de côté (voir `heat_reserved_by`).
pub fn spendable_mc_reserving(db: &CardsDb, pl: &PlayerState, reserved: i64) -> i64 {
    if heat_as_mc(db, pl) {
        pl.mc + (pl.heat - reserved).max(0)
    } else {
        pl.mc
    }
}

/// Convertit juste ce qu'il faut de chaleur en MC pour atteindre `cost`, si la
/// corporation le permet. Renvoie la chaleur consommée (0 le plus souvent).
/// Incrémente `corp_heat_as_mc` à l'endroit exact de la conversion.
pub fn top_up_mc_with_heat(game: &mut GameState, db: &CardsDb, p: usize, cost: i64) -> i64 {
    top_up_mc_with_heat_reserving(game, db, p, cost, 0)
}

/// Idem, `reserved` unités de chaleur intouchables (voir `heat_reserved_by`).
pub fn top_up_mc_with_heat_reserving(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    cost: i64,
    reserved: i64,
) -> i64 {
    if !heat_as_mc(db, &game.players[p]) {
        return 0;
    }
    let missing = cost - game.players[p].mc;
    if missing <= 0 {
        return 0;
    }
    let available = (game.players[p].heat - reserved).max(0);
    let used = missing.min(available);
    if used <= 0 {
        return 0;
    }
    game.players[p].heat -= used;
    game.players[p].mc += used;
    game.corp_heat_as_mc += used as u64;
    used
}

/// (corpo-1) Coût en PLANTES d'une forêt pour un joueur donné : coût du livret
/// (8) moins la remise de sa corporation (Ecoline : « you spend one less
/// plant »). Service UNIQUE — consommé par `action_options` (l'action est-elle
/// offerte ?), `build_forest` (le paiement) et la conversion obligatoire de fin
/// de phase III.
///
/// **Plancher à 1 plante, et non à 0** : la conversion obligatoire de fin de
/// phase III est un `while plants >= forest_plant_cost(...)` dont l'autre
/// condition (`snap_oxygen`) est figée pour toute la phase. Un coût nul y
/// bouclerait indéfiniment. Aucune donnée actuelle n'en approche (remise
/// maximale : 1) ; le plancher supprime la classe de bug, pas seulement le cas.
pub fn forest_plant_cost(db: &CardsDb, pl: &PlayerState) -> i64 {
    let rebate = corp_effects(db, pl).map_or(0, |s| s.forest_plant_rebate);
    (FOREST_PLANT_COST - rebate).max(1)
}

/// (corpo-1) **Service UNIQUE de hausse de NT côté flux** : accorde le pas par
/// `PlayerState::gain_tr` (qui tient la comptabilité de l'invariant TR), puis
/// applique le `TrBoost` d'Unmi — « The first time your TR is raised each phase,
/// you may pay 6 MC to raise your TR 1 step ».
///
/// Le drapeau `tr_raised_this_phase` est posé AVANT d'accorder le pas bonus, et
/// le pas bonus passe par `PlayerState::gain_tr` et non par ce service : la
/// récursion est donc impossible. Le « may » est un vrai choix du joueur, servi
/// par `Policy::choose_option` (branche 0 = payer, l'option imprimée ; branche 1
/// = renoncer), et il n'est proposé que si les 6 MC sont payables — chaleur
/// comprise si la corporation le permettait (elle ne le permet pas ici, Unmi et
/// Helion s'excluent, mais le chemin reste unique).
pub fn gain_tr(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
    game.players[p].gain_tr();
    let first = !game.players[p].tr_raised_this_phase;
    game.players[p].tr_raised_this_phase = true;
    if !first {
        return;
    }
    let Some(boost) = corp_effects(db, &game.players[p]).and_then(|s| s.tr_boost) else {
        return;
    };
    if spendable_mc(db, &game.players[p]) < boost.cost_mc {
        return;
    }
    // Deux branches jouables : payer (0, l'option imprimée) ou renoncer (1).
    policy.observe(&game, p);
    let ctx = ChoiceContext::CorpTrBoost {
        corporation: game.players[p].corporation,
        cost_mc: boost.cost_mc,
        steps: boost.steps,
    };
    if policy.choose_option_ctx(&mut game.rng, p, &ctx) != 0 {
        return;
    }
    top_up_mc_with_heat(game, db, p, boost.cost_mc);
    game.players[p].mc -= boost.cost_mc;
    for _ in 0..boost.steps {
        game.players[p].gain_tr();
    }
    game.corp_tr_boosts += boost.steps as u64;
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
        ResEff::Gain(_) | ResEff::PhaseUpgrade(_) => true,
        ResEff::Put(put) => !put_targets(game, db, p, self_card, put).is_empty(),
        ResEff::RemoveSelf(n) => game.players[p].resources_on(self_card) >= *n,
        ResEff::RemoveAny(kinds, n) => !res_sources(game, db, p, kinds, *n).is_empty(),
    })
}

/// (Découverte) **« Améliorez une carte Phase »** — le SEUL chemin d'octroi
/// d'une amélioration dans tout le moteur.
///
/// Le joueur choisit l'une des dix cartes Phase améliorées mises de côté et en
/// remplace la carte Phase correspondante (livret l. 64). Améliorer une phase
/// DÉJÀ améliorée est permis, à condition de basculer sur l'autre variante
/// (l. 66) : la variante en place est donc retirée des candidates, ce qui
/// interdit le gaspillage sans jamais interdire le geste. Il reste toujours au
/// moins cinq candidates : l'effet n'est jamais sauté, et
/// `phase_upgrades_skipped` ne peut plus bouger.
///
/// Le CHOIX appartient à `Policy` (NEVER 4) ; les candidates sont énumérées
/// dans un ordre totalement déterministe (phase croissante, puis A avant B).
/// (decouverte-projets) **D'où vient l'amélioration.** Ce n'est pas du
/// vocabulaire de carte : c'est un paramètre de service, qui n'existe que pour
/// que `phase_upgrades_by_action` compte la GRANDEUR qu'il annonce (ALWAYS 4) et
/// non la forme de l'encodage. *Fibrous Composite Material*, qui améliorait déjà
/// depuis une action avant ce chantier, y est donc comptée elle aussi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeSource {
    /// Effet appliqué à la POSE de la carte (`ResEff::PhaseUpgrade` en
    /// `on_build`, ou branche d'une alternative de pose).
    Build,
    /// Effet appliqué depuis l'ACTIVATION d'une action de carte bleue.
    Action,
    /// (jokers-corpos) Effet appliqué à la MISE EN PLACE d'une corporation
    /// (« Améliorez votre carte Phase n »). C'est cette source, et elle seule,
    /// qui alimente `corp_phase_upgrades_at_setup`.
    Setup,
}

fn apply_phase_upgrade(
    game: &mut GameState,
    p: usize,
    policy: &mut dyn Policy,
    // (decouverte-projets) Phase IMPOSÉE par le carton, `None` = au choix du
    // joueur. C'est le seul ajout au corps de la règle : les candidates sont
    // filtrées, rien d'autre ne change. Les trois cartes à phase imposée n'ont
    // donc PAS de chemin d'octroi à elles (clause anti-shortcut n° 3).
    target: Option<u8>,
    src: UpgradeSource,
) {
    let mut cands: Vec<(u8, PhaseUpgrade)> = Vec::with_capacity(10);
    for phase in 1u8..=5 {
        if target.is_some_and(|t| t != phase) {
            continue;
        }
        for v in PhaseUpgrade::ALL {
            if game.players[p].phase_upgrade(phase) != Some(v) {
                cands.push((phase, v));
            }
        }
    }
    // Phase libre : au moins 5 candidates. Phase imposée : 2 si la carte Phase
    // est encore normale, 1 si elle est déjà améliorée (la variante en place est
    // retirée, la bascule A ↔ B reste offerte — livret l. 66). Jamais zéro :
    // l'effet ne peut pas être sauté, et `phase_upgrades_skipped` reste à 0.
    debug_assert!(!cands.is_empty(), "aucune amélioration possible : impossible");
    if cands.is_empty() {
        return;
    }
    // Une seule candidate = plus de choix à faire (convention du lot 3 pour les
    // alternatives) : on n'interroge la politique qu'à partir de deux.
    let i = if cands.len() == 1 {
        0
    } else {
        // Le contexte porte les couples que le moteur vient de construire, plus
        // le nom imprimé de chaque carte Phase améliorée, lu dans la table du
        // moteur (`effects::PHASE_UPGRADED`) : celui qui décide n'a aucune règle
        // à rejouer pour savoir ce qu'on lui propose.
        let options: Vec<PhaseUpgradeOption> = cands
            .iter()
            .map(|&(phase, variant)| PhaseUpgradeOption {
                phase,
                variant,
                name: effects::PHASE_UPGRADED[phase as usize - 1][variant.index()].name,
            })
            .collect();
        let ctx = ChoiceContext::PhaseUpgrade {
            candidates: &options,
            imposed_phase: target,
            source: src,
        };
        policy.observe(&game, p);
        policy.choose_option_ctx(&mut game.rng, p, &ctx)
    };
    let (phase, variant) = cands[i.min(cands.len() - 1)];
    let deja = game.players[p].upgrade_phase(phase, variant);
    game.phase_upgrades_granted += 1;
    if deja {
        game.phase_upgrades_reupgraded += 1;
    }
    if target.is_some() {
        game.phase_upgrades_targeted += 1;
    }
    if src == UpgradeSource::Action {
        game.phase_upgrades_by_action += 1;
    }
    // (jokers-corpos) Compteur d'audit au site EXACT de l'octroi : une
    // amélioration due à la mise en place d'une corporation, et rien d'autre.
    if src == UpgradeSource::Setup {
        game.corp_phase_upgrades_at_setup += 1;
    }
}

/// (decouverte-projets) **« Avez-vous un Objectif ? »** — prédicat UNIQUE du
/// moteur, lu par `Req::HasObjective` (D19) et par `Eff::IfObjective` (D35).
///
/// « Objectif » est la tuile MILESTONE du jeu (`state::MilestoneKind`, dont
/// *Terraformer*) ; « Récompense » est `AwardKind`. Le joueur « a un Objectif »
/// dès qu'il en a revendiqué au moins un des trois en jeu — c'est exactement ce
/// que `flow::assign_milestones` écrit, et rien d'autre n'est consulté.
pub fn has_objective(game: &GameState, p: usize) -> bool {
    game.milestones.iter().any(|s| s.achieved_by[p])
}

/// (decouverte-projets) **« Effet : lorsque vous révélez une carte Phase
/// améliorée, gagnez … »** (D05) — levé pour LE SEUL joueur `p`, au moment où
/// il révèle sa carte Phase (planification de `play_round`).
///
/// Il ne lit que `p` : ni la carte Phase de l'adversaire, ni un compteur global
/// des cartes Phase améliorées révélées par les deux joueurs (clause
/// anti-shortcut n° 4). Un joueur ne révèle qu'UNE carte Phase par manche, donc
/// le gain tombe au plus une fois par manche et par carte porteuse (ASK 4).
// Publique comme `apply_blue_action` : c'est le point d'entrée UNIQUE de ce
// mécanisme, celui-là même que `play_round` emprunte. Les tests l'appellent
// directement pour observer une révélation isolée — ils n'ont pas de chemin à
// eux, ils prennent le vrai.
pub fn fire_upgraded_reveal(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) {
    if !db.effects_on {
        return;
    }
    let phase = game.players[p].chosen_phase;
    if game.players[p].phase_upgrade(phase).is_none() {
        return;
    }
    // Les cartes porteuses EN JEU de ce joueur, dans l'ordre de pose.
    let sources: Vec<u16> = game.players[p]
        .played
        .iter()
        .copied()
        .filter(|&c| {
            db.projects[c as usize]
                .effect
                .is_some_and(|e| !e.reveal_bonus.is_empty())
        })
        .collect();
    for c in sources {
        let effs = db.projects[c as usize].effect.unwrap().reveal_bonus;
        for e in effs {
            apply_eff(game, db, p, *e, policy);
        }
        // Compteur d'audit, au site EXACT du versement : le gain et le compteur
        // ne peuvent pas diverger.
        game.upgraded_reveal_bonuses += 1;
    }
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
    // (decouverte-projets) D'où vient l'effet : POSE ou ACTION. Traversé sans
    // être lu, sauf par `ResEff::PhaseUpgrade`.
    src: UpgradeSource,
) {
    match e {
        ResEff::Gain(eff) => apply_eff(game, db, p, *eff, policy),
        // « Améliorez une carte Phase » : le mécanisme existe depuis le
        // chantier `decouverte-phases` — plus rien n'est sauté.
        // (decouverte-projets) `t` = phase imposée par le carton, `None` = au
        // choix du joueur. `src` dit d'où vient l'appel : `apply_res_eff` est
        // emprunté par la POSE et par `Action::Res`, et le compteur
        // `phase_upgrades_by_action` doit distinguer les deux.
        ResEff::PhaseUpgrade(t) => apply_phase_upgrade(game, p, policy, *t, src),
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
                policy.observe(&game, p);
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
            policy.observe(&game, p);
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
    src: UpgradeSource,
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
        // Chaque option porte son rang DANS LE TEXTE IMPRIMÉ (avant filtrage)
        // et les effets que le moteur appliquera si elle est retenue.
        let options: Vec<BranchOption> = playable
            .iter()
            .map(|&i| BranchOption {
                printed_rank: i,
                effects: branches[i],
            })
            .collect();
        let ctx = ChoiceContext::CardAlternative {
            card: self_card,
            source: src,
            branches: &options,
        };
        policy.observe(&game, p);
        let c = policy.choose_option_ctx(&mut game.rng, p, &ctx);
        if c >= playable.len() {
            return; // renoncement explicite (journal D4)
        }
        c
    };
    for e in branches[playable[k]] {
        apply_res_eff(game, db, p, self_card, e, policy, src);
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
    src: UpgradeSource,
) {
    for step in steps {
        match step {
            ResStep::Do(e) => apply_res_eff(game, db, p, self_card, e, policy, src),
            ResStep::Choose(branches) => {
                apply_choice(game, db, p, self_card, branches, policy, src)
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

/// **PLANTES RÉSERVÉES** par une carte : celles que son prérequis « Requires you
/// to spend N plants » l'engage à dépenser à la pose. Ces plantes-là ne sont pas
/// de la monnaie : *Restructured Resources* ne peut pas les dépenser pour
/// réduire le prix de la carte, sinon la dépense de pose serait impayable et
/// l'assertion de `apply_card_effects` sauterait. Symétrique exact de
/// [`heat_reserved_by`] ; lue sur la table d'effets, jamais recalculée ailleurs.
pub fn plants_reserved_by(db: &CardsDb, card_id: u16) -> i64 {
    if !db.effects_on {
        return 0;
    }
    db.projects[card_id as usize].effect.map_or(0, |spec| {
        spec.reqs
            .iter()
            .map(|r| match *r {
                Req::SpendPlants(n) => n,
                _ => 0,
            })
            .sum()
    })
}

/// **(lot cartes-7) Réduction CONDITIONNELLE payée en PLANTES DE LA RÉSERVE** :
/// *Restructured Resources*, « When you play a card, you may spend 1 plant to
/// reduce that card's cost by 5 MC ». Renvoie `(plantes à dépenser, montant)` si
/// une carte en jeu du joueur porte cette réduction ET que le joueur a les
/// plantes, celles que la carte visée s'engage elle-même à dépenser mises de
/// côté ([`plants_reserved_by`]).
///
/// Décalque de [`microbe_discount`] : elle ne passe PAS par `card_discount` (qui
/// somme les réductions FIXES), elle est payante et soumise à une décision du
/// joueur. Mêmes deux appelants — `affordable` (montant potentiel, pour ne pas
/// juger la carte hors budget) et `build_card_with` (décision + dépense) : les
/// deux ne peuvent pas diverger (I2).
///
/// Elle prend la carte visée en argument, ce que `microbe_discount` n'a pas
/// besoin de faire : la monnaie et la dépense de pose sont ici la MÊME réserve.
pub fn plant_discount(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
) -> Option<(i64, i64)> {
    plant_discount_with(game, db, p, card_id, plant_reduction(db, &game.players[p]))
}

/// La réduction en plantes DÉCLARÉE par les cartes en jeu du joueur, sans le
/// moindre test de disponibilité. Séparée de [`plant_discount`] pour que
/// `affordable`, qui examine toute la main, la lise UNE fois au lieu d'une fois
/// par carte : elle ne dépend pas de la carte visée.
fn plant_reduction(db: &CardsDb, pl: &PlayerState) -> Option<(i64, i64)> {
    if !db.effects_on {
        return None;
    }
    for &owned in &pl.played {
        if let Some(spec) = db.projects[owned as usize].effect {
            for r in spec.reductions {
                if let Reduction::PayPlants { plants, amount } = *r {
                    return Some((plants, amount));
                }
            }
        }
    }
    None
}

/// **Le point de décision unique** : la réduction déclarée est-elle disponible
/// pour CETTE carte ? C'est ici, et nulle part ailleurs, que la réserve de
/// plantes est confrontée à ce que la carte visée s'engage elle-même à dépenser.
/// `affordable` et `build_card_with` passent tous deux par cette fonction.
fn plant_discount_with(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    declaree: Option<(i64, i64)>,
) -> Option<(i64, i64)> {
    let (plants, amount) = declaree?;
    let available = game.players[p].plants - plants_reserved_by(db, card_id);
    if available >= plants {
        Some((plants, amount))
    } else {
        None
    }
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
    // (corpo-1) Inventrix : « When playing a card with requirements, you may
    // consider the oxygen or temperature one color HIGHER OR LOWER. » Le
    // prérequis porte sur un PALIER DE COULEUR (violet/rouge/jaune/blanc, bornes
    // du module `effects`) : la souplesse est donc de ±1 palier, jamais de
    // ±1 niveau. Elle ne touche ni les océans (le texte ne les nomme pas), ni
    // les badges, ni les dépenses.
    // La souplesse s'ajoute au test exact par un OU : sans corporation Inventrix
    // le prédicat est bit à bit celui d'avant ce lot (non-régression).
    //
    // (lot cartes-7) *Adaptation Technology* porte le MÊME mécanisme sur une
    // CARTE, et alimente LE MÊME BOOLÉEN. « This cannot be modified further by
    // other effects » est donc encodé par construction : un `||` ne cumule pas.
    // Adaptation Technology + Inventrix = ±1 palier, jamais ±2 (I3).
    //
    // La recherche n'a lieu que si la carte porte réellement un prérequis de
    // PALIER : `flex` n'est lu que dans ces quatre branches, le court-circuit
    // est donc sémantiquement neutre (et il évite de parcourir les cartes en
    // jeu pour les cartes sans prérequis, c'est-à-dire la plupart).
    let palier = spec.reqs.iter().any(|r| {
        matches!(
            r,
            Req::TempMin(_) | Req::TempMax(_) | Req::OxyMin(_) | Req::OxyMax(_)
        )
    });
    //
    // (lot cartes-8) *Special Design* alimente ENCORE le même booléen, mais par
    // un chemin de durée différente : un modificateur armé pour la prochaine
    // carte de la phase (`next_card_mod.color_flex`), et non un permanent. La
    // règle du non-cumul vaut pareil — c'est toujours un `||`.
    let flex = palier
        && (pl.next_card_mod.color_flex
            || corp_effects(db, pl).map_or(false, |s| s.req_color_flex)
            || pl.played.iter().any(|&c| {
                db.projects[c as usize]
                    .effect
                    .map_or(false, |s| s.req_color_flex)
            }));
    let tc = effects::temp_color(temperature) as i16;
    let oc = effects::oxy_color(oxygen) as i16;
    spec.reqs.iter().all(|req| match *req {
        Req::TempMin(n) => {
            temperature >= n || (flex && tc + 1 >= effects::temp_color(n) as i16)
        }
        Req::TempMax(n) => {
            temperature <= n || (flex && tc - 1 <= effects::temp_color(n) as i16)
        }
        Req::OxyMin(n) => oxygen >= n || (flex && oc + 1 >= effects::oxy_color(n) as i16),
        // (lot 6) « Requires red oxygen or lower » — symétrique exact de
        // `TempMax` : palier de couleur, souplesse Inventrix de ±1 palier.
        Req::OxyMax(n) => oxygen <= n || (flex && oc - 1 <= effects::oxy_color(n) as i16),
        Req::OceanMin(n) => oceans >= n,
        Req::OceanMax(n) => oceans <= n,
        Req::Tags(tag, n) => {
            tag.index().map_or(false, |i| pl.tag_counts[i] >= n as u32)
        }
        Req::SpendHeat(n) => pl.heat >= n,
        Req::SpendPlants(n) => pl.plants >= n,
        Req::SpendTr(n) => pl.tr >= n,
        // (lot 5) Seuil de NT SANS dépense (« Requires you to have N or more
        // TR »). Le NT est une ressource de joueur : il se juge à
        // l'état COURANT, comme `Tags` et `Spend*`, jamais sur l'instantané de
        // début de phase — celui-ci ne porte que sur les océans, l'oxygène et la
        // température (livret p.13 l.352). `param` n'entre donc pas ici.
        Req::TrMin(n) => pl.tr >= n,
        // (decouverte-projets) « Requiert un Objectif » (*Private Investor
        // Beach*). Prérequis de JOUEUR — les Objectifs revendiqués sont un état
        // du joueur, pas un paramètre planétaire : il se juge à l'état COURANT,
        // comme `Tags`, `TrMin` et les `Spend*`. `param` n'entre donc pas ici.
        Req::HasObjective => has_objective(game, p),
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

// =============================================================================
// (lot cartes-7) LE TAUX DE DÉFAUSSE — *Composting Factory*, « Cards you discard
// for MC are worth an additional 1 MC. »
//
// Le taux de base est celui du livret : 3 MC par carte, et c'est une règle
// GÉNÉRALE, pas une règle de paiement — l. 96 « à tout moment, **vous pouvez
// défausser une carte Projet de votre main pour gagner 3 MC** », l. 310 (même
// phrase, rappel), l. 348 (paiement d'une carte Projet), l. 437 et 654 (étape de
// fin de manche : « Pour chaque carte ainsi défaussée, le joueur gagne 3 MC,
// **comme toujours** »).
//
// Les QUATRE sites du moteur qui lisaient `SELL_CARD_MC` sont donc tous des
// « cards you discard for MC », la défausse de fin de manche comprise — voir
// §D1 du journal, où cette lecture CONTREDIT celle du contrat (qui supposait
// cette dernière non rémunérée ; le livret et le code disent l'inverse).
//
// Un seul point de calcul (I1), lu au moment où il sert (I6).
// =============================================================================

/// **(lot cartes-7) Point de calcul UNIQUE du taux de défausse** : ce que
/// rapporte UNE carte défaussée pour du MC, pour ce joueur, à cet instant.
///
/// Aucun autre endroit du moteur ne lit `SELL_CARD_MC` pour créditer ou pour
/// juger un paiement : les quatre sites de défausse (affordabilité, paiement
/// d'une carte Projet, vente de carte en phase Action, étape de fin de manche)
/// et la politique de défausse-paiement passent tous par ici.
///
/// `--effects off` : exactement le taux du livret, `SELL_CARD_MC`. Le taux est
/// une RÈGLE, pas un effet ; seul le supplément est un effet de carte.
pub fn discard_mc_rate(db: &CardsDb, pl: &PlayerState) -> i64 {
    if !db.effects_on {
        return SELL_CARD_MC;
    }
    let mut rate = SELL_CARD_MC;
    for &c in &pl.played {
        if let Some(spec) = db.projects[c as usize].effect {
            rate += spec.discard_bonus;
        }
    }
    // (jokers-corpos) La CORPORATION est une source de supplément comme les
    // cartes (Exocorp) : elle entre dans la MÊME somme, il n'y a pas de second
    // calcul du taux. Le commentaire d'origine disait que ce service « ne lit
    // que les cartes posées » — ce n'est plus vrai.
    if let Some(spec) = corp_effects(db, pl) {
        rate += spec.discard_bonus;
    }
    rate
}

/// **(jokers-corpos) Le SUPPLÉMENT du taux de défausse**, en MC par carte :
/// ce que le taux réel du joueur dépasse celui du livret. C'est cette grandeur,
/// et elle seule, que `discard_bonus_mc` accumule aux sites de crédit.
fn discard_bonus_per_card(db: &CardsDb, pl: &PlayerState) -> u64 {
    (discard_mc_rate(db, pl) - SELL_CARD_MC).max(0) as u64
}

// =============================================================================
// (lot cartes-7) LA RÉDUCTION DES ACTIONS STANDARD — *Standard Technology*,
// « You pay 4 MC less for standard actions that cost MC. »
//
// Les actions standard payantes en MC sont exactement trois (livret p.14) :
// forêt 20 MC, température 14 MC, océan 15 MC. La réduction ne touche NI la
// forêt payée en 8 plantes, NI la température payée en 8 chaleurs — elles ne
// coûtent pas de MC — NI la vente de carte, qui RAPPORTE des MC (NEVER 8).
//
// `standard_mc_cost` est le seul point de calcul : `action_options` (le prédicat
// qui décide si l'action est PROPOSÉE) et `phase_action` (le PAIEMENT) l'appellent
// tous deux, ils ne peuvent donc pas diverger (I2) — sans quoi le moteur
// proposerait une action qu'il ne saurait pas payer, ou refuserait une action
// payable.
// =============================================================================

/// (lot cartes-7) Somme des réductions d'actions standard des cartes en jeu.
/// 0 en `--effects off`.
pub fn standard_action_discount(db: &CardsDb, pl: &PlayerState) -> i64 {
    if !db.effects_on {
        return 0;
    }
    let mut d = 0;
    for &c in &pl.played {
        if let Some(spec) = db.projects[c as usize].effect {
            d += spec.standard_discount;
        }
    }
    d
}

/// **(lot cartes-7) Point de calcul UNIQUE du prix d'une action standard payée
/// en MC** : le prix du livret moins la réduction du joueur, jamais négatif.
pub fn standard_mc_cost(db: &CardsDb, pl: &PlayerState, base: i64) -> i64 {
    standard_mc_cost_with(base, standard_action_discount(db, pl))
}

/// **La formule, et elle n'existe qu'ici** : prix du livret moins la réduction,
/// jamais négatif. Séparée pour que `action_options`, qui juge les trois actions
/// standard d'un coup et tourne dans la boucle serrée de la phase III, lise la
/// réduction UNE fois au lieu de trois.
fn standard_mc_cost_with(base: i64, discount: i64) -> i64 {
    (base - discount).max(0)
}

/// (C3) Une carte de coût effectif `cost` est-elle payable par un joueur qui a
/// `mc` MC et `hand_len` cartes en main (la carte à poser comprise) ? Livret
/// p.13, l.348 : MC **et/ou** défausse de cartes à `rate` MC/carte. La carte
/// posée ne pouvant pas se payer elle-même, la monnaie disponible est
/// `hand_len - 1`.
/// Prédicat UNIQUE d'affordabilité : consommé par `affordable` (énumération des
/// options du flux réel) et par la sonde. `build_card_with` en est la
/// contrepartie exacte — il paie de la même façon et assère le résultat — de
/// sorte que les deux ne peuvent pas diverger.
///
/// (lot cartes-7) `rate` vient du service unique [`discard_mc_rate`] : aucun
/// appelant ne fabrique ce taux lui-même.
pub fn payable(mc: i64, hand_len: usize, cost: i64, rate: i64) -> bool {
    mc + rate * (hand_len as i64 - 1).max(0) >= cost
}

/// Indices de main constructibles pour une couleur donnée : paiement (MC et/ou
/// défausse, C3) ET prérequis de la couche d'effets satisfaits (sur
/// l'instantané, C1).
///
/// Prend `&mut GameState` pour alimenter le compteur d'audit
/// `prereq_snapshot_blocks` à l'endroit EXACT où l'exclusion a lieu.
// =============================================================================
// (lot cartes-8) LES POSES SUPPLÉMENTAIRES
//
// Cinq cartes accordent le droit de poser une carte de plus dans la phase en
// cours. Plutôt qu'un chemin de pose par carte, le moteur décrit CHAQUE pose —
// y compris les poses ordinaires des phases I et II — par un `BuildGrant`, et
// n'a qu'un seul chemin pour l'exercer (I1). Ajouter demain « jouez une carte
// rouge de plus » ne demandera pas une ligne de flux de jeu, seulement une
// entrée de données.
// =============================================================================

/// Pose ORDINAIRE de la phase I — Développement : une carte verte, sans plafond
/// de prix, payante.
pub const GRANT_DEVELOPMENT: BuildGrant = BuildGrant {
    colors: &[Color::Green],
    max_printed_cost: None,
    free: false,
};

/// Pose ORDINAIRE de la phase II — Construction : une carte bleue ou rouge,
/// sans plafond de prix, payante. C'est aussi la permission du sélectionneur de
/// phase (« ou en jouer une 2e ») : le livret en fait une pose de plus, à
/// l'identique.
pub const GRANT_CONSTRUCTION: BuildGrant = BuildGrant {
    colors: &[Color::Blue, Color::Red],
    max_printed_cost: None,
    free: false,
};

/// Garde-fou : nombre maximal de poses supplémentaires exercées par un joueur
/// dans une même phase. Aucune carte de la boîte de base n'en accorde plus de
/// deux d'affilée ; cette borne n'existe que pour qu'un encodage fautif
/// (une carte qui s'accorderait une permission à elle-même) s'arrête net au
/// lieu de faire tourner la partie sans fin. Un dépassement casse la partie
/// plutôt que de la fausser en silence (NEVER 3).
const MAX_EXTRA_BUILDS_PER_PHASE: usize = 8;

/// (lot cartes-8) Réduction armée pour la prochaine carte de la phase —
/// service unique, lu par `affordable`, par `build_card_granted` ET par la
/// sonde (I2). Publique pour cette dernière raison : `probe.rs` recalcule le
/// prix pour son compte, et doit voir exactement ce que voit le paiement.
pub fn next_card_discount(pl: &PlayerState) -> i64 {
    pl.next_card_mod.discount
}

/// (lot cartes-8) Enregistre ce que la carte qui vient d'entrer en jeu accorde :
/// des permissions de pose, et/ou un modificateur pour la carte suivante.
///
/// Appelé depuis `build_card_granted` APRÈS que la carte a réellement été mise
/// en jeu et ses effets appliqués — une carte qui n'a pas été posée n'accorde
/// rien. Sans effet quand la couche d'effets est coupée (`--effects off`), comme
/// tout le reste de cette couche (I7).
fn grant_from_card(game: &mut GameState, db: &CardsDb, p: usize, card_id: u16) {
    if !db.effects_on {
        return;
    }
    let Some(spec) = db.projects[card_id as usize].effect else {
        return;
    };
    for g in spec.grants {
        game.players[p].pending_builds.push(*g);
        game.extra_builds_granted += 1;
    }
    if let Some(m) = spec.next_card {
        // Cumul et non remplacement pour la réduction (deux *Work Crews* dans
        // la même phase se suivent) ; « ou » pour la souplesse, qui reste
        // binaire par construction (I3).
        game.players[p].next_card_mod.discount += m.discount;
        game.players[p].next_card_mod.color_flex |= m.color_flex;
        game.next_card_mods_armed += 1;
    }
}

/// (lot cartes-8) Exerce les permissions de pose en attente, tant qu'il en reste
/// et que le joueur veut bien s'en servir.
///
/// La boucle est nécessaire, pas décorative : une carte posée sous permission
/// peut elle-même en accorder une nouvelle (*Special Design* posée grâce à
/// *Work Crews*, par exemple). Elle s'arrête quand la file est vide, quand la
/// politique renonce, ou quand plus rien n'est posable.
fn drain_pending_builds(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    discount: i64,
    policy: &mut dyn Policy,
) {
    let mut exercised = 0usize;
    while let Some(grant) = game.players[p].pending_builds.pop() {
        assert!(
            exercised < MAX_EXTRA_BUILDS_PER_PHASE,
            "plus de {MAX_EXTRA_BUILDS_PER_PHASE} poses supplémentaires dans une \
             même phase : encodage fautif (une carte s'accorde une permission \
             qui se régénère)"
        );
        exercised += 1;
        // La remise de phase du sélectionneur ne s'applique QU'À la pose
        // ordinaire : le livret l'attache au choix de la phase, pas à la carte.
        // Une permission offerte ne reçoit évidemment aucune remise non plus.
        let disc = if grant.free { 0 } else { discount };
        // (jokers-corpos) Les badges jokers de la main reçoivent leur jeton AVANT
        // l'énumération : `affordable` juge alors chaque carte joker sur son
        // badge réel, exactement comme le paiement le fera (I2).
        resolve_hand_jokers(game, db, p, policy);
        let opts = affordable(game, db, p, &grant, disc);
        // « You MAY play an additional card » : renoncer est une option, et
        // c'est `Policy` qui tranche — jamais le moteur (I4).
        policy.observe(&game, p);
        let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) else {
            continue;
        };
        assert!(opts.contains(&idx), "choix de construction hors options");
        build_card_granted(game, db, p, idx, disc, &grant, policy);
        game.extra_builds_used += 1;
    }
}

// (decouverte-projets) Rendue PUBLIQUE, comme `apply_blue_action` avant elle :
// c'est le point d'entrée UNIQUE de l'affordabilité, et l'invariant I2
// (« affordabilité et paiement ne divergent jamais ») ne peut se démontrer
// qu'en interrogeant CETTE fonction-là, celle que la phase de jeu emprunte —
// pas une copie, pas le garde-fou de la sonde.
pub fn affordable(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    grant: &BuildGrant,
    discount: i64,
) -> Vec<usize> {
    let hand_len = game.players[p].hand.len();
    // (lot cartes-8) Réduction armée pour la prochaine carte de la phase
    // (*Work Crews*). Elle entre dans l'affordabilité exactement comme dans le
    // paiement — sinon une carte devenue payable serait jugée hors budget (I2).
    let discount = discount + next_card_discount(&game.players[p]);
    let mut out = Vec::new();
    let mut blocked = 0u64;
    // (lot 3) Réduction payable en microbes : elle compte dans l'affordabilité,
    // sinon une carte jouable serait jugée hors budget (contrat). Calculée une
    // fois par énumération : elle ne dépend pas de la carte examinée.
    let payable_disc = microbe_discount(game, db, p).map_or(0, |(_, _, a)| a);
    // (lot cartes-7) Même taux de défausse que le paiement (I1/I2) : il ne
    // dépend pas de la carte examinée, il est donc lu une fois.
    let rate = discard_mc_rate(db, &game.players[p]);
    // (lot cartes-7) La réduction en plantes DÉCLARÉE ne dépend pas de la carte
    // examinée : lue une fois. Sa DISPONIBILITÉ, elle, en dépend — c'est
    // `plant_discount_with` qui la tranche, dans la boucle, exactement comme
    // `build_card_with` le fait au paiement (I2).
    let plant_red = plant_reduction(db, &game.players[p]);
    for i in 0..hand_len {
        let c = game.players[p].hand[i];
        let card = &db.projects[c as usize];
        // (lot cartes-8) Couleur ET plafond de prix IMPRIMÉ : les deux critères
        // de la permission, jugés ensemble par le service unique
        // `BuildGrant::admits`, le même que celui du paiement (I2).
        if !grant.admits(card.color, card.price) {
            continue;
        }
        // (lot cartes-8) Permission OFFERTE (« without paying its MC cost ») :
        // aucun budget à vérifier. Seuls les prérequis de la carte restent
        // opposables — ils ne sont pas son prix.
        if grant.free {
            if requirements_met(game, db, p, c) {
                out.push(i);
            } else if requirements_met_now(game, db, p, c) {
                blocked += 1;
            }
            continue;
        }
        // (lot cartes-7) Réduction payable en plantes : elle compte dans
        // l'affordabilité, sinon une carte jouable serait jugée hors budget.
        // Elle dépend de la carte visée (ses propres plantes réservées), d'où
        // le calcul DANS la boucle — même service que `build_card_with`.
        let plant_disc = plant_discount_with(game, db, p, c, plant_red).map_or(0, |(_, a)| a);
        let cost = effective_cost(
            card.price,
            discount + card_discount(game, db, p, c) + payable_disc + plant_disc,
        );
        // (corpo-1) Helion : la chaleur compte dans l'affordabilité, sinon une
        // carte payable serait jugée hors budget — MOINS celle que la carte
        // s'engage à dépenser à la pose. Sans Helion, vaut exactement `pl.mc`.
        let mc = spendable_mc_reserving(db, &game.players[p], heat_reserved_by(db, c));
        if !payable(mc, hand_len, cost, rate) {
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
    apply_res_steps(game, db, p, card_id, spec.on_build, policy, UpgradeSource::Build);
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
                gain_tr(game, db, p, policy);
            }
        }
        Eff::Infrastructure(n) => {
            for _ in 0..n {
                raise_infrastructure(game, db, p, policy);
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
                gain_tr(game, db, p, policy);
            }
            game.tr_from_tags += steps as u64;
        }
        // (lot 5) Gain de forêt SANS paiement (groupe C). Chaque pas emprunte
        // `gain_forest`, exactement le chemin de l'action standard payée : un
        // seul chemin, donc un seul comptage de PV, une seule hausse d'oxygène
        // par forêt (R1) et le déclencheur « when you gain a forest VP » levé
        // une fois par forêt (R2). Aucun nom de carte n'intervient : la quantité
        // vient de la table d'effets.
        Eff::Forest(n) => {
            for _ in 0..n {
                gain_forest(game, db, p, policy);
            }
        }
        // (lot 6) « Piochez n cartes, puis défaussez-en d » — brique UNIQUE des
        // trois cartes du groupe C (I3). Les cartes piochées entrent d'abord en
        // main (elles sont donc défaussables), puis la défausse est choisie par
        // `Policy::discard_down`, le point de décision existant : aucune source
        // de hasard nouvelle (I6).
        Eff::DrawDiscard {
            draw,
            discard,
            from_drawn,
        } => {
            let mut drawn: Vec<u16> = Vec::with_capacity(draw as usize);
            for _ in 0..draw {
                if let Some(c) = draw_card(game) {
                    game.players[p].hand.push(c);
                    drawn.push(c);
                }
            }
            // « Keep one of THEM » restreint la défausse aux cartes piochées ;
            // « Then, discard N cards » porte sur la main entière.
            let cands: Vec<u16> = if from_drawn {
                drawn
            } else {
                game.players[p].hand.clone()
            };
            // Combien défausser ? Les deux formulations imprimées ne comptent
            // PAS la même chose, et elles ne coïncident que si la pioche a
            // réellement rendu les `draw` cartes attendues :
            // - « **Keep one of them** and discard the other two » compte les
            //   cartes GARDÉES : si la pioche épuisée n'en a rendu que deux, le
            //   joueur en garde toujours UNE et n'en défausse qu'une ;
            // - « Then, discard N cards » compte les cartes DÉFAUSSÉES, sans
            //   restriction : on en défausse N, bornées par la main.
            let n = if from_drawn {
                let keep = draw.saturating_sub(discard) as usize;
                cands.len().saturating_sub(keep)
            } else {
                (discard as usize).min(cands.len())
            };
            if n == 0 {
                return;
            }
            policy.observe(&game, p);
            let idx = policy.discard_down(&mut game.rng, p, &cands, n);
            for &i in idx.iter().take(n) {
                if i >= cands.len() {
                    continue; // renoncement explicite (convention du lot 3)
                }
                if discard_from_hand(game, p, cands[i]) {
                    game.draw_discard_discards += 1;
                }
            }
        }
        // (decouverte-projets) « Si vous avez un Objectif, gagnez … »
        // La condition est jugée ICI,
        // donc à l'instant de la pose (ASK 3) : le moteur n'a aucun mécanisme
        // de rappel, un Objectif revendiqué plus tard ne rétro-paie rien.
        //
        // Le compteur est incrémenté au MÊME endroit que le versement : il ne
        // peut pas bouger sans que le gain soit versé, ni l'inverse (clause
        // anti-shortcut n° 5).
        Eff::IfObjective(effs) => {
            if !has_objective(game, p) {
                return;
            }
            game.objective_condition_hits += 1;
            for e in effs {
                apply_eff(game, db, p, *e, policy);
            }
        }
    }
}

/// (lot 6) Défausse UNE carte nommée de la main du joueur vers la défausse
/// commune. Point d'écriture unique des deux mécanismes de défausse du lot 6
/// (`Eff::DrawDiscard` et `ActionCost::DiscardCard`) : la carte quitte la main
/// et rejoint `game.discard`, jamais autre chose — la conservation des cartes
/// (invariant 4) est ainsi vraie par construction.
/// Renvoie `true` si la carte était bien en main (un identifiant de carte est
/// unique dans la base : la recherche par valeur ne peut pas se tromper de
/// carte, et aucun indice de main ne peut être invalidé par un retrait
/// précédent).
fn discard_from_hand(game: &mut GameState, p: usize, card: u16) -> bool {
    match game.players[p].hand.iter().position(|&x| x == card) {
        Some(pos) => {
            game.players[p].hand.remove(pos);
            game.discard.push(card);
            true
        }
        None => false,
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
        let (m, h, pl_) = card_derived_production(db, pl, c);
        mc += m;
        heat += h;
        plants += pl_;
    }
    (mc, heat, plants)
}

/// (lot 4, isolé au chantier `decouverte-phases`) La production DÉRIVÉE d'UNE
/// carte en jeu. `derived_production` en est la somme sur les cartes du joueur ;
/// le bonus de la carte Phase IV-A en rejoue exactement une. Une seule
/// implémentation de la division entière, comme avant (NEVER 2).
pub fn card_derived_production(db: &CardsDb, pl: &PlayerState, card_id: u16) -> (i64, i64, i64) {
    if !db.effects_on {
        return (0, 0, 0);
    }
    let Some(spec) = db.projects[card_id as usize].effect else {
        return (0, 0, 0);
    };
    let Some(prod) = spec.prod else {
        return (0, 0, 0);
    };
    if prod.per == 0 {
        return (0, 0, 0);
    }
    let counted = match prod.count {
        ProdCount::Tag(t) => t.index().map_or(0, |i| pl.tag_counts[i] as i64),
        ProdCount::Forests => pl.forests,
    };
    let gained = counted / prod.per as i64;
    match prod.res {
        ProdRes::Mc => (gained, 0, 0),
        ProdRes::Heat => (0, gained, 0),
        ProdRes::Plants => (0, 0, gained),
    }
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
    // (corpo-1) La corporation alimente le MÊME cumul (Tharsis Republic, texte
    // identique à Interplanetary Relations) : un joueur qui a les deux gagne
    // 2/2, comme deux cartes identiques.
    if let Some(bonus) = corp_effects(db, pl).and_then(|s| s.research) {
        draw += bonus.draw;
        keep += bonus.keep;
    }
    (draw, keep)
}

/// (lot 4) Base du livret pour la phase V (p.15) : la COMPÉTENCE imprimée
/// — « tous les joueurs piochent 2 cartes et en conservent 1 » — plus le BONUS
/// du sélectionneur, lu au point de calcul unique.
///
/// La compétence est la même sur la carte de base et sur les deux améliorées
/// (ASK 2) : seul le bonus change. Carte de base : +3 / +1, soit les 5/2
/// historiques. V-A : +2 / +2 → 4 piochées, 3 conservées. V-B : +6 / +1 →
/// 8 piochées, 2 conservées.
pub fn research_base(db: &CardsDb, pl: &PlayerState) -> (usize, usize) {
    let b = selector_bonus(db, pl, 5);
    (2 + b.research_draw, 1 + b.research_keep)
}

/// (lot 4) Cartes piochées / gardées en phase V par un joueur : base du livret
/// (`research_base`) + bonus permanent de ses cartes en jeu (`research_extra`).
/// Joueur ordinaire 2/1 → 3/2 ; sélectionneur 5/2 → 6/3. Chemin unique,
/// consommé par `phase_research`.
pub fn research_draw_keep(db: &CardsDb, pl: &PlayerState) -> (usize, usize) {
    let (base_n, base_keep) = research_base(db, pl);
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
    // (lot cartes-8) Une pose ordinaire est une pose sous permission ordinaire :
    // il n'existe qu'UN chemin de pose dans le moteur (I1). La permission choisie
    // ici n'impose aucune couleur — `build_card_with` est aussi le chemin de la
    // sonde et des tests, qui posent la carte qu'on leur nomme.
    const GRANT_ANY: BuildGrant = BuildGrant {
        colors: &[Color::Green, Color::Blue, Color::Red],
        max_printed_cost: None,
        free: false,
    };
    build_card_granted(game, db, p, idx, discount, &GRANT_ANY, policy)
}

/// (lot cartes-8) Le chemin de pose complet, permission comprise. Seule
/// différence avec `build_card_with` : la permission peut rendre la carte
/// GRATUITE (« without paying its MC cost »).
pub fn build_card_granted(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    idx: usize,
    discount: i64,
    grant: &BuildGrant,
    policy: &mut dyn Policy,
) -> usize {
    let hand_len_before = game.players[p].hand.len();
    let card_id = game.players[p].hand.remove(idx);
    // (jokers-corpos) Le jeton Badge est posé AVANT tout calcul de prix : c'est
    // l'exemple du livret (« si vous choisissez le badge Espace, les
    // savoir-faire Titanium réduiront le coût en MC pour jouer la carte »).
    // Normalement déjà fait par `resolve_hand_jokers` avant l'abordabilité ; ce
    // second appel est le garde-fou des chemins qui posent une carte sans passer
    // par une énumération (la sonde, les tests), et il ne réécrit jamais un
    // jeton déjà posé.
    ensure_joker_tag(game, db, p, card_id, policy);
    // (lot cartes-8) Le modificateur armé pour la prochaine carte de la phase
    // (*Work Crews*, *Special Design*) est consommé PAR CETTE POSE : lu ici,
    // effacé aussitôt. Il vaut pour la carte qu'on est en train de poser, pas
    // pour la suivante — d'où la lecture avant tout calcul de prix, et
    // l'effacement avant que la carte n'entre en jeu (si elle arme à son tour
    // un modificateur, celui-ci vise bien la pose SUIVANTE).
    let armed = std::mem::take(&mut game.players[p].next_card_mod);
    if !armed.is_empty() {
        game.next_card_mods_used += 1;
    }
    // Réduction totale = remise de phase (sélectionneur) + réductions des cartes
    // en jeu (service unique `card_discount`, calculé AVANT la mise en jeu)
    // + réduction armée pour cette carte-ci.
    let fixed_discount = discount + card_discount(game, db, p, card_id) + armed.discount;
    // (lot cartes-8) Permission OFFERTE : le prix devient nul. C'est le prix qui
    // tombe, pas une réduction de plus — « without paying its MC cost » ne
    // rembourse rien et ne se cumule à rien.
    let price = if grant.free {
        game.free_builds += 1;
        0
    } else {
        db.projects[card_id as usize].price
    };

    // (lot 3) Réduction payée en microbes (Anaerobic Microorganisms) : c'est un
    // CHOIX du joueur, pas un automatisme. La branche « y renoncer » n'est
    // proposée que si elle est jouable, c'est-à-dire si la carte reste payable
    // sans la réduction (règle générale de filtrage des branches — journal D7).
    let mut pay_with_resources: Option<(u16, u32)> = None;
    let mut total_discount = fixed_discount;
    // (corpo-1) Chaleur que CETTE carte s'engage à dépenser : Helion ne peut pas
    // la convertir en MC pour en payer le prix.
    let reserved_heat = heat_reserved_by(db, card_id);
    // (lot cartes-7) Taux de défausse du joueur À CET INSTANT, service unique —
    // le même que celui qu'a employé `affordable` pour proposer cette carte.
    let rate = discard_mc_rate(db, &game.players[p]);
    if let Some((src, count, amount)) = microbe_discount(game, db, p) {
        let cost_without = effective_cost(price, total_discount);
        let can_decline = payable(
            spendable_mc_reserving(db, &game.players[p], reserved_heat),
            hand_len_before,
            cost_without,
            rate,
        );
        // Branche 0 = utiliser la réduction (l'option imprimée) ; branche 1 = y
        // renoncer.
        let use_it = if can_decline {
            let ctx = ChoiceContext::MicrobeDiscount {
                card: card_id,
                holder: src,
                count,
                amount,
            };
            policy.observe(&game, p);
            policy.choose_option_ctx(&mut game.rng, p, &ctx) == 0
        } else {
            true
        };
        if use_it {
            pay_with_resources = Some((src, count));
            total_discount += amount;
        }
    }

    // (lot cartes-7) Réduction payée en PLANTES (*Restructured Resources*) :
    // même forme exactement que la précédente, monnaie mise à part. Le « may »
    // est un choix de `Policy` (I4), et la branche « y renoncer » n'est proposée
    // que si elle est jouable — c'est-à-dire si la carte reste payable sans la
    // réduction. Les plantes que la carte visée s'engage elle-même à dépenser
    // sont déjà mises de côté par `plant_discount`.
    let mut pay_with_plants: Option<i64> = None;
    if let Some((plants, amount)) = plant_discount(game, db, p, card_id) {
        let cost_without = effective_cost(price, total_discount);
        let can_decline = payable(
            spendable_mc_reserving(db, &game.players[p], reserved_heat),
            hand_len_before,
            cost_without,
            rate,
        );
        let use_it = if can_decline {
            let ctx = ChoiceContext::PlantDiscount {
                card: card_id,
                plants,
                amount,
            };
            policy.observe(&game, p);
            policy.choose_option_ctx(&mut game.rng, p, &ctx) == 0
        } else {
            true
        };
        if use_it {
            pay_with_plants = Some(plants);
            total_discount += amount;
        }
    }

    let cost = effective_cost(price, total_discount);
    assert!(cost >= 0, "prix payé négatif (réduction non plafonnée)");

    // (corpo-1) Helion : « You MAY use heat as MC ». Ici — et ici seulement — le
    // joueur a une vraie alternative, puisque le livret lui offre déjà de payer
    // en défaussant des cartes à 3 MC. Le choix passe donc par le même chemin
    // que tous les « ou » du moteur (`Policy::choose_option`, branche 0 =
    // employer la chaleur, l'option imprimée ; branche 1 = y renoncer), et il
    // n'est proposé QUE s'il en est un : si la carte n'est pas payable sans la
    // chaleur, il n'y a pas d'alternative à présenter (convention du lot 3 —
    // `choose_option` n'est appelée qu'à partir de 2 branches jouables).
    //
    // Partout ailleurs (actions standard, actions de cartes bleues, pas de NT
    // d'Unmi), aucune défausse n'est offerte : renoncer à la chaleur y
    // reviendrait à renoncer à l'action, ce n'est pas une branche jouable.
    if heat_as_mc(db, &game.players[p]) && game.players[p].mc < cost {
        // La carte à poser est déjà retirée de la main : la monnaie de défausse
        // disponible est `hand.len()`, d'où le `+ 1` attendu par `payable`.
        let can_decline =
            payable(game.players[p].mc, game.players[p].hand.len() + 1, cost, rate);
        let use_heat = if can_decline {
            let ctx = ChoiceContext::HeatAsMc {
                card: card_id,
                cost,
            };
            policy.observe(&game, p);
            policy.choose_option_ctx(&mut game.rng, p, &ctx) == 0
        } else {
            true
        };
        if use_heat {
            top_up_mc_with_heat_reserving(game, db, p, cost, reserved_heat);
        }
    }

    // (C3) Paiement : d'abord les MC, puis la défausse pour le reste. Le
    // nombre de cartes vient de la politique (défaut du trait = minimum).
    let mut discarded = 0usize;
    if game.players[p].mc < cost {
        let hand = game.players[p].hand.clone();
        // (lot cartes-7) La politique décide COMBIEN de cartes défausser : elle
        // reçoit donc le taux réel du joueur, sinon elle en défausserait trop
        // (elle divise le manque par le taux).
        policy.observe(&game, p);
        let n =
            policy.discard_payment_count(&mut game.rng, p, game.players[p].mc, cost, &hand, rate);
        assert!(n <= game.players[p].hand.len(), "défausse-paiement hors main");
        // Quelles cartes : les DERNIÈRES de la main. Le livret laisse le choix
        // libre ; prendre par la fin est déterministe, en O(1), et préserve la
        // tête de main — ce dont dépend la sonde séquence, qui pose toujours à
        // l'indice 0.
        // (jokers-corpos) Ce que le taux MAJORÉ verse au-delà du livret, compté
        // à l'endroit exact du crédit.
        let bonus = discard_bonus_per_card(db, &game.players[p]);
        for _ in 0..n {
            let card = game.players[p].hand.pop().expect("défausse-paiement hors main");
            game.discard.push(card);
            game.players[p].mc += rate;
            game.discard_bonus_mc += bonus;
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
    // (lot cartes-7) Idem pour les plantes de *Restructured Resources* : elles
    // ne quittent la réserve QUE maintenant, la carte étant effectivement posée.
    if let Some(n) = pay_with_plants {
        assert!(game.players[p].plants >= n, "dépense de plantes sans les plantes");
        game.players[p].plants -= n;
    }
    // (jokers-corpos) `put_in_play` dit si un badge joker DÉTERMINÉ vient
    // d'entrer dans les compteurs de badges : c'est l'unique passage par lequel
    // il compte pour les prérequis, les productions et points par badge, les
    // Objectifs et les Récompenses. Compté ici, où seul `GameState` est
    // accessible. Le badge d'une carte ROUGE y entre comme les autres : le
    // moteur garde les événements en jeu, et le livret de base dit qu'une carte
    // rouge n'a plus d'effet « autre que les badges qu'elle fournit ».
    if game.players[p].put_in_play(card_id, db) {
        game.joker_tag_hits += 1;
    }
    // (lot acier-titane) La carte vient d'entrer en jeu : si elle porte un
    // savoir-faire, le compte change MAINTENANT. Rafraîchi ici, juste après
    // `put_in_play` et AVANT tout effet — une carte qui gagnerait un acier et
    // jouerait dans la foulée doit voir le compte à jour. Le prix de CETTE
    // carte, lui, a été calculé plus haut, avant la mise en jeu : elle ne se
    // réduit jamais elle-même, comme toutes les réductions du moteur.
    refresh_capacities(game, db, p);
    // (boites-1) I4 — aucun pouvoir sauté en silence. Une carte dont le texte
    // imprimé n'est pas intégralement appliqué vient d'entrer en jeu : soit
    // elle n'a aucun encodage, soit son encodage porte un effet que le moteur
    // saute (amélioration de phase). Compté ici, à l'endroit de la pose.
    //
    // (decouverte-projets) Il ne compte QUE si la couche d'effets est active —
    // voir `install_corporation` pour la raison. Le commentaire d'origine
    // disait « le compteur ne dépend pas de --effects » ; c'était vrai, et
    // c'était le défaut.
    if !db.projects[card_id as usize].effets_geres() && db.effects_on {
        game.cards_effects_unhandled += 1;
    }
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
    // (lot cartes-8) La carte est en jeu et ses effets sont passés : elle peut
    // maintenant accorder une pose supplémentaire ou armer un modificateur pour
    // la suivante. APRÈS `fire_play_triggers`, donc : ce qu'elle accorde ne
    // dépend pas de l'ordre des déclencheurs, et le modificateur qu'elle arme
    // survit à l'effacement fait plus haut pour son propre compte.
    grant_from_card(game, db, p, card_id);
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
    // (jokers-corpos) Les badges de la carte posée VUS PAR SON PROPRIÉTAIRE :
    // un joker déclaré Science déclenche « lorsque vous jouez un badge
    // science », un joker déclaré Énergie déclenche celui de Sultira.
    let played_tags = game.players[p].tags_of(db, played_id);

    // (corpo-1) La CORPORATION est une source de déclencheurs comme les autres
    // (Saturn Systems : « Each time you play a [jupiter] … gain 1 TR »). Elle
    // n'est jamais « jouée » : « excluding this » n'exige donc aucun traitement,
    // son propre badge ne déclenche rien. Elle ne porte pas de ressources, d'où
    // `src = None` (voir `apply_trig_gain`).
    if let Some(spec) = corp_effects(db, &game.players[p]) {
        let triggers = spec.play_triggers;
        for trig in triggers {
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
                apply_trig_gain(game, db, p, None, *g, mult, policy);
            }
        }
    }

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
                apply_trig_gain(game, db, p, Some(src), *g, mult, policy);
            }
        }
    }
}

/// Applique un gain de déclencheur `mult` fois (facteur = nb de tags satisfaits
/// pour les déclencheurs « par tag », 1 sinon). `src` = carte qui porte le
/// déclencheur : c'est elle qui reçoit les ressources de `ResSelf` et qui sert
/// de référence à « ANOTHER card » dans une alternative.
/// `src = None` : le déclencheur vient de la CORPORATION, qui n'est pas une
/// carte en jeu — les gains à ressources n'y ont pas de réceptacle et sont
/// interdits d'encodage (assertion, pas un cas de jeu).
fn apply_trig_gain(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    src: Option<u16>,
    g: TrigGain,
    mult: i64,
    policy: &mut dyn Policy,
) {
    match g {
        TrigGain::Heat(n) => game.players[p].heat += n * mult,
        TrigGain::Plants(n) => game.players[p].plants += n * mult,
        // (corpo-1) Saturn Systems. Passe par le service unique de hausse de NT
        // (comptabilité de l'invariant TR + `TrBoost` éventuel d'Unmi).
        TrigGain::Tr(n) => {
            let steps = n as i64 * mult.max(0);
            for _ in 0..steps {
                gain_tr(game, db, p, policy);
            }
            if src.is_none() {
                game.corp_trigger_tr += steps as u64;
            }
        }
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
            let src = src.expect("ResSelf sans carte source (déclencheur de corporation)");
            add_resources(game, db, p, src, n * mult.max(0) as u32)
        }
        // (lot 3, CORRIGÉ par moteur-verite-1) Alternative : résolue `mult`
        // fois, comme tout autre gain. Le livret p.9 l.106 tranche : « Si la
        // condition d'un effet est remplie plusieurs fois lorsqu'une carte est
        // jouée, résolvez l'effet correspondant plusieurs fois. » Une carte à
        // deux badges satisfaisants (Adapted Lichen = [microbe]+[plant])
        // déclenche donc DEUX résolutions de Viral Enhancers / Decomposers.
        // Chaque résolution rappelle la politique : le joueur peut choisir une
        // branche différente à chaque fois, ce que le texte imprimé autorise.
        TrigGain::Choose(branches) => {
            let src = src.expect("Choose sans carte source (déclencheur de corporation)");
            for _ in 0..mult.max(0) {
                apply_choice(game, db, p, src, branches, policy, UpgradeSource::Build);
            }
        }
        // (lot cartes-7) *Mars University* : « you MAY discard a card. If that
        // card had a [plant], draw two cards. Otherwise, draw a card. »
        //
        // Résolue `mult` fois comme tout autre gain (livret p.9 l.106). À chaque
        // résolution :
        //   1. la branche « défausser » est FILTRÉE si la main est vide — à zéro
        //      branche jouable, aucune question n'est posée (convention lot 3) ;
        //   2. le « may » est un vrai choix de `Policy` (I4), branche 0 =
        //      défausser (l'option imprimée), branche 1 = renoncer ;
        //   3. QUELLE carte est un `Policy::discard_down(hand, 1)`, le point de
        //      décision existant — aucune source de hasard nouvelle ;
        //   4. le badge regardé est celui de la carte DÉFAUSSÉE, lu avant
        //      qu'elle quitte la main, et la défausse passe par le chemin
        //      unique `discard_from_hand`.
        TrigGain::MayDiscardDraw {
            if_tag,
            draw_if,
            draw_else,
        } => {
            for _ in 0..mult.max(0) {
                if game.players[p].hand.is_empty() {
                    break;
                }
                // `src` vaut `None` quand le déclencheur est porté par la
                // planche de CORPORATION, qui n'est pas une carte en jeu :
                // l'inconnue est déclarée telle quelle, jamais comblée par une
                // carte plausible.
                let ctx = ChoiceContext::DiscardToDraw {
                    card: src,
                    tag: if_tag,
                    draw_if,
                    draw_else,
                };
                policy.observe(&game, p);
                if policy.choose_option_ctx(&mut game.rng, p, &ctx) != 0 {
                    continue;
                }
                let hand = game.players[p].hand.clone();
                policy.observe(&game, p);
                let idx = policy.discard_down(&mut game.rng, p, &hand, 1);
                let Some(&i) = idx.first() else { continue };
                if i >= hand.len() {
                    continue; // renoncement explicite (convention du lot 3)
                }
                let card = hand[i];
                // (jokers-corpos) Badges vus par le joueur : une carte joker
                // défaussée compte pour le badge qu'il lui a donné.
                let had = game.players[p].tags_of(db, card).contains(&if_tag);
                if !discard_from_hand(game, p, card) {
                    continue;
                }
                let n = if had { draw_if } else { draw_else };
                for _ in 0..n {
                    if let Some(c) = draw_card(game) {
                        game.players[p].hand.push(c);
                    }
                }
            }
        }
    }
}

/// **(lot cartes-7) « When you use an "Action:" effect on one of your cards »** —
/// *Assembly Lines*.
///
/// Levé par [`apply_blue_action`] APRÈS une activation qui a réellement produit
/// un effet, pour toutes les cartes en jeu du joueur qui portent un
/// `action_trigger`. Les actions STANDARD (forêt, température, océan, vente de
/// carte) ne passent pas par `apply_blue_action` : elles ne le lèvent jamais,
/// comme le veut le texte imprimé (« on one of **your cards** »).
///
/// Les effets empruntent `apply_action_eff`, le chemin unique des effets
/// d'action — aucun second chemin de gain. Le compteur d'audit
/// `action_mc_bonuses` est incrémenté ici, au site exact du mécanisme.
fn fire_card_action_triggers(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) {
    // Cas courant : personne ne porte ce déclencheur — on sort sans allouer.
    let any = game.players[p].played.iter().any(|&c| {
        db.projects[c as usize]
            .effect
            .map_or(false, |s| !s.action_trigger.is_empty())
    });
    if !any {
        return;
    }
    let sources = game.players[p].played.clone();
    for src in sources {
        let Some(spec) = db.projects[src as usize].effect else {
            continue;
        };
        for e in spec.action_trigger {
            if let ActionEff::Mc(n) = *e {
                game.action_mc_bonuses += n.max(0) as u64;
            }
            apply_action_eff(game, db, p, *e, policy);
        }
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
        apply_trig_gain(game, db, p, Some(src), g, 1, policy);
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
    gain_tr(game, db, p, policy);
    // (lot 3) « When you raise oxygen » du joueur agissant (Herbivores).
    fire_global_trigger(game, db, p, GlobalEvent::Oxygen, policy);
}

/// Hausse d'infrastructure (extension Grain Silos, journal B2) : par pas,
/// +1 TR et pioche 1 carte (sémantique Java `increaseInfrastructure`),
/// cap sur l'instantané de phase comme les autres paramètres.
fn raise_infrastructure(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
    if game.snap_infrastructure >= INFRASTRUCTURE_MAX {
        return;
    }
    if game.infrastructure < INFRASTRUCTURE_MAX {
        game.infrastructure += 1;
    }
    gain_tr(game, db, p, policy);
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
    gain_tr(game, db, p, policy);
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
    gain_tr(game, db, p, policy);
    // (B) déclencheurs « When you flip an ocean tile » du joueur agissant.
    fire_global_trigger(game, db, p, GlobalEvent::Ocean, policy);
}

/// **(lot 5) Gain d'UN jeton PV Forêt — le seul chemin du moteur.**
///
/// Ce que « gagner un PV Forêt » produit, une fois et une seule (livret p. 14,
/// l. 379 ; l. 391 pour l'oxygène déjà au max) :
///
/// 1. `+1` sur `PlayerState::forests` (1 PV au décompte final) ;
/// 2. **un** pas d'oxygène, via `raise_oxygen` — donc `+1 NT` et le déclencheur
///    « when you raise oxygen » (*Herbivores*), le tout plafonné sur
///    l'instantané de début de phase ;
/// 3. l'événement « **when you gain a forest VP** » (*Small Animals*).
///
/// **Le paiement n'est PAS ici.** L'action standard le fait avant d'appeler
/// cette fonction (`build_forest`), les cartes du groupe C ne paient rien
/// (`Eff::Forest`). C'est délibéré : la remise d'Ecoline porte sur « lorsque
/// vous **dépensez des plantes** pour gagner un jeton PV Forêt » — une forêt
/// offerte par une carte n'a aucune plante à remiser.
///
/// Tout gain de forêt du moteur passe par ici : il n'existe aucune autre
/// écriture de `players[p].forests` (garde-fou I2 du lot 5, journal D4).
fn gain_forest(game: &mut GameState, db: &CardsDb, p: usize, policy: &mut dyn Policy) {
    game.players[p].forests += 1;
    raise_oxygen(game, db, p, policy);
    // « When you gain a forest VP » du joueur agissant (Small Animals).
    fire_global_trigger(game, db, p, GlobalEvent::Forest, policy);
}

/// Action standard « forêt » : 8 plantes ou 20 MC, **puis** un gain de forêt par
/// le chemin unique [`gain_forest`] (livret p.14 ; Java `buildForest`).
fn build_forest(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    with_plants: bool,
    policy: &mut dyn Policy,
) {
    if with_plants {
        // (corpo-1) Coût en plantes servi par le service unique : Ecoline paie
        // 7 plantes au lieu de 8 (« you spend one less plant »).
        let cost = forest_plant_cost(db, &game.players[p]);
        assert!(game.players[p].plants >= cost);
        game.players[p].plants -= cost;
        if cost < FOREST_PLANT_COST {
            game.corp_forest_rebates += 1;
        }
    } else {
        // (corpo-1) Helion : les MC d'une forêt sont payables en chaleur.
        // (lot cartes-7) Le prix vient du service unique `standard_mc_cost` :
        // le même que celui qu'a employé `action_options` pour proposer
        // l'action (I2). La forêt payée en PLANTES, elle, n'y touche pas.
        pay_standard_mc(game, db, p, FOREST_MC_COST);
    }
    gain_forest(game, db, p, policy);
}

/// **(lot cartes-7) Paiement d'une action standard qui coûte des MC**, par le
/// service unique [`standard_mc_cost`] — donc au MÊME prix que celui auquel
/// `action_options` a jugé l'action offerte (I2).
///
/// Le compteur d'audit `standard_action_discounts` est incrémenté ICI, à
/// l'endroit exact où le joueur paie moins cher, et nulle part ailleurs.
/// Comme tout coût en MC, il passe par `top_up_mc_with_heat` : Helion peut le
/// payer en chaleur, comme partout.
fn pay_standard_mc(game: &mut GameState, db: &CardsDb, p: usize, base: i64) {
    let cost = standard_mc_cost(db, &game.players[p], base);
    if cost < base {
        game.standard_action_discounts += 1;
    }
    top_up_mc_with_heat(game, db, p, cost);
    assert!(
        game.players[p].mc >= cost,
        "action standard sans le paiement requis"
    );
    game.players[p].mc -= cost;
}

/// **Cette carte posée peut-elle être ACTIVÉE en phase III ?**
///
/// La couleur ne suffit pas. Une carte bleue peut ne porter qu'un effet
/// permanent, sans aucune action à déclencher — *United Planetary Alliance*
/// (« When you draw cards during the research phase… ») en est une, et sa fiche
/// le dit déjà : `atrig: []`, donc `action: None`.
///
/// Tant qu'on filtrait sur la seule couleur, ces cartes étaient proposées :
/// `apply_blue_action` ne faisait rien, mais l'activation était consommée « dans
/// tous les cas ». Le joueur perdait son unique activation de la manche, et la
/// future intelligence artificielle devait apprendre à éviter un coup qui n'a
/// jamais existé dans le jeu.
pub(crate) fn activable_blue(db: &CardsDb, card_id: u16) -> bool {
    let card = &db.projects[card_id as usize];
    card.color == Color::Blue && card.effect.is_some_and(|spec| spec.action.is_some())
}

fn action_options(
    game: &GameState,
    db: &CardsDb,
    p: usize,
    remaining_blue: &[u16],
    // (jokers-corpos) L'activation de l'action de corporation est-elle encore
    // disponible cette phase ? Même budget qu'une carte bleue : une fois, plus
    // les répétitions accordées par le bonus du sélectionneur.
    corp_action_left: bool,
    out: &mut Vec<ActionOpt>,
) {
    out.clear();
    let pl = &game.players[p];
    // (jokers-corpos) L'action de la planche s'offre comme celle d'une carte
    // bleue. Elle n'existe que si la corporation en porte une — donc jamais en
    // `--effects off` (`corp_effects` y rend `None`), et jamais pour les douze
    // planches de la boîte de base.
    if corp_action_left && corp_effects(db, pl).and_then(|s| s.action).is_some() {
        out.push(ActionOpt::CorpAction);
    }
    // (corpo-1) Les seuils passent par les services uniques : `spendable_mc`
    // (Helion, chaleur = MC) et `forest_plant_cost` (Ecoline, 7 plantes). Sans
    // corporation à effet, ils valent exactement `pl.mc` et `FOREST_PLANT_COST`.
    let mc = spendable_mc(db, pl);
    for &c in remaining_blue {
        out.push(ActionOpt::BlueAction(c));
    }
    if pl.plants >= forest_plant_cost(db, pl) {
        out.push(ActionOpt::ForestWithPlants);
    }
    // (lot cartes-7) Les TROIS actions standard payantes en MC sont jugées au
    // prix RÉDUIT du joueur (*Standard Technology*), par la même formule que le
    // paiement : l'affordabilité et le paiement ne peuvent pas diverger (I2).
    // La forêt en plantes et la température en chaleur ne coûtent pas de MC :
    // elles n'y touchent pas (NEVER 8).
    let remise = standard_action_discount(db, pl);
    if mc >= standard_mc_cost_with(FOREST_MC_COST, remise) {
        out.push(ActionOpt::ForestWithMc);
    }
    if pl.heat >= TEMPERATURE_HEAT_COST && game.snap_temperature < TEMPERATURE_MAX {
        out.push(ActionOpt::TemperatureWithHeat);
    }
    if mc >= standard_mc_cost_with(TEMPERATURE_MC_COST, remise)
        && game.snap_temperature < TEMPERATURE_MAX
    {
        out.push(ActionOpt::TemperatureWithMc);
    }
    if mc >= standard_mc_cost_with(OCEAN_MC_COST, remise) && game.snap_oceans < NUM_OCEANS {
        out.push(ActionOpt::OceanWithMc);
    }
    if !pl.hand.is_empty() {
        out.push(ActionOpt::SellCard);
    }
}

/// (lot 6) Lecture d'une ressource de joueur désignée par `ActionRes`.
fn action_res_get(pl: &PlayerState, res: ActionRes) -> i64 {
    match res {
        ActionRes::Heat => pl.heat,
        ActionRes::Mc => pl.mc,
        ActionRes::Plants => pl.plants,
    }
}

/// (lot 6) Écriture correspondante (`n` peut être négatif : c'est la dépense).
fn action_res_add(pl: &mut PlayerState, res: ActionRes, n: i64) {
    match res {
        ActionRes::Heat => pl.heat += n,
        ActionRes::Mc => pl.mc += n,
        ActionRes::Plants => pl.plants += n,
    }
}

/// (C) Applique UN effet d'action de carte bleue. Extrait de `apply_blue_action`
/// au lot 6 pour que les effets de l'action et ceux ajoutés par le bonus de
/// phase empruntent exactement le même code — un seul chemin par effet.
fn apply_action_eff(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    e: ActionEff,
    policy: &mut dyn Policy,
) {
    match e {
        ActionEff::Draw(n) => {
            for _ in 0..n {
                if let Some(c) = draw_card(game) {
                    game.players[p].hand.push(c);
                }
            }
        }
        ActionEff::Plants(n) => game.players[p].plants += n,
        ActionEff::Mc(n) => game.players[p].mc += n,
        // (lot 6) Ajout déclaré, non mécanique (journal D3).
        ActionEff::Heat(n) => game.players[p].heat += n,
        ActionEff::Tr(n) => {
            for _ in 0..n {
                gain_tr(game, db, p, policy);
            }
        }
        ActionEff::Oxygen(n) => {
            for _ in 0..n {
                raise_oxygen(game, db, p, policy);
            }
        }
        // (lot 6) Ajout déclaré, non mécanique : elle emprunte le chemin de
        // hausse de température existant (TR, cap sur l'instantané de phase,
        // déclencheurs « when you raise the temperature »).
        ActionEff::Temperature(n) => {
            for _ in 0..n {
                raise_temperature(game, db, p, policy);
            }
        }
        ActionEff::Reveal(r) => reveal_top(game, db, p, r, policy),
        // (lot acier-titane) Chemin océan unique du moteur : bonus de tuile,
        // +1 NT, déclencheurs « when you flip an ocean tile ».
        ActionEff::Ocean(n) => {
            for _ in 0..n {
                reveal_ocean(game, db, p, policy);
            }
        }
        // (lot acier-titane) Chemin de forêt unique du moteur (lot 5) : le
        // jeton, UN pas d'oxygène, l'événement « when you gain a forest VP ».
        ActionEff::Forest(n) => {
            for _ in 0..n {
                gain_forest(game, db, p, policy);
            }
        }
        // (decouverte-projets) « Action : … améliorer une carte Phase »
        // (les deux cartes de la famille C, nommées dans `effects.rs` et
        // nulle part ailleurs). Même chemin d'octroi que la pose —
        // `apply_phase_upgrade` — avec la source
        // ACTION, qui est ce qui alimente `phase_upgrades_by_action`. Aucune
        // action du jeu n'impose la phase : le paramètre vaut `None`.
        ActionEff::PhaseUpgrade => {
            apply_phase_upgrade(game, p, policy, None, UpgradeSource::Action)
        }
        // (decouverte-projets) « Action : Piochez deux cartes. Puis, défaussez
        // deux cartes. » Le corps de la règle est
        // celui du lot 6, appelé tel quel : il n'existe pas deux façons de
        // piocher puis défausser dans ce moteur.
        ActionEff::DrawDiscard {
            draw,
            discard,
            from_drawn,
        } => {
            apply_eff(
                game,
                db,
                p,
                Eff::DrawDiscard {
                    draw,
                    discard,
                    from_drawn,
                },
                policy,
            );
            // Compteur d'audit au site de l'activation : il compte les
            // ACTIVATIONS de l'action, quand `draw_discard_discards` compte les
            // cartes défaussées. Deux grandeurs, deux compteurs (ALWAYS 4).
            game.draw_then_discard_uses += 1;
        }
    }
}

/// (lot acier-titane) Coût en MC d'un `ActionCost`, quand il s'en paie en MC.
/// Un seul cas dépend de l'état du joueur : `McPerCapacity`, dont le montant est
/// lu au moment de l'ACTIVATION sur le compte de savoir-faire courant (I7).
/// Jamais négatif : « reduce this by … » ne rapporte pas de MC.
fn action_mc_cost(pl: &PlayerState, c: ActionCost) -> i64 {
    match c {
        ActionCost::Mc(n) => n,
        ActionCost::McPerCapacity { base, cap, per } => {
            (base - per * player_capacities(pl).get(cap)).max(0)
        }
        _ => 0,
    }
}

/// (lot acier-titane) Les effets d'une action peuvent-ils encore produire
/// quelque chose ? Aujourd'hui une seule condition : retourner une tuile océan
/// quand il n'en reste plus n'est pas une action, c'est un paiement à vide.
/// C'est déjà la règle de `Action::FlipOceanTagDiscount` (Volcanic Pools, lot 2) ;
/// elle est ici exprimée sur l'EFFET, jamais sur un nom de carte (I6).
/// Le seuil est celui du moteur : l'INSTANTANÉ de début de phase, comme partout.
///
/// Elle porte sur les effets IMPRIMÉS de l'action, pas sur ceux qu'un bonus de
/// phase ajoute (`PhaseBonus::extra`) : un bonus est un supplément, son
/// impossibilité ne doit pas annuler l'action entière. Un `ActionEff::Ocean`
/// ajouté par un bonus alors qu'il ne reste plus d'océan ne produit simplement
/// rien (`reveal_ocean` sort tout de suite). Aucune carte ne combine
/// aujourd'hui les deux ; la distinction est écrite pour Découverte.
fn action_effs_possible(game: &GameState, effect: &[ActionEff]) -> bool {
    !effect
        .iter()
        .any(|e| matches!(e, ActionEff::Ocean(_)) && game.snap_oceans >= NUM_OCEANS)
}

/// (lot 6, brique 6) **Révélation du dessus de la pioche.**
///
/// Les `n` cartes sont RÉELLEMENT retirées du dessus de la pioche par
/// `flow::draw_card` — le chemin de pioche du moteur, remélange de la défausse
/// compris : il n'y a pas de « coup d'œil » parallèle, ni de carte fixe regardée
/// à la place du vrai dessus. Parmi les révélées, celles qui satisfont le filtre
/// imprimé sont les seules candidates à entrer en main ; `Policy::research_keep`
/// (« garder k parmi n », la question exacte de la phase V) tranche, et toutes
/// les autres rejoignent la défausse en rapportant `mc_per_discarded` MC.
fn reveal_top(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    r: Reveal,
    policy: &mut dyn Policy,
) {
    let mut revealed: Vec<u16> = Vec::with_capacity(r.n as usize);
    for _ in 0..r.n {
        match draw_card(game) {
            Some(c) => revealed.push(c),
            // Pioche ET défausse vides : il n'y a plus rien à révéler.
            None => break,
        }
    }
    if revealed.is_empty() {
        return;
    }
    game.cards_revealed += revealed.len() as u64;
    let cands: Vec<u16> = revealed
        .iter()
        .copied()
        .filter(|&c| reveal_matches(&db.projects[c as usize], r.keep))
        .collect();
    let take = (r.take as usize).min(cands.len());
    let mut kept: Vec<u16> = Vec::with_capacity(take);
    // La politique est consultée même à un seul candidat : c'est le moteur qui
    // demande, jamais lui qui décide (convention du lot 3 pour les CIBLES,
    // `choose_res_target`). La règle « on ne demande rien à une seule option »
    // ne vaut que pour les ALTERNATIVES du texte imprimé (`choose_option`).
    //
    // Et elle est consultée **à chaque révélation**, y compris quand rien n'est
    // prenable (`take == 0`) : retourner trois cartes face visible est un geste
    // du jeu, pas un calcul interne. Une politique qui a des yeux (l'écran) doit
    // pouvoir les montrer ; une politique qui n'en a pas garde le corps par
    // défaut de `Policy::reveal_pick`, qui ne décide rien et ne consomme pas le
    // générateur dans ce cas. Le déroulement du jeu, lui, ne bouge pas d'un
    // point : mêmes cartes gardées, mêmes cartes défaussées, même hasard.
    policy.observe(&game, p);
    let idx = policy.reveal_pick(&mut game.rng, p, &revealed, &cands, take, r.keep);
    for &i in idx.iter().take(take) {
        if i < cands.len() {
            kept.push(cands[i]);
        }
    }
    for c in revealed {
        if kept.contains(&c) {
            game.players[p].hand.push(c);
        } else {
            game.discard.push(c);
            game.players[p].mc += r.mc_per_discarded;
        }
    }
}

/// (lot 6) Une carte révélée satisfait-elle le filtre imprimé ?
fn reveal_matches(card: &crate::cards::ProjectCard, f: RevealFilter) -> bool {
    match f {
        RevealFilter::AnyOfTags(tags) => card.tags.iter().any(|t| tags.contains(t)),
        RevealFilter::ColorIsNot(c) => card.color != c,
    }
}

/// (C) Applique l'action réelle d'une carte bleue en jeu (lot 2). Renvoie `true`
/// si un effet a réellement été appliqué (coût payé / effet produit) — seul cas
/// où le compteur `blue_actions` est incrémenté. Renvoie `false` si la carte n'a
/// pas d'action, si le coût fixe n'est pas payable, ou si une action variable
/// tire un montant nul. Les montants « up to X » sont tirés par la politique.
// (lot cartes-8) Rendue publique : c'est le point d'entrée UNIQUE d'une action
// de carte bleue, déjà emprunté par la sonde. Les tests du lot 8 doivent
// pouvoir observer un coût qui n'est PAS payable — un état que la sonde, qui
// part toujours d'un joueur à 5 de note de terraformation, ne sait pas produire.
pub fn apply_blue_action(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    card_id: u16,
    policy: &mut dyn Policy,
) -> bool {
    let Some(spec) = db.projects[card_id as usize].effect else {
        return false;
    };
    let Some(action) = spec.action else {
        return false;
    };
    apply_action_spec(
        game,
        db,
        p,
        ActionSource::Card(card_id),
        action,
        spec.phase_bonus,
        policy,
    )
}

/// **(jokers-corpos) D'où vient l'action que l'on active.** Une action de
/// CORPORATION n'a pas de carte porteuse : c'est la seule différence, et elle ne
/// concerne que les actions à ressources, qui ont besoin d'un réceptacle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSource {
    /// Action d'une carte bleue en jeu.
    Card(u16),
    /// Action portée par la planche de corporation du joueur.
    Corp,
}

/// **(jokers-corpos) Active l'action de la CORPORATION du joueur** — Hyperion
/// Systems, « Action : gagnez 1 MC ». Point d'entrée jumeau de
/// [`apply_blue_action`] : les deux délèguent au même corps, il n'existe pas
/// deux façons d'activer une action dans ce moteur.
pub fn apply_corp_action(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) -> bool {
    let Some(spec) = corp_effects(db, &game.players[p]) else {
        return false;
    };
    let Some(action) = spec.action else {
        return false;
    };
    let phase_bonus = spec.phase_bonus;
    apply_action_spec(game, db, p, ActionSource::Corp, action, phase_bonus, policy)
}

/// (jokers-corpos) **Le corps de l'activation d'une action**, extrait de
/// `apply_blue_action` pour que l'action d'une corporation emprunte exactement
/// le même code — coût, payabilité, effets, bonus de phase, déclencheurs.
fn apply_action_spec(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    src: ActionSource,
    action: Action,
    phase_bonus: Option<PhaseBonus>,
    policy: &mut dyn Policy,
) -> bool {
    // (lot 6, brique 2) Bonus « *If YOU chose the action phase this round … » :
    // il ne dépend que de la phase choisie par le joueur QUI ACTIVE l'action
    // (NEVER 8), lue à l'instant de l'activation sur l'état réel du joueur —
    // celui-là même que la phase de planification a écrit.
    // (decouverte-projets) La condition a deux moitiés, toutes deux lues sur le
    // joueur QUI ACTIVE : la phase qu'il a choisie (`b.phase`, 0 = aucune phase
    // exigée) et, pour D06, le fait que la carte Phase qu'il a RÉVÉLÉE cette
    // manche soit AMÉLIORÉE (`b.require_upgraded`). Jamais celle de
    // l'adversaire, jamais un compteur global (clause anti-shortcut n° 4).
    let bonus: Option<PhaseBonus> = phase_bonus.filter(|b| {
        let pl = &game.players[p];
        (b.phase == 0 || pl.chosen_phase == b.phase)
            && (!b.require_upgraded || pl.phase_upgrade(pl.chosen_phase).is_some())
    });
    // (lot cartes-7) Le résultat est capturé : une activation qui a réellement
    // produit un effet lève « When you use an "Action:" effect on one of your
    // cards » (*Assembly Lines*). Les arms qui sortent par `return false` —
    // coût impayable, effet impossible, montant nul — ne le lèvent pas : rien
    // n'a été « utilisé ».
    let applied = match action {
        Action::Fixed { cost, effect } => {
            // Le bonus peut REMPLACER le coût imprimé (« spend 3 plants
            // instead ») ; sinon le coût est celui de l'action.
            let cost: &[ActionCost] = bonus.and_then(|b| b.cost).unwrap_or(cost);
            // (lot acier-titane) Une action dont l'effet imprimé ne peut plus
            // rien produire ne s'applique pas — on ne paie jamais pour rien.
            if !action_effs_possible(game, effect) {
                return false;
            }
            // Payabilité (Java : *ActionValidator).
            for c in cost {
                let ok = match *c {
                    ActionCost::Heat(n) => game.players[p].heat >= n,
                    // (corpo-1) Helion : les MC d'une action bleue peuvent venir
                    // de la chaleur, comme partout ailleurs.
                    ActionCost::Mc(n) => spendable_mc(db, &game.players[p]) >= n,
                    ActionCost::Plants(n) => game.players[p].plants >= n,
                    // (lot 6) Le coût se paie en CARTES : il faut les avoir.
                    ActionCost::DiscardCard(n) => game.players[p].hand.len() >= n as usize,
                    // (lot cartes-8) Coût en note de terraformation : il faut
                    // les points, et ils ne se convertissent depuis rien.
                    ActionCost::Tr(n) => game.players[p].tr >= n as i64,
                    // (lot acier-titane) Coût en MC diminué par les savoir-faire.
                    ActionCost::McPerCapacity { .. } => {
                        spendable_mc(db, &game.players[p])
                            >= action_mc_cost(&game.players[p], *c)
                    }
                };
                if !ok {
                    return false;
                }
            }
            for c in cost {
                match *c {
                    ActionCost::Heat(n) => game.players[p].heat -= n,
                    ActionCost::Mc(n) => {
                        top_up_mc_with_heat(game, db, p, n);
                        game.players[p].mc -= n;
                    }
                    // (lot acier-titane) Même chemin de dépense que `Mc` — donc
                    // Helion peut le payer en chaleur, comme tout coût en MC.
                    ActionCost::McPerCapacity { .. } => {
                        let n = action_mc_cost(&game.players[p], *c);
                        top_up_mc_with_heat(game, db, p, n);
                        game.players[p].mc -= n;
                    }
                    ActionCost::Plants(n) => game.players[p].plants -= n,
                    // (lot cartes-8) Même retrait que `Req::SpendTr` : service
                    // unique `PlayerState::spend_tr`, qui tient le compteur
                    // d'audit du NT (invariant `tr == 5 + gains - dépenses`).
                    ActionCost::Tr(n) => game.players[p].spend_tr(n as i64),
                    // (lot 6, brique 3) Défausse-coût : QUELLES cartes est une
                    // décision du joueur, prise par la politique existante.
                    ActionCost::DiscardCard(n) => {
                        let hand = game.players[p].hand.clone();
                        policy.observe(&game, p);
                        let idx = policy.discard_down(&mut game.rng, p, &hand, n as usize);
                        let mut paid = 0u8;
                        for &i in idx.iter().take(n as usize) {
                            if i < hand.len() && discard_from_hand(game, p, hand[i]) {
                                paid += 1;
                                game.action_discard_costs += 1;
                            }
                        }
                        // Un coût à moitié prélevé pendant qu'on applique quand
                        // même l'effet serait le pire des bugs. La payabilité a
                        // été vérifiée juste avant : si la politique ne rend pas
                        // `n` indices valides et distincts, c'est un manquement
                        // à son contrat, pas un cas de jeu (même discipline que
                        // `add_resources`).
                        assert_eq!(
                            paid, n,
                            "coût en cartes partiellement payé : la politique doit rendre \
                             {n} indices de main valides et distincts"
                        );
                    }
                }
            }
            for e in effect {
                apply_action_eff(game, db, p, *e, policy);
            }
            // (lot 6) Effets AJOUTÉS par le bonus de phase, après ceux de
            // l'action — ordre du texte imprimé (« … also gain 1 plant »).
            if let Some(b) = bonus {
                for e in b.extra {
                    apply_action_eff(game, db, p, *e, policy);
                }
                // Compteur d'audit, au site EXACT du mécanisme.
                game.action_phase_bonuses += 1;
                // (jokers-corpos) MC versés par un bonus conditionné à la phase
                // que le joueur a LUI-MÊME sélectionnée (`phase != 0`). Les
                // trois cartes de base à `phase: 3` versent des plantes, de la
                // chaleur, ou remplacent un coût — jamais des MC ; D06, qui
                // verse 2 MC, porte `phase: 0` (sa condition est « carte Phase
                // améliorée », pas « phase Action »). Ce compteur est donc nul
                // en boîte de base, et c'est la propriété qui le rend utile.
                if b.phase != 0 {
                    let mc: i64 = b
                        .extra
                        .iter()
                        .map(|e| match *e {
                            ActionEff::Mc(n) => n,
                            _ => 0,
                        })
                        .sum();
                    game.action_phase_self_bonus += mc.max(0) as u64;
                }
                // (decouverte-projets) Le supplément de D06 est un gain lié à
                // une carte Phase AMÉLIORÉE révélée : il est compté comme tel,
                // au même endroit que son versement.
                if b.require_upgraded {
                    game.upgraded_reveal_bonuses += 1;
                }
            }
            true
        }
        // (lot 6, brique 4) « Spend up to N <res> to gain that amount of
        // <res> ». Le plafond imprimé rend les montants
        // ÉNUMÉRABLES : ce sont des branches (1, 2, … N), dans l'ordre du texte,
        // filtrées par ce que le joueur possède — exactement la convention du
        // lot 3 pour les alternatives. `choose_option` n'est consultée qu'à
        // partir de deux branches jouables ; à une seule, il n'y a plus de
        // choix ; à zéro, l'action ne s'applique pas.
        Action::SpendUpTo { spend, gain, cap } => {
            let have = action_res_get(&game.players[p], spend);
            let branches = have.min(cap);
            if branches <= 0 {
                return false;
            }
            let k = if branches == 1 {
                0
            } else {
                // Les options ne sont pas des branches de texte : ce sont des
                // QUANTITÉS croissantes, l'option k valant k+1 unités. Le
                // contexte le dit, pour qu'un écran propose un montant et non
                // une liste de boutons.
                let ctx = ChoiceContext::SpendAmount {
                    source: src,
                    spend,
                    gain,
                    max: branches,
                };
                policy.observe(&game, p);
                let c = policy.choose_option_ctx(&mut game.rng, p, &ctx);
                if c >= branches as usize {
                    return false; // renoncement explicite (convention lot 3)
                }
                c
            };
            let amt = k as i64 + 1;
            action_res_add(&mut game.players[p], spend, -amt);
            action_res_add(&mut game.players[p], gain, amt);
            true
        }
        // « Spend any amount of heat to gain that amount of MC. » (lot 2,
        // inchangé — carte hors périmètre, I4.)
        Action::HeatToMc => {
            let max = game.players[p].heat;
            policy.observe(&game, p);
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
            if spendable_mc(db, &game.players[p]) < cost {
                return false;
            }
            top_up_mc_with_heat(game, db, p, cost);
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
            if spendable_mc(db, &game.players[p]) < cost {
                return false;
            }
            top_up_mc_with_heat(game, db, p, cost);
            game.players[p].mc -= cost;
            raise_temperature(game, db, p, policy);
            true
        }
        // « Discard up to `cap` cards, draw that many. »
        Action::DiscardDraw(cap) => {
            let max = (game.players[p].hand.len() as i64).min(cap);
            policy.observe(&game, p);
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
            // (jokers-corpos) Une action à ressources a besoin d'une carte
            // RÉCEPTACLE : une planche de corporation n'en est pas une. Aucune
            // n'en déclare (test structurel du lot) — la branche est un
            // garde-fou, pas un cas de jeu.
            let ActionSource::Card(card_id) = src else {
                return false;
            };
            let playable: Vec<usize> = (0..branches.len())
                .filter(|&i| branch_playable(game, db, p, card_id, branches[i]))
                .collect();
            if playable.is_empty() {
                return false;
            }
            let k = if playable.len() == 1 {
                0
            } else {
                let options: Vec<BranchOption> = playable
                    .iter()
                    .map(|&i| BranchOption {
                        printed_rank: i,
                        effects: branches[i],
                    })
                    .collect();
                let ctx = ChoiceContext::ActionAlternative {
                    card: card_id,
                    branches: &options,
                };
                policy.observe(&game, p);
                let c = policy.choose_option_ctx(&mut game.rng, p, &ctx);
                if c >= playable.len() {
                    return false; // renoncement explicite (journal D4)
                }
                c
            };
            for e in branches[playable[k]] {
                apply_res_eff(game, db, p, card_id, e, policy, UpgradeSource::Action);
            }
            true
        }
    };
    if applied {
        fire_card_action_triggers(game, db, p, policy);
    }
    applied
}

// =============================================================================
// (Découverte) LE BONUS DU SÉLECTIONNEUR — POINT DE CALCUL UNIQUE
//
// Les cinq phases lisent leur bonus ICI, et nulle part ailleurs. La valeur ne
// vient jamais d'une constante écrite dans le flux de jeu : elle vient de la
// table `effects::PHASE_BASE` ou, si le joueur a amélioré cette carte Phase, de
// `effects::PHASE_UPGRADED`. **Une entrée, jamais deux** : c'est ce qui rend le
// cumul du bonus de base et du bonus amélioré impossible à écrire (NEVER 1).
// =============================================================================

/// (Découverte) Le bonus du sélectionneur d'une phase, tel que le joueur y a
/// droit : la carte Phase de base, ou son amélioration si elle est installée.
///
/// Les champs scalaires sont l'UNION des branches (un « ou » du texte imprimé
/// annonce ce qu'il peut donner ; `alternative` dit qu'il faudra choisir).
/// C'est cet objet que la sonde rend tel quel — elle ne le recalcule pas.
#[derive(Debug, Clone, Copy)]
pub struct SelectorBonus {
    /// Phase décrite (0 = aucune phase demandée).
    pub phase: u8,
    /// Le joueur a-t-il choisi cette phase ? Faux = aucun bonus.
    pub is_selector: bool,
    /// Variante installée sur cette carte Phase, `None` = carte normale.
    pub upgraded: Option<PhaseUpgrade>,
    /// La carte Phase lue (nom imprimé, branches).
    pub spec: &'static SelectorSpec,
    pub mc_discount: i64,
    pub mc: i64,
    pub draw: u8,
    pub extra_activations: u8,
    /// Nombre de poses supplémentaires que le bonus peut accorder.
    pub extra_builds: u8,
    pub research_draw: usize,
    pub research_keep: usize,
    /// Le bonus est un « ou » : une seule de ses branches sera appliquée.
    pub alternative: bool,
}

impl SelectorBonus {
    /// Bonus vide (joueur non sélectionneur, ou phase 0).
    fn none(phase: u8) -> SelectorBonus {
        SelectorBonus {
            phase,
            is_selector: false,
            upgraded: None,
            spec: &effects::SELECTOR_SPEC_NONE,
            mc_discount: 0,
            mc: 0,
            draw: 0,
            extra_activations: 0,
            extra_builds: 0,
            research_draw: 0,
            research_keep: 0,
            alternative: false,
        }
    }
}

/// (Découverte) **Le point de calcul unique.** Bonus du sélectionneur de la
/// phase `phase` (1..=5) pour le joueur `pl`.
///
/// - `phase == 0` ou joueur qui n'a pas choisi cette phase → tout à zéro ;
/// - `--effects off` → les améliorations installées sont ignorées, les cinq
///   bonus retombent bit à bit sur la carte Phase de base (ALWAYS 2) ;
/// - sinon → la carte Phase du joueur pour cette phase, améliorée ou non.
///
/// Fonction PURE : elle ne touche aucun compteur (c'est
/// `selector_bonus_applied` qui compte, et le flux de jeu seul l'appelle).
pub fn selector_bonus(db: &CardsDb, pl: &PlayerState, phase: u8) -> SelectorBonus {
    if !(1..=5).contains(&phase) || pl.chosen_phase != phase {
        return SelectorBonus::none(phase);
    }
    // La couche d'effets coupée : le joueur garde ses cartes améliorées en
    // main, mais aucun de leurs bonus ne s'applique — comme tout effet de
    // carte (I7).
    let upgraded = if db.effects_on {
        pl.phase_upgrade(phase)
    } else {
        None
    };
    let spec: &'static SelectorSpec = match upgraded {
        Some(v) => &effects::PHASE_UPGRADED[phase as usize - 1][v.index()],
        None => &effects::PHASE_BASE[phase as usize - 1],
    };
    let mut b = SelectorBonus::none(phase);
    b.is_selector = true;
    b.upgraded = upgraded;
    b.spec = spec;
    b.alternative = spec.branches.len() > 1;
    for g in spec.branches {
        b.mc_discount = b.mc_discount.max(g.mc_discount);
        b.mc = b.mc.max(g.mc);
        b.draw = b.draw.max(g.draw);
        b.extra_activations = b.extra_activations.max(g.extra_activations);
        b.extra_builds = b.extra_builds.max(g.builds.len() as u8);
        b.research_draw = b.research_draw.max(g.research_draw);
        b.research_keep = b.research_keep.max(g.research_keep);
    }
    b
}

/// (Découverte) Le bonus du sélectionneur tel que la PHASE RÉELLE le lit, plus
/// le comptage du remplacement. Appelé une fois par phase et par joueur, par le
/// flux de jeu **et par lui seul** : la sonde passe par `selector_bonus`, qui
/// ne compte rien.
fn selector_bonus_applied(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    phase: u8,
) -> SelectorBonus {
    let b = selector_bonus(db, &game.players[p], phase);
    if b.upgraded.is_some() {
        game.upgraded_bonus_applied += 1;
    }
    b
}

/// (Découverte) La branche du bonus à appliquer. Une seule branche = rien à
/// demander ; plusieurs = un « ou » du texte imprimé, tranché par `Policy`
/// (NEVER 4) — jamais par le moteur.
fn selector_branch(
    game: &mut GameState,
    b: &SelectorBonus,
    p: usize,
    policy: &mut dyn Policy,
) -> &'static SelectorGrant {
    let branches = b.spec.branches;
    if branches.len() < 2 {
        return &branches[0];
    }
    let ctx = ChoiceContext::SelectorBonus {
        phase: b.phase,
        variant: b.upgraded,
        card_name: b.spec.name,
        branches,
    };
    policy.observe(&game, p);
    let i = policy.choose_option_ctx(&mut game.rng, p, &ctx);
    &branches[i.min(branches.len() - 1)]
}

/// (Découverte) Verse dans la file du lot cartes-8 les permissions de pose
/// accordées par une carte Phase améliorée. **Pas de seconde file, pas de
/// second drainage** (NEVER 2) : ce sont des permissions comme celles des
/// cartes, elles sont exercées par `drain_pending_builds`.
fn grant_selector_builds(game: &mut GameState, p: usize, g: &SelectorGrant) {
    for grant in g.builds {
        game.players[p].pending_builds.push(*grant);
        game.extra_builds_granted += 1;
        game.upgraded_extra_builds += 1;
    }
}

/// Phase I — Développement (livret p.11) : chacun peut jouer 1 carte verte ;
/// sélectionneur : la remise de sa carte Phase (base -3 MC, I-A -6 MC, I-B -3 MC
/// plus une seconde verte). Un passage chacun, dans l'ordre du tour (C4).
fn phase_development(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    for p in game.players_in_turn_order() {
        let bonus = selector_bonus_applied(game, db, p, 1);
        let g = selector_branch(game, &bonus, p, policy);
        let discount = g.mc_discount;
        // (jokers-corpos) Les badges jokers de la main reçoivent leur jeton AVANT
        // l'énumération : `affordable` juge alors chaque carte joker sur son
        // badge réel, exactement comme le paiement le fera (I2).
        resolve_hand_jokers(game, db, p, policy);
        let opts = affordable(game, db, p, &GRANT_DEVELOPMENT, discount);
        policy.observe(&game, p);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card_granted(game, db, p, idx, discount, &GRANT_DEVELOPMENT, policy);
        }
        // (Découverte, I-B) « Vous pouvez jouer une SECONDE carte verte » : la
        // permission est versée APRÈS la pose ordinaire — c'est bien une carte
        // de plus, et le texte réserve la remise à « la première carte ».
        grant_selector_builds(game, p, g);
        // (lot cartes-8) *Automated Factories* et *Tall Station* offrent ici une
        // carte verte à 9 MC ou moins. La permission ne peut naître que de la
        // pose qui précède, d'où le drainage APRÈS elle.
        //
        // Remise nulle : « le coût de LA CARTE que vous jouez lors de cette
        // phase » vise la pose ordinaire, pas les poses supplémentaires (le
        // drainage forçait déjà 0 pour les permissions offertes, les seules que
        // la boîte de base produise en phase I : l'empreinte ne bouge pas).
        drain_pending_builds(game, db, p, 0, policy);
    }
}

/// Phase II — Construction (livret p.12) : chacun peut jouer 1 carte
/// bleue/rouge ; sélectionneur : piocher 1 carte AVANT ou APRÈS avoir joué
/// (C2), OU en jouer une 2e. Un passage chacun, dans l'ordre du tour (C4).
fn phase_construction(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    for p in game.players_in_turn_order() {
        let sb = selector_bonus_applied(game, db, p, 2);
        // La carte Phase II de BASE est un « ou » à trois issues (pioche avant,
        // pioche après, seconde pose) : c'est `Policy::construction_bonus` qui
        // en choisit la branche ET le moment, depuis le lot 3 (C2). Les cartes
        // AMÉLIORÉES ont leur propre forme : II-A donne les deux à la fois,
        // II-B est un « ou » à deux branches tranché par `Policy::choose_option`.
        let bonus = if sb.is_selector && sb.upgraded.is_none() {
            policy.observe(&game, p);
            Some(policy.construction_bonus(&mut game.rng, p))
        } else {
            None
        };
        // Bonus AMÉLIORÉ : la branche est arrêtée avant la pose, comme le
        // « ou » de la carte de base.
        let upgraded: Option<&'static SelectorGrant> = if sb.upgraded.is_some() {
            Some(selector_branch(game, &sb, p, policy))
        } else {
            None
        };

        // (C2) Pioche AVANT : la carte piochée entre en main avant le calcul
        // d'affordabilité, elle peut donc être posée dans la foulée. Le bonus
        // de base tire son nombre de cartes de la table, comme le bonus
        // amélioré : une seule donnée pour les deux (NEVER 3).
        let draw_before = match (bonus, upgraded) {
            (Some(ConstructionBonus::DrawCardBefore), _) => sb.spec.branches[0].draw,
            // (II-A) « Piochez une carte. » Le texte imprimé ne donne pas de
            // moment : la pioche précède la pose, comme sur la carte de base
            // quand le joueur choisit « avant » — c'est le moment qui laisse le
            // plus de jeu, et il est déjà outillé (compteur `draw_before_build`).
            (_, Some(g)) => g.draw,
            _ => 0,
        };
        for _ in 0..draw_before {
            if let Some(c) = draw_card(game) {
                game.players[p].hand.push(c);
                game.draw_before_build += 1;
            }
        }
        // (II-B, branche « OU gagnez 6 MC ») : gain immédiat du sélectionneur.
        if let Some(g) = upgraded {
            game.players[p].mc += g.mc;
        }

        // (jokers-corpos) Les badges jokers de la main reçoivent leur jeton AVANT
        // l'énumération : `affordable` juge alors chaque carte joker sur son
        // badge réel, exactement comme le paiement le fera (I2).
        resolve_hand_jokers(game, db, p, policy);
        let opts = affordable(game, db, p, &GRANT_CONSTRUCTION, 0);
        policy.observe(&game, p);
        if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
            assert!(opts.contains(&idx), "choix de construction hors options");
            build_card_granted(game, db, p, idx, 0, &GRANT_CONSTRUCTION, policy);
        }
        // (Découverte, II-A et II-B) « Vous pouvez jouer une seconde carte bleue
        // ou rouge lors de cette phase » : une permission comme les autres,
        // versée dans la file du lot cartes-8 et exercée par le drainage
        // ci-dessous — pas un second mécanisme de pose (NEVER 2).
        if let Some(g) = upgraded {
            grant_selector_builds(game, p, g);
        }
        // (lot cartes-8) *Asset Liquidation*, *Special Design* et *Work Crews*
        // ouvrent une pose bleue/rouge de plus. Drainé AVANT le bonus du
        // sélectionneur : ce sont deux droits distincts, et celui-ci naît de la
        // carte qu'on vient de poser.
        drain_pending_builds(game, db, p, 0, policy);

        match bonus {
            // (C2) Pioche APRÈS la pose — même donnée que la pioche « avant » :
            // la branche « piochez une carte » de la carte Phase II de base.
            Some(ConstructionBonus::DrawCard) => {
                for _ in 0..sb.spec.branches[0].draw {
                    if let Some(c) = draw_card(game) {
                        game.players[p].hand.push(c);
                        game.draw_after_build += 1;
                    }
                }
            }
            Some(ConstructionBonus::SecondBuild) => {
                // La permission de la SECONDE branche de la carte Phase II de
                // base — lue dans la table, pas écrite ici (NEVER 3).
                let grant = &sb.spec.branches[1].builds[0];
                // (jokers-corpos) Les badges jokers de la main reçoivent leur jeton AVANT
                // l'énumération : `affordable` juge alors chaque carte joker sur son
                // badge réel, exactement comme le paiement le fera (I2).
                resolve_hand_jokers(game, db, p, policy);
                let opts = affordable(game, db, p, grant, 0);
                policy.observe(&game, p);
                if let Some(idx) = policy.choose_build(&mut game.rng, p, &opts) {
                    assert!(opts.contains(&idx), "choix de construction hors options");
                    build_card_granted(game, db, p, idx, 0, grant, policy);
                }
                // (lot cartes-8) La 2e pose du sélectionneur peut elle aussi
                // poser une carte qui en accorde une 3e.
                drain_pending_builds(game, db, p, 0, policy);
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
    let mut passed = [false; NUM_PLAYERS];
    // (jokers-corpos) L'action de la planche de corporation, une fois par phase
    // comme celle d'une carte bleue. Le drapeau vaut pour tous les joueurs :
    // ceux dont la planche ne porte pas d'action ne se verront simplement jamais
    // proposer l'option (`action_options` lit la table).
    let mut corp_action_left = [true; NUM_PLAYERS];
    for p in 0..NUM_PLAYERS {
        remaining_blue[p] = game.players[p]
            .played
            .iter()
            .copied()
            .filter(|&c| activable_blue(db, c))
            .collect();
    }

    // (Découverte) Bonus du sélectionneur de la phase III, lu au DÉBUT de la
    // phase — donc après les phases I et II de la même manche : une amélioration
    // gagnée en phase I vaut dès cette manche-ci (livret l. 64, ASK 1).
    // Parcours dans l'ordre du tour : la révélation de III-A pioche, et l'ordre
    // de pioche est celui du tour.
    for p in order {
        let sb = selector_bonus_applied(game, db, p, 3);
        let g = selector_branch(game, &sb, p, policy);
        // Activations supplémentaires : la valeur vient de la table (base +1,
        // III-A +1, III-B +2) et c'est l'ÉTAT DU JOUEUR qui la porte pendant
        // toute la phase — la boucle d'actions ci-dessous la lit et la
        // décrémente à chaque répétition accordée. Il n'existe pas de second
        // budget à côté du champ.
        game.players[p].extra_blue_activations = g.extra_activations;
        // (III-A) « Révélez les 3 premières cartes de la pioche. Ajoutez à votre
        // main une carte bleue ou rouge ainsi révélée. Défaussez les autres. »
        // Le texte imprimé ne donne aucun moment (ASK 3) : la révélation a lieu
        // AU DÉBUT de la phase, avant la première action — la carte gagnée fait
        // alors partie de la main pendant toute la phase (elle peut être vendue
        // par l'action standard, et elle compte à la limite de main de fin de
        // manche). Le chemin est celui du lot 6 (`reveal_top`), pas un second.
        if let Some(r) = g.reveal {
            reveal_top(game, db, p, r, policy);
        }
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
            action_options(
                game,
                db,
                p,
                &remaining_blue[p],
                corp_action_left[p],
                &mut options,
            );
            policy.observe(&game, p);
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
                    // Bonus du sélectionneur : une activation supplémentaire,
                    // prise sur le budget que porte l'état du joueur.
                    if game.players[p].extra_blue_activations > 0 {
                        game.players[p].extra_blue_activations -= 1;
                        remaining_blue[p].push(card);
                    }
                }
                // (jokers-corpos) L'action de la planche : même comptabilité
                // qu'une carte bleue — l'activation est consommée dans tous les
                // cas, le compteur `blue_actions` ne monte que si un effet a
                // réellement eu lieu, et le bonus du sélectionneur peut la
                // rendre une fois de plus.
                ActionOpt::CorpAction => {
                    if db.effects_on && apply_corp_action(game, db, p, policy) {
                        game.blue_actions += 1;
                    }
                    corp_action_left[p] = false;
                    if game.players[p].extra_blue_activations > 0 {
                        game.players[p].extra_blue_activations -= 1;
                        corp_action_left[p] = true;
                    }
                }
                ActionOpt::ForestWithPlants => build_forest(game, db, p, true, policy),
                ActionOpt::ForestWithMc => build_forest(game, db, p, false, policy),
                ActionOpt::TemperatureWithHeat => {
                    game.players[p].heat -= TEMPERATURE_HEAT_COST;
                    raise_temperature(game, db, p, policy);
                }
                ActionOpt::TemperatureWithMc => {
                    // (lot cartes-7) Prix par le service unique ; (corpo-1)
                    // Helion : chaleur convertie en MC si nécessaire.
                    pay_standard_mc(game, db, p, TEMPERATURE_MC_COST);
                    raise_temperature(game, db, p, policy);
                }
                ActionOpt::OceanWithMc => {
                    pay_standard_mc(game, db, p, OCEAN_MC_COST);
                    reveal_ocean(game, db, p, policy);
                }
                ActionOpt::SellCard => {
                    // La carte vendue est CHOISIE par la politique — le moteur
                    // ne la tire plus lui-même au hasard. Le corps par défaut de
                    // `sell_card` reproduit l'ancien tirage à l'identique.
                    let main = game.players[p].hand.clone();
                    let n = main.len();
                    let i = policy.sell_card(&mut game.rng, p, &main).min(n - 1);
                    let card = game.players[p].hand.remove(i);
                    game.discard.push(card);
                    // (lot cartes-7) « Cards you discard for MC » : la vente de
                    // carte EST une défausse pour du MC, et elle ne coûte rien —
                    // la réduction de *Standard Technology* ne s'y applique
                    // jamais (NEVER 8), le taux de *Composting Factory* si.
                    game.players[p].mc += discard_mc_rate(db, &game.players[p]);
                    game.discard_bonus_mc += discard_bonus_per_card(db, &game.players[p]);
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
        // (corpo-1) Le seuil de conversion obligatoire est le coût RÉEL d'une
        // forêt pour ce joueur (Ecoline : 7 plantes) — même service que l'action
        // volontaire, sinon l'obligation et l'option divergeraient.
        while game.players[p].plants >= forest_plant_cost(db, &game.players[p])
            && game.snap_oxygen < OXYGEN_MAX
        {
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
pub(crate) fn phase_production(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    for p in 0..NUM_PLAYERS {
        let sb = selector_bonus_applied(game, db, p, 4);
        let g = selector_branch(game, &sb, p, policy);
        let bonus = g.mc;
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
        // (Découverte, IV-A) « Activez l'effet de production de l'une de vos
        // cartes vertes une fois de plus lors de cette phase. » APRÈS la
        // production ordinaire : c'est une production DE PLUS, pas une autre.
        if g.replay_green_prod {
            replay_green_production(game, db, p, policy);
        }
    }
}

/// (Découverte, IV-A) **Rejoue la production d'UNE carte verte du joueur**,
/// choisie par la politique (NEVER 4 : le moteur ne choisit pas).
///
/// Le texte imprimé ne distingue pas les productions FIXES (« +2 de production
/// de chaleur », inscrite sur les pistes à la pose) des productions DÉRIVÉES
/// (« 1 MC par badge Terre », recalculée à chaque phase IV) : les deux sont
/// « l'effet de production » de la carte, les deux sont donc candidates
/// (ASK 4). Une carte verte SANS production ne l'est pas — il n'y aurait rien à
/// rejouer.
///
/// Ce que la carte produit est lu par le service unique `card_production` ;
/// les compteurs `derived_*` ne bougent pas : ils mesurent la passe de
/// production ORDINAIRE (c'est cette variation-là que `--probe-produce`
/// rapporte), pas le bonus d'une carte Phase.
fn replay_green_production(
    game: &mut GameState,
    db: &CardsDb,
    p: usize,
    policy: &mut dyn Policy,
) {
    let cands: Vec<u16> = game.players[p]
        .played
        .iter()
        .copied()
        .filter(|&c| {
            db.projects[c as usize].color == Color::Green
                && card_production(db, &game.players[p], c) != (0, 0, 0, 0)
        })
        .collect();
    if cands.is_empty() {
        return;
    }
    // Une seule carte candidate : rien à demander (convention du lot 3 pour les
    // alternatives — on ne consulte la politique qu'à partir de deux options).
    let i = if cands.len() == 1 {
        0
    } else {
        // Chaque option dit CE QU'ELLE RAPPORTE, lu par le service unique
        // `card_production` — celui-là même qui versera le rejeu deux lignes
        // plus bas.
        let options: Vec<ProductionOption> = cands
            .iter()
            .map(|&c| {
                let (mc, heat, plants, cards) = card_production(db, &game.players[p], c);
                ProductionOption {
                    card: c,
                    mc,
                    heat,
                    plants,
                    cards,
                }
            })
            .collect();
        let ctx = ChoiceContext::ReplayProduction {
            candidates: &options,
        };
        policy.observe(&game, p);
        policy.choose_option_ctx(&mut game.rng, p, &ctx)
    };
    if i >= cands.len() {
        return;
    }
    let (mc, heat, plants, cards) = card_production(db, &game.players[p], cands[i]);
    let pl = &mut game.players[p];
    pl.mc += mc;
    pl.heat += heat;
    pl.plants += plants;
    for _ in 0..cards {
        if let Some(c) = draw_card(game) {
            game.players[p].hand.push(c);
        }
    }
}

/// (Découverte, IV-A) **Ce qu'UNE carte en jeu produit lors d'une phase IV** :
/// `(MC, chaleur, plantes, cartes)`.
///
/// Somme de sa production FIXE imprimée (les `Eff::*Prod` de son encodage, ceux
/// mêmes qui ont haussé les pistes du joueur à la pose) et de sa production
/// DÉRIVÉE (`card_derived_production`, recalculée sur les badges du moment).
/// Service unique : `derived_production` en est l'agrégat, et le bonus de la
/// carte Phase IV-A le seul autre lecteur. `(0,0,0,0)` si les effets sont
/// coupés.
///
/// Les deux seuls endroits où l'encodage d'une carte porte une production sont
/// lus : `CardEffects::effects` (production fixe, celle qui hausse les pistes à
/// la pose) et `CardEffects::prod` (production dérivée). Aucune carte n'exprime
/// de production ailleurs — vérifié : aucun `ResEff::Gain(Eff::*Prod)` dans la
/// table.
pub fn card_production(db: &CardsDb, pl: &PlayerState, card_id: u16) -> (i64, i64, i64, i64) {
    if !db.effects_on {
        return (0, 0, 0, 0);
    }
    let (mut mc, mut heat, mut plants, mut cards) = (0i64, 0i64, 0i64, 0i64);
    if let Some(spec) = db.projects[card_id as usize].effect {
        for e in spec.effects {
            match *e {
                Eff::McProd(n) => mc += n,
                Eff::HeatProd(n) => heat += n,
                Eff::PlantProd(n) => plants += n,
                Eff::CardProd(n) => cards += n,
                _ => {}
            }
        }
    }
    let (d_mc, d_heat, d_plants) = card_derived_production(db, pl, card_id);
    (mc + d_mc, heat + d_heat, plants + d_plants, cards)
}

/// Phase V — Recherche (livret p.15) : 2 piochées / 1 gardée ;
/// sélectionneur : 5 piochées / 2 gardées.
fn phase_research(game: &mut GameState, db: &CardsDb, policy: &mut dyn Policy) {
    let mut drawn = Vec::with_capacity(8);
    // Un passage chacun, dans l'ordre du tour (C4).
    for p in game.players_in_turn_order() {
        // Le bonus du sélectionneur de la phase V passe par le point de calcul
        // unique (`research_base` le lit) ; l'appel ci-dessous ne sert qu'au
        // comptage du remplacement, une fois par joueur et par phase.
        selector_bonus_applied(game, db, p, 5);
        let (base_n, _) = research_base(db, &game.players[p]);
        // (lot 4) Base du livret + bonus PERMANENT des cartes en jeu, cumulés
        // par le service unique (2/1 → 3/2 ; sélectionneur 5/2 → 6/3).
        let (n, keep) = research_draw_keep(db, &game.players[p]);
        draw_n(game, n, &mut drawn);
        // Cartes RÉELLEMENT piochées en plus grâce au bonus permanent (une
        // pioche épuisée en donnerait moins) — relevé au site de pioche.
        game.research_extra_draws += drawn.len().saturating_sub(base_n) as u64;
        let keep = keep.min(drawn.len());
        policy.observe(&game, p);
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
        // (28-07) Corrigé 7 → 6. La tuile imprimée dit « 6 badges espace »
        // (`data/cartes-imprimees/objectifs-recompenses/objectifs-recompenses.json`,
        // lue à la photo le 27-07). Le 7 venait du squelette et ne correspondait
        // à aucune source ; les dix autres seuils concordent avec les tuiles.
        MilestoneKind::SpaceBaron => 6,
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

/// **La réserve de tuiles Récompense réellement disponibles** pour la
/// configuration de boîtes courante (« Mélangez les tuiles Récompense, révélez-en
/// 3 », Discovery p.2).
///
/// VISIONNAIRE (« le plus de cartes Phase améliorées ») n'entre dans la réserve
/// que là où le mécanisme des cartes Phase améliorées peut jouer : la boîte
/// Découverte l'apporte, la couche d'effets le fait vivre. Sans l'une ou sans
/// l'autre, aucune carte du jeu ne peut améliorer une carte Phase — la tuile
/// serait une ÉGALITÉ À ZÉRO dans toutes les parties, distribuant 4 PV à chacun
/// sans rien départager. C'est exactement le défaut que COLLECTIONNEUR a traîné
/// jusqu'au 28-07, et la raison pour laquelle `--effects off` doit rester neutre
/// jusque dans les compteurs (ALWAYS 2).
pub fn award_pool(db: &CardsDb) -> Vec<AwardKind> {
    let visionnaire_jouable = db.effects_on && db.boites.contains(Boite::Decouverte);
    AWARD_POOL
        .iter()
        .copied()
        .filter(|&a| a != AwardKind::Visionary || visionnaire_jouable)
        .collect()
}

fn award_value(kind: AwardKind, pl: &PlayerState) -> i64 {
    match kind {
        AwardKind::Celebrity => pl.mc_prod,
        // (28-07) Réparée. « Le plus de ressources sur les cartes » (tuile
        // imprimée). Elle renvoyait 0 pour tout le monde depuis la création du
        // squelette, alors que les ressources posées sur les cartes existent
        // depuis le lot 3 : la récompense était morte, et comptait une égalité
        // à zéro dans toutes les parties où elle sortait.
        //
        // Somme de TOUTES les ressources du joueur, tous types confondus —
        // microbes, animaux, science, flottantes. La tuile ne distingue pas.
        AwardKind::Collector => pl.card_resources.values().map(|&n| n as i64).sum(),
        AwardKind::Generator => pl.heat_prod,
        AwardKind::Industrialist => pl.steel_capacity + pl.titanium_capacity,
        AwardKind::ProjectManager => pl.played.len() as i64,
        AwardKind::Researcher => pl.tag_counts[Tag::Science.index().unwrap()] as i64,
        // (Découverte) « Le plus de cartes Phase améliorées » — les cartes que
        // le joueur possède, pas celles qu'il a jouées ce tour.
        AwardKind::Visionary => pl.phase_upgrades_count(),
    }
}

/// Points d'awards par joueur : 1er = 5 VP, 2e = 2 VP ; égalité au 1er rang :
/// 4 VP chacun et pas de 2e (Discovery p.3). À 2 joueurs, pas d'égalité
/// possible au 2e rang.
/// Points d'awards par joueur, ET la part venant de la seule tuile VISIONNAIRE
/// (les deux joueurs cumulés). Même parcours, même barème : le compteur de
/// bilan `visionary_award_points` ne recalcule rien — il lit la part que ce
/// parcours-ci a réellement distribuée.
pub fn award_points_split(game: &GameState) -> ([i64; NUM_PLAYERS], i64) {
    let mut pts = [0i64; NUM_PLAYERS];
    let mut visionary = 0i64;
    for &award in &game.awards {
        let v0 = award_value(award, &game.players[0]);
        let v1 = award_value(award, &game.players[1]);
        let (a, b) = if v0 == v1 {
            (4, 4)
        } else if v0 > v1 {
            (5, 2)
        } else {
            (2, 5)
        };
        pts[0] += a;
        pts[1] += b;
        if award == AwardKind::Visionary {
            visionary += a + b;
        }
    }
    (pts, visionary)
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

/// Points d'awards par joueur, sans le détail — façade historique.
pub fn award_points(game: &GameState) -> [i64; NUM_PLAYERS] {
    award_points_split(game).0
}

/// **Le score d'un joueur, part par part.** Les cinq termes du décompte du
/// livret (p.16-17 + Discovery p.3), tels que [`score_breakdown`] vient de les
/// additionner — jamais recomptés ailleurs.
///
/// Un total n'explique rien : « 17 » en début de partie a surpris le joueur
/// alors que le chiffre était juste. Ce sont les mêmes additions qu'avant,
/// simplement gardées séparées le temps d'être rendues.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScoreBreakdown {
    /// Niveau de terraformation.
    pub tr: i64,
    /// 1 VP par forêt.
    pub forests: i64,
    /// VP des cartes jouées (fixes + dynamiques), effets ON uniquement.
    pub cards: i64,
    /// 3 VP par Repère (« milestone ») atteint.
    pub milestones: i64,
    /// Récompenses (« awards »), comptées comme si la partie s'arrêtait ici.
    pub awards: i64,
}

impl ScoreBreakdown {
    /// La somme des parts — c'est-à-dire le score, à l'entier près.
    pub fn total(&self) -> i64 {
        self.tr + self.forests + self.cards + self.milestones + self.awards
    }
}

/// Score final + deux compteurs d'audit tirés du MÊME parcours : les points de
/// victoire venant des RESSOURCES posées sur les cartes (`vp_from_resources`)
/// et ceux distribués par la tuile VISIONNAIRE (`visionary_award_points`),
/// tous joueurs confondus.
///
/// Les trois sortent du même passage et des mêmes calculs (`card_points`,
/// `award_points_split`) : les valeurs rapportées sont celles qui comptent
/// réellement au score, jamais un second parcours ni un barème parallèle.
///
/// Façade : le décompte lui-même vit dans [`score_breakdown`], qui garde les
/// parts séparées. Ici on ne fait que les additionner — il n'existe donc
/// toujours qu'UN point de calcul du score.
pub fn score_parts(game: &GameState, db: &CardsDb) -> ([i64; NUM_PLAYERS], i64, i64) {
    let (parts, vp_from_resources, visionary) = score_breakdown(game, db);
    let mut out = [0i64; NUM_PLAYERS];
    for p in 0..NUM_PLAYERS {
        out[p] = parts[p].total();
    }
    (out, vp_from_resources, visionary)
}

/// **Le point de calcul unique du score**, rendu part par part.
///
/// Rigoureusement les additions d'avant, dans le même ordre et sur les mêmes
/// entiers : seul le rangement change (cinq accumulateurs au lieu d'un). Le
/// total, `ScoreBreakdown::total`, est donc identique au bit près — ce que les
/// trois empreintes de référence vérifient.
pub fn score_breakdown(
    game: &GameState,
    db: &CardsDb,
) -> ([ScoreBreakdown; NUM_PLAYERS], i64, i64) {
    let (awards, visionary) = award_points_split(game);
    let mut out = [ScoreBreakdown::default(); NUM_PLAYERS];
    let mut vp_from_resources = 0i64;
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        let mut s = ScoreBreakdown {
            tr: pl.tr,
            forests: pl.forests,
            ..ScoreBreakdown::default()
        };
        if db.effects_on {
            for &c in &pl.played {
                let (total, from_res) = card_points(db, pl, c);
                s.cards += total;
                vp_from_resources += from_res;
            }
        }
        for slot in &game.milestones {
            if slot.achieved_by[p] {
                s.milestones += 3;
            }
        }
        s.awards = awards[p];
        out[p] = s;
    }
    (out, vp_from_resources, visionary)
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
        policy.observe(&game, p);
        let phase = policy.pick_phase(&mut game.rng, p, &allowed);
        assert!(
            allowed.contains(&phase),
            "phase {} interdite (précédente : {:?})",
            phase,
            game.players[p].previous_phase
        );
        game.players[p].chosen_phase = phase;
        game.players[p].previous_phase = Some(phase);
        // Le bonus du sélectionneur de la phase action n'est plus écrit ici :
        // il est relevé AU DÉBUT de la phase III depuis le point de calcul
        // unique. Une amélioration gagnée en phase I ou II vaut dès cette
        // manche-ci (livret l. 64, ASK 1) — ce qu'une valeur figée à la
        // planification aurait rendu impossible.
        game.players[p].extra_blue_activations = 0;
        // (decouverte-projets) La carte Phase vient d'être RÉVÉLÉE par ce
        // joueur : si elle est améliorée, les cartes en jeu qui portent
        // « Effet : lorsque vous révélez une carte Phase améliorée, … »
        // versent leur gain, ici et pour ce
        // joueur seul.
        fire_upgraded_reveal(game, db, p, policy);
        picked[phase as usize] = true;
    }

    // B. Exécution : seules les phases choisies, dans l'ordre I..V.
    for phase in 1u8..=5 {
        if !picked[phase as usize] {
            continue;
        }
        game.snapshot_planet();
        // (corpo-1) Début de phase : « The FIRST TIME your TR is raised EACH
        // PHASE » (Unmi). Le drapeau se remet à zéro ici, à côté de l'instantané
        // planétaire — c'est le seul marqueur de début de phase du moteur.
        for pl in game.players.iter_mut() {
            pl.tr_raised_this_phase = false;
            // (lot cartes-8) « …this phase » ne franchit jamais une frontière de
            // phase : une permission non exercée et un modificateur non
            // consommé meurent ici, même si le joueur n'a rien pu en faire.
            pl.pending_builds.clear();
            pl.next_card_mod = Default::default();
        }
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
            policy.observe(&game, p);
            let mut idx = policy.discard_down(&mut game.rng, p, &hand_snapshot, over);
            assert_eq!(idx.len(), over, "défausse de fin de ronde: mauvais nombre");
            idx.sort_unstable();
            idx.dedup();
            assert_eq!(idx.len(), over, "défausse de fin de ronde: doublons");
            // (lot cartes-7, journal D1) « Pour chaque carte ainsi défaussée,
            // le joueur gagne 3 MC, **comme toujours** » (livret l. 437 et
            // 654) : c'est bien une défausse pour du MC, donc le taux du
            // service unique — contrairement à la lecture proposée par le
            // contrat, que le livret et le code contredisent tous deux.
            let rate = discard_mc_rate(db, &game.players[p]);
            let bonus = discard_bonus_per_card(db, &game.players[p]);
            for &i in idx.iter().rev() {
                let card = game.players[p].hand.remove(i);
                game.discard.push(card);
                game.players[p].mc += rate;
                game.discard_bonus_mc += bonus;
            }
        }
    }

    // (C4, règle maison) La manche est allée à son terme : le premier joueur
    // alterne pour la suivante.
    game.first_player = (game.first_player + 1) % NUM_PLAYERS;
    game.generation += 1;
}

/// (lot acier-titane) Tests des cas limites des effets d'action nouveaux —
/// dans un fichier à part (`src/flow_tests.rs`) parce qu'ils nomment des cartes,
/// ce que le flux de jeu lui-même s'interdit (I6).
#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests_acier_titane;

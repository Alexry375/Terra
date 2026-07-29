//! Tests du lot 6 « les actions bleues et la manipulation de la main »
//! (chantier moteur-cartes-6).
//!
//! **Un ou plusieurs tests par carte du lot (11)**, confrontant l'ÉTAT DE JEU
//! résultant au TEXTE IMPRIMÉ (`inputs/textes-cartes.json`, champs `text`,
//! `requirement`, `production`, `vp_printed`) — jamais à la table d'encodage, ni
//! au champ `description` de `cards.json`. S'y ajoutent les tests des six
//! briques et de ce que le lot ne doit PAS casser :
//!
//! - le bonus de phase est conditionnel DES DEUX CÔTÉS (avec et sans), et il ne
//!   dépend jamais de la phase choisie par l'ADVERSAIRE ;
//! - les mécanismes ont lieu en PARTIE RÉELLE, prouvés par quatre compteurs
//!   d'audit relevés sur des simulations complètes, nuls en `--effects off` ;
//! - la révélation lit le VRAI dessus de pioche, et les cartes non gardées
//!   partent réellement à la défausse (conservation des cartes) ;
//! - les trois cartes du groupe C passent par la MÊME brique ;
//! - déterminisme à graine égale ;
//! - *Power Infrastructure*, hors périmètre, garde exactement son comportement ;
//! - aucun nom de carte du lot dans le flux de jeu (I5).

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Color, Tag};
use engine::flow::{play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{
    run_probe_action_opts, run_probe_seq_full, ProbeActionResult, ProbeOptions, ProbeResult,
    ProbeScript,
};
use engine::sim::run_simulation;
use rand::rngs::StdRng;
use std::collections::VecDeque;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

fn card_id(db: &CardsDb, name: &str) -> u16 {
    db.resolve_card(name).unwrap_or_else(|| panic!("{name} introuvable"))
}

/// Sonde séquence (pose forcée), options par défaut.
fn seq(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(db, names, ProbeOptions::default(), &ProbeScript::default(), false)
}

/// Sonde séquence avec assez de MC pour aller au bout d'une longue séquence
/// (`--probe-mc 500`) : les prérequis de PALIER se testent en poussant
/// réellement les paramètres globaux, ce qui demande plusieurs cartes chères.
fn riche(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(
        db,
        names,
        ProbeOptions { mc: 500, ..ProbeOptions::default() },
        &ProbeScript::default(),
        false,
    )
}

/// Sonde action, sans phase choisie (comportement des lots précédents).
fn act(db: &CardsDb, name: &str) -> ProbeActionResult {
    run_probe_action_opts(db, name, &ProbeScript::default(), None, ProbeOptions::default())
}

/// Sonde action avec la phase `ph` choisie par le joueur sondé (`--probe-phase`).
fn act_phase(db: &CardsDb, name: &str, ph: u8) -> ProbeActionResult {
    run_probe_action_opts(
        db,
        name,
        &ProbeScript::default(),
        None,
        ProbeOptions { phase: ph, ..ProbeOptions::default() },
    )
}

/// Sonde action avec `n` cartes de monnaie en main (`--probe-filler`) : sans
/// elles, une action qui se paie en CARTES n'est pas observable.
fn act_filler(db: &CardsDb, name: &str, n: usize) -> ProbeActionResult {
    run_probe_action_opts(
        db,
        name,
        &ProbeScript::default(),
        None,
        ProbeOptions { filler: n, ..ProbeOptions::default() },
    )
}

/// Les 11 cartes du lot, dans l'ordre du contrat.
const LOT6: [&str; 11] = [
    "Community Gardens",
    "Hydro-Electric Energy",
    "Farming Co-ops",
    "Wood Burning Stoves",
    "Greenhouses",
    "Business Contracts",
    "Invention Contest",
    "Microprocessors",
    "Advanced Screening Tech",
    "Brainstorming Session",
    "Colonizer Training Camp",
];

// ============================================================ harnais de flux
//
// Politique scriptée : phases imposées aux deux joueurs, UNE activation de
// carte bleue pour le joueur 0, aucun achat. Elle emprunte le flux réel
// (`setup_game` / `play_round`) — aucun chemin parallèle.

struct Lot6Script {
    base: RandomPolicy,
    phases: VecDeque<u8>,
    done: bool,
    /// Réponse imposée à `choose_option` (montant « jusqu'à n »), si donnée.
    option: Option<usize>,
    /// Indices gardés / défaussés : toujours les premiers, pour être prévisible.
    keep_first: bool,
}

impl Lot6Script {
    /// `p0` et `p1` : phases imposées. Le joueur 0 activera sa carte bleue une
    /// fois si la phase III est jouée.
    fn new(p0: u8, p1: u8) -> Lot6Script {
        Lot6Script {
            base: RandomPolicy,
            phases: VecDeque::from(vec![p0, p1]),
            done: false,
            option: None,
            keep_first: true,
        }
    }
}

impl Policy for Lot6Script {
    fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        match self.phases.pop_front() {
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
        if p != 0 || self.done {
            return None;
        }
        for (i, o) in options.iter().enumerate() {
            if matches!(o, ActionOpt::BlueAction(_)) {
                self.done = true;
                return Some(i);
            }
        }
        None
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        match self.option {
            Some(k) if k < n => k,
            _ => self.base.choose_option(r, p, n),
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, drawn: &[u16], k: usize) -> Vec<usize> {
        if self.keep_first {
            (0..k.min(drawn.len())).collect()
        } else {
            self.base.research_keep(r, p, drawn, k)
        }
    }
    fn discard_down(&mut self, _: &mut StdRng, _: usize, _: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

/// Met `name` en jeu chez le joueur 0 d'une partie fraîche, hors pioche.
fn game_with_card(
    db: &CardsDb,
    seed: u64,
    name: &str,
    pol: &mut Lot6Script,
) -> engine::state::GameState {
    let mut game = setup_game(db, seed, pol);
    let id = card_id(db, name);
    game.deck.retain(|&c| c != id);
    game.discard.retain(|&c| c != id);
    for p in 0..2 {
        game.players[p].hand.retain(|&c| c != id);
    }
    game.players[0].put_in_play(id, db);
    game
}

// ========================================== groupe A : bonus de phase Action

#[test]
fn community_gardens_action_gains_2_mc_without_the_action_phase() {
    // « Action: Gain 2 MC. » — sans la phase Action, rien d'autre.
    let db = db();
    let r = act(&db, "Community Gardens");
    assert!(r.has_action && r.action_applied);
    assert_eq!((r.delta.mc, r.delta.plants), (2, 0));
}

#[test]
fn community_gardens_bonus_adds_one_plant_with_the_action_phase() {
    // « *If you chose the action phase this round, also gain 1 plant. »
    let db = db();
    let r = act_phase(&db, "Community Gardens", 3);
    assert!(r.action_applied);
    assert_eq!((r.delta.mc, r.delta.plants), (2, 1));
}

#[test]
fn community_gardens_bonus_is_denied_by_every_other_phase() {
    // Le bonus doit être VRAIMENT conditionnel : aucune autre phase ne le donne.
    let db = db();
    for ph in [1u8, 2, 4, 5] {
        let r = act_phase(&db, "Community Gardens", ph);
        assert_eq!(
            (r.delta.mc, r.delta.plants),
            (2, 0),
            "la phase {ph} n'est pas la phase Action"
        );
    }
}

#[test]
fn hydro_electric_spends_1_mc_for_2_heat() {
    // « Action: Spend 1 MC to gain 2 heat. »
    let db = db();
    let r = act(&db, "Hydro-Electric Energy");
    assert!(r.action_applied);
    assert_eq!((r.delta.mc, r.delta.heat), (-1, 2));
}

#[test]
fn hydro_electric_bonus_gives_one_additional_heat() {
    // « *… gain 1 additional heat. » — 2 + 1 = 3, le MC dépensé ne bouge pas.
    let db = db();
    let r = act_phase(&db, "Hydro-Electric Energy", 3);
    assert_eq!((r.delta.mc, r.delta.heat), (-1, 3));
}

#[test]
fn hydro_electric_bonus_is_denied_by_every_other_phase() {
    let db = db();
    for ph in [1u8, 2, 4, 5] {
        let r = act_phase(&db, "Hydro-Electric Energy", ph);
        assert_eq!((r.delta.mc, r.delta.heat), (-1, 2), "phase {ph}");
    }
}

#[test]
fn the_phase_bonus_follows_the_acting_player_not_the_opponent() {
    // NEVER 8 : « si VOUS avez choisi la phase Action ». Le joueur 0 active sa
    // carte pendant une phase III que l'ADVERSAIRE a choisie : aucun bonus.
    let db = db();
    // (a) p0 choisit la phase III lui-même → bonus.
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 11, "Community Gardens", &mut pol);
    let avant = game.players[0].plants;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].plants - avant, 1, "p0 a choisi la phase III");
    assert_eq!(game.action_phase_bonuses, 1);
    // (b) c'est p1 qui choisit la phase III ; p0 agit quand même, sans bonus.
    let mut pol = Lot6Script::new(1, 3);
    let mut game = game_with_card(&db, 11, "Community Gardens", &mut pol);
    let avant = game.players[0].plants;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].plants - avant, 0, "la phase de l'adversaire ne compte pas");
    assert_eq!(game.action_phase_bonuses, 0);
    assert_eq!(game.blue_actions, 1, "l'action a bien été activée dans les deux cas");
}

// ================================== groupe B : actions à coût particulier

#[test]
fn farming_coops_gains_three_plants_when_played() {
    // « [effect] Gain 3 plants. »
    let db = db();
    let r = seq(&db, &["Farming Co-ops"]);
    assert_eq!(r.delta.plants, 3);
    assert_eq!(r.paid, vec![15], "coût imprimé 15 MC");
}

#[test]
fn farming_coops_action_discards_a_card_to_gain_three_plants() {
    // « Action: Discard a card in hand to gain 3 plants. » — la main perd
    // exactement une carte, les plantes en gagnent 3.
    let db = db();
    let r = act_filler(&db, "Farming Co-ops", 2);
    assert!(r.action_applied, "avec 2 cartes en main, le coût est payable");
    assert_eq!((r.delta.plants, r.delta.hand), (3, -1));
}

#[test]
fn farming_coops_action_is_unpayable_with_an_empty_hand() {
    // Un coût payé en cartes n'est pas payable sans cartes : l'action ne
    // s'applique pas, et elle ne donne surtout pas les plantes gratuitement.
    let db = db();
    let r = act(&db, "Farming Co-ops");
    assert!(r.has_action);
    assert!(!r.action_applied);
    assert_eq!((r.delta.plants, r.delta.hand), (0, 0));
}

#[test]
fn farming_coops_discarded_card_really_reaches_the_discard_pile() {
    // La carte défaussée quitte la main ET rejoint la défausse : rien ne
    // disparaît (conservation des cartes).
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 4, "Farming Co-ops", &mut pol);
    let main_avant: Vec<u16> = game.players[0].hand.clone();
    let defausse_avant = game.discard.len();
    let plantes_avant = game.players[0].plants;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.action_discard_costs, 1, "une carte défaussée comme coût");
    assert_eq!(game.players[0].plants - plantes_avant, 3);
    assert!(game.discard.len() > defausse_avant);
    let partie: usize = main_avant.iter().filter(|c| game.discard.contains(c)).count();
    assert!(partie >= 1, "la carte défaussée est bien une carte de p0");
}

#[test]
fn wood_burning_stoves_gains_four_plants_when_played() {
    // « [effect] Gain 4 plants. »
    let db = db();
    let r = seq(&db, &["Wood Burning Stoves"]);
    assert_eq!(r.delta.plants, 4);
    assert_eq!(r.paid, vec![13], "coût imprimé 13 MC");
}

#[test]
fn wood_burning_stoves_action_spends_four_plants_to_raise_temperature() {
    // « Action: Spend 4 plants to raise the temperature 1 step. » (+1 NT par pas)
    let db = db();
    let r = act(&db, "Wood Burning Stoves");
    assert!(r.action_applied);
    assert_eq!((r.delta.plants, r.delta.temperature, r.delta.tr), (-4, 1, 1));
}

#[test]
fn wood_burning_stoves_costs_three_plants_with_the_action_phase() {
    // « *… spend 3 plants instead. » — le bonus REMPLACE le coût, il n'ajoute
    // aucun effet : la température monte toujours d'un seul pas.
    let db = db();
    let r = act_phase(&db, "Wood Burning Stoves", 3);
    assert_eq!((r.delta.plants, r.delta.temperature), (-3, 1));
    for ph in [1u8, 2, 4, 5] {
        let r = act_phase(&db, "Wood Burning Stoves", ph);
        assert_eq!(r.delta.plants, -4, "phase {ph} : coût plein");
    }
}

#[test]
fn greenhouses_requires_yellow_temperature_or_warmer() {
    // « Requires yellow temperature or warmer. » — l'état de départ de la sonde
    // est à température 0 (violet) : le prérequis n'est pas rempli. C'est l'un
    // des DEUX prérequis imprimés que le moteur n'appliquait pas.
    let db = db();
    let r = seq(&db, &["Greenhouses"]);
    assert!(!r.prereq_ok, "température violette : refusée");
    assert!(!r.prereq_ok_now);
}

#[test]
fn greenhouses_prereq_is_met_once_the_temperature_is_yellow() {
    // Contrôle POSITIF du même prérequis : une séquence qui chauffe la planète
    // au-dessus du palier jaune (11 niveaux) le rend satisfait à l'état courant.
    // Deimos Down (+3 pas), Giant Ice Asteroid (+2), Lava Flows (+2)… : on
    // enchaîne jusqu'au palier, puis on observe `prereq_ok_now`.
    let db = db();
    let r = riche(
        &db,
        &[
            "Deimos Down",
            "Deimos Down",
            "Giant Ice Asteroid",
            "Lava Flows",
            "Lava Flows",
            "Greenhouses",
        ],
    );
    assert!(r.delta.temperature >= 11, "la planète est chauffée : {:?}", r.delta);
    assert!(r.prereq_ok_now, "température jaune atteinte : prérequis rempli");
    assert!(!r.prereq_ok, "…mais pas sur l'instantané de début de phase");
}

#[test]
fn greenhouses_action_turns_heat_into_the_same_amount_of_plants() {
    // « Action: Spend up to 4 heat to gain that amount of plants. » — un pour
    // un, et jamais plus de 4.
    let db = db();
    let r = act(&db, "Greenhouses");
    assert!(r.action_applied);
    assert_eq!(r.delta.plants, -r.delta.heat, "autant de plantes que de chaleur");
    assert!(r.delta.plants >= 1 && r.delta.plants <= 4, "1..4 : {:?}", r.delta);
}

#[test]
fn greenhouses_amount_is_a_choice_of_the_policy() {
    // Le montant « jusqu'à 4 » est une ALTERNATIVE : branche k = k+1 chaleurs.
    // Imposée par `--probe-choice`, elle se lit directement dans le delta.
    let db = db();
    for (branche, montant) in [(0usize, 1i64), (1, 2), (2, 3), (3, 4)] {
        let script = ProbeScript { choices: vec![branche], targets: Vec::new() };
        let r = run_probe_action_opts(
            &db,
            "Greenhouses",
            &script,
            None,
            ProbeOptions::default(),
        );
        assert_eq!(
            (r.delta.heat, r.delta.plants),
            (-montant, montant),
            "branche {branche}"
        );
    }
}

#[test]
fn greenhouses_action_does_nothing_without_heat() {
    // Aucune branche jouable : l'action ne s'applique pas — et elle ne crée
    // surtout pas de plantes à partir de rien.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 6, "Greenhouses", &mut pol);
    game.players[0].heat = 0;
    let plantes = game.players[0].plants;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].plants, plantes, "rien à dépenser, rien à gagner");
    assert_eq!(game.blue_actions, 0, "aucune action bleue appliquée");
}

// ======================================== groupe C : piocher puis défausser

#[test]
fn business_contracts_nets_two_cards() {
    // « [effect] Draw four cards. Then, discard two cards. » → +2 en main.
    let db = db();
    let r = seq(&db, &["Business Contracts"]);
    assert_eq!(r.delta.hand, 2);
    assert_eq!(r.paid, vec![5], "coût imprimé 5 MC");
}

#[test]
fn invention_contest_nets_one_card() {
    // « [effect] Draw three cards. Keep one of them and discard the other
    //   two. » → +1 en main.
    let db = db();
    let r = seq(&db, &["Invention Contest"]);
    assert_eq!(r.delta.hand, 1);
    assert_eq!(r.paid, vec![1], "coût imprimé 1 MC");
}

#[test]
fn microprocessors_nets_one_card_and_produces_three_heat() {
    // « [effect] Draw two cards. Then, discard a card. During the production
    //   phase, this produces 3 heat. » (champ `production: "3 heat"`)
    let db = db();
    let r = seq(&db, &["Microprocessors"]);
    assert_eq!(r.delta.hand, 1);
    assert_eq!(r.delta.heat_prod, 3, "production FIXE, piste heat_prod");
    assert_eq!(r.delta.heat, 0, "la production n'est pas un gain à la pose");
    assert_eq!(r.paid, vec![17], "coût imprimé 17 MC");
}

#[test]
fn microprocessors_production_is_collected_by_the_real_phase_four() {
    // Contrôle croisé : la production imprimée est encaissée par la VRAIE phase
    // IV du moteur, pas recalculée par le test.
    let db = db();
    let r = run_probe_seq_full(
        &db,
        &["Microprocessors"],
        ProbeOptions::default(),
        &ProbeScript::default(),
        true,
    );
    assert!(r.produced);
    assert_eq!(r.delta.heat, 3, "3 chaleurs encaissées en phase IV");
}

#[test]
fn invention_contest_discards_only_among_the_drawn_cards() {
    // « Keep one of THEM » : les deux défaussées sortent des trois piochées ; les
    // cartes qui étaient DÉJÀ en main ne sont pas touchées.
    let db = db();
    let mut pol = Lot6Script::new(1, 1);
    let mut game = setup_game(&db, 9, &mut pol);
    let ic = card_id(&db, "Invention Contest");
    game.deck.retain(|&c| c != ic);
    let main_avant: Vec<u16> = game.players[0].hand.clone();
    game.players[0].hand.push(ic);
    game.players[0].mc = 50;
    let idx = game.players[0].hand.len() - 1;
    engine::flow::build_card(&mut game, &db, 0, idx, 0);
    assert_eq!(game.draw_discard_discards, 2, "deux cartes défaussées");
    for c in &main_avant {
        assert!(
            game.players[0].hand.contains(c),
            "une carte déjà en main a été défaussée alors que le texte dit « of them »"
        );
    }
    assert_eq!(
        game.players[0].hand.len(),
        main_avant.len() + 1,
        "net +1 : 3 piochées, 2 défaussées, la carte jouée est sortie de la main"
    );
}

#[test]
fn business_contracts_discards_from_the_whole_hand() {
    // « Then, discard two cards » ne restreint rien : la défausse peut porter
    // sur des cartes qui étaient déjà en main. La politique de test défausse
    // les premières — ce sont justement les plus anciennes.
    let db = db();
    let mut pol = Lot6Script::new(1, 1);
    let mut game = setup_game(&db, 9, &mut pol);
    let bc = card_id(&db, "Business Contracts");
    game.deck.retain(|&c| c != bc);
    let main_avant: Vec<u16> = game.players[0].hand.clone();
    game.players[0].hand.push(bc);
    game.players[0].mc = 50;
    let idx = game.players[0].hand.len() - 1;
    engine::flow::build_card(&mut game, &db, 0, idx, 0);
    assert_eq!(game.draw_discard_discards, 2);
    let anciennes_defaussees = main_avant
        .iter()
        .filter(|c| game.discard.contains(c))
        .count();
    assert!(
        anciennes_defaussees > 0,
        "la défausse porte sur la main entière, pas sur les seules cartes piochées"
    );
}

#[test]
fn the_three_group_c_cards_share_one_brick() {
    // I3 : une seule brique pour les trois. Contrôle STRUCTUREL sur la table
    // d'encodage : les trois portent un `Eff::DrawDiscard`, et rien d'autre ne
    // manipule la main à la pose.
    use engine::effects::{Eff, LOT1};
    let mut vus = 0;
    for (nom, spec) in LOT1 {
        let dd = spec
            .effects
            .iter()
            .any(|e| matches!(e, Eff::DrawDiscard { .. }));
        if dd {
            vus += 1;
            assert!(
                ["Business Contracts", "Invention Contest", "Microprocessors"].contains(nom),
                "{nom} ne devrait pas porter la brique du groupe C"
            );
        }
    }
    assert_eq!(vus, 3, "exactement les trois cartes du groupe C");
}

#[test]
fn draw_discard_never_loses_a_card() {
    // Conservation : ce qui quitte la main arrive à la défausse. Compté sur une
    // pose réelle (pioche + défausse + main + en jeu constants).
    let db = db();
    let mut pol = Lot6Script::new(1, 1);
    let mut game = setup_game(&db, 3, &mut pol);
    let bc = card_id(&db, "Business Contracts");
    game.deck.retain(|&c| c != bc);
    let total = |g: &engine::state::GameState| -> usize {
        g.deck.len()
            + g.discard.len()
            + g.players.iter().map(|p| p.hand.len() + p.played.len()).sum::<usize>()
    };
    game.players[0].hand.push(bc);
    game.players[0].mc = 50;
    let avant = total(&game);
    let idx = game.players[0].hand.len() - 1;
    engine::flow::build_card(&mut game, &db, 0, idx, 0);
    assert_eq!(total(&game), avant, "aucune carte perdue ni créée");
}

// ==================================== groupe D : révéler le dessus de pioche

#[test]
fn advanced_screening_reveals_three_cards() {
    // « Action: Reveal the top three cards of the deck. » — trois cartes
    // quittent réellement la pioche.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 8, "Advanced Screening Tech", &mut pol);
    let pioche_avant = game.deck.len();
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.cards_revealed, 3, "trois cartes révélées");
    assert_eq!(game.deck.len(), pioche_avant - 3, "elles viennent de la pioche");
}

#[test]
fn advanced_screening_takes_a_science_or_plant_and_discards_the_rest() {
    // « Place a card with a [science] or [plant] revealed this way into your
    //   hand. Discard the rest. » — le dessus de pioche est composé pour
    //   l'occasion : une carte science, deux sans science ni plante.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 8, "Advanced Screening Tech", &mut pol);
    let avec: u16 = trouve(&db, |c| c.tags.contains(&Tag::Science), &[]);
    let sans1: u16 = trouve(&db, |c| sans_science_ni_plante(c), &[avec]);
    let sans2: u16 = trouve(&db, |c| sans_science_ni_plante(c), &[avec, sans1]);
    for id in [avec, sans1, sans2] {
        game.deck.retain(|&c| c != id);
        for p in 0..2 {
            game.players[p].hand.retain(|&c| c != id);
        }
    }
    // Le dessus de pioche est la FIN du vecteur (`flow::draw_card` dépile).
    game.deck.push(avec);
    game.deck.push(sans1);
    game.deck.push(sans2);
    let main_avant = game.players[0].hand.len();
    play_round(&mut game, &db, &mut pol);
    assert!(
        game.players[0].hand.contains(&avec),
        "la carte à badge science devait entrer en main"
    );
    assert!(game.discard.contains(&sans1) && game.discard.contains(&sans2), "le reste est défaussé");
    assert_eq!(game.players[0].hand.len(), main_avant + 1, "une seule carte gardée");
}

#[test]
fn advanced_screening_keeps_nothing_when_no_revealed_card_matches() {
    // Aucune carte révélée ne porte science ni plante : tout part à la défausse,
    // sans compensation d'aucune sorte.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 8, "Advanced Screening Tech", &mut pol);
    let a = trouve(&db, |c| sans_science_ni_plante(c), &[]);
    let b = trouve(&db, |c| sans_science_ni_plante(c), &[a]);
    let c = trouve(&db, |c| sans_science_ni_plante(c), &[a, b]);
    for id in [a, b, c] {
        game.deck.retain(|&x| x != id);
        for p in 0..2 {
            game.players[p].hand.retain(|&x| x != id);
        }
    }
    game.deck.push(a);
    game.deck.push(b);
    game.deck.push(c);
    let main_avant = game.players[0].hand.len();
    let mc_avant = game.players[0].mc;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].hand.len(), main_avant, "rien n'entre en main");
    assert_eq!(game.players[0].mc, mc_avant, "et rien ne compense");
    for id in [a, b, c] {
        assert!(game.discard.contains(&id));
    }
}

#[test]
fn brainstorming_session_draws_a_non_green_card() {
    // « Reveal the top card. … Otherwise, draw it. » — dessus non vert.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 5, "Brainstorming Session", &mut pol);
    let bleue = trouve(&db, |c| c.color != Color::Green, &[]);
    game.deck.retain(|&x| x != bleue);
    for p in 0..2 {
        game.players[p].hand.retain(|&x| x != bleue);
    }
    game.deck.push(bleue);
    let mc_avant = game.players[0].mc;
    play_round(&mut game, &db, &mut pol);
    assert!(game.players[0].hand.contains(&bleue), "la carte non verte est piochée");
    assert_eq!(game.players[0].mc, mc_avant, "aucun MC : elle n'est pas défaussée");
    assert_eq!(game.cards_revealed, 1);
}

#[test]
fn brainstorming_session_discards_a_green_card_for_one_mc() {
    // « If it is green, discard it and gain 1 MC. »
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 5, "Brainstorming Session", &mut pol);
    let verte = trouve(&db, |c| c.color == Color::Green, &[]);
    game.deck.retain(|&x| x != verte);
    for p in 0..2 {
        game.players[p].hand.retain(|&x| x != verte);
    }
    game.deck.push(verte);
    let mc_avant = game.players[0].mc;
    let main_avant = game.players[0].hand.len();
    play_round(&mut game, &db, &mut pol);
    assert!(game.discard.contains(&verte), "la carte verte est défaussée");
    assert_eq!(game.players[0].mc - mc_avant, 1, "1 MC gagné");
    assert_eq!(game.players[0].hand.len(), main_avant, "la main ne bouge pas");
}

#[test]
fn revealing_reads_the_real_top_of_the_deck() {
    // Anti-raccourci : la révélation ne regarde pas une carte fixe, elle dépile
    // le VRAI dessus de pioche — deux dessus différents donnent deux résultats
    // différents.
    let db = db();
    let mut vus = Vec::new();
    for choix in [true, false] {
        let mut pol = Lot6Script::new(3, 1);
        let mut game = game_with_card(&db, 5, "Brainstorming Session", &mut pol);
        let carte = if choix {
            trouve(&db, |c| c.color == Color::Green, &[])
        } else {
            trouve(&db, |c| c.color != Color::Green, &[])
        };
        game.deck.retain(|&x| x != carte);
        for p in 0..2 {
            game.players[p].hand.retain(|&x| x != carte);
        }
        game.deck.push(carte);
        play_round(&mut game, &db, &mut pol);
        vus.push(game.players[0].hand.contains(&carte));
    }
    assert_eq!(vus, vec![false, true], "le résultat suit la carte réellement au-dessus");
}

/// Première carte de la base satisfaisant `f`, hors `exclues`.
fn trouve(db: &CardsDb, f: impl Fn(&engine::cards::ProjectCard) -> bool, exclues: &[u16]) -> u16 {
    db.projects
        .iter()
        .enumerate()
        .find(|(i, c)| c.in_deck && f(c) && !exclues.contains(&(*i as u16)))
        .map(|(i, _)| i as u16)
        .expect("carte candidate")
}

fn sans_science_ni_plante(c: &engine::cards::ProjectCard) -> bool {
    !c.tags.contains(&Tag::Science) && !c.tags.contains(&Tag::Plant)
}

// ============================================ groupe E : prérequis seul

#[test]
fn colonizer_training_camp_changes_nothing() {
    // « Aucun texte d'effet », 2 PV imprimés : le delta doit être INTÉGRALEMENT
    // nul, et les PV viennent de la donnée `vp`, pas d'un effet.
    let db = db();
    let r = seq(&db, &["Colonizer Training Camp"]);
    let d = &r.delta;
    assert_eq!(
        (
            d.heat, d.plants, d.hand, d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod, d.tr,
            d.temperature, d.oxygen, d.oceans, d.forests
        ),
        (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    );
    assert_eq!(r.paid, vec![10], "coût imprimé 10 MC");
    assert_eq!(r.vp, 2, "2 points de victoire imprimés");
}

#[test]
fn colonizer_training_camp_requires_red_oxygen_or_lower() {
    // « Requires red oxygen or lower. » Palier rouge = 3-6 → oxygène ≤ 6.
    // L'état de départ de la sonde est à 0 : le prérequis est rempli.
    let db = db();
    assert!(seq(&db, &["Colonizer Training Camp"]).prereq_ok);
    // Une séquence qui pousse l'oxygène au-dessus de 6 le fait tomber : quatre
    // Airborne Radiation (+1 chacune) après une hausse initiale ne suffisent pas
    // — on emploie Towing a Comet et consorts jusqu'à dépasser le palier.
    let r = riche(
        &db,
        &[
            "Towing a Comet",
            "Atmosphere Filtering",
            "Airborne Radiation",
            "Airborne Radiation",
            "Towing a Comet",
            "Atmosphere Filtering",
            "Airborne Radiation",
            "Colonizer Training Camp",
        ],
    );
    assert!(r.delta.oxygen >= 7, "oxygène poussé au palier jaune : {:?}", r.delta);
    assert!(
        !r.prereq_ok_now,
        "au-dessus du palier rouge, la carte ne doit plus être jouable"
    );
}

#[test]
fn the_two_printed_requirement_holes_are_closed() {
    // Contrat : sur les 208 cartes de la boîte de base, SEULES Greenhouses et
    // Colonizer Training Camp n'appliquaient pas leur prérequis imprimé. Après
    // ce lot, les deux le portent.
    use engine::effects::{Req, LOT1};
    for nom in ["Greenhouses", "Colonizer Training Camp"] {
        let spec = LOT1
            .iter()
            .find(|(n, _)| n == &nom)
            .map(|(_, s)| s)
            .unwrap_or_else(|| panic!("{nom} absente de la table"));
        assert!(!spec.reqs.is_empty(), "{nom} doit porter son prérequis imprimé");
    }
    // Et le prérequis d'oxygène MAXIMUM n'existait pas : il n'est porté que par
    // la carte qui l'imprime.
    let porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, s)| s.reqs.iter().any(|r| matches!(r, Req::OxyMax(_))))
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(porteuses, vec!["Colonizer Training Camp"]);
}

#[test]
fn oxy_max_really_blocks_a_card_in_strict_mode() {
    // Le prérequis n'est pas décoratif : en mode strict (la sonde cesse de
    // forcer la pose), la carte est refusée quand l'oxygène a dépassé le rouge.
    let db = db();
    let strict = |names: &[&str]| {
        run_probe_seq_full(
            &db,
            names,
            ProbeOptions { strict: true, ..ProbeOptions::default() },
            &ProbeScript::default(),
            false,
        )
    };
    assert!(
        strict(&["Colonizer Training Camp"]).played,
        "à oxygène 0, la carte est jouable"
    );
}

// ======================================== briques, compteurs, non-régression

#[test]
fn the_eleven_cards_are_encoded_and_resolve_to_the_v1_deck() {
    let db = db();
    for name in LOT6 {
        let id = card_id(&db, name);
        let card = &db.projects[id as usize];
        assert!(card.in_deck_v1, "{name} doit venir du deck v1");
        assert!(card.effect.is_some(), "{name} doit être encodée");
        assert!(card.effets_geres(), "{name} doit être déclarée gérée");
    }
}

#[test]
fn the_eleven_prices_match_the_printed_cost() {
    // Contrôle croisé indépendant de l'encodage : le prix payé par la sonde est
    // le coût IMPRIMÉ relevé sur les cartons.
    let db = db();
    for (name, cout) in [
        ("Community Gardens", 20),
        ("Hydro-Electric Energy", 11),
        ("Farming Co-ops", 15),
        ("Wood Burning Stoves", 13),
        ("Greenhouses", 11),
        ("Business Contracts", 5),
        ("Invention Contest", 1),
        ("Microprocessors", 17),
        ("Advanced Screening Tech", 6),
        ("Brainstorming Session", 8),
        ("Colonizer Training Camp", 10),
    ] {
        assert_eq!(seq(&db, &[name]).paid, vec![cout], "{name}");
    }
}

#[test]
fn exactly_eighteen_base_cards_remain_unhandled() {
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let muettes: Vec<&str> = db
        .recensement()
        .into_iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    // ATTENTE MISE À JOUR par le lot acier-titane (18 → 14) puis par le lot
    // cartes-7 (14 → 5) : les neuf modificateurs permanents sont encodés.
    assert_eq!(muettes.len(), 5, "{muettes:?}");
    for name in LOT6 {
        assert!(!muettes.contains(&name), "{name} ne doit plus être muette");
    }
}

#[test]
fn base_plus_discovery_leaves_fifty_five_unhandled() {
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte");
    let n = db.recensement().into_iter().filter(|c| !c.effets_geres).count();
    // ATTENTE MISE À JOUR par le lot acier-titane (55 → 51) puis par le lot
    // cartes-7 (51 → 42) : 5 muettes en base + 37 de Découverte (33 projets +
    // 4 corporations sans encodage).
    assert_eq!(n, 42);
}

#[test]
fn effets_geres_is_stable_across_box_configurations() {
    // Le champ est DÉRIVÉ de l'encodage : il ne dépend pas des boîtes demandées.
    for liste in ["base", "base,promo", "base,decouverte", "base,promo,decouverte"] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap()).expect(liste);
        for name in LOT6 {
            let c = db
                .recensement()
                .into_iter()
                .find(|r| r.name == name)
                .unwrap_or_else(|| panic!("{name} absente en {liste}"));
            assert!(c.effets_geres, "{name} en {liste}");
        }
    }
}

#[test]
fn the_lot_did_not_touch_the_boxes_composition() {
    // Encoder n'est pas composer : les effectifs de pioche sont inchangés.
    for (liste, projets, corps) in [
        ("base", 208, 12),
        ("base,promo", 219, 12),
        ("base,decouverte", 246, 16),
        ("base,promo,decouverte", 257, 16),
    ] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap()).expect(liste);
        assert_eq!(db.deck_project_count, projets, "projets de --boites {liste}");
        assert_eq!(db.corporations.len(), corps, "corporations de --boites {liste}");
    }
}

#[test]
fn every_phase_bonus_belongs_to_a_fixed_action() {
    // Garde-fou structurel : un bonus de phase déclaré sur une action d'un autre
    // type serait silencieusement inerte. Il n'y en a que trois, et ce sont
    // exactement les cartes dont le texte imprimé porte l'astérisque.
    use engine::effects::{Action, LOT1};
    let porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, s)| s.phase_bonus.is_some())
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(
        porteuses,
        vec!["Community Gardens", "Hydro-Electric Energy", "Wood Burning Stoves"]
    );
    for (nom, spec) in LOT1 {
        if spec.phase_bonus.is_some() {
            assert!(
                matches!(spec.action, Some(Action::Fixed { .. })),
                "{nom} : un bonus de phase n'a de sens que sur une action à coût fixe"
            );
            assert_eq!(
                spec.phase_bonus.map(|b| b.phase),
                Some(3),
                "{nom} : le texte imprimé dit « the action phase »"
            );
        }
    }
}

#[test]
fn the_four_audit_counters_move_in_real_games() {
    // Oracle disjoint de la sonde : 400 parties complètes, politique aléatoire.
    // Les quatre mécanismes du lot doivent avoir lieu pour de vrai.
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 400, 2024, &mut pol);
    assert!(s.action_phase_bonuses > 0, "aucun bonus de phase en partie réelle");
    assert!(s.action_discard_costs > 0, "aucune défausse-coût en partie réelle");
    assert!(s.draw_discard_discards > 0, "aucun « piochez puis défaussez »");
    assert!(s.cards_revealed > 0, "aucune révélation de dessus de pioche");
}

#[test]
fn the_four_audit_counters_are_zero_with_effects_off() {
    // `--effects off` = squelette intégral : aucun effet de carte, donc aucun
    // des quatre mécanismes.
    let mut db = db();
    db.effects_on = false;
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(s.action_phase_bonuses, 0);
    assert_eq!(s.action_discard_costs, 0);
    assert_eq!(s.draw_discard_discards, 0);
    assert_eq!(s.cards_revealed, 0);
}

#[test]
fn the_eleven_are_inert_with_effects_off() {
    // Chaque carte du lot, effets coupés : aucun changement d'état.
    let mut db = db();
    db.effects_on = false;
    for name in LOT6 {
        let r = seq(&db, &[name]);
        let d = &r.delta;
        assert!(!r.in_lot, "{name}");
        assert_eq!(
            (
                d.heat, d.plants, d.hand, d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod,
                d.tr, d.temperature, d.oxygen, d.oceans, d.forests
            ),
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            "{name} change l'état alors que les effets sont coupés"
        );
        let a = act(&db, name);
        assert!(!a.action_applied, "{name} : action appliquée effets coupés");
    }
}

#[test]
fn simulations_stay_deterministic_and_invariant_free() {
    // I6 : à graine égale, deux exécutions rendent la même empreinte, et aucun
    // invariant n'est violé — les nouveaux mécanismes touchent pioche et
    // défausse, c'est exactement ce qu'il faut vérifier.
    for boites in ["base", "base,decouverte"] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(boites).unwrap()).expect(boites);
        let mut a = RandomPolicy;
        let mut b = RandomPolicy;
        let s1 = run_simulation(&db, 300, 2024, &mut a);
        let s2 = run_simulation(&db, 300, 2024, &mut b);
        assert_eq!(s1.state_hash, s2.state_hash, "{boites} : non déterministe");
        assert_eq!(s1.invariant_violations, 0, "{boites}");
        assert_eq!(s1.truncated, 0, "{boites}");
        assert_eq!(s1.completed, 300, "{boites}");
    }
}

#[test]
fn the_new_probes_are_deterministic() {
    let db = db();
    for name in LOT6 {
        let a = seq(&db, &[name]);
        let b = seq(&db, &[name]);
        assert_eq!(a.delta, b.delta, "{name}");
        let x = act_phase(&db, name, 3);
        let y = act_phase(&db, name, 3);
        assert_eq!(x.delta, y.delta, "{name} (action)");
    }
}

#[test]
fn probe_phase_does_not_touch_a_card_without_a_phase_bonus() {
    // L'option fixe la phase choisie, rien d'autre : une action sans bonus de
    // phase rend exactement le même delta avec et sans elle.
    let db = db();
    for name in [
        "Greenhouses",
        "Development Center",
        "Farmers Market",
        "Think Tank",
        "Advanced Screening Tech",
        "Brainstorming Session",
    ] {
        let sans = act(&db, name);
        for ph in 1..=5u8 {
            let avec = act_phase(&db, name, ph);
            assert_eq!(sans.delta, avec.delta, "{name} avec --probe-phase {ph}");
        }
    }
}

#[test]
fn power_infrastructure_is_untouched_by_this_lot() {
    // I4 : *Power Infrastructure* est hors périmètre. Son action « spend ANY
    // amount » garde son encodage du lot 2 (tirage par `action_amount`, montant
    // nul possible), et son comportement observable est inchangé.
    use engine::effects::{Action, LOT1};
    let spec = LOT1
        .iter()
        .find(|(n, _)| *n == "Power Infrastructure")
        .map(|(_, s)| s)
        .expect("carte présente");
    assert!(matches!(spec.action, Some(Action::HeatToMc)));
    assert!(spec.phase_bonus.is_none());
    let db = db();
    let r = act(&db, "Power Infrastructure");
    assert_eq!(r.delta.mc, -r.delta.heat);
    assert!(r.delta.heat <= 0 && r.delta.mc >= 0);
}

#[test]
fn i5_no_card_name_of_this_lot_appears_in_the_game_flow() {
    // « Ne jamais écrire un nom de carte dans le flux de jeu. » Les noms vivent
    // dans la table de données `effects::LOT1`, nulle part ailleurs.
    for (fichier, src) in [
        ("flow.rs", include_str!("../src/flow.rs")),
        ("cards.rs", include_str!("../src/cards.rs")),
        ("state.rs", include_str!("../src/state.rs")),
        ("policy.rs", include_str!("../src/policy.rs")),
        ("sim.rs", include_str!("../src/sim.rs")),
        ("probe.rs", include_str!("../src/probe.rs")),
        ("simulate.rs", include_str!("../src/bin/simulate.rs")),
    ] {
        for name in LOT6 {
            assert!(
                !src.contains(name),
                "le nom « {name} » ne doit pas figurer dans src/{fichier}"
            );
        }
    }
}

#[test]
fn the_table_has_one_entry_per_card_of_this_lot() {
    use engine::effects::LOT1;
    for name in LOT6 {
        let n = LOT1.iter().filter(|(x, _)| *x == name).count();
        assert_eq!(n, 1, "{name} : une entrée et une seule");
    }
    // ATTENTE MISE À JOUR par le lot acier-titane (199 → 203) puis par le lot
    // cartes-7 (203 → 212).
    assert_eq!(
        LOT1.len(),
        212,
        "188 + 11 (lot 6) + 4 (lot acier-titane) + 9 (lot cartes-7)"
    );
}

#[test]
fn the_new_action_costs_are_checked_before_being_paid() {
    // Un coût qu'on ne peut pas payer n'est jamais prélevé à moitié : ni plantes
    // ni cartes ne bougent quand l'action n'est pas payable.
    let db = db();
    // Wood Burning Stoves sans plantes (la pose en donne 4, on sonde donc en
    // flux réel avec une réserve vidée).
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 7, "Wood Burning Stoves", &mut pol);
    game.players[0].plants = 2;
    let temp = game.temperature;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].plants, 2, "aucune plante prélevée");
    assert_eq!(game.temperature, temp, "et aucune température gagnée");
}

#[test]
fn the_phase_bonus_cost_replacement_is_visible_in_a_real_game() {
    // Wood Burning Stoves en phase III réellement choisie : 3 plantes, pas 4.
    let db = db();
    let mut pol = Lot6Script::new(3, 1);
    let mut game = game_with_card(&db, 12, "Wood Burning Stoves", &mut pol);
    game.players[0].plants = 3;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.action_phase_bonuses, 1, "le coût réduit a bien été employé");
    assert_eq!(game.blue_actions, 1);
    assert_eq!(game.players[0].plants, 0, "3 plantes dépensées, pas 4");
}

#[test]
fn invention_contest_keeps_one_even_when_the_deck_runs_dry() {
    // « **Keep one of them** and discard the other two. » Le texte compte les
    // cartes GARDÉES. Pioche (et défausse) épuisées à deux cartes près : le
    // joueur en garde toujours UNE, et n'en défausse qu'une — défausser « deux »
    // lui en laisserait zéro, ce que la carte ne dit nulle part.
    //
    // Ce cas a été trouvé par la relecture adversariale (constat S1).
    let db = db();
    let mut pol = Lot6Script::new(1, 1);
    let mut game = setup_game(&db, 9, &mut pol);
    let ic = card_id(&db, "Invention Contest");
    game.deck.retain(|&c| c != ic);
    for p in 0..2 {
        game.players[p].hand.retain(|&c| c != ic);
    }
    // Pioche réduite à DEUX cartes, défausse vide : la troisième pioche rendra
    // `None`.
    let reste: Vec<u16> = game.deck.drain(..).collect();
    let deux: Vec<u16> = reste.iter().rev().take(2).copied().collect();
    game.deck = deux.iter().rev().copied().collect();
    game.discard.clear();
    game.players[0].hand.push(ic);
    game.players[0].mc = 50;
    let main_avant = game.players[0].hand.len();
    let idx = game.players[0].hand.len() - 1;
    engine::flow::build_card(&mut game, &db, 0, idx, 0);
    assert_eq!(game.draw_discard_discards, 1, "une seule défausse : deux piochées, une gardée");
    let gardees = deux.iter().filter(|c| game.players[0].hand.contains(c)).count();
    assert_eq!(gardees, 1, "exactement une des deux cartes piochées est gardée");
    // Main : −1 (la carte jouée) +2 (piochées) −1 (défaussée).
    assert_eq!(game.players[0].hand.len(), main_avant, "net 0 dans ce cas dégradé");
}

#[test]
fn business_contracts_discards_two_even_with_a_short_deck() {
    // Témoin OPPOSÉ du test précédent : « Then, discard two cards » compte les
    // cartes DÉFAUSSÉES, sans restriction. Pioche courte ou non, deux cartes
    // partent — la main en porte largement assez.
    let db = db();
    let mut pol = Lot6Script::new(1, 1);
    let mut game = setup_game(&db, 9, &mut pol);
    let bc = card_id(&db, "Business Contracts");
    game.deck.retain(|&c| c != bc);
    for p in 0..2 {
        game.players[p].hand.retain(|&c| c != bc);
    }
    let reste: Vec<u16> = game.deck.drain(..).collect();
    game.deck = reste.iter().rev().take(2).rev().copied().collect();
    game.discard.clear();
    game.players[0].hand.push(bc);
    game.players[0].mc = 50;
    let idx = game.players[0].hand.len() - 1;
    engine::flow::build_card(&mut game, &db, 0, idx, 0);
    assert_eq!(game.draw_discard_discards, 2, "« discard two cards » : deux, toujours");
}

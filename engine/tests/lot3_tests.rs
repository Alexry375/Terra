//! Tests du lot 3 — conformité au livret officiel + règles maison.
//!
//! Chaque règle corrigée est prouvée par au moins un test qui ÉCHOUE sur
//! l'ancien comportement (test de non-régression inversé) ; le commentaire de
//! chaque test indique ce que l'ancien moteur produisait. Tout passe par le flux
//! réel (`setup_game` / `play_round` / `build_card*`) ou par la sonde, jamais
//! par une logique réécrite pour le test.
//!
//! - C1 : prérequis de paramètres jugés sur l'instantané de début de phase.
//! - C2 : bonus construction — pioche AVANT ou APRÈS la pose.
//! - C3 : paiement d'une carte par défausse de cartes (3 MC/carte).
//! - C4 : ordre du tour J1/J2 alterné (règle maison).
//! - C5 : égalité sèche + conversion obligatoire jugée sur l'instantané.

use engine::cards::CardsDb;
use engine::flow::{
    build_card, build_card_with, card_discount, play_round, requirements_met,
    requirements_met_now, score, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq, run_probe_seq_opts, ProbeOptions};
use engine::sim::{play_game, run_simulation};
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::VecDeque;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

fn card_id(db: &CardsDb, name: &str) -> u16 {
    db.projects.iter().position(|c| c.name == name).unwrap() as u16
}

/// Remplace la main du joueur `p` par exactement `cards` (le reste retourne
/// dans la pioche, les cartes voulues en sont retirées : la conservation des
/// cartes reste vraie).
fn set_hand(game: &mut GameState, p: usize, cards: &[u16]) {
    let old: Vec<u16> = game.players[p].hand.drain(..).collect();
    game.deck.extend(old);
    game.deck.retain(|c| !cards.contains(c));
    game.players[p].hand.extend_from_slice(cards);
}

/// Politique scriptée du lot 3 : phases par joueur, bonus de construction fixé,
/// construction de la première option offerte (par joueur), actions scriptées
/// par joueur, et journalisation de l'ORDRE des appels (c'est ce qui permet
/// d'observer l'ordre du tour sans le déduire d'une formule).
struct Script {
    base: RandomPolicy,
    phases: [VecDeque<u8>; 2],
    bonus: ConstructionBonus,
    build_first: [bool; 2],
    /// Nombre d'actions `SellCard` que chaque joueur veut encore faire.
    sell_budget: [usize; 2],
    // Journaux.
    build_calls: Vec<(usize, Vec<usize>)>,
    action_calls: Vec<usize>,
}

impl Script {
    fn new(p0: Vec<u8>, p1: Vec<u8>) -> Script {
        Script {
            base: RandomPolicy,
            phases: [VecDeque::from(p0), VecDeque::from(p1)],
            bonus: ConstructionBonus::SecondBuild,
            build_first: [false, false],
            sell_budget: [0, 0],
            build_calls: Vec::new(),
            action_calls: Vec::new(),
        }
    }
}

impl Policy for Script {
    fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        match self.phases[p].pop_front() {
            Some(ph) if allowed.contains(&ph) => ph,
            _ => self.base.pick_phase(r, p, allowed),
        }
    }
    fn choose_build(&mut self, _: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.build_calls.push((p, a.to_vec()));
        if self.build_first[p] {
            a.first().copied()
        } else {
            None
        }
    }
    fn construction_bonus(&mut self, _: &mut StdRng, _: usize) -> ConstructionBonus {
        self.bonus
    }
    fn action_choice(&mut self, _: &mut StdRng, p: usize, options: &[ActionOpt]) -> Option<usize> {
        self.action_calls.push(p);
        if self.sell_budget[p] == 0 {
            return None;
        }
        let idx = options.iter().position(|&o| o == ActionOpt::SellCard)?;
        self.sell_budget[p] -= 1;
        Some(idx)
    }
    fn research_keep(&mut self, _: &mut StdRng, _: usize, _: &[u16], k: usize) -> Vec<usize> {
        (0..k).collect()
    }
    fn discard_down(&mut self, _: &mut StdRng, _: usize, _: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

// ===================================================================== C1
// Prérequis de paramètres jugés sur l'instantané de début de phase.
// Livret p.13, l.352 : « ce prérequis doit être rempli au début de la phase ».

#[test]
fn c1_param_prereq_reads_the_phase_snapshot_not_the_current_value() {
    // Inversé : l'ancien moteur lisait `game.temperature` et répondait `true`.
    let db = db();
    let mut game = setup_game(&db, 1, &mut RandomPolicy);
    let bushes = card_id(&db, "Bushes"); // « red temperature or warmer » = niveau 6
    game.temperature = 0;
    game.snapshot_planet(); // instantané pris à −30 °C
    game.temperature = 10; // la température monte PENDANT la phase
    assert!(
        !requirements_met(&game, &db, 0, bushes),
        "l'instantané de début de phase (0) interdit Bushes"
    );
    assert!(
        requirements_met_now(&game, &db, 0, bushes),
        "l'état courant (10) l'autoriserait — c'est exactement l'écart E6"
    );
    // Et l'inverse : instantané suffisant, valeur courante indifférente.
    game.snapshot_planet();
    assert!(requirements_met(&game, &db, 0, bushes));
}

#[test]
fn c1_tag_and_spend_prereqs_stay_on_the_current_state() {
    // Le livret ne met QUE les océans/oxygène/température sous la règle de
    // l'instantané : tags et dépenses restent jugés à l'état courant.
    let db = db();
    let mut game = setup_game(&db, 2, &mut RandomPolicy);
    let moss = card_id(&db, "Moss"); // OceanMin(3) + SpendPlants(1)
    game.oceans_revealed = 3;
    game.snapshot_planet();
    game.players[0].plants = 0;
    assert!(!requirements_met(&game, &db, 0, moss), "0 plante : dépense impossible");
    // La plante arrive APRÈS l'instantané : elle compte quand même.
    game.players[0].plants = 1;
    assert!(requirements_met(&game, &db, 0, moss), "prérequis de dépense = état courant");
}

#[test]
fn c1_second_build_cannot_use_a_parameter_raised_during_the_same_phase() {
    // Flux RÉEL, phase II, bonus SecondBuild : Lava Flows (+2 températures)
    // fait passer la température de 4 à 6 ; Arctic Algae exige le palier rouge
    // (6). L'ancien moteur la proposait à la 2e pose, le livret l'interdit.
    let db = db();
    let mut pol = Script::new(vec![2], vec![4]);
    pol.bonus = ConstructionBonus::SecondBuild;
    pol.build_first = [true, false];
    let mut game = setup_game(&db, 3, &mut pol);
    let lava = card_id(&db, "Lava Flows");
    let algae = card_id(&db, "Arctic Algae");
    set_hand(&mut game, 0, &[lava, algae]);
    game.players[0].mc = 200;
    game.temperature = 4;

    play_round(&mut game, &db, &mut pol);

    assert!(game.players[0].played.contains(&lava), "Lava Flows posée");
    assert_eq!(game.temperature, 6, "la température est bien montée au palier rouge");
    assert!(
        !game.players[0].played.contains(&algae),
        "Arctic Algae reste injouable : son prérequis n'était pas rempli au début de la phase"
    );
    assert!(
        game.players[0].hand.contains(&algae),
        "elle est toujours en main"
    );
    assert_eq!(
        game.prereq_snapshot_blocks, 1,
        "exactement une exclusion par l'instantané, comptée là où elle a lieu"
    );
}

#[test]
fn c1_snapshot_counter_stays_zero_when_nothing_is_blocked() {
    // Même mise en place SANS hausse de paramètre : aucun blocage à compter
    // (le compteur ne monte pas « tout seul »).
    let db = db();
    let mut pol = Script::new(vec![2], vec![4]);
    pol.bonus = ConstructionBonus::SecondBuild;
    pol.build_first = [true, false];
    let mut game = setup_game(&db, 3, &mut pol);
    let convoy = card_id(&db, "Convoy from Europa"); // rouge, aucun prérequis
    set_hand(&mut game, 0, &[convoy]);
    game.players[0].mc = 200;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.prereq_snapshot_blocks, 0);
}

#[test]
fn c1_counter_is_zero_with_effects_off() {
    // Sans couche d'effets il n'y a aucun prérequis : rien ne peut être bloqué.
    let mut db = db();
    db.effects_on = false;
    let out = play_game(&db, 4242, &mut RandomPolicy);
    assert_eq!(out.prereq_snapshot_blocks, 0);
}

// ===================================================================== C2
// Bonus de phase Construction : piocher AVANT ou APRÈS la pose (livret l.336).

#[test]
fn c2_construction_bonus_offers_the_three_choices_of_the_rulebook() {
    // Inversé : l'énumération n'avait que 2 variantes, la pioche « avant »
    // n'existait pas.
    let mut rng = <StdRng as rand::SeedableRng>::seed_from_u64(7);
    let mut pol = RandomPolicy;
    let mut seen = [false; 3];
    for _ in 0..200 {
        match pol.construction_bonus(&mut rng, 0) {
            ConstructionBonus::DrawCardBefore => seen[0] = true,
            ConstructionBonus::DrawCard => seen[1] = true,
            ConstructionBonus::SecondBuild => seen[2] = true,
        }
    }
    assert_eq!(seen, [true, true, true], "les 3 moments/options sont empruntés");
}

#[test]
fn c2_card_drawn_before_the_build_can_be_played_right_away() {
    // Inversé : avant C2 la carte piochée par le bonus arrivait APRÈS la pose
    // et ne pouvait jamais servir à la pose de la phase.
    let db = db();
    let mut pol = Script::new(vec![2], vec![4]);
    pol.bonus = ConstructionBonus::DrawCardBefore;
    pol.build_first = [true, false];
    let mut game = setup_game(&db, 5, &mut pol);
    let target = card_id(&db, "Subterranean Reservoir"); // rouge, 10 MC
    set_hand(&mut game, 0, &[]); // main vide : rien à poser sans la pioche
    game.deck.retain(|&c| c != target);
    game.deck.push(target); // dessus de pioche
    game.players[0].mc = 200;

    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.draw_before_build, 1, "une pioche AVANT la pose");
    assert_eq!(game.draw_after_build, 0);
    assert!(
        game.players[0].played.contains(&target),
        "la carte piochée avant a été posée dans la foulée"
    );
}

#[test]
fn c2_draw_after_the_build_keeps_its_lot2_behaviour() {
    let db = db();
    let mut pol = Script::new(vec![2], vec![4]);
    pol.bonus = ConstructionBonus::DrawCard;
    pol.build_first = [true, false];
    let mut game = setup_game(&db, 5, &mut pol);
    let target = card_id(&db, "Subterranean Reservoir");
    set_hand(&mut game, 0, &[]);
    game.deck.retain(|&c| c != target);
    game.deck.push(target);
    game.players[0].mc = 200;

    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.draw_after_build, 1, "une pioche APRÈS la pose");
    assert_eq!(game.draw_before_build, 0);
    assert!(
        !game.players[0].played.contains(&target),
        "piochée après la pose : elle reste en main cette phase"
    );
    assert!(game.players[0].hand.contains(&target));
}

#[test]
fn c2_no_draw_counter_moves_without_the_selector_bonus() {
    // Les compteurs sont attachés au bonus du choisissant, pas à une pioche
    // quelconque : un joueur qui ne choisit pas la phase II n'en produit aucun.
    let db = db();
    let mut pol = Script::new(vec![1], vec![4]);
    pol.bonus = ConstructionBonus::DrawCardBefore;
    let mut game = setup_game(&db, 6, &mut pol);
    play_round(&mut game, &db, &mut pol);
    assert_eq!((game.draw_before_build, game.draw_after_build), (0, 0));
}

#[test]
fn c2_both_draw_moments_are_used_in_real_simulations_effects_on_and_off() {
    // Les deux moments sont réellement empruntés par la boucle de jeu, ON comme
    // OFF (c'est une RÈGLE, pas un effet de carte).
    for effects_on in [true, false] {
        let mut db = db();
        db.effects_on = effects_on;
        let s = run_simulation(&db, 200, 42, &mut RandomPolicy);
        assert!(s.draw_before_build > 0, "pioche avant (effets={effects_on})");
        assert!(s.draw_after_build > 0, "pioche après (effets={effects_on})");
    }
}

// ===================================================================== C3
// Paiement d'une carte par défausse de cartes : 3 MC/carte, surplus rendu
// (livret p.13, l.348 — exemple de la carte à 8 MC payée par 3 cartes).

#[test]
fn c3_default_policy_discards_the_minimum_number_of_cards() {
    let mut rng = <StdRng as rand::SeedableRng>::seed_from_u64(1);
    let mut pol = RandomPolicy;
    let hand = vec![0u16; 10];
    // (lot cartes-7) La méthode reçoit désormais le TAUX de défausse du joueur,
    // rendu par `flow::discard_mc_rate` : ici le taux du livret, `SELL_CARD_MC`.
    // Les quatre attentes sont inchangées — c'est bien la même règle.
    let r = SELL_CARD_MC;
    assert_eq!(pol.discard_payment_count(&mut rng, 0, 10, 8, &hand, r), 0, "les MC suffisent");
    assert_eq!(pol.discard_payment_count(&mut rng, 0, 2, 8, &hand, r), 2, "6 manquants → 2 cartes");
    assert_eq!(pol.discard_payment_count(&mut rng, 0, 0, 8, &hand, r), 3, "8 manquants → 3 cartes");
    // Jamais plus de cartes que la main n'en contient.
    assert_eq!(pol.discard_payment_count(&mut rng, 0, 0, 90, &hand, r), 10);
}

#[test]
fn c3_build_pays_with_cards_and_gives_the_surplus_back() {
    // Exemple du livret : carte à 8 MC, 0 MC en poche, 3 cartes défaussées
    // (9 MC) → 1 MC rendu. Inversé : l'ancien `build_card` cassait sur son
    // assert « construction sans les MC requis ».
    let db = db();
    // (corpo-1) Graine 7 → 11 : depuis que les corporations ont leurs effets, la
    // corporation tirée peut RÉDUIRE le prix de la carte, et l'exemple du livret
    // (8 MC → 3 cartes) ne s'appliquerait plus. Graine choisie pour que la
    // corporation du joueur 0 n'accorde aucune réduction sur cette carte —
    // hypothèse ASSERTÉE ci-dessous plutôt que supposée.
    let mut game = setup_game(&db, 11, &mut RandomPolicy);
    let target = card_id(&db, "Geothermal Power"); // verte, 8 MC, sans prérequis
    let fillers: Vec<u16> = game
        .deck
        .iter()
        .copied()
        .filter(|&c| c != target)
        .take(4)
        .collect();
    let mut hand = vec![target];
    hand.extend_from_slice(&fillers);
    set_hand(&mut game, 0, &hand);
    game.players[0].mc = 0;
    assert_eq!(
        card_discount(&game, &db, 0, target),
        0,
        "aucune réduction en jeu : l'exemple du livret porte sur 8 MC pleins"
    );
    let discard_before = game.discard.len();

    let discarded = build_card(&mut game, &db, 0, 0, 0);

    assert_eq!(discarded, 3, "3 cartes défaussées (9 MC pour 8)");
    assert_eq!(game.players[0].mc, 1, "surplus rendu : 9 − 8 = 1 MC");
    assert_eq!(game.discard_payments, 3, "compteur alimenté à l'endroit du paiement");
    assert_eq!(game.discard.len(), discard_before + 3);
    assert!(game.players[0].played.contains(&target));
    assert!(
        !game.discard.contains(&target),
        "la carte posée n'est jamais dans les cartes défaussées pour la payer"
    );
    assert_eq!(game.players[0].hand.len(), 1, "4 cartes de monnaie − 3 défaussées");
}

#[test]
fn c3_mc_are_spent_first_and_no_card_is_discarded_when_they_suffice() {
    let db = db();
    let mut game = setup_game(&db, 8, &mut RandomPolicy);
    let target = card_id(&db, "Geothermal Power");
    let fillers: Vec<u16> = game.deck.iter().copied().filter(|&c| c != target).take(4).collect();
    let mut hand = vec![target];
    hand.extend_from_slice(&fillers);
    set_hand(&mut game, 0, &hand);
    game.players[0].mc = 8;

    let discarded = build_card(&mut game, &db, 0, 0, 0);

    assert_eq!(discarded, 0, "on paie d'abord avec les MC");
    assert_eq!(game.players[0].mc, 0);
    assert_eq!(game.discard_payments, 0);
    assert_eq!(game.players[0].hand.len(), 4, "la monnaie reste en main");
}

#[test]
fn c3_affordability_and_payment_live_in_the_same_path() {
    // Une carte payable UNIQUEMENT par défausse doit être PROPOSÉE (affordable)
    // puis réellement POSÉE (build_card). Inversé : avant C3 elle n'était pas
    // proposée du tout. Le second cas (main réduite à la carte elle-même)
    // montre que la borne est bien `3 × (cartes en main − 1)`.
    let db = db();
    let target = card_id(&db, "Geothermal Power"); // 8 MC
    for (with_currency, expect) in [(false, false), (true, true)] {
        // C'est le joueur 1 qui choisit la phase I : le joueur 0 y joue sans la
        // remise du sélectionneur, le coût reste 8 MC pleins. Le joueur 0 prend
        // la recherche (phase V) : aucune entrée de MC ne brouille le solde.
        let mut pol = Script::new(vec![5], vec![1]);
        pol.build_first = [true, false];
        let mut game = setup_game(&db, 9, &mut pol);
        let mut hand = vec![target];
        if with_currency {
            let fillers: Vec<u16> = game
                .deck
                .iter()
                .copied()
                .filter(|&c| c != target)
                .take(2)
                .collect();
            hand.extend_from_slice(&fillers);
        }
        set_hand(&mut game, 0, &hand);
        game.players[0].mc = 2; // 2 MC seuls : insuffisant pour 8

        play_round(&mut game, &db, &mut pol);

        assert_eq!(
            game.players[0].played.contains(&target),
            expect,
            "monnaie de défausse en main = {with_currency}"
        );
        if expect {
            assert_eq!(game.discard_payments, 2, "6 MC manquants → 2 cartes");
            assert_eq!(game.players[0].mc, 0, "2 + 6 − 8 = 0");
        } else {
            assert_eq!(game.discard_payments, 0);
        }
    }
}

#[test]
fn c3_discard_payment_is_a_rule_and_works_with_effects_off() {
    for effects_on in [true, false] {
        let mut db = db();
        db.effects_on = effects_on;
        let s = run_simulation(&db, 200, 42, &mut RandomPolicy);
        assert!(
            s.discard_payments > 0,
            "défausse-paiement active (effets={effects_on})"
        );
    }
}

#[test]
fn c3_policy_hook_drives_the_number_of_discarded_cards() {
    // Le NOMBRE vient bien de la politique : une politique qui défausse une
    // carte de plus que le minimum rend un surplus plus grand — même chemin,
    // même compteur.
    struct Generous(RandomPolicy);
    impl Policy for Generous {
        fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
            self.0.corp_mulligan(r, p, c)
        }
        fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
            self.0.project_mulligan(r, p, h)
        }
        fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
            self.0.pick_corporation(r, p, c)
        }
        fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
            self.0.pick_phase(r, p, a)
        }
        fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            self.0.choose_build(r, p, a)
        }
        fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
            self.0.construction_bonus(r, p)
        }
        fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            self.0.action_choice(r, p, o)
        }
        fn discard_payment_count(
            &mut self,
            r: &mut StdRng,
            p: usize,
            mc: i64,
            cost: i64,
            hand: &[u16],
            rate: i64,
        ) -> usize {
            (self.0.discard_payment_count(r, p, mc, cost, hand, rate) + 1).min(hand.len())
        }
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            self.0.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            self.0.discard_down(r, p, h, n)
        }
    }
    let db = db();
    // (corpo-1) Graine 10 → 12, même raison qu'en C3 ci-dessus : la corporation
    // du joueur 0 ne doit accorder aucune réduction sur cette carte, sinon le
    // « minimum + 1 » de la politique ne vaudrait plus 4 cartes. Assertée.
    let mut game = setup_game(&db, 12, &mut RandomPolicy);
    let target = card_id(&db, "Geothermal Power");
    let fillers: Vec<u16> = game.deck.iter().copied().filter(|&c| c != target).take(4).collect();
    let mut hand = vec![target];
    hand.extend_from_slice(&fillers);
    set_hand(&mut game, 0, &hand);
    game.players[0].mc = 0;
    assert_eq!(card_discount(&game, &db, 0, target), 0, "8 MC pleins");

    let mut pol = Generous(RandomPolicy);
    let discarded = build_card_with(&mut game, &db, 0, 0, 0, &mut pol);

    assert_eq!(discarded, 4, "minimum 3 + 1 imposé par la politique");
    assert_eq!(game.players[0].mc, 4, "12 MC encaissés − 8 payés");
    assert_eq!(game.discard_payments, 4);
}

// ===================================================================== C4
// Ordre du tour J1/J2 alterné (règle maison).

#[test]
fn c4_first_player_alternates_every_round() {
    // Inversé : avant C4 le joueur 0 commençait toutes les manches.
    let db = db();
    let mut pol = Script::new(vec![4, 5, 4, 5], vec![5, 4, 5, 4]);
    let mut game = setup_game(&db, 11, &mut pol);
    assert_eq!(game.first_player, 0, "manche 1 : joueur 0");
    for _ in 0..4 {
        play_round(&mut game, &db, &mut pol);
    }
    assert_eq!(game.turn_order, vec![0, 1, 0, 1]);
    assert_eq!(game.turn_order_switches(), 3, "manches − 1");
}

#[test]
fn c4_phases_one_and_two_follow_the_turn_order() {
    // Manche 2 : le joueur 1 commence — c'est lui qui reçoit la première offre
    // de construction. Inversé : l'ordre était toujours 0 puis 1.
    let db = db();
    let mut pol = Script::new(vec![4, 1], vec![5, 1]);
    let mut game = setup_game(&db, 12, &mut pol);
    play_round(&mut game, &db, &mut pol); // manche 1 (phases 4 et 5)
    pol.build_calls.clear();
    play_round(&mut game, &db, &mut pol); // manche 2 : phase I, premier joueur = 1
    let order: Vec<usize> = pol.build_calls.iter().map(|(p, _)| *p).collect();
    assert_eq!(order, vec![1, 0], "la phase I suit l'ordre du tour de la manche");
}

#[test]
fn c4_action_phase_alternates_action_by_action() {
    // Inversé : le joueur 0 faisait TOUTES ses actions, puis le joueur 1.
    let db = db();
    let mut pol = Script::new(vec![3], vec![4]);
    pol.sell_budget = [3, 3];
    let mut game = setup_game(&db, 13, &mut pol);
    play_round(&mut game, &db, &mut pol);
    // 3 actions chacun puis un « stop » chacun : appels strictement alternés.
    assert_eq!(pol.action_calls, vec![0, 1, 0, 1, 0, 1, 0, 1]);
}

#[test]
fn c4_a_player_who_passes_leaves_the_turn() {
    // Le joueur 1 passe tout de suite : il n'est plus sollicité, le joueur 0
    // continue seul jusqu'à ce qu'il passe à son tour.
    let db = db();
    let mut pol = Script::new(vec![3], vec![4]);
    pol.sell_budget = [3, 0];
    let mut game = setup_game(&db, 14, &mut pol);
    play_round(&mut game, &db, &mut pol);
    assert_eq!(pol.action_calls, vec![0, 1, 0, 0, 0]);
}

#[test]
fn c4_turn_order_is_read_from_the_game_loop_and_matches_the_rounds_played() {
    // La liste vient de l'état réel de la partie : autant d'entrées que de
    // manches jouées, alternance stricte à partir du joueur 0, et
    // `turn_order_switches` = manches − 1.
    let db = db();
    for seed in [7u64, 21, 99] {
        let out = play_game(&db, seed, &mut RandomPolicy);
        assert!(out.completed);
        assert_eq!(
            out.turn_order.len(),
            out.generations as usize,
            "une entrée par manche jouée (graine {seed})"
        );
        assert_eq!(out.turn_order[0], 0, "manche 1 : joueur 0");
        for w in out.turn_order.windows(2) {
            assert_ne!(w[0], w[1], "alternance stricte");
        }
        assert_eq!(out.turn_order_switches, out.turn_order.len() as u64 - 1);
    }
}

// ===================================================================== C5
// Égalité sèche (aucun départage) + conversion obligatoire sur l'instantané.

#[test]
fn c5_equal_scores_stay_equal_no_tiebreak() {
    // Règle maison : une égalité reste une égalité. `score` ne départage pas,
    // même quand chaleur/MC/plantes diffèrent nettement (critère du livret p.17
    // volontairement NON implémenté).
    let db = db();
    let mut game = setup_game(&db, 15, &mut RandomPolicy);
    game.milestones.iter_mut().for_each(|m| m.achieved_by = [false, false]);
    game.awards = [AwardKind::Collector; 3]; // 4/4 : neutre
    game.players[0].forests = 3;
    game.players[1].forests = 3;
    game.players[0].heat = 40;
    game.players[0].mc = 40;
    game.players[0].plants = 40;
    game.players[1].heat = 0;
    game.players[1].mc = 0;
    game.players[1].plants = 0;
    let s = score(&game, &db);
    assert_eq!(s[0], s[1], "scores égaux : aucun départage n'est appliqué");
}

#[test]
fn c5_draws_are_counted_over_real_simulations() {
    let db = db();
    let s = run_simulation(&db, 1000, 42, &mut RandomPolicy);
    assert!(s.draws > 0, "des égalités existent et sont comptées");
    assert!(s.draws < s.games, "et ce ne sont pas toutes les parties");
}

#[test]
fn c5_forced_conversion_is_judged_on_the_phase_snapshot() {
    // Inversé : l'ancienne boucle testait `game.oxygen < OXYGEN_MAX` (valeur
    // COURANTE) et s'arrêtait dès la 1re forêt ; l'instantané de début de phase
    // (13 < 14) autorise la phase à aller au bout.
    let db = db();
    let mut pol = Script::new(vec![3], vec![4]);
    let mut game = setup_game(&db, 16, &mut pol);
    game.oxygen = OXYGEN_MAX - 1;
    game.players[0].plants = 17;
    game.players[0].heat = 0;
    game.players[1].plants = 0;
    game.players[1].heat = 0;

    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.oxygen, OXYGEN_MAX);
    assert_eq!(game.players[0].forests, 2, "17 plantes → 2 forêts (ancien moteur : 1)");
    assert_eq!(game.players[0].plants, 1);
}

#[test]
fn c5_forced_conversion_still_stops_when_the_snapshot_is_already_maxed() {
    // Le paramètre est au max DÈS le début de la phase : l'obligation tombe
    // (livret p.14 « sauf si le paramètre a déjà atteint son maximum »).
    let db = db();
    let mut pol = Script::new(vec![3], vec![4]);
    let mut game = setup_game(&db, 17, &mut pol);
    game.oxygen = OXYGEN_MAX;
    game.temperature = TEMPERATURE_MAX;
    game.players[0].plants = 17;
    game.players[0].heat = 17;

    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.players[0].plants, 17);
    assert_eq!(game.players[0].heat, 17);
    assert_eq!(game.players[0].forests, 0);
}

// ================================================================== sonde
// Schéma étendu : `discarded`, `prereq_ok_now`, `--probe-mc/-filler/-strict`.

#[test]
fn probe_reports_discarded_cards_and_surplus() {
    // Cas imposé : Comet (25 MC) sans un sou, 9 cartes de monnaie → 27 MC
    // encaissés, 2 rendus.
    let db = db();
    let r = run_probe_seq_opts(
        &db,
        &["Comet"],
        ProbeOptions { mc: 0, filler: 9, strict: false, phase: 0, plants: 20, upgrades: [None; 5], objectif: None },
    );
    assert!(r.played);
    assert_eq!(r.paid, vec![25]);
    assert_eq!(r.discarded, vec![9]);
    // delta.mc = MC après − avant + prix payé = 27 = la valeur des 9 cartes.
    assert_eq!(r.delta.mc, 27, "9 × 3 MC encaissés, dont 2 restent au joueur");
}

#[test]
fn probe_does_not_discard_when_mc_are_enough() {
    let db = db();
    let r = run_probe_seq_opts(
        &db,
        &["Lichen"],
        ProbeOptions { mc: 5, filler: 5, strict: false, phase: 0, plants: 20, upgrades: [None; 5], objectif: None },
    );
    assert!(r.played);
    assert_eq!(r.paid, vec![5]);
    assert_eq!(r.discarded, vec![0]);
    assert_eq!(r.delta.hand, -1, "seule la carte posée quitte la main");
}

#[test]
fn probe_without_options_is_identical_to_lot2() {
    let db = db();
    let a = run_probe_seq(&db, &["Media Group", "Lichen"]);
    let b = run_probe_seq_opts(&db, &["Media Group", "Lichen"], ProbeOptions::default());
    assert_eq!(a.paid, vec![11, 5], "Lichen n'est pas un événement : pas de réduction");
    assert_eq!(a.discarded, vec![0, 0], "aucune défausse hors option");
    assert_eq!(a.delta, b.delta);
    assert_eq!(a.delta.hand, 0, "convention lot 1/2 conservée");
}

#[test]
fn probe_prereq_ok_now_shows_what_the_snapshot_blocks() {
    // Ice Asteroid retourne 2 océans, puis Great Dam en exige 2 : au DÉPART
    // (= instantané) le prérequis est faux, à l'état COURANT il est vrai.
    let db = db();
    let r = run_probe_seq(&db, &["Ice Asteroid", "Great Dam"]);
    assert!(!r.prereq_ok, "prérequis faux sur l'état de départ (règle du jeu)");
    assert!(r.prereq_ok_now, "prérequis vrai à l'état courant, juste avant la pose");
    assert!(r.played, "sans --probe-strict la pose reste forcée (lot 2 inchangé)");
}

#[test]
fn probe_strict_applies_the_real_rule_card_by_card() {
    let db = db();
    let strict = ProbeOptions { strict: true, ..ProbeOptions::default() };
    // Great Dam exige 2 océans révélés : aucun au départ.
    let dam = run_probe_seq_opts(&db, &["Great Dam"], strict);
    assert!(!dam.played);
    assert!(!dam.prereq_ok);
    assert!(!dam.prereq_ok_now);
    assert!(dam.paid.is_empty(), "rien n'a été payé");
    // Lichen n'a aucun prérequis.
    let lichen = run_probe_seq_opts(&db, &["Lichen"], strict);
    assert!(lichen.played);
    // Et la séquence s'arrête à la carte refusée, même si l'état courant
    // l'autoriserait : c'est la règle C1 (instantané = état de départ).
    let seq = run_probe_seq_opts(&db, &["Ice Asteroid", "Great Dam"], strict);
    assert!(!seq.played, "Great Dam refusée : 0 océan au début de la phase");
    assert_eq!(seq.paid.len(), 1, "seule Ice Asteroid a été posée");
}

#[test]
fn probe_strict_refuses_an_unpayable_card() {
    // Payabilité réelle (MC + défausse), pas une règle réécrite pour la sonde.
    let db = db();
    let poor = ProbeOptions { mc: 0, filler: 0, strict: true, phase: 0, plants: 20, upgrades: [None; 5], objectif: None };
    let r = run_probe_seq_opts(&db, &["Comet"], poor);
    assert!(!r.played, "0 MC, aucune monnaie de défausse : Comet est impayable");
    let rich = ProbeOptions { mc: 0, filler: 9, strict: true, phase: 0, plants: 20, upgrades: [None; 5], objectif: None };
    assert!(run_probe_seq_opts(&db, &["Comet"], rich).played);
}

// =========================================================== transversal

#[test]
fn counters_are_deterministic_and_summary_matches_the_games() {
    let db = db();
    let a = run_simulation(&db, 50, 42, &mut RandomPolicy);
    let b = run_simulation(&db, 50, 42, &mut RandomPolicy);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.prereq_snapshot_blocks, b.prereq_snapshot_blocks);
    assert_eq!(a.discard_payments, b.discard_payments);
    assert_eq!(a.draw_before_build, b.draw_before_build);
    assert_eq!(a.draw_after_build, b.draw_after_build);
    assert_eq!(a.draws, b.draws);
    // Le résumé agrège exactement les parties (aucun compteur « de résumé »).
    let total: u64 = a.turn_orders.iter().map(|o| o.len() as u64 - 1).sum();
    assert_eq!(a.turn_order_switches, total);
    assert_eq!(a.turn_orders.len(), 50);
}

#[test]
fn effects_off_still_terminates_without_invariant_violation() {
    let mut db = db();
    db.effects_on = false;
    let s = run_simulation(&db, 100, 42, &mut RandomPolicy);
    assert_eq!(s.completed, 100);
    assert_eq!(s.invariant_violations, 0);
    assert_eq!(s.truncated, 0);
}

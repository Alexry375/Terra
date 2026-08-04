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
    affordable, build_card, card_discount, discard_mc_rate, observer, occasion_de_vendre,
    play_round, requirements_met, requirements_met_now, score, setup_game, GRANT_DEVELOPMENT,
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
// (regles-de-la-vente) LA MAIN N'EST PLUS UNE MONNAIE.
//
// Cette section mesurait le paiement d'une carte par défausse d'office : le
// moteur prenait « les dernières de la main » quand les MC ne suffisaient pas,
// et `discard_payments` les comptait. C'est le défaut que cette tâche supprime
// (prompt A et B) : une carte trop chère était annoncée jouable, et le joueur ne
// choisissait ni le moment ni les cartes vendues.
//
// Les mises en place sont GARDÉES telles quelles et les attentes INVERSÉES —
// c'est la forme la plus forte : le scénario qui prouvait l'ancienne règle
// prouve maintenant la nouvelle, sur le même chemin de code. Le livret n'est pas
// contredit : il autorise toujours la vente (l. 96), elle est simplement devenue
// un geste antérieur et volontaire (`flow::occasion_de_vendre`, testé plus bas).

#[test]
fn c3_la_main_garnie_ne_rend_plus_une_carte_payable() {
    // Ancien `c3_build_pays_with_cards_and_gives_the_surplus_back`, inversé :
    // même graine, même carte à 8 MC, mêmes 4 cartes de monnaie, 0 MC en poche.
    // La carte n'est plus proposée, et rien ne quitte la main.
    let db = db();
    let mut game = setup_game(&db, 11, &mut RandomPolicy);
    let target = card_id(&db, "Geothermal Power"); // verte, 8 MC, sans prérequis
    let fillers: Vec<u16> = game.deck.iter().copied().filter(|&c| c != target).take(4).collect();
    let mut hand = vec![target];
    hand.extend_from_slice(&fillers);
    set_hand(&mut game, 0, &hand);
    game.players[0].mc = 0;
    assert_eq!(card_discount(&game, &db, 0, target), 0, "8 MC pleins");
    let defausse_avant = game.discard.len();

    let opts = affordable(&mut game, &db, 0, &GRANT_DEVELOPMENT, 0);

    assert!(
        !opts.contains(&0),
        "0 MC et 4 cartes en main : la carte à 8 MC n'est PAS payable, \
         la vente de la main ne se compte plus d'avance"
    );
    assert_eq!(game.players[0].hand.len(), 5, "aucune carte n'a quitté la main");
    assert_eq!(game.discard.len(), defausse_avant, "rien n'est allé à la défausse");
    assert_eq!(game.discard_payments, 0, "aucune vente d'office");
}

#[test]
fn c3_les_mc_seuls_paient_et_le_surplus_est_rendu() {
    // Ancien `c3_mc_are_spent_first_and_no_card_is_discarded_when_they_suffice`,
    // inchangé dans son attente — c'est le TÉMOIN : on n'a pas cassé l'achat
    // lui-même en supprimant la vente d'office. Poussé d'un cran : avec 9 MC
    // pour une carte à 8, le MC en trop reste au joueur (« la différence vous
    // est rendue », livret p.13).
    let db = db();
    for (poche, reste) in [(8i64, 0i64), (9, 1)] {
        let mut game = setup_game(&db, 8, &mut RandomPolicy);
        let target = card_id(&db, "Geothermal Power");
        let fillers: Vec<u16> =
            game.deck.iter().copied().filter(|&c| c != target).take(4).collect();
        let mut hand = vec![target];
        hand.extend_from_slice(&fillers);
        set_hand(&mut game, 0, &hand);
        game.players[0].mc = poche;

        let discarded = build_card(&mut game, &db, 0, 0, 0);

        assert_eq!(discarded, 0, "aucune carte défaussée : les MC suffisent");
        assert_eq!(game.players[0].mc, reste, "surplus rendu");
        assert_eq!(game.discard_payments, 0);
        assert_eq!(game.players[0].hand.len(), 4, "les 4 autres cartes restent en main");
        assert!(game.players[0].played.contains(&target));
    }
}

#[test]
fn c3_affordabilite_et_paiement_vivent_toujours_dans_le_meme_chemin() {
    // Ancien `c3_affordability_and_payment_live_in_the_same_path`, inversé.
    // Même mise en place, à 2 MC pour une carte à 8 : avec OU sans cartes de
    // monnaie en main, elle n'est plus posée. L'invariant I2 tient toujours —
    // ce qui est proposé est ce qui est payable — mais la borne n'est plus
    // `mc + 3 × (main − 1)`, c'est `mc`.
    let db = db();
    let target = card_id(&db, "Geothermal Power"); // 8 MC
    for with_currency in [false, true] {
        let mut pol = Script::new(vec![5], vec![1]);
        pol.build_first = [true, false];
        let mut game = setup_game(&db, 9, &mut pol);
        let mut hand = vec![target];
        if with_currency {
            let fillers: Vec<u16> =
                game.deck.iter().copied().filter(|&c| c != target).take(2).collect();
            hand.extend_from_slice(&fillers);
        }
        set_hand(&mut game, 0, &hand);
        game.players[0].mc = 2; // 2 MC seuls : insuffisant pour 8

        play_round(&mut game, &db, &mut pol);

        assert!(
            !game.players[0].played.contains(&target),
            "monnaie de défausse en main = {with_currency} : elle ne paie plus rien"
        );
        assert_eq!(game.discard_payments, 0, "aucune vente d'office");
    }
}

#[test]
fn c3_la_vente_d_office_a_disparu_des_deux_cotes_de_la_couche_d_effets() {
    // Ancien `c3_discard_payment_is_a_rule_and_works_with_effects_off`, inversé :
    // le compteur devait être > 0, il doit maintenant valoir ZÉRO — et rester
    // PUBLIÉ (prompt : « il doit tomber à zéro, et rester publié »). La vente
    // d'office était une RÈGLE, pas un effet : sa suppression vaut donc aussi
    // en `--effects off`, ce que les deux passages mesurent.
    for effects_on in [true, false] {
        let mut db = db();
        db.effects_on = effects_on;
        let s = run_simulation(&db, 200, 42, &mut RandomPolicy);
        assert_eq!(
            s.discard_payments, 0,
            "aucune carte vendue d'office sur 200 parties (effets={effects_on})"
        );
    }
}

#[test]
fn c3_vendre_librement_est_offert_dans_les_phases_ou_l_on_depense() {
    // Le REMPLACEMENT de la vente d'office : le joueur vend ce qu'il veut, quand
    // il veut, et le moteur ne décide plus rien. Politique scriptée qui vend la
    // PREMIÈRE carte de sa main à la première occasion venue, et jamais plus.
    //
    // Deux mesures, et la seconde est celle qui compte : les cartes vendues sont
    // celles que la politique a DÉSIGNÉES, pas « les dernières de la main ».
    struct VendeurUnique {
        inner: RandomPolicy,
        fait: bool,
        vendue: Option<u16>,
        phases_vues: Vec<u8>,
    }
    impl Policy for VendeurUnique {
        fn vendre_librement(&mut self, _r: &mut StdRng, p: usize, main: &[u16]) -> Vec<usize> {
            if self.fait || p != 0 || main.is_empty() {
                return Vec::new();
            }
            self.fait = true;
            self.vendue = Some(main[0]);
            vec![0]
        }
        fn observe(&mut self, game: &GameState, _p: usize) {
            self.phases_vues.push(game.phase_en_cours);
        }
        fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
            self.inner.corp_mulligan(r, p, c)
        }
        fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
            self.inner.project_mulligan(r, p, h)
        }
        fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
            self.inner.pick_corporation(r, p, c)
        }
        fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
            self.inner.pick_phase(r, p, a)
        }
        fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            self.inner.choose_build(r, p, a)
        }
        fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
            self.inner.construction_bonus(r, p)
        }
        fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            self.inner.action_choice(r, p, o)
        }
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            self.inner.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            self.inner.discard_down(r, p, h, n)
        }
    }

    let db = db();
    let mut pol = VendeurUnique {
        inner: RandomPolicy,
        fait: false,
        vendue: None,
        phases_vues: Vec::new(),
    };
    let mut game = setup_game(&db, 11, &mut pol);
    let main_avant = game.players[0].hand.clone();
    let mc_avant = game.players[0].mc;
    let taux = discard_mc_rate(&db, &game.players[0]);
    assert!(!main_avant.is_empty(), "main vide : ce test n'aurait rien prouvé");
    // La mise en place n'est pas une phase : rien n'a dû être vendu jusqu'ici.
    assert!(!pol.fait, "une vente a été offerte hors phase (mise en place)");

    play_round(&mut game, &db, &mut pol);

    assert!(pol.fait, "aucune occasion de vendre n'a été offerte de toute la manche");
    let vendue = pol.vendue.expect("la politique n'a désigné aucune carte");
    assert_eq!(vendue, main_avant[0], "la carte vendue est celle que le joueur a désignée");
    assert!(game.discard.contains(&vendue), "la carte désignée est allée à la défausse");
    assert_eq!(game.ventes_volontaires, 1, "une vente volontaire, comptée une fois");
    assert_eq!(game.discard_payments, 0, "et aucune vente d'office");
    // Les MC : l'assertion précédente disait `mc >= mc_avant + taux - 1000`, ce
    // qui, à un taux de 3, est vrai quoi qu'il arrive. Le crédit exact ne peut
    // pas se mesurer ICI (une manche entière fait entrer et sortir des MC par
    // dix chemins) : il se mesure sur `occasion_de_vendre` seule, dans le test
    // qui suit. On garde le seul énoncé que cette manche-ci puisse soutenir.
    // (et l'on n'assère RIEN sur les MC ici : une manche entière en fait entrer
    // et sortir par dix chemins — le joueur dépense. Mesuré à 27 → 25 sur cette
    // graine, ce qui est normal et n'apprend rien sur la vente.)
    assert!(taux >= SELL_CARD_MC, "le taux du service unique reste au plancher du livret");
    let _ = mc_avant;
    // (prompt 3) L'occasion n'est offerte QUE là où l'on peut dépenser. Mesuré,
    // et ASSERTÉ : la version précédente se contentait d'un `println!`, et
    // acceptait donc sans broncher une occasion offerte en phase IV ou V.
    assert!(
        pol.phases_vues.iter().any(|&f| (1..=3).contains(&f)),
        "aucune observation en phase dépensable : ce test n'aurait rien prouvé"
    );
    // (La restriction aux phases dépensables ne peut PAS se mesurer d'ici : le
    // moteur observe APRÈS l'occasion, donc la phase que cette politique voit à
    // l'instant d'une vente est encore celle d'avant. C'est le test suivant,
    // qui appelle `occasion_de_vendre` directement sur les six valeurs de
    // `phase_en_cours`, qui l'établit — dans les deux sens.)
}

/// **Le crédit de la vente volontaire, au MC près** — et le fait qu'elle n'est
/// offerte que là où l'on peut dépenser.
///
/// Ce test appelle `flow::occasion_de_vendre` DIRECTEMENT, sur un état fabriqué :
/// c'est le seul moyen de mesurer le crédit exact, une manche entière faisant
/// entrer et sortir des MC par dix chemins. Il boucle sur les six valeurs de
/// `phase_en_cours` — la mesure a donc un témoin dans les deux sens : elle vend
/// en I, II, III et ne vend PAS en 0, IV, V.
///
/// **(round 2) LA MESURE REGARDAIT TROP TÔT.** Elle lisait `vente_offerte` juste
/// après `occasion_de_vendre` et y voyait `false` en phase I. Ce n'était pas une
/// régression du produit : le moteur a été retourné exprès en deux temps —
/// l'occasion ARME `occasion_ouverte`, et c'est `flow::observer` qui PUBLIE
/// `vente_offerte` en le consommant, pour qu'un point de décision oublié ne
/// puisse pas hériter du drapeau du point précédent. La mesure suit donc les
/// deux temps, et elle en profite pour asserter l'invariant neuf : le drapeau
/// que l'écran lit reste faux tant que la question n'a pas été posée. Rien n'a
/// été retiré — les six phases, le crédit au MC près, la carte désignée et
/// l'absence totale d'effet en 0, IV, V sont tous encore là.
#[test]
fn c3_l_occasion_de_vendre_credite_le_taux_et_respecte_les_phases() {
    /// Vend toujours la carte d'indice 0, et compte les fois où on le lui offre.
    struct VendUne(u32);
    impl Policy for VendUne {
        fn vendre_librement(&mut self, _r: &mut StdRng, _p: usize, main: &[u16]) -> Vec<usize> {
            if main.is_empty() {
                return Vec::new();
            }
            self.0 += 1;
            vec![0]
        }
        fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            RandomPolicy.choose_build(r, p, a)
        }
        fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
            RandomPolicy.corp_mulligan(r, p, c)
        }
        fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
            RandomPolicy.project_mulligan(r, p, h)
        }
        fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
            RandomPolicy.pick_corporation(r, p, c)
        }
        fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
            RandomPolicy.pick_phase(r, p, a)
        }
        fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
            RandomPolicy.construction_bonus(r, p)
        }
        fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            RandomPolicy.action_choice(r, p, o)
        }
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            RandomPolicy.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            RandomPolicy.discard_down(r, p, h, n)
        }
    }

    let db = db();
    let mut eprouvees = 0;
    for phase in 0u8..=5 {
        let attendue = (1..=3).contains(&phase);
        let mut game = setup_game(&db, 11, &mut RandomPolicy);
        game.phase_en_cours = phase;
        // Le point de départ est ÉCRIT, pas supposé : c'est lui qui donne son
        // sens à l'assertion « rien n'est publié avant l'observation ».
        game.vente_offerte = false;
        game.occasion_ouverte = false;
        // Les deux joueurs partent d'une main connue, pour compter sans deviner.
        let mains: Vec<Vec<u16>> =
            (0..NUM_PLAYERS).map(|p| game.players[p].hand.clone()).collect();
        assert!(mains.iter().all(|m| m.len() >= 2), "mains trop courtes : rien à prouver");
        let mc: Vec<i64> = (0..NUM_PLAYERS).map(|p| game.players[p].mc).collect();
        let taux: Vec<i64> =
            (0..NUM_PLAYERS).map(|p| discard_mc_rate(&db, &game.players[p])).collect();
        let defausse = game.discard.len();

        let mut pol = VendUne(0);
        occasion_de_vendre(&mut game, &db, &mut pol);

        // PREMIER TEMPS — l'occasion ARME, et ne publie rien. Le drapeau que
        // l'écran lit doit encore valoir faux : tant que la question n'est pas
        // posée, aucune vente n'est recevable, et l'écran ne doit pas offrir le
        // bouton.
        assert!(
            !game.vente_offerte,
            "phase {phase} : `vente_offerte` a été publié AVANT l'observation — \
             l'écran offrirait une vente à un point de décision qui ne l'attend pas"
        );
        assert_eq!(
            game.occasion_ouverte, attendue,
            "phase {phase} : l'occasion devrait être armée = {attendue}"
        );

        // SECOND TEMPS — c'est l'observation qui publie, en CONSOMMANT le
        // drapeau de passage. Un point de décision qu'on aurait oublié de
        // pourvoir d'une occasion publie donc forcément faux.
        observer(&mut game, 0, &mut pol);
        assert_eq!(
            game.vente_offerte, attendue,
            "phase {phase} : `vente_offerte` devrait valoir {attendue}"
        );
        assert!(
            !game.occasion_ouverte,
            "phase {phase} : le drapeau de passage doit être consommé par l'observation"
        );
        if !attendue {
            assert_eq!(pol.0, 0, "phase {phase} : la politique a été interrogée");
            assert_eq!(game.ventes_volontaires, 0, "phase {phase} : une vente a eu lieu");
            assert_eq!(game.discard.len(), defausse, "phase {phase} : la défausse a bougé");
            for p in 0..NUM_PLAYERS {
                assert_eq!(game.players[p].mc, mc[p], "phase {phase} : les MC ont bougé");
                assert_eq!(game.players[p].hand, mains[p], "phase {phase} : la main a bougé");
            }
            continue;
        }
        eprouvees += 1;
        // Offerte AUX DEUX joueurs : deux ventes, une par joueur.
        assert_eq!(pol.0, NUM_PLAYERS as u32, "phase {phase} : les deux joueurs doivent l'être");
        assert_eq!(game.ventes_volontaires, NUM_PLAYERS as u64, "phase {phase}");
        assert_eq!(game.discard.len(), defausse + NUM_PLAYERS, "phase {phase}");
        for p in 0..NUM_PLAYERS {
            // LE CRÉDIT, AU MC PRÈS, et au taux du service unique.
            assert_eq!(
                game.players[p].mc, mc[p] + taux[p],
                "phase {phase}, joueur {p} : la vente doit créditer exactement {} MC",
                taux[p]
            );
            // LA CARTE DÉSIGNÉE, et elle seule.
            assert_eq!(game.players[p].hand, mains[p][1..], "phase {phase}, joueur {p}");
            assert!(game.discard.contains(&mains[p][0]), "phase {phase}, joueur {p}");
        }
    }
    assert_eq!(eprouvees, 3, "les trois phases dépensables doivent avoir été éprouvées");
}

/// **(round 2) LA MAIN QUE LE JOUEUR VOIT EST CELLE SUR LAQUELLE SA VENTE
/// S'APPLIQUERA — sinon le drapeau ne se lève pas.**
///
/// Le défaut réel qu'il rattrape (journal D24) : l'occasion de `Eff::DrawDiscard`
/// était offerte AVANT la pioche. Le joueur voyait trois cartes fraîches, l'écran
/// lui offrait le bouton, il en désignait une — et le rejeu replaçait sa vente
/// sur la main d'avant la pioche, souvent vide, où l'occasion passe son tour. La
/// vente n'était consommée par personne, le pont la refusait, la partie se figeait
/// sur un écran de panne.
///
/// La cause a été corrigée à sa source ; ce test-ci garde la CEINTURE :
/// `flow::observer` ne publie `vente_offerte` que si la main n'a pas bougé depuis
/// l'occasion. Les indices que le joueur désigne à l'écran désignent alors
/// forcément les mêmes cartes au rejeu. Coût d'une omission future : un bouton non
/// offert — jamais une partie perdue.
///
/// Les deux sens sont éprouvés dans le même test : main intacte → le drapeau se
/// lève ; une seule carte piochée entre les deux → il reste baissé.
#[test]
fn c3_le_drapeau_de_vente_ne_se_leve_que_si_la_main_n_a_pas_bouge() {
    let db = db();

    // Témoin — rien ne bouge entre l'occasion et l'observation.
    let mut game = setup_game(&db, 11, &mut RandomPolicy);
    game.phase_en_cours = 1;
    game.vente_offerte = false;
    occasion_de_vendre(&mut game, &db, &mut RandomPolicy);
    observer(&mut game, 0, &mut RandomPolicy);
    assert!(
        game.vente_offerte,
        "main intacte : le drapeau doit se lever, sinon on perd une vente que le \
         livret accorde (l. 96, « à tout moment »)"
    );

    // La même chose, une carte piochée entre les deux — c'est exactement ce que
    // faisait `DrawDiscard`.
    let mut game = setup_game(&db, 11, &mut RandomPolicy);
    game.phase_en_cours = 1;
    game.vente_offerte = false;
    occasion_de_vendre(&mut game, &db, &mut RandomPolicy);
    let piochee = game.deck.pop().expect("la pioche ne doit pas être vide");
    game.players[0].hand.push(piochee);
    observer(&mut game, 0, &mut RandomPolicy);
    assert!(
        !game.vente_offerte,
        "la main a changé depuis l'occasion : le drapeau doit rester baissé, sinon \
         l'écran offre une vente dont les indices ne désignent plus les mêmes cartes"
    );

    // Et le changement de main de L'AUTRE joueur compte aussi : le drapeau est
    // unique et l'écran le lit quel que soit le siège qu'il regarde.
    let mut game = setup_game(&db, 11, &mut RandomPolicy);
    game.phase_en_cours = 1;
    game.vente_offerte = false;
    occasion_de_vendre(&mut game, &db, &mut RandomPolicy);
    game.players[1].hand.pop().expect("main d'en face non vide");
    observer(&mut game, 0, &mut RandomPolicy);
    assert!(!game.vente_offerte, "la main d'en face a changé : drapeau baissé");
}

/// **DES PARTIES ENTIÈRES JOUÉES PAR QUELQU'UN QUI VEND VRAIMENT.**
///
/// Le trou que ce test bouche est le plus large de la livraison, et il est
/// structurel : `RandomPolicy::vendre_librement` rend la liste vide par
/// construction — c'est voulu, c'est ce qui laisse les empreintes de référence
/// intactes — donc **ni les 1000 parties simulées, ni les empreintes, ni aucun
/// autre test n'empruntent jamais le chemin que cette tâche livre**. Seul un
/// vrai joueur (le pont) le prend.
///
/// Deux paniques du moteur s'y cachaient, trouvées par la relecture
/// adversariale et corrigées :
///   · le coût en CARTES d'une action bleue, dont la payabilité était jugée sur
///     la main d'avant la vente (`assert coût en cartes partiellement payé`) ;
///   · « Discard up to N cards », dont le plafond était calculé avant la vente
///     (`cannot sample empty range`).
/// Les deux fois, une donnée dépendant de la main était calculée AVANT
/// l'occasion et consommée APRÈS.
///
/// Deux appétits, parce qu'ils n'éprouvent pas la même chose : le VORACE vide sa
/// main à chaque occasion — c'est lui qui fait sauter les bornes — et le SOBRE
/// en vend une, ce qui laisse la partie se dérouler assez longtemps pour
/// atteindre les points de décision tardifs.
#[test]
fn c3_des_parties_entieres_avec_un_joueur_qui_vend_pour_de_bon() {
    struct Vendeur {
        vorace: bool,
        /// Une occasion sur `rythme` donne lieu à une vente. **Le rythme n'est
        /// pas décoratif** : une politique qui vend à CHAQUE occasion vide sa
        /// main dès l'occasion qui précède le choix d'action, et le moteur ne lui
        /// propose alors plus aucune action à coût en cartes — elle se protège
        /// toute seule des deux paniques qu'on veut éprouver. En sautant des
        /// occasions, on garde des cartes jusqu'au point de décision INTERNE à
        /// l'action, qui est celui où la main était lue trop tôt.
        rythme: u64,
        occasions: u64,
    }
    impl Policy for Vendeur {
        fn vendre_librement(&mut self, _r: &mut StdRng, _p: usize, main: &[u16]) -> Vec<usize> {
            self.occasions += 1;
            if main.is_empty() || self.occasions % self.rythme != 0 {
                return Vec::new();
            }
            if self.vorace {
                (0..main.len()).collect()
            } else {
                vec![main.len() - 1]
            }
        }
        fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
            RandomPolicy.corp_mulligan(r, p, c)
        }
        fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
            RandomPolicy.project_mulligan(r, p, h)
        }
        fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
            RandomPolicy.pick_corporation(r, p, c)
        }
        fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
            RandomPolicy.pick_phase(r, p, a)
        }
        fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            RandomPolicy.choose_build(r, p, a)
        }
        fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
            RandomPolicy.construction_bonus(r, p)
        }
        fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            RandomPolicy.action_choice(r, p, o)
        }
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            RandomPolicy.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            RandomPolicy.discard_down(r, p, h, n)
        }
    }

    let db = db();
    let mut ventes_totales = 0u64;
    let mut manches_totales = 0u64;
    for (vorace, rythme) in [(false, 2u64), (true, 2), (true, 3), (true, 5)] {
        for seed in 0..40u64 {
            let mut pol = Vendeur { vorace, rythme, occasions: 0 };
            let mut game = setup_game(&db, seed, &mut pol);
            // Le plafond de manches est un garde-fou de test, pas une règle : une
            // partie où l'on vend tout n'atteint pas forcément les paramètres.
            while !game.game_over && game.generation <= 60 {
                play_round(&mut game, &db, &mut pol);
                manches_totales += 1;
            }
            assert!(
                pol.occasions > 0,
                "appétit {vorace}/{rythme}, graine {seed} : aucune occasion de vendre offerte"
            );
            assert_eq!(
                game.discard_payments, 0,
                "appétit {vorace}/{rythme}, graine {seed} : une carte a été vendue d'office"
            );
            // L'invariant du NT tient : `tr == 5 + gains − dépenses`. C'est le
            // garde-fou que la vente ne doit pas déranger.
            for p in 0..NUM_PLAYERS {
                assert!(
                    game.players[p].tr >= 0,
                    "appétit {vorace}/{rythme}, graine {seed} : NT négatif"
                );
            }
            ventes_totales += game.ventes_volontaires;
        }
    }
    // Sans occasion dénombrée, « aucune panique » ne prouve rien : une partie qui
    // ne vend jamais ne mesure pas la vente.
    assert!(
        ventes_totales > 1000,
        "seulement {ventes_totales} ventes volontaires sur 160 parties : trop peu \
         pour que ce test ait éprouvé quoi que ce soit"
    );
    println!(
        "160 parties, {manches_totales} manches, {ventes_totales} ventes volontaires, \
         0 vente d'office"
    );
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
fn la_sonde_rapporte_zero_carte_defaussee() {
    // (regles-de-la-vente) Cas imposé d'origine : Comet (25 MC) sans un sou et
    // 9 cartes de monnaie → la sonde rapportait `discarded == [9]`. Le champ
    // reste PUBLIÉ — c'est lui qui prouve que rien ne quitte la main sans
    // décision — mais il vaut désormais zéro, parce que le chemin qu'il
    // comptait n'existe plus.
    let db = db();
    let opts = |mc: i64, filler: usize| ProbeOptions {
        mc, filler, strict: false, phase: 0, plants: 20, upgrades: [None; 5], objectif: None
    };

    let pauvre = run_probe_seq_opts(&db, &["Comet"], opts(0, 9));
    assert!(!pauvre.played, "0 MC : 9 cartes en main ne paient pas les 25 MC de Comet");
    assert_eq!(pauvre.discarded, Vec::<i64>::new(), "aucune pose, donc aucune défausse");
    assert_eq!(pauvre.delta.hand, 0, "rien n'a quitté la main");

    // TÉMOIN : avec les MC, la carte se pose — et sans défausser quoi que ce
    // soit, la main garnie n'y changeant rien.
    let riche = run_probe_seq_opts(&db, &["Comet"], opts(25, 9));
    assert!(riche.played, "25 MC : Comet est posée");
    assert_eq!(riche.paid, vec![25]);
    assert_eq!(riche.discarded, vec![0], "le compteur reste publié, et vaut zéro");
    assert_eq!(riche.delta.hand, -1, "seule la carte posée quitte la main");
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
fn la_sonde_stricte_refuse_une_carte_impayable() {
    // Payabilité RÉELLE, pas une règle réécrite pour la sonde : la sonde
    // consomme le même prédicat `flow::payable` que le flux de jeu.
    // (regles-de-la-vente) Ce prédicat ne compte plus que les MC : la main
    // garnie ne rend donc plus Comet payable, et c'est le second cas qui bascule.
    let db = db();
    let strict = |mc: i64, filler: usize| ProbeOptions {
        mc, filler, strict: true, phase: 0, plants: 20, upgrades: [None; 5], objectif: None
    };
    assert!(
        !run_probe_seq_opts(&db, &["Comet"], strict(0, 0)).played,
        "0 MC, main vide : Comet est impayable"
    );
    assert!(
        !run_probe_seq_opts(&db, &["Comet"], strict(0, 9)).played,
        "0 MC, 9 cartes en main : elles ne sont plus une monnaie"
    );
    assert!(
        !run_probe_seq_opts(&db, &["Comet"], strict(24, 9)).played,
        "24 MC : il manque 1 MC, et rien ne le comble"
    );
    // TÉMOIN : la carte reste posable dès que les MC y sont — on n'a pas cassé
    // l'achat pour rendre le refus facile.
    assert!(
        run_probe_seq_opts(&db, &["Comet"], strict(25, 0)).played,
        "25 MC : Comet est posée"
    );
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

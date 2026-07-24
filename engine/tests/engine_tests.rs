//! Tests du squelette moteur. Les tests de mulligan et de flux passent par le
//! flux réel (`setup_game` / `play_round`), le même que celui du binaire
//! `simulate` — pas de fonctions parallèles.

use engine::cards::{CardsDb, Color};
use engine::flow::{
    allowed_phases, assign_milestones, award_points, play_round, score, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::sim::{check_invariants, run_simulation, InvariantTracker, MAX_GENERATIONS};
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::VecDeque;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

/// Politique scriptée pour les tests : décisions forcées là où le test le
/// demande, aléatoire (RNG de la partie) sinon. Enregistre l'ordre des appels.
struct TestPolicy {
    base: RandomPolicy,
    corp_mulligans: [bool; 2],
    project_mulligans: [bool; 2],
    corp_picks: [usize; 2],
    /// Phases servies dans l'ordre des appels (p0, p1, p0, p1, ...).
    phase_script: VecDeque<u8>,
    /// Actions servies dans l'ordre ; épuisé => stop. None = stop explicite.
    action_script: VecDeque<Option<ActionOpt>>,
    construction_bonus: ConstructionBonus,
    build_nothing: bool,
    call_log: Vec<String>,
    /// Mémoire des corporations vues au moment du mulligan.
    corp_mulligan_offers: Vec<(usize, Vec<u16>)>,
    project_mulligan_hands: Vec<(usize, Vec<u16>)>,
}

impl TestPolicy {
    fn new() -> TestPolicy {
        TestPolicy {
            base: RandomPolicy,
            corp_mulligans: [false, false],
            project_mulligans: [false, false],
            corp_picks: [0, 0],
            phase_script: VecDeque::new(),
            action_script: VecDeque::new(),
            construction_bonus: ConstructionBonus::SecondBuild,
            build_nothing: true,
            call_log: Vec::new(),
            corp_mulligan_offers: Vec::new(),
            project_mulligan_hands: Vec::new(),
        }
    }
}

impl Policy for TestPolicy {
    fn corp_mulligan(&mut self, _rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        self.call_log.push(format!("corp_mulligan:{player}"));
        self.corp_mulligan_offers.push((player, corps.to_vec()));
        self.corp_mulligans[player]
    }

    fn project_mulligan(&mut self, _rng: &mut StdRng, player: usize, hand: &[u16]) -> bool {
        self.call_log.push(format!("project_mulligan:{player}"));
        self.project_mulligan_hands.push((player, hand.to_vec()));
        self.project_mulligans[player]
    }

    fn pick_corporation(&mut self, _rng: &mut StdRng, player: usize, _corps: &[u16]) -> usize {
        self.call_log.push(format!("pick_corporation:{player}"));
        self.corp_picks[player]
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        match self.phase_script.pop_front() {
            Some(ph) => {
                assert!(allowed.contains(&ph), "script de phase invalide");
                ph
            }
            None => self.base.pick_phase(rng, player, allowed),
        }
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        if self.build_nothing {
            None
        } else {
            self.base.choose_build(rng, player, affordable)
        }
    }

    fn construction_bonus(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        self.construction_bonus
    }

    fn action_choice(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        match self.action_script.pop_front() {
            Some(Some(opt)) => {
                let idx = options.iter().position(|&o| o == opt);
                assert!(idx.is_some(), "action scriptée {opt:?} absente de {options:?}");
                idx
            }
            _ => None,
        }
    }

    fn research_keep(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        (0..keep).collect()
    }

    fn discard_down(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        (0..n).collect()
    }
}

// ---------------------------------------------------------------- chargement

#[test]
fn cards_load_248_projects_and_16_corporations() {
    // Depuis le chantier cartes-1, la base charge TOUTES les cartes projets
    // (green/blue/red = 331, pour que la sonde trouve aussi les cartes hors
    // pioche — journal B2) mais la pioche v1 reste 248 cartes.
    let db = db();
    assert_eq!(db.v1_project_count, 248, "236 base + Discovery in_deck_v1");
    assert_eq!(db.projects.len(), 331, "toutes cartes green/blue/red");
    assert_eq!(db.corporations.len(), 16);
    let v1 = |color| {
        db.projects
            .iter()
            .filter(|c| c.in_deck_v1 && c.color == color)
            .count()
    };
    assert_eq!(
        (v1(Color::Green), v1(Color::Blue), v1(Color::Red)),
        (136, 72, 40)
    );
}

// ------------------------------------------------------- mulligans (maison)

#[test]
fn corp_mulligan_replaces_both_corporations() {
    let db = db();
    let mut pol = TestPolicy::new();
    pol.corp_mulligans = [true, false];
    let game = setup_game(&db, 11, &mut pol);

    // p0 a mulligané : sa corporation finale ne vient PAS de sa paire initiale.
    let p0_initial = &pol.corp_mulligan_offers[0].1;
    let p0_final = game.players[0].corporation.unwrap();
    assert!(!p0_initial.contains(&p0_final), "les 2 corporations doivent être remplacées");
    // Ses 2 corporations initiales sont écartées.
    for c in p0_initial {
        assert!(game.corp_discard.contains(c));
    }
    // p1 n'a pas mulligané : sa corporation vient de sa paire initiale.
    let p1_initial = &pol.corp_mulligan_offers[1].1;
    assert!(p1_initial.contains(&game.players[1].corporation.unwrap()));
    // Conservation : 2 choisies + écartées + paquet = 16.
    assert_eq!(game.corp_deck.len() + game.corp_discard.len() + 2, 16);
}

#[test]
fn corp_mulligan_refused_keeps_both() {
    let db = db();
    let mut pol = TestPolicy::new();
    pol.corp_mulligans = [false, false];
    let game = setup_game(&db, 12, &mut pol);
    for p in 0..2 {
        let initial = &pol.corp_mulligan_offers[p].1;
        assert!(initial.contains(&game.players[p].corporation.unwrap()));
    }
    // Aucune corporation partie au rebut par mulligan : seulement les 2 non choisies.
    assert_eq!(game.corp_discard.len(), 2);
}

#[test]
fn corp_mulligan_happens_before_project_cards_and_pick_after() {
    // Règle maison n°1 : mulligan corpos AVANT la donne projets ; le choix
    // final de corporation se fait cartes projets en main.
    let db = db();
    let mut pol = TestPolicy::new();
    pol.corp_mulligans = [true, true];
    let _game = setup_game(&db, 13, &mut pol);
    let log = &pol.call_log;
    let last_corp_mull = log.iter().rposition(|s| s.starts_with("corp_mulligan")).unwrap();
    let first_proj = log.iter().position(|s| s.starts_with("project_mulligan")).unwrap();
    let first_pick = log.iter().position(|s| s.starts_with("pick_corporation")).unwrap();
    assert!(last_corp_mull < first_proj, "mulligan corpos avant les cartes projets");
    assert!(first_proj < first_pick, "choix de corporation après la donne projets");
    // Au moment du mulligan projets, la main contient bien 8 cartes.
    for (_, hand) in &pol.project_mulligan_hands {
        assert_eq!(hand.len(), 8);
    }
}

#[test]
fn project_mulligan_replaces_all_eight_at_once() {
    let db = db();
    let mut pol = TestPolicy::new();
    pol.project_mulligans = [true, false];
    let game = setup_game(&db, 14, &mut pol);

    let p0_initial = &pol.project_mulligan_hands[0].1;
    assert_eq!(game.players[0].hand.len(), 8);
    // Les 8 anciennes cartes sont en défausse, aucune n'est restée en main.
    for c in p0_initial {
        assert!(game.discard.contains(c));
        assert!(!game.players[0].hand.contains(c));
    }
    // p1 garde exactement sa main initiale.
    assert_eq!(&game.players[1].hand, &pol.project_mulligan_hands[1].1);
}

#[test]
fn corporation_grants_starting_mc_and_tags() {
    let db = db();
    let mut pol = TestPolicy::new();
    let game = setup_game(&db, 15, &mut pol);
    for p in 0..2 {
        let corp = &db.corporations[game.players[p].corporation.unwrap() as usize];
        assert_eq!(game.players[p].mc, corp.starting_mc, "MC de départ = price (D3)");
        for t in &corp.tags {
            if let Some(i) = t.index() {
                assert!(game.players[p].tag_counts[i] >= 1);
            }
        }
        assert_eq!(game.players[p].tr, 5, "TR de départ = 5");
    }
}

// ------------------------------------------------ contrainte de choix de phase

#[test]
fn phase_cannot_repeat_previous_round() {
    let mut pl = PlayerState::new();
    assert_eq!(allowed_phases(&pl), vec![1, 2, 3, 4, 5], "ronde 1 : tout est permis");
    pl.previous_phase = Some(3);
    assert_eq!(allowed_phases(&pl), vec![1, 2, 4, 5]);
    pl.previous_phase = Some(1);
    assert_eq!(allowed_phases(&pl), vec![2, 3, 4, 5]);
}

#[test]
fn phase_choice_never_repeats_over_full_games() {
    // Sur des parties réelles en politique aléatoire, aucun joueur ne rejoue
    // jamais sa phase de la ronde précédente.
    struct CheckPolicy {
        base: RandomPolicy,
        prev: [Option<u8>; 2],
    }
    impl Policy for CheckPolicy {
        fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
            self.base.corp_mulligan(r, p, c)
        }
        fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
            self.base.project_mulligan(r, p, h)
        }
        fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
            self.base.pick_corporation(r, p, c)
        }
        fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
            let ph = self.base.pick_phase(r, p, allowed);
            assert_ne!(Some(ph), self.prev[p], "phase répétée deux rondes de suite");
            self.prev[p] = Some(ph);
            ph
        }
        fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            self.base.choose_build(r, p, a)
        }
        fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
            self.base.construction_bonus(r, p)
        }
        fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            self.base.action_choice(r, p, o)
        }
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            self.base.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            self.base.discard_down(r, p, h, n)
        }
    }
    let db = db();
    for seed in 0..5u64 {
        let mut pol = CheckPolicy { base: RandomPolicy, prev: [None, None] };
        let mut game = setup_game(&db, seed, &mut pol);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, &db, &mut pol);
        }
        assert!(game.game_over);
    }
}

// ----------------------------------------------------------------- production

#[test]
fn production_pays_mc_prod_plus_tr_plus_selector_bonus() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 20, &mut pol);

    // Productions artificielles (les stubs v1 n'en donnent pas) pour tester la
    // mécanique de la phase — structure réelle, valeurs contrôlées.
    game.players[0].mc_prod = 4;
    game.players[0].heat_prod = 3;
    game.players[0].plant_prod = 1;
    game.players[0].card_prod = 1; // main finale 8+1+1 = 10 : sous la limite
    game.players[1].mc_prod = 2;

    let mc0 = game.players[0].mc;
    let mc1 = game.players[1].mc;
    let tr0 = game.players[0].tr;
    let tr1 = game.players[1].tr;
    let hand0 = game.players[0].hand.len();
    let hand1 = game.players[1].hand.len();

    // p0 sélectionne la production (bonus +4), p1 la recherche.
    pol.phase_script = VecDeque::from(vec![4, 5]);
    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.players[0].mc, mc0 + 4 + tr0 + PRODUCTION_SELECTOR_MC);
    assert_eq!(game.players[0].heat, 3);
    assert_eq!(game.players[0].plants, 1);
    // p1 : pas de bonus sélectionneur de production.
    assert_eq!(game.players[1].mc, mc1 + 2 + tr1);
    // Recherche : p0 non-sélectionneur pioche 2 garde 1 ; p1 pioche 5 garde 2.
    assert_eq!(game.players[0].hand.len(), hand0 + 1 /*card_prod*/ + 1);
    assert_eq!(game.players[1].hand.len(), hand1 + 2);
}

#[test]
fn production_without_selection_gives_no_bonus() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 21, &mut pol);
    let mc0 = game.players[0].mc;
    let tr0 = game.players[0].tr;
    // p0 développement, p1 production : p0 touche production SANS +4.
    pol.phase_script = VecDeque::from(vec![1, 4]);
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].mc, mc0 + tr0, "production = prod MC (0) + TR, sans bonus");
}

// -------------------------------------------------------------- phase action

#[test]
fn standard_projects_costs_and_tr() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 22, &mut pol);
    game.players[0].mc = 100;
    game.players[1].mc = 0;
    let tr0 = game.players[0].tr;
    let tile = game.oceans[0];
    let hand0 = game.players[0].hand.len();

    pol.phase_script = VecDeque::from(vec![3, 5]);
    pol.action_script = VecDeque::from(vec![
        Some(ActionOpt::TemperatureWithMc),
        Some(ActionOpt::OceanWithMc),
        Some(ActionOpt::ForestWithMc),
        None,
    ]);
    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.temperature, 1);
    assert_eq!(game.oceans_revealed, 1);
    assert_eq!(game.oxygen, 1, "la forêt monte l'oxygène");
    assert_eq!(game.players[0].forests, 1);
    assert_eq!(game.players[0].tr, tr0 + 3, "1 TR par hausse de paramètre");
    // 100 - 14 (température) - 15 (océan) - 20 (forêt) + bonus MC de la tuile.
    assert_eq!(game.players[0].mc, 100 - 14 - 15 - 20 + tile.mc);
    assert_eq!(game.players[0].plants, tile.plants, "bonus plantes de la tuile océan");
    // Cartes piochées par la tuile + recherche (non-sélectionneur : +1).
    assert_eq!(game.players[0].hand.len(), hand0 + tile.cards as usize + 1);
}

#[test]
fn sell_card_gives_3_mc() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 23, &mut pol);
    game.players[0].mc = 0;
    let hand0 = game.players[0].hand.len();
    let discard0 = game.discard.len();

    pol.phase_script = VecDeque::from(vec![3, 4]);
    pol.action_script = VecDeque::from(vec![Some(ActionOpt::SellCard), None]);
    play_round(&mut game, &db, &mut pol);

    // +3 MC de la vente, puis production (prod 0 + TR).
    assert_eq!(game.players[0].mc, 3 + game.players[0].tr);
    assert_eq!(game.players[0].hand.len(), hand0 - 1, "pas de phase recherche cette ronde");
    assert_eq!(game.discard.len(), discard0 + 1);
}

#[test]
fn forced_conversions_at_end_of_action_phase() {
    // « Viktig regel » livret p.14 : en fin de phase d'action, plantes et
    // chaleur DOIVENT être converties tant que possible.
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 24, &mut pol);
    game.players[0].plants = 9;
    game.players[0].heat = 17;
    game.players[0].mc = 0;
    let tr0 = game.players[0].tr;

    pol.phase_script = VecDeque::from(vec![3, 4]);
    // Aucune action volontaire : le script est vide => stop immédiat.
    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.players[0].plants, 1, "9 plantes → 1 forêt (8) + reste 1");
    assert_eq!(game.players[0].forests, 1);
    assert_eq!(game.oxygen, 1);
    assert_eq!(game.players[0].heat, 1, "17 chaleur → 2 hausses (16) + reste 1");
    assert_eq!(game.temperature, 2);
    assert_eq!(game.players[0].tr, tr0 + 3);
}

#[test]
fn forced_conversions_skip_parameters_already_maxed() {
    // Livret p.14 : l'obligation de conversion tombe quand le paramètre a
    // déjà atteint son max (état réel) — les plantes/chaleur restent.
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 26, &mut pol);
    game.oxygen = OXYGEN_MAX;
    game.temperature = TEMPERATURE_MAX;
    game.players[0].plants = 9;
    game.players[0].heat = 17;

    pol.phase_script = VecDeque::from(vec![3, 4]);
    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.players[0].plants, 9, "pas de conversion forcée : oxygène au max");
    assert_eq!(game.players[0].heat, 17, "pas de conversion forcée : température au max");
    assert_eq!(game.players[0].forests, 0);
}

#[test]
fn blue_action_stub_consumes_activation() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 25, &mut pol);
    // Met une carte bleue en jeu par le flux réel de construction.
    // Une carte bleue entre en jeu (mise en jeu directe : l'objet de CE test
    // est l'activation en phase action ; la construction est couverte par les
    // autres tests). La carte est retirée de la pioche pour la conservation.
    let blue_id = game
        .deck
        .iter()
        .copied()
        .find(|&c| db.projects[c as usize].color == Color::Blue)
        .unwrap();
    game.deck.retain(|&c| c != blue_id);
    game.players[0].put_in_play(blue_id, &db);
    pol.phase_script = VecDeque::from(vec![3, 4]);
    pol.action_script = VecDeque::from(vec![
        Some(ActionOpt::BlueAction(blue_id)),
        // Deuxième activation : interdite (une fois par phase), l'option ne
        // doit plus être proposée — le script s'arrête là et le test
        // vérifie qu'aucune assertion "action absente" n'a sauté avant.
        None,
    ]);
    play_round(&mut game, &db, &mut pol);
    assert!(game.players[0].played.contains(&blue_id));
}

// ------------------------------------------------------------- fin de partie

#[test]
fn game_ends_when_all_parameters_maxed_and_skips_remaining_phases() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 30, &mut pol);
    game.temperature = TEMPERATURE_MAX;
    game.oxygen = OXYGEN_MAX;
    game.oceans_revealed = NUM_OCEANS - 1;
    game.players[0].mc = 15;
    let hand_sizes = [game.players[0].hand.len(), game.players[1].hand.len()];
    let generation = game.generation;

    pol.phase_script = VecDeque::from(vec![3, 5]);
    pol.action_script = VecDeque::from(vec![Some(ActionOpt::OceanWithMc), None]);
    play_round(&mut game, &db, &mut pol);

    assert!(game.game_over, "3 paramètres au max => fin de partie");
    assert_eq!(game.oceans_revealed, NUM_OCEANS);
    // La phase recherche (5) de la ronde n'a PAS été jouée (livret p.16-17)…
    let tile = {
        // dernière tuile révélée
        game.oceans[(NUM_OCEANS - 1) as usize]
    };
    assert_eq!(
        game.players[0].hand.len(),
        hand_sizes[0] + tile.cards as usize,
        "pas de pioche de recherche après la fin"
    );
    assert_eq!(game.players[1].hand.len(), hand_sizes[1]);
    // … et l'étape de fin (défausse/génération suivante) non plus.
    assert_eq!(game.generation, generation);
}

#[test]
fn truncated_games_are_not_completed() {
    // Une partie qui atteint le plafond n'est jamais comptée complète :
    // politique qui ne fait strictement rien.
    struct DoNothing;
    impl Policy for DoNothing {
        fn corp_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
            false
        }
        fn project_mulligan(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> bool {
            false
        }
        fn pick_corporation(&mut self, _: &mut StdRng, _: usize, _: &[u16]) -> usize {
            0
        }
        fn pick_phase(&mut self, _: &mut StdRng, _: usize, allowed: &[u8]) -> u8 {
            // Toujours production/recherche en alternance : ne termine jamais.
            if allowed.contains(&4) {
                4
            } else {
                5
            }
        }
        fn choose_build(&mut self, _: &mut StdRng, _: usize, _: &[usize]) -> Option<usize> {
            None
        }
        fn construction_bonus(&mut self, _: &mut StdRng, _: usize) -> ConstructionBonus {
            ConstructionBonus::DrawCard
        }
        fn action_choice(&mut self, _: &mut StdRng, _: usize, _: &[ActionOpt]) -> Option<usize> {
            None
        }
        fn research_keep(&mut self, _: &mut StdRng, _: usize, _: &[u16], k: usize) -> Vec<usize> {
            (0..k).collect()
        }
        fn discard_down(&mut self, _: &mut StdRng, _: usize, _: &[u16], n: usize) -> Vec<usize> {
            (0..n).collect()
        }
    }
    let db = db();
    let mut pol = DoNothing;
    let out = engine::sim::play_game(&db, 99, &mut pol);
    assert!(!out.completed, "partie plafonnée => non complétée");
    assert_eq!(out.generations, MAX_GENERATIONS + 1);
}

// -------------------------------------------------------------------- score

#[test]
fn score_counts_tr_forests_milestones_awards() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 40, &mut pol);

    // État contrôlé : TR via gain_tr (cohérence de l'invariant), forêts.
    for _ in 0..7 {
        game.players[0].gain_tr(); // TR 5 -> 12
    }
    game.players[0].forests = 4;
    game.players[1].forests = 1;

    // Milestone 0 revendiqué par p0.
    game.milestones[0].achieved_by = [true, false];

    // Awards contrôlés : Celebrity (prod MC) et Researcher (tags science)
    // départagent p0/p1, Collector (stub 0) est une égalité.
    game.awards = [AwardKind::Celebrity, AwardKind::Collector, AwardKind::Researcher];
    game.players[0].mc_prod = 3; // p0 gagne Celebrity : 5 / 2
    let sci = engine::cards::Tag::Science.index().unwrap();
    game.players[0].tag_counts[sci] = 0;
    game.players[1].tag_counts[sci] = 2; // p1 gagne Researcher : 2 / 5

    let s = score(&game, &db);
    // p0 : TR 12 + 4 forêts + 3 (milestone) + 5 (Celebrity) + 4 (Collector
    // égalité) + 2 (Researcher perdu) = 30
    assert_eq!(s[0], 12 + 4 + 3 + 5 + 4 + 2);
    // p1 : TR 5 + 1 forêt + 0 + 2 + 4 + 5 = 17
    assert_eq!(s[1], 5 + 1 + 2 + 4 + 5);
}

#[test]
fn awards_tie_first_place_gives_4_each() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 41, &mut pol);
    game.awards = [AwardKind::Collector, AwardKind::Collector, AwardKind::Collector];
    let pts = award_points(&game);
    assert_eq!(pts, [12, 12], "égalité => 4 VP chacun, pas de 2e place");
}

#[test]
fn milestone_terraformer_first_claim_locks_it() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 42, &mut pol);
    game.milestones[0] = MilestoneSlot {
        kind: MilestoneKind::Terraformer,
        achieved_by: [false, false],
    };
    // p0 atteint TR 15.
    for _ in 0..10 {
        game.players[0].gain_tr();
    }
    assign_milestones(&mut game);
    assert_eq!(game.milestones[0].achieved_by, [true, false]);
    // p1 l'atteint APRÈS : le milestone est déjà revendiqué, pas de 2e prise.
    for _ in 0..10 {
        game.players[1].gain_tr();
    }
    assign_milestones(&mut game);
    assert_eq!(game.milestones[0].achieved_by, [true, false]);
}

#[test]
fn milestones_simultaneous_claim_scores_both() {
    // Discovery p.3 : revendications simultanées => l'autre joueur prend un
    // jeton 3 VP — les deux scorent.
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 43, &mut pol);
    game.milestones[0] = MilestoneSlot {
        kind: MilestoneKind::Terraformer,
        achieved_by: [false, false],
    };
    for _ in 0..10 {
        game.players[0].gain_tr();
        game.players[1].gain_tr();
    }
    assign_milestones(&mut game);
    assert_eq!(game.milestones[0].achieved_by, [true, true]);
}

// -------------------------------------------------------- limite de main

#[test]
fn hand_limit_discards_down_to_10_for_3_mc_each() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 50, &mut pol);
    // Gonfle la main de p0 à 13 par pioche réelle.
    for _ in 0..5 {
        let c = engine::flow::draw_card(&mut game).unwrap();
        game.players[0].hand.push(c);
    }
    assert_eq!(game.players[0].hand.len(), 13);
    game.players[0].mc = 0;
    let discard0 = game.discard.len();

    // Ronde sans effet sur les mains : développement + action, sans
    // constructions ni actions.
    pol.phase_script = VecDeque::from(vec![1, 3]);
    play_round(&mut game, &db, &mut pol);

    assert_eq!(game.players[0].hand.len(), HAND_LIMIT);
    assert_eq!(game.players[0].mc, 9, "3 cartes défaussées x 3 MC");
    assert_eq!(game.discard.len(), discard0 + 3);
}

// ------------------------------------------------- invariants & déterminisme

#[test]
fn invariants_hold_on_random_games() {
    let db = db();
    for seed in 100..105u64 {
        let mut pol = RandomPolicy;
        let out = engine::sim::play_game(&db, seed, &mut pol);
        assert!(out.completed, "partie aléatoire terminée (seed {seed})");
        assert_eq!(out.violations, 0, "zéro violation d'invariant (seed {seed})");
    }
}

#[test]
fn card_conservation_checked_by_invariants() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 60, &mut pol);
    let mut tracker = InvariantTracker::new(&game);
    assert!(check_invariants(&game, &db, &mut tracker).is_ok());
    let total = game.deck.len()
        + game.discard.len()
        + game.players.iter().map(|p| p.hand.len() + p.played.len()).sum::<usize>();
    assert_eq!(total, 248);

    // Une carte qui disparaît doit être détectée.
    game.players[0].hand.pop();
    assert!(check_invariants(&game, &db, &mut tracker).is_err());
}

#[test]
fn negative_resources_detected_by_invariants() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 61, &mut pol);
    let mut tracker = InvariantTracker::new(&game);
    game.players[0].mc = -1;
    assert!(check_invariants(&game, &db, &mut tracker).is_err());
}

#[test]
fn determinism_same_seed_same_output_different_seed_differs() {
    let db = db();
    let mut p1 = RandomPolicy;
    let a = run_simulation(&db, 30, 7, &mut p1);
    let mut p2 = RandomPolicy;
    let b = run_simulation(&db, 30, 7, &mut p2);
    let mut p3 = RandomPolicy;
    let c = run_simulation(&db, 30, 8, &mut p3);
    assert_eq!(a.state_hash, b.state_hash, "même graine => même empreinte");
    assert_eq!(a.avg_generations, b.avg_generations);
    assert_eq!(a.avg_score_p1, b.avg_score_p1);
    assert_ne!(a.state_hash, c.state_hash, "graines différentes => empreintes différentes");
}

#[test]
fn research_reshuffles_discard_when_deck_empty() {
    let db = db();
    let mut pol = TestPolicy::new();
    let mut game = setup_game(&db, 70, &mut pol);
    // Vide la pioche dans la défausse.
    let cards: Vec<u16> = game.deck.drain(..).collect();
    game.discard.extend(cards);
    assert!(game.deck.is_empty());
    let drawn = engine::flow::draw_card(&mut game);
    assert!(drawn.is_some(), "la défausse est remélangée en pioche");
    assert!(game.discard.is_empty());
}

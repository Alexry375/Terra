//! Tests du lot cartes-7 — **les neuf modificateurs permanents**.
//!
//! Les neuf cartes ne créent aucun flux de jeu : chacune change une valeur qu'un
//! service unique du moteur calcule déjà. Ce fichier vérifie donc, pour chacune,
//! **trois choses distinctes** :
//!
//! 1. le SERVICE rend la bonne valeur (et 0 / le neutre en `--effects off`) ;
//! 2. le service est réellement CONSOMMÉ par le chemin de jeu — affordabilité ET
//!    paiement, jamais l'un sans l'autre (I2) ;
//! 3. ce que la carte ne doit PAS toucher (NEVER 8, NEVER 9, I3).
//!
//! Le texte imprimé fait foi : `inputs/textes-cartes.json`, champ `text`, jamais
//! le champ `description` de `cards.json`.
//!
//! Les tests d'intégration passent par le CHEMIN RÉEL (`flow::setup_game` +
//! `flow::build_card_with` + `flow::play_round`), avec une politique scriptée :
//! aucun état fabriqué que la partie ne produirait pas.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Tag};
use engine::effects::{CardEffects, Reduction};
use engine::flow::{
    build_card_with, discard_mc_rate, payable, plant_discount, plants_reserved_by, play_round,
    requirements_met, research_base, research_draw_keep, research_extra, setup_game,
    standard_action_discount, standard_mc_cost,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{
    run_probe_action_seq, run_probe_seq_corp, ProbeActionResult, ProbeOptions, ProbeResult,
    ProbeScript,
};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::VecDeque;

const CARDS: &str = "../data/cards.json";

/// Les neuf cartes du périmètre, dans l'ordre du contrat.
const LOT7: [&str; 9] = [
    "Interns",
    "Extended Resources",
    "United Planetary Alliance",
    "Composting Factory",
    "Standard Technology",
    "Restructured Resources",
    "Adaptation Technology",
    "Assembly Lines",
    "Mars University",
];

/// Les cinq cartes HORS périmètre : elles doivent rester muettes.
const HORS: [&str; 5] = [
    "Asset Liquidation",
    "Special Design",
    "Work Crews",
    "Automated Factories",
    "Tall Station",
];

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

fn db_off() -> CardsDb {
    let mut d = db();
    d.effects_on = false;
    d
}

fn opts(mc: i64) -> ProbeOptions {
    ProbeOptions { mc, ..ProbeOptions::default() }
}

/// Sonde séquence.
fn seq(db: &CardsDb, names: &[&str], o: ProbeOptions) -> ProbeResult {
    let r = run_probe_seq_corp(db, names, o, &ProbeScript::default(), false, None);
    // GARDE OBLIGATOIRE (contrat, §ALWAYS) : ne jamais juger une valeur avant
    // d'avoir vérifié que la sonde a TROUVÉ la carte. Un nom mal orthographié
    // doit faire tomber le test, pas le faire passer en ne vérifiant rien.
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    r
}

/// Sonde séquence scriptée (les « may » du lot).
fn seq_choix(db: &CardsDb, names: &[&str], o: ProbeOptions, choix: &[usize]) -> ProbeResult {
    let script = ProbeScript { choices: choix.to_vec(), targets: Vec::new(), joker_tag: None };
    let r = run_probe_seq_corp(db, names, o, &script, false, None);
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    r
}

/// Sonde séquence avec corporation imposée.
fn seq_corp(db: &CardsDb, corp: &str, names: &[&str]) -> ProbeResult {
    let r = run_probe_seq_corp(db, names, opts(400), &ProbeScript::default(), false, Some(corp));
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    let c = r.corp.as_ref().expect("--probe-corp doit rendre un objet corp");
    assert!(c.found, "corporation jamais installée : « {} »", c.name);
    assert!(c.encoded, "corporation sans encodage : « {} »", c.name);
    r
}

/// Sonde action sur une séquence.
fn act(db: &CardsDb, names: &[&str], o: ProbeOptions) -> ProbeActionResult {
    let r = run_probe_action_seq(db, names, &ProbeScript::default(), None, o);
    assert!(r.found, "sonde action : carte introuvable « {} »", r.card);
    r
}

// ===================================================== politique scriptée
//
// Elle ne remplace aucune règle : elle ne fait que répondre aux points de
// décision que le moteur pose déjà, de façon déterministe. Tout ce qui n'est pas
// scripté retombe sur `RandomPolicy`.

struct Scriptee {
    base: RandomPolicy,
    /// Réponses à `choose_option` (les « may »).
    choix: VecDeque<usize>,
    /// Indices imposés à `discard_down`.
    defausses: VecDeque<usize>,
    /// Phase imposée à `pick_phase`.
    phase: u8,
    /// Actions imposées à `action_choice` ; épuisée = le joueur passe.
    actions: VecDeque<ActionOpt>,
    /// (moteur-questions-manquantes) Ventes LIBRES que le joueur 0 fait encore,
    /// une carte par occasion. La vente n'est plus une action de la phase Action
    /// (elle y coûtait un échange) : le seul chemin est désormais l'occasion
    /// ouverte avant chaque point de décision, `flow::occasion_de_vendre`.
    ventes_libres: usize,
}

impl Scriptee {
    fn new() -> Scriptee {
        Scriptee {
            base: RandomPolicy,
            choix: VecDeque::new(),
            defausses: VecDeque::new(),
            phase: 4,
            actions: VecDeque::new(),
            ventes_libres: 0,
        }
    }
}

impl Policy for Scriptee {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) {
            self.phase
        } else {
            allowed[0]
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        _r: &mut StdRng,
        _p: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        let want = self.actions.front().copied()?;
        match options.iter().position(|o| *o == want) {
            Some(i) => {
                self.actions.pop_front();
                Some(i)
            }
            // L'action voulue n'est pas offerte : le joueur passe. C'est
            // exactement ce qu'un test d'affordabilité doit pouvoir observer.
            None => None,
        }
    }
    fn vendre_librement(&mut self, _r: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        if joueur == 0 && self.ventes_libres > 0 && !main.is_empty() {
            self.ventes_libres -= 1;
            vec![0]
        } else {
            Vec::new()
        }
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        match self.choix.pop_front() {
            Some(c) => c,
            None => self.base.choose_option(r, p, n),
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        if n == 1 {
            if let Some(i) = self.defausses.pop_front() {
                return vec![i];
            }
        }
        self.base.discard_down(r, p, h, n)
    }
}

/// Partie réelle, mains vidées (les cartes retirées rejoignent la défausse : la
/// conservation des cartes reste vraie).
fn jeu(db: &CardsDb) -> GameState {
    let mut pol = RandomPolicy;
    let mut g = setup_game(db, 7, &mut pol);
    for p in 0..NUM_PLAYERS {
        let h: Vec<u16> = g.players[p].hand.drain(..).collect();
        g.discard.extend(h);
        g.players[p].mc = 0;
        g.players[p].heat = 0;
        g.players[p].plants = 0;
    }
    g
}

/// Fait passer une carte de la pioche (ou de la défausse) à la main du joueur 0.
fn en_main(g: &mut GameState, db: &CardsDb, nom: &str) -> u16 {
    let id = db
        .resolve_card(nom)
        .unwrap_or_else(|| panic!("carte introuvable dans la base : « {nom} »"));
    if let Some(i) = g.deck.iter().position(|&c| c == id) {
        g.deck.remove(i);
    } else if let Some(i) = g.discard.iter().position(|&c| c == id) {
        g.discard.remove(i);
    } else {
        panic!("« {nom} » n'est ni en pioche ni en défausse");
    }
    g.players[0].hand.push(id);
    id
}

/// Pose une carte par le CHEMIN RÉEL (`flow::build_card_with`), MC à volonté.
fn poser(g: &mut GameState, db: &CardsDb, nom: &str, pol: &mut dyn Policy) -> u16 {
    let id = en_main(g, db, nom);
    let idx = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(g, db, 0, idx, 0, pol);
    assert!(g.players[0].played.contains(&id), "« {nom} » n'est pas entrée en jeu");
    id
}

/// L'encodage d'une carte du lot, par son nom.
fn spec(nom: &str) -> &'static CardEffects {
    engine::effects::lookup(nom).unwrap_or_else(|| panic!("« {nom} » n'est pas encodée"))
}

// =========================================================================
// GROUPE A — la phase de recherche
// =========================================================================

#[test]
fn interns_draws_two_additional_cards_in_the_research_phase() {
    // « When you draw cards during the research phase, draw TWO additional
    // cards. » — pioche seule, aucune carte gardée en plus.
    let db = db();
    let r = seq(&db, &["Interns"], opts(400));
    assert_eq!(r.research, (2, 0), "Interns : +2 piochées, +0 gardée");
}

#[test]
fn extended_resources_keeps_one_additional_card() {
    // « When you KEEP cards during the research phase, keep one additional
    // card. » — le texte parle de GARDER, jamais de piocher.
    let db = db();
    let r = seq(&db, &["Extended Resources"], opts(400));
    assert_eq!(r.research, (0, 1), "Extended Resources : +0 piochée, +1 gardée");
}

#[test]
fn united_planetary_alliance_draws_and_keeps_one_more() {
    let db = db();
    let r = seq(&db, &["United Planetary Alliance"], opts(400));
    assert_eq!(r.research, (1, 1));
}

#[test]
fn united_planetary_alliance_is_the_twin_of_interplanetary_relations() {
    // Les deux textes imprimés sont mot pour mot identiques : les deux encodages
    // doivent l'être aussi. Contrôle croisé sur la TABLE, pas sur la sonde.
    assert_eq!(
        spec("United Planetary Alliance").research,
        spec("Interplanetary Relations").research,
        "textes imprimés identiques, encodages identiques"
    );
}

#[test]
fn interns_and_extended_resources_add_up() {
    let db = db();
    let r = seq(&db, &["Interns", "Extended Resources"], opts(400));
    assert_eq!(r.research, (2, 1), "les bonus se cumulent sur les cartes en jeu");
}

#[test]
fn the_three_research_cards_add_up_all_together() {
    let db = db();
    let r = seq(
        &db,
        &["Interns", "Extended Resources", "United Planetary Alliance"],
        opts(400),
    );
    assert_eq!(r.research, (3, 2), "2+0 + 0+1 + 1+1");
}

#[test]
fn a_card_outside_group_a_gives_no_research_bonus() {
    let db = db();
    assert_eq!(seq(&db, &["Power Plant"], opts(400)).research, (0, 0));
    assert_eq!(seq(&db, &["Composting Factory"], opts(400)).research, (0, 0));
}

#[test]
fn the_research_bonus_is_null_with_effects_off() {
    // I7 : la couche d'effets coupée, le bonus n'existe pas.
    let db = db_off();
    assert_eq!(seq(&db, &["Interns"], opts(400)).research, (0, 0));
    assert_eq!(seq(&db, &["United Planetary Alliance"], opts(400)).research, (0, 0));
}

#[test]
fn a_corporation_and_a_card_feed_the_same_research_cumulation() {
    // Tharsis Republic porte le même texte imprimé qu'Interplanetary Relations.
    // Avec Interns en jeu, le cumul doit être 1+2 / 1+0.
    let db = db();
    let r = seq_corp(&db, "Tharsis Republic", &["Interns"]);
    assert_eq!(r.research, (3, 1), "corporation + carte : un seul cumul");
}

#[test]
fn the_probe_reports_exactly_what_the_research_phase_consumes() {
    // Clause anti-shortcut n° 1 : le champ `research` de la sonde ne doit pas
    // être un calcul propre à la sonde. On confronte le SERVICE au consommateur
    // réel de la phase V (`research_draw_keep`), sur un joueur en partie réelle.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    // La corporation du joueur 0 est TIRÉE AU SORT par `jeu()` et peut elle-même
    // porter un bonus de recherche (Tharsis Republic, par exemple). On mesure
    // donc son apport AVANT de poser quoi que ce soit, et on ne teste que le
    // delta dû aux deux cartes — sinon le test dépend de la graine.
    let socle = research_extra(&db, &g.players[0]);
    poser(&mut g, &db, "Interns", &mut pol);
    poser(&mut g, &db, "Extended Resources", &mut pol);

    let pl = &g.players[0];
    let extra = research_extra(&db, pl);
    assert_eq!((extra.0 - socle.0, extra.1 - socle.1), (2, 1));
    let (base_n, base_k) = research_base(&db, pl);
    assert_eq!(
        research_draw_keep(&db, pl),
        (base_n + extra.0, base_k + extra.1),
        "la phase V consomme exactement le service que la sonde expose"
    );
}

#[test]
fn the_research_bonus_follows_the_phase_selector_base() {
    // Base du livret p.15 : 2/1 pour tout le monde, 5/2 pour le sélectionneur de
    // la phase V. Le bonus s'AJOUTE à l'une comme à l'autre.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    // Même précaution qu'au-dessus : la corporation tirée au sort peut ajouter
    // son propre bonus, on le retranche.
    let socle = research_extra(&db, &g.players[0]);
    poser(&mut g, &db, "Interns", &mut pol);
    g.players[0].chosen_phase = 0;
    assert_eq!(
        research_draw_keep(&db, &g.players[0]),
        (4 + socle.0, 1 + socle.1),
        "2+2 / 1+0"
    );
    g.players[0].chosen_phase = 5;
    assert_eq!(
        research_draw_keep(&db, &g.players[0]),
        (7 + socle.0, 2 + socle.1),
        "5+2 / 2+0"
    );
}

#[test]
fn the_research_phase_really_draws_more_cards() {
    // Preuve en PARTIE RÉELLE : le compteur `research_extra_draws` du bilan de
    // simulation est strictement supérieur à sa valeur d'avant le lot (3888,
    // mesurée le 28-07 sur le moteur d'origine).
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 200, 2024, &mut pol);
    assert!(
        s.research_extra_draws > 0,
        "aucune pioche supplémentaire en 200 parties"
    );
}

// =========================================================================
// GROUPE B — le prix payé : Composting Factory
// =========================================================================

#[test]
fn composting_factory_raises_the_discard_rate_by_one_mc() {
    // « Cards you discard for MC are worth an additional 1 MC. »
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC,
        "taux du livret sans la carte"
    );
    poser(&mut g, &db, "Composting Factory", &mut pol);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC + 1,
        "un MC de plus par carte défaussée"
    );
}

#[test]
fn composting_factory_is_neutral_with_effects_off() {
    let db = db_off();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Composting Factory", &mut pol);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC,
        "I7 : effets coupés, taux du livret"
    );
}

#[test]
fn composting_factory_never_changes_the_opponents_rate() {
    // NEVER 9 : un effet ne dépend jamais des cartes de l'adversaire.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Composting Factory", &mut pol);
    assert_eq!(discard_mc_rate(&db, &g.players[0]), SELL_CARD_MC + 1);
    assert_eq!(
        discard_mc_rate(&db, &g.players[1]),
        SELL_CARD_MC,
        "l'adversaire garde le taux du livret"
    );
}

#[test]
fn composting_factory_ne_paie_plus_aucune_carte_d_office() {
    // (regles-de-la-vente) Ce test mesurait le NOMBRE de cartes que le moteur
    // défaussait d'office pour compléter un paiement (10 au taux du livret, 13
    // au taux majoré). C'est précisément le défaut B : le joueur ne choisissait
    // ni le moment ni les cartes. Le paiement d'office a été supprimé — la
    // mesure devient donc : à 0 MC, AUCUNE carte ne quitte la main, et la carte
    // n'est pas posée, quel que soit le taux.
    let db = db();
    let sans = seq(
        &db,
        &["Commercial District"],
        ProbeOptions { mc: 0, filler: 12, ..ProbeOptions::default() },
    );
    assert!(!sans.played, "0 MC : Commercial District (25 MC) n'est pas payable");
    assert_eq!(sans.delta.hand, 0, "aucune carte ne quitte la main sans décision");

    let avec = seq(
        &db,
        &["Composting Factory", "Commercial District"],
        ProbeOptions { mc: 0, filler: 20, ..ProbeOptions::default() },
    );
    assert!(!avec.played, "le taux majoré ne rend rien payable : il n'est plus une monnaie");
    assert_eq!(avec.delta.hand, 0, "aucune carte ne quitte la main sans décision");
}

#[test]
fn composting_factory_majore_toujours_le_taux_du_service_unique() {
    // (regles-de-la-vente) Ce test mesurait le taux À TRAVERS le paiement par
    // défausse, qui n'existe plus. Le TAUX, lui, doit rester intact : le prompt
    // le demande mot pour mot (« le taux reste celui du livret, majorations de
    // corporations comprises, par le service unique existant »). On le mesure
    // donc là où il vit désormais — le service unique — au lieu de le déduire
    // d'un paiement supprimé.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC,
        "sans Composting Factory : le taux du livret"
    );
    poser(&mut g, &db, "Composting Factory", &mut pol);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC + 1,
        "avec Composting Factory : un MC de plus par carte vendue"
    );
    // Et une carte ne se majore jamais elle-même : le taux qui aurait payé
    // Composting Factory était celui d'AVANT sa mise en jeu — c'est ce que
    // mesure l'ordre des deux assertions ci-dessus.
}

#[test]
fn le_taux_de_defausse_n_entre_plus_dans_l_affordabilite() {
    // (regles-de-la-vente) Ce test affirmait l'inverse : à 0 MC et 7 cartes en
    // main, une carte à 25 MC était payable au taux 4 (7 × 4 = 28 ≥ 25) et ne
    // l'était pas au taux 3. C'est le défaut A, mot pour mot — le moteur
    // comptait d'avance la vente d'une main que le joueur n'avait pas décidé de
    // vendre. `payable` ne connaît plus que les MC réels, et n'a donc plus de
    // paramètre de taux à recevoir : l'invariant I2 tient désormais par
    // construction, il n'y a plus deux lectures possibles d'un taux.
    assert!(!payable(0, 25), "0 MC : rien n'est payable, la main n'est pas une monnaie");
    assert!(!payable(24, 25), "24 MC : il manque 1 MC, aucune carte ne le comble");
    assert!(payable(25, 25), "25 MC : payable, au MC près");

    // Et le moteur applique bien cette règle sur le chemin réel : main garnie
    // ou main vide, sans les MC la carte n'est pas posée.
    let db = db();
    for filler in [0usize, 7, 12] {
        let r = seq(
            &db,
            &["Commercial District"],
            ProbeOptions { mc: 0, filler, ..ProbeOptions::default() },
        );
        assert!(!r.played, "{filler} cartes en main : elles ne paient rien (0 MC)");
    }
}

#[test]
fn composting_factory_applies_to_the_card_sale_standard_action() {
    // Site n° 3 : « vendre une carte » EST une défausse pour du MC. Chemin réel :
    // phase Action, une vente LIBRE, une seule carte.
    //
    // (moteur-questions-manquantes) Le chemin a changé, la règle non :
    // l'action standard `SellCard` a été retirée de la phase Action, la vente
    // passe par l'occasion libre. Le site de crédit reste le même service
    // unique (`flow::discard_mc_rate`), et c'est bien lui que ce test mesure.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Composting Factory", &mut pol);
    en_main(&mut g, &db, "Power Plant");
    g.players[0].mc = 0;
    g.players[0].plants = 0;
    g.players[0].heat = 0;
    pol.phase = 3;
    pol.ventes_libres = 1;
    play_round(&mut g, &db, &mut pol);
    assert_eq!(
        g.players[0].mc,
        SELL_CARD_MC + 1,
        "la vente de carte rapporte 4 MC avec Composting Factory"
    );
}

#[test]
fn the_card_sale_gives_three_mc_without_composting_factory() {
    // Contre-épreuve du test précédent, même chemin, sans la carte.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    en_main(&mut g, &db, "Power Plant");
    g.players[0].mc = 0;
    pol.phase = 3;
    pol.ventes_libres = 1;
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].mc, SELL_CARD_MC, "taux du livret");
}

#[test]
fn composting_factory_applies_to_the_end_of_round_hand_limit_discard() {
    // Site n° 4, celui que le contrat croyait hors du texte (journal §D1) : le
    // livret l. 437 et 654 dit « Pour chaque carte ainsi défaussée, le joueur
    // gagne 3 MC, COMME TOUJOURS ». C'est bien une défausse pour du MC.
    //
    // Chemin réel : phase IV (déterministe), puis étape de fin de manche avec
    // 13 cartes en main → 3 défaussées.
    fn mc_apres(avec_carte: bool) -> i64 {
        let db = db();
        let mut pol = Scriptee::new();
        let mut g = jeu(&db);
        if avec_carte {
            poser(&mut g, &db, "Composting Factory", &mut pol);
        }
        for nom in [
            "Power Plant", "Lichen", "Grass", "Moss", "Heather", "Comet",
            "Research", "Bushes", "Trees", "Great Dam", "Space Station",
            "Commercial District", "Circuit Board Factory",
        ] {
            en_main(&mut g, &db, nom);
        }
        assert_eq!(g.players[0].hand.len(), 13);
        g.players[0].mc = 0;
        g.players[0].plants = 0;
        g.players[0].heat = 0;
        pol.phase = 4; // production : déterministe, aucune décision de pose
        play_round(&mut g, &db, &mut pol);
        assert_eq!(g.players[0].hand.len(), HAND_LIMIT, "défaussé jusqu'à 10");
        g.players[0].mc
    }
    let sans = mc_apres(false);
    let avec = mc_apres(true);
    assert_eq!(
        avec - sans,
        3,
        "3 cartes défaussées × 1 MC de plus (sans={sans}, avec={avec})"
    );
}

#[test]
fn the_discard_rate_has_a_single_computation_point() {
    // I1, prouvé sur le MÉCANISME et pas sur les données du jour : le taux est
    // la constante du livret plus la somme des `discard_bonus` des cartes en
    // jeu. Aucune carte hors périmètre n'en porte.
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| e.discard_bonus != 0)
        .count();
    assert_eq!(n, 1, "une seule carte porte un supplément de défausse");
    assert_eq!(spec("Composting Factory").discard_bonus, 1);
}

// =========================================================================
// GROUPE B — le prix payé : Standard Technology
// =========================================================================

#[test]
fn standard_technology_reduces_the_three_mc_standard_actions() {
    // « You pay 4 MC less for standard actions that cost MC. » Les trois actions
    // standard payantes en MC : forêt 20, température 14, océan 15.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Standard Technology", &mut pol);
    let pl = &g.players[0];
    assert_eq!(standard_action_discount(&db, pl), 4);
    assert_eq!(standard_mc_cost(&db, pl, FOREST_MC_COST), 16, "forêt 20 → 16");
    assert_eq!(
        standard_mc_cost(&db, pl, TEMPERATURE_MC_COST),
        10,
        "température 14 → 10"
    );
    assert_eq!(standard_mc_cost(&db, pl, OCEAN_MC_COST), 11, "océan 15 → 11");
}

#[test]
fn standard_technology_leaves_the_prices_alone_without_the_card() {
    let db = db();
    let g = jeu(&db);
    let pl = &g.players[0];
    assert_eq!(standard_action_discount(&db, pl), 0);
    assert_eq!(standard_mc_cost(&db, pl, FOREST_MC_COST), FOREST_MC_COST);
    assert_eq!(standard_mc_cost(&db, pl, TEMPERATURE_MC_COST), TEMPERATURE_MC_COST);
    assert_eq!(standard_mc_cost(&db, pl, OCEAN_MC_COST), OCEAN_MC_COST);
}

#[test]
fn standard_technology_is_neutral_with_effects_off() {
    let db = db_off();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Standard Technology", &mut pol);
    assert_eq!(standard_action_discount(&db, &g.players[0]), 0, "I7");
}

#[test]
fn standard_technology_never_touches_the_actions_paid_in_plants_or_heat() {
    // NEVER 8. Le texte dit « standard actions THAT COST MC » : la forêt payée
    // en 8 plantes et la température payée en 8 chaleurs n'en coûtent pas.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Standard Technology", &mut pol);
    assert_eq!(
        engine::flow::forest_plant_cost(&db, &g.players[0]),
        FOREST_PLANT_COST,
        "la forêt en plantes coûte toujours 8 plantes"
    );
    // Le coût en chaleur est une constante de règle : la réduction ne la lit pas.
    assert_eq!(TEMPERATURE_HEAT_COST, 8);
}

#[test]
fn standard_technology_never_touches_the_card_sale() {
    // NEVER 8 : la vente de carte RAPPORTE des MC, elle n'en coûte pas.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Standard Technology", &mut pol);
    assert_eq!(
        discard_mc_rate(&db, &g.players[0]),
        SELL_CARD_MC,
        "Standard Technology ne change pas le taux de défausse"
    );
}

#[test]
fn standard_technology_never_touches_the_price_of_project_cards() {
    // Le texte vise les ACTIONS STANDARD, pas les cartes Projet. Le prix payé
    // d'une carte posée ensuite doit être son prix imprimé.
    let db = db();
    let r = seq(&db, &["Standard Technology", "Commercial District"], opts(400));
    assert_eq!(r.paid, vec![15, 25], "prix imprimés, aucune réduction de carte");
}

#[test]
fn standard_technology_makes_an_unaffordable_ocean_affordable() {
    // I2, cas de bascule : à 11 MC, l'action océan (15 MC) n'est PAS offerte
    // sans la carte, et l'est avec — puis elle est réellement payée. La
    // politique scriptée passe si l'action n'est pas offerte : c'est le témoin.
    // L'action « température » est choisie plutôt que « océan » : retourner une
    // tuile océan rapporte un bonus de tuile qui brouillerait la mesure des MC.
    fn temperature_apres(avec_carte: bool) -> (u8, i64) {
        let db = db();
        let mut pol = Scriptee::new();
        let mut g = jeu(&db);
        if avec_carte {
            poser(&mut g, &db, "Standard Technology", &mut pol);
        }
        g.players[0].mc = 11;
        g.players[0].plants = 0;
        g.players[0].heat = 0;
        pol.phase = 3;
        pol.actions.push_back(ActionOpt::TemperatureWithMc);
        play_round(&mut g, &db, &mut pol);
        (g.temperature, g.players[0].mc)
    }
    let (sans, mc_sans) = temperature_apres(false);
    assert_eq!(sans, 0, "11 MC < 14 : l'action n'est pas offerte");
    assert_eq!(mc_sans, 11, "et rien n'a été prélevé");
    let (avec, mc) = temperature_apres(true);
    assert_eq!(avec, 1, "11 MC ≥ 10 : l'action est offerte ET payée");
    assert_eq!(mc, 1, "les 10 MC réduits ont été réellement dépensés");
}

#[test]
fn standard_technology_increments_its_counter_at_the_payment_site() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Standard Technology", &mut pol);
    g.players[0].mc = 40;
    g.players[0].plants = 0;
    g.players[0].heat = 0;
    pol.phase = 3;
    pol.actions.push_back(ActionOpt::TemperatureWithMc);
    pol.actions.push_back(ActionOpt::TemperatureWithMc);
    assert_eq!(g.standard_action_discounts, 0);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(
        g.standard_action_discounts, 2,
        "une incrémentation par action standard réduite"
    );
    assert_eq!(g.players[0].mc, 40 - 10 - 10, "deux températures à 10 MC");
    assert_eq!(g.temperature, 2, "les deux actions ont bien eu lieu");
}

#[test]
fn the_counter_stays_at_zero_without_the_card() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    g.players[0].mc = 40;
    pol.phase = 3;
    pol.actions.push_back(ActionOpt::TemperatureWithMc);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.temperature, 1, "l'action a bien eu lieu");
    assert_eq!(g.standard_action_discounts, 0, "aucune réduction à compter");
    assert_eq!(g.players[0].mc, 40 - TEMPERATURE_MC_COST, "prix du livret");
}

#[test]
fn only_one_card_declares_a_standard_action_discount() {
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| e.standard_discount != 0)
        .count();
    assert_eq!(n, 1);
    assert_eq!(spec("Standard Technology").standard_discount, 4);
}

// =========================================================================
// GROUPE B — le prix payé : Restructured Resources
// =========================================================================

#[test]
fn restructured_resources_can_spend_one_plant_to_save_five_mc() {
    // « When you play a card, you may spend 1 plant to reduce that card's cost
    // by 5 MC. » Branche 0 = dépenser (l'option imprimée).
    let db = db();
    let o = ProbeOptions { mc: 400, plants: 5, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[0]);
    assert_eq!(r.delta.plants, -1, "une plante dépensée");
    // PIÈGE CONNU, annoncé par le contrat : `paid` est RECALCULÉ par la sonde
    // (prix imprimé moins les réductions FIXES) et ment dès qu'une réduction se
    // paie autrement qu'en MC. Il annonce donc encore 25. Le témoin fiable est
    // `delta` : la sonde réintègre `paid` dans `delta.mc`, si bien que l'écart
    // entre les deux — +5 — EST la réduction réellement obtenue.
    assert_eq!(r.paid, vec![7, 25], "le `paid` de la sonde ignore la réduction");
    assert_eq!(r.delta.mc, 5, "5 MC de moins réellement sortis de la réserve");
}

#[test]
fn restructured_resources_may_decline_the_plant() {
    // I4 : le « may » est un choix de `Policy`, jamais une convention câblée.
    // Branche 1 = renoncer.
    let db = db();
    let o = ProbeOptions { mc: 400, plants: 5, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[1]);
    assert_eq!(r.delta.plants, 0, "aucune plante dépensée");
    assert_eq!(r.delta.mc, 0, "prix imprimé payé en entier");
}

#[test]
fn restructured_resources_needs_a_plant_to_exist() {
    // Sans plante, aucune branche : la réduction n'est pas offerte, et le choix
    // scripté ne doit rien pouvoir y changer.
    let db = db();
    let o = ProbeOptions { mc: 400, plants: 0, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[0]);
    assert_eq!(r.delta.plants, 0, "0 plante : rien à dépenser");
    assert_eq!(r.delta.mc, 0, "0 plante : prix imprimé payé en entier");
}

#[test]
fn restructured_resources_service_reads_the_players_own_reserve() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    let cible = db.resolve_card("Commercial District").expect("Commercial District");
    assert_eq!(plant_discount(&g, &db, 0, cible), None, "pas la carte, pas de réduction");
    poser(&mut g, &db, "Restructured Resources", &mut pol);
    g.players[0].plants = 0;
    assert_eq!(plant_discount(&g, &db, 0, cible), None, "pas de plante, pas de réduction");
    g.players[0].plants = 1;
    assert_eq!(plant_discount(&g, &db, 0, cible), Some((1, 5)));
}

#[test]
fn restructured_resources_is_neutral_with_effects_off() {
    let db = db_off();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Restructured Resources", &mut pol);
    g.players[0].plants = 5;
    let cible = db.resolve_card("Commercial District").expect("Commercial District");
    assert_eq!(plant_discount(&g, &db, 0, cible), None, "I7");
}

#[test]
fn restructured_resources_never_spends_the_plants_a_card_owes() {
    // Le piège : *Moss* porte « Requires you to spend 1 plant ». Avec une seule
    // plante en réserve, la dépenser pour la réduction rendrait la pose
    // impayable. Le service met donc de côté ce que la carte visée doit.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Restructured Resources", &mut pol);
    let moss = db.resolve_card("Moss").expect("Moss");
    assert_eq!(plants_reserved_by(&db, moss), 1, "Moss doit dépenser 1 plante");
    g.players[0].plants = 1;
    assert_eq!(
        plant_discount(&g, &db, 0, moss),
        None,
        "la seule plante est réservée par la pose de Moss"
    );
    g.players[0].plants = 2;
    assert_eq!(
        plant_discount(&g, &db, 0, moss),
        Some((1, 5)),
        "une plante pour la pose, une pour la réduction"
    );
}

#[test]
fn restructured_resources_survives_a_full_run_of_moss() {
    // Contre-épreuve du précédent, par le CHEMIN RÉEL : la pose de Moss derrière
    // Restructured Resources ne doit jamais faire sauter l'assertion de dépense.
    let db = db();
    for plants in 1..=6i64 {
        for choix in [0usize, 1] {
            let o = ProbeOptions { mc: 400, plants, ..ProbeOptions::default() };
            let r = seq_choix(&db, &["Restructured Resources", "Moss"], o, &[choix]);
            assert!(r.played, "{plants} plantes, choix {choix} : Moss doit être posée");
            // Moss dépense 1 plante à la pose. Avec 2 plantes ou plus ET le
            // choix « dépenser », une seconde plante part pour la réduction.
            let attendu = if plants >= 2 && choix == 0 { -2 } else { -1 };
            assert_eq!(
                r.delta.plants, attendu,
                "{plants} plantes, choix {choix}"
            );
        }
    }
    // Et à 0 plante, la sonde ne force PAS une dépense de pose impayable : la
    // séquence s'arrête proprement au lieu de faire sauter une assertion.
    let o = ProbeOptions { mc: 400, plants: 0, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Moss"], o, &[0]);
    assert!(!r.played, "0 plante : Moss n'est pas posée");
}

#[test]
fn restructured_resources_is_seen_by_affordability_too() {
    // I2 : à 20 MC et une plante, Commercial District (25 MC) n'est payable
    // qu'avec la réduction. Le prédicat d'affordabilité doit la voir, sinon la
    // carte ne serait jamais proposée.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Restructured Resources", &mut pol);
    let cible = db.resolve_card("Commercial District").expect("Commercial District");
    g.players[0].plants = 1;
    let (p, a) = plant_discount(&g, &db, 0, cible).expect("réduction disponible");
    assert_eq!((p, a), (1, 5));
    assert!(
        !payable(20, 25),
        "sans la réduction : 20 MC < 25"
    );
    assert!(
        payable(20, 25 - a),
        "avec la réduction : 20 MC = 20"
    );
}

#[test]
fn restructured_resources_is_visible_to_the_probe_at_a_tight_budget() {
    // Défaut trouvé par la relecture adversariale : le garde-fou de payabilité
    // de la SONDE ne voyait pas la réduction en plantes, alors que
    // `flow::affordable` la voit. La sonde refusait donc de poser une carte que
    // la partie réelle proposerait — et au budget serré, c'est-à-dire le seul
    // cas où la réduction est OBLIGATOIRE, la dépense était indémontrable.
    //
    // 27 MC : Restructured Resources en coûte 7, il en reste 20, et
    // Commercial District (25) ne passe QUE grâce aux 5 MC de la plante.
    let db = db();
    let o = ProbeOptions { mc: 27, plants: 5, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[0]);
    assert!(r.played, "au budget exact, la carte doit être posable");
    assert_eq!(r.delta.plants, -1, "la plante a bien été dépensée");
}

#[test]
fn restructured_resources_is_forced_when_the_card_is_not_payable_without_it() {
    // Corollaire : à ce budget, la branche « renoncer » n'est PAS jouable, donc
    // elle n'est pas proposée — le choix scripté « 1 » ne doit rien pouvoir y
    // faire. C'est la convention du lot 3 : une branche injouable est filtrée
    // AVANT le choix, et à une seule branche on ne demande rien.
    let db = db();
    let o = ProbeOptions { mc: 27, plants: 5, ..ProbeOptions::default() };
    let r = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[1]);
    assert!(r.played, "la carte est posée");
    assert_eq!(
        r.delta.plants, -1,
        "la réduction est obligatoire : le « renoncement » n'était pas offert"
    );
}

#[test]
fn the_probe_reports_the_printed_price_and_delta_tells_the_truth() {
    // Convention conservée des lots précédents : `paid` ne rabat QUE les
    // réductions FIXES. Les réductions PAYANTES (microbes du lot 3, plantes de
    // ce lot) dépendent d'une décision du joueur — les rabattre dans `paid`
    // ferait mentir le champ dans l'autre sens dès que le joueur y renonce.
    // Le témoin est `delta`, et il bascule bien avec le choix.
    let db = db();
    let o = ProbeOptions { mc: 400, plants: 5, ..ProbeOptions::default() };
    let pris = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[0]);
    let refuse = seq_choix(&db, &["Restructured Resources", "Commercial District"], o, &[1]);
    assert_eq!(pris.paid, refuse.paid, "`paid` ne dépend pas de la décision");
    assert_eq!(pris.paid, vec![7, 25], "prix imprimés");
    assert_eq!(
        (pris.delta.mc - refuse.delta.mc, pris.delta.plants - refuse.delta.plants),
        (5, -1),
        "5 MC de moins sortis, 1 plante de plus dépensée"
    );
}

#[test]
fn restructured_resources_is_the_only_pay_plants_reduction() {
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| {
            e.reductions
                .iter()
                .any(|r| matches!(r, Reduction::PayPlants { .. }))
        })
        .count();
    assert_eq!(n, 1);
    assert_eq!(
        spec("Restructured Resources").reductions,
        &[Reduction::PayPlants { plants: 1, amount: 5 }]
    );
}

#[test]
fn a_pay_plants_reduction_is_never_granted_for_free() {
    // Comme `PayResources`, elle vaut 0 dans la somme des réductions FIXES :
    // sinon elle serait accordée sans que la plante soit dépensée.
    let r = Reduction::PayPlants { plants: 1, amount: 5 };
    assert_eq!(r.amount_for(&[Tag::Building], 25), 0);
    assert_eq!(r.amount_for(&[], 3), 0);
    assert_eq!(r.capacity_units(), None, "ce n'est pas un savoir-faire");
}

// =========================================================================
// GROUPE C — Adaptation Technology
// =========================================================================

#[test]
fn adaptation_technology_softens_a_requirement_by_one_color_step() {
    // Témoin du contrat : *Bushes* exige le palier ROUGE de température, la
    // sonde démarre au palier VIOLET. Sans rien, refusé ; avec la carte, permis.
    let db = db();
    assert!(!seq(&db, &["Bushes"], opts(400)).prereq_ok, "témoin : refusé");
    assert!(
        seq(&db, &["Adaptation Technology", "Bushes"], opts(400)).prereq_ok,
        "±1 palier : Bushes devient jouable"
    );
}

#[test]
fn adaptation_technology_never_softens_by_two_color_steps() {
    // I3. *Trees* exige le palier JAUNE, soit DEUX paliers au-dessus du violet.
    let db = db();
    assert!(
        !seq(&db, &["Adaptation Technology", "Trees"], opts(400)).prereq_ok,
        "deux paliers : refusé"
    );
}

#[test]
fn adaptation_technology_and_inventrix_together_still_give_one_step() {
    // « This cannot be modified further by other effects » : la souplesse est
    // BINAIRE. Réunies, les deux sources ne donnent pas ±2.
    let db = db();
    assert!(
        seq_corp(&db, "Inventrix", &["Adaptation Technology", "Bushes"]).prereq_ok,
        "un palier : permis"
    );
    assert!(
        !seq_corp(&db, "Inventrix", &["Adaptation Technology", "Trees"]).prereq_ok,
        "deux paliers : refusé même avec les deux effets réunis"
    );
}

#[test]
fn inventrix_alone_still_behaves_exactly_as_before_the_lot() {
    // Non-régression du mécanisme d'origine.
    let db = db();
    assert!(seq_corp(&db, "Inventrix", &["Bushes"]).prereq_ok);
    assert!(!seq_corp(&db, "Inventrix", &["Trees"]).prereq_ok);
}

#[test]
fn adaptation_technology_never_softens_an_ocean_requirement() {
    // Le texte imprimé dit « the oxygen or temperature ». Rien d'autre.
    // *Great Dam* exige 2 océans retournés ; il n'y en a aucun au départ.
    let db = db();
    assert!(!seq(&db, &["Adaptation Technology", "Great Dam"], opts(400)).prereq_ok);
    assert!(!seq_corp(&db, "Inventrix", &["Adaptation Technology", "Great Dam"]).prereq_ok);
}

#[test]
fn adaptation_technology_never_softens_a_tag_requirement() {
    // *Fusion Power* exige 2 badges Énergie.
    let db = db();
    assert!(!seq(&db, &["Adaptation Technology", "Fusion Power"], opts(400)).prereq_ok);
}

#[test]
fn adaptation_technology_never_softens_a_spend_requirement() {
    // *Tropical Resort* exige de dépenser 5 chaleurs : ce n'est pas un palier.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Adaptation Technology", &mut pol);
    let id = db.resolve_card("Tropical Resort").expect("Tropical Resort");
    g.players[0].heat = 4;
    assert!(!requirements_met(&g, &db, 0, id), "4 chaleurs < 5 : refusé");
    g.players[0].heat = 5;
    assert!(requirements_met(&g, &db, 0, id), "5 chaleurs : permis");
}

#[test]
fn adaptation_technology_softens_an_oxygen_requirement_too() {
    // Le texte nomme l'oxygène AUSSI. *Breathing Filters* exige le palier JAUNE
    // d'oxygène (niveau 7) ; au départ l'oxygène est au palier VIOLET (0), soit
    // deux paliers plus bas — refusé. Au palier ROUGE (niveau 3), la souplesse
    // d'un palier suffit.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Adaptation Technology", &mut pol);
    let id = db.resolve_card("Breathing Filters").expect("Breathing Filters");
    g.oxygen = 0;
    g.snapshot_planet();
    assert!(!requirements_met(&g, &db, 0, id), "violet → jaune : deux paliers");
    g.oxygen = 3;
    g.snapshot_planet();
    assert!(requirements_met(&g, &db, 0, id), "rouge → jaune : un palier");
}

#[test]
fn adaptation_technology_never_helps_the_opponent() {
    // NEVER 9 : la souplesse se lit sur les cartes du joueur qui joue.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Adaptation Technology", &mut pol);
    let bushes = db.resolve_card("Bushes").expect("Bushes");
    assert!(requirements_met(&g, &db, 0, bushes), "le porteur en profite");
    assert!(
        !requirements_met(&g, &db, 1, bushes),
        "l'adversaire n'en profite pas"
    );
}

#[test]
fn adaptation_technology_is_the_only_card_carrying_the_flex() {
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| e.req_color_flex)
        .count();
    assert_eq!(n, 1, "une seule carte porte la souplesse de prérequis");
    assert!(spec("Adaptation Technology").req_color_flex);
}

#[test]
fn adaptation_technology_leaves_requirements_untouched_with_effects_off() {
    // Note calibrée par le contrat : avec `--effects off`, TOUS les prérequis
    // sont jugés satisfaits, la couche d'effets étant coupée. On vérifie donc
    // seulement que la carte n'y change rien — pas qu'elle y bloque.
    let db = db_off();
    assert!(seq(&db, &["Bushes"], opts(400)).prereq_ok);
    assert!(seq(&db, &["Adaptation Technology", "Bushes"], opts(400)).prereq_ok);
    assert!(seq(&db, &["Adaptation Technology", "Trees"], opts(400)).prereq_ok);
}

#[test]
fn the_probe_reads_prereq_ok_just_before_the_last_card_is_played() {
    // Journal §D2. La distinction entre les deux lectures est CONSERVÉE : les
    // prérequis de PARAMÈTRE restent jugés sur l'INSTANTANÉ de début de phase,
    // les autres à l'état courant. Trois cartes qui montent la température de
    // 3+2+2 = 7 niveaux suffisent au palier rouge de *Bushes* à l'état COURANT,
    // jamais sur l'instantané (qui reste à 0 pour toute la sonde).
    let db = db();
    let r = seq(
        &db,
        &["Deimos Down", "Giant Ice Asteroid", "Lava Flows", "Bushes"],
        opts(400),
    );
    assert_eq!(r.delta.temperature, 7, "la température a bien monté");
    assert!(r.prereq_ok_now, "état courant : palier rouge atteint");
    assert!(
        !r.prereq_ok,
        "instantané de début de phase : toujours au palier violet"
    );
}

// =========================================================================
// GROUPE C — Assembly Lines
// =========================================================================

#[test]
fn assembly_lines_gains_one_mc_when_a_card_action_is_used() {
    // « When you use an "Action:" effect on one of your cards, gain 1 MC. »
    // *Circuit Board Factory* : « Action: Draw a card », sans coût.
    let db = db();
    let sans = act(&db, &["Circuit Board Factory"], opts(400));
    assert!(sans.action_applied, "l'action a bien eu lieu");
    assert_eq!(sans.delta.mc, 0, "sans Assembly Lines : aucun MC");

    let avec = act(&db, &["Assembly Lines", "Circuit Board Factory"], opts(400));
    assert!(avec.action_applied);
    assert_eq!(avec.delta.mc, 1, "avec Assembly Lines : +1 MC");
    assert_eq!(avec.delta.hand, sans.delta.hand, "l'action elle-même est inchangée");
}

#[test]
fn assembly_lines_fires_on_an_action_that_costs_something() {
    // *Development Center* : « Action: Spend 2 heat to draw a card. »
    let db = db();
    let sans = act(&db, &["Development Center"], opts(400));
    let avec = act(&db, &["Assembly Lines", "Development Center"], opts(400));
    assert!(sans.action_applied && avec.action_applied);
    assert_eq!(avec.delta.heat, sans.delta.heat, "le coût reste le même");
    assert_eq!(avec.delta.mc - sans.delta.mc, 1, "+1 MC");
}

#[test]
fn assembly_lines_does_not_fire_when_the_action_cannot_be_paid() {
    // « When you USE an action » : une activation qui échoue n'est pas un usage.
    // *Think Tank* coûte 2 MC ; à 0 MC l'action n'a pas lieu.
    let db = db();
    let r = act(
        &db,
        &["Assembly Lines", "Think Tank"],
        ProbeOptions { mc: 0, ..ProbeOptions::default() },
    );
    assert!(!r.action_applied, "coût impayable : aucune activation");
    assert_eq!(r.delta.mc, 0, "aucun MC gagné");
}

#[test]
fn assembly_lines_never_fires_on_a_standard_action() {
    // « on one of YOUR CARDS » : les actions standard (forêt, température,
    // océan, vente de carte) ne sont pas des actions de carte.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    poser(&mut g, &db, "Assembly Lines", &mut pol);
    g.players[0].mc = 40;
    g.players[0].plants = 0;
    g.players[0].heat = 0;
    pol.phase = 3;
    pol.actions.push_back(ActionOpt::TemperatureWithMc);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.temperature, 1, "l'action standard a eu lieu");
    assert_eq!(g.action_mc_bonuses, 0, "aucun MC d'Assembly Lines");
    assert_eq!(
        g.players[0].mc,
        40 - TEMPERATURE_MC_COST,
        "pas un MC de plus"
    );
}

#[test]
fn assembly_lines_increments_its_counter_at_the_trigger_site() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    let cbf = poser(&mut g, &db, "Circuit Board Factory", &mut pol);
    poser(&mut g, &db, "Assembly Lines", &mut pol);
    g.players[0].mc = 0;
    g.players[0].plants = 0;
    g.players[0].heat = 0;
    pol.phase = 3;
    pol.actions.push_back(ActionOpt::BlueAction(cbf));
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.action_mc_bonuses, 1, "un MC compté, une fois");
    assert!(g.players[0].mc >= 1, "le MC a bien été crédité");
}

#[test]
fn assembly_lines_counter_stays_at_zero_with_effects_off() {
    // I7 : les effets coupés, aucune action de carte n'est appliquée.
    let db = db_off();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 100, 2024, &mut pol);
    assert_eq!(s.action_mc_bonuses, 0);
    assert_eq!(s.standard_action_discounts, 0);
}

#[test]
fn assembly_lines_is_the_only_card_carrying_an_action_trigger() {
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| !e.action_trigger.is_empty())
        .count();
    assert_eq!(n, 1);
    assert_eq!(
        spec("Assembly Lines").action_trigger,
        &[engine::effects::ActionEff::Mc(1)]
    );
}

#[test]
fn assembly_lines_has_no_action_of_its_own() {
    // Elle ne se déclenche donc jamais sur elle-même : le texte imprimé ne lui
    // donne aucune ligne « Action: ».
    assert!(spec("Assembly Lines").action.is_none());
}

// =========================================================================
// GROUPE C — Mars University
// =========================================================================

/// Prépare une partie où *Mars University* est en jeu et la main contient
/// exactement les cartes nommées.
fn universite(db: &CardsDb, pol: &mut Scriptee, main: &[&str]) -> GameState {
    let mut g = jeu(db);
    // Posée AVANT de garnir la main : son déclencheur « including this » ne
    // trouve alors aucune carte à défausser, l'état reste net.
    poser(&mut g, db, "Mars University", pol);
    for nom in main {
        en_main(&mut g, db, nom);
    }
    g
}

#[test]
fn mars_university_triggers_on_its_own_science_tag() {
    // « When you play a [science], INCLUDING THIS ». Main garnie AVANT la pose :
    // le déclencheur doit trouver de quoi défausser.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = jeu(&db);
    for nom in ["Power Plant", "Lichen", "Grass"] {
        en_main(&mut g, &db, nom);
    }
    pol.choix.push_back(0); // « may » : défausser
    pol.defausses.push_back(0); // Power Plant, sans badge plante
    let avant = g.players[0].hand.len();
    assert_eq!(avant, 3);
    poser(&mut g, &db, "Mars University", &mut pol);
    // `poser` fait entrer la carte en main puis la joue : net nul. Reste le
    // déclencheur — −1 défaussée, +1 piochée (Power Plant n'a pas de badge
    // plante) — donc une main inchangée. Le témoin est ailleurs : la politique
    // a bien été consultée, et la carte défaussée est partie à la défausse.
    assert_eq!(g.players[0].hand.len(), 3, "−1 défaussée, +1 piochée");
    assert!(pol.choix.is_empty(), "le « may » a bien été posé au joueur");
    assert!(pol.defausses.is_empty(), "une carte a bien été choisie");
    let pp = db.resolve_card("Power Plant").expect("Power Plant");
    assert!(!g.players[0].hand.contains(&pp), "Power Plant a été défaussée");
    assert!(g.discard.contains(&pp), "…et elle est bien dans la défausse");
}

#[test]
fn mars_university_draws_two_cards_when_the_discarded_card_had_a_plant() {
    // « If that card had a [plant], draw two cards. » Le badge regardé est celui
    // de la carte DÉFAUSSÉE.
    let db = db();
    let mut pol = Scriptee::new();
    // *Lichen* porte un badge plante ; *Interns* un badge science, aucun plante.
    let mut g = universite(&db, &mut pol, &["Lichen", "Power Plant"]);
    assert_eq!(g.players[0].hand.len(), 2);
    pol.choix.push_back(0);
    pol.defausses.push_back(0); // Lichen
    poser(&mut g, &db, "Interns", &mut pol);
    // Main 2 → Interns entre et sort (net nul) → −1 défaussée → +2 piochées = 3.
    assert_eq!(
        g.players[0].hand.len(),
        3,
        "badge plante : deux cartes piochées"
    );
}

#[test]
fn mars_university_draws_one_card_otherwise() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &["Lichen", "Power Plant"]);
    pol.choix.push_back(0);
    pol.defausses.push_back(1); // Power Plant : building + energy, aucun plante
    poser(&mut g, &db, "Interns", &mut pol);
    // Main 2 → net nul pour Interns → −1 défaussée → +1 piochée = 2. C'est
    // UNE carte de moins que la branche « badge plante » du test précédent :
    // c'est exactement la différence que le texte imprimé décrit.
    assert_eq!(g.players[0].hand.len(), 2, "aucun badge plante : une carte piochée");
}

#[test]
fn mars_university_may_decline_the_discard() {
    // I4 : « you MAY discard a card ». Branche 1 = renoncer, rien ne se passe.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &["Lichen", "Power Plant"]);
    pol.choix.push_back(1);
    poser(&mut g, &db, "Interns", &mut pol);
    assert_eq!(g.players[0].hand.len(), 2, "rien défaussé, rien pioché");
}

#[test]
fn mars_university_asks_nothing_when_the_hand_is_empty() {
    // Convention du lot 3 : une branche injouable est filtrée AVANT le choix ;
    // à zéro branche jouable, aucune question n'est posée. Le script de choix
    // reste donc intact — c'est le témoin.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &[]);
    pol.choix.push_back(0);
    poser(&mut g, &db, "Interns", &mut pol);
    assert_eq!(g.players[0].hand.len(), 0);
    assert_eq!(
        pol.choix.len(),
        1,
        "main vide : la politique n'a pas été consultée"
    );
}

#[test]
fn mars_university_ignores_a_card_without_a_science_tag() {
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &["Lichen", "Power Plant"]);
    pol.choix.push_back(0);
    pol.defausses.push_back(0);
    // *Commercial District* : badge bâtiment seulement.
    poser(&mut g, &db, "Commercial District", &mut pol);
    assert_eq!(g.players[0].hand.len(), 2, "aucun badge science : rien");
    assert_eq!(pol.choix.len(), 1, "la politique n'a pas été consultée");
}

#[test]
fn mars_university_resolves_once_per_science_tag() {
    // Livret p.9 l.106 : « Si la condition d'un effet est remplie plusieurs fois
    // lorsqu'une carte est jouée, résolvez l'effet correspondant plusieurs
    // fois. » *Research* porte DEUX badges science.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &["Lichen", "Power Plant", "Grass"]);
    pol.choix.push_back(1); // renoncer
    pol.choix.push_back(1); // renoncer une seconde fois
    poser(&mut g, &db, "Research", &mut pol);
    assert!(
        pol.choix.is_empty(),
        "deux badges science : deux consultations de la politique"
    );
    // `Research` porte aussi « Draw 2 cards » à la pose ; les deux résolutions
    // du déclencheur ayant été refusées, la main ne bouge que de ces 2 pioches.
    assert_eq!(g.players[0].hand.len(), 5, "seuls les effets propres ont joué");
}

#[test]
fn mars_university_never_fires_for_the_opponent() {
    // NEVER 9 : le déclencheur est lu sur les cartes du joueur qui joue.
    let db = db();
    let mut pol = Scriptee::new();
    let mut g = universite(&db, &mut pol, &[]);
    // Le joueur 1 joue une carte à badge science : rien ne doit lui arriver.
    let id = db.resolve_card("Interns").expect("Interns");
    if let Some(i) = g.deck.iter().position(|&c| c == id) {
        g.deck.remove(i);
    }
    g.players[1].hand.push(id);
    g.players[1].mc = 1000;
    let avant = g.players[1].hand.len();
    build_card_with(&mut g, &db, 1, 0, 0, &mut pol);
    assert_eq!(
        g.players[1].hand.len() as i64 - avant as i64,
        -1,
        "l'adversaire ne profite pas de Mars University"
    );
}

#[test]
fn mars_university_declares_its_trigger_exactly_once() {
    let t = spec("Mars University").play_triggers;
    assert_eq!(t.len(), 1, "un seul déclencheur");
    assert!(t[0].include_self, "« including this »");
    assert!(t[0].scale_by_matched_tags, "une résolution par badge science");
    assert_eq!(t[0].gains.len(), 1);
}

// =========================================================================
// STRUCTURE, PÉRIMÈTRE, NON-RÉGRESSION
// =========================================================================

#[test]
fn the_table_has_exactly_one_entry_per_card_of_the_lot() {
    use engine::effects::LOT1;
    for nom in LOT7 {
        let n = LOT1.iter().filter(|(x, _)| *x == nom).count();
        assert_eq!(n, 1, "« {nom} » : une entrée et une seule");
    }
}

#[test]
fn the_nine_cards_resolve_to_the_base_box_deck() {
    let db = db();
    for nom in LOT7 {
        let id = db
            .resolve_card(nom)
            .unwrap_or_else(|| panic!("« {nom} » non résolue"));
        let c = &db.projects[id as usize];
        assert!(c.in_deck_v1, "« {nom} » doit venir de la boîte de base");
        assert!(c.effect.is_some(), "« {nom} » doit être encodée");
        assert!(c.effets_geres(), "« {nom} » doit être déclarée gérée");
    }
}

#[test]
fn the_nine_prices_match_the_printed_cost() {
    // Contrôle croisé INDÉPENDANT de l'encodage : le prix réellement payé par la
    // sonde doit être le coût imprimé relevé sur les cartons. Si une entrée
    // résolvait vers un homonyme, le prix trahirait la substitution.
    let db = db();
    for (nom, cout) in [
        ("Interns", 3),
        ("Extended Resources", 10),
        ("United Planetary Alliance", 11),
        ("Composting Factory", 13),
        ("Standard Technology", 15),
        ("Restructured Resources", 7),
        ("Adaptation Technology", 12),
        ("Assembly Lines", 13),
        ("Mars University", 10),
    ] {
        let r = seq(&db, &[nom], opts(400));
        assert_eq!(r.paid, vec![cout], "« {nom} » : coût imprimé");
    }
}

#[test]
fn the_nine_cards_carry_no_requirement_and_no_immediate_effect() {
    // Aucune des neuf n'a de prérequis ni d'effet de pose : ce sont des
    // MODIFICATEURS PERMANENTS. Un encodage qui en porterait un serait un
    // débordement silencieux.
    for nom in LOT7 {
        let e = spec(nom);
        assert!(e.reqs.is_empty(), "« {nom} » ne doit porter aucun prérequis");
        assert!(e.effects.is_empty(), "« {nom} » ne doit rien gagner à la pose");
        assert!(e.action.is_none(), "« {nom} » n'a pas d'action de carte");
        assert!(e.holds.is_none(), "« {nom} » ne porte pas de ressources");
        assert!(e.prod.is_none(), "« {nom} » n'a pas de production dérivée");
    }
}

#[test]
fn the_five_out_of_scope_cards_are_now_encoded_by_the_next_lot() {
    // (lot cartes-8) Ce test interdisait au lot 7 d'encoder les cinq cartes
    // « une carte de plus » — un garde-fou de PÉRIMÈTRE, respecté à l'époque.
    // Le lot suivant les a encodées, comme prévu : le témoin est donc RETOURNÉ,
    // et il devient l'épinglage inverse. Il n'est pas supprimé, parce qu'il
    // continue de prouver que ces cinq cartes-là sont bien celles qui ont
    // changé de camp, et qu'aucune n'a été oubliée en route.
    let db = db();
    for nom in HORS {
        let id = db
            .resolve_card(nom)
            .unwrap_or_else(|| panic!("« {nom} » non résolue"));
        assert!(
            db.projects[id as usize].effect.is_some(),
            "« {nom} » doit être encodée depuis le lot cartes-8"
        );
    }
}

#[test]
fn no_card_remains_unhandled_in_the_base_box() {
    // (lot cartes-8) L'attente passe de « exactement ces cinq-là » à
    // « plus aucune » : la boîte de base est intégralement encodée. C'est
    // l'assertion la plus forte que ce test puisse porter, et la moindre
    // régression la fait échouer EN NOMMANT la carte fautive.
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let muettes: std::collections::BTreeSet<&str> = db
        .recensement()
        .into_iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    assert!(muettes.is_empty(), "encore muettes en boîte de base : {muettes:?}");
}

#[test]
fn the_five_cards_of_the_next_lot_are_live_and_keep_no_research_bonus() {
    // (lot cartes-8) Elles ne sont plus des coquilles vides : elles sont dans
    // le lot. Ce que le test continue d'affirmer sans rien perdre : elles
    // restent JOUABLES (aucune ne casse la pose), et aucune n'a hérité au
    // passage d'un bonus de phase Recherche qu'elle n'a jamais eu.
    let db = db();
    for nom in HORS {
        let r = seq(&db, &[nom], opts(400));
        assert!(r.played, "« {nom} » doit rester jouable");
        assert!(r.in_lot, "« {nom} » est encodée depuis le lot cartes-8");
        assert_eq!(r.research, (0, 0), "« {nom} » n'a pas de bonus de recherche");
    }
}

#[test]
fn no_earlier_lot_card_gained_a_permanent_modifier() {
    // Aucune régression par effet de bord : les cartes des lots précédents ne
    // portent aucun des quatre champs neufs.
    for nom in [
        "Media Group", "Tardigrades", "Birds", "Io Mining Industries",
        "Volcanic Pools", "Lichen", "Comet", "Interplanetary Relations",
        "Advanced Alloys", "Solarpunk", "Olympus Conference", "Think Tank",
    ] {
        let e = spec(nom);
        assert_eq!(e.discard_bonus, 0, "« {nom} »");
        assert_eq!(e.standard_discount, 0, "« {nom} »");
        assert!(!e.req_color_flex, "« {nom} »");
        assert!(e.action_trigger.is_empty(), "« {nom} »");
    }
}

#[test]
fn a_thousand_games_run_clean_in_both_box_configurations() {
    for boites in ["base", "base,decouverte"] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(boites).unwrap())
            .unwrap_or_else(|_| panic!("{boites}"));
        let mut pol = RandomPolicy;
        let s = run_simulation(&db, 1000, 2024, &mut pol);
        assert_eq!(s.completed, 1000, "{boites} : parties complétées");
        assert_eq!(s.invariant_violations, 0, "{boites} : violations");
        assert_eq!(s.truncated, 0, "{boites} : parties tronquées");
    }
}

#[test]
fn the_two_new_counters_are_strictly_positive_in_real_play() {
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 2024, &mut pol);
    assert!(
        s.standard_action_discounts > 0,
        "Standard Technology ne s'exécute jamais en partie"
    );
    assert!(
        s.action_mc_bonuses > 0,
        "Assembly Lines ne s'exécute jamais en partie"
    );
}

#[test]
fn the_lot_lowers_the_unhandled_effects_counter() {
    // Le contrat mesure 3154 avant le lot, sur 1000 parties graine 2024.
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 2024, &mut pol);
    assert!(
        s.cards_effects_unhandled < 3154,
        "effets non gérés : {} (était 3154)",
        s.cards_effects_unhandled
    );
    assert!(
        s.research_extra_draws > 3888,
        "bonus de recherche : {} (était 3888)",
        s.research_extra_draws
    );
}

#[test]
fn the_simulation_stays_deterministic_at_a_fixed_seed() {
    let db = db();
    let mut a = RandomPolicy;
    let mut b = RandomPolicy;
    let s1 = run_simulation(&db, 50, 4242, &mut a);
    let s2 = run_simulation(&db, 50, 4242, &mut b);
    assert_eq!(s1.state_hash, s2.state_hash);
    assert_eq!(s1.standard_action_discounts, s2.standard_action_discounts);
    assert_eq!(s1.action_mc_bonuses, s2.action_mc_bonuses);
    assert_eq!(s1.research_extra_draws, s2.research_extra_draws);
}

#[test]
fn the_probe_stays_deterministic_too() {
    let db = db();
    let a = seq(&db, &["Interns", "Extended Resources"], opts(400));
    let b = seq(&db, &["Interns", "Extended Resources"], opts(400));
    assert_eq!(a.research, b.research);
    assert_eq!(a.delta, b.delta);
    assert_eq!(a.paid, b.paid);
}

#[test]
fn probe_plants_only_changes_the_plant_reserve() {
    // `--probe-plants` est calqué sur `--probe-mc` : il fixe l'état de départ,
    // rien d'autre. Le défaut (20) reproduit le comportement des lots
    // précédents, bit à bit.
    let db = db();
    let defaut = seq(&db, &["Power Plant"], opts(400));
    let explicite = seq(
        &db,
        &["Power Plant"],
        ProbeOptions { mc: 400, plants: 20, ..ProbeOptions::default() },
    );
    assert_eq!(defaut.delta, explicite.delta, "20 plantes = le défaut historique");
    let autre = seq(
        &db,
        &["Power Plant"],
        ProbeOptions { mc: 400, plants: 0, ..ProbeOptions::default() },
    );
    assert_eq!(autre.delta, defaut.delta, "le delta ne dépend pas de la réserve");
}

#[test]
fn an_unknown_card_name_is_reported_as_not_found() {
    // La garde anti-contrôle-creux, vérifiée depuis les tests aussi : un nom
    // inexistant ne doit JAMAIS passer pour trouvé.
    let db = db();
    let r = run_probe_seq_corp(
        &db,
        &["Carte Qui N Existe Pas"],
        opts(400),
        &ProbeScript::default(),
        false,
        None,
    );
    assert!(!r.found);
    assert_eq!(r.research, (0, 0));
    // Et une corporation mal orthographiée est signalée non installée.
    let c = run_probe_seq_corp(
        &db,
        &["Bushes"],
        opts(400),
        &ProbeScript::default(),
        false,
        Some("Inventryx"),
    );
    let corp = c.corp.as_ref().expect("objet corp");
    assert!(!corp.found, "une corporation inconnue doit être signalée");
}

#[test]
fn the_speed_stays_above_the_contract_floor() {
    // Le contrat exige au moins 7 000 parties/s sur 1 000 parties.
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 2024, &mut pol);
    assert!(
        s.games_per_sec > 3000.0,
        "vitesse effondrée : {:.0} parties/s",
        s.games_per_sec
    );
}


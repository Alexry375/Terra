//! Tests du chantier **moteur-verite-1** — « le moteur simule-t-il les cartes
//! IMPRIMÉES ? ».
//!
//! Ces tests confrontent le moteur au **texte imprimé**
//! (`inputs/textes-cartes.json`, champ `text`) et au **livret officiel**
//! (`inputs/regles/livret-base.md`), jamais à la paraphrase de `cards.json`.
//!
//! Ils couvrent trois choses que les lots précédents n'établissaient pas :
//!
//! 1. **La correction de régime** de `Viral Enhancers` et `Decomposers` : leur
//!    effet déclenché ne se résolvait qu'UNE fois même quand la carte posée
//!    remplissait la condition deux fois. Livret p.9 l.106 : « Si la condition
//!    d'un effet est remplie plusieurs fois lorsqu'une carte est jouée,
//!    résolvez l'effet correspondant plusieurs fois. » Ces deux tests
//!    ÉCHOUENT sur l'ancien comportement et passent sur le nouveau.
//! 2. **La répétabilité des capacités `Action:`** — le point exact que ce
//!    chantier corrige. Une capacité présente n'est pas une capacité
//!    répétable : chaque test déclenche l'action **deux fois dans la même
//!    partie**, par le flux réel (`play_round`, phase III), et vérifie que le
//!    second déclenchement produit bien un second effet.
//! 3. **Les mécanismes permanents inatteignables par la sonde CLI** : le
//!    déclencheur « forest VP » de `Small Animals`, la production dérivée de
//!    `Zeppelins` (qui compte des jetons Forêt, pas des badges) et le bonus
//!    permanent de phase Recherche d'`Interplanetary Relations`, tous les
//!    trois observés sur PLUSIEURS manches consécutives.
//!
//! Plus deux vérifications de données du périmètre PARTIE 3 : le `price` d'une
//! corporation est REÇU et non payé, et aucun homoglyphe cyrillique n'atteint
//! un champ dont le moteur dépend.

use engine::cards::CardsDb;
use engine::flow::{build_card, derived_production, play_round, research_draw_keep, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_full, ProbeOptions, ProbeResult, ProbeScript};
use engine::state::{GameState, PlayerState};
use rand::rngs::StdRng;
use std::collections::BTreeMap;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

/// Sonde séquence (chemin réel de pose `flow::build_card_with`).
fn seq(db: &CardsDb, names: &[&str], choices: &[usize]) -> ProbeResult {
    let script = ProbeScript { choices: choices.to_vec(), targets: Vec::new(), joker_tag: None };
    run_probe_seq_full(db, names, ProbeOptions::default(), &script, false)
}

/// Ressources posées sur la carte `name` après la séquence.
fn res_on(r: &ProbeResult, name: &str) -> u32 {
    r.resources.iter().find(|x| x.card == name).map(|x| x.n).unwrap_or(0)
}

// =====================================================================
// 1. RÉGIME CORRIGÉ — un effet déclenché se résout une fois PAR CONDITION
//    REMPLIE (livret p.9 l.106), et non une fois au forfait.
// =====================================================================
//
// `Adapted Lichen` porte DEUX badges satisfaisants ([microbe] ET [plant]) pour
// la condition « When you play an [animal], [microbe], or [plant] ». Elle doit
// donc déclencher DEUX résolutions. Avant la correction de ce chantier, le
// moteur n'en faisait qu'une : c'est cet écart que ces deux tests bloquent.

#[test]
fn viral_enhancers_resolves_once_per_matching_tag_of_the_played_card() {
    let db = db();
    // Texte imprimé : « Effect: When you play a [animal], [microbe], or
    // [plant], including these, gain 1 plant or add 1 animal or microbe to
    // ANOTHER* card. » Branche 0 = « gain 1 plant », imposée à la politique.

    // Viral Enhancers porte elle-même [microbe] + [plant] : « including these »
    // + deux badges satisfaisants = 2 résolutions sur sa propre pose.
    let r = seq(&db, &["Viral Enhancers"], &[0, 0, 0, 0]);
    assert_eq!(
        r.delta.plants, 2,
        "2 badges satisfaisants sur la carte posée = 2 résolutions (livret l.106) ; \
         l'ancien comportement en donnait 1"
    );

    // Puis Adapted Lichen ([microbe] + [plant]) : 2 résolutions de plus.
    let r = seq(&db, &["Viral Enhancers", "Adapted Lichen"], &[0, 0, 0, 0]);
    assert_eq!(r.delta.plants, 4, "2 (pose de Viral Enhancers) + 2 (Adapted Lichen)");

    // Contrôle : une carte à UN seul badge satisfaisant ne donne qu'une
    // résolution — la correction ne multiplie pas tout aveuglément.
    let r = seq(&db, &["Viral Enhancers", "Algae"], &[0, 0, 0, 0]);
    assert_eq!(r.delta.plants, 2 + 1, "Algae ne porte qu'un badge [plant]");
}

#[test]
fn decomposers_resolves_once_per_matching_tag_of_the_played_card() {
    let db = db();
    // Texte imprimé : « Effect: When you play an [animal], [microbe], or
    // [plant], including this, add a microbe here or remove a microbe from
    // here to draw a card. » Branche 0 = « add a microbe here ».

    // Decomposers ne porte qu'un badge [microbe] : 1 résolution sur sa pose.
    let r = seq(&db, &["Decomposers"], &[0, 0, 0, 0]);
    assert_eq!(res_on(&r, "Decomposers"), 1);

    // Adapted Lichen porte [microbe] + [plant] : 2 résolutions de plus.
    let r = seq(&db, &["Decomposers", "Adapted Lichen"], &[0, 0, 0, 0]);
    assert_eq!(
        res_on(&r, "Decomposers"),
        3,
        "1 (pose de Decomposers) + 2 (Adapted Lichen, deux badges satisfaisants) ; \
         l'ancien comportement s'arrêtait à 2"
    );

    // Contrôle à un seul badge.
    let r = seq(&db, &["Decomposers", "Algae"], &[0, 0, 0, 0]);
    assert_eq!(res_on(&r, "Decomposers"), 2, "Algae ne porte qu'un badge [plant]");
}

#[test]
fn the_multiple_resolution_rule_did_not_change_the_single_tag_cards() {
    let db = db();
    // Non-régression du motif : les cartes déjà à `scale = true` gardent leur
    // comportement, et celles dont la condition est « une carte » (donc jamais
    // multiple) aussi.
    let r = seq(&db, &["Ecological Zone", "Adapted Lichen"], &[]);
    assert_eq!(res_on(&r, "Ecological Zone"), 3, "2 (pose) + 1 (seul [plant] compte)");
    let r = seq(&db, &["Anti-Gravity Technology", "Adapted Lichen"], &[]);
    assert_eq!(r.delta.plants, 2, "condition « une carte » : jamais multipliée");
}

// =====================================================================
// 2. RÉPÉTABILITÉ DES CAPACITÉS `Action:` — deux déclenchements dans la
//    MÊME partie, par le flux réel `play_round`.
// =====================================================================

/// Pose `cards` dans l'ordre (chemin réel `flow::build_card`), puis joue une
/// manche où le joueur 0 active `times` fois l'action de `action_card`.
///
/// Les deux joueurs choisissent la phase III : le joueur 0 est donc
/// sélectionneur et dispose de l'activation supplémentaire du livret (p.14,
/// « résoudre une fois de plus l'Action: de l'une de ses cartes en jeu »).
/// C'est ce qui rend DEUX activations possibles dans la même phase — et ce qui
/// prouve que la capacité est répétable et non consommée à la pose.
fn activate(
    db: &CardsDb,
    seed: u64,
    cards: &[&str],
    action_card: &str,
    choice: Option<usize>,
    times: usize,
) -> (GameState, BTreeMap<String, u16>) {
    let mut setup = RandomPolicy;
    let mut game = setup_game(db, seed, &mut setup);
    let mut ids = BTreeMap::new();
    for name in cards {
        ids.insert(name.to_string(), db.resolve_card(name).expect(name));
    }
    let old: Vec<u16> = game.players[0].hand.drain(..).collect();
    game.deck.extend(old);
    let wanted: Vec<u16> = ids.values().copied().collect();
    game.deck.retain(|c| !wanted.contains(c));
    // De quoi payer comptant : la pose est forcée, le budget n'est pas l'objet
    // de la mesure.
    game.players[0].mc = 1000;
    for name in cards {
        game.players[0].hand.push(ids[*name]);
        build_card(&mut game, db, 0, 0, 0);
    }
    // Plantes et chaleur à 0 : la conversion obligatoire de fin de phase III
    // (8 plantes → forêt, 8 chaleur → température) ne doit pas polluer la
    // mesure ni déclencher d'effet global parasite.
    for p in 0..2 {
        game.players[p].plants = 0;
        game.players[p].heat = 0;
    }
    let mut pol =
        ActivateBlue { base: RandomPolicy, target: ids[action_card], remaining: times, choice };
    play_round(&mut game, db, &mut pol);
    assert_eq!(
        pol.remaining, 0,
        "l'action de {action_card} n'a pas pu être activée {times} fois dans la manche"
    );
    (game, ids)
}

#[test]
fn birds_action_is_repeatable_within_the_same_game() {
    let db = db();
    // « Action: Add an animal to this card. » — capacité répétable, PAS un
    // ajout unique à la pose. Ses PV variables (« *=1 VP per animal ») en
    // dépendent entièrement.
    let birds = db.resolve_card("Birds").unwrap();

    // Une activation.
    let (g1, _) = activate(&db, 21, &["Birds"], "Birds", None, 1);
    assert_eq!(g1.players[0].resources_on(birds), 1);

    // DEUX activations dans la même partie : la capacité n'est pas consommée.
    let (g2, _) = activate(&db, 21, &["Birds"], "Birds", None, 2);
    assert_eq!(
        g2.players[0].resources_on(birds),
        2,
        "seconde activation sans effet = la capacité aurait été traitée comme un \
         gain unique à la pose"
    );
    assert_eq!(g2.blue_actions, 2, "deux activations comptées par le flux réel");
}

#[test]
fn extreme_cold_fungus_action_is_repeatable_within_the_same_game() {
    let db = db();
    // « Action: Gain 1 plant or add a microbe to ANOTHER* card. » Branche 0.
    let (g1, _) = activate(&db, 22, &["Extreme-Cold Fungus"], "Extreme-Cold Fungus", Some(0), 1);
    let (g2, _) = activate(&db, 22, &["Extreme-Cold Fungus"], "Extreme-Cold Fungus", Some(0), 2);
    assert_eq!(g1.players[0].plants, 1, "une activation = 1 plante");
    assert_eq!(g2.players[0].plants, 2, "deux activations = 2 plantes (capacité répétable)");
    assert_eq!(g2.blue_actions, 2);
}

#[test]
fn conserved_biome_action_is_repeatable_within_the_same_game() {
    let db = db();
    // « Action: Add a microbe to ANOTHER* card or add an animal to ANOTHER*
    // card. » Conserved Biome ne porte rien elle-même : la cible est une AUTRE
    // carte porteuse (Tardigrades porte des microbes). Branche 0 = microbe.
    let cards = ["Tardigrades", "Conserved Biome"];
    let (g1, ids) = activate(&db, 23, &cards, "Conserved Biome", Some(0), 1);
    let tardi = ids["Tardigrades"];
    assert_eq!(g1.players[0].resources_on(tardi), 1);

    let (g2, ids) = activate(&db, 23, &cards, "Conserved Biome", Some(0), 2);
    assert_eq!(
        g2.players[0].resources_on(ids["Tardigrades"]),
        2,
        "capacité répétable : deux activations posent deux microbes"
    );
    assert_eq!(
        g2.players[0].resources_on(ids["Conserved Biome"]),
        0,
        "« ANOTHER card » : Conserved Biome ne se cible jamais elle-même"
    );
}

#[test]
fn symbiotic_fungus_action_is_repeatable_within_the_same_game() {
    let db = db();
    // « Action: Add a microbe to ANOTHER* card. »
    let cards = ["Tardigrades", "Symbiotic Fungus"];
    // Ligne de base à UNE activation : sans elle, « 2 microbes après 2 choix »
    // n'exclut pas formellement qu'un microbe vienne d'ailleurs.
    let (g1, ids) = activate(&db, 24, &cards, "Symbiotic Fungus", None, 1);
    assert_eq!(g1.players[0].resources_on(ids["Tardigrades"]), 1);
    assert_eq!(g1.blue_actions, 1, "une activation réellement appliquée");

    let (g2, ids) = activate(&db, 24, &cards, "Symbiotic Fungus", None, 2);
    assert_eq!(g2.players[0].resources_on(ids["Tardigrades"]), 2);
    assert_eq!(
        g2.blue_actions, 2,
        "deux activations réellement APPLIQUÉES par le flux — pas seulement choisies"
    );
    assert_eq!(
        g2.players[0].resources_on(ids["Symbiotic Fungus"]),
        0,
        "« ANOTHER* card » : elle ne se cible jamais elle-même"
    );
}

// =====================================================================
// 3. MÉCANISMES PERMANENTS INATTEIGNABLES PAR LA SONDE CLI
// =====================================================================

/// Politique de test : choisit toujours `phase`, ne construit rien, ne prend
/// aucune action volontaire (seule la conversion obligatoire de fin de phase
/// III agit).
struct PhaseOnly {
    base: RandomPolicy,
    phase: u8,
}
impl Policy for PhaseOnly {
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
        if allowed.contains(&self.phase) {
            self.phase
        } else {
            self.base.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, _o: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// Politique de test : phase III, aucune construction, et activation de
/// l'action de la carte bleue `target` tant que `remaining > 0`.
struct ActivateBlue {
    base: RandomPolicy,
    target: u16,
    remaining: usize,
    choice: Option<usize>,
}
impl Policy for ActivateBlue {
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
        if allowed.contains(&3) {
            3
        } else {
            self.base.pick_phase(r, p, allowed)
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, options: &[ActionOpt]) -> Option<usize> {
        if self.remaining == 0 {
            return None;
        }
        let i = options.iter().position(|o| *o == ActionOpt::BlueAction(self.target))?;
        self.remaining -= 1;
        Some(i)
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        match self.choice {
            Some(c) => c,
            None => self.base.choose_option(r, p, n),
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// Met la carte `name` en jeu chez le joueur 0 par le chemin réel de pose.
fn put_in_play_for_p0(game: &mut GameState, db: &CardsDb, name: &str) -> u16 {
    let id = db.resolve_card(name).expect(name);
    game.deck.retain(|&c| c != id);
    game.players[0].mc += 1000;
    game.players[0].hand.insert(0, id);
    build_card(game, db, 0, 0, 0);
    id
}

#[test]
fn small_animals_gains_one_animal_per_forest_vp_over_several_rounds() {
    let db = db();
    // « Effect: When you gain a forest VP, add 1 animal to this card. » Le
    // déclencheur est PERMANENT : il doit répondre à CHAQUE forêt, sur
    // plusieurs manches, pas seulement à la première.
    let mut pol = PhaseOnly { base: RandomPolicy, phase: 3 };
    let mut game = setup_game(&db, 31, &mut pol);
    let sa = put_in_play_for_p0(&mut game, &db, "Small Animals");
    assert_eq!(game.players[0].resources_on(sa), 0, "posée vide");

    // Le livret p.10 interdit de rejouer la phase de la manche précédente :
    // la phase III ne peut donc pas revenir à chaque manche. On joue plusieurs
    // manches et on relève les manches où une forêt a effectivement été bâtie.
    let mut forests_before = 0;
    let mut manches_avec_foret = 0;
    for round in 1..=6 {
        // 8 plantes : la conversion obligatoire de fin de phase III bâtit une
        // forêt — un vrai gain de PV forêt, par le flux du moteur.
        game.players[0].plants = 8;
        game.players[1].plants = 0;
        game.players[1].heat = 0;
        game.players[0].heat = 0;
        play_round(&mut game, &db, &mut pol);
        if game.players[0].forests > forests_before {
            manches_avec_foret += 1;
            forests_before = game.players[0].forests;
        }
        // L'invariant vaut à CHAQUE manche : autant d'animaux que de PV forêt.
        assert_eq!(
            game.players[0].resources_on(sa) as i64,
            game.players[0].forests,
            "manche {round} : 1 animal par PV forêt — le déclencheur doit rester armé"
        );
        if game.game_over {
            break;
        }
    }
    assert!(
        manches_avec_foret >= 2,
        "au moins deux forêts gagnées sur des manches DISTINCTES (vu {manches_avec_foret}) : \
         c'est ce qui distingue un déclencheur permanent d'un effet unique à la pose"
    );
    assert!(forests_before >= 2);
}

#[test]
fn zeppelins_counts_the_same_forest_vp_counter_that_small_animals_watches() {
    let db = db();
    // Zeppelins : « produces 1 MC per forest VP you have ». Small Animals :
    // « When you gain a forest VP ». Les deux doivent lire LE MÊME compteur —
    // sinon le texte imprimé est trahi par une divergence interne.
    let mut pol = PhaseOnly { base: RandomPolicy, phase: 3 };
    let mut game = setup_game(&db, 32, &mut pol);
    let sa = put_in_play_for_p0(&mut game, &db, "Small Animals");
    put_in_play_for_p0(&mut game, &db, "Zeppelins");

    for _ in 0..2 {
        game.players[0].plants = 8;
        game.players[1].plants = 0;
        game.players[1].heat = 0;
        game.players[0].heat = 0;
        play_round(&mut game, &db, &mut pol);
        if game.game_over {
            break;
        }
    }
    let forests = game.players[0].forests;
    assert!(forests >= 1, "au moins une forêt bâtie par le flux réel");
    assert_eq!(game.players[0].resources_on(sa) as i64, forests);
    assert_eq!(
        derived_production(&db, &game.players[0]).0,
        forests,
        "Zeppelins produit 1 MC par PV forêt réellement possédé"
    );
}

#[test]
fn interplanetary_relations_research_bonus_stays_armed_for_the_whole_game() {
    let db = db();
    // « Effect: When you draw cards during the research phase, draw one
    // additional card and keep one additional card. » Effet PERMANENT : il
    // doit s'appliquer à CHAQUE phase V, pas une seule fois à la pose.
    let mut pol = PhaseOnly { base: RandomPolicy, phase: 5 };
    let mut game = setup_game(&db, 33, &mut pol);

    // Référence : un joueur sans la carte.
    let sans = research_draw_keep(&db, &PlayerState::new());
    assert_eq!(sans, (2, 1), "livret nu : pioche 2, garde 1");

    put_in_play_for_p0(&mut game, &db, "Interplanetary Relations");
    assert_eq!(
        research_draw_keep(&db, &game.players[0]),
        (3, 2),
        "avec la carte : +1 pioche, +1 garde"
    );

    // Le livret p.10 interdit de rejouer la phase de la manche précédente : la
    // phase V revient donc une manche sur deux au mieux. On joue plusieurs
    // manches et on compte celles où le bonus a RÉELLEMENT fait piocher en plus
    // (compteur d'audit `research_extra_draws`, relevé au site de pioche).
    let mut manches_avec_bonus = 0;
    for _ in 0..6 {
        let before = game.research_extra_draws;
        play_round(&mut game, &db, &mut pol);
        if game.research_extra_draws > before {
            manches_avec_bonus += 1;
        }
        if game.game_over {
            break;
        }
    }
    assert!(
        manches_avec_bonus >= 2,
        "le bonus doit s'appliquer à CHAQUE phase V, pas seulement à la pose : \
         seulement {manches_avec_bonus} manche(s) l'ont vu"
    );
}

// =====================================================================
// 4. PARTIE 3 — les deux pièges de données
// =====================================================================

#[test]
fn corporation_price_is_starting_mc_and_is_granted_never_paid() {
    let db = db();
    // « Le nombre imprimé en haut à droite d'une corporation est son MC de
    // DÉPART » (livret p.18 ; « You start with N MC. » imprimé sur la carte).
    // Un moteur qui le traiterait comme un coût FERAIT PAYER CrediCor 48 MC.
    let mut vues = std::collections::BTreeSet::new();
    for seed in 0..60u64 {
        let mut pol = RandomPolicy;
        let game = setup_game(&db, seed, &mut pol);
        for p in 0..2 {
            let corp = &db.corporations[game.players[p].corporation.unwrap() as usize];
            assert_eq!(
                game.players[p].mc, corp.starting_mc,
                "corporation {} : le MC de départ doit être REÇU (= price), pas payé",
                corp.name
            );
            assert!(game.players[p].mc > 0, "un joueur ne démarre jamais à 0 ou moins de MC");
            vues.insert(corp.name.clone());
        }
    }
    assert!(vues.len() >= 12, "au moins les 12 corporations de base observées, vu {}", vues.len());
}

#[test]
fn no_cyrillic_homoglyph_reaches_a_field_the_engine_depends_on() {
    let db = db();
    // Les homoglyphes « МС » de cards.json vivent dans `description`, un champ
    // que le moteur ne désérialise pas. Le champ dont il DÉPEND est `name` :
    // la table d'effets y est appariée par égalité stricte de chaîne.
    for c in &db.projects {
        assert!(
            c.name.is_ascii(),
            "le nom de carte {:?} contient un caractère non-ASCII : l'appariement \
             avec la table d'effets deviendrait silencieusement faux",
            c.name
        );
    }
    for c in &db.corporations {
        assert!(c.name.is_ascii(), "nom de corporation non-ASCII : {:?}", c.name);
    }
    // Et le garde-fou de chargement prouve que l'appariement fonctionne bien :
    // `CardsDb::load` a déjà échoué si une entrée de la table ne résolvait pas.
    assert!(db.resolve_card("Energy Subsidies").is_some(), "carte à « МС » dans sa description");
    assert!(db.resolve_card("Power Grid").is_some());
}

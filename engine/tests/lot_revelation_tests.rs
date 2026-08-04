//! Tests du chantier `revelation-montree` — **les cartes révélées sont
//! présentées, toutes, à chaque fois**.
//!
//! Le défaut relevé le 04-08 : la phase Action améliorée (III-A) révèle trois
//! cartes du dessus de la pioche et permet d'en prendre une bleue ou rouge.
//! Le moteur ne soumettait à la politique QUE les cartes prenables, et ne lui
//! soumettait rien du tout quand aucune ne l'était : l'écran ne pouvait donc
//! montrer ni les trois cartes, ni le fait qu'il n'y avait rien à prendre.
//!
//! Ce qui est vérifié ici, et l'oracle de chacun :
//!
//! 1. la politique est consultée **à chaque révélation**, y compris à zéro
//!    carte prenable — compté par une politique espionne ;
//! 2. elle reçoit **les trois cartes révélées**, et pas seulement les
//!    prenables — recoupé sur les identifiants des cartes empilées à la main ;
//! 3. la carte qui entre en main est celle que la politique a DÉSIGNÉE — le
//!    même dessus de pioche, deux réponses, deux cartes différentes ;
//! 4. **rien ne change au jeu** : mêmes cartes gardées, mêmes cartes
//!    défaussées, même consommation du générateur qu'avant le chantier — le
//!    corps par défaut ne tire rien quand il n'y a rien à prendre, et délègue
//!    à `research_keep` sinon.

use engine::cards::{CardsDb, Color, Tag};
use engine::effects::RevealFilter;
use engine::flow::{apply_blue_action, build_card_with, play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::*;
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

/// Un jeu nu : mains vidées, bourses à zéro (rien ne se pose par accident).
fn jeu(db: &CardsDb) -> GameState {
    let mut pol = RandomPolicy;
    let mut g = setup_game(db, 11, &mut pol);
    for p in 0..NUM_PLAYERS {
        let h: Vec<u16> = g.players[p].hand.drain(..).collect();
        g.discard.extend(h);
        g.players[p].mc = 0;
        g.players[p].heat = 0;
        g.players[p].plants = 0;
    }
    g
}

fn id_de(db: &CardsDb, nom: &str) -> u16 {
    db.resolve_card(nom)
        .unwrap_or_else(|| panic!("carte introuvable dans la base : « {nom} »"))
}

/// Retire une carte de partout, puis la pose sur le DESSUS de la pioche (la fin
/// du vecteur : `flow::draw_card` dépile).
fn empiler(g: &mut GameState, id: u16) {
    g.deck.retain(|&c| c != id);
    g.discard.retain(|&c| c != id);
    for p in 0..NUM_PLAYERS {
        g.players[p].hand.retain(|&c| c != id);
    }
    g.deck.push(id);
}

/// Première carte de la base satisfaisant `f`, hors `exclues`.
fn trouve(db: &CardsDb, f: impl Fn(&engine::cards::ProjectCard) -> bool, exclues: &[u16]) -> u16 {
    db.projects
        .iter()
        .enumerate()
        .find(|(i, c)| f(c) && !exclues.contains(&(*i as u16)))
        .map(|(i, _)| i as u16)
        .expect("aucune carte ne satisfait le critère")
}

fn sans_science_ni_plante(c: &engine::cards::ProjectCard) -> bool {
    !c.tags.contains(&Tag::Science) && !c.tags.contains(&Tag::Plant)
}

/// Ce qu'une révélation a soumis à la politique.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Vue {
    revelees: Vec<u16>,
    candidates: Vec<u16>,
    garder: usize,
    filtre: RevealFilter,
}

/// Politique espionne : elle enregistre chaque révélation et répond l'indice
/// imposé. Tout le reste est délégué à `RandomPolicy` — sauf la phase choisie
/// et les poses, tenues pour que la partie soit lisible.
struct Espion {
    phase: u8,
    /// Indice imposé à `reveal_pick` (borné aux candidates).
    prendre: usize,
    vues: Vec<Vue>,
}

impl Espion {
    fn new(phase: u8, prendre: usize) -> Espion {
        Espion { phase, prendre, vues: Vec::new() }
    }
}

impl Policy for Espion {
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, _a: &[u8]) -> u8 {
        self.phase
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, _o: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, drawn: &[u16], k: usize) -> Vec<usize> {
        (0..drawn.len().min(k)).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, hand: &[u16], n: usize) -> Vec<usize> {
        (0..hand.len().min(n)).collect()
    }
    fn reveal_pick(
        &mut self,
        _r: &mut StdRng,
        _p: usize,
        revelees: &[u16],
        candidates: &[u16],
        garder: usize,
        filtre: RevealFilter,
    ) -> Vec<usize> {
        self.vues.push(Vue {
            revelees: revelees.to_vec(),
            candidates: candidates.to_vec(),
            garder,
            filtre,
        });
        if garder == 0 {
            return Vec::new();
        }
        vec![self.prendre.min(candidates.len().saturating_sub(1))]
    }
}

// ===========================================================================
// 1. LA PHASE III-A — « une carte bleue ou rouge ainsi révélée »
// ===========================================================================

/// Trois cartes VERTES au-dessus : rien n'est prenable. Avant ce chantier, la
/// politique n'était pas appelée du tout, et l'écran ne montrait rien. Elle
/// l'est désormais — avec les trois cartes, et `garder = 0`.
#[test]
fn sans_carte_prenable_la_politique_voit_quand_meme_les_trois_cartes() {
    let d = db();
    let mut g = jeu(&d);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let a = trouve(&d, |c| c.color == Color::Green, &[]);
    let b = trouve(&d, |c| c.color == Color::Green, &[a]);
    let c = trouve(&d, |c| c.color == Color::Green, &[a, b]);
    // Empilées dans cet ordre : le dessus est `c`, puis `b`, puis `a`.
    for id in [a, b, c] {
        empiler(&mut g, id);
    }
    let main_avant = g.players[0].hand.len();

    let mut pol = Espion::new(3, 0);
    play_round(&mut g, &d, &mut pol);

    assert_eq!(pol.vues.len(), 1, "une révélation, une consultation");
    let v = &pol.vues[0];
    assert_eq!(v.revelees, vec![c, b, a], "les TROIS cartes révélées, dans l'ordre du tirage");
    assert!(v.candidates.is_empty(), "aucune carte verte n'est prenable");
    assert_eq!(v.garder, 0, "il n'y a rien à prendre, et c'est dit");
    assert_eq!(v.filtre, RevealFilter::ColorIsNot(Color::Green), "le filtre imprimé, tel quel");

    // Et le jeu, lui, n'a pas bougé d'un point.
    assert_eq!(g.players[0].hand.len(), main_avant, "rien n'entre en main");
    for id in [a, b, c] {
        assert!(g.discard.contains(&id), "les trois cartes vont à la défausse");
    }
}

/// Deux cartes prenables sur trois : la politique reçoit les trois, et les deux
/// candidates à part.
#[test]
fn les_cartes_non_prenables_sont_presentees_a_cote_des_prenables() {
    let d = db();
    let mut g = jeu(&d);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let verte = trouve(&d, |c| c.color == Color::Green, &[]);
    let bleue = trouve(&d, |c| c.color == Color::Blue, &[verte]);
    let rouge = trouve(&d, |c| c.color == Color::Red, &[verte, bleue]);
    for id in [verte, bleue, rouge] {
        empiler(&mut g, id);
    }

    let mut pol = Espion::new(3, 0);
    play_round(&mut g, &d, &mut pol);

    let v = pol.vues.first().expect("une révélation");
    assert_eq!(v.revelees.len(), 3, "trois cartes montrées");
    assert!(v.revelees.contains(&verte), "la carte NON prenable est présentée elle aussi");
    assert_eq!(v.candidates.len(), 2, "deux cartes prenables");
    assert!(!v.candidates.contains(&verte), "et la verte n'en est pas");
    assert_eq!(v.garder, 1, "on en prend une");
}

/// **Le joueur choisit LAQUELLE.** Même dessus de pioche, deux réponses, deux
/// cartes différentes en main. C'est la deuxième moitié de la demande du 04-08.
#[test]
fn la_carte_gardee_est_celle_que_la_politique_designe() {
    let d = db();
    let mut obtenues = Vec::new();
    let mut candidates_vues = Vec::new();
    for indice in [0usize, 1usize] {
        let mut g = jeu(&d);
        g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
        let verte = trouve(&d, |c| c.color == Color::Green, &[]);
        let bleue = trouve(&d, |c| c.color == Color::Blue, &[verte]);
        let rouge = trouve(&d, |c| c.color == Color::Red, &[verte, bleue]);
        for id in [verte, bleue, rouge] {
            empiler(&mut g, id);
        }
        let mut pol = Espion::new(3, indice);
        play_round(&mut g, &d, &mut pol);
        let v = pol.vues.first().expect("une révélation").clone();
        let gardee = v.candidates[indice];
        assert!(
            g.players[0].hand.contains(&gardee),
            "réponse {indice} : la carte désignée doit entrer en main"
        );
        let autre = v.candidates[1 - indice];
        assert!(
            g.discard.contains(&autre) && !g.players[0].hand.contains(&autre),
            "réponse {indice} : l'autre candidate part à la défausse"
        );
        candidates_vues.push(v.candidates.clone());
        obtenues.push(gardee);
    }
    assert_eq!(candidates_vues[0], candidates_vues[1], "le même dessus de pioche");
    assert_ne!(obtenues[0], obtenues[1], "deux réponses, deux cartes : le choix compte");
}

// ===========================================================================
// 2. L'AUTRE FILTRE — « une carte science ou plante » (Advanced Screening Tech)
// ===========================================================================

/// Le même mécanisme sert deux cartes au filtre différent : la révélation dit
/// LEQUEL, pour que l'écran puisse expliquer pourquoi une carte n'est pas
/// prenable sans réinventer la règle.
#[test]
fn le_filtre_imprime_accompagne_la_revelation() {
    let d = db();
    let mut g = jeu(&d);
    let ast = id_de(&d, "Advanced Screening Tech");
    g.deck.retain(|&c| c != ast);
    g.players[0].hand.push(ast);
    let idx = g.players[0].hand.len() - 1;
    g.players[0].mc = 1000;
    let mut pose = RandomPolicy;
    build_card_with(&mut g, &d, 0, idx, 0, &mut pose);
    assert!(g.players[0].played.contains(&ast), "la carte doit être en jeu");

    let a = trouve(&d, sans_science_ni_plante, &[ast]);
    let b = trouve(&d, sans_science_ni_plante, &[ast, a]);
    let c = trouve(&d, sans_science_ni_plante, &[ast, a, b]);
    for id in [a, b, c] {
        empiler(&mut g, id);
    }

    let mut pol = Espion::new(3, 0);
    assert!(apply_blue_action(&mut g, &d, 0, ast, &mut pol), "l'action doit s'appliquer");

    let v = pol.vues.first().expect("une révélation");
    assert_eq!(v.revelees, vec![c, b, a], "les trois cartes, toutes présentées");
    assert!(v.candidates.is_empty(), "aucune ne porte science ni plante");
    assert_eq!(v.garder, 0);
    assert_eq!(
        v.filtre,
        RevealFilter::AnyOfTags(&[Tag::Science, Tag::Plant]),
        "le filtre est celui de CETTE carte, pas celui de la phase III-A"
    );
}

// ===========================================================================
// 3. RIEN N'A CHANGÉ AU JEU — le corps par défaut redit l'ancien moteur
// ===========================================================================

/// Politique qui ne connaît QUE `research_keep` : elle n'implémente pas
/// `reveal_pick`. Le corps par défaut doit lui poser exactement l'ancienne
/// question — « garder k parmi les candidates » — et rien à zéro candidate.
struct Ancienne {
    base: RandomPolicy,
    appels: Vec<(Vec<u16>, usize)>,
}

impl Policy for Ancienne {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, _a: &[u8]) -> u8 {
        3
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, _o: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, drawn: &[u16], k: usize) -> Vec<usize> {
        self.appels.push((drawn.to_vec(), k));
        (0..drawn.len().min(k)).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, hand: &[u16], n: usize) -> Vec<usize> {
        (0..hand.len().min(n)).collect()
    }
}

#[test]
fn le_corps_par_defaut_pose_l_ancienne_question_et_seulement_elle() {
    let d = db();
    // Cas A : deux candidates — `research_keep` est appelée sur elles seules.
    let mut g = jeu(&d);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let verte = trouve(&d, |c| c.color == Color::Green, &[]);
    let bleue = trouve(&d, |c| c.color == Color::Blue, &[verte]);
    let rouge = trouve(&d, |c| c.color == Color::Red, &[verte, bleue]);
    for id in [verte, bleue, rouge] {
        empiler(&mut g, id);
    }
    let mut pol = Ancienne { base: RandomPolicy, appels: Vec::new() };
    play_round(&mut g, &d, &mut pol);
    let revelation: Vec<&(Vec<u16>, usize)> =
        pol.appels.iter().filter(|(v, _)| v.len() == 2 && v.contains(&bleue)).collect();
    assert_eq!(revelation.len(), 1, "une question, sur les deux candidates");
    assert_eq!(revelation[0].1, 1, "garder une carte");
    assert!(!revelation[0].0.contains(&verte), "la carte verte n'a jamais été proposée");

    // Cas B : aucune candidate — aucune question n'est posée à l'ancienne
    // politique (et le générateur n'est pas touché).
    let mut g = jeu(&d);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let a = trouve(&d, |c| c.color == Color::Green, &[]);
    let b = trouve(&d, |c| c.color == Color::Green, &[a]);
    let c = trouve(&d, |c| c.color == Color::Green, &[a, b]);
    for id in [a, b, c] {
        empiler(&mut g, id);
    }
    let mut pol = Ancienne { base: RandomPolicy, appels: Vec::new() };
    play_round(&mut g, &d, &mut pol);
    assert!(
        pol.appels.iter().all(|(v, _)| !v.contains(&a) && !v.contains(&b) && !v.contains(&c)),
        "à zéro carte prenable, l'ancienne question n'est pas posée"
    );
}

/// Le corps par défaut ne consomme pas le générateur quand il n'y a rien à
/// prendre : la suite tirée est la même avec et sans révélation vide. C'est ce
/// qui garantit que les empreintes de parties n'ont pas bougé.
#[test]
fn une_revelation_sans_rien_a_prendre_ne_tire_pas_au_hasard() {
    use rand::SeedableRng;
    let mut a = StdRng::seed_from_u64(7);
    let mut b = StdRng::seed_from_u64(7);
    let mut pol = RandomPolicy;
    let vides: Vec<u16> = Vec::new();

    let mut sans = Vec::new();
    let mut avec = Vec::new();
    for _ in 0..40 {
        sans.push(pol.pick_phase(&mut a, 0, &[1, 2, 3, 4, 5]));
        let r = pol.reveal_pick(&mut b, 0, &[1, 2, 3], &vides, 0, RevealFilter::ColorIsNot(Color::Green));
        assert!(r.is_empty(), "rien à prendre, rien de rendu");
        avec.push(pol.pick_phase(&mut b, 0, &[1, 2, 3, 4, 5]));
    }
    assert_eq!(sans, avec, "une révélation vide a consommé le générateur");
}

/// Personne ne se bloque : 200 parties entières avec la politique aléatoire,
/// qui ne connaît pas `reveal_pick`, vont jusqu'au bout.
#[test]
fn deux_cents_parties_vont_au_bout_sans_connaitre_la_nouvelle_question() {
    let d = CardsDb::load_boites(
        CARDS,
        engine::boites::BoiteSet::parse("base,decouverte").expect("boîtes"),
    )
    .expect("cards.json doit se charger");
    let mut pol = RandomPolicy;
    let s = engine::sim::run_simulation(&d, 200, 4242, &mut pol);
    assert_eq!(s.games, 200, "les 200 parties se terminent");
    assert!(s.cards_revealed > 0, "aucune révélation rencontrée : mesure ratée");
}

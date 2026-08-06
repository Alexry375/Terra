//! **MOT-13 — vendre sa dernière carte faisait disparaître une défausse
//! imposée.**
//!
//! La fiche annonçait « 1 cas sur 1 003 ventes, graine 5150 » et citait des
//! lignes qui n'existent plus. Ce fichier est la reproduction faite ici, à la
//! main, avant toute correction — puis le garde-fou qui empêche le défaut de
//! revenir.
//!
//! **Le mécanisme.** `Eff::DrawDiscard` (« piochez `draw` cartes, puis
//! défaussez-en `discard` ») offre une occasion de vendre APRÈS la pioche et
//! AVANT de compter les cartes défaussables. Le joueur qui vend à cette
//! occasion-là réduit `cands`, donc `n` : à la limite, `n` tombe à zéro et il ne
//! défausse rien. Il encaisse les MC de la vente ET échappe au coût de l'effet.
//!
//! **Les deux oracles**, et le second est arrivé après une relecture
//! adversariale qui a montré que le premier ne suffisait pas :
//!
//! - `PlayerState::defausses_imposees_esquivees` compare la défausse DUE — figée
//!   sur la main d'avant l'occasion — aux cartes qui ont RÉELLEMENT quitté la
//!   main par `flow::discard_from_hand`. Il ne lit aucune grandeur du
//!   correctif : c'est un comptage de cartes déplacées.
//! - `PlayerState::gardes_imposees_perdues` compte l'autre moitié de la dette,
//!   celle que « **Keep one of them** and discard the other two » exprime en
//!   cartes GARDÉES. Une vente qui emporte la carte à garder ne rétrécit aucune
//!   défausse — le premier compteur n'y voit rien — mais elle fait garder ZÉRO
//!   carte sur trois. Reproduit aux graines 5008 et 5020 sur la première version
//!   du correctif, corrigé depuis (la réserve prend TOUTES les cartes piochées).
//!
//! **Le joueur d'essai** vend au hasard, une carte à la fois, à une occasion sur
//! six. Il ne connaît pas `Eff::DrawDiscard` et ne vise pas ce point : il vend
//! comme un joueur vendrait, et le défaut vient à lui.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::{GameState, NUM_PLAYERS};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const CARDS: &str = "../data/cards.json";
const MAX_GENERATIONS: u32 = 40;

fn db_decouverte() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("cards.json doit se charger")
}

/// Un joueur au hasard qui VEND, une carte à la fois, à une occasion sur six.
///
/// Son tirage de vente sort d'un RNG À LUI : le RNG de la partie garde
/// exactement la suite qu'il aurait sans lui pour toutes les autres décisions,
/// et deux mesures à la même graine sont identiques.
struct VendeurAuHasard {
    base: RandomPolicy,
    rng: StdRng,
    ventes: u64,
}

impl VendeurAuHasard {
    fn new(graine: u64) -> VendeurAuHasard {
        VendeurAuHasard {
            base: RandomPolicy,
            rng: StdRng::seed_from_u64(graine ^ 0x5e11_5e11),
            ventes: 0,
        }
    }
}

impl Policy for VendeurAuHasard {
    fn vendre_librement(
        &mut self,
        _rng: &mut StdRng,
        _joueur: usize,
        main: &[u16],
    ) -> Vec<usize> {
        if main.is_empty() || !self.rng.gen_bool(1.0 / 6.0) {
            return Vec::new();
        }
        self.ventes += 1;
        vec![self.rng.gen_range(0..main.len())]
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, p: usize, corps: &[u16]) -> bool {
        self.base.corp_mulligan(rng, p, corps)
    }
    fn project_mulligan(&mut self, rng: &mut StdRng, p: usize, main: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(rng, p, main)
    }
    fn pick_corporation(&mut self, rng: &mut StdRng, p: usize, corps: &[u16]) -> usize {
        self.base.pick_corporation(rng, p, corps)
    }
    fn pick_phase(&mut self, rng: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        self.base.pick_phase(rng, p, allowed)
    }
    fn choose_build(&mut self, rng: &mut StdRng, p: usize, aff: &[usize]) -> Option<usize> {
        self.base.choose_build(rng, p, aff)
    }
    fn construction_bonus(&mut self, rng: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(rng, p)
    }
    fn action_choice(&mut self, rng: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.base.action_choice(rng, p, o)
    }
    fn research_keep(&mut self, rng: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(rng, p, d, k)
    }
    fn discard_down(&mut self, rng: &mut StdRng, p: usize, main: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(rng, p, main, n)
    }
}

/// Ce qu'un lot de parties a montré.
struct Mesure {
    parties: u64,
    ventes: u64,
    /// Cartes qu'une vente a fait échapper à une défausse imposée.
    esquivees: u64,
    /// Cartes que le joueur avait le droit de GARDER et qui sont parties
    /// (« Keep one of them » — l'autre moitié de la dette).
    gardes_perdues: u64,
    /// Parties où au moins une carte y a échappé.
    parties_touchees: u64,
}

/// Joue les parties des graines données, avec un vendeur au hasard par partie,
/// et rend ce que les compteurs d'audit du moteur ont vu.
fn mesurer(db: &CardsDb, graines: impl Iterator<Item = u64>) -> Mesure {
    let mut m = Mesure {
        parties: 0,
        ventes: 0,
        esquivees: 0,
        gardes_perdues: 0,
        parties_touchees: 0,
    };
    for graine in graines {
        let mut pol = VendeurAuHasard::new(graine);
        let mut game: GameState = setup_game(db, graine, &mut pol);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, db, &mut pol);
        }
        let esquivees: u64 = (0..NUM_PLAYERS)
            .map(|p| game.players[p].defausses_imposees_esquivees)
            .sum();
        m.parties += 1;
        m.ventes += pol.ventes;
        m.esquivees += esquivees;
        m.gardes_perdues += (0..NUM_PLAYERS)
            .map(|p| game.players[p].gardes_imposees_perdues)
            .sum::<u64>();
        if esquivees > 0 {
            m.parties_touchees += 1;
        }
    }
    m
}

/// **LE TEST DU DÉFAUT.** Rouge avant la correction (la mesure du 06-08 est
/// écrite dans `outputs/result.md`), vert après.
///
/// Il ne demande pas « le compteur est-il petit » : une défausse imposée qui
/// disparaît une fois sur mille reste une règle fausse, et une machine qui
/// apprend cherchera cette fois-là. Le seuil est donc ZÉRO.
#[test]
fn une_vente_ne_fait_pas_disparaitre_une_defausse_imposee() {
    let db = db_decouverte();
    let m = mesurer(&db, 5000..5200);
    println!(
        "    {} partie(s), {} vente(s) proposee(s), {} carte(s) esquivee(s), \
         {} garde(s) perdue(s), {} partie(s) touchee(s)",
        m.parties, m.ventes, m.esquivees, m.gardes_perdues, m.parties_touchees
    );
    assert!(
        m.ventes > 500,
        "mesure vide : {} vente(s) seulement, le banc n'a pas éprouvé la vente",
        m.ventes
    );
    assert_eq!(
        m.esquivees, 0,
        "{} carte(s) ont échappé à une défausse imposée grâce à une vente \
         (sur {} ventes, {} parties, graines 5000..5200)",
        m.esquivees, m.ventes, m.parties
    );
    assert_eq!(
        m.gardes_perdues, 0,
        "{} carte(s) que le joueur avait le droit de GARDER sont parties \
         (« keep one of them »)",
        m.gardes_perdues
    );
}

/// **Un SECOND lot, disjoint du premier**, autour de la graine 5150 que la fiche
/// cite nommément. Deux lots plutôt qu'un : un correctif réglé sur les graines
/// du premier ne rendrait pas celui-ci vert par accident.
#[test]
fn le_second_lot_de_graines_ne_montre_rien_non_plus() {
    let db = db_decouverte();
    let m = mesurer(&db, 5150..5350);
    println!(
        "    {} partie(s), {} vente(s), {} carte(s) esquivee(s), {} garde(s) \
         perdue(s), {} partie(s) touchee(s)",
        m.parties, m.ventes, m.esquivees, m.gardes_perdues, m.parties_touchees
    );
    assert!(m.ventes > 500, "mesure vide : {} vente(s)", m.ventes);
    assert_eq!(
        m.esquivees, 0,
        "{} carte(s) ont échappé à une défausse imposée (graines 5150..5350)",
        m.esquivees
    );
    assert_eq!(m.gardes_perdues, 0, "{} garde(s) imposée(s) perdue(s)", m.gardes_perdues);
}

/// **LE VENDEUR GOURMAND** — celui qui vide sa main entière, à une occasion sur
/// six.
///
/// C'est le pire client réaliste, et c'est celui qu'une machine qui apprend
/// finirait par devenir : là où vendre efface la défausse, il efface tout. Une
/// occasion sur six et non toutes, parce qu'un joueur qui vend TOUT à CHAQUE
/// occasion ne pose plus jamais une seule carte — il ne rencontre donc jamais
/// `Eff::DrawDiscard`, et le banc serait vert sans rien avoir éprouvé (mesuré :
/// 2 645 occasions vendues, 0 défausse imposée rencontrée). D'où la ceinture
/// ci-dessous : le lot ne vaut que si l'effet a réellement été résolu.
#[test]
fn meme_en_vendant_sa_main_entiere_la_defausse_imposee_reste_due() {
    struct ToutVendre(RandomPolicy, u64, StdRng);
    impl Policy for ToutVendre {
        fn vendre_librement(&mut self, _r: &mut StdRng, _j: usize, main: &[u16]) -> Vec<usize> {
            if main.is_empty() || !self.2.gen_bool(1.0 / 6.0) {
                return Vec::new();
            }
            self.1 += 1;
            (0..main.len()).collect()
        }
        fn corp_mulligan(&mut self, rng: &mut StdRng, p: usize, c: &[u16]) -> bool {
            self.0.corp_mulligan(rng, p, c)
        }
        fn project_mulligan(&mut self, rng: &mut StdRng, p: usize, m: &[u16]) -> Vec<usize> {
            self.0.project_mulligan(rng, p, m)
        }
        fn pick_corporation(&mut self, rng: &mut StdRng, p: usize, c: &[u16]) -> usize {
            self.0.pick_corporation(rng, p, c)
        }
        fn pick_phase(&mut self, rng: &mut StdRng, p: usize, a: &[u8]) -> u8 {
            self.0.pick_phase(rng, p, a)
        }
        fn choose_build(&mut self, rng: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
            self.0.choose_build(rng, p, a)
        }
        fn construction_bonus(&mut self, rng: &mut StdRng, p: usize) -> ConstructionBonus {
            self.0.construction_bonus(rng, p)
        }
        fn action_choice(&mut self, rng: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
            self.0.action_choice(rng, p, o)
        }
        fn research_keep(&mut self, rng: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            self.0.research_keep(rng, p, d, k)
        }
        fn discard_down(&mut self, rng: &mut StdRng, p: usize, m: &[u16], n: usize) -> Vec<usize> {
            self.0.discard_down(rng, p, m, n)
        }
    }

    let db = db_decouverte();
    let mut esquivees = 0u64;
    let mut ventes = 0u64;
    let mut defausses = 0u64;
    for graine in 5000..5200u64 {
        let mut pol = ToutVendre(RandomPolicy, 0, StdRng::seed_from_u64(graine ^ 0x60_1d));
        let mut game: GameState = setup_game(&db, graine, &mut pol);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, &db, &mut pol);
        }
        esquivees += (0..NUM_PLAYERS)
            .map(|p| game.players[p].defausses_imposees_esquivees)
            .sum::<u64>();
        ventes += pol.1;
        defausses += game.draw_discard_discards;
    }
    println!(
        "    vendeur gourmand : {ventes} occasion(s) videe(s), {defausses} carte(s) \
         reellement defaussee(s), {esquivees} carte(s) esquivee(s)"
    );
    assert!(
        defausses > 50,
        "le banc n'a rencontré que {defausses} défausse(s) imposée(s) : il n'éprouve rien"
    );
    assert_eq!(
        esquivees, 0,
        "un joueur qui vide sa main échappe à {esquivees} carte(s) de défausse imposée"
    );
}

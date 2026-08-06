//! **L'état dit-il si l'occasion de vendre est encore ouverte ?**
//!
//! `GameState::vente_offerte` dit le DROIT de vendre : « ce point de décision a
//! reçu son occasion ». Il ne dit pas si l'occasion est encore à prendre. Après
//! une vente, le moteur repose la même question sur l'état d'après et le drapeau
//! y vaut encore vrai — un fournisseur sans mémoire revend, et la partie
//! s'arrête (« aucune occasion de vendre n'est ouverte à ce point »).
//!
//! `PlayerState::occasion_de_vendre_ouverte`, publié par `observe::state_view`,
//! dit l'autre moitié. Ces bancs l'éprouvent avec un ORACLE DISJOINT du moteur :
//! la politique tient elle-même le compte de ce qu'elle a vendu depuis la
//! dernière observation, et l'on compare ce compte à ce que l'état publie. Rien
//! ici ne recalcule la règle du moteur.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, setup_game};
use engine::observe::state_view;
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::{GameState, NUM_PLAYERS};
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";
const MAX_GENERATIONS: u32 = 40;

fn db_decouverte() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("cards.json doit se charger")
}

/// Ce qu'un banc a relevé.
#[derive(Default)]
struct Releve {
    observations: u64,
    /// Observations où le DROIT de vendre est publié vrai.
    droit: u64,
    /// Observations où le droit est vrai mais l'occasion de ce siège fermée :
    /// exactement ce que l'ancien état ne savait pas dire.
    droit_mais_ferme: u64,
    /// Désaccords entre l'oracle de la politique et l'état publié.
    fautes: Vec<String>,
}

/// Le juge : une politique qui vend (ou non), et qui CONFRONTE à chaque
/// observation ce que l'état publie à ce qu'elle a elle-même fait.
struct Juge {
    base: RandomPolicy,
    /// Vend-elle dès qu'elle le peut ?
    vend: bool,
    /// Sièges pour lesquels une entrée de vente a été rendue depuis la dernière
    /// observation.
    vendus: [bool; NUM_PLAYERS],
    releve: Releve,
}

impl Juge {
    fn new(vend: bool) -> Juge {
        Juge {
            base: RandomPolicy,
            vend,
            vendus: [false; NUM_PLAYERS],
            releve: Releve::default(),
        }
    }
}

impl Policy for Juge {
    fn vendre_librement(&mut self, _r: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        if !self.vend || main.is_empty() {
            return Vec::new();
        }
        self.vendus[joueur] = true;
        vec![0]
    }

    fn observe(&mut self, game: &GameState, _player: usize) {
        let r = &mut self.releve;
        r.observations += 1;
        if game.vente_offerte {
            r.droit += 1;
        }
        for p in 0..NUM_PLAYERS {
            let ouverte = game.players[p].occasion_de_vendre_ouverte;
            let vide = game.players[p].hand.is_empty();
            if game.vente_offerte && !ouverte {
                r.droit_mais_ferme += 1;
            }
            // ORACLE 1 — une vente rendue depuis la dernière observation ferme
            // l'occasion de ce siège. (La politique vend dès qu'elle le peut :
            // si une occasion neuve s'était glissée entre-temps, elle y aurait
            // vendu aussi, ou le siège n'avait plus de carte.)
            if self.vendus[p] && ouverte {
                r.fautes.push(format!(
                    "siège {p} : l'état dit l'occasion ouverte alors que ce siège \
                     vient de vendre"
                ));
            }
            // ORACLE 2 — aucune vente, une main, et le droit de vendre : alors
            // l'occasion est ouverte. Un état qui fermerait ici coûterait au
            // joueur des ventes parfaitement légales.
            let aucune_vente_apres = (p..NUM_PLAYERS).all(|q| !self.vendus[q]);
            if aucune_vente_apres && game.vente_offerte && !vide && !ouverte {
                r.fautes.push(format!(
                    "siège {p} : l'état ferme l'occasion alors que personne n'a \
                     vendu et que la main n'est pas vide"
                ));
            }
            // ORACLE 3 — hors droit de vendre, jamais d'occasion ouverte.
            if !game.vente_offerte && ouverte {
                r.fautes.push(format!(
                    "siège {p} : occasion ouverte sans droit de vendre"
                ));
            }
        }
        self.vendus = [false; NUM_PLAYERS];
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(rng, p, c)
    }
    fn project_mulligan(&mut self, rng: &mut StdRng, p: usize, m: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(rng, p, m)
    }
    fn pick_corporation(&mut self, rng: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(rng, p, c)
    }
    fn pick_phase(&mut self, rng: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(rng, p, a)
    }
    fn choose_build(&mut self, rng: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(rng, p, a)
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
    fn discard_down(&mut self, rng: &mut StdRng, p: usize, m: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(rng, p, m, n)
    }
}

fn jouer(db: &CardsDb, graines: std::ops::Range<u64>, vend: bool) -> Releve {
    let mut cumul = Releve::default();
    for graine in graines {
        let mut juge = Juge::new(vend);
        let mut game: GameState = setup_game(db, graine, &mut juge);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, db, &mut juge);
        }
        cumul.observations += juge.releve.observations;
        cumul.droit += juge.releve.droit;
        cumul.droit_mais_ferme += juge.releve.droit_mais_ferme;
        for f in juge.releve.fautes {
            if cumul.fautes.len() < 5 {
                cumul.fautes.push(format!("graine {graine} : {f}"));
            }
        }
    }
    cumul
}

/// **LE BANC PRINCIPAL** — un vendeur qui vend dès qu'il peut.
#[test]
fn l_etat_ferme_l_occasion_du_siege_qui_vient_de_vendre() {
    let db = db_decouverte();
    let r = jouer(&db, 7000..7040, true);
    println!(
        "    {} observation(s), {} avec le droit de vendre, {} où le droit est \
         vrai et l'occasion fermée",
        r.observations, r.droit, r.droit_mais_ferme
    );
    assert!(r.observations > 1000, "banc vide : {} observations", r.observations);
    assert!(
        r.droit_mais_ferme > 100,
        "seulement {} point(s) où le droit et l'occasion diffèrent : l'état ne \
         distingue pas les deux, ou le banc n'a rien vendu",
        r.droit_mais_ferme
    );
    assert!(r.fautes.is_empty(), "l'état ment : {:?}", r.fautes);
}

/// **LA CONTRE-ÉPREUVE** — un joueur qui ne vend jamais. L'occasion doit alors
/// être ouverte partout où le droit de vendre l'est, main non vide : un état qui
/// fermerait ici priverait le joueur de ventes légales, et le contrôle 01 de son
/// quota de ventes.
#[test]
fn sans_aucune_vente_l_occasion_suit_le_droit_de_vendre() {
    let db = db_decouverte();
    let r = jouer(&db, 7040..7080, false);
    println!(
        "    {} observation(s), {} avec le droit de vendre, {} où le droit est \
         vrai et l'occasion fermée (mains vides)",
        r.observations, r.droit, r.droit_mais_ferme
    );
    assert!(r.droit > 500, "banc vide : {} droits", r.droit);
    assert!(r.fautes.is_empty(), "l'état ment : {:?}", r.fautes);
}

/// L'information doit sortir de `state_view` : c'est par là, et seulement par
/// là, qu'un fournisseur la lit.
#[test]
fn l_occasion_est_publiee_dans_l_etat_json() {
    let db = db_decouverte();
    let mut juge = Juge::new(true);
    let mut game: GameState = setup_game(&db, 7100, &mut juge);
    play_round(&mut game, &db, &mut juge);
    let vue = state_view(&game, &db);
    let publie = vue
        .get("occasion_de_vendre_ouverte")
        .expect("`occasion_de_vendre_ouverte` doit être publié")
        .as_array()
        .expect("un booléen par siège")
        .clone();
    assert_eq!(publie.len(), NUM_PLAYERS);
    for (p, v) in publie.iter().enumerate() {
        assert_eq!(
            v.as_bool(),
            Some(game.players[p].occasion_de_vendre_ouverte),
            "siège {p} : l'état publié doit dire ce que le moteur pense"
        );
    }
    // `vente_offerte` n'a pas bougé de place : l'écran le lit toujours.
    assert!(vue.get("vente_offerte").is_some());
}

//! **(moteur-questions-manquantes) La vente ne coûte plus un tour, et la
//! défausse est publiée.**
//!
//! Deux règles, mesurées sur le chemin réel du moteur (`setup_game` +
//! `play_round`), jamais sur un état fabriqué à la main :
//!
//! 1. `ActionOpt::SellCard` a disparu de la phase Action. Un joueur sans
//!    ressources n'y reçoit donc plus AUCUNE option — là où sa main lui en
//!    offrait une, qui lui coûtait un échange et ne vendait qu'une carte. Vendre
//!    reste possible, gratuit, et sans limite de nombre, par l'occasion libre.
//! 2. `state_view` publie le CONTENU de la défausse, la plus récemment défaussée
//!    en tête, de la longueur que la vue publie déjà à côté.
//!
//! L'oracle de l'ordre est disjoint du champ mesuré : ce sont les cartes que la
//! politique a DÉSIGNÉES dans sa main, pas le vecteur `game.discard` relu à
//! l'envers.

use engine::cards::CardsDb;
use engine::flow::{discard_mc_rate, play_round, setup_game};
use engine::observe::state_view;
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::GameState;
use rand::rngs::StdRng;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

/// Politique de mesure : elle impose la phase, PASSE toujours (aucune pose,
/// aucune action), vend librement ce qu'on lui dit de vendre — une seule fois —
/// et note chaque liste d'options d'action qu'on lui soumet.
struct Mesure {
    phase: u8,
    /// Indices de main vendus par le joueur 0, à sa première occasion.
    vente: Vec<usize>,
    vendu: bool,
    /// La main du joueur 0 telle qu'elle était au moment de la vente : c'est
    /// l'oracle de l'ordre de la défausse.
    main_a_la_vente: Vec<u16>,
    /// Une entrée par appel à `action_choice` : (joueur, nombre d'options).
    options_vues: Vec<(usize, usize)>,
}

impl Mesure {
    fn new(phase: u8) -> Mesure {
        Mesure {
            phase,
            vente: Vec::new(),
            vendu: false,
            main_a_la_vente: Vec::new(),
            options_vues: Vec::new(),
        }
    }
}

impl Policy for Mesure {
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
        0
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
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _r: &mut StdRng, p: usize, options: &[ActionOpt]) -> Option<usize> {
        self.options_vues.push((p, options.len()));
        None
    }
    fn vendre_librement(&mut self, _r: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        if joueur != 0 || self.vendu || self.vente.is_empty() || main.len() < self.vente.len() {
            return Vec::new();
        }
        self.vendu = true;
        self.main_a_la_vente = main.to_vec();
        self.vente.clone()
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, _d: &[u16], k: usize) -> Vec<usize> {
        (0..k).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

/// Un joueur sans MC, sans plantes, sans chaleur et sans carte bleue posée n'a
/// plus RIEN à faire en phase Action — même la main pleine.
///
/// C'est le test inversé du retrait : avant lui, `action_options` poussait
/// `SellCard` dès que la main n'était pas vide, et cette option-là consommait un
/// échange de la phase.
#[test]
fn un_joueur_demuni_ne_recoit_plus_aucune_option_d_action() {
    let db = db();
    let mut pol = Mesure::new(3);
    let mut game = setup_game(&db, 4242, &mut pol);
    demunir(&mut game, 0);
    assert!(
        !game.players[0].hand.is_empty(),
        "la main doit être garnie : c'est elle qui offrait la vente"
    );
    play_round(&mut game, &db, &mut pol);

    let siennes: Vec<usize> = pol
        .options_vues
        .iter()
        .filter(|(p, _)| *p == 0)
        .map(|(_, n)| *n)
        .collect();
    assert!(
        !siennes.is_empty(),
        "le joueur a bien été sollicité en phase Action (sinon rien n'est mesuré)"
    );
    assert!(
        siennes.iter().all(|&n| n == 0),
        "aucune option pour un joueur démuni, or on lui en a offert : {siennes:?}"
    );
}

/// Vendre ne coûte plus un échange : le joueur qui vend est sollicité autant de
/// fois que celui qui ne vend pas, et il a touché ses MC.
///
/// Référence indépendante : la MÊME partie, même graine, sans la vente.
#[test]
fn vendre_ne_consomme_plus_un_echange_de_la_phase_action() {
    let db = db();

    let mut temoin = Mesure::new(3);
    let mut g0 = setup_game(&db, 4242, &mut temoin);
    demunir(&mut g0, 0);
    play_round(&mut g0, &db, &mut temoin);
    let appels_temoin = temoin.options_vues.iter().filter(|(p, _)| *p == 0).count();

    let mut vendeur = Mesure::new(3);
    vendeur.vente = vec![0, 1, 2];
    let mut g1 = setup_game(&db, 4242, &mut vendeur);
    demunir(&mut g1, 0);
    let main_avant = g1.players[0].hand.len();
    let taux = discard_mc_rate(&db, &g1.players[0]);
    play_round(&mut g1, &db, &mut vendeur);
    let appels_vendeur = vendeur.options_vues.iter().filter(|(p, _)| *p == 0).count();

    assert!(vendeur.vendu, "la vente a bien eu lieu");
    assert_eq!(
        g1.players[0].hand.len(),
        main_avant - 3,
        "trois cartes quittent la main en UNE fois — l'action standard n'en vendait qu'une"
    );
    assert_eq!(
        g1.players[0].mc,
        3 * taux,
        "trois fois le taux du service unique, et rien d'autre"
    );
    assert_eq!(
        appels_vendeur, appels_temoin,
        "vendre n'a pas coûté d'échange : autant de sollicitations que sans vente"
    );
}

/// La défausse publiée : la liste existe, sa longueur est celle que la vue
/// publie déjà, et l'ordre est le plus récent d'abord.
#[test]
fn la_defausse_est_publiee_complete_et_la_plus_recente_en_tete() {
    let db = db();
    let mut pol = Mesure::new(3);
    pol.vente = vec![0, 1, 2];
    let mut game = setup_game(&db, 4242, &mut pol);
    demunir(&mut game, 0);
    play_round(&mut game, &db, &mut pol);
    assert!(pol.vendu, "la vente a bien eu lieu");

    let vue = state_view(&game, &db);
    let liste = vue["defausse"].as_array().expect("« defausse » est une liste");
    let taille = vue["decks"]["discard"].as_u64().expect("la taille publiée");
    assert_eq!(
        liste.len() as u64,
        taille,
        "la liste a exactement la longueur que la vue publie à côté"
    );
    assert!(liste.len() >= 3, "au moins les trois cartes vendues");

    // ORACLE DISJOINT : les cartes que la politique a désignées dans SA main,
    // pas `game.discard` relu à l'envers. Le moteur retire par indice
    // décroissant — la carte d'indice 0 est donc la dernière poussée, et c'est
    // elle qui doit être en tête.
    let attendu: Vec<u64> = pol.main_a_la_vente[..3].iter().map(|&c| c as u64).collect();
    let tete: Vec<u64> = liste[..3]
        .iter()
        .map(|c| c["id"].as_u64().expect("un identifiant de carte"))
        .collect();
    assert_eq!(
        tete, attendu,
        "la plus récemment défaussée en tête, puis les précédentes"
    );
}

/// Une partie entière, jouée par la politique aléatoire : la longueur publiée et
/// la liste publiée ne divergent jamais, manche après manche.
#[test]
fn la_liste_et_la_taille_ne_divergent_jamais_sur_une_partie_entiere() {
    let db = db();
    for graine in [4242u64, 77, 2024] {
        let mut pol = RandomPolicy;
        let mut game = setup_game(&db, graine, &mut pol);
        let mut manches = 0;
        while !game.game_over && manches < 30 {
            play_round(&mut game, &db, &mut pol);
            manches += 1;
            let vue = state_view(&game, &db);
            let liste = vue["defausse"].as_array().expect("« defausse » est une liste");
            let taille = vue["decks"]["discard"].as_u64().expect("la taille publiée");
            assert_eq!(
                liste.len() as u64,
                taille,
                "graine {graine}, manche {manches} : liste et taille divergent"
            );
        }
        assert!(manches > 1, "la partie a bien été jouée (graine {graine})");
    }
}

/// Zéro MC, zéro plante, zéro chaleur, aucune carte posée : plus rien n'est
/// faisable en phase Action, sauf ce que la MAIN offrait naguère.
fn demunir(game: &mut GameState, p: usize) {
    game.players[p].mc = 0;
    game.players[p].plants = 0;
    game.players[p].heat = 0;
    game.players[p].played.clear();
    // La corporation de l'extension Découverte peut porter une action : elle
    // s'offrirait à la phase Action et brouillerait la mesure. On la retire pour
    // ce test-ci, comme les cartes posées.
    game.players[p].corporation = None;
}

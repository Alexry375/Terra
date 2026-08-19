//! **Tests du lot « le secret et l'ordre » — six défauts du déroulement.**
//!
//! Chaque test cite en commentaire la ligne du livret qu'il fait respecter, et
//! chacun a été vu ROUGE sur le code d'avant : le protocole est décrit dans
//! `workspaces/le-secret-et-l-ordre/outputs/result.md`, §Verification.
//!
//! Aucun test ne fabrique un état à la main pour l'observer ensuite : tous
//! partent d'une VRAIE partie jouée par le moteur (`setup_game`, `play_round`,
//! `play_game`, `run_simulation`). Quand un test a besoin d'une situation
//! particulière — une égalité parfaite, une production de cartes — il la pose
//! SUR une partie réelle, comme le fait déjà le reste de la suite.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{
    allowed_phases, play_round, score_breakdown, setup_game, tiebreak_total, winner,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;

const CARTES: &str = "../data/cards.json";

fn boite_de_base() -> CardsDb {
    CardsDb::load_boites(CARTES, BoiteSet::parse("base").unwrap()).expect("boîte de base")
}

fn base_et_extension() -> CardsDb {
    CardsDb::load_boites(CARTES, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base + extension")
}

// =========================================================================
// L'ESPION — une politique qui joue normalement mais note, à chaque point de
// décision, CE QUE LE JOUEUR QUI DÉCIDE A SOUS LES YEUX.
//
// C'est le même CROCHET que la fiche de situation emprunte (`Policy::observe`,
// appelé par `flow::observer`) : l'espion regarde donc au moment exact où la
// fiche est calculée, et jamais « par-dessus l'épaule » du moteur à un instant
// que le jeu réel ne produit pas.
//
// **Ce qu'il ne fait pas, et il faut le dire** : il ne recopie pas les 1472
// cases du vecteur, mais les grandeurs par lesquelles les fuites de ce lot
// passent — la carte Phase révélée, la défausse carte par carte, les
// corporations, le nombre de cartes en main, les MC. Il ne peut donc pas voir
// une fuite qui passerait par la chaleur, les plantes ou une carte posée. La
// raison est structurelle : `engine/src/description.rs` n'appartient pas à la
// bibliothèque `engine`, il est inclus par `#[path]` dans les binaires, et un
// test d'intégration ne peut pas l'appeler. La comparaison case pour case des
// 1472 entrées existe ailleurs, sur les binaires : le contrôle 01 du lot, et le
// banc `web/webapp/verif/juge-descriptions.mjs`.
// =========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
struct Vue {
    joueur: usize,
    generation: u32,
    phase_en_cours: u8,
    /// Ce que la TABLE montre : la carte Phase révélée de chacun.
    revelee: [Option<u8>; NUM_PLAYERS],
    /// Ce que chaque joueur sait de SA propre carte — privé, jamais publié.
    precedente: [Option<u8>; NUM_PLAYERS],
    /// Le CONTENU de la défausse, pas seulement sa taille : c'est l'identité
    /// des cartes rendues que la fiche de situation publie, case par case
    /// (`projet7_defausse`, `projet50_defausse`, …). Compter les cartes ne
    /// suffirait pas à voir la fuite — deux joueurs qui rendent trois cartes
    /// chacun en rendent trois dans les deux cas.
    defausse: Vec<u16>,
    corporations: [Option<u16>; NUM_PLAYERS],
    mains: [usize; NUM_PLAYERS],
    mc: [i64; NUM_PLAYERS],
}

impl Vue {
    /// Tout ce qui est PUBLIC dans cette vue — c'est-à-dire tout sauf le champ
    /// privé `previous_phase`, que la fiche de situation ne publie plus.
    #[allow(clippy::type_complexity)]
    fn publique(
        &self,
    ) -> (
        usize,
        u32,
        u8,
        [Option<u8>; NUM_PLAYERS],
        Vec<u16>,
        [Option<u16>; NUM_PLAYERS],
        [usize; NUM_PLAYERS],
        [i64; NUM_PLAYERS],
    ) {
        (
            self.joueur,
            self.generation,
            self.phase_en_cours,
            self.revelee,
            self.defausse.clone(),
            self.corporations,
            self.mains,
            self.mc,
        )
    }
}

struct Espion {
    fond: RandomPolicy,
    vues: Vec<Vue>,
    /// Premier joueur de la manche, relevé à chaque observation.
    premier: usize,
    /// Phase imposée au premier joueur (sinon : au hasard).
    phase_du_premier: Option<u8>,
    /// Phase imposée aux DEUX joueurs.
    phase_pour_tous: Option<u8>,
    /// Cartes rendues au mulligan par le premier joueur.
    mulligan_du_premier: Option<Vec<usize>>,
    /// Corporation retenue par le premier joueur.
    corpo_du_premier: Option<usize>,
}

impl Espion {
    fn nouveau() -> Espion {
        Espion {
            fond: RandomPolicy,
            vues: Vec::new(),
            premier: 0,
            phase_du_premier: None,
            phase_pour_tous: None,
            mulligan_du_premier: None,
            corpo_du_premier: None,
        }
    }
    /// Les vues offertes au joueur `j`, dans l'ordre.
    fn vues_de(&self, j: usize) -> Vec<Vue> {
        self.vues.iter().filter(|v| v.joueur == j).cloned().collect()
    }
}

impl Policy for Espion {
    fn observe(&mut self, game: &GameState, player: usize) {
        self.premier = game.first_player;
        self.vues.push(Vue {
            joueur: player,
            generation: game.generation,
            phase_en_cours: game.phase_en_cours,
            revelee: [game.players[0].phase_revelee, game.players[1].phase_revelee],
            precedente: [game.players[0].previous_phase, game.players[1].previous_phase],
            defausse: {
                let mut d = game.discard.clone();
                d.sort_unstable();
                d
            },
            corporations: [game.players[0].corporation, game.players[1].corporation],
            mains: [game.players[0].hand.len(), game.players[1].hand.len()],
            mc: [game.players[0].mc, game.players[1].mc],
        });
    }
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        match &self.mulligan_du_premier {
            Some(v) if p == self.premier => v.clone(),
            Some(_) => Vec::new(),
            None => self.fond.project_mulligan(r, p, h),
        }
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        match self.corpo_du_premier {
            Some(i) if p == self.premier => i.min(c.len() - 1),
            Some(_) => 0,
            None => self.fond.pick_corporation(r, p, c),
        }
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        if let Some(ph) = self.phase_pour_tous {
            return if allowed.contains(&ph) { ph } else { allowed[0] };
        }
        if p == self.premier {
            if let Some(ph) = self.phase_du_premier {
                return if allowed.contains(&ph) { ph } else { allowed[0] };
            }
        }
        self.fond.pick_phase(r, p, allowed)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.fond.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.fond.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.fond.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.fond.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.fond.discard_down(r, p, h, n)
    }
}

/// Une manche de planification jouée pour de vrai, avec une phase imposée au
/// premier joueur. Rend (premier joueur, vues de la manche — la mise en place
/// étant mise de côté).
fn manche_avec_phase_imposee(db: &CardsDb, graine: u64, phase: u8) -> (usize, Vec<Vue>) {
    let mut pol = Espion::nouveau();
    let mut jeu = setup_game(db, graine, &mut pol);
    let premier = jeu.first_player;
    pol.vues.clear();
    pol.phase_du_premier = Some(phase);
    play_round(&mut jeu, db, &mut pol);
    (premier, pol.vues)
}

// =========================================================================
// D1 — LE SECRET DE LA CARTE PHASE
//
// docs/regles/livret-base.md:268 — « Chaque joueur choisit SIMULTANÉMENT une
// carte Phase de sa main et la place, FACE CACHÉE, devant lui. »
// docs/regles/livret-base.md:272 — « Une fois que TOUS les joueurs ont fait
// leur choix, les cartes Phase choisies sont révélées. »
// =========================================================================

/// Le secret : deux parties identiques où seul change le choix CACHÉ du premier
/// joueur doivent montrer au second, au moment où il choisit, exactement la
/// même chose (livret `docs/regles/livret-base.md:268`).
#[test]
fn d1_le_secret_de_la_carte_phase_ne_fuit_pas_vers_le_second_joueur() {
    let db = base_et_extension();
    for graine in [700001u64, 1000042, 1000777, 2000013, 4242] {
        let (premier, a) = manche_avec_phase_imposee(&db, graine, 1);
        let (premier_b, b) = manche_avec_phase_imposee(&db, graine, 5);
        assert_eq!(premier, premier_b, "même graine, même premier joueur");
        let second = 1 - premier;
        let va = a.iter().find(|v| v.joueur == second).expect("le second décide");
        let vb = b.iter().find(|v| v.joueur == second).expect("le second décide");
        assert_eq!(
            va.publique(),
            vb.publique(),
            "graine {graine} : la vue du siège {second} change selon le choix caché du siège {premier}"
        );
    }
}

/// Et la preuve que la mesure a bien eu lieu : le choix du premier joueur a
/// RÉELLEMENT changé d'une partie à l'autre. Sans cette garde, le test
/// ci-dessus passerait même si les deux parties étaient identiques.
#[test]
fn d1_le_choix_cache_du_premier_joueur_a_bien_change_entre_les_deux_parties() {
    let db = base_et_extension();
    let (premier, a) = manche_avec_phase_imposee(&db, 700001, 1);
    let (_, b) = manche_avec_phase_imposee(&db, 700001, 5);
    let second = 1 - premier;
    let va = a.iter().find(|v| v.joueur == second).unwrap();
    let vb = b.iter().find(|v| v.joueur == second).unwrap();
    assert_eq!(va.precedente[premier], Some(1));
    assert_eq!(vb.precedente[premier], Some(5));
}

/// La révélation : une fois la manche jouée, la carte de chacun est retournée
/// (livret `docs/regles/livret-base.md:272`).
#[test]
fn d1_la_carte_phase_est_revelee_quand_tous_ont_choisi() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    let mut jeu = setup_game(&db, 700001, &mut pol);
    play_round(&mut jeu, &db, &mut pol);
    for p in 0..NUM_PLAYERS {
        assert_eq!(
            jeu.players[p].phase_revelee,
            Some(jeu.players[p].chosen_phase),
            "après la manche, la carte du joueur {p} est retournée"
        );
    }
}

/// En manche 1, personne n'a encore rien révélé : pendant la planification, les
/// six cases de la carte Phase disent « aucune » des deux côtés
/// (livret `docs/regles/livret-base.md:268`).
#[test]
fn d1_en_manche_1_aucune_carte_phase_n_est_revelee_pendant_la_planification() {
    let db = base_et_extension();
    let (_, vues) = manche_avec_phase_imposee(&db, 1000042, 3);
    let planification: Vec<&Vue> = vues.iter().filter(|v| v.phase_en_cours == 0).collect();
    assert!(!planification.is_empty(), "la planification a bien eu lieu");
    for v in planification {
        assert_eq!(v.revelee, [None, None], "manche 1 : rien n'est encore révélé");
    }
}

/// En manche 2, la carte montrée est celle de la manche 1 — exactement ce qu'un
/// joueur humain lit sur la pile de cartes déjà jouées
/// (livret `docs/regles/livret-base.md:272`).
#[test]
fn d1_en_manche_2_la_phase_montree_est_celle_de_la_manche_precedente() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    let mut jeu = setup_game(&db, 1000042, &mut pol);
    play_round(&mut jeu, &db, &mut pol);
    let manche1 = [jeu.players[0].chosen_phase, jeu.players[1].chosen_phase];
    pol.vues.clear();
    if !jeu.game_over {
        play_round(&mut jeu, &db, &mut pol);
        let planification: Vec<&Vue> = pol
            .vues
            .iter()
            .filter(|v| v.phase_en_cours == 0 && v.generation == 2)
            .collect();
        assert!(!planification.is_empty(), "la manche 2 a bien planifié");
        for v in planification {
            assert_eq!(
                v.revelee,
                [Some(manche1[0]), Some(manche1[1])],
                "pendant la planification de la manche 2, on ne voit que la manche 1"
            );
        }
    }
}

/// **Le gain de révélation ne fuit pas non plus.** Une carte du paquet verse
/// 1 MC « lorsque vous révélez une carte Phase améliorée ». Ce gain était versé
/// à la seconde où le joueur répondait, donc avant que l'autre ait répondu : le
/// MC apparaissait dans la fiche de l'adversaire et lui apprenait que son
/// voisin venait de révéler une carte améliorée — un renseignement sur un choix
/// encore secret. Le livret place la révélation APRÈS que tous ont choisi
/// (`docs/regles/livret-base.md:272`), et le gain avec elle.
#[test]
fn d1_le_gain_de_revelation_d_une_carte_phase_amelioree_ne_fuit_pas() {
    let db = base_et_extension();
    // La seule carte du paquet qui porte ce gain, retrouvée par son nom dans la
    // table de cartes — pas recopiée dans le test.
    let porteuse = db
        .projects
        .iter()
        .position(|c| c.name == "Communications Streamlining")
        .expect("la carte porteuse du gain de révélation existe") as u16;

    // Deux parties réelles, identiques, où seul change le choix caché du
    // premier joueur : l'une prend la carte Phase AMÉLIORÉE, l'autre non.
    let vue_du_second = |phase_du_premier: u8| -> Vue {
        let mut pol = Espion::nouveau();
        let mut jeu = setup_game(&db, 700001, &mut pol);
        let premier = jeu.first_player;
        // Arrangé SUR une partie réelle, comme le fait déjà le reste de la
        // suite : le premier joueur a la carte porteuse en jeu et sa carte
        // Phase 2 améliorée.
        jeu.players[premier].played.push(porteuse);
        jeu.players[premier].upgrade_phase(2, PhaseUpgrade::VariantA);
        pol.vues.clear();
        pol.phase_du_premier = Some(phase_du_premier);
        play_round(&mut jeu, &db, &mut pol);
        pol.vues
            .iter()
            .find(|v| v.joueur == 1 - premier)
            .expect("le second joueur décide")
            .clone()
    };
    let avec = vue_du_second(2); // carte Phase améliorée : le gain se déclenche
    let sans = vue_du_second(3); // carte Phase de base : rien ne se déclenche
    assert_eq!(
        avec.publique(),
        sans.publique(),
        "le gain de révélation du premier joueur se voit dans la fiche du second"
    );
}

/// Le champ privé continue de faire son travail : on ne rejoue pas la même
/// phase deux manches de suite (livret `docs/regles/livret-base.md:270`).
#[test]
fn d1_la_phase_precedente_reste_interdite_a_la_manche_suivante() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    let mut jeu = setup_game(&db, 700001, &mut pol);
    play_round(&mut jeu, &db, &mut pol);
    for p in 0..NUM_PLAYERS {
        let permises = allowed_phases(&jeu.players[p]);
        assert_eq!(permises.len(), 4, "quatre phases restent permises");
        assert!(
            !permises.contains(&jeu.players[p].chosen_phase),
            "la phase de la manche précédente est interdite"
        );
    }
}

// =========================================================================
// D14 — LA MISE EN PLACE EST SIMULTANÉE
//
// Arbitrage d'Alexis du 19-08 : au mulligan de DÉPART, les deux joueurs
// reçoivent l'information en même temps. Le livret ne tranche pas ce point ;
// il fixe en revanche le contenu de la mise en place
// (`docs/regles/livret-base.md:96` pour les cartes Projet).
// =========================================================================

fn mise_en_place_avec(db: &CardsDb, graine: u64, rendus: Vec<usize>, corpo: usize) -> (usize, Vec<Vue>) {
    let mut pol = Espion::nouveau();
    pol.mulligan_du_premier = Some(rendus);
    pol.corpo_du_premier = Some(corpo);
    let jeu = setup_game(db, graine, &mut pol);
    (jeu.first_player, pol.vues)
}

/// Au mulligan des projets, celui qui répond en second ne voit pas les cartes
/// que l'autre vient de rendre.
#[test]
fn d14_la_mise_en_place_ne_montre_pas_au_second_les_cartes_rendues_par_le_premier() {
    let db = base_et_extension();
    for graine in [7u64, 1000005, 1000311, 700001] {
        let (premier, a) = mise_en_place_avec(&db, graine, vec![0, 1, 2], 0);
        let (_, b) = mise_en_place_avec(&db, graine, vec![3, 4, 5], 0);
        let second = 1 - premier;
        // La vue du second AU MOMENT DE SON PROPRE MULLIGAN : la deuxième fois
        // qu'on l'interroge (la première étant le mulligan des corporations).
        let va = &a.iter().filter(|v| v.joueur == second).nth(1).expect("mulligan du second").clone();
        let vb = &b.iter().filter(|v| v.joueur == second).nth(1).expect("mulligan du second").clone();
        assert_eq!(
            va.publique(),
            vb.publique(),
            "graine {graine} : au mulligan, le siège {second} voit ce que l'autre a rendu"
        );
    }
}

/// Au choix final, celui qui répond en second ne voit pas la corporation que
/// l'autre a installée. Le livret distribue deux corporations à chacun et fait
/// choisir chacun (`docs/regles/livret-base.md:211`) sans dire qui choisit en
/// premier ; l'arbitrage du 19-08 tranche : l'information est simultanée.
#[test]
fn d14_la_mise_en_place_ne_montre_pas_au_second_la_corporation_du_premier() {
    let db = base_et_extension();
    for graine in [7u64, 1000005, 1000311, 700001] {
        let (premier, a) = mise_en_place_avec(&db, graine, vec![], 0);
        let (_, b) = mise_en_place_avec(&db, graine, vec![], 1);
        let second = 1 - premier;
        let va = a.iter().filter(|v| v.joueur == second).nth(2).expect("choix du second").clone();
        let vb = b.iter().filter(|v| v.joueur == second).nth(2).expect("choix du second").clone();
        assert_eq!(
            va.publique(),
            vb.publique(),
            "graine {graine} : au choix final, le siège {second} voit la corporation de l'autre"
        );
    }
}

/// La simultanéité n'a rien cassé : les deux mulligans sont bel et bien
/// appliqués, et chacun garde les huit cartes Projet que le livret lui
/// distribue (`docs/regles/livret-base.md:207`). L'échange lui-même est la
/// « règle avancée » du livret (`docs/regles/livret-base.md:213`), reprise en
/// règle maison.
#[test]
fn d14_la_mise_en_place_applique_bien_les_deux_mulligans() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    pol.mulligan_du_premier = Some(vec![0, 1, 2]);
    let jeu = setup_game(&db, 7, &mut pol);
    assert_eq!(jeu.discard.len(), 3, "trois cartes rendues, trois défaussées");
    for p in 0..NUM_PLAYERS {
        assert_eq!(jeu.players[p].hand.len(), 8, "chacun garde huit cartes");
        assert!(jeu.players[p].corporation.is_some(), "chacun a sa corporation");
    }
}

/// La règle maison du mulligan des corporations tient toujours : tout ou rien,
/// les DEUX cartes sont remplacées. Le livret n'échange que les cartes Projet
/// (`docs/regles/livret-base.md:213`) ; l'échange des deux corporations est une
/// règle maison d'Alexis, et ce test vérifie qu'elle n'a pas bougé.
#[test]
fn d14_le_mulligan_des_corporations_reste_tout_ou_rien() {
    struct Echangeur(RandomPolicy);
    impl Policy for Echangeur {
        fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
            true
        }
        fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
            Vec::new()
        }
        fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
            0
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
        fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
            self.0.research_keep(r, p, d, k)
        }
        fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
            self.0.discard_down(r, p, h, n)
        }
    }
    let db = base_et_extension();
    let mut pol = Echangeur(RandomPolicy);
    let jeu = setup_game(&db, 7, &mut pol);
    // Deux joueurs × deux corporations rendues, plus la corporation non retenue
    // de chacun : la pile des corporations écartées en porte six.
    assert_eq!(jeu.corp_discard.len(), 6, "tout ou rien : 2 + 2 rendues, 1 + 1 écartées");
}

/// La simultanéité s'arrête à la mise en place : EN COURS DE PARTIE, la
/// défausse redevient publique (arbitrage du 19-08).
#[test]
fn d14_apres_la_mise_en_place_la_defausse_redevient_publique() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    pol.mulligan_du_premier = Some(vec![0, 1, 2]);
    let mut jeu = setup_game(&db, 7, &mut pol);
    let premier = jeu.first_player;
    pol.vues.clear();
    play_round(&mut jeu, &db, &mut pol);
    let second = 1 - premier;
    let vues = pol.vues_de(second);
    assert!(!vues.is_empty(), "le second a bien décidé pendant la manche");
    assert!(
        vues.iter().any(|v| v.defausse.len() >= 3),
        "en cours de partie, le second voit la défausse de la mise en place"
    );
}

// =========================================================================
// D10 — OBJECTIFS ET RÉCOMPENSES : UN MODULE DE L'EXTENSION
//
// docs/regles/livret-decouverte.md:51 — les tuiles Récompense et Objectif sont
// posées par la mise en place de l'EXTENSION.
// docs/regles/livret-base.md:455 à 459 — le décompte final de la boîte de base
// ne connaît que le NT, les forêts et les PV des cartes jouées.
// =========================================================================

/// En boîte de base seule, aucun point d'Objectif ni de Récompense.
#[test]
fn d10_aucun_point_d_objectif_ni_de_recompense_en_boite_de_base() {
    let db = boite_de_base();
    for graine in [3u64, 11, 1000123, 700001, 42] {
        let mut pol = RandomPolicy;
        let mut jeu = setup_game(&db, graine, &mut pol);
        let (parts, _, _) = score_breakdown(&jeu, &db);
        for p in 0..NUM_PLAYERS {
            assert_eq!(parts[p].awards, 0, "graine {graine} : récompense en boîte de base");
            assert_eq!(parts[p].milestones, 0, "graine {graine} : objectif en boîte de base");
        }
        while !jeu.game_over && jeu.generation <= 20 {
            play_round(&mut jeu, &db, &mut pol);
        }
        let (fin, _, _) = score_breakdown(&jeu, &db);
        for p in 0..NUM_PLAYERS {
            assert_eq!(fin[p].awards, 0, "graine {graine} : récompense au décompte final");
            assert_eq!(fin[p].milestones, 0, "graine {graine} : objectif au décompte final");
        }
    }
}

/// Le score de la boîte de base se réduit aux trois termes du livret
/// (`docs/regles/livret-base.md:455`).
#[test]
fn d10_le_score_de_la_boite_de_base_se_reduit_aux_trois_termes_du_livret() {
    let db = boite_de_base();
    for graine in [3u64, 11, 1000123] {
        let mut pol = RandomPolicy;
        let mut jeu = setup_game(&db, graine, &mut pol);
        while !jeu.game_over && jeu.generation <= 20 {
            play_round(&mut jeu, &db, &mut pol);
        }
        let (parts, _, _) = score_breakdown(&jeu, &db);
        for p in 0..NUM_PLAYERS {
            assert_eq!(
                parts[p].total(),
                parts[p].tr + parts[p].forests + parts[p].cards,
                "boîte de base : le total ne contient que NT + forêts + cartes"
            );
        }
    }
}

/// Avec l'extension, les Récompenses comptent de nouveau
/// (`docs/regles/livret-decouverte.md:51`).
#[test]
fn d10_les_recompenses_comptent_de_nouveau_avec_l_extension() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let jeu = setup_game(&db, 3, &mut pol);
    let (parts, _, _) = score_breakdown(&jeu, &db);
    let total: i64 = (0..NUM_PLAYERS).map(|p| parts[p].awards).sum();
    assert!(total > 0, "avec l'extension, les récompenses distribuent des points");
}

/// Le TIRAGE, lui, reste en place même en boîte de base : le retirer décalerait
/// le générateur de hasard et ferait diverger toutes les parties enregistrées.
#[test]
fn d10_le_tirage_des_objectifs_et_recompenses_reste_en_place_en_boite_de_base() {
    let db = boite_de_base();
    let mut vus = std::collections::BTreeSet::new();
    for graine in 1..=12u64 {
        let mut pol = RandomPolicy;
        let jeu = setup_game(&db, graine, &mut pol);
        let mut clef = String::new();
        for slot in &jeu.milestones {
            clef.push_str(&format!("{:?}/", slot.kind));
        }
        for a in &jeu.awards {
            clef.push_str(&format!("{a:?}/"));
        }
        vus.insert(clef);
    }
    assert!(
        vus.len() > 1,
        "le tirage des Objectifs et Récompenses est toujours fait : il varie avec la graine"
    );
}

// =========================================================================
// D15 — L'EXTENSION NE SE JOUE PAS SEULE
//
// Décision d'Alexis du 19-08 : la configuration est refusée AU CHARGEMENT.
// L'extension n'apporte que quatre corporations, et la mise en place en
// distribue déjà quatre (`docs/regles/livret-base.md:96` : la mise en place
// des cartes Projet suit celle des corporations).
// =========================================================================

/// L'extension seule est refusée, et refusée AVANT qu'une partie ne démarre.
#[test]
fn d15_l_extension_seule_est_refusee_au_chargement() {
    let r = BoiteSet::parse("decouverte");
    assert!(r.is_err(), "« decouverte » seule doit être refusée");
}

/// Et le refus le dit clairement, sans jargon de pioche vide.
#[test]
fn d15_le_message_de_refus_de_l_extension_seule_est_explicite() {
    let e = BoiteSet::parse("decouverte").unwrap_err();
    let bas = e.to_lowercase();
    assert!(bas.contains("base"), "le message nomme la boîte de base : « {e} »");
    assert!(
        !bas.contains("paquet corporations"),
        "le refus n'est pas un plantage de mise en place : « {e} »"
    );
}

/// Les configurations légitimes restent acceptées. Le livret de base se joue
/// seul — c'est lui qui distribue les corporations
/// (`docs/regles/livret-base.md:211`) —, l'extension ne fait que s'y ajouter
/// (`docs/regles/livret-decouverte.md:51`).
#[test]
fn d15_la_boite_de_base_avec_ou_sans_extension_reste_acceptee() {
    assert!(BoiteSet::parse("base").is_ok());
    assert!(BoiteSet::parse("base,decouverte").is_ok());
    assert!(BoiteSet::parse("base,promo,decouverte").is_ok());
    assert!(CardsDb::load_boites(CARTES, BoiteSet::parse("base").unwrap()).is_ok());
    assert!(CardsDb::load_boites(CARTES, BoiteSet::parse("base,decouverte").unwrap()).is_ok());
}

/// Aucune configuration acceptée ne contient l'extension sans la boîte de base.
/// On le vérifie sur TOUTES les combinaisons de boîtes qui s'écrivent, pas sur
/// la seule qui pose problème. (La boîte promotionnelle, elle, n'est pas
/// concernée : le contrat ne tranche que le cas de l'extension.)
#[test]
fn d15_aucune_partie_ne_demarre_sans_la_boite_de_base() {
    let mots = ["base", "promo", "decouverte"];
    let mut acceptees = 0;
    let mut refusees = 0;
    for masque in 1..8u8 {
        let liste: Vec<&str> = (0..3)
            .filter(|i| masque & (1 << i) != 0)
            .map(|i| mots[i])
            .collect();
        let texte = liste.join(",");
        let contient_base = liste.contains(&"base");
        let contient_extension = liste.contains(&"decouverte");
        match BoiteSet::parse(&texte) {
            Ok(_) => {
                acceptees += 1;
                assert!(
                    contient_base || !contient_extension,
                    "« {texte} » accepté : l'extension se jouerait seule"
                );
            }
            Err(_) => {
                refusees += 1;
                assert!(
                    contient_extension && !contient_base,
                    "« {texte} » refusé sans raison"
                );
            }
        }
    }
    assert_eq!(acceptees, 5, "cinq combinaisons légitimes");
    assert_eq!(refusees, 2, "« decouverte » et « promo,decouverte » sont refusées");
}

// =========================================================================
// D11 — LE DÉPARTAGE D'ÉGALITÉ DU LIVRET
//
// docs/regles/livret-base.md:461 — « Le joueur ayant le plus grand nombre de
// PVs remporte la partie. En cas d'égalité, le joueur à égalité ayant le plus
// grand total cumulé de chaleur, de MC et de plantes est déclaré vainqueur.
// Veillez à convertir au préalable toutes les cartes Projet en main en MC. »
// docs/regles/livret-base.md:96 — une carte Projet en main vaut 3 MC.
// =========================================================================

/// Le total de départage additionne bien chaleur, MC et plantes
/// (`docs/regles/livret-base.md:461`).
#[test]
fn d11_le_total_de_departage_cumule_chaleur_mc_et_plantes() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let mut jeu = setup_game(&db, 1000001, &mut pol);
    while !jeu.game_over && jeu.generation <= 20 {
        play_round(&mut jeu, &db, &mut pol);
    }
    // On n'écrit pas la formule une seconde fois — ce serait la répéter, pas la
    // vérifier. On mesure ce que CHAQUE ressource pèse dans le total, et ce que
    // pèsent celles que le livret ne cite pas.
    let avant = tiebreak_total(&jeu.players[0]);
    jeu.players[0].heat += 5;
    assert_eq!(tiebreak_total(&jeu.players[0]) - avant, 5, "la chaleur compte pour 1");
    jeu.players[0].mc += 7;
    assert_eq!(tiebreak_total(&jeu.players[0]) - avant, 12, "les MC comptent pour 1");
    jeu.players[0].plants += 3;
    assert_eq!(tiebreak_total(&jeu.players[0]) - avant, 15, "les plantes comptent pour 1");
    // Et ce que le livret ne cite PAS n'entre pas dans le total : ni le niveau
    // de terraformation, ni les forêts, qui sont des points de victoire et ont
    // déjà départagé au premier critère.
    jeu.players[0].tr += 20;
    jeu.players[0].forests += 4;
    assert_eq!(
        tiebreak_total(&jeu.players[0]) - avant,
        15,
        "ni le NT ni les forêts n'entrent dans le total de départage"
    );
}

/// Une carte Projet en main vaut exactement 3 MC au départage
/// (`docs/regles/livret-base.md:96`).
#[test]
fn d11_une_carte_projet_en_main_vaut_trois_mc_au_departage() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let mut jeu = setup_game(&db, 1000001, &mut pol);
    while !jeu.game_over && jeu.generation <= 20 {
        play_round(&mut jeu, &db, &mut pol);
    }
    let avant = tiebreak_total(&jeu.players[0]);
    let carte = jeu.deck.pop().expect("il reste une carte dans la pioche");
    jeu.players[0].hand.push(carte);
    assert_eq!(tiebreak_total(&jeu.players[0]) - avant, 3);
}

/// **L'ORDRE des deux critères** : à points inégaux, le départage ne sert pas,
/// même si le perdant croule sous les ressources
/// (`docs/regles/livret-base.md:461` : « le joueur ayant le plus grand nombre de
/// PVs remporte la partie » — le second critère ne s'applique qu'« en cas
/// d'égalité »).
#[test]
fn d11_a_pv_inegaux_le_departage_ne_sert_pas() {
    let db = base_et_extension();
    let mut mesurees = 0;
    for graine in [1000001u64, 1000002, 1000003, 1000004, 1000005] {
        let mut pol = RandomPolicy;
        let mut jeu = setup_game(&db, graine, &mut pol);
        while !jeu.game_over && jeu.generation <= 20 {
            play_round(&mut jeu, &db, &mut pol);
        }
        let (parts, _, _) = score_breakdown(&jeu, &db);
        let (s0, s1) = (parts[0].total(), parts[1].total());
        if s0 == s1 {
            continue;
        }
        mesurees += 1;
        let gagnant = if s0 > s1 { 0 } else { 1 };
        let perdant = 1 - gagnant;
        // On écrase le perdant de ressources : mille de chaque, et une main
        // pleine. Le vainqueur ne doit pas changer d'un pouce.
        jeu.players[perdant].heat += 1000;
        jeu.players[perdant].mc += 1000;
        jeu.players[perdant].plants += 1000;
        assert!(
            tiebreak_total(&jeu.players[perdant]) > tiebreak_total(&jeu.players[gagnant]),
            "graine {graine} : le perdant a bien le plus grand total cumulé"
        );
        assert_eq!(
            winner(&jeu, &db),
            Some(gagnant),
            "graine {graine} : les points passent avant le total cumulé"
        );
    }
    assert!(mesurees >= 3, "seulement {mesurees} parties à points inégaux mesurées");
}

/// À PV égaux, c'est le plus grand total cumulé qui l'emporte
/// (`docs/regles/livret-base.md:461`, seconde phrase).
#[test]
fn d11_a_pv_egaux_le_departage_designe_le_plus_grand_total_cumule() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let mut jeu = setup_game(&db, 1000001, &mut pol);
    while !jeu.game_over && jeu.generation <= 20 {
        play_round(&mut jeu, &db, &mut pol);
    }
    // On pose l'égalité de PV SUR la partie réelle, en alignant le niveau de
    // terraformation ; le reste de la partie est celui que le moteur a joué.
    let (parts, _, _) = score_breakdown(&jeu, &db);
    let ecart = parts[0].total() - parts[1].total();
    jeu.players[1].tr += ecart;
    let (parts, _, _) = score_breakdown(&jeu, &db);
    assert_eq!(parts[0].total(), parts[1].total(), "les PV sont désormais à égalité");
    // Le siège 1 prend une longueur d'avance en MC : il doit l'emporter.
    jeu.players[1].mc = jeu.players[0].mc + tiebreak_total(&jeu.players[0]) + 100;
    assert_eq!(winner(&jeu, &db), Some(1), "à PV égaux, le plus riche l'emporte");
    // Et dans l'autre sens.
    jeu.players[0].mc = jeu.players[1].mc + tiebreak_total(&jeu.players[1]) + 100;
    assert_eq!(winner(&jeu, &db), Some(0));
}

/// L'égalité PARFAITE — jusque sur le critère de départage — reste une partie
/// nulle (`docs/regles/livret-base.md:461`).
#[test]
fn d11_l_egalite_parfaite_reste_une_partie_nulle() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let mut jeu = setup_game(&db, 1000001, &mut pol);
    while !jeu.game_over && jeu.generation <= 20 {
        play_round(&mut jeu, &db, &mut pol);
    }
    let (parts, _, _) = score_breakdown(&jeu, &db);
    jeu.players[1].tr += parts[0].total() - parts[1].total();
    jeu.players[1].mc = jeu.players[0].mc;
    jeu.players[1].heat = jeu.players[0].heat;
    jeu.players[1].plants = jeu.players[0].plants;
    let n = jeu.players[0].hand.len();
    jeu.players[1].hand.truncate(n);
    while jeu.players[1].hand.len() < n {
        let c = jeu.deck.pop().expect("pioche non vide");
        jeu.players[1].hand.push(c);
    }
    assert_eq!(winner(&jeu, &db), None, "égalité parfaite : la partie est nulle");
}

/// Sur un échantillon de vraies parties, l'égalité devient l'exception
/// (mesure d'avant-correction : 11 parties nulles sur 400).
#[test]
fn d11_le_departage_fait_disparaitre_presque_toutes_les_egalites() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let bilan = run_simulation(&db, 200, 1000001, &mut pol);
    assert!(
        bilan.draws <= 1,
        "{} parties nulles sur 200 — le départage n'est pas appliqué",
        bilan.draws
    );
}

// =========================================================================
// D16 — LA PHASE IV SUIT L'ORDRE DU TOUR
//
// Règle maison d'Alexis (`docs/regles/README.md`, décision 24-07) : un premier
// joueur est désigné et alterne à chaque manche. La phase IV fait piocher dans
// le paquet COMMUN (livret `docs/regles/livret-base.md:96` : les cartes Projet
// viennent d'une pioche partagée), et parcourue par numéro de siège elle
// donnait toujours le dessus du paquet au siège 0.
// =========================================================================

/// Une manche de production jouée pour de vrai : c'est le PREMIER JOUEUR qui
/// prend la carte du dessus, quel que soit son numéro de siège.
#[test]
fn d16_la_phase_iv_de_production_suit_l_ordre_du_tour() {
    let db = base_et_extension();
    for premier in [0usize, 1] {
        let mut pol = Espion::nouveau();
        pol.phase_pour_tous = Some(4);
        let mut jeu = setup_game(&db, 700001, &mut pol);
        jeu.first_player = premier;
        for p in 0..NUM_PLAYERS {
            jeu.players[p].card_prod = 1;
            jeu.players[p].hand.clear();
        }
        let dessus = *jeu.deck.last().expect("pioche non vide");
        play_round(&mut jeu, &db, &mut pol);
        assert!(
            jeu.players[premier].hand.contains(&dessus),
            "premier joueur {premier} : c'est lui qui prend la carte du dessus"
        );
        assert!(
            !jeu.players[1 - premier].hand.contains(&dessus),
            "le second ne prend pas la carte du dessus"
        );
    }
}

/// Autrement dit : le siège 0 n'a plus la priorité automatique sur le paquet
/// commun en phase IV. Le livret fait piocher une carte par production de carte
/// pendant cette phase (`docs/regles/livret-base.md:403`) ; la pioche étant
/// commune, l'ordre dans lequel les joueurs y passent décide qui prend quoi.
#[test]
fn d16_la_production_ne_donne_plus_le_dessus_du_paquet_au_siege_zero() {
    let db = base_et_extension();
    let mut pol = Espion::nouveau();
    pol.phase_pour_tous = Some(4);
    let mut jeu = setup_game(&db, 700001, &mut pol);
    jeu.first_player = 1;
    for p in 0..NUM_PLAYERS {
        jeu.players[p].card_prod = 1;
        jeu.players[p].hand.clear();
    }
    let dessus = *jeu.deck.last().expect("pioche non vide");
    play_round(&mut jeu, &db, &mut pol);
    assert!(
        !jeu.players[0].hand.contains(&dessus),
        "le siège 0 ne prend plus le dessus du paquet quand il n'est pas premier joueur"
    );
}

// =========================================================================
// LE PREMIER JOUEUR EST TIRÉ AU SORT
//
// Arbitrage d'Alexis du 19-08 : tiré au sort au départ, puis alterné à chaque
// manche — l'alternance est la règle maison C4 de `docs/regles/README.md`.
// =========================================================================

/// Les deux sièges sortent : le premier joueur n'est plus fixé au siège 0.
/// Le livret ne connaît pas de premier joueur — il fait tout résoudre en
/// simultané (`docs/regles/livret-base.md:268`) ; l'ordre du tour est une règle
/// maison (`docs/regles/README.md`, décision 24-07) et son tirage au sort un
/// arbitrage d'Alexis du 19-08.
#[test]
fn premier_joueur_le_tirage_au_sort_fait_sortir_les_deux_sieges() {
    let db = base_et_extension();
    let mut zero = 0;
    let mut un = 0;
    for graine in 1..=60u64 {
        let mut pol = RandomPolicy;
        let jeu = setup_game(&db, graine, &mut pol);
        if jeu.first_player == 0 {
            zero += 1;
        } else {
            un += 1;
        }
    }
    assert!(zero >= 10 && un >= 10, "tirage déséquilibré : {zero} / {un} sur 60");
}

/// Le tirage passe par le générateur de hasard DE LA PARTIE : à graine égale,
/// le rejeu retrouve le même premier joueur. Même source que ci-dessus : règle
/// maison, le livret restant muet sur l'ordre du tour
/// (`docs/regles/livret-base.md:268`).
#[test]
fn premier_joueur_le_tirage_est_reproductible_a_graine_egale() {
    let db = base_et_extension();
    for graine in [1000007u64, 700001, 42, 3] {
        let mut a = RandomPolicy;
        let mut b = RandomPolicy;
        let ja = setup_game(&db, graine, &mut a);
        let jb = setup_game(&db, graine, &mut b);
        assert_eq!(ja.first_player, jb.first_player, "graine {graine}");
    }
}

/// L'ordre du tour de la manche 1 commence bien par le premier joueur tiré.
/// Règle maison C4 (`docs/regles/README.md`) ; le livret, lui, fait choisir
/// tous les joueurs en même temps (`docs/regles/livret-base.md:268`).
#[test]
fn premier_joueur_l_ordre_du_tour_commence_par_le_joueur_tire_au_sort() {
    let db = base_et_extension();
    for graine in [1000007u64, 700001, 42] {
        let mut pol = RandomPolicy;
        let mut jeu = setup_game(&db, graine, &mut pol);
        let tire = jeu.first_player;
        play_round(&mut jeu, &db, &mut pol);
        assert_eq!(jeu.turn_order[0], tire as u8, "graine {graine}");
    }
}

/// Et il alterne ensuite à chaque manche, comme avant : règle maison C4
/// (`docs/regles/README.md`, décision 24-07), qui s'ajoute au livret sans le
/// contredire — celui-ci ne fixe aucun ordre
/// (`docs/regles/livret-base.md:268`).
#[test]
fn premier_joueur_alterne_a_chaque_manche() {
    let db = base_et_extension();
    let mut pol = RandomPolicy;
    let mut jeu = setup_game(&db, 1000007, &mut pol);
    while !jeu.game_over && jeu.generation <= 20 {
        play_round(&mut jeu, &db, &mut pol);
    }
    assert!(jeu.turn_order.len() >= 2, "au moins deux manches jouées");
    for f in jeu.turn_order.windows(2) {
        assert_ne!(f[0], f[1], "le premier joueur alterne d'une manche à l'autre");
    }
}

/// La mise en place elle-même est parcourue dans l'ordre du tour : c'est le
/// premier joueur qui est interrogé en premier. Le livret distribue à « chaque
/// joueur » sans ordre (`docs/regles/livret-base.md:207` et `:211`) ; c'est la
/// règle maison d'ordre du tour qui décide qui répond d'abord.
#[test]
fn premier_joueur_la_mise_en_place_interroge_le_premier_joueur_en_premier() {
    let db = base_et_extension();
    for graine in [7u64, 700001, 1000777, 2000013] {
        let mut pol = Espion::nouveau();
        let jeu = setup_game(&db, graine, &mut pol);
        assert_eq!(
            pol.vues[0].joueur, jeu.first_player,
            "graine {graine} : la première question va au premier joueur"
        );
    }
}

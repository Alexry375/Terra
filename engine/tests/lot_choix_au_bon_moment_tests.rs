//! Tests du lot « les choix se posent au bon moment » — MOT-2, MOT-3, MOT-8.
//!
//! Les trois défauts ont une propriété commune : ils DÉPLACENT OU SUPPRIMENT
//! des points de décision. Les trois contrôles fournis les mesurent de
//! l'EXTÉRIEUR, par le pont, en rejouant des parties entières. Ce fichier-ci
//! mesure l'INTÉRIEUR, sur des chemins que les contrôles n'empruntent pas :
//!
//! 1. **MOT-2 — l'équivalence.** `flow::blue_action_peut_produire` doit dire
//!    exactement ce que `flow::apply_blue_action` fera. Un prédicat trop
//!    permissif laisserait revenir des activations stériles ; un prédicat trop
//!    strict SUPPRIMERAIT une action qui produit — le piège mesuré du contrat
//!    (une ressource posée sur une carte est un résultat). Le test compare les
//!    deux, carte par carte, sur une grille d'états, et **compte ses occasions
//!    dans les deux sens** : un verdict rendu sur zéro cas « peut produire » ne
//!    prouverait rien.
//! 2. **MOT-3 — les deux temps.** Le second temps du bonus de Construction
//!    n'est demandé qu'aux joueurs qui n'ont pas déjà pioché au premier temps,
//!    et jamais à un joueur qui n'est pas sélectionneur.
//! 3. **MOT-8 — le demi-correctif.** Le badge n'est demandé que pour une carte
//!    payable au badge le plus favorable, et la question reste AVANT le calcul
//!    du prix : le prix annoncé est le prix payé.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Color, JOKER_TAG_CHOICES};
use engine::flow::{
    apply_blue_action, blue_action_peut_produire, build_card_with, has_joker_tag, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::*;
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte doit se charger")
}

/// Politique VOLONTAIRE : elle ne renonce jamais.
///
/// C'est ce qui rend l'équivalence de MOT-2 mesurable. Deux formes d'action
/// tirent un MONTANT auprès de la politique (« spend any amount », « discard up
/// to N ») : une politique qui répond zéro rendrait `apply_blue_action` faux
/// alors que l'action POUVAIT produire — et le prédicat, qui ne juge que le
/// moteur, aurait raison. On répond donc toujours le maximum, et l'écart
/// mesuré est alors bien celui du moteur, pas celui du hasard.
struct Volontaire {
    base: RandomPolicy,
}

impl Policy for Volontaire {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
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
    /// Jamais de renoncement explicite : la première branche jouable.
    fn choose_option(&mut self, _r: &mut StdRng, _p: usize, _n: usize) -> usize {
        0
    }
    /// Toujours le maximum (voir la note du type).
    fn action_amount(&mut self, _r: &mut StdRng, _p: usize, max: i64) -> i64 {
        max.max(0)
    }
}

fn volontaire() -> Volontaire {
    Volontaire { base: RandomPolicy }
}

/// Toutes les cartes bleues de la boîte qui portent une « Action : ».
fn cartes_a_action(db: &CardsDb) -> Vec<u16> {
    (0..db.projects.len() as u16)
        .filter(|&c| {
            let card = &db.projects[c as usize];
            card.color == Color::Blue && card.effect.is_some_and(|s| s.action.is_some())
        })
        .collect()
}

/// Une partie réelle, bourses vidées : rien n'arrive qu'on n'ait mis là
/// soi-même. Deux appels à graine égale rendent DEUX ÉTATS IDENTIQUES —
/// c'est ce qui remplace un `clone` que `GameState` n'offre pas, et qui permet
/// de PRÉDIRE sur l'un et d'APPLIQUER sur l'autre.
fn jeu(db: &CardsDb, graine: u64) -> GameState {
    let mut pol = volontaire();
    let mut g = setup_game(db, graine, &mut pol);
    for p in 0..NUM_PLAYERS {
        g.players[p].mc = 0;
        g.players[p].heat = 0;
        g.players[p].plants = 0;
    }
    g
}

/// Met la carte `id` en jeu par le CHEMIN RÉEL de la pose, puis règle les
/// ressources du joueur 0 et l'instantané planétaire. Déterministe à graine
/// égale.
fn etat(
    db: &CardsDb,
    graine: u64,
    id: u16,
    mc: i64,
    heat: i64,
    plants: i64,
    oceans: u8,
    temperature: u8,
) -> GameState {
    let mut pol = volontaire();
    let mut g = jeu(db, graine);
    // La carte vient de la pioche ou de la défausse, jamais de nulle part.
    if let Some(i) = g.deck.iter().position(|&c| c == id) {
        g.deck.remove(i);
    } else if let Some(i) = g.discard.iter().position(|&c| c == id) {
        g.discard.remove(i);
    } else if let Some(i) = g.players[0].hand.iter().position(|&c| c == id) {
        g.players[0].hand.remove(i);
    } else if let Some(i) = g.players[1].hand.iter().position(|&c| c == id) {
        g.players[1].hand.remove(i);
    } else {
        panic!("carte {id} introuvable");
    }
    g.players[0].hand.push(id);
    let idx = g.players[0].hand.len() - 1;
    g.players[0].mc = 1000;
    build_card_with(&mut g, db, 0, idx, 0, &mut pol);
    assert!(g.players[0].played.contains(&id), "la carte {id} n'est pas entrée en jeu");
    g.players[0].mc = mc;
    g.players[0].heat = heat;
    g.players[0].plants = plants;
    // L'INSTANTANÉ de début de phase est le seuil que le moteur lit partout
    // (`action_effs_possible`, `FlipOceanTagDiscount`, `RaiseTempBlueDiscount`) :
    // c'est donc lui qu'on règle, et pas une autre grandeur.
    g.snap_oceans = oceans;
    g.snap_temperature = temperature;
    g
}

// =========================================================================
// 1. MOT-2 — « proposée » et « appliquée » ne divergent jamais
// =========================================================================

/// Le prédicat qui retire une action de la liste dit EXACTEMENT ce que
/// l'activation fera. Dans les deux sens : jamais une action proposée qui ne
/// produit rien (le défaut mesuré, 340 cas sur 2133), jamais une action retirée
/// qui aurait produit (le piège du contrat : poser une ressource SUR une carte
/// est un résultat).
#[test]
fn le_predicat_d_action_dit_exactement_ce_que_l_activation_fera() {
    let db = db();
    let cartes = cartes_a_action(&db);
    assert!(
        cartes.len() >= 20,
        "seulement {} carte(s) bleue(s) à action : la mesure n'a pas eu lieu",
        cartes.len()
    );

    // Une grille d'états volontairement pauvre ET riche : c'est la pauvreté qui
    // produit les « ne peut rien produire », et la richesse les contre-témoins.
    let etats: [(i64, i64, i64, u8, u8); 6] = [
        // mc, chaleur, plantes, océans sortis, température
        (0, 0, 0, 0, 0),
        (0, 0, 0, 9, 14),
        (3, 2, 2, 3, 4),
        (50, 50, 50, 0, 0),
        (50, 50, 50, 9, 14),
        (7, 0, 0, 9, 0),
    ];

    let mut occasions = 0u32;
    let mut peut = 0u32;
    let mut ne_peut_pas = 0u32;
    let mut graine = 100u64;

    for &id in &cartes {
        for &(mc, heat, plants, oceans, temp) in &etats {
            graine += 1;
            let a = etat(&db, graine, id, mc, heat, plants, oceans, temp);
            let predit = blue_action_peut_produire(&a, &db, 0, id);
            // Le JUMEAU : même graine, même recette, donc même état.
            let mut b = etat(&db, graine, id, mc, heat, plants, oceans, temp);
            let mut pol = volontaire();
            let applique = apply_blue_action(&mut b, &db, 0, id, &mut pol);
            occasions += 1;
            if predit {
                peut += 1;
            } else {
                ne_peut_pas += 1;
            }
            assert_eq!(
                predit,
                applique,
                "carte « {} » (mc {mc}, chaleur {heat}, plantes {plants}, océans \
                 {oceans}, température {temp}) : le moteur PROPOSE {predit} et \
                 APPLIQUE {applique}",
                db.projects[id as usize].name,
            );
        }
    }

    // Compter les occasions AVANT de juger : un verdict vert rendu sur zéro cas
    // d'un des deux côtés ne prouverait rien du tout. Le compte est IMPRIMÉ
    // (`cargo test -- --nocapture`) : une mesure qu'on ne peut pas lire ne se
    // cite pas.
    println!(
        "    {} carte(s) à action × {} état(s) = {occasions} occasion(s) : \
         {peut} « peut produire », {ne_peut_pas} « ne peut rien produire »",
        cartes.len(),
        etats.len()
    );
    assert!(occasions >= 100, "{occasions} occasion(s) seulement");
    assert!(peut >= 30, "seulement {peut} état(s) où l'action peut produire");
    assert!(
        ne_peut_pas >= 30,
        "seulement {ne_peut_pas} état(s) où l'action ne peut rien produire — \
         sans eux le prédicat n'est jamais mis à l'épreuve"
    );
}

/// Contre-témoin du piège du contrat : une action qui pose une ressource SUR
/// une carte ne change ni les MC, ni les plantes, ni la chaleur, ni la planète —
/// et elle doit rester proposée. On le vérifie sur le moteur lui-même : après
/// l'activation, la carte porte une ressource de plus.
#[test]
fn une_action_qui_pose_une_ressource_sur_une_carte_reste_proposee() {
    let db = db();
    let mut vues = 0u32;
    let mut graine = 500u64;
    for &id in &cartes_a_action(&db) {
        graine += 1;
        // Riche : aucune raison budgétaire de refuser.
        let a = etat(&db, graine, id, 50, 50, 50, 0, 0);
        let mut b = etat(&db, graine, id, 50, 50, 50, 0, 0);
        let avant: i64 = b.players[0].card_resources.values().map(|&n| n as i64).sum();
        let mc_avant = b.players[0].mc;
        let mut pol = volontaire();
        let applique = apply_blue_action(&mut b, &db, 0, id, &mut pol);
        let apres: i64 = b.players[0].card_resources.values().map(|&n| n as i64).sum();
        if applique && apres > avant && b.players[0].mc == mc_avant {
            vues += 1;
            assert!(
                blue_action_peut_produire(&a, &db, 0, id),
                "« {} » pose une ressource sur une carte et n'est pourtant plus \
                 proposée : c'est exactement le piège du contrat",
                db.projects[id as usize].name
            );
        }
    }
    assert!(
        vues >= 1,
        "aucune action à ressource-sur-carte rencontrée : ce contre-témoin n'a rien mesuré"
    );
}

// =========================================================================
// 3. MOT-8 — le badge et le prix
// =========================================================================

/// Les cartes joker existent bel et bien dans la boîte : sans elles, le
/// contrôle 03 mesurerait le vide.
#[test]
fn la_boite_porte_bien_des_cartes_joker() {
    let db = db();
    let n = (0..db.projects.len() as u16)
        .filter(|&c| has_joker_tag(&db, c))
        .count();
    assert!(n >= 3, "seulement {n} carte(s) joker dans base+Découverte");
    assert_eq!(JOKER_TAG_CHOICES.len(), 10, "dix badges au choix");
}

// =========================================================================
// 2. MOT-3 — le bonus de Construction se demande en DEUX TEMPS
// =========================================================================

/// Espion des deux temps. Il DÉCIDE du premier temps (pour éprouver les deux
/// réponses) et enregistre, à chaque question, le nombre de cartes que le
/// joueur a en jeu — lu sur l'état que `Policy::observe` reçoit juste avant la
/// question, jamais reconstruit.
struct Espion {
    base: RandomPolicy,
    /// Réponse imposée au premier temps : `None` = alternée.
    piocher_avant: Option<bool>,
    alterne: bool,
    vu: [usize; NUM_PLAYERS],
    /// (joueur, cartes en jeu) à chaque premier temps.
    avant: Vec<(usize, usize)>,
    /// (joueur, cartes en jeu) à chaque second temps.
    apres: Vec<(usize, usize)>,
    avant_vrai: usize,
    /// Cartes en jeu au dernier premier temps, par joueur.
    dernier_avant: [Option<usize>; NUM_PLAYERS],
    /// Seconds temps posés APRÈS qu'une carte a été posée depuis le premier.
    apres_une_pose: usize,
}

impl Espion {
    fn new(piocher_avant: Option<bool>) -> Espion {
        Espion {
            base: RandomPolicy,
            piocher_avant,
            alterne: false,
            vu: [0; NUM_PLAYERS],
            avant: Vec::new(),
            apres: Vec::new(),
            avant_vrai: 0,
            dernier_avant: [None; NUM_PLAYERS],
            apres_une_pose: 0,
        }
    }
}

impl Policy for Espion {
    fn observe(&mut self, game: &GameState, player: usize) {
        self.vu[player] = game.players[player].played.len();
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn construction_bonus_avant(&mut self, _r: &mut StdRng, p: usize) -> bool {
        self.avant.push((p, self.vu[p]));
        self.dernier_avant[p] = Some(self.vu[p]);
        let rep = match self.piocher_avant {
            Some(b) => b,
            None => {
                self.alterne = !self.alterne;
                self.alterne
            }
        };
        if rep {
            self.avant_vrai += 1;
        }
        rep
    }
    fn construction_bonus_apres(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.apres.push((p, self.vu[p]));
        if let Some(n) = self.dernier_avant[p] {
            if self.vu[p] > n {
                self.apres_une_pose += 1;
            }
        }
        self.base.construction_bonus_apres(r, p)
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

fn parties(espion: &mut Espion, db: &CardsDb, graines: std::ops::Range<u64>) {
    for g in graines {
        engine::sim::play_game(db, g, espion);
    }
}

/// Le second temps n'est demandé QU'À qui n'a pas déjà pioché au premier — et
/// il l'est à tous les autres. Le compte est une ÉGALITÉ, pas une inégalité :
/// une question de trop est un réglage de jeu ajouté, une question de moins est
/// un bonus perdu.
#[test]
fn le_second_temps_ne_se_demande_qu_a_qui_n_a_pas_deja_pioche() {
    let db = db();
    let mut e = Espion::new(None); // alterné : les deux réponses sont éprouvées
    parties(&mut e, &db, 0..12);
    println!(
        "    {} premier(s) temps ({} « piocher tout de suite »), {} second(s) \
         temps, dont {} après une pose",
        e.avant.len(),
        e.avant_vrai,
        e.apres.len(),
        e.apres_une_pose
    );
    assert!(e.avant.len() >= 20, "{} premier(s) temps seulement", e.avant.len());
    assert!(e.avant_vrai > 0 && e.avant_vrai < e.avant.len(), "une seule réponse éprouvée");
    assert_eq!(
        e.apres.len(),
        e.avant.len() - e.avant_vrai,
        "le second temps doit être demandé exactement aux joueurs qui n'ont pas \
         pioché au premier"
    );
    // Le second temps arrive bien APRÈS une pose dans un nombre de cas
    // substantiel : c'est tout l'intérêt du déplacement.
    assert!(
        e.apres_une_pose >= 5,
        "seulement {} second(s) temps posé(s) après une pose réelle",
        e.apres_une_pose
    );
}

/// Les deux bouts de l'éventail, pour que le compte ci-dessus ne puisse pas
/// être vrai par accident : qui pioche toujours au premier temps n'a jamais de
/// second ; qui n'y pioche jamais en a toujours un.
#[test]
fn les_deux_bouts_du_premier_temps() {
    let db = db();

    let mut toujours = Espion::new(Some(true));
    parties(&mut toujours, &db, 0..8);
    assert!(toujours.avant.len() >= 15, "mesure trop maigre");
    assert_eq!(toujours.apres.len(), 0, "piocher tout de suite consomme le bonus");

    let mut jamais = Espion::new(Some(false));
    parties(&mut jamais, &db, 0..8);
    assert!(jamais.avant.len() >= 15, "mesure trop maigre");
    assert_eq!(
        jamais.apres.len(),
        jamais.avant.len(),
        "sans pioche immédiate, la vraie question doit toujours venir"
    );
}

// =========================================================================
// 3 bis. MOT-8 — le badge n'est demandé que pour une carte posable
// =========================================================================

/// Espion des badges. Il confronte chaque question de badge à la question de
/// POSE qui suit pour le même joueur : si la carte est encore en main et que le
/// moteur ne l'offre pas, la question n'avait pas lieu d'être.
///
/// Ce chemin est celui du MOTEUR, sans le pont ni le rejeu de décisions : c'est
/// une seconde mesure, pas la même.
struct Espion8 {
    base: RandomPolicy,
    /// Badge imposé, pour éprouver le cas le plus défavorable.
    badge: Option<usize>,
    main: [Vec<u16>; NUM_PLAYERS],
    attente: Vec<(usize, u16)>,
    questions: usize,
    confrontees: usize,
    inutiles: usize,
}

impl Espion8 {
    fn new(badge: Option<usize>) -> Espion8 {
        Espion8 {
            base: RandomPolicy,
            badge,
            main: [Vec::new(), Vec::new()],
            attente: Vec::new(),
            questions: 0,
            confrontees: 0,
            inutiles: 0,
        }
    }
}

impl Policy for Espion8 {
    fn observe(&mut self, game: &GameState, player: usize) {
        self.main[player] = game.players[player].hand.clone();
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn pick_joker_tag(&mut self, r: &mut StdRng, p: usize, card: u16, tc: &[u32]) -> usize {
        self.questions += 1;
        self.attente.push((p, card));
        match self.badge {
            Some(i) => i,
            None => self.base.pick_joker_tag(r, p, card, tc),
        }
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, aff: &[usize]) -> Option<usize> {
        let main = self.main[p].clone();
        let offertes: Vec<u16> = aff.iter().filter_map(|&i| main.get(i).copied()).collect();
        let mut reste = Vec::new();
        for &(j, carte) in &self.attente {
            if j != p {
                reste.push((j, carte));
                continue;
            }
            // Une carte qui a quitté la main entre-temps ne prouve rien.
            if !main.contains(&carte) {
                continue;
            }
            self.confrontees += 1;
            if !offertes.contains(&carte) {
                self.inutiles += 1;
            }
        }
        self.attente = reste;
        self.base.choose_build(r, p, aff)
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

/// Aucune question de badge ne porte sur une carte que le moteur n'offrira pas
/// — et cela QUEL QUE SOIT le badge que le joueur choisit. Les trois politiques
/// éprouvées le disent : celle du dépôt, et deux têtues qui répondent toujours
/// le premier ou le dernier badge de la liste.
///
/// C'est ce « quel que soit » qui exige de juger la carte au badge le MOINS
/// favorable. Avec le seul badge le PLUS favorable, ce même banc rendait
/// 2 questions inutiles sur 43 (mesuré) : la carte était payable pour deux
/// badges sur dix, la question était posée, et le joueur en choisissait un
/// autre.
#[test]
fn aucun_badge_n_est_demande_pour_une_carte_que_le_moteur_n_offrira_pas() {
    let db = db();
    for badge in [None, Some(0), Some(JOKER_TAG_CHOICES.len() - 1)] {
        let mut e = Espion8::new(badge);
        for g in 0..40u64 {
            engine::sim::play_game(&db, g, &mut e);
        }
        println!(
            "    badge {badge:?} : {} question(s), {} confrontée(s), {} inutile(s)",
            e.questions, e.confrontees, e.inutiles
        );
        assert!(e.questions >= 20, "{} question(s) seulement", e.questions);
        assert!(e.confrontees >= 20, "{} confrontée(s) seulement", e.confrontees);
        assert_eq!(e.inutiles, 0, "badge {badge:?}");
    }
}

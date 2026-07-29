//! Tests du lot cartes-8 — **les poses supplémentaires**, les cinq dernières
//! cartes muettes de la boîte de base.
//!
//! Ce lot n'ajoute pas une valeur à un calcul existant : il ajoute deux
//! MÉCANISMES au déroulement d'une partie.
//!
//! 1. **La pose supplémentaire.** Une carte posée peut ouvrir le droit d'en
//!    poser une autre dans la même phase — droit lui-même réutilisable, puisque
//!    la carte posée grâce à lui peut en ouvrir un troisième. Décrit par
//!    `effects::BuildGrant`, exercé par le seul `flow::drain_pending_builds`.
//! 2. **Le modificateur de la prochaine carte.** Un effet à DURÉE : armé à la
//!    pose, consommé par la pose suivante du même joueur dans la même phase,
//!    mort à la fin de la phase même s'il n'a jamais servi.
//!
//! Chaque mécanisme est vérifié **dans les deux sens** : qu'il agit quand il
//! doit, et qu'il n'agit pas quand il ne doit pas (autre phase, autre joueur,
//! carte hors plafond, effets coupés).
//!
//! Le texte imprimé fait foi (`data/cartes-imprimees/textes-cartes.json`,
//! champ `text`), jamais le champ `description` de `cards.json`.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Color};
use engine::effects::{ActionCost, ActionEff, Action, BuildGrant};
use engine::flow::{build_card_with, play_round, setup_game, GRANT_CONSTRUCTION, GRANT_DEVELOPMENT};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_action_seq, run_probe_seq_corp, ProbeOptions, ProbeResult, ProbeScript};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::VecDeque;

const CARDS: &str = "../data/cards.json";

/// Les cinq cartes du périmètre.
const LOT8: [&str; 5] = [
    "Asset Liquidation",
    "Special Design",
    "Work Crews",
    "Automated Factories",
    "Tall Station",
];

/// Les trois qui ouvrent une pose bleue/rouge (phase II).
const TROIS_BR: [&str; 3] = ["Asset Liquidation", "Special Design", "Work Crews"];
/// Les deux qui offrent une verte à 9 MC ou moins (phase I).
const DEUX_VERTES: [&str; 2] = ["Automated Factories", "Tall Station"];

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

/// Sonde séquence, avec la garde obligatoire : ne jamais juger une valeur avant
/// d'avoir vérifié que la sonde a TROUVÉ la carte.
fn seq(db: &CardsDb, names: &[&str], o: ProbeOptions) -> ProbeResult {
    let r = run_probe_seq_corp(db, names, o, &ProbeScript::default(), false, None);
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    r
}

/// L'encodage d'une carte, par son nom.
fn spec(nom: &str) -> &'static engine::effects::CardEffects {
    engine::effects::lookup(nom).unwrap_or_else(|| panic!("« {nom} » doit être encodée"))
}

// =========================================================================
// La politique scriptée : elle dit OUI à des poses nommées, et NON au reste.
// =========================================================================

/// Politique qui pose les cartes nommées, dans l'ordre, dès qu'elles sont
/// offertes — et refuse tout le reste. C'est ce qui rend une pose
/// supplémentaire OBSERVABLE : sans script, on ne saurait pas distinguer
/// « le moteur n'a pas offert la 2e pose » de « la politique a renoncé ».
struct Poseur {
    base: RandomPolicy,
    /// Noms à poser, dans l'ordre.
    voulues: VecDeque<u16>,
    /// Ce qui a été réellement posé, dans l'ordre (identifiants de carte).
    posees: Vec<u16>,
    /// Nombre de fois où une pose a été OFFERTE (liste d'options non vide).
    offres: usize,
    phase: u8,
    choix: VecDeque<usize>,
}

impl Poseur {
    fn new(phase: u8) -> Poseur {
        Poseur {
            base: RandomPolicy,
            voulues: VecDeque::new(),
            posees: Vec::new(),
            offres: 0,
            phase,
            choix: VecDeque::new(),
        }
    }
}

impl Policy for Poseur {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) { self.phase } else { allowed[0] }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        // Volontairement inerte : les tests qui veulent poser passent par
        // `PoseurEnMain` ci-dessous, qui connaît la main.
        None
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.base.action_choice(r, p, o)
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
        self.base.discard_down(r, p, h, n)
    }
}

/// Partie réelle, mains vidées et bourses à zéro : rien n'arrive qu'on n'ait
/// mis là soi-même.
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

/// Pose une carte par le CHEMIN RÉEL, MC à volonté.
fn poser(g: &mut GameState, db: &CardsDb, nom: &str, pol: &mut dyn Policy) -> u16 {
    let id = en_main(g, db, nom);
    let idx = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(g, db, 0, idx, 0, pol);
    assert!(g.players[0].played.contains(&id), "« {nom} » n'est pas entrée en jeu");
    id
}

// =========================================================================
// 1. LA BRIQUE — `BuildGrant`, jugée sans passer par une partie
// =========================================================================

#[test]
fn une_permission_juge_la_couleur_et_le_prix_imprime() {
    let libre = BuildGrant { colors: &[Color::Blue, Color::Red], max_printed_cost: None, free: false };
    assert!(libre.admits(Color::Blue, 40), "aucun plafond : le prix ne compte pas");
    assert!(libre.admits(Color::Red, 0));
    assert!(!libre.admits(Color::Green, 1), "la couleur est un critère");

    let plafonnee = BuildGrant { colors: &[Color::Green], max_printed_cost: Some(9), free: true };
    assert!(plafonnee.admits(Color::Green, 9), "9 MC : « 9 MC or less » inclut 9");
    assert!(plafonnee.admits(Color::Green, 0));
    assert!(!plafonnee.admits(Color::Green, 10), "10 MC : hors plafond");
    assert!(!plafonnee.admits(Color::Blue, 3), "la couleur reste un critère");
}

#[test]
fn les_poses_ordinaires_sont_elles_memes_des_permissions() {
    // I1 : il n'existe qu'UNE façon de poser une carte dans le moteur. Les
    // phases I et II ne sont pas des chemins à part, ce sont des permissions.
    assert_eq!(GRANT_DEVELOPMENT.colors, &[Color::Green]);
    assert_eq!(GRANT_DEVELOPMENT.max_printed_cost, None);
    assert!(!GRANT_DEVELOPMENT.free, "la phase I fait payer");
    assert_eq!(GRANT_CONSTRUCTION.colors, &[Color::Blue, Color::Red]);
    assert_eq!(GRANT_CONSTRUCTION.max_printed_cost, None);
    assert!(!GRANT_CONSTRUCTION.free, "la phase II fait payer");
}

// =========================================================================
// 2. LES CINQ CARTES — leur encodage colle au texte imprimé
// =========================================================================

#[test]
fn les_cinq_sont_encodees_et_dans_la_boite_de_base() {
    let db = db();
    for nom in LOT8 {
        let id = db
            .resolve_card(nom)
            .unwrap_or_else(|| panic!("« {nom} » doit exister dans la boîte de base"));
        assert!(db.projects[id as usize].in_deck, "« {nom} » doit être dans la pioche");
        assert!(db.projects[id as usize].effect.is_some(), "« {nom} » doit être encodée");
        assert!(db.projects[id as usize].effets_geres(), "« {nom} » : aucun pouvoir sauté");
    }
}

#[test]
fn les_trois_cartes_de_phase_deux_ouvrent_une_pose_bleue_ou_rouge() {
    // « You may play an additional blue or red card this phase » — la MÊME
    // phrase sur les trois cartes, donc la MÊME permission, à l'identique.
    for nom in TROIS_BR {
        let g = spec(nom).grants;
        assert_eq!(g.len(), 1, "« {nom} » : une permission et une seule");
        assert_eq!(g[0].colors, &[Color::Blue, Color::Red], "« {nom} »");
        assert_eq!(g[0].max_printed_cost, None, "« {nom} » : le texte ne plafonne rien");
        assert!(!g[0].free, "« {nom} » : le texte n'offre pas la carte, il l'autorise");
    }
}

#[test]
fn les_deux_cartes_de_phase_un_offrent_une_verte_a_neuf_mc_ou_moins() {
    // « You may play a green card … that has a printed cost of 9 MC or less
    //   without paying its MC cost. »
    for nom in DEUX_VERTES {
        let g = spec(nom).grants;
        assert_eq!(g.len(), 1, "« {nom} » : une permission et une seule");
        assert_eq!(g[0].colors, &[Color::Green], "« {nom} »");
        assert_eq!(g[0].max_printed_cost, Some(9), "« {nom} » : plafond imprimé");
        assert!(g[0].free, "« {nom} » : « without paying its MC cost »");
    }
}

#[test]
fn seules_deux_cartes_arment_un_modificateur_et_ce_sont_les_bonnes() {
    // *Work Crews* : « You pay 11 MC less for the NEXT card you play this
    // phase. » *Special Design* : « For the NEXT card you play this phase, you
    // may consider the oxygen or temperature one color higher or lower. »
    let wc = spec("Work Crews").next_card.expect("Work Crews arme un modificateur");
    assert_eq!(wc.discount, 11, "11 MC de moins, le montant imprimé");
    assert!(!wc.color_flex, "Work Crews ne touche pas aux prérequis");

    let sd = spec("Special Design").next_card.expect("Special Design arme un modificateur");
    assert_eq!(sd.discount, 0, "Special Design ne touche pas au prix");
    assert!(sd.color_flex, "…mais bien aux paliers d'oxygène et de température");

    for nom in ["Asset Liquidation", "Automated Factories", "Tall Station"] {
        assert!(spec(nom).next_card.is_none(), "« {nom} » n'arme rien");
    }
}

#[test]
fn asset_liquidation_est_la_seule_a_porter_une_action() {
    // « Action: Spend 1 TR to draw three cards. » — premier coût en note de
    // terraformation du moteur.
    let a = spec("Asset Liquidation").action.expect("Asset Liquidation a une action");
    match a {
        Action::Fixed { cost, effect } => {
            assert_eq!(cost, &[ActionCost::Tr(1)], "1 NT, le coût imprimé");
            assert_eq!(effect, &[ActionEff::Draw(3)], "trois cartes, le gain imprimé");
        }
        other => panic!("action inattendue : {other:?}"),
    }
    for nom in ["Special Design", "Work Crews", "Automated Factories", "Tall Station"] {
        assert!(spec(nom).action.is_none(), "« {nom} » n'a pas d'action");
    }
}

#[test]
fn les_deux_productions_imprimees_sont_encodees() {
    // « During the production phase, draw a card » (Automated Factories) et
    // « this produces 3 MC » (Tall Station) : des productions FIXES.
    let db = db();
    let r = seq(&db, &["Automated Factories"], opts(400));
    assert_eq!(r.delta.card_prod, 1, "Automated Factories produit une carte");
    assert_eq!(r.delta.mc_prod, 0, "…et pas de MC");
    let r = seq(&db, &["Tall Station"], opts(400));
    assert_eq!(r.delta.mc_prod, 3, "Tall Station produit 3 MC");
    assert_eq!(r.delta.card_prod, 0, "…et pas de carte");
}

#[test]
fn les_cinq_prix_sont_ceux_du_carton() {
    let db = db();
    for (nom, prix) in [
        ("Asset Liquidation", 0),
        ("Special Design", 3),
        ("Work Crews", 5),
        ("Tall Station", 16),
        ("Automated Factories", 18),
    ] {
        assert_eq!(seq(&db, &[nom], opts(400)).paid, vec![prix], "« {nom} »");
    }
}

// =========================================================================
// 3. LE MÉCANISME — la pose supplémentaire, en partie réelle
// =========================================================================

/// Politique qui pose SYSTÉMATIQUEMENT la première carte offerte, et compte
/// combien de fois une pose lui a été proposée.
struct ToujoursPoser {
    base: RandomPolicy,
    offres: usize,
    poses: usize,
    phase: u8,
}

impl ToujoursPoser {
    fn new(phase: u8) -> ToujoursPoser {
        ToujoursPoser { base: RandomPolicy, offres: 0, poses: 0, phase }
    }
}

impl Policy for ToujoursPoser {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) { self.phase } else { allowed[0] }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, a: &[usize]) -> Option<usize> {
        if a.is_empty() {
            return None;
        }
        self.offres += 1;
        self.poses += 1;
        Some(a[0])
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        self.base.choose_option(r, p, n)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

#[test]
fn work_crews_ouvre_une_seconde_pose_dans_la_meme_phase() {
    // Le témoin le plus direct : main de DEUX cartes bleues/rouges, phase II,
    // politique qui pose tout ce qu'on lui offre. Sans permission, une seule
    // carte peut sortir ; avec, les deux sortent.
    let db = db();
    let mut pol = ToujoursPoser::new(2);
    let mut g = jeu(&db);
    // *Work Crews* d'abord dans la main (indice 0), la seconde derrière.
    let wc = en_main(&mut g, &db, "Work Crews");
    let mg = en_main(&mut g, &db, "Media Group");
    g.players[0].mc = 1000;
    g.players[1].hand.clear();

    play_round(&mut g, &db, &mut pol);

    // Le bonus du sélectionneur de phase peut faire PIOCHER : la main n'est
    // donc pas forcément vide à la fin. Ce qui compte est que les deux cartes
    // visées soient sorties, et elles ne le peuvent qu'à deux poses.
    assert!(g.players[0].played.contains(&wc), "Work Crews doit être posée");
    assert!(
        g.players[0].played.contains(&mg),
        "Media Group ne peut sortir que par la permission de Work Crews"
    );
    assert_eq!(g.extra_builds_granted, 1, "une permission accordée");
    assert_eq!(g.extra_builds_used, 1, "…et exercée");
}

#[test]
fn sans_la_carte_une_seule_pose_est_possible_dans_la_phase() {
    // Le MÊME test, la carte à permission en moins : c'est le contre-témoin.
    // Sans lui, on ne saurait pas si la 2e pose vient de la permission ou d'un
    // droit que la phase accordait déjà.
    let db = db();
    let mut pol = ToujoursPoser::new(2);
    let mut g = jeu(&db);
    en_main(&mut g, &db, "Media Group");
    en_main(&mut g, &db, "Business Contracts");
    g.players[0].mc = 1000;
    g.players[1].hand.clear();

    play_round(&mut g, &db, &mut pol);

    assert_eq!(g.extra_builds_granted, 0, "aucune permission dans cette main");
    assert!(
        g.players[0].played.len() <= 2,
        "au plus la pose ordinaire et le bonus du sélectionneur"
    );
}

#[test]
fn la_permission_meurt_avec_la_phase() {
    // « …this phase. » Une permission non exercée ne franchit pas la frontière.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    // Posée hors phase II par le chemin réel : elle accorde bien la permission…
    poser(&mut g, &db, "Work Crews", &mut pol);
    assert_eq!(
        g.players[0].pending_builds.len(),
        1,
        "la permission est bien accordée à la pose"
    );
    // …et la manche suivante l'efface, faute d'avoir pu être exercée.
    play_round(&mut g, &db, &mut pol);
    assert!(
        g.players[0].pending_builds.is_empty(),
        "la permission ne survit pas au changement de phase"
    );
}

#[test]
fn la_permission_n_est_jamais_imposee() {
    // « You MAY play an additional card » : une politique qui refuse tout ne
    // pose rien, et le moteur ne pose pas à sa place.
    let db = db();
    let mut pol = Poseur::new(2); // `choose_build` rend toujours None
    let mut g = jeu(&db);
    poser(&mut g, &db, "Work Crews", &mut pol);
    let avant = g.players[0].played.len();
    engine::flow::play_round(&mut g, &db, &mut pol);
    // La main peut GROSSIR (le bonus du sélectionneur de phase fait piocher) :
    // ce qui compte est qu'aucune carte n'en soit sortie de force.
    assert_eq!(g.players[0].played.len(), avant, "rien n'a été posé de force");
    assert_eq!(g.extra_builds_used, 0, "aucune permission exercée");
}

#[test]
fn la_permission_offerte_ne_coute_rien_et_respecte_le_plafond() {
    // *Automated Factories* / *Tall Station* : verte, 9 MC imprimés au plus,
    // gratuite. Le joueur est mis à ZÉRO MC : seule une carte OFFERTE peut
    // encore sortir.
    let db = db();
    let mut pol = ToujoursPoser::new(1);
    let mut g = jeu(&db);
    en_main(&mut g, &db, "Tall Station");
    // *Lichen* : verte, 7 MC imprimés — dans le plafond.
    let lichen = en_main(&mut g, &db, "Lichen");
    // Tall Station coûte 16 MC, moins la remise de 3 MC du sélectionneur de la
    // phase Développement (le joueur choisit la phase I) : 13 MC dus.
    g.players[0].mc = 13;
    g.players[1].hand.clear();

    play_round(&mut g, &db, &mut pol);

    assert!(
        g.players[0].played.contains(&lichen),
        "Lichen doit être posée par la permission offerte, sans un MC en poche"
    );
    assert_eq!(g.free_builds, 1, "une carte posée sans payer son prix");
    assert_eq!(
        g.players[0].mc, 0,
        "la bourse est vide : Lichen n'a donc rien coûté du tout"
    );
}

#[test]
fn la_permission_offerte_refuse_une_carte_trop_chere() {
    // « …9 MC or less » : une verte à 10 MC ou plus n'entre pas dans la
    // permission, même si le joueur pourrait la payer autrement.
    let db = db();
    let mut pol = ToujoursPoser::new(1);
    let mut g = jeu(&db);
    en_main(&mut g, &db, "Tall Station");
    // *Commercial District* : verte, 25 MC imprimés — hors plafond.
    let cd = en_main(&mut g, &db, "Commercial District");
    g.players[0].mc = 13;
    g.players[1].hand.clear();

    play_round(&mut g, &db, &mut pol);

    assert!(
        !g.players[0].played.contains(&cd),
        "une verte à 25 MC imprimés est hors de la permission plafonnée à 9"
    );
    assert_eq!(g.free_builds, 0, "aucune carte offerte");
}

#[test]
fn une_permission_ne_profite_jamais_a_l_adversaire() {
    // NEVER 9 : l'effet est celui du joueur qui a posé la carte.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    poser(&mut g, &db, "Work Crews", &mut pol);
    assert_eq!(g.players[0].pending_builds.len(), 1, "le poseur reçoit la permission");
    assert!(g.players[1].pending_builds.is_empty(), "l'adversaire n'en reçoit aucune");
}

// =========================================================================
// 4. LE MÉCANISME — le modificateur de la prochaine carte
// =========================================================================

#[test]
fn work_crews_reduit_de_onze_mc_la_carte_suivante_et_une_seule() {
    // Le témoin est un ÉCART entre deux séquences identiques, l'une précédée de
    // *Work Crews* et l'autre non. Comparer à un prix supposé serait fragile :
    // *Media Group* réduit elle-même les cartes à badge événement de 5 MC, et
    // ce genre d'interaction fausse tout témoin écrit à la main.
    let db = db();
    let sans = seq(&db, &["Media Group", "Business Contracts"], opts(400));
    let avec = seq(&db, &["Work Crews", "Media Group", "Business Contracts"], opts(400));

    assert_eq!(avec.paid[0], 5, "Work Crews à son prix imprimé");
    assert_eq!(sans.paid[0], 11, "Media Group seule : son prix imprimé");
    assert_eq!(
        avec.paid[1], 0,
        "la carte SUIVANTE : 11 − 11 = 0, la réduction s'applique en entier"
    );
    assert_eq!(
        avec.paid[2], sans.paid[1],
        "la TROISIÈME paie exactement ce qu'elle paierait sans Work Crews : \
         le modificateur ne vaut que pour UNE carte"
    );
}

#[test]
fn le_modificateur_est_consomme_meme_par_une_carte_bon_marche() {
    // « the next card you play » : c'est la carte suivante, pas la plus chère.
    // Une réduction gaspillée sur une carte à 0 MC est le comportement imprimé.
    let db = db();
    let r = seq(&db, &["Work Crews", "Asset Liquidation", "Media Group"], opts(400));
    assert_eq!(r.paid[1], 0, "Asset Liquidation coûte déjà 0 MC");
    assert_eq!(
        r.paid[2], 11,
        "Media Group paie plein tarif : la réduction a été consommée par la précédente"
    );
}

#[test]
fn special_design_assouplit_les_paliers_pour_la_carte_suivante() {
    // « you may consider the oxygen or temperature one color higher or lower »
    // — même souplesse qu'*Inventrix* et *Adaptation Technology*, mais armée
    // pour UNE carte. *Bushes* exige une température de palier rouge ; à l'état
    // de départ de la sonde elle est refusée, et la souplesse la fait passer.
    let db = db();
    let sans = seq(&db, &["Bushes"], opts(400));
    assert!(!sans.prereq_ok, "Bushes : prérequis non tenu à l'état de départ");
    let avec = seq(&db, &["Special Design", "Bushes"], opts(400));
    assert!(
        avec.prereq_ok,
        "avec Special Design juste avant, le palier bascule d'un cran"
    );
}

#[test]
fn la_souplesse_de_special_design_ne_touche_ni_les_oceans_ni_les_badges() {
    // I3 et le texte : la souplesse porte sur l'oxygène et la température, rien
    // d'autre. *Great Dam* exige des océans, *Fusion Power* des badges.
    let db = db();
    for nom in ["Great Dam", "Fusion Power"] {
        let avec = seq(&db, &["Special Design", nom], opts(400));
        assert!(
            !avec.prereq_ok,
            "« {nom} » : Special Design ne doit pas assouplir ce prérequis-là"
        );
    }
}

#[test]
fn la_souplesse_ne_vaut_que_pour_une_carte() {
    // Armée, consommée, morte. La DEUXIÈME carte après *Special Design* ne
    // profite plus de rien.
    let db = db();
    let deux = seq(&db, &["Special Design", "Media Group", "Bushes"], opts(400));
    assert!(
        !deux.prereq_ok,
        "la souplesse a été consommée par Media Group : Bushes redevient refusée"
    );
}

#[test]
fn le_modificateur_meurt_avec_la_phase() {
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    poser(&mut g, &db, "Work Crews", &mut pol);
    assert_eq!(g.players[0].next_card_mod.discount, 11, "armé à la pose");
    play_round(&mut g, &db, &mut pol);
    assert_eq!(
        g.players[0].next_card_mod.discount, 0,
        "le modificateur ne survit pas au changement de phase"
    );
}

#[test]
fn poser_une_seconde_carte_qui_arme_consomme_d_abord_la_premiere() {
    // Le point subtil du mécanisme, et il vaut d'être épinglé : *Special Design*
    // est elle-même « la prochaine carte que vous jouez ». La poser CONSOMME
    // donc la réduction armée par *Work Crews* — la carte en profite — puis
    // arme la sienne pour la suivante. C'est la lecture littérale du texte, et
    // c'est ce que le moteur fait.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    poser(&mut g, &db, "Work Crews", &mut pol);
    assert_eq!(g.players[0].next_card_mod.discount, 11, "armé à la pose");
    poser(&mut g, &db, "Special Design", &mut pol);
    assert_eq!(
        g.players[0].next_card_mod.discount, 0,
        "Special Design a consommé la réduction : elle était la carte suivante"
    );
    assert!(
        g.players[0].next_card_mod.color_flex,
        "…et elle arme la sienne pour la carte d'après"
    );
    // Les DEUX permissions de pose, elles, s'accumulent : elles ne sont pas
    // consommées par la pose qui les suit, mais par le drainage de la phase.
    assert_eq!(g.players[0].pending_builds.len(), 2, "deux permissions en attente");
}

// =========================================================================
// 5. L'ACTION D'ASSET LIQUIDATION — le premier coût en note de terraformation
// =========================================================================

#[test]
fn asset_liquidation_echange_un_point_de_note_contre_trois_cartes() {
    let db = db();
    let r = run_probe_action_seq(
        &db,
        &["Asset Liquidation"],
        &ProbeScript::default(),
        None,
        opts(400),
    );
    assert!(r.found, "sonde action : carte introuvable");
    assert!(r.action_applied, "l'action doit s'appliquer");
    assert_eq!(r.delta.tr, -1, "un point de note dépensé");
    assert_eq!(r.delta.hand, 3, "trois cartes piochées");
}

#[test]
fn l_action_est_refusee_sans_le_point_de_note() {
    // Un coût qui ne se paie pas ne s'applique pas — et ne prélève rien.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    let id = poser(&mut g, &db, "Asset Liquidation", &mut pol);
    g.players[0].tr = 0;
    let main_avant = g.players[0].hand.len();
    let applique = engine::flow::apply_blue_action(&mut g, &db, 0, id, &mut pol);
    assert!(!applique, "sans note de terraformation, l'action ne s'applique pas");
    assert_eq!(g.players[0].tr, 0, "rien n'a été prélevé");
    assert_eq!(g.players[0].hand.len(), main_avant, "rien n'a été pioché");
}

#[test]
fn le_cout_en_note_tient_la_comptabilite_du_moteur() {
    // Invariant du moteur : tr == 5 + gains − dépenses. Le nouveau coût passe
    // par le service unique `PlayerState::spend_tr`, donc il est compté.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = jeu(&db);
    let id = poser(&mut g, &db, "Asset Liquidation", &mut pol);
    let dec_avant = g.players[0].tr_decrements;
    assert!(engine::flow::apply_blue_action(&mut g, &db, 0, id, &mut pol));
    assert_eq!(
        g.players[0].tr_decrements,
        dec_avant + 1,
        "la dépense est inscrite au compteur d'audit"
    );
}

// =========================================================================
// 6. CE QUE LE LOT NE DOIT PAS CASSER
// =========================================================================

#[test]
fn effects_off_rend_les_cinq_completement_inertes() {
    // I7 : l'interrupteur coupe la couche d'effets, donc les permissions aussi.
    let db = db_off();
    for nom in LOT8 {
        let r = seq(&db, &[nom], opts(400));
        assert!(!r.in_lot, "« {nom} » : hors lot quand les effets sont coupés");
        assert_eq!(
            r.delta,
            engine::probe::ProbeDelta::default(),
            "« {nom} » change l'état alors que les effets sont coupés"
        );
    }
    let mut pol = RandomPolicy;
    let out = run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(out.extra_builds_granted, 0, "aucune permission sans les effets");
    assert_eq!(out.extra_builds_used, 0);
    assert_eq!(out.free_builds, 0);
    assert_eq!(out.next_card_mods_armed, 0);
    assert_eq!(out.next_card_mods_used, 0);
}

#[test]
fn les_compteurs_bougent_reellement_en_partie() {
    // Deux oracles disjoints : l'encodage ET mille parties.
    let db = db();
    let mut pol = RandomPolicy;
    let out = run_simulation(&db, 1000, 2024, &mut pol);
    assert!(out.extra_builds_granted > 0, "des permissions sont accordées");
    assert!(out.extra_builds_used > 0, "…et certaines sont exercées");
    assert!(
        out.extra_builds_used <= out.extra_builds_granted,
        "on n'exerce jamais plus de permissions qu'il n'en est accordé"
    );
    assert!(out.free_builds > 0, "des cartes sont posées sans payer leur prix");
    assert!(out.next_card_mods_armed > 0, "des modificateurs sont armés");
    assert!(
        out.next_card_mods_used <= out.next_card_mods_armed,
        "un modificateur consommé a forcément été armé"
    );
}

#[test]
fn mille_parties_restent_saines_dans_les_deux_boites() {
    for liste in ["base", "base,decouverte"] {
        let db = CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap())
            .unwrap_or_else(|e| panic!("chargement {liste} : {e}"));
        let mut pol = RandomPolicy;
        let out = run_simulation(&db, 1000, 2024, &mut pol);
        assert_eq!(out.completed, 1000, "{liste} : toutes les parties vont au bout");
        assert_eq!(out.invariant_violations, 0, "{liste} : aucun invariant violé");
        assert_eq!(out.truncated, 0, "{liste} : aucune partie tronquée");
    }
}

#[test]
fn la_boite_de_base_ne_saute_plus_un_seul_pouvoir() {
    // Le résultat du lot, mesuré en PARTIE RÉELLE et non sur le recensement.
    let db = db();
    let mut pol = RandomPolicy;
    let out = run_simulation(&db, 1000, 2024, &mut pol);
    assert_eq!(
        out.cards_effects_unhandled, 0,
        "boîte de base : plus un seul pouvoir imprimé n'est sauté"
    );
}

#[test]
fn le_moteur_reste_deterministe() {
    let db = db();
    let mut pol = RandomPolicy;
    let a = run_simulation(&db, 500, 2024, &mut pol);
    let mut pol = RandomPolicy;
    let b = run_simulation(&db, 500, 2024, &mut pol);
    assert_eq!(a.state_hash, b.state_hash, "deux exécutions, une seule empreinte");
}

#[test]
fn aucun_nom_de_carte_de_ce_lot_dans_le_flux_de_jeu() {
    // I6 : les noms vivent dans la table de données, nulle part ailleurs.
    for (fichier, src) in [
        ("flow.rs", include_str!("../src/flow.rs")),
        ("state.rs", include_str!("../src/state.rs")),
        ("policy.rs", include_str!("../src/policy.rs")),
    ] {
        // Les commentaires citent les cartes par leur nom : c'est de la
        // documentation, pas du flux. On ne cherche donc que dans le CODE.
        let code: String = src
            .lines()
            .map(|l| match l.trim_start().starts_with("//") {
                true => "",
                false => l,
            })
            .collect::<Vec<_>>()
            .join("\n");
        for nom in LOT8 {
            assert!(
                !code.contains(&format!("\"{nom}\"")),
                "« {nom} » est écrit dans le code de {fichier} : le flux de jeu ne \
                 doit jamais nommer une carte"
            );
        }
    }
}

#[test]
fn la_table_a_une_entree_par_carte_du_lot() {
    use engine::effects::LOT1;
    for nom in LOT8 {
        let n = LOT1.iter().filter(|(x, _)| *x == nom).count();
        assert_eq!(n, 1, "« {nom} » : une entrée et une seule");
    }
}

#[test]
fn aucune_carte_des_lots_precedents_n_a_gagne_une_permission() {
    // Non-régression par effet de bord : les deux champs neufs restent vides
    // partout ailleurs.
    for nom in [
        "Media Group", "Tardigrades", "Birds", "Io Mining Industries",
        "Volcanic Pools", "Lichen", "Comet", "Interplanetary Relations",
        "Advanced Alloys", "Solarpunk", "Olympus Conference", "Think Tank",
        "Interns", "Composting Factory", "Mars University", "Assembly Lines",
    ] {
        let e = spec(nom);
        assert!(e.grants.is_empty(), "« {nom} » ne doit accorder aucune permission");
        assert!(e.next_card.is_none(), "« {nom} » ne doit armer aucun modificateur");
    }
}

#[test]
fn le_cout_en_note_ne_touche_aucune_action_existante() {
    // Le nouveau genre de coût ne doit apparaître que sur la carte qui l'a
    // introduit. Vérifié sur la TABLE entière, pas sur une liste choisie.
    use engine::effects::LOT1;
    let porteuses: Vec<&str> = LOT1
        .iter()
        .filter(|(_, e)| match e.action {
            Some(Action::Fixed { cost, .. }) => {
                cost.iter().any(|c| matches!(c, ActionCost::Tr(_)))
            }
            _ => false,
        })
        .map(|(n, _)| *n)
        .collect();
    assert_eq!(porteuses, vec!["Asset Liquidation"], "une seule carte paie en note");
}

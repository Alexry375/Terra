//! Tests du chantier `jokers-corpos` — **les 3 derniers projets muets (badge
//! JOKER) et les 4 corporations de l'extension Découverte**.
//!
//! Discipline : chaque mécanisme est éprouvé **dans les deux sens** — il agit
//! quand il doit, il n'agit pas quand il ne doit pas. Les oracles sont disjoints
//! du code mesuré :
//!
//! - le texte imprimé vient de `inputs/refs/projets-decouverte.json` et
//!   `inputs/refs/corporations-discovery.json`, transcrits à l'image, jamais du
//!   champ `description` de `cards.json` (qui, pour Sultira, omettait « y
//!   compris celui-ci ») ;
//! - la sonde observe l'ÉTAT DE JEU produit par le chemin réel
//!   (`flow::build_card_with`, `flow::install_corporation_with`,
//!   `flow::apply_corp_action`) ;
//! - les compteurs d'audit sont relevés sur des PARTIES COMPLÈTES en politique
//!   aléatoire — un second oracle, indépendant de la sonde.
//!
//! Les sept cartes sont nommées ici, une par une, avec le texte de leur carton.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Tag, JOKER_TAG_CHOICES, TAG_COUNT};
use engine::effects::{Action, ResEff, CORPS};
use engine::flow::{
    build_card, corp_effects, discard_mc_rate, install_corporation_with, setup_game,
};
use engine::policy::{Policy, RandomPolicy};
use engine::probe::{
    run_probe_action_target, run_probe_seq_corp, ProbeActionResult, ProbeOptions, ProbeResult,
    ProbeScript,
};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

const CARDS: &str = "../data/cards.json";

/// Les trois projets à badge JOKER (D26, D39, D20).
const JOKERS: [&str; 3] = ["Local Market", "Political Influence", "Topographic Mapping"];
/// Les quatre corporations de Découverte (D01-D04).
const CORPOS: [&str; 4] = ["Apollo Industries", "Exocorp", "Hyperion Systems", "Sultira"];

fn db() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("chargement base,decouverte")
}

fn db_off() -> CardsDb {
    let mut d = db();
    d.effects_on = false;
    d
}

fn opts() -> ProbeOptions {
    ProbeOptions { mc: 400, ..ProbeOptions::default() }
}

fn script_joker(t: Option<Tag>) -> ProbeScript {
    ProbeScript { choices: Vec::new(), targets: Vec::new(), joker_tag: t }
}

/// Sonde de pose d'une séquence, badge joker imposé (ou non).
fn probe_seq(db: &CardsDb, names: &[&str], t: Option<Tag>) -> ProbeResult {
    run_probe_seq_corp(db, names, opts(), &script_joker(t), false, None)
}

/// Idem, avec un budget imposé — c'est ainsi que l'ABORDABILITÉ devient
/// mesurable (un budget large ne contraint jamais).
fn probe_budget(db: &CardsDb, names: &[&str], t: Option<Tag>, mc: i64) -> ProbeResult {
    let o = ProbeOptions { mc, ..opts() };
    run_probe_seq_corp(db, names, o, &script_joker(t), false, None)
}

/// Sonde de pose, corporation imposée.
fn probe_corp(db: &CardsDb, names: &[&str], corp: &str) -> ProbeResult {
    run_probe_seq_corp(db, names, opts(), &ProbeScript::default(), false, Some(corp))
}

/// Sonde de pose + PRODUCTION réelle (phase IV).
fn probe_prod(db: &CardsDb, names: &[&str], t: Option<Tag>) -> ProbeResult {
    run_probe_seq_corp(db, names, opts(), &script_joker(t), true, None)
}

/// Sonde d'ACTION visant la corporation installée, phase choisie imposée.
fn action_corp(db: &CardsDb, corp: &str, phase: u8) -> ProbeActionResult {
    let o = ProbeOptions { phase, ..opts() };
    run_probe_action_target(
        db,
        &["Lichen"],
        &ProbeScript::default(),
        Some(corp),
        o,
        Some(corp),
    )
}

fn prix_paye(r: &ProbeResult) -> i64 {
    assert!(r.found, "carte introuvable : {}", r.card);
    assert!(r.played, "carte non posée : {}", r.card);
    *r.paid.last().expect("aucun prix relevé")
}

// =========================================================================
// 1. LE BADGE JOKER — « Choisissez un badge et ajoutez-le à cette carte. »
// =========================================================================

/// Les trois cartes portent bien un badge joker dans les DONNÉES, et le badge
/// joker n'est pas un onzième badge : `TAG_COUNT` reste à 10 et `Tag::Dynamic`
/// reste hors décompte.
#[test]
fn les_trois_cartes_portent_un_badge_joker_qui_n_est_pas_un_onzieme_badge() {
    let db = db();
    for nom in JOKERS {
        let id = db.resolve_card(nom).unwrap_or_else(|| panic!("{nom}"));
        assert!(
            db.projects[id as usize].tags.iter().any(|t| t.is_joker()),
            "{nom} doit porter un badge joker"
        );
    }
    // L'AUTRE SENS : *Local Market* est la seule des trois à n'avoir QUE le
    // joker ; *Topographic Mapping* porte en plus un badge Événement imprimé.
    let tm = db.resolve_card("Topographic Mapping").expect("D20");
    assert!(db.projects[tm as usize].tags.contains(&Tag::Event));
    // Le joker n'est jamais compté comme un badge à part entière.
    assert_eq!(TAG_COUNT, 10);
    assert_eq!(Tag::Dynamic.index(), None);
    assert_eq!(JOKER_TAG_CHOICES.len(), TAG_COUNT);
    assert!(!JOKER_TAG_CHOICES.iter().any(|t| t.is_joker()));
}

/// Les dix badges sont choisissables et le badge retenu est celui demandé ;
/// « DYNAMIC », un nom inconnu et une chaîne vide sont refusés.
#[test]
fn les_dix_badges_sont_choisissables_et_dynamic_ne_l_est_pas() {
    for t in JOKER_TAG_CHOICES {
        assert_eq!(
            Tag::parse_joker_choice(t.as_str()),
            Some(t),
            "{} doit être un choix valide",
            t.as_str()
        );
    }
    // L'AUTRE SENS — sans cette moitié, un analyseur qui refuserait TOUT
    // passerait la première.
    for mauvais in ["DYNAMIC", "", "42", "PasUnBadge", "building"] {
        assert_eq!(
            Tag::parse_joker_choice(mauvais),
            None,
            "« {mauvais} » ne doit pas être un choix valide"
        );
    }
    // Et le badge imposé est honoré, pour les dix, sur *Local Market*.
    let db = db();
    for t in JOKER_TAG_CHOICES {
        let r = probe_seq(&db, &["Local Market"], Some(t));
        assert_eq!(r.joker_tag, Some(t.as_str()), "badge retenu pour Local Market");
    }
    // Contre-témoin : une carte SANS badge joker n'en retient aucun.
    let r = probe_seq(&db, &["Cartel"], Some(Tag::Earth));
    assert_eq!(r.joker_tag, None, "Cartel ne porte pas de badge joker");
}

/// Le badge choisi SATISFAIT un prérequis de badge — et seulement le bon.
/// *Beam from a Thorium Asteroid* requiert 1 badge Jupiter.
#[test]
fn le_badge_choisi_satisfait_un_prerequis_de_badge() {
    let db = db();
    let seul = probe_seq(&db, &["Beam from a Thorium Asteroid"], None);
    assert!(!seul.prereq_ok, "témoin : seul, le prérequis Jupiter n'est pas rempli");

    let avec = probe_seq(
        &db,
        &["Political Influence", "Beam from a Thorium Asteroid"],
        Some(Tag::Jupiter),
    );
    assert!(avec.prereq_ok, "joker déclaré JUPITER : le prérequis est rempli");

    // L'AUTRE SENS : un joker déclaré autrement ne remplit pas ce prérequis-là.
    let autre = probe_seq(
        &db,
        &["Political Influence", "Beam from a Thorium Asteroid"],
        Some(Tag::Plant),
    );
    assert!(
        !autre.prereq_ok,
        "joker déclaré PLANT : le prérequis Jupiter reste insatisfait"
    );
}

/// Le badge d'une carte ROUGE survit à sa pose — livret de base : une carte
/// rouge n'a plus d'effet après avoir été jouée, « autre que les badges qu'elle
/// fournit ». *Topographic Mapping* est un événement.
#[test]
fn le_badge_joker_d_une_carte_rouge_compte_apres_sa_pose() {
    let db = db();
    let id = db.resolve_card("Topographic Mapping").expect("D20");
    assert_eq!(
        db.projects[id as usize].color,
        engine::cards::Color::Red,
        "Topographic Mapping est une carte rouge"
    );
    let r = probe_seq(
        &db,
        &["Topographic Mapping", "Beam from a Thorium Asteroid"],
        Some(Tag::Jupiter),
    );
    assert!(r.prereq_ok, "le badge de l'événement compte encore après la pose");
    // L'AUTRE SENS : déclaré autrement, il ne remplit pas ce prérequis.
    let autre = probe_seq(
        &db,
        &["Topographic Mapping", "Beam from a Thorium Asteroid"],
        Some(Tag::Animal),
    );
    assert!(!autre.prereq_ok, "déclaré ANIMAL, le prérequis Jupiter tombe");
}

/// Le badge choisi est COMPTÉ par une production par badge (*Cartel* : 1 MC par
/// badge Terre, le sien compris), et deux cartes joker comptent pour DEUX
/// badges — le jeton est posé par carte, pas par joueur.
#[test]
fn le_badge_choisi_alimente_une_production_par_badge_et_se_compte_par_carte() {
    let db = db();
    let nu = probe_prod(&db, &["Cartel"], None);
    assert_eq!(nu.derived_prod.0, 1, "témoin : Cartel seule produit 1 MC");

    let un = probe_prod(&db, &["Local Market", "Cartel"], Some(Tag::Earth));
    assert_eq!(un.derived_prod.0, 2, "un joker Terre : 2 badges Terre");

    let deux = probe_prod(
        &db,
        &["Local Market", "Political Influence", "Cartel"],
        Some(Tag::Earth),
    );
    assert_eq!(deux.derived_prod.0, 3, "deux jokers Terre : 3 badges Terre");

    // L'AUTRE SENS : déclarés PLANT, les deux jokers ne comptent pas pour Terre.
    let plante = probe_prod(
        &db,
        &["Local Market", "Political Influence", "Cartel"],
        Some(Tag::Plant),
    );
    assert_eq!(plante.derived_prod.0, 1, "déclarés PLANT : un seul badge Terre");
}

/// Le badge choisi alimente les POINTS DE VICTOIRE par badge — l'axe que le
/// done-when nomme et qu'aucun contrôle ne couvre. *Io Mining Industries* vaut
/// 1 PV par badge Jupiter.
#[test]
fn le_badge_choisi_alimente_les_points_de_victoire_par_badge() {
    let db = db();
    let seule = probe_seq(&db, &["Io Mining Industries"], None);
    assert_eq!(seule.vp_total, 1, "témoin : son propre badge Jupiter, 1 PV");

    let jup = probe_seq(
        &db,
        &["Local Market", "Io Mining Industries"],
        Some(Tag::Jupiter),
    );
    assert_eq!(jup.vp_total, 2, "joker déclaré JUPITER : un badge de plus, 1 PV de plus");

    // L'AUTRE SENS : déclaré autrement, il ne rapporte rien sur cet axe.
    let plante = probe_seq(
        &db,
        &["Local Market", "Io Mining Industries"],
        Some(Tag::Plant),
    );
    assert_eq!(plante.vp_total, 1, "joker déclaré PLANT : aucun PV Jupiter de plus");
}

/// **L'exemple du livret** : « si vous choisissez le badge Espace, les
/// savoir-faire Titanium réduiront le coût en MC pour jouer LA CARTE ». Le badge
/// est donc arrêté avant le prix de sa PROPRE carte.
#[test]
fn le_badge_choisi_reduit_le_prix_de_sa_propre_carte() {
    let db = db();
    // Prix nus (témoins de référence).
    assert_eq!(prix_paye(&probe_seq(&db, &["Local Market"], None)), 7);
    assert_eq!(prix_paye(&probe_seq(&db, &["Political Influence"], None)), 10);

    // *Metallurgy* porte un savoir-faire titane : −3 MC sur les cartes espace.
    let espace = probe_seq(&db, &["Metallurgy", "Political Influence"], Some(Tag::Space));
    assert_eq!(prix_paye(&espace), 7, "10 − 3 : l'exemple du livret");
    // L'AUTRE SENS : déclarée bâtiment, la carte paie plein tarif.
    let batiment = probe_seq(
        &db,
        &["Metallurgy", "Political Influence"],
        Some(Tag::Building),
    );
    assert_eq!(prix_paye(&batiment), 10, "déclarée BUILDING : aucune réduction espace");

    // Et les savoir-faire se CUMULENT sur le badge déclaré : 7 − 2 − 2.
    let acier = probe_seq(
        &db,
        &["Blast Furnaces", "Hematite Mining", "Local Market"],
        Some(Tag::Building),
    );
    assert_eq!(prix_paye(&acier), 3, "deux aciers : 7 − 2 − 2");
}

/// L'ABORDABILITÉ voit la même réduction que le paiement (I2) : au budget
/// exact, la carte réduite passe ; un MC de moins, elle ne passe plus ; et
/// déclarée autrement, le même budget ne suffit plus.
#[test]
fn l_abordabilite_voit_le_badge_choisi_comme_le_paiement() {
    let db = db();
    // 14 (Blast Furnaces) + 5 (Local Market réduite) = 19.
    let juste = probe_budget(&db, &["Blast Furnaces", "Local Market"], Some(Tag::Building), 19);
    assert!(juste.played, "au budget exact, la carte réduite passe");
    assert_eq!(prix_paye(&juste), 5);

    let court = probe_budget(&db, &["Blast Furnaces", "Local Market"], Some(Tag::Building), 18);
    assert!(!court.played, "un MC de moins : la carte est refusée");

    // L'AUTRE SENS : déclarée PLANT, la carte coûte 7 et 19 MC ne suffisent plus.
    let plante = probe_budget(&db, &["Blast Furnaces", "Local Market"], Some(Tag::Plant), 19);
    assert!(
        !plante.played,
        "déclarée PLANT : aucune réduction, 19 MC ne suffisent pas"
    );
}

/// Le jeton est DÉFINITIF : une fois posé, il n'est jamais réécrit — et il n'est
/// jamais posé quand la couche d'effets est coupée.
#[test]
fn le_jeton_est_definitif_et_absent_quand_les_effets_sont_coupes() {
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 3, &mut pol);
    let id = db.resolve_card("Local Market").expect("D26");
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    engine::flow::ensure_joker_tag(&mut game, &db, 0, id, &mut pol);
    let premier = game.players[0].joker_tag(id).expect("un jeton est posé");
    let n = game.joker_tag_choices;
    // Un second appel ne rejoue rien : ni le badge, ni le compteur.
    engine::flow::ensure_joker_tag(&mut game, &db, 0, id, &mut pol);
    assert_eq!(game.players[0].joker_tag(id), Some(premier), "badge définitif");
    assert_eq!(game.joker_tag_choices, n, "aucun second choix compté");

    // L'AUTRE SENS : effets coupés, aucun jeton, aucun choix.
    let off = db_off();
    let mut game = setup_game(&off, 3, &mut pol);
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    engine::flow::ensure_joker_tag(&mut game, &off, 0, id, &mut pol);
    assert_eq!(game.players[0].joker_tag(id), None, "squelette : aucun jeton");
    assert_eq!(game.joker_tag_choices, 0);
}

/// L'heuristique par défaut de la politique est MOTIVÉE et vérifiable : elle
/// prend le badge que le joueur possède déjà le plus.
#[test]
fn la_politique_choisit_le_badge_deja_le_plus_possede() {
    let mut pol = RandomPolicy;
    let mut rng = StdRng::seed_from_u64(0);
    let mut counts = [0u32; TAG_COUNT];
    counts[Tag::Jupiter.index().unwrap()] = 3;
    counts[Tag::Plant.index().unwrap()] = 2;
    let i = pol.pick_joker_tag(&mut rng, 0, 0, &counts);
    assert_eq!(JOKER_TAG_CHOICES[i], Tag::Jupiter, "le badge le plus possédé");
    // L'AUTRE SENS : sans aucun badge, le choix est déterministe (premier de
    // l'énumération) — jamais un plantage, jamais un tirage caché.
    let vide = [0u32; TAG_COUNT];
    let j = pol.pick_joker_tag(&mut rng, 0, 0, &vide);
    assert_eq!(JOKER_TAG_CHOICES[j], Tag::Building);
}

/// Le RESTE du texte imprimé des trois cartes : deux productions de MC et une
/// amélioration de carte Phase.
#[test]
fn les_trois_cartes_joker_appliquent_le_reste_de_leur_texte_imprime() {
    let db = db();
    let lm = probe_seq(&db, &["Local Market"], Some(Tag::Earth));
    assert_eq!(lm.delta.mc_prod, 2, "Local Market : production de 2 MC");
    let pi = probe_seq(&db, &["Political Influence"], Some(Tag::Earth));
    assert_eq!(pi.delta.mc_prod, 3, "Political Influence : production de 3 MC");
    let tm = probe_seq(&db, &["Topographic Mapping"], Some(Tag::Earth));
    assert_eq!(tm.upgrades.len(), 1, "Topographic Mapping : une carte Phase améliorée");
    // L'AUTRE SENS : les deux vertes n'améliorent aucune carte Phase, et
    // l'événement ne produit rien.
    assert!(lm.upgrades.is_empty(), "Local Market n'améliore aucune carte Phase");
    assert!(pi.upgrades.is_empty(), "Political Influence n'améliore aucune carte Phase");
    assert_eq!(tm.delta.mc_prod, 0, "Topographic Mapping ne produit rien");
}

/// Politique de test qui impose le badge joker — et rien d'autre. Le choix
/// passe donc par le VRAI point de décision (`Policy::pick_joker_tag`), pas par
/// une écriture directe dans l'état : c'est le chemin que la partie emprunte.
struct PolitiqueJoker {
    base: RandomPolicy,
    badge: Tag,
}

impl Policy for PolitiqueJoker {
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
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
    fn pick_joker_tag(
        &mut self,
        _r: &mut StdRng,
        _p: usize,
        _c: u16,
        _counts: &[u32],
    ) -> usize {
        JOKER_TAG_CHOICES
            .iter()
            .position(|&t| t == self.badge)
            .expect("badge imposé parmi les dix")
    }
}

/// Le badge choisi compte pour les OBJECTIFS et les RÉCOMPENSES, qui lisent
/// `tag_counts` et `unique_tags` — l'unique passage par lequel un badge compte.
/// Objectif *Bâtisseur* (badges bâtiment), *Diversificateur* (badges distincts),
/// Récompense *Chercheur* (badges science).
#[test]
fn le_badge_choisi_compte_pour_les_objectifs_et_les_recompenses() {
    let db = db();
    let id = db.resolve_card("Local Market").expect("D26");
    let batiment = Tag::Building.index().unwrap();
    let science = Tag::Science.index().unwrap();

    // Deux parties de MÊME graine : le seul écart est le badge déclaré. Le choix
    // passe par la politique, la pose par `flow::build_card_with`.
    let mesure = |badge: Option<Tag>| -> (u32, u32, u32) {
        let mut fixe = PolitiqueJoker { base: RandomPolicy, badge: Tag::Building };
        let mut game = setup_game(&db, 31, &mut RandomPolicy);
        game.players[0].hand.clear();
        game.players[0].mc = 1000;
        if let Some(b) = badge {
            fixe.badge = b;
            game.players[0].hand.push(id);
            engine::flow::build_card_with(&mut game, &db, 0, 0, 0, &mut fixe);
        }
        let pl = &game.players[0];
        (pl.tag_counts[batiment], pl.tag_counts[science], pl.unique_tags())
    };
    // Contexte : la corporation seule, avant toute pose.
    let base = mesure(None);

    let (b, s, _) = mesure(Some(Tag::Building));
    assert_eq!(b, base.0 + 1, "joker BÂTIMENT : +1 badge pour l'Objectif Bâtisseur");
    assert_eq!(s, base.1, "et aucun badge science");

    let (b2, s2, _) = mesure(Some(Tag::Science));
    assert_eq!(b2, base.0, "joker SCIENCE : aucun badge bâtiment");
    assert_eq!(s2, base.1 + 1, "+1 badge pour la Récompense Chercheur");

    // *Diversificateur* compte les badges DISTINCTS : un badge inédit en ajoute
    // un, un badge déjà possédé non.
    let etat = setup_game(&db, 31, &mut RandomPolicy);
    let neuf = JOKER_TAG_CHOICES
        .iter()
        .copied()
        .find(|t| etat.players[0].tag_counts[t.index().unwrap()] == 0)
        .expect("au moins un badge manque au joueur");
    let (_, _, u_neuf) = mesure(Some(neuf));
    assert_eq!(u_neuf, base.2 + 1, "un badge inédit : +1 badge distinct");

    let deja = JOKER_TAG_CHOICES
        .iter()
        .copied()
        .find(|t| etat.players[0].tag_counts[t.index().unwrap()] > 0)
        .expect("le joueur possède au moins un badge (celui de sa planche)");
    let (_, _, u_deja) = mesure(Some(deja));
    assert_eq!(u_deja, base.2, "un badge déjà possédé : aucun badge distinct de plus");
}

// =========================================================================
// 2. LES QUATRE CORPORATIONS
// =========================================================================

/// Les quatre planches améliorent LEUR carte Phase à la mise en place — la
/// bonne, une seule — et les douze de base n'en améliorent aucune.
#[test]
fn les_quatre_corporations_ameliorent_leur_carte_phase_a_la_mise_en_place() {
    let db = db();
    for (nom, phase) in [
        ("Apollo Industries", 2u8),
        ("Exocorp", 5),
        ("Hyperion Systems", 3),
        ("Sultira", 1),
    ] {
        let r = probe_corp(&db, &["Lichen"], nom);
        let c = r.corp.as_ref().expect("objet corp");
        assert!(c.found, "{nom} introuvable");
        assert_eq!(c.upgrades, vec![phase], "{nom} améliore la carte Phase {phase}");
    }
    // L'AUTRE SENS : aucune corporation de la boîte de base n'améliore rien.
    for nom in [
        "Credicor",
        "Ecoline",
        "Helion Corporation",
        "Interplanetary Cinematics",
        "Inventrix",
        "Mining Guild",
        "Phobolog",
        "Saturn Systems",
        "Teractor Corporation",
        "Tharsis Republic",
        "Thorgate Corporation",
        "Unmi",
    ] {
        let r = probe_corp(&db, &["Lichen"], nom);
        let c = r.corp.as_ref().expect("objet corp");
        assert!(c.upgrades.is_empty(), "{nom} ne doit améliorer aucune carte Phase");
    }
}

/// **Apollo Industries** — « Lorsque vous jouez un badge [science], piochez une
/// carte. »
#[test]
fn apollo_industries_pioche_sur_un_badge_science_et_pas_sur_un_autre() {
    let db = db();
    // Témoin sans corporation : la main ne bouge pas.
    let nu = probe_seq(&db, &["Artificial Photosynthesis"], None);
    assert_eq!(nu.delta.hand, 0, "témoin : aucune pioche sans Apollo");
    let avec = probe_corp(&db, &["Artificial Photosynthesis"], "Apollo Industries");
    assert_eq!(avec.delta.hand, 1, "Apollo : +1 carte sur un badge science");
    // L'AUTRE SENS : une carte SANS badge science ne fait rien piocher.
    let sans = probe_corp(&db, &["Lichen"], "Apollo Industries");
    assert_eq!(sans.delta.hand, 0, "Apollo : aucune pioche sans badge science");
    // Contre-témoin croisé : une autre corporation ne pioche pas non plus.
    let credicor = probe_corp(&db, &["Artificial Photosynthesis"], "Credicor");
    assert_eq!(credicor.delta.hand, 0, "Credicor ne pioche pas sur un badge science");
}

/// **Apollo Industries** et le BADGE JOKER : un joker déclaré Science déclenche
/// la pioche, un joker déclaré autrement ne la déclenche pas.
#[test]
fn apollo_industries_pioche_sur_un_joker_declare_science() {
    let db = db();
    let sci = run_probe_seq_corp(
        &db,
        &["Local Market"],
        opts(),
        &script_joker(Some(Tag::Science)),
        false,
        Some("Apollo Industries"),
    );
    assert_eq!(sci.delta.hand, 1, "joker déclaré SCIENCE : Apollo pioche");
    let autre = run_probe_seq_corp(
        &db,
        &["Local Market"],
        opts(),
        &script_joker(Some(Tag::Animal)),
        false,
        Some("Apollo Industries"),
    );
    assert_eq!(autre.delta.hand, 0, "joker déclaré ANIMAL : Apollo ne pioche pas");
}

/// **Sultira** — « Chaque fois que vous jouez un badge [énergie], **y compris
/// celui-ci**, gagnez 2 chaleurs. » Le carton fait foi contre `cards.json`, qui
/// omettait la clause.
#[test]
fn sultira_donne_deux_chaleurs_par_badge_energie_y_compris_le_sien() {
    let db = db();
    // « Y compris celui-ci » : 2 chaleurs dès la mise en place.
    let mise = probe_corp(&db, &["Lichen"], "Sultira");
    assert_eq!(
        mise.corp.as_ref().expect("corp").start_heat,
        2,
        "le badge énergie de la planche déclenche l'effet"
    );
    // Contre-témoin : une corporation sans cette clause n'apporte aucune chaleur.
    let credicor = probe_corp(&db, &["Lichen"], "Credicor");
    assert_eq!(credicor.corp.as_ref().expect("corp").start_heat, 0);
    // Et une planche à « excluding this » (Saturn Systems, badge Jupiter) non plus.
    let saturn = probe_corp(&db, &["Lichen"], "Saturn Systems");
    assert_eq!(saturn.delta.tr, 0, "« excluding this » : son propre badge ne déclenche rien");

    // Une carte à badge énergie posée : +2 chaleurs.
    let energie = probe_corp(&db, &["Power Supply Consortium"], "Sultira");
    assert_eq!(energie.delta.heat, 2, "Sultira : 2 chaleurs par badge énergie");
    // L'AUTRE SENS : sans Sultira, rien ; et avec Sultira sur une carte sans
    // badge énergie, rien non plus.
    let nu = probe_seq(&db, &["Power Supply Consortium"], None);
    assert_eq!(nu.delta.heat, 0, "témoin : aucune chaleur sans Sultira");
    assert_eq!(mise.delta.heat, 0, "Sultira : aucune chaleur sur une carte sans énergie");
}

/// **Exocorp** — « Les cartes que vous défaussez pour gagner des MC vous
/// rapportent 1 MC supplémentaire. » Même service unique que *Composting
/// Factory*.
#[test]
fn exocorp_majore_le_taux_de_defausse_et_seulement_lui() {
    let db = db();
    let exo = probe_corp(&db, &["Lichen"], "Exocorp");
    assert_eq!(exo.corp.as_ref().expect("corp").discard_rate, 4, "3 + 1");
    // L'AUTRE SENS : une corporation de base laisse le taux du livret.
    let credicor = probe_corp(&db, &["Lichen"], "Credicor");
    assert_eq!(credicor.corp.as_ref().expect("corp").discard_rate, 3);
    // Et le service unique le confirme sur l'état réel, hors sonde.
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 1, &mut pol);
    let cid = db
        .corporations
        .iter()
        .position(|c| c.name == "Exocorp")
        .expect("Exocorp") as u16;
    game.players[0].corporation = None;
    game.players[0].tag_counts = [0; TAG_COUNT];
    let avant = discard_mc_rate(&db, &game.players[0]);
    assert_eq!(avant, 3, "témoin : le taux du livret");
    install_corporation_with(&mut game, &db, 0, cid, &mut pol);
    assert_eq!(discard_mc_rate(&db, &game.players[0]), 4, "Exocorp majore le taux");
    // L'AUTRE SENS : effets coupés, le taux reste celui du livret.
    let off = db_off();
    assert_eq!(discard_mc_rate(&off, &game.players[0]), 3, "squelette : 3 MC");
}

/// **Hyperion Systems** — « Action : gagnez 1 MC. *Si vous choisissez la phase
/// d'actions lors de cette manche, gagnez 1 MC supplémentaire. »
#[test]
fn hyperion_systems_donne_un_mc_et_deux_si_la_phase_action_est_choisie() {
    let db = db();
    let sans = action_corp(&db, "Hyperion Systems", 0);
    assert!(sans.found && sans.has_action, "la planche porte une action");
    assert!(sans.action_applied, "l'action est déclenchée");
    assert_eq!(sans.delta.mc, 1, "sans la phase Action : 1 MC");

    let avec = action_corp(&db, "Hyperion Systems", 3);
    assert_eq!(avec.delta.mc, 2, "phase Action choisie : 1 + 1 MC");

    // L'AUTRE SENS : une AUTRE phase choisie ne donne pas le supplément.
    let phase1 = action_corp(&db, "Hyperion Systems", 1);
    assert_eq!(phase1.delta.mc, 1, "phase I choisie : 1 MC seulement");

    // Contre-témoin croisé : une corporation de base ne porte aucune action.
    let credicor = probe_corp(&db, &["Lichen"], "Credicor");
    assert!(
        !credicor.corp.as_ref().expect("corp").has_action,
        "Credicor ne porte aucune action"
    );
    let r = action_corp(&db, "Credicor", 3);
    assert!(!r.has_action && !r.action_applied, "rien à activer chez Credicor");
}

/// Un nom inconnu reste REFUSÉ par `--probe-action`, et une corporation NON
/// installée ne porte aucune action activable.
#[test]
fn une_cible_d_action_inconnue_est_refusee() {
    let db = db();
    let r = run_probe_action_target(
        &db,
        &["Lichen"],
        &ProbeScript::default(),
        Some("Hyperion Systems"),
        opts(),
        Some("Corporation Qui N'Existe Pas"),
    );
    assert!(!r.found, "un nom inconnu doit être refusé");
    assert!(!r.action_applied);
    // Corporation connue mais NON installée : trouvée, sans action activable.
    let r = run_probe_action_target(
        &db,
        &["Lichen"],
        &ProbeScript::default(),
        Some("Credicor"),
        opts(),
        Some("Hyperion Systems"),
    );
    assert!(r.found, "le nom est connu de la pioche");
    assert!(!r.has_action, "Hyperion n'est pas la planche du joueur sondé");
    assert!(!r.action_applied);
}

// =========================================================================
// 3. STRUCTURE DE LA TABLE — ce qu'aucune sonde ne peut voir
// =========================================================================

/// La table des corporations décrit les SEIZE planches, et les effets de mise en
/// place des corporations n'emploient que les variantes exprimables sans carte
/// réceptacle : sans cette garde, un encodage serait silencieusement inerte.
#[test]
fn les_effets_de_mise_en_place_des_corporations_sont_tous_applicables() {
    assert_eq!(CORPS.len(), 16, "12 planches de base + 4 de Découverte");
    for (nom, spec) in CORPS {
        for e in spec.setup {
            assert!(
                matches!(e, ResEff::PhaseUpgrade(_) | ResEff::Gain(_)),
                "{nom} : effet de mise en place non applicable ({e:?})"
            );
        }
        // Une action de corporation ne peut pas être une action à RESSOURCES :
        // il n'y aurait pas de carte pour les recevoir.
        if let Some(a) = spec.action {
            assert!(
                !matches!(a, Action::Res(_)),
                "{nom} : une action à ressources exige une carte réceptacle"
            );
        }
    }
    // Les quatre planches de Découverte améliorent chacune UNE carte Phase, et
    // c'est celle du carton.
    for (nom, phase) in [
        ("Apollo Industries", 2u8),
        ("Exocorp", 5),
        ("Hyperion Systems", 3),
        ("Sultira", 1),
    ] {
        let spec = CORPS
            .iter()
            .find(|(n, _)| *n == nom)
            .map(|(_, s)| s)
            .unwrap_or_else(|| panic!("{nom} absente de la table"));
        assert_eq!(spec.setup.len(), 1, "{nom} : une seule amélioration");
        assert!(
            matches!(spec.setup[0], ResEff::PhaseUpgrade(Some(p)) if p == phase),
            "{nom} améliore la carte Phase {phase}"
        );
    }
}

/// **Le miroir, dans l'autre sens** : aucune entrée de `CORPS` n'est orpheline.
///
/// Le garde-fou de `CardsDb::load_boites` ne voit que les boîtes DEMANDÉES : un
/// nom déclaré dans la table mais imprimé sur aucune planche y résoudrait vers
/// 0 corporation et passerait inaperçu. Ce test charge les DEUX boîtes à la
/// fois — la totalité des planches existantes — et exige que chaque entrée y
/// résolve vers exactement une corporation. C'est la moitié que le garde-fou de
/// chargement ne peut pas porter (défaut trouvé en relecture adversariale).
#[test]
fn la_table_des_corporations_n_a_aucune_entree_orpheline() {
    let db = db(); // base + Découverte = toutes les planches imprimées
    assert_eq!(db.corporations.len(), CORPS.len(), "16 planches, 16 entrées");
    for (nom, _) in CORPS {
        let n = db.corporations.iter().filter(|c| c.name == *nom).count();
        assert_eq!(n, 1, "« {nom} » résolue {n} fois : entrée orpheline ou ambiguë");
    }
    // L'AUTRE SENS, déjà tenu par le chargement mais épinglé ici aussi : aucune
    // planche chargée n'échappe à la table.
    for c in &db.corporations {
        assert!(
            CORPS.iter().any(|(n, _)| *n == c.name),
            "{} chargée hors de la table d'effets",
            c.name
        );
    }
}

/// Les sept cartes sont recensées comme GÉRÉES, et le recensement ne compte plus
/// aucune muette dans les deux boîtes.
#[test]
fn les_sept_cartes_sont_recensees_comme_gerees() {
    let db = db();
    let r = db.recensement();
    for nom in JOKERS.iter().chain(CORPOS.iter()) {
        let c = r
            .iter()
            .find(|c| c.name == *nom)
            .unwrap_or_else(|| panic!("{nom} absente du recensement"));
        assert!(c.effets_geres, "{nom} doit être gérée");
    }
    let muettes: Vec<&str> = r
        .iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    assert!(muettes.is_empty(), "plus aucune carte muette : {muettes:?}");
    // Et chacune des quatre planches porte réellement un encodage.
    for nom in CORPOS {
        let c = db
            .corporations
            .iter()
            .find(|c| c.name == nom)
            .unwrap_or_else(|| panic!("{nom} absente de la pioche"));
        assert!(c.effect.is_some(), "{nom} doit être encodée");
    }
}

// =========================================================================
// 4. EN PARTIE RÉELLE — l'oracle disjoint de la sonde
// =========================================================================

/// Les cinq compteurs neufs bougent en partie réelle, et sont NULS quand la
/// couche d'effets est coupée. Quatre d'entre eux sont nuls en boîte de base :
/// les sept cartes appartiennent toutes à l'extension.
#[test]
fn les_cinq_compteurs_bougent_en_partie_reelle_et_sont_nuls_en_squelette() {
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 400, 4242, &mut pol);
    assert!(s.joker_tag_choices > 0, "des badges jokers sont choisis");
    assert!(s.joker_tag_hits > 0, "des badges jokers comptent réellement");
    assert!(
        s.joker_tag_hits <= s.joker_tag_choices,
        "un badge ne peut pas compter sans avoir été choisi"
    );
    assert!(s.corp_phase_upgrades_at_setup > 0, "des cartes Phase améliorées à la mise en place");
    assert!(s.discard_bonus_mc > 0, "le taux majoré est réellement payé");
    assert!(s.action_phase_self_bonus > 0, "le bonus étoilé tombe");
    assert_eq!(s.cards_effects_unhandled, 0, "plus un seul pouvoir imprimé sauté");

    // L'AUTRE SENS — squelette : aucun pouvoir appliqué, aucun compteur.
    let off = db_off();
    let mut pol = RandomPolicy;
    let s = run_simulation(&off, 200, 11, &mut pol);
    assert_eq!(s.joker_tag_choices, 0);
    assert_eq!(s.joker_tag_hits, 0);
    assert_eq!(s.corp_phase_upgrades_at_setup, 0);
    assert_eq!(s.discard_bonus_mc, 0);
    assert_eq!(s.action_phase_self_bonus, 0);

    // Boîte de BASE : les quatre compteurs propres à l'extension sont nuls ; le
    // taux majoré, lui, y existe déjà (*Composting Factory*) — c'est le
    // contre-témoin utile, un compteur nul partout ne prouverait rien.
    let base = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let mut pol = RandomPolicy;
    let s = run_simulation(&base, 400, 4242, &mut pol);
    assert_eq!(s.joker_tag_choices, 0);
    assert_eq!(s.joker_tag_hits, 0);
    assert_eq!(s.corp_phase_upgrades_at_setup, 0);
    assert_eq!(s.action_phase_self_bonus, 0);
    assert!(s.discard_bonus_mc > 0, "Composting Factory agit en boîte de base");
}

/// La boîte de base joue EXACTEMENT comme avant, l'extension a bel et bien
/// changé, et mille parties vont au bout sans casser d'invariant.
#[test]
fn la_boite_de_base_est_intacte_et_l_extension_a_change() {
    let base = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let mut pol = RandomPolicy;
    let s = run_simulation(&base, 1000, 2024, &mut pol);
    assert_eq!(
        format!("{:016x}", s.state_hash),
        "47030e306f1006cd",
        "empreinte de la boîte de base, REFIXÉE le 19-08 (le-secret-et-l-ordre) : le premier joueur est tiré au sort à la mise en place, la mise en place et la planification interrogent les joueurs dans l'ordre du tour, et la phase IV Production le suit elle aussi (D16) — les cartes du paquet commun ne tombent donc plus dans les mêmes mains. Les parties de référence enregistrées doivent être regénérées. Repères précédents : 8e4ec5b0296470e6 (05-08), bf70799ff3fee1d8 (04-08), 7dda3ea2e9b2901b (03-08), c1c52fcbe4e057b0 (01-08), d6a7267472501b13 (31-07)"
    );

    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 4242, &mut pol);
    assert_ne!(
        format!("{:016x}", s.state_hash),
        "b4af3de784992915",
        "les sept cartes doivent changer le cours des parties de l'extension"
    );
    assert_eq!(s.completed, 1000, "toutes les parties achevées");
    assert_eq!(s.invariant_violations, 0, "aucun invariant cassé");
}

/// Le déterminisme à graine fixe tient avec les mécanismes neufs : deux
/// exécutions identiques rendent la même empreinte ET les mêmes compteurs.
#[test]
fn le_determinisme_a_graine_fixe_tient_avec_les_jokers_et_les_corporations() {
    let db = db();
    let mut p1 = RandomPolicy;
    let a = run_simulation(&db, 200, 4242, &mut p1);
    let mut p2 = RandomPolicy;
    let b = run_simulation(&db, 200, 4242, &mut p2);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.joker_tag_choices, b.joker_tag_choices);
    assert_eq!(a.joker_tag_hits, b.joker_tag_hits);
    assert_eq!(a.corp_phase_upgrades_at_setup, b.corp_phase_upgrades_at_setup);
    assert_eq!(a.discard_bonus_mc, b.discard_bonus_mc);
    assert_eq!(a.action_phase_self_bonus, b.action_phase_self_bonus);
}

/// Le jeton du badge joker est posé AVANT la pose en partie réelle : à la fin
/// d'une partie, aucune carte joker EN JEU ne reste sans badge déterminé.
#[test]
fn aucune_carte_joker_posee_ne_reste_sans_badge() {
    let db = db();
    let ids: Vec<u16> = JOKERS
        .iter()
        .map(|n| db.resolve_card(n).unwrap_or_else(|| panic!("{n}")))
        .collect();
    let mut vues = 0usize;
    for graine in 0..40u64 {
        let mut pol = RandomPolicy;
        let mut game = setup_game(&db, graine, &mut pol);
        for _ in 0..12 {
            if game.game_over {
                break;
            }
            engine::flow::play_round(&mut game, &db, &mut pol);
        }
        for p in 0..NUM_PLAYERS {
            for &c in &game.players[p].played {
                if ids.contains(&c) {
                    vues += 1;
                    assert!(
                        game.players[p].joker_tag(c).is_some(),
                        "carte joker en jeu sans jeton (graine {graine})"
                    );
                }
            }
        }
    }
    assert!(vues > 0, "l'épreuve doit rencontrer des cartes joker posées");
}

/// Les pouvoirs de corporation passent par le service unique `corp_effects` :
/// coupés avec `--effects off`, comme tout le reste de la couche d'effets.
#[test]
fn les_pouvoirs_des_quatre_corporations_sont_coupes_en_squelette() {
    let db = db();
    let off = db_off();
    for nom in CORPOS {
        let cid = db
            .corporations
            .iter()
            .position(|c| c.name == nom)
            .unwrap_or_else(|| panic!("{nom}")) as u16;
        let mut pol = RandomPolicy;
        let mut game = setup_game(&off, 5, &mut pol);
        game.players[0].corporation = None;
        install_corporation_with(&mut game, &off, 0, cid, &mut pol);
        assert!(
            corp_effects(&off, &game.players[0]).is_none(),
            "{nom} : aucun pouvoir en squelette"
        );
        assert_eq!(
            game.players[0].phase_upgrades,
            [None; 5],
            "{nom} : aucune carte Phase améliorée en squelette"
        );
        assert_eq!(game.corp_phase_upgrades_at_setup, 0);
        // L'AUTRE SENS : effets actifs, la planche agit.
        let mut game = setup_game(&db, 5, &mut pol);
        game.players[0].corporation = None;
        game.players[0].phase_upgrades = [None; 5];
        install_corporation_with(&mut game, &db, 0, cid, &mut pol);
        assert!(
            game.players[0].phase_upgrades.iter().any(|u| u.is_some()),
            "{nom} : une carte Phase améliorée à la mise en place"
        );
    }
}

/// Une carte posée par le chemin réel voit son badge joker compté dans
/// `tag_counts`, et une seule fois.
#[test]
fn la_pose_compte_le_badge_choisi_une_fois_et_une_seule() {
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 21, &mut pol);
    let id = db.resolve_card("Political Influence").expect("D39");
    game.players[0].hand.clear();
    game.players[0].hand.push(id);
    game.players[0].mc = 1000;
    let avant = game.players[0].tag_counts;
    let hits = game.joker_tag_hits;
    build_card(&mut game, &db, 0, 0, 0);
    let apres = game.players[0].tag_counts;
    let bouges: Vec<usize> = (0..TAG_COUNT).filter(|&i| apres[i] != avant[i]).collect();
    assert_eq!(bouges.len(), 1, "un seul badge compté de plus");
    assert_eq!(
        apres[bouges[0]] - avant[bouges[0]],
        1,
        "et il n'est compté qu'une fois"
    );
    assert_eq!(game.joker_tag_hits, hits + 1, "un badge joker a compté");
    // L'AUTRE SENS : une carte ORDINAIRE ne fait pas monter le compteur.
    let cartel = db.resolve_card("Cartel").expect("Cartel");
    game.players[0].hand.clear();
    game.players[0].hand.push(cartel);
    game.players[0].mc = 1000;
    let hits = game.joker_tag_hits;
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.joker_tag_hits, hits, "Cartel n'a pas de badge joker");
}

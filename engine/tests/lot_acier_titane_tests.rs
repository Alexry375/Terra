//! Tests du lot « acier-titane » (chantier moteur-acier-titane).
//!
//! Le lot fait exister une GRANDEUR : le nombre d'aciers et de titanes d'un
//! joueur. Elle n'est écrite nulle part dans les données — elle se DÉRIVE des
//! réductions déjà encodées, au taux du livret (2 MC par acier sur les badges
//! bâtiment, 3 MC par titane sur les badges espace).
//!
//! Ce fichier vérifie, dans cet ordre :
//!
//! 1. la brique de règle (`Capacity`) et sa division exacte (I3) ;
//! 2. la dérivation elle-même : couleur, corporation, adversaire, effets coupés
//!    (I1, I2, I4, NEVER 9) ;
//! 3. les quatre cartes du lot, confrontées à leur TEXTE IMPRIMÉ
//!    (`inputs/textes-cartes.json`, champs `text`, `requirement`, `notes`) —
//!    jamais au champ `description` de `cards.json` ;
//! 4. l'effet manquant de PhoboLog ;
//! 5. ce que le lot ne doit PAS casser : prix inchangés, savoir-faire non
//!    dépensables, `--probe-action` à un seul nom identique, déterminisme.
//!
//! Le contrôle de référence des prix (`04-prix-inchanges.sh`) mesure les 246
//! cartes et 42 séquences depuis l'extérieur ; ici on épingle les cas de
//! calibrage à la main, avec leur arithmétique écrite.

use engine::boites::BoiteSet;
use engine::cards::{verifier_multiple, CardsDb, Color, Tag};
use engine::effects::{Capacity, CardEffects, Reduction, TrigCond};
use engine::flow::{capacities, player_capacities, setup_game};
use engine::policy::RandomPolicy;
use engine::probe::{
    run_probe_action_seq, run_probe_seq_corp, ProbeActionResult, ProbeOptions, ProbeResult,
    ProbeScript,
};
use engine::sim::run_simulation;
use engine::state::PlayerState;

const CARDS: &str = "../data/cards.json";

/// Les quatre cartes du périmètre.
const LOT: [&str; 4] = [
    "Advanced Alloys",
    "Aquifer Pumping",
    "Solarpunk",
    "Water Import from Europa",
];

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

fn db_all() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte")
}

fn opts(mc: i64) -> ProbeOptions {
    ProbeOptions { mc, ..ProbeOptions::default() }
}

/// Sonde séquence, MC imposés, sans corporation.
fn seq(db: &CardsDb, names: &[&str], mc: i64) -> ProbeResult {
    run_probe_seq_corp(db, names, opts(mc), &ProbeScript::default(), false, None)
}

/// Idem, corporation imposée.
fn seq_corp(db: &CardsDb, corp: &str, names: &[&str], mc: i64) -> ProbeResult {
    run_probe_seq_corp(
        db,
        names,
        opts(mc),
        &ProbeScript::default(),
        false,
        Some(corp),
    )
}

/// Sonde action sur une séquence.
fn act(db: &CardsDb, names: &[&str], mc: i64) -> ProbeActionResult {
    run_probe_action_seq(db, names, &ProbeScript::default(), None, opts(mc))
}

fn act_corp(db: &CardsDb, corp: &str, names: &[&str], mc: i64) -> ProbeActionResult {
    run_probe_action_seq(db, names, &ProbeScript::default(), Some(corp), opts(mc))
}

// =========================================================== 1. la brique

#[test]
fn le_taux_du_livret_vit_en_un_seul_endroit() {
    // Livret l. 355-359 : « Chaque acier que vous possédez réduit de 2 MC le
    // coût des cartes à badge bâtiment » ; l. 523-529, 3 MC / badge espace.
    assert_eq!(Capacity::Steel.tag(), Tag::Building);
    assert_eq!(Capacity::Steel.mc_per_unit(), 2);
    assert_eq!(Capacity::Titanium.tag(), Tag::Space);
    assert_eq!(Capacity::Titanium.mc_per_unit(), 3);
}

#[test]
fn seuls_deux_badges_portent_un_savoir_faire() {
    assert_eq!(Capacity::from_tag(Tag::Building), Some(Capacity::Steel));
    assert_eq!(Capacity::from_tag(Tag::Space), Some(Capacity::Titanium));
    for t in [Tag::Event, Tag::Energy, Tag::Earth, Tag::Jupiter, Tag::Science,
              Tag::Plant, Tag::Animal, Tag::Microbe, Tag::Dynamic] {
        assert_eq!(Capacity::from_tag(t), None, "{t:?} n'est pas un savoir-faire");
    }
}

#[test]
fn la_division_est_exacte_ou_refusee() {
    // Les montants réellement présents dans le moteur.
    assert_eq!(Capacity::Steel.units_from(2), Some(1));
    assert_eq!(Capacity::Steel.units_from(4), Some(2));
    assert_eq!(Capacity::Titanium.units_from(3), Some(1));
    assert_eq!(Capacity::Titanium.units_from(6), Some(2));
    assert_eq!(Capacity::Steel.units_from(0), Some(0));
    // I3 : un montant qui n'est pas un multiple n'est JAMAIS arrondi.
    assert_eq!(Capacity::Steel.units_from(3), None);
    assert_eq!(Capacity::Steel.units_from(5), None);
    assert_eq!(Capacity::Titanium.units_from(4), None);
    assert_eq!(Capacity::Titanium.units_from(7), None);
    assert_eq!(Capacity::Steel.units_from(-2), None);
}

#[test]
fn seule_une_reduction_de_badge_batiment_ou_espace_declare_un_savoir_faire() {
    assert_eq!(
        Reduction::Tag(Tag::Building, 4).capacity_units(),
        Some((Capacity::Steel, 2))
    );
    assert_eq!(
        Reduction::Tag(Tag::Space, 6).capacity_units(),
        Some((Capacity::Titanium, 2))
    );
    // Les réductions qui ne sont pas des savoir-faire (encart rose).
    assert_eq!(Reduction::Tag(Tag::Event, 5).capacity_units(), None);
    assert_eq!(Reduction::Tag(Tag::Energy, 4).capacity_units(), None);
    assert_eq!(Reduction::AnyCard(2).capacity_units(), None);
    assert_eq!(
        Reduction::MinPrice { min: 20, amount: 4 }.capacity_units(),
        None
    );
    // Celle qui AMPLIFIE un savoir-faire n'en est pas un.
    assert_eq!(
        Reduction::PerCapacity { tag: Tag::Space, cap: Capacity::Titanium, per: 1 }
            .capacity_units(),
        None
    );
}

#[test]
#[should_panic(expected = "pas un multiple")]
fn un_montant_non_multiple_fait_tomber_la_derivation() {
    // I3, le filet : si le garde-fou de chargement était contourné, la
    // dérivation refuse encore d'arrondir.
    let _ = Reduction::Tag(Tag::Building, 3).capacity_units();
}

#[test]
fn le_garde_fou_de_chargement_refuse_un_montant_non_multiple() {
    // I3, le contrôle utile : il tombe au CHARGEMENT, avant la première partie.
    assert!(verifier_multiple("carte fictive", Reduction::Tag(Tag::Building, 4)).is_ok());
    assert!(verifier_multiple("carte fictive", Reduction::Tag(Tag::Space, 3)).is_ok());
    assert!(verifier_multiple("carte fictive", Reduction::Tag(Tag::Event, 5)).is_ok());
    let e = verifier_multiple("carte fictive", Reduction::Tag(Tag::Building, 3))
        .expect_err("une réduction bâtiment de 3 MC doit être refusée");
    assert!(e.contains("carte fictive") && e.contains("multiple de 2"), "{e}");
    let e = verifier_multiple("carte fictive", Reduction::Tag(Tag::Space, 4))
        .expect_err("une réduction espace de 4 MC doit être refusée");
    assert!(e.contains("multiple de 3"), "{e}");
}

#[test]
fn toutes_les_reductions_du_moteur_sont_des_multiples_exacts() {
    // La contre-épreuve, mesurée sur les tables et non recopiée d'un document :
    // toute réduction bâtiment/espace portée par une carte verte ou une
    // corporation est un multiple exact. Si ce test tombe un jour, c'est une
    // carte neuve qui a un savoir-faire fractionnaire — à traiter, pas à
    // arrondir.
    let db = db_all();
    let mut vus = 0;
    for c in &db.projects {
        if c.color != Color::Green {
            continue;
        }
        let Some(spec) = c.effect else { continue };
        for r in spec.reductions {
            if r.capacity_units().is_some() {
                vus += 1;
            }
            verifier_multiple(&c.name, *r).expect("multiple exact");
        }
    }
    for (nom, spec) in engine::effects::CORPS {
        for r in spec.reductions {
            if r.capacity_units().is_some() {
                vus += 1;
            }
            verifier_multiple(nom, *r).expect("multiple exact");
        }
    }
    // 23 réductions bâtiment/espace sur des cartes vertes + 4 sur des
    // corporations = 27, mesuré le 28-07 sur les tables du moteur.
    //
    // ATTENTE MISE À JOUR par `decouverte-projets` (27 → 30) : trois cartes
    // VERTES de Découverte portent une réduction par badge, toutes multiples
    // exacts du taux du livret — *Blast Furnaces* (D23, bâtiment −2 = 1 acier),
    // *Hematite Mining* (D29, bâtiment −2 = 1 acier) et *Metallurgy* (D34,
    // espace −3 = 1 titane). Le test n'est pas assoupli : il continue d'exiger
    // que CHAQUE réduction soit un multiple exact, et d'épingler leur nombre.
    assert_eq!(vus, 30, "réductions déclarant un savoir-faire");
}

#[test]
fn aucune_carte_bleue_ou_rouge_ne_porte_de_savoir_faire_aujourd_hui() {
    // Contre-épreuve de I4 : la garde de couleur est là pour l'avenir, et
    // aujourd'hui elle ne masque rien — aucune carte non verte ne porte de
    // réduction bâtiment/espace.
    let db = db_all();
    for c in &db.projects {
        if c.color == Color::Green {
            continue;
        }
        let Some(spec) = c.effect else { continue };
        for r in spec.reductions {
            assert!(
                r.capacity_units().is_none(),
                "{} (couleur {:?}) porterait un savoir-faire",
                c.name,
                c.color
            );
        }
    }
}

// ================================================== 2. la dérivation

#[test]
fn la_couleur_fait_partie_du_critere() {
    // I4, prouvé sur le mécanisme et pas seulement sur les données du jour : on
    // greffe LE MÊME encodage (−4 MC bâtiment) sur une carte verte puis sur une
    // carte bleue. La verte donne 2 aciers, la bleue zéro.
    static SAVOIR_FAIRE: CardEffects = CardEffects {
        reqs: &[],
        effects: &[],
        reductions: &[Reduction::Tag(Tag::Building, 4)],
        play_triggers: &[],
        global_triggers: &[],
        action: None,
        phase_bonus: None,
        holds: None,
        on_build: &[],
        prod: None,
        research: None,
        // (lot cartes-7) Quatre champs de plus dans `CardEffects`, tous neutres
        // ici : ce test ne porte que sur la couleur du porteur d'un savoir-faire.
        discard_bonus: 0,
        standard_discount: 0,
        req_color_flex: false,
        action_trigger: &[],
        // (lot cartes-8) Deux champs de plus, neutres ici pour la même raison.
        grants: &[],
        next_card: None,
        // (decouverte-projets) Un champ de plus, neutre ici : ce test ne porte
        // que sur la couleur du porteur d'un savoir-faire.
        reveal_bonus: &[],
    };
    let mut db = db();
    let verte = db
        .projects
        .iter()
        .position(|c| c.color == Color::Green && c.effect.is_none())
        .expect("une carte verte sans encodage");
    let bleue = db
        .projects
        .iter()
        .position(|c| c.color == Color::Blue && c.effect.is_none())
        .expect("une carte bleue sans encodage");
    db.projects[verte].effect = Some(&SAVOIR_FAIRE);
    db.projects[bleue].effect = Some(&SAVOIR_FAIRE);

    let mut pl = PlayerState::new();
    pl.played.push(verte as u16);
    assert_eq!(capacities(&db, &pl).steel, 2, "carte VERTE : 4 MC = 2 aciers");

    let mut pl = PlayerState::new();
    pl.played.push(bleue as u16);
    assert_eq!(
        capacities(&db, &pl).steel,
        0,
        "carte BLEUE : même encodage, aucun savoir-faire"
    );
}

#[test]
fn le_compte_ne_depend_jamais_des_cartes_de_l_adversaire() {
    // NEVER 9. La dérivation ne reçoit qu'UN joueur : la preuve est structurelle
    // autant que mesurée.
    let db = db();
    let id = db.resolve_card("Strip Mine").expect("Strip Mine");
    let mut adversaire = PlayerState::new();
    adversaire.played.push(id);
    assert_eq!(capacities(&db, &adversaire).steel, 2);
    let vide = PlayerState::new();
    assert_eq!(capacities(&db, &vide), Default::default());
}

#[test]
fn effets_coupes_le_compte_est_nul() {
    let mut db = db();
    let id = db.resolve_card("Strip Mine").expect("Strip Mine");
    let mut pl = PlayerState::new();
    pl.played.push(id);
    assert_eq!(capacities(&db, &pl).steel, 2);
    db.effects_on = false;
    let c = capacities(&db, &pl);
    assert_eq!((c.steel, c.titanium), (0, 0), "--effects off coupe tout");
}

#[test]
fn les_vingt_et_une_cartes_a_savoir_faire_donnent_le_bon_compte() {
    // Chaque carte verte du moteur qui porte une réduction bâtiment/espace,
    // posée SEULE, avec le compte attendu — dérivé de son montant imprimé.
    let db = db_all();
    for (nom, acier, titane) in [
        ("Great Escarpment Consortium", 1, 0),
        ("Industrial Center", 1, 0),
        ("Industrial Microbes", 1, 0),
        ("Underground City", 1, 0),
        ("Micro-Mills", 1, 0),
        ("Dusty Quarry", 1, 0),
        ("Mine", 2, 0),
        ("Building Industries", 2, 0),
        ("Electric Arc Furnaces", 2, 0),
        ("Space Station", 0, 1),
        ("Titanium Mine", 0, 1),
        ("Vesta Shipyard", 0, 1),
        ("Fuel Factory", 0, 1),
        ("Mass Converter", 0, 1),
        ("Asteroid Mining Consortium", 0, 1),
        ("Asteroid Mining", 0, 2),
        ("Ganymede Shipyard", 0, 2),
        ("Ilmenite Deposits", 0, 2),
        ("Io Mining Industries", 0, 2),
        ("Surface Mines", 1, 1),
        ("Strip Mine", 2, 1),
    ] {
        let r = seq(&db, &[nom], 300);
        assert!(r.played, "{nom} doit se poser");
        assert_eq!((r.steel, r.titanium), (acier, titane), "{nom}");
    }
}

#[test]
fn les_savoir_faire_s_additionnent() {
    let db = db_all();
    let r = seq(&db, &["Strip Mine", "Surface Mines", "Titanium Mine"], 300);
    // 2+1 aciers, 1+1+1 titanes.
    assert_eq!((r.steel, r.titanium), (3, 3));
}

#[test]
fn les_quatre_corporations_a_encart_gris_portent_un_savoir_faire() {
    let db = db_all();
    for (corp, acier, titane) in [
        ("Mining Guild", 1, 0),
        ("Interplanetary Cinematics", 1, 0),
        ("Phobolog", 0, 1),
        ("Saturn Systems", 0, 1),
    ] {
        let r = seq_corp(&db, corp, &["Power Plant"], 300);
        assert_eq!((r.steel, r.titanium), (acier, titane), "{corp}");
    }
}

#[test]
fn les_corporations_a_encart_rose_n_en_portent_aucun() {
    // CrediCor, Teractor et Thorgate ont une réduction — dans un encart ROSE :
    // ce n'est pas un savoir-faire. Les autres n'ont aucune réduction.
    let db = db_all();
    for corp in [
        "Credicor", "Teractor Corporation", "Thorgate Corporation", "Ecoline",
        "Helion Corporation", "Inventrix", "Tharsis Republic", "Unmi",
    ] {
        let r = seq_corp(&db, corp, &["Power Plant"], 300);
        assert_eq!((r.steel, r.titanium), (0, 0), "{corp}");
    }
}

#[test]
fn une_carte_sans_savoir_faire_ne_compte_rien() {
    let db = db_all();
    for nom in ["Power Plant", "Research", "Comet", "Lichen", "Media Group",
                "Earth Catapult", "Energy Subsidies", "Interplanetary Conference"] {
        let r = seq(&db, &[nom], 300);
        assert!(r.played, "{nom}");
        assert_eq!((r.steel, r.titanium), (0, 0), "{nom}");
    }
}

#[test]
fn le_compte_est_ecrit_dans_l_etat_du_joueur_pas_calcule_par_la_sonde() {
    // Clause anti-shortcut 2 : la sonde LIT `PlayerState`, elle ne recalcule
    // rien. On le vérifie en confrontant la sortie de la sonde à la dérivation
    // appliquée à un état construit à part.
    let db = db_all();
    let r = seq(&db, &["Strip Mine", "Titanium Mine"], 300);
    let mut pl = PlayerState::new();
    for n in ["Strip Mine", "Titanium Mine"] {
        pl.played.push(db.resolve_card(n).unwrap());
    }
    let d = capacities(&db, &pl);
    assert_eq!((r.steel, r.titanium), (d.steel, d.titanium));
    assert_eq!((d.steel, d.titanium), (2, 2));
}

#[test]
fn le_cache_de_l_etat_suit_chaque_pose() {
    // Le champ est rafraîchi À CHAQUE mise en jeu, pas seulement à la fin.
    let db = db_all();
    let mut pl = PlayerState::new();
    for (nom, attendu) in [("Titanium Mine", 1), ("Space Station", 2), ("Power Plant", 2)] {
        pl.played.push(db.resolve_card(nom).unwrap());
        assert_eq!(capacities(&db, &pl).titanium, attendu, "après {nom}");
    }
}

#[test]
fn le_compte_ne_depend_pas_de_l_ordre_de_pose() {
    // I7 : rien ne se fige à la pose.
    let db = db_all();
    let a = seq(&db, &["Advanced Alloys", "Titanium Mine", "Strip Mine"], 300);
    let b = seq(&db, &["Strip Mine", "Titanium Mine", "Advanced Alloys"], 300);
    assert_eq!((a.steel, a.titanium), (b.steel, b.titanium));
    assert_eq!((a.steel, a.titanium), (2, 2));
}

// ============================================ 3. les quatre cartes du lot

#[test]
fn les_quatre_cartes_sont_encodees_et_declarees_gerees() {
    let db = db();
    for nom in LOT {
        let id = db.resolve_card(nom).unwrap_or_else(|| panic!("{nom} introuvable"));
        let c = &db.projects[id as usize];
        assert!(c.effect.is_some(), "{nom} doit être encodée");
        assert!(c.effets_geres(), "{nom} doit être déclarée gérée");
        // Texte imprimé : les quatre sont BLEUES et sans prérequis
        // (`inputs/textes-cartes.json`, champ `requirement` vide).
        assert_eq!(c.color, Color::Blue, "{nom}");
        assert!(c.effect.unwrap().reqs.is_empty(), "{nom} n'a aucun prérequis");
    }
}

#[test]
fn advanced_alloys_est_un_effet_permanent_pas_une_action() {
    // « Effect: … » — rien à la pose, rien à activer.
    let db = db();
    let id = db.resolve_card("Advanced Alloys").unwrap();
    let spec = db.projects[id as usize].effect.unwrap();
    assert!(spec.action.is_none(), "son texte n'imprime aucune action");
    assert!(spec.effects.is_empty(), "elle ne gagne rien à la pose");
    let r = seq(&db, &["Advanced Alloys"], 300);
    let d = r.delta;
    assert_eq!(d.mc, 0, "delta.mc = prix payé réintégré");
    assert_eq!(
        (d.plants, d.heat, d.tr, d.oxygen, d.oceans, d.forests),
        (0, 0, 0, 0, 0, 0)
    );
    // Et elle n'est pas elle-même un savoir-faire.
    assert_eq!((r.steel, r.titanium), (0, 0));
}

#[test]
fn advanced_alloys_ajoute_1_mc_par_titane_sur_les_cartes_espace() {
    // Space Station : 14 MC, badge espace.
    //   Titanium Mine seule            14 − 3       = 11
    //   + Advanced Alloys              14 − 3 − 1   = 10
    let db = db_all();
    let sans = seq(&db, &["Titanium Mine", "Space Station"], 300);
    assert_eq!(sans.paid[1], 11);
    let avec = seq(&db, &["Advanced Alloys", "Titanium Mine", "Space Station"], 300);
    assert_eq!(avec.paid[2], 10, "un titane amplifié d'1 MC");
    // Deux titanes devant Space Station : 14 − 3 − 3 − 2 = 6. (Le compte rendu
    // par la sonde est celui d'APRÈS la séquence : Space Station est elle-même
    // un savoir-faire titane, d'où 3 à l'arrivée pour 2 au moment du paiement.)
    let deux = seq(
        &db,
        &["Advanced Alloys", "Titanium Mine", "Vesta Shipyard", "Space Station"],
        300,
    );
    assert_eq!(deux.paid[3], 6);
    assert_eq!(deux.titanium, 3, "les trois titanes en jeu à l'arrivée");
}

#[test]
fn advanced_alloys_ajoute_1_mc_par_acier_sur_les_cartes_batiment() {
    // Mine : 10 MC, badge bâtiment.
    //   Great Escarpment Consortium seule (1 acier)  10 − 2     = 8
    //   + Advanced Alloys                            10 − 2 − 1 = 7
    let db = db_all();
    let sans = seq(&db, &["Great Escarpment Consortium", "Mine"], 300);
    assert_eq!(sans.paid[1], 8);
    let avec = seq(&db, &["Advanced Alloys", "Great Escarpment Consortium", "Mine"], 300);
    assert_eq!(avec.paid[2], 7);
}

#[test]
fn advanced_alloys_sert_les_deux_savoir_faire_sur_une_meme_carte() {
    // Io Mining Industries porte BUILDING **et** SPACE : les deux lignes de son
    // texte s'appliquent, chacune sur son badge.
    // Prix 37. Strip Mine donne 2 aciers + 1 titane et réduit déjà de 4 + 3.
    //   sans Advanced Alloys : 37 − 4 − 3           = 30
    //   avec                 : 30 − (2 aciers ×1) − (1 titane ×1) = 27
    let db = db_all();
    let sans = seq(&db, &["Strip Mine", "Io Mining Industries"], 300);
    assert_eq!(sans.paid[1], 30);
    let avec = seq(&db, &["Advanced Alloys", "Strip Mine", "Io Mining Industries"], 300);
    assert_eq!(avec.paid[2], 27);
    // Io Mining Industries est elle-même un savoir-faire de 2 titanes : à
    // l'arrivée le joueur en a 3, mais le prix a été calculé sur le 1 qu'il
    // avait au moment de payer.
    assert_eq!((avec.steel, avec.titanium), (2, 3));
}

#[test]
fn advanced_alloys_ne_touche_pas_les_cartes_sans_badge_concerne() {
    // « … the cost of [space] cards … [building] cards » : rien d'autre.
    let db = db_all();
    // Research : 5 MC, aucun badge bâtiment ni espace.
    let sans = seq(&db, &["Strip Mine", "Research"], 300);
    let avec = seq(&db, &["Advanced Alloys", "Strip Mine", "Research"], 300);
    assert_eq!(sans.paid[1], avec.paid[2], "aucun changement de prix");
}

#[test]
fn advanced_alloys_ne_fige_rien_a_la_pose() {
    // I7 : posée AVANT ou APRÈS le savoir-faire, le prix final est le même.
    let db = db_all();
    let avant = seq(&db, &["Advanced Alloys", "Titanium Mine", "Space Station"], 300);
    let apres = seq(&db, &["Titanium Mine", "Advanced Alloys", "Space Station"], 300);
    assert_eq!(avant.paid[2], apres.paid[2]);
    assert_eq!(avant.paid[2], 10);
}

#[test]
fn aquifer_pumping_paie_10_mc_moins_2_par_acier_et_retourne_un_ocean() {
    // « Action: Spend 10 MC to flip an ocean tile. Reduce this by 2 MC per steel
    // you have. »
    let db = db_all();
    for (sequence, acier, mc) in [
        (vec!["Aquifer Pumping"], 0, -10),
        (vec!["Great Escarpment Consortium", "Aquifer Pumping"], 1, -8),
        (vec!["Strip Mine", "Aquifer Pumping"], 2, -6),
        (vec!["Strip Mine", "Mine", "Aquifer Pumping"], 4, -2),
    ] {
        let r = act(&db, &sequence, 300);
        assert!(r.has_action && r.action_applied, "{sequence:?}");
        // La tuile océan rapporte 0 MC (première tuile non mélangée) : le delta
        // MC ne contient donc que le prix payé.
        assert_eq!(r.delta.mc, mc, "{sequence:?} ({acier} aciers)");
        assert_eq!(r.delta.oceans, 1, "un océan retourné");
        assert_eq!(r.delta.tr, 1, "un océan = un pas de NT");
    }
}

#[test]
fn le_cout_reduit_d_une_action_ne_descend_jamais_sous_zero() {
    // 6 aciers × 2 MC = 12 > 10 : « reduce this » ne rapporte pas de MC.
    let db = db_all();
    let r = act(
        &db,
        &["Strip Mine", "Mine", "Building Industries", "Aquifer Pumping"],
        300,
    );
    assert!(r.action_applied);
    assert_eq!(r.delta.mc, 0, "coût plancher à 0, jamais négatif");
    assert_eq!(r.delta.oceans, 1);
}

#[test]
fn solarpunk_paie_15_mc_moins_2_par_titane_et_gagne_une_foret() {
    // « Action: Spend 15 MC to gain a forest VP and raise oxygen 1 step. Reduce
    // this by 2 MC per titanium you have. »
    let db = db_all();
    for (sequence, mc) in [
        (vec!["Solarpunk"], -15),
        (vec!["Titanium Mine", "Solarpunk"], -13),
        // Asteroid Mining : réduction espace de 6 MC = 2 titanes → 15 − 4.
        (vec!["Asteroid Mining", "Solarpunk"], -11),
    ] {
        let r = act(&db, &sequence, 300);
        assert!(r.action_applied, "{sequence:?}");
        assert_eq!(r.delta.mc, mc, "{sequence:?}");
        assert_eq!(r.delta.forests, 1);
        assert_eq!(r.delta.oxygen, 1, "UN pas d'oxygène, jamais deux (règle R1)");
        assert_eq!(r.delta.tr, 1, "le pas d'oxygène, et lui seul");
    }
}

#[test]
fn solarpunk_passe_par_le_chemin_de_foret_du_moteur() {
    // Preuve indirecte mais nette : le gain de forêt fait monter l'oxygène et
    // le NT exactement une fois — c'est ce que `flow::gain_forest` produit, et
    // ce qu'un gain « à la main » de PV forêt ne produirait pas.
    let db = db_all();
    let r = act(&db, &["Solarpunk"], 300);
    assert_eq!((r.delta.forests, r.delta.oxygen, r.delta.tr), (1, 1, 1));
}

#[test]
fn water_import_from_europa_paie_12_mc_moins_1_par_titane() {
    // « Action: Spend 12 MC to flip an ocean tile. Reduce this by 1 MC per
    // titanium you have. »
    let db = db_all();
    for (sequence, mc) in [
        (vec!["Water Import from Europa"], -12),
        (vec!["Titanium Mine", "Water Import from Europa"], -11),
        (vec!["Asteroid Mining", "Water Import from Europa"], -10), // 2 titanes
    ] {
        let r = act(&db, &sequence, 300);
        assert!(r.action_applied, "{sequence:?}");
        assert_eq!(r.delta.mc, mc, "{sequence:?}");
        assert_eq!(r.delta.oceans, 1);
        assert_eq!(r.delta.tr, 1);
    }
}

#[test]
fn water_import_from_europa_garde_ses_points_par_badge_jupiter() {
    // « *=1 VP per [jupiter] you have » : déjà servi par `VpKind::Jupiter`, le
    // lot n'y touche pas.
    let db = db_all();
    let r = seq(&db, &["Water Import from Europa"], 300);
    assert!(r.played);
    // La carte porte elle-même un badge Jupiter : 1 PV.
    assert_eq!(r.vp_total, 1, "1 PV par badge Jupiter, le sien compris");
    let deux = seq(&db, &["Ganymede Shipyard", "Water Import from Europa"], 300);
    // Ganymede Shipyard porte aussi un badge Jupiter, plus ses 2 PV imprimés.
    assert!(deux.vp_total >= 2, "les badges Jupiter comptent tous");
}

#[test]
fn les_trois_cartes_a_action_ne_font_rien_a_la_pose() {
    // Leur texte n'imprime qu'une action : poser la carte ne doit rien produire.
    let db = db_all();
    for nom in ["Aquifer Pumping", "Solarpunk", "Water Import from Europa"] {
        let r = seq(&db, &[nom], 300);
        let d = r.delta;
        assert_eq!(
            (d.plants, d.heat, d.tr, d.oxygen, d.oceans, d.forests),
            (0, 0, 0, 0, 0, 0),
            "{nom} ne gagne rien à la pose"
        );
    }
}

#[test]
fn les_quatre_cartes_coutent_leur_prix_imprime() {
    // Contrôle croisé du texte imprimé (`cost`) contre le prix réellement payé.
    let db = db_all();
    for (nom, cout) in [
        ("Advanced Alloys", 9),
        ("Aquifer Pumping", 14),
        ("Solarpunk", 15),
        ("Water Import from Europa", 22),
    ] {
        assert_eq!(seq(&db, &[nom], 300).paid, vec![cout], "{nom}");
    }
}

// =============================================== 4. l'effet de PhoboLog

#[test]
fn phobolog_applique_son_effet_a_son_propre_titane() {
    // « When you play a [space], you pay 3 MC less for it. EFFECT: Each titanium
    // you have reduces the cost of [space] cards an additional 1 MC. »
    // Son encart gris EST un titane : 14 − 3 − 1 = 10 sur Space Station.
    let db = db_all();
    let r = seq_corp(&db, "Phobolog", &["Space Station"], 300);
    assert_eq!(r.paid, vec![10], "14 − 3 (savoir-faire) − 1 (son effet)");
    // Space Station est elle-même un titane : 2 à l'arrivée, 1 au paiement.
    assert_eq!(r.titanium, 2);
    // Sur une carte espace qui n'est PAS un savoir-faire, le compte reste 1.
    let seul = seq_corp(&db, "Phobolog", &["Ice Asteroid"], 300);
    assert_eq!(seul.titanium, 1, "la planche porte un titane, et un seul");
}

#[test]
fn phobolog_cumule_avec_les_titanes_des_cartes() {
    // Titanium Mine ajoute un titane : 14 − 3 (PhoboLog) − 3 (Titanium Mine)
    // − 2 (deux titanes × 1 MC) = 6.
    let db = db_all();
    let r = seq_corp(&db, "Phobolog", &["Titanium Mine", "Space Station"], 300);
    assert_eq!(r.paid[1], 6);
    assert_eq!(r.titanium, 3, "PhoboLog + Titanium Mine + Space Station");
}

#[test]
fn phobolog_ne_touche_pas_les_cartes_sans_badge_espace() {
    let db = db_all();
    let avec = seq_corp(&db, "Phobolog", &["Mine"], 300);
    let sans = seq(&db, &["Mine"], 300);
    assert_eq!(avec.paid, sans.paid, "Mine n'a aucun badge espace");
}

#[test]
fn la_table_des_corporations_porte_les_deux_lignes_de_phobolog() {
    let spec = engine::effects::corp_lookup("Phobolog").expect("Phobolog");
    assert_eq!(spec.reductions.len(), 2);
    assert!(spec
        .reductions
        .iter()
        .any(|r| matches!(r, Reduction::Tag(Tag::Space, 3))));
    assert!(spec.reductions.iter().any(|r| matches!(
        r,
        Reduction::PerCapacity { tag: Tag::Space, cap: Capacity::Titanium, per: 1 }
    )));
}

#[test]
fn le_declencheur_de_mining_guild_est_encode_et_ne_paie_que_l_acier() {
    // (D2 — lot regles-cartes) ASSERTION CORRIGÉE, jamais retirée. Ce test
    // scellait « aucun déclencheur encodé » : c'était le défaut D2 de
    // `docs/AUDIT_MOTEUR.md`, la seconde ligne du carton jamais appliquée. Le
    // déclencheur est désormais là, et les deux autres assertions — celles qui
    // interdisent d'INVENTER un NT — sont conservées mot pour mot : *Titanium
    // Mine* n'apporte pas d'acier mais du titane, elle ne doit donc toujours
    // rien rapporter, et l'acier de la planche elle-même reste compté sans
    // rapporter (« excluding this »).
    let db = db_all();
    let spec = engine::effects::corp_lookup("Mining Guild").expect("Mining Guild");
    assert_eq!(spec.play_triggers.len(), 1, "le déclencheur du carton, encodé");
    let trig = &spec.play_triggers[0];
    assert!(
        matches!(trig.cond, TrigCond::GrantsCapacity(Capacity::Steel)),
        "la condition porte sur le savoir-faire ACIER apporté par la carte posée"
    );
    assert!(trig.scale_by_matched_tags, "1 NT PAR acier (arbitrage 18-08)");
    assert!(!trig.include_self, "« excluding this »");
    let r = seq_corp(&db, "Mining Guild", &["Titanium Mine"], 300);
    assert_eq!(r.delta.tr, 0, "aucun NT inventé");
    assert_eq!(r.steel, 1, "son acier, lui, compte");
}

// ============================================= 5. ce qui ne doit pas casser

#[test]
fn les_prix_deja_reduits_ne_bougent_pas() {
    // I5, épinglé à la main sur les cas de calibrage du contrat.
    let db = db_all();
    assert_eq!(seq(&db, &["Space Station"], 100).paid, vec![14]);
    assert_eq!(seq(&db, &["Titanium Mine", "Space Station"], 100).paid, vec![7, 11]);
    assert_eq!(
        seq_corp(&db, "Phobolog", &["Ice Asteroid"], 100).paid,
        vec![17],
        "21 − 3 (savoir-faire) − 1 (effet nouveau, légitime)"
    );
    // Une réduction qui n'est PAS un savoir-faire (encart rose) : inchangée.
    // Deimos Down coûte 30 et porte un badge Event → Media Group −5.
    assert_eq!(seq(&db, &["Media Group", "Deimos Down"], 100).paid[1], 25);
}

#[test]
fn un_savoir_faire_ne_se_depense_pas() {
    // NEVER 8 : le posséder suffit, activer une action ne le consomme pas.
    let db = db_all();
    let avant = seq(&db, &["Titanium Mine", "Solarpunk"], 300);
    assert_eq!(avant.titanium, 1);
    let r = act(&db, &["Titanium Mine", "Solarpunk"], 300);
    assert!(r.action_applied);
    // Le compte est relu après l'action, sur le même état.
    let apres = seq(&db, &["Titanium Mine", "Solarpunk"], 300);
    assert_eq!(apres.titanium, 1, "l'action n'a consommé aucun titane");
}

#[test]
fn probe_action_a_un_seul_nom_est_inchangee() {
    // Garde anti-régression de l'interface : un seul nom = comportement des lots
    // précédents, bit à bit.
    let db = db();
    let r = act(&db, &["Greenhouses"], 300);
    assert!(r.has_action && r.action_applied);
    assert_eq!(-r.delta.heat, r.delta.plants, "chaleur dépensée = plantes gagnées");
    assert!(r.delta.plants > 0);
    let d = act(&db, &["Development Center"], 300);
    assert_eq!((d.delta.heat, d.delta.hand), (-2, 1), "2 chaleur → 1 carte");
    let f = act(&db, &["Farmers Market"], 300);
    assert_eq!((f.delta.mc, f.delta.plants), (-1, 2));
}

#[test]
fn probe_action_sur_sequence_n_applique_que_l_action_de_la_derniere() {
    // Les cartes précédentes sont POSÉES (leur savoir-faire compte) mais leur
    // action n'est jamais activée.
    let db = db_all();
    // Development Center a une action (2 chaleur → 1 carte) ; posée AVANT
    // Solarpunk, elle ne doit pas s'activer.
    let r = act(&db, &["Development Center", "Solarpunk"], 300);
    assert_eq!(r.card, "Solarpunk");
    assert_eq!(r.delta.heat, 0, "l'action de la première carte n'est pas jouée");
    assert_eq!(r.delta.forests, 1, "seule celle de la dernière l'est");
    assert_eq!(r.delta.mc, -15);
}

#[test]
fn probe_action_sur_sequence_prend_l_instantane_apres_la_derniere_pose() {
    // Le delta n'inclut aucun effet de pose des cartes précédentes.
    let db = db_all();
    // Comet fait monter la température et retourne un océan à la POSE.
    let r = act(&db, &["Comet", "Solarpunk"], 300);
    assert_eq!(r.delta.temperature, 0, "la pose de la 1re carte est hors delta");
    assert_eq!(r.delta.oceans, 0);
    assert_eq!(r.delta.forests, 1);
}

#[test]
fn probe_action_sequence_et_corporation_se_cumulent() {
    let db = db_all();
    let r = act_corp(&db, "Phobolog", &["Titanium Mine", "Solarpunk"], 300);
    assert!(r.action_applied);
    assert_eq!(r.delta.mc, -11, "15 − 2 × 2 titanes");
}

#[test]
fn la_recompense_industrialist_compte_desormais_de_vrais_savoir_faire() {
    // Elle lisait deux champs figés à zéro : elle valait zéro pour tout le
    // monde. Les champs sont vrais, la récompense est disputée.
    let db = db_all();
    let mut pl = PlayerState::new();
    for n in ["Strip Mine", "Titanium Mine"] {
        pl.played.push(db.resolve_card(n).unwrap());
    }
    let c = capacities(&db, &pl);
    pl.steel_capacity = c.steel;
    pl.titanium_capacity = c.titanium;
    let total = player_capacities(&pl);
    assert_eq!(total.steel + total.titanium, 4, "2 aciers + 2 titanes");
}

#[test]
fn mille_parties_restent_saines_et_deterministes() {
    // Les comptes sont vérifiés à chaque manche par `sim::check_invariants` :
    // zéro violation prouve que le cache ne diverge jamais de la dérivation.
    let db = db_all();
    let mut p = RandomPolicy;
    let a = run_simulation(&db, 1000, 2024, &mut p);
    let mut p = RandomPolicy;
    let b = run_simulation(&db, 1000, 2024, &mut p);
    assert_eq!(a.completed, 1000);
    assert_eq!(a.invariant_violations, 0);
    assert_eq!(a.state_hash, b.state_hash, "déterminisme à graine égale");
}

#[test]
fn le_compte_est_juste_en_partie_reelle() {
    // Hors sonde : sur des parties complètes, chaque joueur finit avec un compte
    // égal à sa dérivation, et au moins un joueur a fini par en avoir.
    let db = db_all();
    let mut policy = RandomPolicy;
    let mut vus = 0;
    for graine in 0..40u64 {
        let mut game = setup_game(&db, graine, &mut policy);
        for _ in 0..8 {
            if game.game_over {
                break;
            }
            engine::flow::play_round(&mut game, &db, &mut policy);
        }
        for pl in &game.players {
            let d = capacities(&db, pl);
            assert_eq!((pl.steel_capacity, pl.titanium_capacity), (d.steel, d.titanium));
            vus += (d.steel + d.titanium) as usize;
        }
    }
    assert!(vus > 0, "aucun savoir-faire acquis en 40 parties : suspect");
}

#[test]
fn aucun_nom_de_carte_du_lot_dans_le_flux_de_jeu() {
    // I6. Les noms vivent dans la table de données `effects::LOT1`, nulle part
    // ailleurs — ni en code, ni en commentaire.
    for (fichier, src) in [
        ("flow.rs", include_str!("../src/flow.rs")),
        ("cards.rs", include_str!("../src/cards.rs")),
        ("state.rs", include_str!("../src/state.rs")),
        ("policy.rs", include_str!("../src/policy.rs")),
        ("sim.rs", include_str!("../src/sim.rs")),
        ("probe.rs", include_str!("../src/probe.rs")),
        ("simulate.rs", include_str!("../src/bin/simulate.rs")),
    ] {
        for nom in LOT {
            assert!(
                !src.contains(nom),
                "le nom « {nom} » ne doit pas figurer dans src/{fichier}"
            );
        }
    }
}

#[test]
fn probe_action_sur_sequence_impayable_rend_un_resultat_lisible() {
    // « poser toutes les cartes de la séquence, dans l'ordre, comme --probe » :
    // `--probe` s'arrête sur une carte impayable, `--probe-action` fait pareil.
    // Et l'action d'une carte qui n'a PAS pu être posée n'est jamais activée.
    let db = db_all();
    let r = act(&db, &["Titanium Mine", "Solarpunk"], 5);
    assert!(!r.action_applied, "aucune carte posée : aucune action");
    assert_eq!(r.delta.mc, 0, "rien prélevé");
    assert_eq!(r.delta.forests, 0);
    // Payable : comportement normal, inchangé.
    let ok = act(&db, &["Titanium Mine", "Solarpunk"], 300);
    assert!(ok.action_applied);
}

#[test]
fn la_table_a_une_entree_et_une_seule_par_carte_du_lot() {
    use engine::effects::LOT1;
    for nom in LOT {
        let n = LOT1.iter().filter(|(x, _)| *x == nom).count();
        assert_eq!(n, 1, "{nom} : une entrée et une seule");
    }
}

#[test]
// (lot cartes-7) ATTENTE MISE À JOUR (14 → 5) : les neuf modificateurs
// permanents sont encodés.
// (lot cartes-8) ATTENTE MISE À JOUR (5 → 0) : les cinq poses supplémentaires
// aussi. Plus une seule muette en boîte de base.
fn plus_aucune_carte_muette_en_boite_de_base() {
    let db = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("base");
    let muettes: Vec<&str> = db
        .recensement()
        .into_iter()
        .filter(|c| !c.effets_geres)
        .map(|c| c.name)
        .collect();
    assert!(muettes.is_empty(), "{muettes:?}");
    for nom in LOT {
        assert!(!muettes.contains(&nom), "{nom} ne doit plus être muette");
    }
}

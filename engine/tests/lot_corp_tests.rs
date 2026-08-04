//! Tests du chantier moteur-corporations-1 : les 12 corporations de la boîte de
//! base ont leurs effets.
//!
//! Discipline des lots précédents : aucun chemin de test parallèle. Les tests
//! passent soit par la SONDE (`probe::run_probe_seq_corp`, qui emprunte
//! `flow::build_card_with` comme `simulate`), soit par une PARTIE RÉELLE
//! (`flow::setup_game` + `flow::play_round` avec une politique scriptée). Aucun
//! test ne fabrique un état que la partie réelle ne produirait pas.
//!
//! La corporation d'un joueur n'est jamais posée « à la main » : `corp_seed`
//! cherche la première graine à laquelle le tirage réel donne au joueur 0 la
//! corporation voulue. C'est le tirage du moteur, pas un état bricolé.

use engine::cards::{CardsDb, Tag};
use engine::effects::{corp_lookup, CORPS};
use engine::flow::{
    card_discount, forest_plant_cost, heat_reserved_by, play_round, research_extra, setup_game,
    spendable_mc,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_corp, ProbeOptions, ProbeResult, ProbeScript};
use engine::state::{GameState, FOREST_PLANT_COST};
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("base de cartes")
}

/// Les 12 noms exacts du périmètre (planches de la boîte de base).
const BASE: [&str; 12] = [
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
];

// ------------------------------------------------------------------ outillage

/// Sonde : séquence de cartes jouée avec la corporation `corp` imposée.
fn probe(db: &CardsDb, corp: &str, cards: &[&str]) -> ProbeResult {
    run_probe_seq_corp(
        db,
        cards,
        ProbeOptions::default(),
        &ProbeScript::default(),
        false,
        Some(corp),
    )
}

/// Sonde avec MC de départ imposé et réponses de politique imposées.
fn probe_opts(
    db: &CardsDb,
    corp: Option<&str>,
    cards: &[&str],
    mc: i64,
    choices: Vec<usize>,
    produce: bool,
) -> ProbeResult {
    let opts = ProbeOptions { mc, ..ProbeOptions::default() };
    let script = ProbeScript { choices, targets: Vec::new(), joker_tag: None };
    run_probe_seq_corp(db, cards, opts, &script, produce, corp)
}

/// Politique scriptée : phases imposées, aucune construction, aucune action.
/// Elle sert aux tests qui veulent observer UNE phase précise sans que le bruit
/// des autres décisions n'entre en jeu — les points de décision restent ceux du
/// moteur.
struct Passive {
    phases: [u8; 2],
}

impl Passive {
    fn new(p0: u8, p1: u8) -> Passive {
        Passive { phases: [p0, p1] }
    }
}

impl Policy for Passive {
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, _r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        let want = self.phases[p];
        assert!(allowed.contains(&want), "phase {want} interdite pour {p}");
        want
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
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, _d: &[u16], keep: usize) -> Vec<usize> {
        (0..keep).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
    /// Unmi : brancher 0 = payer les 6 MC (l'option imprimée).
    fn choose_option(&mut self, _r: &mut StdRng, _p: usize, _n: usize) -> usize {
        0
    }
}

/// Première graine à laquelle le TIRAGE RÉEL de `setup_game` donne au joueur 0
/// la corporation nommée. Aucune corporation n'est posée à la main.
fn corp_seed(db: &CardsDb, name: &str, phases: (u8, u8)) -> u64 {
    for seed in 0..20_000u64 {
        let mut pol = Passive::new(phases.0, phases.1);
        let game = setup_game(db, seed, &mut pol);
        let c = game.players[0].corporation.expect("corporation choisie");
        if db.corporations[c as usize].name == name {
            return seed;
        }
    }
    panic!("aucune graine ne donne « {name} » au joueur 0");
}

/// Partie réelle dont le joueur 0 a la corporation nommée.
fn game_with(db: &CardsDb, name: &str, phases: (u8, u8)) -> (GameState, Passive) {
    let seed = corp_seed(db, name, phases);
    let mut pol = Passive::new(phases.0, phases.1);
    let game = setup_game(db, seed, &mut pol);
    (game, pol)
}

// ================================================== PARTIE 1 — la pioche à 12

#[test]
fn pioche_exactement_les_12_corporations_de_la_boite_de_base() {
    let db = db();
    let noms: Vec<&str> = db.corporations.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(noms.len(), 12, "la pioche ne contient que la boîte de base");
    for n in BASE {
        assert!(noms.contains(&n), "{n} manque à la pioche");
    }
    // Les quatre corporations de l'EXTENSION : encodées depuis le chantier
    // `jokers-corpos`, elles n'en restent pas moins hors de la pioche de la
    // BOÎTE DE BASE — elles n'ont aucune planche imprimée dedans.
    for intruse in ["Apollo Industries", "Exocorp", "Hyperion Systems", "Sultira"] {
        assert!(!noms.contains(&intruse), "{intruse} ne doit pas être dans la pioche");
    }
}

#[test]
fn la_table_des_corporations_est_la_definition_de_la_boite() {
    // La pioche est un MIROIR de la table déclarée : c'est elle qui décide, pas
    // un filtre par nom écrit ailleurs.
    //
    // (jokers-corpos) La table décrit désormais les SEIZE planches des deux
    // boîtes, et non plus les douze de la seule boîte de base : les quatre
    // planches de Découverte y sont entrées avec leur encodage. Les deux sens du
    // miroir sont conservés — chaque planche de la BOÎTE CHARGÉE figure dans la
    // table, et chaque planche chargée y est retrouvée.
    assert_eq!(CORPS.len(), 16);
    let db = db();
    for n in BASE {
        assert!(
            CORPS.iter().any(|(name, _)| *name == n),
            "{n} chargée dans la pioche mais absente de la table"
        );
    }
    for c in &db.corporations {
        assert!(corp_lookup(&c.name).is_some(), "{} chargée hors table", c.name);
    }
}

#[test]
fn teractor_corporation_est_lentree_de_la_pioche_pas_son_homonyme() {
    // Piège d'appariement : `cards.json` porte DEUX entrées « Teractor
    // Corporation », l'une hors pioche à 48 MC, l'autre dans la pioche à 51.
    let db = db();
    let t: Vec<&engine::cards::Corporation> = db
        .corporations
        .iter()
        .filter(|c| c.name == "Teractor Corporation")
        .collect();
    assert_eq!(t.len(), 1, "une seule Teractor Corporation dans la pioche");
    assert_eq!(t[0].starting_mc, 51, "l'entrée in_deck_v1 est celle à 51 MC");
}

#[test]
fn la_conservation_des_corporations_porte_sur_12() {
    let db = db();
    let mut pol = RandomPolicy;
    let game = setup_game(&db, 2024, &mut pol);
    let total = game.corp_deck.len() + game.corp_discard.len()
        + game.players.iter().filter(|p| p.corporation.is_some()).count();
    assert_eq!(total, 12);
}

// ============================================ PARTIE 3 — production de départ

#[test]
fn ecoline_helion_thorgate_produisent_des_la_premiere_phase_iv() {
    // La production de départ est inscrite sur les pistes FIXES, donc consommée
    // par la VRAIE phase IV (`--probe-produce` l'exécute) — ce n'est pas un gain
    // unique à la mise en place.
    let db = db();
    for (nom, heat, plants) in [
        ("Ecoline", 0, 1),
        ("Helion Corporation", 3, 0),
        ("Thorgate Corporation", 1, 0),
    ] {
        let r = probe_opts(&db, Some(nom), &[], 100, vec![], true);
        assert!(r.produced, "{nom} : la phase de production doit avoir tourné");
        assert_eq!(r.delta.heat, heat, "{nom} : chaleur produite");
        assert_eq!(r.delta.plants, plants, "{nom} : plantes produites");
    }
    // Témoin : une corporation sans production de départ ne gagne rien.
    let r = probe_opts(&db, Some("Credicor"), &[], 100, vec![], true);
    assert_eq!((r.delta.heat, r.delta.plants), (0, 0), "Credicor ne produit rien");
}

#[test]
fn la_production_de_depart_se_repete_a_chaque_phase_iv() {
    // Le piège que le contrat nomme : un gain unique à la mise en place passerait
    // la première phase et échouerait à la seconde. On joue DEUX phases IV
    // réelles et on vérifie que Helion touche 3 chaleur à chacune.
    let db = db();
    let (mut game, mut pol) = game_with(&db, "Helion Corporation", (4, 4));
    game.players[0].heat = 0;
    play_round(&mut game, &db, &mut pol);
    let apres_1 = game.players[0].heat;
    assert_eq!(apres_1, 3, "première phase IV");
    // La phase IV est interdite deux manches de suite (livret p.10) : on passe
    // par une manche de recherche, puis on y revient.
    let mut pol2 = Passive::new(5, 5);
    play_round(&mut game, &db, &mut pol2);
    let mut pol3 = Passive::new(4, 4);
    play_round(&mut game, &db, &mut pol3);
    assert_eq!(game.players[0].heat, 6, "seconde phase IV : la production se répète");
}

// ================================== PARTIE 2 — un effet par corporation (12)

#[test]
fn credicor_reduit_de_4_mc_les_cartes_a_20_mc_ou_plus() {
    let db = db();
    // Commercial District : prix IMPRIMÉ 25 ≥ 20 → 21.
    let avec = probe(&db, "Credicor", &["Commercial District"]);
    assert_eq!(avec.paid, vec![21]);
    // Témoin sans corporation : plein tarif.
    let sans = run_probe_seq_corp(
        &db,
        &["Commercial District"],
        ProbeOptions::default(),
        &ProbeScript::default(),
        false,
        None,
    );
    assert_eq!(sans.paid, vec![25]);
    // Sous le seuil, aucune réduction (Grass : 9 MC).
    let petit = probe(&db, "Credicor", &["Grass"]);
    assert_eq!(petit.paid, vec![9], "le seuil porte sur le prix imprimé");
}

#[test]
fn ecoline_paie_une_foret_une_plante_de_moins() {
    let db = db();
    // 7 plantes : assez pour Ecoline (8 − 1), pas pour les autres.
    let (mut game, mut pol) = game_with(&db, "Ecoline", (3, 3));
    assert_eq!(forest_plant_cost(&db, &game.players[0]), 7);
    game.players[0].plants = 7;
    game.players[0].heat = 0;
    game.players[1].plants = 0;
    game.players[1].heat = 0;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].forests, 1, "la conversion obligatoire a bâti la forêt");
    assert_eq!(game.players[0].plants, 0, "7 plantes dépensées, pas 8");
    assert_eq!(game.corp_forest_rebates, 1, "compteur alimenté au site du paiement");

    // Témoin : une corporation sans remise ne peut pas bâtir avec 7 plantes.
    let (mut g2, mut p2) = game_with(&db, "Credicor", (3, 3));
    assert_eq!(forest_plant_cost(&db, &g2.players[0]), FOREST_PLANT_COST);
    g2.players[0].plants = 7;
    g2.players[0].heat = 0;
    g2.players[1].plants = 0;
    g2.players[1].heat = 0;
    play_round(&mut g2, &db, &mut p2);
    assert_eq!(g2.players[0].forests, 0);
    assert_eq!(g2.players[0].plants, 7);
    assert_eq!(g2.corp_forest_rebates, 0);
}

#[test]
fn helion_corporation_paie_une_carte_avec_sa_chaleur() {
    let db = db();
    // 0 MC, 20 chaleur : Mohole Area coûte 18 → payée en chaleur.
    let r = probe_opts(&db, Some("Helion Corporation"), &["Mohole Area"], 0, vec![], false);
    assert!(r.played, "la carte est posée");
    assert_eq!(r.paid, vec![18]);
    assert_eq!(r.delta.heat, -18, "18 chaleur dépensées comme 18 MC");
    assert!(r.discarded.iter().all(|&d| d == 0), "aucune carte défaussée pour payer");

    // Témoin sans Helion : la même carte n'est pas payable.
    let sans = probe_opts(&db, None, &["Mohole Area"], 0, vec![], false);
    assert!(!sans.played, "sans Helion, 0 MC ne paie pas 18");

    // La chaleur ne sert QUE de complément : avec 10 MC, seules 8 chaleur partent.
    let mixte = probe_opts(&db, Some("Helion Corporation"), &["Mohole Area"], 10, vec![], false);
    assert_eq!(mixte.delta.heat, -8, "la chaleur comble le manque, rien de plus");
}

#[test]
fn helion_le_may_n_est_plus_une_alternative() {
    // (regles-de-la-vente) « You MAY use heat as MC ». Ce « may » n'était une
    // ALTERNATIVE que parce que le moteur offrait, en face, de payer en
    // défaussant des cartes : renoncer à la chaleur voulait dire « je paierai en
    // vendant ». Cette vente d'office est le défaut B, et elle a disparu.
    // Renoncer à la chaleur reviendrait maintenant à renoncer à une carte que le
    // joueur vient de choisir de poser : une seule branche jouable, et la
    // convention du lot 3 interdit d'interroger la politique là-dessus.
    //
    // Le test est donc inversé, mise en place gardée : main garnie ou main vide,
    // scriptée sur la branche 0 ou sur la branche 1, la chaleur paie toujours et
    // aucune carte ne quitte la main. Le script sur la branche 1 est le cas qui
    // compte : il prouve que la question n'est PLUS posée — si elle l'était
    // encore, le « 1 » serait consommé et la chaleur resterait intacte.
    let db = db();
    for filler in [0usize, 6] {
        for branche in [0usize, 1] {
            let opts = ProbeOptions { mc: 0, filler, ..ProbeOptions::default() };
            let script = ProbeScript {
                choices: vec![branche],
                targets: Vec::new(),
                joker_tag: None,
            };
            let r = run_probe_seq_corp(
                &db, &["Mohole Area"], opts, &script, false, Some("Helion Corporation"),
            );
            assert!(r.played, "main {filler}, branche {branche} : la carte est posée");
            assert_eq!(
                r.delta.heat, -18,
                "main {filler}, branche {branche} : la chaleur paie, sans qu'on demande rien"
            );
            assert_eq!(
                r.discarded, vec![0],
                "main {filler}, branche {branche} : aucune carte vendue d'office"
            );
        }
    }
}

#[test]
fn une_sequence_au_dernier_nom_vide_garde_son_comportement_davant_le_lot() {
    // `--probe "Grass;"` : le dernier nom ne résout pas, la sonde rend
    // `found:false` et `paid:[]` — exactement comme avant ce lot. Seule une
    // séquence RÉELLEMENT vide (`--probe-corp` sans `--probe`) prend le chemin
    // neuf.
    let db = db();
    let r = run_probe_seq_corp(
        &db,
        &["Grass", ""],
        ProbeOptions::default(),
        &ProbeScript::default(),
        false,
        None,
    );
    assert!(!r.found);
    assert!(!r.played);
    assert!(r.paid.is_empty());
    assert_eq!(r.card, "");
    // Séquence vide + corporation : chemin neuf, la sonde se déroule.
    let vide = probe_opts(&db, Some("Ecoline"), &[], 100, vec![], true);
    assert!(!vide.found);
    assert!(!vide.played);
    assert!(vide.produced);
    assert_eq!(vide.delta.plants, 1);
}

#[test]
fn le_cout_dune_foret_ne_descend_jamais_a_zero() {
    // Plancher du service : la conversion obligatoire de fin de phase III est un
    // `while plants >= forest_plant_cost(...)` — un coût nul y bouclerait sans
    // fin. Aucune donnée actuelle n'en approche, le plancher ferme la classe.
    let db = db();
    for c in &db.corporations {
        let spec = c.effect.expect("corporation encodée");
        assert!(
            spec.forest_plant_rebate < FOREST_PLANT_COST,
            "{} : une remise >= {FOREST_PLANT_COST} rendrait la forêt gratuite",
            c.name
        );
    }
    let (game, _) = game_with(&db, "Ecoline", (1, 1));
    assert!(forest_plant_cost(&db, &game.players[0]) >= 1);
}

#[test]
fn helion_ne_convertit_pas_la_chaleur_promise_a_un_prerequis() {
    // Régression D14 : « Requires you to spend 5 heat » (Tropical Resort) engage
    // 5 chaleur ; Helion ne doit pas les convertir pour payer le prix, sinon la
    // dépense de pose serait impayable.
    let db = db();
    let id = db.resolve_card("Tropical Resort").unwrap();
    assert_eq!(heat_reserved_by(&db, id), 5);
    let r = probe_opts(&db, Some("Helion Corporation"), &["Tropical Resort"], 0, vec![], false);
    // 20 chaleur, 5 réservées → 15 convertibles ; Tropical Resort coûte 19 :
    // la carte n'est donc PAS payable, et la sonde s'arrête proprement. Sans la
    // réserve, 20 chaleur auraient payé les 19 MC et la dépense de pose aurait
    // sauté sur son assertion (bug D14).
    assert!(!r.played, "la chaleur réservée n'est pas de la monnaie");
    // Avec 4 MC, elle passe : 4 MC + 15 chaleur = 19, et les 5 chaleur réservées
    // sont bien dépensées ensuite par le prérequis.
    let r2 = probe_opts(&db, Some("Helion Corporation"), &["Tropical Resort"], 4, vec![], false);
    assert!(r2.played);
    assert_eq!(r2.delta.heat, -20, "15 converties en MC + 5 dépensées par le prérequis");
    assert_eq!(r2.delta.mc_prod, 4, "l'effet de la carte s'applique bien");
}

#[test]
fn interplanetary_cinematics_reduit_les_building_et_les_event() {
    let db = db();
    // Le texte IMPRIMÉ porte les deux réductions ; `cards.json` n'en décrit
    // qu'une (et invente une production d'acier) — le texte imprimé gagne.
    let bat = probe(&db, "Interplanetary Cinematics", &["Coal Imports"]); // 13, BUILDING
    assert_eq!(bat.paid, vec![11]);
    let ev = probe(&db, "Interplanetary Cinematics", &["Lava Flows"]); // 17, EVENT
    assert_eq!(ev.paid, vec![15]);
    // Une carte portant les deux badges cumule (Comet : SPACE + EVENT → −2).
    let deux = probe(&db, "Interplanetary Cinematics", &["Comet"]); // 25, SPACE+EVENT
    assert_eq!(deux.paid, vec![23]);
}

#[test]
fn inventrix_pioche_3_cartes_a_la_mise_en_place() {
    let db = db();
    let (game, _) = game_with(&db, "Inventrix", (1, 1));
    assert_eq!(game.players[0].hand.len(), 11, "8 cartes de départ + 3 d'Inventrix");
    // Témoin : une corporation sans pioche de départ laisse 8 cartes.
    let (g2, _) = game_with(&db, "Credicor", (1, 1));
    assert_eq!(g2.players[0].hand.len(), 8);
}

#[test]
fn inventrix_assouplit_les_prerequis_dun_palier_de_couleur() {
    let db = db();
    // Bushes exige une température ROUGE ; la sonde part de VIOLET.
    let avec = probe(&db, "Inventrix", &["Bushes"]);
    assert!(avec.prereq_ok, "violet + 1 palier = rouge : prérequis rempli");
    let sans = run_probe_seq_corp(
        &db,
        &["Bushes"],
        ProbeOptions::default(),
        &ProbeScript::default(),
        false,
        None,
    );
    assert!(!sans.prereq_ok, "sans Inventrix, violet ne suffit pas");
    // La souplesse est d'UN palier, pas de deux : Trees exige le JAUNE.
    let trop = probe(&db, "Inventrix", &["Trees"]);
    assert!(!trop.prereq_ok, "violet + 1 = rouge, toujours pas jaune");
    // La pioche de départ est visible dans le delta de main.
    assert_eq!(avec.delta.hand, 3);
}

#[test]
fn mining_guild_reduit_les_building_de_2() {
    let db = db();
    let r = probe(&db, "Mining Guild", &["Coal Imports"]); // 13, BUILDING
    assert_eq!(r.paid, vec![11]);
    // La partie hors portée (production d'acier) n'invente aucun gain.
    assert_eq!(r.delta.tr, 0, "aucun NT : le déclencheur acier n'est pas simulé");
}

#[test]
fn phobolog_reduit_les_space_de_3() {
    let db = db();
    let r = probe(&db, "Phobolog", &["Ice Asteroid"]); // 21, SPACE
    // ATTENTE MISE À JOUR par le lot acier-titane (18 → 17), et c'est la
    // correction d'un manque, pas une régression : la planche porte DEUX
    // lignes. Le −3 MC sur les cartes espace EST son savoir-faire (encart gris,
    // un titane), et son EFFET imprimé — « Each titanium you have reduces the
    // cost of [space] cards an additional 1 MC » — s'applique à ce titane-là.
    // 21 − 3 − 1 = 17. L'ancienne attente valait pour un moteur où le titane
    // n'existait pas ; il existe.
    assert_eq!(r.paid, vec![17]);
    let corp = corp_lookup("Phobolog").unwrap();
    assert_eq!(
        corp.reductions.len(),
        2,
        "les deux lignes de la planche : le −3 Space, et l'effet par titane"
    );
}

#[test]
fn saturn_systems_reduit_les_space_et_gagne_1_nt_par_jupiter() {
    let db = db();
    // Water Import from Europa : 22 MC, SPACE + JUPITER → −3 et +1 NT.
    let r = probe(&db, "Saturn Systems", &["Water Import from Europa"]);
    assert_eq!(r.paid, vec![19]);
    assert_eq!(r.delta.tr, 1, "un badge Jupiter = un pas de NT");
    // Une carte SANS badge Jupiter ne déclenche rien.
    let sans = probe(&db, "Saturn Systems", &["Ice Asteroid"]); // SPACE seul
    assert_eq!(sans.paid, vec![18]);
    assert_eq!(sans.delta.tr, 2, "les 2 NT viennent des océans de la carte, pas du badge");
}

#[test]
fn teractor_corporation_reduit_les_earth_de_3() {
    let db = db();
    let r = probe(&db, "Teractor Corporation", &["Bribed Comittee"]); // 5, EARTH+EVENT
    assert_eq!(r.paid, vec![2]);
}

#[test]
fn tharsis_republic_pioche_et_garde_une_carte_de_plus_en_phase_v() {
    let db = db();
    assert_eq!(research_extra(&db, &engine::state::PlayerState::new()), (0, 0));
    // Joueur 0 non-sélectionneur de la phase V : base 2/1 → 3/2 avec Tharsis.
    let (mut game, mut pol) = game_with(&db, "Tharsis Republic", (1, 5));
    assert_eq!(research_extra(&db, &game.players[0]), (1, 1));
    let avant = game.players[0].hand.len();
    play_round(&mut game, &db, &mut pol);
    assert_eq!(game.players[0].hand.len(), avant + 2, "2 gardées au lieu d'1");
    assert_eq!(game.research_extra_draws, 1, "1 carte piochée en plus, comptée au site");

    // Témoin : une corporation sans bonus garde 1 carte.
    let (mut g2, mut p2) = game_with(&db, "Credicor", (1, 5));
    assert_eq!(research_extra(&db, &g2.players[0]), (0, 0));
    let avant2 = g2.players[0].hand.len();
    play_round(&mut g2, &db, &mut p2);
    assert_eq!(g2.players[0].hand.len(), avant2 + 1);
    assert_eq!(g2.research_extra_draws, 0);
}

#[test]
fn thorgate_corporation_produit_1_chaleur_et_reduit_les_energy() {
    let db = db();
    let r = probe(&db, "Thorgate Corporation", &["Geothermal Power"]); // 8, BUILDING+ENERGY
    assert_eq!(r.paid, vec![5]);
    assert_eq!(r.corp.as_ref().unwrap().start_prod, (0, 1, 0));
}

#[test]
fn unmi_achete_un_pas_de_nt_pour_6_mc_une_fois_par_phase() {
    let db = db();
    // Bribed Comittee donne 2 NT ; Unmi propose le pas bonus au PREMIER.
    let paye = probe_opts(&db, Some("Unmi"), &["Bribed Comittee"], 100, vec![0], false);
    assert_eq!(paye.delta.tr, 3, "2 NT de la carte + 1 acheté");
    assert_eq!(paye.delta.mc, -6, "6 MC dépensés");
    // Le choix est un vrai choix : la branche « renoncer » ne coûte rien.
    let refuse = probe_opts(&db, Some("Unmi"), &["Bribed Comittee"], 100, vec![1], false);
    assert_eq!(refuse.delta.tr, 2);
    assert_eq!(refuse.delta.mc, 0);
    // UNE SEULE fois par phase, même sur quatre pas de NT.
    let deux = probe_opts(
        &db,
        Some("Unmi"),
        &["Bribed Comittee", "Release of Inert Gases"],
        100,
        vec![0, 0],
        false,
    );
    assert_eq!(deux.delta.tr, 5, "4 NT des cartes + 1 seul acheté");
    assert_eq!(deux.delta.mc, -6, "6 MC une seule fois");
    // Sans les 6 MC, l'offre n'est pas faite (et rien ne casse).
    let pauvre = probe_opts(&db, Some("Unmi"), &["Bribed Comittee"], 5, vec![0], false);
    assert_eq!(pauvre.delta.tr, 2, "6 MC impayables : pas de pas bonus");
}

// ======================================== PARTIE 5 — l'interface de sonde

#[test]
fn probe_corp_impose_la_corporation_et_rend_son_descriptif() {
    let db = db();
    let r = probe(&db, "Credicor", &["Grass"]);
    let c = r.corp.expect("objet corp présent avec --probe-corp");
    assert_eq!(c.name, "Credicor");
    assert!(c.found);
    assert!(c.encoded);
    assert_eq!(c.starting_mc, 48);
    assert_eq!(c.start_prod, (0, 0, 0));
}

#[test]
fn probe_corp_inconnue_rend_found_false_sans_interrompre_la_sonde() {
    let db = db();
    // Zetacell existe dans cards.json mais hors boîte de base : introuvable.
    let r = probe_opts(&db, Some("Zetacell"), &[], 100, vec![], true);
    let c = r.corp.expect("objet corp présent");
    assert!(!c.found);
    assert!(!c.encoded);
    assert!(r.produced, "la sonde se déroule quand même");
    // Idem pour une intruse retirée de la pioche.
    let apollo = probe_opts(&db, Some("Apollo Industries"), &[], 100, vec![], true);
    assert!(!apollo.corp.unwrap().found);
}

#[test]
fn une_sonde_sans_probe_corp_est_inchangee() {
    // Contrat : les sondes existantes gardent exactement leur comportement.
    let db = db();
    let r = run_probe_seq_corp(
        &db,
        &["Commercial District"],
        ProbeOptions::default(),
        &ProbeScript::default(),
        false,
        None,
    );
    assert!(r.corp.is_none(), "aucun objet corp sans --probe-corp");
    assert_eq!(r.paid, vec![25], "plein tarif, aucun effet de corporation");
    assert_eq!(r.delta.hand, 0, "convention de delta.hand des lots précédents");
    assert_eq!(r.delta.mc_prod, 4);
    assert!(r.played && r.found && r.in_lot);
}

// ================================================ services uniques & effets off

#[test]
fn les_reductions_de_corporation_passent_par_card_discount() {
    // NEVER 3 : pas de second calcul. La réduction observée par la sonde
    // (`paid`) est exactement celle que rend le service unique.
    let db = db();
    let (game, _) = game_with(&db, "Teractor Corporation", (1, 1));
    let earth = db.resolve_card("Bribed Comittee").unwrap();
    let autre = db.resolve_card("Grass").unwrap();
    assert_eq!(card_discount(&game, &db, 0, earth), 3);
    assert_eq!(card_discount(&game, &db, 0, autre), 0);
    // Le joueur 1 n'a pas cette corporation : sa réduction est la sienne.
    assert_ne!(
        db.corporations[game.players[1].corporation.unwrap() as usize].name,
        "Teractor Corporation"
    );
}

#[test]
fn spendable_mc_ne_compte_la_chaleur_que_pour_helion() {
    let db = db();
    let (mut game, _) = game_with(&db, "Helion Corporation", (1, 1));
    game.players[0].mc = 10;
    game.players[0].heat = 7;
    assert_eq!(spendable_mc(&db, &game.players[0]), 17);
    let (mut g2, _) = game_with(&db, "Credicor", (1, 1));
    g2.players[0].mc = 10;
    g2.players[0].heat = 7;
    assert_eq!(spendable_mc(&db, &g2.players[0]), 10);
}

#[test]
fn effects_off_coupe_tous_les_effets_de_corporation() {
    let mut db = db();
    db.effects_on = false;
    // La pioche reste celle de la boîte de base : composer la boîte n'est pas
    // un effet (journal D5).
    assert_eq!(db.corporations.len(), 12);
    // Production de départ, réduction, pioche de départ : toutes neutralisées.
    let r = probe_opts(&db, Some("Ecoline"), &[], 100, vec![], true);
    assert_eq!((r.delta.heat, r.delta.plants), (0, 0));
    assert_eq!(r.corp.unwrap().start_prod, (0, 0, 0));
    let c = probe(&db, "Credicor", &["Commercial District"]);
    assert_eq!(c.paid, vec![25], "aucune réduction en effets coupés");
    let i = probe(&db, "Inventrix", &["Bushes"]);
    assert_eq!(i.delta.hand, 0, "aucune pioche de départ");
    let mut pol = Passive::new(1, 1);
    let seed = corp_seed(&db, "Inventrix", (1, 1));
    let game = setup_game(&db, seed, &mut pol);
    assert_eq!(game.players[0].hand.len(), 8);
    assert_eq!(game.players[0].mc, 33, "le MC de départ reste : c'est la planche");
    assert_eq!(
        game.players[0].tag_counts[Tag::Science.index().unwrap()],
        1,
        "les badges restent : c'est la planche"
    );
}

#[test]
fn une_partie_reelle_fait_agir_les_corporations() {
    // Preuve d'exécution en FLUX RÉEL (et non en sonde) : sur 200 parties, les
    // quatre mécanismes de corporation se déclenchent réellement.
    let db = db();
    let mut pol = RandomPolicy;
    let s = engine::sim::run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(s.completed, 200);
    assert_eq!(s.invariant_violations, 0);
    assert!(s.corp_heat_as_mc > 0, "Helion : chaleur dépensée comme MC");
    assert!(s.corp_forest_rebates > 0, "Ecoline : forêts à 7 plantes");
    assert!(s.corp_tr_boosts > 0, "Unmi : pas de NT achetés");
    assert!(s.corp_trigger_tr > 0, "Saturn Systems : NT par badge Jupiter");
}

#[test]
fn effects_off_laisse_les_compteurs_de_corporation_a_zero() {
    let mut db = db();
    db.effects_on = false;
    let mut pol = RandomPolicy;
    let s = engine::sim::run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(s.corp_heat_as_mc, 0);
    assert_eq!(s.corp_forest_rebates, 0);
    assert_eq!(s.corp_tr_boosts, 0);
    assert_eq!(s.corp_trigger_tr, 0);
}

#[test]
fn les_douze_corporations_portent_un_effet_declare() {
    // `encoded` de `--dump-corporations` vient de là.
    let db = db();
    for c in &db.corporations {
        assert!(c.effect.is_some(), "{} sans encodage", c.name);
    }
    // Aucune n'est un stub vide : chacune applique au moins une chose.
    for (name, spec) in CORPS {
        let vide = spec.start_prod.mc == 0
            && spec.start_prod.heat == 0
            && spec.start_prod.plants == 0
            && spec.start_draw == 0
            && spec.reductions.is_empty()
            && spec.play_triggers.is_empty()
            && spec.research.is_none()
            && spec.forest_plant_rebate == 0
            && !spec.heat_as_mc
            && !spec.req_color_flex
            && spec.tr_boost.is_none()
            // (jokers-corpos) Les quatre champs des planches de Découverte : une
            // corporation qui ne ferait QUE l'un d'eux n'est pas un stub non plus.
            && spec.setup.is_empty()
            && spec.discard_bonus == 0
            && spec.action.is_none()
            && spec.phase_bonus.is_none();
        assert!(!vide, "{name} est un stub neutre");
    }
}

#[test]
fn le_determinisme_a_graine_fixe_tient_avec_les_corporations() {
    let db = db();
    let mut p1 = RandomPolicy;
    let a = engine::sim::run_simulation(&db, 100, 4242, &mut p1);
    let mut p2 = RandomPolicy;
    let b = engine::sim::run_simulation(&db, 100, 4242, &mut p2);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.corp_tr_boosts, b.corp_tr_boosts);
}

#[test]
fn les_badges_rendus_par_dump_corporations_sont_ceux_de_cards_json() {
    // `--dump-corporations` écrit les badges via `Tag::as_str`, qui doit être
    // l'inverse exact de `Tag::from_str` — sans quoi la sortie ne se comparerait
    // pas au fichier source sans transformation.
    for t in [
        Tag::Building,
        Tag::Space,
        Tag::Science,
        Tag::Plant,
        Tag::Microbe,
        Tag::Animal,
        Tag::Earth,
        Tag::Jupiter,
        Tag::Energy,
        Tag::Event,
        Tag::Dynamic,
    ] {
        assert_eq!(Tag::from_str(t.as_str()), Some(t), "aller-retour cassé pour {t:?}");
    }
    // Et les badges des corporations chargées sont bien ceux des planches.
    let db = db();
    let mining = db.corporations.iter().find(|c| c.name == "Mining Guild").unwrap();
    assert_eq!(
        mining.tags.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        vec!["BUILDING", "BUILDING"],
        "Mining Guild porte DEUX badges Construction imprimés"
    );
}

//! Tests du chantier **moteur-boites-1** : appartenance de boîte, composition
//! commandable de la pioche, et honnêteté du recensement.
//!
//! Ces tests vont au-delà de `inputs/checks/` : ils portent sur des propriétés
//! que les checks ne regardent pas (le drapeau `in_deck_v1` ne décide plus rien,
//! le compteur d'effets non gérés s'incrémente à la carte près, les cartes
//! déclarées non gérées le sont RÉELLEMENT — vérifié par la sonde, oracle
//! disjoint de la table de recensement).

use engine::boites::{Boite, BoiteSet, Kind};
use engine::cards::CardsDb;
use engine::flow::{build_card, setup_game};
use engine::probe::{run_probe_seq_corp, ProbeOptions, ProbeScript};
use engine::policy::RandomPolicy;
use std::collections::HashSet;

const CARDS: &str = "../data/cards.json";

fn db_de(liste: &str) -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse(liste).unwrap())
        .unwrap_or_else(|e| panic!("chargement {liste}: {e}"))
}

// ------------------------------------------------------------ les effectifs

#[test]
fn les_quatre_configurations_ont_les_effectifs_des_planches() {
    for (liste, projets, corps) in [
        ("base", 208, 12),
        ("base,promo", 219, 12),
        ("base,decouverte", 246, 16),
        ("base,promo,decouverte", 257, 16),
    ] {
        let db = db_de(liste);
        assert_eq!(db.deck_project_count, projets, "projets de --boites {liste}");
        assert_eq!(db.corporations.len(), corps, "corporations de --boites {liste}");
        let r = db.recensement();
        assert_eq!(r.len(), projets + corps, "recensement de --boites {liste}");
    }
}

#[test]
fn le_defaut_est_la_boite_de_base_seule() {
    let par_defaut = CardsDb::load(CARDS).unwrap();
    let explicite = db_de("base");
    assert_eq!(par_defaut.deck_project_count, explicite.deck_project_count);
    assert_eq!(par_defaut.corporations.len(), explicite.corporations.len());
    assert!(par_defaut.boites.contains(Boite::Base));
    assert!(!par_defaut.boites.contains(Boite::Promo));
    assert!(!par_defaut.boites.contains(Boite::Decouverte));
}

#[test]
fn une_boite_inconnue_est_refusee_plutot_que_silencieusement_ignoree() {
    assert!(BoiteSet::parse("discovery").is_err());
    assert!(BoiteSet::parse("base,fan").is_err());
    assert!(BoiteSet::parse("").is_err());
    assert!(BoiteSet::parse(",").is_err());
}

// --------------------------------------------- le drapeau v1 ne décide plus

#[test]
fn in_deck_v1_ne_compose_plus_la_pioche() {
    let db = db_de("base");
    // Les 40 cartes que le portage Java distribuait et que les planches ne
    // contiennent pas : 38 de Découverte + les 2 inventées.
    let v1_hors_pioche = db
        .projects
        .iter()
        .filter(|c| c.in_deck_v1 && !c.in_deck)
        .count();
    assert_eq!(v1_hors_pioche, 40, "38 Découverte + 2 cartes inexistantes");

    // …et l'inverse : 11 cartes que le portage EXCLUAIT alors qu'Alexis les
    // possède (les promotionnelles), entrent bien dans la pioche quand on
    // demande leur boîte.
    let promo = db_de("base,promo");
    let hors_v1_en_pioche: Vec<&str> = promo
        .projects
        .iter()
        .filter(|c| !c.in_deck_v1 && c.in_deck)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(hors_v1_en_pioche.len(), 11, "les 11 cartes PROMO 2021");
}

#[test]
fn les_cartes_inexistantes_ne_sont_dans_aucune_configuration() {
    for liste in ["base", "base,promo", "base,decouverte", "base,promo,decouverte"] {
        let db = db_de(liste);
        for fantome in ["Microbiology Patents", "Project Inspection"] {
            let c = db
                .projects
                .iter()
                .find(|c| c.name == fantome)
                .unwrap_or_else(|| panic!("{fantome} doit rester chargée pour la sonde"));
            assert!(!c.in_deck, "{fantome} distribuée en --boites {liste}");
            assert!(c.boite.is_none(), "{fantome} n'appartient à aucune boîte");
            assert!(
                !db.recensement().iter().any(|r| r.name == fantome),
                "{fantome} recensée en --boites {liste}"
            );
        }
    }
}

#[test]
fn aucun_doublon_de_nom_dans_aucune_configuration() {
    for liste in ["base", "base,promo", "base,decouverte", "base,promo,decouverte"] {
        let db = db_de(liste);
        let mut vus = HashSet::new();
        for c in db.recensement() {
            assert!(vus.insert(c.name.to_string()), "{} en double ({liste})", c.name);
        }
    }
}

#[test]
fn toute_carte_retenue_vient_d_une_boite_demandee_et_porte_sa_planche() {
    let planches_de_base: HashSet<&str> = ["P1", "P2", "P3", "P4", "CORP"].into_iter().collect();
    for liste in ["base", "base,promo", "base,decouverte", "base,promo,decouverte"] {
        let demandees = BoiteSet::parse(liste).unwrap();
        for c in db_de(liste).recensement() {
            assert!(demandees.contains(c.boite), "{} hors des boîtes demandées", c.name);
            match c.boite {
                Boite::Base => {
                    let p = c.planche.expect("carte de base sans planche");
                    assert!(planches_de_base.contains(p), "{} sur la planche {p}", c.name);
                    assert_eq!(
                        p == "CORP",
                        c.kind == Kind::Corporation,
                        "{} : planche et genre en désaccord",
                        c.name
                    );
                }
                Boite::Promo => assert_eq!(c.planche, Some("PROMO")),
                // Découverte n'a pas de transcription de planches : le champ
                // doit être nul, jamais inventé.
                Boite::Decouverte => assert_eq!(c.planche, None, "{}", c.name),
            }
        }
    }
}

// ----------------------------------------------------------- corporations

#[test]
fn la_boite_de_base_donne_les_douze_planches_corp_et_le_bon_teractor() {
    let db = db_de("base");
    assert!(db.corporations.iter().all(|c| c.boite == Boite::Base));
    assert!(db.corporations.iter().all(|c| c.planche == Some("CORP")));
    // Les 12 planches sont toutes encodées : la table de boîtes et
    // `effects::CORPS` se recoupent exactement.
    assert!(db.corporations.iter().all(|c| c.effect.is_some()));
    let teractor: Vec<i64> = db
        .corporations
        .iter()
        .filter(|c| c.name == "Teractor Corporation")
        .map(|c| c.starting_mc)
        .collect();
    assert_eq!(teractor, vec![51], "le jumeau promo2021 à 48 MC n'entre pas");
}

#[test]
fn les_corporations_promotionnelles_ne_sont_pas_distribuees_faute_de_donnees() {
    // 5 des 6 planches PROMOCORP sont absentes de cards.json : la famille est
    // signalée, pas distribuée — ajouter `promo` ne change pas la pioche de
    // corporations.
    let base = db_de("base");
    let promo = db_de("base,promo");
    assert_eq!(promo.corporations.len(), base.corporations.len());
    assert!(promo.corporations.iter().all(|c| c.boite == Boite::Base));
    assert!(
        promo.avertissements.iter().any(|a| a.contains("PROMOCORP")),
        "l'absence doit être signalée, pas tue"
    );
}

#[test]
fn decouverte_ajoute_quatre_corporations_encodees() {
    let db = db_de("base,decouverte");
    let d: Vec<&engine::cards::Corporation> = db
        .corporations
        .iter()
        .filter(|c| c.boite == Boite::Decouverte)
        .collect();
    assert_eq!(d.len(), 4);
    for c in &d {
        // TÉMOIN RETOURNÉ par `jokers-corpos` : les quatre planches de
        // l'extension sont encodées. Le test garde son objet — il épingle ce
        // que Découverte ajoute à la pioche — mais dans l'autre sens, qui est
        // plus exigeant : une planche qui redeviendrait muette le ferait échouer.
        assert!(c.effect.is_some(), "{} : Découverte doit être encodée", c.name);
        assert_eq!(c.planche, None);
    }
    let noms: HashSet<&str> = d.iter().map(|c| c.name.as_str()).collect();
    for n in ["Apollo Industries", "Exocorp", "Hyperion Systems", "Sultira"] {
        assert!(noms.contains(n), "{n} attendue en Découverte");
    }
}

#[test]
fn les_noms_exposes_sont_ceux_de_cards_json_meme_en_decouverte() {
    // I2 bis : jamais de nom français dans le recensement. Contrôle par
    // construction — chaque nom recensé doit exister tel quel dans cards.json,
    // et aucun ne doit porter de caractère hors ASCII.
    let db = db_de("base,promo,decouverte");
    let brut = std::fs::read_to_string(CARDS).unwrap();
    let json: serde_json::Value = serde_json::from_str(&brut).unwrap();
    let connus: HashSet<String> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    for c in db.recensement() {
        assert!(connus.contains(c.name), "{} n'est pas un nom de cards.json", c.name);
        assert!(c.name.is_ascii(), "{} : nom non-ASCII exposé", c.name);
    }
}

// -------------------------------------- honnêteté du champ `effets_geres`

#[test]
fn les_cartes_declarees_non_gerees_le_sont_reellement() {
    // Oracle DISJOINT du recensement : la sonde joue la carte par le chemin
    // réel (`flow::build_card_with`) et rend l'état obtenu. Une carte déclarée
    // non gérée doit ne RIEN changer d'autre que le paiement — sinon le
    // recensement calomnierait une carte qui marche.
    // (lot cartes-8) La boîte de base n'a PLUS de carte muette : la boucle y
    // n'aurait plus de sujet. Elle est donc portée sur `base,decouverte`, où
    // 33 projets restent sans encodage — l'oracle disjoint continue de tourner
    // sur des cartes réelles, et le fait que la base soit vide est épinglé
    // séparément juste après.
    let db = db_de("base,decouverte");
    let mut n = 0;
    for c in db.recensement() {
        if c.effets_geres || c.kind != Kind::Project {
            continue;
        }
        // (lot cartes-8) « Non gérée » recouvre DEUX cas : aucun encodage du
        // tout, ou un encodage dont un effet est sauté (*Fibrous Composite
        // Material*, dont l'amélioration de phase n'existe pas). Seul le
        // premier peut être tenu de ne RIEN changer à l'état — le second a des
        // effets bien réels, c'est même tout son objet. Le test d'à côté
        // (`une_carte_encodee_mais_dont_un_effet_est_saute_n_est_pas_declaree_geree`)
        // couvre le second cas.
        let id = db.resolve_card(c.name).expect("carte du recensement");
        if db.projects[id as usize].effect.is_some() {
            continue;
        }
        n += 1;
        let r = run_probe_seq_corp(
            &db,
            &[c.name],
            ProbeOptions::default(),
            &ProbeScript::default(),
            false,
            None,
        );
        assert!(r.found, "{} introuvable", c.name);
        assert!(!r.in_lot, "{} déclarée non gérée mais encodée", c.name);
        let d = &r.delta;
        assert_eq!(
            (
                d.heat, d.plants, d.mc_prod, d.heat_prod, d.plant_prod, d.card_prod, d.tr,
                d.temperature, d.oxygen, d.oceans, d.forests
            ),
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
            "{} change l'état alors qu'elle est déclarée non gérée",
            c.name
        );
    }
    // Ce nombre est un CANARI : il doit changer le jour où des effets sont
    // encodés, et ce jour-là le rapport doit être refait.
    //
    // 27-07 : 62 muettes en boîte de base (mesure d'origine de ce test).
    // 28-07, lot 5 (`moteur-cartes-5`) : 33 d'entre elles sont encodées, il en
    // reste **29**.
    // 28-07, lot 6 (`moteur-cartes-6`) : 11 de plus sont encodées (bonus de
    // phase Action, coûts d'action particuliers, « piochez puis défaussez »,
    // dessus de pioche révélé, prérequis d'oxygène maximum), il en reste
    // **18** — les cartes hors périmètre, qui réclament des mécanismes toujours
    // absents du moteur (acier/titane comme monnaies, actions standard, cartes
    // supplémentaires jouées, phase de recherche modifiée, assouplissements de
    // prérequis). Le canari est déplacé, pas désactivé : la boucle ci-dessus
    // vérifie toujours, carte par carte et par la sonde, que chaque muette
    // déclarée ne change RIEN à l'état.
    // 28-07, lot acier-titane : 4 de plus sont encodées (les seules dont le
    // texte parlait d'un NOMBRE d'aciers ou de titanes), il en reste **14**.
    // 28-07, lot cartes-7 : 9 de plus sont encodées (les modificateurs
    // PERMANENTS — phase de recherche, taux de défausse, prix des actions
    // standard, réduction payée en plantes, souplesse de prérequis,
    // déclencheur d'action de carte, déclencheur de badge science), il en reste
    // **5**.
    // 28-07, lot cartes-8 : les 5 dernières sont encodées (les poses
    // supplémentaires — « jouer une carte de plus dans cette phase »), il en
    // reste **0**. La boîte de base est intégralement encodée ; le canari passe
    // donc sur `base,decouverte`, où 33 projets restent muets. Il est déplacé,
    // jamais désactivé : la boucle ci-dessus vérifie toujours, carte par carte
    // et par la sonde, que chaque muette déclarée ne change RIEN à l'état.
    // 29-07, `decouverte-projets` : les 28 derniers projets muets de
    // l'extension sont encodés. Il en restait **3**, les trois à badge JOKER.
    // 29-07, `jokers-corpos` : ces trois-là sont encodées. Il en reste **0** :
    // tout le contenu imprimé des deux boîtes est appliqué. La boucle ci-dessus
    // n'a plus de sujet — il n'existe plus de carte muette à éprouver — mais le
    // canari, lui, reste, et il est devenu le plus strict possible.
    assert_eq!(n, 0, "projets SANS AUCUN encodage en base + Découverte");
    assert!(
        db_de("base")
            .recensement()
            .into_iter()
            .all(|c| c.effets_geres || c.kind != Kind::Project),
        "la boîte de base doit rester intégralement encodée"
    );
    // (lot cartes-8) Les trois témoins nommés — *Asset Liquidation*,
    // *Special Design*, *Work Crews* — sont encodés depuis ce lot. Remplacés
    // par trois cartes de Découverte, toujours déclarées ABSENT.
    //
    // TÉMOINS RETOURNÉS par `decouverte-projets` : *Ore Leaching*,
    // *Warehouses* et *Metallurgy* sont trois des 28 cartes de ce chantier —
    // elles agissent désormais. Les trois derniers témoins de muettes du moteur
    // étaient alors les cartes à badge JOKER.
    //
    // TÉMOINS RETOURNÉS une dernière fois par `jokers-corpos` : elles sont
    // encodées à leur tour, et il n'existe plus AUCUNE carte déclarée ABSENT.
    // Ce que le test épingle change donc de signe — il exige désormais que ces
    // trois cartes soient GÉRÉES — et la boucle du haut, qui éprouvait par la
    // sonde que toute muette déclarée ne change rien à l'état, tourne sur une
    // liste vide parce que le moteur n'en produit plus.
    for encodee in ["Local Market", "Political Influence", "Topographic Mapping"] {
        let c = db
            .recensement()
            .into_iter()
            .find(|r| r.name == encodee)
            .expect("carte de Découverte");
        assert!(c.effets_geres, "{encodee} (badge joker) doit être gérée");
    }
}

#[test]
fn decouverte_n_est_pas_declaree_geree_en_bloc() {
    let db = db_de("base,decouverte");
    let d: Vec<_> = db
        .recensement()
        .into_iter()
        .filter(|c| c.boite == Boite::Decouverte)
        .collect();
    assert_eq!(d.len(), 42);
    let non_geres = d.iter().filter(|c| !c.effets_geres).count();
    // 42 cartes, dont 7 projets partagent un nom déjà encodé dans `LOT1`
    // (encodage hérité du portage, hors périmètre de ce lot) : 35 sans aucun
    // encodage. S'y ajoutaient `Fibrous Composite Material` et
    // `Cryogenic Shipment` : encodées, mais portant une amélioration de carte
    // Phase que le moteur SAUTAIT (`phase_upgrades_skipped`) — donc pas
    // intégralement gérées. Total 37.
    //
    // TÉMOIN RETOURNÉ par le chantier `decouverte-phases` (37 → 35) : le
    // mécanisme des cartes Phase améliorées existe, l'amélioration n'est plus
    // sautée, ces deux cartes-là sont intégralement gérées. Les 35 autres
    // n'ont toujours aucun encodage — aucune carte n'a été encodée.
    //
    // TÉMOIN RETOURNÉ par `decouverte-projets` (35 → 7) : 28 projets encodés,
    // il restait les 3 projets à badge JOKER et les 4 corporations de
    // Découverte.
    //
    // TÉMOIN RETOURNÉ par `jokers-corpos` (7 → 0) : ces sept-là sont encodées.
    // Découverte n'est toujours pas déclarée gérée « en bloc » — chaque carte
    // l'est une par une, par son propre encodage — et le nombre exact épinglé
    // est désormais zéro.
    assert_eq!(non_geres, 0);
}

// ------------------------------------------------- le compteur I4 compte

#[test]
fn le_compteur_ne_bouge_plus_car_plus_aucune_carte_n_est_muette() {
    // TÉMOIN RETOURNÉ par `jokers-corpos`. Ce test posait une carte ENCODÉE
    // puis une carte MUETTE et vérifiait que le compteur ne bougeait que pour
    // la seconde. Les témoins muets successifs (`Power Plant`, `Interns`,
    // `Work Crews`, `Ore Leaching`, puis les cartes à badge JOKER) ont tous été
    // encodés par le lot suivant ; il n'en reste AUCUN. Le test ne peut plus
    // exercer le second sens sans fabriquer une carte que le jeu ne contient
    // pas — ce que la clause anti-shortcut interdit.
    //
    // Il devient donc l'épinglage le plus fort qui reste vrai, et il est plus
    // exigeant que celui qu'il remplace : poser une carte par le chemin réel ne
    // compte JAMAIS de pouvoir sauté, parce qu'il n'existe plus une seule carte
    // des deux boîtes dont le texte imprimé ne soit pas appliqué.
    let db = db_de("base,decouverte");
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 77, &mut pol);

    let encodee = db.resolve_card("Comet").expect("Comet");
    let joker = db.resolve_card("Local Market").expect("Local Market");
    assert!(db.projects[encodee as usize].effect.is_some());
    // La carte à badge joker, dernier témoin muet en date, est encodée elle aussi.
    assert!(db.projects[joker as usize].effect.is_some());
    assert!(
        db.projects.iter().all(|c| !c.in_deck || c.effets_geres()),
        "plus aucune carte de la pioche ne doit être muette"
    );

    game.players[0].mc = 1000;
    game.players[0].hand.clear();
    game.players[0].hand.push(encodee);
    let avant = game.cards_effects_unhandled;
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(game.cards_effects_unhandled, avant, "carte encodée : rien à compter");

    // La pose de Comet a pu faire piocher (bonus de tuile océan) : on repart
    // d'une main ne contenant QUE la carte visée, sinon l'indice 0 désignerait
    // la carte piochée.
    game.players[0].hand.clear();
    game.players[0].mc = 1000;
    game.players[0].hand.push(joker);
    build_card(&mut game, &db, 0, 0, 0);
    assert_eq!(
        game.cards_effects_unhandled, avant,
        "carte à badge joker : encodée, donc rien à compter non plus"
    );
}

#[test]
fn le_compteur_est_nul_dans_les_deux_configurations_de_boites() {
    // Propriété d'origine : ajouter Découverte, dont aucun pouvoir n'était
    // implémenté, augmentait le nombre de pouvoirs sautés.
    //
    // (lot cartes-8) La moitié « base » avait déjà été RETOURNÉE : 200 parties
    // entières n'y sautaient plus aucun pouvoir.
    //
    // (jokers-corpos) La seconde moitié l'est à son tour. Le compteur mesuré en
    // PARTIE RÉELLE — oracle disjoint du recensement — vaut zéro dans les deux
    // configurations : tout le contenu imprimé des deux boîtes est appliqué.
    // C'est le résultat du chantier, et c'est un épinglage plus strict que
    // l'inégalité qu'il remplace.
    let mut pol = RandomPolicy;
    let base = engine::sim::run_simulation(&db_de("base"), 200, 2024, &mut pol);
    let mut pol = RandomPolicy;
    let disc = engine::sim::run_simulation(&db_de("base,decouverte"), 200, 2024, &mut pol);
    assert_eq!(
        base.cards_effects_unhandled, 0,
        "boîte de base : plus un seul pouvoir sauté en partie réelle"
    );
    assert_eq!(
        disc.cards_effects_unhandled, 0,
        "base + Découverte : plus un seul pouvoir sauté en partie réelle"
    );
}

#[test]
fn changer_de_boites_change_reellement_les_parties() {
    let mut pol = RandomPolicy;
    let a = engine::sim::run_simulation(&db_de("base"), 200, 7, &mut pol);
    let mut pol = RandomPolicy;
    let b = engine::sim::run_simulation(&db_de("base,decouverte"), 200, 7, &mut pol);
    assert_ne!(a.state_hash, b.state_hash, "--boites serait inerte");
    // …et reste déterministe à configuration constante.
    let mut pol = RandomPolicy;
    let a2 = engine::sim::run_simulation(&db_de("base"), 200, 7, &mut pol);
    assert_eq!(a.state_hash, a2.state_hash);
}

#[test]
fn plus_aucune_carte_encodee_ne_voit_un_de_ses_effets_saute() {
    // TÉMOIN RETOURNÉ par le chantier `decouverte-phases`. Il disait :
    // « une carte encodée mais dont un effet est SAUTÉ n'est pas déclarée
    // gérée » — le moteur sautait les améliorations de carte Phase et les
    // comptait dans `phase_upgrades_skipped`, et deux cartes de Découverte en
    // portaient une. Le mécanisme existe désormais : plus AUCUN effet encodé
    // n'est sauté, et le critère I4 se vérifie dans l'autre sens.
    let db = db_de("base,decouverte");
    for c in db.projects.iter() {
        let Some(e) = c.effect else { continue };
        assert!(
            engine::cards::encodage_integral(e),
            "{} : un effet encodé est encore sauté par le moteur",
            c.name
        );
        assert!(c.effets_geres(), "{} : encodage intégral déclaré non géré", c.name);
    }
    // Et le compteur d'audit le dit sur des parties RÉELLES : plus rien n'est
    // sauté, PARCE QUE les améliorations sont réellement accordées (la garde
    // anti-débranchement : `skipped == 0` ET `granted > 0`).
    let mut pol = RandomPolicy;
    let s = engine::sim::run_simulation(&db, 300, 2024, &mut pol);
    assert_eq!(s.phase_upgrades_skipped, 0, "plus aucune amélioration n'est sautée");
    assert!(s.phase_upgrades_granted > 0, "des améliorations sont réellement accordées");
}

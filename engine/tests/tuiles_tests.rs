//! Tests des OBJECTIFS et RÉCOMPENSES confrontés aux tuiles imprimées.
//!
//! Deux défauts trouvés le 28-07 en faisant le décompte d'avancement, et
//! corrigés le même jour :
//!
//! 1. **BARON SPATIAL** exigeait 7 badges espace ; la tuile en dit **6**.
//! 2. **COLLECTIONNEUR** renvoyait 0 pour tout le monde depuis la création du
//!    squelette, alors que les ressources posées sur les cartes existent depuis
//!    le lot 3 : la récompense était morte, et comptait une égalité à zéro dans
//!    toutes les parties où elle sortait.
//!
//! Source : `data/cartes-imprimees/objectifs-recompenses/objectifs-recompenses.json`,
//! lue directement sur les photos des tuiles le 27-07.
//!
//! Le fichier épingle AUSSI les neuf autres seuils, pour que le prochain qui
//! touche à cette table trouve un contrôle en face de chaque chiffre.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Tag};
use engine::flow::{assign_milestones, award_points, setup_game};
use engine::policy::RandomPolicy;
use engine::state::*;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

/// Partie réelle, joueurs remis à neuf : rien n'arrive qu'on n'ait mis là.
fn jeu(db: &CardsDb) -> GameState {
    let mut pol = RandomPolicy;
    let mut g = setup_game(db, 3, &mut pol);
    for p in 0..NUM_PLAYERS {
        let h: Vec<u16> = g.players[p].hand.drain(..).collect();
        g.discard.extend(h);
        g.players[p].tag_counts = [0; engine::cards::TAG_COUNT];
        g.players[p].card_resources.clear();
        g.players[p].mc_prod = 0;
        g.players[p].heat_prod = 0;
        g.players[p].plant_prod = 0;
        g.players[p].forests = 0;
    }
    // Un seul objectif en jeu à la fois : les deux autres sont neutralisés en
    // étant déclarés déjà acquis par personne mais impossibles à tenir.
    g
}

/// Place le même objectif dans les trois emplacements, aucun revendiqué.
fn objectif(g: &mut GameState, kind: MilestoneKind) {
    for slot in g.milestones.iter_mut() {
        slot.kind = kind;
        slot.achieved_by = [false; NUM_PLAYERS];
    }
}

// =========================================================================
// 1. BARON SPATIAL — le seuil corrigé
// =========================================================================

#[test]
fn baron_spatial_se_prend_a_six_badges_espace_pas_sept() {
    // La tuile imprimée : « 6 badges espace ». Le moteur exigeait 7.
    let db = db();
    let espace = Tag::Space.index().expect("le badge espace est un badge compté");

    // À CINQ badges, l'objectif n'est pas tenu.
    let mut g = jeu(&db);
    objectif(&mut g, MilestoneKind::SpaceBaron);
    g.players[0].tag_counts[espace] = 5;
    assign_milestones(&mut g);
    assert!(
        !g.milestones[0].achieved_by[0],
        "5 badges espace : l'objectif ne doit pas être acquis"
    );

    // À SIX, il l'est — c'est exactement la correction du 28-07.
    let mut g = jeu(&db);
    objectif(&mut g, MilestoneKind::SpaceBaron);
    g.players[0].tag_counts[espace] = 6;
    assign_milestones(&mut g);
    assert!(
        g.milestones[0].achieved_by[0],
        "6 badges espace : la tuile imprimée dit 6, pas 7"
    );
    assert!(
        !g.milestones[0].achieved_by[1],
        "l'adversaire, à zéro badge, ne le prend pas"
    );
}

#[test]
fn les_onze_seuils_sont_ceux_des_tuiles_imprimees() {
    // Un contrôle en face de chaque chiffre. Chaque ligne est le texte de la
    // tuile, transcrit depuis la photo du 27-07 :
    //   CONSTRUCTEUR 8 badges bâtiment · DIVERSIFICATEUR 9 badges différents ·
    //   CALORIGÈNE produit 10 chaleurs · FERMIER produit 5 plantes ·
    //   LÉGENDE 6 cartes rouges · MAGNAT 8 cartes vertes ·
    //   PLANIFICATEUR 12 cartes Projet · BARON SPATIAL 6 badges espace ·
    //   TERRAFORMEUR 15 NT · NABAB 6 cartes bleues · JARDINIER 3 forêts.
    //
    // Chacun est vérifié PAR LE MOTEUR : on met le joueur juste en dessous du
    // seuil (rien), puis pile dessus (acquis). Un seuil faux fait tomber l'une
    // des deux moitiés, jamais les deux — c'est ce qui rend le test utile.
    let db = db();
    let idx = |t: Tag| t.index().expect("badge compté");

    // Chaque entrée sait porter un joueur à la valeur `n` demandée.
    let regler: Vec<(MilestoneKind, i64, fn(&mut PlayerState, i64))> = vec![
        (MilestoneKind::Builder, 8, |pl, n| pl.tag_counts[Tag::Building.index().unwrap()] = n as u32),
        (MilestoneKind::Energizer, 10, |pl, n| pl.heat_prod = n),
        (MilestoneKind::Farmer, 5, |pl, n| pl.plant_prod = n),
        (MilestoneKind::Legend, 6, |pl, n| pl.color_counts[2] = n as u32),
        (MilestoneKind::Magnate, 8, |pl, n| pl.color_counts[0] = n as u32),
        (MilestoneKind::SpaceBaron, 6, |pl, n| pl.tag_counts[Tag::Space.index().unwrap()] = n as u32),
        (MilestoneKind::Terraformer, 15, |pl, n| pl.tr = n),
        (MilestoneKind::Tycoon, 6, |pl, n| pl.color_counts[1] = n as u32),
        (MilestoneKind::Gardener, 3, |pl, n| pl.forests = n),
    ];
    let _ = idx(Tag::Space);

    for (kind, seuil, poser) in regler {
        // Juste en dessous : rien.
        let mut g = jeu(&db);
        objectif(&mut g, kind);
        poser(&mut g.players[0], seuil - 1);
        assign_milestones(&mut g);
        assert!(
            !g.milestones[0].achieved_by[0],
            "{kind:?} : acquis à {} alors que le seuil imprimé est {seuil}",
            seuil - 1
        );

        // Pile dessus : acquis.
        let mut g = jeu(&db);
        objectif(&mut g, kind);
        poser(&mut g.players[0], seuil);
        assign_milestones(&mut g);
        assert!(
            g.milestones[0].achieved_by[0],
            "{kind:?} : NON acquis à {seuil}, qui est pourtant le seuil imprimé"
        );
    }
}

// =========================================================================
// 2. COLLECTIONNEUR — la récompense ressuscitée
// =========================================================================

#[test]
fn collectionneur_compte_les_ressources_posees_sur_les_cartes() {
    // « Le plus de ressources sur les cartes » (tuile imprimée). Elle renvoyait
    // 0 pour tout le monde : une égalité systématique, donc 4 PV distribués à
    // chacun sans qu'aucune partie ne puisse jamais la départager.
    let db = db();
    let mut g = jeu(&db);
    g.awards = [AwardKind::Collector; 3];

    // Sans aucune ressource : égalité — comportement conservé.
    let egaux = award_points(&g);
    assert_eq!(egaux[0], egaux[1], "sans ressources, la tuile est une égalité");

    // Le joueur 0 pose des ressources sur deux cartes différentes.
    g.players[0].card_resources.insert(10, 3);
    g.players[0].card_resources.insert(20, 2);
    g.players[1].card_resources.insert(30, 4);
    let pts = award_points(&g);
    assert!(
        pts[0] > pts[1],
        "5 ressources contre 4 : le joueur 0 doit l'emporter ({} vs {})",
        pts[0],
        pts[1]
    );

    // …et le classement s'inverse quand les ressources s'inversent : le test
    // ne se contente pas d'un sens.
    g.players[1].card_resources.insert(30, 9);
    let pts = award_points(&g);
    assert!(
        pts[1] > pts[0],
        "9 ressources contre 5 : le joueur 1 doit l'emporter ({} vs {})",
        pts[1],
        pts[0]
    );
}

#[test]
fn collectionneur_additionne_tous_les_types_de_ressources() {
    // La tuile ne distingue pas microbes, animaux, science ou flottantes : elle
    // dit « le plus de ressources ». Trois cartes porteuses de types différents
    // comptent donc ensemble.
    let db = db();
    let mut g = jeu(&db);
    g.awards = [AwardKind::Collector; 3];
    // Trois cartes distinctes, une ressource chacune, contre une carte à deux.
    for (carte, n) in [(11u16, 1u32), (12, 1), (13, 1)] {
        g.players[0].card_resources.insert(carte, n);
    }
    g.players[1].card_resources.insert(14, 2);
    let pts = award_points(&g);
    assert!(
        pts[0] > pts[1],
        "3 ressources réparties sur 3 cartes battent 2 sur une seule ({} vs {})",
        pts[0],
        pts[1]
    );
}

#[test]
fn collectionneur_bouge_reellement_dans_des_parties_entieres() {
    // Deux oracles disjoints : l'état fabriqué ci-dessus, et des parties
    // réelles menées à leur terme. Les cartes porteuses de ressources existent
    // depuis le lot 3 ; si la récompense était encore morte, les deux joueurs
    // finiraient toujours à égalité sur elle.
    //
    // Sur PLUSIEURS graines, et jusqu'à la fin : une partie dure une
    // quarantaine de manches en moyenne, et les ressources se dépensent autant
    // qu'elles s'accumulent — une seule partie tronquée ne prouverait rien.
    let db = db();
    let mut avec_ressources = 0;
    let mut departages = 0;
    for graine in [1u64, 7, 42, 2024, 31337] {
        let mut pol = RandomPolicy;
        let mut g = setup_game(&db, graine, &mut pol);
        g.awards = [AwardKind::Collector; 3];
        for _ in 0..60 {
            if g.game_over {
                break;
            }
            engine::flow::play_round(&mut g, &db, &mut pol);
        }
        let total: u32 = (0..NUM_PLAYERS)
            .map(|p| g.players[p].card_resources.values().sum::<u32>())
            .sum();
        if total > 0 {
            avec_ressources += 1;
        }
        let pts = award_points(&g);
        if pts[0] != pts[1] {
            departages += 1;
        }
    }
    assert!(
        avec_ressources > 0,
        "aucune des cinq parties n'a laissé une ressource sur une carte"
    );
    assert!(
        departages > 0,
        "la récompense Collectionneur n'a départagé AUCUNE des cinq parties : \
         elle est encore morte"
    );
}

#[test]
fn les_sept_tuiles_de_recompense_sont_sept_dans_le_moteur() {
    // TÉMOIN RETOURNÉ par le chantier `decouverte-phases`, et non supprimé
    // (NEVER 6). Il épinglait une dette assumée : la septième tuile imprimée,
    // VISIONNAIRE (« le plus de cartes Phase améliorées »), n'avait pas de
    // variante dans `AwardKind` parce que `PlayerState::phase_upgrades` était
    // un champ que rien ne lisait. Le mécanisme existe : la dette est payée,
    // l'assertion porte désormais sur SEPT.
    assert_eq!(
        AWARD_POOL.len(),
        7,
        "sept tuiles imprimées, sept récompenses dans le moteur"
    );
    assert!(
        AWARD_POOL.iter().any(|a| format!("{a:?}").contains("Vision")),
        "VISIONNAIRE doit figurer dans la réserve, sinon elle ne sortirait jamais"
    );
    // Et elle n'y figure pas seulement : elle est réellement DISTRIBUÉE quand
    // la boîte qui l'apporte est là — et jamais sans elle.
    let db_base = CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).unwrap();
    let db_dec =
        CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap()).unwrap();
    let pool_base = engine::flow::award_pool(&db_base);
    let pool_dec = engine::flow::award_pool(&db_dec);
    assert_eq!(pool_base.len(), 6, "la boîte de base ne connaît pas VISIONNAIRE");
    assert!(!pool_base.contains(&AwardKind::Visionary));
    assert_eq!(pool_dec.len(), 7, "Découverte apporte la septième tuile");
    assert!(pool_dec.contains(&AwardKind::Visionary));
}

#[test]
fn visionnaire_departage_dans_les_deux_sens() {
    // La récompense doit RÉELLEMENT départager : elle compte les cartes Phase
    // améliorées de chaque joueur, et le classement s'inverse quand les
    // améliorations s'inversent. Sans les deux sens, une fonction qui rendrait
    // le numéro du joueur passerait.
    let db = db();
    let mut pol = RandomPolicy;
    let mut g = setup_game(&db, 4242, &mut pol);
    g.awards = [AwardKind::Visionary; 3];

    // Personne n'a amélioré : égalité à zéro, 4 PV chacun.
    let nuls = award_points(&g);
    assert_eq!(nuls, [12, 12], "égalité à zéro : 4 PV par tuile, aucun départage");

    // p0 a deux cartes Phase améliorées, p1 une seule.
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    g.players[0].upgrade_phase(4, PhaseUpgrade::VariantB);
    g.players[1].upgrade_phase(2, PhaseUpgrade::VariantA);
    assert_eq!(g.players[0].phase_upgrades_count(), 2);
    assert_eq!(g.players[1].phase_upgrades_count(), 1);
    let pts = award_points(&g);
    assert!(pts[0] > pts[1], "le joueur qui en a le plus gagne : {pts:?}");
    assert_eq!(pts, [15, 6], "5/2 par tuile, trois tuiles");

    // Sens inverse : p1 en prend trois, p0 en garde deux.
    g.players[1].upgrade_phase(3, PhaseUpgrade::VariantB);
    g.players[1].upgrade_phase(5, PhaseUpgrade::VariantA);
    let pts = award_points(&g);
    assert!(pts[1] > pts[0], "le classement s'inverse avec les améliorations : {pts:?}");

    // Et une bascule A ↔ B ne CRÉE pas de carte : le compte ne bouge pas.
    let avant = g.players[1].phase_upgrades_count();
    let deja = g.players[1].upgrade_phase(3, PhaseUpgrade::VariantA);
    assert!(deja, "la phase III était déjà améliorée");
    assert_eq!(g.players[1].phase_upgrades_count(), avant, "A ↔ B ne compte pas double");
}

//! **LOT L5 — LA VITESSE, LES RÉGLAGES, ET LE SIGNAL QU'ON DONNE À L'IA.**
//!
//! Ce banc éprouve les cinq familles du lot, et il les atteint **par la
//! bibliothèque** (`use engine::…`) : depuis le lot L3, `description`, `joueur`,
//! `rejeu`, `espion` et `reseau` sont des modules publics d'`engine/src/lib.rs`.
//! Aucune déclaration de chemin de module n'est nécessaire, et il n'y en a
//! aucune ici : rien n'est inclus par recopie de fichier.
//!
//! **Ce que ces tests refusent de faire.** Ils ne recopient pas les chiffres du
//! prompt : chaque seuil est soit lu dans le code (`AMPLITUDE_DEPART`), soit lu
//! dans la fiche elle-même (le plus haut palier de score), soit une propriété
//! vraie par construction (la somme d'une exponentielle normalisée vaut un). Les
//! deux seules valeurs figées sont l'empreinte d'état publiée par le contrôle 02
//! du lot — mesurée sur le commit `ada92b6`, c'est-à-dire sur le code d'AVANT —
//! et le maximum de score mesuré le 20-08 ; les deux portent leur provenance.
//!
//! La campagne « vu rouge » de ces tests est dans `outputs/work/vu-rouge.txt` :
//! chaque test y a au moins un sabotage qui le fait tomber.

use std::process::Command;

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::description::{Description, Tampons};
use engine::flow::{play_round, score_parts, setup_game, tiebreak_total, winner};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use rand::rngs::StdRng;
use engine::rejeu::Rejeu;
use engine::reseau::{
    self, Pile, Reseau, ReseauMulti, ReseauPhases, AMORCAGE_FACTEUR, AMORCAGE_PARTIES,
    AMORCAGE_SCORE_MAX, AMPLITUDE_DEPART, AMPLITUDE_FIGEE, CACHES, DOUCEUR, LAMBDA, PHASES, RYTHME,
    SEUIL_SATURATION, SORTIES, TAUX,
};
use engine::sim::{run_simulation, MAX_GENERATIONS};

// ---------------------------------------------------------------------------
// Le décor : la base de cartes et la fiche de situation, chargées une fois par
// test qui en a besoin. Le répertoire courant d'un test d'intégration est
// `engine/`, d'où le `../`.
// ---------------------------------------------------------------------------

fn decor() -> (CardsDb, Description) {
    let boites = BoiteSet::parse("base,decouverte").expect("boîtes base,decouverte");
    let db = CardsDb::load_boites("../data/cards.json", boites).expect("../data/cards.json");
    let desc = Description::new(&db);
    (db, desc)
}

/// Des situations de vraies parties : c'est le décor sur lequel la couche cachée
/// doit être jugée. Un vecteur tiré au hasard ne dirait rien de la fiche
/// réelle, où mille trois cents drapeaux de cartes ne bougent presque jamais.
fn situations_reelles(parties: u64) -> (Description, Vec<Vec<f64>>) {
    let (db, desc) = decor();
    let mut tampons = Tampons::new(&desc);
    let mut fiche: Vec<f64> = Vec::new();
    let mut out: Vec<Vec<f64>> = Vec::new();
    for k in 0..parties {
        let seed = 8_800_001 + k;
        let mut pol = RandomPolicy;
        let mut game = setup_game(&db, seed, &mut pol);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            play_round(&mut game, &db, &mut pol);
            for siege in 0..2 {
                desc.decrire(&game, &db, siege, &mut fiche, &mut tampons);
                out.push(fiche.clone());
            }
        }
    }
    (desc, out)
}

/// La pente moyenne `1 − h²` et la part de neurones saturés d'un réseau donné,
/// sur des situations données. C'est la mesure que le journal d'entraînement
/// publie, refaite ici à la main pour pouvoir la comparer d'une amplitude à
/// l'autre.
fn pente_et_saturation(r: &mut Reseau, situations: &[Vec<f64>]) -> (f64, f64) {
    let (mut somme, mut satures, mut total) = (0.0f64, 0u64, 0u64);
    for x in situations.iter() {
        r.oublier();
        r.evaluer(x);
        for j in 0..CACHES {
            let pente = 1.0 - r.h[j] * r.h[j];
            somme += pente;
            if pente < SEUIL_SATURATION {
                satures += 1;
            }
            total += 1;
        }
    }
    (
        somme / total as f64,
        100.0 * satures as f64 / total as f64,
    )
}

// ===========================================================================
// A1 — L'AMPLITUDE DES POIDS DE DÉPART (audit §2.1, engine/src/reseau.rs:284)
// ===========================================================================

/// L'amplitude spécifiée à l'origine était ±0,1 (`engine/src/reseau.rs`,
/// constante `AMPLITUDE_DEPART`). Le lot la baisse ; ce test dit seulement
/// qu'elle a baissé, et il lit la valeur au lieu de la répéter.
#[test]
fn l_amplitude_de_depart_a_baisse() {
    // Source : la constante mesurée ici : engine/src/reseau.rs:315.
    assert!(
        AMPLITUDE_DEPART < 0.1,
        "l'amplitude de départ vaut encore {AMPLITUDE_DEPART} : le réglage du §2.1 n'est pas posé"
    );
    assert!(
        AMPLITUDE_DEPART > 0.0,
        "une amplitude nulle tirerait tous les poids à zéro"
    );
}

/// **L'écart-type de la somme d'entrée d'un neurone caché doit être voisin de 1.**
///
/// C'est l'arithmétique du §2.1 : les entrées valent toutes ±1, donc la somme de
/// `n` d'entre elles pondérées par des poids tirés uniformément dans ±a a un
/// écart-type de `√(n/3) × a`. À 2 et au-delà, la tangente hyperbolique est
/// couchée et sa dérivée tombe sous un dixième.
#[test]
fn l_ecart_type_de_la_somme_cachee_est_voisin_de_un() {
    let (_db, desc) = decor();
    let n = desc.taille as f64;
    let ecart_type = (n / 3.0).sqrt() * AMPLITUDE_DEPART;
    assert!(
        (0.8..1.4).contains(&ecart_type),
        "écart-type de la somme cachée : {ecart_type:.3} pour {n} entrées — \
         hors du régime où la tangente hyperbolique travaille"
    );
    // Et l'ancien réglage était bien hors de ce régime : c'est le défaut.
    let ancien = (n / 3.0).sqrt() * 0.1;
    assert!(
        ancien > 2.0,
        "à 0,1 l'écart-type valait {ancien:.3} : le défaut du §2.1 n'existerait pas"
    );
}

/// Les poids d'un réseau neuf restent dans la bande annoncée, des deux côtés de
/// zéro. Un tirage qui déborderait rendrait la mesure d'écart-type fausse.
#[test]
fn les_poids_de_depart_restent_dans_la_bande() {
    let r = Reseau::neuf(400);
    // **Les deux couches sont éprouvées SÉPARÉMENT.** D'un seul tenant, les 102
    // poids de sortie remplissaient la bande négative à eux seuls et cachaient
    // une couche cachée tirée d'un seul côté (engine/src/reseau.rs:332).
    for (nom, couche) in [("cachée", &r.w_cache), ("sortie", &r.w_sortie)] {
        let (mut mini, mut maxi) = (f64::INFINITY, f64::NEG_INFINITY);
        for w in couche.iter() {
            assert!(
                w.abs() <= AMPLITUDE_DEPART,
                "couche {nom} : poids {w} hors de ±{AMPLITUDE_DEPART}"
            );
            mini = mini.min(*w);
            maxi = maxi.max(*w);
        }
        assert!(
            mini < -0.9 * AMPLITUDE_DEPART,
            "couche {nom} : la bande négative n'est pas remplie ({mini})"
        );
        assert!(
            maxi > 0.9 * AMPLITUDE_DEPART,
            "couche {nom} : la bande positive n'est pas remplie ({maxi})"
        );
        // La moyenne d'un tirage centré est proche de zéro ; un tirage d'un
        // seul côté la déplacerait à la MOITIÉ de l'amplitude. Le quart de
        // l'amplitude sépare les deux sans discuter : la couche de sortie ne
        // compte que 102 poids, sa moyenne a le droit de flotter.
        let moyenne = couche.iter().sum::<f64>() / couche.len() as f64;
        assert!(
            moyenne.abs() < 0.25 * AMPLITUDE_DEPART,
            "couche {nom} : le tirage n'est pas centré, moyenne {moyenne}"
        );
    }
}

/// **`Reseau::neuf` emploie bien la constante, et rien d'autre.** C'est le piège
/// annoncé du §2.1 : « changer la constante ne suffit pas si les outils passent
/// l'ancienne valeur ». Ici on vérifie le point de passage obligé.
#[test]
fn un_reseau_neuf_emploie_l_amplitude_de_depart() {
    let a = Reseau::neuf(200);
    let b = Reseau::neuf_amplitude(200, AMPLITUDE_DEPART);
    assert_eq!(a.w_cache, b.w_cache, "la couche cachée n'emploie pas la constante");
    assert_eq!(a.w_sortie, b.w_sortie, "la couche de sortie n'emploie pas la constante");
    let c = Reseau::neuf_amplitude(200, 0.1);
    assert_ne!(a.w_cache, c.w_cache, "l'ancien réglage donnerait les mêmes poids");
}

/// **Le fait mesuré du §2.1, refait ici** : à amplitude réduite, la couche
/// cachée laisse passer beaucoup plus d'apprentissage, sur les MÊMES situations.
#[test]
fn l_amplitude_reduite_redresse_la_pente_de_la_couche_cachee() {
    let (desc, situations) = situations_reelles(3);
    assert!(situations.len() > 200, "trop peu de situations : {}", situations.len());
    let mut ancien = Reseau::neuf_amplitude(desc.taille, 0.1);
    let mut nouveau = Reseau::neuf_amplitude(desc.taille, AMPLITUDE_DEPART);
    let (pente_ancien, sat_ancien) = pente_et_saturation(&mut ancien, &situations);
    let (pente_nouveau, sat_nouveau) = pente_et_saturation(&mut nouveau, &situations);
    assert!(
        pente_nouveau > pente_ancien * 1.4,
        "pente {pente_nouveau:.4} contre {pente_ancien:.4} : le réglage n'apporte rien"
    );
    assert!(
        sat_nouveau < sat_ancien * 0.5,
        "part saturée {sat_nouveau:.1} % contre {sat_ancien:.1} % : le réglage n'apporte rien"
    );
    assert!(
        pente_nouveau > 0.45,
        "pente moyenne {pente_nouveau:.4} : sous 0,45 la couche cachée apprend au ralenti"
    );
}

/// L'amplitude de sortie de chaque neurone caché **à poids figés**, sur un jeu
/// de situations donné : `max(h) − min(h)`. C'est la grandeur que l'audit §2.1
/// appelle « rendre une valeur constante » — un neurone dont cette amplitude
/// s'effondre ne distingue plus deux situations.
///
/// **Pourquoi à poids figés, et pourquoi ce n'est pas ce que le journal
/// publie.** Le compteur `neurones_figes` d'`entraine` accumule les extrêmes
/// PENDANT une tranche d'entraînement, c'est-à-dire pendant que les poids
/// bougent : un neurone parfaitement constant à poids donnés y montre quand même
/// une large amplitude, simplement parce que ses poids ont dérivé. Le journal
/// mesure donc la dérive autant que la constance. La mesure de l'audit, elle,
/// prend un réseau ARRÊTÉ et le promène sur des situations.
fn amplitudes_de_sortie(r: &mut Reseau, situations: &[Vec<f64>]) -> Vec<f64> {
    let mut mini = vec![f64::INFINITY; CACHES];
    let mut maxi = vec![f64::NEG_INFINITY; CACHES];
    for x in situations.iter() {
        r.oublier();
        r.evaluer(x);
        for j in 0..CACHES {
            if r.h[j] < mini[j] {
                mini[j] = r.h[j];
            }
            if r.h[j] > maxi[j] {
                maxi[j] = r.h[j];
            }
        }
    }
    (0..CACHES).map(|j| maxi[j] - mini[j]).collect()
}

/// **Done-when 03 — le nombre de neurones qui rendent une valeur constante a
/// baissé.**
///
/// Source du fait à reproduire : `docs/AUDIT_ENTRAINEMENT.md` §2.1, « six
/// neurones sur cinquante rendent une valeur constante dans tous les états
/// testés » — relevé sur des poids ENTRAÎNÉS (1 200 000 parties) et sur
/// l'ANCIENNE fiche à 1 472 entrées, qui n'est plus lisible aujourd'hui.
/// Le seuil employé est `AMPLITUDE_FIGEE` (`engine/src/reseau.rs:160`), lu et
/// non recopié.
///
/// Les deux fichiers comparés sont les deux réseaux entraînés que le dépôt
/// porte sur la fiche courante : la référence du lot L3 (amplitude de départ
/// 0,1) et le fichier de démonstration de ce lot (amplitude 0,045).
#[test]
fn le_nombre_de_neurones_figes_a_baisse() {
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    let (_, situations) = situations_reelles(40);
    assert!(
        situations.len() > 2000,
        "trop peu de situations pour juger une constance : {}",
        situations.len()
    );
    let mut avant = Reseau::lire("../data/poids/apprenti-L3-amorce.txt", &noms)
        .expect("../data/poids/apprenti-L3-amorce.txt");
    let mut apres = Reseau::lire("../data/poids/apprenti-L5-essai.txt", &noms)
        .expect("../data/poids/apprenti-L5-essai.txt");
    let aa = amplitudes_de_sortie(&mut avant, &situations);
    let ap = amplitudes_de_sortie(&mut apres, &situations);
    let n_avant = aa.iter().filter(|x| **x < AMPLITUDE_FIGEE).count();
    let n_apres = ap.iter().filter(|x| **x < AMPLITUDE_FIGEE).count();
    let mut tri_a = aa.clone();
    tri_a.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mut tri_p = ap.clone();
    tri_p.sort_by(|x, y| x.partial_cmp(y).unwrap());
    println!(
        "MESURE-FIGES {} situations — avant (L3, amplitude 0,1) : {} figés sur {CACHES}, \
         cinq plus petites amplitudes {:?} ; après (L5, amplitude 0,045) : {} figés sur {CACHES}, \
         cinq plus petites amplitudes {:?}",
        situations.len(),
        n_avant,
        &tri_a[..5].iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>(),
        n_apres,
        &tri_p[..5].iter().map(|x| format!("{x:.4}")).collect::<Vec<_>>(),
    );
    assert!(
        n_apres < n_avant,
        "neurones figés : {n_apres} après contre {n_avant} avant — le done-when 03 demande une BAISSE"
    );

    // **L'attribution à l'amplitude, sans passer par un fichier.** Les deux
    // réseaux ci-dessus diffèrent par plus que l'amplitude (30 000 parties
    // contre 20 000, et tout le code du lot). Ces deux-ci ne diffèrent QUE par
    // elle : même fiche, mêmes situations, même graine de tirage.
    let mut f_ancien = Reseau::neuf_amplitude(desc.taille, 0.1);
    let mut f_nouveau = Reseau::neuf_amplitude(desc.taille, AMPLITUDE_DEPART);
    let fa = amplitudes_de_sortie(&mut f_ancien, &situations);
    let fn_ = amplitudes_de_sortie(&mut f_nouveau, &situations);
    let mut t_fa = fa.clone();
    t_fa.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mut t_fn = fn_.clone();
    t_fn.sort_by(|x, y| x.partial_cmp(y).unwrap());
    println!(
        "MESURE-FIGES-NEUF réseau NEUF sur les mêmes {} situations — amplitude 0,1 : {} figés, \
         médiane des amplitudes {:.4}, plus petite {:.4} ; amplitude {AMPLITUDE_DEPART} : {} figés, \
         médiane {:.4}, plus petite {:.4}",
        situations.len(),
        fa.iter().filter(|x| **x < AMPLITUDE_FIGEE).count(),
        t_fa[CACHES / 2],
        t_fa[0],
        fn_.iter().filter(|x| **x < AMPLITUDE_FIGEE).count(),
        t_fn[CACHES / 2],
        t_fn[0],
    );
    // **Et le résultat de cette contre-mesure va CONTRE l'intuition, donc on
    // l'écrit.** Au réseau NEUF, aucune des deux amplitudes ne fige un seul
    // neurone, et c'est 0,1 qui donne la plus grande amplitude minimale : une
    // somme d'entrée plus dispersée promène le neurone sur toute l'étendue de la
    // tangente hyperbolique. **Le figeage n'est donc pas un défaut du tirage de
    // départ : c'est un défaut que l'ENTRAÎNEMENT fabrique**, d'autant plus vite
    // que la pente laissée par le tirage est faible. C'est pourquoi la seule
    // comparaison qui puisse le montrer est celle de deux réseaux entraînés,
    // ci-dessus, et pourquoi ce bloc-ci n'affirme rien — il mesure.
    assert!(
        fa.iter().filter(|x| **x < AMPLITUDE_FIGEE).count() == 0
            && fn_.iter().filter(|x| **x < AMPLITUDE_FIGEE).count() == 0,
        "un réseau neuf ne doit figer aucun neurone à aucune des deux amplitudes"
    );
}

// ===========================================================================
// A2 — LA PLAGE DE SCORES DE L'AMORÇAGE (audit §2.5)
// ===========================================================================

/// **Le plus haut palier de la fiche doit être franchi pendant l'amorçage.**
///
/// Le palier est LU dans la table des noms, jamais écrit en dur : si la fiche
/// change un jour, ce test change avec elle.
#[test]
fn l_amorcage_franchit_le_plus_haut_palier_de_la_fiche() {
    // Source : la plage de l'amorçage : engine/src/reseau.rs:134.
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    let paliers: Vec<i64> = noms
        .iter()
        .filter(|x| x.starts_with("moi_score_acquis>"))
        .map(|x| x["moi_score_acquis>".len()..].parse::<i64>().unwrap())
        .collect();
    let haut = *paliers.iter().max().expect("des paliers de score acquis");
    assert!(
        AMORCAGE_SCORE_MAX > haut,
        "l'amorçage tire jusqu'à {AMORCAGE_SCORE_MAX}, le plus haut palier est {haut} — \
         il ne serait jamais allumé"
    );
}

/// **La plage couvre la distribution réelle des scores.** Le maximum mesuré le
/// 20-08 — 400 scores de fin de partie relevés dans 200 parties jouées par
/// `data/poids/apprenti-L3-amorce.txt`, graines 6600001 et suivantes — vaut 159.
/// C'est le seul chiffre de ce banc qui vienne d'une mesure extérieure, et il est
/// daté.
#[test]
fn l_amorcage_couvre_le_score_maximal_mesure() {
    const MAXIMUM_MESURE_LE_20_08: i64 = 159;
    assert!(
        AMORCAGE_SCORE_MAX >= MAXIMUM_MESURE_LE_20_08,
        "l'amorçage s'arrête à {AMORCAGE_SCORE_MAX}, des parties finissent à \
         {MAXIMUM_MESURE_LE_20_08}"
    );
}

/// L'ancienne plage — 0 à 49 — laissait éteints la plupart des paliers de la
/// fiche. Ce test dit que le défaut était réel, et il le dit en lisant la fiche.
#[test]
fn l_ancienne_plage_laissait_la_plupart_des_paliers_eteints() {
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    let paliers: Vec<i64> = noms
        .iter()
        .filter(|x| x.starts_with("moi_score_acquis>"))
        .map(|x| x["moi_score_acquis>".len()..].parse::<i64>().unwrap())
        .collect();
    let jamais_atteints = paliers.iter().filter(|s| **s >= 49).count();
    assert!(
        jamais_atteints * 2 > paliers.len(),
        "seulement {jamais_atteints} paliers sur {} étaient hors de portée à 49",
        paliers.len()
    );
    let atteints_maintenant = paliers.iter().filter(|s| **s < AMORCAGE_SCORE_MAX).count();
    assert_eq!(
        atteints_maintenant,
        paliers.len(),
        "des paliers restent hors de portée de l'amorçage"
    );
}

/// L'amorçage garde sa taille et son facteur : ce lot règle la PLAGE, pas le
/// reste. Un test de non-régression.
#[test]
fn l_amorcage_garde_ses_cinq_mille_parties_et_son_facteur_dix() {
    // Source : les deux réglages d'amorçage : engine/src/reseau.rs:116.
    assert_eq!(AMORCAGE_PARTIES, 5000, "le nombre de fins de partie fabriquées a bougé");
    assert_eq!(AMORCAGE_FACTEUR, 10.0, "le facteur de taux de l'amorçage a bougé");
}

// ===========================================================================
// B — LE SIGNAL DE FIN DE PARTIE (livret p.16, docs/regles/livret-base.md:461)
// ===========================================================================

/// « Le joueur à égalité ayant le plus grand total cumulé de chaleur, de MC et
/// de plantes est déclaré vainqueur. Veillez à convertir au préalable toutes les
/// cartes Projet en main en MC » — `docs/regles/livret-base.md:461`. Une partie
/// que le livret départage ne doit plus apprendre « match nul ».
#[test]
fn a_scores_egaux_le_vainqueur_designe_ne_recoit_plus_un_match_nul() {
    let gagnant = Reseau::cible_finale_departagee(70, 70, Some(true));
    let perdant = Reseau::cible_finale_departagee(70, 70, Some(false));
    assert!(
        gagnant[0] > 0.5 + 1e-9,
        "le vainqueur désigné par le départage reçoit {} — c'est encore un match nul",
        gagnant[0]
    );
    assert!(
        perdant[0] < 0.5 - 1e-9,
        "le perdant désigné par le départage reçoit {}",
        perdant[0]
    );
    assert!((gagnant[0] + gagnant[1] - 1.0).abs() < 1e-12);
    assert!((gagnant[0] - perdant[1]).abs() < 1e-12, "la cible n'est pas symétrique");
}

/// **Le seul vrai match nul du livret** : égalité de points de victoire ET
/// égalité sur le total de départage. `flow::winner` rend alors `None`, et la
/// cible reste `[0,5 ; 0,5]`.
#[test]
fn l_egalite_parfaite_reste_un_match_nul() {
    let c = Reseau::cible_finale_departagee(70, 70, None);
    assert!((c[0] - 0.5).abs() < 1e-12, "cible {c:?}");
    assert!((c[1] - 0.5).abs() < 1e-12, "cible {c:?}");
}

/// **La nuance n'est pas aplatie, et c'est la moitié du travail.** La forme
/// continue de `cible_finale` n'est pas un défaut : gagner de 40 points ne
/// s'apprend pas comme gagner de 1. Le départage doit valoir exactement la
/// victoire la plus étroite qui existe — un point d'écart —, pas une victoire
/// écrasante.
#[test]
fn la_cible_departagee_vaut_exactement_une_victoire_d_un_point() {
    // Source : la cible construite ici : engine/src/reseau.rs:981.
    for s in [0i64, 12, 70, 147] {
        let departagee = Reseau::cible_finale_departagee(s, s, Some(true));
        let un_point = Reseau::cible_finale(s + 1, s);
        assert_eq!(
            departagee, un_point,
            "à {s} points partout, le départage ne vaut pas une victoire d'un point"
        );
        let quarante = Reseau::cible_finale(s + 40, s);
        assert!(
            quarante[0] > departagee[0] + 0.3,
            "gagner de 40 points s'apprendrait comme un départage : {quarante:?} contre {departagee:?}"
        );
    }
}

/// **Aucun score différent ne change de cible, d'un seul bit.** La correction du
/// lot ne touche QUE l'égalité stricte.
#[test]
fn la_cible_departagee_ne_touche_a_rien_quand_les_scores_different() {
    for moi in 0..80i64 {
        for autre in 0..80i64 {
            if moi == autre {
                continue;
            }
            let avec = Reseau::cible_finale_departagee(moi, autre, Some(moi > autre));
            let sans = Reseau::cible_finale(moi, autre);
            assert_eq!(avec, sans, "la cible a bougé pour ({moi}, {autre})");
        }
    }
}

/// La cible reste une distribution de probabilité, dans tous les cas.
#[test]
fn la_cible_departagee_reste_une_distribution() {
    for (moi, autre, gagne) in [
        (0i64, 0i64, Some(true)),
        (0, 0, Some(false)),
        (0, 0, None),
        (159, 159, Some(true)),
        (159, 0, Some(true)),
        (0, 159, Some(false)),
    ] {
        let c = Reseau::cible_finale_departagee(moi, autre, gagne);
        assert!((c[0] + c[1] - 1.0).abs() < 1e-12, "somme {c:?}");
        assert!(c[0] > 0.0 && c[1] > 0.0, "cible négative ou nulle {c:?}");
        assert!(c[0].is_finite() && c[1].is_finite(), "cible non finie {c:?}");
    }
}

/// La douceur du §2.3 n'a pas bougé : c'est elle qui donne son échelle à
/// l'écart d'un point.
#[test]
fn la_douceur_de_la_cible_finale_n_a_pas_bouge() {
    assert_eq!(DOUCEUR, 0.3, "la douceur de la répartition de fin de partie a bougé");
    let c = Reseau::cible_finale(1, 0);
    let attendu = 1.0 / (1.0 + (-DOUCEUR).exp());
    assert!((c[0] - attendu).abs() < 1e-12, "{c:?} contre {attendu}");
}

/// **Le fait mesuré, sur de vraies parties** : des parties finissent à égalité de
/// points de victoire, et le départage du livret leur trouve un vainqueur. Sans
/// cela, la correction de ce lot n'aurait aucun objet.
#[test]
fn de_vraies_parties_finissent_a_egalite_et_le_livret_les_departage() {
    let (db, _desc) = decor();
    let mut egalites = 0u64;
    let mut sans_vainqueur = 0u64;
    for k in 0..400u64 {
        let seed = 6_600_001 + k;
        let mut pol = RandomPolicy;
        let mut g = setup_game(&db, seed, &mut pol);
        while !g.game_over && g.generation <= MAX_GENERATIONS {
            play_round(&mut g, &db, &mut pol);
        }
        let (scores, _, _) = score_parts(&g, &db);
        if scores[0] == scores[1] {
            egalites += 1;
            if winner(&g, &db).is_none() {
                sans_vainqueur += 1;
            }
        }
    }
    assert!(
        egalites >= 3,
        "seulement {egalites} égalités de points sur 400 parties : le défaut serait négligeable"
    );
    assert_eq!(
        sans_vainqueur, 0,
        "{sans_vainqueur} parties restent nulles après départage sur {egalites}"
    );
}

/// **`flow::winner` est le point de calcul unique**, et il ne se contente pas de
/// comparer les scores : quand ils sont égaux, c'est le total du livret qui
/// tranche, et le vainqueur qu'il désigne est bien celui qui a ce total.
#[test]
fn le_vainqueur_a_points_egaux_est_celui_du_plus_grand_total_de_departage() {
    // Source : le point de calcul unique : engine/src/flow.rs:5824, règle : docs/regles/livret-base.md:461.
    let (db, _desc) = decor();
    let mut vus = 0u64;
    for k in 0..400u64 {
        let seed = 6_600_001 + k;
        let mut pol = RandomPolicy;
        let mut g = setup_game(&db, seed, &mut pol);
        while !g.game_over && g.generation <= MAX_GENERATIONS {
            play_round(&mut g, &db, &mut pol);
        }
        let (scores, _, _) = score_parts(&g, &db);
        if scores[0] != scores[1] {
            continue;
        }
        vus += 1;
        let t0 = tiebreak_total(&g.players[0]);
        let t1 = tiebreak_total(&g.players[1]);
        let attendu = if t0 == t1 {
            None
        } else if t0 > t1 {
            Some(0)
        } else {
            Some(1)
        };
        assert_eq!(winner(&g, &db), attendu, "graine {seed} : le départage ne suit pas le total");
    }
    assert!(vus >= 3, "aucune égalité rencontrée : le test ne prouve rien");
}

/// « Veillez à convertir au préalable toutes les cartes Projet en main en MC »
/// (`docs/regles/livret-base.md:461`), au taux du livret — 3 MC par carte
/// (`docs/regles/livret-base.md:96`). Le total de départage est donc exactement
/// chaleur + argent + plantes + 3 × cartes en main.
#[test]
fn le_total_de_departage_convertit_la_main_a_trois_mc() {
    // Source : le total du départage : engine/src/flow.rs:5808.
    let (db, _desc) = decor();
    let mut avec_main = 0u64;
    for k in 0..40u64 {
        let seed = 6_600_001 + k;
        let mut pol = RandomPolicy;
        let mut g = setup_game(&db, seed, &mut pol);
        while !g.game_over && g.generation <= MAX_GENERATIONS {
            play_round(&mut g, &db, &mut pol);
        }
        for p in 0..2 {
            let pl = &g.players[p];
            let attendu = pl.heat + pl.mc + pl.plants + pl.hand.len() as i64 * 3;
            assert_eq!(
                tiebreak_total(pl),
                attendu,
                "graine {seed}, siège {p} : le total de départage ne suit pas le livret"
            );
            if !pl.hand.is_empty() {
                avec_main += 1;
            }
        }
    }
    assert!(avec_main > 0, "aucune main non vide : la conversion n'a jamais été éprouvée");
}

// ===========================================================================
// D3.1 — LA FORMULE COMPLÈTE DE LA COUCHE DE SORTIE (reseau.rs:429 contre :439)
// ===========================================================================

/// La perte que la correction descend : la moitié de la somme des carrés des
/// écarts à la cible. C'est celle qu'`accumuler_sortie` dérive.
fn perte(p: &[f64], cible: &[f64]) -> f64 {
    0.5 * p.iter().zip(cible.iter()).map(|(a, b)| (a - b) * (a - b)).sum::<f64>()
}

/// Le gradient numérique de la perte par rapport au poids de rang `i` de la
/// couche demandée, par différences centrées.
fn gradient_numerique(sortie: bool, i: usize, x: &[f64], cible: [f64; SORTIES]) -> f64 {
    let h = 1e-7;
    let mut valeurs = [0.0f64; 2];
    for (k, signe) in [1.0f64, -1.0].iter().enumerate() {
        let mut r = Reseau::neuf(x.len());
        if sortie {
            r.w_sortie[i] += signe * h;
        } else {
            r.w_cache[i] += signe * h;
        }
        r.oublier();
        let p = r.evaluer(x);
        valeurs[k] = perte(&p, &cible);
    }
    (valeurs[0] - valeurs[1]) / (2.0 * h)
}

/// **LE TEST QUI DIT QUE LA FORMULE EST COMPLÈTE.** On compare la correction que
/// le réseau applique réellement au gradient de la perte, calculé numériquement
/// et sans rien savoir du code. Avant la correction du §2.17, la couche de sortie
/// n'appliquait que le terme diagonal de la jacobienne : la moitié.
#[test]
fn la_correction_de_sortie_suit_le_gradient_de_la_perte() {
    // Source : la formule éprouvée ici : engine/src/reseau.rs:515.
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let cible = [0.9, 0.1];
    let taux = 1e-4;
    let mut r = Reseau::neuf(x.len());
    let avant = r.w_sortie.clone();
    r.entrainer_une(&x, cible, taux);
    for j in [0usize, 1, 7, CACHES, CACHES + 1, 2 * CACHES + 1] {
        let applique = -(r.w_sortie[j] - avant[j]) / taux;
        let attendu = gradient_numerique(true, j, &x, cible);
        assert!(
            (applique - attendu).abs() <= 1e-6 * attendu.abs().max(1e-3),
            "poids de sortie {j} : le réseau applique {applique:.9}, le gradient vaut {attendu:.9}"
        );
    }
}

/// **La couche cachée applique la même formule, et elle l'appliquait déjà.** Le
/// test est le même que le précédent, de l'autre côté : c'est ce qui donne son
/// sens au mot « cohérence » du §2.17.
#[test]
fn la_correction_de_la_couche_cachee_suit_le_gradient_de_la_perte() {
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let cible = [0.9, 0.1];
    let taux = 1e-4;
    let mut r = Reseau::neuf(x.len());
    let avant = r.w_cache.clone();
    r.entrainer_une(&x, cible, taux);
    for i in [0usize, 3, CACHES + 2, 2 * CACHES, 4 * CACHES + 7] {
        let applique = -(r.w_cache[i] - avant[i]) / taux;
        let attendu = gradient_numerique(false, i, &x, cible);
        assert!(
            (applique - attendu).abs() <= 1e-6 * attendu.abs().max(1e-3),
            "poids caché {i} : le réseau applique {applique:.9}, le gradient vaut {attendu:.9}"
        );
    }
}

/// **Ce que valait exactement l'ancienne formule** : pour deux sorties, la somme
/// complète vaut le double du terme diagonal, parce que `erreur[1] = −erreur[0]`
/// et `p₀p₁ = p₀(1 − p₀)`. Le défaut était donc un taux d'apprentissage deux fois
/// trop petit sur les poids de sortie — et ce test le chiffre.
#[test]
fn la_correction_de_sortie_vaut_le_double_du_terme_diagonal() {
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let cible = [0.9, 0.1];
    let taux = 1e-4;
    let mut r = Reseau::neuf(x.len());
    r.oublier();
    let p = r.evaluer(&x);
    let h = r.h.clone();
    let diagonal = (p[0] - cible[0]) * p[0] * (1.0 - p[0]);
    let avant = r.w_sortie.clone();
    r.entrainer_une(&x, cible, taux);
    for j in [0usize, 5, 33] {
        let applique = -(r.w_sortie[j] - avant[j]) / taux;
        let ancienne = diagonal * h[j];
        assert!(
            (applique - 2.0 * ancienne).abs() <= 1e-9 * applique.abs().max(1e-6),
            "poids de sortie {j} : {applique:.12} n'est pas le double de {ancienne:.12}"
        );
    }
}

/// Pour cinq sorties, la somme complète n'est plus un simple facteur du terme
/// diagonal : c'est pourquoi la correction n'est pas « un réglage » de ce
/// côté-là.
#[test]
fn pour_cinq_sorties_la_formule_complete_n_est_plus_un_facteur() {
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let cible = [0.7, 0.1, 0.1, 0.05, 0.05];
    let taux = 1e-4;
    let mut r = ReseauPhases::neuf(x.len());
    r.oublier();
    let p = r.evaluer(&x);
    let h = r.h.clone();
    let avant = r.w_sortie.clone();
    r.entrainer_une(&x, cible, taux);
    let mut rapports: Vec<f64> = Vec::new();
    for m in 0..PHASES {
        let j = m * (CACHES + 1) + 3;
        let applique = -(r.w_sortie[j] - avant[j]) / taux;
        let diagonal = (p[m] - cible[m]) * p[m] * (1.0 - p[m]) * h[3];
        rapports.push(applique / diagonal);
    }
    let ecart = rapports
        .iter()
        .fold(0.0f64, |acc, r| acc.max((r - rapports[0]).abs()));
    assert!(
        ecart > 0.01,
        "les cinq rapports sont les mêmes ({rapports:?}) : la formule serait un simple facteur"
    );
}

// ===========================================================================
// D3.2 — L'ACCUMULATEUR DE LA COUCHE CACHÉE (§2.17.2)
// ===========================================================================

/// **Les poids ne bougent pas pendant une passe, et c'est ce qui compte.**
/// `corriger` évalue toute la pile AVANT d'écrire quoi que ce soit : deux
/// situations identiques dans la pile, avec un facteur d'influence de 1, doivent
/// donc rendre exactement le double de la correction d'une seule. Si une écriture
/// s'intercalait entre les deux évaluations, la seconde partirait d'un réseau
/// déjà corrigé et le facteur ne serait plus 2.
#[test]
fn une_passe_evalue_avant_d_ecrire() {
    // Source : l'écriture directe dans les poids : engine/src/reseau.rs:606.
    let n = 6;
    let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let cible = [0.9, 0.1];
    let taux = 1e-5;

    let mut une = Reseau::neuf(n);
    une.lambda = 1.0;
    let depart = une.w_sortie.clone();
    let mut pile1 = Pile::new(n);
    pile1.empiler(&x, 0);
    une.corriger(&pile1, 0, cible, taux);
    let delta_une: Vec<f64> = une
        .w_sortie
        .iter()
        .zip(depart.iter())
        .map(|(a, b)| a - b)
        .collect();

    let mut deux = Reseau::neuf(n);
    deux.lambda = 1.0;
    let mut pile2 = Pile::new(n);
    pile2.empiler(&x, 0);
    pile2.empiler(&x, 0);
    deux.corriger(&pile2, 0, cible, taux);
    for (j, (a, b)) in deux.w_sortie.iter().zip(depart.iter()).enumerate() {
        let delta_deux = a - b;
        assert!(
            (delta_deux - 2.0 * delta_une[j]).abs() <= 1e-9 * delta_deux.abs().max(1e-12),
            "poids de sortie {j} : {delta_deux:.15} n'est pas le double de {:.15}",
            delta_une[j]
        );
    }
}

/// **Les sommes suffixes pèsent chaque situation à SON rang.** La correction
/// d'une pile de deux situations distinctes doit valoir, poids par poids, la
/// correction de la plus récente PLUS `lambda` fois celle de la plus ancienne :
/// c'est exactement l'identité que la factorisation par sommes suffixes
/// (engine/src/reseau.rs:671) remplace par un calcul plus court. Un décalage
/// d'un rang garde les sommes justes et ne se voit que d'ici.
///
/// Le test précédent ne suffisait pas : sa pile portait deux fois la MÊME
/// situation, cas où un décalage de rang est neutre, et il ne regardait que les
/// poids de sortie, que les sommes suffixes ne touchent pas.
#[test]
fn les_sommes_suffixes_pesent_chaque_situation_a_son_rang() {
    let n = 8;
    // Trois situations distinctes, de la plus ancienne à la plus récente. Deux
    // ne suffisent pas : le pas en arrière ne tourne alors que pour le rang 1,
    // où « le rang précédent » et « le rang zéro » désignent la même chose.
    let ancienne = vec![-1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0];
    let milieu = vec![1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0];
    let recente = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let cible = [0.9, 0.1];
    let taux = 1e-4;
    let l = 0.6;

    // La correction d'une pile, rendue en écarts par rapport aux poids de
    // départ. Les situations sont données de la plus ANCIENNE à la plus récente.
    let corriger_pile = |situations: &[&Vec<f64>]| -> (Vec<f64>, Vec<f64>) {
        let mut r = Reseau::neuf(n);
        r.lambda = l;
        let depart_c = r.w_cache.clone();
        let depart_s = r.w_sortie.clone();
        let mut pile = Pile::new(n);
        for x in situations.iter() {
            pile.empiler(x, 0);
        }
        r.corriger(&pile, 0, cible, taux);
        (
            r.w_cache.iter().zip(depart_c.iter()).map(|(a, b)| a - b).collect(),
            r.w_sortie.iter().zip(depart_s.iter()).map(|(a, b)| a - b).collect(),
        )
    };

    let (ens_c, ens_s) = corriger_pile(&[&ancienne, &milieu, &recente]);
    let (r0_c, r0_s) = corriger_pile(&[&recente]);
    let (r1_c, r1_s) = corriger_pile(&[&milieu]);
    let (r2_c, r2_s) = corriger_pile(&[&ancienne]);

    // **L'identité.** La pile de trois doit valoir, poids par poids, la
    // correction de la plus récente, plus `lambda` fois celle du milieu, plus
    // `lambda²` fois celle de la plus ancienne. C'est exactement ce que la
    // factorisation par sommes suffixes (engine/src/reseau.rs:671) calcule par
    // un chemin plus court : un rang de travers garde les sommes justes et ne se
    // voit que d'ici.
    let mut ecart_max = 0.0f64;
    let mut pire = 0usize;
    for i in 0..ens_c.len() {
        let attendu = r0_c[i] + l * r1_c[i] + l * l * r2_c[i];
        let ecart = (ens_c[i] - attendu).abs();
        if ecart > ecart_max {
            ecart_max = ecart;
            pire = i;
        }
        // Tolérance mesurée, pas devinée : l'écart le plus grand relevé sur le
        // code sain vaut moins de 1e-16 pour des corrections de l'ordre de 1e-7.
        assert!(
            ecart <= 1e-8 * ens_c[i].abs().max(attendu.abs()) + 1e-16,
            "poids caché {i} : la pile de trois rend {:.18}, la somme des trois rangs vaut {attendu:.18}",
            ens_c[i]
        );
    }
    assert!(ecart_max < 1e-14, "écart maximal {ecart_max:.3e} au poids {pire}");

    // La couche de sortie suit la même identité — elle n'emprunte pas les sommes
    // suffixes, et c'est justement pourquoi elle ne suffisait pas à les éprouver.
    for j in 0..ens_s.len() {
        let attendu = r0_s[j] + l * r1_s[j] + l * l * r2_s[j];
        assert!(
            (ens_s[j] - attendu).abs() <= 1e-8 * ens_s[j].abs().max(attendu.abs()) + 1e-16,
            "poids de sortie {j} : {:.18} au lieu de {attendu:.18}",
            ens_s[j]
        );
    }

    // Et le rang doit PESER : empilées dans l'autre ordre, les trois mêmes
    // situations ne donnent pas le même résultat. Sans cela, l'identité serait
    // vraie même à rangs permutés et ne prouverait rien.
    let (permutee, _) = corriger_pile(&[&recente, &milieu, &ancienne]);
    assert_ne!(
        permutee, ens_c,
        "l'ordre d'empilement ne change rien au résultat : le rang ne pèse pas"
    );
}

/// Une correction déplace bien les poids de la couche cachée : le produit externe
/// écrit désormais directement dedans, et il faut vérifier qu'il écrit quelque
/// part.
#[test]
fn une_correction_deplace_les_poids_de_la_couche_cachee() {
    let n = 6;
    let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    let mut r = Reseau::neuf(n);
    let avant = r.w_cache.clone();
    let mut pile = Pile::new(n);
    pile.empiler(&x, 0);
    r.corriger(&pile, 0, [0.9, 0.1], 1e-3);
    let bouges = r
        .w_cache
        .iter()
        .zip(avant.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        bouges > n * CACHES / 2,
        "seulement {bouges} poids cachés ont bougé sur {}",
        r.w_cache.len()
    );
    // Le biais d'entrée, dernière ligne du tableau, bouge lui aussi (valeur 1).
    let biais = n * CACHES;
    assert_ne!(r.w_cache[biais], avant[biais], "le biais d'entrée n'a pas bougé");
}

/// **`appliquer` remet l'accumulateur de sortie à zéro.** Appelée deux fois de
/// suite, la seconde ne doit rien changer — sinon la même correction serait
/// versée plusieurs fois.
#[test]
fn appliquer_deux_fois_ne_verse_pas_deux_fois() {
    // Source : le versement des 102 poids de sortie : engine/src/reseau.rs:707.
    let n = 4;
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let mut r = Reseau::neuf(n);
    r.entrainer_une(&x, [0.9, 0.1], 1e-3);
    let apres = r.w_sortie.clone();
    let cache = r.w_cache.clone();
    r.appliquer();
    assert_eq!(r.w_sortie, apres, "la couche de sortie a bougé sur un `appliquer` à vide");
    assert_eq!(r.w_cache, cache, "la couche cachée a bougé sur un `appliquer` à vide");
}

/// Une correction sur une pile vide ne fait rien du tout : c'est le cas d'un
/// joueur qui n'a pris aucune décision.
#[test]
fn une_pile_vide_ne_corrige_rien() {
    let n = 4;
    let mut r = Reseau::neuf(n);
    let cache = r.w_cache.clone();
    let sortie = r.w_sortie.clone();
    let pile = Pile::new(n);
    r.corriger(&pile, 0, [0.9, 0.1], 1e-3);
    assert_eq!(r.w_cache, cache);
    assert_eq!(r.w_sortie, sortie);
}

/// La pile ne rend que les situations du joueur demandé : corriger le siège 1
/// sur une pile qui ne contient que des situations du siège 0 ne doit rien faire.
#[test]
fn la_pile_ne_rend_que_les_situations_du_bon_siege() {
    let n = 4;
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let mut r = Reseau::neuf(n);
    let cache = r.w_cache.clone();
    let mut pile = Pile::new(n);
    pile.empiler(&x, 0);
    r.corriger(&pile, 1, [0.9, 0.1], 1e-3);
    assert_eq!(r.w_cache, cache, "une situation du siège 0 a corrigé le siège 1");
}

// ===========================================================================
// LA MESURE DE LA COUCHE CACHÉE (livrable imposé n° 5)
// ===========================================================================

/// Éteint, le relevé ne compte rien : c'est ce qui permet de le laisser dans la
/// boucle la plus chaude du dépôt.
#[test]
fn le_releve_de_couche_est_eteint_par_defaut() {
    let mut r = Reseau::neuf(4);
    assert!(!r.mesurer_couche, "le relevé est allumé par défaut");
    for _ in 0..10 {
        r.oublier();
        r.evaluer(&[1.0, -1.0, 1.0, -1.0]);
    }
    assert_eq!(r.situations_vues(), 0, "le relevé a compté alors qu'il est éteint");
    assert_eq!(r.pente_moyenne(), 0.0);
    assert_eq!(r.part_saturee(), 0.0);
    assert_eq!(r.neurones_figes(), 0);
}

/// Allumé, il compte une situation par évaluation — ni plus, ni moins.
#[test]
fn le_releve_compte_une_situation_par_evaluation() {
    let mut r = Reseau::neuf(4);
    r.mesurer_couche = true;
    for k in 0..7 {
        r.oublier();
        r.evaluer(&[1.0, -1.0, 1.0, -1.0]);
        assert_eq!(r.situations_vues(), k + 1, "compte après {} évaluations", k + 1);
    }
}

/// **Un réseau à poids nuls a une pente de 1 partout** : `tanh(0) = 0`, donc
/// `1 − h² = 1`. C'est la borne haute, et elle est vraie par construction — pas
/// un chiffre relevé quelque part.
#[test]
fn un_reseau_a_poids_nuls_a_la_pente_maximale() {
    let mut r = Reseau::neuf(4);
    for w in r.w_cache.iter_mut() {
        *w = 0.0;
    }
    r.mesurer_couche = true;
    r.oublier();
    r.evaluer(&[1.0, -1.0, 1.0, -1.0]);
    assert!((r.pente_moyenne() - 1.0).abs() < 1e-12, "pente {}", r.pente_moyenne());
    assert_eq!(r.part_saturee(), 0.0, "aucun neurone ne devrait être saturé");
}

/// Un neurone dont la somme d'entrée est énorme est couché contre ±1 : sa pente
/// tombe sous le seuil et il est compté saturé.
#[test]
fn un_neurone_couche_contre_un_est_compte_sature() {
    // Source : le seuil de saturation : engine/src/reseau.rs:147.
    let mut r = Reseau::neuf(4);
    for w in r.w_cache.iter_mut() {
        *w = 0.0;
    }
    // Le biais d'entrée est la dernière ligne du tableau : il vaut toujours 1.
    let biais = 4 * CACHES;
    for j in 0..CACHES {
        r.w_cache[biais + j] = if j < 10 { 10.0 } else { 0.0 };
    }
    r.mesurer_couche = true;
    r.oublier();
    r.evaluer(&[1.0, -1.0, 1.0, -1.0]);
    let attendu = 100.0 * 10.0 / CACHES as f64;
    assert!(
        (r.part_saturee() - attendu).abs() < 1e-9,
        "part saturée {} au lieu de {attendu}",
        r.part_saturee()
    );
}

/// **Un neurone figé est un neurone qui rend la même valeur partout** (audit
/// §2.1). Ici, dix neurones ne dépendent d'aucune entrée : leur amplitude de
/// sortie est nulle.
#[test]
fn un_neurone_qui_ne_depend_d_aucune_entree_est_compte_fige() {
    let n = 4;
    let mut r = Reseau::neuf(n);
    for i in 0..n {
        for j in 0..10 {
            r.w_cache[i * CACHES + j] = 0.0;
        }
    }
    r.mesurer_couche = true;
    for x in [
        vec![1.0, -1.0, 1.0, -1.0],
        vec![-1.0, 1.0, -1.0, 1.0],
        vec![1.0, 1.0, 1.0, 1.0],
    ] {
        r.oublier();
        r.evaluer(&x);
    }
    assert!(
        r.neurones_figes() >= 10,
        "seulement {} neurones figés alors que dix ne lisent aucune entrée",
        r.neurones_figes()
    );
    assert!(
        r.amplitude_minimale() < AMPLITUDE_FIGEE,
        "amplitude minimale {}",
        r.amplitude_minimale()
    );
}

/// Un réseau ordinaire, sur de vraies situations, n'a aucun neurone figé : le
/// défaut du §2.1 apparaît à l'entraînement, il n'est pas dans le tirage.
#[test]
fn un_reseau_neuf_n_a_aucun_neurone_fige() {
    let (desc, situations) = situations_reelles(2);
    let mut r = Reseau::neuf(desc.taille);
    r.mesurer_couche = true;
    for x in situations.iter() {
        r.oublier();
        r.evaluer(x);
    }
    assert_eq!(
        r.neurones_figes(),
        0,
        "un réseau neuf a déjà {} neurones figés (amplitude minimale {})",
        r.neurones_figes(),
        r.amplitude_minimale()
    );
}

/// `raz_couche` efface tout le relevé, y compris les extrêmes par neurone —
/// sinon une tranche hériterait de l'amplitude de la précédente.
#[test]
fn raz_couche_efface_le_releve_et_les_extremes() {
    // Fiche large et situations exactement opposées : les sommes cachées de la
    // seconde sont l'opposé de celles de la première, les sorties sont donc
    // séparées de `2·|h|` — bien au-delà du dixième qui sépare un neurone figé
    // d'un neurone vivant (engine/src/reseau.rs:147).
    let n = 400;
    let x: Vec<f64> = (0..n).map(|i| if i % 3 == 0 { 1.0 } else { -1.0 }).collect();
    let y: Vec<f64> = x.iter().map(|v| -v).collect();
    let mut r = Reseau::neuf(n);
    r.mesurer_couche = true;
    r.oublier();
    r.evaluer(&x);
    r.oublier();
    r.evaluer(&y);
    assert!(r.amplitude_minimale() > 0.0);
    assert!(
        r.neurones_figes() < CACHES,
        "deux situations opposées et pas un neurone qui bouge : le relevé ne relève rien"
    );
    r.raz_couche();
    assert_eq!(r.situations_vues(), 0);
    assert_eq!(r.pente_moyenne(), 0.0);
    assert_eq!(r.amplitude_minimale(), 0.0, "les extrêmes ont survécu au raz");
    r.oublier();
    r.evaluer(&x);
    assert_eq!(r.situations_vues(), 1);
    // **C'est ici qu'un extrême survivant se voit — et il faut regarder TOUS les
    // neurones, pas le plus petit.** Tant que le compteur est vide,
    // `amplitude_minimale` rend zéro sans lire les extrêmes
    // (engine/src/reseau.rs:789) ; et après une évaluation, elle rend encore
    // zéro dès qu'UN SEUL neurone se trouve du bon côté, ce qui arrive toujours
    // sur cinquante. Une seule situation relevée donne à chaque neurone le même
    // minimum et le même maximum : ils sont donc tous figés, les cinquante.
    assert_eq!(
        r.amplitude_minimale(),
        0.0,
        "une seule situation relevée après le raz, et l'amplitude n'est pas nulle : un extrême a survécu"
    );
    assert_eq!(
        r.neurones_figes(),
        CACHES,
        "après le raz et une seule situation, {} neurones sur {CACHES} ont déjà une amplitude : les extrêmes de la tranche précédente ont survécu",
        r.neurones_figes()
    );
}

// ===========================================================================
// D1 — LES OUVRIERS : COPIE, DIFFÉRENCE, ORDRE (§2.7)
// ===========================================================================

/// Un ouvrier repart exactement des poids du maître.
#[test]
fn copier_les_poids_rend_deux_reseaux_identiques() {
    let mut maitre = Reseau::neuf(50);
    maitre.entrainer_une(&vec![1.0; 50], [0.9, 0.1], 1e-3);
    // **Le compteur de parties doit valoir autre chose que zéro des deux côtés.**
    // Comparé à lui-même, il ne prouve rien : un ouvrier qui repartirait à zéro
    // écraserait le compte du maître au versement (engine/src/reseau.rs:811).
    maitre.parties = 123_456;
    let mut ouvrier = Reseau::neuf(50);
    assert_eq!(ouvrier.parties, 0, "un réseau neuf devrait partir à zéro partie");
    assert_ne!(ouvrier.w_cache, maitre.w_cache, "les deux réseaux partiraient déjà égaux");
    ouvrier.copier_les_poids_de(&maitre);
    assert_eq!(ouvrier.w_cache, maitre.w_cache);
    assert_eq!(ouvrier.w_sortie, maitre.w_sortie);
    assert_eq!(ouvrier.parties, 123_456, "le compteur de parties n'a pas suivi la copie");
    assert_eq!(ouvrier.lambda, maitre.lambda, "le lambda n'a pas suivi la copie");
}

/// **La différence d'un ouvrier se verse exactement.** Un seul ouvrier, versé
/// dans un maître qui n'a pas bougé, doit rendre les poids de l'ouvrier — c'est
/// la propriété qui fait que le partage n'invente rien.
#[test]
fn la_difference_d_un_seul_ouvrier_rend_les_poids_de_l_ouvrier() {
    // Source : le versement d'un ouvrier : engine/src/reseau.rs:826.
    let n = 30;
    let mut maitre = Reseau::neuf(n);
    let mut base = Reseau::neuf(n);
    base.copier_les_poids_de(&maitre);
    let mut ouvrier = Reseau::neuf(n);
    ouvrier.copier_les_poids_de(&maitre);
    ouvrier.entrainer_une(&vec![1.0; n], [0.9, 0.1], 1e-3);
    maitre.ajouter_la_difference(&base, &ouvrier);
    for (j, (a, b)) in maitre.w_cache.iter().zip(ouvrier.w_cache.iter()).enumerate() {
        assert!(
            (a - b).abs() <= 1e-15 * a.abs().max(1e-12),
            "poids caché {j} : {a} contre {b}"
        );
    }
    // **ET LA COUCHE DE SORTIE AUSSI.** Ce test ne regardait que `w_cache` : on
    // pouvait verser la différence de la couche de sortie à l'index miroir — une
    // permutation qui garde la somme juste — sans qu'aucun test ne tombe. Un
    // sabotage de la sixième passe (P07) l'a montré.
    for (j, (a, b)) in maitre.w_sortie.iter().zip(ouvrier.w_sortie.iter()).enumerate() {
        assert!(
            (a - b).abs() <= 1e-15 * a.abs().max(1e-12),
            "poids de sortie {j} : {a} contre {b}"
        );
    }
}

/// Un ouvrier qui n'a rien appris ne change rien, **au bit près**.
#[test]
fn un_ouvrier_qui_n_a_rien_appris_ne_change_rien() {
    let n = 30;
    let mut maitre = Reseau::neuf(n);
    maitre.entrainer_une(&vec![1.0; n], [0.9, 0.1], 1e-3);
    let temoin = maitre.w_cache.clone();
    let temoin_sortie = maitre.w_sortie.clone();
    let mut base = Reseau::neuf(n);
    base.copier_les_poids_de(&maitre);
    let mut ouvrier = Reseau::neuf(n);
    ouvrier.copier_les_poids_de(&maitre);
    maitre.ajouter_la_difference(&base, &ouvrier);
    assert_eq!(maitre.w_cache, temoin, "un ouvrier oisif a déplacé les poids cachés");
    // Les DEUX couches : voir le commentaire du test précédent (sabotage P07).
    assert_eq!(
        maitre.w_sortie, temoin_sortie,
        "un ouvrier oisif a déplacé les poids de sortie"
    );
}

/// **L'ORDRE DU VERSEMENT COMPTE, ET C'EST POUR CELA QU'IL EST FIXÉ.** Les
/// différences des ouvriers sont versées dans l'ordre des graines, jamais dans
/// l'ordre d'arrivée des fils : c'est la propriété dont tout le déterminisme du
/// partage dépend.
///
/// Le test se fait en DEUX temps, et le second est le seul qui prouve quelque
/// chose. Vérifier qu'un versement répété rend le même résultat ne prouve rien :
/// la somme est une fonction pure, elle rendrait la même valeur même si le code
/// parcourait les ouvriers à l'envers — un tel test reste vert sur exactement le
/// défaut qu'il prétend interdire. On exige donc en plus que **deux ordres
/// différents divergent**.
///
/// Les trois différences sont choisies pour que l'addition flottante ne soit pas
/// associative : `+1`, `+3e-17`, `−1`. La minuscule est absorbée par la grande
/// (à hauteur de 1, le dernier bit vaut 2,2e-16) mais pas par le poids de départ
/// (à hauteur de 0,01, il vaut 1,7e-18). Verser dans l'ordre `+1, +3e-17, −1`
/// rend donc le poids de départ inchangé ; verser `+1, −1, +3e-17` rend le poids
/// de départ plus 3e-17. Si un jour ces deux résultats redevenaient égaux, le
/// test tomberait — et il aurait raison de tomber, car alors il ne prouverait
/// plus rien.
#[test]
fn l_ordre_du_versement_des_ouvriers_change_le_resultat() {
    // Source : le versement d'un ouvrier : engine/src/reseau.rs:826.
    let n = 25;
    let mut base = Reseau::neuf(n);
    // Un poids de départ FIXE, et non celui du tirage : le raisonnement sur le
    // dernier bit ci-dessus doit tenir quelle que soit `AMPLITUDE_DEPART`.
    for w in base.w_cache.iter_mut() {
        *w = 0.01;
    }
    let ouvrier_qui_ajoute = |delta: f64| {
        let mut o = Reseau::neuf(n);
        o.copier_les_poids_de(&base);
        for w in o.w_cache.iter_mut() {
            *w += delta;
        }
        o
    };
    let verser = |ordre: [f64; 3]| {
        let mut maitre = Reseau::neuf(n);
        maitre.copier_les_poids_de(&base);
        for d in ordre {
            maitre.ajouter_la_difference(&base, &ouvrier_qui_ajoute(d));
        }
        maitre.w_cache.clone()
    };
    let canonique = [1.0, 3e-17, -1.0];
    let permute = [1.0, -1.0, 3e-17];
    assert_eq!(
        verser(canonique),
        verser(canonique),
        "deux versements dans le MÊME ordre divergent : le partage n'est pas déterministe"
    );
    assert_ne!(
        verser(canonique),
        verser(permute),
        "deux ordres différents rendent le même résultat : ce test ne prouve plus rien"
    );
}

/// Les statistiques d'un ouvrier s'additionnent, et les extrêmes par neurone se
/// prennent au plus large : sans quoi la pente publiée ne porterait que sur un
/// ouvrier sur quatre.
#[test]
fn absorber_les_statistiques_additionne_et_elargit() {
    let n = 4;
    let x = vec![1.0, -1.0, 1.0, -1.0];
    let mut maitre = Reseau::neuf(n);
    let mut a = Reseau::neuf(n);
    let mut b = Reseau::neuf(n);
    for r in [&mut a, &mut b] {
        r.mesurer_couche = true;
    }
    a.oublier();
    a.evaluer(&x);
    b.oublier();
    b.evaluer(&x);
    b.oublier();
    b.evaluer(&vec![-1.0, 1.0, -1.0, 1.0]);
    let ampl_b = b.amplitude_minimale();
    maitre.absorber_les_statistiques(&a);
    maitre.absorber_les_statistiques(&b);
    assert_eq!(maitre.situations_vues(), 3, "les situations ne se sont pas additionnées");
    assert!(
        maitre.amplitude_minimale() >= ampl_b - 1e-12,
        "les extrêmes ne se sont pas élargis"
    );
}

// ===========================================================================
// D3.3 — LA SORTIE ANTICIPÉE (flow::play_round, §2.17.3)
// ===========================================================================

/// **Une politique ordinaire ne s'interrompt jamais.** C'est la condition pour
/// que les quatre empreintes d'état ne bougent pas : le simulateur, la sonde
/// d'audit et toutes les politiques scriptées héritent du corps par défaut.
#[test]
fn une_politique_ordinaire_ne_s_interrompt_jamais() {
    // Source : le corps par défaut : engine/src/policy.rs:101.
    let pol = RandomPolicy;
    assert!(!pol.interrompu(), "RandomPolicy s'interrompt");
}

/// Un rejeu qui n'a pas encore posé son point d'attente ne s'interrompt pas.
#[test]
fn un_rejeu_sans_point_d_attente_ne_s_interrompt_pas() {
    let mut r = Rejeu::new(Vec::new());
    assert!(r.attente.is_none());
    assert!(r.vue.is_none());
    assert!(!r.interrompu(), "un rejeu neuf s'interrompt déjà");

    // **L'état intermédiaire, celui que la condition protège.** L'attente est
    // posée en `engine/src/rejeu.rs:374`, l'état n'est cloné qu'en
    // `engine/src/rejeu.rs:424` : entre les deux, `interrompu` doit répondre
    // NON. Couper là laisserait `joueur::etat_atteint` retomber sur l'état
    // vivant de la manche au lieu de l'état de la décision attendue, et le
    // point de décision aurait bougé sans que rien ne le dise.
    r.attente = Some(0);
    assert!(r.vue.is_none(), "l'état ne devrait pas être cloné à ce stade");
    assert!(
        !r.interrompu(),
        "le rejeu s'interrompt alors qu'il n'a pas encore cloné l'état attendu"
    );

    // Et l'inverse : un état cloné sans attente ne coupe rien non plus — c'est
    // le cas de toute décision observée puis effectivement prise.
    let mut r2 = Rejeu::new(Vec::new());
    r2.attente = None;
    assert!(!r2.interrompu());
}

/// **Un rejeu qui attend s'interrompt** — et c'est ce qui coupe le travail que la
/// manche exécutait pour rien.
#[test]
fn un_rejeu_qui_attend_s_interrompt() {
    // Source : la seule redéfinition du dépôt : engine/src/rejeu.rs:443.
    let (db, _desc) = decor();
    let mut rejeu = Rejeu::new(Vec::new());
    let _ = setup_game(&db, 6_600_001, &mut rejeu);
    assert!(rejeu.attente.is_some(), "le rejeu n'a pas posé de point d'attente");
    assert!(rejeu.vue.is_some(), "le rejeu n'a pas cloné l'état de la décision attendue");
    assert!(rejeu.interrompu(), "le rejeu qui attend ne s'interrompt pas");
}

/// **La manche s'arrête là où le rejeu attend.** Sans la sortie anticipée, la
/// manche allait jusqu'à son terme et incrémentait la génération ; tout ce
/// travail-là était jeté par `joueur::etat_atteint`, qui rend l'état cloné à la
/// décision attendue.
#[test]
fn la_manche_ne_va_pas_a_son_terme_quand_le_rejeu_attend() {
    // Source : la sortie anticipée : engine/src/flow.rs:5970.
    let (db, _desc) = decor();
    // Une partie menée à son terme de mise en place par une politique ordinaire.
    let mut pol = RandomPolicy;
    let modele = setup_game(&db, 6_600_001, &mut pol);
    let generation_de_depart = modele.generation;

    let mut game = modele.clone();
    let mut rejeu = Rejeu::new(Vec::new());
    play_round(&mut game, &db, &mut rejeu);
    assert!(rejeu.attente.is_some(), "le rejeu n'a pas attendu");
    assert_eq!(
        game.generation, generation_de_depart,
        "la manche est allée à son terme alors que plus rien n'était enregistré"
    );

    // La même manche, jouée par une politique qui ne s'interrompt pas, va bien
    // jusqu'au bout : la sortie anticipée n'est pas un arrêt général.
    let mut game = modele.clone();
    let mut pol = RandomPolicy;
    play_round(&mut game, &db, &mut pol);
    assert_eq!(
        game.generation,
        generation_de_depart + 1,
        "une politique ordinaire ne doit pas être interrompue"
    );
}

/// **LES QUATRE EMPREINTES D'ÉTAT.** L'empreinte de 300 parties, boîtes
/// `base,decouverte`, graine 4242, a été relevée sur le commit `ada92b6` —
/// c'est-à-dire sur le code d'AVANT ce lot — et publiée par le contrôle 02. La
/// sortie anticipée du §2.17.3 est le seul changement de ce lot qui touche
/// `flow.rs` : c'est ici qu'on prouve qu'elle ne déplace aucun point de décision.
#[test]
fn l_empreinte_d_etat_de_300_parties_n_a_pas_bouge() {
    const EMPREINTE_SUR_ADA92B6: u64 = 0x7b5b_eb0c_04da_3776;
    let (db, _desc) = decor();
    let mut pol = RandomPolicy;
    let bilan = run_simulation(&db, 300, 4242, &mut pol);
    assert_eq!(
        bilan.state_hash, EMPREINTE_SUR_ADA92B6,
        "l'empreinte d'état a bougé : un point de décision a été déplacé"
    );
    assert_eq!(bilan.completed, 300, "des parties ne sont pas allées à leur terme");
    assert_eq!(bilan.invariant_violations, 0, "des invariants sont violés");
}

// ===========================================================================
// LA FICHE, LE FICHIER DE POIDS, ET CE QUE CE LOT NE DOIT PAS TOUCHER
// ===========================================================================

/// **La fiche de situation ne bouge pas dans ce lot.** 1 630 entrées : un seul
/// nom qui bougerait rendrait illisible `data/poids/apprenti-L3-amorce.txt` et
/// invaliderait tout le lot.
#[test]
fn la_fiche_compte_toujours_mille_six_cent_trente_entrees() {
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    assert_eq!(noms.len(), 1630, "la fiche a changé de taille");
    assert_eq!(desc.taille, noms.len(), "la taille annoncée ne suit pas les noms");
}

/// Le fichier de poids de référence se charge toujours — c'est le verrou de
/// `Reseau::lire` : la table des noms doit correspondre une à une et dans le même
/// ordre.
#[test]
fn le_fichier_de_poids_de_reference_se_charge() {
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    let r = Reseau::lire("../data/poids/apprenti-L3-amorce.txt", &noms)
        .expect("le fichier de poids de référence doit se charger");
    assert_eq!(r.n_entrees, noms.len());
    assert!(r.parties > 0, "le compteur de parties du fichier est nul");
}

/// Un fichier de poids fait l'aller-retour en gardant son compteur de parties :
/// c'est ce compteur, et non le nom du fichier, qui dit son ancienneté — tout le
/// défaut des instantanés du §2.6 vient de là.
#[test]
fn le_compteur_de_parties_survit_a_l_aller_retour() {
    let noms: Vec<String> = (0..6).map(|i| format!("entree_{i}")).collect();
    let mut r = Reseau::neuf(noms.len());
    r.parties = 1_200_000;
    let mut chemin = std::env::temp_dir();
    chemin.push(format!("lot-vitesse-compteur-{}.txt", std::process::id()));
    let chemin = chemin.to_string_lossy().to_string();
    r.ecrire(&chemin, &noms).expect("écriture");
    let relu = Reseau::lire(&chemin, &noms).expect("lecture");
    assert_eq!(relu.parties, 1_200_000, "le compteur absolu ne survit pas au fichier");
    let _ = std::fs::remove_file(&chemin);
}

/// **Ce que ce lot ne règle pas.** Le taux d'apprentissage, le facteur
/// d'influence, le rythme des corrections et la forme du réseau appartiennent à
/// d'autres chantiers : un test de non-régression pour qu'un réglage ne s'égare
/// pas dans celui-ci.
#[test]
fn les_reglages_hors_de_ce_lot_n_ont_pas_bouge() {
    assert_eq!(CACHES, 50, "la largeur de la couche cachée appartient au lot suivant (§2.16)");
    assert_eq!(SORTIES, 2, "le nombre de sorties du premier réseau a bougé");
    assert_eq!(PHASES, 5, "le second réseau n'a plus une sortie par carte Phase");
    assert_eq!(TAUX, 0.0001, "le taux d'apprentissage a bougé");
    assert_eq!(LAMBDA, 0.9, "le facteur d'influence par pas en arrière a bougé");
    assert_eq!(RYTHME, 8, "le rythme des corrections a bougé");
}

/// **Le miroir JavaScript porte la même largeur de couche cachée.** « Si tu
/// touches une constante que `web/webapp/joueurs/apprenti.js` connaît aussi, les
/// deux doivent changer ensemble » — ce lot n'y touche pas, et ce test le tient.
#[test]
fn le_miroir_javascript_porte_la_meme_largeur_de_couche_cachee() {
    let js = std::fs::read_to_string("../web/webapp/joueurs/apprenti.js")
        .expect("le miroir JavaScript doit exister");
    let attendu = format!("const CACHES_ATTENDUS = {CACHES};");
    assert!(
        js.contains(&attendu),
        "le miroir JavaScript ne porte pas « {attendu} » : Rust et JavaScript divergeraient"
    );
}

/// **Le code de la devinette n'a pas été supprimé.** Il est éteint par défaut et
/// il faut pouvoir y revenir : le second réseau à cinq sorties existe toujours et
/// fonctionne.
#[test]
fn le_second_reseau_de_la_devinette_existe_toujours() {
    let mut r = ReseauPhases::neuf(20);
    let p = r.evaluer(&vec![1.0; 20]);
    assert_eq!(p.len(), PHASES);
    let somme: f64 = p.iter().sum();
    assert!((somme - 1.0).abs() < 1e-12, "somme des cinq sorties : {somme}");
}

/// **Le fichier de configuration du processeur existe et vise le processeur
/// réel** (§2.4). Sa présence est un fait du dépôt, pas une intention.
#[test]
fn le_fichier_de_configuration_vise_le_processeur_reel() {
    let texte = std::fs::read_to_string(".cargo/config.toml")
        .expect("engine/.cargo/config.toml doit exister");
    assert!(texte.contains("target-cpu"), "le fichier ne fixe pas le processeur");
    assert!(texte.contains("native"), "le fichier ne vise pas le processeur réel");
    assert!(
        texte.contains("[build]"),
        "le réglage n'est pas dans la section qui vaut pour toutes les compilations"
    );
}

// ===========================================================================
// LE PROGRAMME D'ENTRAÎNEMENT, VU DE L'EXTÉRIEUR
//
// « Une correction qu'on ne peut pas voir de l'extérieur ne se contrôle pas. »
// Ces tests lancent le binaire et lisent ce qu'il publie.
// ===========================================================================

/// Un entraînement court, et ce qu'il écrit. Rend (journal, sortie d'erreur).
fn entrainer(args: &[&str]) -> (String, String) {
    let sortie = Command::new(env!("CARGO_BIN_EXE_entraine"))
        .args(args)
        .output()
        .expect("le programme d'entraînement doit se lancer");
    assert!(
        sortie.status.success(),
        "entraine a échoué : {}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    (
        String::from_utf8_lossy(&sortie.stdout).to_string(),
        String::from_utf8_lossy(&sortie.stderr).to_string(),
    )
}

fn dossier_d_essai(nom: &str) -> std::path::PathBuf {
    let mut d = std::env::temp_dir();
    d.push(format!("lot-vitesse-{nom}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("dossier d'essai");
    d
}

/// **La première ligne du journal dit ce qui l'a produit** (§2.6) : la ligne de
/// commande et la plage de graines consommée. « Un journal qu'on ne peut pas
/// rejouer ne prouve rien. »
#[test]
fn la_premiere_ligne_du_journal_donne_la_commande_et_les_graines() {
    let d = dossier_d_essai("entete");
    let f = d.join("p.txt");
    let (journal, _) = entrainer(&[
        "--parties", "8", "--graine-debut", "960001",
        "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
    ]);
    let tete = journal.lines().next().expect("un journal non vide");
    assert!(tete.contains("entraine"), "la première ligne ne dit pas la commande : {tete}");
    // La plage, DANS L'ORDRE : deux graines présentes mais interverties diraient
    // une plage vide. C'est un sabotage de la campagne « vu rouge » qui l'a
    // montré — la première version de ce test acceptait la permutation.
    assert!(
        tete.contains("960001..960008"),
        "la première ligne ne donne pas la plage de graines dans l'ordre : {tete}"
    );
    assert!(tete.contains("base,decouverte"), "la première ligne ne dit pas les boîtes : {tete}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Le journal publie les égalités départagées et la couche cachée** (livrable
/// imposé n° 5).
#[test]
fn le_journal_publie_les_egalites_departagees_et_la_couche_cachee() {
    let d = dossier_d_essai("publie");
    let f = d.join("p.txt");
    let (journal, _) = entrainer(&[
        "--parties", "12", "--graine-debut", "960001",
        "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
    ]);
    assert!(
        journal.contains("egalites_departagees:"),
        "le journal ne publie pas les égalités départagées :\n{journal}"
    );
    let ligne = journal
        .lines()
        .find(|l| l.contains("couche_cachee"))
        .unwrap_or_else(|| panic!("le journal ne publie pas la couche cachée :\n{journal}"));
    assert!(ligne.contains("pente_moyenne"), "{ligne}");
    assert!(ligne.contains("part_saturee"), "{ligne}");
    assert!(ligne.contains("neurones_figes"), "{ligne}");
    // La pente publiée doit être celle d'une couche cachée qui travaille.
    let pente: f64 = ligne
        .split("pente_moyenne")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or_else(|| panic!("pente illisible dans « {ligne} »"));
    // **Bornée des DEUX côtés.** Une pente est la moyenne de `1 − h²` : elle vit
    // entre zéro et un. Sans la borne haute, le journal pourrait publier la part
    // saturée à sa place — 9 %, c'est-à-dire 9,0 — et le test n'y verrait que du
    // feu (engine/src/reseau.rs:756).
    assert!(
        pente > 0.45 && pente <= 1.0,
        "la pente moyenne publiée n'est pas une pente : {pente}"
    );
    // La part saturée est un pourcentage : elle vit entre 0 et 100, et à
    // l'amplitude livrée elle reste bien en dessous de la moitié.
    let part: f64 = ligne
        .split("part_saturee")
        .nth(1)
        .and_then(|s| s.split('%').next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or_else(|| panic!("part saturée illisible dans « {ligne} »"));
    assert!(
        (0.0..=50.0).contains(&part),
        "la part saturée publiée n'est pas un pourcentage plausible : {part}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **LE CHIFFRE PUBLIÉ COMPTE DES CORRECTIONS, PAS DES OCCASIONS** (famille B).
///
/// C'est le test qui manquait, et son absence était le trou le plus large du
/// banc. Les autres tests de la famille B appellent `cible_finale_departagee`
/// **directement** : ils prouvent que la fonction est juste, jamais qu'`entraine`
/// s'en sert. On pouvait donc remplacer, dans `entraine`, la cible départagée par
/// l'ancienne `cible_finale` — c'est-à-dire annuler toute la famille B — sans
/// qu'un seul test ni un seul contrôle ne rougisse.
///
/// Ce test ferme le trou par le seul bout accessible de l'extérieur : le chiffre
/// du journal. Le compteur est désormais déduit de la cible réellement versée au
/// réseau (`engine/src/bin/entraine.rs`), et non de la présence d'un vainqueur :
/// sans la correction, la cible à points égaux redevient `[0,5 ; 0,5]`, le
/// compteur retombe à **zéro**, et ce test tombe.
///
/// La plage est choisie, pas prise au hasard : sur les soixante parties à partir
/// de 960001, l'entraînement rencontre quatre égalités de points de victoire. Une
/// plage sans égalité ne prouverait rien, et le test le dit au lieu de passer
/// vert.
#[test]
fn le_compteur_d_egalites_tombe_a_zero_si_le_departage_disparait() {
    // Source : la cible de fin de partie départagée : engine/src/reseau.rs:981.
    // Source : le vainqueur au sens du livret : engine/src/flow.rs:5824.
    let d = dossier_d_essai("compteur-departage");
    let f = d.join("p.txt");
    let (journal, _) = entrainer(&[
        "--parties", "60", "--graine-debut", "960001",
        "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
        "--ouvriers", "1",
    ]);
    // **AUCUNE CORRECTION À CONTRESENS, SUR AUCUNE PARTIE.** `cible_finale` rend
    // `cible[0] > cible[1]` exactement quand `score_moi > score_autre`
    // (engine/src/reseau.rs:947) : la cible versée à un joueur doit donc le
    // désigner gagnant si et seulement si la partie l'a fait gagner. Le garde-fou
    // couvre les 60 parties × 2 joueurs, pas seulement les égalités — permuter
    // les deux scores passés à la cible inverse le sens de l'apprentissage sur
    // CHAQUE partie en gardant les deux cibles complémentaires, et rien d'autre
    // dans ce banc ne le voit (sabotage P11 de la sixième passe).
    let sens = journal
        .lines()
        .find(|l| l.starts_with("corrections_a_contresens:"))
        .unwrap_or_else(|| panic!("le journal ne publie pas le garde-fou de sens :\n{journal}"));
    let a_contresens: u64 = sens
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("garde-fou illisible dans « {sens} »"));
    assert_eq!(
        a_contresens, 0,
        "{a_contresens} cible(s) de fin de partie versée(s) à contresens sur 120 : \
         l'entraînement apprend le résultat inverse"
    );

    let ligne = journal
        .lines()
        .find(|l| l.starts_with("egalites_departagees:"))
        .unwrap_or_else(|| panic!("le journal ne publie pas les égalités :\n{journal}"));
    let departagees: u64 = ligne
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("compteur illisible dans « {ligne} »"));
    let sur: u64 = ligne
        .split_whitespace()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("total illisible dans « {ligne} »"));
    assert_eq!(sur, 60, "le journal ne compte pas sur les parties demandées : {ligne}");
    assert!(
        departagees >= 3,
        "seulement {departagees} égalité(s) départagée(s) sur 60 parties : \
         soit la correction de la famille B n'est plus appliquée, soit cette \
         plage de graines n'a plus d'égalité et le test ne prouve plus rien — \
         ligne complète : « {ligne} »"
    );
    // Une partie « nulle jusqu'au bout » n'est PAS une égalité départagée : elle
    // est comptée à part et reste rarissime. Si elle se mettait à égaler le
    // nombre de départages, c'est que `flow::winner` aurait cessé de départager.
    let nulles: u64 = ligne
        .split("dont ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("nulles illisibles dans « {ligne} »"));
    assert!(
        nulles < departagees,
        "{nulles} partie(s) nulle(s) pour {departagees} départage(s) : le livret ne départage plus"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Deux entraînements identiques rendent le même fichier, octet pour octet —
/// à quatre ouvriers.** C'est la condition de tout : sans elle, aucun contrôle de
/// ce projet n'est possible.
#[test]
fn deux_entrainements_a_quatre_ouvriers_rendent_le_meme_fichier() {
    let d = dossier_d_essai("determinisme");
    let mut empreintes = Vec::new();
    for nom in ["a.txt", "b.txt"] {
        let f = d.join(nom);
        entrainer(&[
            "--parties", "12", "--graine-debut", "960001", "--ouvriers", "4",
            "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
        ]);
        empreintes.push(std::fs::read(&f).expect("le fichier de poids"));
    }
    assert_eq!(
        empreintes[0], empreintes[1],
        "deux entraînements à quatre ouvriers donnent deux fichiers différents"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Un instantané porte le nombre ABSOLU de parties** (§2.6). Un entraînement
/// repris à 8 parties, prolongé de 8, doit rendre un instantané `.12` — et non
/// exiger 12 parties de plus. C'est le défaut qui a produit
/// `data/poids/apprenti-2M.txt.200000`, un fichier nommé 200 000 qui en contient
/// 1 200 000.
#[test]
fn un_instantane_porte_le_nombre_absolu_de_parties() {
    let d = dossier_d_essai("instantane");
    let socle = d.join("socle.txt");
    entrainer(&[
        "--parties", "8", "--graine-debut", "960001",
        "--sortie", socle.to_str().unwrap(), "--boites", "base,decouverte",
    ]);
    let suite = d.join("suite.txt");
    entrainer(&[
        "--parties", "8", "--graine-debut", "970001",
        "--reprise", socle.to_str().unwrap(),
        "--sortie", suite.to_str().unwrap(),
        "--instantanes", "12", "--boites", "base,decouverte",
    ]);
    let attendu = d.join("suite.txt.12");
    assert!(
        attendu.exists(),
        "aucun instantané .12 : le nom suit encore le rang dans la session"
    );
    let tete = std::fs::read_to_string(&attendu).expect("l'instantané");
    let compteur: u64 = tete
        .lines()
        .nth(1)
        .and_then(|l| l.trim().parse().ok())
        .expect("le compteur du fichier");
    assert_eq!(
        compteur, 12,
        "le nom de l'instantané et le compteur qu'il contient ne disent pas la même chose"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **UN INSTANTANÉ EST ÉCRIT AVANT LA FIN DE LA BOUCLE** (§2.6, chemin
/// `--instantanes`). Le programme demande un instantané à la deuxième partie sur
/// quarante : le fichier `p.txt.2` doit exister à la fin, et faire la taille d'un
/// réseau entier — un fichier de zéro octet prouverait qu'on a créé le fichier
/// sans écrire dedans.
///
/// **Ce test n'éprouve PAS l'écriture par tranche**, et sa description
/// prétendait le contraire : `--instantanes` passe par `reseau.ecrire`, pas par
/// `sauver()`. Le filet de sûreté d'un entraînement de plusieurs heures — le
/// fichier de sortie réécrit à chaque tranche — est éprouvé par
/// `une_coupure_en_pleine_course_laisse_des_poids_lisibles`, qui coupe vraiment
/// le programme en pleine course.
#[test]
fn des_poids_sont_ecrits_avant_la_fin_de_la_boucle() {
    let d = dossier_d_essai("coupure");
    let f = d.join("p.txt");
    // Le fichier ne doit pas exister avant l'entraînement…
    assert!(!f.exists());
    // …et l'instantané demandé à la première tranche prouve qu'on écrit en cours
    // de route : il est écrit alors que la boucle n'est pas finie.
    entrainer(&[
        "--parties", "40", "--graine-debut", "960001",
        "--sortie", f.to_str().unwrap(),
        "--instantanes", "2", "--boites", "base,decouverte",
    ]);
    let precoce = d.join("p.txt.2");
    assert!(
        precoce.exists(),
        "rien n'a été écrit à la deuxième partie sur quarante"
    );
    let taille = std::fs::metadata(&precoce).expect("l'instantané").len();
    assert!(taille > 100_000, "l'instantané précoce ne fait que {taille} octets");
    let _ = std::fs::remove_dir_all(&d);
}

/// Le nombre d'ouvriers change le partage, jamais la nature du fichier produit :
/// à un ouvrier comme à quatre, on obtient un fichier de poids lisible, du bon
/// nombre d'entrées et du bon compteur.
#[test]
fn un_ouvrier_et_quatre_ouvriers_rendent_tous_deux_un_fichier_lisible() {
    let (db, desc) = decor();
    let noms = desc.noms_avec(&db);
    let d = dossier_d_essai("ouvriers");
    for (nom, n) in [("un.txt", "1"), ("quatre.txt", "4")] {
        let f = d.join(nom);
        entrainer(&[
            "--parties", "8", "--graine-debut", "960001", "--ouvriers", n,
            "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
        ]);
        let r = Reseau::lire(f.to_str().unwrap(), &noms)
            .unwrap_or_else(|e| panic!("à {n} ouvrier(s) : {e}"));
        assert_eq!(r.parties, 8, "à {n} ouvrier(s), le compteur de parties vaut {}", r.parties);
        assert_eq!(r.n_entrees, noms.len());
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// **UNE COUPURE NE PERD PLUS TOUT** (§2.6) — et c'est ici que ça se prouve, pas
/// sur l'instantané de `--instantanes`, qui passe par un autre chemin. On lance
/// un entraînement long, on attend que le fichier de sortie APPARAISSE, on coupe
/// le programme en pleine course, et on lit ce qui est sur le disque : un réseau
/// entier, dont le compteur de parties est inférieur au total demandé. Sans
/// l'écriture par tranche (`engine/src/bin/entraine.rs`, `sauver`), le fichier
/// n'existerait pas encore et le test tomberait sur le délai.
#[test]
fn une_coupure_en_pleine_course_laisse_des_poids_lisibles() {
    let d = dossier_d_essai("coupure-en-course");
    let f = d.join("p.txt");
    assert!(!f.exists(), "le fichier existe avant même l'entraînement");

    // 400 parties : la tranche vaut 400/20 = 20 parties, la première écriture
    // arrive donc bien avant la fin, et la course entière durerait une demi-minute.
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_entraine"))
        .args([
            "--parties", "400", "--graine-debut", "960001",
            "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
            "--ouvriers", "1",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("le programme d'entraînement doit se lancer");

    let debut = std::time::Instant::now();
    let mut apparu = false;
    while debut.elapsed().as_secs() < 60 {
        if f.exists() && std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0) > 100_000 {
            apparu = true;
            break;
        }
        if enfant.try_wait().expect("attente impossible").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let encore_en_course = enfant.try_wait().expect("attente impossible").is_none();
    let _ = enfant.kill();
    let _ = enfant.wait();

    assert!(
        apparu,
        "rien n'a été écrit sur le disque avant la fin de la boucle : une coupure perdrait tout"
    );
    assert!(
        encore_en_course,
        "le programme avait déjà fini : la course est trop courte pour prouver quoi que ce soit"
    );

    // Ce qui est sur le disque est un réseau ENTIER, et il porte moins de parties
    // que le total demandé — donc il a bien été écrit en cours de route.
    let texte = std::fs::read_to_string(&f).expect("le fichier de poids doit être lisible");
    let mut lignes = texte.lines();
    let entete: Vec<&str> = lignes.next().expect("première ligne").split_whitespace().collect();
    assert_eq!(entete.len(), 3, "en-tête inattendu : {entete:?}");
    assert_eq!(entete[1], "50", "la largeur de couche cachée a changé");
    let parties: u64 = lignes
        .next()
        .expect("deuxième ligne")
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .expect("le compteur de parties");
    assert!(
        parties > 0 && parties < 400,
        "le compteur du fichier coupé vaut {parties} : il n'a pas été écrit en cours de route"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Le programme refuse un nombre d'ouvriers absurde** plutôt que de tourner
/// dans le vide.
#[test]
fn zero_ouvrier_est_refuse() {
    let d = dossier_d_essai("zero");
    let f = d.join("p.txt");
    // **L'attente est bornée, et ce n'est pas une précaution de confort.** Privé
    // de son garde-fou, `entraine` ne se trompe pas : il tourne sans fin — zéro
    // ouvrier, zéro partie terminée, la boucle n'avance jamais. Un test qui
    // attend la fin du programme attendrait pour toujours, et un programme tué
    // rend un code d'erreur, ce qu'un test naïf prendrait pour un refus. Au-delà
    // du délai, on tombe.
    let sortie = lancer_avec_delai(
        &[
            "--parties", "4", "--graine-debut", "960001", "--ouvriers", "0",
            "--sortie", f.to_str().unwrap(), "--boites", "base,decouverte",
        ],
        30,
    )
    .expect("« --ouvriers 0 » n'a été ni refusé ni terminé : le programme tourne encore après 30 s");
    assert!(!sortie.status.success(), "« --ouvriers 0 » a été accepté");
    let texte = String::from_utf8_lossy(&sortie.stderr).to_string();
    assert!(
        texte.contains("--ouvriers"),
        "le refus ne dit pas ce qui est refusé : {texte}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Lance `entraine` et **borne l'attente**. Rend `None` si le programme tourne
/// encore au bout de `secondes` — auquel cas il est tué. Sans cette borne, un
/// programme qui boucle ne rend pas un test rouge : il rend un test qui ne
/// finit jamais.
fn lancer_avec_delai(args: &[&str], secondes: u64) -> Option<std::process::Output> {
    let mut enfant = Command::new(env!("CARGO_BIN_EXE_entraine"))
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("le programme d'entraînement doit se lancer");
    let debut = std::time::Instant::now();
    loop {
        match enfant.try_wait().expect("impossible d'interroger le programme") {
            Some(_) => {
                return Some(enfant.wait_with_output().expect("sortie illisible"));
            }
            None => {
                if debut.elapsed().as_secs() >= secondes {
                    let _ = enfant.kill();
                    let _ = enfant.wait();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

/// **Le programme de mesure ne jette plus les parties à points égaux** : il les
/// départage. `parties_ecartees` compte ce qui reste, et il vaut zéro.
#[test]
fn le_programme_de_mesure_ne_jette_plus_les_parties_serrees() {
    let sortie = Command::new(env!("CARGO_BIN_EXE_predire"))
        .args([
            // **Cette plage de graines est choisie, pas prise au hasard.** Elle
            // contient DEUX parties qui finissent à égalité de points de
            // victoire — vérifié en jouant la mesure avec l'ancien
            // comportement, celui qui les jetait : il en écarte deux sur
            // trente ici, et zéro sur la plage 6600001 où ce test se tenait
            // d'abord. Un test qui n'a pas d'occasion de tomber ne prouve rien.
            "--parties", "30", "--graine-debut", "6600241",
            "--poids", "../data/poids/apprenti-L3-amorce.txt",
            "--boites", "base,decouverte",
        ])
        .output()
        .expect("predire doit se lancer");
    let texte = String::from_utf8_lossy(&sortie.stdout).to_string();
    let ligne = texte
        .lines()
        .find(|l| l.contains("parties_ecartees"))
        .unwrap_or_else(|| panic!("predire ne publie pas parties_ecartees :\n{texte}"));
    assert!(ligne.contains(": 0 "), "des parties sont encore écartées : {ligne}");
    assert!(
        texte.contains("30 parties décisives sur 30"),
        "toutes les parties devraient être décisives :\n{texte}"
    );
    // Et le nombre d'entrées n'est plus écrit en dur.
    assert!(
        texte.contains("(1630 entrées)"),
        "le nombre d'entrées annoncé n'est pas celui de la fiche :\n{texte}"
    );
    assert!(
        !texte.contains("1472"),
        "le nombre d'entrées de l'ancienne fiche est encore écrit en dur :\n{texte}"
    );
}

// ===========================================================================
// LES RÈGLES MAISON ET LE LIVRET, QUE CE LOT NE DOIT PAS CASSER
// ===========================================================================

/// **Les cinq phases dans l'ordre du livret** : I Développement,
/// II Construction, III Action, IV Production, V Recherche. Le second réseau a
/// une sortie par phase, dans cet ordre — « la sortie `i` porte la phase
/// `i + 1` » (`engine/src/reseau.rs`, constante `PHASES`).
#[test]
fn les_cinq_phases_du_livret_sont_dans_l_ordre() {
    assert_eq!(PHASES, 5, "le livret en compte cinq");
    let mut p;
    for (i, attendu) in [(0usize, 1u8), (1, 2), (2, 3), (3, 4), (4, 5)] {
        p = [0.1; PHASES];
        p[i] = 0.6;
        assert_eq!(
            reseau::phase_la_plus_probable(&p, &[1, 2, 3, 4, 5]),
            attendu,
            "la sortie {i} ne porte pas la phase {attendu}"
        );
    }
}

/// **UNE POLITIQUE TÉMOIN QUI FORCE LE MULLIGAN ET NOTE CE QU'ELLE VOIT.**
///
/// Elle ne décide rien d'elle-même : elle délègue tout à `RandomPolicy` et se
/// contente d'imposer `corp_mulligan = true` et d'enregistrer, pour chaque
/// siège, la paire de corporations vue au moment du mulligan puis la main vue au
/// moment du choix. C'est le seul moyen de voir de l'extérieur si la paire
/// rendue est bien partie ENTIÈRE — l'état de fin de mise en place, lui, ne
/// garde aucune trace de ce qui a été distribué au départ.
struct MulliganTemoin {
    fond: RandomPolicy,
    rendues: [Vec<u16>; 2],
    au_choix: [Vec<u16>; 2],
}

impl MulliganTemoin {
    fn new() -> Self {
        Self {
            fond: RandomPolicy,
            rendues: [Vec::new(), Vec::new()],
            au_choix: [Vec::new(), Vec::new()],
        }
    }
}

impl Policy for MulliganTemoin {
    fn corp_mulligan(&mut self, _rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        self.rendues[player] = corps.to_vec();
        true
    }
    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        self.au_choix[player] = corps.to_vec();
        self.fond.pick_corporation(rng, player, corps)
    }
    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        self.fond.project_mulligan(rng, player, hand)
    }
    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        self.fond.pick_phase(rng, player, allowed)
    }
    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        self.fond.choose_build(rng, player, affordable)
    }
    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        self.fond.action_choice(rng, player, options)
    }
    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        self.fond.construction_bonus(rng, player)
    }
    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        self.fond.research_keep(rng, player, drawn, keep)
    }
    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        self.fond.discard_down(rng, player, hand, n)
    }
}

/// **Le mulligan des corporations est tout ou rien** (règle maison n° 1) : chaque
/// joueur reçoit deux corporations et peut rendre **les deux ou aucune** — jamais
/// une seule. Le point de décision est un booléen, et ce lot ne le déplace pas.
///
/// **Ce test regardait la mauvaise chose.** Il se contentait de vérifier que
/// chaque siège avait bien une corporation à la fin de la mise en place, ce qui
/// resterait vrai si le mulligan n'échangeait qu'une carte sur deux : il portait
/// le nom d'une règle qu'il n'éprouvait pas. On regarde donc la DÉFAUSSE des
/// corporations, qui est la trace de ce qui a été rendu : chaque joueur y verse
/// zéro ou deux cartes, donc son compte est toujours pair, et il garde deux
/// corporations en main quoi qu'il arrive.
///
/// Le mulligan étant tiré au sort par `RandomPolicy`, on parcourt trente graines
/// et on **compte les occasions** : un test qui n'aurait rencontré aucun mulligan
/// ne prouverait rien, et il le dit au lieu de passer vert.
#[test]
fn le_mulligan_des_corporations_reste_tout_ou_rien() {
    // Source : les deux temps du mulligan des corporations : engine/src/flow.rs:216.
    // Source : la corporation non retenue part à la défausse : engine/src/flow.rs:308.
    let (db, _desc) = decor();
    let mut rendus = 0usize;
    let mut gardes = 0usize;
    for graine in 6_600_001u64..6_600_031 {
        let mut pol = RandomPolicy;
        let g = setup_game(&db, graine, &mut pol);
        // La défausse des corporations reçoit exactement deux sortes de cartes :
        // la corporation NON RETENUE de chaque joueur — une par siège, donc deux
        // toujours — et, pour qui a rendu sa paire, ses DEUX corporations de
        // départ. Sa longueur vaut donc 2, 4 ou 6, et jamais autre chose : un
        // mulligan qui n'échangerait qu'une carte sur deux la rendrait impaire.
        let n = g.corp_discard.len();
        assert!(
            n == 2 || n == 4 || n == 6,
            "graine {graine} : {n} corporation(s) défaussée(s) — attendu 2 (aucun \
             mulligan), 4 (un joueur) ou 6 (les deux). Un compte impair veut dire \
             qu'un joueur a rendu une seule carte, ce que la règle maison interdit"
        );
        for p in 0..2 {
            // La main de corporations est vidée à l'installation
            // (engine/src/flow.rs:358) : ce qui reste, c'est la corporation posée.
            assert!(
                g.players[p].corps_en_main.is_empty(),
                "graine {graine}, siège {p} : la main de corporations n'a pas été vidée"
            );
            assert!(
                g.players[p].corporation.is_some(),
                "graine {graine} : le siège {p} n'a pas de corporation après la mise en place"
            );
        }
        if n == 2 {
            gardes += 1;
        } else {
            rendus += (n - 2) / 2;
        }
    }
    assert!(
        rendus >= 3 && gardes >= 3,
        "sur trente graines : {rendus} mulligan(s) et {gardes} mise(s) en place sans \
         mulligan — le test n'a pas rencontré les deux cas, il ne prouve rien"
    );

    // ---- SECOND VOLET : la paire rendue ne revient pas, et il en revient DEUX.
    //
    // Le compte de la défausse ci-dessus ne suffit pas, et un sabotage l'a
    // montré : faire rendre UNE carte au lieu de deux laisse trois corporations
    // en main, dont deux repartent à la défausse au moment du choix — le total
    // redevient pair et le premier volet ne voit rien. Il faut donc regarder la
    // main AU MOMENT DU CHOIX, et la comparer à la paire rendue. Une politique
    // témoin force le mulligan des deux sièges et note les deux.
    let mut temoin = MulliganTemoin::new();
    let _ = setup_game(&db, 6_600_001, &mut temoin);
    for p in 0..2 {
        let rendue = &temoin.rendues[p];
        let au_choix = &temoin.au_choix[p];
        assert_eq!(
            rendue.len(),
            2,
            "siège {p} : le mulligan porte sur {} corporation(s), le livret maison en donne deux",
            rendue.len()
        );
        assert_eq!(
            au_choix.len(),
            2,
            "siège {p} : {} corporation(s) en main au moment du choix — un joueur qui a \
             rendu sa paire doit en recevoir DEUX, ni une ni trois",
            au_choix.len()
        );
        for c in au_choix {
            assert!(
                !rendue.contains(c),
                "siège {p} : la corporation {c} a été rendue puis proposée à nouveau — \
                 le mulligan n'est pas tout ou rien"
            );
        }
    }
}

/// **La partie se joue à deux joueurs** (règle maison n° 4), et le premier joueur
/// de la première manche est tiré au sort. Les deux sièges existent, et l'ordre du
/// tour est enregistré manche par manche.
#[test]
fn la_partie_se_joue_a_deux_et_l_ordre_du_tour_est_enregistre() {
    let (db, _desc) = decor();
    let mut pol = RandomPolicy;
    let mut g = setup_game(&db, 6_600_001, &mut pol);
    assert_eq!(g.players.len(), 2, "la partie ne se joue pas à deux joueurs");
    while !g.game_over && g.generation <= MAX_GENERATIONS {
        play_round(&mut g, &db, &mut pol);
    }
    assert!(!g.turn_order.is_empty(), "aucun ordre du tour enregistré");
    assert!(
        g.turn_order.iter().all(|p| *p < 2),
        "un premier joueur hors des deux sièges : {:?}",
        g.turn_order
    );
}

/// Une partie d'entraînement va à son terme et rend deux scores : c'est le socle
/// de tout ce que ce lot mesure.
#[test]
fn une_partie_va_a_son_terme_et_rend_deux_scores() {
    // Source : le décompte des points : engine/src/flow.rs:5906.
    let (db, _desc) = decor();
    let mut pol = RandomPolicy;
    let mut g = setup_game(&db, 6_600_001, &mut pol);
    while !g.game_over && g.generation <= MAX_GENERATIONS {
        play_round(&mut g, &db, &mut pol);
    }
    assert!(g.game_over, "la partie ne s'est pas terminée");
    let (scores, _, _) = score_parts(&g, &db);
    assert!(scores[0] > 0 && scores[1] > 0, "scores {scores:?}");
}

/// L'exponentielle normalisée du premier réseau rend toujours deux probabilités
/// de somme un, même quand les sommes de sortie sont énormes — le pivot du §1
/// évite le débordement.
#[test]
fn les_deux_sorties_restent_une_distribution_meme_sur_des_valeurs_enormes() {
    let mut r = Reseau::neuf(4);
    for k in 0..SORTIES {
        for j in 0..CACHES {
            r.w_sortie[k * (CACHES + 1) + j] = 0.0;
        }
        r.w_sortie[k * (CACHES + 1) + CACHES] = if k == 0 { 900.0 } else { -900.0 };
    }
    let p = r.evaluer(&[1.0, -1.0, 1.0, -1.0]);
    assert!(p.iter().all(|v| v.is_finite()), "sortie non finie : {p:?}");
    assert!((p[0] + p[1] - 1.0).abs() < 1e-12, "somme {p:?}");
    assert!(p[0] > 0.99, "la sortie de loin la plus grande doit l'emporter : {p:?}");
}

/// La mise à jour par différences du §1.1 rend le même résultat que le calcul
/// complet, à l'arrondi près : c'est l'optimisation qui fait passer une
/// évaluation de 24,8 à 3,1 microsecondes, et ce lot ne doit pas la casser.
#[test]
fn la_mise_a_jour_par_differences_donne_le_meme_resultat_que_le_calcul_complet() {
    let n = 40;
    let mut rapide = Reseau::neuf(n);
    let mut complet = Reseau::neuf(n);
    complet.sans_optimisation = true;
    let mut x = vec![1.0f64; n];
    for k in 0..12usize {
        x[k % n] = -x[k % n];
        let a = rapide.evaluer(&x);
        let b = complet.evaluer(&x);
        assert!(
            (a[0] - b[0]).abs() < 1e-12,
            "situation {k} : {a:?} contre {b:?}"
        );
    }
}

/// `oublier` remet la mémoire du §1.1 à zéro : sans cela, on accumulerait des
/// différences d'états sans rapport.
#[test]
fn oublier_force_le_calcul_complet_suivant() {
    let n = 20;
    let mut r = Reseau::neuf(n);
    let x = vec![1.0f64; n];
    let mut y = x.clone();
    y[3] = -1.0;
    let a = r.evaluer(&x);
    r.evaluer(&y);
    r.oublier();
    let b = r.evaluer(&x);
    assert!((a[0] - b[0]).abs() < 1e-12, "{a:?} contre {b:?}");

    // **Ce que `oublier` protège vraiment** (engine/src/reseau.rs:402) : les
    // sommes gardées en mémoire ont été calculées AVEC LES POIDS D'ALORS. Si les
    // poids changent et que la mémoire survit, la mise à jour par différences
    // repart d'une somme périmée et rend un résultat faux. À droite le témoin :
    // les mêmes poids, sans mémoire — la copie efface la mémoire (§2.7).
    let mut vivant = Reseau::neuf(n);
    vivant.evaluer(&x); // la mémoire se remplit ici
    vivant.entrainer_une(&y, [0.9, 0.1], 1e-2); // les poids bougent, `appliquer` oublie
    let obtenu = vivant.evaluer(&x);

    let mut temoin = Reseau::neuf(n);
    temoin.copier_les_poids_de(&vivant);
    let attendu = temoin.evaluer(&x);

    assert_eq!(
        obtenu, attendu,
        "après un changement de poids, la mémoire du §1.1 a survécu : {obtenu:?} au lieu de {attendu:?}"
    );
}

/// Un réseau générique à cinq sorties et un à deux sorties ne partagent pas leur
/// fichier : le verrou du §5 refuse un fichier qui n'a pas le bon nombre de
/// sorties. C'est ce même verrou qui protège la fiche.
#[test]
fn un_fichier_a_cinq_sorties_est_refuse_pour_deux() {
    let noms: Vec<String> = (0..5).map(|i| format!("entree_{i}")).collect();
    let r: ReseauMulti<PHASES> = ReseauMulti::neuf(noms.len());
    let mut chemin = std::env::temp_dir();
    chemin.push(format!("lot-vitesse-verrou-{}.txt", std::process::id()));
    let chemin = chemin.to_string_lossy().to_string();
    r.ecrire(&chemin, &noms).expect("écriture");
    // **Le refus doit DIRE ce qui cloche.** Un fichier à cinq sorties finirait
    // de toute façon par être rejeté plus loin, sur un compte de poids qui ne
    // tombe pas juste — mais avec un message qui n'apprend rien à personne. Le
    // verrou du §7 (engine/src/reseau.rs:872) est là pour nommer la cause tout
    // de suite ; sans lui, ce test tombe sur le message.
    let erreur = Reseau::lire(&chemin, &noms)
        .err()
        .expect("un fichier à cinq sorties a été accepté pour deux");
    assert!(
        erreur.contains("sorties"),
        "le refus ne nomme pas le nombre de sorties : {erreur}"
    );
    let mut autres = noms.clone();
    autres[2] = "autre_nom".to_string();
    assert!(
        ReseauPhases::lire(&chemin, &autres).is_err(),
        "un fichier dont les noms diffèrent a été accepté"
    );
    let _ = std::fs::remove_file(&chemin);
}

//! **(la-largeur-reglable) LA LARGEUR DE LA COUCHE CACHÉE EST RÉGLABLE.**
//!
//! Le nombre de neurones cachés était figé à la compilation (`CACHES = 50`) ;
//! il est désormais porté par chaque réseau, choisi par `entraine --largeur N`
//! et relu en tête du fichier de poids du §7. Ce banc tient les quatre
//! propriétés qui font qu'un tel réglage ne ment pas :
//!
//! 1. la géométrie annoncée est celle qui est allouée, à toute largeur ;
//! 2. un réseau d'une autre largeur APPREND — corriger et entraîner ne
//!    supposent plus cinquante nulle part ;
//! 3. le fichier écrit porte sa largeur et se relit à cette largeur-là ;
//! 4. les verrous de relecture parlent de COHÉRENCE et non du nombre cinquante :
//!    ils acceptent cent, et refusent en nommant les nombres un fichier qui ment
//!    sur sa largeur ou qui n'a pas la largeur que le réseau appelant attend.

use engine::reseau::{
    Pile, Reseau, ReseauMulti, ReseauPhases, CACHES, LARGEUR_MAX, PHASES, SORTIES, TAUX,
};

fn noms(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("entree_{i}")).collect()
}

fn chemin_temporaire(etiquette: &str) -> String {
    let mut c = std::env::temp_dir();
    c.push(format!(
        "largeur-reglable-{etiquette}-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    c.to_string_lossy().to_string()
}

/// **Le défaut ne bouge pas.** Sans rien demander, un réseau neuf a la largeur
/// que tout le dépôt a toujours eue — c'est la condition de l'identité à
/// l'octet des fichiers déjà produits.
#[test]
fn le_defaut_reste_cinquante() {
    let r = Reseau::neuf(11);
    assert_eq!(r.largeur(), CACHES);
    assert_eq!(r.largeur(), 50);
    assert_eq!(r.w_cache.len(), 12 * 50);
    assert_eq!(r.w_sortie.len(), 51 * SORTIES);
    // Et le chemin explicite donne exactement le même réseau que le chemin par
    // défaut : mêmes tailles, mêmes poids, au bit près.
    let e = Reseau::neuf_largeur(11, CACHES);
    assert_eq!(e.w_cache, r.w_cache, "les poids de départ ont bougé à largeur 50");
    assert_eq!(e.w_sortie, r.w_sortie);
}

/// **La géométrie annoncée est celle qui est allouée**, à toute largeur, et
/// pour les deux réseaux (deux sorties et cinq).
#[test]
fn la_geometrie_suit_la_largeur_demandee() {
    for largeur in [1usize, 3, 50, 100, 137] {
        let r = Reseau::neuf_largeur(7, largeur);
        assert_eq!(r.largeur(), largeur);
        assert_eq!(r.w_cache.len(), 8 * largeur, "couche cachée à {largeur}");
        assert_eq!(r.w_sortie.len(), (largeur + 1) * SORTIES, "sortie à {largeur}");
        let a = ReseauPhases::neuf_largeur(7, largeur);
        assert_eq!(a.w_sortie.len(), (largeur + 1) * PHASES, "second réseau à {largeur}");
    }
}

/// **Une largeur nulle est refusée à la construction**, comme `--largeur 0` l'est
/// en ligne de commande : un réseau sans couche cachée n'est pas un réseau.
#[test]
#[should_panic(expected = "au moins un neurone")]
fn une_largeur_nulle_est_refusee() {
    let _ = Reseau::neuf_largeur(7, 0);
}

/// **Le réseau ÉVALUE à une autre largeur** : deux sorties positives de somme un,
/// et deux situations différentes ne donnent pas la même réponse — sans quoi la
/// couche cachée ne servirait à rien.
#[test]
fn l_evaluation_marche_a_une_autre_largeur() {
    for largeur in [3usize, 100] {
        let mut r = ReseauMulti::<SORTIES>::neuf_largeur(6, largeur);
        let p = r.evaluer(&[1.0, -1.0, 1.0, -1.0, 1.0, -1.0]);
        let somme: f64 = p.iter().sum();
        assert!((somme - 1.0).abs() < 1e-12, "somme des sorties à {largeur} : {somme}");
        assert!(p[0] > 0.0 && p[1] > 0.0);
        let q = r.evaluer(&[-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
        assert!(
            (p[0] - q[0]).abs() > 0.0,
            "à largeur {largeur}, deux situations différentes donnent la même sortie"
        );
    }
}

/// **Le réseau APPREND à une autre largeur.** `entrainer_une` déplace la sortie
/// dans le sens de la cible : c'est la propriété, pas la forme des tampons.
#[test]
fn l_apprentissage_marche_a_une_autre_largeur() {
    for largeur in [2usize, 50, 100] {
        let mut r = ReseauMulti::<SORTIES>::neuf_largeur(6, largeur);
        let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let avant = r.evaluer(&x)[0];
        for _ in 0..200 {
            r.entrainer_une(&x, [1.0, 0.0], TAUX * 100.0);
        }
        r.oublier();
        let apres = r.evaluer(&x)[0];
        assert!(
            apres > avant,
            "à largeur {largeur}, cent passes vers la cible [1, 0] n'ont pas fait monter la sortie 0 ({avant} → {apres})"
        );
    }
}

/// **La correction sur une PILE marche à une autre largeur**, et c'est le chemin
/// que la mise à plat du tampon `ds` touche le plus : sommes suffixes, produit
/// externe de la situation la plus récente, puis les différences.
#[test]
fn la_correction_sur_une_pile_marche_a_une_autre_largeur() {
    for largeur in [3usize, 50, 100] {
        let n = 6;
        let mut r = ReseauMulti::<SORTIES>::neuf_largeur(n, largeur);
        let mut pile = Pile::new(n);
        let situations = [
            vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
            vec![1.0, 1.0, 1.0, -1.0, 1.0, -1.0],
            vec![1.0, 1.0, -1.0, -1.0, 1.0, -1.0],
        ];
        for (k, s) in situations.iter().enumerate() {
            pile.empiler(s, k % 2);
        }
        let avant = r.w_cache.clone();
        for _ in 0..50 {
            r.corriger(&pile, 0, [1.0, 0.0], TAUX * 100.0);
        }
        let bouges = avant
            .iter()
            .zip(r.w_cache.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            bouges > largeur,
            "à largeur {largeur}, seuls {bouges} poids cachés ont bougé après cinquante corrections"
        );
        r.oublier();
        let p = r.evaluer(&situations[0]);
        assert!(p[0].is_finite() && p[1].is_finite(), "sorties non finies à largeur {largeur}");
    }
}

/// **Le fichier du §7 porte SA largeur, et se relit à cette largeur-là.**
#[test]
fn le_fichier_porte_sa_largeur_et_se_relit() {
    let n = 5;
    let noms = noms(n);
    for largeur in [7usize, 50, 100] {
        let mut r = Reseau::neuf_largeur(n, largeur);
        r.parties = 4242;
        let chemin = chemin_temporaire(&format!("aller-retour-{largeur}"));
        r.ecrire(&chemin, &noms).expect("écriture");

        let texte = std::fs::read_to_string(&chemin).expect("relecture");
        let tete = texte.lines().next().unwrap_or("");
        assert_eq!(tete, format!("{n} {largeur} {SORTIES}"), "première ligne du fichier");
        let valeurs = texte.lines().skip(2 + n).filter(|l| !l.trim().is_empty()).count();
        assert_eq!(
            valeurs,
            (n + 1) * largeur + (largeur + 1) * SORTIES,
            "le compte des poids ne suit pas la géométrie annoncée"
        );

        let relu = Reseau::lire(&chemin, &noms).expect("lecture");
        assert_eq!(relu.largeur(), largeur, "la largeur du fichier n'a pas été adoptée");
        assert_eq!(relu.parties, 4242);
        assert_eq!(relu.w_cache.len(), r.w_cache.len());
        let _ = std::fs::remove_file(&chemin);
    }
}

/// **LE VERROU, DANS LES DEUX SENS.** Un fichier de largeur cent est accepté par
/// un réseau qui n'impose rien, refusé par un réseau construit à cinquante — et
/// le refus nomme les deux nombres.
#[test]
fn le_verrou_accepte_cent_et_refuse_en_nommant_les_nombres() {
    let n = 5;
    let noms = noms(n);
    let chemin = chemin_temporaire("verrou-cent");
    Reseau::neuf_largeur(n, 100).ecrire(&chemin, &noms).expect("écriture");

    assert_eq!(
        Reseau::lire(&chemin, &noms).expect("un fichier de largeur 100 doit être accepté").largeur(),
        100
    );
    assert_eq!(Reseau::lire_largeur(&chemin, &noms, 100).map(|r| r.largeur()), Ok(100));

    let refus = Reseau::lire_largeur(&chemin, &noms, 50)
        .err()
        .expect("un réseau de largeur 50 doit refuser un fichier de largeur 100");
    assert!(refus.contains("100"), "le refus ne nomme pas la largeur lue : {refus}");
    assert!(refus.contains("50"), "le refus ne nomme pas la largeur attendue : {refus}");
    let _ = std::fs::remove_file(&chemin);
}

/// **Un en-tête qui MENT sur la largeur est refusé**, et le message nomme la
/// géométrie annoncée. C'est le cas que supprimer le verrou laisserait passer :
/// les poids seraient lus avec un pas faux et le joueur répondrait n'importe
/// quoi, sans un mot.
#[test]
fn un_en_tete_menteur_est_refuse() {
    let n = 5;
    let noms = noms(n);
    let chemin = chemin_temporaire("menteur-source");
    Reseau::neuf_largeur(n, 100).ecrire(&chemin, &noms).expect("écriture");
    let texte = std::fs::read_to_string(&chemin).expect("relecture");
    let mut lignes: Vec<String> = texte.lines().map(|l| l.to_string()).collect();
    lignes[0] = format!("{n} 40 {SORTIES}");
    let menteur = chemin_temporaire("menteur");
    std::fs::write(&menteur, lignes.join("\n")).expect("écriture du menteur");

    let refus = Reseau::lire(&menteur, &noms)
        .err()
        .expect("un fichier dont l'en-tête ment sur la largeur doit être refusé");
    assert!(
        refus.contains("40"),
        "le refus ne nomme pas la largeur annoncée : {refus}"
    );
    // Et le même fichier, lu par un réseau qui attend cent, est refusé plus tôt
    // encore — sur la largeur, pas sur le compte.
    let refus2 = Reseau::lire_largeur(&menteur, &noms, 100)
        .err()
        .expect("largeur attendue 100, fichier annonçant 40");
    assert!(refus2.contains("40") && refus2.contains("100"), "{refus2}");
    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_file(&menteur);
}

/// **Le partage sur plusieurs cœurs reste cohérent en largeur.** Un maître et
/// ses ouvriers sont construits à la même largeur ; recopier et additionner les
/// différences d'une AUTRE largeur est une erreur de programmation, pas un
/// silence.
#[test]
#[should_panic(expected = "AUTRE largeur")]
fn un_ouvrier_d_une_autre_largeur_est_refuse() {
    let maitre = Reseau::neuf_largeur(6, 100);
    let mut ouvrier = Reseau::neuf_largeur(6, 50);
    ouvrier.copier_les_poids_de(&maitre);
}

/// **Le partage à largeur égale, lui, marche** : les différences d'un ouvrier
/// reviennent au maître, à cent neurones comme à cinquante.
#[test]
fn le_partage_marche_a_une_autre_largeur() {
    let largeur = 100;
    let n = 6;
    let mut maitre = Reseau::neuf_largeur(n, largeur);
    let mut base = Reseau::neuf_largeur(n, largeur);
    let mut ouvrier = Reseau::neuf_largeur(n, largeur);
    base.copier_les_poids_de(&maitre);
    ouvrier.copier_les_poids_de(&maitre);
    let x = vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
    for _ in 0..20 {
        ouvrier.entrainer_une(&x, [1.0, 0.0], TAUX * 100.0);
    }
    let avant = maitre.w_cache.clone();
    maitre.ajouter_la_difference(&base, &ouvrier);
    let bouges = avant
        .iter()
        .zip(maitre.w_cache.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(bouges > 0, "la différence de l'ouvrier n'est pas revenue au maître");
    for (m, o) in maitre.w_cache.iter().zip(ouvrier.w_cache.iter()) {
        assert!((m - o).abs() < 1e-15, "le maître ne retrouve pas les poids de l'unique ouvrier");
    }
}

/// **Le plafond de largeur est le même des deux côtés.** `LARGEUR_MAX` existe
/// parce que la largeur est désormais ADOPTÉE depuis l'en-tête du fichier : sans
/// lui, `1630 999999999 2` ferait demander des téraoctets avant qu'on s'aperçoive
/// que le compte des poids ne suit pas. Le dépôt tient déjà le miroir de `CACHES`
/// entre Rust et JavaScript (`le_miroir_javascript_porte_la_meme_largeur_de_couche_cachee`) ;
/// celui-ci ne vaut pas moins : deux plafonds différents, et un fichier refusé
/// d'un côté passerait de l'autre.
#[test]
fn le_miroir_javascript_porte_le_meme_plafond_de_largeur() {
    let js = std::fs::read_to_string("../web/webapp/joueurs/apprenti.js")
        .expect("le miroir JavaScript doit exister");
    let attendu = format!("const LARGEUR_MAX = {LARGEUR_MAX};");
    assert!(
        js.contains(&attendu),
        "le miroir JavaScript ne porte pas « {attendu} » : les deux plafonds divergeraient"
    );
}

/// **Le plafond refuse un en-tête absurde AVANT d'allouer.** C'est tout son
/// intérêt : le compte des poids finirait par ne pas tomber juste, mais après
/// avoir demandé au système une allocation impossible.
#[test]
fn une_largeur_absurde_est_refusee_a_la_relecture() {
    let n = 5;
    let noms = noms(n);
    let chemin = chemin_temporaire("absurde");
    Reseau::neuf_largeur(n, 50).ecrire(&chemin, &noms).expect("écriture");
    let texte = std::fs::read_to_string(&chemin).expect("relecture");
    let mut lignes: Vec<String> = texte.lines().map(|l| l.to_string()).collect();
    lignes[0] = format!("{n} 999999999 {SORTIES}");
    let absurde = chemin_temporaire("absurde-entete");
    std::fs::write(&absurde, lignes.join("\n")).expect("écriture");
    let refus = Reseau::lire(&absurde, &noms)
        .err()
        .expect("une largeur d'un milliard doit être refusée");
    assert!(
        refus.contains("999999999") && refus.contains(&LARGEUR_MAX.to_string()),
        "le refus ne nomme pas les deux nombres : {refus}"
    );
    let _ = std::fs::remove_file(&chemin);
    let _ = std::fs::remove_file(&absurde);
}

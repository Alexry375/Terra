//! **(la-fiche-que-l-ia-regarde) LES SIX DÉFAUTS DE LA FICHE QUE LE RÉSEAU
//! REGARDE.**
//!
//! Un correctif sans test qui échoue sur le code d'avant n'est pas un
//! correctif : c'est une affirmation. Ce fichier tient les six défauts du lot —
//! D3, D4, 2.8, 2.9, 2.10, 2.12 — et les invariants que la fiche ne doit pas
//! perdre en chemin (§ 3.1 valeurs ±1, § 3.2 point de vue, § 3.3 secret de la
//! main d'en face, § 3.5 seuils mesurés, § 3.7 verrou des noms).
//!
//! **Il atteint `engine::description::` par la BIBLIOTHÈQUE**, et non par une
//! déclaration de chemin recopiée dans le fichier de tests : c'est le défaut D4
//! (`docs/AUDIT_MOTEUR.md`, § D4) — les cinq fichiers de la couche qui décrit et
//! qui décide étaient déclarés à l'intérieur de chaque programme exécutable, si
//! bien qu'aucun test d'intégration ne pouvait les toucher, et que le même code
//! compilé deux fois donnait deux types différents.
//!
//! **Ce que « le rouge » veut dire ici.** Chaque groupe de tests a été éprouvé
//! sur une copie du dépôt où le correctif correspondant est débranché, un à la
//! fois ; le relevé est dans `outputs/result.md` § Verification.

use std::collections::{BTreeMap, HashMap};

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Tag, JOKER_TAG_CHOICES, TAG_COUNT};
use engine::flow::{award_value, play_round, score_breakdown, setup_game};
use engine::policy::RandomPolicy;
use engine::state::{GameState, AWARD_POOL, NUM_PLAYERS};

use engine::description;
use engine::description::{
    ecarts, ressources_posees, resume_main, Description, Tampons, NOMS_ECARTS, PRIX_MAIN_VIDE,
    S_ECARTS, S_MAIN_BADGES, S_MAIN_COULEURS, S_MAIN_PRIX_MIN, S_MAIN_PRIX_TOTAL, S_MAIN_PV,
    S_RESSOURCES_POSEES, S_SCORE,
};

const CARTES: &str = "../data/cards.json";

/// La taille de la fiche AVANT ce lot, telle que `decrire --noms` la donnait sur
/// le commit d'entrée (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12 : 257 cartes
/// décrites, dont 11 jamais distribuées).
const TAILLE_AVANT: usize = 1472;

/// L'échelle de score d'avant le lot, gardée ici pour montrer ce qu'elle
/// saturait (`docs/AUDIT_ENTRAINEMENT.md`, § 2.9).
const S_SCORE_AVANT: [i64; 8] = [5, 7, 10, 14, 20, 27, 36, 51];

struct Banc {
    db: CardsDb,
    desc: Description,
}

fn banc_boites(boites: &str) -> Banc {
    let b = BoiteSet::parse(boites).expect("boîtes");
    let db = CardsDb::load_boites(CARTES, b).expect("cartes");
    let desc = Description::new(&db);
    Banc { db, desc }
}

fn banc() -> Banc {
    banc_boites("base,decouverte")
}

fn partie(b: &Banc, graine: u64) -> GameState {
    let mut p = RandomPolicy;
    setup_game(&b.db, graine, &mut p)
}

/// L'état vivant **à la première question de la partie**, celle de l'échange de
/// corporations (règle maison n° 1, `flow.rs`, étape 2 de `setup_game`). C'est
/// le seul moment où la paire tenue en main existe : après, elle est vidée par
/// `flow::install_corporation_with`. `rejeu::rejouer` avec zéro décision rend
/// exactement cet état-là (`docs/AUDIT_MOTEUR.md`, § D3).
fn au_premier_choix(b: &Banc, graine: u64) -> (GameState, usize) {
    let (game, joueur) = engine::rejeu::rejouer(&b.db, graine, vec![]).expect("rejeu");
    (game, joueur.expect("la partie doit s'arrêter sur une question"))
}

fn fiche(b: &Banc, game: &GameState, siege: usize) -> Vec<f64> {
    let mut out = Vec::new();
    let mut t = Tampons::new(&b.desc);
    b.desc.decrire(game, &b.db, siege, &mut out, &mut t);
    out
}

fn noms(b: &Banc) -> Vec<String> {
    b.desc.noms_avec(&b.db)
}

/// La fiche rangée par NOM — la façon dont un humain la lit, et la seule qui
/// permette de dire « cette case-ci a bougé ».
fn fiche_nommee(b: &Banc, game: &GameState, siege: usize) -> HashMap<String, f64> {
    noms(b).into_iter().zip(fiche(b, game, siege)).collect()
}

/// Des situations de VRAIES parties : la mise en place, puis manche après
/// manche, les deux sièges à chaque fois.
fn situations(b: &Banc, parties: u64, graine_debut: u64) -> Vec<Vec<f64>> {
    let mut out = Vec::new();
    for g in 0..parties {
        let mut p = RandomPolicy;
        let mut game = setup_game(&b.db, graine_debut + g, &mut p);
        for siege in 0..NUM_PLAYERS {
            out.push(fiche(b, &game, siege));
        }
        let mut tours = 0;
        while !game.game_over && tours < 40 {
            play_round(&mut game, &b.db, &mut p);
            for siege in 0..NUM_PLAYERS {
                out.push(fiche(b, &game, siege));
            }
            tours += 1;
        }
    }
    out
}

/// Les noms collectés sur un ÉTAT donné et depuis un SIÈGE donné, par le
/// parcours lui-même (§ 3.7 : la même traversée nourrit les valeurs et les
/// noms). `noms_avec` ne sait faire ça que sur sa propre partie neuve, du siège
/// 0 — ce qui ne prouve rien sur les autres états.
fn noms_de(b: &Banc, game: &GameState, siege: usize) -> Vec<String> {
    let mut n = engine::description::Noms { out: Vec::new() };
    let mut t = Tampons::new(&b.desc);
    b.desc.parcours(game, &b.db, siege, &mut n, &mut t);
    n.out
}

/// Les rangs des cases dont le nom commence par `prefixe`.
fn rangs(noms: &[String], prefixe: &str) -> Vec<usize> {
    noms.iter()
        .enumerate()
        .filter(|(_, n)| n.starts_with(prefixe))
        .map(|(i, _)| i)
        .collect()
}

/// Combien de cases, parmi celles-là, ne changent JAMAIS de valeur.
fn figees(situations: &[Vec<f64>], rangs: &[usize]) -> usize {
    let mut n = 0;
    for r in rangs {
        let v = situations[0][*r];
        if situations.iter().all(|s| s[*r] == v) {
            n += 1;
        }
    }
    n
}

/// Les cases d'un thermomètre pour une quantité donnée.
fn cases(seuils: &[i64], q: i64) -> Vec<bool> {
    seuils.iter().map(|s| q > *s).collect()
}

// ===========================================================================
// A. La structure de la fiche — § 3.1, § 3.4, § 3.7
// ===========================================================================

#[test]
fn a01_la_taille_annoncee_est_le_nombre_de_noms() {
    let b = banc();
    assert_eq!(b.desc.taille, noms(&b).len());
}

#[test]
fn a02_aucun_nom_en_double() {
    let b = banc();
    let mut vus: BTreeMap<String, usize> = BTreeMap::new();
    for n in noms(&b) {
        *vus.entry(n).or_insert(0) += 1;
    }
    let doubles: Vec<_> = vus.iter().filter(|(_, v)| **v > 1).collect();
    assert!(doubles.is_empty(), "noms en double : {doubles:?}");
}

/// § 3.1 : toute entrée vaut +1 ou −1, jamais une quantité brute
/// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.1 — la convention que le réseau attend).
#[test]
fn a03_toutes_les_valeurs_valent_plus_ou_moins_un() {
    let b = banc();
    let game = partie(&b, 700001);
    for (i, v) in fiche(&b, &game, 0).iter().enumerate() {
        assert!(*v == 1.0 || *v == -1.0, "case {i} vaut {v}, § 3.1 l'interdit");
    }
}

#[test]
fn a04_la_fiche_a_la_longueur_annoncee() {
    let b = banc();
    let game = partie(&b, 700002);
    assert_eq!(fiche(&b, &game, 0).len(), b.desc.taille);
    assert_eq!(fiche(&b, &game, 1).len(), b.desc.taille);
}

#[test]
fn a05_les_deux_sieges_lisent_la_meme_table_de_noms() {
    let b = banc();
    let mut game = partie(&b, 700003);
    // Un état vécu, pas une mise en place : les deux joueurs ont des mains, des
    // cartes posées et des ressources différentes.
    game.players[0].hand = b.desc.projets[0..7].to_vec();
    game.players[1].hand = b.desc.projets[20..31].to_vec();
    game.players[0].played = b.desc.projets[40..45].to_vec();
    let n0 = noms_de(&b, &game, 0);
    let n1 = noms_de(&b, &game, 1);
    // Nom par nom, et pas seulement en longueur : c'est le verrou du § 3.7 qui
    // se joue là — un fichier de poids sert aux deux sièges.
    assert_eq!(n0, n1, "les deux sièges ne lisent pas la même table de noms");
    assert_eq!(n0, noms(&b), "la table du siège 0 diffère de celle du réseau");
    assert_eq!(n0.len(), fiche(&b, &game, 1).len());
}

#[test]
fn a06_la_fiche_est_deterministe() {
    let b = banc();
    let un = partie(&b, 700004);
    let autre = partie(&b, 700104);
    // Comparer `fiche(x)` à `fiche(x)` ne prouverait rien : c'est la même
    // fonction pure sur la même entrée. Ce qui peut vraiment casser, c'est le
    // TAMPON partagé (`Tampons`, quatre tableaux de booléens réutilisés d'une
    // évaluation à l'autre) : s'il n'est pas remis à zéro, la fiche d'une partie
    // garde la trace de la précédente. On décrit donc une AUTRE partie entre
    // les deux lectures, avec le même tampon.
    let mut t = Tampons::new(&b.desc);
    let mut out = Vec::new();
    b.desc.decrire(&un, &b.db, 0, &mut out, &mut t);
    let premiere = out.clone();
    b.desc.decrire(&autre, &b.db, 1, &mut out, &mut t);
    let entre_deux = out.clone();
    b.desc.decrire(&un, &b.db, 0, &mut out, &mut t);
    assert_eq!(premiere, out, "le tampon garde la trace de la partie précédente");
    assert_ne!(premiere, entre_deux, "deux parties différentes rendent la même fiche");
}

#[test]
fn a07_la_table_des_noms_ne_depend_pas_de_l_etat() {
    let b = banc();
    // Le parcours des noms est fait sur une partie neuve, graine 0 : rien ne
    // garantit tout seul qu'il rende la même table sur un état vécu. Deux
    // appels à `noms()` ne le prouveraient pas — c'est deux fois le même
    // calcul. On collecte donc les noms sur deux parties JOUÉES, différentes
    // l'une de l'autre (§ 3.7, le verrou).
    let mut p = RandomPolicy;
    let mut un = setup_game(&b.db, 700006, &mut p);
    let mut autre = setup_game(&b.db, 700007, &mut p);
    for _ in 0..6 {
        play_round(&mut un, &b.db, &mut p);
        play_round(&mut autre, &b.db, &mut p);
    }
    assert_ne!(fiche(&b, &un, 0), fiche(&b, &autre, 0), "les deux parties sont identiques");
    assert_eq!(noms_de(&b, &un, 0), noms_de(&b, &autre, 1));
    assert_eq!(noms_de(&b, &un, 0), noms(&b));
}

/// Les six défauts du lot sont listés dans `docs/AUDIT_ENTRAINEMENT.md` (2.8,
/// 2.9, 2.10, 2.12) et `docs/AUDIT_MOTEUR.md` (D3, D4).
#[test]
fn a08_la_fiche_a_change_de_taille_par_rapport_a_celle_d_avant() {
    let b = banc();
    assert_ne!(
        b.desc.taille, TAILLE_AVANT,
        "la fiche fait toujours {TAILLE_AVANT} cases : aucun des six défauts n'est réparé"
    );
}

#[test]
fn a09_le_verrou_des_noms_couvre_toute_la_fiche() {
    let b = banc();
    let n = noms(&b);
    // La même égalité qu'`a04`, mais sur des états VÉCUS et des deux sièges :
    // une case dont l'existence dépendrait de l'état (une carte en main
    // décrite seulement quand elle y est, par exemple) passerait `a04` et
    // ferait sauter le verrou en cours de partie.
    let mut p = RandomPolicy;
    let mut game = setup_game(&b.db, 700005, &mut p);
    let mut vus = 0;
    for _ in 0..10 {
        play_round(&mut game, &b.db, &mut p);
        for siege in 0..NUM_PLAYERS {
            assert_eq!(n.len(), fiche(&b, &game, siege).len(), "un nom par case");
            assert_eq!(n, noms_de(&b, &game, siege), "la table des noms a bougé en cours de partie");
            vus += 1;
        }
    }
    assert_eq!(vus, 20);
}

#[test]
fn a10_aucun_nom_vide() {
    let b = banc();
    for n in noms(&b) {
        assert!(!n.trim().is_empty(), "une case sans nom ne peut pas être verrouillée");
    }
}

// ===========================================================================
// B. 2.12 — les onze cartes jamais distribuées
// ===========================================================================

#[test]
fn b01_toutes_les_cartes_decrites_sont_dans_la_pioche() {
    let b = banc();
    for id in b.desc.projets.iter() {
        assert!(
            b.db.projects[*id as usize].in_deck,
            "la carte {id} est décrite alors qu'elle n'est jamais distribuée \
             (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12)"
        );
    }
}

#[test]
fn b02_toutes_les_cartes_de_la_pioche_sont_decrites() {
    let b = banc();
    for (i, c) in b.db.projects.iter().enumerate() {
        if c.in_deck {
            assert!(b.desc.projets.contains(&(i as u16)), "la carte {i} est distribuée sans être décrite");
        }
    }
}

#[test]
fn b03_des_cartes_portent_un_nom_de_boite_sans_etre_distribuees() {
    let b = banc();
    // C'est la mesure du défaut lui-même : si ce compte était nul, il n'y aurait
    // rien à réparer (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12 : 11 cartes).
    let fantomes = b
        .db
        .projects
        .iter()
        .filter(|c| c.boite.is_some() && !c.in_deck)
        .count();
    assert!(fantomes > 0, "aucune carte fantôme : le défaut 2.12 n'existe pas dans ces données");
}

#[test]
fn b04_aucune_carte_fantome_n_est_decrite() {
    let b = banc();
    for (i, c) in b.db.projects.iter().enumerate() {
        if c.boite.is_some() && !c.in_deck {
            assert!(
                !b.desc.projets.contains(&(i as u16)),
                "la carte fantôme {i} occupe encore quatre cases mortes"
            );
        }
    }
}

/// Quatre drapeaux par carte : dans ma main, posée par moi, posée par lui,
/// défaussée (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12).
#[test]
fn b05_quatre_cases_par_projet_decrit() {
    let b = banc();
    let n = rangs(&noms(&b), "projet").len();
    assert_eq!(n, b.desc.projets.len() * 4);
}

#[test]
fn b06_la_table_depend_de_la_composition() {
    let base = banc_boites("base");
    let tout = banc_boites("base,decouverte");
    // La contrepartie assumée du 2.12 : la table n'est plus indépendante des
    // boîtes, et c'est écrit noir sur blanc dans `description.rs`.
    assert_ne!(base.desc.projets.len(), tout.desc.projets.len());
    assert_ne!(base.desc.taille, tout.desc.taille);
}

#[test]
fn b07_la_boite_de_base_est_incluse_dans_la_composition_complete() {
    let base = banc_boites("base");
    let tout = banc_boites("base,decouverte");
    for id in base.desc.projets.iter() {
        assert!(tout.desc.projets.contains(id), "la carte {id} disparaît quand on ajoute une boîte");
    }
}

#[test]
fn b08_les_cases_projet_ne_sont_pas_toutes_figees() {
    let b = banc();
    let s = situations(&b, 3, 700100);
    let r = rangs(&noms(&b), "projet");
    let f = figees(&s, &r);
    assert!(
        f < r.len() / 2,
        "{f} cases projet figées sur {} : la fiche décrit encore des cartes mortes",
        r.len()
    );
}

#[test]
fn b09_les_quatre_cases_d_un_projet_portent_les_quatre_lieux() {
    let b = banc();
    let n = noms(&b);
    let id = b.desc.projets[0];
    for suffixe in ["_main", "_pose_moi", "_pose_adv", "_defausse"] {
        assert!(n.contains(&format!("projet{id}{suffixe}")), "case projet{id}{suffixe} absente");
    }
}

#[test]
fn b10_une_carte_de_ma_main_allume_sa_case() {
    let b = banc();
    let mut game = partie(&b, 700006);
    let id = b.desc.projets[5];
    game.players[0].hand = vec![id];
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f[&format!("projet{id}_main")], 1.0);
}

/// La main d'en face est cachée : le livret de mise en place ne la montre à
/// personne (`docs/regles/livret-base.md`).
#[test]
fn b11_une_carte_de_la_main_adverse_n_allume_aucune_case_de_ma_fiche() {
    let b = banc();
    let mut game = partie(&b, 700007);
    let id = b.desc.projets[7];
    game.players[0].hand = Vec::new();
    game.players[1].hand = vec![id];
    let f = fiche_nommee(&b, &game, 0);
    // § 3.3 : de l'adversaire, la fiche ne lit que le NOMBRE de cartes en main.
    assert_eq!(f[&format!("projet{id}_main")], -1.0);
}

// ===========================================================================
// C. D3 — les corporations tenues en main
// ===========================================================================

/// `docs/AUDIT_MOTEUR.md`, § D3 : les corporations tenues en main ne figuraient
/// nulle part dans la fiche.
#[test]
fn c01_une_case_ma_main_par_corporation() {
    let b = banc();
    let n = noms(&b);
    let k = n.iter().filter(|x| x.ends_with("_ma_main")).count();
    assert_eq!(k, b.desc.corporations.len());
    assert!(k >= 16, "seulement {k} corporations décrites");
}

#[test]
fn c02_aucune_case_de_corporation_tenue_pour_l_adversaire() {
    let b = banc();
    for n in noms(&b) {
        if n.contains("_ma_main") {
            assert!(
                !n.starts_with("adv_") && !n.contains("_adv"),
                "{n} : la paire tenue par l'adversaire est cachée"
            );
        }
    }
}

#[test]
fn c03_la_paire_entre_dans_l_etat_des_la_mise_en_place() {
    let b = banc();
    // (`docs/AUDIT_MOTEUR.md`, § D3) : sans ce champ, la fiche évaluée au moment
    // de l'échange était vide et les deux options se ressemblaient. Le piège est
    // dans le MOMENT : le champ doit être écrit AVANT que la question soit
    // posée. On regarde donc l'état à la question elle-même, pas une partie
    // mise en place jusqu'au bout — où la corporation est toujours installée et
    // où l'assertion ne dirait plus rien.
    let (game, _) = au_premier_choix(&b, 700008);
    for p in 0..NUM_PLAYERS {
        assert!(
            game.players[p].corporation.is_none(),
            "le joueur {p} a déjà sa corporation : la question n'est plus posée"
        );
        assert_eq!(
            game.players[p].corps_en_main.len(),
            2,
            "le joueur {p} ne tient pas ses deux corporations au moment du choix"
        );
    }
}

#[test]
fn c04_la_paire_est_videe_a_l_installation() {
    let b = banc();
    let game = partie(&b, 700009);
    for p in 0..NUM_PLAYERS {
        if game.players[p].corporation.is_some() {
            assert!(
                game.players[p].corps_en_main.is_empty(),
                "le joueur {p} tient encore des corporations après en avoir installé une"
            );
        }
    }
}

#[test]
fn c05_deux_paires_differentes_donnent_deux_fiches_differentes() {
    let b = banc();
    let mut game = partie(&b, 700010);
    game.players[0].corporation = None;
    game.players[0].corps_en_main = vec![0, 1];
    let a = fiche(&b, &game, 0);
    game.players[0].corps_en_main = vec![2, 3];
    let c = fiche(&b, &game, 0);
    // C'EST LE DÉFAUT D3 : avant ce lot, les deux options de l'échange de
    // corporations produisaient exactement le même vecteur, donc la même note,
    // et le réseau tranchait par la marge (`docs/AUDIT_MOTEUR.md`, § D3).
    assert_ne!(a, c, "deux paires différentes décrivent encore la même situation");
}

#[test]
fn c06_la_fiche_ne_montre_pas_la_paire_de_l_adversaire() {
    let b = banc();
    let mut game = partie(&b, 700011);
    game.players[1].corporation = None;
    game.players[1].corps_en_main = vec![0, 1];
    let a = fiche(&b, &game, 0);
    game.players[1].corps_en_main = vec![2, 3];
    let c = fiche(&b, &game, 0);
    assert_eq!(a, c, "ma fiche change quand la paire CACHÉE de l'adversaire change");
}

#[test]
fn c07_la_case_ma_main_designe_la_corporation_tenue() {
    let b = banc();
    let mut game = partie(&b, 700012);
    game.players[0].corporation = None;
    game.players[0].corps_en_main = vec![0];
    let nom = b.db.corporations[0].name.clone();
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f[&format!("corpo_{nom}_ma_main")], 1.0);
}

#[test]
fn c08_les_autres_corporations_restent_a_moins_un() {
    let b = banc();
    let mut game = partie(&b, 700013);
    game.players[0].corporation = None;
    game.players[0].corps_en_main = vec![0];
    let tenue = b.db.corporations[0].name.clone();
    let f = fiche_nommee(&b, &game, 0);
    let mut allumees = 0;
    for nom in b.desc.corporations.iter() {
        if f[&format!("corpo_{nom}_ma_main")] == 1.0 {
            allumees += 1;
            assert_eq!(*nom, tenue);
        }
    }
    assert_eq!(allumees, 1);
}

#[test]
fn c09_main_vide_de_corporations_donne_seize_moins_un() {
    let b = banc();
    let mut game = partie(&b, 700014);
    game.players[0].corps_en_main = Vec::new();
    let f = fiche_nommee(&b, &game, 0);
    for nom in b.desc.corporations.iter() {
        assert_eq!(f[&format!("corpo_{nom}_ma_main")], -1.0);
    }
}

/// Une corporation installée est posée face visible sur la table
/// (`docs/regles/livret-base.md`, mise en place) : elle est publique.
#[test]
fn c10_la_corporation_installee_reste_publique_des_deux_cotes() {
    let b = banc();
    let n = noms(&b);
    for nom in b.desc.corporations.iter() {
        assert!(n.contains(&format!("corpo_{nom}_moi")));
        assert!(n.contains(&format!("corpo_{nom}_adv")));
    }
}

#[test]
fn c11_la_corporation_de_l_adversaire_est_lue_une_fois_installee() {
    let b = banc();
    let mut game = partie(&b, 700015);
    game.players[1].corporation = Some(3);
    let nom = b.db.corporations[3].name.clone();
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f[&format!("corpo_{nom}_adv")], 1.0);
}

#[test]
fn c12_les_cases_de_paire_tenue_vivent_pendant_la_mise_en_place() {
    let b = banc();
    // Ces cases ne servent qu'AVANT l'installation, au point de décision de
    // l'échange : c'est donc là qu'il faut les regarder allumées. Sur quarante
    // parties, la fiche du joueur qui choisit doit porter exactement deux cases
    // `corpo_…_ma_main` à +1 — ses deux corporations — et aucune autre.
    for g in 0..40u64 {
        let (game, joueur) = au_premier_choix(&b, 700200 + g);
        let f = fiche_nommee(&b, &game, joueur);
        let allumees: Vec<&String> = f
            .keys()
            .filter(|k| k.ends_with("_ma_main") && f[*k] > 0.0)
            .collect();
        assert_eq!(
            allumees.len(),
            2,
            "partie {}, siège {joueur} : {} case(s) de paire tenue allumée(s) au moment du choix",
            700200 + g,
            allumees.len()
        );
        for c in game.players[joueur].corps_en_main.iter() {
            let nom = &b.db.corporations[*c as usize].name;
            assert_eq!(
                f[&format!("corpo_{nom}_ma_main")],
                1.0,
                "la corporation {nom} est tenue en main et la fiche ne la voit pas"
            );
        }
    }
}

// ===========================================================================
// D. 2.8 — le résumé de ma main
// ===========================================================================

/// `docs/AUDIT_ENTRAINEMENT.md`, § 2.8 : la main n'avait aucun résumé de son
/// contenu, et une carte donnée n'est en main que dans 4 % des situations.
#[test]
fn d01_au_moins_soixante_cases_de_resume_de_main() {
    let b = banc();
    let k = noms(&b)
        .iter()
        .filter(|n| n.starts_with("moi_main_") && !n.contains("payable"))
        .count();
    assert!(k >= 60, "seulement {k} cases moi_main_ hors payable");
}

/// L'interdit dur du lot : le contenu de la main d'en face n'est jamais lu
/// (`docs/regles/livret-base.md`, la main reste secrète).
#[test]
fn d02_aucune_case_de_resume_pour_la_main_adverse() {
    let b = banc();
    let k = noms(&b)
        .iter()
        .filter(|n| n.starts_with("adv_main_") && !n.contains("payable"))
        .count();
    assert_eq!(k, 0, "le contenu de la main d'en face est résumé, c'est interdit (§ 3.3)");
}

#[test]
fn d03_le_nombre_de_cartes_de_l_adversaire_reste_publie() {
    let b = banc();
    assert!(noms(&b).iter().any(|n| n.starts_with("adv_main>")));
}

#[test]
fn d04_resume_d_une_main_vide() {
    let b = banc();
    let r = resume_main(&b.db, &[]);
    assert_eq!(r.badges, [0; TAG_COUNT]);
    assert_eq!(r.couleurs, [0; 3]);
    assert_eq!(r.pv_imprimes, 0);
    assert_eq!(r.prix_total, 0);
    assert_eq!(r.prix_min, PRIX_MAIN_VIDE);
}

/// Les prix imprimés viennent de `data/cards.json`, transcrits des cartes
/// (`docs/regles/livret-base.md` et `docs/cartes/`).
#[test]
fn d05_le_prix_dune_main_vide_est_au_dessus_de_tous_les_prix() {
    let b = banc();
    for c in b.db.projects.iter() {
        assert!(
            c.price < PRIX_MAIN_VIDE,
            "la carte {} coûte {} : le prix de la main vide ment",
            c.name,
            c.price
        );
    }
}

#[test]
fn d06_resume_compte_les_badges() {
    let b = banc();
    let id = b.desc.projets[0];
    let r = resume_main(&b.db, &[id]);
    let attendu: i64 = b.db.projects[id as usize]
        .tags
        .iter()
        .filter(|t| t.index().is_some())
        .count() as i64;
    assert_eq!(r.badges.iter().sum::<i64>(), attendu);
}

#[test]
fn d07_resume_compte_les_couleurs() {
    let b = banc();
    let main: Vec<u16> = b.desc.projets[0..10].to_vec();
    let r = resume_main(&b.db, &main);
    assert_eq!(r.couleurs.iter().sum::<i64>(), main.len() as i64);
}

#[test]
fn d08_resume_additionne_les_points_imprimes() {
    let b = banc();
    let main: Vec<u16> = b.desc.projets[0..8].to_vec();
    let attendu: i64 = main.iter().map(|id| b.db.projects[*id as usize].vp).sum();
    assert_eq!(resume_main(&b.db, &main).pv_imprimes, attendu);
}

#[test]
fn d09_resume_additionne_les_prix() {
    let b = banc();
    let main: Vec<u16> = b.desc.projets[0..8].to_vec();
    let attendu: i64 = main.iter().map(|id| b.db.projects[*id as usize].price).sum();
    assert_eq!(resume_main(&b.db, &main).prix_total, attendu);
}

#[test]
fn d10_resume_retient_le_prix_le_plus_bas() {
    let b = banc();
    let main: Vec<u16> = b.desc.projets[0..8].to_vec();
    let attendu = main.iter().map(|id| b.db.projects[*id as usize].price).min().unwrap();
    assert_eq!(resume_main(&b.db, &main).prix_min, attendu);
}

#[test]
fn d11_un_badge_joker_indetermine_ne_compte_nulle_part() {
    let b = banc();
    // `Tag::Dynamic` est le rond gris « ? » de l'extension Découverte
    // (`docs/regles/livret-decouverte.md`) : tant qu'aucun jeton n'est posé, il
    // n'est aucun des dix badges. C'est déjà la règle de `PlayerState::tag_counts`.
    let joker = b
        .db
        .projects
        .iter()
        .enumerate()
        .find(|(_, c)| c.tags.iter().any(|t| *t == Tag::Dynamic));
    if let Some((i, c)) = joker {
        let r = resume_main(&b.db, &[i as u16]);
        let comptes: i64 = r.badges.iter().sum();
        assert!(comptes < c.tags.len() as i64, "le badge joker a été compté comme un badge");
    }
}

#[test]
fn d12_changer_ma_main_change_les_cases_de_resume() {
    let b = banc();
    let mut game = partie(&b, 700016);
    game.players[0].hand = b.desc.projets[0..8].to_vec();
    let a = fiche_nommee(&b, &game, 0);
    game.players[0].hand = b.desc.projets[100..108].to_vec();
    let c = fiche_nommee(&b, &game, 0);
    let bouges = a
        .keys()
        .filter(|k| k.starts_with("moi_main_") && !k.contains("payable") && a[*k] != c[*k])
        .count();
    assert!(bouges > 0, "aucune case de résumé ne bouge quand la main change : elles ne résument rien");
}

#[test]
fn d13_changer_la_main_adverse_ne_change_rien_a_ma_fiche() {
    let b = banc();
    let mut game = partie(&b, 700017);
    game.players[1].hand = b.desc.projets[0..8].to_vec();
    let a = fiche(&b, &game, 0);
    game.players[1].hand = b.desc.projets[100..108].to_vec();
    let c = fiche(&b, &game, 0);
    // Même NOMBRE de cartes, contenu différent : ma fiche doit être identique
    // au bit près. C'est l'interdit dur du § 3.3.
    assert_eq!(a, c, "ma fiche lit le contenu de la main d'en face");
}

#[test]
fn d14_changer_la_taille_de_la_main_adverse_ne_touche_que_son_compte() {
    let b = banc();
    let mut game = partie(&b, 700018);
    game.players[1].hand = b.desc.projets[0..4].to_vec();
    let a = fiche_nommee(&b, &game, 0);
    game.players[1].hand = b.desc.projets[0..12].to_vec();
    let c = fiche_nommee(&b, &game, 0);
    let bouge = a.keys().filter(|k| a[*k] != c[*k]).count();
    // Sans cette garde, une fiche qui cesserait de publier le compte de la main
    // d'en face rendrait la boucle vide — et le test vert.
    assert!(bouge > 0, "passer de 4 à 12 cartes en face n'a rien changé");
    for k in a.keys() {
        if a[k] != c[k] {
            assert!(k.starts_with("adv_main>"), "{k} a bougé : ce n'est pas le compte de la main adverse");
        }
    }
}

#[test]
fn d15_le_resume_ignore_les_cartes_posees() {
    let b = banc();
    let mut game = partie(&b, 700019);
    game.players[0].hand = b.desc.projets[0..6].to_vec();
    let a = fiche_nommee(&b, &game, 0);
    // Rangé par NOM : deux `HashMap` ne se parcourent pas dans le même ordre,
    // comparer des listes de valeurs brutes ne comparerait rien.
    let avant: std::collections::BTreeMap<&String, f64> = a
        .iter()
        .filter(|(k, _)| k.starts_with("moi_main_") && !k.contains("payable"))
        .map(|(k, v)| (k, *v))
        .collect();
    game.players[0].played = b.desc.projets[50..56].to_vec();
    let c = fiche_nommee(&b, &game, 0);
    let apres: std::collections::BTreeMap<&String, f64> = c
        .iter()
        .filter(|(k, _)| k.starts_with("moi_main_") && !k.contains("payable"))
        .map(|(k, v)| (k, *v))
        .collect();
    assert_eq!(avant.len(), apres.len());
    // Comparer les longueurs ne dit rien : elles sont égales par construction.
    // Ce qui compte, c'est que POSER six cartes ne bouge aucune case du résumé
    // de la main — et que le résumé des six cartes posées, lui, est DIFFÉRENT,
    // sans quoi la propriété serait vraie par accident.
    assert_eq!(avant, apres, "poser des cartes a changé le résumé de la main");
    let r_main = resume_main(&b.db, &b.desc.projets[0..6]);
    let r_posees = resume_main(&b.db, &b.desc.projets[50..56]);
    assert_ne!(
        (r_main.prix_total, r_main.pv_imprimes, r_main.badges),
        (r_posees.prix_total, r_posees.pv_imprimes, r_posees.badges),
        "les deux paquets se résument pareil : le test ne prouverait rien"
    );
}

#[test]
fn d16_la_fiche_publie_le_meme_resume_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700020);
    game.players[0].hand = b.desc.projets[0..9].to_vec();
    let r = resume_main(&b.db, &game.players[0].hand);
    let f = fiche_nommee(&b, &game, 0);
    for (i, tag) in JOKER_TAG_CHOICES.iter().enumerate() {
        for s in S_MAIN_BADGES[i] {
            let attendu = if r.badges[i] > *s { 1.0 } else { -1.0 };
            assert_eq!(
                f[&format!("moi_main_badge_{}>{s}", tag.as_str())],
                attendu,
                "point de calcul divergent pour le badge {}",
                tag.as_str()
            );
        }
    }
}

#[test]
fn d17_la_fiche_publie_le_meme_prix_total_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700021);
    game.players[0].hand = b.desc.projets[10..18].to_vec();
    let r = resume_main(&b.db, &game.players[0].hand);
    let f = fiche_nommee(&b, &game, 0);
    for s in S_MAIN_PRIX_TOTAL {
        assert_eq!(f[&format!("moi_main_prix_total>{s}")], if r.prix_total > *s { 1.0 } else { -1.0 });
    }
}

#[test]
fn d18_la_fiche_publie_le_meme_prix_minimum_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700022);
    game.players[0].hand = b.desc.projets[20..26].to_vec();
    let r = resume_main(&b.db, &game.players[0].hand);
    let f = fiche_nommee(&b, &game, 0);
    for s in S_MAIN_PRIX_MIN {
        assert_eq!(f[&format!("moi_main_prix_min>{s}")], if r.prix_min > *s { 1.0 } else { -1.0 });
    }
}

#[test]
fn d19_la_fiche_publie_les_memes_points_imprimes_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700023);
    game.players[0].hand = b.desc.projets[30..40].to_vec();
    let r = resume_main(&b.db, &game.players[0].hand);
    let f = fiche_nommee(&b, &game, 0);
    for s in S_MAIN_PV {
        assert_eq!(f[&format!("moi_main_pv_imprimes>{s}")], if r.pv_imprimes > *s { 1.0 } else { -1.0 });
    }
}

/// **Le nom de la case et la carte qui le porte.** `d16`–`d19` comparent la
/// fiche à `resume_main` — la fonction que la fiche appelle : une permutation
/// des compteurs à l'intérieur de `resume_main` les laisse tous verts (mesuré :
/// remplacer `r.badges[i] += 1` par `r.badges[(i + 1) % TAG_COUNT] += 1` ne fait
/// tomber aucun des 139 autres tests). Ici l'attendu est recompté depuis les
/// cartes elles-mêmes, badge par NOM (`Tag::as_str`) et couleur par NOM
/// (`Color::nom_fr`), sans jamais passer par les index qui rangent les
/// compteurs. C'est le seul test du fichier qui relie `moi_main_badge_SPACE` à
/// une carte qui porte vraiment le badge SPACE.
#[test]
fn d23_chaque_badge_et_chaque_couleur_comptent_les_cartes_qui_les_portent() {
    let b = banc();
    let mut game = partie(&b, 700025);
    let main = b.desc.projets[0..24].to_vec();
    game.players[0].hand = main.clone();
    let mut attendu: BTreeMap<String, i64> = BTreeMap::new();
    for id in main.iter() {
        let c = &b.db.projects[*id as usize];
        for t in c.tags.iter() {
            if t.is_joker() {
                continue; // un badge joker non déterminé ne compte pour aucun
            }
            *attendu.entry(format!("main_badge_{}", t.as_str())).or_insert(0) += 1;
        }
        *attendu.entry(format!("main_couleur_{}", c.color.nom_fr())).or_insert(0) += 1;
    }
    // Le test ne prouverait rien sur une main trop pauvre.
    assert!(
        attendu.values().filter(|v| **v > 0).count() >= 8,
        "main trop pauvre pour éprouver le résumé : {attendu:?}"
    );
    let f = fiche_nommee(&b, &game, 0);
    let mut compares = 0;
    for (i, tag) in JOKER_TAG_CHOICES.iter().enumerate() {
        let q = *attendu.get(&format!("main_badge_{}", tag.as_str())).unwrap_or(&0);
        for seuil in S_MAIN_BADGES[i] {
            let nom = format!("moi_main_badge_{}>{seuil}", tag.as_str());
            assert_eq!(
                f[&nom],
                if q > *seuil { 1.0 } else { -1.0 },
                "{nom} : {q} cartes de la main portent ce badge"
            );
            compares += 1;
        }
    }
    for (i, coul) in ["verte", "bleue", "rouge"].iter().enumerate() {
        let q = *attendu.get(&format!("main_couleur_{coul}")).unwrap_or(&0);
        for seuil in S_MAIN_COULEURS[i] {
            let nom = format!("moi_main_couleur_{coul}>{seuil}");
            assert_eq!(
                f[&nom],
                if q > *seuil { 1.0 } else { -1.0 },
                "{nom} : {q} cartes de la main sont de cette couleur"
            );
            compares += 1;
        }
    }
    assert_eq!(compares, 56, "le résumé de main a changé de taille");
}

#[test]
fn d20_les_trois_couleurs_sont_resumees() {
    let b = banc();
    let n = noms(&b);
    for coul in ["verte", "bleue", "rouge"] {
        assert!(n.iter().any(|x| x.starts_with(&format!("moi_main_couleur_{coul}>"))), "couleur {coul} absente");
    }
}

#[test]
fn d21_les_dix_badges_sont_resumes() {
    let b = banc();
    let n = noms(&b);
    for tag in JOKER_TAG_CHOICES.iter() {
        assert!(
            n.iter().any(|x| x.starts_with(&format!("moi_main_badge_{}>", tag.as_str()))),
            "badge {} absent du résumé de main",
            tag.as_str()
        );
    }
}

#[test]
fn d22_les_cases_de_resume_ne_sont_pas_toutes_figees() {
    let b = banc();
    let s = situations(&b, 3, 700300);
    let n = noms(&b);
    let r: Vec<usize> = n
        .iter()
        .enumerate()
        .filter(|(_, x)| x.starts_with("moi_main_") && !x.contains("payable"))
        .map(|(i, _)| i)
        .collect();
    let f = figees(&s, &r);
    assert!(f < r.len() / 2, "{f} cases de résumé figées sur {}", r.len());
}

// ===========================================================================
// E. 2.9 — les six écarts
// ===========================================================================

/// `docs/AUDIT_ENTRAINEMENT.md`, § 2.9 : aucune entrée n'exprimait la différence
/// entre les deux joueurs.
#[test]
fn e01_au_moins_trente_cases_d_ecart() {
    let b = banc();
    let k = rangs(&noms(&b), "ecart_").len();
    assert!(k >= 30, "seulement {k} cases ecart_");
}

#[test]
fn e02_les_six_grandeurs_de_l_audit_sont_couvertes() {
    let b = banc();
    let n = noms(&b);
    // `docs/AUDIT_ENTRAINEMENT.md`, § 2.9 : score acquis, niveau de
    // terraformation, cartes posées, argent, production d'argent, forêts.
    for g in ["score_acquis", "nt", "posees", "mc", "prod_mc", "forets"] {
        let k = n.iter().filter(|x| x.starts_with(&format!("ecart_{g}>"))).count();
        assert!(k >= 3, "la grandeur {g} n'a que {k} case(s) d'écart");
    }
}

#[test]
fn e03_des_seuils_negatifs_existent() {
    let b = banc();
    let k = noms(&b).iter().filter(|x| x.starts_with("ecart_") && x.contains(">-")).count();
    assert!(k >= 6, "seulement {k} seuils d'écart négatifs : un écart peut être négatif");
}

#[test]
fn e04_une_seule_serie_d_ecarts() {
    let b = banc();
    let k = noms(&b)
        .iter()
        .filter(|x| x.starts_with("moi_ecart_") || x.starts_with("adv_ecart_"))
        .count();
    assert_eq!(k, 0, "l'écart de l'adversaire est l'opposé du mien : le publier deux fois coûte des poids");
}

#[test]
fn e05_les_ecarts_sont_antisymetriques() {
    let b = banc();
    let mut game = partie(&b, 700024);
    // Sur une mise en place, quatre des six écarts valent 0 — et `0 == -0`
    // laisserait passer n'importe quoi. On écarte donc les deux joueurs d'abord.
    game.players[0].mc = 41;
    game.players[1].mc = 12;
    game.players[0].tr = 27;
    game.players[0].forests = 3;
    game.players[0].played = b.desc.projets[0..5].to_vec();
    let (parts, _, _) = score_breakdown(&game, &b.db);
    let a = ecarts(&game.players[0], &game.players[1], parts[0].acquis(), parts[1].acquis());
    let c = ecarts(&game.players[1], &game.players[0], parts[1].acquis(), parts[0].acquis());
    assert!(
        a.iter().filter(|x| **x != 0).count() >= 4,
        "trop d'écarts nuls : l'antisymétrie ne prouverait rien ({a:?})"
    );
    for i in 0..6 {
        assert_eq!(a[i], -c[i], "l'écart {} n'est pas antisymétrique", NOMS_ECARTS[i]);
    }
}

#[test]
fn e06_l_ecart_de_score_suit_le_decompte_unique() {
    let b = banc();
    let mut game = partie(&b, 700025);
    // Un écart RÉEL, sinon le test passerait aussi sur un écart toujours nul.
    game.players[0].tr = 31;
    game.players[1].tr = 12;
    let (parts, _, _) = score_breakdown(&game, &b.db);
    let e = ecarts(&game.players[0], &game.players[1], parts[0].acquis(), parts[1].acquis());
    assert_ne!(parts[0].acquis(), parts[1].acquis(), "sans écart réel, le test ne mord pas");
    assert_eq!(e[0], parts[0].acquis() - parts[1].acquis());
}

/// **Le test qui mord si les écarts cessent d'être calculés.** Les six
/// quantités sont recalculées ICI, à la main, sans passer par
/// `description::ecarts` : un test qui compare la fiche à la fonction qu'elle
/// appelle ne peut pas voir une fonction fautive
/// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.9).
#[test]
fn e18_la_fiche_publie_les_six_ecarts_recalcules_a_la_main() {
    let b = banc();
    let mut game = partie(&b, 700056);
    game.players[0].tr = 30;
    game.players[1].tr = 12;
    game.players[0].mc = 80;
    game.players[1].mc = 15;
    game.players[0].mc_prod = 9;
    game.players[1].mc_prod = 1;
    game.players[0].forests = 5;
    game.players[1].forests = 0;
    game.players[0].played = b.desc.projets[0..7].to_vec();
    game.players[1].played = b.desc.projets[20..22].to_vec();
    let (parts, _, _) = score_breakdown(&game, &b.db);
    let attendu: [i64; 6] = [
        parts[0].acquis() - parts[1].acquis(),
        30 - 12,
        7 - 2,
        80 - 15,
        9 - 1,
        5 - 0,
    ];
    let f = fiche_nommee(&b, &game, 0);
    for (i, nom) in NOMS_ECARTS.iter().enumerate() {
        assert_ne!(attendu[i], 0, "l'écart {nom} vaut zéro : le test ne mordrait pas");
        for s in S_ECARTS[i] {
            assert_eq!(
                f[&format!("ecart_{nom}>{s}")],
                if attendu[i] > *s { 1.0 } else { -1.0 },
                "la case ecart_{nom}>{s} ne dit pas l'écart réel de {}",
                attendu[i]
            );
        }
    }
}

#[test]
fn e07_l_ecart_de_niveau_de_terraformation() {
    let b = banc();
    let mut game = partie(&b, 700026);
    game.players[0].tr = 20;
    game.players[1].tr = 12;
    assert_eq!(ecarts(&game.players[0], &game.players[1], 0, 0)[1], 8);
}

#[test]
fn e08_l_ecart_de_cartes_posees() {
    let b = banc();
    let mut game = partie(&b, 700027);
    game.players[0].played = b.desc.projets[0..5].to_vec();
    game.players[1].played = b.desc.projets[10..12].to_vec();
    assert_eq!(ecarts(&game.players[0], &game.players[1], 0, 0)[2], 3);
}

#[test]
fn e09_l_ecart_d_argent() {
    let b = banc();
    let mut game = partie(&b, 700028);
    game.players[0].mc = 40;
    game.players[1].mc = 55;
    assert_eq!(ecarts(&game.players[0], &game.players[1], 0, 0)[3], -15);
}

#[test]
fn e10_l_ecart_de_production_d_argent() {
    let b = banc();
    let mut game = partie(&b, 700029);
    game.players[0].mc_prod = 7;
    game.players[1].mc_prod = 3;
    assert_eq!(ecarts(&game.players[0], &game.players[1], 0, 0)[4], 4);
}

#[test]
fn e11_l_ecart_de_forets() {
    let b = banc();
    let mut game = partie(&b, 700030);
    game.players[0].forests = 2;
    game.players[1].forests = 6;
    assert_eq!(ecarts(&game.players[0], &game.players[1], 0, 0)[5], -4);
}

#[test]
fn e12_la_fiche_publie_les_memes_ecarts_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700031);
    game.players[0].mc = 60;
    game.players[1].mc = 20;
    let (parts, _, _) = score_breakdown(&game, &b.db);
    let e = ecarts(&game.players[0], &game.players[1], parts[0].acquis(), parts[1].acquis());
    let f = fiche_nommee(&b, &game, 0);
    for (i, nom) in NOMS_ECARTS.iter().enumerate() {
        for s in S_ECARTS[i] {
            assert_eq!(f[&format!("ecart_{nom}>{s}")], if e[i] > *s { 1.0 } else { -1.0 });
        }
    }
}

#[test]
fn e13_l_ecart_change_de_signe_avec_le_siege() {
    let b = banc();
    let mut game = partie(&b, 700032);
    game.players[0].mc = 90;
    game.players[1].mc = 10;
    let a = fiche_nommee(&b, &game, 0);
    let c = fiche_nommee(&b, &game, 1);
    let differentes = S_ECARTS[3]
        .iter()
        .filter(|s| a[&format!("ecart_mc>{s}")] != c[&format!("ecart_mc>{s}")])
        .count();
    assert!(differentes > 0, "l'écart d'argent est le même vu des deux sièges");
}

#[test]
fn e14_six_series_de_seuils_d_ecart() {
    assert_eq!(S_ECARTS.len(), 6);
    assert_eq!(NOMS_ECARTS.len(), 6);
}

#[test]
fn e15_chaque_serie_d_ecart_est_croissante() {
    for (i, s) in S_ECARTS.iter().enumerate() {
        for w in s.windows(2) {
            assert!(w[0] < w[1], "les seuils de l'écart {} ne sont pas croissants", NOMS_ECARTS[i]);
        }
    }
}

#[test]
fn e16_chaque_serie_d_ecart_a_du_negatif_et_du_positif() {
    for (i, s) in S_ECARTS.iter().enumerate() {
        assert!(s.iter().any(|x| *x < 0), "l'écart {} n'a aucun seuil négatif", NOMS_ECARTS[i]);
        assert!(s.iter().any(|x| *x > 0), "l'écart {} n'a aucun seuil positif", NOMS_ECARTS[i]);
    }
}

#[test]
fn e17_les_cases_d_ecart_ne_sont_pas_toutes_figees() {
    let b = banc();
    let s = situations(&b, 3, 700400);
    let r = rangs(&noms(&b), "ecart_");
    let f = figees(&s, &r);
    assert!(f < r.len() / 2, "{f} cases d'écart figées sur {}", r.len());
}

// ===========================================================================
// F. 2.9 — l'échelle de score, désaturée
// ===========================================================================

/// `docs/AUDIT_ENTRAINEMENT.md`, § 2.9, deuxième moitié : l'échelle de score
/// saturait au-dessus de 51 points.
#[test]
fn f01_l_echelle_de_score_est_strictement_croissante() {
    for w in S_SCORE.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn f02_aucun_intervalle_de_huit_points_dans_l_echelle() {
    for w in S_SCORE.windows(2) {
        // Un palier `s` allume la case dès que la quantité DÉPASSE `s` : entre
        // `s_i` et `s_(i+1)` tiennent les valeurs `s_i+1 .. s_(i+1)`, soit un
        // écart maximal de `s_(i+1) - s_i - 1`. Un intervalle de 8 laisse donc
        // passer 7 points d'écart, pas 8 ; c'est 9 qui serait fautif.
        assert!(
            w[1] - w[0] <= 8,
            "l'intervalle {}..{} fait {} points : deux joueurs séparés de 8 y tiennent tout entiers",
            w[0],
            w[1],
            w[1] - w[0]
        );
    }
}

#[test]
fn f03_deux_scores_separes_de_huit_ne_partagent_pas_de_case() {
    let plafond = *S_SCORE.last().unwrap();
    for a in 0..=plafond {
        for d in 8..=40 {
            let c = a + d;
            if c > plafond {
                continue;
            }
            assert_ne!(cases(S_SCORE, a), cases(S_SCORE, c), "les scores {a} et {c} sont décrits pareil");
        }
    }
}

#[test]
fn f04_l_ancienne_echelle_saturait() {
    // La mesure du défaut : sur l'échelle d'avant, 40 et 50 tombent dans la même
    // case (`docs/AUDIT_ENTRAINEMENT.md`, § 2.9).
    assert_eq!(cases(&S_SCORE_AVANT, 40), cases(&S_SCORE_AVANT, 50));
    assert_ne!(cases(S_SCORE, 40), cases(S_SCORE, 50));
}

#[test]
fn f05_l_ancienne_echelle_confondait_tout_au_dessus_de_cinquante_et_un() {
    assert_eq!(cases(&S_SCORE_AVANT, 52), cases(&S_SCORE_AVANT, 140));
    assert_ne!(cases(S_SCORE, 52), cases(S_SCORE, 83));
}

#[test]
fn f06_l_echelle_a_plus_de_paliers_qu_avant() {
    assert!(S_SCORE.len() > S_SCORE_AVANT.len());
}

/// § 3.5 : un seuil n'est retenu que si la fraction des situations qui le
/// franchissent tombe entre 2 % et 98 % (`docs/AUDIT_ENTRAINEMENT.md`, § 2.7).
#[test]
fn f07_l_echelle_monte_au_dela_du_quantile_haut() {
    // Deux scores séparés de 8 points ne peuvent se confondre QUE dans la case
    // ouverte du haut, celle qui est au-dessus du dernier palier (voir `f02` et
    // `f03`). Fermer cette case oblige donc à poser des paliers dans la queue de
    // la distribution — au-dessus du quantile 98 % du § 3.5, qui vaut 76 sur
    // l'IA livrée (`mesures --parties 200 --graine-debut 200001 --poids
    // data/poids/apprenti-L3-amorce.txt --seuils 50`). Une échelle qui s'arrête
    // à la bande sature toujours ; l'arbitrage est déclaré dans `result.md`.
    assert!(*S_SCORE.last().unwrap() >= 140);
}

/// Ce qu'un palier ne peut PAS être : au-dessus du plus haut score jamais relevé.
/// Une telle case ne s'allumerait jamais, dans aucune partie.
#[test]
fn f10_aucun_palier_au_dessus_du_plus_haut_score_releve() {
    // `score_acquis min=3 max=153` sur les 200 parties de référence.
    assert!(
        *S_SCORE.last().unwrap() <= 153,
        "un palier au-dessus de 153 n'a jamais été franchi par aucune partie relevée"
    );
}

/// L'escalier du haut est régulier : au-delà du dernier palier venu des
/// quantiles (83), les paliers ajoutés montent de 8 en 8 — le pas maximal qui
/// interdit qu'un écart de 8 points passe inaperçu (voir `f02`).
#[test]
fn f11_l_escalier_du_haut_monte_d_un_pas_constant() {
    let hauts: Vec<i64> = S_SCORE.iter().copied().filter(|s| *s >= 83).collect();
    assert!(hauts.len() >= 9, "l'escalier du haut est trop court : {hauts:?}");
    for w in hauts.windows(2) {
        assert_eq!(w[1] - w[0], 8, "l'escalier du haut n'est pas régulier : {hauts:?}");
    }
}

/// La contrainte qui arrête l'escalier : un palier que les parties ne franchissent
/// jamais est une case morte, deux fois — une par joueur — et le contrôle des
/// cases figées n'en tolère que trente en tout.
#[test]
fn f12_l_echelle_ne_monte_pas_au_dela_de_ce_que_les_parties_atteignent() {
    // Relevé : `mesures --fiche --parties 60 --graine-debut 913001 --poids
    // data/poids/apprenti-L3-amorce.txt` rend 20 cases figées sur 1 630, dont
    // `moi_/adv_score_acquis>139` et `>147` — le score acquis n'y dépasse jamais
    // 139. Ajouter des paliers bien au-dessus ne décrirait plus rien et coûterait
    // deux cases mortes chacun.
    let trop_hauts = S_SCORE.iter().filter(|s| **s > 139).count();
    assert!(
        trop_hauts <= 3,
        "{trop_hauts} paliers au-dessus du plus haut score relevé : autant de cases mortes"
    );
}

#[test]
fn f08_la_fiche_porte_l_echelle_pour_les_deux_joueurs() {
    let b = banc();
    let n = noms(&b);
    for prefixe in ["moi_", "adv_"] {
        let k = n.iter().filter(|x| x.starts_with(&format!("{prefixe}score_acquis>"))).count();
        assert_eq!(k, S_SCORE.len());
    }
}

#[test]
fn f09_deux_scores_eloignes_donnent_des_cases_differentes_dans_la_fiche() {
    let b = banc();
    let mut game = partie(&b, 700033);
    game.players[0].tr = 40;
    game.players[1].tr = 20;
    let f = fiche_nommee(&b, &game, 0);
    let mienne: Vec<f64> = S_SCORE.iter().map(|s| f[&format!("moi_score_acquis>{s}")]).collect();
    let sienne: Vec<f64> = S_SCORE.iter().map(|s| f[&format!("adv_score_acquis>{s}")]).collect();
    assert_ne!(mienne, sienne, "vingt points d'écart et la fiche décrit les deux joueurs pareil");
}

// ===========================================================================
// G. 2.10 — ressources posées et classement des récompenses
// ===========================================================================

/// `docs/AUDIT_ENTRAINEMENT.md`, § 2.10 : le classement des Récompenses n'était
/// déductible que dans 72 à 74 % des cas.
#[test]
fn g01_trois_cases_de_classement_par_recompense() {
    let b = banc();
    let n = noms(&b);
    for kind in AWARD_POOL.iter() {
        let nom = format!("{kind:?}");
        for suffixe in ["je_mene", "egalite", "il_mene"] {
            assert!(
                n.contains(&format!("recompense_{nom}_classement_{suffixe}")),
                "classement absent pour {nom} ({suffixe})"
            );
        }
    }
}

#[test]
fn g02_le_nombre_de_cases_de_classement() {
    let b = banc();
    let k = noms(&b).iter().filter(|x| x.contains("_classement_")).count();
    assert_eq!(k, AWARD_POOL.len() * 3);
}

#[test]
fn g03_les_trois_cases_sont_mutuellement_exclusives() {
    let b = banc();
    let game = partie(&b, 700034);
    let f = fiche_nommee(&b, &game, 0);
    for kind in AWARD_POOL.iter() {
        let nom = format!("{kind:?}");
        let allumees = ["je_mene", "egalite", "il_mene"]
            .iter()
            .filter(|s| f[&format!("recompense_{nom}_classement_{s}")] == 1.0)
            .count();
        assert!(allumees <= 1, "{nom} : {allumees} cases de classement allumées à la fois");
    }
}

#[test]
fn g04_une_recompense_absente_donne_trois_moins_un() {
    let b = banc();
    let game = partie(&b, 700035);
    let f = fiche_nommee(&b, &game, 0);
    for kind in AWARD_POOL.iter() {
        if game.awards.contains(kind) {
            continue;
        }
        let nom = format!("{kind:?}");
        for suffixe in ["je_mene", "egalite", "il_mene"] {
            assert_eq!(
                f[&format!("recompense_{nom}_classement_{suffixe}")],
                -1.0,
                "{nom} n'est pas en jeu : il n'y a rien à mener"
            );
        }
    }
}

/// Trois tuiles Récompense sont tirées par partie
/// (`docs/regles/livret-decouverte.md`, module Objectifs et Récompenses).
#[test]
fn g05_une_recompense_en_jeu_allume_exactement_une_case() {
    let b = banc();
    let game = partie(&b, 700036);
    let f = fiche_nommee(&b, &game, 0);
    for kind in game.awards.iter() {
        let nom = format!("{kind:?}");
        let allumees = ["je_mene", "egalite", "il_mene"]
            .iter()
            .filter(|s| f[&format!("recompense_{nom}_classement_{s}")] == 1.0)
            .count();
        assert_eq!(allumees, 1, "{nom} est en jeu : le classement doit trancher");
    }
}

#[test]
fn g06_le_classement_suit_le_bareme_unique() {
    let b = banc();
    let game = partie(&b, 700037);
    let f = fiche_nommee(&b, &game, 0);
    for kind in game.awards.iter() {
        let nom = format!("{kind:?}");
        let v0 = award_value(*kind, &game.players[0]);
        let v1 = award_value(*kind, &game.players[1]);
        let attendu = if v0 > v1 {
            "je_mene"
        } else if v0 == v1 {
            "egalite"
        } else {
            "il_mene"
        };
        assert_eq!(f[&format!("recompense_{nom}_classement_{attendu}")], 1.0, "{nom} mal classé");
    }
}

#[test]
fn g07_je_mene_quand_ma_grandeur_est_plus_grande() {
    let b = banc();
    let mut game = partie(&b, 700038);
    game.awards = [
        engine::state::AwardKind::Celebrity,
        engine::state::AwardKind::Generator,
        engine::state::AwardKind::ProjectManager,
    ];
    game.players[0].mc_prod = 9;
    game.players[1].mc_prod = 2;
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f["recompense_Celebrity_classement_je_mene"], 1.0);
}

#[test]
fn g08_il_mene_quand_sa_grandeur_est_plus_grande() {
    let b = banc();
    let mut game = partie(&b, 700039);
    game.awards = [
        engine::state::AwardKind::Celebrity,
        engine::state::AwardKind::Generator,
        engine::state::AwardKind::ProjectManager,
    ];
    game.players[0].mc_prod = 1;
    game.players[1].mc_prod = 8;
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f["recompense_Celebrity_classement_il_mene"], 1.0);
}

#[test]
fn g09_egalite_quand_les_grandeurs_sont_egales() {
    let b = banc();
    let mut game = partie(&b, 700040);
    game.awards = [
        engine::state::AwardKind::Celebrity,
        engine::state::AwardKind::Generator,
        engine::state::AwardKind::ProjectManager,
    ];
    game.players[0].mc_prod = 4;
    game.players[1].mc_prod = 4;
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f["recompense_Celebrity_classement_egalite"], 1.0);
}

#[test]
fn g10_le_classement_se_renverse_avec_le_siege() {
    let b = banc();
    let mut game = partie(&b, 700041);
    game.awards = [
        engine::state::AwardKind::Celebrity,
        engine::state::AwardKind::Generator,
        engine::state::AwardKind::ProjectManager,
    ];
    game.players[0].mc_prod = 9;
    game.players[1].mc_prod = 2;
    assert_eq!(fiche_nommee(&b, &game, 0)["recompense_Celebrity_classement_je_mene"], 1.0);
    assert_eq!(fiche_nommee(&b, &game, 1)["recompense_Celebrity_classement_il_mene"], 1.0);
}

#[test]
fn g11_les_ressources_posees_sont_publiees_pour_les_deux_joueurs() {
    let b = banc();
    let n = noms(&b);
    for prefixe in ["moi_", "adv_"] {
        let k = n.iter().filter(|x| x.starts_with(&format!("{prefixe}ressources_posees_"))).count();
        assert_eq!(k, S_RESSOURCES_POSEES.len(), "les ressources posées manquent côté {prefixe}");
    }
}

#[test]
fn g12_les_ressources_posees_additionnent_toutes_les_cartes() {
    let b = banc();
    let mut game = partie(&b, 700042);
    game.players[0].card_resources.insert(b.desc.projets[0], 3);
    game.players[0].card_resources.insert(b.desc.projets[1], 4);
    assert_eq!(ressources_posees(&game.players[0]), 7);
}

#[test]
fn g13_une_partie_neuve_n_a_aucune_ressource_posee() {
    let b = banc();
    let game = partie(&b, 700043);
    for p in 0..NUM_PLAYERS {
        assert_eq!(ressources_posees(&game.players[p]), 0);
    }
}

#[test]
fn g14_la_fiche_publie_les_memes_ressources_que_la_fonction() {
    let b = banc();
    let mut game = partie(&b, 700044);
    game.players[0].card_resources.insert(b.desc.projets[2], 5);
    let q = ressources_posees(&game.players[0]);
    let f = fiche_nommee(&b, &game, 0);
    for s in S_RESSOURCES_POSEES {
        assert_eq!(f[&format!("moi_ressources_posees_total>{s}")], if q > *s { 1.0 } else { -1.0 });
    }
}

/// Tuile « le plus de ressources sur les cartes »
/// (`docs/regles/livret-decouverte.md`).
#[test]
fn g15_la_recompense_collectionneur_devient_deductible() {
    let b = banc();
    let mut game = partie(&b, 700045);
    game.awards = [
        engine::state::AwardKind::Collector,
        engine::state::AwardKind::Generator,
        engine::state::AwardKind::ProjectManager,
    ];
    game.players[0].card_resources.insert(b.desc.projets[0], 6);
    // « Le plus de ressources sur les cartes » (tuile imprimée) : la grandeur ne
    // figurait NULLE PART dans la fiche (`docs/AUDIT_ENTRAINEMENT.md`, § 2.10).
    assert_eq!(award_value(engine::state::AwardKind::Collector, &game.players[0]), 6);
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f["recompense_Collector_classement_je_mene"], 1.0);
    assert_eq!(f["moi_ressources_posees_total>3"], 1.0);
}

#[test]
fn g16_les_ressources_de_l_adversaire_sont_publiques() {
    let b = banc();
    let mut game = partie(&b, 700046);
    game.players[1].card_resources.insert(b.desc.projets[3], 9);
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f["adv_ressources_posees_total>3"], 1.0);
}

#[test]
fn g17_les_cases_de_classement_ne_sont_pas_toutes_figees() {
    let b = banc();
    let s = situations(&b, 20, 700500);
    let n = noms(&b);
    let r: Vec<usize> = n
        .iter()
        .enumerate()
        .filter(|(_, x)| x.contains("_classement_"))
        .map(|(i, _)| i)
        .collect();
    // Mesuré sur ces 20 parties : AUCUNE des 21 cases ne reste figée. La borne
    // est donc le zéro mesuré, pas un « moins que tout » qui passerait avec 20
    // cases mortes sur 21.
    let f = figees(&s, &r);
    assert_eq!(r.len(), 21, "le classement ne fait plus 21 cases");
    assert_eq!(f, 0, "{f} des 21 cases de classement ne bougent jamais");
}

#[test]
fn g18_la_presence_d_une_recompense_reste_publiee() {
    let b = banc();
    let n = noms(&b);
    for kind in AWARD_POOL.iter() {
        assert!(n.contains(&format!("recompense_{kind:?}_presente")));
    }
}

/// Le nom seul ne suffit pas : une case `_presente` qui vaudrait toujours −1
/// laisserait le réseau croire qu'aucune récompense n'est en jeu.
#[test]
fn g19_la_presence_est_allumee_pour_les_recompenses_en_jeu() {
    let b = banc();
    let game = partie(&b, 700045);
    let f = fiche_nommee(&b, &game, 0);
    for kind in AWARD_POOL.iter() {
        let attendu = if game.awards.contains(kind) { 1.0 } else { -1.0 };
        assert_eq!(
            f[&format!("recompense_{kind:?}_presente")],
            attendu,
            "la récompense {kind:?} est mal annoncée"
        );
    }
    let en_jeu = AWARD_POOL.iter().filter(|k| game.awards.contains(k)).count();
    assert_eq!(en_jeu, 3, "trois récompenses sont tirées par partie");
}

// ===========================================================================
// H. § 3.2 et § 3.3 — le point de vue et le secret
// ===========================================================================

#[test]
fn h01_les_deux_sieges_ne_lisent_pas_la_meme_fiche() {
    let b = banc();
    let mut game = partie(&b, 700047);
    game.players[0].mc = 70;
    game.players[1].mc = 12;
    assert_ne!(fiche(&b, &game, 0), fiche(&b, &game, 1));
}

/// § 3.2 : le joueur qui regarde vient toujours en premier
/// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.2).
#[test]
fn h02_ce_que_je_lis_de_moi_l_autre_le_lit_de_l_adversaire() {
    let b = banc();
    let mut game = partie(&b, 700048);
    game.players[0].mc = 44;
    let a = fiche_nommee(&b, &game, 0);
    let c = fiche_nommee(&b, &game, 1);
    for s in description::S_MC {
        assert_eq!(a[&format!("moi_mc>{s}")], c[&format!("adv_mc>{s}")], "§ 3.2 : le point de vue ne bascule pas");
    }
}

/// La défausse est face visible sur la table (`docs/regles/livret-base.md`).
#[test]
fn h03_la_defausse_reste_publique() {
    let b = banc();
    let mut game = partie(&b, 700049);
    let id = b.desc.projets[9];
    game.players[0].hand.retain(|x| *x != id);
    game.players[1].hand.retain(|x| *x != id);
    game.discard.push(id);
    let f = fiche_nommee(&b, &game, 0);
    // § 3.3bis : la défausse est publique, le comptage des cartes passées a été
    // accordé le 11-08.
    assert_eq!(f[&format!("projet{id}_defausse")], 1.0);
}

#[test]
fn h04_les_cartes_posees_de_l_adversaire_sont_publiques() {
    let b = banc();
    let mut game = partie(&b, 700050);
    let id = b.desc.projets[11];
    game.players[1].played = vec![id];
    let f = fiche_nommee(&b, &game, 0);
    assert_eq!(f[&format!("projet{id}_pose_adv")], 1.0);
}

#[test]
fn h05_le_nombre_de_cartes_de_l_adversaire_est_lu() {
    let b = banc();
    let mut game = partie(&b, 700051);
    game.players[1].hand = b.desc.projets[0..2].to_vec();
    let a = fiche(&b, &game, 0);
    game.players[1].hand = b.desc.projets[0..14].to_vec();
    let c = fiche(&b, &game, 0);
    assert_ne!(a, c, "le nombre de cartes en main d'en face doit rester visible");
}

#[test]
fn h06_aucun_nom_de_case_ne_promet_le_contenu_de_la_main_adverse() {
    let b = banc();
    for n in noms(&b) {
        let interdit = n.starts_with("adv_main_badge_")
            || n.starts_with("adv_main_couleur_")
            || n.starts_with("adv_main_pv_")
            || n.starts_with("adv_main_prix_");
        assert!(!interdit, "{n} : le contenu de la main d'en face est interdit");
    }
}

#[test]
fn h07_une_seule_fonction_accede_aux_joueurs() {
    let b = banc();
    let mut game = partie(&b, 700052);
    // Preuve par le comportement : la fiche du siège 1 est celle du siège 0 avec
    // les rôles échangés — impossible si le parcours lisait `players[0]` en dur.
    // Comparer les longueurs ne prouverait rien du tout ; il faut comparer les
    // cases deux à deux, `moi_` d'un côté contre `adv_` de l'autre.
    game.players[0].mc = 61;
    game.players[1].mc = 7;
    game.players[0].tr = 33;
    game.players[1].tr = 14;
    let a = fiche_nommee(&b, &game, 0);
    let c = fiche_nommee(&b, &game, 1);
    let mut compares = 0;
    for (nom, v) in a.iter() {
        if let Some(reste) = nom.strip_prefix("moi_") {
            // Les résumés de main (2.8) n'ont pas de jumeau `adv_`, et c'est
            // voulu : la main d'en face n'est jamais lue (§ 3.3).
            let jumeau = format!("adv_{reste}");
            if let Some(w) = c.get(&jumeau) {
                assert_eq!(*v, *w, "§ 3.2 : {nom} ne bascule pas en {jumeau}");
                compares += 1;
            }
        }
    }
    assert!(compares > 100, "seulement {compares} cases comparées");
    assert_ne!(fiche(&b, &game, 0), fiche(&b, &game, 1));
}

// ===========================================================================
// I. Les préfixes imposés par le contrat
// ===========================================================================

#[test]
fn i01_prefixe_des_resumes_de_main() {
    let b = banc();
    assert!(noms(&b).iter().any(|n| n.starts_with("moi_main_badge_")));
}

#[test]
fn i02_prefixe_des_ecarts() {
    let b = banc();
    assert!(noms(&b).iter().any(|n| n.starts_with("ecart_")));
}

#[test]
fn i03_prefixe_des_ressources_posees() {
    let b = banc();
    let n = noms(&b);
    assert!(n.iter().any(|x| x.starts_with("moi_ressources_posees_")));
    assert!(n.iter().any(|x| x.starts_with("adv_ressources_posees_")));
}

#[test]
fn i04_prefixe_du_classement_des_recompenses() {
    let b = banc();
    assert!(noms(&b)
        .iter()
        .any(|n| n.starts_with("recompense_") && n.contains("_classement_")));
}

#[test]
fn i05_suffixe_des_corporations_tenues_en_main() {
    let b = banc();
    assert!(noms(&b).iter().any(|n| n.starts_with("corpo_") && n.ends_with("_ma_main")));
}

#[test]
fn i06_les_thermometres_portent_leur_seuil_dans_leur_nom() {
    let b = banc();
    for n in noms(&b).iter().filter(|x| x.starts_with("ecart_")) {
        assert!(n.contains('>'), "{n} devrait porter son seuil");
    }
}

#[test]
fn i07_les_drapeaux_ne_portent_pas_de_seuil() {
    let b = banc();
    for n in noms(&b).iter().filter(|x| x.contains("_classement_")) {
        assert!(!n.contains('>'), "{n} est un drapeau, pas un thermomètre");
    }
}

// ===========================================================================
// J. § 3.5 — les seuils viennent d'une mesure
// ===========================================================================

#[test]
fn j01_les_seuils_de_badges_de_main_sont_croissants() {
    for s in S_MAIN_BADGES.iter() {
        for w in s.windows(2) {
            assert!(w[0] < w[1]);
        }
    }
}

/// Dix badges, le rond gris « ? » non compris
/// (`docs/regles/livret-decouverte.md`, badge joker).
#[test]
fn j02_dix_series_de_badges_de_main() {
    assert_eq!(S_MAIN_BADGES.len(), TAG_COUNT);
    assert_eq!(S_MAIN_BADGES.len(), JOKER_TAG_CHOICES.len());
}

#[test]
fn j03_trois_series_de_couleurs_de_main() {
    assert_eq!(S_MAIN_COULEURS.len(), 3);
}

#[test]
fn j04_les_seuils_de_couleurs_sont_croissants() {
    for s in S_MAIN_COULEURS.iter() {
        for w in s.windows(2) {
            assert!(w[0] < w[1]);
        }
    }
}

#[test]
fn j05_les_seuils_de_prix_total_sont_croissants() {
    for w in S_MAIN_PRIX_TOTAL.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn j06_les_seuils_de_prix_minimum_sont_croissants() {
    for w in S_MAIN_PRIX_MIN.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn j07_les_seuils_de_points_imprimes_sont_croissants() {
    for w in S_MAIN_PV.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn j08_les_seuils_de_ressources_posees_sont_croissants() {
    for w in S_RESSOURCES_POSEES.windows(2) {
        assert!(w[0] < w[1]);
    }
}

#[test]
fn j09_aucune_serie_de_seuils_n_est_vide() {
    let series: Vec<&[i64]> = vec![
        S_SCORE,
        S_MAIN_PV,
        S_MAIN_PRIX_TOTAL,
        S_MAIN_PRIX_MIN,
        S_RESSOURCES_POSEES,
    ];
    for s in series {
        assert!(!s.is_empty());
    }
    for s in S_ECARTS.iter() {
        assert!(!s.is_empty());
    }
    for s in S_MAIN_BADGES.iter() {
        assert!(!s.is_empty());
    }
    for s in S_MAIN_COULEURS.iter() {
        assert!(!s.is_empty());
    }
}

#[test]
fn j10_le_prix_du_seuil_de_main_vide_est_hors_de_l_echelle_des_prix() {
    assert!(*S_MAIN_PRIX_MIN.last().unwrap() < PRIX_MAIN_VIDE);
}

#[test]
fn j11_une_main_vide_allume_toutes_les_cases_de_prix_minimum() {
    let b = banc();
    let mut game = partie(&b, 700053);
    game.players[0].hand = Vec::new();
    let f = fiche_nommee(&b, &game, 0);
    for s in S_MAIN_PRIX_MIN {
        assert_eq!(f[&format!("moi_main_prix_min>{s}")], 1.0, "main vide : « rien de bon marché ici »");
    }
}

// ===========================================================================
// K. Les cases figées — la mesure qui protège contre les entrées mortes
// ===========================================================================

/// La mesure des cases figées est celle qui protège contre les entrées mortes
/// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12).
#[test]
fn k01_la_fiche_entiere_n_est_pas_majoritairement_figee() {
    let b = banc();
    let s = situations(&b, 4, 700600);
    let tous: Vec<usize> = (0..b.desc.taille).collect();
    let f = figees(&s, &tous);
    assert!(f < b.desc.taille / 2, "{f} cases figées sur {}", b.desc.taille);
}

#[test]
fn k02_les_cases_de_corporation_tenue_existent_et_sont_lisibles() {
    let b = banc();
    let s = situations(&b, 2, 700700);
    let n = noms(&b);
    let r: Vec<usize> = n
        .iter()
        .enumerate()
        .filter(|(_, x)| x.ends_with("_ma_main"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(r.len(), b.desc.corporations.len());
    for i in r {
        for f in s.iter() {
            assert!(f[i] == 1.0 || f[i] == -1.0);
        }
    }
}

#[test]
fn k03_deux_situations_d_une_meme_partie_ne_sont_pas_identiques() {
    let b = banc();
    let s = situations(&b, 1, 700800);
    assert!(s.len() > 2);
    assert_ne!(s[0], s[s.len() - 1]);
}

#[test]
fn k04_la_fiche_distingue_les_deux_options_d_un_echange_de_corporations() {
    let b = banc();
    let mut game = partie(&b, 700054);
    game.players[0].corporation = None;
    let mut vues = Vec::new();
    for paire in [vec![0u16, 1], vec![2, 3], vec![4, 5]] {
        game.players[0].corps_en_main = paire;
        vues.push(fiche(&b, &game, 0));
    }
    assert_ne!(vues[0], vues[1]);
    assert_ne!(vues[1], vues[2]);
    assert_ne!(vues[0], vues[2]);
}

/// Le compte des familles neuves du lot (`docs/AUDIT_ENTRAINEMENT.md`, § 2.8 à
/// 2.12, et `docs/AUDIT_MOTEUR.md`, § D3).
#[test]
fn k05_le_nombre_de_cases_neuves_est_celui_attendu() {
    let b = banc();
    let n = noms(&b);
    // Famille par famille, et au nombre exact : une borne globale « au moins
    // 150 » laisserait supprimer les seize cases de la corporation tenue (D3)
    // sans que rien ne tombe.
    let compte = |f: &dyn Fn(&String) -> bool| n.iter().filter(|x| f(x)).count();
    let ecarts = compte(&|x: &String| x.starts_with("ecart_"));
    let ma_main = compte(&|x: &String| x.ends_with("_ma_main"));
    let classement = compte(&|x: &String| x.contains("_classement_"));
    let posees = compte(&|x: &String| x.contains("ressources_posees_"));
    let resume = compte(&|x: &String| x.starts_with("moi_main_") && !x.contains("payable"));
    assert_eq!(
        (ecarts, ma_main, classement, posees, resume),
        (46, 16, 21, 6, 79),
        "une famille de cases neuves a changé de taille"
    );
    let neuves = ecarts + ma_main + classement + posees + resume;
    assert_eq!(neuves, 168, "seulement {neuves} cases neuves");
}

#[test]
fn k06_la_fiche_de_base_seule_reste_coherente() {
    let b = banc_boites("base");
    let game = partie(&b, 700055);
    let v = fiche(&b, &game, 0);
    assert_eq!(v.len(), b.desc.taille);
    for x in v {
        assert!(x == 1.0 || x == -1.0);
    }
}

#[test]
fn k07_les_deux_compositions_ont_les_memes_familles_de_cases_neuves() {
    let base = banc_boites("base");
    let tout = banc_boites("base,decouverte");
    for prefixe in ["ecart_", "moi_main_badge_", "moi_ressources_posees_"] {
        assert_eq!(
            rangs(&noms(&base), prefixe).len(),
            rangs(&noms(&tout), prefixe).len(),
            "la famille {prefixe} dépend de la composition, elle ne devrait pas"
        );
    }
}

#[test]
fn k08_seules_les_cases_de_carte_dependent_de_la_composition() {
    let base = banc_boites("base");
    let tout = banc_boites("base,decouverte");
    let d = tout.desc.taille - base.desc.taille;
    // Quatre cases par carte projet, trois par corporation (installée chez moi,
    // installée chez lui, tenue dans ma main) : ce sont les deux seules familles
    // dont le nombre suit la composition.
    let attendu = (tout.desc.projets.len() - base.desc.projets.len()) * 4
        + (tout.desc.corporations.len() - base.desc.corporations.len()) * 3;
    assert_eq!(d, attendu);
}

#[test]
fn k09_les_corporations_decrites_sont_celles_de_la_composition() {
    let base = banc_boites("base");
    let tout = banc_boites("base,decouverte");
    assert!(base.desc.corporations.len() < tout.desc.corporations.len());
    for nom in base.desc.corporations.iter() {
        assert!(tout.desc.corporations.contains(nom));
    }
}

#[test]
fn k10_trois_cases_par_corporation() {
    let b = banc();
    let n = noms(&b);
    let k = n.iter().filter(|x| x.starts_with("corpo_")).count();
    assert_eq!(k, b.desc.corporations.len() * 3);
}

/// § 3.3 : une case qui répète une autre case ne dit rien de neuf au réseau —
/// elle coûte un poids par option et n'apporte aucune information. Sur 200
/// parties, deux cases VIVANTES (celles qui changent de valeur au moins une
/// fois) ne doivent jamais porter la même colonne de valeurs, ni deux colonnes
/// exactement opposées. Les seuls couples tolérés sont déclarés dans
/// `outputs/result.md` : les cinq paliers `main_badge_EVENT` /
/// `main_couleur_rouge`, que le critère 6 du prompt impose tous les deux
/// (« au moins un palier par symbole, un palier par couleur ») et qui
/// coïncident parce que dans la composition livrée toute carte rouge porte le
/// symbole ÉVÉNEMENT et réciproquement — 0 désaccord sur les 264 cartes de
/// `base,decouverte`, 16 sur les 388 du fichier ; et les trois couples hérités
/// `global_generation>1` / `previous_phase_aucune`, qui existaient avant ce lot.
#[test]
fn k11_aucune_case_vivante_n_en_repete_une_autre() {
    let b = banc();
    let n = noms(&b);
    let s = situations(&b, 200, 940000);
    let vivantes: Vec<usize> = (0..n.len())
        .filter(|i| s.iter().any(|r| r[*i] != s[0][*i]))
        .collect();
    println!(
        "{} situations, {} cases vivantes sur {}",
        s.len(),
        vivantes.len(),
        n.len()
    );
    let colonne = |i: usize| -> Vec<f64> { s.iter().map(|r| r[i]).collect() };
    let cols: Vec<Vec<f64>> = vivantes.iter().map(|i| colonne(*i)).collect();
    let mut couples: Vec<String> = Vec::new();
    for a in 0..vivantes.len() {
        for c in (a + 1)..vivantes.len() {
            let (x, y) = (&cols[a], &cols[c]);
            let jumelles = x == y || x.iter().zip(y.iter()).all(|(u, v)| *u == -*v);
            if jumelles {
                couples.push(format!("{} / {}", n[vivantes[a]], n[vivantes[c]]));
            }
        }
    }
    couples.sort();
    let attendus = vec![
        "global_generation>1 / adv_previous_phase_aucune".to_string(),
        "global_generation>1 / moi_previous_phase_aucune".to_string(),
        "moi_main_badge_EVENT>0 / moi_main_couleur_rouge>0".to_string(),
        "moi_main_badge_EVENT>1 / moi_main_couleur_rouge>1".to_string(),
        "moi_main_badge_EVENT>2 / moi_main_couleur_rouge>2".to_string(),
        "moi_main_badge_EVENT>3 / moi_main_couleur_rouge>3".to_string(),
        "moi_main_badge_EVENT>4 / moi_main_couleur_rouge>4".to_string(),
        "moi_previous_phase_aucune / adv_previous_phase_aucune".to_string(),
    ];
    let mut attendus = attendus;
    attendus.sort();
    assert!(
        vivantes.len() > 1400,
        "trop peu de cases vivantes : {}",
        vivantes.len()
    );
    assert_eq!(couples, attendus, "couples de cases jumelles inattendus");
}

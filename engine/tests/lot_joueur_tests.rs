//! **(le-joueur-sans-voyance) LES QUATRE DÉFAUTS DE LA FAÇON DONT L'IA DÉCIDE.**
//!
//! Un correctif sans test qui échoue sur le code d'avant n'est pas un correctif :
//! c'est une affirmation.
//!
//! **Ce que « le rouge » veut dire ici, exactement.** Sur le commit d'avant, ce
//! fichier ne compile même pas : ni `graine_essais`, ni `combinaisons_completes`,
//! ni `vente` n'existent. Le rouge a donc été constaté autrement, et c'est plus
//! sévère : sur une copie du code d'AUJOURD'HUI, chaque correctif a été débranché
//! tour à tour, et l'on a relevé quels tests tombent. Le détail est dans
//! `outputs/result.md` §Verification ; le résumé :
//!
//! - V1, rebattage des trois tas retiré ............. 3 tests rouges
//! - V1, garde-fou `DejaVu` neutralisé .............. 2 tests rouges
//! - 2.11, énumération débranchée ................... 6 tests rouges
//! - 2.14, compteur commun sans décalage ............ 3 tests rouges
//! - 2.15, `vendre_librement` de nouveau vide ....... 5 tests rouges
//! - 2.15, l'entrée de vente perd son numéro ........ 1 test rouge
//!
//! Les autres tests sont des garde-fous : ils tiennent des invariants que ce lot
//! ne doit pas casser (reproductibilité, rejouabilité du journal, bornes).
//!
//! **Aucun test n'appelle une fonction interne à la place d'une partie.** Les
//! quatre défauts se mesurent sur des parties entières, jouées par le chemin
//! réel : `setup_game`, puis `play_round` manche après manche, avec le vrai
//! réseau lu sur le disque. Les trois tests qui appellent une fonction isolée
//! (`rebattre_l_avenir`, `ecarter_les_cartes_du_futur`) le font EN PLUS des tests
//! de partie, jamais à leur place.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, setup_game};
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;
use rand::rngs::StdRng;
use serde_json::Value;

use engine::{description, joueur, rejeu, reseau};

use description::Description;
use joueur::Joueur;
use reseau::{Pile, Reseau};

const CARTES: &str = "../data/cards.json";
// (L3) La fiche a changé de taille : le verrou des noms du §3.7 refuse tous les
// fichiers de poids d'avant le lot. Celui-ci est appris sur la fiche neuve
// (`entraine --parties 30000 --graine-debut 10500001`).
const POIDS: &str = "../data/poids/apprenti-L3-amorce.txt";

/// Le matériel commun : les cartes et la fiche de situation, chargées une fois
/// par test. Les boîtes sont celles des contrôles du lot.
struct Banc {
    db: CardsDb,
    desc: Description,
}

fn banc() -> Banc {
    let boites = BoiteSet::parse("base,decouverte").expect("boîtes");
    let db = CardsDb::load_boites(CARTES, boites).expect("cartes");
    let desc = Description::new(&db);
    Banc { db, desc }
}

/// Les réglages d'une partie d'essai — ce que les options de `jouer` règlent.
#[derive(Clone, Copy)]
struct Reglages {
    graine_essais: u64,
    combinaisons_completes: bool,
    vente: bool,
    apprendre: bool,
}

impl Default for Reglages {
    fn default() -> Reglages {
        Reglages {
            graine_essais: 0,
            combinaisons_completes: true,
            vente: true,
            apprendre: false,
        }
    }
}

/// Ce qu'une partie laisse derrière elle.
struct Bilan {
    decisions: Vec<Value>,
    essais: u64,
    essais_mulligan: u64,
    ventes_volontaires: u64,
    occasions_de_vente: u64,
    essais_vente: u64,
    essais_refuses: u64,
    rebattages_sautes: u64,
    corrections_mise_en_place: u64,
    corrections_par_siege: [u64; 2],
    ventes_du_moteur: u64,
    generations: u32,
    scores: [i64; 2],
    mains_finales: [Vec<u16>; 2],
    deck_final: usize,
    /// La main du siège 0 à la sortie de la mise en place, mulligan compris.
    main_apres_mulligan: Vec<u16>,
}

/// **UNE VRAIE PARTIE, PAR LE CHEMIN RÉEL.** C'est exactement ce que fait le
/// binaire `jouer` (et, à l'apprentissage près, le binaire `entraine`) : rien
/// n'est fabriqué, rien n'est court-circuité.
fn jouer_une_partie(b: &Banc, graine: u64, r: Reglages) -> Bilan {
    let noms = b.desc.noms_avec(&b.db);
    let mut reseau = Reseau::lire(POIDS, &noms).expect("poids");
    let mut pile = Pile::new(b.desc.taille);
    let mut j = Joueur::new(&b.db, &b.desc, &mut reseau, &mut pile, graine);
    j.exploration = 0.0;
    j.apprendre = r.apprendre;
    j.graine_essais = r.graine_essais;
    j.combinaisons_completes = r.combinaisons_completes;
    j.vente = r.vente;
    j.nouvelle_partie(graine);

    let mut game = setup_game(&b.db, graine, &mut j);
    let main_apres_mulligan = game.players[0].hand.clone();
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        j.debut_manche(&game);
        play_round(&mut game, &b.db, &mut j);
    }
    Bilan {
        decisions: j.journal.clone(),
        essais: j.essais,
        essais_mulligan: j.essais_mulligan,
        ventes_volontaires: j.ventes_volontaires,
        occasions_de_vente: j.occasions_de_vente,
        essais_vente: j.essais_vente,
        essais_refuses: j.essais_refuses,
        rebattages_sautes: j.rebattages_sautes,
        corrections_mise_en_place: j.corrections_mise_en_place,
        corrections_par_siege: j.corrections_par_siege,
        ventes_du_moteur: game.ventes_volontaires,
        generations: game.generation,
        scores: {
            let (sc, _, _) = engine::flow::score_parts(&game, &b.db);
            [sc[0], sc[1]]
        },
        mains_finales: [game.players[0].hand.clone(), game.players[1].hand.clone()],
        deck_final: game.deck.len(),
        main_apres_mulligan,
    }
}

/// Les cartes que le siège `p` a en main juste avant de répondre à sa `n`-ième
/// décision de mise en place — relevé par le crochet que le moteur offre à
/// toute politique (`Policy::observe`), donc au moment exact où le joueur
/// décide.
fn main_a_la_mise_en_place(b: &Banc, graine: u64, rang: usize) -> (usize, Vec<u16>) {
    struct Espion {
        rang: usize,
        vu: usize,
        pris: Option<(usize, Vec<u16>)>,
    }
    impl engine::policy::Policy for Espion {
        fn observe(&mut self, game: &GameState, player: usize) {
            if self.vu == self.rang && self.pris.is_none() {
                self.pris = Some((player, game.players[player].hand.clone()));
            }
            self.vu += 1;
        }
        fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
            false
        }
        fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
            Vec::new()
        }
        fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
            0
        }
        fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, a: &[u8]) -> u8 {
            a[0]
        }
        fn choose_build(&mut self, _r: &mut StdRng, _p: usize, a: &[usize]) -> Option<usize> {
            a.first().copied()
        }
        fn construction_bonus(
            &mut self,
            _r: &mut StdRng,
            _p: usize,
        ) -> engine::policy::ConstructionBonus {
            engine::policy::ConstructionBonus::DrawCardBefore
        }
        fn action_choice(
            &mut self,
            _r: &mut StdRng,
            _p: usize,
            o: &[engine::policy::ActionOpt],
        ) -> Option<usize> {
            if o.is_empty() {
                None
            } else {
                Some(0)
            }
        }
        fn research_keep(
            &mut self,
            _r: &mut StdRng,
            _p: usize,
            d: &[u16],
            k: usize,
        ) -> Vec<usize> {
            (0..k.min(d.len())).collect()
        }
        fn discard_down(&mut self, _r: &mut StdRng, _p: usize, h: &[u16], n: usize) -> Vec<usize> {
            (0..n.min(h.len())).collect()
        }
    }
    let mut e = Espion {
        rang,
        vu: 0,
        pris: None,
    };
    let _ = setup_game(&b.db, graine, &mut e);
    e.pris.expect("la mise en place pose au moins ce rang-là")
}

fn empreinte(decisions: &[Value]) -> String {
    serde_json::to_string(decisions).expect("journal sérialisable")
}

// ═══════════════════════════════════════════════════════════════════════════
// V1 — LE JOUEUR NE VOIT PLUS LE HASARD FUTUR
//
// Il essayait ses coups en rejouant la VRAIE partie depuis sa VRAIE graine : le
// paquet y était mélangé exactement comme dans la partie, et il lisait donc à
// l'avance les cartes qu'il allait recevoir. Démontré le 18-08 sur la graine
// 700001 : quelles que soient les cartes rendues au mulligan, les cartes reçues
// étaient toujours les mêmes.
// ═══════════════════════════════════════════════════════════════════════════

/// **Le test central de la voyance.** À graine de partie fixée, changer la
/// graine des essais change la partie jouée : c'est la preuve que les essais
/// explorent un avenir TIRÉ et que cet avenir pèse sur les décisions. Sur le
/// code d'avant, la graine des essais n'existait pas et les essais rejouaient la
/// vraie partie : les deux journaux étaient identiques.
#[test]
fn v1_deux_graines_d_essai_donnent_deux_parties_differentes() {
    let b = banc();
    let mut differentes = 0;
    for graine in [1000001u64, 1000002, 1000003] {
        let a = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 11,
                ..Default::default()
            },
        );
        let c = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 22,
                ..Default::default()
            },
        );
        if empreinte(&a.decisions) != empreinte(&c.decisions) {
            differentes += 1;
        }
    }
    assert!(
        differentes > 0,
        "changer la graine des essais ne change rien : les essais n'explorent pas un avenir tiré"
    );
}

/// **La reproductibilité est sacrée.** À graine de partie ET graine d'essais
/// fixées, la partie se rejoue identique au dernier chiffre près — c'est ce qui
/// permet de rejouer une partie enregistrée.
#[test]
fn v1_meme_graine_d_essai_rejoue_la_meme_partie_reproductibilite() {
    let b = banc();
    for graine in [1000001u64, 1000002, 1000003] {
        let a = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 11,
                ..Default::default()
            },
        );
        let c = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 11,
                ..Default::default()
            },
        );
        assert_eq!(
            empreinte(&a.decisions),
            empreinte(&c.decisions),
            "graine {graine} : deux exécutions identiques donnent deux parties différentes"
        );
        assert_eq!(a.essais, c.essais, "graine {graine} : le nombre d'essais varie");
    }
}

/// **LA DÉMONSTRATION DU 18-08, RETOURNÉE.** Ce jour-là, sur une graine donnée,
/// « quelles que soient les cartes rendues au mulligan, les cartes reçues sont
/// toujours les mêmes » : l'IA lisait le dessus de la pioche. Désormais les
/// cartes GARDÉES restent les siennes — ce sont celles qu'elle a sous les yeux —
/// et les cartes REÇUES changent avec la graine des essais, parce qu'elles sont
/// tirées d'un paquet rebattu et non lues dans l'avenir réel.
#[test]
fn v1_les_cartes_recues_au_mulligan_ne_sont_plus_celles_de_la_vraie_partie() {
    let b = banc();
    // La main que le moteur a DISTRIBUÉE au siège 0, relevée au moment où il
    // répond au mulligan des projets (les deux premières décisions de la mise en
    // place sont le mulligan des corporations, mains encore vides).
    let (j2, m2) = main_a_la_mise_en_place(&b, 1000001, 2);
    let (_j3, m3) = main_a_la_mise_en_place(&b, 1000001, 3);
    let distribuee = if j2 == 0 { m2 } else { m3 };
    assert_eq!(distribuee.len(), 8, "huit cartes projets à la mise en place");

    let a = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 11,
            ..Default::default()
        },
    );
    let c = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 22,
            ..Default::default()
        },
    );
    assert_ne!(
        a.main_apres_mulligan, c.main_apres_mulligan,
        "les cartes reçues au mulligan ne dépendent pas de l'avenir tiré : la voyance est encore là"
    );
    // Et ce qui a été GARDÉ est bien à lui : les cartes que sa réponse ne rend
    // pas sont, dans l'ordre, en tête de sa main d'après. Le joueur n'a pas changé
    // de main — il a seulement imaginé d'autres cartes de remplacement.
    let rang = if j2 == 0 { 2 } else { 3 };
    let rendus: Vec<usize> = a.decisions[rang]
        .as_array()
        .expect("le mulligan des projets est une liste d'indices")
        .iter()
        .map(|x| x.as_u64().expect("indice") as usize)
        .collect();
    let gardees: Vec<u16> = distribuee
        .iter()
        .enumerate()
        .filter(|(i, _)| !rendus.contains(i))
        .map(|(_, &carte)| carte)
        .collect();
    assert_eq!(
        a.main_apres_mulligan[..gardees.len()].to_vec(),
        gardees,
        "les cartes gardées ne sont plus celles que le joueur avait sous les yeux"
    );
    assert!(
        !rendus.is_empty(),
        "cette graine ne rend aucune carte : elle ne prouve rien sur la repioche"
    );
}

/// **Le paquet de l'essai n'est plus celui de la partie.** Le rebattage change
/// l'ordre du paquet sans en changer le contenu : l'essai explore un avenir
/// plausible, tiré des mêmes cartes.
#[test]
fn v1_l_essai_rebat_le_paquet_sans_en_changer_le_contenu() {
    let b = banc();
    let mut g = setup_game(&b.db, 1000001, &mut engine::policy::RandomPolicy);
    let avant = g.deck.clone();
    joueur::rebattre_l_avenir(&mut g, 12345);
    assert_ne!(avant, g.deck, "le paquet de l'essai n'a pas été rebattu");
    let mut a = avant.clone();
    let mut c = g.deck.clone();
    a.sort_unstable();
    c.sort_unstable();
    assert_eq!(a, c, "le rebattage a changé le CONTENU du paquet");
}

/// Deux graines d'essai rebattent différemment ; la même graine rebat pareil.
#[test]
fn v1_le_rebattage_du_paquet_est_reproductible_a_graine_d_essai_fixee() {
    let b = banc();
    let base = setup_game(&b.db, 1000001, &mut engine::policy::RandomPolicy);
    let mut x = base.clone();
    let mut y = base.clone();
    let mut z = base.clone();
    joueur::rebattre_l_avenir(&mut x, 11);
    joueur::rebattre_l_avenir(&mut y, 11);
    joueur::rebattre_l_avenir(&mut z, 22);
    assert_eq!(x.deck, y.deck, "même graine, rebattage différent");
    assert_ne!(x.deck, z.deck, "graines différentes, même rebattage");
}

/// **La voyance du mulligan, prise sur le fait.** Les cartes que l'essai fait
/// apparaître dans la main du joueur ne sont plus celles de la vraie partie :
/// celles qu'il a sous les yeux restent, les autres sont retirées et remplacées
/// par des cartes tirées du paquet rebattu.
#[test]
fn v1_les_cartes_du_futur_sont_ecartees_de_la_main_a_la_mise_en_place() {
    let b = banc();
    let mut g = setup_game(&b.db, 1000001, &mut engine::policy::RandomPolicy);
    // Ce que le joueur connaît : les cinq premières cartes de sa main.
    let connue: Vec<u16> = g.players[0].hand.iter().copied().take(5).collect();
    let avant = g.players[0].hand.clone();
    joueur::ecarter_les_cartes_du_futur(&connue, &mut g, 0, 9999);
    let apres = g.players[0].hand.clone();
    assert_eq!(avant.len(), apres.len(), "la main a changé de taille");
    assert_eq!(
        apres[..5].to_vec(),
        connue,
        "les cartes que le joueur avait sous les yeux ont bougé"
    );
    assert_ne!(
        avant, apres,
        "les cartes venues de l'avenir sont restées dans la main"
    );
}

/// **Une partie enregistrée se rejoue à l'identique.** C'est la contrainte qui
/// prime sur tout le reste : le journal des décisions, entrées de vente
/// comprises, doit se redérouler sans faute par le rejeu natif — celui-là même
/// que le pont du navigateur imite.
#[test]
fn v1_le_journal_d_une_partie_se_rejoue_sans_faute_reproductibilite() {
    let b = banc();
    for graine in [1000001u64, 1000002, 1000003, 1000004] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 5,
                ..Default::default()
            },
        );
        let (etat, attente) = rejeu::rejouer(&b.db, graine, p.decisions.clone())
            .unwrap_or_else(|e| panic!("graine {graine} : le journal ne se rejoue pas — {e}"));
        assert!(
            attente.is_none(),
            "graine {graine} : le journal rejoué s'arrête avant la fin de la partie"
        );
        assert!(
            etat.game_over,
            "graine {graine} : la partie rejouée n'est pas terminée"
        );
        assert_eq!(
            etat.ventes_volontaires, p.ventes_du_moteur,
            "graine {graine} : le rejeu ne retrouve pas les mêmes ventes"
        );
        // Et l'état final EN ENTIER, pas seulement le drapeau de fin : scores,
        // génération, mains, paquet. Comparer deux fins de partie sur « la partie
        // est finie » ne prouverait rien.
        assert_eq!(etat.generation, p.generations, "graine {graine} : générations");
        let (scores, _, _) = engine::flow::score_parts(&etat, &b.db);
        assert_eq!(
            [scores[0], scores[1]],
            p.scores,
            "graine {graine} : le rejeu ne retrouve pas les mêmes scores"
        );
        assert_eq!(
            [etat.players[0].hand.clone(), etat.players[1].hand.clone()],
            p.mains_finales,
            "graine {graine} : le rejeu ne retrouve pas les mêmes mains"
        );
        assert_eq!(etat.deck.len(), p.deck_final, "graine {graine} : paquet");
    }
}

/// **(2.15) CHAQUE ENTRÉE DE VENTE PORTE LE NUMÉRO DE SON OCCASION, ET LE REJEU
/// LE RESPECTE.**
///
/// Sans numéro, une entrée décidée à une occasion se faisait consommer à une
/// occasion antérieure : le moteur en ouvre une avant chaque point de décision et
/// pour chaque siège, y compris devant des points qu'il finit par ne pas poser.
/// La preuve : on décale le numéro d'une seule entrée, et cette vente-là ne doit
/// plus avoir lieu.
#[test]
fn la_vente_porte_le_numero_de_son_occasion_et_le_rejeu_le_respecte() {
    let b = banc();
    let p = jouer_une_partie(
        &b,
        1000023,
        Reglages {
            graine_essais: 5,
            ..Default::default()
        },
    );
    let ventes: Vec<usize> = p
        .decisions
        .iter()
        .enumerate()
        .filter(|(_, d)| d.get("vendre").is_some())
        .map(|(i, _)| i)
        .collect();
    assert!(ventes.len() > 5, "trop peu de ventes pour éprouver quoi que ce soit");
    for &i in &ventes {
        assert!(
            p.decisions[i]["vendre"].get("occasion").is_some(),
            "l'entrée de vente du rang {i} ne porte pas son numéro d'occasion"
        );
    }
    // Le témoin : le journal intact se rejoue avec toutes ses ventes.
    let (intact, _) = rejeu::rejouer(&b.db, 1000023, p.decisions.clone()).expect("rejeu intact");
    assert_eq!(intact.ventes_volontaires, p.ventes_du_moteur);
    // Et le journal saboté : une entrée dont le numéro ne correspond plus à aucune
    // occasion atteinte ne peut plus être consommée, donc la vente n'a pas lieu.
    let mut sabote = p.decisions.clone();
    let cible = ventes[ventes.len() / 2];
    sabote[cible]["vendre"]["occasion"] = serde_json::json!(u32::MAX);
    match rejeu::rejouer(&b.db, 1000023, sabote) {
        // Soit le rejeu s'arrête en faute (l'entrée bloque la décision suivante),
        // soit il va au bout avec une vente de moins : les deux disent que le
        // numéro a été pris au sérieux. Ce qu'il ne doit PAS faire, c'est vendre
        // autant qu'avant.
        Ok((etat, _)) => assert!(
            etat.ventes_volontaires < intact.ventes_volontaires,
            "numéro d'occasion falsifié et pourtant autant de ventes : le numéro n'est pas lu"
        ),
        Err(_) => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2.11 — TOUTES LES COMBINAISONS À L'ÉCHANGE DES CARTES DE DÉPART
//
// Le joueur partait de la liste vide, ajoutait la carte dont l'ajout améliore
// le plus, et s'arrêtait au premier tour où aucune addition SEULE n'améliore :
// au plus 37 des 256 sous-ensembles visités, et une solution moins bonne
// retenue 6 fois sur 11 mains réelles.
// ═══════════════════════════════════════════════════════════════════════════

/// Les 256 sous-ensembles, par siège : 512 essais au moins pour l'échange des
/// cartes de départ d'une partie à deux joueurs.
#[test]
fn mulligan_essaie_les_deux_cent_cinquante_six_combinaisons() {
    let b = banc();
    for graine in [1000001u64, 1000002] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                ..Default::default()
            },
        );
        assert!(
            p.essais_mulligan >= 512,
            "graine {graine} : {} essais au mulligan, 512 attendus (256 par siège)",
            p.essais_mulligan
        );
        assert!(
            p.essais >= p.essais_mulligan,
            "les essais du mulligan ne peuvent pas dépasser les essais de la partie"
        );
    }
}

/// La construction carte par carte, elle, n'en visite qu'une poignée : c'est le
/// défaut. Le même binaire, le même réseau, la même graine — seul le mode
/// change.
#[test]
fn mulligan_carte_par_carte_ne_visite_qu_une_fraction_des_combinaisons() {
    let b = banc();
    let complet = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 7,
            ..Default::default()
        },
    );
    let glouton = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 7,
            combinaisons_completes: false,
            ..Default::default()
        },
    );
    assert!(
        glouton.essais_mulligan < 256,
        "la construction carte par carte visite {} combinaisons : ce n'est plus elle",
        glouton.essais_mulligan
    );
    assert!(
        complet.essais_mulligan > glouton.essais_mulligan,
        "l'énumération complète ne coûte pas plus cher que la construction carte par carte"
    );
}

/// **Ce que la construction carte par carte rate.** Elle ne retire jamais une
/// carte déjà ajoutée et n'en ajoute jamais deux ensemble ; l'énumération
/// complète trouve donc, au moins parfois, un autre sous-ensemble — et elle rend
/// nettement plus de cartes. L'audit mesurait 4,45 contre 2,82, mais **avec la
/// voyance encore active** ; ce lot, V1 corrigé, mesure **4,16 contre 2,12** sur
/// 80 mains. Le témoin à règles écrites en rend 6.
#[test]
fn mulligan_l_enumeration_complete_choisit_d_autres_combinaisons() {
    let b = banc();
    let mut differents = 0;
    let mut rendues_completes = 0usize;
    let mut rendues_gloutonnes = 0usize;
    for graine in 1000001u64..1000009 {
        let complet = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                ..Default::default()
            },
        );
        let glouton = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                combinaisons_completes: false,
                ..Default::default()
            },
        );
        for rang in 2..4 {
            let a = complet.decisions[rang].as_array().expect("liste").len();
            let g = glouton.decisions[rang].as_array().expect("liste").len();
            rendues_completes += a;
            rendues_gloutonnes += g;
            if complet.decisions[rang] != glouton.decisions[rang] {
                differents += 1;
            }
        }
    }
    assert!(
        differents > 0,
        "l'énumération complète rend exactement les mêmes cartes que la construction carte par carte"
    );
    assert!(
        rendues_completes > rendues_gloutonnes,
        "l'énumération complète rend {rendues_completes} cartes contre {rendues_gloutonnes} : \
         elle ne débloque rien"
    );
}

/// **L'énumération complète s'arrête à huit cartes, et c'est mesuré, pas
/// décrété.** Le constat n° 7 de l'audit a démontré qu'elle coûte dix à seize
/// fois plus cher sur les défausses de fin de manche (jusqu'à 19 448
/// combinaisons) : elle ne vaut que pour l'échange des cartes de départ.
#[test]
fn mulligan_l_enumeration_est_bornee_a_huit_cartes() {
    assert_eq!(joueur::LARGEUR_ENUMERATION, 8);
    let b = banc();
    // Et le comportement, pas seulement la constante : une partie entière ne
    // dépense QUE 2 × 2^8 essais à l'échange des cartes de départ. Ni moins —
    // l'énumération serait incomplète — ni plus : elle déborderait sur les autres
    // décisions à nombre libre, ce que le constat n° 7 de l'audit interdit.
    for graine in [1000001u64, 1000002, 1000003] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                ..Default::default()
            },
        );
        assert_eq!(
            p.essais_mulligan, 512,
            "graine {graine} : {} essais au mulligan, 512 attendus (2 × 2^8, ni plus ni moins)",
            p.essais_mulligan
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2.15 — L'IA PEUT VENDRE UNE CARTE
//
// `vendre_librement` rendait une liste toujours vide : le seul endroit trouvé où
// une action légale était entièrement interdite à l'IA, alors que le livret
// l'autorise à tout moment — « vous pouvez défausser une carte Projet de votre
// main pour gagner 3 MC » (`docs/regles/livret-base.md:96`).
// ═══════════════════════════════════════════════════════════════════════════

/// L'IA vend, et le moteur l'enregistre : le compteur du moteur et celui du
/// joueur disent la même chose.
#[test]
fn la_vente_est_essayee_et_l_ia_vend_vraiment() {
    let b = banc();
    let mut total = 0;
    for graine in [1000001u64, 1000002, 1000003] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 3,
                ..Default::default()
            },
        );
        assert!(
            p.occasions_de_vente > 0,
            "graine {graine} : aucune occasion de vente offerte"
        );
        assert_eq!(
            p.ventes_volontaires, p.ventes_du_moteur,
            "graine {graine} : le joueur et le moteur ne comptent pas les mêmes ventes"
        );
        total += p.ventes_volontaires;
    }
    assert!(total > 0, "l'IA ne vend toujours jamais sur trois parties");
}

/// **Le drapeau coupe la vente**, comme l'audit l'exige : « prévois un drapeau
/// pour couper l'option si le coût explose ». Éteint, le joueur se comporte
/// comme avant ce lot, et ne dépense pas un essai à la vente.
#[test]
fn le_drapeau_coupe_la_vente_et_ses_essais() {
    let b = banc();
    let sans = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 3,
            vente: false,
            ..Default::default()
        },
    );
    assert_eq!(sans.ventes_volontaires, 0, "vente coupée, l'IA vend quand même");
    assert_eq!(sans.ventes_du_moteur, 0, "vente coupée, le moteur voit des ventes");
    assert_eq!(
        sans.occasions_de_vente, 0,
        "vente coupée, le joueur dépense quand même des essais à la vente"
    );
    assert_eq!(sans.essais_vente, 0, "vente coupée, des essais lui sont quand même dépensés");
    // Et l'effet, pas seulement le compteur : la même partie, vente allumée,
    // dépense des milliers d'essais de plus. C'est ce que le drapeau économise.
    let avec = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 3,
            ..Default::default()
        },
    );
    assert!(
        avec.essais_vente > 1000,
        "vente allumée, seulement {} essais lui sont consacrés : elle n'est pas vraiment essayée",
        avec.essais_vente
    );
    // (L3) Sur UNE partie, ce compte dépend de la trajectoire : allumer la vente
    // change les coups joués, donc la longueur de la partie, et il arrive qu'une
    // partie plus courte coûte moins d'essais au total malgré la vente. La
    // propriété visée — « essayer la vente coûte des essais » — est une propriété
    // de MOYENNE. On la mesure donc sur huit parties au lieu d'une : c'est un
    // contrôle plus sévère que l'ancien, pas plus doux.
    let mut total_avec = 0u64;
    let mut total_sans = 0u64;
    for g in 0..8u64 {
        total_avec += jouer_une_partie(
            &b,
            1000001 + g,
            Reglages {
                graine_essais: 3,
                ..Default::default()
            },
        )
        .essais;
        total_sans += jouer_une_partie(
            &b,
            1000001 + g,
            Reglages {
                graine_essais: 3,
                vente: false,
                ..Default::default()
            },
        )
        .essais;
    }
    assert!(
        total_avec > total_sans,
        "sur huit parties, la vente allumée ({total_avec} essais) ne coûte pas plus que coupée ({total_sans})"
    );
}

/// **Zéro ou une carte**, jamais deux : la prudence que l'audit impose pour un
/// premier pas. Chaque entrée de vente du journal porte exactement un indice.
#[test]
fn vendre_ne_porte_jamais_plus_d_une_carte_a_la_fois() {
    let b = banc();
    let mut entrees = 0;
    for graine in [1000001u64, 1000002, 1000003, 1000004] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 3,
                ..Default::default()
            },
        );
        for d in &p.decisions {
            let Some(v) = d.get("vendre") else { continue };
            entrees += 1;
            let cartes = v.get("cartes").and_then(Value::as_array).expect("cartes");
            assert_eq!(
                cartes.len(),
                1,
                "une entrée de vente porte {} cartes, une seule est permise",
                cartes.len()
            );
            assert!(
                v.get("joueur").and_then(Value::as_u64).is_some(),
                "une entrée de vente doit nommer son siège"
            );
        }
    }
    assert!(entrees > 0, "aucune entrée de vente dans les journaux");
}

/// **La vente est NOTÉE par le réseau, elle n'est pas tirée au sort.** Deux
/// preuves : elle est reproductible à graine fixée (un tirage ne le serait pas
/// sans graine), et elle suit l'avenir imaginé — changer la graine des essais
/// change ce que le joueur décide de vendre.
#[test]
fn la_vente_est_notee_par_le_reseau_et_non_tiree_au_hasard() {
    let b = banc();
    let a = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 3,
            ..Default::default()
        },
    );
    let deux = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 3,
            ..Default::default()
        },
    );
    assert_eq!(
        a.ventes_volontaires, deux.ventes_volontaires,
        "à graine fixée, le nombre de ventes doit être reproductible"
    );
    assert_eq!(empreinte(&a.decisions), empreinte(&deux.decisions));
    let autre = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 44,
            ..Default::default()
        },
    );
    assert_ne!(
        empreinte(&a.decisions),
        empreinte(&autre.decisions),
        "la vente ne dépend pas de l'avenir que le joueur imagine"
    );
}

/// La vente n'a lieu que dans les phases où l'on peut dépenser : le moteur
/// n'offre pas d'occasion pendant la mise en place, et le joueur n'en fabrique
/// pas.
#[test]
fn aucune_vente_pendant_la_mise_en_place() {
    let b = banc();
    let p = jouer_une_partie(
        &b,
        1000001,
        Reglages {
            graine_essais: 3,
            ..Default::default()
        },
    );
    for (rang, d) in p.decisions.iter().enumerate().take(6) {
        assert!(
            d.get("vendre").is_none(),
            "une vente a été enregistrée au rang {rang}, en pleine mise en place"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2.14 — LA MISE EN PLACE EST APPRISE
//
// Le compteur qui décide quand corriger le réseau était remis à zéro à chaque
// partie et partagé par les DEUX sièges : la première correction tombait donc
// toujours à la huitième décision. Or la mise en place n'en pose que 6, 7 ou 8
// (mesuré sur 60 graines : 26 parties à 6, 26 à 7, 8 à 8) — seules ces
// dernières, 13 %, voyaient une décision de mise en place corrigée. Second
// effet : les deux sièges ne recevaient pas le même nombre de corrections.
// ═══════════════════════════════════════════════════════════════════════════

/// Ce qu'une partie d'entraînement laisse comme trace d'apprentissage.
struct Trace {
    mise_en_place: u64,
    par_siege: [u64; 2],
}

/// **UNE CAMPAGNE D'AUTO-JEU, PAR LE CHEMIN DE L'ENTRAÎNEMENT.** Un seul réseau,
/// une partie après l'autre, `nouvelle_partie` entre chacune : c'est très
/// exactement la boucle du binaire `entraine`. Rien n'est simulé.
fn campagne(b: &Banc, graines: std::ops::Range<u64>, rythme: u64) -> Vec<Trace> {
    let noms = b.desc.noms_avec(&b.db);
    let mut reseau = Reseau::lire(POIDS, &noms).expect("poids");
    let mut pile = Pile::new(b.desc.taille);
    let mut j = Joueur::new(&b.db, &b.desc, &mut reseau, &mut pile, graines.start);
    j.exploration = 0.0;
    j.apprendre = true;
    j.rythme = rythme;
    j.graine_essais = 3;
    let mut traces = Vec::new();
    let (mut vues, mut par_siege) = (0u64, [0u64; 2]);
    for graine in graines {
        j.nouvelle_partie(graine);
        let mut game = setup_game(&b.db, graine, &mut j);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            j.debut_manche(&game);
            play_round(&mut game, &b.db, &mut j);
        }
        traces.push(Trace {
            mise_en_place: j.corrections_mise_en_place - vues,
            par_siege: [
                j.corrections_par_siege[0] - par_siege[0],
                j.corrections_par_siege[1] - par_siege[1],
            ],
        });
        vues = j.corrections_mise_en_place;
        par_siege = j.corrections_par_siege;
    }
    traces
}

/// **LE SEUIL DU LOT : au moins 60 % des parties voient une décision de mise en
/// place recevoir une correction**, contre 13 % avant ce lot. Mesuré sur une
/// campagne d'auto-jeu réelle, pas sur un modèle.
#[test]
fn mise_en_place_corrigee_dans_au_moins_60_pourcent_des_parties() {
    let b = banc();
    let traces = campagne(&b, 1000001..1000121, reseau::RYTHME);
    let n = traces.len();
    let apprises = traces.iter().filter(|t| t.mise_en_place > 0).count();
    let part = apprises as f64 / n as f64;
    println!("mise en place apprise : {apprises}/{n} = {:.1} %", part * 100.0);
    assert!(
        part >= 0.60,
        "seulement {:.1} % des parties voient la mise en place corrigée, 60 % attendus",
        part * 100.0
    );
}

/// **Les deux sièges reçoivent un nombre comparable de corrections.** Le
/// compteur commun donnait l'avantage à l'un des deux ; il y en a maintenant un
/// par siège.
#[test]
fn les_deux_sieges_recoivent_des_corrections_comparables() {
    let b = banc();
    let traces = campagne(&b, 1000001..1000061, reseau::RYTHME);
    let total: [u64; 2] = traces.iter().fold([0, 0], |mut a, t| {
        a[0] += t.par_siege[0];
        a[1] += t.par_siege[1];
        a
    });
    println!("corrections par siège : {} / {}", total[0], total[1]);
    assert!(
        total[0] > 0 && total[1] > 0,
        "un siège ne reçoit aucune correction"
    );
    let ecart = (total[0] as f64 - total[1] as f64).abs() / (total[0] + total[1]) as f64;
    assert!(
        ecart <= 0.10,
        "les deux sièges reçoivent {} et {} corrections : écart de {:.1} %, plus de 10 %",
        total[0],
        total[1],
        ecart * 100.0
    );
}

/// **La première correction ne tombe plus toujours à la huitième décision.** Le
/// décalage est tiré de la graine de la partie : d'une partie à l'autre, le
/// nombre de corrections de mise en place varie.
#[test]
fn le_compteur_d_apprentissage_ne_tombe_plus_toujours_au_meme_rang() {
    let b = banc();
    let traces = campagne(&b, 1000001..1000041, reseau::RYTHME);
    let mut vues: Vec<u64> = traces.iter().map(|t| t.mise_en_place).collect();
    vues.sort_unstable();
    vues.dedup();
    assert!(
        vues.len() > 1,
        "toutes les parties voient exactement le même nombre de corrections de mise en place : \
         le compteur retombe toujours au même rang"
    );
}

/// **Le décalage est tiré de la GRAINE DE LA PARTIE, jamais de l'horloge** :
/// deux campagnes identiques rendent les mêmes corrections, partie pour partie.
/// C'est la contrainte du déterminisme au dernier chiffre.
#[test]
fn le_compteur_decale_reste_reproductible_a_graine_fixee() {
    let b = banc();
    let a = campagne(&b, 1000001..1000011, reseau::RYTHME);
    let c = campagne(&b, 1000001..1000011, reseau::RYTHME);
    for (i, (x, y)) in a.iter().zip(c.iter()).enumerate() {
        assert_eq!(
            x.mise_en_place, y.mise_en_place,
            "partie {i} : deux campagnes identiques donnent des corrections différentes"
        );
        assert_eq!(x.par_siege, y.par_siege, "partie {i} : corrections par siège instables");
    }
}

/// Le rythme reste celui du §2.2 : une prise sur K décisions, et non une
/// correction forcée sur chaque décision de mise en place — ce que l'audit
/// interdit explicitement (« 12 à 15 % de l'apprentissage pour 1,7 % des
/// décisions »).
#[test]
fn la_mise_en_place_n_est_pas_corrigee_de_force_a_chaque_decision() {
    let b = banc();
    let traces = campagne(&b, 1000001..1000031, reseau::RYTHME);
    let total: u64 = traces.iter().map(|t| t.mise_en_place).sum();
    let moyenne = total as f64 / traces.len() as f64;
    println!("corrections de mise en place par partie : {moyenne:.2}");
    assert!(
        moyenne < 3.0,
        "{moyenne:.2} corrections de mise en place par partie : c'est un forçage, pas un rythme"
    );
    assert!(
        moyenne > 0.3,
        "{moyenne:.2} correction par partie : la mise en place reste sous-apprise"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CE QUE LE JOUEUR FAIT QUAND ON NE LUI RÈGLE RIEN
//
// Les tests ci-dessus passent par des réglages explicites, pour comparer deux
// modes. Ceux-ci prennent le joueur tel que le programme le construit, sans
// toucher à un seul drapeau : c'est le chemin que l'entraînement empruntera.
// ═══════════════════════════════════════════════════════════════════════════

/// Une partie jouée par un joueur qu'on n'a pas réglé : rien n'est touché que la
/// graine des essais, qui est une option de ligne de commande.
fn jouer_sans_rien_regler(b: &Banc, graine: u64, graine_essais: u64) -> Bilan {
    let noms = b.desc.noms_avec(&b.db);
    let mut reseau = Reseau::lire(POIDS, &noms).expect("poids");
    let mut pile = Pile::new(b.desc.taille);
    let mut j = Joueur::new(&b.db, &b.desc, &mut reseau, &mut pile, graine);
    j.graine_essais = graine_essais;
    j.nouvelle_partie(graine);
    let mut game = setup_game(&b.db, graine, &mut j);
    let main_apres_mulligan = game.players[0].hand.clone();
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        j.debut_manche(&game);
        play_round(&mut game, &b.db, &mut j);
    }
    Bilan {
        decisions: j.journal.clone(),
        essais: j.essais,
        essais_mulligan: j.essais_mulligan,
        ventes_volontaires: j.ventes_volontaires,
        occasions_de_vente: j.occasions_de_vente,
        essais_vente: j.essais_vente,
        essais_refuses: j.essais_refuses,
        rebattages_sautes: j.rebattages_sautes,
        corrections_mise_en_place: j.corrections_mise_en_place,
        corrections_par_siege: j.corrections_par_siege,
        ventes_du_moteur: game.ventes_volontaires,
        generations: game.generation,
        scores: {
            let (sc, _, _) = engine::flow::score_parts(&game, &b.db);
            [sc[0], sc[1]]
        },
        mains_finales: [game.players[0].hand.clone(), game.players[1].hand.clone()],
        deck_final: game.deck.len(),
        main_apres_mulligan,
    }
}

/// **Sans qu'on lui règle quoi que ce soit, le joueur essaie les 256
/// combinaisons et essaie de vendre.** C'est le chemin par défaut, celui que
/// l'entraînement prendra : un correctif qui ne s'allume qu'à la demande n'est
/// pas un correctif.
#[test]
fn par_defaut_le_joueur_enumere_les_combinaisons_et_essaie_la_vente() {
    let b = banc();
    let mut ventes = 0;
    for graine in [1000001u64, 1000002, 1000003] {
        let p = jouer_sans_rien_regler(&b, graine, 3);
        assert!(
            p.essais_mulligan >= 512,
            "graine {graine} : {} essais au mulligan sans réglage, 512 attendus",
            p.essais_mulligan
        );
        assert!(
            p.occasions_de_vente > 0,
            "graine {graine} : la vente n'est pas essayée sans réglage"
        );
        ventes += p.ventes_volontaires;
    }
    assert!(ventes > 0, "sans réglage, l'IA ne vend jamais");
}

/// Et la partie reste une partie : elle se termine, et son score se calcule.
#[test]
fn par_defaut_la_partie_va_jusqu_a_son_terme() {
    let b = banc();
    let p = jouer_sans_rien_regler(&b, 1000001, 3);
    assert!(p.generations > 1, "la partie n'a pas dépassé la première manche");
    assert!(
        p.decisions.len() > 100,
        "seulement {} décisions : la partie n'a pas été jouée",
        p.decisions.len()
    );
}

/// **LE GARDE-FOU DU REBATTAGE.** Un rejeu d'essai repart du début de la manche
/// et rejoue les réponses déjà données. Si le rebattage touchait les cartes que
/// ces réponses-là ont fait piocher, le moteur ne pourrait plus les honorer :
/// l'option serait « injouable » au lieu d'être jugée. Mesuré avant le garde-fou
/// (`rebattre_le_reste` et son `garder`) : 3 à 20 % des essais refusés. Sur le
/// code d'avant ce lot, qui ne rebattait rien : 0 %.
#[test]
fn le_rebattage_ne_rend_pas_injouables_les_reponses_deja_donnees() {
    let b = banc();
    let (mut essais, mut refuses) = (0u64, 0u64);
    for graine in [1000001u64, 1000002, 1000003, 1000004, 1000005] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                ..Default::default()
            },
        );
        essais += p.essais;
        refuses += p.essais_refuses;
    }
    let part = refuses as f64 / essais as f64;
    println!("essais refusés : {refuses}/{essais} = {:.2} %", part * 100.0);
    assert!(
        part < 0.03,
        "{:.1} % des essais sont refusés : le rebattage fait diverger le rejeu AVANT la décision essayée",
        part * 100.0
    );
}

/// **(V1) LE REBATTAGE EN COURS DE MANCHE, ÉPROUVÉ SUR SES TROIS TAS.**
///
/// Le paquet de projets n'est pas seul à cacher l'avenir : les tuiles Océan
/// encore face cachée portent un bonus tiré avec la graine de la partie, et le
/// paquet des corporations aussi. Un essai qui posait un océan encaissait donc le
/// VRAI bonus. Ce test éprouve les deux garde-fous à la fois : ce qui est déjà
/// sorti ne bouge pas, ce qui est caché est rebattu, et rien n'est perdu.
#[test]
fn v1_le_rebattage_couvre_les_oceans_et_les_corporations_sans_toucher_au_visible() {
    let b = banc();
    let base = setup_game(&b.db, 1000001, &mut engine::policy::RandomPolicy);
    let mut g = base.clone();
    let vu = joueur::DejaVu {
        cartes: 5,
        oceans: 3,
        corpos: 2,
    };
    joueur::rebattre_le_reste(&mut g, 4242, vu);

    // Le haut du paquet — les cinq prochaines cartes — n'a pas bougé.
    let n = base.deck.len();
    assert_eq!(base.deck[n - 5..], g.deck[n - 5..], "le haut du paquet a été rebattu");
    assert_ne!(base.deck[..n - 5], g.deck[..n - 5], "le fond du paquet n'a pas été rebattu");
    let (mut a, mut c) = (base.deck.clone(), g.deck.clone());
    a.sort_unstable();
    c.sort_unstable();
    assert_eq!(a, c, "le rebattage a changé le contenu du paquet");

    // Les trois premières tuiles Océan sont déjà connues : elles ne bougent pas.
    // (`OceanTile` ne se compare pas directement : on compare les identités, qui
    // sont précisément ce que le mélange réordonne.)
    let ids = |t: &[engine::state::OceanTile]| t.iter().map(|o| o.id).collect::<Vec<u8>>();
    assert_eq!(
        ids(&base.oceans[..3]),
        ids(&g.oceans[..3]),
        "des tuiles Océan déjà retournées ont bougé"
    );
    assert_ne!(
        ids(&base.oceans[3..]),
        ids(&g.oceans[3..]),
        "les tuiles Océan cachées n'ont pas été rebattues : l'essai encaisse le vrai bonus"
    );
    let (mut ia, mut ic) = (ids(&base.oceans), ids(&g.oceans));
    ia.sort_unstable();
    ic.sort_unstable();
    assert_eq!(ia, ic, "le rebattage a changé le contenu des tuiles Océan");

    // Et les deux prochaines corporations restent en place.
    let m = base.corp_deck.len();
    assert_eq!(base.corp_deck[m - 2..], g.corp_deck[m - 2..], "corporations connues");
}

/// **La voyance résiduelle est comptée, pas cachée.** Quand la pioche a été
/// rechargée depuis le début de la manche, le rebattage est sauté et cet essai-là
/// revoit l'avenir réel. Le compteur existe pour que la part reste sous les yeux.
#[test]
fn v1_la_voyance_residuelle_reste_marginale_et_comptee() {
    let b = banc();
    let (mut essais, mut sautes) = (0u64, 0u64);
    for graine in [1000001u64, 1000002, 1000003, 1000004, 1000005] {
        let p = jouer_une_partie(
            &b,
            graine,
            Reglages {
                graine_essais: 7,
                ..Default::default()
            },
        );
        essais += p.essais;
        sautes += p.rebattages_sautes;
    }
    let part = sautes as f64 / essais as f64;
    println!("rebattages sautés : {sautes}/{essais} = {:.2} %", part * 100.0);
    assert!(
        part < 0.10,
        "{:.1} % des essais revoient l'avenir réel : la voyance n'est plus marginale",
        part * 100.0
    );
}

/// **Les deux côtés doivent bouger ensemble.** La borne de l'énumération est
/// recopiée dans le miroir JavaScript ; si l'un des deux change seul, les deux
/// joueurs ne rendent plus les mêmes cartes au mulligan et rien ne le signale.
#[test]
fn la_borne_d_enumeration_est_la_meme_des_deux_cotes() {
    let js = std::fs::read_to_string("../web/webapp/joueurs/apprenti.js")
        .expect("le miroir JavaScript doit être lisible");
    let ligne = js
        .lines()
        .find(|l| l.contains("LARGEUR_ENUMERATION ="))
        .expect("apprenti.js ne déclare plus LARGEUR_ENUMERATION");
    let valeur: usize = ligne
        .split('=')
        .nth(1)
        .and_then(|x| x.trim().trim_end_matches(';').parse().ok())
        .expect("valeur illisible");
    assert_eq!(
        valeur,
        joueur::LARGEUR_ENUMERATION,
        "la borne vaut {valeur} en JavaScript et {} en Rust",
        joueur::LARGEUR_ENUMERATION
    );
}

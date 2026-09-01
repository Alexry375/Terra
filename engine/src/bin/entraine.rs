//! **(le-juge-apprend) Point d'accroche n°3 — l'entraînement.**
//!
//!     entraine --parties N --graine-debut G --sortie chemin
//!              [--exploration x] [--boites …] [--instantanes "10000,50000"]
//!              [--lambda 0.9] [--rythme 8] [--ouvriers 4] [--sans-optimisation]
//!              [--largeur 50]
//!
//! `--largeur N` est le nombre de neurones de la couche cachée (50 par défaut,
//! la valeur de tout ce que le dépôt a produit jusqu'ici). Elle est publiée dans
//! la première ligne du journal et écrite en tête du fichier de poids : un
//! réglage qu'on ne peut pas relire dans le journal d'un entraînement passé est
//! un réglage qu'on ne peut pas reproduire. À 50, **le fichier produit est le
//! même, octet pour octet, qu'avec l'option absente**.
//!
//! `--rythme K` est le rythme des corrections du §2.2 (une situation sur K,
//! K = 8 livré) et `--lambda` le facteur d'influence par pas en arrière (0,9
//! livré) : les deux balayages que le §2.2 demande de mesurer et de croiser.
//!
//! Il joue N parties du réseau **contre lui-même** (les deux sièges partagent les
//! mêmes poids et apprennent de la même partie), écrit le fichier de poids du §7,
//! et imprime une ligne JSON par tranche :
//!
//!     {"parties": n, "erreur": x, "justes": y}
//!
//! `erreur` est l'écart moyen de prédiction (racine de la moyenne des carrés des
//! erreurs accumulées sur la tranche) ; `justes` la proportion de parties dont le
//! vainqueur était bien celui que le réseau donnait gagnant **à mi-partie** —
//! exactement le protocole du §3.0, où trois champs bien choisis désignent le
//! vainqueur 82,5 fois sur 100.
//!
//! **Tout est semé.** Deux entraînements lancés avec les mêmes arguments
//! produisent le même fichier, octet pour octet : les poids de départ viennent
//! d'un générateur de graine fixe, l'amorçage aussi, et chaque partie tire sa
//! graine de `--graine-debut + rang`. Aucune horloge n'entre dans le calcul —
//! **y compris à quatre ouvriers**, où les différences de poids sont
//! additionnées dans l'ordre des graines et jamais dans l'ordre d'arrivée.
//!
//! **Les graines restent au-dessus de 100000** : le binaire refuse de descendre
//! plus bas, parce que la balance du dépôt joue les graines 1 à N et qu'un
//! apprentissage qui les aurait vues serait une récitation.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **(L5) CE QUE LE JOURNAL PUBLIE, ET POURQUOI.** « Une correction qu'on ne peut
//! pas voir de l'extérieur ne se contrôle pas. » Trois chiffres sortent donc du
//! programme et non d'un raisonnement :
//!
//! - la **première** ligne donne la ligne de commande et la plage de graines
//!   consommée — un journal qu'on ne peut pas rejouer ne prouve rien ;
//! - `egalites_departagees` dit combien de parties à égalité de points de
//!   victoire ont reçu un vainqueur au lieu d'apprendre « match nul » ;
//! - la **pente moyenne** de la couche cachée et la **part de neurones saturés**
//!   sur la dernière tranche disent si le réglage de l'amplitude de départ a
//!   servi.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score_parts, setup_game, winner};
use engine::sim::MAX_GENERATIONS;
use std::time::Instant;

use engine::{description, joueur, rejeu, reseau};

use description::Description;
use joueur::Joueur;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use reseau::{
    Pile, Reseau, ReseauPhases, AMORCAGE_FACTEUR, AMORCAGE_PARTIES, AMORCAGE_SCORE_MAX, TAUX,
};

fn mourir(msg: &str) -> ! {
    eprintln!("entraine: {msg}");
    std::process::exit(2);
}

/// **(L5, §2.6) LE DÉLAI MAXIMAL ENTRE DEUX ÉCRITURES DE POIDS : trente secondes.**
///
/// L'enregistrement était **après** la boucle des parties : une coupure perdait
/// tout le travail depuis le dernier instantané. C'est déjà arrivé —
/// `data/mesures/entrainement-A-2M.log` s'arrête net à 200 000 parties alors
/// qu'un million avait été demandé, 800 000 parties perdues. Les poids sont
/// désormais écrits à chaque tranche de journal, **et** au plus trente secondes
/// après la précédente écriture : sur un entraînement d'une nuit, une tranche
/// vaut des heures, et « ne perdre que la dernière tranche » ne voudrait pas dire
/// grand-chose sans ce second garde-fou.
///
/// Cela ne touche pas au déterminisme : ces écritures ne changent aucun poids,
/// elles les recopient. Le fichier final est le même, octet pour octet.
const SAUVEGARDE_SECONDES: f64 = 30.0;

/// **L'amorçage du §2.7** : cinq mille fins de partie FABRIQUÉES — on ne joue
/// pas, on part d'un état vide marqué « partie finie » et on donne à chaque
/// joueur un score tiré au hasard entre 0 et [`AMORCAGE_SCORE_MAX`] — entraînées
/// vers la cible du §2.3, taux multiplié par dix.
///
/// Un réseau tiré au hasard ne sait même pas que « plus de points, c'est
/// mieux » ; sans cet amorçage il met très longtemps à le découvrir seul.
fn amorcer(reseau: &mut Reseau, noms: &[String], graine: u64) {
    let n = noms.len();
    let rang_fin = noms
        .iter()
        .position(|x| x == "global_fin_de_partie")
        .unwrap_or_else(|| mourir("entrée « global_fin_de_partie » introuvable"));
    let debut_moi = noms
        .iter()
        .position(|x| x.starts_with("moi_score_acquis>"))
        .unwrap_or_else(|| mourir("entrées « moi_score_acquis> » introuvables"));
    let debut_adv = noms
        .iter()
        .position(|x| x.starts_with("adv_score_acquis>"))
        .unwrap_or_else(|| mourir("entrées « adv_score_acquis> » introuvables"));
    let seuils: Vec<i64> = noms
        .iter()
        .filter(|x| x.starts_with("moi_score_acquis>"))
        .map(|x| x["moi_score_acquis>".len()..].parse::<i64>().unwrap())
        .collect();

    let mut rng = StdRng::seed_from_u64(graine);
    let mut x = vec![-1.0f64; n];
    for _ in 0..AMORCAGE_PARTIES {
        let s_moi = rng.gen_range(0..=AMORCAGE_SCORE_MAX);
        let s_adv = rng.gen_range(0..=AMORCAGE_SCORE_MAX);
        x.fill(-1.0);
        x[rang_fin] = 1.0;
        for (k, s) in seuils.iter().enumerate() {
            x[debut_moi + k] = if s_moi > *s { 1.0 } else { -1.0 };
            x[debut_adv + k] = if s_adv > *s { 1.0 } else { -1.0 };
        }
        let cible = Reseau::cible_finale(s_moi, s_adv);
        reseau.entrainer_une(&x, cible, TAUX * AMORCAGE_FACTEUR);
    }
    reseau.raz_stats();
}

/// Ce qu'une partie d'entraînement rapporte à celui qui tient les compteurs.
#[derive(Default, Clone, Copy)]
struct Bilan {
    /// La partie était-elle décisive au sens du §3.0 (scores différents et au
    /// moins une prédiction relevée) ?
    decisive: bool,
    /// Le vainqueur était-il celui que le réseau donnait gagnant à mi-partie ?
    juste: bool,
    /// Les deux joueurs ont fini à égalité de points de victoire, et le
    /// départage du livret a désigné un vainqueur.
    egalite_departagee: bool,
    /// Égalité de points de victoire **et** égalité sur le total de départage :
    /// le seul vrai match nul du livret.
    nulle_jusqu_au_bout: bool,
    /// **Le nombre de cibles de fin de partie versées À CONTRESENS** dans cette
    /// partie — c'est-à-dire désignant comme gagnant celui que la partie a fait
    /// perdre. Il doit valoir zéro, toujours. Voir le garde-fou dans
    /// `une_partie`.
    contresens: usize,
}

/// **UNE PARTIE D'ENTRAÎNEMENT, LE SEUL EXEMPLAIRE.** Le déroulement à un ouvrier
/// et celui d'un ouvrier parmi quatre passent tous deux par ici : il n'y a pas
/// deux façons d'apprendre une partie dans ce programme.
fn une_partie(j: &mut Joueur, desc: &Description, seed: u64) -> Bilan {
    let db = j.db;
    j.nouvelle_partie(seed);
    let mut game = setup_game(db, seed, j);
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        j.debut_manche(&game);
        play_round(&mut game, db, j);
    }
    let (scores, _, _) = score_parts(&game, db);

    // ---- (L5, famille B) LE VAINQUEUR AU SENS DU LIVRET, ET LUI SEUL.
    //
    // `flow::winner` est le point de calcul unique du départage (lot L1). On ne
    // recompare pas les scores ici : le livret regarde aussi la chaleur,
    // l'argent, les plantes et les cartes en main converties
    // (`docs/regles/livret-base.md:461`), et un second point de vérité
    // divergerait. `None` — l'égalité parfaite jusque sur ce total — est le seul
    // vrai match nul, et c'est le seul cas où la cible reste [0,5 ; 0,5].
    let vainqueur = winner(&game, db);
    let egalite_de_points = scores[0] == scores[1];

    // ---- la correction de fin de partie (§2.3), pour les deux joueurs
    let mut fin = Vec::new();
    let mut tampons = description::Tampons::new(desc);
    for p in 0..2 {
        desc.decrire(&game, db, p, &mut fin, &mut tampons);
        j.pile.empiler(&fin, p);
    }
    // **(L5, famille B) LE COMPTEUR SE DÉDUIT DE LA CIBLE, PAS DU VAINQUEUR.**
    // Compter `egalite_de_points && vainqueur.is_some()` compterait les
    // OCCASIONS de départager, et non les corrections faites : on pourrait
    // revenir à `cible_finale` — c'est-à-dire annuler tout le travail de la
    // famille B — sans que le chiffre publié bouge d'une unité, ni qu'aucun
    // contrôle rougisse. On regarde donc la cible réellement versée au réseau :
    // hors départage elle vaut `[0,5 ; 0,5]` à points égaux, et le compteur
    // retombe à zéro dès que la correction disparaît.
    let mut departages = 0usize;
    let mut contresens = 0usize;
    for p in 0..2 {
        let cible = Reseau::cible_finale_departagee(
            scores[p],
            scores[1 - p],
            vainqueur.map(|gagnant| gagnant == p),
        );
        // `vainqueur.is_some()` exclut la vraie nulle : elle garde la cible
        // `[0,5 ; 0,5]`, elle n'est pas un départage, et elle a son propre
        // compteur (`nulle_jusqu_au_bout`).
        // **LE SENS SE VÉRIFIE SUR TOUTES LES PARTIES, PAS SEULEMENT LES
        // ÉGALITÉS.** `cible_finale` rend `cible[0] > cible[1]` exactement quand
        // `score_moi > score_autre` (engine/src/reseau.rs:947) : la cible versée
        // doit donc désigner comme gagnant celui que la partie a fait gagner. Un
        // sabotage de la sixième passe l'a montré — permuter les deux scores
        // passés à la cible inverse le sens de l'apprentissage sur CHAQUE partie,
        // en gardant les deux cibles parfaitement complémentaires, et aucun test
        // ne le voyait. Le compteur ci-dessous doit rester à zéro, et il est
        // publié : un entraînement de plusieurs heures qui apprendrait à
        // l'envers le dirait dès sa première tranche.
        let la_cible_me_donne_gagnant = cible[0] > cible[1];
        let je_devrais_gagner = if egalite_de_points {
            vainqueur == Some(p)
        } else {
            scores[p] > scores[1 - p]
        };
        if la_cible_me_donne_gagnant != je_devrais_gagner {
            contresens += 1;
        }
        if egalite_de_points && vainqueur.is_some() {
            // **ET DANS LE BON SENS.** Vérifier que la cible est dissymétrique
            // ne suffisait pas : on pouvait inverser le vainqueur passé à
            // `cible_finale_departagee` — apprendre à chaque joueur le résultat
            // de l'autre — sans que le compteur bouge d'une unité, parce que la
            // cible restait tout aussi dissymétrique, à l'envers près. Les
            // sommes restaient justes ; seul le SENS de l'apprentissage
            // s'inversait, et c'est exactement la forme de défaut qu'un test
            // qui ne remonte pas jusqu'à la donnée d'origine ne voit jamais.
            //
            // On confronte donc la cible au vainqueur du livret LU À NEUF, sans
            // repasser par l'expression qui construit `je_gagne` : si la cible
            // désigne comme gagnant quelqu'un d'autre que `flow::winner`, la
            // partie n'est pas comptée, et le chiffre publié tombe à zéro.
            if la_cible_me_donne_gagnant == je_devrais_gagner {
                departages += 1;
            }
        }
        j.reseau.corriger(j.pile, p, cible, j.taux);
    }

    // ---- le vainqueur était-il celui qu'on donnait gagnant à mi-partie ?
    let mut bilan = Bilan {
        contresens,
        // Les DEUX joueurs doivent avoir reçu une cible départagée : une partie
        // où un seul l'aurait reçue serait une incohérence, pas un départage.
        egalite_departagee: departages == 2,
        nulle_jusqu_au_bout: egalite_de_points && vainqueur.is_none(),
        ..Bilan::default()
    };
    if scores[0] != scores[1] && !j.predictions.is_empty() {
        let milieu = j.predictions.len() / 2;
        let predit_0 = j.predictions[milieu] > 0.5;
        bilan.decisive = true;
        bilan.juste = predit_0 == (scores[0] > scores[1]);
    }
    bilan
}

/// Les réglages d'un joueur d'entraînement, recopiés à l'identique dans chaque
/// ouvrier : deux ouvriers qui ne joueraient pas avec les mêmes réglages ne
/// seraient pas le même entraînement.
#[derive(Clone, Copy)]
struct Reglages {
    exploration: f64,
    rythme: u64,
    devinette: bool,
    /// (L5, §E1) Le droit de vendre une carte, allumé par défaut comme avant ce
    /// lot. Éteint, il retire les options de vente de chaque décision : c'est le
    /// poste dont le §E veut connaître le prix en temps ET en force.
    vente: bool,
}

/// Les compteurs de mise au point qu'un `Joueur` accumule et que le journal final
/// publie. À plusieurs ouvriers ils s'additionnent, dans l'ordre des graines.
#[derive(Default, Clone, Copy)]
struct Cumuls {
    essais: u64,
    t_essais: f64,
    t_apprentissage: f64,
    passes: u64,
    pas_avance: u64,
    plafonds: u64,
    corrections_adversaire: u64,
    sautees_adversaire: u64,
    phases_rencontrees: u64,
    /// (L5, §E1) Cartes vendues volontairement, et occasions offertes.
    ventes_volontaires: u64,
    occasions_de_vente: u64,
}

impl Cumuls {
    /// Les compteurs d'un ouvrier qui vient de finir sa partie.
    fn absorber_ouvrier(&mut self, autre: &Cumuls) {
        self.essais += autre.essais;
        self.t_essais += autre.t_essais;
        self.t_apprentissage += autre.t_apprentissage;
        self.passes += autre.passes;
        self.pas_avance += autre.pas_avance;
        self.plafonds += autre.plafonds;
        self.corrections_adversaire += autre.corrections_adversaire;
        self.sautees_adversaire += autre.sautees_adversaire;
        self.phases_rencontrees += autre.phases_rencontrees;
        self.ventes_volontaires += autre.ventes_volontaires;
        self.occasions_de_vente += autre.occasions_de_vente;
    }

    fn absorber(&mut self, j: &Joueur) {
        self.essais += j.essais;
        self.t_essais += j.t_essais;
        self.t_apprentissage += j.t_apprentissage;
        self.passes += j.passes;
        self.pas_avance += j.pas_avance;
        self.plafonds += j.plafonds;
        self.corrections_adversaire += j.corrections_adversaire;
        self.sautees_adversaire += j.sautees_adversaire;
        self.phases_rencontrees += j.phases_rencontrees;
        self.ventes_volontaires += j.ventes_volontaires;
        self.occasions_de_vente += j.occasions_de_vente;
    }
}

/// **(L5, §2.7) UN OUVRIER.** Il possède sa copie des poids, sa pile de
/// situations et, s'il y en a un, sa copie du second réseau. Il ne partage rien
/// avec ses voisins : c'est ce qui permet de le faire tourner sur un autre cœur
/// sans verrou, et de garder le déterminisme.
struct Ouvrier {
    reseau: Reseau,
    pile: Pile,
    adversaire: Option<ReseauPhases>,
    bilan: Bilan,
    cumuls: Cumuls,
}

impl Ouvrier {
    fn neuf(taille: usize, avec_adversaire: bool, largeur: usize) -> Ouvrier {
        Ouvrier {
            reseau: Reseau::neuf_largeur(taille, largeur),
            pile: Pile::new(taille),
            adversaire: if avec_adversaire {
                Some(ReseauPhases::neuf_largeur(taille, largeur))
            } else {
                None
            },
            bilan: Bilan::default(),
            cumuls: Cumuls::default(),
        }
    }

    /// La partie de ce groupe, jouée sur la copie de l'ouvrier. C'est le seul
    /// code qui tourne sur un autre cœur, et il ne touche à rien de partagé :
    /// ses poids, sa pile, ses tampons, et la base de cartes en lecture seule.
    fn travailler(&mut self, db: &CardsDb, desc: &Description, reglages: Reglages, graine: u64) {
        let mut cumuls = Cumuls::default();
        let bilan;
        {
            let mut j = Joueur::new(db, desc, &mut self.reseau, &mut self.pile, graine);
            j.exploration = reglages.exploration;
            j.apprendre = true;
            j.rythme = reglages.rythme;
            j.adversaire = self.adversaire.as_mut();
            j.devinette = reglages.devinette;
            j.vente = reglages.vente;
            bilan = une_partie(&mut j, desc, graine);
            cumuls.absorber(&j);
        }
        self.bilan = bilan;
        self.cumuls = cumuls;
    }
}

/// Le relevé de couche cachée qu'on publie en fin de journal — pris une fois,
/// pour que la lecture ne dépende pas de quel chemin a tourné.
struct Releve {
    pente: f64,
    part_saturee: f64,
    figes: usize,
    situations: u64,
    amplitude_min: f64,
}

impl Releve {
    fn de(r: &Reseau) -> Releve {
        Releve {
            pente: r.pente_moyenne(),
            part_saturee: r.part_saturee(),
            figes: r.neurones_figes(),
            situations: r.situations_vues(),
            amplitude_min: r.amplitude_minimale(),
        }
    }
}

/// **LE SUIVI D'UN ENTRAÎNEMENT** : les compteurs de tranche, le journal, les
/// écritures de sûreté et les instantanés. Un seul exemplaire, partagé par le
/// chemin à un ouvrier et par celui à plusieurs — sans quoi les deux
/// publieraient des choses différentes.
struct Suivi<'a> {
    noms: &'a [String],
    sortie: String,
    parties: u64,
    parties_reprises: u64,
    tranche: u64,
    debut_mesure: u64,
    instantanes: Vec<u64>,
    instantanes_faits: Vec<u64>,
    justes_tranche: u64,
    decisives_tranche: u64,
    egalites_departagees: u64,
    /// Cumul du garde-fou de sens : voir `Bilan::contresens`. Publié en fin de
    /// journal, et il doit rester à zéro.
    contresens: u64,
    nulles_jusqu_au_bout: u64,
    derniere_sauvegarde: Instant,
    t0: Instant,
    cumuls: Cumuls,
}

impl Suivi<'_> {
    /// Faut-il relever la couche cachée pour un groupe de `taille` parties qui
    /// commence au rang `g` ? Oui dès que le groupe **mord** sur la dernière
    /// tranche : sinon, un groupe de quatre qui commence juste avant la
    /// frontière ferait manquer la mesure entière.
    fn mesurer(&self, g: u64, taille: u64) -> bool {
        g + taille > self.debut_mesure
    }

    /// Tout ce qui suit un groupe de parties : compteurs, ligne de journal,
    /// écriture de sûreté, instantanés. `g_fin` est le rang (dans la session) de
    /// la dernière partie du groupe, plus un.
    fn apres_groupe(&mut self, g_fin: u64, bilans: &[Bilan], reseau: &mut Reseau) {
        for bilan in bilans.iter() {
            if bilan.decisive {
                self.decisives_tranche += 1;
                if bilan.juste {
                    self.justes_tranche += 1;
                }
            }
            if bilan.egalite_departagee {
                self.egalites_departagees += 1;
            }
            if bilan.nulle_jusqu_au_bout {
                self.nulles_jusqu_au_bout += 1;
            }
            self.contresens += bilan.contresens as u64;
        }

        // **LA FRONTIÈRE SE FRANCHIT, ELLE NE COÏNCIDE PAS.** À plusieurs
        // ouvriers, `g_fin` avance par pas de `--ouvriers` : le test
        // `g_fin % tranche == 0` n'est vrai que sur les multiples COMMUNS de la
        // tranche et du nombre d'ouvriers. À 1 000 parties, tranche 50 et
        // 4 ouvriers, il rendait 10 lignes de journal au lieu de 20 — et la
        // sauvegarde de sûreté du §2.6, qui est le seul filet d'un entraînement
        // de plusieurs heures, sautait une tranche sur deux. On compare donc les
        // numéros de tranche du début et de la fin du groupe.
        let debut_du_groupe = g_fin - bilans.len() as u64;
        let fin_de_tranche =
            g_fin / self.tranche > debut_du_groupe / self.tranche || g_fin == self.parties;
        if fin_de_tranche {
            let justes = if self.decisives_tranche > 0 {
                self.justes_tranche as f64 / self.decisives_tranche as f64
            } else {
                0.0
            };
            println!(
                "{{\"parties\": {}, \"erreur\": {:.6}, \"justes\": {:.4}}}",
                g_fin,
                reseau.erreur_moyenne(),
                justes
            );
            use std::io::Write;
            let _ = std::io::stdout().flush();
            reseau.raz_stats();
            self.justes_tranche = 0;
            self.decisives_tranche = 0;
        }

        // (L5, §2.6) Les poids sur le disque : à chaque tranche, et au plus
        // SAUVEGARDE_SECONDES après la dernière écriture. Une coupure ne perd
        // plus que ce qui s'est appris depuis.
        if fin_de_tranche
            || self.derniere_sauvegarde.elapsed().as_secs_f64() >= SAUVEGARDE_SECONDES
        {
            sauver(reseau, &self.sortie, self.noms);
            self.derniere_sauvegarde = Instant::now();
        }

        // (L5, §2.6) Les instantanés de la courbe de force (§6), NOMMÉS PAR LE
        // NOMBRE ABSOLU DE PARTIES.
        //
        // Le nom était construit sur le rang dans la session (`g + 1`) alors que
        // le compteur inscrit DANS le fichier est le total absolu : un fichier
        // nommé `.200000` pouvait contenir un réseau à 1 200 000 parties, et le
        // dépôt en porte un — `data/poids/apprenti-2M.txt.200000`. C'est le même
        // défaut qui faisait mentir `--instantanes` sur une reprise : demander
        // « un instantané à 500 000 parties » sur un fichier repris à 400 000 en
        // exigeait 500 000 de PLUS. Les deux se corrigent d'un seul geste — le
        // rang de la session disparaît, il ne reste que le compteur absolu.
        for k in 0..bilans.len() as u64 {
            let n = self.parties_reprises + g_fin - bilans.len() as u64 + k + 1;
            if self.instantanes.contains(&n) && !self.instantanes_faits.contains(&n) {
                self.instantanes_faits.push(n);
                let chemin = format!("{}.{n}", self.sortie);
                if let Err(e) = reseau.ecrire(&chemin, self.noms) {
                    eprintln!("entraine: instantané {chemin} non écrit : {e}");
                } else {
                    eprintln!(
                        "instantané : {chemin} ({n} parties au total, {:.1} s)",
                        self.t0.elapsed().as_secs_f64()
                    );
                }
            }
        }
    }
}

/// **(L5, §2.6) ÉCRIRE LES POIDS SANS RISQUER DE LES PERDRE.**
///
/// On écrit à côté puis on renomme : un renommage est atomique sur le même
/// système de fichiers, donc une coupure pendant l'écriture laisse le fichier
/// PRÉCÉDENT intact au lieu d'un fichier tronqué. Écrire directement sur la
/// cible échangerait « on perd tout depuis le dernier instantané » contre « on
/// perd tout si la coupure tombe pendant une écriture ».
fn sauver(reseau: &Reseau, sortie: &str, noms: &[String]) {
    let provisoire = format!("{sortie}.en-cours");
    if let Err(e) = reseau.ecrire(&provisoire, noms) {
        eprintln!("entraine: écriture de {provisoire} : {e}");
        return;
    }
    if let Err(e) = std::fs::rename(&provisoire, sortie) {
        eprintln!("entraine: renommage vers {sortie} : {e}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parties: u64 = 10_000;
    let mut graine_debut: u64 = 100_000;
    let mut sortie = String::new();
    let mut exploration: f64 = 0.05;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut instantanes: Vec<u64> = Vec::new();
    let mut sans_optimisation = false;
    // (L5, §E1) La vente reste allumée par défaut : ce lot ne change pas le
    // joueur, il donne seulement de quoi MESURER ce qu'elle coûte.
    let mut vente = true;
    let mut lambda = reseau::LAMBDA;
    let mut rythme = reseau::RYTHME;
    let mut amplitude = reseau::AMPLITUDE_DEPART;
    // (L5, §2.7) Le nombre d'ouvriers. Quatre par défaut : la machine a quatre
    // cœurs physiques, et le §2.7 les demande tous.
    let mut ouvriers: usize = 4;
    // (la-largeur-reglable) La largeur de la couche cachée. `reseau::CACHES` par
    // défaut : sans l'option, l'entraînement produit exactement le fichier
    // d'avant ce chantier.
    let mut largeur: usize = reseau::CACHES;
    // (il-devine) Les quatre options du point d'accroche n°1.
    let mut sortie_adversaire = String::new();
    let mut reprise = String::new();
    let mut reprise_adversaire = String::new();
    let mut devinette = false;
    let mut i = 1;
    while i < args.len() {
        let mut avance = 2;
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--parties" => parties = val(i).parse().unwrap_or_else(|_| mourir("--parties")),
            "--graine-debut" => {
                graine_debut = val(i).parse().unwrap_or_else(|_| mourir("--graine-debut"))
            }
            "--sortie" => sortie = val(i),
            "--exploration" => {
                exploration = val(i).parse().unwrap_or_else(|_| mourir("--exploration"))
            }
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            "--instantanes" => {
                instantanes = val(i)
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().parse().unwrap_or_else(|_| mourir("--instantanes")))
                    .collect()
            }
            "--lambda" => lambda = val(i).parse().unwrap_or_else(|_| mourir("--lambda")),
            "--rythme" => rythme = val(i).parse().unwrap_or_else(|_| mourir("--rythme")),
            "--amplitude-depart" => {
                amplitude = val(i).parse().unwrap_or_else(|_| mourir("--amplitude-depart"))
            }
            "--ouvriers" => {
                ouvriers = val(i).parse().unwrap_or_else(|_| mourir("--ouvriers"));
                if ouvriers == 0 {
                    mourir("--ouvriers 0 : il en faut au moins un");
                }
            }
            "--largeur" => {
                largeur = val(i).parse().unwrap_or_else(|_| mourir("--largeur"));
                if largeur == 0 {
                    mourir("--largeur 0 : la couche cachée a au moins un neurone");
                }
                // Le même plafond qu'en relecture : un entraînement dont le
                // fichier serait ensuite refusé par `Reseau::lire` ne servirait
                // à rien, et il vaut mieux le dire avant les heures de calcul.
                if largeur > reseau::LARGEUR_MAX {
                    mourir(&format!(
                        "--largeur {largeur} : au plus {} neurones cachés (le fichier produit serait refusé en relecture)",
                        reseau::LARGEUR_MAX
                    ));
                }
            }
            "--vente" => {
                vente = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!("--vente attend « on » ou « off », pas « {autre} »")),
                };
            }
            "--sans-optimisation" => {
                sans_optimisation = true;
                avance = 1;
            }
            // (il-devine) Point d'accroche n°1 — les quatre options du chantier.
            "--sortie-adversaire" => sortie_adversaire = val(i),
            "--reprise" => reprise = val(i),
            "--reprise-adversaire" => reprise_adversaire = val(i),
            "--devinette" => {
                devinette = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!("--devinette attend « on » ou « off », pas « {autre} »")),
                }
            }
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += avance;
    }
    if sortie.is_empty() {
        mourir("--sortie est obligatoire");
    }
    if graine_debut < 100_000 {
        mourir("l'entraînement est confiné aux graines 100000 et au-delà (prompt § « Où tu as le droit de régler »)");
    }

    // ------------------------------------------------------------------
    // (L5, §2.6) LA PREMIÈRE LIGNE DU JOURNAL : CE QUI L'A PRODUIT.
    //
    // « Un journal qu'on ne peut pas rejouer ne prouve rien. » La ligne de
    // commande complète et la plage de graines consommée, avant tout le reste —
    // avant même l'amorçage, pour qu'une coupure précoce laisse quand même de
    // quoi rejouer.
    // ------------------------------------------------------------------
    let ligne_de_commande = args.join(" ");
    println!(
        "{{\"commande\": {}, \"graines\": \"{}..{}\", \"parties\": {}, \"boites\": \"{}\", \
         \"ouvriers\": {}, \"largeur\": {}, \"amplitude_depart\": {}, \"lambda\": {}, \
         \"rythme\": {}, \"exploration\": {}, \"reprise\": \"{}\"}}",
        serde_json::to_string(&ligne_de_commande).unwrap_or_else(|_| "\"?\"".into()),
        graine_debut,
        graine_debut + parties.saturating_sub(1),
        parties,
        boites_txt,
        ouvriers,
        largeur,
        amplitude,
        lambda,
        rythme,
        exploration,
        reprise
    );
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let boites = match BoiteSet::parse(&boites_txt) {
        Ok(b) => b,
        Err(e) => mourir(&e),
    };
    let chemin = if std::path::Path::new(&cartes).exists() {
        cartes.clone()
    } else {
        format!("../{cartes}")
    };
    let db = match CardsDb::load_boites(&chemin, boites) {
        Ok(db) => db,
        Err(e) => mourir(&e),
    };
    let desc = Description::new(&db);
    let noms = desc.noms_avec(&db);

    // ------------------------------------------------------------------
    // (il-devine, §5) LA REPRISE D'ENTRAÎNEMENT
    //
    // « `entraine.rs` crée TOUJOURS un réseau neuf. Il n'existe aucun moyen de
    // continuer un entraînement interrompu, ni d'en repartir après un
    // changement. » `--reprise` charge un fichier de poids existant au lieu de
    // tirer des valeurs au hasard, et `--reprise-adversaire` fait de même pour le
    // second réseau, indépendamment.
    //
    // **Les trois refus du §5 sont ceux de `Reseau::lire`** : un nombre d'entrées
    // qui ne correspond pas, un nombre de cachés ou de sorties qui ne correspond
    // pas, et — le plus important et le moins évident — des noms d'entrées qui ne
    // correspondent pas un à un et dans le même ordre. Deux descriptions peuvent
    // avoir le même nombre d'entrées et ne pas décrire la même chose ; reprendre
    // là-dessus donne un réseau qui a l'air de marcher et qui a tout appris de
    // travers. Le message est clair et le code de sortie non nul (`mourir`).
    //
    // **La validation a lieu ICI, avant l'amorçage**, et c'est délibéré : un refus
    // doit être instantané. Placé après, il coûterait les quelques minutes de
    // l'amorçage pour dire « non ».
    // ------------------------------------------------------------------
    let mut reseau = if reprise.is_empty() {
        Reseau::neuf_amplitude_largeur(desc.taille, amplitude, largeur)
    } else {
        // **La reprise passe par le verrou de largeur** : reprendre des poids
        // appris à cent neurones dans un entraînement lancé à cinquante donnerait
        // un réseau qui a l'air de marcher et qui a tout appris de travers.
        match Reseau::lire_largeur(&reprise, &noms, largeur) {
            Ok(r) => {
                eprintln!("reprise : {reprise} — {} parties déjà vues", r.parties);
                r
            }
            Err(e) => mourir(&format!("reprise refusée — {e}")),
        }
    };
    let parties_reprises = reseau.parties;
    reseau.sans_optimisation = sans_optimisation;
    reseau.lambda = lambda;
    let mut pile = Pile::new(desc.taille);

    // Le second réseau n'existe que si on demande où l'écrire (point d'accroche
    // n°1 : « Absente : pas de second réseau, l'entraînement se comporte
    // exactement comme aujourd'hui »).
    if !reprise_adversaire.is_empty() && sortie_adversaire.is_empty() {
        mourir("--reprise-adversaire sans --sortie-adversaire : le second réseau serait chargé puis jeté");
    }
    let mut adversaire: Option<ReseauPhases> = if sortie_adversaire.is_empty() {
        None
    } else if reprise_adversaire.is_empty() {
        Some(ReseauPhases::neuf_amplitude_largeur(desc.taille, amplitude, largeur))
    } else {
        match ReseauPhases::lire_largeur(&reprise_adversaire, &noms, largeur) {
            Ok(r) => {
                eprintln!(
                    "reprise du second réseau : {reprise_adversaire} — {} parties déjà vues",
                    r.parties
                );
                Some(r)
            }
            Err(e) => mourir(&format!("reprise du second réseau refusée — {e}")),
        }
    };
    let parties_reprises_adversaire = adversaire.as_ref().map(|r| r.parties).unwrap_or(0);

    // §4 : « Un joueur à qui on ne donne pas de second réseau doit jouer comme
    // avant, pas planter et pas se dégrader en silence. » On le dit donc.
    if devinette && adversaire.is_none() {
        eprintln!(
            "entraine: --devinette on sans --sortie-adversaire : il n'y a aucun second réseau, \
             la devinette reste ÉTEINTE (le joueur répond la première option à la place de l'autre)"
        );
        devinette = false;
    }

    let t0 = Instant::now();
    if reprise.is_empty() {
        amorcer(&mut reseau, &noms, reseau::GRAINE_POIDS);
        eprintln!("amorçage : {AMORCAGE_PARTIES} fins de partie fabriquées en {:.1} s (scores tirés entre 0 et {AMORCAGE_SCORE_MAX})", t0.elapsed().as_secs_f64());
    } else {
        // **Pas d'amorçage sur une reprise, et c'est le fond du §5.** L'amorçage
        // du §2.7 existe parce qu'« un réseau tiré au hasard ne sait même pas que
        // plus de points, c'est mieux ». Un réseau repris le sait déjà : lui
        // repasser cinq mille fins de partie fabriquées au taux multiplié par dix
        // écraserait une partie de ce qu'il a appris. Reprendre, c'est continuer,
        // pas recommencer.
        eprintln!("amorçage sauté : on reprend des poids déjà entraînés");
    }


    // ------------------------------------------------------------------
    // LE SUIVI : compteurs de tranche, journal, sauvegardes, instantanés.
    // ------------------------------------------------------------------
    let tranche = (parties / 20).max(1);
    let mut suivi = Suivi {
        noms: &noms,
        sortie: sortie.clone(),
        parties,
        parties_reprises,
        tranche,
        // (L5, livrable 5) La couche cachée n'est relevée que sur la DERNIÈRE
        // tranche — celle que le journal publie. `evaluer` est la boucle la plus
        // chaude du dépôt : la mesurer sur toute la course coûterait du temps à
        // chaque partie pour un chiffre qui n'est lu qu'une seule fois.
        debut_mesure: parties.saturating_sub(tranche),
        instantanes,
        instantanes_faits: Vec::new(),
        justes_tranche: 0,
        decisives_tranche: 0,
        egalites_departagees: 0,
        nulles_jusqu_au_bout: 0,
        contresens: 0,
        derniere_sauvegarde: Instant::now(),
        t0,
        cumuls: Cumuls::default(),
    };

    let reglages = Reglages {
        exploration,
        rythme,
        devinette,
        vente,
    };
    let releve: Releve;

    if ouvriers == 1 {
        // ---- LE CHEMIN À UN SEUL OUVRIER : l'entraînement séquentiel, celui
        // d'avant ce lot. Le joueur travaille directement sur les poids du
        // maître ; il n'y a ni copie, ni différence, ni fil d'exécution.
        let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, graine_debut);
        j.exploration = exploration;
        j.apprendre = true;
        j.rythme = rythme;
        j.adversaire = adversaire.as_mut();
        j.devinette = devinette;
        j.vente = vente;
        for g in 0..parties {
            j.reseau.mesurer_couche = suivi.mesurer(g, 1);
            let bilan = une_partie(&mut j, &desc, graine_debut + g);
            // (il-devine, §5) « Le compteur de parties du fichier repris
            // S'AJOUTE aux nouvelles : un fichier à 200 000 parties repris pour
            // 50 000 de plus en annonce 250 000. »
            j.reseau.parties = parties_reprises + g + 1;
            if let Some(a) = j.adversaire.as_deref_mut() {
                a.parties = parties_reprises_adversaire + g + 1;
            }
            suivi.apres_groupe(g + 1, &[bilan], &mut *j.reseau);
        }
        suivi.cumuls.absorber(&j);
        releve = Releve::de(&*j.reseau);
    } else {
        // ---- LE CHEMIN À PLUSIEURS OUVRIERS (§2.7).
        //
        // Au début d'un groupe de `ouvriers` parties, chaque ouvrier RECOPIE les
        // poids courants ; il joue sa partie en appliquant ses corrections sur sa
        // copie ; à la fin du groupe on additionne les différences **dans
        // l'ordre des graines**, jamais dans l'ordre d'arrivée. Deux exécutions
        // identiques font donc exactement les mêmes additions dans le même
        // ordre : le déterminisme octet pour octet tient.
        //
        // Ce qui est refusé d'avance, et qui n'est pas ici : la mise à jour
        // concurrente sans verrou des poids partagés (chaque correction touche
        // 81 550 poids, les cœurs s'entre-gêneraient), et la répartition des
        // OPTIONS d'une même décision (elle annulerait la mise à jour par
        // différences qui fait passer une évaluation de 24,8 à 3,1 µs).
        //
        // **Ce que cela change à l'apprentissage** : quatre parties apprennent
        // depuis les mêmes poids au lieu de s'enchaîner. C'est le regroupement
        // par quatre, déclaré dans l'audit et accepté d'avance — et sa
        // conséquence sur la FORCE est mesurée en duel, pas supposée.
        let mut base = Reseau::neuf_largeur(desc.taille, largeur);
        let mut base_adversaire: Option<ReseauPhases> = if adversaire.is_some() {
            Some(ReseauPhases::neuf_largeur(desc.taille, largeur))
        } else {
            None
        };
        let mut equipe: Vec<Ouvrier> = (0..ouvriers)
            .map(|_| Ouvrier::neuf(desc.taille, adversaire.is_some(), largeur))
            .collect();

        let mut g: u64 = 0;
        while g < parties {
            let taille = (parties - g).min(ouvriers as u64) as usize;
            let mesurer = suivi.mesurer(g, taille as u64);
            base.copier_les_poids_de(&reseau);
            if let (Some(ba), Some(a)) = (base_adversaire.as_mut(), adversaire.as_ref()) {
                ba.copier_les_poids_de(a);
            }
            for o in equipe.iter_mut().take(taille) {
                o.reseau.copier_les_poids_de(&reseau);
                o.reseau.mesurer_couche = mesurer;
                o.reseau.raz_stats();
                o.reseau.raz_couche();
                if let (Some(oa), Some(a)) = (o.adversaire.as_mut(), adversaire.as_ref()) {
                    oa.copier_les_poids_de(a);
                    // **LE SECOND RÉSEAU SE REMET À ZÉRO COMME LE PREMIER.**
                    // `copier_les_poids_de` vide l'accumulateur mais pas les
                    // statistiques : sans ces deux lignes,
                    // `absorber_les_statistiques` reverse à chaque groupe le
                    // cumul de l'ouvrier DEPUIS LE DÉBUT, et l'erreur du second
                    // réseau croît en O(n²). Rien ne le publie aujourd'hui —
                    // c'est une mine, pas une panne, et on la désamorce.
                    oa.raz_stats();
                    oa.raz_couche();
                }
            }
            let refdb = &db;
            let refdesc = &desc;
            std::thread::scope(|portee| {
                for (k, o) in equipe.iter_mut().take(taille).enumerate() {
                    let graine = graine_debut + g + k as u64;
                    portee.spawn(move || o.travailler(refdb, refdesc, reglages, graine));
                }
            });
            let mut bilans: Vec<Bilan> = Vec::with_capacity(taille);
            // **L'ORDRE DES GRAINES, ET RIEN D'AUTRE.** C'est ici que le
            // déterminisme se joue.
            for o in equipe.iter().take(taille) {
                reseau.ajouter_la_difference(&base, &o.reseau);
                reseau.absorber_les_statistiques(&o.reseau);
                if let (Some(a), Some(ba), Some(oa)) = (
                    adversaire.as_mut(),
                    base_adversaire.as_ref(),
                    o.adversaire.as_ref(),
                ) {
                    a.ajouter_la_difference(ba, oa);
                    a.absorber_les_statistiques(oa);
                }
                suivi.cumuls.absorber_ouvrier(&o.cumuls);
                bilans.push(o.bilan);
            }
            g += taille as u64;
            reseau.parties = parties_reprises + g;
            if let Some(a) = adversaire.as_mut() {
                a.parties = parties_reprises_adversaire + g;
            }
            suivi.apres_groupe(g, &bilans, &mut reseau);
        }
        releve = Releve::de(&reseau);
    }

    if let Err(e) = reseau.ecrire(&sortie, &noms) {
        mourir(&format!("écriture de {sortie} : {e}"));
    }
    // (il-devine, §2.4) Le second réseau va dans SON PROPRE fichier, au même
    // format que le premier — seul le troisième nombre de la première ligne
    // change, 5 au lieu de 2. Deux fichiers, deux verrous indépendants.
    if let Some(a) = adversaire.as_ref() {
        if let Err(e) = a.ecrire(&sortie_adversaire, &noms) {
            mourir(&format!("écriture de {sortie_adversaire} : {e}"));
        }
        eprintln!(
            "second réseau : {sortie_adversaire} — {} corrections, \
             {} sautées (meilleure note nulle ou négative, §2.2)",
            suivi.cumuls.corrections_adversaire, suivi.cumuls.sautees_adversaire
        );
        if devinette {
            eprintln!(
                "  devinette allumée : {} `pick_phase` adverses rencontrés \
                 pendant les avances ({:.3} par essai d'option, §8)",
                suivi.cumuls.phases_rencontrees,
                suivi.cumuls.phases_rencontrees as f64 / suivi.cumuls.essais.max(1) as f64
            );
        }
    }

    // ------------------------------------------------------------------
    // (L5, livrable 5) LES CHIFFRES QUE LE PROGRAMME PUBLIE.
    //
    // « Une correction qu'on ne peut pas voir de l'extérieur ne se contrôle
    // pas. » Ces deux lignes ferment le journal.
    // ------------------------------------------------------------------
    println!(
        "egalites_departagees: {} sur {parties} parties ({:.2} %, dont {} \
         partie(s) nulle(s) jusqu'au total de departage)",
        suivi.egalites_departagees,
        100.0 * suivi.egalites_departagees as f64 / parties.max(1) as f64,
        suivi.nulles_jusqu_au_bout
    );
    // **LE GARDE-FOU DE SENS, PUBLIÉ.** Un entraînement de plusieurs heures qui
    // apprendrait à l'envers doit le dire, et le dire tôt : ce nombre vaut zéro
    // ou le fichier de poids produit ne vaut rien.
    println!("corrections_a_contresens: {}", suivi.contresens);
    println!(
        "couche_cachee (derniere tranche, {} situations) : pente_moyenne {:.4}, \
         part_saturee {:.1} %, neurones_figes {} sur {}, amplitude_minimale {:.4}",
        releve.situations, releve.pente, releve.part_saturee, releve.figes,
        largeur, releve.amplitude_min
    );
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    // (L5, §E1) Le chiffre que le §E réclame : combien l'IA vend par partie.
    println!(
        "ventes: {} volontaires sur {} occasions ({:.1} par partie, vente {})",
        suivi.cumuls.ventes_volontaires,
        suivi.cumuls.occasions_de_vente,
        suivi.cumuls.ventes_volontaires as f64 / parties.max(1) as f64,
        if vente { "on" } else { "off" }
    );
    eprintln!(
        "fini : {parties} parties, {} essais d'option, {:.1} s ({:.1} ms par partie, {ouvriers} ouvrier(s))",
        suivi.cumuls.essais,
        t0.elapsed().as_secs_f64(),
        1000.0 * t0.elapsed().as_secs_f64() / parties as f64
    );
    eprintln!(
        "  dont essais {:.1} s, apprentissage {:.1} s ({} passes, rythme K={rythme}, λ={lambda}) — \
         sommes sur tous les ouvriers, et non du temps d'horloge",
        suivi.cumuls.t_essais, suivi.cumuls.t_apprentissage, suivi.cumuls.passes
    );
    // §4.1 : « result.md doit dire combien de fois le plafond a été atteint ».
    eprintln!(
        "  avance vers le repère (§4.1) : {} pas au total ({:.2} par essai), plafond de {} atteint {} fois",
        suivi.cumuls.pas_avance,
        suivi.cumuls.pas_avance as f64 / suivi.cumuls.essais.max(1) as f64,
        rejeu::PLAFOND_AVANCE,
        suivi.cumuls.plafonds,
    );
}

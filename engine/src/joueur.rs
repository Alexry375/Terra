//! **(le-juge-apprend) LE JOUEUR QUI ESSAIE SES OPTIONS, côté Rust.**
//!
//! À chaque point de décision (§4) : pour chaque option proposée par le moteur,
//! construire la réponse correspondante, demander au moteur l'état qui en
//! résulterait, décrire cet état **du point de vue du joueur qui décidait**, le
//! faire passer dans le réseau, et garder l'option dont la probabilité de
//! victoire est la plus haute.
//!
//! **Comment on obtient « l'état qui en résulterait » sans pont.** Une manche du
//! moteur n'est pas reprenable au milieu : `play_round` déroule la planification
//! puis les cinq phases, et rien ne permet d'y rentrer en cours de route. On
//! garde donc un clone de l'état **au début de la manche** et on rejoue la manche
//! depuis là, avec les réponses déjà données plus l'option essayée
//! (`rejeu::Rejeu`, le jumeau natif du harnais du pont). C'est le même principe
//! que le pont — « la partie EST la graine plus la liste des décisions » — mais
//! borné à une manche au lieu d'une partie : mesuré à quelques microsecondes par
//! essai, là où rejouer depuis la graine en coûterait deux cents.
//!
//! Les décisions de la mise en place (mulligans, corporation) précèdent la
//! première manche : leur point de reprise est `setup_game` lui-même.
//!
//! **Les choix multiples** (garder k cartes parmi n, en défausser n) n'ont pas
//! une option par réponse mais une combinaison : on les construit **de proche en
//! proche**, en essayant à chaque tour chacune des cartes qui restent et en
//! gardant la meilleure. Chaque carte ajoutée est donc, elle aussi, essayée.
//!
//! **LE REPÈRE DU §4.1 (corrigé le 15-08).** Appliquer l'option ne suffit pas :
//! l'état qui en résulte ne se trouve pas au même endroit de la partie selon
//! l'option. « Poser une carte » mène à la décision d'après, « passer » mène
//! beaucoup plus loin — la phase se termine, la production est encaissée, la
//! manche suivante commence — et plus tard paraît toujours meilleur. On avance
//! donc, après l'option, **jusqu'au prochain point de décision du joueur qui
//! choisit** : toutes les options sont jugées au même instant, « la prochaine
//! fois que j'aurai la main ». L'avance répond à la place de l'autre (`Premiere`)
//! et ne dépasse jamais soixante pas.
//!
//! **L'apprentissage** (§2) vit ici parce que son point de rendez-vous est une
//! décision : une prise sur K (K = 8), sur l'état du repère de l'option retenue —
//! celui-là même que le joueur vient de juger, et aucun autre.

use crate::cards::CardsDb;
use crate::choice::ChoiceContext;
use crate::effects::RevealFilter;
use crate::flow::{play_round, setup_game};
use crate::policy::{ActionOpt, ConstructionBonus, Policy};
use crate::sim::MAX_GENERATIONS;
use crate::state::GameState;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};

use crate::description::{Description, Tampons};
use crate::rejeu::{Devinette, Rejeu};
use crate::reseau::{cible_phases, Pile, Reseau, ReseauPhases, FACTEUR_CIBLE, PHASES, TAUX_ADVERSAIRE};

/// **La marge de départage, et pourquoi elle est indispensable.**
///
/// Deux options peuvent mener à des situations rigoureusement identiques — c'est
/// courant : huit cartes vertes à rejouer dont sept ne changent rien d'observable.
/// Leurs notes sont alors égales *en arithmétique réelle*, mais pas au dernier bit :
/// la mise à jour par différences du §1.1 additionne les sommes cachées dans un
/// ordre qui dépend de l'option évaluée juste avant, et le JavaScript, lui, refait
/// chaque évaluation en entier. Mesuré sur une décision réelle (graine 2, rang 361) :
/// 0,46726923574014501 d'un côté, 0,46726923574014506 de l'autre — cinq
/// dix-millionièmes de milliardième d'écart, et deux joueurs qui ne choisissent plus
/// la même option.
///
/// On ne départage donc une option que si elle est meilleure d'une marge qui dépasse
/// franchement le bruit de calcul ; à égalité, la PREMIÈRE l'emporte. Les deux côtés
/// appliquent la même règle, avec la même marge.
pub const MARGE: f64 = 1e-12;

/// **(le-joueur-sans-voyance, V1) LA GRAINE DES ESSAIS PAR DÉFAUT.**
///
/// Zéro n'est pas « pas de graine » : c'est une graine comme une autre. Elle est
/// mélangée à la graine de la partie et au rang de la décision, si bien que deux
/// parties différentes n'explorent pas le même avenir imaginaire même quand
/// personne n'a passé `--graine-essais`.
pub const GRAINE_ESSAIS_DEFAUT: u64 = 0;

/// **(2.11) Jusqu'où l'énumération complète est permise.** Huit cartes = 256
/// sous-ensembles, la taille exacte de la main de départ. Au-delà, on retombe sur
/// la construction carte par carte : voir le commentaire de `choisir_liste`.
pub const LARGEUR_ENUMERATION: usize = 8;

/// Le brasseur de `splitmix64` : il transforme un compteur en graine sans
/// structure. Employé pour dériver la graine d'un rejeu d'essai de trois
/// nombres — la graine des essais, celle de la partie, le rang de la décision —
/// et pour tirer le décalage d'apprentissage du §2.14. Déterministe, donc
/// rejouable au dernier bit : c'est la contrainte qui prime sur tout le reste.
pub fn brasser(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// **(V1) CE QUE LE JOUEUR N'A PAS LE DROIT DE CONNAÎTRE, ET QU'ON LUI RETIRE.**
///
/// L'essai d'un coup rejouait la partie avec la graine RÉELLE : le paquet y était
/// mélangé exactement comme dans la vraie partie, et le joueur lisait donc à
/// l'avance les cartes qu'il allait recevoir. Démontré le 18-08 sur la graine
/// 700001 : quelles que soient les cartes rendues au mulligan, les cartes reçues
/// étaient toujours les mêmes.
///
/// On ne change pas la graine de la partie — la main du joueur et ses
/// corporations sont ce qu'elles sont, il les a sous les yeux. On rebat **ce qui
/// n'est pas encore sorti** : l'ordre du paquet, et le générateur que le moteur
/// consommera pendant l'essai. L'essai explore alors un avenir plausible au lieu
/// de l'avenir réel.
///
/// **La reproductibilité survit** parce que la graine est un nombre, pas une
/// horloge : à graine de partie et graine d'essais fixées, le même rebattage est
/// refait à l'identique, donc la même partie se rejoue au dernier chiffre près.
pub fn rebattre_l_avenir(g: &mut GameState, graine: u64) {
    rebattre_le_reste(g, graine, DejaVu::rien());
}

/// **CE QUE LE JOUEUR A DÉJÀ VU, ET QU'UN REBATTAGE NE DOIT PAS DÉFAIRE.**
///
/// Un rejeu d'essai repart du début de la manche et rejoue les réponses déjà
/// données avant d'essayer la sienne. Tout ce que ces réponses-là ont fait sortir
/// — cartes piochées, tuiles Océan retournées, corporations tirées — est **déjà
/// arrivé dans la vraie partie** : le joueur l'a sous les yeux. Le rebattre ferait
/// juger l'option sur une situation qui n'est pas la sienne, et ferait refuser des
/// réponses que le moteur ne pourrait plus honorer.
#[derive(Clone, Copy, Default)]
pub struct DejaVu {
    /// Cartes déjà piochées depuis le point de reprise (le haut du paquet).
    pub cartes: usize,
    /// Tuiles Océan déjà retournées à la décision qu'on essaie.
    pub oceans: usize,
    /// Corporations déjà tirées depuis le point de reprise (le haut du paquet).
    pub corpos: usize,
}

impl DejaVu {
    /// Rien n'est encore sorti : tout est rebattu.
    pub fn rien() -> DejaVu {
        DejaVu::default()
    }
}

/// **Le même rebattage, mais qui LAISSE INTACT LE HAUT DU PAQUET.**
///
/// Un rejeu d'essai repart du début de la manche et rejoue les réponses déjà
/// données avant d'essayer la sienne. Les cartes que ces réponses-là ont fait
/// piocher sont **déjà sorties dans la vraie partie** : le joueur les a en main,
/// il les voit. Les rebattre ferait juger l'option sur une main qui n'est pas la
/// sienne — et, pire, ferait refuser des réponses enregistrées que le moteur ne
/// pourrait plus honorer (mesuré : 3 à 20 % des essais rendus « injouables »
/// avant que ce garde-fou n'existe).
///
/// `garder` est donc le nombre de cartes du HAUT du paquet — la fin du vecteur,
/// puisque `flow::draw_card` fait `pop()` — que le rejeu va repiocher d'ici la
/// décision qu'on essaie. Elles ne bougent pas. Tout ce qui est en dessous, et
/// que personne n'a encore vu, est rebattu.
pub fn rebattre_le_reste(g: &mut GameState, graine: u64, vu: DejaVu) {
    let mut rng = StdRng::seed_from_u64(graine);
    // Fisher-Yates, du dernier vers le premier : le même brassage que celui du
    // moteur, sur la part du paquet dont plus personne ne connaît l'ordre.
    let n = g.deck.len().saturating_sub(vu.cartes);
    for i in (1..n).rev() {
        let j = rng.gen_range(0..=i);
        g.deck.swap(i, j);
    }
    // **Le paquet de projets n'est pas le seul à cacher l'avenir.** Les tuiles
    // Océan encore face cachée portent chacune un bonus (des cartes, des MC, des
    // plantes) que `setup_game` a tiré avec la graine de la partie : un essai qui
    // pose un océan encaissait donc le VRAI bonus. Les tuiles déjà retournées,
    // elles, sont publiques et ne bougent pas. Même raisonnement pour le paquet
    // des corporations.
    let d = (g.oceans_revealed as usize).max(vu.oceans);
    if d < g.oceans.len() {
        for i in (d + 1..g.oceans.len()).rev() {
            let j = rng.gen_range(d..=i);
            g.oceans.swap(i, j);
        }
    }
    let c = g.corp_deck.len().saturating_sub(vu.corpos);
    for i in (1..c).rev() {
        let j = rng.gen_range(0..=i);
        g.corp_deck.swap(i, j);
    }
    // Et le générateur de la partie d'essai : sans cela, les tirages que le
    // moteur fait pendant l'essai (remélange d'une pioche épuisée, effets de
    // cartes) resteraient ceux de la vraie partie.
    g.rng = StdRng::seed_from_u64(brasser(graine));
}

/// **L'état atteint par un rejeu, et le piège du §4.**
///
/// « Une option peut terminer la partie. L'état rendu porte alors `game_over` et les
/// scores sont définitifs ; c'est un cas normal, pas une erreur. » Il faut donc
/// distinguer les deux issues, exactement comme le pont le fait
/// (`wasm/src/lib.rs`, `let termine = pol.attente.is_none()`) :
///
/// - une décision attend → l'état à décrire est celui que `observe` a retenu juste
///   avant elle ;
/// - plus aucune décision → **l'état FINAL**, et surtout pas `vue`. `Rejeu::observe`
///   écrit `vue` à chaque observation, y compris aux points de décision que le moteur
///   finit par ne pas poser (liste d'options vide) : `vue` peut donc porter un état de
///   milieu de manche alors que la partie est finie. C'est le seul moment où l'entrée
///   `global_fin_de_partie` vaut +1 et où les scores sont définitifs — s'y tromper fait
///   juger la dernière décision de la partie sur un état périmé, et fait diverger le
///   joueur Rust du joueur JavaScript, qui, lui, reçoit l'état final du pont.
fn etat_atteint(rejeu: &mut Rejeu, g: GameState) -> Option<GameState> {
    if rejeu.erreur.is_some() {
        // Le moteur a refusé cette réponse : elle n'est pas jouable, et l'état
        // atteint après un repli n'a rien à voir avec elle.
        return None;
    }
    if rejeu.attente.is_none() {
        return Some(g); // la partie est finie : l'état final, scores compris
    }
    Some(rejeu.vue.take().unwrap_or(g))
}

/// **(V1) LES CARTES QUE L'ESSAI A FAIT SORTIR DE L'AVENIR RÉEL.**
///
/// Pendant la mise en place, le rejeu d'essai est obligé de repasser par
/// `setup_game` : c'est le seul point de reprise avant la première manche, et
/// `flow.rs` n'offre aucune prise pour rebattre le paquet en cours de route.
/// Les cartes que le mulligan fait repiocher sont donc, dans l'essai, celles
/// de la vraie partie — la voyance démontrée le 18-08.
///
/// On la retire au résultat : toute carte de la main du joueur qui n'était
/// pas dans la main qu'il avait SOUS LES YEUX au moment de décider est une
/// carte venue de l'avenir. Elle retourne au paquet, le paquet est rebattu, et
/// l'on en retire autant. Le joueur juge alors « si je rends ces cartes-là,
/// j'en recevrai d'autres, tirées du paquet » — ce qui est la vérité de la
/// règle, et non « je recevrai celles-ci ».
///
/// Les cartes GARDÉES, elles, ne bougent pas : elles sont à lui, il les
/// connaît, et c'est tout l'objet de sa décision.
pub fn ecarter_les_cartes_du_futur(
    main_connue: &[u16],
    g: &mut GameState,
    joueur: usize,
    graine: u64,
) {
    let mut connues: Vec<u16> = main_connue.to_vec();
    let mut gardees: Vec<u16> = Vec::with_capacity(g.players[joueur].hand.len());
    let mut du_futur: Vec<u16> = Vec::new();
    for &c in &g.players[joueur].hand {
        match connues.iter().position(|&x| x == c) {
            Some(i) => {
                connues.remove(i);
                gardees.push(c);
            }
            None => du_futur.push(c),
        }
    }
    if du_futur.is_empty() {
        return;
    }
    let combien = du_futur.len();
    g.deck.append(&mut du_futur);
    rebattre_l_avenir(g, brasser(graine ^ 0x0CA5_7E5D_0000_0002));
    for _ in 0..combien {
        match g.deck.pop() {
            Some(c) => gardees.push(c),
            None => break,
        }
    }
    g.players[joueur].hand = gardees;
}

/// Où reprendre pour rejouer une décision.
enum Reprise {
    /// Avant la première manche : on rejoue `setup_game` depuis la graine.
    MiseEnPlace,
    /// Le clone de l'état au début de la manche en cours.
    Manche(Box<GameState>),
}

pub struct Joueur<'a> {
    pub db: &'a CardsDb,
    pub desc: &'a Description,
    pub reseau: &'a mut Reseau,
    pub pile: &'a mut Pile,
    seed: u64,
    reprise: Reprise,
    /// Réponses déjà données depuis le point de reprise, au format du pont.
    reponses: Vec<Value>,
    /// Probabilité de prendre une option au hasard (§5).
    pub exploration: f64,
    /// Corriger le réseau à chaque carte Phase (faux quand on mesure la force).
    pub apprendre: bool,
    pub taux: f64,
    /// Description de l'état vu par `observe`, et le siège concerné.
    vue: Vec<f64>,
    vue_siege: usize,
    tampons: Tampons,
    /// Tampon d'évaluation, réutilisé (aucune allocation dans la boucle chaude).
    essai: Vec<f64>,
    /// Prédiction de victoire du siège 0, une par génération : elle sert à dire
    /// si le vainqueur était bien celui que le réseau donnait gagnant à
    /// mi-partie.
    pub predictions: Vec<f64>,
    pub generation_vue: u32,
    /// **Toutes les réponses de la partie, dans l'ordre**, au format du pont :
    /// c'est la partie elle-même, rejouable telle quelle par `pont.pas`.
    pub journal: Vec<Value>,
    /// Rang de décision à tracer : le joueur imprime alors la note de chaque
    /// option sur la sortie d'erreur. Sert à comparer les deux côtés quand ils
    /// se départagent différemment.
    pub tracer_rang: i64,
    /// Nombre d'essais faits (mesure de coût).
    pub essais: u64,
    /// **(2.11) Les essais dépensés à l'échange des cartes de départ**, les deux
    /// sièges confondus. Publié par `jouer` : c'est ce qui rend l'énumération
    /// complète vérifiable de l'extérieur (256 sous-ensembles par siège).
    pub essais_mulligan: u64,
    /// **(2.15) Les essais dépensés à la vente**, à part eux aussi : c'est le
    /// chiffre par lequel le surcoût de la vente s'attribue, au lieu de se
    /// deviner en comparant deux parties qui ne sont pas les mêmes.
    pub essais_vente: u64,
    /// **Les essais que le moteur a REFUSÉS** : le rejeu a rencontré une réponse
    /// qu'il ne pouvait plus honorer. Un essai refusé vaut « moins l'infini »,
    /// c'est-à-dire une option écartée sans avoir été jugée. C'est la mesure qui
    /// dit si le rebattage du paquet fait diverger le rejeu AVANT la décision
    /// qu'on essaie — voir `deck_vu`.
    pub essais_refuses: u64,
    /// **Les essais où le paquet n'a PAS pu être rebattu** parce que la pioche
    /// avait été rechargée depuis le début de la manche (voir `deck_vu`). Ces
    /// essais-là revoient l'avenir réel : c'est une voyance résiduelle, comptée et
    /// déclarée plutôt que cachée.
    pub rebattages_sautes: u64,
    /// **(2.15) Combien d'occasions de vente la partie a ouvertes**, et combien
    /// en étaient ouvertes au début de la manche en cours. La première numérote
    /// les entrées de vente ; la seconde dit au rejeu d'essai à quel numéro
    /// reprendre le compte (voir `Rejeu::occasions`).
    occasions_partie: u64,
    occasions_au_debut_de_manche: u64,
    /// **(V1) La graine des rejeux d'essai** (`--graine-essais`). Elle ne touche
    /// pas au déroulement de la vraie partie : elle ne sert qu'à rebattre ce que
    /// le joueur n'a pas le droit de connaître pendant qu'il essaie ses coups.
    pub graine_essais: u64,
    /// **(2.11) Énumérer les 2^n sous-ensembles au mulligan** plutôt que de
    /// construire la liste carte par carte. Allumé par défaut ; l'interrupteur
    /// existe pour mesurer le surcoût, et le résultat de la mesure est dans
    /// `result.md`.
    pub combinaisons_completes: bool,
    /// **(2.15) La vente est-elle essayée ?** Allumée par défaut. Le drapeau est
    /// celui que l'audit exige « pour couper l'option si le coût explose » — et
    /// il sert à mesurer ce coût au lieu de le supposer.
    pub vente: bool,
    /// **(2.15) Combien de fois le joueur a choisi de vendre**, et combien
    /// d'occasions lui ont été offertes (main non vide, phase dépensable).
    pub ventes_volontaires: u64,
    pub occasions_de_vente: u64,
    /// **(2.14) Combien de corrections sont tombées sur une décision de MISE EN
    /// PLACE**, et combien chaque siège en a reçu en tout. Les deux mesures que
    /// le §2.14 demande : la mise en place est-elle apprise, et les deux sièges
    /// sont-ils traités pareil.
    pub corrections_mise_en_place: u64,
    pub corrections_par_siege: [u64; 2],
    /// Pas d'avance consommés au total, et nombre de fois où le plafond du §4.1
    /// (soixante pas) a arrêté une avance. Les deux sont rapportés dans
    /// `result.md`, comme le §4.1 l'exige.
    pub pas_avance: u64,
    pub plafonds: u64,
    /// Somme des écarts « meilleure note − pire note » sur les décisions à
    /// plusieurs options, et leur nombre : la mesure qui dit si le réseau
    /// départage quelque chose (§2.2).
    pub somme_ecart: f64,
    pub compte_ecart: u64,
    /// **Le rythme du §2.2** : on prend une situation d'entraînement sur K.
    /// Corriger à chacune des 341 décisions d'une partie coûterait tout le temps
    /// de calcul et diluerait l'ancrage sur le résultat réel ; K = 8 donne une
    /// quarantaine de corrections par partie, l'ordre de grandeur de la
    /// référence. Réglable par `--rythme`.
    pub rythme: u64,
    /// **(2.14) Décisions vues depuis le début de la partie, UN COMPTEUR PAR
    /// SIÈGE, et démarré à un décalage tiré au sort.**
    ///
    /// Il était unique, commun aux deux sièges, et remis à zéro à chaque partie :
    /// la première correction tombait donc TOUJOURS à la huitième décision, alors
    /// que la mise en place n'en pose que 6, 7 ou 8. Seules 13 % des parties
    /// voyaient une décision de mise en place corrigée, et les deux sièges ne
    /// recevaient pas le même nombre de corrections.
    ///
    /// Le décalage est tiré d'un générateur semé par la GRAINE DE LA PARTIE —
    /// jamais l'horloge : deux exécutions identiques doivent rendre le même
    /// fichier de poids, octet pour octet.
    compteurs: [u64; 2],
    /// Description de l'état AU REPÈRE de l'option retenue : c'est elle, et elle
    /// seule, que le réseau apprend (§2.2 corrigé le 15-08).
    repere: Vec<f64>,
    /// **(V1) La main que le joueur a SOUS LES YEUX à la décision en cours**, et
    /// le siège à qui elle appartient. Écrite par `observe`, et seulement pendant
    /// la mise en place — c'est le seul moment où le rejeu d'essai est obligé de
    /// repasser par `setup_game`, donc par le paquet de la vraie partie. Toute
    /// carte que l'essai fait apparaître dans sa main et qui n'est pas là-dedans
    /// vient de l'avenir : on la lui retire (voir `ecarter_les_cartes_du_futur`).
    main_connue: Vec<u16>,
    main_connue_siege: usize,
    /// **(V1) Combien de cartes restaient au paquet à la décision en cours.**
    ///
    /// La différence avec le paquet du début de la manche donne le nombre de
    /// cartes déjà piochées depuis ce point de reprise : ce sont celles que le
    /// rejeu va repiocher avant d'atteindre la décision qu'on essaie, et qu'il ne
    /// faut donc surtout pas rebattre (voir `rebattre_le_reste`).
    ///
    /// **Le cas où la pioche a été rechargée** : quand elle s'épuise,
    /// `flow::draw_card` y reverse la défausse et le paquet GRANDIT — la
    /// différence n'a alors plus de sens, et l'ordre d'après rechargement dépend
    /// d'un tirage que le rejeu ne refera pas à l'identique. On préfère alors ne
    /// RIEN rebattre plutôt que de rendre le rejeu faux : cet essai-là revoit
    /// l'avenir réel, et on le compte (`rebattages_sautes`). Ne pas compter la
    /// défausse dans ce total est délibéré : elle grossit aussi des cartes jouées
    /// et vendues, qui ne sont pas des pioches.
    deck_vu: usize,
    /// Tuiles Océan retournées, et corporations restantes, à la décision en cours
    /// — mêmes raisons que `deck_vu`, pour les deux autres tas cachés.
    oceans_vus: usize,
    corpos_vues: usize,
    /// Chronomètres de mise au point : où passe le temps d'une partie.
    pub t_essais: f64,
    pub t_apprentissage: f64,
    pub passes: u64,

    // ---- (il-devine) le second réseau : celui qui devine la carte Phase
    /// **Le second réseau (§1).** Absent, rien de ce chantier ne s'allume : le
    /// joueur apprend et joue exactement comme au round 2.
    pub adversaire: Option<&'a mut ReseauPhases>,
    /// **L'interrupteur du §4** : le joueur se sert-il du second réseau pour
    /// répondre à la place de l'autre pendant l'avance du §4.1 ? **Éteint par
    /// défaut**, et éteint veut dire « première option », comme avant.
    ///
    /// Il est indépendant de l'apprentissage : le §7 fait entraîner le second
    /// réseau à l'étape 3 et ne s'en sert qu'à l'étape 6. Apprendre et s'en
    /// servir sont deux choses, et l'interrupteur ne commande que la seconde.
    pub devinette: bool,
    /// Taux d'apprentissage du second réseau (§2.2), réglable pour la mesure.
    pub taux_adversaire: f64,
    /// Facteur de contraste de la cible du second réseau (§2.2), idem.
    pub facteur_cible: f64,
    /// Les notes de la décision en cours, une par option, telles que `choisir`
    /// vient de les calculer. Vide quand la décision n'a pas été notée (une seule
    /// option, ou exploration). C'est la matière première de la cible du §2.2.
    notes: Vec<f64>,
    /// Tampon de description de la devinette, réutilisé.
    x_devinette: Vec<f64>,
    /// Combien de fois le second réseau a été corrigé, et combien de fois la
    /// correction a été sautée parce que la meilleure note valait zéro ou moins
    /// (§2.2, première précaution).
    pub corrections_adversaire: u64,
    pub sautees_adversaire: u64,
    /// **(il-devine, §7 étape 5) La mesure de la devinette.** Allumée, chaque
    /// `pick_phase` est l'occasion de demander au second réseau ce que l'AUTRE
    /// joueur aurait deviné, et de comparer à la phase réellement choisie. Elle ne
    /// corrige rien. Elle oblige à garder l'état vivant à chaque observation — un
    /// clone par décision —, ce qui la réserve au binaire `deviner`.
    pub mesurer_devinette: bool,
    etat_vu: Option<GameState>,
    pub devinettes: u64,
    pub devinettes_justes: u64,
    /// Somme des `1 / nombre de phases autorisées` sur les mêmes décisions : la
    /// part qu'obtiendrait une réponse tirée au sort, **mesurée et non postulée**
    /// (le point d'accroche n°2 l'exige : « pas la valeur théorique 0,25 »).
    pub somme_hasard: f64,
    /// **Le relevé de RÉFÉRENCE, éteint par défaut, et il doit le rester.**
    ///
    /// Il refait la même mesure du point de vue du joueur QUI CHOISIT — la tâche
    /// d'imitation sur laquelle le réseau s'entraîne, et non celle qu'il sert au
    /// §3. L'écart entre les deux dit exactement ce que coûte le changement de
    /// point de vue, et c'est le seul moyen de distinguer « la devinette est
    /// mauvaise » de « elle apprend bien mais ne se transpose pas » (§8).
    ///
    /// **Mais il décrit la situation du point de vue de celui qu'on prédit**, et
    /// c'est précisément la configuration que le §1 interdit au joueur et que le
    /// §7 déclare suspecte. Rien n'en sort — ni décision, ni apprentissage, ni
    /// ligne JSON — mais un contrôle caché qui relèverait les sièges décrits
    /// verrait les deux. Il est donc **éteint par défaut** et ne s'allume qu'à la
    /// demande explicite (`deviner --reference-point-de-vue on`), jamais dans le
    /// chemin qu'un contrôle emprunte.
    pub reference_point_de_vue: bool,
    pub devinettes_justes_soi: u64,
    /// **Combien de `pick_phase` adverses ont été rencontrés pendant les avances**
    /// — la mesure que le §8 exige avant toute conclusion sur l'utilité de la
    /// devinette (« croire qu'un `pick_phase` adverse est rencontré à chaque
    /// avance » est un des pièges annoncés).
    pub phases_rencontrees: u64,
}

impl<'a> Joueur<'a> {
    pub fn new(
        db: &'a CardsDb,
        desc: &'a Description,
        reseau: &'a mut Reseau,
        pile: &'a mut Pile,
        seed: u64,
    ) -> Joueur<'a> {
        let tampons = Tampons::new(desc);
        Joueur {
            db,
            desc,
            reseau,
            pile,
            seed,
            reprise: Reprise::MiseEnPlace,
            reponses: Vec::new(),
            exploration: 0.0,
            apprendre: false,
            taux: crate::reseau::TAUX,
            vue: Vec::new(),
            vue_siege: 0,
            tampons,
            essai: Vec::new(),
            predictions: Vec::new(),
            generation_vue: 0,
            journal: Vec::new(),
            tracer_rang: -1,
            essais: 0,
            essais_mulligan: 0,
            essais_vente: 0,
            essais_refuses: 0,
            rebattages_sautes: 0,
            occasions_partie: 0,
            occasions_au_debut_de_manche: 0,
            graine_essais: GRAINE_ESSAIS_DEFAUT,
            combinaisons_completes: true,
            vente: true,
            ventes_volontaires: 0,
            occasions_de_vente: 0,
            corrections_mise_en_place: 0,
            corrections_par_siege: [0; 2],
            main_connue: Vec::new(),
            main_connue_siege: 0,
            deck_vu: 0,
            oceans_vus: 0,
            corpos_vues: 0,
            pas_avance: 0,
            plafonds: 0,
            somme_ecart: 0.0,
            compte_ecart: 0,
            rythme: crate::reseau::RYTHME,
            compteurs: [0; 2],
            repere: Vec::new(),
            t_essais: 0.0,
            t_apprentissage: 0.0,
            passes: 0,
            adversaire: None,
            devinette: false,
            taux_adversaire: TAUX_ADVERSAIRE,
            facteur_cible: FACTEUR_CIBLE,
            notes: Vec::new(),
            x_devinette: Vec::new(),
            corrections_adversaire: 0,
            sautees_adversaire: 0,
            phases_rencontrees: 0,
            mesurer_devinette: false,
            etat_vu: None,
            devinettes: 0,
            devinettes_justes: 0,
            somme_hasard: 0.0,
            reference_point_de_vue: false,
            devinettes_justes_soi: 0,
        }
    }

    /// À appeler avant chaque `play_round` : le point de reprise devient le
    /// début de cette manche-ci.
    pub fn debut_manche(&mut self, game: &GameState) {
        self.reprise = Reprise::Manche(Box::new(game.clone()));
        self.reponses.clear();
        // (2.15) Le rejeu d'essai reprendra le compte des occasions de vente ici.
        self.occasions_au_debut_de_manche = self.occasions_partie;
    }

    /// À appeler au début d'une partie neuve.
    pub fn nouvelle_partie(&mut self, seed: u64) {
        self.seed = seed;
        self.reprise = Reprise::MiseEnPlace;
        self.reponses.clear();
        self.journal.clear();
        self.pile.vider();
        self.reseau.oublier();
        self.predictions.clear();
        self.generation_vue = 0;
        // **(2.14) LE DÉCALAGE DU COMPTEUR, TIRÉ DE LA GRAINE DE LA PARTIE.**
        // Remis à zéro, le compteur faisait tomber la première correction
        // toujours à la huitième décision ; la mise en place n'en pose que 6, 7
        // ou 8, donc elle n'était apprise que dans 13 % des parties. Le décalage
        // vient d'un générateur semé par la graine — jamais de l'horloge : deux
        // exécutions identiques doivent rendre le même fichier de poids.
        self.compteurs = [0; 2];
        if self.rythme > 0 {
            let mut r = StdRng::seed_from_u64(brasser(seed ^ 0x00DE_CA1A_6E00_0001));
            for c in self.compteurs.iter_mut() {
                *c = r.gen_range(0..self.rythme);
            }
        }
        self.main_connue.clear();
        self.occasions_partie = 0;
        self.occasions_au_debut_de_manche = 0;
    }

    /// **(V1) La graine du rejeu d'essai de la décision en cours.**
    ///
    /// Elle mêle trois nombres : la graine des essais (`--graine-essais`), la
    /// graine de la partie, et le RANG de la décision. Le rang y est pour que
    /// deux décisions successives n'explorent pas le même avenir imaginaire ; le
    /// numéro de l'option n'y est PAS, et c'est le point : toutes les options
    /// d'une même décision sont jugées sur le même tirage, sinon on comparerait
    /// des paquets et non des coups.
    fn graine_du_rejeu(&self) -> u64 {
        brasser(self.graine_essais)
            ^ brasser(self.seed ^ 0xA5A5_A5A5_A5A5_A5A5)
            ^ brasser(self.journal.len() as u64)
            // Le rang d'une occasion de vente déclinée : sans lui, une occasion
            // que le joueur laisse passer n'ajoute rien au journal et partagerait
            // son avenir imaginé avec la décision qui la suit.
            ^ brasser(self.occasions_partie.wrapping_mul(0x1000_0001))
    }


    /// **L'état atteint si l'on répondait `candidate`, AU REPÈRE DU §4.1.**
    ///
    /// Ce n'est pas l'état qui suit immédiatement l'option : c'est celui du
    /// **prochain point de décision de `joueur`**, ou la fin de la partie. La
    /// première version de ce contrat évaluait l'état immédiat, et le défaut est
    /// mesuré : « passer » mène à un état plus lointain — production encaissée,
    /// manche suivante entamée — que « poser une carte », qui mène à la décision
    /// d'après. Le réseau comparait alors un état de maintenant à un état de plus
    /// tard, et plus tard paraît toujours meilleur. Le joueur du round 1 avait
    /// appris à attendre : 1001 générations sans jamais terraformer.
    ///
    /// L'avance est faite **dans le rejeu lui-même** (`Rejeu::jusqu_a`) : tant
    /// que la décision atteinte est celle de l'autre joueur, on répond à sa place
    /// et le moteur continue. Un seul rejeu par option, au lieu d'un par pas.
    fn etat_apres(&mut self, joueur: usize, candidate: &Value) -> Option<GameState> {
        let mut reponses = self.reponses.clone();
        reponses.push(candidate.clone());
        self.rejouer_l_essai(joueur, reponses)
    }

    /// **L'état atteint si l'on ne répondait RIEN de plus** — le rejeu part des
    /// réponses déjà données et avance jusqu'au repère du §4.1.
    ///
    /// C'est l'option « ne rien vendre » du §2.15 : l'occasion de vente n'est pas
    /// une question du moteur mais une entrée qu'on glisse ou non dans la liste
    /// des réponses. Ne rien glisser, c'est laisser passer l'occasion — et cela
    /// s'évalue comme le reste, par un rejeu.
    fn etat_sans_reponse(&mut self, joueur: usize) -> Option<GameState> {
        let reponses = self.reponses.clone();
        self.rejouer_l_essai(joueur, reponses)
    }

    /// Le rejeu d'un essai, une fois la liste de réponses arrêtée.
    fn rejouer_l_essai(&mut self, joueur: usize, reponses: Vec<Value>) -> Option<GameState> {
        let t0 = std::time::Instant::now();
        // **(V1) L'AVENIR DE L'ESSAI N'EST PLUS CELUI DE LA PARTIE.** La graine
        // est la même pour toutes les options de cette décision-ci, et différente
        // d'une décision à l'autre. Calculée ici, avant que le rejeu n'emprunte
        // les tampons du joueur.
        let graine = self.graine_du_rejeu();
        let occasions = match self.reprise {
            Reprise::MiseEnPlace => 0,
            Reprise::Manche(_) => self.occasions_au_debut_de_manche,
        };
        let mut rejeu = Rejeu::jusqu_a(reponses, joueur).depuis_occasion(occasions);
        // **(il-devine, §3/§4) La devinette, si elle est allumée.** Elle ne
        // s'attache qu'ici, c'est-à-dire à l'avance vers le repère du §4.1 : c'est
        // le seul endroit où l'on répond à la place de l'adversaire. Éteinte, ou
        // sans second réseau, `rejeu.devinette` reste `None` et le rejeu est
        // exactement celui d'avant.
        if self.devinette {
            if let Some(reseau) = self.adversaire.as_deref_mut() {
                rejeu.devinette = Some(Devinette {
                    db: self.db,
                    desc: self.desc,
                    reseau,
                    tampons: &mut self.tampons,
                    x: &mut self.x_devinette,
                    // Le point de vue est celui du joueur qui décide — jamais
                    // celui de l'adversaire qu'on prédit (§1).
                    moi: joueur,
                });
            }
        }
        self.essais += 1;
        let mut r = match &self.reprise {
            Reprise::MiseEnPlace => {
                let mut g = setup_game(self.db, self.seed, &mut rejeu);
                // La mise en place a dû repasser par la vraie graine — c'est là
                // que sont la main et les corporations que le joueur a sous les
                // yeux. Le paquet est rebattu pour les MANCHES qui suivent ; pour
                // une décision dont le repère reste DANS la mise en place, l'état
                // rendu est celui que `Rejeu::observe` a cloné à l'intérieur de
                // `setup_game`, donc avant ce rebattage : là, c'est
                // `ecarter_les_cartes_du_futur` qui fait tout le travail.
                rebattre_l_avenir(&mut g, graine);
                while rejeu.attente.is_none() && !g.game_over && g.generation <= MAX_GENERATIONS {
                    play_round(&mut g, self.db, &mut rejeu);
                }
                etat_atteint(&mut rejeu, g)
            }
            Reprise::Manche(base) => {
                let mut g = (**base).clone();
                // Ce qui a déjà été pioché depuis le début de la manche est
                // connu du joueur : on ne rebat que ce qui est en dessous.
                let recharge = self.deck_vu > base.deck.len();
                if recharge {
                    self.rebattages_sautes += 1;
                }
                let vu = DejaVu {
                    // Pioche rechargée : on garde tout le paquet, donc on ne
                    // rebat rien — voir `deck_vu`.
                    cartes: if recharge {
                        base.deck.len()
                    } else {
                        base.deck.len() - self.deck_vu
                    },
                    oceans: self.oceans_vus,
                    corpos: base.corp_deck.len().saturating_sub(self.corpos_vues),
                };
                rebattre_le_reste(&mut g, graine, vu);
                while rejeu.attente.is_none() && !g.game_over && g.generation <= MAX_GENERATIONS {
                    play_round(&mut g, self.db, &mut rejeu);
                }
                etat_atteint(&mut rejeu, g)
            }
        };
        // Et l'on retire de l'état atteint les cartes que le mulligan a fait
        // repiocher dans le paquet de la vraie partie (voir la fonction).
        if matches!(self.reprise, Reprise::MiseEnPlace) && self.main_connue_siege == joueur {
            if let Some(g) = r.as_mut() {
                ecarter_les_cartes_du_futur(&self.main_connue, g, joueur, graine);
            }
        }
        if r.is_none() {
            self.essais_refuses += 1;
        }
        self.pas_avance += rejeu.pas_avance as u64;
        if rejeu.plafond_atteint {
            self.plafonds += 1;
        }
        self.phases_rencontrees += rejeu.phases_de_l_autre as u64;
        self.t_essais += t0.elapsed().as_secs_f64();
        r
    }

    /// **Le cœur du §4** : essayer chaque candidate, garder celle dont ma
    /// probabilité de victoire est la plus haute. Rend l'indice retenu dans
    /// `candidates`, et enregistre la réponse pour la suite du rejeu.
    fn choisir(&mut self, rng: &mut StdRng, joueur: usize, candidates: &[Value]) -> usize {
        if candidates.is_empty() {
            return 0; // le moteur n'offre rien : il n'y a rien à essayer
        }
        // (il-devine, §2.1) Les notes de cette décision-ci, pour la cible du
        // second réseau. Vides tant qu'aucune option n'a été notée — une seule
        // option, ou exploration : il n'y a alors pas d'« avis motivé sur chaque
        // phase », et le §2.1 ne veut corriger que là où il y en a un.
        self.notes.clear();
        let choix = if candidates.len() == 1 {
            0
        } else if self.exploration > 0.0 && rng.gen::<f64>() < self.exploration {
            // L'exploration du §5 : sans elle, deux joueurs identiques et
            // déterministes rejouent sans cesse des parties très ressemblantes.
            rng.gen_range(0..candidates.len())
        } else {
            let tracer = self.tracer_rang >= 0 && self.journal.len() as i64 == self.tracer_rang;
            let mut meilleur = 0usize;
            let mut meilleure_note = f64::NEG_INFINITY;
            // **L'écart d'évaluation entre les options d'une même décision.**
            // C'est la mesure qui dit si le réseau DÉPARTAGE quelque chose : au
            // round 1 elle valait 0,016, le niveau du bruit, parce que le réseau
            // n'était jamais entraîné sur les situations qu'il jugeait (§2.2).
            let mut note_min = f64::INFINITY;
            for (i, c) in candidates.iter().enumerate() {
                let note = match self.etat_apres(joueur, c) {
                    Some(g) => {
                        // Toujours MON point de vue : celui du joueur qui
                        // décidait, jamais celui à qui la main revient.
                        self.desc
                            .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                        let p = self.reseau.evaluer(&self.essai);
                        p[0]
                    }
                    None => f64::NEG_INFINITY,
                };
                if tracer {
                    eprintln!("rang {} option {i} : note {note:.17}", self.journal.len());
                }
                self.notes.push(note);
                if note.is_finite() && note < note_min {
                    note_min = note;
                }
                if note > meilleure_note + MARGE {
                    meilleure_note = note;
                    meilleur = i;
                }
            }
            if meilleure_note.is_finite() && note_min.is_finite() && candidates.len() > 1 {
                self.somme_ecart += meilleure_note - note_min;
                self.compte_ecart += 1;
            }
            meilleur
        };
        self.apprendre_au_repere(joueur, &candidates[choix]);
        self.reponses.push(candidates[choix].clone());
        self.journal.push(candidates[choix].clone());
        choix
    }

    /// **Un choix MULTIPLE.** Une décision multiple n'a pas une option par
    /// réponse mais une COMBINAISON, et le moteur n'accepte que les combinaisons
    /// de la taille exacte qu'il demande : une liste à moitié construite est
    /// refusée, pas évaluée. On ne peut donc pas l'assembler en ajoutant une
    /// carte à la fois — chaque candidat essayé doit être une réponse complète.
    ///
    /// Deux cas, et ils ne se traitent pas pareil :
    ///
    /// - **nombre libre** (le mulligan projets, de 0 à 8) : toute liste est une
    ///   réponse valable, y compris la liste vide. On part d'elle et on ajoute la
    ///   carte qui améliore le plus, tant qu'une addition améliore.
    /// - **nombre imposé** (garder k cartes, en défausser n) : on part des k
    ///   premières — une réponse complète, donc évaluable — et on essaie de
    ///   REMPLACER chaque carte retenue par chacune des autres, en gardant tout
    ///   remplacement qui améliore. Chaque carte est ainsi essayée à chaque
    ///   place. Deux tours suffisent en pratique et bornent le coût.
    ///
    /// Le JavaScript fait exactement la même chose, dans le même ordre : c'est ce
    /// que vérifie `web/webapp/verif/juge-meme-option.mjs`.
    fn choisir_liste(
        &mut self,
        rng: &mut StdRng,
        joueur: usize,
        n: usize,
        attendu: usize,
        libre: bool,
    ) -> Vec<usize> {
        let mut pris: Vec<usize> = Vec::new();
        if self.exploration > 0.0 && rng.gen::<f64>() < self.exploration {
            let mut reste: Vec<usize> = (0..n).collect();
            let combien = if libre { rng.gen_range(0..=n) } else { attendu.min(n) };
            for _ in 0..combien {
                let k = rng.gen_range(0..reste.len());
                pris.push(reste.remove(k));
            }
            self.apprendre_au_repere(joueur, &json!(pris));
            self.reponses.push(json!(pris));
            self.journal.push(json!(pris));
            return pris;
        }
        if libre && self.combinaisons_completes && n <= LARGEUR_ENUMERATION {
            // **(2.11) LES 256 SOUS-ENSEMBLES, ET PLUS 37.**
            //
            // La construction carte par carte ci-dessous part de la liste vide,
            // ajoute la carte qui améliore le plus, et s'arrête au premier tour où
            // aucune addition SEULE n'améliore : elle ne retire jamais une carte
            // déjà ajoutée et n'en ajoute jamais deux ensemble. Mesuré sur onze
            // mains réelles, elle reste bloquée sur une solution moins bonne 6 fois
            // sur 11 ; le fait qui survit dans tous les cas est qu'ajouter deux
            // cartes ensemble améliore là où aucune addition seule n'améliore.
            //
            // Les masques sont parcourus dans l'ordre croissant : le masque 0, «
            // ne rien rendre », vient donc en premier et l'emporte à égalité (la
            // marge du départage s'en charge). C'est le même ordre des deux côtés,
            // Rust et JavaScript.
            //
            // **La borne est là pour une raison mesurée** : le constat n° 7 de
            // l'audit a démontré que l'énumération complète est dix à seize fois
            // plus chère sur les défausses de fin de manche (jusqu'à 19 448
            // combinaisons). Elle ne vaut que pour l'échange des cartes de départ,
            // qui est la seule décision « à nombre libre » du jeu.
            let mut meilleure = f64::NEG_INFINITY;
            for masque in 0u32..(1u32 << n) {
                let mut cand: Vec<usize> = Vec::with_capacity(n);
                for i in 0..n {
                    if (masque >> i) & 1 == 1 {
                        cand.push(i);
                    }
                }
                let x = self.noter_liste(joueur, &cand);
                if x > meilleure + MARGE {
                    meilleure = x;
                    pris = cand;
                }
            }
        } else if libre {
            let mut note = self.noter_liste(joueur, &pris);
            while pris.len() < n {
                let mut meilleur: Option<(usize, f64)> = None;
                for i in 0..n {
                    if pris.contains(&i) {
                        continue;
                    }
                    pris.push(i);
                    let x = self.noter_liste(joueur, &pris);
                    pris.pop();
                    if meilleur.is_none() || x > meilleur.unwrap().1 + MARGE {
                        meilleur = Some((i, x));
                    }
                }
                match meilleur {
                    Some((i, x)) if x > note + MARGE => {
                        pris.push(i);
                        note = x;
                    }
                    _ => break,
                }
            }
        } else {
            pris = (0..attendu.min(n)).collect();
            let mut note = self.noter_liste(joueur, &pris);
            for _tour in 0..2 {
                let mut ameliore = false;
                for p in 0..pris.len() {
                    for c in 0..n {
                        if pris.contains(&c) {
                            continue;
                        }
                        let ancien = pris[p];
                        pris[p] = c;
                        let x = self.noter_liste(joueur, &pris);
                        if x > note + MARGE {
                            note = x;
                            ameliore = true;
                        } else {
                            pris[p] = ancien;
                        }
                    }
                }
                if !ameliore {
                    break;
                }
            }
        }
        self.apprendre_au_repere(joueur, &json!(pris));
        self.reponses.push(json!(pris));
        self.journal.push(json!(pris));
        pris
    }

    fn noter_liste(&mut self, joueur: usize, pris: &[usize]) -> f64 {
        let c = json!(pris);
        match self.etat_apres(joueur, &c) {
            Some(g) => {
                self.desc
                    .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                self.reseau.evaluer(&self.essai)[0]
            }
            None => f64::NEG_INFINITY,
        }
    }

    /// **LE POINT DE RENDEZ-VOUS DE L'APPRENTISSAGE — CORRIGÉ LE 15-08 (§2.2).**
    ///
    /// « Le réseau s'entraîne exactement sur les situations qu'il juge, et sur
    /// aucune autre. » La version du round 1 corrigeait au choix de la carte
    /// Phase — un instant où `phase_en_cours` vaut toujours 0, alors que le
    /// joueur évalue des situations en pleine phase. Mesuré : l'écart
    /// d'évaluation entre deux options d'une même décision tombait à 0,016, le
    /// niveau du bruit. Le réseau ne départageait plus rien.
    ///
    /// La situation apprise est donc **celle du repère du §4.1, pour l'option
    /// RETENUE** : très exactement l'état sur lequel le joueur vient de fonder
    /// son choix. Une prise sur `rythme` décisions (K = 8), parce que corriger
    /// aux 341 décisions d'une partie coûterait tout le temps de calcul et
    /// diluerait encore l'ancrage sur le résultat réel.
    ///
    /// Le coût est d'un rejeu de plus par prise — un essai sur K, moins de trois
    /// pour cent des essais — contre la complication d'un tampon retenu au vol
    /// pendant la boucle des candidates.
    fn apprendre_au_repere(&mut self, joueur: usize, retenue: &Value) {
        // (2.14) UN COMPTEUR PAR SIÈGE. Commun aux deux, il donnait à l'un plus de
        // corrections qu'à l'autre — et sa remise à zéro faisait tomber la
        // première correction toujours à la huitième décision.
        debug_assert!(joueur < 2, "siège hors bornes : {joueur}");
        let siege = joueur.min(1);
        self.compteurs[siege] += 1;
        if !self.apprendre || self.rythme == 0 || self.compteurs[siege] % self.rythme != 0 {
            return;
        }
        let Some(g) = self.etat_apres(joueur, retenue) else {
            return;
        };
        let mut repere = std::mem::take(&mut self.repere);
        self.desc
            .decrire(&g, self.db, joueur, &mut repere, &mut self.tampons);
        // §2.2 : la situation présente passe dans le réseau, ses deux
        // probabilités deviennent LA CIBLE ; puis on remonte la pile.
        // §2.1 : on empile juste avant de corriger.
        self.reseau.oublier();
        let p = self.reseau.evaluer(&repere);
        self.pile.empiler(&repere, joueur);
        let t0 = std::time::Instant::now();
        self.reseau.corriger(self.pile, joueur, p, self.taux);
        self.t_apprentissage += t0.elapsed().as_secs_f64();
        self.passes += 1;
        // (2.14) Les deux mesures du §2.14 : la mise en place est-elle apprise, et
        // les deux sièges reçoivent-ils un nombre comparable de corrections.
        self.corrections_par_siege[siege] += 1;
        if matches!(self.reprise, Reprise::MiseEnPlace) {
            self.corrections_mise_en_place += 1;
        }
        self.repere = repere;
    }

    /// **(il-devine, §7 étape 5) LA MESURE QUI DÉCIDE — et de quel point de vue
    /// elle est prise.**
    ///
    /// « Rejoue des parties et, à chaque `pick_phase` de chaque joueur, compare la
    /// phase que le second réseau donne comme la plus probable à celle réellement
    /// choisie. Le hasard donnerait 25 %. »
    ///
    /// **La description est prise du point de vue de l'AUTRE joueur** — celui qui
    /// ne choisit pas. C'est le seul point de vue qui mesure ce que le chantier
    /// livre : au §3, la devinette sert à prêter une intention à l'adversaire, et
    /// celui qui devine n'est jamais celui qui choisit. Le §7 le dit d'ailleurs
    /// par la bande, en déclarant **suspect** tout chiffre au-dessus de 60 % :
    /// « vérifie que tu ne décris pas la situation du point de vue de celui que tu
    /// prédis ». Décrire du point de vue du joueur qui choisit donnerait un
    /// chiffre plus flatteur et sans rapport avec l'usage — c'est la tâche
    /// d'imitation sur laquelle le réseau s'entraîne, pas celle qu'il sert.
    ///
    /// On relève quand même cette seconde valeur ([`Joueur::devinettes_justes_soi`]),
    /// parce que l'écart entre les deux dit exactement ce que coûte le changement
    /// de point de vue — et parce qu'un chiffre sans référence indépendante ne
    /// vaut rien.
    ///
    /// **Elle ne corrige rien**, et elle est prise AVANT la correction du §2.1 :
    /// mesurer un réseau qu'on vient de corriger vers cette cible-ci serait se
    /// mesurer soi-même.
    fn mesurer_la_devinette(&mut self, joueur: usize, autorisees: &[u8], choisie: u8) {
        if !self.mesurer_devinette || autorisees.is_empty() {
            return;
        }
        let Some(game) = self.etat_vu.take() else {
            return;
        };
        if self.adversaire.is_some() {
            let autre = 1 - joueur;
            let mut x = std::mem::take(&mut self.x_devinette);

            // Le point de vue de CELUI QUI DEVINE : l'autre joueur.
            self.desc
                .decrire(&game, self.db, autre, &mut x, &mut self.tampons);
            let devinee = {
                let reseau = self.adversaire.as_deref_mut().unwrap();
                reseau.oublier();
                let p = reseau.evaluer(&x);
                crate::reseau::phase_la_plus_probable(&p, autorisees)
            };

            // Et, SI ON LE DEMANDE EXPLICITEMENT, le point de vue du joueur qui
            // choisit — le relevé de référence du §8, jamais celui qu'on
            // rapporte, et jamais allumé dans le chemin d'un contrôle.
            if self.reference_point_de_vue {
                self.desc
                    .decrire(&game, self.db, joueur, &mut x, &mut self.tampons);
                let devinee_soi = {
                    let reseau = self.adversaire.as_deref_mut().unwrap();
                    reseau.oublier();
                    let p = reseau.evaluer(&x);
                    crate::reseau::phase_la_plus_probable(&p, autorisees)
                };
                if devinee_soi == choisie {
                    self.devinettes_justes_soi += 1;
                }
            }
            self.x_devinette = x;

            self.devinettes += 1;
            if devinee == choisie {
                self.devinettes_justes += 1;
            }
            self.somme_hasard += 1.0 / autorisees.len() as f64;
        }
        self.etat_vu = Some(game);
    }

    /// **(il-devine) LE SEUL POINT DE RENDEZ-VOUS DE L'APPRENTISSAGE DU SECOND
    /// RÉSEAU (§2.1).**
    ///
    /// « Le joueur passe déjà par `pick_phase` à chaque manche, pour lui-même. À
    /// cet instant précis, il vient d'évaluer les quatre phases autorisées et il
    /// tient une note par phase. C'est là, et seulement là, qu'on corrige le
    /// second réseau. Pas ailleurs : ce sont les seules situations où l'on dispose
    /// à la fois de la description et d'un avis motivé sur chaque phase. »
    ///
    /// **L'entrée** est `self.vue` : la description de l'état vivant que `observe`
    /// a écrite juste avant cette décision, **du point de vue du joueur qui
    /// choisit**. C'est mot pour mot ce que le §1 impose — les mêmes 1472 valeurs,
    /// calculées par la même fonction, aucune description nouvelle.
    ///
    /// **La cible** est celle du §2.2 : les notes des cinq phases passées à
    /// l'exponentielle normalisée avec le facteur 20, les phases non autorisées
    /// portant la note zéro. Si la meilleure note vaut zéro ou moins, on saute —
    /// la division n'a pas de sens.
    ///
    /// **La correction** est celle du §2.3 : `entrainer_une`, c'est-à-dire évaluer,
    /// accumuler l'erreur `sortie_i − cible_i`, la remonter dans les poids comme
    /// `corriger` le fait, appliquer. **Pas de trace d'éligibilité, pas de λ, pas
    /// de pile** : le premier réseau apprend d'un résultat qui n'arrivera qu'à la
    /// fin de la partie et doit garder la mémoire du chemin ; le second apprend
    /// d'une cible immédiatement disponible, on corrige sur-le-champ et on oublie.
    ///
    /// Rien de tout cela ne dépend de l'interrupteur du §4 : on apprend dès qu'un
    /// second réseau est là, qu'on s'en serve ou non.
    fn apprendre_la_phase(&mut self, joueur: usize, autorisees: &[u8]) {
        if !self.apprendre || self.adversaire.is_none() {
            return;
        }
        // **La garde qui interdit la fuite.** `self.vue` est écrite par
        // `Joueur::observe` pour le siège qu'on lui a passé, et l'invariant
        // « `observe` précède immédiatement `pick_phase` pour le même joueur »
        // tient aujourd'hui dans `flow.rs` — un fichier que je n'ai pas le droit
        // de toucher et qui peut bouger. S'il cassait, on entraînerait le second
        // réseau sur la description du siège D'EN FACE, c'est-à-dire sur sa main,
        // appariée à la cible du joueur qui choisit : une fuite silencieuse
        // qu'aucun contrôle existant ne verrait. On refuse plutôt que d'apprendre
        // n'importe quoi.
        if self.vue_siege != joueur {
            return;
        }
        if self.vue.is_empty() || self.notes.len() != autorisees.len() {
            // Décision non notée (une seule option, ou exploration) : pas d'avis
            // motivé, donc pas de cible. Le §2.1 n'en veut pas.
            return;
        }
        let mut notes = [0.0f64; PHASES];
        for (i, phase) in autorisees.iter().enumerate() {
            let k = (*phase as usize).wrapping_sub(1);
            if k < PHASES {
                notes[k] = self.notes[i];
            }
        }
        match cible_phases(&notes, self.facteur_cible) {
            Some(cible) => {
                let taux = self.taux_adversaire;
                let vue = &self.vue;
                if let Some(reseau) = self.adversaire.as_deref_mut() {
                    reseau.entrainer_une(vue, cible, taux);
                }
                self.corrections_adversaire += 1;
            }
            None => self.sautees_adversaire += 1,
        }
    }

    /// **Le relevé qui alimente `justes`**, une fois par génération et par
    /// joueur, à la carte Phase : la probabilité que le réseau accorde au siège 0
    /// sur l'état vivant. Il ne corrige rien — c'est une mesure, et elle reste
    /// prise au même endroit qu'au round 1 pour que les deux courbes se
    /// comparent.
    fn relever_prediction(&mut self) {
        if self.vue.is_empty() {
            return;
        }
        self.reseau.oublier();
        let p = self.reseau.evaluer(&self.vue);
        let p0 = if self.vue_siege == 0 { p[0] } else { p[1] };
        if self.predictions.len() < self.generation_vue as usize {
            self.predictions.resize(self.generation_vue as usize, 0.5);
        }
        if self.predictions.len() == self.generation_vue as usize {
            self.predictions.push(p0);
        }
    }
}

impl Policy for Joueur<'_> {
    /// L'état vivant, juste avant chaque décision : on en garde la description
    /// du point de vue du joueur qui va décider.
    fn observe(&mut self, game: &GameState, player: usize) {
        self.desc
            .decrire(game, self.db, player, &mut self.vue, &mut self.tampons);
        self.vue_siege = player;
        self.generation_vue = game.generation.saturating_sub(1);
        // (il-devine) La mesure du §7 étape 5 a besoin de l'état lui-même, pour
        // pouvoir le décrire une seconde fois du point de vue de l'autre joueur.
        // Un clone par décision : réservé au binaire `deviner`, éteint partout
        // ailleurs, y compris pendant l'entraînement.
        if self.mesurer_devinette {
            self.etat_vu = Some(game.clone());
        }
        // **(V1) LA MAIN QU'IL A SOUS LES YEUX**, et seulement pendant la mise en
        // place : c'est le seul moment où le rejeu d'essai doit repasser par
        // `setup_game`, donc par le paquet de la vraie partie. Tout ce que l'essai
        // ajoutera à cette main-là vient de l'avenir et lui sera retiré. Deux
        // douzaines d'octets recopiés six à huit fois par partie.
        if matches!(self.reprise, Reprise::MiseEnPlace) {
            self.main_connue.clear();
            self.main_connue.extend_from_slice(&game.players[player].hand);
            self.main_connue_siege = player;
        }
        // (V1) Et l'état des deux tas à cet instant : voir `deck_vu`.
        self.deck_vu = game.deck.len();
        self.oceans_vus = game.oceans_revealed as usize;
        self.corpos_vues = game.corp_deck.len();
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, _corps: &[u16]) -> bool {
        let c = [json!(0), json!(1)];
        self.choisir(rng, player, &c) == 1
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        // (2.11) Ce que l'échange des cartes de départ coûte en essais, à part :
        // c'est le compteur que `jouer` publie, et par lequel l'énumération
        // complète se vérifie de l'extérieur.
        let avant = self.essais;
        let r = self.choisir_liste(rng, player, hand.len(), 0, true);
        self.essais_mulligan += self.essais - avant;
        r
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        let c: Vec<Value> = (0..corps.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        self.relever_prediction();
        let c: Vec<Value> = (0..allowed.len()).map(|i| json!(i)).collect();
        let i = self.choisir(rng, player, &c);
        let phase = allowed[i];
        // (il-devine) La mesure du §7 étape 5 d'abord — elle doit interroger le
        // réseau tel qu'il est AVANT la correction de cette décision-ci.
        self.mesurer_la_devinette(player, allowed, phase);
        // (il-devine, §2.1) Puis on corrige le second réseau ICI, et nulle part
        // ailleurs — après `choisir`, qui vient de noter chaque phase autorisée,
        // et avec `self.vue`, que seul `Joueur::observe` écrit (le rejeu des
        // essais emploie sa propre politique et n'y touche pas).
        self.apprendre_la_phase(player, allowed);
        phase
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        // L'indice `affordable.len()` est « passer », comme chez le pont.
        let c: Vec<Value> = (0..=affordable.len()).map(|i| json!(i)).collect();
        let i = self.choisir(rng, player, &c);
        if i < affordable.len() {
            Some(affordable[i])
        } else {
            None
        }
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let c = [json!(0), json!(1), json!(2)];
        match self.choisir(rng, player, &c) {
            0 => ConstructionBonus::DrawCardBefore,
            1 => ConstructionBonus::DrawCard,
            _ => ConstructionBonus::SecondBuild,
        }
    }

    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        let c = [json!(0), json!(1)];
        self.choisir(rng, player, &c) == 0
    }

    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let c = [json!(0), json!(1)];
        if self.choisir(rng, player, &c) == 1 {
            ConstructionBonus::SecondBuild
        } else {
            ConstructionBonus::DrawCard
        }
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None;
        }
        let c: Vec<Value> = (0..=options.len()).map(|i| json!(i)).collect();
        let i = self.choisir(rng, player, &c);
        if i < options.len() {
            Some(i)
        } else {
            None
        }
    }

    /// **Le montant est TOUJOURS une décision, même quand il n'y a qu'un montant
    /// possible.** Le pont pose la question sans condition (`Harnais::action_amount`)
    /// et la page y répond : une réponse entre donc dans la liste des décisions.
    /// Court-circuiter le cas `max <= 0` ferait consommer une réponse de plus au
    /// JavaScript qu'au Rust, et tout le rejeu se décalerait d'un cran — le genre
    /// de divergence qui ne se voit que sur des milliers de décisions.
    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        let c: Vec<Value> = if max <= 0 {
            vec![json!(0)]
        } else {
            (0..=max).map(|i| json!(i)).collect()
        };
        self.choisir(rng, player, &c) as i64
    }

    /// **(2.15) LA VENTE EST UNE DÉCISION ESSAYÉE, COMME LES AUTRES.**
    ///
    /// Elle rendait une liste toujours vide : c'était le seul endroit du projet où
    /// une action légale était entièrement interdite à l'IA, alors que le livret
    /// l'autorise à tout moment — « vous pouvez défausser une carte Projet de
    /// votre main pour gagner 3 MC » (`docs/regles/livret-base.md:96`). Le réseau
    /// n'était donc jamais entraîné sur des situations où un joueur a converti sa
    /// main en argent, situations qu'un adversaire humain produit.
    ///
    /// Le commentaire d'avant affirmait que l'essayer coûterait « plusieurs fois
    /// le prix de tout le reste ». Ce chiffre n'avait jamais été mesuré ; il l'est
    /// maintenant, et il est dans `result.md`.
    ///
    /// **Ce n'est pas une question du moteur mais une ENTRÉE d'occasion** : elle
    /// se glisse, ou non, dans la liste des réponses. « Ne rien vendre » s'évalue
    /// donc par un rejeu sans entrée, « vendre la carte i » par un rejeu portant
    /// l'entrée. Les deux sont notées par le réseau au repère du §4.1, exactement
    /// comme les options d'une décision ordinaire — et rien n'est enregistré quand
    /// le joueur ne vend pas, si bien que le journal d'une partie sans vente est
    /// celui d'avant, à la ligne près.
    ///
    /// **Zéro ou une carte**, comme la prudence de l'audit l'impose : les
    /// sous-ensembles quelconques attendront d'avoir un coût mesuré.
    ///
    /// **CE QU'ELLE NE FAIT PAS, ET IL FAUT LE DIRE.** Elle est notée comme une
    /// décision ordinaire, mais elle n'est pas APPRISE comme une décision
    /// ordinaire : `apprendre_au_repere` n'est pas appelé ici, le compteur du
    /// rythme ne bouge pas, et ces situations-là ne deviennent jamais des cibles
    /// du réseau. C'est un écart assumé au §2.2 (« le réseau s'entraîne exactement
    /// sur les situations qu'il juge ») : le moteur ouvre de l'ordre de quatre
    /// cents occasions de vente par partie contre quatre cents décisions, et les
    /// verser au compteur noierait la mise en place que le point 2.14 vient tout
    /// juste de sortir du sous-échantillonnage. Déclaré dans `result.md`.
    fn vendre_librement(&mut self, _rng: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        // Le numéro de cette occasion, compté même quand la vente est coupée :
        // c'est un rang dans la partie, pas un compteur de ventes.
        let numero = self.occasions_partie;
        self.occasions_partie += 1;
        if !self.vente || main.is_empty() {
            return Vec::new();
        }
        self.occasions_de_vente += 1;
        let avant = self.essais;
        let mut meilleure = match self.etat_sans_reponse(joueur) {
            Some(g) => {
                self.desc
                    .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                self.reseau.evaluer(&self.essai)[0]
            }
            None => f64::NEG_INFINITY,
        };
        let mut choix: Option<usize> = None;
        for i in 0..main.len() {
            let c = json!({ "vendre": { "joueur": joueur, "occasion": numero, "cartes": [i] } });
            let note = match self.etat_apres(joueur, &c) {
                Some(g) => {
                    self.desc
                        .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                    self.reseau.evaluer(&self.essai)[0]
                }
                None => f64::NEG_INFINITY,
            };
            if note > meilleure + MARGE {
                meilleure = note;
                choix = Some(i);
            }
        }
        self.essais_vente += self.essais - avant;
        match choix {
            Some(i) => {
                let c = json!({ "vendre": { "joueur": joueur, "occasion": numero, "cartes": [i] } });
                self.reponses.push(c.clone());
                self.journal.push(c);
                self.ventes_volontaires += 1;
                vec![i]
            }
            None => Vec::new(),
        }
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        if n == 0 {
            return 0;
        }
        let c: Vec<Value> = (0..n).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let c: Vec<Value> = (0..candidates.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let c: Vec<Value> = (0..candidates.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        _card: u16,
        _tag_counts: &[u32],
    ) -> usize {
        let n = crate::cards::JOKER_TAG_CHOICES.len();
        let c: Vec<Value> = (0..n).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, drawn.len(), keep, false)
    }

    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        _revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        _filter: RevealFilter,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, candidates.len(), keep, false)
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, hand.len(), n, false)
    }
}

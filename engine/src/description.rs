//! **(le-juge-apprend) CE QUE LE RÉSEAU VOIT : la description d'une situation.**
//!
//! C'est la pièce du risque numéro un. Les poids sont APPRIS ici, en Rust, et
//! RELUS par `web/webapp/joueurs/description.js`. Si les deux côtés ne rangent
//! pas les mêmes nombres dans le même ordre, les poids ne veulent plus rien dire
//! une fois relus.
//!
//! Trois précautions, et elles sont toutes les trois structurelles :
//!
//! 1. **Une seule source d'ordre.** Le parcours est écrit UNE fois
//!    ([`Description::parcours`]) et sert aux deux usages : produire les valeurs
//!    (implémentation [`Valeurs`]) et produire la table des noms
//!    (implémentation [`Noms`]). Il est donc impossible qu'un nom se décale d'un
//!    rang par rapport à sa valeur — l'erreur classique de ce genre de code, et
//!    celle qu'aucun contrôle ne voit.
//! 2. **Le même parcours, écrit dans le même ordre, existe en JavaScript**, et
//!    le §7 impose que le fichier de poids porte les noms : au chargement, le
//!    JavaScript régénère les siens et refuse de jouer au premier écart.
//! 3. **Une seule fonction accède aux joueurs**, [`Description::joueur`], et
//!    elle prend le siège en paramètre. De l'adversaire, le parcours ne lit que
//!    ce que le §3.3 autorise : le NOMBRE de cartes en main, jamais leur
//!    identité. La main d'en face est publiée par le moteur (« mode bac à
//!    sable ») ; elle n'est lue nulle part ici, et c'est vérifiable en une
//!    lecture.
//!
//! **Convention de valeurs (§3.1)** : toute entrée vaut +1 ou −1, jamais 0/1,
//! jamais une quantité brute. Les quantités passent par un thermomètre : l'entrée
//! `i` vaut +1 si la quantité est STRICTEMENT supérieure au seuil `i`.
//!
//! **Les seuils viennent d'une mesure**, pas d'une préférence : 1000 parties au
//! hasard sur les graines 100000 à 100999 (`engine/src/bin/mesures.rs`), et un
//! seuil n'est retenu que si la fraction des situations qui le franchissent tombe
//! entre 2 % et 98 % (§3.5). Ils sont recopiés à l'identique dans le JavaScript.

use crate::cards::{CardsDb, TAG_COUNT, JOKER_TAG_CHOICES};
use crate::state::{GameState, PlayerState, AWARD_POOL, MILESTONE_POOL, NUM_PLAYERS};

// ---------------------------------------------------------------------------
// (2.8) LE RÉSUMÉ DU CONTENU D'UNE MAIN — point de calcul UNIQUE
// ---------------------------------------------------------------------------

/// **Le prix qu'on annonce pour la carte la moins chère d'une main VIDE.**
///
/// Répondre 0 dirait « j'ai une carte gratuite sous la main », le contraire de
/// la vérité. On répond au-dessus de tous les prix imprimés du jeu — « rien de
/// bon marché ici ». La main n'est vide qu'à la mise en place, avant la
/// distribution des huit projets.
pub const PRIX_MAIN_VIDE: i64 = 99;

/// **(2.8) Ce qu'une main CONTIENT**, résumé en six grandeurs.
///
/// La main était décrite par 246 drapeaux, un par carte existante, et par aucun
/// résumé de son contenu : une carte donnée est en main dans 4 % des situations,
/// si bien que la valeur générale d'une main n'était apprenable qu'à travers des
/// milliers de poids qu'il aurait fallu ajuster ensemble
/// (`docs/AUDIT_ENTRAINEMENT.md`, §2.8).
///
/// **Point de calcul unique** : la fiche ([`Description::parcours`]) et la mesure
/// des seuils (`engine/src/bin/mesures.rs`) appellent tous deux cette
/// fonction-ci. Sans cela, un seuil pourrait être mesuré sur une grandeur qui
/// n'est pas celle que la fiche publie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeMain {
    /// Nombre de cartes de la main portant chaque badge, dans l'ordre de
    /// `JOKER_TAG_CHOICES`. Un badge JOKER non déterminé (`Tag::Dynamic`) ne
    /// compte nulle part — c'est déjà la règle de `PlayerState::tag_counts`.
    pub badges: [i64; TAG_COUNT],
    /// Nombre de cartes de la main par couleur, dans l'ordre de `Color::index`
    /// (verte, bleue, rouge).
    pub couleurs: [i64; 3],
    /// Somme des points de victoire IMPRIMÉS des cartes de la main. Les points
    /// dynamiques n'en sont pas : ils dépendent d'un état que la carte n'a pas
    /// encore, puisqu'elle n'est pas posée.
    pub pv_imprimes: i64,
    /// Somme des prix imprimés.
    pub prix_total: i64,
    /// Prix de la carte la moins chère, ou [`PRIX_MAIN_VIDE`] si la main est vide.
    pub prix_min: i64,
}

/// Le résumé d'une main de projets. `main` est une liste d'identifiants de
/// `CardsDb::projects`.
pub fn resume_main(db: &CardsDb, main: &[u16]) -> ResumeMain {
    let mut r = ResumeMain {
        badges: [0; TAG_COUNT],
        couleurs: [0; 3],
        pv_imprimes: 0,
        prix_total: 0,
        prix_min: PRIX_MAIN_VIDE,
    };
    for id in main.iter() {
        let c = &db.projects[*id as usize];
        for t in c.tags.iter() {
            if let Some(i) = t.index() {
                r.badges[i] += 1;
            }
        }
        r.couleurs[c.color.index()] += 1;
        r.pv_imprimes += c.vp;
        r.prix_total += c.price;
        if c.price < r.prix_min {
            r.prix_min = c.price;
        }
    }
    r
}

/// **(2.9) Les six écarts décisifs entre le joueur qui regarde et l'autre**,
/// dans l'ordre où la fiche les publie : score acquis, niveau de terraformation,
/// cartes posées, argent, production d'argent, forêts.
///
/// `acquis` porte les scores acquis des deux joueurs, dans l'ordre des sièges —
/// ils viennent de `flow::score_breakdown`, le point de calcul unique du score,
/// et ne sont donc pas recalculés ici.
///
/// Toutes les grandeurs de la fiche étaient publiées en valeur ABSOLUE, joueur
/// par joueur ; aucune entrée n'exprimait la différence entre les deux, alors que
/// ce que le réseau doit produire est une probabilité de victoire, c'est-à-dire
/// une fonction de la seule différence (`docs/AUDIT_ENTRAINEMENT.md`, §2.9).
pub fn ecarts(moi: &PlayerState, adv: &PlayerState, acquis_moi: i64, acquis_adv: i64) -> [i64; 6] {
    [
        acquis_moi - acquis_adv,
        moi.tr - adv.tr,
        moi.played.len() as i64 - adv.played.len() as i64,
        moi.mc - adv.mc,
        moi.mc_prod - adv.mc_prod,
        moi.forests - adv.forests,
    ]
}

/// Les noms des six écarts, dans le même ordre — la source unique de ces
/// libellés, employée par la fiche comme par la mesure des seuils.
pub const NOMS_ECARTS: [&str; 6] = [
    "score_acquis",
    "nt",
    "posees",
    "mc",
    "prod_mc",
    "forets",
];

/// **(2.10) Les ressources posées sur les cartes en jeu d'un joueur**, tous
/// types confondus. Elles ne figuraient nulle part dans la fiche, et c'est la
/// seule des sept récompenses qui n'était déductible d'aucune entrée publiée
/// (`docs/AUDIT_ENTRAINEMENT.md`, §2.10).
pub fn ressources_posees(pl: &PlayerState) -> i64 {
    pl.card_resources.values().map(|&n| n as i64).sum()
}

// ---------------------------------------------------------------------------
// Les seuils, relevés le 15-08 (mesures --parties 1000 --graine-debut 100000)
// ---------------------------------------------------------------------------

pub const S_GENERATION: &[i64] = &[1, 8, 15, 22, 28, 34, 40, 50];
pub const S_TEMPERATURE: &[i64] = &[0, 1, 2, 5, 8, 13, 18];
pub const S_OXYGENE: &[i64] = &[0, 1, 2, 5, 7, 11, 13];
pub const S_OCEANS: &[i64] = &[0, 1, 3, 4, 6, 8];
pub const S_PIOCHE: &[i64] = &[14, 64, 95, 122, 147, 171, 195, 222];
pub const S_DEFAUSSE: &[i64] = &[6, 20, 38, 58, 79, 101, 127, 169];

pub const S_MC: &[i64] = &[0, 3, 8, 14, 22, 33, 53, 144];
pub const S_CHALEUR: &[i64] = &[0, 1, 3, 5, 8, 14, 49];
pub const S_PLANTES: &[i64] = &[0, 1, 2, 4, 5, 8, 18];
pub const S_PROD_MC: &[i64] = &[0, 1, 2, 4, 6, 11];
pub const S_PROD_CHALEUR: &[i64] = &[0, 1, 2, 4, 5, 8, 14];
pub const S_PROD_PLANTES: &[i64] = &[0, 1, 2, 3, 5];
pub const S_PROD_CARTES: &[i64] = &[0, 1, 2];
pub const S_NT: &[i64] = &[5, 6, 8, 10, 13, 17, 22, 30];
pub const S_FORETS: &[i64] = &[0, 1, 2, 3, 5, 9];
// (2.9) **L'ÉCHELLE DE SCORE, DÉSATURÉE.** Elle allait de 5 à 51 en huit
// paliers ; au-dessus de 51 tous les scores tombaient dans la même case, et
// entre 36 et 51 il y a quinze points de large. Deux joueurs séparés de 8 points
// ou plus y étaient donc décrits à l'identique dans 4,8 % des situations
// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.9). Deux scores partagent une case si et
// seulement s'ils tombent entre les deux mêmes seuils : AUCUN intervalle entre
// deux paliers consécutifs ne doit donc atteindre 8. Les paliers viennent de la
// mesure à douze quantiles (`mesures --parties 200 --graine-debut 200001
// --poids data/poids/apprenti-1M.txt --seuils 12`, relevé
// `outputs/work/seuils-200.txt` : 5, 6, 8, 11, 17, 25, 34, 43, 54, 65, 83) ; les
// valeurs intercalées 29, 38, 48, 59, 71 et 77 comblent les intervalles trop
// larges. L'escalier se prolonge ensuite de 8 en 8 jusqu'à 147, parce que
// s'arrêter à 83 laissait encore 15,1 pour mille de situations indiscernables :
// tous les intervalles valant 8 ou moins, deux scores séparés de 8 ne peuvent
// plus se confondre QUE dans la case ouverte du haut, et il fallait donc monter
// ce dernier palier. Ces paliers-là sont forcément dans la queue de la
// distribution : le quantile 98 % du § 3.5 vaut 76 sur l'IA livrée, et le plus
// haut score relevé 153. L'escalier s'arrête à 147 parce que chaque palier
// au-dessus de ce que les parties atteignent est une case morte de plus — deux,
// une par joueur — pour un budget de trente ; le relevé en compte vingt en tout,
// dont quatre du haut de l'échelle. L'arbitrage entre ce critère et la bande du
// § 3.5, qui se contredisent, est déclaré dans `outputs/result.md`.
pub const S_SCORE: &[i64] = &[
    5, 6, 8, 11, 17, 25, 29, 34, 38, 43, 48, 54, 59, 65, 71, 77, 83, 91, 99, 107, 115, 123, 131,
    139, 147,
];
pub const S_MAIN: &[i64] = &[6, 8, 9, 10, 12];
pub const S_POSEES: &[i64] = &[0, 3, 6, 8, 11, 15, 18, 26];
pub const S_ACIER: &[i64] = &[0, 1, 2, 3];
pub const S_TITANE: &[i64] = &[0, 1, 3];
pub const S_REPERES: &[i64] = &[0, 1, 2];
pub const S_PAYABLE: &[i64] = &[0, 3, 6, 8, 9, 10, 11];
pub const S_PAYABLE_VERTE: &[i64] = &[0, 1, 2, 3, 4, 5, 7];
pub const S_PAYABLE_BLEUE: &[i64] = &[0, 1, 2, 3, 5];
pub const S_PAYABLE_ROUGE: &[i64] = &[0, 1, 2, 3];

/// Un jeu de seuils par badge, dans l'ordre de `JOKER_TAG_CHOICES`.
pub const S_BADGES: [&[i64]; 10] = [
    &[0, 1, 2, 4, 5, 7, 10], // BUILDING
    &[0, 1, 2, 3, 4, 6],     // SPACE
    &[0, 1, 2, 4, 6],        // SCIENCE
    &[0, 1, 2, 3],           // PLANT
    &[0, 1, 2, 3],           // MICROBE
    &[0, 1],                 // ANIMAL
    &[0, 1, 2, 3, 4],        // EARTH
    &[0, 1, 2],              // JUPITER
    &[0, 1, 2, 4],           // ENERGY
    &[0, 1, 2, 3, 5],        // EVENT
];

// ---------------------------------------------------------------------------
// Les seuils des séries neuves du lot 3, relevés le 19-08 :
//
//     ./engine/target/release/mesures --parties 200 --graine-debut 200001 \
//         --poids data/poids/apprenti-1M.txt --boites base,decouverte --seuils 8
//
// 200 parties jouées par une IA entraînée, 152 752 observations, relevé complet
// dans `workspaces/la-fiche-que-l-ia-regarde/outputs/work/seuils-200-k8.txt`.
// Huit quantiles, comme les thermomètres déjà en place (§ 3.5) : la mesure ne
// retient un palier que si la fraction des situations qui le franchissent tombe
// entre 2 % et 98 %, ce qui explique qu'une série en porte moins que huit.
// ---------------------------------------------------------------------------

/// (2.9) Les seuils des six écarts, dans l'ordre de [`NOMS_ECARTS`]. Un écart
/// est signé : les paliers le sont aussi.
pub const S_ECARTS: [&[i64]; 6] = [
    &[-41, -15, -4, -1, 0, 3, 14, 40],     // score_acquis
    &[-22, -7, -2, -1, 0, 1, 6, 21],       // nt
    &[-13, -6, -3, -1, 0, 2, 5, 12],       // posees
    &[-121, -26, -12, -4, 3, 11, 25, 120], // mc
    &[-9, -3, -2, -1, 0, 1, 2, 8],         // prod_mc
    &[-8, -2, -1, 0, 1, 7],                // forets
];

/// (2.8) Les seuils du nombre de cartes de MA main portant chaque badge, dans
/// l'ordre de `JOKER_TAG_CHOICES` — le même que [`S_BADGES`], qui compte les
/// badges POSÉS.
pub const S_MAIN_BADGES: [&[i64]; 10] = [
    &[0, 1, 2, 3, 4, 5], // BUILDING
    &[0, 1, 2, 3, 5, 6], // SPACE
    &[0, 1, 2, 3, 4],    // SCIENCE
    &[0, 1, 2],          // PLANT
    &[0, 1],             // MICROBE
    &[0, 1],             // ANIMAL
    &[0, 1, 2],          // EARTH
    &[0, 1, 2, 3],       // JUPITER
    &[0, 1, 2],          // ENERGY
    &[0, 1, 2, 3, 4],    // EVENT
];

/// (2.8) Les seuils du nombre de cartes de MA main par couleur, dans l'ordre de
/// `Color::index` : verte, bleue, rouge.
pub const S_MAIN_COULEURS: [&[i64]; 3] = [
    &[0, 1, 2, 3, 4, 5, 7], // verte
    &[0, 1, 2, 3, 4],       // bleue
    &[0, 1, 2, 3, 4],       // rouge
];

/// (2.8) Points de victoire imprimés cumulés de MA main.
pub const S_MAIN_PV: &[i64] = &[0, 1, 2, 3, 4, 6, 9];
/// (2.8) Prix imprimés cumulés de MA main.
pub const S_MAIN_PRIX_TOTAL: &[i64] = &[14, 72, 101, 119, 136, 154, 177, 227];
/// (2.8) Prix de la carte la moins chère de MA main ([`PRIX_MAIN_VIDE`] si elle
/// est vide).
pub const S_MAIN_PRIX_MIN: &[i64] = &[0, 2, 3, 4, 5, 7, 9, 16];
/// (2.10) Ressources posées sur les cartes en jeu d'un joueur, tous types
/// confondus.
pub const S_RESSOURCES_POSEES: &[i64] = &[0, 3, 13];

/// Les dix améliorations de carte Phase, dans l'ordre imprimé
/// (`PlayerState::phase_upgrade_labels`).
pub const AMELIORATIONS: [&str; 10] = ["1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B", "5A", "5B"];

// ---------------------------------------------------------------------------
// Le collecteur : deux façons de recevoir le même parcours
// ---------------------------------------------------------------------------

/// Ce qui reçoit le parcours. Le nom d'une entrée est la concaténation
/// `a + n + b + c` (le nombre est omis s'il vaut −1) : passer le nom en morceaux
/// évite toute allocation quand on ne collecte que les valeurs, c'est-à-dire des
/// centaines de millions de fois pendant un entraînement.
pub trait Sortie {
    fn drapeau(&mut self, a: &str, n: i64, b: &str, c: &str, v: bool);
    fn thermo(&mut self, a: &str, n: i64, b: &str, c: &str, q: i64, seuils: &[i64]);
}

/// Collecteur de VALEURS : +1 / −1, dans l'ordre du parcours.
pub struct Valeurs<'a> {
    pub out: &'a mut Vec<f64>,
}

impl Sortie for Valeurs<'_> {
    #[inline(always)]
    fn drapeau(&mut self, _a: &str, _n: i64, _b: &str, _c: &str, v: bool) {
        self.out.push(if v { 1.0 } else { -1.0 });
    }
    #[inline(always)]
    fn thermo(&mut self, _a: &str, _n: i64, _b: &str, _c: &str, q: i64, seuils: &[i64]) {
        for s in seuils {
            self.out.push(if q > *s { 1.0 } else { -1.0 });
        }
    }
}

/// Collecteur de NOMS : la table des entrées du §3.4, celle que le fichier de
/// poids porte et que le JavaScript régénère pour la comparer (§7).
pub struct Noms {
    pub out: Vec<String>,
}

impl Noms {
    fn nom(a: &str, n: i64, b: &str, c: &str) -> String {
        if n < 0 {
            format!("{a}{b}{c}")
        } else {
            format!("{a}{n}{b}{c}")
        }
    }
}

impl Sortie for Noms {
    fn drapeau(&mut self, a: &str, n: i64, b: &str, c: &str, _v: bool) {
        self.out.push(Noms::nom(a, n, b, c));
    }
    fn thermo(&mut self, a: &str, n: i64, b: &str, c: &str, _q: i64, seuils: &[i64]) {
        for s in seuils {
            self.out.push(format!("{}>{s}", Noms::nom(a, n, b, c)));
        }
    }
}

// ---------------------------------------------------------------------------
// La description
// ---------------------------------------------------------------------------

/// Les tables qui ne dépendent que de la base de cartes : quelles cartes ont un
/// rang dans le vecteur, et lequel.
pub struct Description {
    /// **(2.12) Identifiants des cartes projets qui sont RÉELLEMENT DANS LA
    /// PIOCHE de la composition demandée** — le drapeau `in_deck` de
    /// `CardsDb::load_boites`, et non plus l'appartenance à une boîte nommée.
    ///
    /// **La contrepartie est assumée, et la voici noir sur blanc.** Ce champ se
    /// lisait naguère `c.boite.is_some()` : la table était alors la même pour
    /// toutes les compositions, et le commentaire d'alors s'en prévalait. Mais
    /// onze cartes portent un nom de boîte sans jamais être distribuées
    /// (`in_deck_v1` faux : cartes de démarrage, doublons de test), et leurs
    /// quarante-quatre entrées valaient −1 dans toutes les situations de toutes
    /// les parties — quarante-quatre poids appris sur du vide
    /// (`docs/AUDIT_ENTRAINEMENT.md`, § 2.12).
    ///
    /// **Donc : la table dépend désormais des boîtes.** Deux compositions
    /// différentes donnent deux fiches de tailles différentes et deux tables de
    /// noms différentes. Ce n'est pas un piège silencieux : le § 3.7 impose que
    /// le fichier de poids porte la table des noms, et `reseau::Reseau::lire`
    /// refuse au premier nom qui ne correspond pas. Un fichier de poids appris
    /// sur `base,decouverte` est donc REFUSÉ, et non pas mal interprété, si on
    /// le présente à une partie `base`. Le rang d'une entrée désigne toujours la
    /// même chose (§ 3.3) — à composition fixée, ce qui est la seule portée que
    /// le verrou des noms garantit.
    pub projets: Vec<u16>,
    /// `id de projet -> rang dans `projets``, ou `usize::MAX`.
    rang_projet: Vec<usize>,
    /// Noms des corporations, triés : côté JavaScript l'état ne publie que le
    /// NOM de la corporation (`observe.rs`), jamais son identifiant.
    pub corporations: Vec<String>,
    /// Noms imprimés des récompenses, précalculés : `format!("{kind:?}")` dans
    /// la boucle chaude allouerait sept chaînes par évaluation.
    noms_awards: Vec<String>,
    /// Nombre d'entrées du vecteur.
    pub taille: usize,
}

/// Les tampons de travail d'une description, réutilisés d'une évaluation à
/// l'autre : sans eux, chaque évaluation allouerait quatre vecteurs de la taille
/// du paquet — et il y en a des centaines de millions dans un entraînement.
pub struct Tampons {
    dans_main: Vec<bool>,
    pose_moi: Vec<bool>,
    pose_adv: Vec<bool>,
    defausse: Vec<bool>,
}

impl Tampons {
    pub fn new(d: &Description) -> Tampons {
        let n = d.projets.len();
        Tampons {
            dans_main: vec![false; n],
            pose_moi: vec![false; n],
            pose_adv: vec![false; n],
            defausse: vec![false; n],
        }
    }
}

impl Description {
    pub fn new(db: &CardsDb) -> Description {
        let mut projets: Vec<u16> = Vec::new();
        for (i, c) in db.projects.iter().enumerate() {
            // (2.12) `in_deck`, pas `boite.is_some()` : voir le commentaire du
            // champ `projets`. Le contrôle 06 le vérifie composition par
            // composition — quatre cases par projet distribué, ni plus ni moins.
            if c.in_deck {
                projets.push(i as u16);
            }
        }
        let mut rang_projet = vec![usize::MAX; db.projects.len()];
        for (rang, id) in projets.iter().enumerate() {
            rang_projet[*id as usize] = rang;
        }
        let mut corporations: Vec<String> = db.corporations.iter().map(|c| c.name.clone()).collect();
        corporations.sort();
        corporations.dedup();
        let noms_awards = AWARD_POOL.iter().map(|k| format!("{k:?}")).collect();
        let mut d = Description {
            projets,
            rang_projet,
            corporations,
            noms_awards,
            taille: 0,
        };
        d.taille = d.noms_avec(db).len();
        d
    }

    /// La table des noms. Elle ne dépend pas de l'état : le parcours est donc
    /// fait sur une partie neuve, dont seules les valeurs — jetées — changent.
    pub fn noms_avec(&self, db: &CardsDb) -> Vec<String> {
        let mut policy = crate::policy::RandomPolicy;
        let game = crate::flow::setup_game(db, 0, &mut policy);
        let mut n = Noms { out: Vec::new() };
        let mut t = Tampons {
            dans_main: vec![false; self.projets.len()],
            pose_moi: vec![false; self.projets.len()],
            pose_adv: vec![false; self.projets.len()],
            defausse: vec![false; self.projets.len()],
        };
        self.parcours(&game, db, 0, &mut n, &mut t);
        n.out
    }

    /// Le vecteur de description de `game`, du point de vue du siège `siege`.
    /// `out` est vidé puis rempli : l'appelant garde son tampon d'une évaluation
    /// à l'autre (aucune allocation dans la boucle chaude).
    pub fn decrire(
        &self,
        game: &GameState,
        db: &CardsDb,
        siege: usize,
        out: &mut Vec<f64>,
        t: &mut Tampons,
    ) {
        out.clear();
        let mut v = Valeurs { out };
        self.parcours(game, db, siege, &mut v, t);
    }

    /// **La seule fonction qui accède aux joueurs**, et elle prend le siège en
    /// paramètre. Rendre la triche impossible par construction plutôt que par
    /// discipline (§3.3).
    #[inline(always)]
    fn joueur<'a>(&self, game: &'a GameState, siege: usize, moi: bool) -> &'a crate::state::PlayerState {
        let p = if moi { siege } else { (siege + 1) % NUM_PLAYERS };
        &game.players[p]
    }

    /// **LE PARCOURS — la source unique de l'ordre des entrées.**
    ///
    /// Il est écrit une fois et sert aux valeurs comme aux noms ; le JavaScript
    /// en tient la copie conforme, dans le même ordre.
    pub fn parcours<S: Sortie>(
        &self,
        game: &GameState,
        db: &CardsDb,
        siege: usize,
        s: &mut S,
        t: &mut Tampons,
    ) {
        // Les deux joueurs, liés une fois pour tout le parcours : la section a
        // en a besoin depuis que le classement des récompenses (2.10) y figure.
        // `joueur` reste la seule fonction qui accède aux joueurs, et elle prend
        // toujours le siège en paramètre (§ 3.3).
        let moi = self.joueur(game, siege, true);
        let adv = self.joueur(game, siege, false);

        // ------------------------------------------------------- a. le global
        s.drapeau("global_", -1, "", "fin_de_partie", game.game_over);
        s.thermo("global_", -1, "", "generation", game.generation as i64, S_GENERATION);
        s.thermo("global_", -1, "", "temperature", game.temperature as i64, S_TEMPERATURE);
        s.thermo("global_", -1, "", "oxygene", game.oxygen as i64, S_OXYGENE);
        s.thermo("global_", -1, "", "oceans", game.oceans_revealed as i64, S_OCEANS);
        s.thermo("global_", -1, "", "pioche", game.deck.len() as i64, S_PIOCHE);
        s.thermo("global_", -1, "", "defausse", game.discard.len() as i64, S_DEFAUSSE);

        // Un rang par TYPE de repère, jamais par position : trois sont tirés au
        // hasard parmi onze à chaque partie (§3.3, le piège annoncé).
        for kind in MILESTONE_POOL.iter() {
            let nom = kind.name();
            let slot = game.milestones.iter().find(|m| m.kind == *kind);
            s.drapeau("repere_", -1, nom, "_present", slot.is_some());
            s.drapeau(
                "repere_",
                -1,
                nom,
                "_atteint",
                slot.map_or(false, |m| m.achieved_by.iter().any(|x| *x)),
            );
            s.drapeau(
                "repere_",
                -1,
                nom,
                "_par_moi",
                slot.map_or(false, |m| m.achieved_by[siege]),
            );
        }
        // (2.10) **QUI MÈNE SUR CHAQUE RÉCOMPENSE.** `_presente` disait qu'une
        // tuile est en jeu, et rien de plus : le réseau devait déduire le
        // classement des grandeurs publiées ailleurs, ce qui n'était possible
        // que pour cinq des sept (`docs/AUDIT_ENTRAINEMENT.md`, § 2.10) — la
        // production de chaleur et les ressources posées ne figuraient nulle
        // part, et deux tuiles restaient donc indéchiffrables.
        //
        // Le barème n'est PAS recopié ici : `flow::award_value` est le point de
        // calcul unique du moteur, celui-là même que `flow::award_points_split`
        // emploie pour distribuer les points en fin de partie. Une seule règle,
        // deux lecteurs.
        //
        // Trois drapeaux mutuellement exclusifs plutôt qu'un thermomètre de la
        // différence : ce qui compte n'est pas l'ampleur de l'avance mais son
        // signe, puisque la tuile paie 5/2 ou 4/4 (livret Découverte p. 3). Une
        // récompense absente du jeu vaut −1 partout : « il n'y a rien à mener ».
        for (i, kind) in AWARD_POOL.iter().enumerate() {
            let presente = game.awards.contains(kind);
            s.drapeau("recompense_", -1, &self.noms_awards[i], "_presente", presente);
            let (v_moi, v_adv) = if presente {
                (
                    crate::flow::award_value(*kind, moi),
                    crate::flow::award_value(*kind, adv),
                )
            } else {
                (0, 0)
            };
            s.drapeau(
                "recompense_",
                -1,
                &self.noms_awards[i],
                "_classement_je_mene",
                presente && v_moi > v_adv,
            );
            s.drapeau(
                "recompense_",
                -1,
                &self.noms_awards[i],
                "_classement_egalite",
                presente && v_moi == v_adv,
            );
            s.drapeau(
                "recompense_",
                -1,
                &self.noms_awards[i],
                "_classement_il_mene",
                presente && v_moi < v_adv,
            );
        }
        for ph in 0u8..=5 {
            s.drapeau("phase_en_cours_", ph as i64, "", "", game.phase_en_cours == ph);
        }

        // ---------------------------------------------- b. une entrée par carte
        //
        // Quatre drapeaux par carte projet : dans MA main, posée par moi, posée
        // par l'adversaire, dans la défausse. La défausse est une information
        // publique et le propriétaire du projet a accordé le comptage des cartes
        // passées (§3.3, décision du 11-08). La main d'en face, elle, n'est
        // jamais lue.
        t.dans_main.fill(false);
        t.pose_moi.fill(false);
        t.pose_adv.fill(false);
        t.defausse.fill(false);
        for id in moi.hand.iter() {
            if let Some(r) = self.rang(*id) {
                t.dans_main[r] = true;
            }
        }
        for id in moi.played.iter() {
            if let Some(r) = self.rang(*id) {
                t.pose_moi[r] = true;
            }
        }
        for id in adv.played.iter() {
            if let Some(r) = self.rang(*id) {
                t.pose_adv[r] = true;
            }
        }
        for id in game.discard.iter() {
            if let Some(r) = self.rang(*id) {
                t.defausse[r] = true;
            }
        }
        for (r, id) in self.projets.iter().enumerate() {
            let id = *id as i64;
            s.drapeau("projet", id, "", "_main", t.dans_main[r]);
            s.drapeau("projet", id, "", "_pose_moi", t.pose_moi[r]);
            s.drapeau("projet", id, "", "_pose_adv", t.pose_adv[r]);
            s.drapeau("projet", id, "", "_defausse", t.defausse[r]);
        }
        // La corporation de l'adversaire est publique une fois installée.
        let corpo_moi = moi.corporation.map(|c| db.corporations[c as usize].name.as_str());
        let corpo_adv = adv.corporation.map(|c| db.corporations[c as usize].name.as_str());
        // (D3) **LES CORPORATIONS QUE JE TIENS EN MAIN.** Sans ces cases, les
        // deux options de l'échange de corporations décrivaient exactement la
        // même situation : le réseau évaluait « je garde » et « je rends » avec
        // la même fiche, donc avec la même note, et tranchait par la marge
        // (`docs/AUDIT_MOTEUR.md`, § D3). La paire tirée entre dans l'état à
        // `flow::setup_game` étape 1, AVANT que la question soit posée.
        //
        // **Côté `moi_` seulement, et c'est un interdit dur** : la paire tenue
        // par l'adversaire est cachée (livret de mise en place). La corporation
        // INSTALLÉE, elle, est publique des deux côtés — ce sont les cases
        // `_moi` et `_adv` ci-dessus, et elles ne changent pas.
        //
        // Le champ est vidé par `flow::install_corporation_with` : une fois la
        // corporation choisie, ces seize cases retombent toutes à −1 et
        // l'information vit dans `corpo_…_moi`. Elles ne sont donc vraies que
        // pendant la mise en place ; c'est là que la décision se prend.
        for nom in self.corporations.iter() {
            s.drapeau("corpo_", -1, nom, "_moi", corpo_moi == Some(nom.as_str()));
            s.drapeau("corpo_", -1, nom, "_adv", corpo_adv == Some(nom.as_str()));
            s.drapeau(
                "corpo_",
                -1,
                nom,
                "_ma_main",
                moi.corps_en_main
                    .iter()
                    .any(|c| db.corporations[*c as usize].name == *nom),
            );
        }

        // -------------------------------------------------- c. par joueur, ×2
        // « Le joueur qui regarde vient toujours en premier, l'adversaire
        // ensuite » (§3.2).
        // Un SEUL passage de score : `score_breakdown` calcule la ventilation des
        // deux joueurs d'un coup, et c'est le point de calcul unique du moteur.
        let (parts, _, _) = crate::flow::score_breakdown(game, db);
        for (prefixe, pl) in [("moi_", moi), ("adv_", adv)] {
            s.thermo(prefixe, -1, "", "mc", pl.mc, S_MC);
            s.thermo(prefixe, -1, "", "chaleur", pl.heat, S_CHALEUR);
            s.thermo(prefixe, -1, "", "plantes", pl.plants, S_PLANTES);
            s.thermo(prefixe, -1, "", "prod_mc", pl.mc_prod, S_PROD_MC);
            s.thermo(prefixe, -1, "", "prod_chaleur", pl.heat_prod, S_PROD_CHALEUR);
            s.thermo(prefixe, -1, "", "prod_plantes", pl.plant_prod, S_PROD_PLANTES);
            s.thermo(prefixe, -1, "", "prod_cartes", pl.card_prod, S_PROD_CARTES);
            s.thermo(prefixe, -1, "", "nt", pl.tr, S_NT);
            s.thermo(prefixe, -1, "", "forets", pl.forests, S_FORETS);
            let p_index = if prefixe == "moi_" { siege } else { (siege + 1) % NUM_PLAYERS };
            s.thermo(prefixe, -1, "", "score_acquis", parts[p_index].acquis(), S_SCORE);
            // De l'adversaire : le NOMBRE de cartes en main, jamais leur identité.
            s.thermo(prefixe, -1, "", "main", pl.hand.len() as i64, S_MAIN);
            s.thermo(prefixe, -1, "", "posees", pl.played.len() as i64, S_POSEES);
            // (2.10) Les ressources POSÉES SUR LES CARTES, tous types
            // confondus. Elles ne figuraient nulle part : la récompense
            // Collectionneur (« le plus de ressources sur les cartes ») était
            // donc indéchiffrable, et la valeur d'une carte à ressources
            // invisible. Publique des deux côtés — `observe::state_view` les
            // publie carte par carte depuis le lot 3. Point de calcul unique :
            // `description::ressources_posees`, la même fonction que la mesure
            // des seuils appelle.
            s.thermo(
                prefixe,
                -1,
                "",
                "ressources_posees_total",
                ressources_posees(pl),
                S_RESSOURCES_POSEES,
            );
            for (i, t) in JOKER_TAG_CHOICES.iter().enumerate() {
                s.thermo(prefixe, -1, "badge_", t.as_str(), pl.tag_counts[i] as i64, S_BADGES[i]);
            }
            s.thermo(prefixe, -1, "", "acier", pl.steel_capacity, S_ACIER);
            s.thermo(prefixe, -1, "", "titane", pl.titanium_capacity, S_TITANE);
            // (D1) LA CARTE PHASE, TELLE QUE LA TABLE LA VOIT.
            //
            // Ces six cases lisent `phase_revelee`, et non plus
            // `previous_phase`. La différence est tout le défaut : le moteur
            // interroge les joueurs l'un après l'autre, et `previous_phase` est
            // écrit à la seconde où chacun répond. Les six cases livraient donc
            // au second interrogé la carte que le premier venait de poser FACE
            // CACHÉE (livret `docs/regles/livret-base.md:268`).
            //
            // `phase_revelee`, lui, n'est écrit qu'une fois les deux réponses
            // données (ligne 272 : « une fois que TOUS les joueurs ont fait leur
            // choix, les cartes Phase choisies sont révélées »). Pendant l'étape
            // de planification, ces cases montrent donc la carte de la manche
            // PRÉCÉDENTE — exactement ce qu'un joueur humain lit sur la pile de
            // cartes Phase déjà jouées ; dès la résolution, celle de la manche en
            // cours, comme avant.
            //
            // Le MÊME champ des deux côtés : une case nommée `previous_phase_3`
            // veut dire la même chose qu'on la lise de soi ou de l'adversaire,
            // à savoir « la carte Phase retournée sur la table ». C'est ce qui
            // rend la fiche du second interrogé insensible au choix caché du
            // premier, quel que soit le siège qui la regarde.
            s.drapeau(prefixe, -1, "previous_phase_", "aucune", pl.phase_revelee.is_none());
            for ph in 1u8..=5 {
                s.drapeau(
                    prefixe,
                    -1,
                    "previous_phase_",
                    match ph {
                        1 => "1",
                        2 => "2",
                        3 => "3",
                        4 => "4",
                        _ => "5",
                    },
                    pl.phase_revelee == Some(ph),
                );
            }
            // Lu sur le tableau du joueur plutôt que sur
            // `phase_upgrade_labels()`, qui allouerait des chaînes à chaque
            // évaluation. Même ordre imprimé que la table `AMELIORATIONS`, donc
            // même rang que côté JavaScript, qui lit les étiquettes publiées.
            for (i, a) in AMELIORATIONS.iter().enumerate() {
                let ph = (i / 2 + 1) as u8;
                let variante = if i % 2 == 0 {
                    crate::state::PhaseUpgrade::VariantA
                } else {
                    crate::state::PhaseUpgrade::VariantB
                };
                s.drapeau(
                    prefixe,
                    -1,
                    "amelioration_",
                    a,
                    pl.phase_upgrade(ph) == Some(variante),
                );
            }
            let p_repere = p_index;
            s.thermo(
                prefixe,
                -1,
                "",
                "reperes_atteints",
                game.milestones.iter().filter(|m| m.achieved_by[p_repere]).count() as i64,
                S_REPERES,
            );
        }

        // ------------------------------------------------- d. la jouabilité
        // Ce que je peux faire MAINTENANT. `main_payable` est publié par le
        // moteur : ni la page ni ce module ne savent ce qu'une carte coûte.
        let payable = crate::flow::main_payable(game, db, siege);
        let n_payable = payable.iter().filter(|x| **x).count() as i64;
        s.thermo("moi_", -1, "", "main_payable", n_payable, S_PAYABLE);
        for (coul, seuils, cle) in [
            ("verte", S_PAYABLE_VERTE, "payable_verte"),
            ("bleue", S_PAYABLE_BLEUE, "payable_bleue"),
            ("rouge", S_PAYABLE_ROUGE, "payable_rouge"),
        ] {
            let n = moi
                .hand
                .iter()
                .enumerate()
                .filter(|(i, id)| {
                    payable.get(*i).copied().unwrap_or(false)
                        && db.projects[**id as usize].color.nom_fr() == coul
                })
                .count() as i64;
            s.thermo("moi_", -1, "", cle, n, seuils);
        }

        // ------------------------------------- e. (2.8) ce que MA main contient
        //
        // La main était décrite par un drapeau par carte existante, et par aucun
        // résumé : une carte donnée est en main dans 4 % des situations, si bien
        // que « cette main-ci vaut cher » n'était apprenable qu'à travers des
        // milliers de poids qu'il aurait fallu ajuster ensemble
        // (`docs/AUDIT_ENTRAINEMENT.md`, § 2.8). Six grandeurs, calculées par
        // `description::resume_main` — la même fonction que la mesure des
        // seuils, sans quoi un palier serait mesuré sur une grandeur que la
        // fiche ne publie pas.
        //
        // **Réservé au joueur qui regarde.** Aucune case `adv_main_` : le
        // CONTENU de la main d'en face est caché (§ 3.3). Son NOMBRE de cartes
        // (`adv_main`) reste publié, il l'a toujours été et il est légitime.
        let resume = resume_main(db, &moi.hand);
        for (i, tag) in JOKER_TAG_CHOICES.iter().enumerate() {
            s.thermo(
                "moi_",
                -1,
                "main_badge_",
                tag.as_str(),
                resume.badges[i],
                S_MAIN_BADGES[i],
            );
        }
        for (i, coul) in ["verte", "bleue", "rouge"].iter().enumerate() {
            s.thermo(
                "moi_",
                -1,
                "main_couleur_",
                coul,
                resume.couleurs[i],
                S_MAIN_COULEURS[i],
            );
        }
        s.thermo("moi_", -1, "", "main_pv_imprimes", resume.pv_imprimes, S_MAIN_PV);
        s.thermo("moi_", -1, "", "main_prix_total", resume.prix_total, S_MAIN_PRIX_TOTAL);
        s.thermo("moi_", -1, "", "main_prix_min", resume.prix_min, S_MAIN_PRIX_MIN);

        // ------------------------------------------- f. (2.9) les six écarts
        //
        // Toutes les grandeurs ci-dessus sont publiées en valeur ABSOLUE, joueur
        // par joueur ; aucune n'exprimait la DIFFÉRENCE, alors que ce que le
        // réseau doit produire est une probabilité de victoire, c'est-à-dire une
        // fonction de la seule différence (`docs/AUDIT_ENTRAINEMENT.md`, § 2.9).
        //
        // **Une seule série**, pas une par joueur : l'écart de l'adversaire est
        // l'opposé du mien, le publier deux fois n'apprend rien et coûte des
        // poids. Les paliers sont donc signés (§ [`S_ECARTS`]).
        //
        // Les scores acquis viennent de `parts`, c'est-à-dire du même et unique
        // `flow::score_breakdown` que les cases `score_acquis` ci-dessus : les
        // deux publications ne peuvent pas se contredire.
        let e = ecarts(moi, adv, parts[siege].acquis(), parts[(siege + 1) % NUM_PLAYERS].acquis());
        for (i, nom) in NOMS_ECARTS.iter().enumerate() {
            s.thermo("ecart_", -1, "", nom, e[i], S_ECARTS[i]);
        }
    }

    #[inline(always)]
    fn rang(&self, id: u16) -> Option<usize> {
        match self.rang_projet.get(id as usize) {
            Some(&r) if r != usize::MAX => Some(r),
            _ => None,
        }
    }
}

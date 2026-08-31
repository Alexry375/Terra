//! **L'ÉTALON DE MESURE, PORTÉ EN RUST.**
//!
//! Jumeau exact de `web/webapp/joueurs/reflechi.js`. Il ne connaît aucune règle
//! du jeu : il choisit parmi ce que le moteur vient d'énumérer, à l'aide d'une
//! échelle de valeur écrite à la main. Tous les chiffres de progression du
//! projet sont des taux de victoire contre lui ; le porter, c'est rendre la
//! mesure native — à condition qu'il décide **exactement** comme l'original,
//! faute de quoi les mesures d'aujourd'hui ne se compareraient plus à celles
//! d'hier.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **CE QUI REND LE PORTAGE MOINS DIRECT QU'IL N'EN A L'AIR.**
//!
//! L'étalon JavaScript ne voit jamais les arguments du trait [`Policy`]. Il voit
//! un DESCRIPTEUR JSON fabriqué par le pont (`web/webapp/wasm/src/lib.rs`,
//! `Harnais::decrire_choix` et les vingt méthodes qui l'entourent) : un `type`,
//! une liste d'`options` portant chacune un `libelle`, un drapeau `passer`,
//! parfois un `a_choisir`. Et plusieurs de ses décisions se prennent SUR LE
//! LIBELLÉ — « Forêt (plantes) » vaut 100, une option dont le libellé commence
//! par « Ne rien » perd six points. Porter l'étalon, c'est donc porter deux
//! choses : l'échelle de valeur, et la fabrication des libellés du pont. Ce
//! fichier refait les seconds à partir du moteur (`crate::choice`), puis leur
//! applique la première.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **IL NE REGARDE PAS LA MAIN D'EN FACE**, comme l'original. Une seule fonction
//! de ce fichier touche à `game.players` — [`mon_siege`] — et elle prend le
//! siège qui décide, jamais un autre. Tout le reste travaille sur ce qu'elle
//! rend.
//!
//! **IL NE TIRE RIEN AU SORT.** Aucune méthode ne touche au générateur de la
//! partie : à décision et à état égaux, la réponse est toujours la même.

use crate::cards::{CardsDb, JOKER_TAG_CHOICES, TAG_COUNT};
use crate::choice::{
    action_res_quantity, describe_branch, describe_selector_grant, spend_amount_quantity, tag_label,
    ChoiceContext,
};
use crate::effects::RevealFilter;
use crate::flow::main_payable;
use crate::policy::{ActionOpt, ConstructionBonus, Policy};
use crate::state::{GameState, NUM_OCEANS, OXYGEN_MAX, TEMPERATURE_MAX};
use rand::rngs::StdRng;

// ────────────────────────────────────────────────────────────────── les réglages

/// Les poids de l'échelle de valeur, recopiés un à un de `REGLAGES`
/// (`reflechi.js`, ligne 76). Un seul de ces nombres qui changerait ferait de la
/// balance native un autre instrument que l'ancienne.
pub mod reglages {
    // — La valeur d'une carte projet, en main comme sur la table.
    pub const PV_EN_MAIN: f64 = 6.0;
    pub const PRIX_EN_MAIN: f64 = 0.35;
    pub const PV_A_LA_POSE: f64 = 7.0;
    pub const PRIX_A_LA_POSE: f64 = 0.3;
    pub const BADGE_EN_MAIN: f64 = 0.8;

    // — Le choix de la carte Phase (la décision la plus fréquente).
    pub const DEV_BASE: f64 = 1.0;
    pub const DEV_PAR_CARTE: f64 = 5.0;
    pub const CON_BASE: f64 = 1.0;
    pub const CON_PAR_CARTE: f64 = 5.0;
    pub const ACT_BASE: f64 = 0.0;
    pub const ACT_PAR_MC: f64 = 0.05;
    pub const ACT_PAR_PLANTE: f64 = 0.8;
    pub const PRO_BASE: f64 = 7.0;
    pub const PRO_DECROISSANCE: f64 = 0.3;
    pub const PRO_FIN_DE_PARTIE: f64 = 7.0;
    pub const REC_BASE: f64 = 3.0;
    pub const REC_PAR_CARTE_MANQUANTE: f64 = 3.5;
    pub const MAIN_VISEE: f64 = 7.0;
    pub const SEUIL_MULLIGAN: f64 = 0.0;

    // — Les actions standard, en phase Action.
    pub const FORET_PLANTES: f64 = 100.0;
    pub const FORET_MC: f64 = 62.0;
    pub const OCEAN: f64 = 52.0;
    pub const TEMPERATURE: f64 = 48.0;
    pub const ACTION_DE_CARTE: f64 = 40.0;
    pub const PASSER: f64 = 0.0;

    // — La vente.
    pub const PRIX_VENTE_MINI: i64 = 10;
    pub const GARDE_MINI: usize = 4;
}

// ─────────────────────────────────────── ce que le joueur a le droit de voir

/// Une carte de MA main, telle que la vue de l'état la publie : ni points de
/// victoire ni badges — `observe::player_view` ne donne, carte par carte, que le
/// prix, la couleur et la payabilité. C'est la raison pour laquelle l'étalon
/// juge sa vente sur le prix et la portée, jamais sur [`valeur_en_main`].
#[derive(Clone, Copy)]
pub struct CarteDeMain {
    pub prix: i64,
    pub verte: bool,
}

/// Le revenu de production du siège, dans les quatre monnaies que la vue publie.
#[derive(Clone, Copy, Default)]
pub struct Production {
    pub mc: f64,
    pub plantes: f64,
    pub chaleur: f64,
    pub cartes: f64,
}

/// **MON SIÈGE, ET RIEN QUE LUI.** Pendant exact de `monSiege` : la seule
/// structure du fichier qui vienne des joueurs, et elle ne vient que du siège qui
/// décide.
#[derive(Clone)]
pub struct MonSiege {
    /// Ce que la VUE publie de ma main : prix et couleur.
    pub main: Vec<CarteDeMain>,
    /// Les identifiants de la même main, dans le même ordre. Le pont les emploie
    /// pour donner une carte complète — points, badges, prix — à chaque option
    /// d'une pose (`Harnais::carte_de_main`) ; l'étalon les y lit.
    pub identifiants: Vec<u16>,
    pub payable: Vec<bool>,
    pub mc: f64,
    pub plantes: f64,
    pub badges: [u32; TAG_COUNT],
    pub production: Production,
}

/// Ce que `monSiege` rend, relevé sur l'état vivant du siège qui décide.
pub fn mon_siege(game: &GameState, db: &CardsDb, siege: usize) -> MonSiege {
    let pl = &game.players[siege];
    let main: Vec<CarteDeMain> = pl
        .hand
        .iter()
        .map(|&id| match db.projects.get(id as usize) {
            Some(c) => CarteDeMain {
                prix: c.price,
                verte: c.color.nom_fr() == "verte",
            },
            None => CarteDeMain {
                prix: 0,
                verte: false,
            },
        })
        .collect();
    // « payable » vient de MON `main_payable` : ce que J'AI les moyens de payer.
    // Le défaut est NON payable, comme dans l'original — un défaut permissif ne
    // dégraderait pas le choix de phase, il l'inverserait.
    let mut payable = main_payable(game, db, siege);
    payable.resize(main.len(), false);
    MonSiege {
        main,
        identifiants: pl.hand.clone(),
        payable,
        mc: pl.mc as f64,
        plantes: pl.plants as f64,
        badges: pl.tag_counts,
        production: Production {
            mc: pl.mc_prod as f64,
            plantes: pl.plant_prod as f64,
            chaleur: pl.heat_prod as f64,
            cartes: pl.card_prod as f64,
        },
    }
}

/// Ce qui n'appartient à personne : la planète. 0 au départ, 1 quand elle est
/// terminée. Jumeau de `leMonde`.
pub fn avancement_du_monde(game: &GameState) -> f64 {
    let part = |x: f64, max: f64| -> f64 {
        if max != 0.0 {
            (x / max).min(1.0)
        } else {
            0.0
        }
    };
    (part(game.oxygen as f64, OXYGEN_MAX as f64)
        + part(game.temperature as f64, TEMPERATURE_MAX as f64)
        + part(game.oceans_revealed as f64, NUM_OCEANS as f64))
        / 3.0
}

// ───────────────────────────────────────────────────── l'échelle de valeur

/// Une carte telle que le pont l'ÉNUMÈRE dans une option (`carte_json`) : là, et
/// là seulement, les points de victoire et les badges existent.
#[derive(Clone, Copy, Default)]
pub struct CarteOption {
    pub pv: f64,
    pub badges: f64,
    pub prix: f64,
}

/// La carte d'identifiant `id`, sous la forme que les options du pont portent.
/// Un identifiant inconnu donne une option sans prix, sans badge et sans point —
/// `carte_json` n'écrit alors aucune de ces clefs, et l'étalon y lit des zéros.
pub fn carte_option(db: &CardsDb, id: u16) -> CarteOption {
    match db.projects.get(id as usize) {
        Some(c) => CarteOption {
            pv: c.vp as f64,
            badges: c.tags.len() as f64,
            prix: c.price as f64,
        },
        None => CarteOption::default(),
    }
}

/// La valeur d'une carte projet qu'on GARDE (main, pioche, révélation).
///
/// L'ordre des trois termes est celui de l'original, et il compte : à opérations
/// identiques et dans le même ordre, les deux langages rendent le même nombre au
/// bit près, et un départage serré ne bascule pas d'un côté à l'autre.
pub fn valeur_en_main(c: CarteOption) -> f64 {
    reglages::PV_EN_MAIN * c.pv + reglages::BADGE_EN_MAIN * c.badges
        - reglages::PRIX_EN_MAIN * c.prix
}

/// La valeur d'une carte qu'on POSE maintenant. Le prix compte POSITIVEMENT, à
/// l'inverse de la main : le moteur n'énumère que les cartes payables, donc entre
/// deux cartes qu'on peut s'offrir, la plus chère est la plus forte.
pub fn valeur_a_la_pose(c: CarteOption) -> f64 {
    reglages::PV_A_LA_POSE * c.pv + reglages::PRIX_A_LA_POSE * c.prix
}

/// La valeur d'un revenu de production, ramenée à une échelle commune.
pub fn valeur_production(p: Production) -> f64 {
    1.0 * p.mc + 0.7 * p.chaleur + 1.2 * p.plantes + 2.0 * p.cartes
}

// ────────────────────────────────────────── la lecture des libellés à la main

/// Un caractère est-il un caractère de mot au sens de `\b` en JavaScript ?
///
/// La question n'est pas anecdotique : `\b` du JavaScript ne connaît que
/// `[A-Za-z0-9_]`. Une lettre accentuée n'en est donc PAS un, et `/^non\b/`
/// accepte « nonè ». Un portage qui emploierait la notion de mot d'Unicode
/// répondrait autrement sur des libellés français.
fn caractere_de_mot(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `/^motif\b/` : commence par `motif`, suivi d'une frontière de mot.
fn commence_par_mot(t: &str, motif: &str) -> bool {
    match t.strip_prefix(motif) {
        Some(reste) => match reste.chars().next() {
            None => true,
            Some(c) => !caractere_de_mot(c),
        },
        None => false,
    }
}

/// `/\bmot\b/` : le texte contient-il `mot` entouré de frontières de mot ?
fn contient_mot(t: &str, mot: &str) -> bool {
    let octets = t.as_bytes();
    let m = mot.as_bytes();
    if m.is_empty() || octets.len() < m.len() {
        return false;
    }
    for i in 0..=(octets.len() - m.len()) {
        if !t.is_char_boundary(i) || &octets[i..i + m.len()] != m {
            continue;
        }
        let avant = t[..i].chars().next_back();
        let apres = t[i + m.len()..].chars().next();
        let debut = avant.map(|c| !caractere_de_mot(c)).unwrap_or(true);
        let fin = apres.map(|c| !caractere_de_mot(c)).unwrap_or(true);
        if debut && fin {
            return true;
        }
    }
    false
}

/// Les espaces que `\s` reconnaît en JavaScript.
fn espace_js(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\u{0b}'
            | '\u{0c}'
            | '\r'
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

/// **DERNIER RECOURS** — quand un point de décision n'est pas reconnu par nom, on
/// ne répond pas au hasard : on lit le libellé, écrit pour un humain, et qui dit
/// en clair ce qu'on gagne. Grossier, et assumé : ce chemin ne sert qu'aux points
/// de décision rares.
///
/// L'ordre des additions est celui de l'original, y compris celui des gains
/// chiffrés, parcourus de gauche à droite comme `matchAll` les rend.
pub fn valeur_du_libelle(libelle: &str) -> f64 {
    let t = libelle.to_lowercase();
    let mut v = 0.0f64;
    // « ne rien faire » est presque toujours le choix vide.
    if t.starts_with("ne rien")
        || commence_par_mot(&t, "non")
        || commence_par_mot(&t, "garder")
        || t.starts_with("décider après")
        || t.starts_with("decider apres")
    {
        v -= 6.0;
    }
    if commence_par_mot(&t, "oui")
        || t.starts_with("poser")
        || t.starts_with("piocher")
        || t.starts_with("améliorer")
        || t.starts_with("ameliorer")
        || t.starts_with("gagner")
    {
        v += 4.0;
    }
    // Les gains chiffrés : « +2 plantes », « −10 MC », « 1 pas ».
    for (positif, n) in gains_chiffres(&t) {
        v += (if positif { 1.0 } else { -1.0 }) * n.min(12.0) * 0.8;
    }
    if contient_mot(&t, "pas") {
        v += 3.0; // un pas de terraformation = un point de NT
    }
    if t.contains("carte") {
        v += 1.5;
    }
    v
}

/// `matchAll(/([+\-−])\s*(\d+)/g)` : les couples (signe positif ?, nombre), dans
/// l'ordre où le texte les donne et sans recouvrement.
fn gains_chiffres(t: &str) -> Vec<(bool, f64)> {
    let cs: Vec<char> = t.chars().collect();
    let mut sortie = Vec::new();
    let mut i = 0usize;
    while i < cs.len() {
        let c = cs[i];
        if c == '+' || c == '-' || c == '\u{2212}' {
            let mut j = i + 1;
            while j < cs.len() && espace_js(cs[j]) {
                j += 1;
            }
            let debut = j;
            while j < cs.len() && cs[j].is_ascii_digit() {
                j += 1;
            }
            if j > debut {
                let chiffres: String = cs[debut..j].iter().collect();
                // `Number("0012")` vaut 12 ; un nombre trop grand devient
                // l'infini, exactement comme en JavaScript.
                let n: f64 = chiffres.parse().unwrap_or(f64::INFINITY);
                sortie.push((c == '+', n));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    sortie
}

// ──────────────────────────────────────────────────────── outils de sélection

/// L'indice du maximum, premier arrivé premier servi. La comparaison est
/// STRICTEMENT supérieure : à note égale, le premier indice l'emporte. C'est le
/// point qui casse les portages, et c'est le juge du critère B qui le mesure.
pub fn meilleur(n: usize, note: impl Fn(usize) -> f64) -> usize {
    let mut i_best = 0usize;
    let mut v_best = f64::NEG_INFINITY;
    for i in 0..n {
        let v = note(i);
        if v > v_best {
            v_best = v;
            i_best = i;
        }
    }
    i_best
}

/// Les `k` meilleurs indices, rendus dans l'ordre CROISSANT.
///
/// Le tri de l'original est `note(b) - note(a) || a - b` : note décroissante,
/// puis indice croissant. C'est un ordre TOTAL, la stabilité du tri n'y ajoute
/// donc rien — mais il faut le refaire terme pour terme, sans quoi deux options
/// de même note ne sortiraient pas dans le même ordre.
///
/// Une note qui ne se compare pas fait rendre `0` à la soustraction du
/// JavaScript, qui retombe alors sur l'indice : c'est le cas `None` ci-dessous.
pub fn les_meilleurs(n: usize, k: usize, note: impl Fn(usize) -> f64) -> Vec<usize> {
    let mut ordre: Vec<(f64, usize)> = (0..n).map(|i| (note(i), i)).collect();
    ordre.sort_by(|a, b| match b.0.partial_cmp(&a.0) {
        Some(std::cmp::Ordering::Equal) | None => a.1.cmp(&b.1),
        Some(o) => o,
    });
    let mut pris: Vec<usize> = ordre.into_iter().take(k.min(n)).map(|(_, i)| i).collect();
    pris.sort_unstable();
    pris
}

// ───────────────────────────────────────────────────── les points de décision

/// I à V : quelle carte Phase choisir. C'est la décision la plus fréquente.
pub fn noter_phase(phase: u8, moi: &MonSiege, avancement: f64) -> f64 {
    let payables = moi
        .main
        .iter()
        .enumerate()
        .filter(|(i, _)| moi.payable[*i])
        .count();
    let vertes = moi
        .main
        .iter()
        .enumerate()
        .filter(|(i, c)| moi.payable[*i] && c.verte)
        .count();
    let autres = payables - vertes;
    let prod_totale = valeur_production(moi.production);
    match phase {
        // Développement — c'est là que les cartes vertes se posent.
        1 => reglages::DEV_BASE + reglages::DEV_PAR_CARTE * (vertes.min(3) as f64),
        // Construction — c'est là que les bleues et les rouges se posent.
        2 => reglages::CON_BASE + reglages::CON_PAR_CARTE * (autres.min(3) as f64),
        // Action — les actions standard et celles des cartes posées.
        3 => {
            reglages::ACT_BASE
                + reglages::ACT_PAR_MC * moi.mc.min(40.0)
                + reglages::ACT_PAR_PLANTE * moi.plantes.min(12.0)
        }
        // Production — un revenu qui rapporte à chaque manche restante.
        4 => (reglages::PRO_BASE
            - reglages::PRO_DECROISSANCE * prod_totale
            - reglages::PRO_FIN_DE_PARTIE * avancement)
            .max(0.0),
        // Recherche — on ne joue pas ce qu'on n'a pas en main.
        5 => {
            reglages::REC_BASE
                + reglages::REC_PAR_CARTE_MANQUANTE
                    * (reglages::MAIN_VISEE - moi.main.len() as f64).max(0.0)
        }
        _ => 0.0,
    }
}

/// III — Action : ce qu'on active. Les actions standard font le score.
pub fn noter_action(libelle: &str) -> f64 {
    if libelle.starts_with("Forêt") {
        return if libelle.to_lowercase().contains("plante") {
            reglages::FORET_PLANTES
        } else {
            reglages::FORET_MC
        };
    }
    if libelle.starts_with("Océan") {
        return reglages::OCEAN;
    }
    if libelle.starts_with("Température") {
        return reglages::TEMPERATURE;
    }
    if libelle.starts_with("Action de") {
        return reglages::ACTION_DE_CARTE;
    }
    valeur_du_libelle(libelle)
}

/// Le libellé d'une action standard, tel que le pont l'écrit (`nom_action`).
pub fn libelle_action(db: &CardsDb, o: &ActionOpt) -> String {
    match o {
        ActionOpt::ForestWithPlants => "Forêt (plantes)".to_string(),
        ActionOpt::ForestWithMc => "Forêt (MC)".to_string(),
        ActionOpt::TemperatureWithHeat => "Température (chaleur)".to_string(),
        ActionOpt::TemperatureWithMc => "Température (MC)".to_string(),
        ActionOpt::OceanWithMc => "Océan (MC)".to_string(),
        ActionOpt::BlueAction(i) => match db.projects.get(*i as usize) {
            Some(c) => format!("Action de {}", c.name),
            None => format!("Action de la carte bleue {i}"),
        },
        ActionOpt::CorpAction => "Action de la corporation".to_string(),
    }
}

/// **CE JOUEUR VEND** — mêmes deux critères que l'original : le PRIX et la
/// PORTÉE. Une carte que je n'ai pas les moyens de payer et qui coûte au moins
/// `PRIX_VENTE_MINI` est une carte qui dort ; les plus chères dorment le plus
/// longtemps, et ce sont elles qui partent d'abord. Jamais en dessous de
/// `GARDE_MINI` cartes en main : une main vide ne pose plus rien.
///
/// Rend les indices de main à vendre, dans l'ordre croissant ; la liste vide veut
/// dire « je ne vends rien ».
pub fn vente_eventuelle(moi: &MonSiege) -> Vec<usize> {
    let mut candidates: Vec<(usize, i64)> = moi
        .main
        .iter()
        .enumerate()
        .map(|(i, c)| (i, c.prix))
        .filter(|(i, prix)| !moi.payable[*i] && *prix >= reglages::PRIX_VENTE_MINI)
        .collect();
    // La plus chère d'abord ; à prix égal, l'indice le plus petit.
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut a_vendre: Vec<usize> = Vec::new();
    for (i, _) in candidates {
        if moi.main.len() - a_vendre.len() <= reglages::GARDE_MINI {
            break;
        }
        a_vendre.push(i);
    }
    a_vendre.sort_unstable();
    a_vendre
}

// ───────────────────────────────────────────────────────────────── le cerveau

/// **L'ÉTALON.** Sans mémoire d'une décision à l'autre : les trois champs qu'il
/// porte sont des relevés de l'état vivant, écrasés à chaque observation, jamais
/// une trace de ce qui a été joué.
pub struct Reflechi<'a> {
    db: &'a CardsDb,
    /// Le siège que ce cerveau occupe. Il ne lit jamais l'autre.
    siege: usize,
    /// Ce que mon siège a le droit de voir, relevé à l'observation qui précède
    /// chacune de MES décisions.
    vue: Option<MonSiege>,
    /// L'avancement de la planète, relevé au même instant.
    avancement: f64,
    /// Le même relevé, pris à l'OCCASION DE VENTE — qui précède l'observation.
    /// `None` dès que l'occasion en cours ne prépare pas une de mes décisions.
    occasion: Option<MonSiege>,
}

impl<'a> Reflechi<'a> {
    pub fn new(db: &'a CardsDb, siege: usize) -> Reflechi<'a> {
        Reflechi {
            db,
            siege,
            vue: None,
            avancement: 0.0,
            occasion: None,
        }
    }

    /// Le siège que ce cerveau occupe.
    pub fn siege(&self) -> usize {
        self.siege
    }

    /// Ce que j'ai sous les yeux. Absent avant ma première observation — le
    /// moteur observe toujours avant de me demander quoi que ce soit.
    fn moi(&self) -> Option<&MonSiege> {
        self.vue.as_ref()
    }

    /// Combien de cartes de ma main je peux m'offrir.
    fn payables(&self) -> usize {
        match self.moi() {
            Some(moi) => moi.payable.iter().filter(|p| **p).count(),
            None => 0,
        }
    }

    /// Le libellé de l'option `i` d'un choix contextuel, tel que le pont l'écrit
    /// (`Harnais::decrire_choix`). Les points de décision que l'étalon reconnaît
    /// par leur nom ne passent pas par ici ; les autres n'ont que le libellé.
    fn libelle_du_contexte(&self, ctx: &ChoiceContext, i: usize) -> String {
        match ctx {
            ChoiceContext::CorpTrBoost { cost_mc, steps, .. } => {
                if i == 0 {
                    format!("Payer {cost_mc} MC et gagner {steps} NT")
                } else {
                    "Ne pas payer".to_string()
                }
            }
            ChoiceContext::CardAlternative { branches, .. }
            | ChoiceContext::ActionAlternative { branches, .. } => {
                describe_branch(branches[i].effects)
            }
            ChoiceContext::MicrobeDiscount { count, amount, .. } => {
                if i == 0 {
                    format!("Oui : −{amount} MC contre {count} microbe(s)")
                } else {
                    "Non : garder les microbes et payer le prix plein".to_string()
                }
            }
            ChoiceContext::PlantDiscount { plants, amount, .. } => {
                if i == 0 {
                    format!("Oui : −{amount} MC contre {plants} plante(s)")
                } else {
                    "Non : garder les plantes et payer le prix plein".to_string()
                }
            }
            ChoiceContext::HeatAsMc { .. } => {
                if i == 0 {
                    "Oui : payer en convertissant de la chaleur".to_string()
                } else {
                    "Non : payer en défaussant des cartes".to_string()
                }
            }
            ChoiceContext::DiscardToDraw {
                tag,
                draw_if,
                draw_else,
                ..
            } => {
                if i == 0 {
                    format!(
                        "Défausser une carte (piocher {draw_if} avec un badge {}, {draw_else} \
                         sinon)",
                        tag_label(*tag)
                    )
                } else {
                    "Ne rien défausser".to_string()
                }
            }
            ChoiceContext::SpendAmount { spend, gain, .. } => {
                let q = spend_amount_quantity(i);
                format!(
                    "Dépenser {} pour gagner {}",
                    action_res_quantity(*spend, q),
                    action_res_quantity(*gain, q)
                )
            }
            ChoiceContext::SelectorBonus { branches, .. } => describe_selector_grant(&branches[i]),
            // Ces deux-là sont reconnus par leur NOM : l'étalon lit la phase et
            // la variante d'un côté, la production de l'autre. Le libellé du pont
            // ne leur sert à rien, on ne le fabrique donc pas.
            ChoiceContext::PhaseUpgrade { .. } | ChoiceContext::ReplayProduction { .. } => {
                String::new()
            }
        }
    }
}

impl Policy for Reflechi<'_> {
    /// L'état vivant, juste avant chacune de MES décisions : on en garde ce que
    /// mon siège a le droit de voir, et rien d'autre.
    fn observe(&mut self, game: &GameState, player: usize) {
        if player != self.siege {
            return;
        }
        self.vue = Some(mon_siege(game, self.db, player));
        self.avancement = avancement_du_monde(game);
    }

    /// **L'ÉTAT À L'OCCASION DE VENTE**, qui précède l'observation.
    ///
    /// L'original décide sa vente au moment où le moteur lui pose une question,
    /// et le pont la replace à l'occasion qui précède cette question-là. En
    /// natif, l'occasion vient d'abord, elle est offerte AUX DEUX SIÈGES, et elle
    /// n'apporte pas l'état. Sans ce relevé, l'étalon vendrait à l'occasion d'une
    /// décision qui n'est pas la sienne — ce que l'original ne fait jamais, la
    /// page n'interrogeant que le fournisseur du siège qui décide.
    ///
    /// **ET SEULEMENT SI UNE QUESTION SUIT.** Le moteur ouvre aussi des occasions
    /// à des points qui ne demandent rien (`question_posee` faux) : la page n'y
    /// affiche rien et n'interroge personne, l'original n'y vend donc jamais. Le
    /// relevé est effacé à CHAQUE occasion, y compris celles-là — un relevé qui
    /// survivrait à son occasion ferait vendre à la suivante, qui n'est pas la
    /// même main ni la même question (mesuré le 31-08, graine 4 sièges échangés,
    /// décision 128 : une vente de trop, et les deux parties divergeaient).
    fn observer_l_occasion(&mut self, game: &GameState, decideur: usize, question_posee: bool) {
        self.occasion = if question_posee && decideur == self.siege {
            Some(mon_siege(game, self.db, decideur))
        } else {
            None
        };
    }

    fn corp_mulligan(&mut self, _rng: &mut StdRng, _player: usize, corps: &[u16]) -> bool {
        // « Garder » (option 0) ou « Remplacer les 2 » (option 1) : on garde si la
        // meilleure des deux corporations en main annonce au moins 24 MC de
        // départ. `true` veut dire remplacer.
        let mut mieux = 0i64;
        for c in corps {
            if let Some(co) = self.db.corporations.get(*c as usize) {
                mieux = mieux.max(co.starting_mc);
            }
        }
        mieux < 24
    }

    fn project_mulligan(&mut self, _rng: &mut StdRng, _player: usize, hand: &[u16]) -> Vec<usize> {
        // Nombre libre : on remplace tout ce qui est sous la barre.
        let mut a_jeter = Vec::new();
        for (i, id) in hand.iter().enumerate() {
            if valeur_en_main(carte_option(self.db, *id)) < reglages::SEUIL_MULLIGAN {
                a_jeter.push(i);
            }
        }
        a_jeter
    }

    fn pick_corporation(&mut self, _rng: &mut StdRng, _player: usize, corps: &[u16]) -> usize {
        // On lit ce que la corporation annonce : son capital de départ et ses
        // badges. Rien d'autre n'est visible à cet instant.
        meilleur(corps.len(), |i| {
            match self.db.corporations.get(corps[i] as usize) {
                Some(c) => c.starting_mc as f64 + 2.0 * c.tags.len() as f64,
                None => 0.0,
            }
        })
    }

    fn pick_phase(&mut self, _rng: &mut StdRng, _player: usize, allowed: &[u8]) -> u8 {
        let Some(moi) = self.moi() else {
            return allowed[0];
        };
        let avancement = self.avancement;
        allowed[meilleur(allowed.len(), |i| noter_phase(allowed[i], moi, avancement))]
    }

    fn choose_build(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        let n = affordable.len();
        // Sans option, le pont pose quand même la question et l'étalon rend
        // l'indice de « passer ».
        if n == 0 {
            return None;
        }
        // Poser vaut mieux que passer : le moteur n'énumère que ce qui est
        // payable, et une carte en main ne rapporte rien. La carte de l'option est
        // celle de la main OBSERVÉE, comme le pont la publie.
        let vide: Vec<u16> = Vec::new();
        let main = match self.moi() {
            Some(moi) => &moi.identifiants,
            None => &vide,
        };
        let choix = meilleur(n, |i| match main.get(affordable[i]) {
            Some(id) => valeur_a_la_pose(carte_option(self.db, *id)),
            None => valeur_a_la_pose(CarteOption::default()),
        });
        Some(affordable[choix])
    }

    fn construction_bonus(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        let libelles = [
            "Piocher 1 carte AVANT de poser",
            "Piocher 1 carte APRÈS avoir posé",
            "Poser une carte bleue/rouge supplémentaire",
        ];
        match meilleur(3, |i| note_construction_bonus(libelles[i])) {
            0 => ConstructionBonus::DrawCardBefore,
            1 => ConstructionBonus::DrawCard,
            _ => ConstructionBonus::SecondBuild,
        }
    }

    fn construction_bonus_avant(&mut self, _rng: &mut StdRng, _player: usize) -> bool {
        let libelles = ["Piocher 1 carte tout de suite", "Décider après avoir posé"];
        meilleur(2, |i| note_construction_bonus(libelles[i])) == 0
    }

    fn construction_bonus_apres(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        let libelles = [
            "Piocher 1 carte",
            "Poser une carte bleue/rouge supplémentaire",
        ];
        if meilleur(2, |i| note_construction_bonus(libelles[i])) == 1 {
            ConstructionBonus::SecondBuild
        } else {
            ConstructionBonus::DrawCard
        }
    }

    fn action_choice(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        // Le pont n'ouvre pas la question quand il n'y a rien à activer.
        if options.is_empty() {
            return None;
        }
        let libelles: Vec<String> = options.iter().map(|o| libelle_action(self.db, o)).collect();
        let i_best = meilleur(libelles.len(), |i| noter_action(&libelles[i]));
        // « passer » est toujours offert ici, et vaut `REGLAGES.passer` = 0.
        if noter_action(&libelles[i_best]) <= reglages::PASSER {
            None
        } else {
            Some(i_best)
        }
    }

    fn action_amount(&mut self, _rng: &mut StdRng, _player: usize, max: i64) -> i64 {
        // Le moteur ne propose un montant que pour une dépense qu'il vient
        // d'autoriser, et qui rapporte quelque chose en face : on dépense le
        // maximum.
        max
    }

    /// **LA VENTE, replacée à l'occasion.**
    ///
    /// L'étalon JavaScript vend en RÉPONDANT : le pont consomme son entrée au
    /// point d'occasion qui précède la question, puis repose la même question sur
    /// l'état d'après. En natif, l'occasion est le seul endroit où une vente
    /// puisse entrer — on y refait donc le même jugement, sur l'état relevé par
    /// [`Policy::observer_l_occasion`], qui est celui que la question aurait
    /// montré.
    ///
    /// Rien n'est vendu si l'occasion ne prépare pas une de MES décisions : la
    /// page n'interroge que le fournisseur du siège qui décide.
    fn vendre_librement(&mut self, _rng: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        if joueur != self.siege {
            return Vec::new();
        }
        let Some(moi) = self.occasion.take() else {
            return Vec::new();
        };
        // CEINTURE, ET RIEN DE PLUS — je le dis parce qu'il serait facile de la
        // prendre pour la garde `mains_a_l_occasion` de `flow::observer`, qu'elle
        // n'est pas. Elle vérifie que le relevé pris par `observer_l_occasion`
        // porte bien sur la main qu'on me présente ici ; comme rien ne s'exécute
        // entre les deux appels, elle est vraie à tous les sites connus. La vraie
        // garde du pont compare les mains des DEUX sièges au moment de l'occasion
        // à celles du moment de la QUESTION, qui vient après — et cet écart-là,
        // aucune politique ne peut le voir depuis ici, faute de connaître
        // l'avenir. Il est déclaré au carnet, § « Ce qui reste douteux ».
        if moi.identifiants != main {
            return Vec::new();
        }
        vente_eventuelle(&moi)
    }

    /// La voie anonyme n'est empruntée par aucun site du moteur — le pont y
    /// déclare d'ailleurs une faute. On rend le premier indice plutôt que le
    /// tirage du corps par défaut : l'étalon ne touche à aucun générateur, même
    /// sur un chemin mort.
    fn choose_option(&mut self, _rng: &mut StdRng, _player: usize, _n: usize) -> usize {
        0
    }

    fn choose_option_ctx(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        // Le pont ne pose pas de question sans option.
        if n == 0 {
            return 0;
        }
        match ctx {
            // « Améliorez une carte Phase » : celle qu'on choisira le plus
            // souvent. Entre la variante A et la B, le libellé ne dit rien qu'on
            // sache lire sans connaître les règles : on tranche pour A, parce
            // qu'il faut trancher et que le départage doit rester déterministe.
            ChoiceContext::PhaseUpgrade { candidates, .. } => {
                let Some(moi) = self.moi() else { return 0 };
                let avancement = self.avancement;
                meilleur(n, |i| {
                    noter_phase(candidates[i].phase, moi, avancement)
                        + if candidates[i].variant.label() == "A" {
                            0.01
                        } else {
                            0.0
                        }
                })
            }
            ChoiceContext::ReplayProduction { candidates } => meilleur(n, |i| {
                valeur_production(Production {
                    mc: candidates[i].mc as f64,
                    plantes: candidates[i].plants as f64,
                    chaleur: candidates[i].heat as f64,
                    cartes: candidates[i].cards as f64,
                })
            }),
            // « Poser une carte de plus » ne vaut que si on a de quoi la poser.
            ChoiceContext::SelectorBonus { branches, .. } => {
                let payables = self.payables();
                let libelles: Vec<String> = branches.iter().map(describe_selector_grant).collect();
                meilleur(n, |i| {
                    if libelles[i].contains("poser") {
                        if payables > 0 {
                            10.0
                        } else {
                            0.0
                        }
                    } else {
                        valeur_du_libelle(&libelles[i])
                    }
                })
            }
            // Payer avec ses ressources plutôt qu'en MC : on accepte quand la
            // remise est franche (au moins 4 MC par ressource dépensée), sinon on
            // garde ses plantes — elles deviennent des forêts, donc des points.
            // Le pont n'écrit la ressource et la réduction que sur l'option 0 ;
            // l'option 1 (« payer le prix plein ») porte deux zéros.
            ChoiceContext::PlantDiscount { plants, amount, .. } => {
                note_reduction(&[(*plants, *amount), (0, 0)])
            }
            ChoiceContext::MicrobeDiscount { count, amount, .. } => {
                note_reduction(&[(*count as i64, *amount), (0, 0)])
            }
            // Échanger une carte contre une (ou deux) : bon quand la main est
            // fournie. L'option 1 est « Ne rien défausser ».
            ChoiceContext::DiscardToDraw { .. } => {
                let fournie = match self.moi() {
                    Some(moi) => moi.main.len() > 2,
                    None => false,
                };
                let garder = [false, true];
                meilleur(n, |i| {
                    if garder[i.min(1)] == fournie {
                        0.0
                    } else {
                        1.0
                    }
                })
            }
            // Point de décision non reconnu : on lit le libellé. Jamais le
            // hasard. « Passer » n'est pas offert sur un choix contextuel — le
            // pont ne pose pas le drapeau —, le repli de l'original reste donc
            // sans emploi ici.
            _ => {
                let libelles: Vec<String> =
                    (0..n).map(|i| self.libelle_du_contexte(ctx, i)).collect();
                meilleur(n, |i| valeur_du_libelle(&libelles[i]))
            }
        }
    }

    fn choose_res_target(&mut self, _rng: &mut StdRng, _player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        // Poser la ressource sur la carte qui en tirera le plus : à défaut d'en
        // savoir plus, celle qui porte déjà des points.
        meilleur(candidates.len(), |i| {
            valeur_en_main(carte_option(self.db, candidates[i]))
        })
    }

    fn choose_res_source(&mut self, _rng: &mut StdRng, _player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        // En retirer une : sur la carte la moins précieuse.
        meilleur(candidates.len(), |i| {
            -valeur_en_main(carte_option(self.db, candidates[i]))
        })
    }

    fn pick_joker_tag(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _card: u16,
        _tag_counts: &[u32],
    ) -> usize {
        // Le badge dont on a déjà le plus : c'est la synergie la plus probable.
        // L'original le lit sur SON siège, dans la vue de l'état — pas sur
        // l'argument du moteur. Les deux disent la même chose (`player_view`
        // publie les badges dans l'ordre de `JOKER_TAG_CHOICES`) ; on garde la
        // lecture de l'original.
        let badges = match self.moi() {
            Some(moi) => moi.badges,
            None => [0u32; TAG_COUNT],
        };
        meilleur(JOKER_TAG_CHOICES.len(), |i| {
            badges.get(i).copied().unwrap_or(0) as f64
        })
    }

    fn research_keep(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        // On garde les MEILLEURES.
        les_meilleurs(drawn.len(), keep, |i| {
            valeur_en_main(carte_option(self.db, drawn[i]))
        })
    }

    fn reveal_pick(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        _filter: RevealFilter,
    ) -> Vec<usize> {
        // Le pont MONTRE toujours la révélation, même quand rien n'est prenable :
        // la question est posée avec `a_choisir: 0`, et la réponse est la liste
        // vide. On ne retombe donc pas sur le corps par défaut du trait, qui
        // sortirait sans rien demander.
        les_meilleurs(candidates.len(), keep, |i| {
            valeur_en_main(carte_option(self.db, candidates[i]))
        })
    }

    fn discard_down(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        // Le pont ne pose pas de question sans réponse possible.
        if hand.is_empty() || n == 0 {
            return Vec::new();
        }
        // On jette les MOINS bonnes.
        les_meilleurs(hand.len(), n, |i| {
            -valeur_en_main(carte_option(self.db, hand[i]))
        })
    }
}

/// La note des trois libellés du bonus de Construction, telle que l'original
/// l'écrit : poser une carte de plus vaut mieux que piocher ; piocher tout de
/// suite vaut mieux que décider après (la carte piochée devient posable).
fn note_construction_bonus(t: &str) -> f64 {
    if t.contains("supplémentaire") || t.contains("supplementaire") {
        return 3.0;
    }
    if t.contains("tout de suite") {
        return 2.0;
    }
    if t.starts_with("Piocher") {
        return 2.0;
    }
    0.0
}

/// La note des deux options d'une remise payée en ressources. `options` porte,
/// option par option, (ressources dépensées, MC de réduction).
fn note_reduction(options: &[(i64, i64)]) -> usize {
    meilleur(options.len(), |i| {
        let (ressources, reduction) = options[i];
        if ressources == 0 {
            0.5 // « Non : payer le prix plein »
        } else if reduction as f64 / ressources as f64 >= 4.0 {
            1.0
        } else {
            0.0
        }
    })
}

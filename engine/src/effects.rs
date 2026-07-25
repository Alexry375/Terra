//! Couche d'effets déclarative — lot 1 (chantier moteur-cartes-1).
//!
//! Chaque carte du lot est encodée par une entrée `(nom, CardEffects)` de la
//! table statique [`LOT1`] : prérequis (`Req`) vérifiés AVANT de proposer la
//! carte à la construction, effets (`Eff`) appliqués à la pose, productions
//! comptées par la phase de production existante (champs `*_prod`).
//! L'encodage est déclaratif : aucune logique par carte, uniquement des
//! données interprétées par `check_requirements` / `apply_on_build`
//! (appelées depuis `flow::build_card`, le même chemin pour `simulate`,
//! `--probe` et les tests).
//!
//! Sémantique des paliers de couleur (oracle Java `PlanetFactory` +
//! `Planet.isValidParameter`, journal B5) — bornes en NIVEAUX du moteur :
//! - température (0..=19) : P = 0-5, R = 6-10, Y = 11-15, W = 16-19 ;
//! - oxygène (0..=14) : P = 0-2, R = 3-6, Y = 7-11, W = 12-14 ;
//! - océans ouverts : P = 0-1, R = 2-3, Y = 4-6, W = 7-9.
//! « red or warmer » = min du palier R ; « red or colder » = max du palier R.
//!
//! Le texte imprimé (champ `description` de cards.json) fait foi ; conflits
//! avec le code Java au journal + lot1.md (B4 : Nitrogen-Rich Asteroid).

use crate::cards::Tag;

// Bornes de paliers (niveaux) — voir doc du module.
pub const TEMP_P_MAX: u8 = 5;
pub const TEMP_R_MIN: u8 = 6;
pub const TEMP_R_MAX: u8 = 10;
pub const TEMP_Y_MIN: u8 = 11;
pub const TEMP_W_MIN: u8 = 16;
pub const OXY_R_MIN: u8 = 3;
pub const OXY_Y_MIN: u8 = 7;
/// Palier BLANC d'oxygène (Birds : « Requires white oxygen »).
pub const OXY_W_MIN: u8 = 12;

/// Prérequis d'une carte (vérifiés avant la pose ; les `Spend*` exigent la
/// capacité de payer, la dépense elle-même est appliquée à la pose).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Req {
    /// Niveau de température courant >= n (paliers : voir doc du module).
    TempMin(u8),
    /// Niveau de température courant <= n.
    TempMax(u8),
    /// Niveau d'oxygène courant >= n.
    OxyMin(u8),
    /// Océans révélés >= n.
    OceanMin(u8),
    /// Océans révélés <= n (Dusty Quarry : « 3 or fewer ocean tiles »).
    OceanMax(u8),
    /// Au moins n tags du type donné en jeu (corporation incluse).
    Tags(Tag, u8),
    /// Dépense à la pose : n chaleur.
    SpendHeat(i64),
    /// Dépense à la pose : n plantes.
    SpendPlants(i64),
    /// Dépense à la pose : n TR.
    SpendTr(i64),
}

/// Effets appliqués à la pose. Les hausses de paramètres réutilisent les
/// fonctions du squelette (TR + caps sur l'instantané de phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eff {
    /// Gain (ou perte si négatif) immédiat de MC.
    Mc(i64),
    /// Gain immédiat de chaleur.
    Heat(i64),
    /// Gain immédiat de plantes.
    Plants(i64),
    /// Pioche immédiate de n cartes.
    Draw(u8),
    /// Hausse de production de MC.
    McProd(i64),
    /// Hausse de production de chaleur.
    HeatProd(i64),
    /// Hausse de production de plantes.
    PlantProd(i64),
    /// Hausse de production de cartes (pioche en phase de production).
    CardProd(i64),
    /// Température +n pas (TR par pas, cap instantané de phase).
    Temperature(u8),
    /// Oxygène +n pas.
    Oxygen(u8),
    /// Révèle n océans (bonus de tuile + TR).
    Ocean(u8),
    /// TR +n.
    Tr(u8),
    /// Infrastructure +n pas (par pas : +1 TR, pioche 1 carte — sémantique
    /// Java `increaseInfrastructure`, journal B2).
    Infrastructure(u8),
    /// Gain conditionnel de plantes si au moins n tags du type donné en jeu
    /// (Nitrogen-Rich Asteroid : le texte imprimé dit « 3 or more », le Java
    /// teste `== 3` — le texte gagne, journal B4).
    PlantsIfTags(Tag, u8, i64),
}

// ================================================================ lot 2 (A/B/C)
//
// Vocabulaire persistant du lot 2 (chantier moteur-cartes-2) : réductions de
// coût (A), déclencheurs de pose et globaux (B), actions de cartes bleues (C).
// Le texte imprimé fait foi (voir lot2.md pour les conflits, notamment
// Asteroid Mining : Java = revenu titane 2 → −6 MC Space ; imprimé = −6 MC Space
// encodé directement, sans modéliser le titane).

/// (A) Réduction de coût permanente accordée par une carte EN JEU aux cartes
/// jouées ensuite. Sommée par `flow::card_discount` (service unique).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reduction {
    /// Toute carte (Earth Catapult −2, Research Outpost −1).
    AnyCard(i64),
    /// Cartes portant le tag donné (Media Group : Event −5 ; Asteroid Mining :
    /// Space −6 ; Energy Subsidies : Energy −4 ; Interplanetary Conference :
    /// Earth −3 et Jupiter −3, cumulables).
    Tag(Tag, i64),
    /// (lot 3) Réduction CONDITIONNELLE et PAYANTE : retirer `count` ressources
    /// de type `kind` de la carte qui porte cette réduction pour payer `amount`
    /// MC de moins (Anaerobic Microorganisms : 2 microbes → −10 MC).
    ///
    /// Elle ne fait PAS partie de la somme fixe de `flow::card_discount` (elle
    /// dépend des ressources posées et d'une décision du joueur) : elle est
    /// servie par `flow::microbe_discount` et consommée par `flow::affordable`
    /// (montant potentiel) puis `flow::build_card_with` (décision + retrait).
    PayResources { kind: ResKind, count: u32, amount: i64 },
}

impl Reduction {
    /// Réduction FIXE applicable à une carte de tags donnés. Les réductions
    /// conditionnelles (`PayResources`) valent 0 ici : elles ont leur propre
    /// chemin, elles ne sont jamais accordées gratuitement.
    pub fn amount_for(self, tags: &[Tag]) -> i64 {
        match self {
            Reduction::AnyCard(n) => n,
            Reduction::Tag(t, n) => {
                if tags.contains(&t) {
                    n
                } else {
                    0
                }
            }
            Reduction::PayResources { .. } => 0,
        }
    }
}

// ============================================ lot 3 : ressources sur les cartes
//
// Vocabulaire des jetons microbe / animal / science empilés sur une carte en
// jeu. Il est DÉCLARATIF : `flow` interprète ces données, il n'existe aucune
// exception par carte. Toute pose et tout retrait passent par le service unique
// `flow::add_resources` / `flow::remove_resources`.

/// Type de ressource qu'une carte peut porter (Java `CardCollectableResource`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResKind {
    Microbe,
    Animal,
    Science,
}

impl ResKind {
    /// Nom utilisé par la sonde (`resources[].kind`).
    pub fn name(self) -> &'static str {
        match self {
            ResKind::Microbe => "microbe",
            ResKind::Animal => "animal",
            ResKind::Science => "science",
        }
    }
}

/// Quelle carte reçoit la ressource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResTarget {
    /// La carte qui porte l'effet elle-même (aucun choix à faire).
    SelfCard,
    /// « ANOTHER card » : une autre carte porteuse que celle qui porte l'effet.
    Another,
    /// « ANY card » : n'importe quelle carte porteuse, la carte qui porte
    /// l'effet comprise (Large Convoy, CEO's Favorite Project — leur texte ne
    /// dit PAS « ANOTHER »).
    Any,
}

/// Combien de ressources sont posées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResAmount {
    Fixed(u32),
    /// Quantité dépendant du type porté par la CIBLE : `microbe` si la cible
    /// porte des microbes, `other` sinon (Imported Hydrogen et Cryogenic
    /// Shipment : « 3 microbes ou 2 animaux »).
    ByKind { microbe: u32, other: u32 },
}

/// Pose de ressources sur une carte à choisir parmi les porteuses acceptées.
#[derive(Debug, Clone, Copy)]
pub struct ResPut {
    pub target: ResTarget,
    /// Types de carte porteuse acceptés comme cible.
    pub kinds: &'static [ResKind],
    pub amount: ResAmount,
}

/// Effet élémentaire du vocabulaire « ressources ».
#[derive(Debug, Clone, Copy)]
pub enum ResEff {
    /// Gain classique du lot 1 (plantes, pioche, océan, TR…), pour exprimer
    /// l'ORDRE DU TEXTE IMPRIMÉ à l'intérieur d'une branche.
    Gain(Eff),
    /// Pose de ressources.
    Put(ResPut),
    /// Retire n ressources de la carte qui porte l'effet.
    RemoveSelf(u32),
    /// Retire n ressources d'une carte AU CHOIX du joueur, parmi les porteuses
    /// des types donnés (Decomposing Fungus : 1 animal OU 1 microbe).
    RemoveAny(&'static [ResKind], u32),
    /// « Améliore une carte Phase » : mécanisme d'un lot ultérieur. L'effet est
    /// SAUTÉ et compté dans `phase_upgrades_skipped` — aucune compensation.
    PhaseUpgrade,
}

/// Étape d'un effet à ressources : soit un effet direct, soit une alternative.
#[derive(Debug, Clone, Copy)]
pub enum ResStep {
    Do(ResEff),
    /// Alternative « … ou … ». Les branches sont numérotées **dans l'ordre du
    /// texte imprimé** ; les branches impossibles sont filtrées AVANT de
    /// présenter le choix à `Policy::choose_option`.
    Choose(&'static [&'static [ResEff]]),
}

/// Condition d'un déclencheur de pose : quelles cartes jouées le déclenchent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrigCond {
    /// N'importe quelle carte (Anti-Gravity Technology).
    AnyCard,
    /// Carte portant au moins un tag donné.
    Tag(Tag),
    /// Carte portant au moins un des deux tags (Interplanetary : Earth ou Jupiter).
    EitherTag(Tag, Tag),
    /// (lot 3) Carte portant au moins un tag de la liste (Ecological Zone :
    /// Animal ou Plante ; Anaerobic/Decomposers/Viral Enhancers : Animal,
    /// Microbe ou Plante).
    AnyOfTags(&'static [Tag]),
}

impl TrigCond {
    /// Nombre de tags de la carte posée qui satisfont la condition (0 = pas de
    /// déclenchement). Sémantique Java `countCardTags`.
    pub fn matched_tags(self, tags: &[Tag]) -> u32 {
        match self {
            TrigCond::AnyCard => 1,
            TrigCond::Tag(t) => tags.iter().filter(|&&x| x == t).count() as u32,
            TrigCond::EitherTag(a, b) => {
                tags.iter().filter(|&&x| x == a || x == b).count() as u32
            }
            TrigCond::AnyOfTags(list) => {
                tags.iter().filter(|x| list.contains(x)).count() as u32
            }
        }
    }
}

/// Gain élémentaire d'un déclencheur.
#[derive(Debug, Clone, Copy)]
pub enum TrigGain {
    Heat(i64),
    Plants(i64),
    Draw(u8),
    /// (lot 3) Ajoute n ressources sur la carte SOURCE du déclencheur
    /// (Fish, Livestock, Small Animals, Herbivores, Physics Complex,
    /// Ecological Zone, Anaerobic Microorganisms).
    ResSelf(u32),
    /// (lot 3) Alternative offerte au joueur (Viral Enhancers, Decomposers).
    /// Appliquée UNE fois par déclenchement : elle n'est jamais multipliée par
    /// le nombre de tags (les deux cartes concernées sont au forfait).
    Choose(&'static [&'static [ResEff]]),
}

/// (B) Déclencheur de pose : « When you play … ». Évalué à la pose d'une carte,
/// pour toutes les cartes persistantes en jeu du joueur.
#[derive(Debug, Clone, Copy)]
pub struct PlayTrigger {
    pub cond: TrigCond,
    pub gains: &'static [TrigGain],
    /// true = gains multipliés par le nombre de tags satisfaisants de la carte
    /// posée (Java `countCardTags` : Olympus/Energy Subsidies/Impact Analysis/
    /// Interplanetary) ; false = forfait si ≥1 tag (Optimal Aerobraking/Recycled/
    /// Anti-Gravity).
    pub scale_by_matched_tags: bool,
    /// true = la carte se déclenche sur sa propre pose (« including this ») —
    /// Java `onBuiltEffectApplicableToItself` (défaut false).
    pub include_self: bool,
}

/// (B) Déclencheur global : « When you raise the temperature / flip an ocean ».
/// Fixé aux hausses de paramètres du joueur agissant.
#[derive(Debug, Clone, Copy)]
pub enum GlobalTrigger {
    OnRaiseTemperature(&'static [TrigGain]),
    OnFlipOcean(&'static [TrigGain]),
    /// (lot 3) « When you raise oxygen » (Herbivores). Se déclenche aussi sur
    /// la hausse d'oxygène d'une construction de forêt, comme le Java
    /// (`onOxygenChangedEffect`).
    OnRaiseOxygen(&'static [TrigGain]),
    /// (lot 3) « When you build a forest » (Small Animals, Java
    /// `onForestBuiltEffect`).
    OnBuildForest(&'static [TrigGain]),
}

/// Coût d'activation d'une action de carte bleue (payé une fois par activation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCost {
    Heat(i64),
    Mc(i64),
    Plants(i64),
}

/// Effet d'une action de carte bleue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEff {
    Draw(u8),
    Plants(i64),
    Mc(i64),
    Tr(u8),
    Oxygen(u8),
}

/// (C) Action d'une carte bleue, activable une fois par phase III.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// Coût fixe puis effet fixe.
    Fixed {
        cost: &'static [ActionCost],
        effect: &'static [ActionEff],
    },
    /// Power Infrastructure : dépenser X chaleur (0..=chaleur) → X MC.
    HeatToMc,
    /// Redrafted Contracts : défausser jusqu'à n cartes → piocher autant.
    DiscardDraw(i64),
    /// Volcanic Pools : payer max(0, base − nb de tags `per_tag` en jeu) MC →
    /// flip un océan (base 12, per_tag Energy).
    FlipOceanTagDiscount { base: i64, per_tag: Tag },
    /// Developed Infrastructure : payer base − (`reduction` si ≥ `threshold`
    /// cartes bleues en jeu) MC → hausse de température 1 pas (base 10,
    /// threshold 5, reduction 5).
    RaiseTempBlueDiscount { base: i64, threshold: u32, reduction: i64 },
    /// (lot 3) Action à ressources : une alternative dont les branches sont
    /// numérotées dans l'ordre du texte imprimé (une seule branche = pas de
    /// choix). Tardigrades, Birds, Nitrite Reducting Bacteria, Fibrous
    /// Composite Material, Decomposing Fungus, GHG Production Bacteria,
    /// Regolith Eaters, et — depuis l'ADDENDUM round 2, qui les a reclassées
    /// d'après le scan des cartes imprimées — Symbiotic Fungus, Extreme-Cold
    /// Fungus et Conserved Biome.
    Res(&'static [&'static [ResEff]]),
}

/// Encodage complet d'une carte du lot.
#[derive(Debug)]
pub struct CardEffects {
    pub reqs: &'static [Req],
    pub effects: &'static [Eff],
    /// (A) réductions offertes aux poses suivantes (lot 2).
    pub reductions: &'static [Reduction],
    /// (B) déclencheurs de pose (lot 2).
    pub play_triggers: &'static [PlayTrigger],
    /// (B) déclencheurs globaux (lot 2).
    pub global_triggers: &'static [GlobalTrigger],
    /// (C) action de carte bleue (lot 2).
    pub action: Option<Action>,
    /// (lot 3) Type de ressource porté par la carte. Une carte porteuse est
    /// INITIALISÉE À 0 à sa pose : elle devient cible valide même vide (règle
    /// du jeu, oracle Java `Player.initResources`).
    pub holds: Option<ResKind>,
    /// (lot 3) Effets à ressources appliqués à la POSE, dans l'ordre du texte
    /// imprimé (après `effects`, avant les déclencheurs de pose).
    pub on_build: &'static [ResStep],
}

/// Cherche l'encodage d'une carte par nom exact. None = carte hors lot (stub).
pub fn lookup(name: &str) -> Option<&'static CardEffects> {
    LOT1.iter().find(|(n, _)| *n == name).map(|(_, e)| e)
}

macro_rules! card {
    // Forme lot 1 : reqs + effects seulement (champs lot 2 vides).
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*]) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[], play_triggers: &[], global_triggers: &[], action: None,
            holds: None, on_build: &[],
        })
    };
    // Forme lot 2 : réductions / déclencheurs / action.
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*],
     red: [$($rd:expr),*], ptrig: [$($pt:expr),*], gtrig: [$($gt:expr),*],
     action: $act:expr) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[$($rd),*], play_triggers: &[$($pt),*],
            global_triggers: &[$($gt),*], action: $act,
            holds: None, on_build: &[],
        })
    };
    // Forme lot 3 : tous les champs explicites (ressources posées comprises).
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*],
     red: [$($rd:expr),*], ptrig: [$($pt:expr),*], gtrig: [$($gt:expr),*],
     action: $act:expr, holds: $h:expr, on_build: [$($ob:expr),*]) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[$($rd),*], play_triggers: &[$($pt),*],
            global_triggers: &[$($gt),*], action: $act,
            holds: $h, on_build: &[$($ob),*],
        })
    };
}

use Eff::*;
use Req::*;

// Raccourcis de lecture pour la table (lot 3) : types de porteuse acceptés
// comme cible, et ensembles de tags des déclencheurs « Animal/Microbe/Plante ».
const K_MICROBE: &[ResKind] = &[ResKind::Microbe];
const K_ANIMAL: &[ResKind] = &[ResKind::Animal];
const K_MICROBE_ANIMAL: &[ResKind] = &[ResKind::Microbe, ResKind::Animal];
/// « a card that holds resources » : n'importe quel type porté.
const K_ANY: &[ResKind] = &[ResKind::Microbe, ResKind::Animal, ResKind::Science];
const T_ANIMAL_PLANT: &[Tag] = &[Tag::Animal, Tag::Plant];
const T_ANIMAL_MICROBE_PLANT: &[Tag] = &[Tag::Animal, Tag::Microbe, Tag::Plant];

/// Pose de `n` ressources sur la carte elle-même.
const fn put_self(n: u32) -> ResEff {
    ResEff::Put(ResPut {
        target: ResTarget::SelfCard,
        kinds: K_ANY,
        amount: ResAmount::Fixed(n),
    })
}

/// Pose de `n` ressources sur une AUTRE carte portant l'un de `kinds`.
const fn put_another(kinds: &'static [ResKind], n: u32) -> ResEff {
    ResEff::Put(ResPut {
        target: ResTarget::Another,
        kinds,
        amount: ResAmount::Fixed(n),
    })
}

/// Table des cartes aux effets COMPLETS et fidèles au texte imprimé : lot 1
/// (63 cartes, chantier cartes-1) SUIVI du lot 2 (47 cartes, chantier
/// cartes-2 : réductions/déclencheurs/actions) — 110 entrées. Nom historique
/// `LOT1` conservé (référencé par les tests et le garde-fou de chargement).
/// Correspondances carte → classe Java → conflits : `outputs/lot1.md` (lot 1)
/// et `outputs/lot2.md` (lot 2).
pub static LOT1: &[(&str, CardEffects)] = &[
    // ------------------------------------------------- les 10 cartes imposées
    card!("Comet", reqs: [], effects: [Temperature(1), Ocean(1)]),
    card!("Farming", reqs: [TempMin(TEMP_W_MIN)],
          effects: [Plants(2), McProd(2), PlantProd(2)]),
    card!("Lichen", reqs: [], effects: [PlantProd(1)]),
    card!("Deep Well Heating", reqs: [], effects: [Temperature(1), HeatProd(1)]),
    card!("Interstellar Colony Ship", reqs: [Tags(Tag::Science, 4)], effects: []),
    card!("Algae", reqs: [OceanMin(5)], effects: [PlantProd(2)]),
    card!("Bushes", reqs: [TempMin(TEMP_R_MIN)], effects: [Plants(2), PlantProd(2)]),
    card!("Acquired Company", reqs: [], effects: [CardProd(1)]),
    card!("Lunar Beam", reqs: [SpendTr(1)], effects: [HeatProd(4)]),
    card!("Grain Silos", reqs: [], effects: [Infrastructure(2), Plants(4)]),
    // ------------------------------------------------------- vertes (37)
    card!("Adapted Lichen", reqs: [], effects: [PlantProd(1)]),
    card!("Aerated Magma", reqs: [OxyMin(OXY_R_MIN)],
          effects: [CardProd(1), HeatProd(2)]),
    card!("Airborne Radiation", reqs: [OxyMin(OXY_R_MIN)],
          effects: [Oxygen(1), HeatProd(2)]),
    card!("Archaebacteria", reqs: [TempMax(TEMP_P_MAX)], effects: [PlantProd(1)]),
    card!("Artificial Photosynthesis", reqs: [],
          effects: [PlantProd(1), HeatProd(1)]),
    card!("Balanced Portfolios", reqs: [SpendTr(1)], effects: [McProd(3)]),
    card!("Biomass Combustors", reqs: [SpendPlants(2)], effects: [HeatProd(5)]),
    card!("Blueprints", reqs: [], effects: [CardProd(1), McProd(2)]),
    card!("Coal Imports", reqs: [], effects: [HeatProd(3)]),
    card!("Commercial District", reqs: [], effects: [McProd(4)]),
    card!("Dandelions", reqs: [TempMin(TEMP_R_MIN)],
          effects: [CardProd(1), PlantProd(1)]),
    card!("Designed Microorganisms", reqs: [TempMax(TEMP_R_MAX)],
          effects: [PlantProd(2)]),
    card!("Diversified Interests", reqs: [],
          effects: [Plants(3), Heat(3), PlantProd(1)]),
    card!("Economic Growth", reqs: [], effects: [McProd(3)]),
    card!("Food Factory", reqs: [SpendPlants(2)], effects: [McProd(4)]),
    card!("Fueled Generators", reqs: [SpendTr(1)], effects: [HeatProd(2)]),
    card!("Fusion Power", reqs: [Tags(Tag::Energy, 2)], effects: [CardProd(1)]),
    card!("Gene Repair", reqs: [Tags(Tag::Science, 3)], effects: [McProd(2)]),
    card!("Geothermal Power", reqs: [], effects: [HeatProd(2)]),
    card!("Grass", reqs: [TempMin(TEMP_R_MIN)], effects: [Plants(3), PlantProd(1)]),
    card!("Great Dam", reqs: [OceanMin(2)], effects: [HeatProd(2)]),
    card!("Heather", reqs: [], effects: [Plants(1), PlantProd(1)]),
    card!("Imported GHG", reqs: [], effects: [Heat(5), HeatProd(1)]),
    card!("Industrial Farming", reqs: [], effects: [McProd(1), PlantProd(2)]),
    card!("Kelp Farming", reqs: [OceanMin(6)],
          effects: [Plants(2), McProd(2), PlantProd(3)]),
    card!("Mohole Area", reqs: [], effects: [HeatProd(4)]),
    card!("Monocultures", reqs: [SpendTr(1)], effects: [PlantProd(2)]),
    card!("Moss", reqs: [OceanMin(3), SpendPlants(1)], effects: [PlantProd(1)]),
    card!("Smelting", reqs: [], effects: [Draw(2), HeatProd(5)]),
    card!("Soil Warming", reqs: [], effects: [Temperature(1), PlantProd(2)]),
    card!("Solar Trapping", reqs: [], effects: [Draw(1), Heat(3), HeatProd(1)]),
    card!("Space Heater", reqs: [], effects: [Draw(1), HeatProd(2)]),
    card!("Sponsors", reqs: [], effects: [McProd(2)]),
    card!("Trees", reqs: [TempMin(TEMP_Y_MIN)], effects: [Plants(1), PlantProd(3)]),
    card!("Tropical Resort", reqs: [SpendHeat(5)], effects: [McProd(4)]),
    card!("Tundra Farming", reqs: [TempMin(TEMP_Y_MIN)],
          effects: [Plants(1), McProd(2), PlantProd(1)]),
    card!("Wave Power", reqs: [OceanMin(3)], effects: [HeatProd(3)]),
    // ------------------------------------------------------- rouges (16)
    card!("Artificial Lake", reqs: [TempMin(TEMP_Y_MIN)], effects: [Ocean(1)]),
    card!("Atmosphere Filtering", reqs: [Tags(Tag::Science, 2)],
          effects: [Oxygen(1)]),
    card!("Breathing Filters", reqs: [OxyMin(OXY_Y_MIN)], effects: []),
    card!("Bribed Comittee", reqs: [], effects: [Tr(2)]),
    card!("Convoy from Europa", reqs: [], effects: [Draw(1), Ocean(1)]),
    card!("Crater", reqs: [Tags(Tag::Event, 3)], effects: [Ocean(1)]),
    card!("Deimos Down", reqs: [], effects: [Temperature(3), Mc(7)]),
    card!("Giant Ice Asteroid", reqs: [], effects: [Temperature(2), Ocean(2)]),
    card!("Ice Asteroid", reqs: [], effects: [Ocean(2)]),
    card!("Investment Loan", reqs: [SpendTr(1)], effects: [Mc(10)]),
    card!("Lava Flows", reqs: [], effects: [Temperature(2)]),
    card!("Nitrogen-Rich Asteroid", reqs: [],
          effects: [Tr(2), Temperature(1), Plants(2),
                    PlantsIfTags(Tag::Plant, 3, 4)]),
    card!("Release of Inert Gases", reqs: [], effects: [Tr(2)]),
    card!("Research", reqs: [], effects: [Draw(2)]),
    card!("Subterranean Reservoir", reqs: [], effects: [Ocean(1)]),
    card!("Towing a Comet", reqs: [], effects: [Oxygen(1), Ocean(1), Plants(2)]),

    // ================================================= LOT 2 (chantier cartes-2)
    // Réductions (A), déclencheurs (B), actions bleues (C). Texte imprimé fait
    // foi ; correspondances et conflits dans outputs/lot2.md.

    // ---- 10 imposées --------------------------------------------------------
    // (A) « When you play a card, you pay 2 MC less for it. »
    card!("Earth Catapult", reqs: [], effects: [],
          red: [Reduction::AnyCard(2)], ptrig: [], gtrig: [], action: None),
    // (A) « …pay 1 MC less for it. »
    card!("Research Outpost", reqs: [], effects: [],
          red: [Reduction::AnyCard(1)], ptrig: [], gtrig: [], action: None),
    // (A) « When you play an Event, you pay 5 MC less for it. »
    card!("Media Group", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Event, 5)], ptrig: [], gtrig: [], action: None),
    // (A) « When you play a Space, you pay 6 MC less for it. » (Java = titane 2 →
    // −6 Space ; imprimé encodé directement.)
    card!("Asteroid Mining", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 6)], ptrig: [], gtrig: [], action: None),
    // (A+B) « When you play an Energy tag, you pay 4 MC less for it and you draw
    // a card. » (draw = nb de tags Energy de la carte posée, Java countCardTags.)
    card!("Energy Subsidies", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Energy, 4)],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Energy),
                    gains: &[TrigGain::Draw(1)], scale_by_matched_tags: true,
                    include_self: false }],
          gtrig: [], action: None),
    // (C) « Action: Spend 2 heat to draw a card. »
    card!("Development Center", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Heat(2)],
                    effect: &[ActionEff::Draw(1)] })),
    // (C) « Action: Spend 1 MC to gain 2 plants. »
    card!("Farmers Market", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Mc(1)],
                    effect: &[ActionEff::Plants(2)] })),
    // (C) « Requires yellow temperature or warmer. Action: Spend 8 heat to raise
    // your TR 1 step. » (le prérequis jaune est à la POSE ; l'action ne teste que
    // la chaleur, Java CaretakerContractActionValidator.)
    card!("Caretaker Contract", reqs: [TempMin(TEMP_Y_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Heat(8)],
                    effect: &[ActionEff::Tr(1)] })),
    // (B) « When you play an Event tag, you gain 2 heat and 2 plants. » (forfait.)
    card!("Optimal Aerobraking", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Event),
                    gains: &[TrigGain::Heat(2), TrigGain::Plants(2)],
                    scale_by_matched_tags: false, include_self: false }],
          gtrig: [], action: None),
    // (B) « When you play a Science tag, including this, draw a card. » (draw = nb
    // de tags Science de la carte posée ; include_self = true.)
    card!("Olympus Conference", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Science),
                    gains: &[TrigGain::Draw(1)], scale_by_matched_tags: true,
                    include_self: true }],
          gtrig: [], action: None),

    // ---- A : réductions supplémentaires -------------------------------------
    card!("Asteroid Mining Consortium", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    card!("Electric Arc Furnaces", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Building, 4)], ptrig: [], gtrig: [], action: None),
    card!("Great Escarpment Consortium", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    card!("Mine", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Building, 4)], ptrig: [], gtrig: [], action: None),
    card!("Space Station", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    // « When you play a Space, you pay 3 less » (tag imprimé BUILDING sans effet).
    card!("Titanium Mine", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    card!("Vesta Shipyard", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    card!("Ganymede Shipyard", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 6)], ptrig: [], gtrig: [], action: None),
    card!("Ilmenite Deposits", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 6)], ptrig: [], gtrig: [], action: None),
    // Deux réductions distinctes (Building −2 ET Space −3).
    card!("Surface Mines", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Building, 2), Reduction::Tag(Tag::Space, 3)],
          ptrig: [], gtrig: [], action: None),
    // Production + réduction.
    card!("Industrial Center", reqs: [], effects: [McProd(3)],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    card!("Industrial Microbes", reqs: [], effects: [HeatProd(1)],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    card!("Underground City", reqs: [], effects: [McProd(1)],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    card!("Micro-Mills", reqs: [], effects: [HeatProd(1)],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    // Dépense à la pose + réduction.
    card!("Building Industries", reqs: [SpendHeat(4)], effects: [],
          red: [Reduction::Tag(Tag::Building, 4)], ptrig: [], gtrig: [], action: None),
    card!("Fuel Factory", reqs: [SpendHeat(3)], effects: [McProd(1)],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    card!("Strip Mine", reqs: [SpendTr(1)], effects: [],
          red: [Reduction::Tag(Tag::Building, 4), Reduction::Tag(Tag::Space, 3)],
          ptrig: [], gtrig: [], action: None),
    // Production + réduction ; « 1 VP per Jupiter tag » = vp_dynamic du JSON.
    card!("Io Mining Industries", reqs: [], effects: [McProd(2)],
          red: [Reduction::Tag(Tag::Space, 6)], ptrig: [], gtrig: [], action: None),
    card!("Mass Converter", reqs: [Tags(Tag::Science, 4)], effects: [HeatProd(3)],
          red: [Reduction::Tag(Tag::Space, 3)], ptrig: [], gtrig: [], action: None),
    // « Requires 3 or fewer ocean tiles » → OceanMax(3).
    card!("Dusty Quarry", reqs: [OceanMax(3)], effects: [],
          red: [Reduction::Tag(Tag::Building, 2)], ptrig: [], gtrig: [], action: None),
    // (A+B) « Earth or Jupiter tag, excluding this, pay 3 MC less and draw a card. »
    card!("Interplanetary Conference", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Earth, 3), Reduction::Tag(Tag::Jupiter, 3)],
          ptrig: [PlayTrigger { cond: TrigCond::EitherTag(Tag::Earth, Tag::Jupiter),
                    gains: &[TrigGain::Draw(1)], scale_by_matched_tags: true,
                    include_self: false }],
          gtrig: [], action: None),

    // ---- B : déclencheurs supplémentaires -----------------------------------
    // « Requires 5 Science. When you play a card, gain 2 heat and 2 plants. »
    card!("Anti-Gravity Technology", reqs: [Tags(Tag::Science, 5)], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::AnyCard,
                    gains: &[TrigGain::Heat(2), TrigGain::Plants(2)],
                    scale_by_matched_tags: false, include_self: false }],
          gtrig: [], action: None),
    // « When you play an Event tag, draw a card. » (draw = nb tags Event.)
    card!("Impact Analysis", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Event),
                    gains: &[TrigGain::Draw(1)], scale_by_matched_tags: true,
                    include_self: false }],
          gtrig: [], action: None),
    // « When you play an Event, draw 2 cards. » (forfait.)
    card!("Recycled Detritus", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Event),
                    gains: &[TrigGain::Draw(2)], scale_by_matched_tags: false,
                    include_self: false }],
          gtrig: [], action: None),
    // « When you raise the temperature, gain 2 plants. »
    card!("Volcanic Soil", reqs: [], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnRaiseTemperature(&[TrigGain::Plants(2)])],
          action: None),
    // « Requires red temperature or warmer. When you flip an ocean tile, gain 4 plants. »
    card!("Arctic Algae", reqs: [TempMin(TEMP_R_MIN)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnFlipOcean(&[TrigGain::Plants(4)])],
          action: None),

    // ---- C : actions bleues supplémentaires ---------------------------------
    card!("Circuit Board Factory", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[], effect: &[ActionEff::Draw(1)] })),
    card!("Matter Manufactoring", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Mc(1)],
                    effect: &[ActionEff::Draw(1)] })),
    card!("Artificial Jungle", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Plants(1)],
                    effect: &[ActionEff::Draw(1)] })),
    card!("Ironworks", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Heat(4)],
                    effect: &[ActionEff::Oxygen(1)] })),
    card!("Steelworks", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Heat(6)],
                    effect: &[ActionEff::Mc(2), ActionEff::Oxygen(1)] })),
    // « Requires 5 Science. Action: Draw 2 cards. »
    card!("Ai Central", reqs: [Tags(Tag::Science, 5)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[], effect: &[ActionEff::Draw(2)] })),
    // « Action: Spend 2 MC to draw a card. » (« 1 VP per 3 blue cards » = vp_dynamic.)
    card!("Think Tank", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Mc(2)],
                    effect: &[ActionEff::Draw(1)] })),
    // « Action: Spend any amount of heat to gain that amount of MC. »
    card!("Power Infrastructure", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [], action: Some(Action::HeatToMc)),
    // « Action: Discard up to three cards in hand. Draw that many cards. »
    card!("Redrafted Contracts", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [], action: Some(Action::DiscardDraw(3))),
    // « Action: Spend 12 MC to flip an ocean tile. Reduce this by 1 MC per Energy
    // tag you have. » (Java countPlayedTags(ENERGY), max(0, 12−n).)
    card!("Volcanic Pools", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::FlipOceanTagDiscount { base: 12, per_tag: Tag::Energy })),
    // « Action: Spend 10 MC to raise the temperature 1 step. Reduce this by 5 MC
    // if you have 5 or more blue cards in play. »
    card!("Developed Infrastructure", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::RaiseTempBlueDiscount { base: 10, threshold: 5, reduction: 5 })),

    // ================================================= LOT 3 (chantier cartes-3)
    // Ressources posées sur les cartes (microbe / animal / science) : 28 cartes.
    // Correspondances carte → classe Java, encodage et conflits : outputs/lot3.md.
    // Le texte imprimé (`description` de cards.json) fait foi ; les classes
    // `Buffed…` sont des variantes maison, JAMAIS la source (voir journal D1).

    // ---- A. Conteneurs : cartes qui PORTENT des ressources (14) -------------

    // « Action: Add 1 microbe to this card. 1 VP per 3 microbes on this card. »
    card!("Tardigrades", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[&[put_self(1)]])),
          holds: Some(ResKind::Microbe), on_build: []),
    // « Requires white oxygen. Add an animal to this card. 1 VP per animal. »
    // (l'ajout est l'ACTION de la carte — Java `CardAction.ADD_ANIMAL` ;
    // `buildProject` n'appelle que `initResources`, d'où 0 animal à la pose.)
    card!("Birds", reqs: [OxyMin(OXY_W_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[&[put_self(1)]])),
          holds: Some(ResKind::Animal), on_build: []),
    // « Add 3 microbes to this card. Action: Add 1 microbe to this card or
    //   remove 3 microbes to flip an ocean tile. »
    card!("Nitrite Reducting Bacteria", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_self(1)],
              &[ResEff::RemoveSelf(3), ResEff::Gain(Ocean(1))],
          ])),
          holds: Some(ResKind::Microbe), on_build: [ResStep::Do(put_self(3))]),
    // « Add 3 science resources to this card. Action: Add 1 science to this card
    //   or remove 3 science to upgrade a phase. » (amélioration non gérée : D8.)
    card!("Fibrous Composite Material", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_self(1)],
              &[ResEff::RemoveSelf(3), ResEff::PhaseUpgrade],
          ])),
          holds: Some(ResKind::Science), on_build: [ResStep::Do(put_self(3))]),
    // « Place 2 microbes on this card. Action: Remove 1 animal or 1 microbe from
    //   one of your cards to gain 3 plants. »
    card!("Decomposing Fungus", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[&[
              ResEff::RemoveAny(K_MICROBE_ANIMAL, 1),
              ResEff::Gain(Plants(3)),
          ]])),
          holds: Some(ResKind::Microbe), on_build: [ResStep::Do(put_self(2))]),
    // « Requires red oxygen or higher. Action: Add 1 microbe to this card, or
    //   remove 2 microbes to raise the temperature 1 step. »
    card!("GHG Production Bacteria", reqs: [OxyMin(OXY_R_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_self(1)],
              &[ResEff::RemoveSelf(2), ResEff::Gain(Temperature(1))],
          ])),
          holds: Some(ResKind::Microbe), on_build: []),
    // « Requires red temperature or warmer. Action: Add 1 microbe to this card,
    //   or remove 2 microbes from this card to raise oxygen 1 step. »
    card!("Regolith Eaters", reqs: [TempMin(TEMP_R_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_self(1)],
              &[ResEff::RemoveSelf(2), ResEff::Gain(Oxygen(1))],
          ])),
          holds: Some(ResKind::Microbe), on_build: []),
    // « Requires red temperature or warmer. When you flip an ocean tile, add
    //   1 animal to this card. 1 VP per animal on this card. »
    card!("Fish", reqs: [TempMin(TEMP_R_MIN)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnFlipOcean(&[TrigGain::ResSelf(1)])],
          action: None, holds: Some(ResKind::Animal), on_build: []),
    // « Requires yellow oxygen or higher. When you raise the temperature, add
    //   1 animal to this card. 1 VP per animal on this card. »
    card!("Livestock", reqs: [OxyMin(OXY_Y_MIN)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnRaiseTemperature(&[TrigGain::ResSelf(1)])],
          action: None, holds: Some(ResKind::Animal), on_build: []),
    // « Requires red temperature or warmer. When you build a forest, add
    //   1 animal to this card. 1 VP per 2 animals on this card. »
    card!("Small Animals", reqs: [TempMin(TEMP_R_MIN)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnBuildForest(&[TrigGain::ResSelf(1)])],
          action: None, holds: Some(ResKind::Animal), on_build: []),
    // « Requires 5 oceans to be flipped. When you raise oxygen, flip an ocean
    //   tile, or raise temperature, add 1 animal to this card. » (trois
    //   déclencheurs distincts, Java onOxygen/onOcean/onTemperature.)
    card!("Herbivores", reqs: [OceanMin(5)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnRaiseOxygen(&[TrigGain::ResSelf(1)]),
                  GlobalTrigger::OnFlipOcean(&[TrigGain::ResSelf(1)]),
                  GlobalTrigger::OnRaiseTemperature(&[TrigGain::ResSelf(1)])],
          action: None, holds: Some(ResKind::Animal), on_build: []),
    // « Requires 4 Science tags. When you raise the temperature, add 1 science
    //   resource to this card. 1 VP per 2 science res on this card. »
    card!("Physics Complex", reqs: [Tags(Tag::Science, 4)], effects: [],
          red: [], ptrig: [],
          gtrig: [GlobalTrigger::OnRaiseTemperature(&[TrigGain::ResSelf(1)])],
          action: None, holds: Some(ResKind::Science), on_build: []),
    // « When you play a Animal or Plant, including these, add an animal to this
    //   card. » (Java `countCardTags` → +1 PAR tag concerné ; ses propres tags
    //   PLANT+ANIMAL lui donnent donc 2 animaux à sa pose.)
    card!("Ecological Zone", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::AnyOfTags(T_ANIMAL_PLANT),
                    gains: &[TrigGain::ResSelf(1)], scale_by_matched_tags: true,
                    include_self: true }],
          gtrig: [], action: None,
          holds: Some(ResKind::Animal), on_build: []),
    // « When you play an Animal, Microbe, or Plant, including this, add a
    //   microbe to this card. When you play a card, you may remove 2 microbes
    //   from this card to pay 10 MC less for that card. » (réduction : D7.)
    card!("Anaerobic Microorganisms", reqs: [], effects: [],
          red: [Reduction::PayResources { kind: ResKind::Microbe, count: 2, amount: 10 }],
          ptrig: [PlayTrigger { cond: TrigCond::AnyOfTags(T_ANIMAL_MICROBE_PLANT),
                    gains: &[TrigGain::ResSelf(1)], scale_by_matched_tags: true,
                    include_self: true }],
          gtrig: [], action: None,
          holds: Some(ResKind::Microbe), on_build: []),

    // ---- B. Cartes qui posent des ressources ailleurs, sans en porter (14) --

    // « Requires red temperature or warmer. ACTION: Add a microbe to ANOTHER*
    //   card. » — ADDENDUM round 2 : le scan de la carte imprimée porte bien
    //   « Action: », comme la classe Java (`SymbioticFungudActionProcessor`,
    //   `isActiveCard() == true`, aucun `buildProject`). Effet de pose supprimé.
    card!("Symbiotic Fungus", reqs: [TempMin(TEMP_R_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[&[put_another(K_MICROBE, 1)]])),
          holds: None, on_build: []),
    // « Requires purple temperature. ACTION: Gain 1 plant OR add a microbe to
    //   ANOTHER* card. » — ADDENDUM round 2 (scan) : action, pas pose. Branche 0
    //   = plantes, branche 1 = microbe : ordre du texte imprimé.
    card!("Extreme-Cold Fungus", reqs: [TempMax(TEMP_P_MAX)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[ResEff::Gain(Plants(1))],
              &[put_another(K_MICROBE, 1)],
          ])),
          holds: None, on_build: []),
    // « ACTION: Add a microbe to ANOTHER* card OR add an animal to ANOTHER*
    //   card. 1 VP per 2 forests you have. » — ADDENDUM round 2 (scan) : action,
    //   pas pose. Les VP viennent du vp_dynamic FOREST du JSON.
    card!("Conserved Biome", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_another(K_MICROBE, 1)],
              &[put_another(K_ANIMAL, 1)],
          ])),
          holds: None, on_build: []),
    // « When you play a Plant, Microbe, or Animal tags, including these, gain
    //   1 plant or add 1 animal or microbe to ANOTHER* card. » (forfait 1 :
    //   Java n'utilise PAS countCardTags pour la quantité, seulement pour
    //   savoir si l'effet se déclenche.)
    card!("Viral Enhancers", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::AnyOfTags(T_ANIMAL_MICROBE_PLANT),
                    gains: &[TrigGain::Choose(&[
                        &[ResEff::Gain(Plants(1))],
                        &[put_another(K_MICROBE_ANIMAL, 1)],
                    ])],
                    scale_by_matched_tags: false, include_self: true }],
          gtrig: [], action: None, holds: None, on_build: []),
    // « Requires red oxygen or higher. When you play an Animal, Microbe, or
    //   Plant, including this, add a microbe here or remove a microbe from here
    //   to draw a card. » (1 VP fixe dans cards.json ; forfait 1.)
    card!("Decomposers", reqs: [OxyMin(OXY_R_MIN)], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::AnyOfTags(T_ANIMAL_MICROBE_PLANT),
                    gains: &[TrigGain::Choose(&[
                        &[put_self(1)],
                        &[ResEff::RemoveSelf(1), ResEff::Gain(Draw(1))],
                    ])],
                    scale_by_matched_tags: false, include_self: true }],
          gtrig: [], action: None,
          holds: Some(ResKind::Microbe), on_build: []),
    // « Add 2 microbes to ANOTHER card. During the production phase, this
    //   produces 1 plant and 3 heat. »
    card!("Astrofarm", reqs: [], effects: [PlantProd(1), HeatProd(3)],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(put_another(K_MICROBE, 2))]),
    // « Requires red temperature or warmer. Add 1 animal to ANOTHER card and
    //   gain 3 plants. During the production phase, this produces 2 MC. »
    card!("Eos Chasma National Park", reqs: [TempMin(TEMP_R_MIN)],
          effects: [McProd(2)],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(put_another(K_ANIMAL, 1)),
                     ResStep::Do(ResEff::Gain(Plants(3)))]),
    // « Add 2 resources to a card that holds resources. » (pas « ANOTHER » :
    //   cible = n'importe quelle porteuse ; la carte elle-même n'en porte pas.)
    card!("CEO's Favorite Project", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(ResEff::Put(ResPut {
              target: ResTarget::Any, kinds: K_ANY, amount: ResAmount::Fixed(2) }))]),
    // « Requires you to spend 3 heat. Gain 4 plants and add 2 animals or
    //   microbes to ANOTHER card. »
    card!("Local Heat Trapping", reqs: [SpendHeat(3)], effects: [],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(ResEff::Gain(Plants(4))),
                     ResStep::Do(put_another(K_MICROBE_ANIMAL, 2))]),
    // « Raise your TR 1 step. Gain 4 plants. Add 2 animals to ANOTHER card.
    //   Add 3 microbes to ANOTHER card. » (deux cibles, dans l'ordre du texte.)
    card!("Imported Nitrogen", reqs: [], effects: [Tr(1), Plants(4)],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(put_another(K_ANIMAL, 2)),
                     ResStep::Do(put_another(K_MICROBE, 3))]),
    // « Flip an ocean tile. Gain 3 plants, or add 3 microbes or 2 animals to
    //   ANOTHER card. » (quantité selon le type porté par la cible.)
    card!("Imported Hydrogen", reqs: [], effects: [Ocean(1)],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Choose(&[
              &[ResEff::Gain(Plants(3))],
              &[ResEff::Put(ResPut { target: ResTarget::Another,
                    kinds: K_MICROBE_ANIMAL,
                    amount: ResAmount::ByKind { microbe: 3, other: 2 } })],
          ])]),
    // « Flip an ocean tile. Draw two cards. Gain 5 plants or add 3 animals to
    //   ANOTHER card. » — ADDENDUM round 2 : le scan dit « ANOTHER », le Java
    //   disait « ANY ». Le texte imprimé gagne.
    card!("Large Convoy", reqs: [], effects: [Ocean(1), Draw(2)],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Choose(&[
              &[ResEff::Gain(Plants(5))],
              &[put_another(K_ANIMAL, 3)],
          ])]),
    // « Upgrade a Phase card. Add 3 microbes or 2 animals to ANOTHER card. »
    // (amélioration de phase non gérée : sautée et comptée, aucune
    //  compensation — D8.)
    card!("Cryogenic Shipment", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(ResEff::PhaseUpgrade),
                     ResStep::Do(ResEff::Put(ResPut { target: ResTarget::Another,
                         kinds: K_MICROBE_ANIMAL,
                         amount: ResAmount::ByKind { microbe: 3, other: 2 } }))]),
    // « Requires an Animal, Microbe, and Plant tags. » (3 VP fixes dans
    //  cards.json ; aucune ressource — carte de complément.)
    card!("Advanced Ecosystems",
          reqs: [Tags(Tag::Animal, 1), Tags(Tag::Microbe, 1), Tags(Tag::Plant, 1)],
          effects: []),
];

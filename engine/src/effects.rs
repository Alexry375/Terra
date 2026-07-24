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
}

impl Reduction {
    /// Réduction applicable à une carte de tags donnés.
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
        }
    }
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
        }
    }
}

/// Gain élémentaire d'un déclencheur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrigGain {
    Heat(i64),
    Plants(i64),
    Draw(u8),
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
        })
    };
    // Forme lot 2 : tous les champs explicites.
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*],
     red: [$($rd:expr),*], ptrig: [$($pt:expr),*], gtrig: [$($gt:expr),*],
     action: $act:expr) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[$($rd),*], play_triggers: &[$($pt),*],
            global_triggers: &[$($gt),*], action: $act,
        })
    };
}

use Eff::*;
use Req::*;

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
];

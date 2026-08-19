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

use crate::cards::{Color, Tag};

// Bornes de paliers (niveaux) — voir doc du module.
pub const TEMP_P_MAX: u8 = 5;
pub const TEMP_R_MIN: u8 = 6;
pub const TEMP_R_MAX: u8 = 10;
pub const TEMP_Y_MIN: u8 = 11;
pub const TEMP_W_MIN: u8 = 16;
pub const OXY_R_MIN: u8 = 3;
/// Borne HAUTE du palier ROUGE d'oxygène (P 0-2, R 3-6) — « Requires red oxygen
/// **or lower** » (Colonizer Training Camp, lot 6) : oxygène ≤ 6.
pub const OXY_R_MAX: u8 = 6;
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
    /// (lot 6) Niveau d'oxygène courant <= n — « Requires red oxygen or lower »
    /// (*Colonizer Training Camp*, seule carte de la boîte de base à le porter).
    ///
    /// **Brique 1 du lot 6.** `Req` avait `TempMax` et `OceanMax` mais pas cette
    /// variante : la carte était donc jouable à n'importe quel niveau
    /// d'oxygène, ce qui est faux. Comme les autres prérequis de PARAMÈTRE, il
    /// est jugé sur l'instantané de début de phase par `requirements_met`
    /// (livret p.13 l.352) — c'est `reqs_satisfied` qui porte la règle, cette
    /// variante n'en invente aucune.
    OxyMax(u8),
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
    /// (lot 5) NT courant du joueur >= n, **sans le dépenser** (Energy Storage :
    /// « Requires you to have 7 or more TR »).
    ///
    /// Le NT est une ressource de JOUEUR, pas un paramètre global : ce prérequis
    /// est donc évalué à l'ÉTAT COURANT, comme `Tags` et `Spend*`, et non sur
    /// l'instantané de début de phase (livret p.13 l.352, qui ne parle que des
    /// océans, de l'oxygène et de la température).
    ///
    /// **Divergence déclarée** (journal D1) : le contrat du lot 5 affirmait que
    /// le vocabulaire existant suffisait aux groupes A et B. Il manquait cette
    /// variante — `SpendTr` dépense le NT, elle ne teste pas un seuil.
    TrMin(i64),
    /// (decouverte-projets) **« Requiert un Objectif »** — *Private Investor
    /// Beach* (D19), seule carte du jeu à le porter.
    ///
    /// Prérequis de JOUEUR, pas de paramètre planétaire : il est donc évalué à
    /// l'ÉTAT COURANT dans `flow::reqs_satisfied`, avec `Tags`, `TrMin` et les
    /// `Spend*`, et jamais sur l'instantané de début de phase (livret p.13
    /// l.352, qui ne parle que des océans, de l'oxygène et de la température).
    ///
    /// Même prédicat que `Eff::IfObjective` (`flow::has_objective`) : il n'y a
    /// qu'une définition d'« avoir un Objectif » dans le moteur.
    ///
    /// **Ce prérequis ne figure pas dans le tableau du contrat** ; il figure
    /// dans `data/cartes-imprimees/projets-decouverte/projets-decouverte.json`, qui fait foi (`reqs_fr` :
    /// « Requiert un Objectif. »). Voir `result.md`, § Où je vous contredis.
    HasObjective,
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
    /// (lot 4) Hausse de NT d'UN PAS PAR BADGE du type donné, lue au moment de
    /// l'application (Terraforming Ganymede : « Raise your TR 1 step per Jupiter
    /// tag you have, including this »). La carte étant mise en jeu AVANT
    /// l'application de ses effets (`put_in_play` puis `apply_card_effects`),
    /// son propre badge est déjà compté : « including this » ne demande aucun
    /// traitement particulier. Chaque pas emprunte le chemin de hausse de NT
    /// existant (`PlayerState::gain_tr`, comptabilisé pour l'invariant TR).
    TrPerTag(Tag),
    /// (lot 5) **Gain de n jetons PV Forêt, sans rien payer.**
    ///
    /// Unique brique neuve du lot 5. Le texte imprimé des quatre cartes du
    /// groupe C dit « **Gain a forest VP** and raise oxygen 1 step » —
    /// mot pour mot la formule de l'action standard du livret (p. 14, l. 379 :
    /// « Dépenser 8 plantes pour gagner un PV Forêt et augmenter l'oxygène d'un
    /// niveau »). Ce n'est donc PAS « une forêt PLUS un pas d'oxygène » : c'est
    /// la description de ce qu'un gain de forêt produit. *Plantation* (« Gain 2
    /// forest VPs and raise oxygen 2 steps ») vaut `Forest(2)` — 2 forêts,
    /// 2 pas d'oxygène, jamais 4 (règle R1 du contrat, journal D2).
    ///
    /// Chaque pas emprunte `flow::gain_forest`, **le seul** chemin de gain de
    /// forêt du moteur, celui-là même qu'emprunte l'action standard payée : le
    /// compteur `PlayerState::forests`, la hausse d'oxygène (donc le NT) et le
    /// déclencheur « when you gain a forest VP » (*Small Animals*) sont ainsi
    /// servis une fois et une seule, sans second chemin parallèle (journal D4).
    Forest(u8),
    /// (lot 6) **Piochez `draw` cartes, puis défaussez-en `discard`.**
    ///
    /// **Brique 5 du lot 6**, et la SEULE brique des trois cartes du groupe C
    /// (I3) : *Business Contracts* (4/2), *Invention Contest* (3/2),
    /// *Microprocessors* (2/1). Les trois partagent ce chemin unique
    /// (`flow::apply_eff`), elles ne diffèrent que par leurs données.
    ///
    /// `from_drawn` porte la seule vraie différence de TEXTE IMPRIMÉ entre
    /// elles : *Invention Contest* dit « Keep one of **them** and discard the
    /// other two » — la défausse est restreinte aux cartes piochées ; les deux
    /// autres disent « Then, discard N cards », sans restriction, donc la
    /// défausse porte sur la main entière (la carte jouée en est déjà sortie,
    /// `build_card_with` la retire avant d'appliquer les effets).
    ///
    /// Le drapeau change AUSSI ce que le texte compte, ce qui ne se voit que
    /// pioche épuisée : « **keep one** of them » compte les cartes GARDÉES
    /// (deux cartes rendues au lieu de trois ⇒ on en garde toujours une, on n'en
    /// défausse qu'une), « discard N cards » compte les cartes DÉFAUSSÉES. Voir
    /// `flow::apply_eff`.
    ///
    /// Le CHOIX des cartes défaussées passe par `Policy::discard_down` — le
    /// point de décision que le moteur emploie déjà pour la limite de main :
    /// aucune source de hasard nouvelle (I6).
    DrawDiscard {
        draw: u8,
        discard: u8,
        /// true = la défausse est restreinte aux cartes qui viennent d'être
        /// piochées (*Invention Contest*).
        from_drawn: bool,
    },
    /// (decouverte-projets) **Gain conditionné à la possession d'un Objectif** —
    /// *Award Winning Reflector Material* (D35), « Si vous avez un Objectif,
    /// gagnez 4 chaleurs. »
    ///
    /// « Objectif » = tuile MILESTONE du moteur (`state::MilestoneKind`) ;
    /// « Récompense » = `AwardKind`. Le prédicat est
    /// `flow::has_objective(game, p)` : au moins l'un des trois Objectifs en jeu
    /// est revendiqué par CE joueur.
    ///
    /// La condition se juge **au moment de la pose**, comme tout effet immédiat
    /// (ASK 3) : un Objectif revendiqué plus tard ne rétro-paie rien. Quand elle
    /// est vraie, `flow::apply_eff` verse les effets imbriqués ET incrémente
    /// `objective_condition_hits` — le compteur et le gain sont indissociables.
    ///
    /// La forme est GÉNÉRIQUE (une liste d'effets) plutôt que
    /// « HeatIfObjective(n) » : c'est la condition qui est la brique, pas la
    /// chaleur.
    IfObjective(&'static [Eff]),
}

// ================================================== lot 4 : production dérivée
//
// Les cartes vertes dont la production DÉPEND DU NOMBRE DE BADGES du joueur
// (livret FR p.13 l.180 : « Certaines cartes de production augmentent leur
// production lorsque vous avez plus d'un badge spécifique »). La quantité n'est
// JAMAIS inscrite sur les pistes `*_prod` : elle est recalculée à chaque phase
// IV par le service unique `flow::derived_production`.
//
// « including this » n'est pas une règle à part : la carte est en jeu au moment
// du décompte, donc son propre badge compte, comme tous les autres. Le calcul
// est uniforme — aucune carte ne s'exclut, aucune ne se compte deux fois.

/// (lot 4) Ressource gagnée par une production dérivée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProdRes {
    Mc,
    Heat,
    Plants,
}

/// (lot 4) Ce que la carte compte pour calculer sa production.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProdCount {
    /// Badges d'un type donné possédés par le joueur, corporation comprise
    /// (`PlayerState::tag_counts`, tenu à jour par `put_in_play`).
    Tag(Tag),
    /// Jetons Forêt du joueur (`PlayerState::forests`) — Zeppelins, dont le
    /// texte imprimé dit « 1 MC per forest VP you have » : des FORÊTS, pas des
    /// badges.
    Forests,
}

/// (lot 4) Production DÉRIVÉE : recalculée à chaque phase de production.
/// Quantité gagnée = compteur / `per`, en division ENTIÈRE (Medical Lab :
/// 1 MC par 2 badges Construction → 1 seul badge ne rapporte rien).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedProd {
    pub res: ProdRes,
    pub count: ProdCount,
    pub per: u32,
}

/// (lot 4) Bonus PERMANENT de phase Recherche (Interplanetary Relations :
/// « When you draw cards during the research phase, draw one additional card
/// and keep one additional card »). Cumulé sur les cartes en jeu par le service
/// unique `flow::research_extra` et consommé par la seule phase V — jamais par
/// la mise en place, ni par la production de cartes, ni par une pioche d'effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchBonus {
    /// Cartes piochées en plus.
    pub draw: usize,
    /// Cartes gardées en plus.
    pub keep: usize,
}

// ================================================================ lot 2 (A/B/C)
//
// Vocabulaire persistant du lot 2 (chantier moteur-cartes-2) : réductions de
// coût (A), déclencheurs de pose et globaux (B), actions de cartes bleues (C).
// Le texte imprimé fait foi (voir lot2.md pour les conflits, notamment
// Asteroid Mining : Java = revenu titane 2 → −6 MC Space ; imprimé = −6 MC Space
// encodé directement, sans modéliser le titane).

// ======================================== lot acier-titane : les savoir-faire
//
// **L'acier et le titane ne sont pas des jetons.** Le livret (FR, l. 355-359 et
// 523-529) : « Chaque acier que vous possédez réduit de 2 MC le coût des cartes
// à badge bâtiment que vous jouez », « chaque titane … de 3 MC … badge espace ».
// Ce sont des savoir-faire PERMANENTS : on ne les dépense pas, les posséder
// suffit.
//
// Le moteur encodait déjà l'EFFET NET de chaque savoir-faire — une carte à
// 2 aciers porte `Reduction::Tag(Tag::Building, 4)`. Le compte n'a donc pas
// besoin d'une seconde source de données : il se LIT sur les réductions déjà
// encodées, en divisant par le taux du livret. C'est la seule façon qu'un
// savoir-faire ajouté demain compte tout seul (I2).

/// (lot acier-titane) Un savoir-faire permanent : l'acier ou le titane.
///
/// Porte les deux constantes de règle, et elles n'existent nulle part ailleurs
/// dans le moteur : le badge sur lequel le savoir-faire agit, et sa valeur en MC
/// par unité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capacity {
    /// Acier : 2 MC par unité sur les cartes à badge bâtiment.
    Steel,
    /// Titane : 3 MC par unité sur les cartes à badge espace.
    Titanium,
}

impl Capacity {
    /// Badge sur lequel ce savoir-faire réduit le coût (livret l. 355-359).
    pub const fn tag(self) -> Tag {
        match self {
            Capacity::Steel => Tag::Building,
            Capacity::Titanium => Tag::Space,
        }
    }

    /// Valeur d'UNE unité, en MC de réduction (livret l. 355-359 et 523-529).
    pub const fn mc_per_unit(self) -> i64 {
        match self {
            Capacity::Steel => 2,
            Capacity::Titanium => 3,
        }
    }

    /// Le savoir-faire qui agit sur ce badge, s'il y en a un. C'est le seul
    /// point du moteur qui fait le chemin inverse `Tag → Capacity` : une
    /// réduction sur un autre badge (événement, énergie, Terre, Jupiter…) n'est
    /// jamais un savoir-faire.
    pub const fn from_tag(t: Tag) -> Option<Capacity> {
        match t {
            Tag::Building => Some(Capacity::Steel),
            Tag::Space => Some(Capacity::Titanium),
            _ => None,
        }
    }

    /// **La dérivation, à l'unité près.** Combien d'unités de ce savoir-faire
    /// vaut une réduction de `amount` MC ?
    ///
    /// `None` quand `amount` n'est pas un multiple EXACT du taux (I3) : la
    /// fonction n'arrondit jamais, elle refuse. `flow::capacities` traduit ce
    /// `None` en panique, et `CardsDb::load` le fait remonter beaucoup plus tôt
    /// — au chargement des tables, avant la première partie.
    pub const fn units_from(self, amount: i64) -> Option<i64> {
        let per = self.mc_per_unit();
        if amount < 0 || amount % per != 0 {
            None
        } else {
            Some(amount / per)
        }
    }
}

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
    /// (lot corporations) Cartes dont le **prix IMPRIMÉ** est >= `min`
    /// (Credicor : « a card with a printed cost of 20 MC or more » → −4 MC).
    /// Le seuil porte sur `ProjectCard::price`, jamais sur le coût déjà réduit :
    /// sinon deux réductions se conditionneraient l'une l'autre selon leur ordre.
    MinPrice { min: i64, amount: i64 },
    /// (lot 3) Réduction CONDITIONNELLE et PAYANTE : retirer `count` ressources
    /// de type `kind` de la carte qui porte cette réduction pour payer `amount`
    /// MC de moins (Anaerobic Microorganisms : 2 microbes → −10 MC).
    ///
    /// Elle ne fait PAS partie de la somme fixe de `flow::card_discount` (elle
    /// dépend des ressources posées et d'une décision du joueur) : elle est
    /// servie par `flow::microbe_discount` et consommée par `flow::affordable`
    /// (montant potentiel) puis `flow::build_card_with` (décision + retrait).
    PayResources { kind: ResKind, count: u32, amount: i64 },
    /// (lot cartes-7) Réduction CONDITIONNELLE et PAYANTE, mais dont la monnaie
    /// est prise sur la RÉSERVE DU JOUEUR et non sur une carte : *Restructured
    /// Resources*, « When you play a card, you may spend **1 plant** to reduce
    /// that card's cost by **5 MC**. »
    ///
    /// Décalque exact de [`Reduction::PayResources`], à la monnaie près. Comme
    /// elle, elle vaut 0 dans [`Reduction::amount_for`] — son montant dépend
    /// d'une DÉCISION du joueur (le « may »), jamais accordée gratuitement. Elle
    /// est servie par `flow::plant_discount` et consommée par `flow::affordable`
    /// (montant potentiel) puis `flow::build_card_with` (choix + dépense).
    PayPlants { plants: i64, amount: i64 },
    /// (lot acier-titane) Réduction ADDITIONNELLE de `per` MC **par unité** du
    /// savoir-faire `cap` que le joueur possède, sur les cartes portant `tag`.
    ///
    /// Deux planches l'emploient, avec exactement leur texte imprimé :
    /// *Advanced Alloys* (« Each titanium you have reduces the cost of [space]
    /// cards an additional 1 MC. Each steel you have … [building] … 1 MC. ») et
    /// *PhoboLog* (« EFFECT: Each titanium you have reduces the cost of [space]
    /// cards an additional 1 MC. »).
    ///
    /// Elle vaut 0 dans [`Reduction::amount_for`], qui ne connaît que la carte
    /// visée : son montant dépend de l'ÉTAT DU JOUEUR. `flow::card_discount`,
    /// qui a cet état, est le seul à la résoudre — au moment du paiement, jamais
    /// figée à la pose (I7).
    ///
    /// `tag` et `cap` sont deux données distinctes parce que le texte imprimé
    /// les distingue : rien n'oblige un savoir-faire à n'amplifier que son
    /// propre badge, et deux tables séparées seraient une règle inventée.
    PerCapacity { tag: Tag, cap: Capacity, per: i64 },
}

impl Reduction {
    /// Réduction FIXE applicable à une carte de tags et de PRIX IMPRIMÉ donnés.
    /// Les réductions conditionnelles (`PayResources`) valent 0 ici : elles ont
    /// leur propre chemin, elles ne sont jamais accordées gratuitement.
    pub fn amount_for(self, tags: &[Tag], price: i64) -> i64 {
        match self {
            Reduction::AnyCard(n) => n,
            // (D20) Le NOMBRE de badges, pas leur présence — livret de base
            // l. 106, exactement la règle qui fait déjà compter les badges à la
            // moitié « piochez » de la même phrase imprimée (*Energy Subsidies* :
            // « you pay 4 MC less for it AND draw a card », dont le `ptrig`
            // porte `scale_by_matched_tags: true`). Aucune carte de la pioche ne
            // porte aujourd'hui deux fois le même badge parmi ceux concernés :
            // la correction ne change aucune partie, elle ferme la classe de
            // défaut.
            Reduction::Tag(t, n) => n * tags.iter().filter(|&&x| x == t).count() as i64,
            Reduction::MinPrice { min, amount } => {
                if price >= min {
                    amount
                } else {
                    0
                }
            }
            Reduction::PayResources { .. } => 0,
            // (lot cartes-7) Dépend d'un « may » du joueur : jamais accordée
            // gratuitement, résolue par `flow::plant_discount`.
            Reduction::PayPlants { .. } => 0,
            // (lot acier-titane) Dépend du nombre d'aciers/titanes du joueur,
            // que cette fonction ne connaît pas : résolue par
            // `flow::card_discount`, comme `PayResources` l'est par
            // `flow::microbe_discount`.
            Reduction::PerCapacity { .. } => 0,
        }
    }

    /// (lot acier-titane) Les unités de savoir-faire que cette réduction
    /// DÉCLARE, quand elle en est une. C'est **le** point de lecture de la
    /// dérivation : `flow::capacities` ne connaît rien d'autre.
    ///
    /// - `Tag(Building, n)` → `(Steel, n/2)`, `Tag(Space, n)` → `(Titanium, n/3)` ;
    /// - toute autre réduction, y compris `PerCapacity` (qui AMPLIFIE un
    ///   savoir-faire sans en être un) → `None`.
    ///
    /// `panic!` si le montant n'est pas un multiple exact du taux (I3) : voir
    /// `Capacity::units_from`. Le garde-fou de `CardsDb::load` rend ce cas
    /// impossible en pratique — c'est lui le contrôle utile, celui-ci est le
    /// filet.
    pub fn capacity_units(self) -> Option<(Capacity, i64)> {
        let Reduction::Tag(t, n) = self else {
            return None;
        };
        let cap = Capacity::from_tag(t)?;
        let units = cap.units_from(n).unwrap_or_else(|| {
            panic!(
                "réduction {t:?} de {n} MC : pas un multiple de {} — \
                 un savoir-faire ne s'arrondit pas (I3)",
                cap.mc_per_unit()
            )
        });
        Some((cap, units))
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
    /// « Améliorez une carte Phase » : le joueur remplace l'une de ses cinq
    /// cartes Phase par l'une des dix améliorées (`flow::apply_phase_upgrade`).
    /// L'effet était SAUTÉ et compté dans `phase_upgrades_skipped` jusqu'au
    /// chantier `decouverte-phases` ; il est appliqué depuis.
    ///
    /// **(decouverte-projets) Le paramètre est la PHASE IMPOSÉE** :
    /// - `None` — « Améliorez **une** carte Phase », le joueur choisit laquelle
    ///   (comportement d'avant ce chantier, bit à bit) ;
    /// - `Some(n)` — « Améliorez **votre carte Phase n** » : D05 (III), D37 (I),
    ///   D40 (IV). Seule la PHASE est imposée ; la VARIANTE (A ou B) reste un
    ///   choix du joueur, tranché par `Policy::choose_option` comme d'habitude
    ///   (NEVER 7).
    ///
    /// C'est un paramètre de l'effet, pas trois exceptions dans le flux : il
    /// n'existe toujours qu'un seul chemin d'octroi, `apply_phase_upgrade`
    /// (NEVER 1, clause anti-shortcut n° 3).
    PhaseUpgrade(Option<u8>),
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
    /// **(D2) La carte posée apporte un savoir-faire** du secteur donné —
    /// *Mining Guild*, « EFFECT: Each time you play steel production, excluding
    /// this, gain 1 TR ».
    ///
    /// C'est la seule condition du moteur qui ne se lit pas sur les badges de la
    /// carte : elle se lit sur les unités de savoir-faire que la carte APPORTE,
    /// dérivées de ses réductions encodées par le service unique
    /// `flow::capacites_apportees` (lot acier-titane). [`TrigCond::matched_tags`]
    /// rend donc 0 pour elle — les badges seuls ne peuvent pas y répondre — et
    /// c'est `flow::trig_matched`, qui tient la carte posée, qui la résout.
    ///
    /// Le nombre rendu est le NOMBRE D'UNITÉS apportées : avec
    /// `scale_by_matched_tags`, une carte qui apporte deux aciers accorde deux
    /// fois le gain (arbitrage d'Alexis du 18-08, carton en main : « 1 niveau de
    /// terraformation par acier gagné »).
    GrantsCapacity(Capacity),
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
            // (D2) Les badges ne disent rien du savoir-faire apporté : cette
            // fonction n'a pas de quoi répondre, et elle ne devine pas. La
            // condition est résolue par `flow::trig_matched`, qui tient la carte
            // posée — c'est le seul appelant des conditions de déclenchement du
            // moteur.
            TrigCond::GrantsCapacity(_) => 0,
        }
    }
}

/// Gain élémentaire d'un déclencheur.
#[derive(Debug, Clone, Copy)]
pub enum TrigGain {
    Heat(i64),
    Plants(i64),
    Draw(u8),
    /// (lot corporations) Hausse de NT (Saturn Systems : « Each time you play a
    /// [jupiter], excluding this, gain 1 TR »). Passe par le chemin de hausse de
    /// NT existant, donc comptabilisée pour l'invariant TR.
    Tr(u8),
    /// (lot 3) Ajoute n ressources sur la carte SOURCE du déclencheur
    /// (Fish, Livestock, Small Animals, Herbivores, Physics Complex,
    /// Ecological Zone, Anaerobic Microorganisms).
    ResSelf(u32),
    /// (lot 3) Alternative offerte au joueur (Viral Enhancers, Decomposers).
    ///
    /// **CORRIGÉ par moteur-verite-1** — ce commentaire disait l'inverse :
    /// « appliquée UNE fois par déclenchement, jamais multipliée par le nombre
    /// de tags (les deux cartes concernées sont au forfait) ». C'était faux, et
    /// recopié du moteur Java. Le livret officiel (p.9, l.106) tranche : « Si la
    /// condition d'un effet est remplie plusieurs fois lorsqu'une carte est
    /// jouée, résolvez l'effet correspondant plusieurs fois. »
    ///
    /// `Choose` obéit donc au même `mult` que tous les autres gains : elle est
    /// résolue une fois par condition remplie, et **la politique est
    /// reconsultée à chaque résolution** — le joueur peut prendre une branche
    /// différente à chaque fois, ce que le texte imprimé autorise. Une carte à
    /// deux badges satisfaisants déclenche deux résolutions.
    /// Voir `flow::apply_trig_gain`.
    Choose(&'static [&'static [ResEff]]),
    /// (lot cartes-7) « **You MAY discard a card.** If that card had a [tag],
    /// draw `draw_if` cards. Otherwise, draw `draw_else`. » — *Mars University*.
    ///
    /// Le « may » est un vrai choix de `Policy` (I4) : branche 0 = défausser,
    /// l'option imprimée ; branche 1 = renoncer. La branche « défausser » est
    /// FILTRÉE avant le choix quand la main est vide — à zéro branche jouable,
    /// aucune question n'est posée (convention du lot 3).
    ///
    /// *Quelle* carte est un `Policy::discard_down(hand, 1)`, le point de
    /// décision existant : aucune source de hasard nouvelle. Le badge regardé
    /// est celui de la carte **défaussée**, lu avant qu'elle quitte la main.
    ///
    /// Comme tout autre gain, elle est résolue `mult` fois (livret p.9 l.106).
    MayDiscardDraw {
        /// Badge cherché sur la carte défaussée.
        if_tag: Tag,
        /// Cartes piochées si la carte défaussée le portait.
        draw_if: u8,
        /// Cartes piochées sinon.
        draw_else: u8,
    },
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
    /// (lot 3) « **When you gain a forest VP** » (Small Animals).
    ///
    /// **Doc corrigée par le lot 5 (journal D3)** — elle disait « when you
    /// *build* a forest », traduction reprise du Java (`onForestBuiltEffect`).
    /// Le texte imprimé de *Small Animals* dit exactement : « Effect: When you
    /// **gain a forest VP**, add 1 animal to this card. » La condition porte
    /// donc sur le GAIN du jeton, quelle qu'en soit l'origine — l'action
    /// standard payée (8 plantes / 20 MC) comme l'effet d'une carte du groupe C
    /// (`Eff::Forest`). Les deux passent par `flow::gain_forest`, qui lève
    /// l'événement une fois par forêt gagnée : *Plantation* (2 forêts) pose
    /// 2 animaux (livret l. 106 : condition remplie plusieurs fois → effet
    /// résolu plusieurs fois).
    ///
    /// Le NOM de la variante est conservé : le renommer toucherait le lot 3
    /// sans rien prouver de plus.
    OnBuildForest(&'static [TrigGain]),
}

/// Coût d'activation d'une action de carte bleue (payé une fois par activation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCost {
    Heat(i64),
    Mc(i64),
    Plants(i64),
    /// (lot 6) **Défausser `n` cartes de sa main comme COÛT de l'action**
    /// (*Farming Co-ops* : « Action: Discard a card in hand to gain 3 plants »).
    ///
    /// **Brique 3 du lot 6.** Les coûts existants se paient en ressources ;
    /// celui-ci se paie en cartes, et il n'est payable que si la main en porte
    /// assez. Les cartes défaussées sont choisies par `Policy::discard_down`
    /// (point de décision existant) et rejoignent la défausse commune : la
    /// conservation des cartes reste vraie (I6, invariant 4).
    DiscardCard(u8),
    /// (lot cartes-8) **Dépenser `n` points de note de terraformation** comme
    /// coût de l'action — *Asset Liquidation*, « Action: Spend 1 TR to draw
    /// three cards. »
    ///
    /// La note de terraformation est à la fois une ressource et la moitié du
    /// score final : la dépenser est un vrai sacrifice. Le moteur sait déjà la
    /// dépenser en PRÉREQUIS (`Req::SpendTr`, *Water Import from Europa*) ;
    /// c'est le même retrait, au même compteur d'audit (`tr_decrements`), à
    /// l'endroit d'une action.
    Tr(u8),
    /// (lot acier-titane) **Coût en MC réduit par un savoir-faire** : `base` MC,
    /// moins `per` MC par unité de `cap` que le joueur possède, jamais négatif.
    ///
    /// Trois planches l'emploient, mot pour mot leur texte imprimé :
    /// *Aquifer Pumping* (« Spend 10 MC … Reduce this by 2 MC per steel you
    /// have »), *Solarpunk* (15 MC, −2 par titane), *Water Import from Europa*
    /// (12 MC, −1 par titane).
    ///
    /// Le montant est calculé à l'ACTIVATION, sur le compte du joueur à cet
    /// instant : un savoir-faire acquis après la pose de la carte compte (I7).
    /// Comme tout coût en MC, il passe par `flow::spendable_mc` /
    /// `top_up_mc_with_heat` — Helion peut le payer en chaleur, comme partout.
    McPerCapacity { base: i64, cap: Capacity, per: i64 },
}

/// Effet d'une action de carte bleue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEff {
    Draw(u8),
    Plants(i64),
    Mc(i64),
    Tr(u8),
    Oxygen(u8),
    /// (lot 6) Gain immédiat de chaleur (*Hydro-Electric Energy*).
    ///
    /// **Ajout DÉCLARÉ, non mécanique** (journal D3) : ce n'est pas une brique,
    /// c'est une valeur de plus dans une énumération existante, qui écrit sur la
    /// réserve de chaleur du joueur comme `Eff::Heat` le fait déjà à la pose.
    Heat(i64),
    /// (lot 6) Hausse de température de n pas (*Wood Burning Stoves*).
    ///
    /// **Ajout DÉCLARÉ, non mécanique** (journal D3) : elle emprunte
    /// `flow::raise_temperature`, exactement comme `ActionEff::Oxygen` emprunte
    /// `raise_oxygen` depuis le lot 2 (TR, caps sur l'instantané de phase,
    /// déclencheurs « when you raise the temperature »).
    Temperature(u8),
    /// (lot 6) **Révélation du dessus de la pioche** (*Advanced Screening
    /// Tech*, *Brainstorming Session*) — voir [`Reveal`].
    ///
    /// **Brique 6 du lot 6.**
    Reveal(Reveal),
    /// (lot acier-titane) **Retourne `n` tuiles océan** — *Aquifer Pumping*,
    /// *Water Import from Europa* : « Spend … MC to flip an ocean tile ».
    ///
    /// Emprunte `flow::reveal_ocean`, le chemin océan unique du moteur : bonus
    /// de la tuile, +1 NT, déclencheurs « when you flip an ocean tile », cap sur
    /// l'instantané de début de phase. Une action qui poserait un océan alors
    /// qu'il n'en reste plus ne s'applique pas du tout (voir
    /// `flow::action_effs_possible`) : le joueur ne paie jamais pour rien —
    /// c'est déjà la règle de `Action::FlipOceanTagDiscount`.
    Ocean(u8),
    /// (lot acier-titane) **Gain de `n` jetons PV Forêt** — *Solarpunk* :
    /// « Spend 15 MC to gain a forest VP and raise oxygen 1 step ».
    ///
    /// Emprunte `flow::gain_forest`, le chemin de forêt unique du moteur, exactement
    /// comme `Eff::Forest` du lot 5 : le jeton, UN pas d'oxygène (donc le NT), et
    /// l'événement « when you gain a forest VP ». « … and raise oxygen 1 step »
    /// décrit ce que la forêt produit, ce n'est pas un second effet — l'oxygène
    /// ne monte jamais deux fois (règle R1, lot 5).
    Forest(u8),
    /// (decouverte-projets) **« Action : … améliorer une carte Phase »** —
    /// *Experimental Technology* (D07, « Dépensez 1 NT pour améliorer une carte
    /// Phase ») et *Virtual Employee Development* (D12, sans coût).
    ///
    /// Emprunte `flow::apply_phase_upgrade`, le chemin d'octroi UNIQUE du
    /// moteur, avec `UpgradeSource::Action` — c'est ce qui alimente
    /// `phase_upgrades_by_action`. Le coût, lui, n'est pas ici : il est déclaré
    /// dans `ActionCost` (`Tr(1)` pour D07, brique du lot cartes-8).
    ///
    /// La phase n'est jamais imposée par une action : aucun carton ne le
    /// demande. Le paramètre de `ResEff::PhaseUpgrade` n'est donc pas dupliqué
    /// ici (diff minimal) ; le jour où un carton l'exigerait, il s'ajouterait
    /// de la même façon.
    PhaseUpgrade,
    /// (decouverte-projets) **« Action : piochez deux cartes. Puis, défaussez
    /// deux cartes. »** — *Software Streamlining* (D11).
    ///
    /// La règle existe depuis le lot 6 sous la forme `Eff::DrawDiscard`, côté
    /// POSE ; cette variante la demande côté ACTION et **délègue au même corps
    /// de règle** (`flow::apply_eff`) — il n'y a pas deux façons de piocher puis
    /// défausser. `from_drawn: false` : « Puis, défaussez deux cartes » porte
    /// sur la main ENTIÈRE d'après la pioche (les cartes piochées sont
    /// défaussables), et la défausse est obligatoire (ASK 6).
    DrawDiscard {
        draw: u8,
        discard: u8,
        from_drawn: bool,
    },
}

/// (lot 6) Ressource d'un coût ou d'un gain VARIABLE (brique 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRes {
    Heat,
    Mc,
    Plants,
}

/// (lot 6) **Révélation du dessus de la pioche** (brique 6).
///
/// `n` cartes sont réellement retirées du dessus de la pioche par
/// `flow::draw_card` — le chemin de pioche du moteur, remélange de la défausse
/// compris. Celles qui satisfont `keep` sont les seules à pouvoir entrer en
/// main ; `take` en borne le nombre, et le CHOIX passe par
/// `Policy::research_keep` (« garder k parmi n », la question exacte que la
/// phase V pose déjà). Toutes les autres rejoignent la défausse, chacune
/// rapportant `mc_per_discarded` MC.
///
/// - *Advanced Screening Tech* : « Reveal the top three cards. Place a card with
///   a [science] or [plant] revealed this way into your hand. Discard the
///   rest. » → `n: 3, keep: AnyOfTags([Science, Plant]), take: 1,
///   mc_per_discarded: 0`.
/// - *Brainstorming Session* : « Reveal the top card. If it is green, discard it
///   and gain 1 MC. Otherwise, draw it. » → `n: 1, keep: ColorIsNot(Green),
///   take: 1, mc_per_discarded: 1` — une carte non verte est gardée, une carte
///   verte est défaussée et rapporte 1 MC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reveal {
    pub n: u8,
    pub keep: RevealFilter,
    pub take: u8,
    pub mc_per_discarded: i64,
}

/// (lot 6) Ce qui rend une carte révélée éligible à entrer en main.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealFilter {
    /// Carte portant au moins un des badges donnés.
    AnyOfTags(&'static [Tag]),
    /// Carte dont la COULEUR n'est pas celle donnée.
    ColorIsNot(Color),
}

/// (lot 6) **Bonus d'action conditionné à la phase choisie** (brique 2).
///
/// Texte imprimé : « *If you chose the action phase this round, … ». Le bonus
/// est jugé sur `PlayerState::chosen_phase` du joueur QUI ACTIVE l'action —
/// jamais sur celle de l'adversaire (NEVER 8) — au moment de l'activation.
///
/// Deux formes, toutes deux imprimées sur des cartes de la boîte de base :
/// - **effets en plus** (`extra`) : *Community Gardens* « also gain 1 plant »,
///   *Hydro-Electric Energy* « gain 1 additional heat » ;
/// - **coût de remplacement** (`cost`) : *Wood Burning Stoves* « spend 3 plants
///   instead » — le coût imprimé de 4 plantes devient 3.
#[derive(Debug, Clone, Copy)]
pub struct PhaseBonus {
    /// Phase que le joueur doit avoir choisie ce tour (3 = phase Action pour les
    /// trois cartes concernées ; la valeur reste une donnée de la table).
    ///
    /// **(decouverte-projets) `0` = aucune phase exigée** — la condition ne
    /// porte alors que sur `require_upgraded`. *Drone Assisted Construction*
    /// (D06) dit « si vous jouez une carte Phase améliorée lors de cette
    /// manche », sans nommer de phase.
    pub phase: u8,
    /// (decouverte-projets) **La carte Phase que le joueur a révélée cette
    /// manche doit être AMÉLIORÉE** — *Drone Assisted Construction* (D06),
    /// « *Si vous jouez une carte Phase améliorée lors de cette manche, gagnez
    /// 2 MC supplémentaires. »
    ///
    /// Lue sur `PlayerState::phase_upgrade(chosen_phase)` du joueur QUI ACTIVE
    /// — jamais celle de l'adversaire, exactement comme `phase`. Même lecture
    /// que `CardEffects::reveal_bonus` (D05), exprimée à l'endroit d'une action
    /// (ASK 5). Quand elle est vraie, `flow::apply_blue_action` incrémente
    /// `upgraded_reveal_bonuses` au site du versement.
    pub require_upgraded: bool,
    /// Coût qui REMPLACE celui de l'action quand la condition est remplie.
    /// `None` = coût inchangé.
    pub cost: Option<&'static [ActionCost]>,
    /// Effets appliqués EN PLUS de ceux de l'action.
    pub extra: &'static [ActionEff],
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
    ///
    /// **Inchangée au lot 6.** « Spend ANY amount » n'a pas de plafond imprimé :
    /// le montant ne peut pas s'énumérer, il est tiré par
    /// `Policy::action_amount`. Un tirage nul y vaut renoncement
    /// (`action_applied: false`) — comportement du lot 2, conservé bit à bit,
    /// parce que *Power Infrastructure* est hors du périmètre du lot 6 (I4).
    HeatToMc,
    /// (lot 6, brique 4) **Coût VARIABLE « jusqu'à n »** : dépenser X unités de
    /// `spend` pour gagner autant d'unités de `gain` (*Greenhouses* : « Spend up
    /// to 4 heat to gain that amount of plants »).
    ///
    /// Le plafond IMPRIMÉ change la nature de la décision : les montants
    /// possibles s'énumèrent (1, 2, 3, 4), c'est donc une ALTERNATIVE, et le
    /// moteur a déjà un point de décision pour cela — `Policy::choose_option`,
    /// avec sa convention du lot 3 : les branches injouables sont filtrées
    /// AVANT le choix, et à une seule branche jouable on ne demande rien.
    /// Un montant nul n'est pas une branche du texte imprimé : « spend up to 4
    /// heat **to gain that amount of plants** » décrit un échange, et « ne rien
    /// faire » s'exprime dans le moteur en ne choisissant pas l'action, pas en
    /// la choisissant pour 0. Une carte sans la moindre unité à dépenser n'a
    /// donc aucune branche jouable : l'action ne s'applique pas.
    ///
    /// Conséquence utile pour l'audit : le montant est scriptable par
    /// `--probe-choice` (branche 0 = 1 unité, branche k = k+1 unités).
    SpendUpTo {
        spend: ActionRes,
        gain: ActionRes,
        /// Plafond imprimé (« up to N »), toujours ≥ 1.
        cap: i64,
    },
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

// =============================================================================
// (lot cartes-8) POSES SUPPLÉMENTAIRES ET MODIFICATEURS DE LA PROCHAINE CARTE
//
// Cinq cartes — les cinq dernières muettes de la boîte de base — accordent le
// droit de poser une carte DE PLUS dans la phase en cours :
//
//   « You may play an additional blue or red card this phase. »
//        — Asset Liquidation, Special Design, Work Crews (posées en phase II)
//   « You may play a green card from your hand that has a printed cost of
//     9 MC or less without paying its MC cost. »
//        — Automated Factories, Tall Station (posées en phase I)
//
// Ces deux textes ne sont PAS deux mécanismes : c'est le même, avec des
// paramètres différents (couleurs autorisées, plafond de prix imprimé,
// gratuité). D'où une seule structure, [`BuildGrant`], et un seul chemin de
// pose supplémentaire dans le flux de jeu (I1). La pose ORDINAIRE d'une phase
// est elle-même décrite par un `BuildGrant` : il n'existe donc qu'UNE façon de
// poser une carte dans tout le moteur.
// =============================================================================

/// (lot cartes-8) **Permission de poser une carte.** Décrit ce qu'une pose
/// autorise : quelles couleurs, jusqu'à quel prix imprimé, et à quel titre
/// (payante ou offerte).
///
/// Les poses ordinaires des phases I et II sont des permissions comme les
/// autres — voir `flow::GRANT_DEVELOPMENT` et `flow::GRANT_CONSTRUCTION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildGrant {
    /// Couleurs posables sous cette permission.
    pub colors: &'static [Color],
    /// Plafond sur le **prix imprimé** de la carte (`None` = aucun plafond).
    ///
    /// C'est bien le prix IMPRIMÉ, celui du carton, et non le prix effectivement
    /// payé : « a printed cost of 9 MC or less ». Une carte à 12 MC ramenée à
    /// 8 MC par un savoir-faire reste hors de portée d'une permission plafonnée
    /// à 9 ; une carte à 9 MC y entre même si aucune réduction ne s'applique.
    pub max_printed_cost: Option<i64>,
    /// La carte est-elle **offerte** ? `true` = « without paying its MC cost » :
    /// aucun MC n'est dû, aucune défausse n'est demandée, aucune chaleur n'est
    /// convertie. Les dépenses de PRÉREQUIS de la carte posée (« Requires you
    /// to spend 2 plants ») restent dues : elles ne sont pas son prix.
    pub free: bool,
}

impl BuildGrant {
    /// La carte `price` / `color` entre-t-elle dans cette permission ?
    pub fn admits(&self, color: Color, printed_price: i64) -> bool {
        self.colors.contains(&color)
            && self.max_printed_cost.is_none_or(|max| printed_price <= max)
    }
}

/// (lot cartes-8) **Modificateur de la PROCHAINE carte posée dans la phase.**
/// Un effet à durée : il s'arme à la pose de la carte qui le porte, s'applique
/// à la pose suivante du même joueur dans la même phase, puis disparaît —
/// qu'il ait servi ou non (fin de phase).
///
/// C'est le troisième genre d'effet du moteur : il y avait le PERMANENT (une
/// réduction tant que la carte est en jeu) et l'INSTANTANÉ (appliqué à la pose,
/// terminé) ; celui-ci attend un événement, une seule fois.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NextCardMod {
    /// MC de moins sur la prochaine carte — *Work Crews*, « You pay 11 MC less
    /// for the next card you play this phase. »
    pub discount: i64,
    /// Souplesse d'un palier sur l'oxygène ou la température pour la prochaine
    /// carte — *Special Design*, « For the next card you play this phase, you
    /// may consider the oxygen or temperature one color higher or lower. »
    ///
    /// **Binaire, comme `req_color_flex`** (I3) : deux sources de souplesse ne
    /// s'additionnent jamais en ±2 paliers. `flow::reqs_satisfied` en fait un
    /// « ou », jamais une somme.
    pub color_flex: bool,
}

impl NextCardMod {
    /// Rien d'armé ? Sert à savoir s'il y a quelque chose à consommer.
    pub fn is_empty(&self) -> bool {
        self.discount == 0 && !self.color_flex
    }
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
    /// (lot 6) Bonus de l'action conditionné à la phase choisie par CE joueur
    /// (brique 2). N'a de sens qu'avec `action: Some(Action::Fixed { .. })` —
    /// un test structurel (`lot6_tests`) vérifie qu'aucune entrée de la table ne
    /// le déclare ailleurs, faute de quoi il serait silencieusement inerte.
    pub phase_bonus: Option<PhaseBonus>,
    /// (lot 3) Type de ressource porté par la carte. Une carte porteuse est
    /// INITIALISÉE À 0 à sa pose : elle devient cible valide même vide (règle
    /// du jeu, oracle Java `Player.initResources`).
    pub holds: Option<ResKind>,
    /// (lot 3) Effets à ressources appliqués à la POSE, dans l'ordre du texte
    /// imprimé (après `effects`, avant les déclencheurs de pose).
    pub on_build: &'static [ResStep],
    /// (lot 4) Production DÉRIVÉE de la carte, recalculée à chaque phase IV par
    /// `flow::derived_production`. Rien n'est jamais inscrit sur les pistes
    /// `mc_prod`/`heat_prod`/`plant_prod` : celles-ci restent réservées aux
    /// productions FIXES.
    pub prod: Option<DerivedProd>,
    /// (lot 4) Bonus permanent de phase Recherche, cumulé par
    /// `flow::research_extra`.
    pub research: Option<ResearchBonus>,
    // ======================================================== lot cartes-7
    // Quatre modificateurs PERMANENTS de plus. Aucun ne crée de flux de jeu :
    // chacun change une valeur qu'un service unique du moteur calcule déjà.
    /// (lot cartes-7) MC **supplémentaires** par carte défaussée pour du MC —
    /// *Composting Factory*, « Cards you discard for MC are worth an additional
    /// 1 MC. » Cumulé par le service unique `flow::discard_mc_rate`, qui est le
    /// SEUL point de calcul du taux de défausse du moteur.
    pub discard_bonus: i64,
    /// (lot cartes-7) MC de moins sur les actions standard qui coûtent des MC —
    /// *Standard Technology*, « You pay 4 MC less for standard actions that cost
    /// MC. » Cumulé par le service unique `flow::standard_action_discount` et
    /// appliqué par `flow::standard_mc_cost`, consommé à la fois par
    /// l'affordabilité (`flow::action_options`) et par le paiement
    /// (`flow::phase_action`) — les deux ne peuvent pas diverger (I2).
    ///
    /// Ne touche NI la forêt payée en plantes, NI la température payée en
    /// chaleur, NI la vente de carte : le texte dit « that cost MC ».
    pub standard_discount: i64,
    /// (lot cartes-7) « When playing a card with requirements, you may consider
    /// the oxygen or temperature one color higher or lower. **This cannot be
    /// modified further by other effects.** » — *Adaptation Technology*.
    ///
    /// Même mécanisme que `CorpEffects::req_color_flex` (*Inventrix*), et
    /// **le même booléen** : `flow::reqs_satisfied` en fait un `||`, jamais une
    /// somme. Adaptation Technology + Inventrix = ±1 palier, jamais ±2 (I3) —
    /// c'est l'encodage littéral de la seconde phrase imprimée.
    pub req_color_flex: bool,
    /// (lot cartes-7) « When you use an "Action:" effect on one of your cards,
    /// gain 1 MC. » — *Assembly Lines*.
    ///
    /// Appliqué par `flow::apply_blue_action` APRÈS une activation d'action de
    /// carte bleue qui a réellement produit un effet, via `apply_action_eff` —
    /// le chemin unique des effets d'action. Les actions STANDARD ne passent pas
    /// par là : elles ne déclenchent rien, comme le veut le texte (« on one of
    /// **your cards** »).
    pub action_trigger: &'static [ActionEff],
    // ======================================================== lot cartes-8
    /// (lot cartes-8) Permissions de pose SUPPLÉMENTAIRE accordées au moment
    /// où cette carte entre en jeu, dans la phase en cours. Consommées par
    /// `flow::drain_pending_builds`, qui est le seul endroit du moteur où une
    /// permission supplémentaire s'exerce (I1).
    ///
    /// Un « may » du texte imprimé : la permission est OFFERTE, jamais imposée.
    /// C'est `Policy::choose_build` qui décide de s'en servir ou non — le même
    /// chemin de choix que la pose ordinaire (I4).
    pub grants: &'static [BuildGrant],
    /// (lot cartes-8) Modificateur armé pour la PROCHAINE carte que ce joueur
    /// posera dans la phase en cours. Cumulé par `flow::arm_next_card_mod`,
    /// consommé par `flow::build_card_granted`, effacé en début de phase par
    /// `flow::play_round`.
    pub next_card: Option<NextCardMod>,
    // ==================================================== decouverte-projets
    /// **(decouverte-projets) « Effet : lorsque vous révélez une carte Phase
    /// AMÉLIORÉE, gagnez … »** — *Communications Streamlining* (D05), seule
    /// carte du jeu à le porter.
    ///
    /// C'est le troisième genre d'effet à durée du moteur, après la réduction
    /// permanente et le modificateur de la prochaine carte : un effet levé par
    /// un ÉVÉNEMENT de la boucle de jeu, la révélation d'une carte Phase.
    ///
    /// Levé par `flow::fire_upgraded_reveal`, appelé dans la planification de
    /// `play_round` juste après que le joueur a choisi — c'est-à-dire révélé —
    /// sa carte Phase, **et pour ce joueur seul** : le texte dit « **vous** »
    /// (clause anti-shortcut n° 4). Un joueur ne révèle qu'une carte Phase par
    /// manche : le gain tombe donc au plus une fois par manche et par carte
    /// porteuse en jeu (ASK 4).
    pub reveal_bonus: &'static [Eff],
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
            holds: None, on_build: &[], prod: None, research: None,
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme lot 4a : production DÉRIVÉE (recalculée à chaque phase IV).
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*], prod: $pd:expr) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[], play_triggers: &[], global_triggers: &[], action: None,
            holds: None, on_build: &[], prod: Some($pd), research: None,
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme lot 4b : bonus permanent de phase Recherche.
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*], research: $rb:expr) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[], play_triggers: &[], global_triggers: &[], action: None,
            holds: None, on_build: &[], prod: None, research: Some($rb),
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
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
            holds: None, on_build: &[], prod: None, research: None,
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme lot 6 : action + bonus conditionné à la phase choisie (brique 2).
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*],
     action: $act:expr, phase: $pb:expr) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[], play_triggers: &[], global_triggers: &[],
            action: $act,
            holds: None, on_build: &[], prod: None, research: None,
            phase_bonus: $pb,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme lot 7 : MODIFICATEURS PERMANENTS. Aucune de ces cartes n'a de
    // prérequis ni d'effet immédiat — elles ne font que changer une valeur
    // qu'un service unique du moteur calcule déjà. Les cinq champs sont donc
    // tous exposés, et rien d'autre.
    ($name:literal, red: [$($rd:expr),*], ptrig: [$($pt:expr),*],
     research: $rs:expr, discard: $dbn:expr, standard: $sd:expr,
     flex: $fx:literal, atrig: [$($at:expr),*]) => {
        ($name, CardEffects {
            reqs: &[], effects: &[],
            reductions: &[$($rd),*], play_triggers: &[$($pt),*],
            global_triggers: &[], action: None,
            holds: None, on_build: &[], prod: None, research: $rs,
            phase_bonus: None,
            discard_bonus: $dbn, standard_discount: $sd,
            req_color_flex: $fx, action_trigger: &[$($at),*],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme lot 8 : POSES SUPPLÉMENTAIRES. Les cinq dernières muettes de la
    // boîte de base accordent une pose de plus et/ou arment un modificateur pour
    // la carte suivante ; deux d'entre elles ont aussi une production fixe et
    // une une action bleue. D'où ces quatre champs, et rien d'autre.
    ($name:literal, effects: [$($e:expr),*], grants: [$($g:expr),*],
     next: $nc:expr, action: $act:expr) => {
        ($name, CardEffects {
            reqs: &[], effects: &[$($e),*],
            reductions: &[], play_triggers: &[], global_triggers: &[],
            action: $act,
            holds: None, on_build: &[], prod: None, research: None,
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[$($g),*], next_card: $nc, reveal_bonus: &[],
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
            holds: $h, on_build: &[$($ob),*], prod: None, research: None,
            phase_bonus: None,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[],
        })
    };
    // Forme decouverte-projets : les 28 projets muets de l'extension. Cinq
    // champs suffisent à les décrire toutes — effets immédiats et productions
    // fixes (`effects`), réduction permanente (`red`), action de carte bleue
    // (`action`, avec son éventuel bonus conditionné `phase`), améliorations de
    // carte Phase et alternatives à la pose (`on_build`), et le gain levé par la
    // révélation d'une carte Phase améliorée (`reveal`).
    //
    // Aucun de ces champs n'est neuf sauf le dernier : ce lot ajoute des
    // VARIANTES à des énumérations existantes, pas des énumérations (ALWAYS 6).
    ($name:literal, reqs: [$($r:expr),*], effects: [$($e:expr),*],
     red: [$($rd:expr),*], action: $act:expr, phase: $pb:expr,
     on_build: [$($ob:expr),*], reveal: [$($rv:expr),*]) => {
        ($name, CardEffects {
            reqs: &[$($r),*], effects: &[$($e),*],
            reductions: &[$($rd),*], play_triggers: &[], global_triggers: &[],
            action: $act,
            holds: None, on_build: &[$($ob),*], prod: None, research: None,
            phase_bonus: $pb,
            discard_bonus: 0, standard_discount: 0,
            req_color_flex: false, action_trigger: &[],
            grants: &[], next_card: None, reveal_bonus: &[$($rv),*],
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
/// (lot 6) « a card with a [science] or [plant] » (Advanced Screening Tech).
const T_SCIENCE_PLANT: &[Tag] = &[Tag::Science, Tag::Plant];

/// Pose de `n` ressources sur la carte elle-même.
const fn put_self(n: u32) -> ResEff {
    ResEff::Put(ResPut {
        target: ResTarget::SelfCard,
        kinds: K_ANY,
        amount: ResAmount::Fixed(n),
    })
}

/// (decouverte-projets) « Améliorez **une** carte Phase » — la phase est au
/// choix du joueur. Raccourci de lecture de la table : les quinze cartes de la
/// famille A et trois autres l'emploient mot pour mot.
const UPGRADE_ANY: ResStep = ResStep::Do(ResEff::PhaseUpgrade(None));

/// (decouverte-projets) « Améliorez **votre carte Phase n** » — D05 (III),
/// D37 (I), D40 (IV). La phase vient du carton, la variante reste au joueur.
const fn upgrade_of(phase: u8) -> ResStep {
    ResStep::Do(ResEff::PhaseUpgrade(Some(phase)))
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
/// Permission des trois cartes de phase II : une bleue ou une rouge de plus,
/// sans plafond de prix, payante.
const ONE_MORE_BLUE_OR_RED: BuildGrant = BuildGrant {
    colors: &[Color::Blue, Color::Red],
    max_printed_cost: None,
    free: false,
};
/// Permission des deux cartes de phase I : une verte à 9 MC imprimés ou
/// moins, offerte.
const ONE_FREE_CHEAP_GREEN: BuildGrant = BuildGrant {
    colors: &[Color::Green],
    max_printed_cost: Some(9),
    free: true,
};


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
    // (B) « When you play an Event tag, you gain 2 heat and 2 plants. »
    // (D19) `scale_by_matched_tags: true` — livret de base l. 106 : « Si la
    // condition d'un effet est remplie plusieurs fois lorsqu'une carte est
    // jouée, résolvez l'effet correspondant plusieurs fois. » Le « forfait »
    // annoncé ici était une lecture du portage Java, pas du livret ; les neuf
    // autres déclencheurs de la même famille comptaient déjà les badges.
    card!("Optimal Aerobraking", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Event),
                    gains: &[TrigGain::Heat(2), TrigGain::Plants(2)],
                    scale_by_matched_tags: true, include_self: false }],
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
    // (D19) `scale_by_matched_tags: true`, même raison qu'*Optimal Aerobraking* :
    // livret de base l. 106. Une carte à deux badges Événement fait piocher
    // quatre cartes, pas deux.
    card!("Recycled Detritus", reqs: [], effects: [],
          red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Event),
                    gains: &[TrigGain::Draw(2)], scale_by_matched_tags: true,
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
    //   or remove 3 science to upgrade a phase. »
    //   (D24) L'amélioration EST gérée : `ResEff::PhaseUpgrade(None)` emprunte le
    //   chemin unique d'octroi `flow::apply_phase_upgrade`, avec la source
    //   ACTION — c'est elle qui alimente `phase_upgrades_by_action`. Le
    //   commentaire d'origine annonçait le contraire du code.
    card!("Fibrous Composite Material", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Res(&[
              &[put_self(1)],
              &[ResEff::RemoveSelf(3), ResEff::PhaseUpgrade(None)],
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
                    scale_by_matched_tags: true, include_self: true }],
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
                    scale_by_matched_tags: true, include_self: true }],
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
    // (D24) L'amélioration EST gérée : `ResEff::PhaseUpgrade(None)` posé en
    // `on_build` passe par `flow::apply_phase_upgrade` (source BUILD). Rien
    // n'est sauté, `phase_upgrades_skipped` ne bouge pas — mesuré nul sur deux
    // mille parties. Le commentaire d'origine annonçait le contraire du code.
    card!("Cryogenic Shipment", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [], action: None, holds: None,
          on_build: [ResStep::Do(ResEff::PhaseUpgrade(None)),
                     ResStep::Do(ResEff::Put(ResPut { target: ResTarget::Another,
                         kinds: K_MICROBE_ANIMAL,
                         amount: ResAmount::ByKind { microbe: 3, other: 2 } }))]),
    // « Requires an Animal, Microbe, and Plant tags. » (3 VP fixes dans
    //  cards.json ; aucune ressource — carte de complément.)
    card!("Advanced Ecosystems",
          reqs: [Tags(Tag::Animal, 1), Tags(Tag::Microbe, 1), Tags(Tag::Plant, 1)],
          effects: []),

    // ================================================= LOT 4 (chantier cartes-4)
    // Productions DÉRIVÉES (« 1 <ressource> par <badge> que vous avez »), hausse
    // de NT par badge, bonus permanent de phase Recherche. Rien de tout cela ne
    // touche les pistes `*_prod` : la production dérivée est RECALCULÉE à chaque
    // phase IV par `flow::derived_production` (livret FR p.13 l.180).
    // Correspondances et lectures : `outputs/lot4.md`.

    // ---- 14 productions dérivées -------------------------------------------
    // « …produces 1 heat per Earth you have, including this. »
    card!("Atmospheric Insulators", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Heat, count: ProdCount::Tag(Tag::Earth),
                              per: 1 }),
    // « …produces 1 MC per Earth tag you have, including this. »
    card!("Cartel", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Earth),
                              per: 1 }),
    // « …produces 1 plant per Plant you have. » — la carte porte un badge
    // MICROBE et compte les badges PLANTE, qu'elle n'a pas : seule, elle ne
    // produit rien (scan n° 152, livret-extrait §Insects).
    card!("Insects", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Plants, count: ProdCount::Tag(Tag::Plant),
                              per: 1 }),
    // « …produces 1 MC per Science tag you have, including this. »
    card!("Lightning Harvest", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Science),
                              per: 1 }),
    // « …produces 1 MC per 2 Building you have, including this. » — division
    // ENTIÈRE : un seul badge Construction ne rapporte rien.
    card!("Medical Lab", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Building),
                              per: 2 }),
    // « …produces 1 MC per Microbe tag you have, including this. »
    card!("Microbiology Patents", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Microbe),
                              per: 1 }),
    // « …produces 1 MC per Earth tag you have, including this. »
    card!("Miranda Resort", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Earth),
                              per: 1 }),
    // « …produces 1 MC per Energy you have, including this. »
    card!("Power Grid", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Energy),
                              per: 1 }),
    // « …produces 1 heat per Space you have, including this. » (nom orthographié
    // « Sattellite Farms » dans cards.json — faute d'origine, clé de résolution.)
    card!("Sattellite Farms", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Heat, count: ProdCount::Tag(Tag::Space),
                              per: 1 }),
    // « …produces 1 MC per Space you have, including this. »
    card!("Satellites", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Space),
                              per: 1 }),
    // « …produces 1 MC per Event you have. » (la carte ne porte aucun badge.)
    card!("Venture Capitalism", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Tag(Tag::Event),
                              per: 1 }),
    // « …produces 1 heat per Energy tag you have, including this. » — le scan de
    // la carte n° 206 porte « including this », que cards.json omet : sans effet,
    // le calcul est uniforme.
    card!("Windmills", reqs: [], effects: [],
          prod: DerivedProd { res: ProdRes::Heat, count: ProdCount::Tag(Tag::Energy),
                              per: 1 }),
    // « Requires red oxygen or higher. …produces 1 plant per Microbe tag you
    //   have, including this. »
    card!("Worms", reqs: [OxyMin(OXY_R_MIN)], effects: [],
          prod: DerivedProd { res: ProdRes::Plants, count: ProdCount::Tag(Tag::Microbe),
                              per: 1 }),
    // « Requires red oxygen or higher. …produces 1 MC per forest VP you have. »
    // Des jetons FORÊT, pas des badges (scan n° 208).
    card!("Zeppelins", reqs: [OxyMin(OXY_R_MIN)], effects: [],
          prod: DerivedProd { res: ProdRes::Mc, count: ProdCount::Forests, per: 1 }),

    // ---- les 3 autres -------------------------------------------------------
    // « …produces 3 MC. 1 VP per 2 Earth tags you have. » Production FIXE :
    // piste `mc_prod`, comme une carte verte ordinaire. Ses PV variables sont
    // déjà calculés par `flow::card_points` (vp_dynamic EARTH 1/2) : rien ici.
    card!("Immigration Shuttles", reqs: [], effects: [McProd(3)]),
    // « Raise your TR 1 step per Jupiter tag you have, including this. »
    card!("Terraforming Ganymede", reqs: [], effects: [TrPerTag(Tag::Jupiter)]),
    // « When you draw cards during the research phase, draw one additional card
    //   and keep one additional card. 1 VP per 4 cards you have played. »
    // Les PV sont déjà calculés par `card_points` (vp_dynamic ANY_CARD 1/4).
    card!("Interplanetary Relations", reqs: [], effects: [],
          research: ResearchBonus { draw: 1, keep: 1 }),

    // ================================================= LOT 5 (chantier cartes-5)
    // 33 cartes MUETTES de la boîte de base rendues vivantes. Source du texte :
    // `data/cartes-imprimees/textes-cartes.json` champ `text` — la transcription des cartons —
    // JAMAIS le champ `description` de `cards.json`. Correspondances carte par
    // carte, texte imprimé cité et traces de sonde : `outputs/cartes5.md`.
    //
    // Aucune de ces cartes n'a demandé de mécanisme neuf hors deux briques :
    // `Eff::Forest(n)` (groupe C) et `Req::TrMin(n)` (Energy Storage — divergence
    // vs contrat, journal D1).

    // ---- Groupe A : production seule (20) -----------------------------------
    // Toutes disent « During the production phase, this produces … » : ce sont
    // des hausses de PISTE (`*Prod`), consommées à CHAQUE phase IV, jamais des
    // gains immédiats. « you draw a card » à l'intérieur de cette phrase est une
    // production de CARTES (`CardProd`), pas un `Draw` à la pose (journal D6).

    // « Requires a [jupiter]. During the production phase, this produces 1 plant
    //   and 3 heat. »
    card!("Beam from a Thorium Asteroid", reqs: [Tags(Tag::Jupiter, 1)],
          effects: [PlantProd(1), HeatProd(3)]),
    // « During the production phase, draw a card. »
    card!("Callisto Penal Mines", reqs: [], effects: [CardProd(1)]),
    // « Requires you to have 7 or more TR. During the production phase, draw two
    //   cards. » — seule carte du lot à porter un seuil de NT (`Req::TrMin`).
    card!("Energy Storage", reqs: [TrMin(7)], effects: [CardProd(2)]),
    // « During the production phase, this produces 3 heat. »
    card!("Giant Space Mirror", reqs: [], effects: [HeatProd(3)]),
    // « During the production phase, this produces 2 heat. »
    card!("Import of Advanced GHG", reqs: [], effects: [HeatProd(2)]),
    // « Requires red oxygen or higher. …produces 1 MC and 2 heat. »
    card!("Low-Atmo Shields", reqs: [OxyMin(OXY_R_MIN)],
          effects: [McProd(1), HeatProd(2)]),
    // « Requires red oxygen or higher. …produces 2 plants and 2 heat. »
    card!("Methane from Titan", reqs: [OxyMin(OXY_R_MIN)],
          effects: [PlantProd(2), HeatProd(2)]),
    // « Requires red oxygen or higher. …produces 2 MC. »
    card!("Natural Preserve", reqs: [OxyMin(OXY_R_MIN)], effects: [McProd(2)]),
    // « …produces 1 MC, 1 plant, and 1 heat. »
    card!("New Portfolios", reqs: [],
          effects: [McProd(1), PlantProd(1), HeatProd(1)]),
    // « …produces 2 plants. »
    card!("Nitropholic Moss", reqs: [], effects: [PlantProd(2)]),
    // « …produces 1 MC and 3 heat. » (PV imprimé −1 : donnée de cards.json,
    //   déjà comptée au score, rien à encoder ici.)
    card!("Nuclear Plants", reqs: [], effects: [McProd(1), HeatProd(3)]),
    // « …produces 1 heat. »
    card!("Power Plant", reqs: [], effects: [HeatProd(1)]),
    // « …produces 2 MC and 1 heat. »
    card!("Power Supply Consortium", reqs: [], effects: [McProd(2), HeatProd(1)]),
    // « Requires 3 [science]. …produces 3 heat. » (la bande de phase imprimée dit
    //   « I-II », champ que le moteur ne lit nulle part — journal D5.)
    card!("Quantum Extractor", reqs: [Tags(Tag::Science, 3)], effects: [HeatProd(3)]),
    // « Requires 2 ocean tiles to be flipped. …produces 2 MC. »
    card!("Rad Suits", reqs: [OceanMin(2)], effects: [McProd(2)]),
    // « …produces 2 plants. » (PV imprimé −1, donnée de cards.json.)
    card!("Slash and Burn Agriculture", reqs: [], effects: [PlantProd(2)]),
    // « …produces 1 heat. »
    card!("Solar Power", reqs: [], effects: [HeatProd(1)]),
    // « …produces 5 heat. »
    card!("Soletta", reqs: [], effects: [HeatProd(5)]),
    // « …produces 3 heat. »
    card!("Tectonic Stress Power", reqs: [], effects: [HeatProd(3)]),
    // « During the production phase, you draw a card and this produces 4 heat. »
    //   La pioche est DANS la phase de production → `CardProd(1)` (journal D6).
    card!("Undersea Vents", reqs: [], effects: [CardProd(1), HeatProd(4)]),

    // ---- Groupe B : effet immédiat, éventuellement suivi d'une production (9) -
    // « [effect] … » = gain à la POSE. Quand la carte porte AUSSI « During the
    // production phase … », les deux coexistent : l'ordre de la table suit
    // l'ordre du texte imprimé.

    // « [effect] Draw a card. »
    card!("Lagrange Observatory", reqs: [], effects: [Draw(1)]),
    // « Requires white temperature. [effect] Flip an ocean tile. »
    card!("Ice Cap Melting", reqs: [TempMin(TEMP_W_MIN)], effects: [Ocean(1)]),
    // « Requires yellow temperature or warmer. [effect] Flip an ocean tile. »
    card!("Permafrost Extraction", reqs: [TempMin(TEMP_Y_MIN)], effects: [Ocean(1)]),
    // « Requires yellow temperature or warmer. [effect] Flip 2 ocean tiles. »
    card!("Lake Mariners", reqs: [TempMin(TEMP_Y_MIN)], effects: [Ocean(2)]),
    // « [effect] Flip an ocean tile. [effect] Draw two cards. »
    card!("Technology Demonstration", reqs: [], effects: [Ocean(1), Draw(2)]),
    // « Requires red temperature or warmer. [effect] Flip an ocean tile. During
    //   the production phase, this produces 2 heat. »
    card!("Trapped Heat", reqs: [TempMin(TEMP_R_MIN)], effects: [Ocean(1), HeatProd(2)]),
    // « [effect] Raise the temperature 1 step. [effect] Flip an ocean tile.
    //   [effect] Draw two cards. » (trois lignes d'effet, dans cet ordre.)
    card!("Phobos Falls", reqs: [], effects: [Temperature(1), Ocean(1), Draw(2)]),
    // « [effect] Gain 3 plants. During the production phase, this produces 2 MC. »
    card!("Trading Post", reqs: [], effects: [Plants(3), McProd(2)]),
    // « Requires red temperature or warmer. [effect] Gain 2 plants. During the
    //   production phase, this produces 1 plant. »
    card!("Noctis Farming", reqs: [TempMin(TEMP_R_MIN)],
          effects: [Plants(2), PlantProd(1)]),

    // ---- Groupe C : gain de forêt (4) ---------------------------------------
    // « Gain a forest VP and raise oxygen 1 step » n'est PAS deux effets : c'est
    // la description d'un gain de forêt, mot pour mot la formule de l'action
    // standard du livret (p. 14, l. 379). Un seul `Eff::Forest(n)` — le rapport
    // forêts/oxygène est de 1 pour 1 sur les quatre cartes (R1, journal D2), et
    // le gain lève « when you gain a forest VP » n fois (R2, journal D3).

    // « Requires white temperature. [effect] Gain a forest VP and raise oxygen
    //   1 step. »
    card!("Mangrove", reqs: [TempMin(TEMP_W_MIN)], effects: [Forest(1)]),
    // « Requires 4 [science]. [effect] Gain 2 forest VPs and raise oxygen
    //   2 steps. » → 2 forêts, 2 pas d'oxygène. JAMAIS 4.
    card!("Plantation", reqs: [Tags(Tag::Science, 4)], effects: [Forest(2)]),
    // « [effect] Gain a forest VP and raise oxygen 1 step. During the production
    //   phase, this produces 2 MC. »
    card!("Protected Valley", reqs: [], effects: [Forest(1), McProd(2)]),
    // « [effect] Gain a forest VP and raise oxygen 1 step. During the production
    //   phase, this produces 1 heat. »
    card!("Biothermal Power", reqs: [], effects: [Forest(1), HeatProd(1)]),

    // ================================================= LOT 6 (chantier cartes-6)
    // Les 11 cartes muettes qui tournent autour de « ce que le joueur active
    // pendant son tour » et de « ce qu'il fait de sa main ». Source du texte :
    // `data/cartes-imprimees/textes-cartes.json`, champs `text`, `requirement`, `production` et
    // `vp_printed` — JAMAIS le champ `description` de `cards.json`. Texte cité
    // carte par carte et traces de sonde : `outputs/cartes6.md`.
    //
    // Six briques ajoutées au vocabulaire, aucune ligne de logique par carte :
    // `Req::OxyMax`, `CardEffects::phase_bonus`, `ActionCost::DiscardCard`,
    // `Action::SpendUpTo`, `Eff::DrawDiscard`, `ActionEff::Reveal`.

    // ---- Groupe A : bonus si vous avez choisi la phase Action (2) ------------
    // « Action: Gain 2 MC. *If you chose the action phase this round, also gain
    //   1 plant. » — le bonus AJOUTE un effet, le coût (nul) ne change pas.
    card!("Community Gardens", reqs: [], effects: [],
          action: Some(Action::Fixed { cost: &[], effect: &[ActionEff::Mc(2)] }),
          phase: Some(PhaseBonus { phase: 3, require_upgraded: false, cost: None,
                    extra: &[ActionEff::Plants(1)] })),
    // « Action: Spend 1 MC to gain 2 heat. *If you chose the action phase this
    //   round, gain 1 additional heat. » — « additional » : 2 + 1 = 3 chaleurs,
    //   le MC dépensé reste 1.
    card!("Hydro-Electric Energy", reqs: [], effects: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Mc(1)],
                    effect: &[ActionEff::Heat(2)] }),
          phase: Some(PhaseBonus { phase: 3, require_upgraded: false, cost: None,
                    extra: &[ActionEff::Heat(1)] })),

    // ---- Groupe B : actions à coût particulier (3) --------------------------
    // « [effect] Gain 3 plants. Action: Discard a card in hand to gain
    //   3 plants. » — le gain de pose et le gain d'action sont distincts.
    card!("Farming Co-ops", reqs: [], effects: [Plants(3)],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[ActionCost::DiscardCard(1)],
                    effect: &[ActionEff::Plants(3)] })),
    // « [effect] Gain 4 plants. Action: Spend 4 plants to raise the temperature
    //   1 step. *If you chose the action phase this round, spend 3 plants
    //   instead. » — le bonus REMPLACE le coût (4 → 3), il n'ajoute rien.
    card!("Wood Burning Stoves", reqs: [], effects: [Plants(4)],
          action: Some(Action::Fixed { cost: &[ActionCost::Plants(4)],
                    effect: &[ActionEff::Temperature(1)] }),
          phase: Some(PhaseBonus { phase: 3, require_upgraded: false,
                    cost: Some(&[ActionCost::Plants(3)]), extra: &[] })),
    // « Requires yellow temperature or warmer. Action: Spend up to 4 heat to
    //   gain that amount of plants. » — prérequis imprimé jusqu'ici NON appliqué
    //   (l'un des deux trous de la boîte de base, fermé par ce lot).
    card!("Greenhouses", reqs: [TempMin(TEMP_Y_MIN)], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::SpendUpTo { spend: ActionRes::Heat,
                    gain: ActionRes::Plants, cap: 4 })),

    // ---- Groupe C : piocher puis défausser (3) ------------------------------
    // UNE seule brique pour les trois (I3) : `Eff::DrawDiscard`.
    // « [effect] Draw four cards. Then, discard two cards. » (net +2)
    card!("Business Contracts", reqs: [],
          effects: [DrawDiscard { draw: 4, discard: 2, from_drawn: false }]),
    // « [effect] Draw three cards. Keep one of them and discard the other
    //   two. » (net +1) — « of them » restreint la défausse aux cartes piochées.
    card!("Invention Contest", reqs: [],
          effects: [DrawDiscard { draw: 3, discard: 2, from_drawn: true }]),
    // « [effect] Draw two cards. Then, discard a card. During the production
    //   phase, this produces 3 heat. » (net +1, puis production FIXE de 3
    //   chaleurs — champ `production: "3 heat"` du texte imprimé.)
    card!("Microprocessors", reqs: [],
          effects: [DrawDiscard { draw: 2, discard: 1, from_drawn: false },
                    HeatProd(3)]),

    // ---- Groupe D : révéler le dessus de la pioche (2) ----------------------
    // « Action: Reveal the top three cards of the deck. Place a card with a
    //   [science] or [plant] revealed this way into your hand. Discard the
    //   rest. »
    card!("Advanced Screening Tech", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[],
                    effect: &[ActionEff::Reveal(Reveal {
                        n: 3,
                        keep: RevealFilter::AnyOfTags(T_SCIENCE_PLANT),
                        take: 1,
                        mc_per_discarded: 0 })] })),
    // « Action: Reveal the top card of the deck. If it is green, discard it and
    //   gain 1 MC. Otherwise, draw it. »
    card!("Brainstorming Session", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed { cost: &[],
                    effect: &[ActionEff::Reveal(Reveal {
                        n: 1,
                        keep: RevealFilter::ColorIsNot(Color::Green),
                        take: 1,
                        mc_per_discarded: 1 })] })),

    // ---- Groupe E : prérequis seul (1) --------------------------------------
    // « Requires red oxygen or lower. » AUCUN texte d'effet ; 2 PV imprimés
    // (donnée `vp` de cards.json, déjà comptée au score). Son encodage est donc
    // exactement sa condition d'entrée — et c'est le second trou de prérequis
    // de la boîte de base, fermé ici.
    card!("Colonizer Training Camp", reqs: [OxyMax(OXY_R_MAX)], effects: []),

    // ============================================ LOT ACIER-TITANE (4 cartes)
    //
    // Les quatre cartes de la boîte de base dont le texte parle d'un NOMBRE
    // d'aciers ou de titanes. Elles étaient muettes pour cette seule raison :
    // le nombre n'existait pas. Il existe désormais (`flow::capacities`), et
    // leur encodage n'est que la transcription de leur texte imprimé
    // (`data/cartes-imprimees/textes-cartes.json`, champs `text` / `requirement` / `notes` —
    // aucune des quatre ne porte de prérequis).
    //
    // Les quatre sont BLEUES : elles se servent des savoir-faire des autres,
    // elles n'en sont pas (I4).

    // « Effect: Each titanium you have reduces the cost of [space] cards an
    // additional 1 MC. Each steel you have reduces the cost of [building] cards
    // an additional 1 MC. » — un EFFET permanent, pas une action : rien ne se
    // passe à la pose, tout se voit sur le prix des cartes jouées ensuite.
    card!("Advanced Alloys", reqs: [], effects: [],
          red: [Reduction::PerCapacity { tag: Tag::Space, cap: Capacity::Titanium, per: 1 },
                Reduction::PerCapacity { tag: Tag::Building, cap: Capacity::Steel, per: 1 }],
          ptrig: [], gtrig: [], action: None),

    // « Action: Spend 10 MC to flip an ocean tile. Reduce this by 2 MC per steel
    // you have. »
    card!("Aquifer Pumping", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed {
              cost: &[ActionCost::McPerCapacity { base: 10, cap: Capacity::Steel, per: 2 }],
              effect: &[ActionEff::Ocean(1)] })),

    // « Action: Spend 15 MC to gain a forest VP and raise oxygen 1 step. Reduce
    // this by 2 MC per titanium you have. » — « and raise oxygen 1 step »
    // décrit la forêt, l'oxygène ne monte pas deux fois (règle R1 du lot 5).
    card!("Solarpunk", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed {
              cost: &[ActionCost::McPerCapacity { base: 15, cap: Capacity::Titanium, per: 2 }],
              effect: &[ActionEff::Forest(1)] })),

    // « *=1 VP per [jupiter] you have. Action: Spend 12 MC to flip an ocean
    // tile. Reduce this by 1 MC per titanium you have. » — les PV par badge
    // Jupiter sont déjà servis par `VpKind::Jupiter` (donnée `vp_dynamic` de
    // cards.json) : seule l'action restait à encoder.
    card!("Water Import from Europa", reqs: [], effects: [],
          red: [], ptrig: [], gtrig: [],
          action: Some(Action::Fixed {
              cost: &[ActionCost::McPerCapacity { base: 12, cap: Capacity::Titanium, per: 1 }],
              effect: &[ActionEff::Ocean(1)] })),

    // ================================================= LOT 7 (chantier cartes-7)
    // Les 9 dernières cartes muettes de la boîte de base qui soient des
    // MODIFICATEURS PERMANENTS : aucune ne crée de flux de jeu, chacune change
    // une valeur qu'un service unique du moteur calcule déjà. Source du texte :
    // `data/cartes-imprimees/textes-cartes.json` champ `text` — jamais le champ `description`
    // de `cards.json`. Traces de sonde : `outputs/cartes-7.md`.

    // ---- Groupe A : la phase de recherche (3) -------------------------------
    // Trois LIGNES DE TABLE, pas trois mécanismes : `ResearchBonus` existe
    // depuis le lot 4, il est servi par le service unique `flow::research_extra`
    // et déjà porté par *Interplanetary Relations* et *Tharsis Republic*.

    // « Effect: When you draw cards during the research phase, draw TWO
    //   additional cards. » — pioche seule, aucune carte gardée en plus.
    card!("Interns", red: [], ptrig: [],
          research: Some(ResearchBonus { draw: 2, keep: 0 }),
          discard: 0, standard: 0, flex: false, atrig: []),
    // « Effect: When you KEEP cards during the research phase, keep ONE
    //   additional card. » — le texte parle de garder, pas de piocher.
    card!("Extended Resources", red: [], ptrig: [],
          research: Some(ResearchBonus { draw: 0, keep: 1 }),
          discard: 0, standard: 0, flex: false, atrig: []),
    // « Effect: When you draw cards during the research phase, draw one
    //   additional card and keep one additional card. » (texte mot pour mot
    //   identique à *Interplanetary Relations*.)
    card!("United Planetary Alliance", red: [], ptrig: [],
          research: Some(ResearchBonus { draw: 1, keep: 1 }),
          discard: 0, standard: 0, flex: false, atrig: []),

    // ---- Groupe B : le prix payé (3) ----------------------------------------

    // « Effect: Cards you discard for MC are worth an additional 1 MC. »
    // Le taux de base (3 MC, livret l. 96 / 310 / 348 / 437) devient une valeur
    // CALCULÉE par `flow::discard_mc_rate`, jamais une constante lue nue.
    card!("Composting Factory", red: [], ptrig: [],
          research: None, discard: 1, standard: 0, flex: false, atrig: []),
    // « Effect: You pay 4 MC less for standard actions that cost MC. »
    // Exactement les trois actions standard payantes en MC (forêt 20,
    // température 14, océan 15) — ni la forêt en plantes, ni la température en
    // chaleur, ni la vente de carte (qui rapporte). Voir `flow::standard_mc_cost`.
    card!("Standard Technology", red: [], ptrig: [],
          research: None, discard: 0, standard: 4, flex: false, atrig: []),
    // « Effect: When you play a card, you may spend 1 plant to reduce that
    //   card's cost by 5 MC. » Le « may » passe par `Policy::choose_option`.
    card!("Restructured Resources", red: [Reduction::PayPlants { plants: 1, amount: 5 }],
          ptrig: [], research: None, discard: 0, standard: 0, flex: false, atrig: []),

    // ---- Groupe C : les déclencheurs (3) ------------------------------------

    // « Effect: When playing a card with requirements, you may consider the
    //   oxygen or temperature one color higher or lower. This cannot be modified
    //   further by other effects. » — même booléen que le `req_color_flex`
    //   d'*Inventrix* : un `||`, donc ±1 palier même réunis (I3).
    card!("Adaptation Technology", red: [], ptrig: [],
          research: None, discard: 0, standard: 0, flex: true, atrig: []),
    // « Effect: When you use an "Action:" effect on one of your cards, gain
    //   1 MC. » — l'action d'une carte bleue en phase III, jamais une action
    //   standard.
    card!("Assembly Lines", red: [], ptrig: [],
          research: None, discard: 0, standard: 0, flex: false,
          atrig: [ActionEff::Mc(1)]),
    // « Effect: When you play a [science], INCLUDING THIS, you may discard a
    //   card. If that card had a [plant], draw two cards. Otherwise, draw a
    //   card. » — même forme qu'*Olympus Conference* (« When you play a
    //   [science], including this, … ») : `include_self` et une résolution par
    //   badge science satisfaisant (livret p.9 l.106).
    card!("Mars University", red: [],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Science),
                    gains: &[TrigGain::MayDiscardDraw { if_tag: Tag::Plant,
                              draw_if: 2, draw_else: 1 }],
                    scale_by_matched_tags: true, include_self: true }],
          research: None, discard: 0, standard: 0, flex: false, atrig: []),

    // ================================================== lot cartes-8
    // LES CINQ DERNIÈRES MUETTES DE LA BOÎTE DE BASE — « une carte de plus ».
    //
    // Trois d'entre elles s'expriment mot pour mot pareil (« You may play an
    // additional blue or red card this phase ») et deux autres mot pour mot
    // pareil (« You may play a green card … 9 MC or less … without paying its
    // MC cost ») : d'où DEUX permissions nommées, écrites une fois.

    // « [effect] You may play an additional blue or red card this phase.
    //   Action: Spend 1 TR to draw three cards. » — la seule des cinq à porter
    //   aussi une action, et le premier coût en NT du moteur.
    card!("Asset Liquidation", effects: [],
          grants: [ONE_MORE_BLUE_OR_RED], next: None,
          action: Some(Action::Fixed { cost: &[ActionCost::Tr(1)],
                                       effect: &[ActionEff::Draw(3)] })),
    // « [effect] You may play an additional blue or red card this phase.
    //   [effect] For the next card you play this phase, you may consider the
    //   oxygen or temperature one color higher or lower. » — deux effets
    //   distincts sur la même carte : une permission ET un modificateur armé.
    card!("Special Design", effects: [],
          grants: [ONE_MORE_BLUE_OR_RED],
          next: Some(NextCardMod { discount: 0, color_flex: true }),
          action: None),
    // « [effect] You may play an additional blue or red card this phase.
    //   [effect] You pay 11 MC less for the next card you play this phase. »
    card!("Work Crews", effects: [],
          grants: [ONE_MORE_BLUE_OR_RED],
          next: Some(NextCardMod { discount: 11, color_flex: false }),
          action: None),
    // « You may play a green card from your hand that has a printed cost of
    //   9 MC or less without paying its MC cost. During the production phase,
    //   draw a card. » — la production de cartes est une production FIXE, du
    //   même genre que celle d'*Acquired Company*.
    card!("Automated Factories", effects: [CardProd(1)],
          grants: [ONE_FREE_CHEAP_GREEN], next: None, action: None),
    // « …without paying its MC cost. During the production phase, this
    //   produces 3 MC. »
    card!("Tall Station", effects: [McProd(3)],
          grants: [ONE_FREE_CHEAP_GREEN], next: None, action: None),

    // =========================================================================
    // (decouverte-projets) LES 28 DERNIERS PROJETS MUETS DE L'EXTENSION
    //
    // Source du texte : `data/cartes-imprimees/projets-decouverte/projets-decouverte.json`, transcription à
    // l'image des cartons — jamais le champ `description` de `cards.json`. Le
    // code `Dnn` est celui du carton. Quand le carton et `cards.json` divergent,
    // le carton gagne, et la divergence est déclarée dans `result.md`.
    //
    // Trois conventions de lecture, appliquées partout :
    //
    // 1. L'ORDRE du texte imprimé est celui de `on_build` — c'est à cela que
    //    sert `ResEff::Gain`. « Améliorez une carte Phase. Piochez une carte. »
    //    n'est pas « Piochez une carte. Améliorez une carte Phase. » : la
    //    seconde pioche aurait lieu avant que le joueur ait vu son amélioration.
    // 2. Une PRODUCTION (encart « Lors de la phase de production… ») n'est pas
    //    un effet immédiat : elle va dans `effects`, sur les pistes fixes que la
    //    phase IV encaisse à chaque génération.
    // 3. Les PRÉREQUIS imprimés sont encodés même quand le contrat ne les cite
    //    pas (D12, D17, D19) : une carte à moitié encodée serait un stub
    //    étiqueté.
    // =========================================================================

    // ---- A. Amélioration au CHOIX + un effet déjà connu (15) ----------------
    // « Améliorez une carte Phase. Effet : lorsque vous jouez une carte, le coût
    //   associé est réduit de 1 MC. » (D09, bleue)
    card!("Hohmann Transfer Shipping", reqs: [], effects: [],
          red: [Reduction::AnyCard(1)], action: None, phase: None,
          on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. Piochez une carte. » (D16, rouge)
    card!("Exosuits", reqs: [], effects: [],
          red: [], action: None, phase: None,
          on_build: [UPGRADE_ANY, ResStep::Do(ResEff::Gain(Draw(1)))], reveal: []),
    // « Améliorez DEUX cartes Phase. » + « Requiert un niveau de température
    //   jaune ou plus chaud. » (D17, rouge) — SEULE carte de l'extension à
    //   accorder deux améliorations d'un coup. Deux étapes distinctes : deux
    //   décisions du joueur, chacune libre (ASK 1).
    card!("Imported Construction Crews", reqs: [TempMin(TEMP_Y_MIN)], effects: [],
          red: [], action: None, phase: None,
          on_build: [UPGRADE_ANY, UPGRADE_ANY], reveal: []),
    // « Augmentez la température de 2 niveaux. Piochez deux cartes. Améliorez
    //   une carte Phase. » (D18, rouge)
    card!("Ore Leaching", reqs: [], effects: [],
          red: [], action: None, phase: None,
          on_build: [ResStep::Do(ResEff::Gain(Temperature(2))),
                     ResStep::Do(ResEff::Gain(Draw(2))),
                     UPGRADE_ANY],
          reveal: []),
    // « Améliorez une carte Phase. » + production : 2 plantes. (D22, verte)
    card!("Biofoundries", reqs: [], effects: [PlantProd(2)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. Lorsque vous jouez un badge bâtiment, le coût
    //   associé est réduit de 2 MC. » (D23, verte — savoir-faire acier ×1)
    card!("Blast Furnaces", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Building, 2)], action: None, phase: None,
          on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 2 MC et 1 chaleur. (D27)
    card!("Manufacturing Hub", reqs: [], effects: [McProd(2), HeatProd(1)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 1 chaleur. (D28)
    card!("Heat Reflective Glass", reqs: [], effects: [HeatProd(1)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez UNE carte Phase. » + production : 3 MC et 1 plante. (D30)
    // ASK 2 : l'exemplaire physique d'Alexis porte un « 2 » écrit à la main
    // par-dessus le mot « une ». Décision du 27-07 : on suit le TEXTE D'ORIGINE
    // — UNE seule amélioration. Cette carte n'est donc PAS un second D17.
    card!("Hydroponic Gardens", reqs: [], effects: [McProd(3), PlantProd(1)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 4 chaleurs. (D32)
    card!("Industrial Complex", reqs: [], effects: [HeatProd(4)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 1 MC. (D33)
    card!("Martian Museum", reqs: [], effects: [McProd(1)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. Lorsque vous jouez un badge espace, le coût
    //   associé est réduit de 3 MC. » (D34, verte — savoir-faire titane ×1)
    card!("Metallurgy", reqs: [], effects: [],
          red: [Reduction::Tag(Tag::Space, 3)], action: None, phase: None,
          on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 2 chaleurs. (D36)
    card!("Oxidation Byproducts", reqs: [], effects: [HeatProd(2)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 1 plante. (D38)
    card!("Magnetic Field Generator", reqs: [], effects: [PlantProd(1)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Améliorez une carte Phase. » + production : 2 MC. (D42)
    card!("Warehouses", reqs: [], effects: [McProd(2)],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),

    // ---- B. Amélioration d'une phase IMPOSÉE (3) ----------------------------
    // La phase est imposée par le carton, la VARIANTE reste au joueur : c'est
    // le paramètre de `ResEff::PhaseUpgrade`, pas trois cas dans le flux.
    //
    // « Améliorez votre carte Phase III. Effet : lorsque vous révélez une carte
    //   Phase améliorée, gagnez 1 MC. » (D05, BLEUE — sa couleur est l'une des
    //   sept corrigées par ce chantier ; il lui faut rester en jeu pour que son
    //   « Effet : » permanent existe.)
    card!("Communications Streamlining", reqs: [], effects: [],
          red: [], action: None, phase: None,
          on_build: [upgrade_of(3)], reveal: [Mc(1)]),
    // « Améliorez votre carte Phase I. » + production : 1 chaleur. (D37)
    card!("Perfluorocarbon Production", reqs: [], effects: [HeatProd(1)],
          red: [], action: None, phase: None,
          on_build: [upgrade_of(1)], reveal: []),
    // « Améliorez votre carte Phase IV. » + production : 1 plante. (D40)
    card!("Biological Factories", reqs: [], effects: [PlantProd(1)],
          red: [], action: None, phase: None,
          on_build: [upgrade_of(4)], reveal: []),

    // ---- C. Amélioration par une ACTION de carte bleue (2) ------------------
    // « Action : Dépensez 1 NT pour améliorer une carte Phase. » (D07)
    // Le coût existait (`ActionCost::Tr(1)`, lot cartes-8) ; l'effet est neuf.
    card!("Experimental Technology", reqs: [], effects: [],
          red: [],
          action: Some(Action::Fixed { cost: &[ActionCost::Tr(1)],
                                       effect: &[ActionEff::PhaseUpgrade] }),
          phase: None, on_build: [], reveal: []),
    // « Action : Améliorez une carte Phase. » + « Requiert 3 badges science. »
    //   (D12 — le prérequis est imprimé, le contrat ne le cite pas.)
    card!("Virtual Employee Development", reqs: [Tags(Tag::Science, 3)], effects: [],
          red: [],
          action: Some(Action::Fixed { cost: &[],
                                       effect: &[ActionEff::PhaseUpgrade] }),
          phase: None, on_build: [], reveal: []),

    // ---- D. Bonus lié aux cartes Phase améliorées (2, dont D05 ci-dessus) ---
    // « Action : Gagnez 2 MC. *Si vous jouez une carte Phase améliorée lors de
    //   cette manche, gagnez 2 MC supplémentaires. » (D06)
    // Le supplément est un bonus d'action conditionné : la brique existe
    // (`PhaseBonus`), sa condition est neuve (`require_upgraded`). `phase: 0` =
    // le carton ne nomme aucune phase.
    card!("Drone Assisted Construction", reqs: [], effects: [],
          red: [],
          action: Some(Action::Fixed { cost: &[], effect: &[ActionEff::Mc(2)] }),
          phase: Some(PhaseBonus { phase: 0, require_upgraded: true,
                                   cost: None, extra: &[ActionEff::Mc(2)] }),
          on_build: [], reveal: []),

    // ---- E. Réduction de coût par badge (1) --------------------------------
    // « Lorsque vous jouez un badge bâtiment, le coût associé est réduit de
    //   2 MC. » + production : piochez deux cartes. (D29 — savoir-faire acier ×1)
    card!("Hematite Mining", reqs: [], effects: [CardProd(2)],
          red: [Reduction::Tag(Tag::Building, 2)], action: None, phase: None,
          on_build: [], reveal: []),

    // ---- F. Conditions et effets isolés (6) --------------------------------
    // « Améliorez une carte Phase. Action : Piochez deux cartes. Puis,
    //   défaussez deux cartes. » (D11)
    card!("Software Streamlining", reqs: [], effects: [],
          red: [],
          action: Some(Action::Fixed {
              cost: &[],
              effect: &[ActionEff::DrawDiscard { draw: 2, discard: 2,
                                                 from_drawn: false }] }),
          phase: None, on_build: [UPGRADE_ANY], reveal: []),
    // « Augmentez l'oxygène de 1 niveau OU améliorez une carte Phase. » (D14,
    //   rouge) — alternative du texte imprimé, branches dans l'ordre du carton,
    //   tranchées par `Policy::choose_option` (NEVER 7).
    card!("Biomedical Imports", reqs: [], effects: [],
          red: [], action: None, phase: None,
          on_build: [ResStep::Choose(&[
              &[ResEff::Gain(Oxygen(1))],
              &[ResEff::PhaseUpgrade(None)],
          ])],
          reveal: []),
    // « Révélez une tuile Océan. » + « Requiert un Objectif. » (D19, rouge —
    //   le prérequis est imprimé, le contrat ne le cite pas.)
    card!("Private Investor Beach", reqs: [HasObjective], effects: [Ocean(1)],
          red: [], action: None, phase: None, on_build: [], reveal: []),
    // Production : 4 MC. (D21)
    card!("3D Printing", reqs: [], effects: [McProd(4)],
          red: [], action: None, phase: None, on_build: [], reveal: []),
    // « Si vous avez un Objectif, gagnez 4 chaleurs. » + production :
    //   3 chaleurs. (D35) — le gain conditionnel est IMMÉDIAT (réserve de
    //   chaleur), la production est une piste fixe : deux grandeurs distinctes.
    card!("Award Winning Reflector Material", reqs: [],
          effects: [IfObjective(&[Heat(4)]), HeatProd(3)],
          red: [], action: None, phase: None, on_build: [], reveal: []),
    // Production : 3 chaleurs. (D41)
    card!("Nuclear Detonation Site", reqs: [], effects: [HeatProd(3)],
          red: [], action: None, phase: None, on_build: [], reveal: []),

    // ======================================= (jokers-corpos) LES 3 CARTES
    // À BADGE JOKER — les trois dernières muettes du jeu.
    //
    // « Choisissez un badge et ajoutez-le à cette carte » ne s'encode PAS ici :
    // le badge joker est une donnée de la carte (`Tag::Dynamic` dans
    // `cards.json`), et le jeton qu'on pose dessus est un état de JOUEUR
    // (`PlayerState::joker_tags`), posé par `flow::ensure_joker_tag` avant tout
    // calcul de prix. Ce que ces trois entrées apportent est le RESTE de leur
    // texte imprimé — deux productions fixes et une amélioration de carte
    // Phase — qui, lui, se décrit avec le vocabulaire existant.

    // « Choisissez un badge et ajoutez-le à cette carte. Lors de la phase de
    //   production, cette carte produit 2 MC. » (D26, verte, 7 MC)
    card!("Local Market", reqs: [], effects: [McProd(2)],
          red: [], action: None, phase: None, on_build: [], reveal: []),
    // Idem, 3 MC. (D39, verte, 10 MC)
    card!("Political Influence", reqs: [], effects: [McProd(3)],
          red: [], action: None, phase: None, on_build: [], reveal: []),
    // « Choisissez un badge et ajoutez-le à cette carte. Améliorez une carte
    //   Phase. » (D20, ROUGE — un événement) : la phase est au choix du joueur,
    //   d'où `UPGRADE_ANY`. La carte part en jeu comme les autres rouges et son
    //   badge continue d'y compter — livret de base : une carte rouge n'a plus
    //   d'effet après avoir été jouée, « autre que les badges qu'elle fournit ».
    card!("Topographic Mapping", reqs: [], effects: [],
          red: [], action: None, phase: None, on_build: [UPGRADE_ANY], reveal: []),
];

// ======================================== LOT CORPORATIONS (chantier corpo-1)
//
// Les 12 planches de corporation de la BOÎTE DE BASE. Même discipline que la
// table `LOT1` des cartes projets : des DONNÉES interprétées par `flow`, jamais
// une exception codée par corporation. La source du texte est
// `data/cartes-imprimees/textes-cartes.json` (champ `text`, transcription des planches
// imprimées) — surtout PAS le champ `description` de `cards.json`, qui est une
// paraphrase infidèle sur quatre corporations (Interplanetary Cinematics,
// Mining Guild, Phobolog, Saturn Systems : voir `outputs/corporations.md`).
//
// Cette table est aussi la DÉFINITION des deux boîtes : `CardsDb::load_boites`
// exige que chaque corporation chargée y figure, et refuse le chargement sinon.
//
// (jokers-corpos) Le commentaire d'origine annonçait que les quatre planches de
// Découverte (Apollo Industries, Exocorp, Hyperion Systems, Sultira)
// « reviendraient dans la pioche par le même chemin, sans toucher au
// chargement, le jour où le chantier des améliorations de phase existerait ».
// C'est exactement ce qui s'est passé : elles ont reçu une entrée ICI, et
// `install_corporation_with` applique leur `setup` par le chemin d'octroi
// unique. Elles restent hors de la pioche de la BOÎTE DE BASE, où elles n'ont
// aucune planche imprimée — c'est la table de boîtes qui en décide, pas celle-ci.

/// Production de départ FIXE d'une corporation, inscrite sur les pistes
/// `mc_prod` / `heat_prod` / `plant_prod` du joueur à la mise en place — donc
/// consommée par la phase IV à CHAQUE génération, jamais une seule fois.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StartProd {
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
}

/// (corpo) Unmi : « The first time your TR is raised each phase, you may pay
/// 6 MC to raise your TR 1 step. »
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrBoost {
    pub cost_mc: i64,
    pub steps: u8,
}

/// Encodage complet d'une corporation. Les quatre premiers champs réemploient
/// le vocabulaire des cartes projets ; les quatre derniers sont NEUFS, parce
/// qu'aucune carte projet n'exprimait ces mécanismes (journal D3).
#[derive(Debug)]
pub struct CorpEffects {
    /// « You start with N heat/plant production ».
    pub start_prod: StartProd,
    /// « At the start of the game, draw N cards » (Inventrix).
    pub start_draw: u8,
    /// Réductions permanentes, servies par `flow::card_discount`.
    pub reductions: &'static [Reduction],
    /// « Each time you play a … » — servis par les déclencheurs de pose du lot 2.
    pub play_triggers: &'static [PlayTrigger],
    /// Bonus permanent de phase Recherche, servi par `flow::research_extra`.
    pub research: Option<ResearchBonus>,
    /// (NEUF) Ecoline : plantes en moins pour une forêt, servi par
    /// `flow::forest_plant_cost`.
    pub forest_plant_rebate: i64,
    /// (NEUF) Helion : la chaleur est dépensable comme des MC
    /// (`flow::spendable_mc` / `flow::top_up_mc_with_heat`).
    pub heat_as_mc: bool,
    /// (NEUF) Inventrix : un prérequis de température ou d'oxygène peut être
    /// jugé un palier de couleur plus haut OU plus bas (`flow::reqs_satisfied`).
    pub req_color_flex: bool,
    /// (NEUF) Unmi : premier pas de NT de chaque phase, doublable contre 6 MC
    /// (`flow::gain_tr`).
    pub tr_boost: Option<TrBoost>,
    // ==================================================== lot jokers-corpos
    /// **(jokers-corpos) Effets appliqués à la MISE EN PLACE de la
    /// corporation** — « Améliorez votre carte Phase n », que les quatre
    /// planches de Découverte portent toutes.
    ///
    /// Le vocabulaire est celui des cartes projets, `ResEff`, et l'octroi passe
    /// par le chemin UNIQUE `flow::apply_phase_upgrade` avec
    /// `UpgradeSource::Setup` : une corporation n'a pas de mécanisme
    /// d'amélioration à elle, et le déroulement ne compare aucun nom.
    ///
    /// Seuls `PhaseUpgrade` et `Gain` y sont exprimables : les variantes à
    /// ressources exigent une carte réceptacle, qu'une planche n'est pas. Un
    /// test structurel du lot (`lot_jokers_corpos_tests`) le vérifie sur toute
    /// la table, faute de quoi un encodage serait silencieusement inerte.
    pub setup: &'static [ResEff],
    /// **(jokers-corpos) MC supplémentaires par carte défaussée pour du MC** —
    /// Exocorp, « Les cartes que vous défaussez pour gagner des MC vous
    /// rapportent 1 MC supplémentaire ».
    ///
    /// Exactement le champ `CardEffects::discard_bonus` de *Composting
    /// Factory*, porté par une planche : le même service unique
    /// `flow::discard_mc_rate` les cumule tous les deux, il n'y a pas de second
    /// calcul du taux.
    pub discard_bonus: i64,
    /// **(jokers-corpos) Action activable portée par la planche** — Hyperion
    /// Systems, « Action : gagnez 1 MC ».
    ///
    /// Même vocabulaire et même chemin d'activation que l'action d'une carte
    /// bleue (`flow::apply_action_spec`) ; elle s'offre en phase III comme les
    /// autres, une fois par phase, et consomme une activation.
    pub action: Option<Action>,
    /// **(jokers-corpos) Bonus d'action conditionné à la phase choisie** —
    /// Hyperion Systems, « *Si vous choisissez la phase d'actions lors de cette
    /// manche, gagnez 1 MC supplémentaire ». Champ jumeau de
    /// `CardEffects::phase_bonus`, lu par le même code.
    pub phase_bonus: Option<PhaseBonus>,
}

/// Paliers de couleur (0 = violet, 1 = rouge, 2 = jaune, 3 = blanc) — bornes du
/// module. Sert au seul `req_color_flex` d'Inventrix.
pub fn temp_color(level: u8) -> u8 {
    if level >= TEMP_W_MIN {
        3
    } else if level >= TEMP_Y_MIN {
        2
    } else if level >= TEMP_R_MIN {
        1
    } else {
        0
    }
}

/// Idem pour l'oxygène (P 0-2, R 3-6, Y 7-11, W 12-14).
pub fn oxy_color(level: u8) -> u8 {
    if level >= OXY_W_MIN {
        3
    } else if level >= OXY_Y_MIN {
        2
    } else if level >= OXY_R_MIN {
        1
    } else {
        0
    }
}

macro_rules! corp {
    // Forme des 12 planches de la boîte de base : aucune n'a de mise en place à
    // effets, ni d'action, ni de taux de défausse majoré.
    ($name:literal, prod: $sp:expr, draw: $dr:literal,
     red: [$($rd:expr),*], ptrig: [$($pt:expr),*], research: $rs:expr,
     forest: $fo:literal, heat_as_mc: $hm:literal, flex: $fx:literal,
     tr_boost: $tb:expr) => {
        ($name, CorpEffects {
            start_prod: $sp, start_draw: $dr,
            reductions: &[$($rd),*], play_triggers: &[$($pt),*],
            research: $rs, forest_plant_rebate: $fo, heat_as_mc: $hm,
            req_color_flex: $fx, tr_boost: $tb,
            setup: &[], discard_bonus: 0, action: None, phase_bonus: None,
        })
    };
    // (jokers-corpos) Forme des 4 planches de Découverte : TOUTES améliorent
    // une carte Phase à la mise en place ; l'une porte un déclencheur de pose,
    // l'une un taux de défausse majoré, l'une une action. D'où ces quatre
    // champs, et rien d'autre.
    ($name:literal, setup: [$($su:expr),*], ptrig: [$($pt:expr),*],
     discard: $db:expr, action: $ac:expr, phase: $pb:expr) => {
        ($name, CorpEffects {
            start_prod: NO_PROD, start_draw: 0,
            reductions: &[], play_triggers: &[$($pt),*],
            research: None, forest_plant_rebate: 0, heat_as_mc: false,
            req_color_flex: false, tr_boost: None,
            setup: &[$($su),*], discard_bonus: $db, action: $ac,
            phase_bonus: $pb,
        })
    };
}

const NO_PROD: StartProd = StartProd { mc: 0, heat: 0, plants: 0 };

/// Les 16 corporations des deux boîtes : les 12 de la boîte de base dans
/// l'ordre de leur NUMÉRO IMPRIMÉ (209 → 220), puis les 4 de Découverte
/// (D01 → D04). Cet ordre est celui de la lecture, il n'a aucun effet sur le
/// moteur : la pioche suit l'ordre de chargement de `cards.json`, et c'est cet
/// ordre-là que rend `--dump-corporations`.
pub static CORPS: &[(&str, CorpEffects)] = &[
    // 209 CrediCor — « You start with 48 MC. EFFECT: When you play a card with a
    // printed cost of 20 MC or more, you pay 4 MC less for it. »
    corp!("Credicor", prod: NO_PROD, draw: 0,
          red: [Reduction::MinPrice { min: 20, amount: 4 }], ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 210 Ecoline — « You start with 1 plant production and 27 MC. EFFECT: When
    // you spend plants to gain a forest VP token and raise oxygen, you spend one
    // less plant. »
    corp!("Ecoline", prod: StartProd { mc: 0, heat: 0, plants: 1 }, draw: 0,
          red: [], ptrig: [],
          research: None, forest: 1, heat_as_mc: false, flex: false, tr_boost: None),
    // 211 Helion — « You start with 3 heat production and 28 MC. EFFECT: You may
    // use heat as MC. You may not use MC as heat. »
    corp!("Helion Corporation", prod: StartProd { mc: 0, heat: 3, plants: 0 }, draw: 0,
          red: [], ptrig: [],
          research: None, forest: 0, heat_as_mc: true, flex: false, tr_boost: None),
    // 212 Interplanetary Cinematics — « You start with 46 MC. When you play a
    // [building], you pay 2 MC less for it. EFFECT: When you play an [event], you
    // pay 2 MC less for it. » (cards.json dit « 1 steel production » + event seul :
    // paraphrase fausse, le texte imprimé gagne.)
    corp!("Interplanetary Cinematics", prod: NO_PROD, draw: 0,
          red: [Reduction::Tag(Tag::Building, 2), Reduction::Tag(Tag::Event, 2)],
          ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 213 Inventrix — « At the start of the game, draw 3 cards. You start with
    // 33 MC. EFFECT: When playing a card with requirements, you may consider the
    // oxygen or temperature one color higher or lower. »
    corp!("Inventrix", prod: NO_PROD, draw: 3,
          red: [], ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: true, tr_boost: None),
    // 214 Mining Guild — « You start with 27 MC. When you play a [building], you
    // pay 2 MC less for it. EFFECT: Each time you play steel production,
    // excluding this, gain 1 TR. »
    // (transcription `data/cartes-imprimees/textes-cartes.json`, entrée « Mining
    // Guild » ; ses notes décrivent l'encart : « phase I, vignette outils :
    // +1 [TR] », la vignette outils étant celle du savoir-faire acier —
    // livret de base l. 527.)
    //
    // (D2) LES DEUX LIGNES DE LA PLANCHE SONT MAINTENANT LÀ. La première est le
    // −2 MC des cartes bâtiment, qui EST aussi son savoir-faire acier (encart
    // gris, un acier, dérivé par `flow::capacities`). La seconde est ce
    // déclencheur : « chaque fois que vous jouez de la production d'acier ».
    //
    // Le commentaire d'origine renvoyait le déclencheur à une notion « que le
    // moteur ne modélise pas » : c'est devenu faux au lot acier-titane, qui
    // dérive exactement combien d'aciers chaque carte apporte
    // (`flow::capacites_apportees`). C'était le dernier pouvoir imprimé sauté
    // en silence du jeu.
    //
    // `include_self: false` = « excluding this » : l'acier de la planche
    // elle-même ne rapporte rien, la corporation n'étant jamais « jouée ».
    // `scale_by_matched_tags: true` = un niveau de terraformation PAR ACIER
    // apporté — arbitrage d'Alexis du 18-08, carton en main, appuyé sur le
    // livret de base l. 106. Une carte à deux aciers en accorde donc deux.
    corp!("Mining Guild", prod: NO_PROD, draw: 0,
          red: [Reduction::Tag(Tag::Building, 2)],
          ptrig: [PlayTrigger { cond: TrigCond::GrantsCapacity(Capacity::Steel),
                    gains: &[TrigGain::Tr(1)], scale_by_matched_tags: true,
                    include_self: false }],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 215 PhoboLog — « You start with 20 MC. When you play a [space], you pay
    // 3 MC less for it. EFFECT: Each titanium you have reduces the cost of
    // [space] cards an additional 1 MC. »
    //
    // (lot acier-titane) Le commentaire d'origine disait « le titane n'est pas
    // modélisé … seul le −3 est encodé » : il est devenu faux, l'EFFET est
    // désormais encodé lui aussi. Les deux lignes de la planche sont donc là :
    // le −3 MC des cartes espace, qui EST son savoir-faire (encart gris, un
    // titane), et l'effet qui amplifie chaque titane d'1 MC — son propre titane
    // compris, puisque `flow::capacities` lit les réductions de la corporation
    // comme celles des cartes.
    corp!("Phobolog", prod: NO_PROD, draw: 0,
          red: [Reduction::Tag(Tag::Space, 3),
                Reduction::PerCapacity { tag: Tag::Space, cap: Capacity::Titanium, per: 1 }],
          ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 216 Saturn Systems — « You start with 24 MC. When you play a [space], you
    // pay 3 MC less for it. EFFECT: Each time you play a [jupiter], excluding
    // this, gain 1 TR. » « excluding this » = le badge [jupiter] de la planche
    // elle-même ne rapporte rien : la corporation n'est jamais « jouée ».
    // `scale_by_matched_tags: true` = livret p.9 l.106 (condition remplie
    // plusieurs fois → effet résolu autant de fois).
    corp!("Saturn Systems", prod: NO_PROD, draw: 0,
          red: [Reduction::Tag(Tag::Space, 3)],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Jupiter),
                    gains: &[TrigGain::Tr(1)], scale_by_matched_tags: true,
                    include_self: false }],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 217 Teractor — « You start with 51 MC. EFFECT: When you play an [earth],
    // you pay 3 MC less for it. »
    corp!("Teractor Corporation", prod: NO_PROD, draw: 0,
          red: [Reduction::Tag(Tag::Earth, 3)], ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 218 Tharsis Republic — « You start with 40 MC. EFFECT: When you draw cards
    // during the research phase, draw one additional card and keep one additional
    // card. » (texte identique à Interplanetary Relations, lot 4.)
    corp!("Tharsis Republic", prod: NO_PROD, draw: 0,
          red: [], ptrig: [],
          research: Some(ResearchBonus { draw: 1, keep: 1 }),
          forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 219 Thorgate — « You start with 1 heat production and 45 MC. EFFECT: When
    // you play a [energy], you pay 3 MC less for it. »
    corp!("Thorgate Corporation", prod: StartProd { mc: 0, heat: 1, plants: 0 }, draw: 0,
          red: [Reduction::Tag(Tag::Energy, 3)], ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false, tr_boost: None),
    // 220 United Nations Mars Initiative — « You start with 35 MC. EFFECT: The
    // first time your TR is raised each phase, you may pay 6 MC to raise your TR
    // 1 step. »
    corp!("Unmi", prod: NO_PROD, draw: 0,
          red: [], ptrig: [],
          research: None, forest: 0, heat_as_mc: false, flex: false,
          tr_boost: Some(TrBoost { cost_mc: 6, steps: 1 })),

    // =================================================== (Découverte) D01-D04
    // Les quatre planches de l'extension. Toutes améliorent une carte Phase
    // IMPOSÉE à la mise en place — `ResEff::PhaseUpgrade(Some(n))`, le même
    // paramètre que les trois cartes projets à phase imposée (D05, D37, D40).
    //
    // Leur ENCART DE PHASES (I-II, I-V, III) n'appelle aucun mécanisme nouveau :
    // un déclencheur « lorsque vous jouez un badge » ne peut se lever qu'aux
    // phases où l'on pose une carte (I et II) ; un taux de défausse marqué I-V
    // vaut partout ; une action ne s'active qu'en phase III. Voir result.md,
    // § bornes de phase.

    // D01 Apollo Industries — « Vous commencez avec 33 MC. Améliorez votre carte
    // Phase II. Effet (I-II) : Lorsque vous jouez un badge [science], piochez
    // une carte. » Comme Saturn Systems, `scale_by_matched_tags: true` (livret
    // p.9 l.106 : condition remplie plusieurs fois → effet résolu autant de
    // fois) et `include_self: false` — le badge [espace] de la planche n'est pas
    // un badge [science], et la planche n'est de toute façon jamais « jouée ».
    corp!("Apollo Industries",
          setup: [ResEff::PhaseUpgrade(Some(2))],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Science),
                    gains: &[TrigGain::Draw(1)], scale_by_matched_tags: true,
                    include_self: false }],
          discard: 0, action: None, phase: None),

    // D02 Exocorp — « Vous commencez avec 26 MC. Améliorez votre carte Phase V.
    // Effet (I-V) : Les cartes que vous défaussez pour gagner des MC vous
    // rapportent 1 MC supplémentaire. » Le taux de base est celui du livret
    // (3 MC) ; seul le supplément est un effet, cumulé par le service unique
    // `flow::discard_mc_rate` — exactement comme *Composting Factory*.
    corp!("Exocorp",
          setup: [ResEff::PhaseUpgrade(Some(5))],
          ptrig: [],
          discard: 1, action: None, phase: None),

    // D03 Hyperion Systems — « Vous commencez avec 30 MC. Améliorez votre carte
    // Phase III. Action (III) : Gagnez 1 MC. *Si vous choisissez la phase
    // d'actions lors de cette manche, gagnez 1 MC supplémentaire. »
    // Le bonus étoilé est le `PhaseBonus` du lot 6, mot pour mot : phase 3, un
    // effet EN PLUS (« supplémentaire »), aucun coût de remplacement.
    corp!("Hyperion Systems",
          setup: [ResEff::PhaseUpgrade(Some(3))],
          ptrig: [],
          discard: 0,
          action: Some(Action::Fixed { cost: &[], effect: &[ActionEff::Mc(1)] }),
          phase: Some(PhaseBonus { phase: 3, require_upgraded: false,
                                   cost: None, extra: &[ActionEff::Mc(1)] })),

    // D04 Sultira — « Vous commencez avec 38 MC. Améliorez votre carte Phase I.
    // Effet (I-II) : Chaque fois que vous jouez un badge [énergie], **y compris
    // celui-ci**, gagnez 2 chaleurs. »
    //
    // « Y COMPRIS CELUI-CI » : `include_self: true`. Contrairement au « excluding
    // this » de Saturn Systems, le badge [énergie] de la planche elle-même
    // déclenche l'effet — d'où 2 chaleurs dès la mise en place, levées par
    // `flow::fire_corp_self_triggers` contre les badges de la planche. Rien
    // n'est écrit en dur : c'est le déclencheur qui les produit.
    // (`cards.json` omettait la clause ; le carton fait foi — voir result.md.)
    corp!("Sultira",
          setup: [ResEff::PhaseUpgrade(Some(1))],
          ptrig: [PlayTrigger { cond: TrigCond::Tag(Tag::Energy),
                    gains: &[TrigGain::Heat(2)], scale_by_matched_tags: true,
                    include_self: true }],
          discard: 0, action: None, phase: None),
];

/// Cherche l'encodage d'une corporation par nom exact.
pub fn corp_lookup(name: &str) -> Option<&'static CorpEffects> {
    CORPS.iter().find(|(n, _)| *n == name).map(|(_, e)| e)
}

// =============================================================================
// (Découverte) LES CARTES PHASE — de base et AMÉLIORÉES
//
// Le BONUS DU SÉLECTIONNEUR de chaque phase est une DONNÉE, jamais une
// constante lue dans le flux de jeu. Une carte Phase améliorée REMPLACE la
// carte Phase correspondante dans la main du joueur (livret l. 64) : le moteur
// le rend littéral en ne lisant JAMAIS deux entrées de cette table pour une
// même phase. Le cumul du bonus de base et du bonus amélioré n'est pas
// « absent », il est impossible à écrire (NEVER 1).
//
// Lecture unique : `flow::selector_bonus`. Les cinq phases y passent.
// =============================================================================

/// (Découverte) Une branche de bonus de sélectionneur. **Plusieurs branches =
/// un « ou » du texte imprimé**, tranché par `Policy` — jamais par le moteur
/// (NEVER 4).
#[derive(Debug, Clone, Copy)]
pub struct SelectorGrant {
    /// MC de moins sur la carte jouée pendant cette phase (phase I).
    pub mc_discount: i64,
    /// MC gagnés (phase IV ; branche « OU gagnez 6 MC » de II-B).
    pub mc: i64,
    /// Cartes piochées (phase II).
    pub draw: u8,
    /// Activations d'action supplémentaires (phase III).
    pub extra_activations: u8,
    /// Permissions de pose supplémentaires (I-B, II-A, II-B). C'est le
    /// mécanisme du lot cartes-8, emprunté tel quel : aucune seconde file
    /// (NEVER 2).
    pub builds: &'static [BuildGrant],
    /// Cartes piochées / conservées EN PLUS de la compétence imprimée
    /// (phase V, « tous les joueurs piochent 2 et en conservent 1 »).
    pub research_draw: usize,
    pub research_keep: usize,
    /// Révélation du dessus de la pioche (III-A) — brique du lot 6.
    pub reveal: Option<Reveal>,
    /// Rejouer la production d'une carte verte du joueur (IV-A).
    pub replay_green_prod: bool,
}

/// Aucun bonus : la valeur d'un joueur qui n'est pas le sélectionneur.
pub const SELECTOR_NONE: SelectorGrant = SelectorGrant {
    mc_discount: 0,
    mc: 0,
    draw: 0,
    extra_activations: 0,
    builds: &[],
    research_draw: 0,
    research_keep: 0,
    reveal: None,
    replay_green_prod: false,
};

/// (Découverte) Une carte Phase — de base ou améliorée : son nom imprimé et son
/// bonus de sélectionneur. La COMPÉTENCE n'y figure pas : elle est identique
/// mot pour mot sur la carte de base et sur ses deux améliorations (ASK 2), et
/// elle vit déjà dans le flux de chaque phase.
#[derive(Debug, Clone, Copy)]
pub struct SelectorSpec {
    /// Nom imprimé de la carte (les noms vivent dans les données, NEVER 5).
    pub name: &'static str,
    /// Les branches du bonus, DANS L'ORDRE DU TEXTE IMPRIMÉ. Une seule branche
    /// = pas d'alternative.
    pub branches: &'static [SelectorGrant],
}

/// Le « bonus » d'un joueur qui n'a pas choisi la phase — une seule branche,
/// vide. Existe pour que le flux puisse toujours lire `branches[0]`.
pub static SELECTOR_SPEC_NONE: SelectorSpec = SelectorSpec {
    name: "",
    branches: &[SELECTOR_NONE],
};

/// Permission de la carte Phase améliorée I-B : « Vous pouvez jouer une seconde
/// carte verte lors de cette phase dont le coût IMPRIMÉ est de 12 MC ou moins. »
const SECOND_GREEN_UNDER_12: BuildGrant = BuildGrant {
    colors: &[Color::Green],
    max_printed_cost: Some(12),
    free: false,
};

/// Permission des cartes Phase améliorées II-A et II-B : « une seconde carte
/// bleue ou rouge lors de cette phase ». C'est exactement la pose ordinaire de
/// la phase II — la même donnée, pas une copie (`flow::GRANT_CONSTRUCTION`).
const SECOND_BLUE_OR_RED: &[BuildGrant] = &[crate::flow::GRANT_CONSTRUCTION];

/// (III-A) « Révélez les 3 premières cartes de la pioche. Ajoutez à votre main
/// une carte bleue ou rouge ainsi révélée. Défaussez les autres cartes. »
/// « bleue ou rouge » = toute carte qui n'est pas verte : le moteur n'a que
/// trois couleurs, `ColorIsNot(Green)` est le filtre exact, pas une
/// approximation.
const REVEAL_BLUE_OR_RED: Reveal = Reveal {
    n: 3,
    keep: RevealFilter::ColorIsNot(Color::Green),
    take: 1,
    mc_per_discarded: 0,
};

/// **Les cinq cartes Phase de la boîte de base** (livret p.11-15). Les valeurs
/// sont celles que le moteur employait en dur avant ce chantier ; les deux
/// constantes historiques restent la source (`state::DEV_SELECTOR_DISCOUNT`,
/// `state::PRODUCTION_SELECTOR_MC`).
///
/// Phase V : la compétence imprimée donne 2 piochées / 1 conservée À TOUS les
/// joueurs ; le bonus du sélectionneur vaut donc **+3 piochées / +1 conservée**
/// (5/2 au total), et non 5/2.
pub static PHASE_BASE: [SelectorSpec; 5] = [
    // I — DÉVELOPPEMENT : « Le coût de la carte que vous jouez lors de cette
    // phase est réduit de 3 MC. »
    SelectorSpec {
        name: "Development",
        branches: &[SelectorGrant {
            mc_discount: crate::state::DEV_SELECTOR_DISCOUNT,
            mc: 0,
            draw: 0,
            extra_activations: 0,
            builds: &[],
            research_draw: 0,
            research_keep: 0,
            reveal: None,
            replay_green_prod: false,
        }],
    },
    // II — CONSTRUCTION : « Piochez une carte avant ou après avoir joué une
    // carte lors de cette phase OU jouez une carte bleue ou rouge
    // supplémentaire. » Deux branches, plus le MOMENT de la pioche : ce dernier
    // reste tranché par `Policy::construction_bonus` (C2 du lot 3), qui EST le
    // choix de branche de cette carte-ci.
    SelectorSpec {
        name: "Construction",
        branches: &[
            SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 1,
                extra_activations: 0,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            },
            SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 0,
                extra_activations: 0,
                builds: SECOND_BLUE_OR_RED,
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            },
        ],
    },
    // III — ACTION : « Vous pouvez activer une "Action :" une fois de plus. »
    SelectorSpec {
        name: "Action",
        branches: &[SelectorGrant {
            mc_discount: 0,
            mc: 0,
            draw: 0,
            extra_activations: 1,
            builds: &[],
            research_draw: 0,
            research_keep: 0,
            reveal: None,
            replay_green_prod: false,
        }],
    },
    // IV — PRODUCTION : « Gagnez 4 MC. »
    SelectorSpec {
        name: "Production",
        branches: &[SelectorGrant {
            mc_discount: 0,
            mc: crate::state::PRODUCTION_SELECTOR_MC,
            draw: 0,
            extra_activations: 0,
            builds: &[],
            research_draw: 0,
            research_keep: 0,
            reveal: None,
            replay_green_prod: false,
        }],
    },
    // V — RECHERCHE : le sélectionneur pioche 5 et en garde 2, dont 2/1 de
    // compétence : le BONUS vaut +3 / +1.
    SelectorSpec {
        name: "Research",
        branches: &[SelectorGrant {
            mc_discount: 0,
            mc: 0,
            draw: 0,
            extra_activations: 0,
            builds: &[],
            research_draw: 3,
            research_keep: 1,
            reveal: None,
            replay_green_prod: false,
        }],
    },
];

/// **Les dix cartes Phase améliorées** (extension Découverte, transcription
/// `data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`), indexées `[phase - 1][variante]`
/// (variante 0 = A, 1 = B).
///
/// Chaque entrée donne le bonus COMPLET de la carte améliorée : il REMPLACE
/// celui de la carte de base, il ne s'y ajoute pas (livret l. 64).
pub static PHASE_UPGRADED: [[SelectorSpec; 2]; 5] = [
    [
        // I-A — « Le coût de la carte que vous jouez lors de cette phase est
        // réduit de 6 MC. » (le double de la carte de base, pas 3 + 6).
        SelectorSpec {
            name: "Development (phase améliorée A)",
            branches: &[SelectorGrant {
                mc_discount: 6,
                mc: 0,
                draw: 0,
                extra_activations: 0,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            }],
        },
        // I-B — « Le coût de la PREMIÈRE carte que vous jouez lors de cette
        // phase est réduit de 3 MC. Vous pouvez jouer une seconde carte verte
        // lors de cette phase dont le coût imprimé est de 12 MC ou moins. »
        SelectorSpec {
            name: "Development (phase améliorée B)",
            branches: &[SelectorGrant {
                mc_discount: 3,
                mc: 0,
                draw: 0,
                extra_activations: 0,
                builds: &[SECOND_GREEN_UNDER_12],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            }],
        },
    ],
    [
        // II-A — « Piochez une carte. Vous pouvez jouer une seconde carte bleue
        // ou rouge lors de cette phase. » LES DEUX : une seule branche.
        SelectorSpec {
            name: "Construction (phase améliorée A)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 1,
                extra_activations: 0,
                builds: SECOND_BLUE_OR_RED,
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            }],
        },
        // II-B — « Jouez une carte bleue ou une carte rouge supplémentaire lors
        // de cette phase. OU Gagnez 6 MC. » Un vrai « ou » : deux branches,
        // dans l'ordre du texte imprimé.
        SelectorSpec {
            name: "Construction (phase améliorée B)",
            branches: &[
                SelectorGrant {
                    mc_discount: 0,
                    mc: 0,
                    draw: 0,
                    extra_activations: 0,
                    builds: SECOND_BLUE_OR_RED,
                    research_draw: 0,
                    research_keep: 0,
                    reveal: None,
                    replay_green_prod: false,
                },
                SelectorGrant {
                    mc_discount: 0,
                    mc: 6,
                    draw: 0,
                    extra_activations: 0,
                    builds: &[],
                    research_draw: 0,
                    research_keep: 0,
                    reveal: None,
                    replay_green_prod: false,
                },
            ],
        },
    ],
    [
        // III-A — « Vous pouvez activer un de vos effets "Action :" une fois de
        // plus. Révélez les 3 premières cartes de la pioche. Ajoutez à votre
        // main une carte bleue ou rouge ainsi révélée. Défaussez les autres. »
        SelectorSpec {
            name: "Action (phase améliorée A)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 0,
                extra_activations: 1,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: Some(REVEAL_BLUE_OR_RED),
                replay_green_prod: false,
            }],
        },
        // III-B — « Vous pouvez activer deux de vos effets "Action :" une fois
        // de plus. »
        SelectorSpec {
            name: "Action (phase améliorée B)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 0,
                extra_activations: 2,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            }],
        },
    ],
    [
        // IV-A — « Gagnez 1 MC. Activez l'effet de production de l'une de vos
        // cartes vertes une fois de plus lors de cette phase. »
        SelectorSpec {
            name: "Production (phase améliorée A)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 1,
                draw: 0,
                extra_activations: 0,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: true,
            }],
        },
        // IV-B — « Gagnez 7 MC. »
        SelectorSpec {
            name: "Production (phase améliorée B)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 7,
                draw: 0,
                extra_activations: 0,
                builds: &[],
                research_draw: 0,
                research_keep: 0,
                reveal: None,
                replay_green_prod: false,
            }],
        },
    ],
    [
        // V-A — « Piochez 2 cartes supplémentaires et conservez-en 2
        // supplémentaires. » Sur la compétence 2/1 : 4 piochées, 3 conservées.
        // MOINS de cartes vues que la carte de base (5), PLUS de cartes gardées
        // (3 contre 2) : c'est l'arbitrage imprimé, pas une coquille.
        SelectorSpec {
            name: "Research (phase améliorée A)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 0,
                extra_activations: 0,
                builds: &[],
                research_draw: 2,
                research_keep: 2,
                reveal: None,
                replay_green_prod: false,
            }],
        },
        // V-B — « Piochez 6 cartes supplémentaires et conservez-en 1
        // supplémentaire. » Sur la compétence 2/1 : 8 piochées, 2 conservées.
        SelectorSpec {
            name: "Research (phase améliorée B)",
            branches: &[SelectorGrant {
                mc_discount: 0,
                mc: 0,
                draw: 0,
                extra_activations: 0,
                builds: &[],
                research_draw: 6,
                research_keep: 1,
                reveal: None,
                replay_green_prod: false,
            }],
        },
    ],
];

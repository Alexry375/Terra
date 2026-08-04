//! État de jeu : joueurs, planète, pioches, milestones/awards.
//!
//! Sources des constantes :
//! - Température -30..+8 par pas de 2 (20 niveaux), oxygène 0..14 (15 niveaux),
//!   9 océans : livret de base (aperçu p.2, plateau p.4) et
//!   `PlanetFactory.createMars` du moteur Java.
//! - Bonus des tuiles océan : `PlanetFactory.generateOceans` (Java).
//! - TR de départ 5, main de départ 8, limite de main 10, défausse 3 MC :
//!   `Constants.java` (STARTING_RT, DEFAULT_START_HAND_SIZE,
//!   MAX_HAND_SIZE_LAST_ROUND) + livret (avslutningssteget p.16).

use crate::cards::{CardsDb, Color, Tag, TAG_COUNT};
use crate::effects::{BuildGrant, NextCardMod};
use rand::rngs::StdRng;
use std::collections::BTreeMap;

pub const NUM_PLAYERS: usize = 2;
/// Niveau max de température (index 19 == +8 °C).
pub const TEMPERATURE_MAX: u8 = 19;
/// Niveau max d'oxygène (14 %).
pub const OXYGEN_MAX: u8 = 14;
/// Nombre de tuiles océan.
pub const NUM_OCEANS: u8 = 9;
/// Niveau max d'infrastructure (piste Infrastructure, 15 niveaux 0..=14 —
/// `PlanetFactory` Java). Hors condition de fin de partie v1 : seule la carte
/// imposée Grain Silos (hors pioche v1) la fait monter, via la sonde/les tests.
pub const INFRASTRUCTURE_MAX: u8 = 14;

pub const STARTING_TR: i64 = 5;
pub const STARTING_HAND: usize = 8;
pub const HAND_LIMIT: usize = 10;
pub const SELL_CARD_MC: i64 = 3;

// Actions standard (livret p.14 + Constants.java).
pub const FOREST_PLANT_COST: i64 = 8;
pub const FOREST_MC_COST: i64 = 20;
pub const TEMPERATURE_HEAT_COST: i64 = 8;
pub const TEMPERATURE_MC_COST: i64 = 14;
pub const OCEAN_MC_COST: i64 = 15;

// Bonus du sélectionneur de phase (faskort du livret p.11-15).
pub const DEV_SELECTOR_DISCOUNT: i64 = 3;
pub const PRODUCTION_SELECTOR_MC: i64 = 4;

/// Bonus d'une tuile océan (cartes, MC, plantes) — `PlanetFactory` Java.
#[derive(Debug, Clone, Copy)]
pub struct OceanTile {
    /// Rang de la tuile dans `OCEAN_TILES`, AVANT mélange — son identité sur la
    /// planche imprimée. Ne participe à aucune règle : il n'existe que pour que
    /// l'écran puisse montrer QUELLE tuile a été retournée, au lieu d'en
    /// inventer une. Le mélange réordonne le tableau, pas les identités.
    pub id: u8,
    pub cards: u8,
    pub mc: i64,
    pub plants: i64,
}

/// Les 9 tuiles océan du jeu de base (ordre avant mélange).
pub const OCEAN_TILES: [OceanTile; 9] = [
    OceanTile { id: 0, cards: 0, mc: 0, plants: 2 },
    OceanTile { id: 1, cards: 0, mc: 4, plants: 0 },
    OceanTile { id: 2, cards: 1, mc: 1, plants: 0 },
    OceanTile { id: 3, cards: 0, mc: 2, plants: 1 },
    OceanTile { id: 4, cards: 1, mc: 0, plants: 1 },
    OceanTile { id: 5, cards: 1, mc: 0, plants: 0 },
    OceanTile { id: 6, cards: 0, mc: 1, plants: 1 },
    OceanTile { id: 7, cards: 1, mc: 0, plants: 0 },
    OceanTile { id: 8, cards: 0, mc: 0, plants: 2 },
];

/// (Découverte) **Améliorations de carte Phase.** Chaque phase dispose de deux
/// options d'amélioration, A et B (livret l. 52) ; chaque joueur possède ses
/// dix cartes, à son dos de couleur — d'où un tableau dans `PlayerState` et
/// jamais dans `GameState` (NEVER 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseUpgrade {
    VariantA,
    VariantB,
}

impl PhaseUpgrade {
    /// Les deux variantes dans l'ordre imprimé (A puis B).
    pub const ALL: [PhaseUpgrade; 2] = [PhaseUpgrade::VariantA, PhaseUpgrade::VariantB];

    /// Indice de variante : A = 0, B = 1 (indexe `effects::PHASE_UPGRADED`).
    pub fn index(self) -> usize {
        match self {
            PhaseUpgrade::VariantA => 0,
            PhaseUpgrade::VariantB => 1,
        }
    }

    /// Étiquette imprimée de la variante.
    pub fn label(self) -> &'static str {
        match self {
            PhaseUpgrade::VariantA => "A",
            PhaseUpgrade::VariantB => "B",
        }
    }

    /// Lecture d'une étiquette de variante (« A » / « B »). Tout le reste est
    /// refusé : `--probe-upgrade 1C` n'existe pas.
    pub fn from_label(s: &str) -> Option<PhaseUpgrade> {
        match s {
            "A" => Some(PhaseUpgrade::VariantA),
            "B" => Some(PhaseUpgrade::VariantB),
            _ => None,
        }
    }
}

/// (Découverte) Lecture d'une désignation d'amélioration `<phase><variante>`,
/// forme `"1B"` — le format de `--probe-upgrade` et des tests. Renvoie
/// `(phase 1..=5, variante)`. Toute autre forme est refusée : c'est ce refus
/// qui empêche un argument mal formé d'être ignoré en silence.
pub fn parse_phase_upgrade(s: &str) -> Option<(u8, PhaseUpgrade)> {
    if s.len() != 2 || !s.is_ascii() {
        return None;
    }
    let phase = s[0..1].parse::<u8>().ok()?;
    if !(1..=5).contains(&phase) {
        return None;
    }
    Some((phase, PhaseUpgrade::from_label(&s[1..2])?))
}

/// Milestones (pool du moteur Java, base + Discovery). 3 en jeu par partie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneKind {
    /// 8 tags bâtiment.
    Builder,
    /// 9 tags différents.
    Diversifier,
    /// 10 de production de chaleur.
    Energizer,
    /// 5 de production de plantes.
    Farmer,
    /// 6 cartes rouges.
    Legend,
    /// 8 cartes vertes.
    Magnate,
    /// 12 cartes jouées.
    Planner,
    /// 6 tags espace. (28-07 : le commentaire disait 7, la tuile imprimée dit
    /// 6 — corrigé en même temps que le seuil de `flow::milestone_goal`.)
    SpaceBaron,
    /// TR >= 15.
    Terraformer,
    /// 6 cartes bleues.
    Tycoon,
    /// 3 forêts.
    Gardener,
}

impl MilestoneKind {
    /// (decouverte-projets) Nom de la tuile Objectif, tel que `--probe-objectif`
    /// l'accepte et que les messages d'erreur l'affichent. Ce sont les noms du
    /// moteur (pool Java), pas des étiquettes inventées ici.
    pub fn name(self) -> &'static str {
        match self {
            MilestoneKind::Builder => "Builder",
            MilestoneKind::Diversifier => "Diversifier",
            MilestoneKind::Energizer => "Energizer",
            MilestoneKind::Farmer => "Farmer",
            MilestoneKind::Legend => "Legend",
            MilestoneKind::Magnate => "Magnate",
            MilestoneKind::Planner => "Planner",
            MilestoneKind::SpaceBaron => "SpaceBaron",
            MilestoneKind::Terraformer => "Terraformer",
            MilestoneKind::Tycoon => "Tycoon",
            MilestoneKind::Gardener => "Gardener",
        }
    }

    /// (decouverte-projets) Lecture d'un nom d'Objectif. L'inverse EXACT de
    /// [`MilestoneKind::name`] : la comparaison se fait sur le pool, il n'y a
    /// pas de seconde liste à tenir à jour. Tout nom absent du pool est REFUSÉ
    /// (`None`) — c'est ce refus qui empêche `--probe-objectif` d'ignorer un
    /// argument mal formé en silence.
    pub fn from_name(s: &str) -> Option<MilestoneKind> {
        MILESTONE_POOL.iter().copied().find(|k| k.name() == s)
    }
}

pub const MILESTONE_POOL: [MilestoneKind; 11] = [
    MilestoneKind::Builder,
    MilestoneKind::Diversifier,
    MilestoneKind::Energizer,
    MilestoneKind::Farmer,
    MilestoneKind::Legend,
    MilestoneKind::Magnate,
    MilestoneKind::Planner,
    MilestoneKind::SpaceBaron,
    MilestoneKind::Terraformer,
    MilestoneKind::Tycoon,
    MilestoneKind::Gardener,
];

/// Awards — **sept tuiles imprimées, sept variantes** depuis le chantier
/// `decouverte-phases`. La septième, VISIONNAIRE, attendait le mécanisme des
/// cartes Phase améliorées : sans lui, sa valeur aurait été nulle pour tout le
/// monde dans toutes les parties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwardKind {
    /// Production de MC.
    Celebrity,
    /// Ressources posées sur cartes. (28-07 : le commentaire disait « toujours
    /// 0 en v1 — stub », ce n'est plus vrai — `flow::award_value` en fait la
    /// somme réelle depuis la réparation de la récompense.)
    Collector,
    /// Production de chaleur.
    Generator,
    /// (lot acier-titane) Somme des savoir-faire : aciers + titanes.
    ///
    /// Le commentaire d'origine disait « toujours 0 en v1 — stub » : ce n'est
    /// plus vrai, `flow::award_value` lit deux comptes désormais dérivés des
    /// cartes en jeu. La récompense est donc réellement disputée.
    Industrialist,
    /// Nombre de cartes jouées.
    ProjectManager,
    /// Tags science.
    Researcher,
    /// (Découverte) VISIONNAIRE — « le plus de cartes Phase améliorées ».
    /// Valeur : `PlayerState::phase_upgrades_count`.
    Visionary,
}

pub const AWARD_POOL: [AwardKind; 7] = [
    AwardKind::Celebrity,
    AwardKind::Collector,
    AwardKind::Generator,
    AwardKind::Industrialist,
    AwardKind::ProjectManager,
    AwardKind::Researcher,
    AwardKind::Visionary,
];

/// Un milestone en jeu + qui l'a revendiqué (revendication simplifiée D8).
#[derive(Debug, Clone, Copy)]
pub struct MilestoneSlot {
    pub kind: MilestoneKind,
    pub achieved_by: [bool; NUM_PLAYERS],
}

/// État d'un joueur.
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    pub tr: i64,
    pub forests: i64,
    // Productions (toujours 0 en v1 — les cartes stub n'en donnent pas ;
    // structure présente pour les chantiers cartes).
    pub mc_prod: i64,
    pub heat_prod: i64,
    pub plant_prod: i64,
    pub card_prod: i64,
    /// (lot acier-titane) **Savoir-faire acier** du joueur : nombre d'aciers
    /// qu'il possède. Chacun réduit de 2 MC le coût des cartes à badge bâtiment
    /// (livret l. 355-359). Ce n'est pas une réserve de jetons : on ne le
    /// dépense pas, le posséder suffit.
    ///
    /// **Champ-CACHE, pas source de vérité** : sa seule écriture est
    /// `flow::refresh_capacities`, qui recopie la dérivation
    /// `flow::capacities` (I1, I2). `sim::check_invariants` recompare les deux
    /// à chaque manche : ils ne peuvent pas diverger sans que 1000 parties le
    /// disent. Le commentaire d'origine (« stub v1 ») est caduc.
    pub steel_capacity: i64,
    /// (lot acier-titane) **Savoir-faire titane** : 3 MC par unité sur les
    /// cartes à badge espace. Mêmes règles d'écriture que `steel_capacity`.
    pub titanium_capacity: i64,
    /// Main (indices dans CardsDb.projects).
    pub hand: Vec<u16>,
    /// Cartes jouées.
    pub played: Vec<u16>,
    /// Corporation choisie (indice dans CardsDb.corporations).
    pub corporation: Option<u16>,
    /// Compteurs de tags en jeu (corporation incluse).
    pub tag_counts: [u32; TAG_COUNT],
    /// Compteurs de couleurs jouées (vert/bleu/rouge).
    pub color_counts: [u32; 3],
    /// Phase choisie cette ronde (1-5), 0 = pas encore choisie.
    pub chosen_phase: u8,
    /// Phase choisie à la ronde précédente (interdite cette ronde).
    pub previous_phase: Option<u8>,
    /// Activations bonus de la phase action, RELEVÉES AU DÉBUT DE LA PHASE III
    /// depuis le point de calcul unique `flow::selector_bonus` (base : +1 ;
    /// III-A : +1 ; III-B : +2).
    ///
    /// Écrit au début de la phase, et non à la planification : une amélioration
    /// gagnée en phase I ou II vaut dès cette manche-ci (livret l. 64, ASK 1).
    pub extra_blue_activations: u8,
    /// (Découverte) **Les cartes Phase améliorées de CE joueur**, une case par
    /// phase (indice = phase - 1), `None` = carte Phase normale. Améliorer une
    /// phase déjà améliorée écrase la case : A ↔ B, jamais deux à la fois
    /// (livret l. 66). Chaque joueur a les siennes : rien n'est partagé
    /// (NEVER 7, ASK 5).
    pub phase_upgrades: [Option<PhaseUpgrade>; 5],
    /// (corpo-1) Le NT de ce joueur a-t-il déjà été haussé pendant la phase en
    /// cours ? Remis à `false` au début de chaque phase réellement exécutée
    /// (`flow::play_round`, à côté de `snapshot_planet`). Sert au seul
    /// `TrBoost` d'Unmi (« The FIRST TIME your TR is raised EACH PHASE »).
    pub tr_raised_this_phase: bool,
    /// Compteur d'audit : nombre d'incréments de TR accordés (invariant TR).
    pub tr_increments: u64,
    /// Compteur d'audit : TR dépensés (« Requires you to spend 1 TR »).
    /// Invariant : tr == 5 + tr_increments - tr_decrements.
    pub tr_decrements: u64,
    /// (lot 3) Ressources posées sur les cartes EN JEU du joueur : identifiant
    /// de carte (indice dans `CardsDb.projects`) → quantité.
    ///
    /// `BTreeMap` et non une table de hachage : l'ordre d'itération doit être
    /// TOTALEMENT déterministe, puisque c'est lui qui ordonne la liste de
    /// candidats présentée à la politique (donc les tirages du RNG de la
    /// partie, donc la reproductibilité à graine fixe). Trié par identifiant de
    /// carte, comme l'exige le contrat.
    ///
    /// Une carte n'y entre QUE si elle porte un type de ressource
    /// (`CardEffects::holds`), à sa pose et à 0 (`Player.initResources` du
    /// moteur Java) : une carte non porteuse n'est jamais un réceptacle.
    pub card_resources: BTreeMap<u16, u32>,
    /// (lot cartes-8) **Permissions de pose supplémentaire en attente**, gagnées
    /// pendant la phase en cours et pas encore exercées. Une carte posée peut en
    /// ajouter : la file se vide donc en boucle, jusqu'à ce que le joueur
    /// renonce ou qu'il n'ait plus rien de posable
    /// (`flow::drain_pending_builds`).
    ///
    /// **Transitoire.** Vidée au début de chaque phase par `flow::play_round`,
    /// à côté de `tr_raised_this_phase` : « this phase » du texte imprimé ne
    /// franchit jamais une frontière de phase, même si le joueur n'a pas pu
    /// s'en servir.
    pub pending_builds: Vec<BuildGrant>,
    /// (lot cartes-8) **Modificateur armé pour la prochaine carte** posée par ce
    /// joueur dans la phase en cours. Cumulé à la pose de *Work Crews* ou de
    /// *Special Design*, consommé par la pose suivante, effacé en début de
    /// phase — mêmes règles de vie que `pending_builds`.
    pub next_card_mod: NextCardMod,
    /// **(jokers-corpos) Les JETONS BADGE posés sur les badges jokers de ce
    /// joueur** : identifiant de carte → badge choisi.
    ///
    /// Livret Découverte, « BADGE JOKER » : « le joueur qui l'a révélée choisit
    /// à quel badge équivaut le joker. […] vous devez prendre un jeton Badge
    /// correspondant au badge choisi et le placer sur le badge joker. Désormais,
    /// il déclenchera les effets relatifs à ce badge. »
    ///
    /// Une entrée PAR CARTE, jamais par joueur : deux cartes joker déclarées
    /// Terre valent deux badges Terre. L'entrée est écrite une seule fois
    /// (`flow::ensure_joker_tag` ne réécrit jamais une case occupée) : le choix
    /// est DÉFINITIF, comme le veut le carton.
    ///
    /// `BTreeMap` et non une table de hachage, pour la même raison que
    /// `card_resources` : l'ordre d'itération doit être totalement déterministe.
    ///
    /// Le badge choisi n'est pas un onzième badge : il est l'un des dix, et
    /// c'est `tags_of` qui fait la substitution.
    pub joker_tags: BTreeMap<u16, Tag>,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            mc: 0,
            heat: 0,
            plants: 0,
            tr: STARTING_TR,
            forests: 0,
            mc_prod: 0,
            heat_prod: 0,
            plant_prod: 0,
            card_prod: 0,
            steel_capacity: 0,
            titanium_capacity: 0,
            hand: Vec::new(),
            played: Vec::new(),
            corporation: None,
            tag_counts: [0; TAG_COUNT],
            color_counts: [0; 3],
            chosen_phase: 0,
            previous_phase: None,
            extra_blue_activations: 0,
            phase_upgrades: [None; 5],
            tr_raised_this_phase: false,
            tr_increments: 0,
            tr_decrements: 0,
            card_resources: BTreeMap::new(),
            pending_builds: Vec::new(),
            next_card_mod: NextCardMod::default(),
            joker_tags: BTreeMap::new(),
        }
    }

    /// **(jokers-corpos) Le badge choisi pour la carte `card_id`**, `None` si
    /// elle ne porte pas de badge joker ou si le choix n'a pas encore été fait.
    /// Lecture seule : l'écriture passe exclusivement par
    /// `flow::ensure_joker_tag`.
    pub fn joker_tag(&self, card_id: u16) -> Option<Tag> {
        self.joker_tags.get(&card_id).copied()
    }

    /// **(jokers-corpos) POINT DE LECTURE UNIQUE des badges d'une carte, vus par
    /// CE joueur** : les badges imprimés, le badge joker remplacé par le jeton
    /// posé dessus.
    ///
    /// Tout le moteur passe par ici dès qu'il lit les badges d'une carte qui
    /// appartient à un joueur — le décompte de `put_in_play`, la réduction de
    /// prix `flow::card_discount`, les déclencheurs de pose. Un joker déclaré
    /// Espace y est donc **Espace**, pas « Dynamic » : `TAG_COUNT` reste à 10 et
    /// `Tag::Dynamic` reste hors décompte, comme le veut le contrat.
    ///
    /// Un joker SANS jeton (choix pas encore fait, ou couche d'effets coupée)
    /// reste `Tag::Dynamic`, dont `Tag::index()` rend `None` : il ne compte
    /// nulle part, exactement le comportement d'avant ce chantier.
    pub fn tags_of(&self, db: &CardsDb, card_id: u16) -> Vec<Tag> {
        let printed = &db.projects[card_id as usize].tags;
        // Cas courant, 243 cartes sur 246 : aucun badge joker, rien à substituer.
        if !printed.iter().any(|t| t.is_joker()) {
            return printed.clone();
        }
        let token = self.joker_tag(card_id);
        printed
            .iter()
            .map(|&t| if t.is_joker() { token.unwrap_or(t) } else { t })
            .collect()
    }

    /// (lot 3) Ressources posées sur une carte donnée (0 si la carte ne porte
    /// rien ou n'est pas en jeu). Lecture seule : l'écriture passe
    /// exclusivement par `flow::add_resources` / `flow::remove_resources`.
    pub fn resources_on(&self, card_id: u16) -> u32 {
        self.card_resources.get(&card_id).copied().unwrap_or(0)
    }

    /// Fait entrer une carte en jeu (tags + couleur) — effet unique : aucun (stub).
    ///
    /// (jokers-corpos) Les badges comptés sont ceux que `tags_of` rend : le
    /// badge joker y est déjà remplacé par le jeton posé dessus. C'est LE point
    /// par lequel un joker déclaré Jupiter satisfait un prérequis « 1 badge
    /// Jupiter », alimente une production par badge, un point de victoire par
    /// badge, un Objectif ou une Récompense — il n'y en a pas d'autre.
    ///
    /// Renvoie `true` si un badge joker déterminé vient d'être compté (le
    /// compteur d'audit `joker_tag_hits` est incrémenté au site appelant, qui
    /// seul tient `GameState`).
    pub fn put_in_play(&mut self, card_id: u16, db: &CardsDb) -> bool {
        let card = &db.projects[card_id as usize];
        let tags = self.tags_of(db, card_id);
        let joker_compte = db.projects[card_id as usize]
            .tags
            .iter()
            .any(|t| t.is_joker())
            && self.joker_tag(card_id).is_some();
        for t in &tags {
            if let Some(i) = t.index() {
                self.tag_counts[i] += 1;
            }
        }
        self.color_counts[card.color.index()] += 1;
        self.played.push(card_id);
        joker_compte
    }

    pub fn played_count(&self, color: Color) -> u32 {
        self.color_counts[color.index()]
    }

    pub fn unique_tags(&self) -> u32 {
        self.tag_counts.iter().filter(|&&c| c > 0).count() as u32
    }

    /// (Découverte) L'amélioration installée sur la carte Phase `phase`
    /// (1..=5), `None` hors bornes ou si la carte n'est pas améliorée.
    pub fn phase_upgrade(&self, phase: u8) -> Option<PhaseUpgrade> {
        if (1..=5).contains(&phase) {
            self.phase_upgrades[phase as usize - 1]
        } else {
            None
        }
    }

    /// (Découverte) Installe une amélioration sur la carte Phase `phase` et dit
    /// si la phase était DÉJÀ améliorée (bascule A ↔ B). Écriture unique du
    /// tableau : c'est ici, et nulle part ailleurs, qu'une carte Phase change.
    pub fn upgrade_phase(&mut self, phase: u8, v: PhaseUpgrade) -> bool {
        assert!((1..=5).contains(&phase), "phase à améliorer hors bornes");
        let was = self.phase_upgrades[phase as usize - 1];
        self.phase_upgrades[phase as usize - 1] = Some(v);
        was.is_some()
    }

    /// (Découverte) Nombre de cartes Phase améliorées possédées — la valeur de
    /// la récompense VISIONNAIRE.
    pub fn phase_upgrades_count(&self) -> i64 {
        self.phase_upgrades.iter().filter(|u| u.is_some()).count() as i64
    }

    /// (Découverte) Étiquettes des améliorations installées, triées :
    /// `["1B", "5A"]`. Rendu par la sonde (`upgrades`).
    pub fn phase_upgrade_labels(&self) -> Vec<String> {
        (1u8..=5)
            .filter_map(|ph| self.phase_upgrade(ph).map(|v| format!("{ph}{}", v.label())))
            .collect()
    }

    /// Incrémente le TR (comptabilisé pour l'invariant de cohérence).
    pub fn gain_tr(&mut self) {
        self.tr += 1;
        self.tr_increments += 1;
    }

    /// Dépense `n` TR (« Requires you to spend n TR ») — comptabilisé pour
    /// l'invariant. Le prérequis (tr >= n) est vérifié en amont par la couche
    /// d'effets ; l'assert attrape tout chemin qui l'aurait contourné.
    pub fn spend_tr(&mut self, n: i64) {
        assert!(self.tr >= n, "dépense de TR sans le TR requis");
        self.tr -= n;
        self.tr_decrements += n as u64;
    }
}

/// État complet d'une partie.
pub struct GameState {
    pub rng: StdRng,
    /// Pioche projets (le dessus = fin du Vec).
    pub deck: Vec<u16>,
    pub discard: Vec<u16>,
    /// Paquet corporations restant.
    pub corp_deck: Vec<u16>,
    /// Corporations écartées (mulligan, non choisies).
    pub corp_discard: Vec<u16>,
    pub oceans: [OceanTile; 9],
    pub oceans_revealed: u8,
    /// Niveau de température (0..=19).
    pub temperature: u8,
    /// Niveau d'oxygène (0..=14).
    pub oxygen: u8,
    /// Niveau d'infrastructure (0..=14) — extension pour Grain Silos (B2),
    /// jamais monté par la pioche v1.
    pub infrastructure: u8,
    pub players: [PlayerState; NUM_PLAYERS],
    pub generation: u32,
    pub milestones: [MilestoneSlot; 3],
    pub awards: [AwardKind; 3],
    pub game_over: bool,
    /// Compteur d'audit : activations d'actions bleues ayant réellement appliqué
    /// leur effet (lot 2). 0 en `--effects off`.
    pub blue_actions: u64,
    // Instantané planétaire au début de la phase en cours (D6).
    pub snap_temperature: u8,
    pub snap_oxygen: u8,
    pub snap_oceans: u8,
    pub snap_infrastructure: u8,
    // ------------------------------------------------- lot 3 (conformité)
    /// (C4, règle maison) Premier joueur de la manche en cours. Manche 1 :
    /// joueur 0 ; alterne à chaque manche jouée entièrement.
    pub first_player: usize,
    /// (C4) Premier joueur de CHAQUE manche réellement jouée, dans l'ordre.
    /// Écrit par `flow::play_round` au début de la manche : c'est l'ordre
    /// effectivement emprunté par la boucle de jeu, pas une formule.
    ///
    /// Invariant sur une partie TERMINÉE (`game_over`) : `turn_order.len() ==
    /// generation`, parce que la manche qui déclenche la fin de partie sort de
    /// `play_round` avant l'incrément de `generation`. Sur une partie
    /// tronquée par le plafond de `sim::MAX_GENERATIONS` (jamais atteint en
    /// pratique), la liste compte une manche de moins que `generation`.
    pub turn_order: Vec<u8>,
    /// (C1) Compteur d'audit : nombre de fois qu'une carte payable a été exclue
    /// des options de construction parce que ses prérequis de paramètres
    /// n'étaient pas remplis sur l'INSTANTANÉ de début de phase, alors que
    /// l'état COURANT les aurait autorisés. 0 en `--effects off`.
    ///
    /// C'est un compteur d'EXCLUSIONS, pas de cartes distinctes : en phase II
    /// avec le bonus `SecondBuild`, l'énumération a lieu deux fois et une même
    /// carte bloquée est comptée deux fois. Le mécanisme est structurellement
    /// rare (il faut qu'un paramètre franchisse un palier PENDANT la phase, et
    /// qu'un joueur ait ensuite en main une carte payable gênée par ce
    /// palier) : de l'ordre de 2 à 10 par millier de parties aléatoires.
    pub prereq_snapshot_blocks: u64,
    /// (C2) Compteur d'audit : pioches du bonus de construction prises AVANT la
    /// pose de la carte de la phase.
    pub draw_before_build: u64,
    /// (C2) Idem, pioches prises APRÈS la pose.
    pub draw_after_build: u64,
    /// (C3) Compteur d'audit : nombre TOTAL de cartes défaussées pour payer des
    /// cartes Projet (3 MC / carte). Règle, donc actif aussi en `--effects off`.
    pub discard_payments: u64,
    /// (regles-de-la-vente) **La phase que le moteur résout à cet instant**, 1 à
    /// 5, ou **0 hors phase** : mise en place, planification de la manche, étape
    /// de fin de manche. Écrite par `flow::play_round`, le seul endroit qui
    /// connaisse la phase en cours.
    ///
    /// Elle existe parce que deux consommateurs doivent en dire la MÊME chose :
    /// l'écran, qui allume la carte Phase en cours, et l'occasion de vendre, qui
    /// n'est offerte que dans les phases où l'on peut dépenser. L'écran la
    /// déduisait naguère du TYPE de la décision reçue — une déduction juste mais
    /// qui n'avait aucun moyen de s'accorder avec le moteur sur l'étape de fin de
    /// manche, où elle gardait la dernière phase résolue.
    pub phase_en_cours: u8,
    /// (regles-de-la-vente) Une vente volontaire est-elle offerte au point de
    /// décision où le moteur se trouve ? Écrit par `flow::occasion_de_vendre`,
    /// juste avant chaque décision, et publié par `observe::state_view` : c'est
    /// ce drapeau que l'écran lit pour offrir — ou non — son bouton de vente.
    /// Sans lui, l'écran devrait deviner, et offrirait une vente que le moteur
    /// refuserait.
    pub vente_offerte: bool,
    /// (regles-de-la-vente) **Une occasion vient-elle d'être OUVERTE, et pas
    /// encore observée ?** Drapeau de passage, écrit par
    /// `flow::occasion_de_vendre` et CONSOMMÉ par `flow::observer` — qui le vide
    /// en le recopiant dans `vente_offerte`.
    ///
    /// Ce détour d'un seul champ est ce qui rend l'invariant impossible à
    /// oublier. `vente_offerte` était naguère écrit par la seule
    /// `occasion_de_vendre` : à un point de décision qui n'avait pas reçu son
    /// occasion, il gardait la valeur du point PRÉCÉDENT, c'est-à-dire vrai.
    /// L'écran offrait alors le bouton en toute bonne foi, la page inscrivait sa
    /// vente, et le rejeu ne rencontrait aucune occasion pour la consommer :
    /// elle tombait dans un point de décision, le pont la refusait, et la partie
    /// se figeait. Consommé, le drapeau vaut faux à tout point de décision qui
    /// n'a pas SON occasion : l'écran n'y offre rien, et une omission coûte au
    /// pire une vente non proposée — jamais une partie perdue.
    pub occasion_ouverte: bool,
    /// (regles-de-la-vente, round 2) **Les mains telles qu'elles étaient à la
    /// dernière occasion offerte**, une par joueur.
    ///
    /// Le drapeau de passage ci-dessus dit « une occasion a été ouverte pour ce
    /// point de décision-ci ». Il ne disait pas encore « et les indices que le
    /// joueur lira à l'écran désigneront les mêmes cartes ». Or plusieurs
    /// occasions sont hoistées au-dessus de ce qui prépare la question, et l'une
    /// d'elles — `Eff::DrawDiscard` — passait par-dessus une PIOCHE : le joueur
    /// voyait trois cartes neuves, l'écran lui offrait le bouton, et le rejeu
    /// replaçait sa vente sur la main d'avant la pioche. Le moteur refusait
    /// l'entrée et la partie se figeait.
    ///
    /// La cause a été corrigée à sa source (l'occasion suit la pioche), mais
    /// l'invariant est désormais VÉRIFIÉ plutôt que raisonné : `flow::observer`
    /// ne publie `vente_offerte` que si la main de chaque joueur est encore
    /// celle sur laquelle l'occasion a été offerte. Une occasion qu'on hoisterait
    /// demain au-dessus d'une pioche coûterait au pire un bouton non offert —
    /// jamais une partie perdue.
    pub mains_a_l_occasion: Vec<Vec<u16>>,
    /// (regles-de-la-vente) Compteur d'audit : cartes vendues VOLONTAIREMENT,
    /// hors phase III (l'action standard de vente a son propre chemin). C'est le
    /// pendant positif de `discard_payments` — celui-là doit valoir zéro,
    /// celui-ci prouve que la vente reste possible.
    pub ventes_volontaires: u64,
    // --------------------------------------- lot 3 (ressources sur les cartes)
    /// Ressources posées sur des cartes, comptées EN UNITÉS, incrémentées dans
    /// le service unique `flow::add_resources`, au moment exact de l'ajout.
    /// 0 en `--effects off`.
    pub res_added: u64,
    /// Idem pour les retraits (`flow::remove_resources`).
    pub res_removed: u64,
    /// Poses de ressources sautées faute de carte cible valide (l'effet est
    /// perdu, sans compensation d'aucune sorte). Incrémenté dans
    /// `flow::apply_res_*`, à l'endroit où la cible manque.
    pub res_targets_missing: u64,
    /// Améliorations de carte Phase demandées par une carte et NON gérées.
    ///
    /// **Vaut 0 depuis le chantier `decouverte-phases`** : `flow::apply_res_eff`
    /// applique désormais `ResEff::PhaseUpgrade` par `flow::apply_phase_upgrade`,
    /// qui ne renonce JAMAIS (il reste toujours au moins cinq améliorations
    /// possibles, la bascule A ↔ B comprise). Le compteur reste en place — c'est
    /// lui qui prouve que plus rien n'est sauté, et le jour où un texte
    /// imprimé introduirait une amélioration impossible à accorder, il le dirait.
    pub phase_upgrades_skipped: u64,
    // ------------------------------------------- (Découverte) cartes Phase
    // Quatre compteurs qui rendent le mécanisme des cartes Phase améliorées
    // observable EN PARTIE RÉELLE, et pas seulement sous la sonde. Chacun est
    // incrémenté à l'endroit EXACT du mécanisme, jamais dans une fonction de
    // résumé, jamais depuis la sonde. Tous nuls en `--effects off` : une
    // amélioration naît d'un effet de carte, et aucun bonus amélioré n'est lu
    // quand la couche d'effets est coupée.
    /// Améliorations de carte Phase RÉELLEMENT accordées
    /// (`flow::apply_phase_upgrade`).
    pub phase_upgrades_granted: u64,
    /// Parmi elles, celles qui portaient sur une phase DÉJÀ améliorée —
    /// bascules A ↔ B (livret l. 66).
    pub phase_upgrades_reupgraded: u64,
    /// Fois où le bonus du sélectionneur lu était celui d'une carte Phase
    /// AMÉLIORÉE, et remplaçait donc celui de la carte de base
    /// (`flow::selector_bonus_applied`). Une par phase et par joueur concerné.
    pub upgraded_bonus_applied: u64,
    /// Permissions de pose supplémentaire accordées par une carte Phase
    /// améliorée (I-B, II-A, II-B) — versées dans la file `pending_builds` du
    /// lot cartes-8, jamais dans une seconde file (NEVER 2).
    pub upgraded_extra_builds: u64,
    // ------------------------------------- (decouverte-projets) cinq de plus
    // Mêmes règles que les précédents : incrémentés au SITE EXACT du mécanisme,
    // jamais dans un résumé, jamais depuis la sonde ; tous nuls en
    // `--effects off` et en boîte de base seule (aucune des cartes qui les
    // alimentent n'appartient à la boîte de base).
    /// Améliorations portant sur une phase IMPOSÉE par le carton — D05
    /// (phase III), D37 (phase I), D40 (phase IV). Sous-ensemble strict de
    /// `phase_upgrades_granted` : `flow::apply_phase_upgrade` les compte au
    /// même endroit, quand son paramètre de phase vaut `Some(_)`.
    pub phase_upgrades_targeted: u64,
    /// Améliorations venues d'une ACTION de carte bleue — D07, D12, et
    /// *Fibrous Composite Material* qui le faisait déjà avant ce chantier.
    /// Sous-ensemble strict de `phase_upgrades_granted`, compté par la SOURCE
    /// de l'appel (`flow::UpgradeSource::Action`), jamais par la forme de
    /// l'encodage.
    pub phase_upgrades_by_action: u64,
    /// Gains liés aux cartes Phase AMÉLIORÉES révélées par leur porteur :
    /// D05 (« lorsque vous révélez une carte Phase améliorée, gagnez 1 MC »,
    /// levé par `flow::fire_upgraded_reveal`) et D06 (le supplément de son
    /// action, `PhaseBonus::require_upgraded`).
    pub upgraded_reveal_bonuses: u64,
    /// Fois où la condition « Si vous avez un Objectif » (D35) était VRAIE au
    /// moment de la pose, donc fois où les 4 chaleurs ont été versées
    /// (`flow::apply_eff`, `Eff::IfObjective`).
    pub objective_condition_hits: u64,
    /// Activations de l'action « piochez deux cartes, puis défaussez-en deux »
    /// (D11) qui ont réellement eu lieu (`flow::apply_action_eff`).
    pub draw_then_discard_uses: u64,
    /// (boites-1) Nombre de fois qu'une carte SANS ENCODAGE est entrée en jeu
    /// au cours de la partie : projet construit dont `effect` est `None`, ou
    /// corporation installée dont `effect` est `None`. Incrémenté à l'endroit
    /// exact de la mise en jeu (`flow::build_card_with`,
    /// `flow::install_corporation`), jamais recalculé après coup.
    ///
    /// C'est la contrepartie jouée du champ `effets_geres` de `--dump-deck` :
    /// aucun pouvoir imprimé n'est sauté en silence, chaque saut est compté.
    /// Il vaut donc 0 seulement si toutes les cartes jouées sont encodées.
    pub cards_effects_unhandled: u64,
    // ------------------------------------------- lot 4 (productions dérivées)
    /// (lot 4) MC crédités par la PRODUCTION DÉRIVÉE, tous joueurs, cumulés sur
    /// la partie. Incrémenté dans `flow::phase_production`, à l'endroit exact du
    /// crédit — jamais dans une fonction de résumé, jamais depuis la sonde.
    /// 0 en `--effects off` (`flow::derived_production` renvoie alors (0,0,0)).
    pub derived_mc: u64,
    /// (lot 4) Idem pour la chaleur.
    pub derived_heat: u64,
    /// (lot 4) Idem pour les plantes.
    pub derived_plants: u64,
    /// (lot 4) Pas de NT gagnés par `Eff::TrPerTag` (Terraforming Ganymede),
    /// comptés au moment de l'application de l'effet. 0 en `--effects off`.
    pub tr_from_tags: u64,
    /// (lot 4) Cartes supplémentaires RÉELLEMENT piochées en phase Recherche
    /// grâce au bonus permanent (Interplanetary Relations). Incrémenté dans
    /// `flow::phase_research`, au site de pioche. 0 en `--effects off`.
    pub research_extra_draws: u64,
    // ---------------------------------------- lot cartes-8 (poses de plus)
    // Cinq compteurs qui rendent les poses supplémentaires observables EN
    // PARTIE RÉELLE. Chacun est incrémenté à l'endroit exact du mécanisme.
    // Tous nuls en `--effects off` : la permission naît d'un effet de carte.
    /// Permissions de pose supplémentaire ACCORDÉES (`flow::grant_from_card`).
    pub extra_builds_granted: u64,
    /// Permissions RÉELLEMENT exercées, c'est-à-dire poses supplémentaires
    /// effectuées (`flow::drain_pending_builds`). Toujours ≤ `granted` : le
    /// texte imprimé dit « you MAY », et la politique peut y renoncer, ou
    /// n'avoir aucune carte posable.
    pub extra_builds_used: u64,
    /// Cartes posées SANS payer leur prix (`flow::build_card_granted`,
    /// permission `free`) — *Automated Factories*, *Tall Station*.
    pub free_builds: u64,
    /// Modificateurs « prochaine carte de la phase » ARMÉS
    /// (*Work Crews*, *Special Design*).
    pub next_card_mods_armed: u64,
    /// Modificateurs réellement CONSOMMÉS par une pose suivante. Peut être
    /// inférieur à `armed` : un modificateur meurt avec la phase s'il n'a
    /// trouvé aucune carte à modifier.
    pub next_card_mods_used: u64,
    // -------------------------------------------------- lot corporations
    // Quatre compteurs qui rendent les effets de corporation observables EN
    // PARTIE RÉELLE (et pas seulement en sonde). Chacun est incrémenté à
    // l'endroit exact du mécanisme, jamais dans une fonction de résumé. Tous
    // nuls en `--effects off` (les effets de corporation y sont coupés).
    /// Chaleur convertie en MC par Helion (`flow::top_up_mc_with_heat`).
    pub corp_heat_as_mc: u64,
    /// Forêts construites en plantes à prix réduit par Ecoline
    /// (`flow::forest_plant_cost`, relevé dans `flow::build_forest`).
    pub corp_forest_rebates: u64,
    /// Pas de NT achetés 6 MC par Unmi (`flow::gain_tr`).
    pub corp_tr_boosts: u64,
    /// Pas de NT accordés par un déclencheur de pose de corporation
    /// (Saturn Systems), comptés dans `flow::apply_trig_gain`.
    pub corp_trigger_tr: u64,
    // ------------------------------------------------------------- lot 6
    // Quatre compteurs qui rendent les mécanismes du lot 6 observables EN
    // PARTIE RÉELLE, et pas seulement sous la sonde. Chacun est incrémenté à
    // l'endroit EXACT du mécanisme (jamais dans une fonction de résumé, jamais
    // depuis la sonde), et tous sont nuls en `--effects off` — les effets de
    // carte y sont coupés.
    /// Bonus d'action accordés parce que le joueur avait choisi la phase
    /// imprimée sur la carte — `flow::apply_blue_action`.
    pub action_phase_bonuses: u64,
    /// Cartes défaussées comme COÛT d'une action —
    /// `flow::apply_blue_action`.
    pub action_discard_costs: u64,
    /// Cartes défaussées par un effet « piochez n puis défaussez d »
    /// (groupe C) — `flow::apply_eff`.
    pub draw_discard_discards: u64,
    /// Cartes RÉELLEMENT révélées du dessus de la pioche —
    /// `flow::reveal_top`.
    pub cards_revealed: u64,
    // ------------------------------------------------------- lot cartes-7
    // Deux compteurs qui rendent les mécanismes du lot 7 observables EN PARTIE
    // RÉELLE, et pas seulement sous la sonde. Chacun est incrémenté à l'endroit
    // EXACT du mécanisme, jamais dans une fonction de résumé, jamais depuis la
    // sonde ; tous deux nuls en `--effects off` (le service qui les porte y rend
    // 0 de lui-même).
    /// Actions standard payées MOINS CHER grâce à *Standard Technology* —
    /// incrémenté dans `flow::pay_standard_mc`, une fois par action réduite.
    pub standard_action_discounts: u64,
    /// MC gagnés par *Assembly Lines* sur l'activation d'une action de carte —
    /// incrémenté dans `flow::fire_card_action_triggers`, en MC.
    pub action_mc_bonuses: u64,
    // ---------------------------------------------------- lot jokers-corpos
    // Cinq compteurs, chacun incrémenté au SITE EXACT de son mécanisme, jamais
    // dans une fonction de résumé. Tous nuls en `--effects off` : les services
    // qui les portent y sortent avant d'agir.
    /// **Badges jokers réellement CHOISIS** — incrémenté dans
    /// `flow::ensure_joker_tag`, une fois par carte joker recevant son jeton.
    pub joker_tag_choices: u64,
    /// **Badges jokers qui ont réellement COMPTÉ** — incrémenté à la pose, quand
    /// un jeton déterminé entre dans les compteurs de badges du joueur
    /// (`PlayerState::put_in_play`, l'unique passage par lequel un badge compte
    /// pour les prérequis, les productions et points par badge, les Objectifs et
    /// les Récompenses). Toujours ≤ `joker_tag_choices` : une carte joker
    /// choisie peut rester en main.
    pub joker_tag_hits: u64,
    /// **Cartes Phase améliorées par la MISE EN PLACE d'une corporation** —
    /// incrémenté dans `flow::apply_phase_upgrade`, source `Setup`.
    pub corp_phase_upgrades_at_setup: u64,
    /// **MC versés en plus par un taux de défausse MAJORÉ** (*Composting
    /// Factory*, Exocorp) — incrémenté aux sites de crédit d'une défausse pour
    /// du MC, en MC au-delà du taux du livret.
    pub discard_bonus_mc: u64,
    /// **MC versés par un bonus d'action conditionné à la phase que le joueur a
    /// LUI-MÊME sélectionnée** (`PhaseBonus` à `phase != 0`) — incrémenté dans
    /// `flow::apply_action_spec`, au site du versement.
    pub action_phase_self_bonus: u64,
}

impl GameState {
    pub fn all_parameters_maxed(&self) -> bool {
        self.temperature == TEMPERATURE_MAX
            && self.oxygen == OXYGEN_MAX
            && self.oceans_revealed == NUM_OCEANS
    }

    /// Prend l'instantané planétaire de début de phase.
    pub fn snapshot_planet(&mut self) {
        self.snap_temperature = self.temperature;
        self.snap_oxygen = self.oxygen;
        self.snap_oceans = self.oceans_revealed;
        self.snap_infrastructure = self.infrastructure;
    }

    /// (C4) Les joueurs dans l'ordre du tour de la manche en cours.
    pub fn players_in_turn_order(&self) -> [usize; NUM_PLAYERS] {
        [self.first_player, (self.first_player + 1) % NUM_PLAYERS]
    }

    /// (C4) Nombre d'alternances observées dans l'ordre du tour réellement
    /// joué (comptées sur `turn_order`, pas déduites du nombre de manches).
    pub fn turn_order_switches(&self) -> u64 {
        self.turn_order
            .windows(2)
            .filter(|w| w[0] != w[1])
            .count() as u64
    }
}

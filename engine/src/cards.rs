//! Chargement de `cards.json` / `cards_v2.json` et base de cartes en mémoire.
//!
//! Depuis le chantier cartes-1 : TOUTES les cartes projets (green/blue/red)
//! sont chargées, avec leur drapeau `in_deck_v1` — la pioche ne contient que
//! les 248 cartes v1, mais la sonde `--probe` doit trouver aussi les cartes
//! hors pioche (Grain Silos, imposée — journal B2). Corporations : les 16
//! `in_deck_v1`. Les VP (`vp`, `vp_dynamic`) viennent de `cards_v2.json` ;
//! les effets du lot 1 sont résolus par nom dans la table statique
//! `effects::LOT1`.

use crate::effects::{self, CardEffects};
use serde::Deserialize;

/// Tags du jeu (livret de base p.5 + Discovery « wild tag »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Building,
    Space,
    Science,
    Plant,
    Microbe,
    Animal,
    Earth,
    Jupiter,
    Energy,
    Event,
    /// Tag wild de Discovery — stub neutre en v1 (compté comme aucun tag, D16).
    Dynamic,
}

pub const TAG_COUNT: usize = 10; // tags comptés (Dynamic exclu)

impl Tag {
    pub fn from_str(s: &str) -> Option<Tag> {
        match s {
            "BUILDING" => Some(Tag::Building),
            "SPACE" => Some(Tag::Space),
            "SCIENCE" => Some(Tag::Science),
            "PLANT" => Some(Tag::Plant),
            "MICROBE" => Some(Tag::Microbe),
            "ANIMAL" => Some(Tag::Animal),
            "EARTH" => Some(Tag::Earth),
            "JUPITER" => Some(Tag::Jupiter),
            "ENERGY" => Some(Tag::Energy),
            "EVENT" => Some(Tag::Event),
            "DYNAMIC" => Some(Tag::Dynamic),
            _ => None,
        }
    }

    /// Index dans les compteurs de tags ; None pour Dynamic (stub).
    pub fn index(self) -> Option<usize> {
        match self {
            Tag::Building => Some(0),
            Tag::Space => Some(1),
            Tag::Science => Some(2),
            Tag::Plant => Some(3),
            Tag::Microbe => Some(4),
            Tag::Animal => Some(5),
            Tag::Earth => Some(6),
            Tag::Jupiter => Some(7),
            Tag::Energy => Some(8),
            Tag::Event => Some(9),
            Tag::Dynamic => None,
        }
    }
}

/// Couleur d'une carte projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Green,
    Blue,
    Red,
}

impl Color {
    pub fn index(self) -> usize {
        match self {
            Color::Green => 0,
            Color::Blue => 1,
            Color::Red => 2,
        }
    }
}

/// Type de VP dynamiques portés en v1 (décompte du score). Les types portant
/// sur des ressources posées sur les cartes (ANIMAL, MICROBE, SCIENCE…)
/// valent 0 en v1 : `Unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpKind {
    Jupiter,
    Earth,
    Forest,
    BlueCard,
    AnyCard,
    /// Ressources sur cartes, non modélisées en v1 → 0 point.
    Unsupported,
}

/// VP dynamiques d'une carte : `points` par tranche de `resources` unités
/// comptées (sémantique Java `WinPointsService.getWinPoints` :
/// floor(n / resources) * points).
#[derive(Debug, Clone, Copy)]
pub struct VpDynamic {
    pub kind: VpKind,
    pub resources: i64,
    pub points: i64,
}

/// Carte projet : prix + tags + couleur + VP + effets du lot 1 (None = stub).
#[derive(Debug, Clone)]
pub struct ProjectCard {
    pub name: String,
    pub color: Color,
    pub price: i64,
    pub tags: Vec<Tag>,
    /// Dans la pioche v1 ? (les cartes hors pioche ne sont accessibles que
    /// par la sonde et les tests.)
    pub in_deck_v1: bool,
    /// VP fixes imprimés (extraction Mission A, 0 par défaut).
    pub vp: i64,
    pub vp_dynamic: Option<VpDynamic>,
    /// Effets déclaratifs (lot 1) ; None = stub neutre.
    pub effect: Option<&'static CardEffects>,
}

/// Corporation (stub neutre : tags + MC de départ = champ `price`, D3).
#[derive(Debug, Clone)]
pub struct Corporation {
    pub name: String,
    pub starting_mc: i64,
    pub tags: Vec<Tag>,
}

/// Base de cartes chargée une fois au démarrage.
pub struct CardsDb {
    pub projects: Vec<ProjectCard>,
    pub corporations: Vec<Corporation>,
    /// Nombre de cartes projets `in_deck_v1` (taille de la pioche complète —
    /// invariant de conservation).
    pub v1_project_count: usize,
    /// Interrupteur `--effects on|off` : `false` = squelette intégral
    /// (stubs neutres, ni prérequis ni VP de cartes au score).
    pub effects_on: bool,
}

#[derive(Deserialize)]
struct RawVpDynamic {
    #[serde(rename = "type")]
    kind: String,
    resources: i64,
    points: i64,
}

#[derive(Deserialize)]
struct RawCard {
    name: String,
    category: String,
    tags: Vec<String>,
    price: Option<i64>,
    in_deck_v1: bool,
    /// Champs Mission A (absents de l'ancien cards.json : défauts neutres).
    #[serde(default)]
    vp: i64,
    #[serde(default)]
    vp_dynamic: Option<RawVpDynamic>,
}

fn vp_kind(s: &str) -> VpKind {
    match s {
        "JUPITER" => VpKind::Jupiter,
        "EARTH" => VpKind::Earth,
        "FOREST" => VpKind::Forest,
        "BLUE_CARD" => VpKind::BlueCard,
        "ANY_CARD" => VpKind::AnyCard,
        _ => VpKind::Unsupported,
    }
}

impl CardsDb {
    /// Charge `cards_v2.json` (ou l'ancien `cards.json`, champs VP absents →
    /// défauts neutres). Projets : toutes les cartes green/blue/red ;
    /// corporations : les `in_deck_v1`. Effets par défaut : ACTIVÉS.
    pub fn load(path: &str) -> Result<CardsDb, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("lecture {path}: {e}"))?;
        let raw: Vec<RawCard> =
            serde_json::from_str(&data).map_err(|e| format!("parse {path}: {e}"))?;

        let mut projects = Vec::new();
        let mut corporations = Vec::new();

        for c in raw.into_iter() {
            let tags: Vec<Tag> = c
                .tags
                .iter()
                .filter_map(|t| Tag::from_str(t))
                .collect();
            match c.category.as_str() {
                "green" | "blue" | "red" => {
                    let color = match c.category.as_str() {
                        "green" => Color::Green,
                        "blue" => Color::Blue,
                        _ => Color::Red,
                    };
                    projects.push(ProjectCard {
                        effect: effects::lookup(&c.name),
                        in_deck_v1: c.in_deck_v1,
                        vp: c.vp,
                        vp_dynamic: c.vp_dynamic.as_ref().map(|d| VpDynamic {
                            kind: vp_kind(&d.kind),
                            resources: d.resources,
                            points: d.points,
                        }),
                        name: c.name,
                        color,
                        price: c.price.unwrap_or(0),
                        tags,
                    });
                }
                "corporation" if c.in_deck_v1 => corporations.push(Corporation {
                    name: c.name,
                    starting_mc: c.price.unwrap_or(0),
                    tags,
                }),
                // Hors périmètre v1 : corporations hors pioche,
                // buffedCorporation, crysis.
                _ => {}
            }
        }

        let v1_project_count = projects.iter().filter(|c| c.in_deck_v1).count();
        if v1_project_count == 0 || corporations.len() < 4 {
            return Err(format!(
                "base de cartes suspecte: {} projets v1, {} corporations",
                v1_project_count,
                corporations.len()
            ));
        }

        // Garde-fou : chaque entrée de la table d'effets correspond à
        // exactement une carte projet (sinon l'encodage dérive du JSON).
        for (name, _) in effects::LOT1 {
            let n = projects.iter().filter(|c| c.name == *name).count();
            if n != 1 {
                return Err(format!(
                    "table d'effets: carte '{name}' résolue {n} fois dans {path}"
                ));
            }
        }

        Ok(CardsDb {
            projects,
            corporations,
            v1_project_count,
            effects_on: true,
        })
    }
}

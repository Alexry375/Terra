//! Chargement de `cards.json` et base de cartes en mémoire.
//!
//! Périmètre v1 : seules les cartes `in_deck_v1 == true` sont chargées.
//! 248 cartes projets (pioche) + 16 corporations (paquet séparé).
//! Les effets uniques ne sont PAS interprétés (stub neutre) : une carte se
//! résume à (nom, couleur, prix, tags).

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

/// Carte projet (stub neutre : prix + tags + couleur, aucun effet).
#[derive(Debug, Clone)]
pub struct ProjectCard {
    pub name: String,
    pub color: Color,
    pub price: i64,
    pub tags: Vec<Tag>,
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
}

#[derive(Deserialize)]
struct RawCard {
    name: String,
    category: String,
    tags: Vec<String>,
    price: Option<i64>,
    in_deck_v1: bool,
}

impl CardsDb {
    /// Charge et filtre `cards.json` (in_deck_v1 uniquement).
    pub fn load(path: &str) -> Result<CardsDb, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("lecture {path}: {e}"))?;
        let raw: Vec<RawCard> =
            serde_json::from_str(&data).map_err(|e| format!("parse {path}: {e}"))?;

        let mut projects = Vec::new();
        let mut corporations = Vec::new();

        for c in raw.into_iter().filter(|c| c.in_deck_v1) {
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
                        name: c.name,
                        color,
                        price: c.price.unwrap_or(0),
                        tags,
                    });
                }
                "corporation" => corporations.push(Corporation {
                    name: c.name,
                    starting_mc: c.price.unwrap_or(0),
                    tags,
                }),
                other => {
                    return Err(format!(
                        "catégorie inattendue pour une carte in_deck_v1: {other}"
                    ))
                }
            }
        }

        if projects.is_empty() || corporations.len() < 4 {
            return Err(format!(
                "base de cartes suspecte: {} projets, {} corporations",
                projects.len(),
                corporations.len()
            ));
        }

        Ok(CardsDb {
            projects,
            corporations,
        })
    }
}

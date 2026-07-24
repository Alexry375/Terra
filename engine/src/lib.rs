//! Moteur de simulation Terraforming Mars: Ares Expedition (v1, 2 joueurs).
//!
//! Squelette : état de jeu, boucle de phases, production, paramètres globaux,
//! fin de partie, score, règles maison de mulligan. Les effets uniques des
//! cartes sont des stubs neutres (voir ARCHITECTURE.md).

pub mod cards;
pub mod flow;
pub mod policy;
pub mod sim;
pub mod state;

//! Moteur de simulation Terraforming Mars: Ares Expedition (v1, 2 joueurs).
//!
//! Squelette : état de jeu, boucle de phases, production, paramètres globaux,
//! fin de partie, score, règles maison de mulligan — plus la couche d'effets
//! déclarative du lot 1 (`effects`) et la sonde d'audit (`probe`). Les cartes
//! hors lot restent des stubs neutres (voir ARCHITECTURE.md).

pub mod boites;
pub mod cards;
pub mod effects;
pub mod flow;
pub mod observe;
pub mod policy;
pub mod probe;
pub mod sim;
pub mod state;

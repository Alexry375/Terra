//! Moteur de simulation Terraforming Mars: Ares Expedition (v1, 2 joueurs).
//!
//! Squelette : état de jeu, boucle de phases, production, paramètres globaux,
//! fin de partie, score, règles maison de mulligan — plus la couche d'effets
//! déclarative du lot 1 (`effects`) et la sonde d'audit (`probe`). Les cartes
//! hors lot restent des stubs neutres (voir ARCHITECTURE.md).

pub mod boites;
pub mod cards;
pub mod choice;
pub mod effects;
pub mod flow;
pub mod observe;
pub mod policy;
pub mod probe;
pub mod sim;
pub mod state;

// (D4) **LA COUCHE QUI DÉCRIT LES SITUATIONS ET CHOISIT LES COUPS, DANS LA
// BIBLIOTHÈQUE.** Ces cinq fichiers étaient déclarés par `#[path]` à
// l'intérieur de chaque programme exécutable : aucun test d'intégration ne
// pouvait les atteindre, et c'est là que vivaient les deux seuls défauts
// d'architecture connus (`docs/AUDIT_MOTEUR.md`, §D4). Ils sont désormais des
// modules publics comme les autres ; les programmes les empruntent au lieu de
// les recompiler chacun pour soi. Aucune ligne de logique n'a changé à la
// remontée — la preuve est dans `outputs/result.md` (mêmes décisions, à graine,
// poids et boîtes égaux).
pub mod description;
pub mod espion;
pub mod joueur;
pub mod rejeu;
pub mod reseau;

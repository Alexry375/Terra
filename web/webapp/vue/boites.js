// QUELLE BOÎTE EST SUR LA TABLE.
//
// (les-ecrans-manquants) Le jeu se joue en boîte de base seule, ou en boîte de
// base plus l'extension « Découverte ». Trois choses de l'écran en dépendent, et
// aucune ne le savait :
//
//   • les Objectifs et les Récompenses ne rapportent RIEN sans l'extension. Le
//     moteur les tient quand même en mémoire — `state.milestones` et
//     `state.awards` sont remplis dans les deux boîtes, seul le barème diffère —
//     et la page recopiait donc, en boîte de base, une tuile qui s'allume comme
//     « prise » et deux cases de points qui ne comptent pour personne. Elle
//     annonçait un avantage qui n'existe pas.
//
// LA PAGE NE DÉCIDE RIEN ICI. Elle ne recalcule aucun barème et ne devine aucune
// règle : elle apprend seulement, une fois par partie, ce que l'adresse ou le
// rendez-vous ont déjà fixé (`interface.js`, `lireAdresse`), et elle s'en sert
// pour ne pas MONTRER ce qui ne compte pas. Le score, lui, reste celui du
// moteur, part par part.
//
// Pourquoi un module plutôt qu'un argument : le décor est bâti (`batir`) AVANT
// que la composition ne soit connue — en ligne, elle arrive du rendez-vous, donc
// après. Le réglage doit donc pouvoir arriver plus tard que la construction, et
// être lu à chaque redessin.

/** Vrai quand l'extension « Découverte » est de la partie. */
let decouverte = false;

/**
 * Enregistre la composition de la partie qui commence.
 * `boites` est la chaîne du moteur : `"base"` ou `"base,decouverte"`.
 */
export function reglerBoites(boites) {
  const liste = String(boites || "")
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  decouverte = liste.includes("decouverte");
}

/**
 * Vrai quand les Objectifs et les Récompenses rapportent des points dans cette
 * partie — c'est-à-dire quand l'extension est là.
 */
export function honneursComptent() {
  return decouverte;
}

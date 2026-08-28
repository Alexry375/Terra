// LES QUESTIONS QUE LE MOTEUR POSE AUX DEUX JOUEURS EN MÊME TEMPS — mesurées,
// jamais écrites à la main.
//
// POURQUOI CE FICHIER EXISTE. La page portait cette liste en dur, à deux
// endroits qui ne se connaissaient pas : `vue/mots.js` en connaissait trois
// (`corp_mulligan`, `project_mulligan`, `pick_phase`) et `distant.js` une seule
// (`pick_phase`). Le moteur, lui, en pose CINQ. Les deux listes étaient fausses
// le jour où elles ont été écrites et l'écart s'est creusé lot après lot :
// c'est exactement ce qu'une liste écrite à la main fait — elle vieillit au
// premier lot suivant, sans que rien ne le dise.
//
// CE QU'EST UNE QUESTION POSÉE AUX DEUX JOUEURS EN MÊME TEMPS, ET COMMENT ON LE
// SAIT. La définition n'est pas une opinion, c'est une propriété OBSERVABLE de
// la suite des questions du moteur : un type est simultané quand **chacune** de
// ses occurrences, sur des parties entières, a pour voisine immédiate une
// occurrence du même type posée à l'AUTRE siège. Le moteur interroge toujours
// les deux sièges l'un après l'autre — il n'a pas d'autre façon de poser une
// question à deux personnes — et c'est cette paire, systématique et sans
// exception, qui distingue « les deux répondent en même temps » de « chacun son
// tour ». Une question posée à un seul joueur n'a pas de voisine ; une question
// posée aux deux par intermittence (la défausse de fin de manche) en a une fois
// sur deux, et n'est donc pas retenue.
//
// LE SEUIL, ET POURQUOI IL Y EN A UN. Un type vu une ou deux fois seulement
// pourrait être apparié par accident. On exige donc au moins autant
// d'occurrences que de parties mesurées avant de conclure quoi que ce soit.
//
// CE QUE ÇA COÛTE, MESURÉ. Six parties entières jouées hors écran par le moteur
// du navigateur : 3,0 s (base + Découverte) et 2,9 s (boîte de base) sur la
// machine de mesure du 22-08. Le résultat ne dépend que de la composition des
// boîtes : il est donc calculé UNE fois par page et par composition, et gardé.
//
// CE QU'IL NE FAUT PAS Y METTRE. Aucune règle du jeu, aucun nom de question
// écrit en dur. Ce fichier ne sait pas ce qu'est une corporation ni une carte
// Phase : il compte des voisinages dans une suite de questions.

import { creerPartie, jouerJusquAuBout } from "./partie.js";
import { fournisseurAleatoire } from "./fournisseurs.js";

/** Combien de parties entières on joue pour mesurer. */
export const PARTIES_MESUREES = 6;
/** La première graine de mesure. Elles n'ont rien à voir avec la partie jouée. */
export const PREMIERE_GRAINE = 7001;

// La mesure ne dépend que de la composition des boîtes ; on la garde.
const memoire = new Map();

/**
 * Joue `PARTIES_MESUREES` parties entières hors écran et rend l'ensemble des
 * types de question dont CHAQUE occurrence est appariée à l'autre siège.
 *
 * Reproductible : mêmes graines, mêmes tirages, même liste — les deux
 * navigateurs d'une partie en ligne obtiennent donc exactement le même
 * ensemble, sans avoir à se le dire.
 *
 * @param {object} pont    le pont wasm (`pont.js`)
 * @param {string} boites  la composition des boîtes de la partie en cours
 * @returns {Promise<Set<string>>}
 */
export async function mesurerQuestionsSimultanees(pont, boites) {
  const dejaVu = memoire.get(boites);
  if (dejaVu) return dejaVu;

  const compte = Object.create(null);
  const apparie = Object.create(null);

  for (let g = PREMIERE_GRAINE; g < PREMIERE_GRAINE + PARTIES_MESUREES; g++) {
    const p = creerPartie(pont, { graine: g, boites });
    const suite = [];
    try {
      await jouerJusquAuBout(
        p,
        [fournisseurAleatoire(g * 3, "a"), fournisseurAleatoire(g * 5, "b")],
        (pa) => suite.push({ t: pa.decision.type, j: pa.decision.joueur }),
      );
    } catch {
      // Une partie de mesure qui s'arrête ne condamne pas la mesure : les autres
      // suffisent, et le seuil dira si l'on en a assez vu.
      continue;
    }
    for (let i = 0; i < suite.length; i++) {
      const t = suite[i].t;
      compte[t] = (compte[t] || 0) + 1;
      const av = suite[i - 1];
      const ap = suite[i + 1];
      if ((av && av.t === t && av.j !== suite[i].j)
        || (ap && ap.t === t && ap.j !== suite[i].j)) {
        apparie[t] = (apparie[t] || 0) + 1;
      }
    }
  }

  const trouvees = new Set(
    Object.keys(compte).filter(
      (t) => compte[t] >= PARTIES_MESUREES && apparie[t] === compte[t],
    ),
  );
  memoire.set(boites, trouvees);
  return trouvees;
}

/** Efface la mesure gardée (utile aux bancs qui changent de boîtes). */
export function oublierQuestionsSimultanees() {
  memoire.clear();
  mesurees = new Set();
}

// ------------------------------------------- l'ensemble mesuré de LA partie
//
// Il est posé UNE fois, au démarrage de la partie, par `interface.js`. Les deux
// lecteurs — l'affichage (`vue/mots.js`) et le mode en ligne (`distant.js`) —
// lisent le même ensemble : c'est ce qui interdit qu'ils divergent, ce qui était
// exactement le défaut d'avant (trois types d'un côté, un seul de l'autre).

let mesurees = new Set();

/**
 * Pose la liste mesurée. Une mesure absente laisse l'ensemble VIDE : l'écran
 * n'annonce alors aucune simultanéité et le mode en ligne n'anticipe rien — ce
 * qui est muet, mais jamais faux. Une simultanéité inventée, elle, serait une
 * fuite ou un blocage.
 */
export function reglerQuestionsSimultanees(types) {
  mesurees = types instanceof Set ? types : new Set(types || []);
}

/** La question de ce type est-elle posée aux deux joueurs en même temps ? */
export function estSimultanee(type) {
  return mesurees.has(type);
}

/** L'ensemble mesuré, pour qui a besoin de le parcourir (bancs, journal). */
export function questionsSimultanees() {
  return mesurees;
}

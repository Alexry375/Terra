// LE GESTE — on attrape une carte de sa main et on la pose sur la table.
//
// UN SEUL CHEMIN, DEUX ENTRÉES. Le clic et le glisser-déposer ne sont pas deux
// mécaniques : ce sont deux façons d'entrer dans `poserLaCarte`, qui seule
// anime la pose et rend l'indice au moteur. Écrire deux chemins, ce serait
// n'en éprouver qu'un — et livrer l'autre cassé.
//
//   pointerup sans déplacement           -> `parClic`   ┐
//   pointerup au-dessus de la table      -> `parDepot`  ┘-> poserLaCarte()
//
// POURQUOI LES ÉVÉNEMENTS POINTEUR, ET PAS LE GLISSER-DÉPOSER NATIF. Le second
// (`dragstart`) ne fonctionne pas au doigt sur un écran tactile, et aucun
// contrôle automatique ne peut l'emprunter. Les événements pointeur, eux, sont
// exactement ceux d'une vraie souris et d'un vrai doigt : ce qu'une machine
// pilote est alors ce qu'un joueur fait.
//
// LA PAGE NE JUGE RIEN. Une carte se pose si et seulement si elle porte
// `data-choix` — c'est-à-dire si le moteur vient de l'énumérer. On ne calcule
// jamais qu'une carte est jouable ; on recopie.

import { attraper, tenir, poserSur, relacher } from "./anim.js";
import { cacher as cacherLoupe } from "./loupe.js";

// En deçà, le pointeur n'a pas voyagé : c'est un clic, pas un glissement.
const SEUIL = 6;

// La décision ouverte au geste, posée par `vue/scene.js` : { rang, repondre }.
let ouverte = null;
// Une pose est en cours : la carte vole encore, la réponse n'est pas partie. On
// refuse le second geste, sinon deux cartes partiraient pour une seule question.
let enVol = false;

/**
 * OUVRE LE GESTE. Appelé par la scène quand la décision se joue depuis la main.
 * @param {number}   rang      le rang de la décision, pour ne pas répondre à une autre
 * @param {Function} repondre  ce qui rend l'indice au moteur
 */
export function ouvrirGeste(rang, repondre) {
  ouverte = { rang, repondre };
  document.body.dataset.geste = "main";
}

/** Referme le geste : plus aucune carte de la main n'est posable. */
export function fermerGeste() {
  ouverte = null;
  // Une partie qui s'achève pendant un vol laisserait le verrou fermé, et la
  // partie suivante n'accepterait plus une seule carte.
  enVol = false;
  delete document.body.dataset.geste;
}

/** La zone où l'on dépose une carte pour la jouer. */
function table() {
  return document.querySelector("[data-table-siege]");
}

/** Le point est-il sur la table ? */
function surLaTable(x, y) {
  const t = table();
  if (!t) return false;
  const r = t.getBoundingClientRect();
  return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
}

/**
 * LA POSE — l'unique chemin. La carte quitte la main, voyage jusqu'à la table et
 * s'y pose ; c'est seulement une fois posée que l'indice part au moteur, sinon
 * l'écran se réécrirait sous la carte en vol.
 *
 * @param {Element} figure  la carte attrapée
 * @param {object}  prise   le fac-similé déjà en main, si on l'y tenait déjà
 */
export async function poserLaCarte(figure, prise = null) {
  if (enVol || !ouverte || !figure) {
    relacher(prise);
    if (figure) delete figure.dataset.enMain;
    return;
  }
  const brut = figure.dataset.choix;
  if (brut === undefined) { // le moteur ne l'a pas énumérée : elle ne se joue pas
    relacher(prise);
    delete figure.dataset.enMain;
    return;
  }
  const indice = Number(brut);
  if (!Number.isInteger(indice)) {
    relacher(prise);
    delete figure.dataset.enMain;
    return;
  }

  enVol = true;
  const { repondre: rendre, rang } = ouverte;
  // La carte est en l'air : sa place dans la main reste vide derrière elle.
  figure.dataset.enMain = "oui";
  cacherLoupe();
  const volante = prise || attraper(figure);
  try {
    await poserSur(volante, table(), { ms: 1100, tour: 5, grossir: 1.3 });
  } finally {
    relacher(volante);
    enVol = false;
  }
  // LA QUESTION A-T-ELLE CHANGÉ PENDANT LE VOL ? Elle le peut : au siège tenu
  // par un programme, celui-ci répond au bout de 320 ms alors que la carte vole
  // encore 1 100 ms. Rendre l'indice sans vérifier, c'est répondre à la question
  // SUIVANTE avec le choix de la précédente. Les cartes Phase ont la même garde
  // (`vue/scene.js`).
  if (!ouverte || ouverte.rang !== rang) {
    delete figure.dataset.enMain;
    return;
  }
  rendre(indice);
}

/**
 * ARME UNE CARTE DE LA MAIN. Les deux gestes sont branchés ici, sur les mêmes
 * événements pointeur — un clic EST un `pointerdown` suivi d'un `pointerup` sans
 * déplacement, on n'a donc rien de plus à écouter.
 */
export function armerCarte(figure) {
  let prise = null;

  figure.addEventListener("pointerdown", (e) => {
    if (e.button !== undefined && e.button !== 0) return;
    if (figure.dataset.choix === undefined || !ouverte) return;
    // Sans cela le navigateur commence sa propre sélection de texte ou son
    // propre glisser d'image, et le geste nous échappe au premier pixel.
    e.preventDefault();
    prise = { x: e.clientX, y: e.clientY, voyage: false };
    try {
      figure.setPointerCapture(e.pointerId);
    } catch {
      // Capture refusée (pointeur déjà relâché) : les écouteurs sur l'élément
      // suffisent pour un clic, qui est le cas restant.
    }
  });

  figure.addEventListener("pointermove", (e) => {
    if (!prise) return;
    const dx = e.clientX - prise.x;
    const dy = e.clientY - prise.y;
    if (!prise.volante && Math.hypot(dx, dy) < SEUIL) return;
    if (!prise.volante) {
      // ON SOULÈVE LA CARTE HORS DE LA MAIN. Un fac-similé part dans la couche de
      // vol : la bande de la main est en `overflow: hidden`, et la carte y serait
      // coupée net dès qu'elle en sortirait — on la verrait disparaître au lieu
      // de la voir voyager. L'agrandissement au survol, lui, se referme : il
      // n'a rien à faire au-dessus d'une carte qu'on est en train de porter.
      cacherLoupe();
      figure.dataset.enMain = "oui";
      prise.volante = attraper(figure);
    }
    tenir(prise.volante, dx, dy);
    const t = table();
    if (t) t.dataset.survolee = surLaTable(e.clientX, e.clientY) ? "oui" : "non";
  });

  const lacher = (e) => {
    if (!prise) return;
    const { volante } = prise;
    prise = null;
    const t = table();
    if (t) delete t.dataset.survolee;
    try {
      figure.releasePointerCapture(e.pointerId);
    } catch {
      // Rien à relâcher : la capture n'avait pas été prise.
    }
    if (!volante) {
      parClic(figure);
    } else if (surLaTable(e.clientX, e.clientY)) {
      parDepot(figure, volante);
    } else {
      // Lâchée hors de la table : la carte revient en main, rien n'est joué.
      relacher(volante);
      delete figure.dataset.enMain;
    }
  };
  figure.addEventListener("pointerup", lacher);
  figure.addEventListener("pointercancel", lacher);
}

/** PREMIÈRE ENTRÉE : on a cliqué la carte. Elle part seule se poser. */
function parClic(figure) {
  poserLaCarte(figure);
}

/** SECONDE ENTRÉE : on a lâché la carte sur la table. Elle finit son voyage. */
function parDepot(figure, volante) {
  poserLaCarte(figure, volante);
}

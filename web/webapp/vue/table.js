// LA TABLE DES PHASES — chaque joueur pose SA carte, et range la précédente.
//
// Sur une vraie table, chaque joueur a devant lui deux cartes Phase : celle qu'il
// vient de révéler, debout, et celle de la manche d'avant, COUCHÉE à côté. La
// seconde n'est pas un souvenir : c'est la règle en objet — « on ne peut pas
// jouer deux manches de suite la même carte Phase » (livret p. 10,
// `engine::flow::allowed_phases`). Elle dit ce que ce joueur ne peut pas
// reprendre.
//
// DEUX CARTES, PAS UNE. Quand les deux joueurs choisissent la même phase, on voit
// deux cartes — une par joueur, chacune devant son propriétaire. C'est la
// décision du joueur, le 02-08, qui revient sur son choix précédent.
//
// ------------------------------------------------------------------------
// LE PIÈGE DE SÉQUENCE, ET POURQUOI LA MISE DE CÔTÉ SE FAIT AU DÉBUT DE MANCHE
// ------------------------------------------------------------------------
//
// Le moteur écrit `previous_phase` AU MOMENT où chaque joueur choisit
// (`flow.rs:4389` : `chosen_phase = phase; previous_phase = Some(phase)`), et il
// interroge toujours le joueur 0 en premier. Relevé sur la graine 909 :
//
//     manche 2, question à j0 : previous_phase = [1, 5]   <- les deux de la manche 1
//     manche 2, question à j1 : previous_phase = [5, 5]   <- 5 EST le choix secret de j0
//
// Lire `previous_phase` au moment où chacun répond révélerait donc, au siège 1,
// la carte que le joueur 0 vient de poser face cachée. On la lit UNE FOIS, au
// premier `pick_phase` de la manche, où les deux valeurs sont encore celles de la
// manche précédente — et on la garde toute la manche. Les deux cartes sont ainsi
// de côté avant que quiconque ne choisisse, aux deux sièges.
//
// CE QUI RESTE CACHÉ. La carte Phase de la manche EN COURS de l'adversaire ne
// paraît pas tant que le moteur ne l'a pas révélée, c'est-à-dire tant que la
// planification dure. Celle de la manche PRÉCÉDENTE, elle, est publique : elle a
// été révélée à la manche d'avant.

import {
  imagePhase, imageAmelioration, phaseNom, phaseRomain, EQUIPAGES, nomJoueur,
} from "./materiel.js";

/**
 * L'amélioration que ce joueur possède sur cette phase, ou `null`. Lue sur
 * `players[].phase_upgrades`, la liste d'étiquettes que le moteur publie
 * (« 2B » = phase 2, amélioration B). On ne devine rien : pas d'étiquette,
 * pas d'amélioration.
 */
function amelioration(etat, joueur, phase) {
  if (!etat || !phase) return null;
  const p = (etat.players || []).find((x) => x.player === joueur);
  const liste = p && p.phase_upgrades;
  if (!Array.isArray(liste)) return null;
  return liste.find((c) => Number(String(c)[0]) === Number(phase)) || null;
}
import { survolableImage } from "./loupe.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";
import { enPlanification, phaseEnCours } from "./phases.js";
import {
  voler, coucher, duree, mettreEnScene, oublierMiseEnScene,
} from "./anim.js";

// Ce que la table retient d'une manche à l'autre.
let precedentes = [null, null]; // la phase de la manche d'avant, par joueur
let mancheDesPrecedentes = 0; // pour ne relever qu'une fois par manche
let maPhase = null; // { manche, phase } — ce que JE viens de poser
let derniereSignature = "";

export function construireTable() {
  const z = document.createElement("aside");
  z.className = "table-phases";
  z.id = "table-phases";
  z.innerHTML =
    `<span class="table-phases__mot">${MOT.phaseTable}</span>` +
    [0, 1].map((j) =>
      `<div class="dock" id="dock-${j}" data-dock="${j}" style="--teinte:${EQUIPAGES[j].teinte}">` +
      `<span class="dock__qui">${nomJoueur(j)}</span>` +
      `<div class="dock__courante" id="dock-courante-${j}"></div>` +
      `<div class="dock__precedente" id="dock-precedente-${j}"></div>` +
      `</div>`).join("");
  document.body.appendChild(z);
}

/** Où la carte Phase du joueur `j` va se poser — la cible du vol. */
export function emplacementPhase(j) {
  return ref("#dock-courante-" + j);
}

/**
 * LA POSE. La carte choisie quitte la liste du milieu et vient se poser sur la
 * table, en tournant. On rend la promesse : la réponse au moteur n'est envoyée
 * qu'une fois la carte posée, sinon l'écran se réécrirait sous la carte en vol.
 *
 * @param {Element} source  la carte cliquée dans la liste
 * @param {number}  phase   le numéro de phase choisi
 * @param {number}  siege   le joueur qui pose
 * @param {number}  manche  la manche en cours
 */
export async function poserPhase(source, phase, siege, manche) {
  maPhase = { manche, phase };
  await voler(source, emplacementPhase(siege), { ms: 700, tour: 360, grossir: 1.1 });
}

/**
 * Réécrit la table des phases.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 * @param {number} siege     le joueur assis en bas de l'écran
 */
export function majTable(etat, decision, siege) {
  // (cartes-qui-bougent) CE QUI VIENT D'ARRIVER SE MET EN SCÈNE ICI, et pas
  // ailleurs. `interface.js` appelle `majTable` APRÈS `majPlateaux`, `majJoueurs`
  // et `majMains` : les places d'arrivée (la main, les deux plateaux, les deux
  // barres) sont déjà celles de l'état nouveau, et un vol qui les vise ne se
  // trompe pas d'endroit. C'est le dernier point de rendu qui appartienne à ce
  // chantier — `interface.js` ne lui appartient pas.
  //
  // L'appel est AVANT toute sortie anticipée de cette fonction : la table des
  // phases, elle, ne se redessine que si sa signature a changé, et la plupart
  // des évènements du jeu ne la changent pas.
  mettreEnScene(etat, siege);

  const manche = etat.generation || 0;

  // LE RELEVÉ DU DÉBUT DE MANCHE — une seule fois, au premier `pick_phase`, pour
  // les deux joueurs à la fois. Voir l'en-tête : plus tard, la valeur ment.
  if (decision && decision.type === "pick_phase" && manche !== mancheDesPrecedentes) {
    mancheDesPrecedentes = manche;
    precedentes = etat.players.map((p) => p.previous_phase || null);
  }

  const planifie = enPlanification();
  const courantes = etat.players.map((p, j) => {
    // MA carte : je viens de la poser, je sais donc laquelle c'est avant même que
    // le moteur ne la révèle — c'est mon propre choix, pas une fuite.
    if (planifie) {
      return j === siege && maPhase && maPhase.manche === manche ? maPhase.phase : null;
    }
    // Celle d'en face : seulement une fois la planification close, c'est-à-dire
    // une fois que le moteur l'a révélée (livret l. 272).
    return p.chosen_phase || null;
  });

  const enJeu = phaseEnCours();
  const signature =
    `${manche}#${courantes.join(",")}#${precedentes.join(",")}#${enJeu}#${siege}`;
  if (signature === derniereSignature) return;
  const changementDeManche = mancheDesPrecedentes === manche &&
    !derniereSignature.startsWith(`${manche}#`);
  derniereSignature = signature;

  for (const j of [0, 1]) {
    // LA CARTE POSÉE EST CELLE QU'ON POSSÈDE, améliorée ou non — même règle que
    // dans la scène où on la choisit (`vue/scene.js`). Sans cela on choisissait
    // sa carte améliorée et on en voyait poser une de base sur la table.
    dessinerCase(ref("#dock-courante-" + j), courantes[j], j, enJeu, false,
      amelioration(etat, j, courantes[j]));
    const boite = ref("#dock-precedente-" + j);
    const neuve = dessinerCase(boite, precedentes[j], j, 0, true,
      amelioration(etat, j, precedentes[j]));
    // LA CARTE TOURNE. On ne la découvre pas couchée : on la VOIT se coucher, au
    // moment où la manche s'ouvre. Ailleurs (redessin ordinaire) elle est déjà en
    // place, et rejouer la rotation serait un tic.
    if (changementDeManche && neuve) coucher(neuve, 700);
  }
}

/**
 * Une case de la table : la carte Phase qu'elle porte, ou rien.
 *
 * ON NE REMPLACE QUE CE QUI CHANGE. Une carte qui n'a pas bougé garde son nœud :
 * sans cela, la révélation de la carte d'en face arrachait aussi la mienne du
 * document, alors qu'elle est posée là depuis un moment. Un nœud arraché sous le
 * curseur, c'est un survol qui n'aboutit pas — et, pour une machine qui pilote la
 * page, une erreur franche.
 *
 * @param {boolean} couchee  la carte est celle de la manche d'avant, à plat
 * @returns {Element|null}   la carte si elle vient d'être posée, sinon `null`
 */
function dessinerCase(boite, phase, joueur, enJeu, couchee, code = null) {
  if (!boite) return null;
  // Le code d'amélioration entre dans la clé : sans lui, une carte déjà posée
  // garderait son image de base le jour où le joueur l'améliore.
  const cle = phase ? `${joueur}:${phase}:${code || ""}` : "";
  if (boite.dataset.cle === cle) {
    // Même carte : seule la mise en lumière de la phase en cours peut avoir
    // changé, et elle se pose sans rien reconstruire.
    const f = boite.firstElementChild;
    if (f && !couchee) {
      if (phase === enJeu) f.dataset.phaseEnCours = "oui";
      else delete f.dataset.phaseEnCours;
    }
    return null;
  }
  boite.dataset.cle = cle;
  boite.textContent = "";
  if (!phase) return null;

  const src = (code && imageAmelioration(code)) || imagePhase(phase);
  const f = document.createElement("figure");
  f.className = "carte phase-posee" + (couchee ? " phase-posee--couchee" : "");
  // Chaque carte porte SON joueur : c'est ce qui distingue deux cartes de la même
  // phase, une par joueur, de la carte unique qu'on montrait avant.
  f.dataset.phasePosee = `${joueur}:${phase}`;
  if (couchee) {
    f.dataset.phasePrecedente = "oui";
  } else {
    f.dataset.phaseChoisie = String(phase);
    if (phase === enJeu) f.dataset.phaseEnCours = "oui";
  }

  const im = document.createElement("img");
  im.src = src;
  im.alt = code
    ? `upgraded Phase card ${phaseNom(phase)} ${code.slice(1)}`
    : `Phase card ${phaseNom(phase)}`;
  im.draggable = false;
  f.appendChild(im);

  const t = document.createElement("span");
  t.className = "phase-posee__mot";
  t.textContent = `${phaseRomain(phase)} · ${phaseNom(phase)}`;
  f.appendChild(t);

  survolableImage(f, src, `phase ${joueur}:${phase}`);
  boite.appendChild(f);
  // La carte arrive : elle se pose, elle n'apparaît pas.
  if (!couchee && duree(1)) f.classList.add("phase-posee--arrive");
  return f;
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierTable() {
  // (cartes-qui-bougent) La mémoire de la mise en scène part avec le reste :
  // sans cet oubli, la première main de la partie suivante serait lue comme une
  // pioche géante par rapport à la dernière de la précédente.
  oublierMiseEnScene();
  precedentes = [null, null];
  mancheDesPrecedentes = 0;
  maPhase = null;
  derniereSignature = "";
  for (const j of [0, 1]) {
    for (const s of ["#dock-courante-", "#dock-precedente-"]) {
      const z = ref(s + j);
      if (!z) continue;
      delete z.dataset.cle;
      z.textContent = "";
    }
  }
}

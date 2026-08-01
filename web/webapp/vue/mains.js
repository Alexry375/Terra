// LES DEUX MAINS DU JOUEUR QUI DÉCIDE — les projets à droite, les Phases à gauche.
//
// Une main tenue en éventail : les cartes projet se décalent vers la droite de
// l'écran, en arc de cercle, comme si on les tenait. La seconde main — les
// cartes Phase — borde le côté gauche.
//
// CE QUI EST JOUABLE. Une carte porte `data-jouable="oui"` SI ET SEULEMENT SI le
// moteur vient de l'énumérer parmi les options de la décision en cours. C'est
// une recopie d'identifiants, pas un jugement : la page ne sait pas ce que coûte
// une carte, et ne le saura jamais.

import { carte } from "./cartes.js";
import { imagePhase, imageAmelioration, phaseNom, phaseRomain, nomJoueur } from "./materiel.js";
import { survolable } from "./loupe.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";

// L'ÉVENTAIL. Les cartes tournent autour d'un point situé loin à leur droite —
// le poignet de la main qui les tient. Deux réglages seulement :
//
//   • l'ouverture (le pas d'angle entre deux cartes) reste modeste, sinon les
//     cartes se couchent et deviennent illisibles ;
//   • le PAS VERTICAL entre deux cartes est choisi pour que la main remplisse
//     la hauteur disponible sans en sortir.
//
// Le rayon — donc la distance du poignet — se DÉDUIT de ces deux-là :
// rayon = pas vertical / sin(pas d'angle). Rien n'est posé au hasard.
const LARGEUR = 112; // doit suivre `--cw` de la feuille de style
const HAUTEUR = 156; // doit suivre `--ch`
const ANGLE_MAX = 20; // ouverture totale de l'éventail, en degrés, de part et d'autre
const PAS_MAX = 7; // angle entre deux cartes voisines, au plus
const RECOUVREMENT = 0.5; // deux cartes voisines ne s'écartent pas de plus de la moitié

export function construireMains() {
  const projets = document.createElement("aside");
  projets.className = "main main--projets";
  projets.id = "main-projets";
  projets.innerHTML =
    `<span class="main__mot" id="main-mot">${MOT.hand}</span>` +
    `<div class="main__arc" id="arc"></div>`;
  document.body.appendChild(projets);

  const phases = document.createElement("aside");
  phases.className = "main main--phases";
  phases.id = "main-phases";
  phases.innerHTML =
    `<span class="main__mot" id="phases-mot">${MOT.stages}</span>` +
    `<div class="phases-main" id="phases-main"></div>`;
  document.body.appendChild(phases);
}

/**
 * Réécrit les deux mains de celui qui décide.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 */
export function majMains(etat, decision) {
  const j = decision ? decision.joueur : etat.first_player ?? 0;
  const p = etat.players[j];
  if (!p) return;

  // Les identifiants que le moteur vient d'énumérer : rien d'autre n'est jouable.
  const proposees = new Set();
  for (const o of (decision && decision.options) || []) {
    const c = o && o.carte ? o.carte : o;
    if (c && c.id !== undefined && c.id !== null) proposees.add(String(c.id));
  }

  eventail(j, p, proposees, !!decision);
  cartesPhase(j, p);
}

/** L'éventail des cartes projet, à droite. */
function eventail(j, p, proposees, active) {
  const z = ref("#arc");
  if (!z) return;
  ref("#main-mot").textContent = `${MOT.hand} · ${nomJoueur(j)}`;

  const signature =
    j + "#" + p.hand.map((c) => c.id).join("|") + "#" + [...proposees].sort().join(",") +
    "#" + (active ? "1" : "0");
  if (z.dataset.signature === signature) return;
  z.dataset.signature = signature;
  z.textContent = "";

  const n = p.hand.length;
  const { pas, pivot } = ouverture(n, z.clientHeight || 860);
  z.style.setProperty("--pivot", Math.round(pivot) + "px");
  p.hand.forEach((c, i) => {
    const f = carte(c, { classe: "carte--main" });
    f.dataset.carteId = String(c.id);
    // Recopie stricte : jouable = énumérée par le moteur, un point c'est tout.
    if (active) f.dataset.jouable = proposees.has(String(c.id)) ? "oui" : "non";
    f.style.setProperty("--angle", (((i - (n - 1) / 2) * pas)).toFixed(2) + "deg");
    f.style.zIndex = String(i + 1);
    survolable(f, c);
    z.appendChild(f);
  });
}

/**
 * L'ouverture de l'éventail pour `n` cartes dans une hauteur `hauteur`.
 * @returns {{pas: number, pivot: number}} l'angle entre deux cartes, en degrés,
 *          et la distance du poignet mesurée depuis le bord gauche d'une carte.
 */
function ouverture(n, hauteur) {
  if (n <= 1) return { pas: 0, pivot: 600 };
  const pas = Math.min((2 * ANGLE_MAX) / (n - 1), PAS_MAX);
  // Le pas vertical : au plus la moitié d'une carte, et jamais plus que ce que
  // la hauteur disponible permet — la main ne sort pas de son bord.
  const ecart = Math.min(RECOUVREMENT * HAUTEUR, (hauteur - HAUTEUR) / (n - 1));
  const rayon = ecart / Math.sin((pas * Math.PI) / 180);
  return { pas, pivot: rayon + LARGEUR / 2 };
}

/**
 * La seconde main : les cinq cartes Phase. Celle que le moteur dit choisie est
 * mise en avant ; celles que `phase_upgrades` déclare améliorées montrent leur
 * face améliorée, celle qui est réellement imprimée.
 */
function cartesPhase(j, p) {
  const z = ref("#phases-main");
  if (!z) return;
  ref("#phases-mot").textContent = `${MOT.stages} · ${nomJoueur(j)}`;

  const ups = p.phase_upgrades || [];
  const signature = `${j}#${p.chosen_phase || 0}#${p.previous_phase || 0}#${ups.join(",")}`;
  if (z.dataset.signature === signature) return;
  z.dataset.signature = signature;
  z.textContent = "";

  for (let n = 1; n <= 5; n++) {
    const code = ups.find((u) => Number(u[0]) === n);
    const src = code ? imageAmelioration(code) : imagePhase(n);
    const d = document.createElement("div");
    d.className = "phase-main";
    d.dataset.phase = String(n);
    if (p.chosen_phase === n) d.classList.add("phase-main--choisie");
    if (p.previous_phase === n) d.classList.add("phase-main--precedente");
    if (code) d.classList.add("phase-main--amelioree");
    const im = document.createElement("img");
    im.src = src || imagePhase(n);
    im.alt = `Phase card ${phaseNom(n)}`;
    im.draggable = false;
    d.appendChild(im);
    const t = document.createElement("span");
    t.className = "phase-main__mot";
    t.textContent = phaseRomain(n);
    d.appendChild(t);
    z.appendChild(d);
  }
}

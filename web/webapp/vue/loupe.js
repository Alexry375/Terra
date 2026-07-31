// LA LOUPE — n'importe quelle carte se lit en grand, au survol.
//
// Les cartes de la main sont petites : c'est leur place. Mais un joueur doit
// pouvoir lire la sienne sans plisser les yeux. La loupe montre la carte en
// grand SANS RIEN DÉPLACER : elle est en `position: fixed` et surtout en
// `pointer-events: none`. Elle ne peut donc jamais recouvrir un choix cliquable
// — ni pour une main, ni pour une machine qui pilote la page.

import { imageCarte } from "./materiel.js";
import { normaliser } from "./cartes.js";

const LARGEUR = 300;
const MARGE = 16;

// La loupe ne s'ouvre que sur un survol VOULU. Quand une décision se pose, le
// curseur se retrouve immobile au-dessus d'une carte qui vient d'apparaître : le
// navigateur envoie alors un survol que personne n'a demandé. On l'ignore
// jusqu'au prochain mouvement réel de la souris.
let gele = true;

export function construireLoupe() {
  const l = document.createElement("div");
  l.id = "loupe";
  l.innerHTML = "<img alt=''>";
  document.body.appendChild(l);
  document.addEventListener("mousemove", () => { gele = false; }, { passive: true });
}

/** Appelé quand l'écran change sous un curseur immobile. */
export function figer() {
  gele = true;
}

/** Rend un élément survolable : au survol, sa carte s'affiche en grand. */
export function survolable(element, c) {
  const n = normaliser(c);
  if (!n) return;
  const src = imageCarte(n.nom);
  if (!src) return;
  element.addEventListener("mouseenter", () => montrer(src, element));
  element.addEventListener("mouseleave", cacher);
}

function montrer(src, ancre) {
  if (gele) return;
  const l = document.getElementById("loupe");
  if (!l) return;
  const im = l.firstElementChild;
  if (im.getAttribute("src") !== src) im.src = src;

  const hauteur = LARGEUR * (569 / 409);
  const r = ancre.getBoundingClientRect();

  // À côté de la carte survolée si la place existe, sinon de l'autre côté.
  let x = r.right + MARGE;
  if (x + LARGEUR > window.innerWidth - MARGE) x = r.left - LARGEUR - MARGE;
  x = Math.max(MARGE, Math.min(x, window.innerWidth - LARGEUR - MARGE));

  let y = r.top + r.height / 2 - hauteur / 2;
  y = Math.max(MARGE, Math.min(y, window.innerHeight - hauteur - MARGE));

  l.style.left = Math.round(x) + "px";
  l.style.top = Math.round(y) + "px";
  l.classList.add("loupe--visible");
}

export function cacher() {
  document.getElementById("loupe")?.classList.remove("loupe--visible");
}

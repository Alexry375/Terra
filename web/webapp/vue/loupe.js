// LA LOUPE — n'importe quelle carte se lit en grand, au survol.
//
// Les cartes de la main sont petites : c'est leur place. Mais un joueur doit
// pouvoir lire la sienne sans plisser les yeux. La loupe montre la carte en
// grand SANS RIEN DÉPLACER : elle est en `position: fixed` et surtout en
// `pointer-events: none`. Elle ne peut donc jamais recouvrir un choix cliquable
// — ni pour une main, ni pour une machine qui pilote la page.
//
// TOUTE CARTE S'AGRANDIT, SANS EXCEPTION. Les cartes Phase et les cartes Phase
// améliorées ne sont pas des cartes projet : elles n'ont ni identifiant ni fiche,
// seulement une image imprimée. Elles passent donc par `survolableImage`, et le
// résultat au survol est le même — c'est tout ce que le joueur demande.

import { normaliser, carte } from "./cartes.js";

// La largeur MINIMALE de la loupe est posée ICI, et la feuille de style ne la
// redéfinit pas : c'est le même nombre qui dessine l'image et qui calcule sa
// place. Deux constantes désaccordées, et la carte sort de l'écran par le bas —
// c'est-à-dire que le seul moyen de lecture prévu ne lit plus rien.
const LARGEUR = 348;
const MARGE = 16;
const RATIO = 569 / 409;

// AGRANDIR VEUT DIRE AGRANDIR. L'ancienne règle refusait d'ouvrir la loupe
// au-dessus d'une carte déjà grande — d'où des cartes qui ne s'agrandissaient
// pas, ce que le joueur a signalé le 02-08. La loupe vaut désormais au moins
// `LARGEUR`, et au moins ce facteur fois la carte survolée : elle apporte
// toujours quelque chose, quelle que soit la taille de départ.
const GAIN = 1.35;

// LE SURVOL QUE PERSONNE N'A DEMANDÉ. Quand une décision se pose, le curseur se
// retrouve immobile au-dessus d'une carte qui vient d'apparaître : le navigateur
// envoie alors un survol involontaire. On le reconnaît à ceci que le pointeur est
// EXACTEMENT là où il était quand l'écran a changé.
//
// On compare des positions, et non plus un drapeau levé par `mousemove` : ce
// drapeau ne se levait jamais à temps, parce que `mouseenter` précède
// `mousemove`. Résultat, le premier survol après chaque décision n'ouvrait rien —
// c'est-à-dire une carte sur deux, pour qui survole carte après carte.
let pointeur = { x: -1, y: -1 };
let gel = null;

export function construireLoupe() {
  const l = document.createElement("div");
  l.id = "loupe";
  document.body.appendChild(l);
  document.addEventListener("mousemove", (e) => {
    pointeur = { x: e.clientX, y: e.clientY };
  }, { passive: true });
}

/** Appelé quand l'écran change sous un curseur immobile. */
export function figer() {
  gel = { ...pointeur };
}

/** Le pointeur a-t-il bougé depuis que l'écran a changé ? */
function volontaire(e) {
  const x = e && e.clientX !== undefined ? e.clientX : pointeur.x;
  const y = e && e.clientY !== undefined ? e.clientY : pointeur.y;
  const immobile = !!gel && gel.x === x && gel.y === y;
  pointeur = { x, y };
  if (immobile) return false;
  gel = null;
  return true;
}

/**
 * Rend un élément survolable : au survol, sa carte s'affiche en grand.
 * @param {Element} element  ce qu'on survole
 * @param {object}  c        la carte, dans n'importe laquelle des formes du moteur
 */
export function survolable(element, c) {
  const n = normaliser(c);
  if (!n) return;
  // Deux cartes du jeu n'ont pas d'image découpée : `cartes.js` sait déjà les
  // montrer (un dos et leur nom en clair). On agrandit CE rendu-là plutôt que de
  // ne rien ouvrir — une carte qui ne s'agrandit pas est le défaut qu'on répare.
  // `points: true` — (MOT-15) LA CARTE AGRANDIE DIT CE QUE SES RESSOURCES
  // RAPPORTENT DÉJÀ. C'est le geste que LIS-3 demandait : on agrandit une carte
  // pour la lire, et « 4 microbes » ne disait pas combien de points cela fait.
  // La petite carte, elle, garde sa seule pastille de compte : il n'y a pas la
  // place d'y écrire une phrase, et la loupe est à un survol.
  brancher(element, () => carte(n, { classe: "carte--loupe", muette: false, points: true }),
    n.nom || String(n.id));
}

/**
 * Rend survolable un élément dont le sujet est une IMAGE imprimée sans fiche de
 * carte : une carte Phase, une carte Phase améliorée, un dos.
 *
 * @param {Element} element   ce qu'on survole
 * @param {string}  src       l'image à montrer en grand
 * @param {string}  identite  ce qu'on agrandit, pour qui nous lit du dehors
 */
export function survolableImage(element, src, identite) {
  if (!src) return;
  brancher(element, () => {
    const f = document.createElement("figure");
    f.className = "carte carte--loupe";
    const im = document.createElement("img");
    im.src = src;
    im.alt = identite;
    im.draggable = false;
    f.appendChild(im);
    return f;
  }, identite);
}

function brancher(element, fabriquer, identite) {
  element.addEventListener("mouseenter", (e) => {
    if (!volontaire(e)) return;
    montrer(fabriquer(), identite, element);
  });
  element.addEventListener("mouseleave", cacher);
}

function montrer(contenu, identite, ancre) {
  const l = document.getElementById("loupe");
  if (!l) return;

  const r = ancre.getBoundingClientRect();
  // Assez grande pour apporter quelque chose, jamais plus grande que l'écran.
  const largeur = Math.max(
    120,
    Math.min(
      Math.max(LARGEUR, r.width * GAIN),
      window.innerWidth - 2 * MARGE,
      (window.innerHeight - 2 * MARGE) / RATIO
    )
  );
  const hauteur = largeur * RATIO;

  l.textContent = "";
  contenu.style.width = largeur + "px";
  l.appendChild(contenu);

  // À côté de la carte survolée si la place existe, sinon de l'autre côté.
  let x = r.right + MARGE;
  if (x + largeur > window.innerWidth - MARGE) x = r.left - largeur - MARGE;
  x = Math.max(MARGE, Math.min(x, window.innerWidth - largeur - MARGE));

  let y = r.top + r.height / 2 - hauteur / 2;
  y = Math.max(MARGE, Math.min(y, window.innerHeight - hauteur - MARGE));

  l.style.left = Math.round(x) + "px";
  l.style.top = Math.round(y) + "px";
  // Ce qui est agrandi se DÉCLARE : un contrôle extérieur peut alors vérifier que
  // c'est bien la carte survolée, et pas une autre.
  l.dataset.agrandi = identite || "";
  l.classList.add("loupe--visible");
}

export function cacher() {
  const l = document.getElementById("loupe");
  if (!l) return;
  l.classList.remove("loupe--visible");
  delete l.dataset.agrandi;
  // On VIDE, on ne masque pas. Rien de caché ne transite ici — seules des cartes
  // publiques sont survolables — mais laisser dans le document une carte qu'on
  // ne montre plus est exactement le motif que ce chantier s'interdit.
  l.textContent = "";
}

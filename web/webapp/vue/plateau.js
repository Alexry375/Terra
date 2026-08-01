// LE PLATEAU DE JEU — permanent, et sous les yeux des deux joueurs.
//
// Ce module ne connaît AUCUNE règle. Il pose sur la table ce que le moteur dit
// avoir été joué (`players[].played`) et ce qu'il dit de la planète
// (`planet.oceans`). Il ne décide pas ce qui est jouable, ne compte rien, ne
// devine rien : il place des images.
//
// L'EMPILEMENT EN ESCALIER — le point délicat. Sur une vraie table on empile en
// gardant lisibles deux zones de chaque carte recouverte :
//
//   • la colonne de badges, en haut à gauche ;
//   • le rectangle en bas à gauche (production, effet, action).
//
// Ces deux zones tiennent dans la bande GAUCHE de la carte (les badges sur 22 %
// de la largeur, le rectangle sur 30 %). Il suffit donc que chaque carte posée
// par-dessus soit décalée vers la droite d'une fraction plus grande que 30 %
// pour qu'aucune des deux ne soit jamais recouverte, quelle que soit la hauteur.
// Le décalage vertical vers le haut ne sert pas à démasquer : il donne
// l'escalier montant vers la droite d'une vraie table.
//
// TOUT TIENT DANS L'ÉCRAN. Le plateau est posé dans un cadre de taille fixe ;
// quand les piles dépassent, l'ensemble est RÉDUIT (`transform: scale`) jusqu'à
// tenir. Rien n'est masqué, rien ne défile : on voit tout, plus petit.

import { carte } from "./cartes.js";
import {
  EQUIPAGES, nomJoueur, imageOcean, TUILES_OCEAN,
} from "./materiel.js";
import { survolable } from "./loupe.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";

const RATIO = 569 / 409; // les cartes, telles qu'elles ont été découpées
const LARGEUR = 110; // largeur d'une carte AVANT réduction
// 40 % : le contrôle géométrique n'exige que 30 % (la largeur du rectangle du
// bas), mais sur les scans le rectangle imprimé va jusqu'à 37 % de la carte.
// On découvre donc 40 % — la zone entière, pas le minimum qui la coupe.
const DECALAGE_X = 0.4;
const DECALAGE_Y = 0.09; // vers le haut, en fraction de la hauteur
const PAR_PILE = 7; // au-delà, une nouvelle pile de la même couleur, à côté
const ECART = 14; // entre deux piles
const COULEURS = ["verte", "bleue", "rouge"]; // l'ordre des piles sur la table

// ------------------------------------------------------------------ le décor

/** Les deux plateaux, la carte des océans et la case des VP. Une seule fois. */
export function construirePlateaux() {
  for (const j of [0, 1]) {
    const s = document.createElement("section");
    s.className = "plateau";
    s.id = "plateau-" + j;
    s.dataset.plateau = String(j);
    s.style.setProperty("--teinte", EQUIPAGES[j].teinte);
    // Le plateau d'en face est en vis-à-vis, comme de l'autre côté d'une table.
    // Le survol, lui, rend toujours la carte à l'endroit (voir `loupe.js`).
    if (j === 1) s.dataset.visAVis = "oui";
    s.innerHTML =
      `<span class="plateau__mot">${MOT.inPlay} · ${nomJoueur(j)}</span>` +
      `<div class="plateau__cadre"><div class="plateau__piles" id="piles-${j}"></div></div>`;
    document.body.appendChild(s);
  }

  const o = document.createElement("section");
  o.className = "oceans";
  o.id = "oceans";
  o.dataset.oceans = "";
  o.innerHTML =
    `<span class="oceans__mot">${MOT.oceanMap}</span>` +
    `<div class="oceans__grille" id="oceans-grille"></div>`;
  document.body.appendChild(o);

  // On agrandit la carte des océans pour la détailler ; on la referme d'un clic.
  o.addEventListener("click", () => {
    o.classList.toggle("oceans--grande");
  });

  const g = o.querySelector("#oceans-grille");
  TUILES_OCEAN.forEach((_, i) => {
    const d = document.createElement("div");
    d.className = "ocean";
    d.dataset.oceanRevele = "non";
    const im = document.createElement("img");
    im.src = imageOcean(i);
    im.alt = "";
    im.draggable = false;
    d.appendChild(im);
    g.appendChild(d);
  });
}

/**
 * La case qui masque les points de victoire des deux joueurs. Elle se coche et
 * se décoche en cours de partie : elle vit sur la racine du document, donc elle
 * survit à tous les redessins.
 */
export function construireMasqueVP(hote) {
  const l = document.createElement("label");
  l.className = "masque-vp";
  const c = document.createElement("input");
  c.type = "checkbox";
  c.dataset.masquerPv = "";
  c.addEventListener("change", () => {
    if (c.checked) document.documentElement.dataset.pvMasques = "oui";
    else delete document.documentElement.dataset.pvMasques;
  });
  const t = document.createElement("span");
  t.textContent = MOT.hideVp;
  l.appendChild(c);
  l.appendChild(t);
  hote.appendChild(l);
}

// ------------------------------------------------------------------ le rendu

/** Réécrit les deux plateaux et la carte des océans à partir de l'état. */
export function majPlateaux(etat, decision) {
  for (const p of etat.players) {
    const j = p.player;
    const s = ref("#plateau-" + j);
    if (s) s.classList.toggle("plateau--actif", !!decision && decision.joueur === j);
    piles(j, p);
  }
  oceans(etat.planet.oceans);
}

/**
 * Les piles d'un joueur : une par couleur, coupée tous les `PAR_PILE` cartes.
 * Rien n'est réécrit tant que rien n'a bougé — l'écran se redessine à chaque
 * décision, et une partie en compte plusieurs centaines.
 */
function piles(j, p) {
  const z = ref("#piles-" + j);
  if (!z) return;
  const signature = p.played.map((c) => `${c.id}:${c.resources ?? 0}`).join("|");
  if (z.dataset.signature === signature) return;
  z.dataset.signature = signature;
  z.textContent = "";

  // Les cartes gardent leur indice dans `played` : c'est le chemin exact que
  // leur pastille de ressources doit déclarer.
  const parCouleur = new Map();
  p.played.forEach((c, k) => {
    const couleur = c.couleur || "verte";
    if (!parCouleur.has(couleur)) parCouleur.set(couleur, []);
    parCouleur.get(couleur).push({ c, k });
  });

  const ordre = [
    ...COULEURS.filter((c) => parCouleur.has(c)),
    ...[...parCouleur.keys()].filter((c) => !COULEURS.includes(c)),
  ];

  let largeurTotale = 0;
  let hauteurTotale = 0;
  for (const couleur of ordre) {
    const cartes = parCouleur.get(couleur);
    for (let debut = 0; debut < cartes.length; debut += PAR_PILE) {
      const morceau = cartes.slice(debut, debut + PAR_PILE);
      const pile = fabriquerPile(j, couleur, morceau, j === 1);
      z.appendChild(pile.noeud);
      largeurTotale += pile.largeur + ECART;
      hauteurTotale = Math.max(hauteurTotale, pile.hauteur);
    }
  }
  largeurTotale = Math.max(0, largeurTotale - ECART);

  z.style.width = largeurTotale + "px";
  z.style.height = hauteurTotale + "px";
  z.dataset.naturelle = `${largeurTotale}x${hauteurTotale}`;
  reduire(z);
}

/**
 * Une pile : les cartes en escalier, la plus récente au-dessus.
 *
 * `visAVis` = le plateau est retourné d'un demi-tour (le joueur d'en face). Ce
 * demi-tour ne change RIEN à ce qui reste visible de chaque carte : la carte
 * posée par-dessus recouvre toujours la même partie de celle du dessous, la
 * bande de badges et le rectangle du bas restent découverts. Il change en
 * revanche l'ordre dans lequel les cartes se succèdent À L'ÉCRAN — de la droite
 * vers la gauche. On les inscrit donc dans le document dans cet ordre-là, pour
 * que le document raconte la table telle qu'on la voit.
 */
function fabriquerPile(j, couleur, morceau, visAVis = false) {
  const dx = Math.round(LARGEUR * DECALAGE_X);
  // `ceil` : le scan n'a pas EXACTEMENT le rapport nominal, et un demi-pixel
  // d'écart devient un pixel de débordement une fois le plateau agrandi.
  const hauteurCarte = Math.ceil(LARGEUR * RATIO) + 1;
  const dy = Math.round(hauteurCarte * DECALAGE_Y);
  const n = morceau.length;

  const pile = document.createElement("div");
  pile.className = "pile";
  pile.dataset.pile = couleur;
  const largeur = LARGEUR + (n - 1) * dx;
  const hauteur = hauteurCarte + (n - 1) * dy;
  pile.style.width = largeur + "px";
  pile.style.height = hauteur + "px";

  const noeuds = morceau.map(({ c, k }, i) => {
    const f = carte(c, {
      classe: "carte--jeu",
      chemin: `players.${j}.played.${k}.resources`,
    });
    if (c.id === undefined || c.id === null) {
      // L'identifiant vient du moteur (`players.N.played`). S'il manquait, on
      // le dirait plutôt que de laisser passer un numéro inventé.
      console.warn("plateau.js : carte en jeu sans identifiant du moteur —", c.name);
    }
    f.dataset.carteEnJeu = String(c.id ?? c.name ?? k);
    f.dataset.couleur = c.couleur || couleur;
    f.style.width = LARGEUR + "px";
    f.style.left = i * dx + "px";
    f.style.top = (n - 1 - i) * dy + "px";
    f.style.zIndex = String(i + 1);
    survolable(f, c);
    return f;
  });
  if (visAVis) noeuds.reverse();
  for (const f of noeuds) pile.appendChild(f);

  return { noeud: pile, largeur, hauteur };
}

/**
 * METTRE À L'ÉCHELLE, JAMAIS MASQUER. Le cadre a la taille que l'écran lui
 * laisse ; les piles s'y ajustent — plus petites quand elles débordent, plus
 * grandes quand la place est là. Aucune carte n'est retirée, aucun défilement
 * n'apparaît.
 *
 * Deux garde-fous, tous deux appris à l'usage :
 *   • on ne SORT PAS sans avoir posé d'échelle. Sur un écran très bas le cadre
 *     peut se retrouver sans hauteur ; laisser les cartes à leur taille
 *     naturelle les ferait alors couper par le cadre — c'est-à-dire masquer,
 *     exactement ce que le chantier interdit. On réduit jusqu'au bout ;
 *   • on autorise l'AGRANDISSEMENT (jusqu'à deux fois) : sur un grand écran,
 *     laisser la moitié du plateau vide avec des cartes minuscules serait
 *     absurde — le point de tout ceci est qu'on les lise.
 */
const ECHELLE_MAX = 2;
const ECHELLE_MIN = 0.02;

function reduire(z) {
  const cadre = z.parentElement;
  if (!cadre) return;
  const [l, h] = (z.dataset.naturelle || "0x0").split("x").map(Number);
  if (!l || !h) return; // aucune carte posée : rien à mettre à l'échelle
  // Deux pixels de marge de chaque côté : le rendu d'une image n'est pas au
  // pixel entier, et l'agrandissement multiplie l'écart.
  const s = Math.min(ECHELLE_MAX, (cadre.clientWidth - 4) / l, (cadre.clientHeight - 4) / h);
  // On pose l'ÉCHELLE, pas la transformation : la feuille de style compose
  // elle-même le demi-tour du plateau d'en face avec cette réduction.
  z.style.setProperty("--echelle", Math.max(ECHELLE_MIN, s).toFixed(4));
}

/** Les deux plateaux se re-mesurent quand la fenêtre change de taille. */
export function replacerPlateaux() {
  for (const j of [0, 1]) {
    const z = ref("#piles-" + j);
    if (z) reduire(z);
  }
}

/**
 * La carte des océans. Le moteur ne rend qu'un COMPTE (`planet.oceans`) : les
 * `n` premières tuiles de la planche sont donc celles qui sont retournées. On
 * ne choisit pas lesquelles, on n'en invente aucune.
 */
function oceans(combien) {
  const g = ref("#oceans-grille");
  if (!g) return;
  if (g.dataset.combien === String(combien)) return;
  g.dataset.combien = String(combien);
  [...g.children].forEach((d, i) => {
    d.dataset.oceanRevele = i < combien ? "oui" : "non";
  });
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierPlateaux() {
  for (const j of [0, 1]) {
    const z = ref("#piles-" + j);
    if (z) {
      delete z.dataset.signature;
      z.textContent = "";
    }
  }
  const g = ref("#oceans-grille");
  if (g) delete g.dataset.combien;
}

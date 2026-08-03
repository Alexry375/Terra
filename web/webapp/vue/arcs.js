// LES DEUX ARCS — la température et l'oxygène, comme sur le plateau imprimé.
//
// Sur le plateau (photo `docs/regles/photos/photo-08.jpg`, et le livret), ces
// deux compteurs ne sont pas des barres : ce sont deux arcs de cercle posés de
// part et d'autre du disque de Mars, gradués case par case, chacun divisé en
// QUATRE zones de couleur (livret l. 77 : violet, rouge, jaune, blanc). Un cube
// transparent y marque la valeur courante (l. 201).
//
// Les graduations sont celles du livret, relues ligne à ligne :
//
//   • température (l. 84, l. 499) : de -30 °C à +8 °C par pas de 2 °C, vingt
//     cases. Le cube part sur -30 °C ;
//   • oxygène (l. 83, l. 501) : de 0 % à 14 % par pas de 1 %. Le cube part sur
//     0 %, et les quatorze cases GAGNABLES sont 1 % … 14 %.
//
// CE MODULE NE CALCULE AUCUNE RÈGLE. Le moteur compte en PAS (`planet.temperature`
// va de 0 à 19, `planet.oxygen` de 0 à 14) ; l'arc n'applique que la conversion
// d'échelle du plateau imprimé — `-30 + 2 × pas` pour les degrés, l'identité
// pour les pourcents. Le bandeau, lui, continue d'afficher la valeur BRUTE du
// moteur sous son `data-valeur`, pour que l'écran reste vérifiable.

import { MOT } from "./mots.js";

// --------------------------------------------------------------- la géométrie

// L'arc vit dans un dessin de 100 × 200 unités qui s'étire avec le panneau. Le
// centre du cercle est posé sur le bord du dessin qui regarde le MILIEU DE LA
// TABLE : l'arc bombe donc vers le bord de l'écran et creuse sa concavité vers
// le centre — exactement la disposition du plateau, où les deux arcs encadrent
// Mars. Pour le panneau de gauche, ce centre est donc en x = 98 (`sens = -1`) ;
// pour celui de droite, en x = 2 (`sens = +1`).
const VUE = { l: 100, h: 200 };
const CENTRE_Y = 100;
const RAYON = 84;
const DEDANS = 72; // rayon intérieur des graduations
const DEHORS = 92; // rayon extérieur des graduations
const ANGLE_BAS = 72; // degrés, sous l'horizontale
const ANGLE_HAUT = -72;

/**
 * Le point d'une fraction de course (0 = première case, 1 = dernière), à un
 * rayon donné. `sens` vaut +1 pour l'arc de gauche (centre au bord gauche) et
 * -1 pour celui de droite, qui en est le miroir.
 */
function point(fraction, rayon, sens) {
  const deg = ANGLE_BAS + fraction * (ANGLE_HAUT - ANGLE_BAS);
  const rad = (deg * Math.PI) / 180;
  const cx = sens > 0 ? 2 : VUE.l - 2;
  return [cx + sens * rayon * Math.cos(rad), CENTRE_Y + rayon * Math.sin(rad)];
}

// ----------------------------------------------------------------- les couleurs

// Les quatre zones du livret, chacune parcourue d'un bout à l'autre : violet →
// rose, rouge → orange, jaune, gris clair. C'est la lecture de la photo du
// plateau, du plus froid (en bas) au plus chaud (en haut).
const ZONES = [
  { de: "#5f1a72", a: "#d63fa8" }, // violet → rose
  { de: "#d8332a", a: "#f07a24" }, // rouge → orange
  { de: "#e2c81f", a: "#f4ea63" }, // jaune
  { de: "#cfcfcf", a: "#f6f6f6" }, // gris clair
];

const hex = (c) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
const melange = (a, b, t) =>
  "#" + hex(a).map((v, i) => Math.round(v + (hex(b)[i] - v) * t)
    .toString(16).padStart(2, "0")).join("");

/** La couleur d'une case, d'après sa zone et sa place dans cette zone. */
function couleur(zone, rang, taille) {
  const z = ZONES[zone];
  return melange(z.de, z.a, taille > 1 ? rang / (taille - 1) : 0);
}

// ------------------------------------------------------------- les deux pistes

/**
 * Une piste, c'est la liste de ses cases : la valeur imprimée sur la case, et
 * la zone de couleur à laquelle le plateau la rattache. Les frontières de zone
 * sont relevées sur le plateau imprimé, case par case.
 */
function piste(valeurs, frontieres) {
  return valeurs.map((v, i) => {
    let zone = 0;
    while (zone < frontieres.length && i >= frontieres[zone]) zone++;
    const debut = zone === 0 ? 0 : frontieres[zone - 1];
    const fin = zone < frontieres.length ? frontieres[zone] : valeurs.length;
    return { valeur: v, zone, rang: i - debut, taille: fin - debut };
  });
}

// Température : -30 … +8 par pas de 2. Zones lues sur le plateau : violet de
// -30 à -20 (six cases), rouge de -18 à -10 (cinq), jaune de -8 à 0 (cinq),
// gris de +2 à +8 (quatre).
const CRANS_TEMP = piste(
  Array.from({ length: 20 }, (_, i) => -30 + 2 * i),
  [6, 11, 16],
);

// Oxygène : 1 % … 14 %. Zones lues sur le plateau, où la case 0 % est violette
// elle aussi mais n'est pas une case à gagner : violet 1-2, rouge 3-6,
// jaune 7-11, gris 12-14.
const CRANS_O2 = piste(
  Array.from({ length: 14 }, (_, i) => i + 1),
  [2, 6, 11],
);

/**
 * Les deux arcs, tels que la page les connaît. `pas` est le compteur du moteur
 * (`planet.temperature`, `planet.oxygen`) ; `course` le convertit en fraction
 * d'arc, `lecture` en valeur du plateau imprimé.
 *
 * Les deux fractions partent de la même idée : le cube de départ est à zéro.
 * Pour la température, zéro EST une case du plateau (-30 °C) ; pour l'oxygène,
 * zéro est la case 0 %, avant la première case gagnable.
 */
const ARCS = {
  temperature: {
    // Panneau de GAUCHE : le cercle est centré sur le bord droit du dessin,
    // l'arc bombe donc vers le bord de l'écran et sa concavité regarde la table.
    sens: -1,
    mot: MOT.arcTemp,
    crans: CRANS_TEMP,
    max: 19,
    // La case i porte -30 + 2i, et le pas i du moteur atteint cette case-là.
    course: (pas) => pas / 19,
    fractionCran: (i) => i / 19,
    lecture: (pas) => -30 + 2 * pas,
    // Le SIGNE, même quand il est positif : « +2 » se lit comme une
    // température, « 2 » comme un compteur. Le bandeau écrit la même chose
    // (`vue/monde.js`, `degre`) — les deux ne doivent jamais différer d'un
    // caractère, sinon on croit lire deux grandeurs.
    ecrire: (v) => (v > 0 ? "+" + v : String(v)),
  },
  oxygen: {
    // Panneau de DROITE : le miroir du précédent.
    sens: 1,
    mot: MOT.arcOxygen,
    crans: CRANS_O2,
    max: 14,
    course: (pas) => pas / 14,
    // La case i porte i+1 %, et le pas i+1 du moteur l'atteint.
    fractionCran: (i) => (i + 1) / 14,
    lecture: (pas) => pas,
  },
};

// -------------------------------------------------------------------- le décor

function svg(nom, attrs) {
  const e = document.createElementNS("http://www.w3.org/2000/svg", nom);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

/** Bâtit les deux arcs et les pose sur la page. Appelé une fois. */
export function construireArcs() {
  for (const [quoi, a] of Object.entries(ARCS)) {
    const s = document.createElement("section");
    s.className = "arc arc--" + quoi;
    s.id = "arc-" + quoi;
    s.dataset.arc = quoi;

    const dessin = svg("svg", {
      class: "arc__dessin",
      viewBox: `0 0 ${VUE.l} ${VUE.h}`,
      preserveAspectRatio: "xMidYMid meet",
      // Le dessin est redit en toutes lettres à côté (`arc__n`, `arc__mot`) :
      // il n'apporte rien de plus à qui ne le voit pas.
      "aria-hidden": "true",
      focusable: "false",
    });

    // L'épaisseur d'une case : la corde entre deux graduations, moins le jour
    // qui les sépare sur le carton imprimé.
    const pas = Math.abs(a.fractionCran(1) - a.fractionCran(0));
    const epaisseur = RAYON * pas * ((ANGLE_BAS - ANGLE_HAUT) * Math.PI / 180) * 0.78;

    // La gorge : le carton sombre sur lequel les cases sont imprimées. Elle va
    // d'un bout à l'autre de la piste, cases gagnées ou non.
    const [gx1, gy1] = point(a.fractionCran(0), RAYON, a.sens);
    const [gx2, gy2] = point(a.fractionCran(a.crans.length - 1), RAYON, a.sens);
    dessin.appendChild(svg("path", {
      class: "arc__gorge",
      d: `M ${gx1.toFixed(2)} ${gy1.toFixed(2)} A ${RAYON} ${RAYON} 0 0 `
        + `${a.sens > 0 ? 0 : 1} ${gx2.toFixed(2)} ${gy2.toFixed(2)}`,
      "stroke-width": (DEHORS - DEDANS + 3).toFixed(2),
    }));

    a.crans.forEach((c, i) => {
      const f = a.fractionCran(i);
      const [x1, y1] = point(f, DEDANS, a.sens);
      const [x2, y2] = point(f, DEHORS, a.sens);
      const t = svg("line", {
        class: "cran",
        x1: x1.toFixed(2), y1: y1.toFixed(2), x2: x2.toFixed(2), y2: y2.toFixed(2),
        "stroke-width": epaisseur.toFixed(2),
      });
      // La couleur du plateau imprimé voyage par une variable et non par
      // l'attribut `stroke` : la feuille de style doit pouvoir éteindre une
      // case non gagnée, et une règle CSS l'emporte toujours sur l'attribut.
      t.style.setProperty("--teinte-cran", couleur(c.zone, c.rang, c.taille));
      t.dataset.cran = quoi;
      t.dataset.cranValeur = String(c.valeur);
      dessin.appendChild(t);
    });

    // Le cube du plateau : un repère posé PAR-DESSUS la case courante, qui
    // glisse d'une case à l'autre quand le moteur fait monter le paramètre.
    const repere = svg("g", { class: "arc__repere" });
    repere.dataset.repere = quoi;
    repere.appendChild(svg("circle", { class: "arc__repere-halo", cx: 0, cy: 0, r: 8.5 }));
    repere.appendChild(svg("circle", { class: "arc__repere-oeil", cx: 0, cy: 0, r: 4.6 }));
    dessin.appendChild(repere);

    s.appendChild(dessin);

    // La valeur courante, en chiffres, dans la concavité de l'arc. Elle est
    // écrite en HTML et non dans le dessin : c'est du texte, il doit se
    // mesurer, se sélectionner et se lire comme le reste de l'écran.
    const n7 = document.createElement("b");
    n7.className = "arc__n";
    n7.dataset.arcLecture = quoi;
    n7.textContent = "—";
    s.appendChild(n7);

    const mot = document.createElement("span");
    mot.className = "arc__mot";
    mot.textContent = a.mot;
    s.appendChild(mot);

    document.body.appendChild(s);
  }
}

// -------------------------------------------------------------------- le rendu

const dernier = new Map(); // arc -> dernier pas rendu

/** Réécrit les deux arcs à partir de l'état rendu par le moteur. */
export function majArcs(etat) {
  poserArc("temperature", etat.planet.temperature);
  poserArc("oxygen", etat.planet.oxygen);
}

function poserArc(quoi, pas) {
  if (dernier.get(quoi) === pas) return;
  dernier.set(quoi, pas);
  const a = ARCS[quoi];
  const s = document.getElementById("arc-" + quoi);
  if (!s) return;

  // Une case est ACQUISE quand le moteur a dépassé sa valeur : comme le cube du
  // plateau, il n'a jamais reculé d'une case de toute l'histoire du jeu.
  const crans = s.querySelectorAll("[data-cran]");
  a.crans.forEach((c, i) => {
    const atteint = a.lecture(pas) >= c.valeur;
    crans[i].classList.toggle("cran--acquis", atteint);
  });

  const [x, y] = point(a.course(pas), RAYON, a.sens);
  const repere = s.querySelector("[data-repere]");
  if (repere) repere.setAttribute("transform", `translate(${x.toFixed(2)} ${y.toFixed(2)})`);

  const lecture = s.querySelector("[data-arc-lecture]");
  if (lecture) {
    const v = a.lecture(pas);
    lecture.textContent = a.ecrire ? a.ecrire(v) : String(v);
  }
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierArcs() {
  dernier.clear();
}

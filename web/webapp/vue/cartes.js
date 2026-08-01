// Une carte à l'écran EST son image imprimée.
//
// Le moteur nomme les cartes de deux façons selon l'endroit : `name` dans l'état
// (`players[].hand`, `players[].played`), `nom` dans les options d'une décision.
// Une option de construction emballe même la carte dans `option.carte`. Ce module
// absorbe ces trois formes et rend toujours le même objet d'affichage.

import { imageCarte, dosDeCarte } from "./materiel.js";
import { MOT, nomLisible } from "./mots.js";

/** Ramène les trois formes du moteur à une seule. Ne calcule rien. */
export function normaliser(c) {
  if (!c) return null;
  const carte = c.carte ? c.carte : c;
  const nom = carte.nom ?? carte.name ?? null;
  if (!nom) return null;
  return {
    nom,
    couleur: carte.couleur ?? null,
    prix: carte.prix ?? carte.price ?? null,
    pv: carte.pv ?? null,
    badges: carte.badges ?? null,
    ressources: carte.resources ?? null,
    id: carte.id ?? null,
  };
}

/**
 * Fabrique une carte affichée.
 *
 * @param {object} c      carte, dans n'importe laquelle des formes du moteur
 * @param {object} o
 * @param {string} o.classe  classes supplémentaires
 * @param {boolean} o.muette pas de plaque de nom sous l'image
 * @param {string}  o.chemin  chemin dans `etat` des ressources posées, s'il est connu
 */
export function carte(c, { classe = "", muette = true, chemin = null } = {}) {
  const n = normaliser(c);
  const f = document.createElement("figure");
  f.className = "carte " + classe;
  if (n && n.couleur) f.dataset.couleur = n.couleur;

  if (!n) {
    f.classList.add("carte--dos");
    const im = document.createElement("img");
    im.src = dosDeCarte();
    im.alt = MOT.faceDown;
    im.draggable = false;
    f.appendChild(im);
    return f;
  }

  const src = imageCarte(n.nom);
  const im = document.createElement("img");
  im.draggable = false;
  im.alt = n.nom;
  if (src) {
    im.src = src;
  } else {
    // Deux cartes du jeu n'ont pas été découpées des planches officielles. On ne
    // les invente pas : on montre le dos réel et on nomme la carte en clair.
    f.classList.add("carte--sans-image");
    im.src = dosDeCarte();
  }
  f.appendChild(im);

  if (!src || !muette) {
    const p = document.createElement("figcaption");
    p.className = "carte__plaque";
    p.textContent = nomLisible(n.nom);
    f.appendChild(p);
  }

  // Les ressources posées sur une carte (microbes, animaux…) : le moteur les
  // donne dans `played[].resources`, on ne les compte pas nous-même. Quand on
  // sait où la carte se trouve dans l'état, la pastille déclare son chemin.
  if (n.ressources) {
    const j = document.createElement("span");
    j.className = "carte__ressources";
    j.textContent = String(n.ressources);
    if (chemin) j.dataset.valeur = chemin;
    f.appendChild(j);
  }

  return f;
}

/** Le nom lisible d'une option, quelle que soit sa forme. */
export function libelle(o, defaut = "") {
  return o?.libelle ?? o?.nom ?? o?.name ?? defaut;
}

// Une carte à l'écran EST son image imprimée.
//
// Le moteur nomme les cartes de deux façons selon l'endroit : `name` dans l'état
// (`players[].hand`, `players[].played`), `nom` dans les options d'une décision.
// Une option de construction emballe même la carte dans `option.carte`. Ce module
// absorbe ces trois formes et rend toujours le même objet d'affichage.

import { imageCarte, dosProjet, dosCorporation } from "./materiel.js";
import { MOT } from "./mots.js";

/** Ramène les trois formes du moteur à une seule. Ne calcule rien. */
export function normaliser(c) {
  if (!c) return null;
  const carte = c.carte ? c.carte : c;
  const nom = carte.nom ?? carte.name ?? null;
  if (!nom) return null;
  return {
    nom,
    // LA SORTE. Le moteur range ses cartes dans deux tables séparées et le
    // numéro publié est un rang DANS l'une ou DANS l'autre : le numéro 7
    // désigne aussi bien la carte projet « Arctic Algae » que la corporation
    // « Inventrix ». Le pont dit désormais laquelle (`wasm/src/lib.rs`,
    // `carte_json` / `corpo_json`). L'état, lui, ne rend que des cartes projet
    // (`observe.rs`, `players[].hand` et `players[].played`) : son silence vaut
    // donc « projet », et c'est le seul défaut admis ici.
    sorte: carte.sorte ?? "projet",
    couleur: carte.couleur ?? null,
    prix: carte.prix ?? carte.price ?? null,
    pv: carte.pv ?? null,
    badges: carte.badges ?? null,
    ressources: carte.resources ?? null,
    id: carte.id ?? null,
  };
}

/**
 * LE SEUL IDENTIFIANT QUI DÉSIGNE UNE CARTE SANS AMBIGUÏTÉ : sa sorte et son
 * numéro, ensemble. Un numéro seul n'en désigne aucune — voir `normaliser`.
 *
 * Mesuré le 02-08 sur 70 parties : 3 fois, comparer les numéros seuls faisait
 * disparaître une corporation de la main du joueur comme doublon d'une carte
 * projet, et reportait sur cette carte projet la réponse « joue cette
 * corporation ». Rend `null` pour une carte sans numéro.
 */
export function cle(c) {
  const n = normaliser(c);
  if (n === null || n.id === null || n.id === undefined) return null;
  return n.sorte + "#" + n.id;
}

/**
 * Fabrique une carte affichée.
 *
 * @param {object} c      carte, dans n'importe laquelle des formes du moteur
 * @param {object} o
 * @param {string} o.classe  classes supplémentaires
 * @param {boolean} o.muette pas de plaque de nom sous l'image
 * @param {string}  o.chemin  chemin dans `etat` des ressources posées, s'il est connu
 * @param {string}  o.dos     quel dos montrer quand la face manque :
 *                            « projet » (défaut) ou « corporation »
 */
export function carte(c, { classe = "", muette = true, chemin = null, dos = "projet" } = {}) {
  const n = normaliser(c);
  const f = document.createElement("figure");
  f.className = "carte " + classe;
  if (n && n.couleur) f.dataset.couleur = n.couleur;
  // Le dos d'une carte projet n'est pas celui d'une corporation : celui qu'on
  // montre dit de quelle SORTE est la carte cachée, et c'est une information
  // publique. Se tromper de dos, c'est annoncer la mauvaise sorte.
  const leDos = dos === "corporation" ? dosCorporation() : dosProjet();

  if (!n) {
    f.classList.add("carte--dos");
    const im = document.createElement("img");
    im.src = leDos;
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
    im.src = leDos;
  }
  f.appendChild(im);

  if (!src || !muette) {
    const p = document.createElement("figcaption");
    p.className = "carte__plaque";
    p.textContent = n.nom;
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

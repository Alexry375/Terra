// Une carte à l'écran EST son image imprimée.
//
// Le moteur nomme les cartes de deux façons selon l'endroit : `name` dans l'état
// (`players[].hand`, `players[].played`), `nom` dans les options d'une décision.
// Une option de construction emballe même la carte dans `option.carte`. Ce module
// absorbe ces trois formes et rend toujours le même objet d'affichage.

import { imageCarte, dosProjet, dosCorporation, imageBadge, nomBadge } from "./materiel.js";
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
    // `ressources` se relit sous SES DEUX noms — celui du moteur et celui qu'on
    // vient de lui donner. La loupe repasse dans `normaliser` une carte DÉJÀ
    // normalisée (`loupe.js`) : sans cette seconde lecture, le compte se perdait
    // au second passage, et la carte AGRANDIE — celle qu'on ouvre justement pour
    // lire — était la seule à ne rien dire de ses ressources. C'est ce qui
    // empêchait MOT-15 de s'afficher là où le contrat le demande.
    ressources: carte.resources ?? carte.ressources ?? null,
    // (MOT-14) LE BADGE CHOISI POUR LE BADGE JOKER de cette carte, tel que le
    // moteur le publie (`observe.rs`, `played[].joker` — la chaîne de
    // `Tag::as_str`, celle-là même qui nomme les familles de `players[].tags`).
    // Absent des cartes sans badge joker. La page ne le devine pas : mémoriser
    // sa propre réponse ne dirait rien du badge de l'adversaire, ni d'une partie
    // reprise en cours de route.
    //
    // (MOT-15) CE QUE LES RESSOURCES POSÉES RAPPORTENT DÉJÀ, en points de
    // victoire (`played[].pv_ressources`). Le compte de ressources ci-dessus ne
    // suffit pas à le déduire : il faudrait le barème de la carte, et le
    // recopier ici ferait un SECOND endroit qui compte les points. Le nombre
    // vient du service unique du moteur (`flow::card_points`, celui que
    // `score_breakdown` appelle), on ne fait que le lire.
    //
    // Les deux clefs gardent le nom du moteur, et non un nom d'affichage : la
    // loupe repasse une carte DÉJÀ normalisée dans `normaliser`, et un champ
    // renommé ne survivrait pas au second passage.
    joker: carte.joker ?? null,
    pv_ressources: carte.pv_ressources ?? null,
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
 * @param {boolean} o.points  dire ce que les ressources posées rapportent déjà
 *                            (MOT-15) : réservé à la carte AGRANDIE, où il y a
 *                            la place de l'écrire en toutes lettres
 */
export function carte(c, {
  classe = "", muette = true, chemin = null, dos = "projet", points = false,
} = {}) {
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

  let plaque = null;
  if (!src || !muette) {
    plaque = document.createElement("figcaption");
    plaque.className = "carte__plaque";
    plaque.textContent = n.nom;
    f.appendChild(plaque);
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

  // (MOT-14) LE BADGE CHOISI POUR LE BADGE JOKER, sur la carte posée — pour les
  // DEUX joueurs, puisque `plateau.js` dessine les deux tables par ce même
  // appel. Le rond gris « ? » de l'extension Découverte ne disait pas ce que le
  // joueur en avait fait ; le jeton posé dessus le dit maintenant. On montre le
  // jeton imprimé de la famille, celui-là même que la barre du joueur emploie
  // pour compter les badges — jamais un dessin à part.
  if (n.joker) {
    const src = imageBadge(n.joker);
    const b = document.createElement("span");
    b.className = "carte__joker";
    b.title = MOT.jokerTag + " : " + nomBadge(n.joker);
    if (src) {
      const ib = document.createElement("img");
      ib.src = src;
      ib.alt = nomBadge(n.joker);
      ib.draggable = false;
      b.appendChild(ib);
    } else {
      // Famille inconnue de la planche de jetons : on écrit le mot plutôt que
      // de laisser un rond vide. Le joueur doit voir le choix, pas une case.
      b.textContent = nomBadge(n.joker);
      b.classList.add("carte__joker--mot");
    }
    f.appendChild(b);
  }

  // (MOT-15) CE QUE LES RESSOURCES POSÉES RAPPORTENT DÉJÀ. Sur la carte
  // agrandie seulement : c'est là qu'on lit une carte, et c'est là qu'il y a la
  // place. Le nombre vient du moteur entier — la page n'additionne rien et ne
  // connaît aucun barème.
  //
  // On l'écrit dès qu'il y a des ressources, même à zéro : « 3 microbes,
  // 0 point » est une information, et son absence se lirait comme un oubli.
  //
  // La ligne se range DANS la plaque de nom, sous le nom — et non en bande
  // séparée : la loupe se place d'après la hauteur de l'IMAGE seule, une bande
  // de plus déborderait par le bas de l'écran sur les cartes du bord.
  if (points && n.pv_ressources !== null && n.ressources) {
    const v = document.createElement("span");
    v.className = "carte__pv";
    const b = document.createElement("b");
    b.textContent = String(n.pv_ressources);
    const i = document.createElement("i");
    i.textContent = MOT.vpFromResources;
    v.append(b, " ", i);
    (plaque || f).appendChild(v);
  }

  return f;
}

/** Le nom lisible d'une option, quelle que soit sa forme. */
export function libelle(o, defaut = "") {
  return o?.libelle ?? o?.nom ?? o?.name ?? defaut;
}

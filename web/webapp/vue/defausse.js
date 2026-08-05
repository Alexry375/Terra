// LES DEUX PAQUETS — la pioche d'où les cartes viennent, la défausse où elles
// vont, et ce qu'on peut y lire.
//
// ------------------------------------------------------------------ ANI-6
//
// Une carte piochée apparaissait dans la main, une carte défaussée disparaissait
// de l'écran : rien ne disait D'OÙ ni VERS OÙ. Un vol a besoin de deux points, et
// il n'en existait aucun — le bandeau ne portait que deux nombres, « Deck 246
// + 18 ». Ce module pose ces deux points, dans la colonne de DROITE : le sens
// des vols en découle, et c'est celui que le contrat fixe — la pioche arrive par
// la droite, la défausse s'en va en sens inverse (`vue/anim.js`).
//
// ------------------------------------------------------------------ CNF-2
//
// Trois exigences, dictées mot pour mot par le propriétaire du projet le 04-08,
// et tenues dans cet ordre :
//
//   1. la dernière carte défaussée est TOUJOURS VISIBLE, face découverte, posée
//      sur la pile de défausse ;
//   2. CLIQUER DESSUS ouvre une fenêtre montrant toutes les cartes défaussées,
//      avec un défilement ;
//   3. l'ordre est LE PLUS RÉCENT D'ABORD — la dernière défaussée en haut à
//      gauche, l'avant-dernière à sa droite, cinq par ligne.
//
// LA DÉFAUSSE EST COMMUNE aux deux joueurs, et c'est VOULU : le but est de voir
// ce que l'adversaire a jeté. Ce n'est pas une règle officielle du jeu, c'est une
// OPTION DE PARTIE — le panneau d'options l'allume et l'éteint comme les autres.
// Éteinte, la pile ne montre rien et la fenêtre ne s'ouvre pas.
//
// L'ORDRE N'EST PAS INVENTÉ ICI, ET NE PEUT PAS L'ÊTRE. Le moteur publie la pile
// carte par carte, la plus récemment défaussée en tête (`engine/src/observe.rs`,
// clef `defausse`) ; on la lit et on la pose telle quelle. Tenir une liste à
// l'écran aurait paru marcher — jusqu'au premier remélange, où la pile du moteur
// se vide dans la pioche et où la nôtre aurait continué de montrer des cartes
// qui n'y sont plus.
//
// AUCUN NOMBRE ICI. Les deux épaisseurs sont déjà écrites dans le bandeau
// (`vue/monde.js`, `decks.deck` et `decks.discard`), chacune déclarant son chemin
// dans l'état. Les redire ferait deux éléments porteurs du même `data-valeur`,
// dont un seul serait tenu à jour : un écran qui se contredit. Le dock montre les
// OBJETS, le bandeau compte.

import { carte } from "./cartes.js";
import { dosProjet } from "./materiel.js";
import { MOT } from "./mots.js";

// ------------------------------------------------------------------ l'option

// L'OPTION, ET SON UNIQUE POINT D'ÉCRITURE. Elle vit ici, et `vue/options.js` la
// lit et la pose par ces deux fonctions — jamais en écrivant l'attribut
// lui-même. C'est la leçon du réglage des animations, qui avait fini avec deux
// mémoires et un interrupteur qui mentait (voir l'en-tête de `vue/anim.js`).
//
// ALLUMÉE PAR DÉFAUT : c'est le confort demandé, et une option qu'il faudrait
// découvrir pour en profiter ne rendrait service à personne.
let visible = true;

/** L'option est-elle allumée ? */
export function defausseVisible() {
  return visible;
}

/**
 * Allume ou éteint « voir la défausse ». Éteinte : la pile ne montre rien, et la
 * fenêtre ne s'ouvre pas — celle qui serait ouverte se referme à l'instant.
 */
export function reglerDefausse(oui) {
  visible = !!oui;
  // `data-voir-defausse`, et surtout PAS `data-defausse` : cette marque-là
  // désigne la PILE, un seul élément de la page, et c'est par elle qu'on la
  // trouve. Posée sur la racine du document, elle aurait fait de `<html>` la
  // première pile venue — mesuré : les vols partaient alors vers le centre de la
  // fenêtre, et `[data-defausse] .carte` désignait n'importe quelle carte de
  // l'écran. Une marque de RÉGLAGE et une marque d'OBJET ne portent jamais le
  // même nom.
  document.documentElement.dataset.voirDefausse = visible ? "oui" : "non";
  if (!visible) fermerFenetre();
  redessiner();
}

// ------------------------------------------------------------------ le dock

// Ce que le moteur publie, retenu pour la fenêtre : la liste des cartes
// défaussées, la plus récente en tête. Jamais complétée ni corrigée ici.
let pile = [];
// La signature du dessus déjà dessiné : on ne refait pas une carte qui n'a pas
// changé. L'écran se réécrit à chaque décision, et une partie en compte
// plusieurs centaines.
let dessus = "";

/** Le dock des deux paquets. Appelé une fois, par `vue/plateau.js`. */
export function construireDefausse() {
  if (document.getElementById("paquets")) return;
  document.documentElement.dataset.voirDefausse = visible ? "oui" : "non";

  const z = document.createElement("section");
  z.id = "paquets";
  z.className = "paquets";
  z.dataset.paquets = "";

  const mot = document.createElement("span");
  mot.className = "paquets__mot";
  mot.textContent = MOT.piles;
  z.appendChild(mot);

  const rang = document.createElement("div");
  rang.className = "paquets__rang";

  // LA PIOCHE — un dos de carte projet, et jamais une face : ce qu'elle contient
  // n'est pas une information du jeu. C'est de là que partent les vols de pioche.
  const pioche = document.createElement("div");
  pioche.className = "paquet paquet--pioche";
  pioche.id = "pioche";
  pioche.dataset.pioche = "";
  pioche.title = MOT.deck;
  const dos = document.createElement("img");
  dos.src = dosProjet();
  dos.alt = MOT.deck;
  dos.draggable = false;
  pioche.appendChild(dos);
  rang.appendChild(pioche);

  // LA DÉFAUSSE — l'emplacement où les cartes jetées viennent se poser, et où la
  // dernière reste face découverte.
  const defausse = document.createElement("div");
  defausse.className = "paquet paquet--defausse";
  defausse.id = "defausse";
  defausse.dataset.defausse = "";
  defausse.title = MOT.discardOpen;
  defausse.addEventListener("click", ouvrirFenetre);
  rang.appendChild(defausse);

  z.appendChild(rang);
  document.body.appendChild(z);
}

/** Où la défausse se trouve à l'écran — la cible des vols. */
export function ancreDefausse() {
  return document.getElementById("defausse");
}

/** Où la pioche se trouve à l'écran — l'origine des vols. */
export function ancrePioche() {
  return document.getElementById("pioche");
}

/**
 * Réécrit la pile à partir de l'état rendu par le moteur.
 * @param {object} etat  l'état (`etat.defausse`, la plus récente en tête)
 */
export function majDefausse(etat) {
  pile = Array.isArray(etat && etat.defausse) ? etat.defausse : [];
  redessiner();
  // La fenêtre ouverte suit la partie : une carte défaussée pendant qu'on la lit
  // s'y ajoute, en tête, comme partout ailleurs.
  if (document.getElementById("fenetre-defausse")) remplirFenetre();
}

/** Le dessus de la pile : la dernière carte défaussée, face découverte. */
function redessiner() {
  const z = document.getElementById("defausse");
  if (!z) return;
  const tete = visible && pile.length ? pile[0] : null;
  const cle = tete ? `${tete.id}` : "";
  if (z.dataset.cle === cle) return;
  z.dataset.cle = cle;
  z.textContent = "";
  if (!tete) return;
  // La carte est fabriquée par la fabrique commune (`vue/cartes.js`) : l'écran ne
  // connaît qu'une seule façon de dessiner une carte, et son image porte le nom
  // que le moteur donne.
  const f = carte(tete, { classe: "carte--defausse" });
  f.dataset.defausseDessus = "";
  z.appendChild(f);
}

// ------------------------------------------------------------------ la fenêtre

/**
 * TOUTES LES CARTES DÉFAUSSÉES, la plus récente en haut à gauche, cinq par ligne.
 *
 * CE QUI EST INTERDIT ICI, et qui a été demandé explicitement : PAS DE LOUPE.
 * Cliquer une carte de cette fenêtre ne doit rien agrandir — les cartes y sont
 * déjà à la taille d'une carte Phase au moment où l'on en choisit une, c'est-à-
 * dire lisibles sans rien de plus. Aucune des cartes posées ici ne passe donc par
 * `vue/loupe.js`.
 */
function ouvrirFenetre() {
  if (!visible || !pile.length) return;
  if (document.getElementById("fenetre-defausse")) return fermerFenetre();

  const voile = document.createElement("div");
  voile.id = "fenetre-defausse";
  voile.className = "fdef";
  // Cliquer à côté referme : le geste qu'on fait sans y penser.
  voile.addEventListener("click", (e) => {
    if (e.target === voile) fermerFenetre();
  });

  const cadre = document.createElement("div");
  cadre.className = "fdef__cadre";

  const tete = document.createElement("header");
  tete.className = "fdef__tete";
  const titre = document.createElement("h2");
  titre.className = "fdef__titre";
  titre.textContent = MOT.discardPile;
  const dit = document.createElement("span");
  dit.className = "fdef__dit";
  dit.id = "fdef-compte";
  const fermer = document.createElement("button");
  fermer.type = "button";
  fermer.className = "fdef__fermer";
  fermer.dataset.fermerDefausse = "";
  fermer.textContent = MOT.close;
  fermer.addEventListener("click", fermerFenetre);
  tete.append(titre, dit, fermer);
  cadre.appendChild(tete);

  // LA GRILLE PORTE LA MARQUE, et c'est elle qui défile : « avec un défilement »
  // est une exigence, et un défilement se mesure sur l'élément qui déborde.
  const grille = document.createElement("div");
  grille.className = "fdef__grille";
  grille.id = "fdef-grille";
  grille.dataset.fenetreDefausse = "";
  cadre.appendChild(grille);

  voile.appendChild(cadre);
  document.body.appendChild(voile);
  remplirFenetre();
}

/** Referme la fenêtre. Sans effet si elle n'est pas ouverte. */
export function fermerFenetre() {
  document.getElementById("fenetre-defausse")?.remove();
}

/**
 * Le contenu de la fenêtre, dans l'ordre du moteur : `etat.defausse` est déjà la
 * plus récente d'abord. On ne trie rien, on ne renverse rien — le seul ordre qui
 * existe est le sien.
 */
function remplirFenetre() {
  const g = document.getElementById("fdef-grille");
  if (!g) return;
  const compte = document.getElementById("fdef-compte");
  if (compte) compte.textContent = MOT.discardCount(pile.length);
  const signature = pile.map((c) => c.id).join(",");
  if (g.dataset.signature === signature) return;
  g.dataset.signature = signature;
  g.textContent = "";
  for (const c of pile) {
    const f = carte(c, { classe: "carte--defaussee" });
    // PAS DE LOUPE : aucune de ces cartes n'est rendue survolable, et un clic
    // dessus ne fait rien du tout. C'est la demande, mot pour mot.
    g.appendChild(f);
  }
}

/** Remet la mémoire à zéro (nouvelle partie, table vidée). */
export function oublierDefausse() {
  pile = [];
  dessus = "";
  fermerFenetre();
  const z = document.getElementById("defausse");
  if (z) {
    delete z.dataset.cle;
    z.textContent = "";
  }
}

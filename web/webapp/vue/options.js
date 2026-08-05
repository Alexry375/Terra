// LES OPTIONS — une sortie, à tout instant.
//
// Le manque signalé par le joueur : « Je ne vois toujours pas de bouton options
// avec "retour au menu principal" et "aide" ». Une fois la partie lancée, la
// seule issue était de recharger la page.
//
// Ce module pose donc DEUX choses, et les tient pendant toute la partie :
//
//   1. un bouton toujours visible et toujours cliquable — à chacune des cinq
//      phases, pendant qu'une décision attend, pendant une annonce, et jusque
//      sur l'écran final ;
//   2. un panneau qui propose reprendre, l'aide, les réglages et le retour au
//      menu. L'aide et les réglages s'affichent DANS ce panneau, sans jamais
//      faire disparaître les quatre entrées : on n'est jamais coincé dedans.
//
// LES RÉGLAGES AGISSENT VRAIMENT. Un interrupteur qui n'éteint rien est un
// mensonge : chacun de ceux qui sont affichés a un effet mesurable de
// l'extérieur de la page. Il n'y en a donc que deux — les autres attendent le
// moteur qui les rendrait effectifs.

import { viderScene } from "./scene.js";
import { adversaireAgit, oublierMains } from "./mains.js";
import { oublier } from "./monde.js";
import { oublierPlateaux } from "./plateau.js";
import { oublierGains } from "./joueurs.js";
import { oublierPhases } from "./phases.js";
// COUTURE : la table des phases est un apport de `table-vivante`, que ce
// chantier-ci n'avait pas sous les yeux. Vider la table sans elle laisserait les
// cartes Phase posées devant chaque joueur après le retour au menu.
import { oublierTable } from "./table.js";
import { reglerAnimations, animationsActives } from "./anim.js";
// COUTURE : `cartes-qui-bougent` ajoute une option de PARTIE, et non de confort.
// Voir la défausse n'est pas une règle officielle — c'est un arrangement entre
// les deux joueurs, qui s'allume et s'éteint comme le reste.
import { reglerDefausse, defausseVisible } from "./defausse.js";
import { oublierRefs } from "./ecrire.js";
import { vueAide } from "./aide.js";
import { MOT } from "./mots.js";

// ---------------------------------------------------------------- les réglages
//
// Chaque réglage se décrit ici, avec SON effet. `lire` reprend l'état réel du
// document plutôt qu'un souvenir : le masquage des points de victoire a déjà un
// interrupteur ailleurs dans la page (`vue/plateau.js`), et les deux doivent
// dire la même chose.
const REGLAGES = [
  {
    cle: "animations",
    nom: MOT.setAnimations,
    note: MOT.setAnimationsNote,
    // COUTURE (menu-et-options × table-vivante). Ce chantier-ci écrivait
    // lui-même `html[data-animations]` ; `table-vivante` tient de son côté, dans
    // `vue/anim.js`, la durée des vols de cartes et `body[data-animations]`.
    // Deux mémoires pour un seul réglage : basculer l'interrupteur aurait éteint
    // les transitions CSS sans arrêter les vols. On délègue donc à
    // `reglerAnimations`, devenu l'unique point d'écriture, qui pose les DEUX
    // attributs — la règle de `style-menu.css` (sur la racine) et celle de
    // `style-table.css` (sur le corps) s'éteignent ensemble, et les durées
    // JavaScript avec elles.
    lire: () => animationsActives(),
    poser(actif) {
      reglerAnimations(actif);
    },
  },
  {
    cle: "points-de-victoire",
    nom: MOT.vp,
    note: MOT.setScoreNote,
    lire: () => document.documentElement.dataset.pvMasques !== "oui",
    poser(actif) {
      // `style.css` porte déjà la règle qui range les jauges de score
      // (`html[data-pv-masques="oui"] [data-role="vp"]`) : on la pilote, on ne
      // la réécrit pas. Le décompte FINAL, lui, n'est jamais masqué — la partie
      // est finie, c'est le moment de le lire.
      if (actif) delete document.documentElement.dataset.pvMasques;
      else document.documentElement.dataset.pvMasques = "oui";
    },
  },
  {
    // (CNF-2) VOIR LA DÉFAUSSE. Ce n'est pas une règle du jeu : le livret ne
    // donne pas le droit de fouiller la pile. C'est une option de partie, que
    // les deux joueurs se donnent ou non — d'où sa place ici, et non dans le
    // moteur. Éteinte, la pile ne montre rien et la fenêtre ne s'ouvre pas.
    //
    // L'écriture passe par `vue/defausse.js`, qui en est l'unique point : ce
    // module-ci lit et demande, il ne pose pas l'attribut lui-même. C'est la
    // leçon du réglage des animations, qui avait fini avec deux mémoires.
    cle: "defausse",
    nom: MOT.setDiscard,
    note: MOT.setDiscardNote,
    lire: () => defausseVisible(),
    poser(actif) {
      reglerDefausse(actif);
    },
  },
];

// ------------------------------------------------------------------ le montage

let bouton = null;
let panneau = null;
let zoneVue = null;
let vueCourante = null;
let vueReglages = null;
let retourAuMenu = null;

/** Le dessin du bouton : un engrenage, tracé ici et non chargé d'ailleurs. */
function engrenage() {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 32 32");
  svg.setAttribute("aria-hidden", "true");
  const dents = 8;
  const pas = Math.PI / dents;
  let d = "";
  for (let i = 0; i < dents * 2; i++) {
    const r = i % 2 ? 9.2 : 12.6;
    const a = i * pas - Math.PI / 2;
    d += `${i ? "L" : "M"}${(16 + r * Math.cos(a)).toFixed(2)},${(16 + r * Math.sin(a)).toFixed(2)}`;
  }
  d += "Z";
  // Le moyeu, percé par la règle du pair-impair : l'engrenage est un anneau.
  d += "M11.2,16a4.8,4.8 0 1,0 9.6,0a4.8,4.8 0 1,0 -9.6,0Z";
  const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
  p.setAttribute("d", d);
  p.setAttribute("fill-rule", "evenodd");
  svg.appendChild(p);
  return svg;
}

function entree(action, mot) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "options__entree";
  b.dataset.optionsAction = action;
  b.textContent = mot;
  return b;
}

function batirPanneau() {
  const z = document.createElement("div");
  z.id = "options";
  z.dataset.optionsPanneau = "";
  z.hidden = true;

  const cadre = document.createElement("div");
  cadre.className = "options__cadre";

  const nav = document.createElement("nav");
  nav.className = "options__nav";
  const titre = document.createElement("span");
  titre.className = "options__titre";
  titre.textContent = MOT.options;
  nav.appendChild(titre);

  const actions = [
    ["reprendre", MOT.resume, () => fermerOptions()],
    ["aide", MOT.help, () => montrerVue("aide")],
    ["reglages", MOT.settings, () => montrerVue("reglages")],
    ["menu", MOT.backToMenu, () => retourAuMenu?.()],
  ];
  for (const [action, mot, faire] of actions) {
    const b = entree(action, mot);
    b.addEventListener("click", faire);
    nav.appendChild(b);
  }
  cadre.appendChild(nav);

  zoneVue = document.createElement("section");
  zoneVue.className = "options__vue";
  cadre.appendChild(zoneVue);

  z.appendChild(cadre);

  // Cliquer à côté du cadre referme : c'est le geste qu'on fait sans y penser.
  z.addEventListener("click", (e) => {
    if (e.target === z) fermerOptions();
  });
  return z;
}

// ------------------------------------------------------------- les deux vues

function ligneReglage(r) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "reglage";
  b.dataset.reglage = r.cle;

  const nom = document.createElement("span");
  nom.className = "reglage__nom";
  nom.textContent = r.nom;
  b.appendChild(nom);

  const note = document.createElement("span");
  note.className = "reglage__note";
  note.textContent = r.note;
  b.appendChild(note);

  const etat = document.createElement("span");
  etat.className = "reglage__etat";
  b.appendChild(etat);

  const peindre = () => {
    const actif = r.lire();
    b.dataset.reglageEtat = actif ? "oui" : "non";
    etat.textContent = actif ? MOT.stateOn : MOT.stateOff;
  };
  b.addEventListener("click", () => {
    r.poser(!r.lire());
    peindre();
  });
  b.peindre = peindre;
  peindre();
  return b;
}

function batirReglages() {
  const z = document.createElement("div");
  z.className = "reglages";

  const tete = document.createElement("div");
  tete.className = "reglages__tete";
  const h = document.createElement("h2");
  h.textContent = MOT.settings;
  tete.appendChild(h);
  const p = document.createElement("p");
  p.textContent = MOT.settingsLead;
  tete.appendChild(p);
  z.appendChild(tete);

  for (const r of REGLAGES) z.appendChild(ligneReglage(r));
  return z;
}

/** Montre l'une des deux vues DANS le panneau — les quatre entrées restent là. */
function montrerVue(quoi) {
  if (!panneau) return;
  if (quoi === "aide") {
    zoneVue.replaceChildren(vueAide());
  } else {
    if (!vueReglages) vueReglages = batirReglages();
    // L'état affiché est relu du document : un autre coin de la page peut avoir
    // bougé le même interrupteur entre-temps.
    for (const b of vueReglages.querySelectorAll("[data-reglage]")) b.peindre();
    zoneVue.replaceChildren(vueReglages);
  }
  vueCourante = quoi;
  for (const b of panneau.querySelectorAll("[data-options-action]")) {
    b.dataset.actif = b.dataset.optionsAction === quoi ? "oui" : "non";
  }
}

// ------------------------------------------------------------- ouvrir, fermer

export function ouvrirOptions() {
  if (!panneau) return;
  panneau.hidden = false;
  montrerVue(vueCourante === "aide" ? "aide" : "reglages");
  panneau.querySelector('[data-options-action="reprendre"]')?.focus({ preventScroll: true });
}

export function fermerOptions() {
  if (panneau) panneau.hidden = true;
}

export function optionsOuvertes() {
  return !!panneau && !panneau.hidden;
}

/** Le bouton n'existe que pendant une partie : sur l'accueil, il n'a rien à ouvrir. */
export function montrerBoutonOptions(oui) {
  if (bouton) bouton.hidden = !oui;
  if (!oui) fermerOptions();
}

/**
 * Pose le bouton et le panneau, une fois pour toutes.
 * @param {{auMenu: () => void}} o  ce qu'il faut faire pour revenir au menu
 */
export function installerOptions({ auMenu }) {
  retourAuMenu = auMenu;

  // COUTURE : ce chantier lisait ici `?animations=non`. `table-vivante` le lit
  // déjà dans `interface.js` (`lireCadre`) et le pose par `reglerAnimations`
  // avant que ce module ne soit installé. Deux lectures du même paramètre
  // finiraient par se contredire ; on garde la sienne, qui est la première, et
  // on se contente de rejouer l'état courant pour que les deux attributs soient
  // posés même si l'ordre d'appel changeait un jour.
  reglerAnimations(animationsActives());

  bouton = document.createElement("button");
  bouton.id = "options-ouvrir";
  bouton.type = "button";
  bouton.dataset.optionsOuvrir = "";
  bouton.setAttribute("aria-label", MOT.optionsOpen);
  bouton.title = MOT.options;
  bouton.hidden = true;
  bouton.appendChild(engrenage());
  bouton.addEventListener("click", () => {
    if (optionsOuvertes()) fermerOptions();
    else ouvrirOptions();
  });
  document.body.appendChild(bouton);

  panneau = batirPanneau();
  document.body.appendChild(panneau);

  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    if (optionsOuvertes()) fermerOptions();
    else if (!bouton.hidden) ouvrirOptions();
  });
}

// -------------------------------------------------------------- vider la table

/**
 * VIDER LA TABLE — le retour au menu n'est pas un rideau tiré devant la partie.
 *
 * On emprunte exactement le chemin que le jeu emploie déjà au début de chaque
 * partie (`interface.js`, `lancer`) : chaque module oublie ce qu'il montrait, et
 * ses conteneurs se vident. On y ajoute deux filets, parce que « la table est
 * vide » doit être vrai et pas seulement probable :
 *
 *   • le souvenir de la corporation (`data-corpo`) est effacé, sans quoi la
 *     partie suivante ne la repeindrait pas si elle tombait sur la même ;
 *   • aucune carte ne survit, d'où qu'elle vienne.
 */
export function viderTable() {
  viderScene();
  adversaireAgit(null);
  oublier();
  oublierPlateaux();
  oublierMains();
  oublierPhases();
  // COUTURE : ajouté par la fusion. `table-vivante` a sorti les cartes Phase de
  // `vue/phases.js` pour les poser sur la table (`vue/table.js`) ; oublier les
  // phases ne suffit donc plus à vider les deux docks.
  oublierTable();
  // Sans cet oubli, la premiere reserve de la partie suivante serait lue
  // comme un GAIN par rapport a la partie precedente : un « +38 » au premier
  // ecran, sur un compteur qui n'a rien recu.
  oublierGains();
  oublierRefs();

  document.getElementById("final")?.remove();
  document.getElementById("chargement")?.remove();

  // L'annonce de la dernière manche jouée : elle appartient à la partie qu'on
  // vient de quitter. La laisser, c'est laisser derrière l'accueil un numéro de
  // manche périmé qui déclare encore son chemin (`data-valeur="generation"`).
  const annonce = document.getElementById("annonce");
  if (annonce) {
    annonce.textContent = "";
    annonce.classList.remove("annonce--vive");
  }

  for (const z of document.querySelectorAll("[data-corpo]")) {
    delete z.dataset.corpo;
    // COUTURE : `table-vivante` a déplacé la mémoire du dessin de la corporation
    // dans `data-etat-corpo` (`vue/joueurs.js`), pour que `data-corpo` ne porte
    // jamais un nom qu'on n'a pas le droit de lire. C'est donc CELLE-CI qu'il
    // faut effacer, sans quoi la partie suivante ne repeindrait pas une
    // corporation tombée deux fois de suite.
    delete z.dataset.etatCorpo;
    z.textContent = "";
  }
  for (const c of document.querySelectorAll(".carte, [data-carte-id]")) c.remove();

  // Plus personne n'a la parole : l'attribut est retiré, pas vidé.
  delete document.body.dataset.actif;
}

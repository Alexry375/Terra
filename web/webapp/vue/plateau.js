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
  EQUIPAGES, nomJoueur, faceOcean, dosOcean, cleOcean, NB_OCEANS,
} from "./materiel.js";
import { survolable } from "./loupe.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";
import { avantLaFinDuRattrapage } from "./anim.js";

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

// ------------------------------------------------- les durées de la planche
//
// LE RETOURNEMENT SE VOIT, ET LE JOUEUR CHOISIT OÙ. Trois durées, et une seule
// règle : `?animations=non` (ou l'interrupteur du panneau d'options) les met
// toutes à zéro côté JavaScript, pendant que `style-menu.css` met les durées
// CSS à 1 ms — jamais à 0 s, sans quoi certains navigateurs ne signalent plus la
// fin d'une transition. Rien de ce qui est DÉCIDÉ n'en dépend : la tuile
// révélée est la même, son bonus est le même, et la partie avance pareil.
const FLIP = 620; // la durée du demi-tour, celle qu'écrit `style-monde.css`
const DELAI_CHOIX = 2600; // le temps laissé au joueur pour désigner un emplacement
const TENUE_ANNONCE = 1600; // le temps que la grande tuile reste sous les yeux

/** La durée réellement appliquée. Même interrupteur que `vue/anim.js`, lu sur
 *  le document : ce module n'a pas à savoir par quel chemin il a été posé. */
function duree(ms) {
  return document.documentElement.dataset.animations === "non" ? 0 : ms;
}

/**
 * LA PAGE REJOUE UNE PARTIE DÉJÀ JOUÉE (après un rechargement). Les océans se
 * sont révélés il y a longtemps : les annoncer maintenant, en grand, au milieu
 * de l'écran, c'est présenter comme neuf ce qui est vieux. On les POSE, sans
 * un mot — même chemin qu'à la fin de la partie, où plus personne ne désigne.
 * Lu sur le document, comme l'interrupteur des durées juste au-dessus.
 */
function enRattrapage() {
  return document.documentElement.dataset.rattrapage === "oui";
}

// L'ÉCRAN PARLE ANGLAIS AU JOUEUR, et tout son vocabulaire vit dans
// `vue/mots.js` — un seul endroit, pour qu'aucune phrase ne se retrouve écrite
// deux fois de deux façons. Ces deux-là y sont remontées le 04-08 : le chantier
// des océans n'avait pas le droit d'écrire dans ce fichier, il les avait donc
// laissées ici en attendant.
const CONSIGNE = MOT.oceanPick;
const ANNONCE_CHOIX = MOT.oceanRevealPick;

// LA MÉMOIRE DU CHOIX. L'écran est réécrit à chaque décision, et une partie en
// compte plusieurs centaines : sans cette table, la tuile choisie sauterait
// d'emplacement au premier redessin.
//
//   `emplacementParRang[r]` — l'emplacement où la r-ième révélation du moteur a
//                             été posée. C'est la SEULE chose que le joueur
//                             choisit : le bonus, lui, reste celui du moteur.
//   `fileRevelations`       — les révélations annoncées par le moteur qui
//                             n'ont pas encore d'emplacement.
//   `attente`               — le choix ouvert en ce moment, et son minuteur.
let emplacementParRang = [];
let fileRevelations = [];
let attente = null;
// LA PARTIE EST FINIE : plus rien n'attend. Mesuré au siège 1, graine 2024 : les
// deux dernières révélations tombaient dans les toutes dernières décisions, et
// la planche restait à sept tuiles face visible pendant que l'écran final
// s'affichait — le joueur ne saurait jamais ce qu'il y avait dessous. Quand le
// moteur déclare la fin, tout ce qui attend se retourne à l'instant.
let partieFinie = false;

// ------------------------------------------------------------------ le décor

/** Les deux plateaux, la carte des océans et la case des VP. Une seule fois. */
export function construirePlateaux() {
  // L'animation de retournement se coupe par l'adresse (`?animations=non`),
  // sans quoi aucune mesure automatique ne peut travailler proprement. Le
  // réglage ne change QUE des durées : la tuile est révélée au même instant,
  // avec le même contenu, dans les deux cas.
  if (new URLSearchParams(location.search).get("animations") === "non") {
    document.documentElement.dataset.animations = "non";
  }

  for (const j of [0, 1]) {
    const s = document.createElement("section");
    s.className = "plateau";
    s.id = "plateau-" + j;
    s.dataset.plateau = String(j);
    s.style.setProperty("--teinte", EQUIPAGES[j].teinte);
    // Le vis-à-vis n'est PAS écrit ici : il dépend du siège regardé, pas du
    // numéro du joueur (`majPlateaux`). Le survol, lui, rend toujours la carte à
    // l'endroit (voir `loupe.js`).
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

  // On agrandit la carte des océans pour la détailler.
  //
  // CNF-5 (05-08) — ON LA REFERME D'UN CLIC N'IMPORTE OÙ. Le clic était posé
  // sur la planche elle-même : une fois agrandie, elle occupe le centre de
  // l'écran, et il fallait retomber DESSUS pour la refermer — cliquer à côté,
  // le geste naturel, ne faisait rien.
  //
  // L'écouteur vit donc sur le document. Ouverte, la planche se referme quel
  // que soit l'endroit cliqué ; fermée, seul un clic sur elle l'ouvre. Un même
  // clic ne peut pas faire les deux : le premier cas rend la main aussitôt.
  //
  // Désigner une tuile continue de fonctionner : la grille arrête ce clic-là
  // (`stopPropagation` ci-dessous), il n'atteint jamais le document.
  document.addEventListener("click", (ev) => {
    if (o.classList.contains("oceans--grande")) {
      o.classList.remove("oceans--grande");
      return;
    }
    if (o.contains(ev.target)) o.classList.add("oceans--grande");
  });

  // DÉSIGNER L'EMPLACEMENT QUI SE RETOURNE. L'écouteur est posé sur la grille,
  // une fois pour toutes : les emplacements, eux, ne sont jamais recréés. Il
  // arrête le clic avant la section, sinon désigner une tuile agrandirait ou
  // refermerait la planche du même geste.
  const grille = o.querySelector("#oceans-grille");
  grille.addEventListener("click", (ev) => {
    if (!attente) return;
    const d = ev.target.closest(".ocean");
    if (!d || d.dataset.oceanChoisissable === undefined) return;
    ev.stopPropagation();
    choisirEmplacement([...grille.children].indexOf(d));
  });

  // LES NEUF EMPLACEMENTS, TOUS RETOURNÉS AU DÉPART. Une tuile encore
  // retournée ne porte QUE son dos : pas d'identité, pas de bonus, pas de
  // chiffre, et pas même le nom de fichier de sa face — le scan de la face
  // n'entre dans le document qu'au moment où le moteur la révèle
  // (`revelerTuile`). C'est la seule façon de ne rien laisser deviner : une
  // information cachée mais présente reste une information donnée.
  const g = o.querySelector("#oceans-grille");
  for (let i = 0; i < NB_OCEANS; i++) {
    const d = document.createElement("div");
    d.className = "ocean";
    d.dataset.oceanTuile = "";
    d.dataset.oceanRevelee = "non";

    const pivot = document.createElement("div");
    pivot.className = "ocean__pivot";
    const dos = document.createElement("div");
    dos.className = "ocean__dos";
    const im = document.createElement("img");
    im.src = dosOcean();
    im.alt = MOT.oceanFaceDown;
    im.draggable = false;
    dos.appendChild(im);
    pivot.appendChild(dos);
    d.appendChild(pivot);
    g.appendChild(d);
  }
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

/**
 * Réécrit les deux plateaux et la carte des océans à partir de l'état.
 *
 * @param {number} siege  le joueur assis en bas de l'écran : c'est LUI qui voit
 *                        sa table à l'endroit, et l'autre de l'autre bord.
 */
export function majPlateaux(etat, decision, siege = 0) {
  for (const p of etat.players) {
    const j = p.player;
    const s = ref("#plateau-" + j);
    if (s) {
      s.classList.toggle("plateau--actif", !!decision && decision.joueur === j);
      s.dataset.visAVis = j === siege ? "non" : "oui";
    }
    piles(j, p, j !== siege);
  }
  // `game_over` vient du moteur, comme tout le reste : c'est le même fait que
  // celui qu'`interface.js` déclare sur le corps du document. La planche s'en
  // sert pour ne rien laisser en attente à la fin de la partie.
  oceans(etat.planet.oceans_revealed_tiles || [], !!etat.game_over);
}

/**
 * Les piles d'un joueur : une par couleur, coupée tous les `PAR_PILE` cartes.
 * Rien n'est réécrit tant que rien n'a bougé — l'écran se redessine à chaque
 * décision, et une partie en compte plusieurs centaines.
 */
function piles(j, p, visAVis) {
  const z = ref("#piles-" + j);
  if (!z) return;
  const signature =
    (visAVis ? "f#" : "n#") + p.played.map((c) => `${c.id}:${c.resources ?? 0}`).join("|");
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
      const pile = fabriquerPile(j, couleur, morceau, visAVis);
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
 * LA PLANCHE DES OCÉANS. Neuf emplacements en permanence ; le moteur publie les
 * tuiles DÉJÀ retournées, dans l'ordre où elles l'ont été
 * (`planet.oceans_revealed_tiles` : `{id, cards, mc, plants}`), et on leur donne
 * EXACTEMENT le bonus qu'il annonce.
 *
 * QUEL EMPLACEMENT SE RETOURNE ? Ce n'était pas une question : la r-ième tuile
 * révélée allait sur le r-ième emplacement, et le joueur regardait faire. Le
 * propriétaire du projet a demandé le 04-08 à CHOISIR, et a autorisé
 * explicitement que ce choix soit PUREMENT VISUEL — le bonus reste celui que le
 * moteur a tiré, seul l'emplacement change. Alors :
 *
 *   1. le moteur annonce une révélation → elle entre dans `fileRevelations` ;
 *   2. la planche s'ouvre, les emplacements encore face cachée deviennent
 *      désignables, et une grande tuile paraît au milieu de l'écran — pour que
 *      l'événement ne se rate pas depuis son siège ;
 *   3. le joueur désigne un emplacement, ou ne fait rien : au bout de
 *      `DELAI_CHOIX` c'est le premier emplacement libre qui se retourne. LA
 *      PARTIE N'ATTEND JAMAIS CE CHOIX — rien, dans la boucle de jeu, ne dépend
 *      de cette file ; elle vit sur des minuteurs, à côté.
 *
 * Ce que le moteur ne publie pas — les tuiles encore retournées — n'entre pas
 * dans le document : c'est ce qui rend la fuite impossible plutôt que
 * seulement improbable. Une tuile en attente d'emplacement n'y entre pas non
 * plus : elle reste dans `fileRevelations`, en mémoire, et n'apparaît qu'au
 * moment où elle se retourne.
 */
function oceans(revelees, fin) {
  const g = ref("#oceans-grille");
  if (!g) return;
  partieFinie = !!fin;
  const signature = revelees.map((t) => `${t.id}:${cleOcean(t)}`).join("|");
  if (g.dataset.revelees === signature) {
    // Rien de neuf, mais la partie vient peut-être de s'achever : ce qui
    // attendait un choix ne peut plus l'attendre.
    if (partieFinie) haterLesRevelations();
    return;
  }
  g.dataset.revelees = signature;

  // L'ÉTAT PEUT RECULER (`verif/recul-etat.py` en a compté 46 sur une partie) :
  // moins de tuiles révélées qu'avant, c'est une planche à refaire de zéro.
  if (revelees.length < emplacementParRang.length) {
    oublierChoixOceans();
    [...g.children].forEach(retournerFaceCachee);
  }

  // Les révélations déjà placées reprennent LEUR emplacement, celui qui a été
  // choisi : c'est ce qui fait qu'un redessin ne déplace jamais une tuile.
  emplacementParRang.forEach((i, rang) => {
    const t = revelees[rang];
    const d = g.children[i];
    if (t && d && d.dataset.oceanRevelee !== "oui") revelerTuile(d, t);
  });

  // UNE RÉVÉLATION EN ATTENTE QUE LE MOTEUR N'ANNONCE PLUS n'a pas à se
  // retourner : l'état a reculé sous elle. On la retire, et le choix ouvert
  // avec elle — retourner une tuile que le moteur ne publie pas serait
  // exactement le mensonge que cette planche s'interdit.
  fileRevelations = fileRevelations.filter((r) => r.rang < revelees.length);
  if (attente && !fileRevelations.length) {
    clearTimeout(attente.minuteur);
    attente = null;
    fermerLeChoix();
    fermerAnnonce();
  }

  // Les nouvelles attendent qu'on leur désigne un emplacement.
  for (let rang = emplacementParRang.length; rang < revelees.length; rang++) {
    if (!fileRevelations.some((r) => r.rang === rang)) {
      fileRevelations.push({ rang, tuile: revelees[rang] });
    }
  }
  ouvrirLeChoix();
  if (partieFinie) haterLesRevelations();
}

/**
 * TOUT SE RETOURNE, MAINTENANT. À la fin de la partie il n'y a plus personne
 * pour désigner quoi que ce soit, et une tuile laissée face cachée serait une
 * tuile que le joueur ne verra jamais. La boucle est bornée par le nombre
 * d'emplacements : elle ne peut pas tourner sans fin, même si un emplacement
 * venait à manquer.
 */
function haterLesRevelations() {
  for (let garde = 0; garde <= NB_OCEANS && (attente || fileRevelations.length); garde++) {
    if (attente) choisirEmplacement(attente.defaut);
    else ouvrirLeChoix();
  }
}

// LE DERNIER INSTANT DU RATTRAPAGE. Tout ce qui reste en attente se pose ici,
// sans mise en scène, tant que `data-rattrapage` vaut encore « oui ». Sans
// cela, la dernière tuile de la file se retournait en grand APRÈS le retour à
// la normale — cent millisecondes plus tard, mesuré le 04-08 — et c'est
// exactement ce que le joueur voyait à chaque rechargement.
avantLaFinDuRattrapage(haterLesRevelations);

/** Les emplacements encore face cachée, dans l'ordre de la planche. */
function emplacementsLibres(g) {
  return [...g.children].filter((d) => d.dataset.oceanRevelee !== "oui");
}

/**
 * Ouvre le choix de la prochaine révélation en attente. Sans effet s'il y a
 * déjà un choix ouvert : les révélations se suivent, jamais ne se chevauchent.
 */
function ouvrirLeChoix() {
  if (attente || !fileRevelations.length) return;
  const g = ref("#oceans-grille");
  const o = ref("#oceans");
  if (!g || !o) return;
  const libres = emplacementsLibres(g);
  if (!libres.length) {
    // Neuf emplacements pour neuf tuiles : on ne devrait jamais passer ici. Si
    // le moteur en annonçait une dixième, on le dirait plutôt que de la perdre.
    console.warn("plateau.js : une tuile océan révélée sans emplacement libre");
    fileRevelations.shift();
    return;
  }

  const { tuile } = fileRevelations[0];
  const defaut = [...g.children].indexOf(libres[0]);
  attente = { tuile, defaut, minuteur: null };

  // LA PARTIE EST FINIE, ou la page rattrape son retard : on ne pose plus de
  // question et on n'annonce rien, on montre.
  if (partieFinie || enRattrapage()) {
    choisirEmplacement(defaut);
    return;
  }
  ouvrirAnnonce();

  // Un seul emplacement libre : il n'y a rien à choisir, on le retourne.
  if (libres.length > 1) {
    o.classList.add("oceans--choix");
    for (const d of libres) d.dataset.oceanChoisissable = "";
    poserConsigne(o);
  }

  attente.minuteur = setTimeout(
    () => choisirEmplacement(defaut),
    libres.length > 1 ? duree(DELAI_CHOIX) : 0,
  );
}

/**
 * L'emplacement `i` se retourne, avec la tuile que le moteur a tirée. Appelé
 * par le clic du joueur comme par le minuteur : les deux chemins sont le même.
 */
function choisirEmplacement(i) {
  if (!attente) return;
  const g = ref("#oceans-grille");
  const d = g && g.children[i];
  if (!d || d.dataset.oceanRevelee === "oui") return;

  const { tuile, minuteur } = attente;
  clearTimeout(minuteur);
  attente = null;
  const rang = fileRevelations.shift().rang;
  fermerLeChoix();

  emplacementParRang[rang] = i;
  revelerTuile(d, tuile);
  retournerLAnnonce(tuile);

  // La révélation suivante, s'il y en a une, attend que celle-ci ait fini de se
  // montrer : deux grandes tuiles au milieu de l'écran en même temps, ce serait
  // deux événements qu'on rate au lieu d'un qu'on voit. À la fin de la partie,
  // en revanche, `haterLesRevelations` les enchaîne sans attendre.
  // En rattrapage, la suivante enchaîne TOUT DE SUITE : un `setTimeout`, même
  // à durée nulle, rend la main au moteur, qui prend de l'avance — et la file
  // finit de se vider après la fin du rattrapage, animations rallumées.
  if (enRattrapage()) ouvrirLeChoix();
  else if (!partieFinie) setTimeout(ouvrirLeChoix, duree(FLIP + TENUE_ANNONCE));
}

/** Éteint la désignation : plus rien n'est cliquable sur la planche. */
function fermerLeChoix() {
  const o = ref("#oceans");
  const g = ref("#oceans-grille");
  if (o) o.classList.remove("oceans--choix");
  if (g) for (const d of g.children) delete d.dataset.oceanChoisissable;
  ref("#oceans-consigne")?.remove();
}

/** La consigne, en clair, au-dessus de la planche. Elle disparaît avec le choix. */
function poserConsigne(o) {
  if (o.querySelector("#oceans-consigne")) return;
  const s = document.createElement("span");
  s.className = "oceans__consigne";
  s.id = "oceans-consigne";
  s.textContent = CONSIGNE;
  o.prepend(s);
}

// ------------------------------------------------- la grande tuile du milieu
//
// LE RETOURNEMENT SE VOIT. La planche fait cent dix points de large dans un coin
// de l'écran, et ses tuiles une cinquantaine : un demi-tour de 620 ms y passe
// totalement inaperçu — c'est très exactement le reproche du 04-08, « aucune
// tuile n'est face visible même quand on en retourne ». La tuile se retourne
// donc AUSSI en grand, au milieu de l'écran, avec son bonus écrit en toutes
// lettres.
//
// Cette annonce ne peut rien bloquer : elle est en `pointer-events: none`, donc
// un clic la traverse — ni une main ni une machine qui pilote la page ne peut
// s'y heurter. Et elle n'existe dans le document que pendant qu'elle se montre.

/** La grande tuile paraît, DE DOS : rien de la tuile n'est encore dans la page. */
function ouvrirAnnonce() {
  fermerAnnonce();
  const a = document.createElement("div");
  a.id = "ocean-annonce";
  a.dataset.oceanAnnonce = "";
  a.setAttribute("aria-hidden", "true");

  const t = document.createElement("div");
  t.className = "annonce-ocean__tuile";
  const pivot = document.createElement("div");
  pivot.className = "annonce-ocean__pivot";
  const dos = document.createElement("div");
  dos.className = "annonce-ocean__dos";
  const im = document.createElement("img");
  im.src = dosOcean();
  im.alt = MOT.oceanFaceDown;
  im.draggable = false;
  dos.appendChild(im);
  pivot.appendChild(dos);
  t.appendChild(pivot);

  const mot = document.createElement("p");
  mot.className = "annonce-ocean__mot";
  mot.textContent = ANNONCE_CHOIX;

  a.appendChild(t);
  a.appendChild(mot);
  document.body.appendChild(a);
}

/** Elle se retourne à son tour, dit le bonus, puis s'efface. */
function retournerLAnnonce(t) {
  const a = ref("#ocean-annonce");
  if (!a) return;
  // La partie est finie et l'écran des scores est là : une grande tuile qui se
  // retourne par-dessus n'annonce plus rien, elle recouvre.
  if (partieFinie) {
    fermerAnnonce();
    return;
  }
  const pivot = a.querySelector(".annonce-ocean__pivot");
  const mot = a.querySelector(".annonce-ocean__mot");
  if (!pivot || !mot) return;
  const face = document.createElement("div");
  face.className = "annonce-ocean__face";
  remplirFace(face, t, "annonce-ocean__sansscan");
  pivot.prepend(face);
  mot.textContent = bonusEnMots(t);
  requestAnimationFrame(() => a.classList.add("annonce-ocean--retournee"));
  setTimeout(fermerAnnonce, duree(FLIP + TENUE_ANNONCE) + 40);
}

function fermerAnnonce() {
  ref("#ocean-annonce")?.remove();
}

/**
 * La face d'une tuile : le scan du bonus que le moteur annonce, ou ce bonus
 * écrit en toutes lettres quand aucun scan ne le porte — jamais une tuile qui
 * annoncerait autre chose. Le même dessin sert la planche et l'annonce.
 */
function remplirFace(hote, t, classeSansScan) {
  hote.textContent = "";
  const src = faceOcean(t);
  if (src) {
    const im = document.createElement("img");
    im.src = src;
    im.alt = `${MOT.oceanFaceUp} — ${bonusEnMots(t)}`;
    im.draggable = false;
    hote.appendChild(im);
    return;
  }
  const s = document.createElement("span");
  s.className = classeSansScan;
  s.textContent = bonusEnMots(t);
  hote.appendChild(s);
}

/** Retourne un emplacement face visible, avec le bonus que le moteur annonce. */
function revelerTuile(d, t) {
  const pivot = d.querySelector(".ocean__pivot");
  if (!pivot) return;
  let face = pivot.querySelector(".ocean__face");
  if (!face) {
    face = document.createElement("div");
    face.className = "ocean__face";
    // EN TÊTE, avant le dos : « la première image de cette tuile » doit être
    // celle qu'on voit — le dos tant qu'elle est retournée, la face après.
    pivot.prepend(face);
  }
  remplirFace(face, t, "ocean__sansscan");

  d.dataset.oceanId = String(t.id);
  d.dataset.oceanBonus = `cards=${t.cards | 0},mc=${t.mc | 0},plants=${t.plants | 0}`;
  d.title = `${MOT.oceanFaceUp} — ${bonusEnMots(t)}`;
  // LE SURVOL PORTE SUR LA FACE, PAS SUR LE DOS. Mesuré le 04-08 :
  // `document.elementFromPoint` au centre d'une tuile retournée rend l'image du
  // DOS — Chrome garde la face arrière dans le test de survol malgré
  // `backface-visibility: hidden`, alors que le rendu, lui, est juste. Un clic
  // et une infobulle tombaient donc sur le dos d'une tuile visible. La face
  // porte son propre `title` et sa propre marque ; la feuille de style, elle,
  // retire le dos du test de survol dès que la tuile est retournée.
  face.dataset.oceanFace = "";
  face.title = d.title;
  d.dataset.oceanRevelee = "oui";
  delete d.dataset.oceanChoisissable;
  // La classe est posée au tour d'après pour que le navigateur ait vu l'état
  // « retournée » : sans ce délai, il n'y a pas de transition à animer.
  requestAnimationFrame(() => d.classList.add("ocean--retournee"));
  // Le dos ne quitte le test de survol qu'une fois le demi-tour FINI : avant, il
  // est encore ce que l'on voit. On compte le temps plutôt que d'attendre la fin
  // de la transition — une fin de transition qui ne viendrait pas laisserait le
  // survol faux pour toujours, alors qu'un minuteur, lui, arrive toujours.
  setTimeout(() => {
    if (d.dataset.oceanRevelee === "oui") d.classList.add("ocean--posee");
  }, duree(FLIP) + 60);
}

/** Remet un emplacement face cachée (nouvelle partie). */
function retournerFaceCachee(d) {
  d.classList.remove("ocean--retournee");
  d.classList.remove("ocean--posee");
  d.dataset.oceanRevelee = "non";
  delete d.dataset.oceanId;
  delete d.dataset.oceanBonus;
  delete d.dataset.oceanChoisissable;
  d.removeAttribute("title");
  const face = d.querySelector(".ocean__face");
  if (face) face.remove();
}

/** Oublie tout choix en cours et toute révélation en attente. */
function oublierChoixOceans() {
  if (attente) clearTimeout(attente.minuteur);
  attente = null;
  emplacementParRang = [];
  fileRevelations = [];
  fermerLeChoix();
  fermerAnnonce();
}

/** Le bonus d'une tuile, dit en toutes lettres, tel que le moteur le publie. */
function bonusEnMots(t) {
  const parts = [];
  if (t.cards) parts.push(`${t.cards} ${t.cards > 1 ? MOT.manyCards : MOT.oneCard}`);
  if (t.mc) parts.push(`${t.mc} ${MOT.mc}`);
  if (t.plants) parts.push(`${t.plants} ${MOT.plants}`);
  return parts.join(" + ") || MOT.oceanFaceUp;
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
  oublierChoixOceans();
  partieFinie = false;
  const g = ref("#oceans-grille");
  if (g) {
    delete g.dataset.revelees;
    [...g.children].forEach(retournerFaceCachee);
  }
}

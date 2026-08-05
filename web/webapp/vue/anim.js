// LE GESTE QUI DURE — poser une carte prend du temps, et ce temps se règle.
//
// Une carte qui saute d'un endroit à l'autre n'est pas posée : elle est
// téléportée. Tout ce module existe pour qu'on VOIE la main prendre la carte et
// la poser. Il ne décide jamais de ce qui est posé — seulement du temps que ça
// met.
//
// `?animations=non` met toutes les durées à zéro. C'est un réglage de DURÉE, pas
// de résultat : la carte part du même endroit, arrive au même endroit, et la
// réponse envoyée au moteur est la même. Sans lui, aucun contrôle automatique ne
// pourrait jouer une partie entière dans un temps raisonnable — et un contrôle
// qui doit attendre une animation finit par mesurer l'animation, pas le jeu.

// COUTURE (table-vivante × menu-et-options). Les deux chantiers ont écrit, sans
// se voir, le MÊME interrupteur d'animations, chacun par son bout :
//
//   · table-vivante  — ce module. Il tient `actives`, qui gouverne les durées
//     JavaScript (`duree`, `pause`, les vols de `poserSur`), et pose
//     `body[data-animations]`, sur quoi `style-table.css` accroche sa règle.
//   · menu-et-options — le réglage « Animations » du panneau et la lecture de
//     `?animations=non` dans `vue/options.js`. Il pose
//     `html[data-animations]`, sur quoi `style-menu.css` accroche la sienne.
//
// Deux mémoires pour un seul réglage, c'est un réglage qui ment : basculer
// l'interrupteur du panneau aurait éteint les transitions CSS de l'un sans
// toucher aux vols de l'autre. `reglerAnimations` devient donc l'UNIQUE point
// d'écriture — il pose les deux attributs — et `vue/options.js` l'appelle au
// lieu d'écrire lui-même. Aucun des deux comportements n'est perdu : les deux
// feuilles de style gardent leur sélecteur d'origine, intact.
let actives = true;

// LE RATTRAPAGE — quand la page REJOUE une partie déjà jouée.
//
// (04-08, en partie à deux.) Après un rechargement, la page redemande au
// rendez-vous toutes les décisions déjà prises et les repasse au moteur pour
// revenir à l'instant présent. Chacune redéclenchait sa mise en scène : les
// grandes tuiles océan se retournaient une à une, du début de la partie
// jusqu'au coup courant. Ce n'est pas seulement long — c'est FAUX : ces
// évènements ont déjà eu lieu, les rejouer annonce comme neuf ce qui est vieux.
//
// Le rattrapage éteint donc les durées, exactement comme `?animations=non`, mais
// SANS toucher au réglage du joueur : c'est un second interrupteur en série,
// pas une seconde valeur du même. Quand le rattrapage finit, le réglage choisi
// reprend la main tel qu'il était.
let rattrapage = false;

/** Les deux attributs, posés d'après l'état RÉEL des deux interrupteurs. */
function appliquer() {
  const eteint = !actives || rattrapage;
  document.body.dataset.animations = eteint ? "non" : "oui";
  // La règle de `style-menu.css` porte sur la racine, celle de `style-table.css`
  // sur le corps : les deux doivent voir la même chose.
  if (eteint) document.documentElement.dataset.animations = "non";
  else delete document.documentElement.dataset.animations;
  // Le rattrapage est dit À PART, car il ne veut pas dire la même chose que
  // « pas d'animations ». Une durée nulle joue quand même la mise en scène, en
  // accéléré ; le rattrapage, lui, demande de ne pas la jouer du tout. Les
  // modules qui annoncent un évènement (la grande tuile océan) lisent celui-ci.
  if (rattrapage) document.documentElement.dataset.rattrapage = "oui";
  else delete document.documentElement.dataset.rattrapage;
}

// CE QUI DOIT ÊTRE PURGÉ AVANT QUE LE RATTRAPAGE SE TERMINE.
//
// Une mise en scène différée (`setTimeout`) survit à la fin du rattrapage : le
// moteur, lui, va plus vite que la file d'attente de l'écran. Mesuré le 04-08 :
// rattrapage fini à 293 ms, grande tuile océan parue à 393 ms — cent
// millisecondes plus tard, animations rallumées, exactement le défaut signalé.
// Éteindre les durées ne suffisait donc pas ; il faut donner à ces modules un
// dernier instant, PENDANT que le rattrapage compte encore, pour vider ce
// qu'ils gardent en attente.
const aPurger = new Set();

/** S'abonner à ce dernier instant. Appelé une fois, au chargement du module. */
export function avantLaFinDuRattrapage(f) {
  aPurger.add(f);
}

/**
 * L'unique écriture du réglage des animations, quel qu'en soit le chemin :
 * `?animations=non` (lu par `interface.js`) ou l'interrupteur du panneau
 * d'options. Les deux attributs sont posés ensemble — jamais l'un sans l'autre.
 */
export function reglerAnimations(oui) {
  actives = !!oui;
  appliquer();
}

/** Le rattrapage commence ou finit. Le réglage du joueur n'est pas touché. */
export function reglerRattrapage(oui) {
  const futur = !!oui;
  // La purge a lieu AVANT la bascule, tant que `rattrapage` vaut encore vrai :
  // ce que les abonnés vident doit se vider sans mise en scène, et ils lisent
  // l'état courant pour le savoir.
  if (rattrapage && !futur) for (const f of aPurger) f();
  rattrapage = futur;
  appliquer();
}

/**
 * LE RÉGLAGE DU JOUEUR, et lui seul — jamais l'état momentané du rattrapage.
 * `vue/options.js` le lit pour afficher l'interrupteur ET le réécrit tel quel
 * (`reglerAnimations(animationsActives())`) : s'il rendait « éteint » pendant un
 * rattrapage, le panneau afficherait un réglage que le joueur n'a pas choisi et
 * finirait par l'écrire pour de bon.
 */
export function animationsActives() {
  return actives;
}

/** La durée réellement appliquée : celle demandée, ou zéro. */
export function duree(ms) {
  return actives && !rattrapage ? ms : 0;
}

export const pause = (ms) => new Promise((r) => setTimeout(r, duree(ms)));

/**
 * La couche où voyagent les cartes en vol. Créée à la première pose.
 *
 * Elle est INDISPENSABLE : la bande de la main est en `overflow: hidden`, et une
 * carte qu'on soulève y serait coupée net dès qu'elle en sort. Une carte qu'on
 * attrape quitte donc la main pour cette couche-ci, qui est posée par-dessus tout
 * l'écran et ne coupe rien.
 */
export function couche() {
  let c = document.getElementById("vol");
  if (!c) {
    c = document.createElement("div");
    c.id = "vol";
    document.body.appendChild(c);
  }
  return c;
}

/** Le rectangle d'un élément, ou null s'il n'est pas affiché. */
function boite(el) {
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return r.width && r.height ? r : null;
}

/**
 * (cartes-qui-bougent) **CE QUI VOLE DIT CE QU'IL EST.**
 *
 * Tout nœud posé dans la couche `#vol` porte `data-vol="<motif>"` — `pioche`,
 * `defausse`, `pose`, `jauge`, `mc`, `jeton`. C'est la seule contrainte de forme
 * du contrat, et elle a une raison : de l'extérieur de la page, un observateur
 * de mutations voit alors NON SEULEMENT que quelque chose a remué, mais QUOI.
 * Sans cette marque, un banc ne peut plus distinguer un vol de carte d'un vol de
 * jeton, et une mise en scène qui se trompe d'évènement passerait inaperçue.
 *
 * `pose` est le défaut : c'est le geste d'origine de ce module (`vue/geste.js`,
 * qui n'a pas le droit d'être touché par ce chantier) et il n'a pas à se
 * déclarer pour garder sa marque.
 */
const MOTIFS = new Set(["pioche", "defausse", "pose", "jauge", "mc", "jeton"]);

/**
 * LE FAC-SIMILÉ, ET SON UNIQUE FABRIQUE. Carte attrapée dans une main, jeton
 * d'une jauge, pièce de mégacrédits : tous les objets qui voyagent naissent ici,
 * dans la même couche, et repartent ensuite par le même `poserSur`. Il n'y a
 * qu'un mécanisme de vol dans cette page.
 */
function fabriquer({ depart, image, texte, motif, classe }) {
  const noeud = document.createElement("div");
  noeud.className = "vol__carte" + (classe ? " " + classe : "");
  noeud.dataset.vol = MOTIFS.has(motif) ? motif : "pose";
  noeud.style.left = depart.left + "px";
  noeud.style.top = depart.top + "px";
  noeud.style.width = depart.width + "px";
  noeud.style.height = depart.height + "px";
  if (image) {
    const im = document.createElement("img");
    im.src = image;
    im.alt = "";
    im.draggable = false;
    noeud.appendChild(im);
  } else if (texte) {
    const s = document.createElement("span");
    s.className = "vol__mot";
    s.textContent = texte;
    noeud.appendChild(s);
  }
  couche().appendChild(noeud);
  // `echelle` suit la taille COURANTE du fac-similé. Sans elle, un second vol
  // repartait de `scale(1.1)` en dur alors que la carte venait de se poser à sa
  // taille d'arrivée : elle regonflait d'un coup avant de repartir.
  return { noeud, depart, dx: 0, dy: 0, echelle: 1 };
}

/**
 * ATTRAPER UNE CARTE. On fabrique un fac-similé posé exactement sur l'original,
 * dans la couche de vol. C'est LUI qu'on promène ensuite : l'original reste dans
 * la main, où le moteur le réécrira, et rien n'est jamais coupé par le bord d'une
 * bande.
 *
 * @param {Element} source  la carte affichée qu'on attrape
 * @param {string}  motif   ce qui vole (voir `MOTIFS`) ; `pose` par défaut
 * @returns {{noeud: Element, depart: DOMRect}|null}
 */
export function attraper(source, motif = "pose") {
  const depart = boite(source);
  if (!depart) return null;
  // (cartes-qui-bougent) LA POSE DEPUIS LA MAIN SE SIGNALE ELLE-MÊME. Elle est
  // déjà mise en scène par `vue/geste.js`, qui n'appartient pas à ce chantier :
  // la mise en scène des évènements doit donc savoir qu'une carte vole DÉJÀ,
  // sinon elle en lancerait une seconde par-dessus. On reconnaît le geste à ce
  // qu'il attrape — une carte de la main, et rien d'autre ne porte cette classe.
  if (source.classList && source.classList.contains("carte--main")) {
    posesDeLaMain = performance.now();
  }
  const image = source.querySelector("img");
  return fabriquer({
    depart,
    image: image ? image.currentSrc || image.src : null,
    motif,
  });
}

/**
 * (cartes-qui-bougent) **FAIRE VOLER UNE MATIÈRE QUI N'EST PAS UNE CARTE** — un
 * jeton de chaleur qui monte à la jauge de température, une pièce qui quitte la
 * bourse, un « +2 » qui vient se poser sur une carte en jeu.
 *
 * Le mécanisme est le MÊME que celui d'une carte : même couche, même fabrique,
 * même `poserSur`. Ce qui change tient en deux points — l'objet est carré et de
 * taille choisie (une pièce n'a pas les proportions d'une carte), et son point
 * de départ est le CENTRE d'un élément plutôt que sa boîte entière : on part du
 * bac de mégacrédits, pas du rectangle de toute la barre d'équipage.
 *
 * @param {object}  o
 * @param {Element} o.depuis  d'où l'objet part (on prend le centre de sa boîte)
 * @param {Element} o.vers    où il arrive
 * @param {string}  o.src     l'image de l'objet, si elle existe
 * @param {string}  o.texte   à défaut d'image, le mot porté (« +2 »)
 * @param {string}  o.motif   la marque `data-vol`
 * @param {number}  o.cote    le côté de l'objet, en points
 */
export async function volerMatiere({
  depuis, vers, src = null, texte = "", motif = "jeton", cote = 44, ratio = 1,
  ms = 760, tour = 0, grossir = 1.3, cadrer = "jeton",
}) {
  const d = boite(depuis);
  const a = boite(vers);
  if (!d || !a) return;
  const haut = cote * ratio;
  const depart = {
    left: d.left + d.width / 2 - cote / 2,
    top: d.top + d.height / 2 - haut / 2,
    width: cote,
    height: haut,
  };
  const prise = fabriquer({
    depart, image: src, texte, motif,
    classe: src ? "vol__jeton" : "vol__jeton vol__jeton--mot",
  });
  try {
    await poserSur(prise, vers, { ms, tour, grossir, cadrer });
  } finally {
    relacher(prise);
  }
}

/** La carte suit la main : elle se tient un peu haut et un peu de travers. */
export function tenir(prise, dx, dy) {
  if (!prise) return;
  prise.dx = dx;
  prise.dy = dy;
  prise.echelle = 1.1;
  prise.noeud.style.transform =
    `translate(${dx}px, ${dy}px) scale(1.1) rotate(${Math.max(-8, Math.min(8, dx * 0.02))}deg)`;
}

/** On lâche la carte : le fac-similé disparaît, l'original reprend sa place. */
export function relacher(prise) {
  if (prise) prise.noeud.remove();
}

/**
 * POSER LA CARTE. Le fac-similé, d'où qu'il en soit de son voyage, se rend au
 * centre de `cible` : il grossit, il porte, puis il redescend à la taille de la
 * place qui l'attend. C'est cette troisième image qui fait la différence entre
 * poser et laisser tomber.
 *
 * @param {object}  prise   ce que `attraper` a rendu
 * @param {Element} cible   où l'on pose
 * @param {object}  o
 * @param {number}  o.ms      durée du voyage
 * @param {number}  o.tour    rotation finale, en degrés
 * @param {number}  o.grossir agrandissement au sommet du voyage
 */
export async function poserSur(prise, cible, { ms = 900, tour = 0, grossir = 1.22, cadrer = "boite" } = {}) {
  const arrivee = boite(cible);
  if (!prise || !arrivee) return;
  const { noeud, depart, dx: dx0, dy: dy0, echelle: e0 = 1.1 } = prise;

  const dx = arrivee.left + arrivee.width / 2 - (depart.left + depart.width / 2);
  const dy = arrivee.top + arrivee.height / 2 - (depart.top + depart.height / 2);
  // L'échelle d'arrivée : la carte prend la taille de la place qui l'attend.
  //
  // « boite » = la carte doit TENIR dans la cible (une grande zone d'accueil) :
  // on prend la plus petite des deux échelles. « place » = la carte doit
  // RECOUVRIR exactement la cible, qui est déjà une carte de même forme : on
  // cale sur sa largeur. Sans cette seconde façon, une carte qui vient se poser
  // sur son emplacement définitif s'y arrêtait plus petite que lui, et le
  // raccord se voyait.
  //
  // (cartes-qui-bougent) « jeton » = l'objet GARDE SA TAILLE. Un jeton de
  // chaleur qui monte à la jauge, une pièce qui quitte la bourse : ils arrivent
  // SUR une cible qui n'a pas leur forme (un mot de bandeau large et bas, une
  // carte en jeu haute et étroite). Les mettre à l'échelle de cette cible les
  // écraserait ou les ferait remplir tout l'écran.
  const echelle = cadrer === "place"
    ? arrivee.width / depart.width
    : cadrer === "jeton"
      ? 1
      : Math.min(arrivee.width / depart.width, arrivee.height / depart.height, 1.6);
  const fin = `translate(${dx}px, ${dy}px) scale(${echelle}) rotate(${tour}deg)`;

  // Le fac-similé se souvient d'où il est : un vol peut en suivre un autre.
  const retenir = () => { prise.dx = dx; prise.dy = dy; prise.echelle = echelle; };

  if (!actives) {
    noeud.style.transform = fin;
    retenir();
    return;
  }
  const trajet = noeud.animate(
    [
      {
        transform: `translate(${dx0}px, ${dy0}px) scale(${e0}) rotate(0deg)`,
        offset: 0,
      },
      {
        transform: `translate(${dx0 + (dx - dx0) * 0.34}px, ${dy0 + (dy - dy0) * 0.34 - 30}px) ` +
          `scale(${grossir}) rotate(${tour * 0.35}deg)`,
        offset: 0.36,
      },
      {
        transform: `translate(${dx}px, ${dy}px) scale(${echelle * 1.07}) rotate(${tour}deg)`,
        offset: 0.82,
      },
      { transform: fin, offset: 1 },
    ],
    { duration: duree(ms), easing: "cubic-bezier(.22,.68,.22,1)", fill: "forwards" }
  );
  try {
    await trajet.finished;
  } catch {
    // L'animation a été interrompue (page fermée, élément retiré) : le voyage
    // n'a plus d'objet, la réponse au moteur, si.
  }
  retenir();
}

/**
 * LE RACCORD. Le fac-similé vient de se poser EXACTEMENT sur la carte
 * définitive : il s'efface, et c'est elle qu'on voit dessous. Sans ce fondu, la
 * grande carte disparaissait d'un coup et la petite apparaissait ailleurs — le
 * défaut qu'Alexis a signalé le 04-08, « il manque l'animation qui dépose ces
 * cartes en suspension ».
 */
export async function fondre(prise, ms = 220) {
  if (!prise) return;
  if (!actives) return;
  const a = prise.noeud.animate(
    [{ opacity: 1 }, { opacity: 0 }],
    { duration: duree(ms), easing: "ease-out", fill: "forwards" }
  );
  try {
    await a.finished;
  } catch {
    // Interrompue : le fac-similé est retiré juste après, de toute façon.
  }
}

/**
 * ATTENDRE QU'UNE PLACE APPARAISSE à l'écran. Le moteur vient de recevoir la
 * réponse ; la carte posée n'entre dans le document qu'au redessin suivant.
 *
 * Rend l'élément, ou `null` au bout de `patience` — et `null` est un cas NORMAL,
 * pas une panne : une carte rouge à effet immédiat part à la défausse et n'a
 * aucune place sur le plateau. L'appelant fait alors simplement disparaître le
 * fac-similé sur place.
 */
export function attendrePlace(trouver, patience = 900) {
  return new Promise((resoudre) => {
    const fin = performance.now() + patience;
    const essai = () => {
      let el = null;
      try {
        el = trouver();
      } catch {
        el = null;
      }
      if (el && boite(el)) return resoudre(el);
      if (performance.now() >= fin) return resoudre(null);
      requestAnimationFrame(essai);
    };
    essai();
  });
}

/**
 * FAIRE VOYAGER UNE CARTE d'un bout à l'autre, sans qu'on l'ait tenue : la carte
 * Phase qu'on désigne d'un clic et qui s'en va se poser toute seule.
 */
export async function voler(source, cible, options = {}) {
  const prise = attraper(source, options.vol);
  if (!prise) return;
  try {
    await poserSur(prise, cible, options);
  } finally {
    relacher(prise);
  }
}

/**
 * FAIRE TOURNER UNE CARTE SUR PLACE — la carte de la manche précédente qu'on
 * couche sur le côté. On anime l'élément lui-même : c'est sa boîte qui doit
 * finir plus large que haute, et une carte couchée se reconnaît à ça.
 */
// ===========================================================================
// (cartes-qui-bougent) LES ÉVÉNEMENTS QUI SE VOIENT — ANI-6 puis ANI-1
// ===========================================================================
//
// LE DÉFAUT. « Les nombres changent et rien ne bouge » : une carte piochée
// apparaissait dans la main, une carte défaussée disparaissait, l'oxygène
// montait d'un cran — et le joueur ne voyait jamais que le RÉSULTAT, jamais le
// geste. Mesuré au scellement : 95 évènements sur 199 changeaient un nombre à
// l'écran sans que rien ne remue, l'oxygène et les jetons Forêt à 100 %.
//
// CE MODULE NE CONNAÎT AUCUNE RÈGLE, et ne devine aucun évènement. Il compare
// l'état que le moteur vient de rendre à celui d'avant, exactement comme
// `vue/monde.js` le fait déjà pour la planète, et met en scène l'écart qui a EU
// LIEU. Jamais un écart attendu, jamais un écart calculé.
//
// TROIS RÈGLES QUI NE SE NÉGOCIENT PAS :
//
//   1. RIEN NE FAIT ATTENDRE LE MOTEUR. Aucun de ces vols n'est attendu par
//      qui que ce soit : ils sont lancés et oubliés. `?animations=non` met
//      leurs durées à zéro, et les décisions comme les scores restent
//      identiques — c'est ce que le garde-fou compare.
//   2. RIEN NE BLOQUE UN CLIC. Tout vit dans la couche `#vol`, qui est en
//      `pointer-events: none`.
//   3. RIEN NE SE REJOUE PENDANT LE RATTRAPAGE. Après un rechargement, la page
//      repasse au moteur toutes les décisions déjà prises : ces évènements ont
//      déjà eu lieu, les remettre en scène annoncerait comme neuf ce qui est
//      vieux. On tient la mémoire à jour, et on se tait.
//
// DES DEUX CÔTÉS DE LA TABLE. Chaque évènement est mis en scène chez celui qui
// agit ET chez celui qui regarde : la pioche de l'adversaire vole vers son
// paquet de dos, sa dépense quitte sa bourse à lui.

import { imageCarte, dosProjet } from "./materiel.js";

const RATIO_CARTE = 569 / 409;

// Ce que l'écran a vu au rendu précédent. `null` = premier rendu de la partie :
// il n'y a pas d'avant, donc pas d'écart — sans cette garde, toute la mise en
// place se mettrait en scène d'un coup.
let vu = null;

// AU PLUS TROIS OBJETS PAR ÉVÉNEMENT. Une phase Recherche défausse parfois sept
// cartes d'un coup : sept fac-similés lancés ensemble donnent une bouillie que
// personne ne lit, et autant de nœuds à animer à chaque image. Trois disent
// « plusieurs cartes partent ».
const PAR_EVENEMENT = 3;

// LE PLAFOND EST PAR MOTIF, ET IL EST HAUT. Deux défauts mesurés par mon banc
// `verif/vols-et-paquets.py`, et corrigés ici :
//
//   · un plafond GLOBAL laissait les familles se voler la place — les six vols
//     de pioche des deux joueurs partaient d'abord, et il ne restait plus rien
//     pour les défausses. Une famille d'évènements ne doit jamais pouvoir en
//     éteindre une autre : le plafond est donc par motif ;
//   · un plafond BAS (quatre) laissait encore tomber des vols en rafale.
//     L'adversaire répond toutes les 180 ms et chaque réponse peut lancer trois
//     vols, alors qu'un vol dure une demi-seconde : trois ou quatre rendus se
//     chevauchent, et les derniers évènements passaient muets — 3 défausses et
//     2 pioches sur une partie, à des rangs qui changeaient d'une exécution à
//     l'autre, ce qui a mis le temps en cause plutôt que la géométrie.
//
// Le plafond ne sert plus qu'à empêcher une saturation franche : douze
// fac-similés d'une même famille en l'air, c'est déjà plus que ce qu'un écran
// montre. Il n'est jamais atteint dans une partie ordinaire.
const EN_VOL_MAX = 12;
const enVol = new Map();

/** Lance un vol sans jamais l'attendre, et sans jamais laisser l'écran saturer. */
function lancer(motif, faire) {
  const n = enVol.get(motif) || 0;
  if (n >= EN_VOL_MAX) return;
  enVol.set(motif, n + 1);
  Promise.resolve()
    .then(faire)
    .catch(() => {
      // Un vol interrompu (élément retiré, partie abandonnée) n'est pas une
      // panne : la mise en scène est un ornement, le jeu continue sans elle.
    })
    .finally(() => { enVol.set(motif, (enVol.get(motif) || 1) - 1); });
}

const trouver = (sel) => document.querySelector(sel);

/**
 * LE PREMIER DE CES ENDROITS QUI A VRAIMENT UNE BOÎTE.
 *
 * Un élément présent dans le document n'a pas forcément de surface : la rangée
 * de la main est haute de zéro quand elle est VIDE, et c'est précisément
 * l'instant où l'on défausse sa dernière carte. `boite` rend alors `null`, le vol
 * est abandonné, et l'évènement passe muet — mon banc `verif/vols-et-paquets.py`
 * en a compté 3 sur 49 défausses (graine 4242, rangs 30, 236, 256). On descend
 * donc une liste d'endroits, du plus précis au plus sûr : la rangée, la bande qui
 * la contient (elle, garde sa hauteur), puis la table.
 */
function premiereBoite(...selecteurs) {
  for (const s of selecteurs) {
    const e = trouver(s);
    if (e && boite(e)) return e;
  }
  return null;
}

/** La rangée de cartes du joueur `j`, vue depuis le siège regardé. */
function mainDe(j, siege) {
  return j === siege
    ? premiereBoite("#mienne-rang", "#main-mienne", "#milieu")
    : premiereBoite("#adverse-rang", "#main-adverse", "#milieu");
}

/** Le nombre de cartes réellement DESSINÉES dans la main du siège regardé. */
function cartesDessinees() {
  return document.querySelectorAll("[data-main-siege] [data-carte-cle]").length;
}

/** Ce qu'on retient d'un état, et rien d'autre. */
function relever(etat, siege) {
  const d = etat.defausse || [];
  return {
    mains: (etat.players || []).map((p) => (p.hand || []).length),
    dessinees: cartesDessinees(),
    defausse: (etat.decks && etat.decks.discard) || 0,
    tete: d.length ? String(d[0].id) : "",
    siege,
  };
}

/**
 * ANI-6 — LA PIOCHE ET LA DÉFAUSSE ONT UN CHEMIN.
 *
 * Le sens est fixé par le contrat : la pioche ARRIVE PAR LA DROITE — le dock des
 * paquets vit dans la colonne de droite (`vue/defausse.js`) — et la défausse s'en
 * va en sens inverse, de la main vers ce même dock. Les deux se voient des deux
 * côtés de la table.
 */
function piochesEtDefausses(avant, apres, etat) {
  const pioche = trouver("[data-pioche]");
  const pile = trouver("[data-defausse]");
  const siege = apres.siege;

  // LES PIOCHES. Au siège regardé, on compte les cartes DESSINÉES : `vue/mains.js`
  // met dans la main les cartes que la question nomme et que l'état ne montre
  // pas encore, et une carte qui PARAÎT doit se voir arriver, d'où qu'elle
  // vienne. En face, il n'y a que des dos, et leur nombre est exactement
  // `hand.length`.
  for (const j of [0, 1]) {
    const gagnees = j === siege
      ? apres.dessinees - avant.dessinees
      : (apres.mains[j] || 0) - (avant.mains[j] || 0);
    if (gagnees <= 0 || !pioche) continue;
    const cible = mainDe(j, siege);
    if (!cible) continue;
    for (let k = 0; k < Math.min(gagnees, PAR_EVENEMENT); k++) {
      lancer("pioche", () => volerMatiere({
        depuis: pioche, vers: cible, src: dosProjet(), motif: "pioche",
        cote: 52, ratio: RATIO_CARTE, cadrer: "boite",
        ms: 520 + k * 70, grossir: 1.15,
      }));
    }
  }

  // LES DÉFAUSSES. Le moteur publie la pile carte par carte, la plus récente en
  // tête (`observe.rs`, clef `defausse`) : les cartes qui viennent d'y entrer
  // sont les premières de cette liste, et ce sont elles qui volent, face
  // découverte — on doit VOIR ce qui part.
  const liste = etat.defausse || [];
  const entrees = apres.defausse - avant.defausse;
  const teteNeuve = apres.tete !== avant.tete && apres.tete !== "";
  if (!pile || (!teteNeuve && entrees <= 0)) return;
  const combien = Math.min(Math.max(entrees, teteNeuve ? 1 : 0), PAR_EVENEMENT);

  // D'OÙ ELLE PART. De la main qui vient de se vider, si l'on peut la nommer ;
  // sinon de la table, où se joue tout ce qui n'appartient à personne.
  let depuis = null;
  for (const j of [0, 1]) {
    if ((apres.mains[j] || 0) < (avant.mains[j] || 0)) depuis = mainDe(j, siege);
  }
  if (!depuis) depuis = premiereBoite("#milieu", "#scene");
  if (!depuis) return;

  for (let k = 0; k < combien; k++) {
    const c = liste[k];
    if (!c) break;
    lancer("defausse", () => volerMatiere({
      depuis, vers: pile, src: imageCarte(c.name) || dosProjet(), motif: "defausse",
      cote: 58, ratio: RATIO_CARTE, cadrer: "boite",
      ms: 540 + k * 70, tour: 8, grossir: 1.2,
    }));
  }
}

/**
 * LA MISE EN SCÈNE D'UN INSTANT. Appelée à chaque rendu, depuis `vue/table.js`
 * — le dernier module de rendu de ce chantier, appelé APRÈS `majMains` et
 * `majPlateaux` : les places d'arrivée existent déjà dans le document.
 *
 * @param {object} etat   l'état rendu par le moteur
 * @param {number} siege  le joueur assis en bas de l'écran
 */
export function mettreEnScene(etat, siege) {
  if (!etat || !etat.players) return;
  const apres = relever(etat, siege);
  const avant = vu;
  vu = apres;
  // Le rattrapage rejoue une partie déjà jouée : la mémoire se tient à jour, la
  // scène se tait. Un changement de siège regardé n'est pas un évènement non
  // plus — c'est le même instant, vu d'ailleurs.
  if (rattrapage || !avant || avant.siege !== siege) return;
  piochesEtDefausses(avant, apres, etat);
}

/** Remet la mémoire à zéro (nouvelle partie, table vidée). */
export function oublierMiseEnScene() {
  vu = null;
}

export async function coucher(el, ms = 700) {
  if (!el || !actives) return;
  const a = el.animate(
    [
      { transform: "rotate(0deg) scale(1)" },
      { transform: "rotate(52deg) scale(1.1)", offset: 0.45 },
      { transform: "rotate(90deg) scale(1)" },
    ],
    { duration: duree(ms), easing: "cubic-bezier(.3,.7,.25,1)" }
  );
  try {
    await a.finished;
  } catch {
    // Interrompue : la carte est de toute façon posée par la feuille de style.
  }
}

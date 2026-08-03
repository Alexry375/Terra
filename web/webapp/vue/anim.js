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

/**
 * L'unique écriture du réglage des animations, quel qu'en soit le chemin :
 * `?animations=non` (lu par `interface.js`) ou l'interrupteur du panneau
 * d'options. Les deux attributs sont posés ensemble — jamais l'un sans l'autre.
 */
export function reglerAnimations(oui) {
  actives = !!oui;
  document.body.dataset.animations = oui ? "oui" : "non";
  // La règle de `style-menu.css` porte sur la racine, celle de `style-table.css`
  // sur le corps : les deux doivent voir la même chose.
  if (oui) delete document.documentElement.dataset.animations;
  else document.documentElement.dataset.animations = "non";
}

export function animationsActives() {
  return actives;
}

/** La durée réellement appliquée : celle demandée, ou zéro. */
export function duree(ms) {
  return actives ? ms : 0;
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
 * ATTRAPER UNE CARTE. On fabrique un fac-similé posé exactement sur l'original,
 * dans la couche de vol. C'est LUI qu'on promène ensuite : l'original reste dans
 * la main, où le moteur le réécrira, et rien n'est jamais coupé par le bord d'une
 * bande.
 *
 * @param {Element} source  la carte affichée qu'on attrape
 * @returns {{noeud: Element, depart: DOMRect}|null}
 */
export function attraper(source) {
  const depart = boite(source);
  if (!depart) return null;
  const image = source.querySelector("img");
  const noeud = document.createElement("div");
  noeud.className = "vol__carte";
  noeud.style.left = depart.left + "px";
  noeud.style.top = depart.top + "px";
  noeud.style.width = depart.width + "px";
  noeud.style.height = depart.height + "px";
  if (image) {
    const im = document.createElement("img");
    im.src = image.currentSrc || image.src;
    im.alt = "";
    im.draggable = false;
    noeud.appendChild(im);
  }
  couche().appendChild(noeud);
  return { noeud, depart, dx: 0, dy: 0 };
}

/** La carte suit la main : elle se tient un peu haut et un peu de travers. */
export function tenir(prise, dx, dy) {
  if (!prise) return;
  prise.dx = dx;
  prise.dy = dy;
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
export async function poserSur(prise, cible, { ms = 900, tour = 0, grossir = 1.22 } = {}) {
  const arrivee = boite(cible);
  if (!prise || !arrivee) return;
  const { noeud, depart, dx: dx0, dy: dy0 } = prise;

  const dx = arrivee.left + arrivee.width / 2 - (depart.left + depart.width / 2);
  const dy = arrivee.top + arrivee.height / 2 - (depart.top + depart.height / 2);
  // L'échelle d'arrivée : la carte prend la taille de la place qui l'attend.
  const echelle = Math.min(arrivee.width / depart.width, arrivee.height / depart.height, 1.6);
  const fin = `translate(${dx}px, ${dy}px) scale(${echelle}) rotate(${tour}deg)`;

  if (!actives) {
    noeud.style.transform = fin;
    return;
  }
  const trajet = noeud.animate(
    [
      {
        transform: `translate(${dx0}px, ${dy0}px) scale(1.1) rotate(0deg)`,
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
}

/**
 * FAIRE VOYAGER UNE CARTE d'un bout à l'autre, sans qu'on l'ait tenue : la carte
 * Phase qu'on désigne d'un clic et qui s'en va se poser toute seule.
 */
export async function voler(source, cible, options = {}) {
  const prise = attraper(source);
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

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
  // `echelle` suit la taille COURANTE du fac-similé. Sans elle, un second vol
  // repartait de `scale(1.1)` en dur alors que la carte venait de se poser à sa
  // taille d'arrivée : elle regonflait d'un coup avant de repartir.
  return { noeud, depart, dx: 0, dy: 0, echelle: 1 };
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
  const echelle = cadrer === "place"
    ? arrivee.width / depart.width
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

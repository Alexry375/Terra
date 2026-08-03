// LE CADRE DE JEU — ma main en bas, celle de l'adversaire en haut, retournée.
//
// C'est le seul point de vue que l'écran connaît : celui du SIÈGE regardé
// (`?siege=`). Ce siège n'est pas « le joueur qui décide » — un programme peut
// le tenir pendant qu'on le regarde jouer (`?decide=programme`).
//
// MA MAIN EST MA MAIN. On ne répond plus à la question « quelle carte joues-tu ? »
// en pointant une liste au milieu de l'écran : on attrape la carte, là où elle
// est, et on la pose sur la table. C'est pourquoi `data-choix` — l'indice que le
// moteur attend — est porté ICI, par la carte elle-même, et plus par une vignette
// fabriquée ailleurs. Le geste est branché par `vue/geste.js`.
//
// CE QUI EST CACHÉ N'EST PAS DANS LA PAGE. La zone adverse ne reçoit aucun nom
// de carte, aucun identifiant, aucune image de face : uniquement des dos et un
// NOMBRE. Rendre une carte transparente ou la pousser hors de l'écran ne serait
// pas la cacher — il suffirait d'ouvrir les outils du navigateur pour la lire.
//
// CE QUI EST JOUABLE. Une carte de ma main porte `data-jouable="oui"` si et
// seulement si le moteur vient de l'énumérer parmi les options de la décision en
// cours ET que cette décision est celle de mon siège. Recopie d'identifiants,
// pas jugement : la page ne sait pas ce que coûte une carte.

import { carte, cle, normaliser } from "./cartes.js";
import { dosProjet, nomJoueur } from "./materiel.js";
import { survolable, survolableImage } from "./loupe.js";
import { armerCarte } from "./geste.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";

// La largeur d'une carte en main. Elle est TENUE : sous 90 px le coût imprimé
// n'est plus lisible (mesuré sur les images du jeu). Quand la main s'allonge,
// les cartes se recouvrent — elles ne rétrécissent pas.
export const LARGEUR = 116;
const ECART = 10;
// DEUX CARTES VOISINES NE SE RECOUVRENT JAMAIS DE PLUS DE LA MOITIÉ. La borne
// n'est pas décorative : le clic comme la prise au pointeur visent le CENTRE de
// la carte. Passé 50 % de recouvrement, ce centre appartient à la carte voisine —
// on croit attraper une carte et l'on en attrape une autre. 44 % laisse la marge ;
// à quinze cartes (la main la plus longue mesurée sur dix parties) le
// recouvrement réel n'est que de 27 %.
const SERRAGE_MAX = 0.44;

// Les dos de l'adversaire : plus petits, ce sont des objets sans rien à lire.
const LARGEUR_DOS = 62;
const ECART_DOS = 6;

export function construireMains() {
  const adverse = document.createElement("aside");
  adverse.className = "main main--adverse";
  adverse.id = "main-adverse";
  adverse.dataset.main = "adverse";
  adverse.dataset.mainAdverse = "";
  adverse.dataset.cartes = "0";
  adverse.innerHTML =
    `<div class="main__tete">` +
    `<span class="main__mot" id="adverse-mot"></span>` +
    `<span class="main__agit" id="adverse-agit"></span>` +
    `</div>` +
    `<div class="main__rang" id="adverse-rang"></div>`;
  document.body.appendChild(adverse);

  const mienne = document.createElement("aside");
  mienne.className = "main main--mienne";
  mienne.id = "main-mienne";
  mienne.dataset.main = "mienne";
  // La zone qui contient la main du siège regardé. C'est elle qu'on interroge
  // pour savoir si une carte à jouer est bien DANS la main.
  mienne.dataset.mainSiege = "";
  mienne.innerHTML =
    `<div class="main__tete"><span class="main__mot" id="mienne-mot"></span>` +
    `<span class="main__geste" id="mienne-geste"></span></div>` +
    `<div class="main__rang" id="mienne-rang"></div>`;
  document.body.appendChild(mienne);
}

// Le plan de la décision en cours, quand elle se joue DEPUIS LA MAIN :
// { rang, indices: Map(clé de carte -> indice de réponse) }. La scène le lit pour
// savoir qu'elle n'a plus à dessiner ces options-là.
//
// LA CLÉ, ET NON LE NUMÉRO. Voir `cartes.js` : un numéro seul ne désigne pas une
// carte, puisque les projets et les corporations sont numérotés séparément.
let plan = null;

/**
 * LA DÉCISION SE JOUE-T-ELLE DEPUIS LA MAIN ? Règle calculée, pas liste écrite
 * d'avance : une décision est « de main » quand elle est de forme simple et que
 * TOUTES ses options-cartes désignent une carte projet que je tiens. C'est vrai
 * de « quelle carte joues-tu » et de la vente d'une carte ; c'est faux quand les
 * options sont des cartes déjà POSÉES sur un plateau (prendre ou poser une
 * ressource, rejouer une production), qui ne sont pas dans ma main et qu'on ne
 * peut donc pas en sortir.
 *
 * ET C'EST FAUX DES CORPORATIONS. Elles se choisissent au milieu de l'écran, en
 * grand, d'un clic — comme les cartes Phase, et pour la même raison : ce ne sont
 * pas des cartes qu'on joue depuis sa main, ce sont deux cartes qu'on nous
 * présente et dont on en garde une. Une carte de sorte « corporation » suffit
 * donc à écarter la décision de la main, quoi qu'en dise le reste.
 *
 * Calculée plutôt qu'énumérée : si le moteur ajoute demain une question sur les
 * cartes de la main, elle passera par la main sans qu'on ait à y penser.
 */
function planDeLaDecision(d, clesEnMain) {
  if (!d || d.multiple || d.montant) return null;
  const options = d.options || [];
  const indices = new Map();
  let cartes = 0;
  for (let i = 0; i < options.length; i++) {
    const c = normaliser(options[i]);
    if (!c || c.id === null || c.id === undefined) continue;
    if (c.sorte !== "projet") return null; // une corporation ne se joue pas d'ici
    cartes += 1;
    indices.set(cle(options[i]), i);
  }
  // TOUTES les options, ou aucune. Une décision qui mêlerait des cartes de la
  // main et des options d'une autre nature ferait disparaître ces dernières de
  // l'écran — on ne peut pas choisir ce qu'on ne voit pas. Le moteur n'en produit
  // pas aujourd'hui (mesuré sur 40 graines) ; le jour où il en produira une, elle
  // restera au milieu, entière.
  if (!cartes || cartes !== options.length || indices.size !== cartes) return null;
  for (const k of indices.keys()) {
    if (!clesEnMain.has(k)) return null; // une option n'est pas dans ma main
  }
  return { rang: d.rang, indices };
}

/** La décision `d` est-elle celle que la main tient en ce moment ? */
export function decisionDeMain(d) {
  return !!plan && !!d && plan.rang === d.rang;
}

/**
 * Réécrit les deux mains DEPUIS LE SIÈGE REGARDÉ.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 * @param {number} siege     le joueur assis en bas de l'écran
 */
export function majMains(etat, decision, siege) {
  const moi = etat.players[siege];
  const lui = etat.players[1 - siege];
  if (!moi || !lui) return;

  // Les identifiants que le moteur vient d'énumérer POUR MON SIÈGE : une
  // décision de l'adversaire ne rend rien de ma main jouable.
  const active = !!decision && decision.joueur === siege;
  const proposees = new Set();
  if (active) {
    for (const o of decision.options || []) {
      const k = cle(o);
      if (k) proposees.add(k);
    }
  }

  const cartes = cartesEnMain(etat, moi, decision, active);
  const clesEnMain = new Set(cartes.map((c) => cle(c)).filter(Boolean));
  plan = active ? planDeLaDecision(decision, clesEnMain) : null;

  maMain(siege, cartes, proposees, active);
  mainAdverse(1 - siege, lui.hand.length);
}

/**
 * CE QUE JE TIENS RÉELLEMENT. L'état rend `hand` — les cartes projet, et elles
 * seules.
 *
 * LES CORPORATIONS N'ENTRENT PLUS ICI. Elles y entraient jusqu'au 02-08, et
 * c'était une faute : les deux cartes Corporation de la mise en place ne se
 * jouent pas depuis la main, elles se présentent au milieu de l'écran et on en
 * désigne une, comme une carte Phase. Les glisser dans la main coûtait deux
 * choses : le geste de les poser, qui n'a pas de sens pour elles, et surtout la
 * confusion des numéros — une corporation portant le même numéro qu'une carte
 * projet tenue disparaissait de l'écran, et la carte projet héritait de sa
 * réponse (3 parties sur 70, mesuré). Le choix vit désormais dans `vue/scene.js`,
 * et cette fonction ne connaît plus que des cartes projet.
 *
 * ET L'ÉTAT PEUT AVOIR UNE MANCHE DE RETARD SUR LA QUESTION. Le pont ne relève
 * `state_view` qu'aux points d'observation du moteur (`wasm/src/lib.rs`,
 * `Policy::observe`), et plusieurs questions se posent APRÈS, dans la même action.
 * La vente d'une carte en est le cas net : `flow.rs:3930` passe la main RÉELLE
 * (`game.players[p].hand.clone()`) alors que `etat.hand` date d'avant l'action.
 * Mesuré sur dix parties : 342 ventes, 342 fois `etat.hand` incomplet, et pas une
 * seule fois l'inverse — jusqu'à une main annoncée VIDE pour une carte bel et bien
 * proposée (graine 777, rang 24). Le descripteur est donc la source la plus
 * fraîche, et c'est lui qu'on suit : les cartes qu'il nomme et que l'état ne
 * montre pas encore rejoignent la main. Sans cela, le moteur proposerait de
 * vendre des cartes qui ne sont nulle part à l'écran.
 */
function cartesEnMain(etat, p, decision, active) {
  const cartes = [...p.hand];
  const vues = new Set(cartes.map((c) => cle(c)));

  // Les cartes que la question NOMME et que l'état ne montre pas encore. On
  // écarte celles qui sont POSÉES sur un plateau : celles-là ne sont plus dans
  // une main, et les décisions qui les désignent (prendre ou poser une ressource,
  // rejouer une production) parlent bien de la table, pas de la main.
  //
  // Et l'on n'accueille QUE des cartes projet : une corporation qui arriverait
  // ici serait ce qu'on vient d'écarter.
  if (active && decision && !decision.multiple && !decision.montant) {
    const posees = new Set();
    for (const j of etat.players || []) {
      for (const c of j.played || []) posees.add(cle(c));
    }
    for (const o of decision.options || []) {
      const c = normaliser(o);
      if (!c || c.id === null || c.id === undefined) continue;
      if (c.sorte !== "projet") continue;
      const k = cle(o);
      if (vues.has(k) || posees.has(k)) continue;
      vues.add(k);
      cartes.push(c);
    }
  }
  return cartes;
}

/** Ma main, en bas, en clair — et c'est d'ici qu'on joue. */
function maMain(j, cartes, proposees, active) {
  const z = ref("#mienne-rang");
  if (!z) return;
  ref("#mienne-mot").textContent = `${MOT.hand} · ${nomJoueur(j)} · ${cartes.length}`;
  // Le mode d'emploi du geste, et seulement quand il y a un geste à faire.
  ref("#mienne-geste").textContent = plan ? MOT.dragHint : "";

  // ON NE REFAIT LA MAIN QUE SI LES CARTES CHANGENT. Ce qui bouge à CHAQUE
  // décision, c'est la marque : quelle carte est jouable, et quel indice elle
  // rend. Reconstruire les quinze cartes pour cela seul, c'est arracher du
  // document, plusieurs centaines de fois par partie, des images que quelqu'un
  // est peut-être en train de lire — un survol qui n'aboutit pas pour une main,
  // une erreur franche pour une machine qui pilote la page. Les marques se
  // posent donc SUR les cartes déjà là.
  // La signature se fait sur les CLÉS et non sur les numéros : deux mains
  // différentes ne doivent jamais produire la même signature, sans quoi la
  // seconde ne serait pas redessinée. Les numéros seuls ne le garantissent pas.
  const structure = j + "#" + cartes.map((c) => cle(c)).join("|");
  if (z.dataset.signature !== structure) {
    z.dataset.signature = structure;
    z.textContent = "";
    for (const c of cartes) {
      const f = carte(c, { classe: "carte--main" });
      f.dataset.carteId = String(c.id);
      // La CLÉ à côté du numéro : c'est elle qui désigne la carte sans
      // ambiguïté, le numéro seul reste pour les contrôles qui le lisent depuis
      // toujours. Toute carte de la main est un projet, mais on l'écrit plutôt
      // que de le supposer.
      f.dataset.carteCle = cle(c) || "";
      survolable(f, c);
      armerCarte(f);
      z.appendChild(f);
    }
  }
  for (const f of z.children) {
    const k = f.dataset.carteCle;
    if (active) f.dataset.jouable = proposees.has(k) ? "oui" : "non";
    else delete f.dataset.jouable;
    // L'INDICE DE LA RÉPONSE, PORTÉ PAR LA CARTE. C'est le moteur qui vient de
    // l'énumérer ; la page ne fait que le recopier sur l'objet qu'on touche.
    const indice = plan ? plan.indices.get(k) : undefined;
    if (indice === undefined) delete f.dataset.choix;
    else f.dataset.choix = String(indice);
    // Une carte qui n'est plus en vol reprend sa place pleine.
    delete f.dataset.enMain;
  }
  serrer(z, cartes.length, largeurReelle(z), ECART, SERRAGE_MAX);
}

/**
 * LA LARGEUR QU'UNE CARTE OCCUPE VRAIMENT. Elle suit la hauteur de la fenêtre
 * (`style-table.css`) pour que la main ne soit jamais rognée : la calculer avec
 * une constante périmée ferait recouvrir les cartes de plus que la moitié de leur
 * largeur réelle — c'est-à-dire cacher leur centre, celui qu'on vise pour les
 * attraper.
 */
function largeurReelle(z) {
  const im = z.querySelector(".carte--main img");
  const l = im ? im.getBoundingClientRect().width : 0;
  return l > 1 ? l : LARGEUR;
}

/**
 * La main de l'adversaire, en haut, retournée. La seule chose qui en sorte est
 * son NOMBRE de cartes — la seule information publique d'une main tenue.
 *
 * Ces dos sont ceux des cartes PROJET : c'est ce que l'adversaire tient. Servir
 * le dos des corporations annoncerait des corporations qu'il n'a pas.
 */
function mainAdverse(j, combien) {
  const zone = ref("#main-adverse");
  const z = ref("#adverse-rang");
  if (!zone || !z) return;
  zone.dataset.cartes = String(combien);
  ref("#adverse-mot").textContent =
    `${MOT.opponent} · ${nomJoueur(j)} · ${combien} ${combien === 1 ? MOT.oneCard : MOT.manyCards}`;

  if (z.dataset.combien !== String(combien)) {
    z.dataset.combien = String(combien);
    z.textContent = "";
    const dos = dosProjet();
    for (let i = 0; i < combien; i++) {
      const f = document.createElement("figure");
      f.className = "carte carte--dos carte--adverse";
      const im = document.createElement("img");
      im.src = dos;
      // Aucun nom ici : le texte de remplacement d'un dos ne dit que « dos ».
      im.alt = MOT.faceDown;
      im.draggable = false;
      f.appendChild(im);
      // Agrandir un dos ne montre qu'un dos : rien n'en sort, et l'écran tient sa
      // promesse — toute carte survolée s'agrandit.
      survolableImage(f, dos, MOT.faceDown);
      z.appendChild(f);
    }
  }
  serrer(z, combien, LARGEUR_DOS, ECART_DOS, SERRAGE_MAX);
}

/**
 * LES CARTES SE RECOUVRENT, ELLES NE RÉTRÉCISSENT PAS. Une main de quinze
 * cartes ne tient pas côte à côte dans la largeur de l'écran ; les réduire
 * rendrait le prix illisible, donc le jeu injouable. On calcule le recouvrement
 * nécessaire, borné pour que le centre de chaque carte reste découvert — c'est
 * lui qu'on vise pour l'attraper.
 */
function serrer(z, n, largeur, ecart, maximum) {
  if (n <= 1) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const dispo = z.clientWidth || 1200;
  const naturel = n * largeur + (n - 1) * ecart;
  if (naturel <= dispo) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const chevauchement = Math.min(largeur * maximum, (naturel - dispo) / (n - 1) + ecart);
  z.style.setProperty("--serrage", Math.round(-chevauchement) + "px");
}

/**
 * Le recouvrement est calculé en PIXELS, à partir de la largeur disponible : il
 * ne veut plus rien dire dès que la fenêtre change de taille. On le reprend
 * alors, sans réécrire les cartes.
 */
export function replacerMains() {
  const m = ref("#mienne-rang");
  if (m) serrer(m, m.childElementCount, largeurReelle(m), ECART, SERRAGE_MAX);
  const a = ref("#adverse-rang");
  if (a) serrer(a, a.childElementCount, LARGEUR_DOS, ECART_DOS, SERRAGE_MAX);
}

/**
 * L'ADVERSAIRE AGIT — on voit QU'IL agit, jamais QUOI.
 *
 * @param {string|null} quoi  ce qu'il est en train de faire, en anglais court ;
 *                            `null` éteint l'état.
 */
export function adversaireAgit(quoi) {
  const zone = ref("#main-adverse");
  const mot = ref("#adverse-agit");
  if (!zone || !mot) return;
  if (quoi) {
    zone.dataset.agit = "oui";
    mot.textContent = quoi;
  } else {
    delete zone.dataset.agit;
    mot.textContent = "";
  }
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierMains() {
  plan = null;
  for (const s of ["#mienne-rang", "#adverse-rang"]) {
    const z = ref(s);
    if (!z) continue;
    delete z.dataset.signature;
    delete z.dataset.combien;
    z.textContent = "";
  }
  adversaireAgit(null);
}

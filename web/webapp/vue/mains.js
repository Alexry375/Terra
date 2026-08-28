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
// DEUX FAITS, DEUX ATTRIBUTS — et il ne faut surtout pas les confondre.
//
// (LIS-14) `data-jouable` a voulu dire deux choses successivement, et la
// deuxième était une faute de SENS, pas de calcul. Le chantier
// « regles-de-la-vente » lui a fait dire « j'ai de quoi la payer »
// (`players[].main_payable`, rendu par `flow::main_payable`). Or `main_payable`
// répond à « ai-je de quoi PAYER cette carte ? » — sans la couleur autorisée par
// la phase, sans les permissions, sans les prérequis (`engine/src/flow.rs`,
// lignes 2104-2121). En faire un « je peux JOUER cette carte », c'est promettre
// autre chose que ce que le moteur tient : mesuré par `verif/jouable.py`, des
// cartes marquées jouables que le moteur n'offre pas — huit d'un coup à la
// décision 4 d'une partie. Un contour vert trompeur ne se remarque qu'en
// essayant de jouer la carte ; personne ne le signale, et il ment quand même.
//
// Chaque attribut a donc UNE définition, et une seule :
//
//   · `data-jouable`  — « le moteur vient d'énumérer cette carte parmi les
//     options de la décision en cours, POUR MON SIÈGE ». C'est la seule liste
//     qui dit ce qui se joue à l'instant, et c'est le moteur qui l'écrit.
//   · `data-payable`  — « le moteur dit que j'ai de quoi la payer »
//     (`main_payable`). Utile à tout instant de la partie, y compris hors d'une
//     question : main de onze cartes, 26 MC en poche, on veut savoir ce qu'on
//     peut s'offrir. Il grandit quand on vend, ce qui est la propriété qu'on
//     voulait voir.
//
// CE QUE LE CONTOUR VERT MONTRE, feuille de style à l'appui (`style.css`) :
// pendant une question qui se joue DEPUIS LA MAIN, il suit `data-jouable` et
// désigne donc exactement les cartes que le moteur offre ; le reste du temps il
// suit `data-payable` et dit ce qu'on a les moyens de payer. L'information de la
// bourse ne disparaît pas — elle cesse seulement de parler par-dessus la seule
// question posée. Le rang porte `data-pose="oui|non"` pour que la feuille sache
// laquelle des deux règles s'applique.
//
// CE QUI EST JOUABLE MAINTENANT reste aussi dit par `data-choix`, l'indice que
// le moteur attend, porté par la carte — c'est lui que le geste recopie.
//
// Recopie, pas jugement, dans les trois cas : la page ne sait toujours pas ce
// qu'une carte coûte.

import { carte, cle, normaliser } from "./cartes.js";
import { dosProjet, nomJoueur } from "./materiel.js";
import { survolable } from "./loupe.js";
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
    // (les-ecrans-manquants) L'annonce d'une vente d'en face. Vide au repos, et
    // vide veut dire SANS BOÎTE : un mot laissé transparent serait un mot qu'un
    // banc trouve et qu'une main ne voit pas.
    `<span class="main__vendu" id="adverse-vente"></span>` +
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

  // CNF-1 — `vue/geste.js` déplace la carte dans le document au fil du
  // glissement, puis annonce que la rangée a changé. On relit alors la rangée
  // telle qu'elle est : c'est le document qui fait foi, et l'ordre retenu
  // survit ainsi au prochain rendu, quand une pioche ou une pose reconstruira
  // la main. On écoute une fois pour toutes, la rangée n'étant jamais recréée.
  const rangee = mienne.querySelector("#mienne-rang");
  rangee.addEventListener("main-triee", () => retenirOrdre(rangee));
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

  // CE QUE JE PEUX PAYER, carte par carte, tel que le
  // moteur le rend (`main_payable`, dans l'ordre de `hand`). Les cartes que la
  // question NOMME sans que l'état ne les montre encore (voir `cartesEnMain`)
  // n'y figurent pas : le moteur vient de les proposer, elles sont donc
  // payables, et c'est `proposees` qui répond pour elles.
  const payables = new Set();
  const dits = moi.main_payable || [];
  moi.hand.forEach((c, i) => {
    const k = cle(c);
    if (k && dits[i]) payables.add(k);
  });

  maMain(siege, cartes, proposees, payables);
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

/**
 * CNF-1 — L'ORDRE QUE LE JOUEUR S'EST DONNÉ, carte par carte.
 *
 * Le moteur publie la main dans SON ordre, et il a raison de le faire : c'est
 * son état. Le rangement, lui, est une affaire d'écran — il ne remonte jamais
 * au moteur et ne change aucune réponse. La réponse rendue est l'indice que le
 * moteur a énuméré, retrouvé PAR LA CLÉ de la carte (`plan.indices`), jamais
 * par sa place dans la rangée : trier sa main ne peut donc pas jouer une autre
 * carte que celle qu'on touche.
 *
 * Clé de carte -> rang voulu. Une carte inconnue (celle qu'on vient de piocher)
 * n'a pas de rang : elle prend la fin, dans l'ordre du moteur.
 */
const ORDRE = new Map();

/** Relit la rangée telle qu'elle est, et retient cet ordre-là. */
function retenirOrdre(z) {
  ORDRE.clear();
  [...z.children].forEach((f, i) => {
    if (f.dataset.carteCle) ORDRE.set(f.dataset.carteCle, i);
  });
}

/** Range les cartes du moteur selon l'ordre voulu, sans en perdre aucune. */
function selonMonOrdre(cartes) {
  if (ORDRE.size === 0) return cartes;
  const rang = (c) => {
    const r = ORDRE.get(cle(c));
    return r === undefined ? Number.MAX_SAFE_INTEGER : r;
  };
  // Tri STABLE (celui de `Array.prototype.sort` l'est) : deux cartes sans rang
  // voulu restent dans l'ordre où le moteur les a données.
  return [...cartes].sort((a, b) => rang(a) - rang(b));
}

/** Ma main, en bas, en clair — et c'est d'ici qu'on joue. */
function maMain(j, cartes, proposees, payables) {
  const z = ref("#mienne-rang");
  if (!z) return;
  cartes = selonMonOrdre(cartes);
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
  // LA QUESTION SE JOUE-T-ELLE DEPUIS LA MAIN ? C'est ce qui décide lequel des
  // deux faits le contour vert montre (`style.css`). On l'écrit sur le rang, et
  // toujours — « non » est une réponse, l'absence n'en est pas une.
  z.dataset.pose = plan ? "oui" : "non";

  for (const f of z.children) {
    const k = f.dataset.carteCle;
    // (LIS-14) CE QUE LE MOTEUR OFFRE, ET RIEN D'AUTRE. `proposees` est
    // l'énumération de la décision en cours pour MON siège — pas un calcul de la
    // page, pas une liste que la page tiendrait elle-même. Hors décision, ou
    // pendant celle de l'adversaire, elle est vide : aucune carte de ma main
    // n'est jouable à cet instant, et l'écran le dit.
    f.dataset.jouable = proposees.has(k) ? "oui" : "non";
    // CE QUE J'AI LES MOYENS DE PAYER, à tout instant de la partie. Une carte
    // que la question nomme et que l'état ne montre pas encore est payable par
    // construction : le moteur vient de la proposer.
    f.dataset.payable = (payables.has(k) || proposees.has(k)) ? "oui" : "non";
    // L'INDICE DE LA RÉPONSE, PORTÉ PAR LA CARTE. C'est le moteur qui vient de
    // l'énumérer ; la page ne fait que le recopier sur l'objet qu'on touche.
    const indice = plan ? plan.indices.get(k) : undefined;
    if (indice === undefined) delete f.dataset.choix;
    else f.dataset.choix = String(indice);
    // Une carte qui n'est plus en vol reprend sa place pleine.
    delete f.dataset.enMain;
  }
  serrer(z, cartes.length, ECART, SERRAGE_MAX, LARGEUR);
}

/**
 * LA LARGEUR QU'UNE CARTE OCCUPE VRAIMENT. Elle suit la hauteur de la fenêtre
 * (`style-table.css`) pour que la main ne soit jamais rognée : la calculer avec
 * une constante périmée ferait recouvrir les cartes de plus que la moitié de leur
 * largeur réelle — c'est-à-dire cacher leur centre, celui qu'on vise pour les
 * attraper.
 *
 * C'est la largeur NATURELLE qu'on lit — celle que la feuille de style donne —
 * et jamais celle d'un rétrécissement précédent : `serrer` efface son propre
 * réglage avant de mesurer, sinon la main fondrait un peu plus à chaque rendu.
 */
function largeurReelle(z, defaut) {
  const im = z.querySelector("img");
  const l = im ? im.getBoundingClientRect().width : 0;
  return l > 1 ? l : defaut;
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
      // LIS-13 (Alexis, 04-08) — PAS DE LOUPE SUR CES DOS. Ils sont tous
      // identiques et ne montrent rien : les agrandir n'apprend rien à
      // personne, et la loupe qui s'ouvrait à chaque passage de souris sur la
      // main d'en face gênait la lecture du reste. La promesse « toute carte
      // s'agrandit » vaut pour les cartes qui ont quelque chose à montrer.
      z.appendChild(f);
    }
  }
  serrer(z, combien, ECART_DOS, SERRAGE_MAX, LARGEUR_DOS);
}

/**
 * LES CARTES SE RECOUVRENT, ELLES NE RÉTRÉCISSENT PAS — tant que la place le
 * permet. Une main de quinze cartes ne tient pas côte à côte dans la largeur de
 * l'écran ; les réduire rend le prix illisible. On calcule donc le recouvrement
 * nécessaire, borné pour que le centre de chaque carte reste découvert : c'est
 * lui qu'on vise pour l'attraper, et c'est lui que le contrat appelle
 * « cliquable ».
 *
 * (round 2) LA BORNE NE SUFFISAIT PAS, PARCE QU'ELLE POUVAIT NE PAS TENIR. Le
 * recouvrement était plafonné à `maximum` mais rien ne garantissait que n cartes
 * ainsi serrées entrent dans la largeur disponible : au-delà, le rang débordait
 * de sa zone, et les cartes de gauche s'en allaient sous ce qui vit dans le coin
 * — le panneau de vente. Cette fonction rend maintenant une garantie COMPLÈTE,
 * pour toute longueur de main et toute taille de fenêtre :
 *
 *   1. deux voisines ne se recouvrent jamais de plus de `maximum` (< 50 %), donc
 *      chaque carte avance d'au moins (1 − `maximum`) de sa largeur et son
 *      centre reste devant sa voisine ;
 *   2. les n cartes ainsi posées TIENNENT dans `dispo`. Quand la largeur
 *      naturelle ne le permet pas, c'est elle qui cède — la carte rétrécit,
 *      jusqu'à ce que le compte tombe juste.
 *
 * L'ordre des deux est délibéré : une carte trop petite se lit mal, une carte
 * qu'on ne peut pas désigner ne se joue pas du tout. La lisibilité cède la
 * première. Mesuré : à 1280×720, une fois les deux gouttières réservées
 * (`style-vente.css`), il reste 784 px pour des cartes de 92 — le rétrécissement
 * ne commence donc qu'à QUINZE cartes, et les treize de la partie la plus longue
 * mesurée gardent leurs 92 px.
 *
 * @param {Element} z        le rang de cartes
 * @param {number} n         combien de cartes
 * @param {number} ecart     l'écart entre deux cartes quand la place abonde
 * @param {number} maximum   le recouvrement maximal, en part de la largeur
 * @param {number} defaut    la largeur à supposer si aucune image n'est encore là
 */
function serrer(z, n, ecart, maximum, defaut) {
  // On repart TOUJOURS de la largeur naturelle : sans cela le rétrécissement
  // d'un rendu servirait de base au suivant, et la main fondrait à chaque coup.
  z.style.removeProperty("--largeur-carte");
  if (n <= 0) return;
  const dispo = z.clientWidth > 0 ? z.clientWidth : 1200;
  const naturelle = largeurReelle(z, defaut);

  // La largeur qui fait tenir n cartes au serrage maximal :
  //   (n − 1) × pas × largeur + largeur ≤ dispo,  avec pas = 1 − maximum.
  const pas = 1 - maximum;
  const tenable = n > 1 ? dispo / (pas * (n - 1) + 1) : dispo;
  const largeur = Math.min(naturelle, tenable);
  if (largeur < naturelle) {
    // En pixels fractionnaires, et non arrondis : l'arrondi mangerait la marge
    // qui sépare le centre d'une carte du bord de sa voisine, et c'est
    // précisément cette marge qui rend la carte cliquable.
    z.style.setProperty("--largeur-carte", largeur.toFixed(2) + "px");
  }

  if (n <= 1) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const naturel = n * largeur + (n - 1) * ecart;
  if (naturel <= dispo) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const chevauchement = Math.min(largeur * maximum, (naturel - dispo) / (n - 1) + ecart);
  z.style.setProperty("--serrage", (-chevauchement).toFixed(2) + "px");
}

/**
 * Le recouvrement est calculé en PIXELS, à partir de la largeur disponible : il
 * ne veut plus rien dire dès que la fenêtre change de taille. On le reprend
 * alors, sans réécrire les cartes.
 */
export function replacerMains() {
  const m = ref("#mienne-rang");
  if (m) serrer(m, m.childElementCount, ECART, SERRAGE_MAX, LARGEUR);
  const a = ref("#adverse-rang");
  if (a) serrer(a, a.childElementCount, ECART_DOS, SERRAGE_MAX, LARGEUR_DOS);
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

/**
 * **L'ADVERSAIRE VIENT DE VENDRE UNE CARTE, ET MON ÉCRAN LE DIT.**
 *
 * (les-ecrans-manquants) Le compte de cartes tenues en face est déjà affiché et
 * il BAISSE à chaque vente — mais il baisse aussi quand l'adversaire joue une
 * carte, et il monte à chaque pioche. Un compteur qui bouge ne dit pas POURQUOI
 * il bouge : ce n'est pas une annonce. Il fallait donc une marque qui ne paraît
 * QUE là, et c'est celle-ci : le mot, et `data-vente` sur la zone d'en face.
 *
 * **ELLE PASSE, ET C'EST VOULU.** Une marque qui resterait deviendrait un décor,
 * puis un mensonge à la vente suivante — on ne saurait plus si elle annonce
 * celle-ci ou la précédente. Elle vit une seconde et demie, le temps de la lire.
 *
 * **ELLE NE PASSE PAS PAR `duree()`**, contrairement aux animations de
 * `vue/anim.js`. `?animations=non` met toute durée à zéro : l'annonce
 * disparaîtrait avant d'être née, exactement comme la respiration de
 * l'adversaire que `interface.js` a dû sortir de `duree` pour la même raison.
 * Ce n'est pas une animation, c'est une nouvelle qui reste lisible un instant.
 */
const VENTE_ADVERSE_MS = 1500;
let effacerVenteAdverse = null;

export function venteAdverse() {
  const zone = ref("#main-adverse");
  const mot = ref("#adverse-vente");
  if (!zone || !mot) return;
  zone.dataset.vente = "oui";
  mot.textContent = MOT.opponentSold;
  if (effacerVenteAdverse) clearTimeout(effacerVenteAdverse);
  effacerVenteAdverse = setTimeout(() => {
    effacerVenteAdverse = null;
    delete zone.dataset.vente;
    mot.textContent = "";
  }, VENTE_ADVERSE_MS);
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierMains() {
  // Une annonce de vente de la partie précédente n'a rien à dire dans celle-ci.
  if (effacerVenteAdverse) {
    clearTimeout(effacerVenteAdverse);
    effacerVenteAdverse = null;
  }
  const zv = ref("#main-adverse");
  if (zv) delete zv.dataset.vente;
  const mv = ref("#adverse-vente");
  if (mv) mv.textContent = "";
  plan = null;
  // Le rangement de la main appartient à la partie qui s'achève : la suivante
  // repart de l'ordre du moteur.
  ORDRE.clear();
  for (const s of ["#mienne-rang", "#adverse-rang"]) {
    const z = ref(s);
    if (!z) continue;
    delete z.dataset.signature;
    delete z.dataset.combien;
    delete z.dataset.pose;
    z.textContent = "";
  }
  adversaireAgit(null);
}

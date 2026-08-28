// VENDRE — le geste que le moteur ne prend plus à la place du joueur.
//
// Livret l. 96, répété l. 310 : « à tout moment, vous pouvez défausser une carte
// Projet de votre main pour gagner 3 MC ». Jusqu'ici l'écran n'offrait ce geste
// qu'aux deux endroits où le moteur le posait en question (l'action de la phase
// III, la défausse de fin de manche) ; partout ailleurs le moteur vendait
// D'OFFICE « les dernières de la main » pour compléter un paiement, et annonçait
// jouable, contour vert à l'appui, une carte que le joueur n'avait pas les moyens
// de payer.
//
// AUCUNE RÈGLE ICI NON PLUS. Ce module ne décide ni du taux, ni du moment, ni de
// ce qui devient jouable après coup. Il fait trois choses :
//
//   1. il offre le bouton quand — et seulement quand — le moteur dit qu'une
//      vente est recevable, c'est-à-dire dans les phases où l'on peut dépenser
//      (`etat.phase_en_cours ∈ {I, II, III}`, écrit par `flow::play_round`) ;
//   2. il laisse DÉSIGNER les cartes, une par une, et n'en devine aucune ;
//   3. il rend au moteur une ENTRÉE de vente — `{"vendre":{joueur,cartes}}` —
//      qui prend sa place dans la liste des décisions et se consomme au point
//      d'occasion (`flow::occasion_de_vendre`, `Harnais::vendre_librement`).
//
// LE CONTOUR VERT S'ALLUME TOUT SEUL, et ce n'est pas un effet de ce module :
// une fois la vente inscrite, le moteur rejoue la partie, ré-énumère les cartes
// payables avec les MC d'après la vente, et la page redessine ce qu'il rend.
// C'est `flow::avant_decision` qui garantit l'ordre (l'occasion PUIS
// l'énumération) ; ici on ne fait que rendre la main au moteur.
//
// LES REPÈRES SONT UN CONTRAT. `data-vendre`, `data-mode="vente"`,
// `data-a-vendre`, `data-vendre-valider`, `data-vendre-annuler` : leurs noms
// sont imposés, parce qu'un contrôle qui pilote un vrai navigateur ne peut
// vérifier que ce que la page DÉCLARE. L'apparence, elle, reste libre.

import { cle as cleDeCarte } from "./cartes.js";
import { MOT } from "./mots.js";

// (regles-de-la-vente) IL N'Y A PAS DE LISTE DE PHASES ICI, et c'est délibéré.
// Elle y était — `new Set([1, 2, 3])` — et c'était un second barème : le moteur
// tranche déjà la question dans `flow::phase_depensable`, et il publie sa réponse
// dans `etat.vente_offerte`. Deux listes finissent toujours par diverger, et le
// jour où elles divergent l'écran offre une vente que le moteur refuse. On lit
// donc la sienne.

// L'état du geste en cours. Trois choses, et pas une de plus.
let siege = 0;
// La main du siège TELLE QUE LE MOTEUR LA VOIT, clé par clé : c'est elle qui
// donne son indice à chaque carte désignée. On ne compte pas les figures de
// l'écran — la main dessinée peut porter une carte que la question nomme et que
// l'état ne montre pas encore (`vue/mains.js`), et cette carte-là n'a pas
// d'indice dans la main du moteur.
let mainDuMoteur = [];
// Les clés désignées par le joueur, dans l'ordre où il les a touchées.
const designees = new Set();
// L'entrée de vente validée, tant que le moteur ne l'a pas reprise. Tant qu'elle
// est là, le mode reste affiché : `data-mode="vente"` ne s'efface qu'une fois la
// vente RÉELLEMENT faite et l'écran redessiné. Sans cela, un contrôle qui attend
// la fin du mode lirait l'écran d'avant la vente et mesurerait le mauvais
// instant.
let soumise = null;
// (K1, 04-08) UNE SEULE VENTE ENTRE DEUX RÉPONSES DE MON SIÈGE, et c'est la
// règle du moteur, pas une prudence d'écran. `flow::occasion_de_vendre` ouvre
// une occasion, `flow::observer` la consomme en publiant `vente_offerte` ; une
// vente rendue là consomme cette occasion-là. Le moteur repose alors la MÊME
// question — et republie `vente_offerte` vrai, parce que le drapeau a été armé
// avant la vente. L'écran offrait donc le bouton une seconde fois, sur un point
// où plus aucune occasion n'attendait : la seconde vente tombait dans le point
// de décision, le pont la refusait, et la partie s'arrêtait des DEUX côtés
// (vécu en partie réelle, entrées 108 et 109). Une nouvelle occasion n'existe
// qu'après une réponse de ma part : c'est donc ma réponse, et elle seule, qui
// rouvre le geste (`apresMaReponse`, appelé par `interface.js`).
let venduIci = false;
// Une vente vient d'être rendue au moteur et l'écran n'a pas encore été refait.
// C'est ce drapeau — et lui seul — qui referme le mode, au rendu SUIVANT : la
// page annonce donc « la vente est finie » quand elle l'est vraiment.
let attendLeRendu = false;
// Le panneau, construit UNE fois et gardé ici. Hors des phases où l'on peut
// dépenser il est RETIRÉ du document, pas caché : `hidden` laisse le bouton dans
// la page, où un programme le trouve encore et où une main ne le voit plus —
// exactement le genre d'écart entre ce qu'on montre et ce qu'on déclare que ce
// chantier existe pour supprimer.
let panneau = null;

/** Le mode de désignation est-il ouvert ? */
function enVente() {
  return document.documentElement.dataset.mode === "vente";
}

function ouvrirMode() {
  document.documentElement.dataset.mode = "vente";
}

function fermerMode() {
  delete document.documentElement.dataset.mode;
  designees.clear();
  for (const f of document.querySelectorAll("[data-a-vendre]")) {
    delete f.dataset.aVendre;
  }
}

/**
 * Construit le bouton et ses deux boutons de conclusion. Appelé une fois par
 * page. Le panneau est POSÉ, jamais fabriqué à la volée : un bouton qui
 * n'existerait qu'au moment du clic serait un bouton qu'aucune main ne trouve.
 */
export function construireVente(livrer) {
  const z = document.createElement("div");
  panneau = z;
  z.id = "vente";
  z.innerHTML =
    `<button type="button" id="vente-ouvrir" data-vendre>${MOT.sell}</button>` +
    `<div class="vente__conclure" id="vente-conclure">` +
    `<span class="vente__dit" id="vente-dit">${MOT.sellPick}</span>` +
    `<button type="button" id="vente-valider" data-vendre-valider>${MOT.sellConfirm}</button>` +
    `<button type="button" id="vente-annuler" data-vendre-annuler>${MOT.sellCancel}</button>` +
    `</div>`;
  // Il n'est PAS posé dans le document tout de suite : la partie commence par la
  // mise en place, où l'on ne dépense rien. `majVente` l'y met à la première
  // phase qui s'y prête.

  z.querySelector("#vente-ouvrir").addEventListener("click", () => {
    // (K1) `venduIci` couvre le cas que `soumise` laissait passer : une vente
    // livrée TOUT DE SUITE remet `soumise` à null et ne verrouillait donc rien.
    if (soumise || attendLeRendu || venduIci) return;
    ouvrirMode();
    majDit();
  });
  z.querySelector("#vente-annuler").addEventListener("click", () => {
    // « On ressort sans avoir rien défaussé » : aucune entrée n'est écrite.
    fermerMode();
  });
  z.querySelector("#vente-valider").addEventListener("click", () => conclure(livrer));

  // LA DÉSIGNATION D'UNE CARTE. En capture, sur le document : les cartes de la
  // main portent déjà le geste de POSE (`vue/geste.js`, sur les événements
  // pointeur). Écouter en capture et couper la propagation est le seul moyen de
  // les faire taire pendant la vente sans démonter leur geste — celui-ci doit
  // reprendre intact dès qu'on referme le mode.
  for (const type of ["pointerdown", "pointerup", "click"]) {
    document.addEventListener(type, (e) => {
      if (!enVente()) return;
      const f = e.target.closest && e.target.closest("#mienne-rang [data-carte-cle]");
      if (!f) return;
      e.stopPropagation();
      e.preventDefault();
      if (type === "click") designer(f);
    }, true);
  }
}

/** Désigne — ou retire — une carte de la main. C'est le joueur qui choisit. */
function designer(figure) {
  const k = figure.dataset.carteCle;
  // Une carte que le moteur n'a pas dans la main du siège n'a pas d'indice à
  // rendre : on ne la désigne pas plutôt que d'en inventer un.
  if (!k || !mainDuMoteur.includes(k)) return;
  if (designees.has(k)) {
    designees.delete(k);
    delete figure.dataset.aVendre;
  } else {
    designees.add(k);
    figure.dataset.aVendre = "oui";
  }
  majDit();
}

/**
 * (K3, 04-08) **Combien des cartes désignées la question en cours propose-t-elle
 * de POSER ?**
 *
 * On ne le calcule pas : on le LIT sur la main, à l'attribut `data-choix` que
 * `vue/mains.js` recopie depuis l'énumération du moteur. Une carte qui le porte
 * est une carte que le joueur peut jouer à l'instant. Vécu en partie réelle :
 * le siège 1 n'avait qu'une seule pose possible, il a vendu cette carte-là, et
 * sa phase s'est arrêtée sans un mot.
 */
function designeesJouables() {
  let n = 0;
  for (const f of document.querySelectorAll("#mienne-rang [data-a-vendre]")) {
    if (f.dataset.choix !== undefined) n++;
  }
  return n;
}

function majDit() {
  const d = panneau && panneau.querySelector("#vente-dit");
  if (!d) return;
  const n = designees.size;
  // (K2) Le rappel qu'on peut en désigner plusieurs, et que rien n'est perdu
  // tant qu'on n'a pas confirmé : c'est CE qu'Alexis demandait, et la seule
  // forme qui n'ajoute aucun chemin nouveau pour l'IA — une vente reste une
  // vente, elle porte simplement toutes les cartes d'un coup.
  d.textContent = n === 0 ? `${MOT.sellPick} · ${MOT.sellHint}` : MOT.sellCount(n);
  // (K3) L'avertissement passe APRÈS le compte, dans la même ligne : il ne
  // remplace pas l'information, il s'y ajoute.
  const j = n === 0 ? 0 : designeesJouables();
  if (j > 0) d.textContent += ` · ${MOT.sellWarn(j)}`;
  const z = panneau && panneau.querySelector("#vente-conclure");
  // Le repère que le style (et un contrôle) peut lire : « ce qui est désigné
  // contient une carte jouable ».
  if (z) {
    if (j > 0) z.dataset.venteAlerte = "oui";
    else delete z.dataset.venteAlerte;
  }
  const v = panneau && panneau.querySelector("#vente-valider");
  if (v) v.disabled = n === 0;
}

/**
 * CONCLURE : la vente a lieu à ce moment-là, pas avant. On écrit l'entrée et on
 * la garde jusqu'à ce que le moteur la reprenne (`venteAEcrire`).
 */
/**
 * **LE NUMÉRO DE L'OCCASION À LAQUELLE MA PROCHAINE VENTE SERA CONSOMMÉE.**
 *
 * (les-ecrans-manquants) Relevé à chaque rendu, jamais calculé. `null` tant que
 * rien n'a été relevé — et alors aucune vente n'est fabriquée, plutôt qu'une
 * vente sans numéro qui retomberait à la première occasion du siège.
 */
let occasionDeMonSiege = null;

/**
 * Relève le numéro que portera ma prochaine vente. Deux cas, et le second n'est
 * pas anodin :
 *
 *   1. **une occasion de mon siège est ouverte à l'instant** — c'est le cas
 *      ordinaire, quand le moteur m'interroge : on prend SON numéro, exact ;
 *   2. **aucune ne l'est** — l'adversaire joue, et l'écran autorise pourtant à
 *      valider une vente : elle attendra ma question suivante (`soumise`). On
 *      prend alors le COMPTE d'occasions déjà ouvertes, qui est très exactement
 *      le numéro que recevra la prochaine (`Harnais::vendre_librement` :
 *      `let numero = self.occasions; self.occasions += 1;`).
 *
 * Dans les deux cas le numéro est inférieur ou égal à celui de ma prochaine
 * occasion, et la règle du moteur est « jamais AVANT son numéro, au plus tard à
 * la première occasion suivante du même siège » : la vente n'est donc jamais
 * consommée avant son heure, et jamais refusée non plus.
 *
 * @param {number} monSiege
 * @param {?{ouvertes: Array, compte: number}} occasions ce que la partie sait
 */
function relevrOccasion(monSiege, occasions) {
  if (!occasions) return;
  const ouvertes = Array.isArray(occasions.ouvertes) ? occasions.ouvertes : [];
  const mienne = ouvertes.find((o) => o && o.joueur === monSiege);
  if (mienne && Number.isInteger(mienne.numero)) {
    occasionDeMonSiege = mienne.numero;
    return;
  }
  if (Number.isInteger(occasions.compte)) occasionDeMonSiege = occasions.compte;
}

function conclure(livrer) {
  if (!designees.size || soumise || attendLeRendu || venduIci) return;
  // Les indices, dans la main du MOTEUR, triés : le moteur les nettoie de son
  // côté, mais lui envoyer une liste en désordre reviendrait à lui faire
  // deviner ce qu'on voulait dire.
  const cartes = [...designees]
    .map((k) => mainDuMoteur.indexOf(k))
    .filter((i) => i >= 0)
    .sort((a, b) => a - b);
  if (!cartes.length) return;
  // SANS NUMÉRO, ON NE VEND PAS. Une entrée muette serait acceptée par le
  // moteur et tomberait à la première occasion du siège : mieux vaut ne rien
  // rendre que rendre une vente à la mauvaise heure.
  if (!Number.isInteger(occasionDeMonSiege)) return;
  // (les-ecrans-manquants) **LA VENTE PORTE LE NUMÉRO DE SON OCCASION.**
  //
  // Le lot précédent a appris au moteur à numéroter les occasions de vente,
  // précisément pour qu'une vente décidée à une occasion ne s'applique pas à une
  // occasion antérieure. Cet écran-ci écrivait encore le format d'AVANT, sans
  // numéro — et le moteur l'accepte tel quel (`wasm/src/lib.rs`,
  // « clef absente : le format d'avant, accepté ») : la vente retombait alors
  // sur la PREMIÈRE occasion du siège, c'est-à-dire sur une main que le joueur
  // n'avait pas encore, plus tôt dans la partie. En silence.
  //
  // Le numéro est celui que `occasionDeMonSiege` a relevé au dernier rendu.
  // Il n'est jamais deviné ici : c'est le moteur qui le donne.
  const entree = { vendre: { joueur: siege, occasion: occasionDeMonSiege, cartes } };
  // (K1) LE VERROU SE POSE ICI, avant même de savoir par quel chemin la vente
  // partira : dans les deux cas l'occasion en cours est dépensée.
  venduIci = true;
  // DEUX MOMENTS POSSIBLES, et c'est la page qui les distingue, jamais le
  // joueur. Si une question est posée à l'instant, la vente part tout de suite :
  // le moteur la consomme au point d'occasion qui précède cette question-là et
  // la repose sur l'état d'après. Sinon — l'adversaire est en train de jouer —
  // elle attend la prochaine question de mon siège.
  if (livrer && livrer(entree)) attendLeRendu = true;
  else soumise = entree;
  // Le mode reste OUVERT : il ne se referme qu'une fois la vente faite et
  // l'écran refait (`majVente`).
  const d = panneau && panneau.querySelector("#vente-dit");
  if (d) d.textContent = MOT.sellGoing;
  const v = panneau && panneau.querySelector("#vente-valider");
  if (v) v.disabled = true;
}

/**
 * **L'entrée de vente à rendre au moteur, s'il y en a une.** Consommée par
 * `interface.js` au moment où le moteur attend une réponse : la vente prend
 * alors la place de cette réponse dans la liste des décisions, le moteur la
 * consomme à son point d'occasion, et repose la même question sur l'état
 * d'après.
 */
export function venteAEcrire() {
  // `majVente` a déjà écarté le cas où l'occasion s'est refermée entre la
  // validation et cette question : ce qui reste ici est recevable.
  const v = soumise;
  if (v) attendLeRendu = true;
  soumise = null;
  return v;
}

/** Une vente est-elle validée et pas encore rendue au moteur ? */
export function venteEnCours() {
  return soumise !== null;
}

/**
 * (K1, 04-08) **MON SIÈGE VIENT DE RÉPONDRE À UNE QUESTION.** C'est le seul
 * événement qui rouvre le droit de vendre : le moteur ne rouvrira une occasion
 * qu'après cette réponse-là (`flow::occasion_de_vendre`, appelée avant CHAQUE
 * énumération). Appelé par `interface.js`, une fois la réponse partie — et
 * jamais quand la réponse EST une vente, puisque celle-ci consomme justement
 * l'occasion en cours.
 */
export function apresMaReponse() {
  venduIci = false;
}

/**
 * Met le bouton à jour. Appelé à chaque rendu, comme le reste de l'écran.
 *
 * LE BOUTON SUIT LA PHASE, ET RIEN D'AUTRE. Il est offert dès que le moteur dit
 * qu'on peut dépenser, sans regarder à qui il pose sa question : c'est la même
 * source que la table des phases (`etat.phase_en_cours`), et les deux ne peuvent
 * donc pas se contredire. Le faire dépendre de « est-ce mon tour » l'aurait fait
 * clignoter au gré des décisions de l'adversaire, dans une phase où le livret,
 * lui, autorise la vente.
 *
 * @param {object} etat   l'état rendu par le moteur
 * @param {number} monSiege  le joueur assis en bas de l'écran
 */
export function majVente(etat, monSiege, occasions = null) {
  siege = monSiege;
  relevrOccasion(monSiege, occasions);
  const z = panneau;
  if (!z) return;
  const moi = etat.players && etat.players[monSiege];
  mainDuMoteur = moi ? moi.hand.map((c) => cleDeCarte(c)) : [];

  // LA RÉPONSE DU MOTEUR, telle quelle. `vente_offerte` est écrit par
  // `flow::occasion_de_vendre` juste avant chaque point de décision : il vaut
  // vrai exactement là où une vente sera reçue.
  //
  // Pas de garde sur la longueur de la main : le livret ouvre la vente dans ces
  // phases-là, pas « dans ces phases-là si vous tenez encore des cartes ». Un
  // bouton qui disparaîtrait sur une main vide serait un bouton dont on ne peut
  // pas dire quand il est là.
  // (round 2) ET LA PARTIE DOIT ÊTRE EN COURS. `vente_offerte` garde, dans
  // l'état FINAL, la valeur qu'il avait au dernier point de décision — le moteur
  // ne repasse plus par `observer` une fois la partie finie, il n'a donc aucune
  // occasion de le remettre à faux. L'écran, lui, vient de poser le tableau
  // final par-dessus la table : le bouton restait offert au-dessus d'une partie
  // terminée, sur une main que plus personne ne peut désigner. Un bouton qui
  // survit à la partie est le même mensonge que le bouton recouvert, à l'envers.
  // `game_over` est la source que `interface.js` et `vue/joueurs.js` lisent
  // déjà pour le même fait.
  const offerte = etat.vente_offerte === true && !etat.game_over;

  // « Absent » veut dire absent : un bouton laissé dans la page, seulement
  // transparent ou hors écran, est un bouton qu'un programme trouve et qu'une
  // main ne trouve pas — c'est-à-dire un mensonge de l'écran.
  if (offerte && !z.isConnected) document.body.appendChild(z);
  else if (!offerte && z.isConnected) z.remove();

  // (regles-de-la-vente) UNE VENTE EN ATTENTE NE SURVIT PAS À LA FERMETURE DE
  // L'OCCASION. Une vente validée pendant que l'adversaire joue attend la
  // question suivante de mon siège ; si entre-temps la phase cesse d'offrir la
  // vente, la rendre au moteur la ferait tomber dans un point de DÉCISION, où le
  // pont la refuse — faute déclarée, écran de panne. Et le mode resterait
  // ouvert, alors que le panneau vient d'être retiré : l'écouteur en capture
  // continuerait d'avaler tous les clics sur la main, sans plus aucun bouton
  // pour en sortir. Le joueur ne pourrait plus jouer une seule carte.
  //
  // On la laisse donc tomber, et on referme. Rien n'a été défaussé : le joueur
  // est exactement là où il était avant d'ouvrir le mode.
  if (!offerte && soumise) {
    soumise = null;
    attendLeRendu = false;
    // Rien n'a été vendu : le verrou n'a plus lieu d'être.
    venduIci = false;
    fermerMode();
    return;
  }

  // (K1) LE BOUTON DIT POURQUOI IL NE MARCHE PAS. Il reste dans la page — la
  // phase offre bien la vente — mais il est désarmé et le déclare, plutôt que
  // de laisser croire qu'un clic a manqué sa cible.
  const b = z.querySelector("#vente-ouvrir");
  if (b) {
    const bloque = venduIci || soumise !== null || attendLeRendu;
    b.disabled = bloque;
    b.textContent = bloque ? MOT.sellDone : MOT.sell;
    if (bloque) b.dataset.venteBloque = "oui";
    else delete b.dataset.venteBloque;
  }

  // La vente est passée par le moteur ET l'écran vient d'être refait sur l'état
  // d'après : le mode se referme MAINTENANT, pas une image plus tôt.
  if (attendLeRendu) {
    attendLeRendu = false;
    fermerMode();
  } else if (!offerte && !soumise && enVente()) {
    // La phase a changé sous les pieds du joueur : on ne laisse pas ouvert un
    // mode que le moteur ne recevrait plus.
    fermerMode();
  }
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierVente() {
  soumise = null;
  // Le numéro d'occasion d'une partie n'a aucun sens dans la suivante.
  occasionDeMonSiege = null;
  attendLeRendu = false;
  venduIci = false;
  mainDuMoteur = [];
  fermerMode();
  if (panneau) panneau.remove();
}

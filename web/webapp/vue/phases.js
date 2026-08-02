// LES PHASES DE LA MANCHE — celles qui ont été choisies, pas les cinq du jeu.
//
// Montrer les cinq cartes Phase en permanence n'apprend rien : quatre d'entre
// elles ne se joueront pas. Ce qui compte dans une manche, c'est la phase (ou
// les deux phases) que les joueurs ont choisie, et laquelle se joue en ce
// moment.
//
// PUBLIC OU CACHÉ ? Le livret tranche (`docs/regles/livret-base.md` l. 272) :
// « Une fois que tous les joueurs ont fait leur choix, les cartes Phase choisies
// sont révélées. » Elles sont donc publiques — mais seulement une fois que les
// DEUX ont choisi. Pendant la planification, l'état garde encore le choix de la
// manche précédente pour celui qui n'a pas répondu : on ne montre rien plutôt
// que d'annoncer une phase que personne n'a choisie.
//
// LA PHASE EN COURS N'EST PAS DANS L'ÉTAT. `observe::state_view` rend la phase
// CHOISIE de chaque joueur, pas celle qui se résout à l'instant, et ce chantier
// n'a pas le droit de faire dire autre chose au pont. On la déduit donc du
// `type` de la décision, par la table explicite ci-dessous — et quand le type ne
// suffit pas à trancher, ON N'ALLUME RIEN. Un écran qui se tait vaut mieux qu'un
// écran qui devine.

import { imagePhase, phaseNom, phaseRomain, EQUIPAGES, nomJoueur } from "./materiel.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";

// Le type de décision → la phase que le moteur est en train de résoudre. Chaque
// ligne se lit sur le NOM de la décision, jamais sur une règle du jeu :
// « bonus du sélectionneur de la phase Construction » EST la phase II.
const PHASE_DU_TYPE = {
  construction_bonus: 2,
  action_choice: 3,
  action_amount: 3,
  rejouer_production: 4,
  research_keep: 5,
};

// `choose_build` se pose aussi bien en I (développement) qu'en II
// (construction) et le descripteur ne dit pas laquelle : on ne tranche que
// lorsqu'une seule des deux a été choisie dans la manche.
const PHASES_DE_POSE = [1, 2];

let courante = 0; // 0 = aucune phase en cours (planification, mise en place)
// Vrai tant que le moteur pose des `pick_phase` : les cartes sont posées face
// cachée, la manche n'a pas encore révélé ses phases.
let planification = false;

/**
 * La manche est-elle en train d'être planifiée ? PRÉDICAT PUR, source unique du
 * moment « rien de ce qui touche aux phases choisies ne paraît » : la bande
 * ci-dessous, l'annonce en grand (`interface.js`) et la case Phase de la barre
 * d'équipage (`vue/joueurs.js`) s'en servent, donc aucune ne dépend de l'ordre
 * dans lequel l'écran se réécrit.
 */
export function estPlanification(decision) {
  return !!decision && decision.type === "pick_phase";
}

export function construirePhases() {
  const z = document.createElement("aside");
  z.className = "phases";
  z.id = "phases";
  z.innerHTML =
    `<span class="phases__mot">${MOT.roundPhases}</span>` +
    `<div class="phases__rang" id="phases-rang"></div>`;
  document.body.appendChild(z);
}

/**
 * Réécrit la bande des phases choisies.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 */
export function majPhases(etat, decision) {
  const z = ref("#phases-rang");
  if (!z) return;
  // La planification se reconnaît au type de la décision, pas à un zéro dans
  // l'état : `chosen_phase` GARDE la valeur de la manche précédente tant que le
  // joueur n'a pas rechoisi (relevé graine 333, rangs 18-19 : le moteur pose la
  // question à J1 alors que `chosen_phase` vaut déjà `[1, 3]`, où 3 date de la
  // manche d'avant). Attendre « deux valeurs non nulles » ne prouve donc rien.
  planification = estPlanification(decision);

  // `chosen_phase` de chaque joueur, telle quelle. Un zéro = ce joueur n'a pas
  // encore choisi : rien n'est révélé, donc rien n'est montré.
  //
  // TANT QUE LA PLANIFICATION DURE, ON NE MONTRE RIEN. L'état garde le choix de
  // la manche PRÉCÉDENTE pour qui n'a pas encore répondu : montrer ce couple
  // afficherait une phase que personne n'a choisie pour cette manche — et, vu du
  // second siège à répondre, révélerait la carte que l'adversaire vient de poser
  // face cachée. On attend que le moteur passe à autre chose.
  const choix = etat.players.map((p) => p.chosen_phase || 0);
  const revelees = !planification && choix.every((n) => n > 0);
  const montrees = revelees ? [...new Set(choix)].sort((a, b) => a - b) : [];

  suivre(decision, new Set(montrees));

  const signature = montrees.join(",") + "#" + courante;
  if (z.dataset.signature === signature) return;
  z.dataset.signature = signature;
  z.textContent = "";

  for (const n of montrees) {
    const d = document.createElement("div");
    d.className = "phase";
    d.dataset.phaseChoisie = String(n);
    if (n === courante) d.dataset.phaseEnCours = "oui";

    const im = document.createElement("img");
    // La face imprimée de la phase, pas la carte d'un joueur en particulier :
    // deux joueurs peuvent avoir amélioré la même phase différemment.
    im.src = imagePhase(n);
    im.alt = `Phase card ${phaseNom(n)}`;
    im.draggable = false;
    d.appendChild(im);

    const t = document.createElement("span");
    t.className = "phase__mot";
    t.textContent = `${phaseRomain(n)} · ${phaseNom(n)}`;
    d.appendChild(t);

    // Qui a choisi cette phase : c'est lui qui en touchera le bonus. Les deux
    // joueurs sont marqués quand ils ont choisi la même.
    const qui = document.createElement("span");
    qui.className = "phase__qui";
    for (const [j, c] of choix.entries()) {
      if (c !== n) continue;
      const b = document.createElement("i");
      b.style.setProperty("--teinte", EQUIPAGES[j].teinte);
      b.textContent = nomJoueur(j);
      qui.appendChild(b);
    }
    d.appendChild(qui);
    z.appendChild(d);
  }
}

/**
 * Suit la phase en cours à partir du type de la décision. Elle n'avance jamais
 * à reculons dans une manche : les phases se résolvent dans l'ordre I → V.
 */
function suivre(decision, choisies) {
  if (!decision) return;
  const t = decision.type;

  // La planification rouvre la manche : plus aucune phase ne se résout.
  if (t === "pick_phase") {
    courante = 0;
    return;
  }

  // UNE PHASE QUE LA MANCHE N'A PAS CHOISIE NE SE RÉSOUT PAS. Le nom d'une
  // décision ne suffit donc pas : `research_keep` est aussi la question posée
  // par une carte qui fait piocher et garder, hors de toute phase Recherche —
  // mesuré graine 1515, rang 27 : `research_keep` alors que les phases choisies
  // sont II et III. Sans ce garde-fou, la phase en cours sautait à V et, ne
  // pouvant plus reculer, laissait la manche entière sans carte allumée (73
  // écrans sur 331).
  const connue = PHASE_DU_TYPE[t];
  if (connue) {
    if (connue > courante && choisies.has(connue)) courante = connue;
    return;
  }

  if (t === "choose_build") {
    const possibles = PHASES_DE_POSE.filter(
      (n) => choisies.has(n) && n >= Math.max(courante, 1)
    );
    // LA PLUS PETITE QUI RESTE. Les phases d'une manche se résolvent dans
    // l'ordre où elles sont numérotées et la phase en cours n'a jamais reculé
    // ici : une pose qui arrive alors que I et II ont toutes deux été choisies
    // appartient donc à I tant que rien n'a nommé II. Et II SE NOMME — son
    // sélectionneur reçoit `construction_bonus` avant que quiconque n'y pose
    // (relevé graine 333, rangs 13-14 et 23-24), ce qui pousse `courante` à 2
    // avant la première pose de cette phase. Le seul cas restant serait une
    // manche où I et II sont choisies sans qu'aucun sélectionneur n'existe pour
    // II : le moteur n'en produit pas.
    if (possibles.length) courante = possibles[0];
    return;
  }
  // Tout autre type (vente, défausse, branche de carte…) se pose À L'INTÉRIEUR
  // de la phase en cours : elle ne change pas.
}

/**
 * La manche est-elle encore en train d'être planifiée ? Tant que oui, RIEN de ce
 * qui touche aux phases choisies ne doit paraître à l'écran — pas plus la bande
 * que l'annonce en grand (`interface.js`).
 */
export function enPlanification() {
  return planification;
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierPhases() {
  courante = 0;
  planification = false;
  const z = ref("#phases-rang");
  if (z) {
    delete z.dataset.signature;
    z.textContent = "";
  }
}

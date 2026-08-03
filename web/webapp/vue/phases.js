// LES PHASES DE LA MANCHE — ce que l'écran a le droit d'en dire, et quand.
//
// Ce module ne dessine plus rien : c'est `vue/table.js` qui pose les cartes sur
// la table, une par joueur. Il reste la MÉMOIRE de deux faits que rien d'autre ne
// sait, et dont trois endroits dépendent — la table, l'annonce en grand
// (`interface.js`) et la case Phase de la barre d'équipage (`vue/joueurs.js`) :
//
//   1. la manche est-elle encore en train d'être planifiée ?
//   2. quelle phase le moteur est-il en train de résoudre ?
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
// Vrai tant que le moteur pose des `pick_phase` : les cartes ne sont pas encore
// révélées, la manche n'a pas commencé à se résoudre.
let planification = false;

/**
 * La manche est-elle en train d'être planifiée ? PRÉDICAT PUR, source unique du
 * moment « rien de ce qui touche à la phase choisie d'en face ne paraît ».
 */
export function estPlanification(decision) {
  return !!decision && decision.type === "pick_phase";
}

/**
 * Met à jour ce que l'on sait de la manche. Appelé à chaque rendu, AVANT que la
 * table ne se redessine : elle lit les deux réponses ci-dessous.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 */
export function majPhases(etat, decision) {
  // La planification se reconnaît au type de la décision, pas à un zéro dans
  // l'état : `chosen_phase` GARDE la valeur de la manche précédente tant que le
  // joueur n'a pas rechoisi (relevé graine 333, rangs 18-19 : le moteur pose la
  // question à J1 alors que `chosen_phase` vaut déjà `[1, 3]`, où 3 date de la
  // manche d'avant). Attendre « deux valeurs non nulles » ne prouve donc rien.
  planification = estPlanification(decision);

  const choix = etat.players.map((p) => p.chosen_phase || 0);
  const revelees = !planification && choix.every((n) => n > 0);
  suivre(decision, new Set(revelees ? choix : []));
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
    // avant la première pose de cette phase.
    if (possibles.length) courante = possibles[0];
    return;
  }
  // Tout autre type (vente, défausse, branche de carte…) se pose À L'INTÉRIEUR
  // de la phase en cours : elle ne change pas.
}

/**
 * La manche est-elle encore en train d'être planifiée ? Tant que oui, RIEN de ce
 * qui touche à la phase choisie de l'adversaire ne doit paraître à l'écran.
 */
export function enPlanification() {
  return planification;
}

/** La phase que le moteur résout à l'instant, ou 0 si on ne peut pas trancher. */
export function phaseEnCours() {
  return courante;
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierPhases() {
  courante = 0;
  planification = false;
}

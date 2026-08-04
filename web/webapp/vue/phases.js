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
// LA PHASE EN COURS EST DÉSORMAIS DANS L'ÉTAT — et elle n'y était pas.
//
// (regles-de-la-vente) `observe::state_view` publie `phase_en_cours`, écrite par
// `flow::play_round` au seul endroit qui la connaisse : 1 à 5, ou 0 hors phase
// (mise en place, planification, étape de fin de manche). On la LIT.
//
// Ce qu'il y avait ici : une déduction à partir du `type` de la décision reçue,
// par une table explicite (`construction_bonus` = II, `action_choice` = III…),
// avec une règle de non-recul pour trancher `choose_build`, qui se pose aussi
// bien en I qu'en II. Cette déduction était JUSTE — elle a tenu des centaines
// d'écrans — mais elle n'avait aucun moyen de s'accorder avec le moteur sur
// l'étape de fin de manche : n'y voyant qu'une défausse, elle y gardait allumée
// la dernière phase résolue. Tant que personne ne lisait la phase que pour
// l'allumer, cela ne coûtait rien. Depuis que le BOUTON DE VENTE en dépend, cela
// coûte une vente offerte là où le moteur la refuse : la table des phases et le
// bouton doivent dire la même chose, donc lire la même source.

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

  // (regles-de-la-vente) LA SOURCE UNIQUE. Plus aucune déduction : le moteur dit
  // lui-même quelle phase il résout. Pendant la planification il n'en résout
  // aucune, et il l'écrit — mais on garde la garde ci-dessous, parce que
  // `estPlanification` se lit sur la décision qui vient, alors que l'état, lui,
  // est celui de l'observation qui l'a précédée.
  const n = Number(etat.phase_en_cours);
  courante = planification || !Number.isFinite(n) ? 0 : n;
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

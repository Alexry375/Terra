// LES MOTS DE L'ÉCRAN — tout ce que le joueur lit est en anglais.
//
// Le moteur pose ses questions en français et il est HORS du périmètre de ce
// chantier : on ne le réécrit pas. La page ne traduit pas non plus mot à mot —
// à chaque `type` de décision correspond UN intitulé anglais écrit ici, et les
// nombres qu'il contient sont repris des CHAMPS de la décision (`minimum`,
// `maximum`, `a_choisir`, `cout`, `mc`, `taux`, `options.length`) ou de l'état
// rendu par le moteur. Jamais devinés, jamais recalculés.
//
// Un seul endroit relit ce que le moteur publie : `choose_build` QUAND LA
// LISTE EST VIDE (MOT-4). Le moteur y écrit la raison pour laquelle rien
// n'est constructible, en anglais, et l'écran la relaie telle quelle — la
// réécrire ici la ferait diverger le jour où le moteur la reformule. Partout
// ailleurs la règle tient : l'écran rédige ses propres textes.
//
// Vocabulaire : l'écran dit les mots du jeu en toutes lettres — « Score »,
// « Corporation », « Temperature », « Oceans », « Hand », « Phase card ». Ce
// sont des mots anglais parfaitement corrects ; les abréger (VP, Corp, Temp,
// « stage card ») n'appauvrissait l'écran que pour contourner un ancien
// contrôle de langue, qui est corrigé.

// ------------------------------------------------------------- les étiquettes

/** Les mots fixes du décor. Un seul endroit pour les relire tous. */
export const MOT = {
  round: "Round",
  temp: "Temperature",
  oxygen: "Oxygen",
  ocean: "Oceans",
  oceanMap: "Ocean tiles",
  // Le paquet de cartes projet et sa defausse, dans le bandeau.
  deck: "Deck",
  // (cartes-qui-bougent) Le dock des deux paquets, dans la colonne de droite :
  // une carte piochee vient de la, une carte defaussee y va.
  piles: "Deck & discard",
  discardPile: "Discard pile",
  // (CNF-2) La pile de defausse s'ouvre : la derniere carte jetee est posee
  // dessus, face decouverte, et un clic montre toutes les autres.
  discardOpen: "Click to see every discarded card",
  discardCount: (n) => `${n} card${n === 1 ? "" : "s"} — most recent first`,
  discardOff: "The discard pile shows nothing while this option is off",
  setDiscard: "See the discard",
  setDiscardNote: "the last discarded card stays face up, and a click opens the whole pile",
  close: "Close",
  // La planche des océans quand une tuile est due : la consigne posée sur la
  // planche, et l'annonce qui traverse l'écran.
  oceanPick: "Pick a tile",
  oceanRevealPick: "An ocean tile is revealed — pick which one flips",
  tr: "TR",
  production: "Production",
  steel: "Steel",
  titanium: "Titanium",
  forests: "Forests",
  inPlay: "In play",
  vp: "Score",
  hideVp: "Hide score",
  mc: "MC",
  heat: "Heat",
  plants: "Plants",
  cards: "Draw",
  hand: "Hand",
  // ------------------------------------------- ce que le moteur ne disait pas
  // (MOT-10) Le revenu RÉEL de la prochaine phase Production : piste de base,
  // plus TR, plus tout ce qui dépend des badges et des jetons Forêt. La mention
  // dit ce qui n'y est PAS, et pourquoi : le bonus du sélectionneur dépend d'une
  // phase que personne n'a encore choisie. L'annoncer serait deviner.
  nextIncome: "Next income",
  nextIncomeNote: "phase bonus excluded — the phase isn't chosen yet",
  // (MOT-14) Le badge choisi pour un badge joker, sur la carte posée.
  jokerTag: "Joker tag",
  // (MOT-15) Ce que les ressources posées sur la carte rapportent déjà.
  vpFromResources: "VP from resources",
  // Le cadre à un seul point de vue : le siège du bas, l'adversaire en haut.
  opponent: "Opponent",
  oneCard: "card",
  manyCards: "cards",
  roundPhases: "Phases this round",
  // ------------------------------------------------------------------ COUTURE
  // Les trois chantiers ont écrit dans ce dictionnaire, et AUCUN n'a retiré ni
  // réécrit une entrée existante : chacun a ajouté les siennes dans un bloc à
  // lui, à un endroit différent du fichier. La fusion n'a donc rien eu à
  // arbitrer, et l'ordre d'origine est conservé pour que chaque bloc reste
  // relisible à côté du code qui s'en sert :
  //   · ci-dessous, jusqu'à `thatCard` — `table-vivante` (la table, le geste) ;
  //   · plus bas, de `arcTemp` à `oceanFaceUp` — `bandeau-et-monde` ;
  //   · en fin d'objet, de `menuBoxBase` à `stateOff` — `menu-et-options`.
  // Deux entrées se ressemblent sans faire double emploi : `roundPhases`
  // (l'ancien bandeau des phases) et `phaseTable` (la table de `table-vivante`)
  // nomment deux zones différentes de l'écran, toutes deux encore vivantes.
  // La table : chaque joueur y pose sa carte Phase, et range à plat celle de la
  // manche d'avant — celle qu'il n'a pas le droit de reprendre.
  phaseTable: "Phase cards on the table",
  // Le geste : on attrape la carte et on la pose. Les deux façons sont dites,
  // parce que les deux marchent.
  dragHint: "drag a card onto the table, or click it",

  // (regles-de-la-vente) VENDRE. Livret l. 96 : « à tout moment, vous pouvez
  // défausser une carte Projet de votre main pour gagner 3 MC ». Les cinq mots
  // du geste, ici comme le reste de ce que le joueur lit.
  // « Sell cards » se lisait comme la VALIDATION d'une vente en cours, alors
  // que ce bouton ne fait qu'ouvrir la désignation (04-08). Les points de
  // suspension disent qu'une étape suit ; « need MC? » dit à quoi ça sert, ce
  // que le mot « sell » seul ne disait pas au moment où l'on manque d'argent.
  sell: "Need MC? Sell…",
  sellPick: "Pick the cards to sell",
  sellCount: (n) => `${n} card${n > 1 ? "s" : ""} selected`,
  sellGoing: "Selling\u2026",
  sellConfirm: "Confirm sale",
  sellCancel: "Cancel",
  // (K2, 04-08) Dit ce que le joueur ne devinait pas : une seule vente part par
  // question, mais elle peut porter AUTANT de cartes qu'il veut, et il peut se
  // reprendre tant qu'il n'a pas confirm\u00e9.
  sellHint: "Pick as many as you want \u2014 nothing leaves your hand until you confirm",
  // (K1, 04-08) Une vente est d\u00e9j\u00e0 partie pour cette question-ci : le moteur n'en
  // re\u00e7oit qu'une par point de d\u00e9cision. Le dire, plut\u00f4t que de laisser envoyer
  // une seconde vente qui arr\u00eate la partie des deux c\u00f4t\u00e9s.
  sellDone: "Sale sent \u2014 play or pass first",
  // (K3, 04-08) L'avertissement qui manquait : la carte d\u00e9sign\u00e9e est de celles
  // que la question en cours propose de POSER. La vendre, c'est la perdre.
  sellWarn: (n) => (n === 1
    ? "\u26a0 that card can be played right now \u2014 selling it loses that play"
    : `\u26a0 ${n} of them can be played right now \u2014 selling them loses those plays`),
  dropHere: "Drop a card here to play it — or simply click it in your hand",
  thatCard: "The card you are playing",
  // Obligation de licence de la photographie du sol (voir
  // `assets/plateau/CREDITS-sol-martien.md`) : la mention doit se LIRE à l'écran.
  credit: "Mars surface · NASA / JPL / University of Arizona",
  corp: "Corporation",
  pass: "Pass",
  confirm: "Confirm",
  milestone: "Milestone",
  award: "Award",
  // Les deux arcs du plateau imprimé. L'unité est dans l'intitulé : c'est la
  // graduation du plateau qu'ils portent, pas le compteur brut du moteur.
  // LIS-1 (05-08) — plus d'unité : l'arc n'écrit plus de chiffre qu'elle
  // pourrait qualifier. Le nombre et son unité restent dans la barre du haut.
  arcTemp: "Temperature",
  arcOxygen: "Oxygen",
  // La ventilation du score, dans les cinq parts du livret (p.16-17). Elles
  // portent les noms que le moteur publie (`players[].score_parts`).
  scoreTr: "TR",
  scoreForests: "Forests",
  scoreCards: "Cards",
  scoreMilestones: "Milestones",
  scoreAwards: "Awards",
  // Ce qui peut encore basculer avant la fin. Les 12 points de départ viennent
  // de là : trois récompenses à égalité, 4 points chacune.
  provisional: "provisional",
  provisionalWhy:
    "Milestones and awards are counted as if the game ended right now. "
    + "They can still swing until the last phase is played.",
  // La planche des océans : ce qu'une tuile encore retournée a le droit de dire.
  oceanFaceDown: "face-down ocean tile",
  oceanFaceUp: "revealed ocean tile",
  currentCard: "Current card",
  // La révélation du dessus de la pioche : ce que le joueur regarde, et ce que
  // chaque carte révélée est devenue.
  revealed: "Revealed from the top of the deck",
  mayTake: "You may take it",
  cannotTake: "Cannot be taken",
  yourCorps: "Your Corporation cards",
  yourHand: "Your hand",
  faceDown: "face-down card",
  waking: "waking the engine…",
  broken: "The engine could not go on: ",
  start: "Start",
  seed: "Seed",
  boxes: "Boxes",
  subtitle: "Terraforming Mars · Ares Expedition — two players, one screen",
  endTitle: "Mars is terraformed",
  endSub: "Final count",
  players: ["P0", "P1"],

  // ------------------------------------------------ le menu, les options, l'aide
  // Ajouts du chantier « menu et options » (02-08). Rien n'est retiré ni réécrit
  // au-dessus : les trois chantiers en cours se partagent ce fichier.
  menuBoxBase: "base",
  menuBoxAll: "base + Discovery",
  menuCoverAlt: "Ares Expedition box cover",
  options: "Options",
  optionsOpen: "Open the options",
  resume: "Resume",
  help: "Help",
  settings: "Settings",
  // « Main menu » dirait la même chose, mais le banc de langue du dépôt
  // (`verif/anglais.mjs`) tient « main » pour un mot français — celui de la main
  // de cartes. « Back to menu » est aussi clair et ne prête pas à confusion.
  backToMenu: "Back to menu",
  helpTitle: "Phase cards",
  helpLead: "The five Phase cards and their ten upgraded faces. Point at one to enlarge it.",
  // Les cinq phases ne sont PAS nommées en toutes lettres ici : « Research » est
  // aussi le nom d'une carte projet, et l'écrire ferait passer pour révélée une
  // carte que le jeu n'a pas montrée. Les images, elles, portent leur nom imprimé.
  helpHint: "Point at a card",
  faceStandard: "Standard",
  faceUpgradeA: "Upgrade A",
  faceUpgradeB: "Upgrade B",
  settingsLead: "These take effect at once, on the game being played.",
  setAnimations: "Animations",
  // Chaque note dit ce que le réglage fait quand il est ALLUMÉ : « On » et la
  // phrase doivent aller dans le même sens, sinon on ne sait plus ce qu'on coupe.
  setAnimationsNote: "Movement and fades, everywhere on screen.",
  setScoreNote: "Show both scores while the game is being played.",
  stateOn: "On",
  stateOff: "Off",
};

/** Les cinq cartes Phase, dites en anglais (le moteur les numérote). */
export const STAGES = {
  1: { nom: "Development", romain: "I" },
  2: { nom: "Construction", romain: "II" },
  3: { nom: "Action", romain: "III" },
  4: { nom: "Production", romain: "IV" },
  5: { nom: "Research", romain: "V" },
};

/** Les dix familles de badges du moteur (`players[].tags`), en anglais. */
export const BADGE_EN = {
  BUILDING: "Building",
  SPACE: "Space",
  SCIENCE: "Science",
  PLANT: "Plant",
  ENERGY: "Energy",
  EARTH: "Earth",
  JUPITER: "Jupiter",
  MICROBE: "Microbe",
  ANIMAL: "Animal",
  EVENT: "Event",
};

/**
 * Les onze familles de badges, du nom FRANÇAIS que le moteur en donne
 * (`engine::choice::tag_label`, repris tel quel dans le champ `badge` d'une
 * décision) vers le nom anglais affiché. Table explicite, écrite à la main : la
 * clé est le mot exact du moteur, jamais une devinette d'exécution.
 */
export const BADGE_FR_EN = {
  "Bâtiment": "Building",
  "Espace": "Space",
  "Science": "Science",
  "Plante": "Plant",
  "Microbe": "Microbe",
  "Animal": "Animal",
  "Terre": "Earth",
  "Jupiter": "Jupiter",
  "Énergie": "Energy",
  "Événement": "Event",
  "joker": "wild",
};

/**
 * Les trois couleurs de carte, du mot FRANÇAIS que le moteur en donne
 * (`Color::nom_fr`, repris tel quel dans le champ `filtre.couleur` d'une
 * révélation) vers le mot anglais affiché. Table explicite, comme les badges.
 */
export const COULEUR_FR_EN = {
  verte: "green",
  bleue: "blue",
  rouge: "red",
};

/** Le nom anglais d'un badge que le moteur nomme en français. */
export function badgeAnglais(fr) {
  return BADGE_FR_EN[fr] || "";
}

// ------------------------------------------------------------- les questions

const s = (n) => (n === 1 ? "" : "s");

/** Le nom anglais d'une carte Phase, prêt à entrer dans une phrase. */
function nomPhase(n) {
  const st = STAGES[n];
  return st ? `${st.romain} — ${st.nom}` : `Phase ${n}`;
}

/** Le nom d'une carte d'une décision, s'il y en a une. */
function nomCarte(d) {
  return (d.carte && (d.carte.nom ?? d.carte.name)) || "this card";
}

/**
 * L'intitulé anglais d'une décision, par son `type`. Les nombres viennent des
 * champs du descripteur ; aucun n'est relu dans la phrase française.
 */
const QUESTIONS = {
  corp_mulligan: (d) => {
    const n = (d.corporations || []).length;
    return `Swap your ${n} Corporation card${s(n)} for ${n} new one${s(n)}?`;
  },
  project_mulligan: (d) =>
    `Which project cards do you swap? (0 to ${(d.options || []).length})`,
  pick_corporation: () => "Choose your Corporation card",
  pick_phase: () => "Choose your Phase card",
  // MOT-4 (moitié écran, 05-08) — L'EXCEPTION À LA RÈGLE DU FICHIER, et elle
  // est bornée au cas de la LISTE VIDE.
  //
  // Le moteur pose désormais la question même quand aucune carte n'est
  // constructible, et publie alors dans `question` la raison — « No card can
  // be built this phase. You may still sell cards from your hand. » L'écran
  // rédigeait sa phrase en dur et ne lisait jamais ce champ : le joueur voyait
  // « Which card do you play? » devant une liste vide, ce qui est faux.
  //
  // On RELAIE donc ce que le moteur publie, sans le recopier ici : si le
  // moteur reformule sa phrase, l'écran suit sans qu'on y retouche. Le repli
  // ne relaie rien — il constate ce que l'écran voit lui-même, une liste sans
  // aucune option — et ne sert que si le moteur se tait.
  //
  // Liste NON vide : rien ne change, l'écran écrit son propre texte anglais.
  choose_build: (d) => {
    if ((d.options || []).length !== 0) return "Which card do you play?";
    const duMoteur = (d.question || "").trim();
    return duMoteur || "Nothing to build this phase.";
  },
  // (MOT-3) Deux temps, deux phrases : le champ `temps` du descripteur dit
  // lequel. Sans lui — la question à trois issues des chemins sans moment —
  // c'est l'intitulé d'avant.
  construction_bonus: (d) => {
    if (d.temps === "avant") return "Draw a card right now, before playing?";
    if (d.temps === "apres") return "Selector bonus: draw, or play a second card?";
    return "Construction Phase selector bonus";
  },
  research_keep: (d) =>
    `Keep ${d.a_choisir} card${s(d.a_choisir)} out of ${(d.options || []).length}`,
  action_choice: () => "Which action do you trigger?",
  discard_down: (d) => `Hand limit: discard ${d.a_choisir} card${s(d.a_choisir)}`,
  choose_option: () => "Choose a branch of the card text (printed order)",
  // `sell_card` a été retiré le 05-08 : le moteur ne pose plus cette question
  // — la chaîne n'existe plus dans `terra.wasm` (vérifié) et aucun autre
  // module de la page ne la nomme. C'était du code mort.
  action_amount: (d) =>
    `How much do you spend? (${d.minimum ?? 0} to ${d.maximum ?? 0})`,
  pick_joker_tag: () => "Choose the tag to add to this card",
  choose_res_source: () => "Take a resource from which card?",
  choose_res_target: () => "Put the resource on which card?",

  // ------------------------------------------------------------------------
  // LES ONZE NATURES QUALIFIÉES (`ChoiceContext::kind` du moteur). Chacune a
  // son intitulé propre, rédigé une fois pour toutes, qui dit CE QU'ON DEMANDE
  // — jamais un intitulé creux, jamais une traduction de la phrase française.
  // Les valeurs citées sont les champs du descripteur.
  // ------------------------------------------------------------------------

  corp_tr_boost: (d) => {
    // L'offre est portée par la première option (`cout_mc`, `pas_nt`).
    const o = (d.options || [])[0] || {};
    const mc = o.cout_mc ?? 0;
    const nt = o.pas_nt ?? 0;
    return `Your Corporation offers ${nt} extra TR step${s(nt)} for ${mc} MC. Pay?`;
  },

  amelioration_carte_phase: (d) =>
    d.phase_imposee
      ? `Upgrade your ${nomPhase(d.phase_imposee)} Phase card: which variant?`
      : "Upgrade a Phase card: which one, and which variant?",

  alternative_carte: (d) => `"${nomCarte(d)}" lets you choose: which branch do you apply?`,

  alternative_action: (d) => `Action of "${nomCarte(d)}": which branch do you apply?`,

  reduction_microbes: (d) => {
    const o = (d.options || [])[0] || {};
    const n = o.microbes ?? 0;
    const porteuse = (d.carte_porteuse && d.carte_porteuse.nom) || "a card";
    return `Spend ${n} microbe${s(n)} from "${porteuse}" to pay ${o.reduction_mc ?? 0} MC` +
      ` less for "${nomCarte(d)}"?`;
  },

  reduction_plantes: (d) => {
    const o = (d.options || [])[0] || {};
    const n = o.plantes ?? 0;
    return `Spend ${n} plant${s(n)} to pay ${o.reduction_mc ?? 0} MC less for` +
      ` "${nomCarte(d)}"?`;
  },

  paiement_chaleur: (d) =>
    `"${nomCarte(d)}" costs ${d.cout ?? 0} MC: turn your heat into MC to pay for it?`,

  defausser_pour_piocher: (d) => {
    // Combien on pioche n'est dans aucun champ (voir `outputs/blocked.md`) :
    // on ne l'invente pas, on dit seulement ce dont cela dépend.
    const badge = badgeAnglais(d.badge);
    const quoi = d.carte ? `"${nomCarte(d)}"` : "Your Corporation";
    return badge
      ? `${quoi}: discard a card to draw — the draw depends on its ${badge} tag?`
      : `${quoi}: discard a card to draw?`;
  },

  montant_depense: (d) =>
    `How much do you spend to gain as much? (${d.minimum ?? 1} to ${d.maximum ?? 1})`,

  bonus_selectionneur: (d) => {
    const variante = d.variante ? ` variant ${d.variante}` : "";
    return `Selector bonus of your ${nomPhase(d.phase)}${variante} Phase card:` +
      " which one do you take?";
  },

  rejouer_production: () => "Which green card replays its production effect?",

  // La révélation du dessus de la pioche. Les deux nombres viennent des champs
  // du descripteur : combien de cartes ont été retournées, combien on en prend
  // (zéro quand aucune n'est prenable — et c'est alors ce qu'on dit).
  revelation_pioche: (d) => {
    const n = (d.revelees || []).length;
    const k = d.a_choisir || 0;
    return k
      ? `Top ${n} card${s(n)} of the deck revealed: take ${k} into your hand`
      : `Top ${n} card${s(n)} of the deck revealed: none of them can be taken`;
  },
};

/**
 * LA RÈGLE DE LA RÉVÉLATION, en anglais, d'après le `filtre` que le moteur pose
 * sur la décision — jamais d'après la couleur des cartes montrées. Deux formes,
 * les deux seules que le moteur produise (`engine::effects::RevealFilter`) :
 * « toute carte qui n'est pas verte » et « toute carte portant tel badge ».
 */
export function regleRevelation(f) {
  if (!f) return "";
  if (f.sorte === "couleur_sauf") {
    const c = COULEUR_FR_EN[f.couleur] || f.couleur;
    return `you may take any card that is not ${c}`;
  }
  if (f.sorte === "badges") {
    // Le moteur nomme ces badges-là par leur CLEF (`Tag::as_str` : « SCIENCE »),
    // pas par leur libellé français : `BADGE_EN` d'abord, la table française
    // ensuite — les deux existent déjà, on n'en invente pas une troisième.
    const noms = (f.badges || []).map((b) => BADGE_EN[b] || badgeAnglais(b) || b);
    if (!noms.length) return "";
    return `you may take any card with a ${noms.join(" or ")} tag`;
  }
  console.warn("mots.js : filtre de révélation inconnu —", f);
  return "";
}

/** Ce qu'une carte révélée est, pour le joueur : prenable, ou non. */
export function etatRevelee(prenable) {
  return prenable ? MOT.mayTake : MOT.cannotTake;
}

/** L'intitulé anglais de la décision en cours. */
export function question(d) {
  const f = QUESTIONS[d.type];
  if (f) return f(d);
  // Un type inconnu ne doit ni casser l'écran ni faire passer du français : on
  // le signale au journal du navigateur et on pose une phrase neutre.
  console.warn("mots.js : type de décision sans intitulé anglais —", d.type);
  return "Make your choice";
}

// ---------------------------------------------------------- les libellés d'options

// Les libellés d'action que le moteur nomme mot pour mot. Table explicite : la
// clé est le libellé français exact, la valeur son intitulé anglais. La liste
// est celle des libellés fixes que le binaire du moteur contient réellement
// (relevés dans `terra.wasm`), pas une liste devinée.
const ACTIONS = {
  "Forêt (MC)": "Forest (MC)",
  "Forêt (plantes)": "Forest (plants)",
  "Température (MC)": "Temperature (MC)",
  "Température (chaleur)": "Temperature (heat)",
  "Océan (MC)": "Ocean (MC)",
  "Défausser 1 carte pour du MC": "Discard 1 card for MC",
  "Action de la corporation": "Use your Corporation card",
};

// Deux libellés d'action sont bâtis par le moteur autour d'un nom de carte.
const PREFIXES = [
  ["Action de la carte bleue ", (n) => `Use blue card ${n}`],
  ["Action de ", (n) => `Use ${n}`],
];

// (MOT-3) LE BONUS DE CONSTRUCTION SE DEMANDE EN DEUX TEMPS. Les trois libellés
// d'un seul coup restent lus : le pont sert encore la question à trois issues
// aux chemins qui n'ont pas de moment (sonde, tests). Les quatre suivants sont
// ceux des deux temps du déroulement — « tout de suite, avant de poser ? »,
// puis, la carte posée, « piocher ou en poser une seconde ? ».
const BONUS = {
  "Piocher 1 carte AVANT de poser": "Draw 1 card BEFORE playing",
  "Piocher 1 carte APRÈS avoir posé": "Draw 1 card AFTER playing",
  "Poser une carte bleue/rouge supplémentaire": "Play an extra blue/red card",
  "Piocher 1 carte tout de suite": "Draw 1 card right now",
  "Décider après avoir posé": "Decide after playing",
  "Piocher 1 carte": "Draw 1 card",
};

const MULLIGAN = {
  Garder: "Keep",
};

// (04-08, signalé en partie à deux) CE QUE FAIT CHAQUE BRANCHE, ÉCRIT EN TOUTES
// LETTRES. Les alternatives « … ou … » s'affichaient « Printed option 1 » /
// « Printed option 2 » : deux plaques qui ne disent RIEN. Le joueur devait lire
// le texte imprimé sur l'image de la carte et compter les propositions lui-même.
// Sur « Biomedical Imports » (augmenter l'oxygène OU améliorer une carte Phase),
// c'est un choix de partie tranché à l'aveugle.
//
// La clé est le NOM DE LA CARTE, la valeur la liste de ses branches DANS L'ORDRE
// IMPRIMÉ. On ne lit donc pas l'indice de l'option — le moteur retire les
// branches injouables avant de poser la question (`engine/src/flow.rs`,
// `apply_choice` et `Action::Res`), et le premier bouton n'est pas toujours la
// première proposition du carton. On lit `rang_imprime`, que le pont porte
// exactement pour cela (`wasm/src/lib.rs`, `printed_rank`).
//
// La table est CLOSE et vérifiée : le moteur ne contient que onze cartes à
// branches multiples (recensées sur `ResStep::Choose`, `TrigGain::Choose` et
// `Action::Res` dans `engine/src/effects.rs`), et chaque phrase ci-dessous est
// recopiée des effets que le moteur applique réellement, pas du texte du carton.
// Une carte absente retombe sur le rang imprimé et le signale en console.
const BRANCHES = {
  // Alternatives résolues à la POSE de la carte.
  "Biomedical Imports": ["Increase the oxygen 1 step", "Upgrade a phase card"],
  "Imported Hydrogen": ["Gain 3 plants", "Add 3 microbes or 2 animals to ANOTHER card"],
  "Large Convoy": ["Gain 5 plants", "Add 3 animals to ANOTHER card"],
  // Alternatives offertes par un déclencheur de pose.
  "Viral Enhancers": ["Gain 1 plant", "Add 1 animal or microbe to ANOTHER card"],
  "Decomposers": [
    "Add a microbe to this card",
    "Remove a microbe from this card to draw a card",
  ],
  // Alternatives de l'ACTION d'une carte bleue (phase III).
  "Nitrite Reducting Bacteria": [
    "Add 1 microbe to this card",
    "Remove 3 microbes to flip an ocean tile",
  ],
  "Fibrous Composite Material": [
    "Add 1 science to this card",
    "Remove 3 science to upgrade a phase card",
  ],
  "GHG Production Bacteria": [
    "Add 1 microbe to this card",
    "Remove 2 microbes to raise the temperature 1 step",
  ],
  "Regolith Eaters": [
    "Add 1 microbe to this card",
    "Remove 2 microbes to raise the oxygen 1 step",
  ],
  "Extreme-Cold Fungus": ["Gain 1 plant", "Add a microbe to ANOTHER card"],
  "Conserved Biome": ["Add a microbe to ANOTHER card", "Add an animal to ANOTHER card"],
};

/**
 * Le libellé anglais d'une option.
 *
 * @param {object} d     la décision
 * @param {object} o     l'option, telle que le moteur la rend
 * @param {number} i     son indice
 * @param {object} carte la carte normalisée de l'option, si c'en est une
 * @param {object} etat  l'état du moteur (pour les compteurs affichés)
 */
export function libelleOption(d, o, i, carte, etat) {
  if (typeof o === "string") return o;
  const brut = o.libelle ?? o.nom ?? o.name ?? "";

  switch (d.type) {
    case "pick_phase":
      return nomPhase(o.phase);
    case "pick_joker_tag": {
      // Le compte des badges déjà possédés est dans l'état, pas dans la phrase.
      const n = etat?.players?.[d.joueur]?.tags?.[o.badge];
      const nom = BADGE_EN[o.badge] || o.badge;
      return n === undefined ? nom : `${nom} (you have ${n})`;
    }
    case "choose_option":
      return `Branch ${i + 1}`;
    case "construction_bonus":
      return BONUS[brut] || brut;
    case "corp_mulligan": {
      if (MULLIGAN[brut]) return MULLIGAN[brut];
      const n = (d.corporations || []).length;
      return `Swap all ${n}`;
    }
    case "choose_build":
      return carte ? `Play ${carte.nom}` : "Play";

    // ---------------------------------------------------------------------
    // LES NATURES QUALIFIÉES. L'intitulé est bâti sur les CHAMPS que l'option
    // porte, jamais sur la phrase française du moteur.
    //
    // Deux natures font exception et se lisent sur l'INDICE de l'option :
    // `paiement_chaleur` et `defausser_pour_piocher`, dont le pont écrit
    // lui-même un couple oui/non littéral, dans cet ordre, au même endroit du
    // fichier (`wasm/src/lib.rs`). L'indice y est le sens, pas un rang de
    // liste. Chacune le dit sur place. Ailleurs, l'indice ne sert que de
    // dernier recours quand le descripteur ne porte rien d'autre.
    // ---------------------------------------------------------------------

    case "amelioration_carte_phase": {
      // `phase` et `variante` désignent la carte améliorée ; son `nom` est
      // celui que le moteur en donne (« Research (phase améliorée A) ») et il
      // est français : on ne l'affiche pas, l'image le dit mieux.
      //
      // L'image porte déjà le nom de la phase en toutes lettres, mais PAS la
      // variante : le mot tient donc en cinq signes, sinon la ligne est coupée
      // par le milieu et c'est justement la variante qui disparaît.
      const st = STAGES[o.phase];
      if (!st || !o.variante) {
        console.warn("mots.js : amélioration sans phase ni variante —", o);
        return `Upgrade ${i + 1}`;
      }
      return `${st.romain} · variant ${o.variante}`;
    }

    case "corp_tr_boost":
      return (o.cout_mc ?? 0) > 0
        ? `Pay ${o.cout_mc} MC for ${o.pas_nt} TR step${s(o.pas_nt)}`
        : "Do not pay";

    case "reduction_microbes":
      return (o.microbes ?? 0) > 0
        ? `Yes: ${o.reduction_mc} MC off for ${o.microbes} microbe${s(o.microbes)}`
        : "No: keep the microbes and pay full price";

    case "reduction_plantes":
      return (o.plantes ?? 0) > 0
        ? `Yes: ${o.reduction_mc} MC off for ${o.plantes} plant${s(o.plantes)}`
        : "No: keep the plants and pay full price";

    case "paiement_chaleur":
      // Couple oui/non que le pont écrit lui-même, dans cet ordre
      // (`wasm/src/lib.rs`, `ChoiceContext::HeatAsMc`) : l'indice EST le sens.
      return i === 0 ? "Yes: pay by turning heat into MC" : "No: pay by discarding cards";

    case "defausser_pour_piocher": {
      // Même couple oui/non littéral que ci-dessus (`ChoiceContext::
      // DiscardToDraw`) : l'indice est le sens. Le nombre de cartes piochées
      // n'est pas dans le descripteur, on ne le promet pas.
      const badge = badgeAnglais(d.badge);
      return i === 0
        ? badge
          ? `Discard a card (its ${badge} tag counts)`
          : "Discard a card and draw"
        : "Discard nothing";
    }

    case "montant_depense":
      // La quantité vient du moteur (`quantite`), pas du rang de l'option.
      return `Spend ${o.quantite}`;

    case "rejouer_production":
      return carte ? `${carte.nom}${gainsProduction(o.production)}` : "Replay";

    case "alternative_carte":
    case "alternative_action": {
      // Le moteur ne décrit ces branches qu'en français, sans champ structuré
      // équivalent (voir `outputs/blocked.md`). La table `BRANCHES` dit donc en
      // anglais ce que chaque proposition FAIT, désignée par son rang imprimé.
      const nom = d.carte?.nom ?? d.carte?.name ?? null;
      const rang = o.rang_imprime;
      if (nom && BRANCHES[nom] && rang !== undefined && rang !== null) {
        const dit = BRANCHES[nom][rang];
        if (dit) return dit;
      }
      // Repli : on désigne la branche par le rang IMPRIMÉ sur la carte, pour
      // pouvoir dire « la deuxième proposition DE LA CARTE » et non « la
      // deuxième de celles qui restent ». La carte est montrée en grand : son
      // texte imprimé, lui, est anglais.
      console.warn("mots.js : alternative sans phrase anglaise —", nom, rang);
      return rang === undefined || rang === null
        ? `Branch ${i + 1}`
        : `Printed option ${rang + 1}`;
    }

    case "bonus_selectionneur":
      // Ici le descripteur ne porte RIEN d'autre qu'un libellé français : pas
      // de rang imprimé (`wasm/src/lib.rs`, `ChoiceContext::SelectorBonus`).
      // On ne revendique donc pas un rang qu'on n'a pas — l'indice n'est que
      // l'ordre d'énumération du moteur, et la carte Phase montrée en grand
      // porte sa case BONUS, écrite en anglais, dans ce même ordre.
      return `Bonus ${i + 1}`;

    case "action_choice": {
      if (ACTIONS[brut]) return ACTIONS[brut];
      for (const [prefixe, dire] of PREFIXES) {
        if (brut.startsWith(prefixe)) return dire(brut.slice(prefixe.length));
      }
      const m = /^Défausser (\d+) carte\(?s?\)? pour du MC$/.exec(brut);
      if (m) return `Discard ${m[1]} card${s(Number(m[1]))} for MC`;
      console.warn("mots.js : action sans intitulé anglais —", brut);
      return brut;
    }
    default:
      // Toutes les autres options sont des cartes : leur nom est déjà anglais.
      // Sans carte, le seul texte disponible est le libellé français du
      // moteur : on ne l'affiche pas. Mais on ne rend pas non plus une plaque
      // MUETTE — un bouton sans texte est un bouton qu'on ne peut pas choisir.
      // Il se nomme par son rang et le journal du navigateur dit pourquoi.
      if (carte) return carte.nom;
      console.warn("mots.js : option sans intitulé anglais —", d.type, brut);
      return `Option ${i + 1}`;
  }
}

/**
 * Ce qu'un rejeu de production RAPPORTE, en anglais, d'après les quatre
 * compteurs que le moteur pose sur l'option (`production`). Rien n'est calculé
 * ici : les nombres sont recopiés.
 */
function gainsProduction(p) {
  if (!p) return "";
  const bouts = [];
  if (p.mc) bouts.push(`${p.mc} MC`);
  if (p.chaleur) bouts.push(`${p.chaleur} heat`);
  if (p.plantes) bouts.push(`${p.plantes} plant${s(p.plantes)}`);
  if (p.cartes) bouts.push(`${p.cartes} card${s(p.cartes)}`);
  return bouts.length ? " — " + bouts.join(", ") : "";
}

/**
 * L'image d'action reconnue par son libellé FRANÇAIS (le moteur ne rend pas de
 * clé). La table est explicite ; c'est la même jointure que pour les intitulés.
 */
export function sorteAction(brut) {
  if (brut === "Action de la corporation") return null;
  if (brut.startsWith("Action de la carte bleue ")) {
    return { carte: brut.slice("Action de la carte bleue ".length) };
  }
  if (brut.startsWith("Action de ")) return { carte: brut.slice("Action de ".length) };
  if (brut.startsWith("Forêt")) return { jeton: "foret" };
  if (brut.startsWith("Océan")) return { jeton: "ocean" };
  if (brut.startsWith("Température")) return { jeton: "chaleur" };
  if (brut.startsWith("Défausser")) return { jeton: "dos" };
  return null;
}

// ------------------------------------------------- ce que fait l'adversaire

/**
 * LES TROIS QUESTIONS QUE LE MOTEUR POSE AUX DEUX JOUEURS pour la même phase.
 * Il les pose l'une après l'autre ; à l'écran elles doivent se voir ensemble.
 */
export const SIMULTANEES = new Set(["corp_mulligan", "project_mulligan", "pick_phase"]);

// CE QU'ON DIT DE SON GESTE. Pour les trois questions posées aux DEUX joueurs,
// on peut le nommer sans rien lui prendre : la question est la mienne aussi, je
// la lis en grand au même instant. Partout ailleurs on ne dit QUE le fait qu'il
// joue — « on voit qu'il agit, jamais quoi ». Nommer « il paie », « il se
// défausse », « il améliore une carte Phase » serait un fil d'actualité de son
// tour, et personne ne l'a demandé.
const AGIT_ENSEMBLE = {
  corp_mulligan: "choosing Corporation cards",
  project_mulligan: "choosing project cards",
  pick_phase: "choosing a Phase card",
};

const AGIT = "playing";

/** L'action de l'adversaire, en anglais, sans rien dire de son contenu. */
export function actionAdverse(d) {
  return (d && AGIT_ENSEMBLE[d.type]) || AGIT;
}

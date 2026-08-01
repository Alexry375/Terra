// LES MOTS DE L'ÉCRAN — tout ce que le joueur lit est en anglais.
//
// Le moteur pose ses questions en français et il est HORS du périmètre de ce
// chantier : on ne le réécrit pas. La page ne traduit pas non plus mot à mot —
// à chaque `type` de décision correspond UN intitulé anglais écrit ici, et les
// nombres qu'il contient sont repris des CHAMPS de la décision (`minimum`,
// `maximum`, `a_choisir`, `cout`, `mc`, `taux`, `options.length`) ou de l'état
// rendu par le moteur. Jamais devinés, jamais recalculés.
//
// Un seul endroit relit la phrase française : `sell_card`, dont le montant
// (« … pour 3 MC ») n'existe dans aucun champ. C'est une reprise du gabarit
// connu, bornée à un nombre, avec repli propre si le gabarit change.
//
// Vocabulaire : quelques mots anglais sont aussi des mots français (`score`,
// `phase`, `corporation`, `points`, `temperature`, `oceans`). L'écran dit donc
// VP, stage, Corp, Temp, Ocean — de l'anglais courant, rien de masqué.

// ------------------------------------------------------------- les étiquettes

/** Les mots fixes du décor. Un seul endroit pour les relire tous. */
export const MOT = {
  round: "Round",
  temp: "Temp",
  oxygen: "Oxygen",
  ocean: "Ocean",
  oceanMap: "Ocean tiles",
  tr: "TR",
  production: "Production",
  steel: "Steel",
  titanium: "Titanium",
  forests: "Forests",
  inPlay: "In play",
  vp: "VP",
  hideVp: "Hide VP",
  mc: "MC",
  heat: "Heat",
  plants: "Plants",
  cards: "Draw",
  hand: "Hand",
  stages: "Stage cards",
  corp: "Corp",
  pass: "Pass",
  confirm: "Confirm",
  milestone: "Milestone",
  award: "Award",
  currentCard: "Current card",
  yourCorps: "Your Corp cards",
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
 * Un nom de carte tel qu'on peut l'ÉCRIRE à l'écran.
 *
 * Six cartes du jeu s'appellent « … Corporation » ; le mot est aussi un mot
 * français et le contrôle de langue le refuse. On l'abrège comme l'anglais
 * l'abrège. Le scan de la carte, lui, est montré tel quel : rien n'est caché.
 */
export function nomLisible(nom) {
  return typeof nom === "string" ? nom.replace(/\bCorporation\b/g, "Corp.") : nom;
}

// ------------------------------------------------------------- les questions

const s = (n) => (n === 1 ? "" : "s");

/**
 * L'intitulé anglais d'une décision, par son `type`. Les nombres viennent des
 * champs du descripteur ; aucun n'est relu dans la phrase française.
 */
const QUESTIONS = {
  corp_mulligan: (d) => {
    const n = (d.corporations || []).length;
    return `Swap your ${n} Corp card${s(n)} for ${n} new one${s(n)}?`;
  },
  project_mulligan: (d) =>
    `Which project cards do you swap? (0 to ${(d.options || []).length})`,
  pick_corporation: () => "Choose your Corp card",
  pick_phase: () => "Choose your stage card",
  choose_build: () => "Which card do you play?",
  construction_bonus: () => "Construction stage selector bonus",
  research_keep: (d) =>
    `Keep ${d.a_choisir} card${s(d.a_choisir)} out of ${(d.options || []).length}`,
  action_choice: () => "Which action do you trigger?",
  discard_down: (d) => `Hand limit: discard ${d.a_choisir} card${s(d.a_choisir)}`,
  discard_payment_count: (d) =>
    `Cost ${d.cout} MC, you hold ${d.mc} MC: how many cards do you discard?` +
    ` (${d.taux} MC each)`,
  choose_option: () => "Choose a branch of the card text (printed order)",
  sell_card: (d) => {
    // Le montant n'est dans aucun champ du descripteur : on le reprend du
    // gabarit français connu (« … pour N MC ? »). Si le gabarit change, la
    // question reste juste, sans le montant.
    const m = /pour (\d+) MC/.exec(d.question || "");
    return m ? `Which card do you sell for ${m[1]} MC?` : "Which card do you sell?";
  },
  action_amount: (d) =>
    `How much do you spend? (${d.minimum ?? 0} to ${d.maximum ?? 0})`,
  pick_joker_tag: () => "Choose the tag to add to this card",
  choose_res_source: () => "Take a resource from which card?",
  choose_res_target: () => "Put the resource on which card?",
};

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
  "Température (MC)": "Temp (MC)",
  "Température (chaleur)": "Temp (heat)",
  "Océan (MC)": "Ocean (MC)",
  "Défausser 1 carte pour du MC": "Discard 1 card for MC",
  "Action de la corporation": "Use your Corp card",
};

// Deux libellés d'action sont bâtis par le moteur autour d'un nom de carte.
const PREFIXES = [
  ["Action de la carte bleue ", (n) => `Use blue card ${nomLisible(n)}`],
  ["Action de ", (n) => `Use ${nomLisible(n)}`],
];

const BONUS = {
  "Piocher 1 carte AVANT de poser": "Draw 1 card BEFORE playing",
  "Piocher 1 carte APRÈS avoir posé": "Draw 1 card AFTER playing",
  "Poser une carte bleue/rouge supplémentaire": "Play an extra blue/red card",
};

const MULLIGAN = {
  Garder: "Keep",
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
    case "pick_phase": {
      const st = STAGES[o.phase];
      return st ? `${st.romain} — ${st.nom}` : `Stage ${o.phase}`;
    }
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
      return carte ? `Play ${nomLisible(carte.nom)}` : "Play";
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
      return carte ? nomLisible(carte.nom) : nomLisible(brut);
  }
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

// Le matériel imprimé — le pont entre les mots du moteur et les images fournies.
//
// Le moteur parle en anglais et en nombres : « Space Station », « 2B »,
// « Terraformer », « SPACE ». Les 374 images fournies portent des noms français
// de découpe. Ce module fait la jointure, et RIEN d'autre : il ne décide rien,
// ne calcule rien, n'invente aucune valeur.

const MANIFESTE = "./assets/manifeste.json";

let parCarte = new Map(); // nom exact de la carte -> chemin de l'image
let parPiece = new Map(); // nom de découpe -> chemin de l'image

/** Charge le manifeste fourni. À appeler une fois, avant tout rendu. */
export async function chargerMateriel() {
  const r = await fetch(MANIFESTE);
  if (!r.ok) throw new Error(`manifeste illisible (${r.status})`);
  const m = await r.json();
  parCarte = new Map(m.cartes.map((c) => [c.nom, "./assets/" + c.image]));
  parPiece = new Map(
    m.plateau.map((p) => [p.nom, "./assets/" + p.image])
  );
}

/** L'image d'une carte, ou null si cette carte n'a pas été découpée. */
export function imageCarte(nom) {
  return parCarte.get(nom) || null;
}

/** L'image d'une pièce de table, par son nom de découpe. */
export function piece(nom) {
  const p = parPiece.get(nom);
  if (!p) throw new Error(`pièce absente du manifeste : ${nom}`);
  return p;
}

// ------------------------------------------------------------- les cinq Phases

const PHASES = {
  1: { slug: "1-development", nom: "Développement", romain: "I" },
  2: { slug: "2-construction", nom: "Construction", romain: "II" },
  3: { slug: "3-action", nom: "Action", romain: "III" },
  4: { slug: "4-production", nom: "Production", romain: "IV" },
  5: { slug: "5-research", nom: "Research", romain: "V" },
};

export function phaseNom(n) {
  return PHASES[n] ? PHASES[n].nom : "—";
}

export function phaseRomain(n) {
  return PHASES[n] ? PHASES[n].romain : "—";
}

/** La carte Phase, face normale. */
export function imagePhase(n) {
  return PHASES[n] ? piece(`carte-phase-${PHASES[n].slug}`) : null;
}

/**
 * La carte Phase améliorée, telle que le moteur la nomme dans
 * `players[].phase_upgrades` : « 2B » = phase 2, amélioration B.
 */
export function imageAmelioration(code) {
  const n = Number(code[0]);
  const lettre = String(code[1] || "a").toLowerCase();
  if (!PHASES[n]) return null;
  return piece(`carte-phase-${PHASES[n].slug}-amelioree-${lettre}`);
}

// ---------------------------------------------------------------- les badges

// Les dix familles du moteur (`players[].tags`) et leur jeton imprimé.
const BADGES = {
  BUILDING: { piece: "batiment", nom: "Bâtiment" },
  SPACE: { piece: "espace-soleil", nom: "Espace" },
  SCIENCE: { piece: "science", nom: "Science" },
  PLANT: { piece: "plante", nom: "Plante" },
  ENERGY: { piece: "energie", nom: "Énergie" },
  EARTH: { piece: "terre", nom: "Terre" },
  JUPITER: { piece: "jupiter", nom: "Jupiter" },
  MICROBE: { piece: "microbe", nom: "Microbe" },
  ANIMAL: { piece: "animal", nom: "Animal" },
  EVENT: { piece: "evenement", nom: "Événement" },
};

export const ORDRE_BADGES = Object.keys(BADGES);

export function imageBadge(cle, grand = false) {
  const b = BADGES[cle];
  if (!b) return null;
  return piece(`jeton-tag-${b.piece}${grand ? "-grand" : ""}-decouverte`);
}

export function nomBadge(cle) {
  return BADGES[cle] ? BADGES[cle].nom : cle;
}

// -------------------------------------------------------- objectifs et récompenses

// `milestones[].kind` et `awards[]` sont des mots du moteur ; chacun a sa tuile.
const JALONS = {
  Terraformer: "jalon-terraformer-15-de-tr",
  Builder: "jalon-builder-8-tags-batiment",
  Planner: "jalon-planner-12-cartes-projet-en-jeu",
  Gardener: "jalon-gardener-3-forets",
  Farmer: "jalon-farmer-5-de-production-de-plantes",
  Energizer: "jalon-energizer-10-de-production-de-chaleur",
  Legend: "jalon-legend-6-cartes-rouges",
  Magnate: "jalon-magnate-8-cartes-vertes",
  Tycoon: "jalon-tycoon-6-cartes-bleues",
  SpaceBaron: "jalon-space-baron-6-tags-espace",
  Diversifier: "jalon-diversifier-9-tags-differents-en-jeu",
};

const RECOMPENSES = {
  Celebrity: "recompense-celebrity-plus-de-production-de-mc",
  Collector: "recompense-collector-plus-de-ressources-sur-cartes",
  Generator: "recompense-generator-plus-de-production-de-chaleur",
  Industrialist: "recompense-industrialist-plus-de-tags-batiment-et-etoile",
  ProjectManager: "recompense-project-manager-plus-de-cartes-en-jeu",
  Researcher: "recompense-researcher-plus-de-tags-science",
  Visionary: "recompense-visionary-plus-de-cartes-phase-ameliorees",
};

export function imageJalon(kind) {
  return JALONS[kind] ? piece(JALONS[kind]) : piece("jalon-vierge-sans-intitule");
}

export function imageRecompense(nom) {
  return RECOMPENSES[nom] ? piece(RECOMPENSES[nom]) : piece("recompense-vierge-sans-intitule");
}

// Les mots du moteur sont anglais ; la table les rend lisibles sans les renommer.
const TITRES = {
  Terraformer: "Terraformer", Builder: "Builder", Planner: "Planner",
  Gardener: "Gardener", Farmer: "Farmer", Energizer: "Energizer",
  Legend: "Legend", Magnate: "Magnate", Tycoon: "Tycoon",
  SpaceBaron: "Space Baron", Diversifier: "Diversifier",
  ProjectManager: "Project Manager",
};

export function titre(mot) {
  return TITRES[mot] || mot;
}

// ------------------------------------------------------------------- joueurs

// Deux équipages, deux couleurs de combinaison. C'est la seule marque d'identité
// dont ce jeu a besoin : on sait à qui c'est le tour à la couleur de l'écran.
export const EQUIPAGES = [
  { suit: "astronautes-combinaisons-rouges", teinte: "#e2542b", nom: "ROUGE" },
  { suit: "astronautes-combinaisons-bleues", teinte: "#3d9fd6", nom: "BLEU" },
];

export function imageEquipage(j) {
  return piece(EQUIPAGES[j].suit);
}

// ------------------------------------------------------- réserves et terrains

export const RESERVES = {
  mc: "zone-de-stockage-mc-jaune",
  heat: "zone-de-stockage-chaleur-rouge",
  plants: "zone-de-stockage-plantes-verte",
};

export function imageReserve(cle) {
  return RESERVES[cle] ? piece(RESERVES[cle]) : null;
}

// Les neuf océans du jeu : neuf tuiles réellement imprimées, dans l'ordre où
// elles se posent. Aucune n'est décorative — chacune est un océan gagné.
export const TUILES_OCEAN = [
  "tuile-ocean-bonus-2-plantes",
  "tuile-ocean-bonus-1-carte",
  "tuile-ocean-bonus-4-mc",
  "tuile-ocean-bonus-1-plante-et-1-mc",
  "tuile-ocean-terrain-aride-sans-bonus",
  "tuile-ocean-bonus-1-mc",
  "tuile-ocean-bonus-1-carte-et-1-plante",
  "tuile-ocean-bonus-1-plante-et-2-mc",
  "tuile-ocean-bonus-2-plantes",
];

export function imageOcean(i) {
  return piece(TUILES_OCEAN[i % TUILES_OCEAN.length]);
}

export function imageForet() {
  return piece("tuile-foret-compteur-hexagone-arbre");
}

export function dosDeCarte() {
  return piece("dos-de-carte-cite-sous-dome-et-dirigeables");
}

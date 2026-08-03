// Le matériel imprimé — le pont entre les mots du moteur et les images fournies.
//
// Le moteur parle en anglais et en nombres : « Space Station », « 2B »,
// « Terraformer », « SPACE ». Les 374 images fournies portent des noms français
// de découpe. Ce module fait la jointure, et RIEN d'autre : il ne décide rien,
// ne calcule rien, n'invente aucune valeur.

// COUTURE — deux chantiers ont écrit dans ce fichier, en deux endroits qui ne
// se touchent pas, et les deux apports sont repris entiers :
//
//   · `bandeau-et-monde` — la section des neuf océans : `NB_OCEANS`, `dosOcean`,
//     `faceOcean`, `cleOcean`. Elle REMPLACE l'ancienne table `TUILES_OCEAN` et
//     l'ancien `imageOcean(i)`, qui distribuaient une face par rang sans jamais
//     demander au moteur laquelle était révélée. `imageOcean()` survit sous le
//     même nom, sans argument : `vue/scene.js` (chantier `table-vivante`)
//     l'appelle encore pour illustrer une action, et il rend désormais le dos.
//   · `table-vivante` — les deux dos, en bas de fichier : `dosDeCarte` se scinde
//     en `dosProjet` et `dosCorporation`. Plus aucun appelant ne demande
//     `dosDeCarte` : `vue/cartes.js`, `vue/mains.js` et `vue/scene.js` sont eux
//     aussi repris de `table-vivante`, qui les avait déjà mis à jour.

import { STAGES, BADGE_EN, MOT } from "./mots.js";

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

// Le nom de découpe de l'image est français (c'est le nom du fichier fourni) ;
// le nom AFFICHÉ vient de `mots.js`, en anglais, comme tout ce que le joueur lit.
const PHASES = {
  1: { slug: "1-development" },
  2: { slug: "2-construction" },
  3: { slug: "3-action" },
  4: { slug: "4-production" },
  5: { slug: "5-research" },
};

export function phaseNom(n) {
  return STAGES[n] ? STAGES[n].nom : "—";
}

export function phaseRomain(n) {
  return STAGES[n] ? STAGES[n].romain : "—";
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
// Le nom de découpe du jeton est français ; le nom affiché vient de `mots.js`.
const BADGES = {
  BUILDING: "batiment",
  SPACE: "espace-soleil",
  SCIENCE: "science",
  PLANT: "plante",
  ENERGY: "energie",
  EARTH: "terre",
  JUPITER: "jupiter",
  MICROBE: "microbe",
  ANIMAL: "animal",
  EVENT: "evenement",
};

export const ORDRE_BADGES = Object.keys(BADGES);

export function imageBadge(cle, grand = false) {
  const b = BADGES[cle];
  if (!b) return null;
  return piece(`jeton-tag-${b}${grand ? "-grand" : ""}-decouverte`);
}

export function nomBadge(cle) {
  return BADGE_EN[cle] || cle;
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
  { suit: "astronautes-combinaisons-rouges", teinte: "#e2542b", nom: "RED" },
  { suit: "astronautes-combinaisons-bleues", teinte: "#3d9fd6", nom: "BLUE" },
];

/** Le nom court d'un joueur, tel qu'il s'écrit à l'écran. */
export function nomJoueur(j) {
  return MOT.players[j] ?? "P" + j;
}

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

// ------------------------------------------------------------- les neuf océans

/** Le nombre d'emplacements de la planche. C'est `state::NUM_OCEANS`. */
export const NB_OCEANS = 9;

/**
 * LE DOS DES TUILES — la face orange du livret (l. 72 : « choisissez une tuile
 * Océan dont la face orange est visible »). C'est le scan
 * `tuile-ocean-terrain-aride-sans-bonus`, recopié sous un nom NEUTRE : aucune
 * des neuf tuiles du moteur n'a un bonus nul, cette image ne peut donc pas être
 * une face — et son nom de découpe contient le mot « bonus », qui aurait
 * suffi à faire fuiter une tuile encore retournée par son seul `src`.
 */
export function dosOcean() {
  return piece("tuile-ocean-dos-orange");
}

/**
 * LA FACE D'UNE TUILE RÉVÉLÉE, par son bonus tel que le moteur le publie
 * (`oceans_revealed_tiles[] = {id, cards, mc, plants}`). On ne devine pas :
 * la clef est le triplet exact, et les sept triplets de `state::OCEAN_TILES`
 * ont chacun leur scan.
 *
 * Deux noms de découpe trompent, et c'est le scan qui tranche :
 * `…bonus-1-mc` porte une carte ET un MC (1/1/0), pas un MC seul ; c'est la
 * seule tuile du jeu à mêler les deux.
 */
const FACES_OCEAN = {
  "0-0-2": "tuile-ocean-bonus-2-plantes",
  "0-4-0": "tuile-ocean-bonus-4-mc",
  "1-1-0": "tuile-ocean-bonus-1-mc",
  "0-2-1": "tuile-ocean-bonus-1-plante-et-2-mc",
  "1-0-1": "tuile-ocean-bonus-1-carte-et-1-plante",
  "1-0-0": "tuile-ocean-bonus-1-carte",
  "0-1-1": "tuile-ocean-bonus-1-plante-et-1-mc",
};

/** La clef d'un bonus publié par le moteur. */
export function cleOcean(t) {
  return `${t.cards | 0}-${t.mc | 0}-${t.plants | 0}`;
}

/**
 * L'image de la face d'une tuile révélée, ou `null` si le moteur publie un
 * bonus dont aucun scan ne rend compte. On rend `null` plutôt qu'une image
 * approchante : montrer « terrain aride » pour « 1 carte et 1 MC » serait
 * afficher un bonus qui n'est pas celui du moteur.
 */
export function faceOcean(tuile) {
  const nom = FACES_OCEAN[cleOcean(tuile)];
  return nom ? piece(nom) : null;
}

/**
 * LE JETON OCÉAN GÉNÉRIQUE — l'image qui dit « un océan », sans en désigner un.
 * C'est la tuile face orange : celle qu'on prend sur la planche pour la
 * retourner. Elle sert d'illustration aux actions du jeu (« reveal an ocean »),
 * là où montrer une face précise annoncerait un bonus que personne n'a encore
 * gagné.
 */
export function imageOcean() {
  return dosOcean();
}

export function imageForet() {
  return piece("tuile-foret-compteur-hexagone-arbre");
}

// DEUX DOS, ET CHACUN LE SIEN. Le jeu imprime deux dos différents, et les
// confondre fait mentir l'écran : une main d'adversaire couverte de dos de
// corporation annonce des corporations qu'il ne tient pas. L'attribution vient du
// joueur, qui a la boîte en main (02-08).
//
//   campement martien sur Mars aride -> les cartes PROJET
//   cité verte sous un dôme          -> les cartes CORPORATION

/** Le dos d'une carte projet — celui des mains, des pioches, des défausses. */
export function dosProjet() {
  return piece("dos-de-carte-campement-martien-et-dirigeables");
}

/** Le dos d'une carte corporation — et de rien d'autre. */
export function dosCorporation() {
  return piece("dos-de-carte-cite-sous-dome-et-dirigeables");
}

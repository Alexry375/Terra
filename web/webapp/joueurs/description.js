// **(le-juge-apprend) CE QUE LE RÉSEAU VOIT — la description d'une situation,
// côté JavaScript.**
//
// C'est le jumeau exact de `engine/src/description.rs`. Les poids sont APPRIS en
// Rust et RELUS ici : si les deux côtés ne rangent pas les mêmes nombres dans le
// même ordre, les poids ne veulent plus rien dire une fois relus, et le joueur
// est mauvais sans qu'on comprenne pourquoi. C'est le risque numéro un du
// chantier, et il est tenu par trois choses :
//
//   1. UNE SEULE SOURCE D'ORDRE de ce côté-ci aussi : `parcours()` est écrit une
//      fois et sert aux valeurs (`decrire`) comme aux noms (`nomsDesEntrees`).
//      Un nom ne peut donc pas se décaler d'un rang par rapport à sa valeur.
//   2. LE MÊME ORDRE QU'EN RUST, ligne pour ligne, avec les mêmes seuils —
//      recopiés du même relevé (1000 parties, graines 100000-100999).
//   3. LE VERROU DU §7 : le fichier de poids porte le nom de chaque entrée, et
//      `apprenti.js` régénère les siens au chargement pour les comparer un par
//      un. Au premier écart, il refuse de jouer et dit lequel.
//
// **Ce module ne connaît aucune règle du jeu** : il lit l'état publié par le
// moteur (`engine::observe::state_view`) et rien d'autre. Ce qu'une carte coûte,
// ce qu'un score vaut, quelles cartes sont payables — tout cela est déjà calculé
// par le moteur et publié.
//
// **Il ne lit JAMAIS la main de l'adversaire.** Le moteur publie les deux mains
// (« mode bac à sable ») ; une seule fonction ici accède aux joueurs, `joueur()`,
// et elle prend le siège en paramètre. De l'adversaire, le parcours ne prend que
// le NOMBRE de cartes en main — jamais leur identité.

import { PROJETS, CORPORATIONS } from "./paquet.js";

// ────────────────────────────────────── les seuils (mesurés, pas préférés)
//
// `engine/src/bin/mesures.rs --parties 1000 --graine-debut 100000` : un seuil
// n'est retenu que si la fraction des situations qui le franchissent tombe entre
// 2 % et 98 % (§3.5). Ces tableaux sont la copie conforme de ceux de
// `engine/src/description.rs`.

export const S_GENERATION = [1, 8, 15, 22, 28, 34, 40, 50];
export const S_TEMPERATURE = [0, 1, 2, 5, 8, 13, 18];
export const S_OXYGENE = [0, 1, 2, 5, 7, 11, 13];
export const S_OCEANS = [0, 1, 3, 4, 6, 8];
export const S_PIOCHE = [14, 64, 95, 122, 147, 171, 195, 222];
export const S_DEFAUSSE = [6, 20, 38, 58, 79, 101, 127, 169];

export const S_MC = [0, 3, 8, 14, 22, 33, 53, 144];
export const S_CHALEUR = [0, 1, 3, 5, 8, 14, 49];
export const S_PLANTES = [0, 1, 2, 4, 5, 8, 18];
export const S_PROD_MC = [0, 1, 2, 4, 6, 11];
export const S_PROD_CHALEUR = [0, 1, 2, 4, 5, 8, 14];
export const S_PROD_PLANTES = [0, 1, 2, 3, 5];
export const S_PROD_CARTES = [0, 1, 2];
export const S_NT = [5, 6, 8, 10, 13, 17, 22, 30];
export const S_FORETS = [0, 1, 2, 3, 5, 9];
// (2.9) L'échelle de score, désaturée — copie conforme de `S_SCORE`
// (`engine/src/description.rs`). Aucun intervalle entre deux paliers consécutifs
// n'atteint 8 : deux joueurs séparés de 8 points ne peuvent plus tomber dans la
// même case, sauf au-delà du dernier palier — lequel monte à 147 pour que cette
// dernière case, la seule qui puisse encore confondre deux joueurs, se referme
// sur les scores réellement atteints.
export const S_SCORE = [
  5, 6, 8, 11, 17, 25, 29, 34, 38, 43, 48, 54, 59, 65, 71, 77, 83, 91, 99, 107, 115, 123, 131, 139,
  147,
];
export const S_MAIN = [6, 8, 9, 10, 12];
export const S_POSEES = [0, 3, 6, 8, 11, 15, 18, 26];
export const S_ACIER = [0, 1, 2, 3];
export const S_TITANE = [0, 1, 3];
export const S_REPERES = [0, 1, 2];
export const S_PAYABLE = [0, 3, 6, 8, 9, 10, 11];
export const S_PAYABLE_VERTE = [0, 1, 2, 3, 4, 5, 7];
export const S_PAYABLE_BLEUE = [0, 1, 2, 3, 5];
export const S_PAYABLE_ROUGE = [0, 1, 2, 3];

// ─── les seuils des séries neuves du lot 3 ────────────────────────────────
//
// `mesures --parties 200 --graine-debut 200001 --poids data/poids/apprenti-1M.txt
// --boites base,decouverte --seuils 8`, 152 752 observations. Copie conforme de
// `engine/src/description.rs` — toute divergence est refusée au chargement par
// le verrou des noms du §7.

/** (2.9) Les six écarts, dans l'ordre de `NOMS_ECARTS`. Un écart est signé. */
export const S_ECARTS = [
  [-41, -15, -4, -1, 0, 3, 14, 40], // score_acquis
  [-22, -7, -2, -1, 0, 1, 6, 21], // nt
  [-13, -6, -3, -1, 0, 2, 5, 12], // posees
  [-121, -26, -12, -4, 3, 11, 25, 120], // mc
  [-9, -3, -2, -1, 0, 1, 2, 8], // prod_mc
  [-8, -2, -1, 0, 1, 7], // forets
];

/** (2.9) Les noms des six écarts, dans le même ordre. */
export const NOMS_ECARTS = ["score_acquis", "nt", "posees", "mc", "prod_mc", "forets"];

/** (2.8) Badges de MA main, dans l'ordre de `BADGES`. */
export const S_MAIN_BADGES = [
  [0, 1, 2, 3, 4, 5], // BUILDING
  [0, 1, 2, 3, 5, 6], // SPACE
  [0, 1, 2, 3, 4], // SCIENCE
  [0, 1, 2], // PLANT
  [0, 1], // MICROBE
  [0, 1], // ANIMAL
  [0, 1, 2], // EARTH
  [0, 1, 2, 3], // JUPITER
  [0, 1, 2], // ENERGY
  [0, 1, 2, 3, 4], // EVENT
];

/** (2.8) Couleurs de MA main : verte, bleue, rouge. */
export const S_MAIN_COULEURS = [
  [0, 1, 2, 3, 4, 5, 7], // verte
  [0, 1, 2, 3, 4], // bleue
  [0, 1, 2, 3, 4], // rouge
];

export const S_MAIN_PV = [0, 1, 2, 3, 4, 6, 9];
export const S_MAIN_PRIX_TOTAL = [14, 72, 101, 119, 136, 154, 177, 227];
export const S_MAIN_PRIX_MIN = [0, 2, 3, 4, 5, 7, 9, 16];
export const S_RESSOURCES_POSEES = [0, 3, 13];

/**
 * (2.8) Le prix annoncé pour la carte la moins chère d'une main VIDE. Répondre 0
 * dirait « j'ai une carte gratuite sous la main », le contraire de la vérité.
 * Copie conforme de `description::PRIX_MAIN_VIDE`.
 */
export const PRIX_MAIN_VIDE = 99;

/** Les trois couleurs, dans l'ordre de `cards::Color::index`. */
export const COULEURS = ["verte", "bleue", "rouge"];

/** Un jeu de seuils par badge, dans l'ordre de `cards::JOKER_TAG_CHOICES`. */
export const S_BADGES = [
  [0, 1, 2, 4, 5, 7, 10], // BUILDING
  [0, 1, 2, 3, 4, 6], // SPACE
  [0, 1, 2, 4, 6], // SCIENCE
  [0, 1, 2, 3], // PLANT
  [0, 1, 2, 3], // MICROBE
  [0, 1], // ANIMAL
  [0, 1, 2, 3, 4], // EARTH
  [0, 1, 2], // JUPITER
  [0, 1, 2, 4], // ENERGY
  [0, 1, 2, 3, 5], // EVENT
];

/** Les badges publiés par l'état, dans l'ordre de `cards::JOKER_TAG_CHOICES`. */
export const BADGES = [
  "BUILDING",
  "SPACE",
  "SCIENCE",
  "PLANT",
  "MICROBE",
  "ANIMAL",
  "EARTH",
  "JUPITER",
  "ENERGY",
  "EVENT",
];

/** Les types de repères du jeu, dans l'ordre de `state::MILESTONE_POOL`. */
export const REPERES = [
  "Builder",
  "Diversifier",
  "Energizer",
  "Farmer",
  "Legend",
  "Magnate",
  "Planner",
  "SpaceBaron",
  "Terraformer",
  "Tycoon",
  "Gardener",
];

/** Les types de récompenses, dans l'ordre de `state::AWARD_POOL`. */
export const RECOMPENSES = [
  "Celebrity",
  "Collector",
  "Generator",
  "Industrialist",
  "ProjectManager",
  "Researcher",
  "Visionary",
];

/** Les dix améliorations de carte Phase, dans l'ordre imprimé. */
export const AMELIORATIONS = ["1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B", "5A", "5B"];

// ─────────────────────────────────────────────────────── les deux collecteurs

/** Collecteur de VALEURS : +1 / −1, dans l'ordre du parcours. */
function collecteurValeurs() {
  const out = [];
  return {
    out,
    drapeau(a, n, b, c, v) {
      out.push(v ? 1 : -1);
    },
    thermo(a, n, b, c, q, seuils) {
      for (const s of seuils) out.push(q > s ? 1 : -1);
    },
  };
}

/** Collecteur de NOMS : la table des entrées du §3.4, celle du verrou du §7. */
function collecteurNoms() {
  const out = [];
  const nom = (a, n, b, c) => (n < 0 ? `${a}${b}${c}` : `${a}${n}${b}${c}`);
  return {
    out,
    drapeau(a, n, b, c) {
      out.push(nom(a, n, b, c));
    },
    thermo(a, n, b, c, q, seuils) {
      for (const s of seuils) out.push(`${nom(a, n, b, c)}>${s}`);
    },
  };
}

// ──────────────────────────────────────────────────────────────── le parcours

/** Rang d'une carte projet dans le vecteur, ou −1 si elle n'en a pas. */
const RANG_PROJET = new Map(PROJETS.map((id, i) => [id, i]));

/**
 * **La seule fonction qui accède aux joueurs**, et elle prend le siège en
 * paramètre : la triche est impossible par construction, pas par discipline.
 */
function joueur(etat, siege, moi) {
  return etat.players[moi ? siege : (siege + 1) % 2];
}

/**
 * **LE PARCOURS — la source unique de l'ordre des entrées de ce côté-ci.**
 * Copie conforme de `Description::parcours` (`engine/src/description.rs`).
 */
export function parcours(etat, siege, s) {
  // Les deux joueurs, liés une fois pour tout le parcours : la section a en a
  // besoin depuis que le classement des récompenses (2.10) y figure.
  const moi = joueur(etat, siege, true);
  const adv = joueur(etat, siege, false);

  // ─────────────────────────────────────────────────────────── a. le global
  s.drapeau("global_", -1, "", "fin_de_partie", etat.game_over === true);
  s.thermo("global_", -1, "", "generation", etat.generation, S_GENERATION);
  s.thermo("global_", -1, "", "temperature", etat.planet.temperature, S_TEMPERATURE);
  s.thermo("global_", -1, "", "oxygene", etat.planet.oxygen, S_OXYGENE);
  s.thermo("global_", -1, "", "oceans", etat.planet.oceans, S_OCEANS);
  s.thermo("global_", -1, "", "pioche", etat.decks.deck, S_PIOCHE);
  s.thermo("global_", -1, "", "defausse", etat.decks.discard, S_DEFAUSSE);

  // Un rang par TYPE de repère, jamais par position : trois sont tirés au hasard
  // à chaque partie, et le rang d'une entrée doit désigner la même chose partout.
  for (const nom of REPERES) {
    const slot = etat.milestones.find((m) => m.kind === nom);
    s.drapeau("repere_", -1, nom, "_present", slot !== undefined);
    s.drapeau("repere_", -1, nom, "_atteint", slot !== undefined && slot.achieved_by.some((x) => x));
    s.drapeau("repere_", -1, nom, "_par_moi", slot !== undefined && slot.achieved_by[siege] === true);
  }
  // (2.10) Qui mène sur chaque récompense. Le barème n'est PAS recopié ici : le
  // moteur publie, joueur par joueur, ce que chaque tuile en jeu lui vaut
  // (`observe::player_view`, champ `valeurs_recompenses`, calculé par le point
  // unique `flow::award_value`). Ce module ne fait que comparer deux nombres.
  for (const nom of RECOMPENSES) {
    const presente = etat.awards.includes(nom);
    s.drapeau("recompense_", -1, nom, "_presente", presente);
    const vMoi = presente ? (moi.valeurs_recompenses || {})[nom] ?? 0 : 0;
    const vAdv = presente ? (adv.valeurs_recompenses || {})[nom] ?? 0 : 0;
    s.drapeau("recompense_", -1, nom, "_classement_je_mene", presente && vMoi > vAdv);
    s.drapeau("recompense_", -1, nom, "_classement_egalite", presente && vMoi === vAdv);
    s.drapeau("recompense_", -1, nom, "_classement_il_mene", presente && vMoi < vAdv);
  }
  for (let ph = 0; ph <= 5; ph++) {
    s.drapeau("phase_en_cours_", ph, "", "", etat.phase_en_cours === ph);
  }

  // ──────────────────────────────────────────── b. une entrée par carte projet
  //
  // Dans MA main, posée par moi, posée par l'adversaire, dans la défausse. La
  // défausse est publique et le comptage des cartes passées a été accordé
  // (§3.3) ; la main d'en face, elle, n'est jamais lue.
  const n = PROJETS.length;
  const dansMain = new Uint8Array(n);
  const poseMoi = new Uint8Array(n);
  const poseAdv = new Uint8Array(n);
  const defausse = new Uint8Array(n);
  for (const c of moi.hand) {
    const r = RANG_PROJET.get(c.id);
    if (r !== undefined) dansMain[r] = 1;
  }
  for (const c of moi.played) {
    const r = RANG_PROJET.get(c.id);
    if (r !== undefined) poseMoi[r] = 1;
  }
  for (const c of adv.played) {
    const r = RANG_PROJET.get(c.id);
    if (r !== undefined) poseAdv[r] = 1;
  }
  for (const c of etat.defausse) {
    const r = RANG_PROJET.get(c.id);
    if (r !== undefined) defausse[r] = 1;
  }
  for (let r = 0; r < n; r++) {
    const id = PROJETS[r];
    s.drapeau("projet", id, "", "_main", dansMain[r] === 1);
    s.drapeau("projet", id, "", "_pose_moi", poseMoi[r] === 1);
    s.drapeau("projet", id, "", "_pose_adv", poseAdv[r] === 1);
    s.drapeau("projet", id, "", "_defausse", defausse[r] === 1);
  }
  // La corporation de l'adversaire est publique une fois installée.
  // (D3) Les corporations que JE tiens en main — côté `moi_` seulement : la
  // paire tenue par l'adversaire est cachée. La corporation INSTALLÉE, elle,
  // est publique des deux côtés (les deux cases ci-dessus).
  const mesCorpos = moi.corps_en_main || [];
  for (const nom of CORPORATIONS) {
    s.drapeau("corpo_", -1, nom, "_moi", moi.corporation === nom);
    s.drapeau("corpo_", -1, nom, "_adv", adv.corporation === nom);
    s.drapeau("corpo_", -1, nom, "_ma_main", mesCorpos.includes(nom));
  }

  // ───────────────────────────────────────────────────────── c. par joueur, ×2
  // Le joueur qui regarde vient toujours en premier, l'adversaire ensuite (§3.2).
  for (const [prefixe, pl] of [
    ["moi_", moi],
    ["adv_", adv],
  ]) {
    s.thermo(prefixe, -1, "", "mc", pl.mc, S_MC);
    s.thermo(prefixe, -1, "", "chaleur", pl.heat, S_CHALEUR);
    s.thermo(prefixe, -1, "", "plantes", pl.plants, S_PLANTES);
    s.thermo(prefixe, -1, "", "prod_mc", pl.production.mc, S_PROD_MC);
    s.thermo(prefixe, -1, "", "prod_chaleur", pl.production.heat, S_PROD_CHALEUR);
    s.thermo(prefixe, -1, "", "prod_plantes", pl.production.plants, S_PROD_PLANTES);
    s.thermo(prefixe, -1, "", "prod_cartes", pl.production.cards, S_PROD_CARTES);
    s.thermo(prefixe, -1, "", "nt", pl.tr, S_NT);
    s.thermo(prefixe, -1, "", "forets", pl.forests, S_FORETS);
    s.thermo(prefixe, -1, "", "score_acquis", pl.score_acquis, S_SCORE);
    // De l'adversaire : le NOMBRE de cartes en main, jamais leur identité.
    s.thermo(prefixe, -1, "", "main", pl.hand.length, S_MAIN);
    s.thermo(prefixe, -1, "", "posees", pl.played.length, S_POSEES);
    // (2.10) Les ressources POSÉES SUR LES CARTES, tous types confondus. Le
    // moteur les publie carte par carte (`played[].resources`) ; ce module les
    // additionne, il ne sait pas ce qu'elles valent.
    let ressourcesPosees = 0;
    for (const c of pl.played) ressourcesPosees += c.resources || 0;
    s.thermo(prefixe, -1, "", "ressources_posees_total", ressourcesPosees, S_RESSOURCES_POSEES);
    for (let i = 0; i < BADGES.length; i++) {
      s.thermo(prefixe, -1, "badge_", BADGES[i], pl.tags[BADGES[i]] ?? 0, S_BADGES[i]);
    }
    s.thermo(prefixe, -1, "", "acier", pl.steel_capacity, S_ACIER);
    s.thermo(prefixe, -1, "", "titane", pl.titanium_capacity, S_TITANE);
    // (D1) La carte Phase TELLE QUE LA TABLE LA VOIT — `phase_revelee`, et non
    // plus `previous_phase`, exactement comme côté moteur
    // (`engine/src/description.rs`). Pendant l'étape de planification, c'est la
    // carte de la manche PRÉCÉDENTE : les cartes ne sont révélées qu'une fois
    // que tous les joueurs ont choisi (livret
    // `docs/regles/livret-base.md:268` et `:272`).
    const phaseVue = pl.phase_revelee;
    s.drapeau(
      prefixe,
      -1,
      "previous_phase_",
      "aucune",
      phaseVue === null || phaseVue === undefined,
    );
    for (let ph = 1; ph <= 5; ph++) {
      s.drapeau(prefixe, -1, "previous_phase_", String(ph), phaseVue === ph);
    }
    for (const a of AMELIORATIONS) {
      s.drapeau(prefixe, -1, "amelioration_", a, pl.phase_upgrades.includes(a));
    }
    const p = pl.player;
    s.thermo(
      prefixe,
      -1,
      "",
      "reperes_atteints",
      etat.milestones.filter((m) => m.achieved_by[p] === true).length,
      S_REPERES,
    );
  }

  // ────────────────────────────────────────────────────────── d. la jouabilité
  // Ce que je peux faire MAINTENANT. `main_payable` est publié par le moteur :
  // ce module ne sait pas ce qu'une carte coûte, et n'a pas à le savoir.
  const payable = moi.main_payable || [];
  s.thermo("moi_", -1, "", "main_payable", payable.filter((x) => x).length, S_PAYABLE);
  for (const [couleur, seuils, cle] of [
    ["verte", S_PAYABLE_VERTE, "payable_verte"],
    ["bleue", S_PAYABLE_BLEUE, "payable_bleue"],
    ["rouge", S_PAYABLE_ROUGE, "payable_rouge"],
  ]) {
    let k = 0;
    for (let i = 0; i < moi.hand.length; i++) {
      if (payable[i] && moi.hand[i].couleur === couleur) k++;
    }
    s.thermo("moi_", -1, "", cle, k, seuils);
  }

  // ──────────────────────────── e. (2.8) ce que MA main contient
  //
  // Six grandeurs, réservées au joueur qui regarde : aucune case `adv_main_`,
  // le CONTENU de la main d'en face est caché. Son NOMBRE de cartes
  // (`adv_main`) reste publié, il l'a toujours été.
  //
  // Les badges, la couleur, les points imprimés et le prix de chaque carte sont
  // publiés par le moteur (`observe::player_view`) : ce module ne connaît pas
  // le paquet, il lit la main telle qu'on la lui donne. Copie conforme de
  // `description::resume_main`.
  const badgesMain = new Array(BADGES.length).fill(0);
  const couleursMain = [0, 0, 0];
  let pvMain = 0;
  let prixMain = 0;
  let prixMinMain = PRIX_MAIN_VIDE;
  for (const c of moi.hand) {
    for (const t of c.tags || []) {
      const i = BADGES.indexOf(t);
      if (i >= 0) badgesMain[i]++;
    }
    const ic = COULEURS.indexOf(c.couleur);
    if (ic >= 0) couleursMain[ic]++;
    pvMain += c.vp || 0;
    prixMain += c.price || 0;
    if ((c.price || 0) < prixMinMain) prixMinMain = c.price || 0;
  }
  for (let i = 0; i < BADGES.length; i++) {
    s.thermo("moi_", -1, "main_badge_", BADGES[i], badgesMain[i], S_MAIN_BADGES[i]);
  }
  for (let i = 0; i < COULEURS.length; i++) {
    s.thermo("moi_", -1, "main_couleur_", COULEURS[i], couleursMain[i], S_MAIN_COULEURS[i]);
  }
  s.thermo("moi_", -1, "", "main_pv_imprimes", pvMain, S_MAIN_PV);
  s.thermo("moi_", -1, "", "main_prix_total", prixMain, S_MAIN_PRIX_TOTAL);
  s.thermo("moi_", -1, "", "main_prix_min", prixMinMain, S_MAIN_PRIX_MIN);

  // ──────────────────────────── f. (2.9) les six écarts
  //
  // Une seule série : l'écart de l'adversaire est l'opposé du mien. Même ordre
  // et mêmes grandeurs que `description::ecarts`.
  const ecarts = [
    moi.score_acquis - adv.score_acquis,
    moi.tr - adv.tr,
    moi.played.length - adv.played.length,
    moi.mc - adv.mc,
    moi.production.mc - adv.production.mc,
    moi.forests - adv.forests,
  ];
  for (let i = 0; i < NOMS_ECARTS.length; i++) {
    s.thermo("ecart_", -1, "", NOMS_ECARTS[i], ecarts[i], S_ECARTS[i]);
  }
}

// ────────────────────────────────────────────────────── les deux points d'entrée

/**
 * Le vecteur de description de `etat`, du point de vue du siège `siege`.
 * @returns {number[]} des +1 et des −1, jamais autre chose (§3.1).
 */
export function decrire(etat, siege) {
  const c = collecteurValeurs();
  parcours(etat, siege, c);
  return c.out;
}

/**
 * La table des entrées : un nom par rang du vecteur, dans le même ordre.
 *
 * Elle ne dépend pas de l'état — seulement du paquet — mais le parcours, lui,
 * lit un état : on lui en donne donc un vide, dont toutes les valeurs sont
 * jetées. C'est ce qui garantit qu'il n'existe pas deux parcours à tenir
 * d'accord, mais un seul.
 */
export function nomsDesEntrees() {
  const c = collecteurNoms();
  parcours(ETAT_VIDE, 0, c);
  return c.out;
}

/** Un état sans partie : sert uniquement à dérouler le parcours pour les noms. */
const JOUEUR_VIDE = (p) => ({
  player: p,
  corporation: null,
  corps_en_main: [],
  valeurs_recompenses: {},
  mc: 0,
  heat: 0,
  plants: 0,
  tr: 0,
  forests: 0,
  production: { mc: 0, heat: 0, plants: 0, cards: 0 },
  steel_capacity: 0,
  titanium_capacity: 0,
  tags: {},
  hand: [],
  main_payable: [],
  played: [],
  chosen_phase: 0,
  previous_phase: null,
  phase_revelee: null,
  phase_upgrades: [],
  score: 0,
  score_acquis: 0,
});

const ETAT_VIDE = {
  generation: 0,
  game_over: false,
  phase_en_cours: 0,
  planet: { temperature: 0, oxygen: 0, oceans: 0 },
  decks: { deck: 0, discard: 0 },
  defausse: [],
  milestones: [],
  awards: [],
  players: [JOUEUR_VIDE(0), JOUEUR_VIDE(1)],
};

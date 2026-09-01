// **(le-juge-apprend) `apprenti` — LE JOUEUR QUI A APPRIS À JUGER UNE SITUATION.**
//
// Il n'a aucune échelle de valeur écrite à la main, aucune règle du jeu, aucun
// barème : à chaque point de décision il essaie **chaque option que le moteur a
// énumérée**, demande au moteur l'état qui en résulterait, décrit cet état
// (`description.js`) et le fait passer dans le réseau entraîné en Rust
// (`engine/src/bin/entraine.rs`). Il garde l'option dont SA probabilité de
// victoire est la plus haute. Rien d'autre ne décide à sa place — remplacez les
// poids, et le joueur change du tout au tout.
//
// ─────────────────────────────────────────────────────────────────────────────
// **LE VERROU DU §7, ET POURQUOI IL EXISTE.**
//
// Les poids sont appris en Rust et relus ici. Si les deux côtés ne rangeaient pas
// les mêmes nombres dans le même ordre, les poids ne voudraient plus rien dire et
// le joueur serait mauvais sans qu'on comprenne pourquoi. Le fichier de poids
// porte donc le NOM de chacune de ses entrées ; au chargement, ce module
// régénère les siens (`nomsDesEntrees()`) et les compare un par un. Au premier
// écart, il REFUSE DE JOUER et dit lequel.
//
// ─────────────────────────────────────────────────────────────────────────────
// **COMMENT IL OBTIENT « L'ÉTAT QUI EN RÉSULTERAIT ».**
//
// Le pont sait rejouer une partie depuis sa graine avec n'importe quelle liste de
// décisions (`pont.pas`) : « la partie EST la graine plus la liste des décisions »
// (`adversaire.md`). Essayer un coup sans le jouer, c'est donc
// `pont.pas(graine, boites, [...décisions déjà prises, la réponse essayée])`.
//
// ─────────────────────────────────────────────────────────────────────────────
// **LE REPÈRE DU §4.1 : TOUTES LES OPTIONS SONT JUGÉES AU MÊME INSTANT.**
//
// Appliquer l'option ne suffit pas. L'état qui en résulte ne se trouve pas au
// même endroit de la partie selon l'option : « poser une carte » mène à la
// décision d'après, « passer » mène beaucoup plus loin — la phase se termine, la
// production est encaissée, la manche suivante commence — et plus tard paraît
// toujours meilleur, puisqu'on a encaissé sa production entre-temps. Le joueur du
// round 1 avait ainsi appris à ATTENDRE : 1001 générations sans jamais
// terraformer ni finir la partie.
//
// Après l'option, on avance donc jusqu'au **prochain point de décision du joueur
// qui choisit** — ou jusqu'à la fin de la partie. `pont.pas` dit de qui est la
// prochaine décision ; tant que ce n'est pas la mienne, on répond à la place de
// l'autre (`reponseParDefaut`, la première option, comme le Rust) et on rappelle
// `pas`. Jamais plus de soixante pas.
//
// Mais un fournisseur ne reçoit que `(decision, etat)` : ni la graine de la
// partie, ni la liste des décisions, ni le pont — et `partie.js` comme `pont.js`
// sont hors territoire. Ce module reçoit donc le pont par la ligne que le prompt
// autorise dans `duel.mjs`, et **enveloppe `pont.pas` en simple observateur** :
// l'enveloppe délègue à la fonction d'origine, ne touche à aucune valeur, et se
// contente de relever la graine, les boîtes et le tableau vivant des décisions
// que `partie.js` tient déjà. Aucune information cachée n'est lue au passage :
// une décision est un indice dans des options que le moteur a publiées.
//
// Sans pont, le joueur ne peut rien essayer : il le dit une fois et répond alors
// la première option (voir `MODE_DEGRADE` plus bas).
//
// ─────────────────────────────────────────────────────────────────────────────
// **IL NE REGARDE PAS LA MAIN D'EN FACE.** Mieux : il ne lit pas du tout l'`etat`
// qu'on lui passe. Il ne juge que des états qu'il obtient lui-même du moteur, et
// `description.js` n'y prend de l'adversaire que le NOMBRE de ses cartes. Un banc
// qui lui repose la même question avec une autre main adverse obtient donc
// exactement la même réponse.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { formeDeLaReponse, nombreDeChoix } from "../fournisseurs.js";
import { decrire, nomsDesEntrees } from "./description.js";

const ICI = dirname(fileURLToPath(import.meta.url));
/** Le chemin est calculé depuis L'EMPLACEMENT DU MODULE, jamais depuis le
 *  répertoire courant : la balance, les bancs et la page ne sont pas lancés du
 *  même endroit (§9). */
export const POIDS_PAR_DEFAUT = resolve(ICI, "../../../data/poids/apprenti.txt");

/**
 * **La marge de départage.** Deux options peuvent mener à des situations
 * rigoureusement identiques ; leurs notes sont alors égales en arithmétique réelle,
 * mais pas forcément au dernier bit — le Rust met ses sommes cachées à jour par
 * DIFFÉRENCES (l'optimisation du §1.1) et arrive à 0,46726923574014506 là où ce
 * module, qui refait chaque calcul en entier, arrive à 0,46726923574014501. Sans
 * marge, les deux joueurs ne choisissent plus la même option sur ces égalités-là.
 * On n'écarte donc une option que si elle est meilleure d'une marge qui dépasse
 * franchement le bruit de calcul ; à égalité, la PREMIÈRE l'emporte. La même règle
 * et la même marge sont écrites dans `engine/src/joueur.rs`.
 */
export const MARGE = 1e-12;

/**
 * **La largeur de couche cachée PAR DÉFAUT du dépôt : cinquante neurones.**
 * Miroir exact de `reseau::CACHES` — les deux doivent changer ensemble, et un
 * test du moteur le tient (`le_miroir_javascript_porte_la_meme_largeur_de_couche_cachee`).
 *
 * Depuis le lot « la largeur réglable », ce n'est plus la seule largeur possible :
 * un fichier de poids porte la sienne en tête (§7 : `entrées caches sorties`) et
 * `lirePoids` la lit de là, comme le fait `Reseau::lire` côté Rust. Cette
 * constante reste la largeur du dépôt — celle de `data/poids/` — et c'est la
 * largeur qu'exige `lirePoidsLargeur` quand on ne lui en nomme pas d'autre.
 */
const CACHES_ATTENDUS = 50;
/**
 * **Le plafond de largeur accepté en relecture**, même valeur et même raison que
 * `reseau::LARGEUR_MAX` : la largeur d'un fichier est adoptée telle quelle, et un
 * fichier corrompu qui annoncerait un milliard de neurones ferait demander des
 * téraoctets avant qu'on s'aperçoive que le compte des poids ne suit pas.
 */
const LARGEUR_MAX = 10000;
const SORTIES_ATTENDUES = 2;
/**
 * **(il-devine §1) Le second réseau a CINQ sorties, une par carte Phase**, dans
 * l'ordre du moteur : la sortie `i` porte la phase `i + 1`.
 */
export const PHASES = 5;
/**
 * **(il-devine §3) La marge de départage des cinq sorties.** Même valeur et même
 * raison que `MARGE_PHASE` d'`engine/src/reseau.rs` : `Math.exp` et `f64::exp`
 * diffèrent d'un dernier bit sur environ une valeur sur dix, et un maximum sur
 * cinq probabilités n'a pas de marge. Voir `phaseLaPlusProbable`.
 */
export const MARGE_PHASE = 1e-12;

/**
 * **Le plafond d'avance du §4.1**, et il est impératif : « cette avance ne doit
 * jamais dépasser un nombre de pas fixé (prends soixante), sinon une option qui
 * déclenche une longue cascade ferait boucler le joueur. » La même valeur est
 * écrite dans `engine/src/rejeu.rs`.
 */
export const PLAFOND_AVANCE = 60;

/**
 * **(2.11) Jusqu'où l'énumération complète est permise à l'échange des cartes de
 * départ.** Huit cartes = 256 sous-ensembles, la taille exacte de la main de
 * départ. Au-delà, on retombe sur la construction carte par carte. Même valeur
 * que `joueur::LARGEUR_ENUMERATION` côté Rust — les deux doivent bouger ensemble.
 */
export const LARGEUR_ENUMERATION = 8;

/** Compteurs de l'avance, relevés par les bancs (le §4.1 demande le second). */
export let pasDAvance = 0;
export let plafondsAtteints = 0;
/**
 * **(il-devine §8) Combien de `pick_phase` adverses les avances ont
 * rencontrés** — compté que la devinette soit allumée ou non, parce que c'est ce
 * chiffre qui dit si elle peut servir à quelque chose. « Croire qu'un
 * `pick_phase` adverse est rencontré à chaque avance » est un des pièges
 * annoncés : il faut le mesurer avant de conclure.
 */
export let phasesRencontrees = 0;
export function razCompteursAvance() {
  pasDAvance = 0;
  plafondsAtteints = 0;
  phasesRencontrees = 0;
}

/**
 * **La réponse qu'on prête à l'AUTRE pendant l'avance du §4.1 : la première
 * option.**
 *
 * Le §4.1 laisse deux voies — le réseau lui-même, ou « un choix simple et
 * fixe ». C'est la première option qui est retenue, parce qu'elle est la seule
 * qui soit reproductible **à l'identique des deux côtés** : le vrai critère du
 * §4 est que le joueur Rust et le joueur JavaScript choisissent la même option
 * dans la même situation, et le hasard du moteur natif n'est pas accessible au
 * pont. Copie conforme de la politique `Premiere` d'`engine/src/rejeu.rs`.
 */
export function reponseParDefaut(d) {
  const forme = formeDeLaReponse(d);
  if (forme === "montant") return d.minimum ?? 0;
  if (forme === "multiple") {
    const n = d.options ? d.options.length : 0;
    return [...Array(Math.min(d.a_choisir ?? 0, n)).keys()];
  }
  return 0;
}

// ──────────────────────────────────────────────────────────── le fichier de poids

/**
 * Lit un fichier de poids du §7 et **vérifie le verrou** : la table des noms du
 * fichier doit être exactement celle que ce dépôt régénère.
 *
 * **La largeur de la couche cachée est LUE dans le fichier**, jamais imposée :
 * un couple de poids appris à cent neurones se rejoue ici à cent, comme côté
 * Rust (`Reseau::lire`). Ce qui est refusé, c'est un fichier INCOHÉRENT — dont le
 * nombre de poids ne correspond pas à la géométrie qu'il annonce — et, quand
 * `cachesAttendus` est nommé, un fichier d'une autre largeur que celle attendue.
 *
 * Supprimer ce verrou plutôt que le rendre juste rendrait muette la seule chose
 * qu'il protège : des poids appris sur une autre géométrie, relus sans un mot,
 * donnent un joueur qui répond n'importe quoi.
 *
 * @param {string} chemin
 * @param {number} sortiesAttendues
 * @param {?number} cachesAttendus  `null` : la largeur du fichier fait foi.
 */
export function lirePoids(chemin, sortiesAttendues = SORTIES_ATTENDUES, cachesAttendus = null) {
  const lignes = readFileSync(chemin, "utf8").split("\n");
  const tete = (lignes[0] || "").trim().split(/\s+/).map(Number);
  const [nEntrees, caches, sorties] = tete;
  if (!Number.isInteger(nEntrees) || !Number.isInteger(caches) || !Number.isInteger(sorties)) {
    throw new Error(`poids illisibles : première ligne « ${lignes[0]} » (§7 : entrées caches sorties)`);
  }
  if (caches < 1 || caches > LARGEUR_MAX) {
    throw new Error(
      `poids inattendus : ${caches} neurones cachés annoncés — une couche cachée en a au moins un ` +
        `et au plus ${LARGEUR_MAX} (${chemin})`,
    );
  }
  if (sorties !== sortiesAttendues) {
    throw new Error(
      `poids inattendus : ${sorties} sorties, ce réseau-là en attend ${sortiesAttendues} — ${chemin}`,
    );
  }
  if (cachesAttendus !== null && caches !== cachesAttendus) {
    throw new Error(
      `poids inattendus : ${caches} neurones cachés, ce réseau-là en attend ${cachesAttendus} ` +
        `— des poids appris sur une autre géométrie ne veulent rien dire ici (${chemin})`,
    );
  }
  const parties = Number((lignes[1] || "").trim());
  const noms = lignes.slice(2, 2 + nEntrees).map((l) => l.replace(/\r$/, ""));

  // LE VERROU. Une divergence entre les deux descriptions devient impossible à
  // ne pas voir : elle ne peut plus se manifester par « le joueur est
  // mystérieusement mauvais ».
  const attendus = nomsDesEntrees();
  if (attendus.length !== nEntrees) {
    throw new Error(
      `le fichier de poids décrit ${nEntrees} entrées, ce dépôt en produit ${attendus.length} : ` +
        `les poids ont été appris sur une AUTRE description (${chemin})`,
    );
  }
  for (let i = 0; i < nEntrees; i++) {
    if (noms[i] !== attendus[i]) {
      throw new Error(
        `divergence de description au rang ${i} : le fichier dit « ${noms[i] ?? "(rien)"} », ` +
          `ce dépôt produit « ${attendus[i]} » — les poids ne veulent plus rien dire (${chemin})`,
      );
    }
  }

  const total = (nEntrees + 1) * caches + (caches + 1) * sorties;
  const nombres = new Float64Array(total);
  let k = 0;
  // On parcourt le fichier JUSQU'AU BOUT, et pas seulement jusqu'à `total` : un
  // fichier qui MENT sur sa largeur en annonce moins qu'il n'en porte, et
  // s'arrêter au compte annoncé le laisserait passer sans un mot. C'est le
  // verrou de cohérence — le même que `Reseau::lire` côté Rust.
  for (let i = 2 + nEntrees; i < lignes.length; i++) {
    const t = lignes[i].trim();
    if (t === "") continue;
    if (k < total) nombres[k] = Number(t);
    k++;
  }
  if (k !== total) {
    throw new Error(
      `le fichier de poids porte ${k} nombres, il en faut ${total} pour ${nEntrees} entrées, ` +
        `${caches} neurones cachés et ${sorties} sorties (${chemin})`,
    );
  }
  const nCache = (nEntrees + 1) * caches;
  return {
    nEntrees,
    caches,
    sorties,
    parties,
    noms,
    // Rangés PAR ENTRÉE, comme en Rust : `wCache[i * caches + j]`, la ligne
    // `i === nEntrees` portant le biais d'entrée.
    wCache: nombres.subarray(0, nCache),
    wSortie: nombres.subarray(nCache),
  };
}

/**
 * **La relecture STRICTE : pour un juge dont la largeur est déjà fixée.**
 * Miroir de `Reseau::lire_largeur` côté Rust. Sans troisième argument, la largeur
 * exigée est celle du dépôt ([`CACHES_ATTENDUS`]) — un fichier appris à une autre
 * largeur est alors refusé en nommant les deux nombres.
 *
 * @param {string} chemin
 * @param {number} sortiesAttendues
 * @param {number} cachesAttendus
 */
export function lirePoidsLargeur(
  chemin,
  sortiesAttendues = SORTIES_ATTENDUES,
  cachesAttendus = CACHES_ATTENDUS,
) {
  return lirePoids(chemin, sortiesAttendues, cachesAttendus);
}

// ────────────────────────────────────────────────────────────────── le réseau

/**
 * L'évaluation du réseau : somme pondérée, tangente hyperbolique, puis
 * exponentielle normalisée. **La sortie 0 est la probabilité que gagne le joueur
 * du point de vue duquel la situation est décrite** — jamais « le siège 0 ».
 */
export function evaluer(poids, x) {
  const { nEntrees, caches, sorties, wCache, wSortie } = poids;
  const sommes = new Float64Array(caches);
  const biais = nEntrees * caches;
  for (let j = 0; j < caches; j++) sommes[j] = wCache[biais + j];
  for (let i = 0; i < nEntrees; i++) {
    const v = x[i];
    if (v === 0) continue;
    const base = i * caches;
    for (let j = 0; j < caches; j++) sommes[j] += v * wCache[base + j];
  }
  const h = new Float64Array(caches);
  for (let j = 0; j < caches; j++) h[j] = Math.tanh(sommes[j]);
  const s = new Float64Array(sorties);
  for (let k = 0; k < sorties; k++) {
    const base = k * (caches + 1);
    let acc = wSortie[base + caches];
    for (let j = 0; j < caches; j++) acc += h[j] * wSortie[base + j];
    s[k] = acc;
  }
  // **(il-devine) Le pivot, et pourquoi il n'est pas le même des deux côtés.**
  //
  // Pour le second réseau (cinq sorties), le §1 impose de retrancher LA PLUS
  // GRANDE des cinq valeurs : « sans cela une valeur de 800 fait un infini, et
  // toute la suite devient un pas un nombre ».
  //
  // Pour le premier réseau (deux sorties), c'est la PREMIÈRE valeur, et cela ne
  // bouge pas. Retrancher le maximum donnerait le même nombre en arithmétique
  // réelle mais pas au dernier bit — et les poids appris, le Rust et le contrôle
  // 10 dépendent de ce dernier bit. Copie conforme de `ReseauMulti::pivot`.
  let pivot = s[0];
  if (sorties !== SORTIES_ATTENDUES) {
    for (let k = 0; k < sorties; k++) if (s[k] > pivot) pivot = s[k];
  }
  let total = 0;
  const e = new Float64Array(sorties);
  for (let k = 0; k < sorties; k++) {
    e[k] = Math.exp(s[k] - pivot);
    total += e[k];
  }
  const p = new Array(sorties);
  for (let k = 0; k < sorties; k++) p[k] = e[k] / total;
  return p;
}

// ─────────────────────────────────────────────────────────────── la devinette

/**
 * **(il-devine §3, pas 3 et 4) LA LECTURE DES CINQ SORTIES.**
 *
 * `p` sont les cinq probabilités du second réseau, `p[i]` portant la phase
 * `i + 1`. `autorisees` sont les phases que le moteur autorise à ce joueur cette
 * manche — quatre sur cinq d'ordinaire, cinq à la toute première.
 *
 * 1. **mettre à zéro les phases non autorisées**, puis renormaliser sur celles
 *    qui restent. Ce n'est pas décoratif : une seule des cinq est interdite cette
 *    manche, et c'est souvent la plus probable — un joueur qui vient de jouer
 *    Production a de bonnes raisons de vouloir la rejouer, et il n'a pas le
 *    droit ;
 * 2. **rendre la plus probable** et, **en cas d'égalité, la plus petite** : on
 *    parcourt dans l'ordre croissant et on ne remplace la meilleure que si elle
 *    l'emporte de [`MARGE_PHASE`].
 *
 * **Pourquoi une marge et pas une égalité stricte.** Le §3 dit « égalité
 * stricte », et ce serait suffisant si les deux côtés calculaient exactement le
 * même nombre. Ils ne le font pas : `Math.exp` de Node diffère de `f64::exp` de
 * Rust d'un dernier bit sur environ une valeur sur dix (mesuré le 16-08 : 196
 * écarts sur 2000 tirages entre −3 et 3 ; `Math.tanh`, lui, concorde au bit
 * près). Sans marge, deux phases séparées d'un dernier bit se départageraient
 * d'un côté et pas de l'autre, et toute la partie divergerait — un maximum n'a
 * pas de marge, contrairement aux notes du premier réseau. La marge absorbe cet
 * écart (de l'ordre de 1e−16) et laisse intactes les différences réelles.
 *
 * **Copie conforme de `phase_la_plus_probable` d'`engine/src/reseau.rs`**, au
 * pas près : c'est ce que vérifie le banc `juge-meme-option-devinette.mjs`.
 */
export function phaseLaPlusProbable(p, autorisees) {
  const q = new Float64Array(PHASES);
  let total = 0;
  for (const ph of autorisees) {
    const i = ph - 1;
    if (i >= 0 && i < PHASES) {
      q[i] = p[i];
      total += p[i];
    }
  }
  if (total > 0) {
    for (let i = 0; i < PHASES; i++) q[i] /= total;
  } else {
    // Aucune phase autorisée ne porte de probabilité : on rend la plus PETITE
    // autorisée — même règle qu'à l'égalité juste en dessous. Déterministe.
    return autorisees.length ? Math.min(...autorisees) : 1;
  }
  let meilleure = 0;
  let valeur = -Infinity;
  for (let i = 0; i < PHASES; i++) {
    if (q[i] > valeur + MARGE_PHASE) {
      valeur = q[i];
      meilleure = i;
    }
  }
  return meilleure + 1;
}

/**
 * Est-ce une décision de carte Phase, et de qui ? Le descripteur du moteur porte
 * `type: "pick_phase"` et une option par phase autorisée, chacune avec son
 * numéro (`options[i].phase`) — c'est ainsi que `reflechi.js` la lit déjà.
 */
function estChoixDePhase(d) {
  return !!d && d.type === "pick_phase" && Array.isArray(d.options) && d.options.length > 0;
}

// ──────────────────────────────────────────────────────── l'espion du pont

/**
 * Enveloppe `pont.pas` en OBSERVATEUR : la fonction d'origine fait tout le
 * travail et rend tout ce qu'elle rendait ; on relève seulement avec quels
 * arguments la partie l'appelle. `decisions` est le tableau que `partie.js`
 * tient à jour — on en garde la référence, pas une copie.
 */
function espionner(pont) {
  if (pont.__espionApprenti) return pont.__espionApprenti;
  const origine = pont.pas.bind(pont);
  const espion = { graine: null, boites: null, decisions: null, origine };
  pont.pas = (graine, boites, decisions, essais) => {
    espion.graine = graine;
    espion.boites = boites;
    espion.decisions = decisions;
    return origine(graine, boites, decisions, essais);
  };
  Object.defineProperty(pont, "__espionApprenti", { value: espion, enumerable: false });
  return espion;
}

// ───────────────────────────────────────────────────────────── le fournisseur

/**
 * **Les poids déjà lus, rangés PAR CHEMIN.** La balance construit un joueur par
 * partie et par siège ; avec un seul emplacement de cache, deux joueurs aux poids
 * différents se chasseraient l'un l'autre et chaque partie relirait un fichier de
 * 1,4 Mo deux fois. Une table par chemin coûte quelques mégaoctets et supprime
 * trois cents relectures sur un duel de 150 graines.
 */
const _poidsParChemin = new Map();

function poidsEnCache(chemin, sorties) {
  const cle = `${sorties}:${chemin}`;
  let p = _poidsParChemin.get(cle);
  if (p === undefined) {
    p = lirePoids(chemin, sorties);
    _poidsParChemin.set(cle, p);
  }
  return p;
}

/**
 * Le fournisseur `apprenti`, de la même forme que `fournisseurReflechi`.
 *
 * @param {number} graine  acceptée pour respecter la signature des autres
 *   fournisseurs ; ce joueur ne tire rien au sort (l'exploration du §5 ne sert
 *   qu'à l'entraînement, jamais à la mesure).
 * @param {string} nom
 * @param {object|string} [poids]  poids déjà lus, ou chemin d'un fichier ;
 *   par défaut `data/poids/apprenti.txt`.
 * @param {object} [pont]   le pont, pour essayer les options (voir l'en-tête).
 * @param {string} [boites] la composition des boîtes de la partie.
 * @param {string} [adversaire] **(il-devine §4)** chemin du fichier du SECOND
 *   réseau, celui qui devine la carte Phase de l'autre. Laissé indéfini, la
 *   variable d'environnement `APPRENTI_ADVERSAIRE` en tient lieu. **Absent ou
 *   vide des deux façons : la devinette est ÉTEINTE** et le joueur se comporte
 *   exactement comme avant. Passer une chaîne vide éteint donc explicitement la
 *   devinette d'un seul côté, ce dont la balance a besoin pour n'allumer qu'un
 *   siège.
 */
export function fournisseurApprenti(
  graine,
  nom = "apprenti",
  poids,
  pont,
  boites,
  adversaire,
  graineEssais,
) {
  let p = poids;
  if (p === undefined || typeof p === "string") {
    // **Le défaut du contrat est `data/poids/apprenti.txt`, et il le reste.**
    // `APPRENTI_POIDS` n'est là que pour la MESURE : le §2.2 demande de comparer
    // trois rythmes croisés avec trois facteurs d'influence, soit neuf jeux de
    // poids. La balance lisant toujours le même fichier, il faudrait sinon les
    // essayer un par un en se les recopiant dessus — une demi-journée de
    // calendrier sur une machine à huit cœurs. Sans cette variable, le
    // comportement est celui que le point d'accroche n°4 décrit, au mot près.
    const chemin =
      typeof p === "string"
        ? p
        : (globalThis.process?.env?.APPRENTI_POIDS || POIDS_PAR_DEFAUT);
    p = poidsEnCache(chemin, SORTIES_ATTENDUES);
  }

  // **(il-devine §4) L'INTERRUPTEUR, CÔTÉ JAVASCRIPT.**
  //
  // « `APPRENTI_ADVERSAIRE` porte le chemin du fichier du second réseau. Absente
  // ou vide : la devinette est éteinte et le joueur se comporte exactement comme
  // aujourd'hui. » **Éteint est le défaut**, et il ne se déduit pas de la
  // présence d'un fichier sur le disque : il faut le nommer.
  const cheminAdversaire =
    adversaire === undefined
      ? (globalThis.process?.env?.APPRENTI_ADVERSAIRE || "")
      : (adversaire || "");
  const pAdversaire = cheminAdversaire ? poidsEnCache(cheminAdversaire, PHASES) : null;

  const espion = pont ? espionner(pont) : null;
  let degradeDit = false;

  // **(le-pont-ne-triche-plus) LA GRAINE DES REJEUX D'ESSAI.**
  //
  // Zéro par défaut, comme `joueur::GRAINE_ESSAIS_DEFAUT` : c'est une VALEUR, pas
  // une absence, et le pont rebat aussi bien avec elle qu'avec une autre. Deux
  // valeurs différentes donnent deux parties différentes à graine de partie
  // fixée ; une même valeur redonne toujours la même. La variable
  // `APPRENTI_GRAINE_ESSAIS` n'existe que pour les bancs qui ont besoin de la
  // faire varier — le défaut du contrat reste zéro, comme celui du binaire natif.
  const gEssais = Number(
    graineEssais !== undefined && graineEssais !== null
      ? graineEssais
      : (globalThis.process?.env?.APPRENTI_GRAINE_ESSAIS || 0),
  );

  /**
   * Le descripteur d'essai passé au pont : il dit AVEC QUELLE graine imaginer
   * l'avenir, et À QUEL INSTANT de la partie l'essai se place. Le rang est le
   * nombre de décisions déjà inscrites — l'exact jumeau de `journal.len()` côté
   * Rust — et `occasion` n'est renseigné que pour l'essai d'une vente.
   */
  function essaiDe(rang, occasion) {
    return occasion === undefined
      ? { graine: gEssais, rang }
      : { graine: gEssais, rang, occasion };
  }

  /**
   * **L'état atteint si l'on répondait `reponse`, AU REPÈRE DU §4.1.**
   *
   * Pas l'état qui suit immédiatement l'option : celui du **prochain point de
   * décision du joueur qui choisit**, ou la fin de la partie. Sans cela, les
   * options ne sont pas jugées au même instant de la partie — « passer » mène à
   * un état plus lointain, production encaissée et manche suivante entamée, que
   * « poser une carte » — et plus tard paraît toujours meilleur : le joueur
   * apprend à attendre (1001 générations mesurées au round 1, sans jamais
   * terraformer).
   *
   * Tant que la décision atteinte n'est pas la mienne, on répond **à la place de
   * l'autre** par [`reponseParDefaut`] — la première option, exactement comme la
   * politique `Premiere` du Rust — et on rappelle `pas`. L'avance ne dépasse
   * jamais [`PLAFOND_AVANCE`] pas : au-delà, on évalue là où on en est, et on le
   * compte.
   */
  /**
   * **(il-devine §3) La réponse prêtée à l'autre à un choix de carte Phase.**
   *
   * C'est **la seule décision de l'adversaire dont le traitement change** :
   * toutes les autres continuent de recevoir `reponseParDefaut`.
   *
   * 1. la description est prise **du point de vue du joueur qui décide**
   *    (`siege`), jamais de celui de l'adversaire qu'on prédit. Cette
   *    description-là contiendrait sa main, et un joueur qui lit la main d'en
   *    face triche (§1) ;
   * 2. elle passe dans le second réseau, qui rend cinq probabilités ;
   * 3. les phases que le moteur n'offre pas sont mises à zéro et le reste est
   *    renormalisé ;
   * 4. on rend la plus probable — et la réponse attendue par le moteur est
   *    l'INDICE de cette phase dans `d.options`, pas son numéro.
   *
   * Le Rust fait exactement la même chose au même endroit (`Rejeu::pick_phase`,
   * `Devinette::phase`).
   */
  function phaseDevinee(d, etat, siege) {
    const autorisees = d.options.map((o) => o.phase);
    const p = evaluer(pAdversaire, decrire(etat, siege));
    const phase = phaseLaPlusProbable(p, autorisees);
    const i = autorisees.indexOf(phase);
    // `phaseLaPlusProbable` ne rend jamais une phase hors des autorisées ; la
    // garde est là pour que la moindre surprise retombe sur le comportement
    // d'avant plutôt que sur un indice `-1` que le moteur refuserait.
    return i >= 0 ? i : reponseParDefaut(d);
  }

  function etatApres(reponse, siege) {
    const base = espion.decisions || [];
    return etatDe([...base, reponse], siege, essaiDe(base.length));
  }

  /**
   * L'état atteint au repère du §4.1 en rejouant `decisions`, l'avenir rebattu
   * selon `essais`. Toute l'avance vers le repère se fait avec LE MÊME
   * descripteur d'essai : sans cela, chaque pas de l'avance imaginerait un autre
   * avenir et les options ne seraient plus comparées sur le même tirage.
   */
  function etatDe(depart, siege, essais) {
    let decisions = depart;
    try {
      let r = espion.origine(espion.graine, espion.boites ?? boites, decisions, essais);
      let pas = 0;
      while (r && r.termine !== true && r.decision && r.decision.joueur !== siege) {
        if (pas >= PLAFOND_AVANCE) {
          plafondsAtteints++;
          break;
        }
        pas++;
        pasDAvance++;
        // (il-devine §3) La devinette, si elle est allumée et si c'est un choix
        // de carte Phase. Sinon, la première option, comme avant.
        let reponseDeLAutre;
        if (pAdversaire && estChoixDePhase(r.decision) && r.etat) {
          phasesRencontrees++;
          reponseDeLAutre = phaseDevinee(r.decision, r.etat, siege);
        } else {
          if (estChoixDePhase(r.decision)) phasesRencontrees++;
          reponseDeLAutre = reponseParDefaut(r.decision);
        }
        decisions = [...decisions, reponseDeLAutre];
        r = espion.origine(espion.graine, espion.boites ?? boites, decisions, essais);
      }
      return r && r.etat ? r.etat : null;
    } catch {
      // Le moteur a refusé cette réponse : elle n'est pas jouable, on l'écarte.
      return null;
    }
  }

  /** La note d'une réponse : MA probabilité de victoire dans l'état qui suit. */
  function noter(reponse, siege) {
    const etat = etatApres(reponse, siege);
    if (!etat) return -Infinity;
    // Toujours du point de vue du joueur QUI DÉCIDAIT, jamais de celui à qui la
    // main revient : une inversion ici donne un joueur qui joue contre lui-même.
    return evaluer(p, decrire(etat, siege))[0];
  }

  function meilleure(reponses, siege) {
    let choix = reponses[0];
    let note = -Infinity;
    for (const r of reponses) {
      const n = noter(r, siege);
      if (n > note + MARGE) {
        note = n;
        choix = r;
      }
    }
    return choix;
  }

  /**
   * **Un choix multiple.** Le moteur n'accepte que les combinaisons de la taille
   * exacte qu'il demande : une liste à moitié construite est refusée, pas
   * évaluée. Chaque candidat essayé doit donc être une réponse complète.
   *
   * - **nombre libre** (le mulligan projets) : toute liste vaut réponse, y
   *   compris la vide. **(2.11, le-joueur-sans-voyance)** On essaie désormais les
   *   2^n sous-ensembles quand n ne dépasse pas huit — 256 au mulligan de départ,
   *   au lieu des 37 que la construction carte par carte visitait au mieux. Elle
   *   partait de la liste vide, ajoutait la carte dont l'ajout améliore le plus et
   *   s'arrêtait au premier tour où aucune addition SEULE n'améliore : mesuré sur
   *   onze mains réelles, elle restait bloquée sur une solution moins bonne 6 fois
   *   sur 11. Au-delà de huit, on garde la construction carte par carte : le
   *   constat n° 7 de l'audit a mesuré que l'énumération coûte dix à seize fois
   *   plus cher sur les défausses de fin de manche (jusqu'à 19 448 combinaisons).
   * - **nombre imposé** : on part des k premières — complète, donc évaluable —
   *   et on essaie de REMPLACER chaque carte retenue par chacune des autres.
   *   Deux tours, ce qui borne le coût.
   *
   * Même énumération et même ordre de parcours que `Joueur::choisir_liste`
   * (`engine/src/joueur.rs`) — masques croissants, donc « ne rien rendre » en
   * premier et gagnant à égalité.
   *
   * **(le-pont-ne-triche-plus) Les deux écarts que le lot « le joueur sans
   * voyance » avait laissés ouverts sont refermés** : le pont accepte une graine
   * d'essais — `pont.pas(graine, boîtes, décisions, essais)` — et le fournisseur
   * reçoit les occasions de vente (méthode `vendre`, plus bas). `juge-meme-option.mjs`
   * peut donc redevenir vert, et c'est lui qui prouve la recopie.
   */
  function meilleureListe(d, siege) {
    const n = d.options ? d.options.length : 0;
    const libre = d.a_choisir === undefined || d.a_choisir === null;
    let pris = [];
    if (libre && n <= LARGEUR_ENUMERATION) {
      let meilleureNote = -Infinity;
      for (let masque = 0; masque < 1 << n; masque++) {
        const cand = [];
        for (let i = 0; i < n; i++) if ((masque >> i) & 1) cand.push(i);
        const x = noter(cand, siege);
        if (x > meilleureNote + MARGE) {
          meilleureNote = x;
          pris = cand;
        }
      }
      return pris;
    }
    if (libre) {
      let note = noter([...pris], siege);
      while (pris.length < n) {
        let meilleur = null;
        let meilleureNote = -Infinity;
        for (let i = 0; i < n; i++) {
          if (pris.includes(i)) continue;
          const x = noter([...pris, i], siege);
          if (meilleur === null || x > meilleureNote + MARGE) {
            meilleureNote = x;
            meilleur = i;
          }
        }
        if (meilleur === null || !(meilleureNote > note + MARGE)) break;
        pris.push(meilleur);
        note = meilleureNote;
      }
      return pris;
    }
    const attendu = Math.min(d.a_choisir, n);
    pris = [...Array(attendu).keys()];
    let note = noter([...pris], siege);
    for (let tour = 0; tour < 2; tour++) {
      let ameliore = false;
      for (let p = 0; p < pris.length; p++) {
        for (let c = 0; c < n; c++) {
          if (pris.includes(c)) continue;
          const ancien = pris[p];
          pris[p] = c;
          const x = noter([...pris], siege);
          if (x > note + MARGE) {
            note = x;
            ameliore = true;
          } else {
            pris[p] = ancien;
          }
        }
      }
      if (!ameliore) break;
    }
    return pris;
  }

  return {
    nom,
    poids: p,
    decider(d) {
      const siege = d.joueur ?? 0;
      const forme = formeDeLaReponse(d);
      if (!espion || espion.decisions === null) {
        // MODE_DEGRADE : sans pont, on ne peut essayer aucune option. On répond
        // alors la première, de façon parfaitement déterministe — et on le dit,
        // une fois, plutôt que de laisser croire que le réseau a joué.
        if (!degradeDit) {
          degradeDit = true;
          if (globalThis.process) {
            process.stderr.write(
              "apprenti : aucun pont fourni, le joueur ne peut essayer aucune option " +
                "(il répond la première ; voir l'en-tête de joueurs/apprenti.js)\n",
            );
          }
        }
        return reponseParDefaut(d);
      }
      if (forme === "multiple") return meilleureListe(d, siege);
      if (forme === "montant") {
        const min = d.minimum ?? 0;
        const max = d.maximum ?? 0;
        const rs = [];
        for (let v = min; v <= max; v++) rs.push(v);
        return meilleure(rs, siege);
      }
      const n = nombreDeChoix(d);
      const rs = [];
      for (let i = 0; i < n; i++) rs.push(i);
      return meilleure(rs, siege);
    },
    /**
     * **(le-pont-ne-triche-plus, 2.15) VENDRE, OU LAISSER PASSER L'OCCASION.**
     *
     * Le moteur ne pose pas de question ici : il ouvre une occasion, et l'occasion
     * s'évalue comme tout le reste, par des essais. On compare l'état atteint si
     * l'on ne vend rien à l'état atteint si l'on vend chaque carte de la main, une
     * par une, et l'on ne vend que si une carte fait STRICTEMENT mieux que ne rien
     * vendre — à égalité, on garde la carte. Même ordre de parcours, même marge et
     * même arbitrage que `Joueur::vendre_librement` (`engine/src/joueur.rs`).
     *
     * Tous ces essais portent le numéro de l'occasion : c'est lui qui entre dans
     * la graine du rejeu, pour que deux occasions déclinées de suite n'imaginent
     * pas le même avenir, et c'est lui que porte l'entrée si la vente est faite.
     *
     * @param {{numero: number, joueur: number, main: object[]}} occ
     * @returns {number[]} les indices vendus — liste vide si l'on ne vend rien.
     */
    vendre(occ) {
      if (!espion || espion.decisions === null) return [];
      const siege = occ.joueur;
      const base = espion.decisions;
      const essais = essaiDe(base.length, occ.numero);
      const notePour = (entree) => {
        const etat = etatDe(entree === null ? [...base] : [...base, entree], siege, essais);
        if (!etat) return -Infinity;
        return evaluer(p, decrire(etat, siege))[0];
      };
      let note = notePour(null);
      let choix = null;
      const n = (occ.main || []).length;
      for (let i = 0; i < n; i++) {
        const x = notePour({ vendre: { cartes: [i], joueur: siege, occasion: occ.numero } });
        if (x > note + MARGE) {
          note = x;
          choix = i;
        }
      }
      return choix === null ? [] : [choix];
    },
  };
}

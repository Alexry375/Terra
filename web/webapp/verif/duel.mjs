#!/usr/bin/env node
// LA BALANCE — deux fournisseurs nommés, l'un contre l'autre, et un verdict.
//
//   node web/webapp/verif/duel.mjs <joueurA> <joueurB> [graines] [boites]
//
// Elle répond à une seule question : **l'un des deux est-il meilleur, ou bien
// l'écart pourrait-il être dû au hasard des cartes ?** Sans elle, on ne pourra
// jamais dire qu'un adversaire artificiel a progressé.
//
// QUATRE PRÉCAUTIONS, ET AUCUNE N'EST DÉCORATIVE.
//
// 1. LES SIÈGES S'ÉCHANGENT. Chaque graine est jouée DEUX fois : une fois avec A
//    au siège 0, une fois avec A au siège 1. Sans cela on mesurerait l'avantage
//    du siège autant que la valeur du joueur — et le siège 1 n'est éprouvé par
//    aucun autre banc du dépôt.
//
// 2. LES DEUX JOUEURS NE PARTAGENT PAS LEUR HASARD. Deux fournisseurs aléatoires
//    construits sur la même graine feraient le MÊME tirage à chaque question :
//    la mesure ne voudrait plus rien dire. Chaque camp reçoit donc sa propre
//    graine, tirée de la graine de partie par deux mélanges différents
//    (`graineDuCamp`).
//
// 3. ELLE EST REPRODUCTIBLE AU CARACTÈRE PRÈS. Aucun appel à l'horloge, aucun
//    hasard non semé, aucun parcours de dossier : deux exécutions avec les mêmes
//    arguments impriment exactement les mêmes lignes. C'est ce qui permet de
//    comparer la mesure d'aujourd'hui à celle de demain.
//
// 4. ELLE DIT SI L'ÉCART VEUT DIRE QUELQUE CHOSE. 52 % sur 200 parties n'est pas
//    une victoire. Le calcul est écrit en clair plus bas, et le verdict est
//    imprimé en toutes lettres.
//
// Elle imprime enfin le nombre de décisions jouées : un banc qui n'a pas joué ne
// prouve rien.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { fournisseurReflechi } from "../joueurs/reflechi.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// LES JOUEURS QUE LA BALANCE CONNAÎT, par leur nom de ligne de commande. La
// table est écrite à la main, jamais découverte en parcourant un dossier : un
// banc dont la liste des joueurs dépend du contenu d'un répertoire ne se répète
// pas d'une machine à l'autre.
const JOUEURS = {
  hasard: (graine, nom) => fournisseurAleatoire(graine, nom),
  reflechi: (graine, nom) => fournisseurReflechi(graine, nom),
};

const BOITES_PAR_DEFAUT = "base,decouverte";

// ─────────────────────────────────────────────────────────── ligne de commande

const args = process.argv.slice(2);
const nomA = args[0];
const nomB = args[1];
const graines = args[2] === undefined ? 100 : Number(args[2]);
const boites = args[3] === undefined ? BOITES_PAR_DEFAUT : args[3];

function inconnu(nom) {
  // Une seule ligne, et elle NOMME les joueurs disponibles : c'est ainsi qu'un
  // contrôle extérieur peut savoir ce que la balance sait peser.
  console.log(
    `joueur inconnu : « ${nom} » — joueurs connus : ${Object.keys(JOUEURS).join(", ")}`,
  );
  process.exit(2);
}

if (!nomA || !nomB) {
  console.log(
    `il faut deux joueurs : duel.mjs <joueurA> <joueurB> [graines] [boites] — joueurs connus : ${Object.keys(JOUEURS).join(", ")}`,
  );
  process.exit(2);
}
if (!JOUEURS[nomA]) inconnu(nomA);
if (!JOUEURS[nomB]) inconnu(nomB);
if (!Number.isInteger(graines) || graines < 1) {
  console.log(`nombre de graines invalide : « ${args[2]} »`);
  process.exit(2);
}

// ────────────────────────────────────────────────────────────────── le duel

/**
 * La graine du CAMP, dérivée de la graine de partie par un mélange propre à
 * chaque camp. Deux camps ne peuvent donc pas tirer la même suite, et la suite
 * d'un camp ne dépend pas du siège où il est assis : c'est bien le joueur qu'on
 * compare, pas sa place.
 */
function graineDuCamp(graine, camp) {
  const sel = camp === 0 ? 0x9e3779b9 : 0x85ebca6b;
  let x = (graine * 2654435761) ^ sel;
  x = Math.imul(x ^ (x >>> 15), 0x2545f491) >>> 0;
  return x || 1;
}

const pont = await ouvrirPontDepuis(RACINE);

let victoiresA = 0;
let victoiresB = 0;
let nuls = 0;
let decisions = 0;
let ecartTotal = 0;

for (let g = 1; g <= graines; g++) {
  for (const echange of [false, true]) {
    const a = JOUEURS[nomA](graineDuCamp(g, 0), nomA);
    const b = JOUEURS[nomB](graineDuCamp(g, 1), nomB);
    const partie = creerPartie(pont, { graine: g, boites });
    await jouerJusquAuBout(partie, echange ? [b, a] : [a, b], () => {
      decisions++;
    });
    const scores = partie.scores || [0, 0];
    const scoreA = echange ? scores[1] : scores[0];
    const scoreB = echange ? scores[0] : scores[1];
    ecartTotal += scoreA - scoreB;
    if (scoreA > scoreB) victoiresA++;
    else if (scoreB > scoreA) victoiresB++;
    else nuls++;
  }
}

// ───────────────────────────────────────────── l'écart veut-il dire quelque chose ?
//
// LE CALCUL, EN LANGAGE SIMPLE. Si les deux joueurs se valaient exactement,
// chaque partie décisive serait un tirage à pile ou face. Sur `n` parties
// décisives, on attendrait donc `n / 2` victoires pour A, avec un écart typique
// (l'écart-type d'une pièce équilibrée) de `racine(n) / 2` — soit environ 7
// victoires sur 200 parties.
//
// On compte de combien d'écarts typiques on s'éloigne de la moitié :
//
//     ecarts = (victoires de A − n / 2) / (racine(n) / 2)
//
// Au-delà de DEUX écarts typiques, une pièce équilibrée ne produirait ce
// résultat que dans moins de 5 % des cas : on ne peut plus mettre l'écart sur le
// compte du hasard des cartes. En dessous, on ne sait pas trancher — et ne pas
// savoir doit se dire.
//
// Deux exemples, pour fixer l'échelle sur 200 parties : 107 victoires contre 89
// font moins de 1,3 écart typique (c'est du bruit) ; 150 victoires en font plus
// de sept.

const parties = graines * 2;
const decisives = victoiresA + victoiresB;
const attendu = decisives / 2;
const ecartTypique = Math.sqrt(decisives) / 2;
const ecarts = ecartTypique > 0 ? (victoiresA - attendu) / ecartTypique : 0;
const SEUIL_ECARTS = 2;
const significatif = Math.abs(ecarts) >= SEUIL_ECARTS;

const pourcent = (x) => ((100 * x) / parties).toFixed(1);

console.log(
  `duel : « ${nomA} » contre « ${nomB} » — ${graines} graines × 2 sièges = ${parties} parties (boîtes ${boites})`,
);
console.log(`« ${nomA} » gagne ${victoiresA} parties sur ${parties} (${pourcent(victoiresA)} %)`);
console.log(`« ${nomB} » gagne ${victoiresB} parties sur ${parties} (${pourcent(victoiresB)} %)`);
console.log(`nuls : ${nuls}`);
console.log(
  `écart de score moyen (« ${nomA} » − « ${nomB} ») : ${(ecartTotal / parties).toFixed(2)} point(s)`,
);
console.log(`décisions jouées : ${decisions}`);
console.log(
  `parties décisives : ${decisives} — attendu à l'équilibre : ${attendu.toFixed(1)} victoires, ` +
    `écart typique : ${ecartTypique.toFixed(1)}`,
);
console.log(`on est à ${ecarts.toFixed(2)} écart(s) typique(s) de l'équilibre (seuil : ${SEUIL_ECARTS})`);
console.log(
  significatif
    ? `verdict : écart significatif — « ${ecarts > 0 ? nomA : nomB} » est le meilleur des deux`
    : "verdict : dans le bruit — cet écart ne distingue pas les deux joueurs",
);

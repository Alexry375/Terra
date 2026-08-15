#!/usr/bin/env node
// **(le-juge-apprend) `apprenti` NE REGARDE PAS LA MAIN D'EN FACE.**
//
//   node web/webapp/verif/juge-main-cachee.mjs [décisions] [poids]
//
// Le moteur publie les DEUX mains (« mode bac à sable », `engine/src/observe.rs`).
// Un joueur qui lirait celle de l'adversaire paraîtrait brillant sans l'être, et
// serait intransposable à une partie contre un humain. Le contrat l'interdit
// absolument : de l'adversaire, on n'a droit qu'au NOMBRE de ses cartes.
//
// Ce banc l'éprouve **de l'extérieur, sans lire le code** — exactement comme le
// fait le banc du dépôt pour `reflechi` : on repose au fournisseur la même
// question dans deux états qui ne diffèrent que par la main d'en face, et on
// compare ses réponses. Il éprouve aussi la DESCRIPTION, qui est ce que le réseau
// voit : le vecteur doit être identique au bit près.
//
// La main de rechange est tirée de la défausse et de la main du joueur qui
// décide : des identifiants de cartes réels, pour que rien ne trahisse la
// substitution.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { fournisseurApprenti } from "../joueurs/apprenti.js";
import { decrire } from "../joueurs/description.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "../..");
const combien = Number(process.argv[2] ?? 60);
const poids = process.argv[3] ?? resolve(DEPOT, "data/poids/apprenti.txt");

const pont = await ouvrirPontDepuis(RACINE);

/** Une copie de l'état où la main de l'ADVERSAIRE a été remplacée. */
function autreMainAdverse(etat, siege) {
  const copie = structuredClone(etat);
  const adv = copie.players[(siege + 1) % 2];
  const rechange = [...copie.defausse, ...copie.players[siege].hand];
  if (rechange.length === 0) return null;
  // Autant de cartes qu'avant : le NOMBRE est la seule chose qu'un joueur
  // honnête a le droit de lire, il ne doit donc pas changer.
  adv.hand = adv.hand.map((_, i) => structuredClone(rechange[i % rechange.length]));
  adv.main_payable = adv.hand.map(() => false);
  return copie;
}

let compares = 0;
let differences = 0;
let vecteursDifferents = 0;
const exemples = [];

for (let g = 1; compares < combien && g <= 40; g++) {
  const joueur = fournisseurApprenti(1, "apprenti", poids, pont, "base,decouverte");
  const temoin = fournisseurAleatoire(g * 31 + 7, "hasard");
  const partie = creerPartie(pont, { graine: g, boites: "base,decouverte" });
  const aVoir = [];
  await jouerJusquAuBout(partie, [temoin, temoin], (p) => {
    if (aVoir.length < 200) aVoir.push({ d: p.decision, etat: structuredClone(p.etat) });
  });
  for (const { d, etat } of aVoir) {
    if (compares >= combien) break;
    const siege = d.joueur ?? 0;
    const truque = autreMainAdverse(etat, siege);
    if (!truque) continue;
    // 1. la DESCRIPTION ne doit pas bouger d'un bit
    const a = decrire(etat, siege);
    const b = decrire(truque, siege);
    let ecart = -1;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) {
        ecart = i;
        break;
      }
    }
    if (ecart >= 0) {
      vecteursDifferents++;
      if (exemples.length < 3) exemples.push(`description : rang ${ecart} change quand la main d'en face change`);
    }
    // 2. la RÉPONSE ne doit pas bouger non plus
    const r1 = JSON.stringify(joueur.decider(d, etat));
    const r2 = JSON.stringify(joueur.decider(d, truque));
    compares++;
    if (r1 !== r2) {
      differences++;
      if (exemples.length < 3) exemples.push(`réponse : ${d.type} rend ${r1} puis ${r2}`);
    }
  }
}

console.log(`${compares} question(s) reposées avec une autre main d'en face`);
console.log(`  descriptions qui changent : ${vecteursDifferents}`);
console.log(`  réponses qui changent     : ${differences}`);
for (const e of exemples) console.log(`  ✗ ${e}`);
if (compares < 20) {
  console.log("KO trop peu de questions comparées pour conclure");
  process.exit(1);
}
if (differences || vecteursDifferents) {
  console.log("KO le joueur regarde la main d'en face");
  process.exit(1);
}
console.log(`OK ni la description ni la réponse ne bougent quand la main d'en face change (${compares} questions)`);

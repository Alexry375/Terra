#!/usr/bin/env node
// **(le-juge-apprend) LES DEUX DESCRIPTIONS D'UNE MÊME SITUATION CONCORDENT.**
//
//   node web/webapp/verif/juge-descriptions.mjs [graines] [pas] [boites]
//
// C'est le banc du risque numéro un : les poids sont APPRIS en Rust et RELUS en
// JavaScript, et si les deux côtés ne rangent pas les mêmes nombres dans le même
// ordre, les poids ne veulent plus rien dire une fois relus.
//
// Il joue de VRAIES parties par le pont, s'arrête à des rangs réguliers, et
// compare, valeur par valeur :
//   · ce que `engine/src/bin/decrire` imprime pour cette situation ;
//   · ce que `joueurs/description.js` calcule pour la même.
//
// ─────────────────────────────────────────────────────────────────────────────
// **POURQUOI CE BANC EXISTE À CÔTÉ DU CONTRÔLE 01.**
//
// Le contrôle 01 fourni dans `inputs/checks/` crée sa partie avec
// `boites: ["base", "decouverte"]` — un TABLEAU. `partie.js` le passe tel quel à
// `pont.pas`, et le wasm lit ce champ par `chaine(v, "boites", "base")` : un
// tableau n'est pas une chaîne, la valeur est donc ignorée en silence et la
// partie se joue en boîte BASE SEULE, pendant que le binaire, lui, reçoit
// `--boites base,decouverte`. Les deux côtés ne décrivent alors pas la même
// partie, et aucune description correcte ne peut les faire concorder. Mesure :
//
//   pont.pas(101, "base", [])                  → pioche 208
//   pont.pas(101, "base,decouverte", [])       → pioche 246
//   pont.pas(101, ["base","decouverte"], [])   → pioche 208   ← la valeur est perdue
//
// Ce banc-ci passe donc les boîtes en CHAÎNE, comme le fait la balance du dépôt,
// et compare bien plus de situations que les dix du contrôle.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { decrire, nomsDesEntrees } from "../joueurs/description.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "../..");
const BIN = resolve(DEPOT, "engine/target/release/decrire");

const graines = Number(process.argv[2] ?? 12);
const pas = Number(process.argv[3] ?? 25);
const boites = process.argv[4] ?? "base,decouverte";
const TOLERANCE = 1e-9;

// LA TABLE DES NOMS, DES DEUX CÔTÉS — le verrou du §7. Si les deux côtés ne
// savent pas nommer leurs entrées à l'identique, le verrou ne verrouille rien.
const nomsJs = nomsDesEntrees();
const nomsRs = JSON.parse(
  execFileSync(BIN, ["--noms"], { cwd: DEPOT, maxBuffer: 1e9 }).toString().trim(),
).noms;
let fautes = 0;
console.log(`table des entrées : ${nomsRs.length} noms côté Rust, ${nomsJs.length} côté JavaScript`);
if (nomsRs.length !== nomsJs.length) {
  console.log(`  ✗ longueurs différentes`);
  fautes++;
} else {
  const ecarts = nomsRs.map((n, i) => (n === nomsJs[i] ? null : i)).filter((i) => i !== null);
  if (ecarts.length) {
    console.log(`  ✗ ${ecarts.length} nom(s) différents, le premier au rang ${ecarts[0]} : « ${nomsRs[ecarts[0]]} » contre « ${nomsJs[ecarts[0]]} »`);
    fautes++;
  } else {
    console.log(`  ✓ les ${nomsRs.length} noms concordent, rang par rang`);
  }
}
if (new Set(nomsJs).size !== nomsJs.length) {
  console.log(`  ✗ des noms sont en double : la table ne peut plus servir de verrou`);
  fautes++;
}

const pont = await ouvrirPontDepuis(RACINE);
let compares = 0;
const longueurs = new Set();
const vus = new Map();

for (let g = 1; g <= graines; g++) {
  const partie = creerPartie(pont, { graine: g, boites });
  const f0 = fournisseurAleatoire(g * 7 + 1, "a");
  const f1 = fournisseurAleatoire(g * 13 + 3, "b");
  const cas = [];
  let n = 0;
  await jouerJusquAuBout(partie, [f0, f1], (p) => {
    if (n % pas === 0) {
      const siege = p.decision.joueur ?? 0;
      cas.push({ rang: n, siege, decisions: p.decisions.slice(), entrees: decrire(p.etat, siege) });
    }
    n++;
  });
  for (const c of cas) {
    longueurs.add(c.entrees.length);
    vus.set(JSON.stringify(c.entrees), (vus.get(JSON.stringify(c.entrees)) ?? 0) + 1);
    const dec = c.decisions.map((d) => JSON.stringify(d)).join(",");
    let entreesRs;
    try {
      const sortie = execFileSync(
        BIN,
        ["--graine", String(g), "--decisions", dec, "--siege", String(c.siege), "--boites", boites],
        { cwd: DEPOT, maxBuffer: 1e9 },
      ).toString();
      entreesRs = JSON.parse(sortie.trim().split("\n").pop()).entrees;
    } catch (e) {
      console.log(`  ✗ graine ${g} rang ${c.rang} : decrire a échoué — ${String(e.message).split("\n")[0]}`);
      fautes++;
      continue;
    }
    compares++;
    if (entreesRs.length !== c.entrees.length) {
      console.log(`  ✗ graine ${g} rang ${c.rang} : ${entreesRs.length} entrées côté Rust, ${c.entrees.length} côté JavaScript`);
      fautes++;
      continue;
    }
    const ecarts = [];
    for (let i = 0; i < entreesRs.length; i++) {
      if (Math.abs(entreesRs[i] - c.entrees[i]) > TOLERANCE) ecarts.push(i);
    }
    if (ecarts.length) {
      const i = ecarts[0];
      console.log(`  ✗ graine ${g} rang ${c.rang} : ${ecarts.length} écart(s), le premier au rang ${i} (« ${nomsJs[i]} » : ${entreesRs[i]} contre ${c.entrees[i]})`);
      fautes++;
    }
    // §3.1 : toutes les valeurs valent +1 ou −1, jamais une quantité brute.
    const hors = c.entrees.filter((x) => Math.abs(Math.abs(x) - 1) > TOLERANCE).length;
    if (hors) {
      console.log(`  ✗ graine ${g} rang ${c.rang} : ${hors} valeur(s) qui ne sont ni +1 ni −1`);
      fautes++;
    }
  }
}

console.log(`${compares} situation(s) comparées sur ${graines} parties ; longueur du vecteur : ${[...longueurs]}`);
const jumeaux = [...vus.values()].filter((n) => n > 1).length;
console.log(`${vus.size} vecteur(s) distincts (${jumeaux} groupe(s) de situations jumelles)`);
if (longueurs.size > 1) {
  console.log(`  ✗ la description n'a pas une longueur fixe`);
  fautes++;
}
if (compares === 0) {
  console.log("KO aucune situation n'a pu être comparée");
  process.exit(1);
}
if (fautes) {
  console.log(`KO ${fautes} désaccord(s) entre les deux descriptions`);
  process.exit(1);
}
console.log(`OK les deux descriptions concordent à la valeur près sur ${compares} situations de vraies parties`);

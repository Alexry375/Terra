#!/usr/bin/env node
// **(le-pont-ne-triche-plus, critère F / défaut D23) LE MOTEUR ET LE NAVIGATEUR
// LISENT LE MÊME FICHIER DE CARTES, À L'OCTET.**
//
//   node web/webapp/verif/cartes-identiques.mjs
//
// `data/cards.json` est le fichier que le moteur natif charge ; `web/webapp/assets/cards.json`
// est celui que le shim WASI sert au pont. Ce sont DEUX fichiers sur le disque, et
// rien dans le dépôt ne les tenait ensemble : une carte corrigée d'un côté et pas
// de l'autre, et les deux moitiés du projet ne jouent plus au même jeu — sans que
// rien ne le dise. Tous les autres bancs deviennent alors des mensonges : ils
// comparent Rust et JavaScript sur des règles différentes.
//
// On ne compare ni la taille, ni la date, ni le nombre de cartes : le CONTENU,
// octet par octet, et l'on nomme le premier octet qui diffère.

import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "..", "..");
const MOTEUR = resolve(DEPOT, "data/cards.json");
const NAVIGATEUR = resolve(DEPOT, "web/webapp/assets/cards.json");

const empreinte = (o) => createHash("md5").update(o).digest("hex");

let compares = 0;
const fautes = [];

function lire(chemin) {
  try {
    return readFileSync(chemin);
  } catch (e) {
    fautes.push(`illisible : ${chemin} — ${e.message}`);
    return null;
  }
}

const a = lire(MOTEUR);
const b = lire(NAVIGATEUR);

if (a && b) {
  compares = Math.max(a.length, b.length);
  console.log(`moteur      data/cards.json              ${a.length} octets, md5 ${empreinte(a)}`);
  console.log(`navigateur  web/webapp/assets/cards.json ${b.length} octets, md5 ${empreinte(b)}`);
  if (a.length !== b.length) {
    fautes.push(`tailles différentes : ${a.length} contre ${b.length} octets`);
  }
  const n = Math.min(a.length, b.length);
  let premier = -1;
  let differents = 0;
  for (let i = 0; i < n; i++) {
    if (a[i] !== b[i]) {
      differents++;
      if (premier < 0) premier = i;
    }
  }
  differents += Math.abs(a.length - b.length);
  if (differents > 0) {
    fautes.push(
      `${differents} octet(s) différent(s), le premier à l'octet ${premier} ` +
        `(moteur 0x${(a[premier] ?? 0).toString(16)}, navigateur 0x${(b[premier] ?? 0).toString(16)})`,
    );
  }
  // Et l'empreinte, qui est ce que l'œil humain compare dans un journal.
  if (empreinte(a) !== empreinte(b)) fautes.push("empreintes md5 différentes");
}

for (const f of fautes) console.log(`  ✗ ${f}`);
if (fautes.length > 0) {
  console.log(`ROUGE ${fautes.length} écart(s) sur ${compares} octets comparés entre les deux fichiers de cartes`);
  process.exit(1);
}
console.log(`VERT ${compares} octets comparés, les deux fichiers de cartes sont identiques`);

#!/usr/bin/env node
// Preuve scriptée : une partie COMPLÈTE jouée par le pont pas-à-pas, du choix
// de corporation au score final.
//
// Elle emprunte exactement le chemin de la page : le même `creerPartie` et le
// même `jouerJusquAuBout` que `index.html`, avec un fournisseur de décisions
// branché à la place de l'humain. Ce n'est PAS le mode simulation du moteur —
// chaque décision est demandée par le moteur, puis répondue par l'hôte.
//
// Dernière ligne : {"terminee": true, "scores": [a, b], "manches": n}

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

let graine = 1;
let boites = "base,decouverte";
let bavard = false;
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--seed") graine = Number(args[++i]);
  else if (args[i] === "--boites") boites = args[++i];
  else if (args[i] === "--bavard") bavard = true;
  else {
    process.stderr.write(`partie-pas-a-pas: argument inconnu: ${args[i]}\n`);
    process.exit(2);
  }
}

const pont = await ouvrirPontDepuis(RACINE);
const partie = creerPartie(pont, { graine, boites });

// Les DEUX joueurs sont tenus par un fournisseur : le bac à sable ne fait
// aucune différence entre eux, c'est tout l'intérêt du point d'entrée unique.
const fournisseurs = [
  fournisseurAleatoire(graine * 2 + 1, "aléatoire J1"),
  fournisseurAleatoire(graine * 2 + 2, "aléatoire J2"),
];

let n = 0;
await jouerJusquAuBout(partie, fournisseurs, (p) => {
  n++;
  if (bavard) {
    const d = p.decision;
    process.stderr.write(
      `${String(n).padStart(5)} manche ${p.etat.generation} · J${d.joueur} · ${d.type} · ${d.question}\n`
    );
  }
});

if (!partie.termine) {
  process.stderr.write("partie-pas-a-pas: la partie ne s'est pas terminée\n");
  process.exit(1);
}
process.stderr.write(`decisions: ${n}\n`);
console.log(
  JSON.stringify({
    terminee: true,
    scores: partie.scores,
    manches: partie.manches,
  })
);

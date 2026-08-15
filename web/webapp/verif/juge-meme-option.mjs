#!/usr/bin/env node
// **(le-juge-apprend) LE JOUEUR RUST ET LE JOUEUR JAVASCRIPT CHOISISSENT LA MÊME
// OPTION DANS LA MÊME SITUATION.**
//
//   node web/webapp/verif/juge-meme-option.mjs [graines] [poids] [boites]
//
// C'est le vrai critère du §4. Les poids sont appris en Rust et relus en
// JavaScript ; que les deux descriptions concordent ne suffit pas, il faut que
// les deux JOUEURS en tirent la même conclusion — même réseau, même évaluation,
// même arbitrage entre options.
//
// Le banc ne fabrique aucune situation : il fait jouer **la partie entière** aux
// deux côtés, avec les mêmes poids, et compare les deux listes de réponses,
// décision par décision. Deux listes égales veulent dire que les deux joueurs ont
// choisi la même option à chacune des décisions — et comme chaque réponse change
// la partie, un seul désaccord ferait diverger tout ce qui suit : le test est
// bien plus sévère qu'une comparaison situation par situation.
//
// Ni exploration ni apprentissage des deux côtés : on compare deux jugements.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurApprenti } from "../joueurs/apprenti.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "../..");
const BIN = resolve(DEPOT, "engine/target/release/jouer");

const graines = Number(process.argv[2] ?? 3);
const poids = process.argv[3] ?? resolve(DEPOT, "data/poids/apprenti.txt");
const boites = process.argv[4] ?? "base,decouverte";

const pont = await ouvrirPontDepuis(RACINE);
let decisions = 0;
let accords = 0;
let parties = 0;
const desaccords = [];

for (let g = 1; g <= graines; g++) {
  // ---- le joueur Rust joue la partie entière
  let rust;
  try {
    rust = JSON.parse(
      execFileSync(BIN, ["--graine", String(g), "--poids", poids, "--boites", boites], {
        cwd: DEPOT,
        maxBuffer: 1e9,
      })
        .toString()
        .trim()
        .split("\n")
        .pop(),
    );
  } catch (e) {
    console.log(`✗ graine ${g} : le joueur Rust a échoué — ${String(e.message).split("\n")[0]}`);
    process.exit(1);
  }

  // ---- le joueur JavaScript joue la même, aux deux sièges
  const partie = creerPartie(pont, { graine: g, boites });
  const a = fournisseurApprenti(1, "apprenti", poids, pont, boites);
  const b = fournisseurApprenti(2, "apprenti", poids, pont, boites);
  await jouerJusquAuBout(partie, [a, b]);
  const js = partie.decisions;
  parties++;

  const n = Math.max(js.length, rust.decisions.length);
  for (let i = 0; i < n; i++) {
    decisions++;
    const x = JSON.stringify(js[i] ?? null);
    const y = JSON.stringify(rust.decisions[i] ?? null);
    if (x === y) accords++;
    else if (desaccords.length < 5) desaccords.push(`graine ${g}, décision ${i} : Rust ${y}, JavaScript ${x}`);
  }
  const memeScore =
    JSON.stringify(partie.scores) === JSON.stringify(rust.scores) ? "mêmes scores" : "SCORES DIFFÉRENTS";
  console.log(
    `graine ${g} : ${js.length} décisions côté JavaScript, ${rust.decisions.length} côté Rust — ${memeScore} (${JSON.stringify(rust.scores)})`,
  );
}

console.log(`${parties} partie(s), ${decisions} décision(s) comparées, ${accords} accord(s)`);
for (const d of desaccords) console.log(`  ✗ ${d}`);
if (decisions < 200) {
  console.log(`KO seulement ${decisions} décisions comparées : le §4 en demande au moins 200`);
  process.exit(1);
}
if (accords !== decisions) {
  console.log(`KO ${decisions - accords} désaccord(s) sur ${decisions} décisions`);
  process.exit(1);
}
console.log(`OK les deux joueurs choisissent la même option sur ${decisions} décisions de vraies parties`);

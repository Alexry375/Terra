#!/usr/bin/env node
// **(il-devine) Point d'accroche n°5 — LE JOUEUR RUST ET LE JOUEUR JAVASCRIPT
// CHOISISSENT LA MÊME OPTION, DEVINETTE ALLUMÉE.**
//
//   node web/webapp/verif/juge-meme-option-devinette.mjs \
//        [graines] [poids] [poids-adversaire] [boites]
//
// `juge-meme-option.mjs` couvre déjà le cas éteint et doit rester vert. Celui-ci
// couvre le cas allumé, et il est plus exigeant qu'il n'y paraît.
//
// ─────────────────────────────────────────────────────────────────────────────
// **POURQUOI CE BANC NE POUVAIT PAS ÊTRE UNE COPIE DE L'AUTRE.**
//
// Le premier réseau se départage à la MARGE : deux options dont les notes ne
// diffèrent qu'au dernier bit sont déclarées à égalité, et la première l'emporte
// des deux côtés (`MARGE = 1e-12`). C'est ce qui absorbe l'écart entre le Rust,
// qui met ses sommes cachées à jour par différences, et le JavaScript, qui refait
// chaque évaluation en entier.
//
// **La devinette, elle, prend un maximum sur cinq probabilités — et un maximum
// n'a pas de marge.** Un dernier bit de différence entre les deux côtés suffirait
// à faire deviner deux phases différentes, donc à faire diverger toute la partie
// qui suit. C'est pourquoi les deux côtés forcent le calcul complet du second
// réseau (`Reseau::oublier` avant chaque évaluation, côté Rust ; le JavaScript
// n'a jamais fait autrement) et départagent les égalités STRICTES par la plus
// petite phase. Ce banc est la vérification de cette chaîne-là.
//
// ─────────────────────────────────────────────────────────────────────────────
// **ET IL SE MÉFIE DE SON PROPRE VERT.** Un banc « devinette allumée » qui
// comparerait deux joueurs dont la devinette ne change rien serait vert sans rien
// prouver — exactement le piège que le prompt annonce (« le banc meme-option qui
// passe vert parce que la devinette est éteinte des deux côtés »). Il rejoue donc
// AUSSI la partie devinette éteinte côté Rust et compte les décisions qui
// changent. Si la devinette ne déplaçait aucune décision, il le dirait et
// refuserait de se déclarer concluant.

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
const adversaire = process.argv[4] ?? resolve(DEPOT, "data/poids/apprenti-adversaire.txt");
const boites = process.argv[5] ?? "base,decouverte";

/** Le joueur Rust joue la partie entière et rend sa liste de réponses. */
function cotéRust(g, avecDevinette) {
  const args = ["--graine", String(g), "--poids", poids, "--boites", boites];
  if (avecDevinette) args.push("--poids-adversaire", adversaire, "--devinette", "on");
  try {
    return JSON.parse(
      execFileSync(BIN, args, { cwd: DEPOT, maxBuffer: 1e9 }).toString().trim().split("\n").pop(),
    );
  } catch (e) {
    console.log(
      `✗ graine ${g} : le joueur Rust a échoué (devinette ${avecDevinette ? "allumée" : "éteinte"}) — ` +
        String(e.message).split("\n")[0],
    );
    process.exit(1);
  }
}

const pont = await ouvrirPontDepuis(RACINE);
let decisions = 0;
let accords = 0;
let deplacees = 0;
let parties = 0;
const desaccords = [];

for (let g = 1; g <= graines; g++) {
  const rust = cotéRust(g, true);
  const rustEteint = cotéRust(g, false);

  // Les fournisseurs AVANT la partie : c'est à leur fabrication que l'espion du
  // pont s'installe, et `creerPartie` appelle `pont.pas` tout de suite. Le
  // sixième argument est le chemin du second réseau : la devinette est donc
  // allumée des deux côtés, sans passer par l'environnement.
  const a = fournisseurApprenti(1, "apprenti", poids, pont, boites, adversaire);
  const b = fournisseurApprenti(2, "apprenti", poids, pont, boites, adversaire);
  const partie = creerPartie(pont, { graine: g, boites });
  await jouerJusquAuBout(partie, [a, b]);
  const js = partie.decisions;
  parties++;

  const n = Math.max(js.length, rust.decisions.length);
  for (let i = 0; i < n; i++) {
    decisions++;
    const x = JSON.stringify(js[i] ?? null);
    const y = JSON.stringify(rust.decisions[i] ?? null);
    if (x === y) accords++;
    else if (desaccords.length < 5) {
      desaccords.push(`graine ${g}, décision ${i} : Rust ${y}, JavaScript ${x}`);
    }
  }

  // Ce que la devinette a déplacé, côté Rust, à poids identiques.
  const m = Math.max(rust.decisions.length, rustEteint.decisions.length);
  for (let i = 0; i < m; i++) {
    if (
      JSON.stringify(rust.decisions[i] ?? null) !== JSON.stringify(rustEteint.decisions[i] ?? null)
    ) {
      deplacees++;
    }
  }

  const memeScore =
    JSON.stringify(partie.scores) === JSON.stringify(rust.scores) ? "mêmes scores" : "SCORES DIFFÉRENTS";
  console.log(
    `graine ${g} : ${js.length} décisions côté JavaScript, ${rust.decisions.length} côté Rust — ` +
      `${memeScore} (${JSON.stringify(rust.scores)}) ; devinette éteinte : ` +
      `${rustEteint.decisions.length} décisions`,
  );
}

console.log(
  `${parties} partie(s), ${decisions} décision(s) comparées devinette ALLUMÉE, ${accords} accord(s)`,
);
console.log(
  `décisions que la devinette déplace (Rust allumée contre Rust éteinte) : ${deplacees}`,
);
for (const d of desaccords) console.log(`  ✗ ${d}`);

if (decisions < 200) {
  console.log(`KO seulement ${decisions} décisions comparées : il en faut au moins 200`);
  process.exit(1);
}
if (accords !== decisions) {
  console.log(
    `KO ${decisions - accords} désaccord(s) sur ${decisions} décisions, devinette allumée`,
  );
  process.exit(1);
}
if (deplacees === 0) {
  // Vert sans rien prouver : on refuse.
  console.log(
    "KO la devinette ne déplace AUCUNE décision : ce banc serait vert sans rien vérifier " +
      "(second réseau introuvable, interrupteur inopérant, ou poids sans contenu)",
  );
  process.exit(1);
}
console.log(
  `OK les deux joueurs choisissent la même option sur ${decisions} décisions, devinette allumée — ` +
    `et la devinette déplace bien ${deplacees} décision(s)`,
);

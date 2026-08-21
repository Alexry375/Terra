#!/usr/bin/env node
// **(le-pont-ne-triche-plus, critère C / défaut V1) LE NAVIGATEUR NE VOIT PLUS
// L'AVENIR QUAND IL ESSAIE UN COUP.**
//
//   node web/webapp/verif/juge-l-avenir-cache.mjs [graines] [décisions] [poids] [boites]
//
// Essayer un coup, dans ce moteur, c'est rejouer la partie depuis sa graine avec
// une décision de plus. Tel quel, le joueur qui essaie LIT L'AVENIR : les cartes
// qu'il piochera, le bonus des tuiles Océan face cachée, l'ordre du paquet des
// corporations sont exactement ceux de la vraie partie. Il ne choisit plus le
// meilleur coup, il choisit le coup qui gagne DANS CETTE DONNE-LÀ.
//
// Le natif a été guéri au lot précédent : avant de rejouer un essai, il rebat
// tout ce que le joueur n'a pas encore vu, à partir d'une graine d'essais.
// Le pont, lui, ne l'était pas. Ce banc mesure les deux moitiés de la guérison :
//
//   1. **Le rebattage a lieu et n'est pas décoratif.** À graine de partie égale,
//      deux graines d'essais différentes doivent donner deux parties différentes
//      — côté natif comme côté pont. Un rebattage branché mais ignoré rendrait
//      partout le même résultat, et ce banc le verrait.
//   2. **Le navigateur rebat COMME le natif.** À graine de partie égale ET
//      graine d'essais égale, le joueur JavaScript doit choisir, décision après
//      décision, exactement ce que le joueur Rust choisit. C'est la preuve que
//      les deux côtés essaient sur le MÊME paquet rebattu : il suffirait qu'un
//      seul essai voie un paquet différent pour que les deux parties divergent.
//   3. **Un essai n'est pas la vraie partie.** L'essai d'une option doit rendre
//      autre chose que le rejeu nu de la même liste de décisions — sinon la
//      voyance est intacte, quel que soit le code écrit autour.
//
// On compare les PREMIÈRES décisions de chaque partie et non la partie entière :
// une divergence se propage (chaque réponse change la suite), donc un préfixe
// commun de plusieurs dizaines de décisions par graine, sur vingt graines, est
// déjà un accord sur des milliers d'essais — la mise en place seule en compte
// 512, un par sous-ensemble de main essayé aux deux sièges.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, offrirLesOccasions } from "../partie.js";
import { fournisseurApprenti } from "../joueurs/apprenti.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "../..");
const BIN = resolve(DEPOT, "engine/target/release/jouer");

const graines = Number(process.argv[2] ?? 24);
const PROFONDEUR = Number(process.argv[3] ?? 200);
const poids = process.argv[4] ?? resolve(DEPOT, "data/poids/apprenti.txt");
const boites = process.argv[5] ?? "base,decouverte";

// Deux graines d'essais quelconques, mais FIXES : le banc doit être reproductible.
const ESSAIS_A = 101;
const ESSAIS_B = 202;

const pont = await ouvrirPontDepuis(RACINE);

/** Le joueur Rust joue la partie entière avec une graine d'essais donnée. */
function natif(graine, graineEssais) {
  const brut = execFileSync(
    BIN,
    ["--graine", String(graine), "--poids", poids, "--boites", boites, "--graine-essais", String(graineEssais)],
    { cwd: DEPOT, maxBuffer: 1e9, stdio: ["ignore", "pipe", "ignore"] },
  )
    .toString()
    .trim()
    .split("\n")
    .pop();
  return JSON.parse(brut);
}

/** Le joueur JavaScript joue les `profondeur` premières décisions de la même. */
async function navigateur(graine, graineEssais, profondeur) {
  const a = fournisseurApprenti(1, "apprenti", poids, pont, boites, undefined, graineEssais);
  const b = fournisseurApprenti(2, "apprenti", poids, pont, boites, undefined, graineEssais);
  const partie = creerPartie(pont, { graine, boites });
  let garde = 0;
  while (!partie.termine && partie.decisions.length < profondeur) {
    if (++garde > 100000) throw new Error("boucle de décisions anormalement longue");
    await offrirLesOccasions(partie, [a, b]);
    if (partie.termine || partie.decisions.length >= profondeur) break;
    const d = partie.decision;
    if (!d) throw new Error("le moteur n'a rendu ni décision ni fin de partie");
    partie.repondre(await [a, b][d.joueur].decider(d, partie.etat));
  }
  return partie;
}

let cas = 0;
let comparees = 0;
let accords = 0;
const fautes = [];

function verifier(nom, f) {
  cas++;
  try {
    const detail = f();
    if (detail) console.log(`    ${nom} — ${detail}`);
  } catch (e) {
    fautes.push(`${nom} : ${e.message}`);
    console.log(`  ✗ ${nom} : ${e.message}`);
  }
}

const debut = Date.now();
for (let g = 1; g <= graines; g++) {
  const A = natif(g, ESSAIS_A);
  const B = natif(g, ESSAIS_B);

  // ── 1. le rebattage n'est pas décoratif, côté natif
  verifier(`graine ${g} : deux graines d'essais donnent deux parties différentes (natif)`, () => {
    if (JSON.stringify(A.decisions) === JSON.stringify(B.decisions)) {
      throw new Error(
        `les graines d'essais ${ESSAIS_A} et ${ESSAIS_B} rendent le MÊME journal de ` +
          `${A.decisions.length} décisions : le rebattage natif ne sert à rien`,
      );
    }
    return null;
  });

  // ── 2. le navigateur rebat comme le natif
  const partie = await navigateur(g, ESSAIS_A, PROFONDEUR);
  const js = partie.decisions;
  const n = Math.min(PROFONDEUR, Math.max(js.length, A.decisions.length));
  let premierDesaccord = -1;
  let accordsIci = 0;
  for (let i = 0; i < n; i++) {
    comparees++;
    const x = JSON.stringify(js[i] ?? null);
    const y = JSON.stringify(A.decisions[i] ?? null);
    if (x === y) {
      accords++;
      accordsIci++;
    } else if (premierDesaccord < 0) premierDesaccord = i;
  }
  verifier(`graine ${g} : le navigateur choisit comme le natif sur ${n} décisions`, () => {
    if (premierDesaccord >= 0) {
      const i = premierDesaccord;
      throw new Error(
        `décision ${i} — natif ${JSON.stringify(A.decisions[i] ?? null)}, ` +
          `navigateur ${JSON.stringify(js[i] ?? null)}`,
      );
    }
    return null;
  });

  // ── 3. un essai n'est pas le rejeu de la vraie partie
  verifier(`graine ${g} : un essai rebat, et deux graines d'essais divergent (pont)`, () => {
    // On cherche un rang où l'essai se distingue. Il en existe forcément un dès
    // que le joueur consomme quelque chose qu'il n'avait pas encore vu ; on
    // regarde les premiers rangs jouables de la partie.
    let vuDifferentDuReel = false;
    let vuDeuxGrainesDifferentes = false;
    const bornes = Math.min(js.length, 10);
    for (let r = 1; r < bornes; r++) {
      const D = js.slice(0, r + 1);
      // Une entrée « vendre » n'est pas une décision : elle se consomme à une
      // occasion. Un essai calé sur son rang ne s'atteint jamais, et le moteur
      // le refuse maintenant explicitement. On saute ces rangs-là.
      if (js[r] && js[r].vendre !== undefined) continue;
      const reel = JSON.stringify(pont.pas(g, boites, D));
      const ea = JSON.stringify(pont.pas(g, boites, D, { graine: ESSAIS_A, rang: r }));
      const eb = JSON.stringify(pont.pas(g, boites, D, { graine: ESSAIS_B, rang: r }));
      const bis = JSON.stringify(pont.pas(g, boites, D, { graine: ESSAIS_A, rang: r }));
      if (bis !== ea) throw new Error(`rang ${r} : la même graine d'essais rend deux essais différents`);
      if (ea !== reel) vuDifferentDuReel = true;
      if (ea !== eb) vuDeuxGrainesDifferentes = true;
      if (vuDifferentDuReel && vuDeuxGrainesDifferentes) break;
    }
    if (!vuDifferentDuReel) {
      throw new Error(
        `sur les ${bornes} premiers rangs, l'essai rend exactement le rejeu de la vraie ` +
          `partie : le pont laisse encore lire l'avenir`,
      );
    }
    if (!vuDeuxGrainesDifferentes) {
      throw new Error(`sur les ${bornes} premiers rangs, les graines d'essais ${ESSAIS_A} et ${ESSAIS_B} ne changent rien`);
    }
    return null;
  });

  console.log(
    `  graine ${g} : natif ${A.decisions.length} décisions (essais ${ESSAIS_A}) / ` +
      `${B.decisions.length} (essais ${ESSAIS_B}), navigateur d'accord sur ${accordsIci} décisions sur ${n} comparées`,
  );
}

const secondes = ((Date.now() - debut) / 1000).toFixed(1);
console.log(`graines : ${graines} — ${cas} cas, ${comparees} décisions comparées, ${accords} accord(s), ${secondes} s`);
for (const f of fautes.slice(0, 8)) console.log(`  ✗ ${f}`);
if (fautes.length > 0) {
  console.log(`ROUGE ${fautes.length} faute(s) sur ${cas} cas et ${comparees} décisions comparées`);
  process.exit(1);
}
console.log(
  `VERT ${cas} cas et ${comparees} décisions comparées sur ${graines} graines : ` +
    `le navigateur essaie sur un paquet rebattu, exactement comme le natif`,
);

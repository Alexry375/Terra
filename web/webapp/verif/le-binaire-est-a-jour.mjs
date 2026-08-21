#!/usr/bin/env node
// **(le-pont-ne-triche-plus, critère H / défaut D25) LE BINAIRE LIVRÉ EST BIEN
// CELUI QUE LA SOURCE PRODUIT AUJOURD'HUI.**
//
//   node web/webapp/verif/le-binaire-est-a-jour.mjs
//
// `web/webapp/terra.wasm` est un fichier compilé, versionné à côté de sa source.
// Rien n'oblige les deux à rester d'accord : on corrige `wasm/src/lib.rs`, on
// oublie `web/construire.sh`, et le dépôt continue de livrer l'ANCIEN moteur.
// Tous les bancs du navigateur mesurent alors un binaire que plus personne n'a
// écrit — et ils sont verts, parce qu'ils mesurent bien quelque chose.
//
// On ne regarde ni la date du fichier, ni sa taille : on RECOMPILE la source
// dans le dossier de construction habituel (`web/work/target`, celui de
// `web/construire.sh`) et on compare le CONTENU du binaire fraîchement produit
// avec celui que le dépôt livre. Ce banc ne réécrit jamais
// `web/webapp/terra.wasm` : il le juge, il ne le répare pas.
//
// Trois cas, et il faut les trois :
//   1. la recompilation rend octet pour octet le binaire livré ;
//   2. le binaire livré se charge et répond à un pas de partie ;
//   3. le binaire livré CONNAÎT la graine d'essais — deux graines d'essais
//      différentes doivent rendre deux essais différents. Un binaire d'avant ce
//      lot passerait le cas 2 sans broncher et échouerait ici.

import { execFileSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { ouvrirPontDepuis } from "../pont.js";

const VERIF = dirname(fileURLToPath(import.meta.url));
const LIVRAISON = resolve(VERIF, "..");
const DEPOT = resolve(LIVRAISON, "..", "..");
const LIVRE = resolve(LIVRAISON, "terra.wasm");
const CONSTRUCTION = resolve(DEPOT, "web/work/target");
const SOURCE = resolve(DEPOT, "web/webapp/wasm");
const FRAIS = resolve(CONSTRUCTION, "wasm32-wasip1/release/terra_pont.wasm");

const empreinte = (o) => createHash("md5").update(o).digest("hex");
const fautes = [];
let cas = 0;

function verifier(nom, f) {
  cas++;
  try {
    const detail = f();
    console.log(`  ✓ ${nom}${detail ? " — " + detail : ""}`);
  } catch (e) {
    fautes.push(`${nom} : ${e.message}`);
    console.log(`  ✗ ${nom} : ${e.message}`);
  }
}

// ── cas 1 : le contenu, pas la date ─────────────────────────────────────────
let octetsLivres = null;
verifier("le binaire livré existe", () => {
  if (!existsSync(LIVRE)) throw new Error(`absent : ${LIVRE}`);
  octetsLivres = readFileSync(LIVRE);
  return `${octetsLivres.length} octets, md5 ${empreinte(octetsLivres)}`;
});

verifier("la source recompilée rend le binaire livré, octet pour octet", () => {
  // Le MÊME dossier de construction que `web/construire.sh`, pour que la
  // recompilation soit un non-événement quand rien n'a bougé.
  execFileSync("cargo", ["build", "--release", "--target", "wasm32-wasip1"], {
    cwd: SOURCE,
    env: { ...process.env, CARGO_TARGET_DIR: CONSTRUCTION },
    stdio: ["ignore", "ignore", "pipe"],
    timeout: 600_000,
  });
  if (!existsSync(FRAIS)) throw new Error(`la compilation n'a rien produit : ${FRAIS}`);
  const frais = readFileSync(FRAIS);
  if (!octetsLivres) throw new Error("binaire livré illisible");
  if (frais.length !== octetsLivres.length) {
    throw new Error(`tailles différentes : livré ${octetsLivres.length}, recompilé ${frais.length} octets`);
  }
  let premier = -1;
  let differents = 0;
  for (let i = 0; i < frais.length; i++) {
    if (frais[i] !== octetsLivres[i]) {
      differents++;
      if (premier < 0) premier = i;
    }
  }
  if (differents > 0) {
    throw new Error(
      `le binaire livré est périmé (ou abîmé) : ${differents} octet(s) diffèrent, ` +
        `le premier à l'octet ${premier} — livré 0x${octetsLivres[premier].toString(16)}, ` +
        `recompilé 0x${frais[premier].toString(16)}. Relance web/construire.sh.`,
    );
  }
  return `md5 ${empreinte(frais)} des deux côtés`;
});

// ── cas 2 et 3 : le binaire répond, et il connaît la graine d'essais ─────────
let pont = null;
try {
  pont = await ouvrirPontDepuis(LIVRAISON);
} catch (e) {
  cas++;
  fautes.push(`le binaire livré ne se charge pas : ${e.message}`);
  console.log(`  ✗ le binaire livré ne se charge pas : ${e.message}`);
}

if (pont) {
  verifier("le binaire livré répond à un pas de partie", () => {
    const r = pont.pas(4242, "base", []);
    if (!r || !r.decision) throw new Error("réponse sans décision : " + JSON.stringify(r).slice(0, 160));
    return `décision « ${r.decision.type} » à la graine 4242`;
  });

  verifier("le binaire livré connaît la graine d'essais (rebattage vivant)", () => {
    // Rang 1 : les deux mulligans de corporation sont répondus, et la question
    // suivante dépend de ce que le joueur n'a pas encore vu — donc du rebattage.
    const a = pont.pas(4242, "base", [0, 0], { graine: 1, rang: 1 });
    const b = pont.pas(4242, "base", [0, 0], { graine: 2, rang: 1 });
    const ja = JSON.stringify(a);
    const jb = JSON.stringify(b);
    if (ja === jb) {
      throw new Error(
        "deux graines d'essais différentes rendent le MÊME essai : ce binaire ignore " +
          "`graine_essais` — il date d'avant le lot « le pont ne triche plus »",
      );
    }
    // Et le rebattage doit rester reproductible : même graine, même essai.
    const bis = pont.pas(4242, "base", [0, 0], { graine: 1, rang: 1 });
    if (JSON.stringify(bis) !== ja) {
      throw new Error("la même graine d'essais rend deux essais différents : le rebattage n'est pas reproductible");
    }
    return "graines 1 et 2 divergent, la graine 1 se répète à l'identique";
  });
}

if (fautes.length > 0) {
  console.log(`ROUGE ${fautes.length} faute(s) sur ${cas} cas vérifiés — le binaire livré n'est pas celui de la source`);
  process.exit(1);
}
console.log(`VERT ${cas} cas vérifiés, le binaire livré est exactement celui que la source produit`);

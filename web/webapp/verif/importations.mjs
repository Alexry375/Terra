// LE BANC DE LA COUTURE — aucune importation ne pointe dans le vide.
//
// Écrit pour le chantier de fusion, et pour lui seul. Les trois livraisons ont
// été faites en parallèle sans se voir : chacune a pu renommer une fonction que
// les deux autres appellent encore. Deux cas réels, dans cette fusion même :
//
//   `dosDeCarte()`   -> `dosProjet()` / `dosCorporation()`   (table-vivante)
//   `imageOcean(i)`  -> `imageOcean()`, `TUILES_OCEAN` retiré (bandeau-et-monde)
//
// Reprendre `vue/plateau.js` chez l'un et `vue/scene.js` chez l'autre, c'est
// donc risquer un module qui appelle un nom que plus personne n'exporte. Le
// navigateur ne le dirait qu'à l'exécution, et seulement si le chemin est pris.
//
// Ce banc le dit tout de suite, sans navigateur : il relève les `export` de
// chaque module de la livraison, puis vérifie que chaque
// `import { … } from "./…"` désigne un fichier qui existe ET des noms
// réellement exportés. Il ne juge rien d'autre — ni le style, ni les règles.
//
// Depuis la racine du workspace :  node outputs/web/webapp/verif/importations.mjs
// Depuis ce dossier :              node importations.mjs

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

// La livraison, c'est le dossier qui contient ce banc-ci, moins ce qui n'est pas
// du code de la page.
const RACINE = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const IGNORES = new Set(["assets", "polices", "wasm", "verif"]);

/** Tous les modules de la page, en chemins absolus. */
function modules(dossier) {
  const trouves = [];
  for (const e of fs.readdirSync(dossier, { withFileTypes: true })) {
    const chemin = path.join(dossier, e.name);
    if (e.isDirectory()) {
      if (IGNORES.has(e.name)) continue;
      trouves.push(...modules(chemin));
    } else if (e.name.endsWith(".js") || e.name.endsWith(".mjs")) {
      trouves.push(chemin);
    }
  }
  return trouves;
}

/**
 * Les noms qu'un module exporte. On lit le texte plutôt que d'importer : un
 * module de la page touche au document dès qu'on l'évalue, et ce banc doit
 * pouvoir tourner sans navigateur.
 */
function exportes(source) {
  const noms = new Set();
  for (const m of source.matchAll(
    /^export\s+(?:async\s+)?(?:function|class|const|let|var)\s+([A-Za-z0-9_$]+)/gm)) {
    noms.add(m[1]);
  }
  for (const m of source.matchAll(/^export\s*\{([^}]*)\}/gm)) {
    for (const brut of m[1].split(",")) {
      const nom = brut.trim().split(/\s+as\s+/).pop();
      if (nom) noms.add(nom);
    }
  }
  if (/^export\s+default/m.test(source)) noms.add("default");
  return noms;
}

const fichiers = modules(RACINE);
const table = new Map(fichiers.map((f) => [f, exportes(fs.readFileSync(f, "utf8"))]));

const fautes = [];
let liens = 0;

for (const fichier of fichiers) {
  const source = fs.readFileSync(fichier, "utf8");
  // `import * as x from`, `import { a, b } from`, `import x from` — les trois
  // formes qu'emploie ce dépôt, et uniquement les chemins relatifs : rien n'est
  // jamais chargé de l'extérieur de la page.
  const motif =
    /import\s+(?:\*\s+as\s+[A-Za-z0-9_$]+|\{([^}]*)\}|([A-Za-z0-9_$]+))\s+from\s+["'](\.[^"']+)["']/g;
  for (const m of source.matchAll(motif)) {
    const cible = path.resolve(path.dirname(fichier), m[3]);
    const ici = path.relative(RACINE, fichier);
    liens += 1;
    if (!fs.existsSync(cible)) {
      fautes.push(`${ici} : le module « ${m[3]} » n'existe pas`);
      continue;
    }
    if (!m[1]) continue; // `import * as` ou défaut : rien de nommé à vérifier
    for (const brut of m[1].split(",")) {
      const nom = brut.trim().split(/\s+as\s+/)[0];
      if (!nom) continue;
      if (!table.get(cible)?.has(nom)) {
        fautes.push(
          `${ici} : importe « ${nom} » de « ${m[3]} », qui ne l'exporte pas`);
      }
    }
  }
}

console.log(`${fichiers.length} modules, ${liens} importations verifiees`);
if (fautes.length) {
  for (const f of fautes) console.log("  " + f);
  console.log(`KO ${fautes.length} importation(s) pointent dans le vide`);
  process.exit(1);
}
console.log("OK toutes les importations resolvent");

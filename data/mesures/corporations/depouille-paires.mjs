#!/usr/bin/env node
// COMBIEN COUTE UN MAUVAIS CHOIX DE CORPORATION ?
//
//   node data/mesures/corporations/depouille-paires.mjs paires.jsonl [autre.jsonl ...]
//
// La force d'une corporation est son ECART DE SCORE moyen, mesure sur 799
// parties ou elle etait TIREE AU SORT (voir depouille-corpos.mjs). Passer d'une
// corporation a +14 a une corporation a -7, c'est donc perdre 21 points d'ecart.
//
// La PERTE d'un choix = force de la meilleure des deux proposees - force de la
// prise. Elle vaut 0 quand le joueur prend la bonne, et la difference entiere
// quand il prend l'autre. Moyennee sur des centaines de choix, elle repond a la
// question en points de score, pas en pourcentages abstraits.
//
// DEUX PRECAUTIONS, et elles limitent vraiment la portee du resultat :
//
// 1. L'incertitude sur la force de chaque corporation est de +-5,5 points. Sur
//    une paire du milieu de tableau, on ne SAIT PAS laquelle est la meilleure :
//    le « bon choix » qu'on compte est parfois arbitraire. Le depouillement
//    separe donc les paires TRANCHEES (ecart > 11 points, soit deux fois
//    l'incertitude : le signe est sur) des paires AMBIGUES.
// 2. Les erreurs sur les forces sont correlees d'une paire a l'autre — la meme
//    erreur sur Tharsis Republic affecte toutes les paires qui la contiennent.
//    Moyenner ne les efface donc pas entierement.
//
// Deux points de comparaison sont imprimes : le joueur PARFAIT (perte nulle par
// construction) et le joueur au HASARD, calcule sur les memes paires.
import { readFileSync } from "node:fs";

const FORCE = {
  "Apollo Industries": 14.02, "Tharsis Republic": 13.71, "Exocorp": 11.88,
  "Teractor Corporation": 4.27, "Sultira": 1.78, "Helion Corporation": -0.49,
  "Thorgate Corporation": -1.74, "Phobolog": -2.18, "Ecoline": -4.28,
  "Unmi": -4.74, "Credicor": -4.77, "Hyperion Systems": -5.35,
  "Interplanetary Cinematics": -5.78, "Mining Guild": -6.45,
  "Inventrix": -6.76, "Saturn Systems": -6.94,
};
const TRANCHE = 11; // deux fois l'incertitude : en-deca, le classement ne tranche pas

for (const f of process.argv.slice(2)) {
  const lignes = readFileSync(f, "utf8").split("\n").filter((l) => l.trim()).map((l) => JSON.parse(l));
  const bloc = { tout: [], tranchees: [], ambigues: [] };
  let inconnues = 0, hasard = 0, n = 0;

  for (const c of lignes) {
    if (c.proposees.length !== 2 || !c.prise) { inconnues++; continue; }
    const [x, y] = c.proposees;
    if (FORCE[x] === undefined || FORCE[y] === undefined) { inconnues++; continue; }
    const meilleure = FORCE[x] >= FORCE[y] ? x : y;
    const perte = FORCE[meilleure] - FORCE[c.prise];
    const ecartPaire = Math.abs(FORCE[x] - FORCE[y]);
    const obs = { perte, bon: c.prise === meilleure };
    bloc.tout.push(obs);
    (ecartPaire > TRANCHE ? bloc.tranchees : bloc.ambigues).push(obs);
    hasard += ecartPaire / 2; // un joueur au hasard perd l'ecart une fois sur deux
    n++;
  }

  const resume = (nom, a) => {
    if (!a.length) return `  ${nom.padEnd(22)} (aucune)`;
    const bons = a.filter((o) => o.bon).length;
    const m = a.reduce((s, o) => s + o.perte, 0) / a.length;
    const varr = a.reduce((s, o) => s + (o.perte - m) ** 2, 0) / Math.max(1, a.length - 1);
    const err = 2 * Math.sqrt(varr / a.length);
    return `  ${nom.padEnd(22)} ${String(a.length).padStart(5)} choix   ` +
      `bons ${((bons / a.length) * 100).toFixed(1).padStart(5)} %   ` +
      `perte moyenne ${m.toFixed(2).padStart(5)} ±${err.toFixed(2)} point(s)`;
  };

  console.log(`\n===== ${f} =====`);
  console.log(resume("toutes les paires", bloc.tout));
  console.log(resume("paires tranchees", bloc.tranchees));
  console.log(resume("paires ambigues", bloc.ambigues));
  console.log(`  ${"joueur au hasard".padEnd(22)} ${String(n).padStart(5)} choix   bons  50.0 %   perte moyenne ${(hasard / n).toFixed(2).padStart(5)} point(s)`);
  console.log(`  ${"joueur parfait".padEnd(22)} ${String(n).padStart(5)} choix   bons 100.0 %   perte moyenne  0.00 point(s)`);
  if (inconnues) console.log(`  (${inconnues} choix ecartes : corporation hors classement ou paire incomplete)`);
}

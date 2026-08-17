#!/usr/bin/env node
// Trois questions : la force mesuree d'une corporation s'explique-t-elle par son
// argent de depart ? l'IA la reconnait-elle ? le temoin `reflechi` la reconnait-il ?
// On donne Pearson (lien lineaire) ET Spearman (lien de rang, insensible aux
// valeurs extremes) : si les deux divergent, c'est qu'un point tire tout.

// force = ecart de score moyen, mesure sur 799 parties (corporation tiree au sort)
const force = {
  "Apollo Industries": 14.02, "Tharsis Republic": 13.71, "Exocorp": 11.88,
  "Teractor Corporation": 4.27, "Sultira": 1.78, "Helion Corporation": -0.49,
  "Thorgate Corporation": -1.74, "Phobolog": -2.18, "Ecoline": -4.28,
  "Unmi": -4.74, "Credicor": -4.77, "Hyperion Systems": -5.35,
  "Interplanetary Cinematics": -5.78, "Mining Guild": -6.45,
  "Inventrix": -6.76, "Saturn Systems": -6.94,
};
// argent de depart, releve dans les donnees de cartes
const argent = {
  "Teractor Corporation": 51, "Credicor": 48, "Interplanetary Cinematics": 46,
  "Thorgate Corporation": 45, "Tharsis Republic": 40, "Sultira": 38, "Unmi": 35,
  "Inventrix": 33, "Apollo Industries": 33, "Hyperion Systems": 30,
  "Helion Corporation": 28, "Ecoline": 27, "Mining Guild": 27, "Exocorp": 26,
  "Saturn Systems": 24, "Phobolog": 20,
};
import { readFileSync } from "node:fs";
const lire = (f) => Object.fromEntries(
  JSON.parse(readFileSync(f, "utf8")).corporations.map((c) => [c.nom, c.taux]));

const noms = Object.keys(force);
const pearson = (a, b) => {
  const ma = a.reduce((x, y) => x + y) / a.length, mb = b.reduce((x, y) => x + y) / b.length;
  let n = 0, da = 0, db = 0;
  for (let i = 0; i < a.length; i++) { n += (a[i] - ma) * (b[i] - mb); da += (a[i] - ma) ** 2; db += (b[i] - mb) ** 2; }
  return n / Math.sqrt(da * db);
};
const rangs = (v) => {
  const tri = v.map((x, i) => [x, i]).sort((p, q) => p[0] - q[0]);
  const r = new Array(v.length);
  tri.forEach(([, i], k) => { r[i] = k + 1; });
  return r;
};
const spearman = (a, b) => pearson(rangs(a), rangs(b));

const F = noms.map((n) => force[n]);
const A = noms.map((n) => argent[n]);
const P = noms.map((n) => lire(process.argv[2])[n]);
const R = noms.map((n) => lire(process.argv[3])[n]);

const dire = (t, x, y) =>
  console.log(`${t.padEnd(42)} Pearson ${pearson(x, y).toFixed(2).padStart(6)}   Spearman ${spearman(x, y).toFixed(2).padStart(6)}`);

console.log("(1 = lien parfait, 0 = aucun lien, -1 = lien inverse)\n");
dire("argent de depart  vs  force reelle", A, F);
dire("preference de l'IA  vs  force reelle", P, F);
dire("preference de reflechi  vs  force reelle", R, F);
dire("preference de l'IA  vs  argent de depart", P, A);
dire("preference de reflechi  vs  argent de depart", R, A);

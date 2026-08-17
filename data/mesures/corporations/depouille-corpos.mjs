#!/usr/bin/env node
// DEPOUILLEMENT DU TOURNOI DES CORPORATIONS
//   node depouille-corpos.mjs fichier.jsonl [fichier2.jsonl ...]
//
// Chaque partie donne DEUX observations, une par siege. On mesure pour chaque
// corporation : son taux de victoire, et son ECART DE SCORE moyen (le second
// est bien plus sensible : le taux de victoire jette l'ampleur de l'ecart).
//
// L'incertitude donnee est l'ecart typique de la moyenne (68 % de confiance) ;
// je double pour l'intervalle usuel a 95 %.
import { readFileSync } from "node:fs";

const stat = new Map();
const touche = (nom) => {
  if (!stat.has(nom)) stat.set(nom, { n: 0, v: 0, nul: 0, ecarts: [] });
  return stat.get(nom);
};

let parties = 0, incompletes = 0;
for (const f of process.argv.slice(2)) {
  for (const l of readFileSync(f, "utf8").split("\n")) {
    if (!l.trim()) continue;
    const p = JSON.parse(l);
    if (!p.complete || p.score0 == null || p.score1 == null) { incompletes++; continue; }
    if (!p.corpo0 || !p.corpo1) { incompletes++; continue; }
    parties++;
    const paires = [[p.corpo0, p.score0 - p.score1], [p.corpo1, p.score1 - p.score0]];
    for (const [nom, ecart] of paires) {
      const s = touche(nom);
      s.n++; s.ecarts.push(ecart);
      if (ecart > 0) s.v++; else if (ecart === 0) s.nul++;
    }
  }
}

const moy = (a) => a.reduce((x, y) => x + y, 0) / a.length;
const lignes = [...stat.entries()].map(([nom, s]) => {
  const m = moy(s.ecarts);
  const varr = s.ecarts.length > 1
    ? s.ecarts.reduce((acc, x) => acc + (x - m) ** 2, 0) / (s.ecarts.length - 1) : 0;
  const errEcart = Math.sqrt(varr / s.ecarts.length);
  const taux = s.v / s.n;
  const errTaux = Math.sqrt(taux * (1 - taux) / s.n);
  return { nom, n: s.n, victoires: s.v, nuls: s.nul, taux, errTaux, ecart: m, errEcart };
});
lignes.sort((a, b) => b.ecart - a.ecart);

console.log(`parties depouillees : ${parties}  (ecartees : ${incompletes})`);
console.log(`observations : ${lignes.reduce((a, l) => a + l.n, 0)}\n`);
console.log("corporation                 jouee  victoires   taux ±2σ        ecart de score ±2σ");
for (const l of lignes) {
  console.log(
    l.nom.padEnd(26) +
    String(l.n).padStart(6) +
    String(l.victoires).padStart(10) +
    `   ${(l.taux * 100).toFixed(1).padStart(5)} % ±${(l.errTaux * 200).toFixed(1).padStart(4)}` +
    `      ${(l.ecart >= 0 ? "+" : "") + l.ecart.toFixed(2).padStart(6)} ±${(l.errEcart * 2).toFixed(2).padStart(5)}`
  );
}

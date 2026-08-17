const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurReflechi } = await import(`${RACINE}/joueurs/reflechi.js`);
const pont = await ouvrirPontDepuis(RACINE);
const vu = new Map();
for (let g = 1; g <= 150; g++) {
  const f = [fournisseurReflechi(g*7+1,"a"), fournisseurReflechi(g*13+3,"b")];
  const partie = creerPartie(pont, { graine: g, boites: "base,decouverte" });
  let n = 0;
  while (!partie.termine && n++ < 12) {
    const d = partie.decision;
    if (!d) break;
    if (/choisissez votre corporation/i.test(d.question || "")) {
      for (const o of d.options || []) if (!vu.has(o.libelle)) vu.set(o.libelle, o);
    }
    partie.repondre(await f[d.joueur].decider(d, partie.etat));
  }
}
const l = [...vu.values()].sort((a,b) => (b.mc_depart||0)-(a.mc_depart||0));
for (const o of l) console.log(`${(o.libelle||"").padEnd(26)} ${String(o.mc_depart||0).padStart(3)} MC   badges: ${(o.badges||[]).join(",") || "aucun"}`);
console.log("--- exemple brut ---"); console.log(JSON.stringify(l[0]));

#!/usr/bin/env node
// QUE FAIT VRAIMENT L'IA PENDANT LA MISE EN PLACE ?
//
//   node data/mesures/corporations/mise-en-place.mjs <donnes> [joueur]
//   (APPRENTI_POIDS designe le niveau)
//
// Le tournoi des corporations TIRAIT AU SORT le mulligan et le choix : c'etait
// necessaire pour mesurer la force d'une corporation sans biais de selection,
// mais cela ne dit rien du comportement reel. Ce banc-ci ne force rien : l'IA
// decide tout, comme en partie. On releve les TROIS decisions de mise en place
// (flow.rs:55-60) :
//   2. mulligan corporations — les DEUX ou aucune, sans voir les projets
//   4. mulligan projets       — carte par carte, de 0 a 8
//   5. choix final de corporation — cartes projets en main
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 200);
const QUI = process.argv[3] || "apprenti";
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);
const { fournisseurReflechi } = await import(`${RACINE}/joueurs/reflechi.js`);
const pont = await ouvrirPontDepuis(RACINE);
const faire = (g, n) => QUI === "reflechi"
  ? fournisseurReflechi(g, n)
  : fournisseurApprenti(g, n, undefined, pont, BOITES);

const EST_MULL_CORP = (q) => /remplacer vos .* corporations/i.test(q || "");
const EST_MULL_PROJ = (q) => /quelles cartes projets remplacez/i.test(q || "");
const EST_CHOIX     = (q) => /choisissez votre corporation/i.test(q || "");

let mcGarde = 0, mcRemplace = 0, nChoix = 0;
const projRendues = [];   // combien de cartes projets rendues, par joueur

for (let g = 1; g <= DONNES; g++) {
  const f = [faire(g * 7 + 1, "a"), faire(g * 13 + 3, "b")];
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  let faits = 0, garde = 0;
  while (!partie.termine && faits < 2 && ++garde < 80) {
    const d = partie.decision;
    if (!d) break;
    const r = await f[d.joueur].decider(d, partie.etat);
    if (EST_MULL_CORP(d.question)) {
      const lib = (d.options || [])[typeof r === "number" ? r : (r?.indice ?? -1)]?.libelle || "";
      if (/garde|conserv/i.test(lib)) mcGarde++; else mcRemplace++;
    } else if (EST_MULL_PROJ(d.question)) {
      projRendues.push(Array.isArray(r) ? r.length : (Array.isArray(r?.indices) ? r.indices.length : 0));
    } else if (EST_CHOIX(d.question)) { nChoix++; faits++; }
    partie.repondre(r);
  }
}

const somme = projRendues.reduce((a, b) => a + b, 0);
const distrib = {};
for (const k of projRendues) distrib[k] = (distrib[k] || 0) + 1;
console.log(JSON.stringify({
  joueur: QUI, poids: process.env.APPRENTI_POIDS || "(defaut)", donnes: DONNES,
  mulligan_corporations: { garde: mcGarde, remplace: mcRemplace },
  mulligan_projets: {
    occasions: projRendues.length,
    cartes_rendues_au_total: somme,
    moyenne_par_joueur: projRendues.length ? Number((somme / projRendues.length).toFixed(2)) : null,
    distribution: distrib,
  },
  choix_de_corporation: nChoix,
}, null, 1));

#!/usr/bin/env node
// QUELLES CORPORATIONS L'IA CHOISIT-ELLE ?
//
//   node corpos.mjs <joueur> <donnes>          (APPRENTI_POIDS designe le niveau)
//
// On ne joue PAS la partie jusqu'au bout : le choix de corporation tombe dans
// les six premieres decisions, et rien de ce qui suit ne changerait le releve.
// On s'arrete des que les deux sieges ont choisi.
//
// Ce qui est releve, et c'est le seul chiffre honnete : le taux de choix
// CONDITIONNEL — « quand cette corporation est proposee, combien de fois est-elle
// prise ». Compter les choix bruts dirait surtout lesquelles sortent souvent du
// paquet, pas lesquelles l'IA prefere.
//
// DEUX PIEGES, tous deux trouves a l'usage le 16-08 :
//  1. `d.question` porte le LIBELLE FRANCAIS (« Choisissez votre corporation »),
//     pas le nom interne `pick_corporation`. Chercher le nom interne ne trouve
//     jamais rien et fait jouer la partie entiere sans qu'on s'en apercoive.
//  2. Les fournisseurs doivent etre construits AVANT `creerPartie`. L'apprenti
//     s'installe sur le pont pour pouvoir essayer les options ; cree apres, il
//     rate la premiere decision et repond la premiere option en silence.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const QUI = process.argv[2] || "apprenti";
const DONNES = Number(process.argv[3] || 60);
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurReflechi } = await import(`${RACINE}/joueurs/reflechi.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);

const pont = await ouvrirPontDepuis(RACINE);

function faire(graine, nom) {
  if (QUI === "reflechi") return fournisseurReflechi(graine, nom);
  return fournisseurApprenti(graine, nom, undefined, pont, BOITES);
}

const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");
const EST_MULLIGAN = (q) => /remplacer vos .* corporations/i.test(q || "");

const propose = new Map();
const choisie = new Map();
const mulligan = { garde: 0, remplace: 0 };
let choix = 0;

for (let g = 1; g <= DONNES; g++) {
  const a = faire(g * 7 + 1, "a");
  const b = faire(g * 13 + 3, "b");
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  const f = [a, b];
  let faits = 0;
  let garde = 0;
  while (!partie.termine && faits < 2 && ++garde < 60) {
    const d = partie.decision;
    if (!d) break;
    const r = await f[d.joueur].decider(d, partie.etat);
    const i = typeof r === "number" ? r : (r?.indice ?? -1);
    if (EST_CHOIX(d.question)) {
      const noms = (d.options || []).map((o) => o.libelle);
      for (const n of noms) propose.set(n, (propose.get(n) || 0) + 1);
      if (noms[i]) choisie.set(noms[i], (choisie.get(noms[i]) || 0) + 1);
      choix++;
      faits++;
    } else if (EST_MULLIGAN(d.question)) {
      const lib = (d.options || [])[i]?.libelle || "";
      if (/garde|conserv|keep/i.test(lib)) mulligan.garde++;
      else mulligan.remplace++;
    }
    partie.repondre(r);
  }
}

const lignes = [...propose.keys()].map((n) => {
  const p = propose.get(n) || 0;
  const c = choisie.get(n) || 0;
  return { nom: n, propose: p, prise: c, taux: p ? Number((c / p).toFixed(3)) : 0 };
});
lignes.sort((x, y) => y.taux - x.taux || y.propose - x.propose);

console.log(JSON.stringify({
  joueur: QUI,
  poids: process.env.APPRENTI_POIDS || "(par defaut)",
  donnes: DONNES,
  choix,
  mulligan,
  corporations: lignes,
}, null, 1));

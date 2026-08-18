#!/usr/bin/env node
// QUELLES AMELIORATIONS DE CARTE PHASE L'IA CHOISIT-ELLE ?
//
//   node data/mesures/corporations/ameliorations.mjs <donnes> [joueur]
//   (APPRENTI_POIDS designe le niveau)
//
// Chaque carte Phase a DEUX variantes d'amelioration, A et B (effects.rs,
// table PHASE_UPGRADED). Pour la phase V Recherche, l'arbitraire est net :
//   V-A : +2 piochees, +2 gardees  -> sur la base 2/1 : 4 vues, 3 gardees
//   V-B : +6 piochees, +1 gardee   -> sur la base 2/1 : 8 vues, 2 gardees
// A garde plus, B voit plus. Ce banc dit lequel l'IA prefere, en jouant des
// parties ENTIERES sans rien imposer.
//
// Deux decisions distinctes existent (wasm/src/lib.rs:965-988) : la phase peut
// etre IMPOSEE (« quelle variante ? ») ou LIBRE (« laquelle, et en quelle
// variante ? »). On compte les deux separement : le choix de variante n'a pas
// le meme sens quand la phase est subie.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 40);
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

const EST_AMELIO = (q) => /améliorez (votre carte phase|une carte phase)/i.test(q || "");

const parPhase = {};      // phase -> { A_prise, B_prise, A_proposee, B_proposee }
const phaseImposee = { oui: 0, non: 0 };
const phaseChoisieLibrement = {};  // quand la phase est libre : quelle phase prise
let total = 0, parties = 0;

for (let g = 1; g <= DONNES; g++) {
  const f = [faire(g * 7 + 1, "a"), faire(g * 13 + 3, "b")];
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  let garde = 0;
  while (!partie.termine && ++garde < 200000) {
    const d = partie.decision;
    if (!d) break;
    const r = await f[d.joueur].decider(d, partie.etat);
    if (EST_AMELIO(d.question)) {
      const i = typeof r === "number" ? r : (r?.indice ?? -1);
      const o = (d.options || [])[i];
      if (o) {
        const ph = o.phase, v = o.variante;
        // On compte ce qui est PROPOSE autant que ce qui est pris : un « 12-0 »
        // ne veut rien dire si la variante B n'a jamais ete offerte.
        const neuf = () => ({ A_prise: 0, B_prise: 0, A_proposee: 0, B_proposee: 0 });
        for (const opt of d.options || []) {
          parPhase[opt.phase] = parPhase[opt.phase] || neuf();
          if (opt.variante === "A" || opt.variante === "B") parPhase[opt.phase][opt.variante + "_proposee"]++;
        }
        parPhase[ph] = parPhase[ph] || neuf();
        if (v === "A" || v === "B") parPhase[ph][v + "_prise"]++;
        const libre = d.phase_imposee === null || d.phase_imposee === undefined;
        if (libre) { phaseImposee.non++; phaseChoisieLibrement[ph] = (phaseChoisieLibrement[ph] || 0) + 1; }
        else phaseImposee.oui++;
        total++;
      }
    }
    partie.repondre(r);
  }
  parties++;
}

console.log(JSON.stringify({
  joueur: QUI, poids: process.env.APPRENTI_POIDS || "(defaut)",
  parties, ameliorations_relevees: total,
  variante_par_phase: parPhase,
  phase_imposee: phaseImposee,
  phase_choisie_quand_libre: phaseChoisieLibrement,
}, null, 1));

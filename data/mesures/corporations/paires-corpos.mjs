#!/usr/bin/env node
// L'IA CHOISIT-ELLE LA BONNE DES DEUX CORPORATIONS QU'ON LUI PROPOSE ?
//
//   node data/mesures/corporations/paires-corpos.mjs <donnes> [joueur] > paires.jsonl
//   joueur : « apprenti » (defaut, APPRENTI_POIDS designe le niveau) ou
//            « reflechi » — le temoin a regles ecrites a la main, qui juge une
//            corporation sur son argent de depart. C'est l'heuristique humaine
//            spontanee : il sert d'echelle basse.
//
// Le releve de preference (`corpos.mjs`) donne un taux global : « quand cette
// corporation est proposee, combien de fois est-elle prise ». Il ne dit pas
// CONTRE QUOI. Or c'est la seule chose qui compte : preferer Credicor a Unmi
// n'est pas la meme faute que preferer Credicor a Tharsis Republic.
//
// Ce banc enregistre donc LA PAIRE ENTIERE et le choix fait. Le depouillement
// peut alors repondre a la vraie question : de combien de points l'IA se lese,
// en moyenne, par rapport a un joueur qui prendrait toujours la meilleure des
// deux d'apres le classement mesure sur 799 parties.
//
// On s'arrete des que les deux sieges ont choisi : la suite de la partie ne
// changerait pas le releve. Les deux pieges de `corpos.mjs` s'appliquent ici
// aussi — libelle francais, et fournisseurs construits AVANT `creerPartie`.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 400);
const QUI = process.argv[3] || "apprenti";
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);
const { fournisseurReflechi } = await import(`${RACINE}/joueurs/reflechi.js`);

const pont = await ouvrirPontDepuis(RACINE);

const faire = (graine, nom) =>
  QUI === "reflechi"
    ? fournisseurReflechi(graine, nom)
    : fournisseurApprenti(graine, nom, undefined, pont, BOITES);

const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");

for (let g = 1; g <= DONNES; g++) {
  const a = faire(g * 7 + 1, "a");
  const b = faire(g * 13 + 3, "b");
  const f = [a, b];
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  let faits = 0, garde = 0;
  while (!partie.termine && faits < 2 && ++garde < 60) {
    const d = partie.decision;
    if (!d) break;
    const r = await f[d.joueur].decider(d, partie.etat);
    const i = typeof r === "number" ? r : (r?.indice ?? -1);
    if (EST_CHOIX(d.question)) {
      console.log(JSON.stringify({
        graine: g,
        siege: d.joueur,
        proposees: (d.options || []).map((o) => o.libelle),
        prise: (d.options || [])[i]?.libelle ?? null,
      }));
      faits++;
    }
    partie.repondre(r);
  }
}

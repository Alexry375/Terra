#!/usr/bin/env node
// QUE VAUT UNE CORPORATION QUAND L'IA LA CHOISIT ELLE-MEME ?
//
//   node data/mesures/corporations/choix-libre.mjs <donnes> <depart> > choix-libre.jsonl
//   (APPRENTI_POIDS designe le niveau)
//
// Le tournoi (tournoi-corpos.mjs) TIRAIT AU SORT la corporation : c'est ce qui
// rend le classement honnete, mais cela mesure la force MOYENNE, sur une main
// quelconque. Ici, au contraire, rien n'est impose : l'IA choisit, comme en
// partie. On sait donc ce que vaut chaque corporation DANS LES CONDITIONS OU
// ELLE EST CHOISIE.
//
// LE BIAIS EST ASSUME, et il faut le dire en clair au depouillement : une
// corporation rarement choisie n'apparait que quand l'autre est pire encore.
// Son resultat n'est donc PAS comparable a celui d'une corporation souvent
// choisie. La lecture utile est la comparaison de la MEME corporation entre les
// deux bancs : imposee au hasard d'un cote, choisie de l'autre. Si l'ecart est
// positif, l'IA sait reconnaitre les situations ou elle est bonne.
//
// Second biais, plus discret : ici les DEUX joueurs ont choisi. Dans le
// tournoi, aucun des deux. L'ecart de score reste comparable parce qu'il est
// symetrique, mais le taux de victoire, lui, se lit avec precaution.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 200);
const DEPART = Number(process.argv[3] || 900000);
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);
const pont = await ouvrirPontDepuis(RACINE);

const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");

for (let d = 0; d < DONNES; d++) {
  const graine = DEPART + d;
  const f = [
    fournisseurApprenti(graine * 7 + 1, "a", undefined, pont, BOITES),
    fournisseurApprenti(graine * 13 + 3, "b", undefined, pont, BOITES),
  ];
  const partie = creerPartie(pont, { graine, boites: BOITES });
  const corpo = [null, null];
  const propose = [null, null];
  let garde = 0;
  while (!partie.termine && ++garde < 100000) {
    const dec = partie.decision;
    if (!dec) break;
    const r = await f[dec.joueur].decider(dec, partie.etat);
    if (EST_CHOIX(dec.question)) {
      const i = typeof r === "number" ? r : (r?.indice ?? -1);
      corpo[dec.joueur] = (dec.options || [])[i]?.libelle ?? null;
      propose[dec.joueur] = (dec.options || []).map((o) => o.libelle);
    }
    partie.repondre(r);
  }
  const sc = partie.scores || [];
  console.log(JSON.stringify({
    graine,
    corpo0: corpo[0], corpo1: corpo[1],
    propose0: propose[0], propose1: propose[1],
    score0: sc[0] ?? null, score1: sc[1] ?? null,
    complete: partie.partieComplete === true,
  }));
}

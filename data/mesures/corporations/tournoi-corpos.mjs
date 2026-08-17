#!/usr/bin/env node
// LE TOURNOI DES CORPORATIONS — laquelle fait vraiment gagner ?
//
//   node tournoi-corpos.mjs <donnes> <depart>      (APPRENTI_POIDS = le niveau)
//
// Le releve de preference (`corpos.mjs`) dit ce que l'IA AIME. Il ne dit pas ce
// qui GAGNE : l'IA peut se tromper, et surtout une corporation rarement choisie
// n'est jouee que quand l'autre est pire encore — c'est un biais de selection qui
// la fait paraitre mauvaise quoi qu'il arrive.
//
// LA CORRECTION, et c'est tout le principe de ce banc : **le choix de
// corporation est tire au sort**, jamais decide. Le mulligan aussi. Tout le
// reste de la partie est joue par l'IA, des deux cotes, avec les memes poids.
// La corporation devient alors independante de sa qualite supposee, et la
// comparaison est propre.
//
// Chaque partie donne DEUX observations, une par siege, et chaque donne est
// jouee deux fois avec les sieges echanges : l'avantage de place se compense.
//
// Sortie : une ligne JSON par partie, pour que le depouillement soit refait sans
// rejouer quoi que ce soit.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 40);
const DEPART = Number(process.argv[3] || 500000);
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);

const pont = await ouvrirPontDepuis(RACINE);

// Un hasard SEME, jamais `Math.random` : le banc doit se rejouer a l'identique.
function tirage(graine) {
  let x = (graine >>> 0) || 1;
  return () => {
    x ^= x << 13; x >>>= 0;
    x ^= x >> 17;
    x ^= x << 5; x >>>= 0;
    return x / 4294967296;
  };
}

const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");
const EST_MULLIGAN = (q) => /remplacer vos .* corporations/i.test(q || "");

for (let d = 0; d < DONNES; d++) {
  const graine = DEPART + d;
  // Deux passages sur la MEME donne, avec deux tirages de corporation bien
  // decorreles. Echanger les sieges ne servirait a rien : les deux joueurs sont
  // le meme reseau, donc echanger les fournisseurs ne change pas une virgule.
  // Ce qui doit varier, c'est la corporation — sinon les deux passages
  // produisent la partie identique, et c'est arrive au premier essai.
  for (const passage of [0, 1]) {
    const dedie = tirage((graine ^ (passage * 0x9e3779b9)) >>> 0);
    const a = fournisseurApprenti(graine * 7 + 1, "a", undefined, pont, BOITES);
    const b = fournisseurApprenti(graine * 13 + 3, "b", undefined, pont, BOITES);
    const f = [a, b];
    const partie = creerPartie(pont, { graine, boites: BOITES });
    const corpo = [null, null];
    let garde = 0;
    while (!partie.termine && ++garde < 100000) {
      const dec = partie.decision;
      if (!dec) break;
      let r;
      if (EST_MULLIGAN(dec.question) || EST_CHOIX(dec.question)) {
        const n = (dec.options || []).length || 1;
        r = Math.floor(dedie() * n);
        if (EST_CHOIX(dec.question)) corpo[dec.joueur] = dec.options[r]?.libelle ?? null;
      } else {
        r = await f[dec.joueur].decider(dec, partie.etat);
      }
      partie.repondre(r);
    }
    const sc = partie.scores || [];
    console.log(JSON.stringify({
      graine, passage,
      corpo0: corpo[0], corpo1: corpo[1],
      score0: sc[0] ?? null, score1: sc[1] ?? null,
      complete: partie.partieComplete === true,
    }));
  }
}

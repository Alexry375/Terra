#!/usr/bin/env node
// FAUT-IL REMPLACER SES DEUX CORPORATIONS ? — le banc qui tranche
//
//   node data/mesures/corporations/banc-mulligan.mjs <donnes> <depart> > mulligan.jsonl
//   (APPRENTI_POIDS designe le niveau)
//
// L'IA ne remplace JAMAIS ses corporations : 400 fois sur 400 elle garde
// (mise-en-place-1M.json). Est-ce un choix eclaire, ou une option jamais
// exploree ? Une IA par renforcement n'apprend que ce qu'elle essaie.
//
// LE SEUIL VIENT D'UN CALCUL, PAS D'UNE INTUITION. Garder, c'est prendre le
// meilleur de ses deux corporations. Remplacer, c'est prendre le meilleur de
// deux nouvelles : l'esperance de ce tirage vaut +3,74 points d'ecart de score
// (moyenne du max sur les 120 paires, forces mesurees sur 799 parties). On
// remplace donc si la meilleure des deux qu'on tient est SOUS ce seuil — soit
// toute paire sans Apollo Industries, Tharsis Republic, Exocorp ni Teractor.
//
// LE PLAN DE MESURE EST APPARIE, et c'est ce qui fait sa puissance : la MEME
// donne est jouee DEUX fois pour le meme siege, une fois en remplacant, une
// fois en gardant. Meme adversaire, memes cartes, meme reseau : la seule
// difference entre les deux mondes est la decision testee. On compare le score
// du siege a lui-meme, pas a celui d'en face.
//
// ET ON NE JOUE QUE CE QUI COMPTE : quand la regle ne se declenche pas, les
// deux mondes sont identiques par construction et l'ecart vaut exactement zero.
// Inutile de payer deux parties pour l'apprendre — on sonde d'abord la paire de
// corporations (quelques millisecondes), et on ne joue en entier que les cas ou
// la regle change quelque chose. Les cas non declenches sont comptes a part :
// ils servent a chiffrer la FREQUENCE de la regle, pas son effet.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 80);
const DEPART = Number(process.argv[3] || 700000);
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);
const pont = await ouvrirPontDepuis(RACINE);

const FORCE = {
  "Apollo Industries": 14.02, "Tharsis Republic": 13.71, "Exocorp": 11.88,
  "Teractor Corporation": 4.27, "Sultira": 1.78, "Helion Corporation": -0.49,
  "Thorgate Corporation": -1.74, "Phobolog": -2.18, "Ecoline": -4.28,
  "Unmi": -4.74, "Credicor": -4.77, "Hyperion Systems": -5.35,
  "Interplanetary Cinematics": -5.78, "Mining Guild": -6.45,
  "Inventrix": -6.76, "Saturn Systems": -6.94,
};
const SEUIL = 3.74;

const EST_MULL_CORP = (q) => /remplacer vos .* corporations/i.test(q || "");
const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");
const nomDe = (o) => o?.nom ?? o?.name ?? o?.libelle ?? null;

const fournisseurs = (graine) => [
  fournisseurApprenti(graine * 7 + 1, "a", undefined, pont, BOITES),
  fournisseurApprenti(graine * 13 + 3, "b", undefined, pont, BOITES),
];

// Joue une partie entiere en imposant `remplacer` au seul siege `siege`.
// `remplacer === null` : on n'impose rien, l'IA decide (elle gardera).
async function jouer(graine, siege, remplacer) {
  const f = fournisseurs(graine);
  const partie = creerPartie(pont, { graine, boites: BOITES });
  const corpo = [null, null];
  let vues = null, garde = 0;
  while (!partie.termine && ++garde < 100000) {
    const dec = partie.decision;
    if (!dec) break;
    let r;
    if (EST_MULL_CORP(dec.question) && dec.joueur === siege) {
      vues = (dec.corporations || []).map(nomDe);
      r = remplacer === null
        ? await f[dec.joueur].decider(dec, partie.etat)
        : (remplacer ? 1 : 0);
    } else {
      r = await f[dec.joueur].decider(dec, partie.etat);
    }
    if (EST_CHOIX(dec.question)) {
      const i = typeof r === "number" ? r : (r?.indice ?? -1);
      corpo[dec.joueur] = (dec.options || [])[i]?.libelle ?? null;
    }
    partie.repondre(r);
  }
  const sc = partie.scores || [];
  return {
    vues, corpo: corpo[siege], corpo_adverse: corpo[1 - siege],
    score: sc[siege] ?? null, score_adverse: sc[1 - siege] ?? null,
    complete: partie.partieComplete === true,
  };
}

// Sonde rapide : quelles deux corporations le siege recoit-il ? On s'arrete a
// la decision de mulligan, sans jouer la partie.
async function sonder(graine, siege) {
  const f = fournisseurs(graine);
  const partie = creerPartie(pont, { graine, boites: BOITES });
  let garde = 0;
  while (!partie.termine && ++garde < 40) {
    const dec = partie.decision;
    if (!dec) break;
    if (EST_MULL_CORP(dec.question) && dec.joueur === siege) {
      return (dec.corporations || []).map(nomDe);
    }
    partie.repondre(await f[dec.joueur].decider(dec, partie.etat));
  }
  return null;
}

for (let d = 0; d < DONNES; d++) {
  const graine = DEPART + d;
  for (const siege of [0, 1]) {
    const vues = await sonder(graine, siege);
    const forces = (vues || []).map((n) => (n in FORCE ? FORCE[n] : 0));
    const meilleure = forces.length ? Math.max(...forces) : 0;
    const declenche = vues !== null && meilleure < SEUIL;
    if (!declenche) {
      console.log(JSON.stringify({ graine, siege, vues, force_avant: meilleure, declenche: false }));
      continue;
    }
    const avec = await jouer(graine, siege, true);   // on remplace
    const sans = await jouer(graine, siege, false);  // on garde
    console.log(JSON.stringify({
      graine, siege, vues, force_avant: meilleure, declenche: true,
      remplace: { corpo: avec.corpo, score: avec.score, score_adverse: avec.score_adverse, complete: avec.complete },
      garde:    { corpo: sans.corpo, score: sans.score, score_adverse: sans.score_adverse, complete: sans.complete },
    }));
  }
}

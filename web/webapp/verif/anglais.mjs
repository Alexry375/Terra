// COUVERTURE DE LANGUE — au-delà du contrôle livré.
//
// Le contrôle `01` relève le texte de 60 décisions d'UNE partie. Ce banc-ci
// pousse beaucoup plus loin : il rejoue plusieurs parties entières et fait
// passer par `mots.js` la question ET chaque option de CHAQUE décision, puis
// vérifie qu'il n'en sort ni accent ni mot français. Il ne pilote pas de
// navigateur : `mots.js` ne dépend pas du document.
//
// Il vit HORS du dossier servi : `verif/tests.mjs` (livré avec l'interface)
// interdit à un fichier servi de désigner un chemin qui remonte, et ce banc a
// besoin de `vue/mots.js`. Depuis la racine du workspace :
//
//   node outputs/verif/anglais.mjs [graines...]

import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { question, libelleOption, MOT, STAGES, BADGE_EN } from "../vue/mots.js";
import { normaliser } from "../vue/cartes.js";

// Les mots qui trahissent du français resté à l'écran.
//
// ⚠️ CORRIGÉ LE 05-08, APRÈS AVOIR RÉPARÉ LES CHEMINS DE CE FICHIER. La liste
// d'origine contenait douze mots qui S'ÉCRIVENT PAREIL EN ANGLAIS — « phase »,
// « corporation », « score », « points », « main », « temperature »,
// « oceans », « tour », « fin »… Le banc criait donc sur l'anglais parfaitement
// correct de l'écran : 3 404 fautes annoncées, dont pas une seule vraie. Un
// contrôle qui hurle à chaque phrase ne se lit plus, et c'est probablement pour
// cela qu'on l'a laissé mourir.
//
// N'entre ici qu'un mot dont la présence dans une phrase anglaise ne peut
// s'expliquer que par du français oublié. La détection des ACCENTS, plus bas,
// reste le filet principal — elle, elle ne peut pas se tromper.
const MOTS = new Set(["le", "la", "les", "des", "une", "vos", "votre", "choisissez",
  "carte", "cartes", "joueur", "remplacer", "garder", "manche", "chaleur",
  "victoire", "defausser", "gagnez", "piocher", "pioche",
  "corporations", "valider", "niveau", "tuile", "tuiles",
  "oxygene", "partie"]);
const ACCENTS = /[àâäéèêëîïôöùûüçÀÂÄÉÈÊËÎÏÔÖÙÛÜÇ]/;

const fautes = [];
function verifier(texte, ou) {
  if (typeof texte !== "string") return;
  if (ACCENTS.test(texte)) fautes.push([ou, "accent", texte]);
  for (const m of texte.match(/[A-Za-z']+/g) || []) {
    if (MOTS.has(m.toLowerCase())) fautes.push([ou, "mot francais : " + m, texte]);
  }
}

// Les étiquettes fixes du décor passent par la même moulinette.
for (const [k, v] of Object.entries(MOT)) {
  if (Array.isArray(v)) v.forEach((x) => verifier(x, "MOT." + k));
  else verifier(v, "MOT." + k);
}
for (const s of Object.values(STAGES)) verifier(s.nom, "STAGES");
for (const b of Object.values(BADGE_EN)) verifier(b, "BADGE_EN");

const graines = process.argv.slice(2).map(Number);
const listeGraines = graines.length ? graines : [1, 7, 41, 77, 202, 909, 1234, 3, 55, 404];

const pont = await ouvrirPontDepuis(new URL("..", import.meta.url).pathname);
let decisions = 0;
let options = 0;
const types = new Set();
for (const graine of listeGraines) {
  for (const boites of ["base", "base,decouverte"]) {
    const partie = creerPartie(pont, { graine, boites });
    const f = fournisseurAleatoire(graine);
    while (!partie.termine) {
      const d = partie.decision;
      types.add(d.type);
      decisions++;
      verifier(question(d), `question ${d.type}`);
      (d.options || []).forEach((o, i) => {
        options++;
        verifier(libelleOption(d, o, i, normaliser(o), partie.etat), `option ${d.type}`);
      });
      partie.repondre(f.decider(d, partie.etat));
    }
  }
}

console.log(`parties : ${listeGraines.length * 2}, decisions : ${decisions}, options : ${options}`);
console.log(`types de decision couverts : ${[...types].sort().join(", ")}`);
if (fautes.length) {
  for (const f of fautes.slice(0, 20)) console.log("KO", f.join(" | "));
  console.log(`KO ${fautes.length} texte(s) non anglais`);
  process.exit(1);
}
console.log("OK aucun accent, aucun mot francais dans les textes rendus par mots.js");

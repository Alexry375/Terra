#!/usr/bin/env node
// Banc de tests de la livraison — au-delà des checks fournis.
//
// Il n'utilise QUE le WebAssembly de ce dossier : aucune référence au binaire
// du dépôt (l'équivalence avec lui est l'affaire du check 02).
//
// Ce qui est vérifié ici et nulle part ailleurs :
//   • la partie EST « graine + liste de décisions » (rejeu exact, à tout rang) ;
//   • l'état rendu est VIVANT — il concorde, champ à champ, avec les données que
//     le moteur passe à la politique au même instant. C'est le test qui aurait
//     échoué sur l'instantané de début de manche de la manche 1 ;
//   • la page ne peut rien lire hors du dossier ;
//   • une réponse hors bornes est refusée par le moteur, pas absorbée.

import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire, formeDeLaReponse, nombreDeChoix } from "../fournisseurs.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

let passes = 0;
const echecs = [];
function ok(nom, condition, detail) {
  if (condition) {
    passes++;
    console.log(`  ok   ${nom}`);
  } else {
    echecs.push(nom);
    console.log(`  KO   ${nom}${detail ? " — " + detail : ""}`);
  }
}
function egal(nom, a, b) {
  ok(nom, JSON.stringify(a) === JSON.stringify(b), `${JSON.stringify(a)} ≠ ${JSON.stringify(b)}`);
}

const pont = await ouvrirPontDepuis(RACINE);

// ---------------------------------------------------------------------------
console.log("1. les interrogations du moteur passent bien par le wasm");

const deckBD = pont.lignes({ op: "dump_deck", boites: "base,decouverte" });
const deckB = pont.lignes({ op: "dump_deck", boites: "base" });
ok("dump_deck base+Découverte est plus fourni que base seule",
   deckBD.length > deckB.length, `${deckBD.length} vs ${deckB.length}`);
ok("chaque ligne de dump_deck est un objet JSON nommé",
   deckBD.every((l) => typeof JSON.parse(l).name === "string"));

const corpos = pont.lignes({ op: "dump_corporations", boites: "base,decouverte" });
ok("dump_corporations rend des corporations avec un MC de départ",
   corpos.length > 0 && corpos.every((l) => Number.isInteger(JSON.parse(l).starting_mc)),
   `${corpos.length} corporations`);

const nomsDuDeck = new Set(deckBD.map((l) => JSON.parse(l).name));
const sonde = JSON.parse(
  pont.lignes({ op: "probe", probe: [...nomsDuDeck][0], boites: "base,decouverte" })[0]
);
ok("probe trouve une carte du recensement", sonde.found === true);
const inconnue = JSON.parse(
  pont.lignes({ op: "probe", probe: "carte qui n'existe pas", boites: "base,decouverte" })[0]
);
ok("probe ne fabrique pas de réponse pour une carte inconnue", inconnue.found === false);

let refus = null;
try { pont.lignes({ op: "op-qui-nexiste-pas" }); } catch (e) { refus = e.message; }
ok("une op inconnue est refusée, pas devinée", refus !== null, String(refus));

let refusBoite = null;
try { pont.lignes({ op: "dump_deck", boites: "boite-imaginaire" }); }
catch (e) { refusBoite = e.message; }
ok("une boîte inconnue est refusée", refusBoite !== null, String(refusBoite));

// ---------------------------------------------------------------------------
console.log("2. la partie EST « graine + liste de décisions »");

const boites = "base,decouverte";
const graine = 424242;
const p1 = creerPartie(pont, { graine, boites });
const journal = [];
await jouerJusquAuBout(p1, [fournisseurAleatoire(11), fournisseurAleatoire(12)], (p) => {
  // LES OCCASIONS DE VENTE SONT RELEVÉES AVEC L'ÉTAT, au même instant : elles
  // portent la main que le moteur offre à la vente, et c'est le seul endroit où
  // cette main se lit hors d'une question (voir la concordance « vente »).
  journal.push({ decision: p.decision, etat: p.etat, occasions: p.occasions });
});
ok("la partie se termine d'elle-même", p1.termine && p1.partieComplete);
ok("elle a duré plusieurs manches", p1.manches > 1, `${p1.manches}`);
ok("elle a demandé de nombreuses décisions", journal.length > 50, `${journal.length}`);

const p2 = creerPartie(pont, { graine, boites });
await jouerJusquAuBout(p2, [fournisseurAleatoire(11), fournisseurAleatoire(12)]);
egal("mêmes décisions → mêmes scores", p2.scores, p1.scores);
egal("mêmes décisions → mêmes manches", p2.manches, p1.manches);
egal("mêmes décisions → même liste de décisions", p2.decisions, p1.decisions);

const p3 = creerPartie(pont, { graine: graine + 1, boites });
await jouerJusquAuBout(p3, [fournisseurAleatoire(11), fournisseurAleatoire(12)]);
ok("une autre graine donne une autre partie",
   JSON.stringify(p3.decisions) !== JSON.stringify(p1.decisions) ||
   JSON.stringify(p3.scores) !== JSON.stringify(p1.scores));

// Rejeu à un rang quelconque : le moteur redonne EXACTEMENT la même décision.
for (const k of [0, 1, 7, Math.floor(journal.length / 2), journal.length - 1]) {
  const r = pont.pas(graine, boites, p1.decisions.slice(0, k));
  egal(`rejeu au rang ${k} : même décision`, r.decision, journal[k].decision);
  egal(`rejeu au rang ${k} : même état`, r.etat, journal[k].etat);
}
ok("le rang annoncé est le nombre de décisions déjà prises",
   journal.every((e, i) => e.decision.rang === i));

// ---------------------------------------------------------------------------
console.log("3. l'état rendu est celui du moteur, et il est VIVANT");

const etat0 = journal[0].etat;
ok("l'état a la forme de observe::state_view",
   etat0.planet !== undefined && etat0.planet.temperature_max !== undefined &&
   etat0.decks !== undefined && Array.isArray(etat0.players) &&
   etat0.players.length === 2 && etat0.players[0].tags !== undefined &&
   etat0.players[0].phase_upgrades !== undefined);
ok("les DEUX mains sont visibles (bac à sable)",
   journal.some((e) => e.etat.players[0].hand.length > 0 && e.etat.players[1].hand.length > 0));

// Concordance état ↔ descripteur : les valeurs que le moteur passe à la
// politique au moment du choix doivent être celles de l'état rendu au même
// moment. Un instantané pris plus tôt les ferait diverger.
// ⚠️ LA PREMIÈRE FAMILLE A ÉTÉ DÉPLACÉE LE 28-08 (les-sept-bancs-rouges), et
// c'est un déplacement, pas un retrait. Elle lisait le type
// `discard_payment_count` — « combien de cartes le moteur doit-il prendre pour
// compléter ce paiement ? » —, SUPPRIMÉ par le chantier `regles-de-la-vente` et
// remplacé par la vente libre (`engine/src/policy.rs::vendre_librement`). Le
// compteur ne pouvait donc plus jamais dépasser zéro : le test criait au loup à
// chaque exécution alors que rien n'était cassé, et un banc qu'on n'écoute plus
// ne garde plus rien.
//
// CE QUI A PRIS SA PLACE N'EST PAS UNE QUESTION, C'EST UNE OCCASION. Le moteur
// ne demande jamais « voulez-vous vendre ? » : il fait savoir, avant chacun de
// ses points de décision, qu'ici une vente serait recevable, et il publie avec
// l'occasion LA MAIN du joueur à cet instant (`partie.occasions`). La propriété
// mesurée est exactement la même qu'avant — ce que le moteur passe à la
// politique est ce que l'état rendu montre — mesurée sur le mécanisme qui a
// remplacé l'ancien.
//
// UNE QUATRIÈME FAMILLE S'Y AJOUTE, et elle ne nomme aucun type : toute décision
// qui DIT la main (`d.main`) doit dire celle de l'état. Aujourd'hui c'est le
// mulligan des projets ; demain ce sera ce qui viendra, sans qu'on ait à
// réécrire ce test.
let vivants = { vente: 0, main: 0, phase: 0, mainDite: 0 };
let discordances = 0;
const ecarts = [];
function ecart(quoi) {
  discordances++;
  if (ecarts.length < 5) ecarts.push(quoi);
}
/** La liste de cartes du moteur et la main de l'état sont-elles la même ? */
function memeMain(cartes, joueur) {
  return Array.isArray(cartes) && cartes.length === joueur.hand.length
    && cartes.every((c, i) => c.nom === joueur.hand[i].name);
}
for (const { decision: d, etat, occasions } of journal) {
  const j = etat.players[d.joueur];
  for (const occ of occasions) {
    vivants.vente++;
    if (!memeMain(occ.main, etat.players[occ.joueur])) {
      ecart(`occasion de vente ${occ.numero}, joueur ${occ.joueur}`);
    }
  }
  if (Array.isArray(d.main)) {
    vivants.mainDite++;
    if (!memeMain(d.main, j)) ecart(`${d.type} rang ${d.rang} : le champ « main »`);
  }
  if (d.type === "choose_build") {
    vivants.main++;
    for (const o of d.options) {
      if (!o.carte || o.carte.nom !== j.hand[o.indice_main].name) {
        ecart(`choose_build rang ${d.rang}, option ${o.indice_main}`);
      }
    }
  }
  if (d.type === "pick_phase") {
    vivants.phase++;
    // La phase de la manche précédente est exclue par le moteur.
    if (d.options.some((o) => o.phase === j.previous_phase && j.previous_phase !== 0)) {
      ecart(`pick_phase rang ${d.rang} : la phase précédente est encore offerte`);
    }
  }
}
ok("les quatre familles de concordance ont été rencontrées",
   vivants.vente > 0 && vivants.main > 0 && vivants.phase > 0 && vivants.mainDite > 0,
   JSON.stringify(vivants));
ok("aucune discordance entre l'état rendu et ce que le moteur passe à la politique",
   discordances === 0, `${discordances} discordances : ${ecarts.join(" | ")}`);

ok("le score courant est rendu par le moteur pour les deux joueurs",
   journal.every((e) => e.etat.players.every((j) => Number.isInteger(j.score))));
ok("les paramètres planétaires ne reculent jamais",
   journal.every((e, i) => i === 0 ||
     (e.etat.planet.oxygen >= journal[i - 1].etat.planet.oxygen &&
      e.etat.planet.temperature >= journal[i - 1].etat.planet.temperature &&
      e.etat.planet.oceans >= journal[i - 1].etat.planet.oceans)));

// ---------------------------------------------------------------------------
console.log("4. les descripteurs de décision sont exploitables sans connaître les règles");

const types = new Set(journal.map((e) => e.decision.type));
ok("plusieurs familles de décisions ont été traversées", types.size >= 6, [...types].join(","));
// ⚠️ LE CAS « MULTIPLE » A ÉTÉ CORRIGÉ LE 28-08 (les-sept-bancs-rouges). Il
// exigeait `Number.isInteger(d.a_choisir)` sur TOUTE décision multiple. Or le
// mulligan des projets est à nombre LIBRE : son descripteur ne porte pas
// `a_choisir`, et c'est précisément ainsi qu'il DIT « de 0 à n »
// (`web/webapp/wasm/src/lib.rs::project_mulligan`, et le reste du dépôt le lit
// déjà ainsi : `fournisseurs.js:62`, `verif/pilote.py:133`). Le test était donc
// rouge sur toute partie complète, pour un descripteur parfaitement répondable.
// L'exigence remise à l'endroit : des options exploitables toujours, et un
// nombre COHÉRENT quand il y en a un.
const echoueRepondable = [];
ok("chaque décision est répondable à partir de son seul descripteur",
   journal.every(({ decision: d }) => {
     const bon = (() => {
       switch (formeDeLaReponse(d)) {
         case "montant": return Number.isInteger(d.minimum) && Number.isInteger(d.maximum) &&
                                d.minimum <= d.maximum;
         case "multiple":
           if (!Array.isArray(d.options) || d.options.length === 0) return false;
           if (!d.options.every((o) => typeof o.libelle === "string")) return false;
           // Absent = nombre libre ; présent = il doit tenir dans les options.
           return d.a_choisir === undefined
             || (Number.isInteger(d.a_choisir) && d.a_choisir >= 0
                 && d.a_choisir <= d.options.length);
         default: return nombreDeChoix(d) > 0 && typeof d.question === "string" &&
                         d.options.every((o) => typeof o.libelle === "string");
       }
     })();
     if (!bon && echoueRepondable.length < 3) {
       echoueRepondable.push(`${d.type} rang ${d.rang}`);
     }
     return bon;
   }), echoueRepondable.join(" | "));

// ---------------------------------------------------------------------------
console.log("5. une réponse hors bornes est refusée par le moteur");

const p4 = creerPartie(pont, { graine: 7, boites });
const decisionAvant = JSON.stringify(p4.decision);
let refusHorsBornes = null;
try {
  p4.repondre(99999); // aucun choix ne porte cet indice
} catch (e) { refusHorsBornes = e.message; }
ok("un indice hors bornes remonte une erreur du moteur",
   refusHorsBornes !== null && /hors de/.test(refusHorsBornes), String(refusHorsBornes));
// Une réponse refusée ne doit pas rester dans la liste : sinon la partie est
// empoisonnée et tous les coups suivants échouent.
egal("une réponse refusée ne reste pas dans la liste", p4.decisions, []);
egal("la décision en cours est inchangée après un refus",
     JSON.stringify(p4.decision), decisionAvant);
let reprise = null;
try { p4.repondre(0); } catch (e) { reprise = e.message; }
ok("la partie reste jouable après un refus", reprise === null, String(reprise));

// ---------------------------------------------------------------------------
console.log("5 bis. les requêtes malformées sont refusées, jamais devinées");

function refuse(nom, requete, motif) {
  const r = pont.appeler(requete);
  const m = r.ok === false ? r.erreur : null;
  ok(nom, m !== null && (!motif || motif.test(m)), `réponse: ${JSON.stringify(r).slice(0, 120)}`);
}
refuse("graine illisible refusée (pas rabattue sur 0)",
       { op: "bilan", games: 1, seed: 1.5, boites }, /seed/);
refuse("graine négative refusée", { op: "bilan", games: 1, seed: -3, boites }, /seed/);
refuse("games absurde refusé avant d'atteindre le moteur",
       { op: "bilan", games: "18446744073709551615", seed: 0, boites }, /games/);
refuse("decisions non-liste refusée (pas « aucune décision »)",
       { op: "pas", seed: 1, decisions: "pas-un-tableau", boites }, /decisions/);
refuse("probe non-chaîne refusé", { op: "probe", probe: { a: 1 }, boites }, /probe/);
refuse("probe_filler négatif refusé", { op: "probe", probe: "Grass", probe_filler: -3, boites },
       /probe_filler/);
refuse("probe_choice non-liste refusé",
       { op: "probe", probe: "Grass", probe_choice: 3, boites }, /probe_choice/);
refuse("probe_phase hors bornes refusé",
       { op: "probe", probe: "Grass", probe_phase: 9, boites }, /probe.phase/);

// Une graine au-delà de 2^53 doit désigner SA partie, pas celle de 0.
const grande = "18446744073709551615";
const bGrande = pont.lignes({ op: "bilan", games: 2, seed: grande, boites: "base" })[0];
const bZero = pont.lignes({ op: "bilan", games: 2, seed: "0", boites: "base" })[0];
ok("une graine au-delà de 2^53 n'est pas confondue avec 0", bGrande !== bZero);
egal("la même grande graine redonne le même bilan",
     pont.lignes({ op: "bilan", games: 2, seed: grande, boites: "base" })[0], bGrande);

// ---------------------------------------------------------------------------
console.log("5 ter. le correctif d'échantillonnage de `usize` est bien actif");

// `Hohmann Transfer Shipping` accorde une carte Phase améliorée : le moteur y
// consulte `Policy::choose_option`, donc `rng.gen_range(0..n)` sur un `usize`.
// Sans le `rand` corrigé (`outputs/vendor/rand-usize64`), le WebAssembly tire
// sur 32 bits et répond « 5A » là où le binaire natif répond « 4B ». C'est le
// seul témoin interne : sans lui, une recompilation sans `[patch.crates-io]`
// ferait rediverger 19 cartes en silence, et seul le check 02 — extérieur à la
// livraison — s'en apercevrait.
const temoin = JSON.parse(
  pont.lignes({ op: "probe", probe: "Hohmann Transfer Shipping", boites: "base,decouverte" })[0]
);
egal("le tirage sur usize suit la largeur 64 bits (rand corrigé actif)",
     temoin.upgrades, ["4B"]);

// ---------------------------------------------------------------------------
console.log("6. le dossier est autosuffisant");

function fichiersServis(dir, acc = []) {
  for (const e of readdirSync(dir)) {
    if (e === "wasm" || e === "node_modules") continue; // sources de construction
    const p = join(dir, e);
    if (statSync(p).isDirectory()) fichiersServis(p, acc);
    else if (/\.(html|js|mjs|css)$/.test(e)) acc.push(p);
  }
  return acc;
}
const servis = fichiersServis(RACINE);
ok("des fichiers servis ont été trouvés", servis.length >= 5, `${servis.length}`);

// ⚠️ RÉÉCRIT LE 28-08 (les-sept-bancs-rouges), ET C'EST LA MESURE QUI CHANGE,
// PAS LE VERDICT. Ce test comptait les REMONTÉES `../` et les confrontait à une
// liste blanche de trois noms de fichiers. Dix-neuf remontées plus tard il était
// rouge en permanence — alors que dix-huit d'entre elles pointent vers des
// fichiers INTERNES au dossier servi (`verif/x.mjs` remonte vers
// `web/webapp/pont.js`, qui est dedans). Ce n'est pas la remontée qu'il faut
// interdire, c'est la SORTIE : on résout donc chaque chemin relatif et l'on
// regarde OÙ IL TOMBE. Autoriser toutes les remontées aurait rendu ce test
// vert et muet ; ceci le rend vert et parlant.
//
// DEUX CERCLES, PARCE QU'IL Y A DEUX SORTES DE FICHIERS ICI. Ceux que la page
// sert et qu'un navigateur va chercher (tout sauf `verif/`) ne doivent RIEN
// réclamer au-dessus de `web/webapp/` — à une exception nommée près, le fichier
// de poids du joueur apprenti. Les bancs de `verif/`, eux, ne sont jamais servis
// à une page : ils tournent depuis le dépôt et ont le droit d'aller chercher le
// binaire natif ou les poids, mais pas de sortir du dépôt.
const DEPOT = resolve(RACINE, "../..");
/** La seule sortie légitime du dossier servi, nommée fichier par fichier. */
const SORTIE_TOLEREE = new Set(["joueurs/apprenti.js"]);
const relatifs = (t) => {
  const out = [];
  // Les chaînes qui SONT un chemin relatif, et les `url(...)` des feuilles de
  // style, qui n'en sont pas une.
  for (const m of t.matchAll(/["'`](\.\.?\/[^"'`\n]*)["'`]/g)) out.push(m[1]);
  for (const m of t.matchAll(/url\(\s*(\.\.?\/[^)\s]*)\s*\)/g)) out.push(m[1]);
  return out;
};
const sorties = [];
const horsDepot = [];
const externes = [];
let chemins = 0;
for (const f of servis) {
  const t = readFileSync(f, "utf8");
  const rel = f.slice(RACINE.length + 1);
  const banc = rel.startsWith("verif/");
  for (const l of t.split("\n")) {
    if (/^\s*(\/\/|\*|<!--)/.test(l)) continue;
    for (const cible of relatifs(l)) {
      chemins++;
      const resolu = resolve(dirname(f), cible.split(/[?#]/)[0]);
      const dedans = resolu === RACINE || resolu.startsWith(RACINE + "/");
      if (!dedans && !banc && !SORTIE_TOLEREE.has(rel)) {
        sorties.push(`${rel}: ${cible}`);
      }
      if (banc && resolu !== DEPOT && !resolu.startsWith(DEPOT + "/")) {
        horsDepot.push(`${rel}: ${cible}`);
      }
    }
    if (/https?:\/\//.test(l) && !/w3\.org|localhost|127\.0\.0\.1/.test(l)) {
      externes.push(`${f}: ${l.trim()}`);
    }
  }
}
// Une mesure qui ne trouverait plus un seul chemin relatif ne prouverait rien :
// elle serait verte pour n'avoir rien lu.
ok("des chemins relatifs ont bien été résolus", chemins >= 15, `${chemins} chemins`);
ok("aucun chemin ne sort du dossier servi", sorties.length === 0, sorties.join(" | "));
ok("aucun banc de vérification ne sort du dépôt", horsDepot.length === 0, horsDepot.join(" | "));
ok("aucune ressource externe n'est chargée", externes.length === 0, externes.join(" | "));

// ---------------------------------------------------------------------------
console.log("7. plusieurs parties complètes, boîtes de base seules comprises");

for (const [g, b] of [[1, "base"], [2, "base"], [3, "base,decouverte"], [4, "base,decouverte"]]) {
  const p = creerPartie(pont, { graine: g, boites: b });
  await jouerJusquAuBout(p, [fournisseurAleatoire(g * 3), fournisseurAleatoire(g * 5)]);
  ok(`partie complète graine ${g} (${b})`,
     p.termine && p.partieComplete && p.manches > 1 &&
     p.scores.length === 2 && p.scores.every(Number.isInteger),
     JSON.stringify({ scores: p.scores, manches: p.manches }));
}

console.log(`\n${passes} tests passés, ${echecs.length} échoués`);
if (echecs.length) {
  for (const e of echecs) console.log(`  ÉCHEC : ${e}`);
  process.exit(1);
}

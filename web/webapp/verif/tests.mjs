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
  journal.push({ decision: p.decision, etat: p.etat });
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
let vivants = { mc: 0, main: 0, phase: 0 };
let discordances = 0;
for (const { decision: d, etat } of journal) {
  const j = etat.players[d.joueur];
  if (d.type === "discard_payment_count") {
    vivants.mc++;
    if (d.mc !== j.mc) discordances++;
    // Le moteur a déjà retiré la carte posée de la main quand il demande le
    // paiement : la main du descripteur et celle de l'état sont donc la même.
    if (d.main.length !== j.hand.length) discordances++;
    if (d.main.some((c, i) => c.nom !== j.hand[i].name)) discordances++;
  }
  if (d.type === "choose_build") {
    vivants.main++;
    for (const o of d.options) {
      if (!o.carte || o.carte.nom !== j.hand[o.indice_main].name) discordances++;
    }
  }
  if (d.type === "pick_phase") {
    vivants.phase++;
    // La phase de la manche précédente est exclue par le moteur.
    if (d.options.some((o) => o.phase === j.previous_phase && j.previous_phase !== 0)) {
      discordances++;
    }
  }
}
ok("les trois familles de concordance ont été rencontrées",
   vivants.mc > 0 && vivants.main > 0 && vivants.phase > 0, JSON.stringify(vivants));
ok("aucune discordance entre l'état rendu et ce que le moteur passe à la politique",
   discordances === 0, `${discordances} discordances`);

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
ok("chaque décision est répondable à partir de son seul descripteur",
   journal.every(({ decision: d }) => {
     switch (formeDeLaReponse(d)) {
       case "montant": return Number.isInteger(d.minimum) && Number.isInteger(d.maximum) &&
                              d.minimum <= d.maximum;
       case "multiple": return Array.isArray(d.options) && Number.isInteger(d.a_choisir) &&
                               d.a_choisir <= d.options.length;
       default: return nombreDeChoix(d) > 0 && typeof d.question === "string" &&
                       d.options.every((o) => typeof o.libelle === "string");
     }
   }));

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
const remontees = [];
const externes = [];
for (const f of servis) {
  const t = readFileSync(f, "utf8");
  for (const l of t.split("\n")) {
    if (/^\s*(\/\/|\*|<!--)/.test(l)) continue;
    if (/["'`]\.\.\//.test(l) && !l.includes("../pont.js") && !l.includes("../partie.js") &&
        !l.includes("../fournisseurs.js")) remontees.push(`${f}: ${l.trim()}`);
    if (/https?:\/\//.test(l) && !/w3\.org|localhost|127\.0\.0\.1/.test(l)) {
      externes.push(`${f}: ${l.trim()}`);
    }
  }
}
ok("aucun chemin ne remonte hors du dossier", remontees.length === 0, remontees.join(" | "));
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

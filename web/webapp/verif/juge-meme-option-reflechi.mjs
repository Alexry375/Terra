#!/usr/bin/env node
// **(l-etalon-natif) L'ÉTALON RUST ET L'ÉTALON JAVASCRIPT PRENNENT LA MÊME
// DÉCISION, DÉCISION PAR DÉCISION.**
//
//   node web/webapp/verif/juge-meme-option-reflechi.mjs [graines] [joueur] [boites]
//                                                        [--adversaire=<nom>] [--echange]
//
// C'est le seul critère qui compte pour ce lot. Le joueur « reflechi » est
// l'étalon de mesure du projet : tous les taux de victoire annoncés depuis le
// mois d'août se lisent contre lui. Le porter en Rust n'a de sens que s'il
// décide EXACTEMENT comme l'original ; un portage approchant vaut moins que rien,
// puisqu'il déplacerait la référence sans le dire.
//
// Le banc ne fabrique aucune situation de laboratoire : il fait jouer la partie
// ENTIÈRE aux deux côtés sur la même graine, et compare les deux listes de
// réponses. Comme chaque réponse change la partie, un seul désaccord fait
// diverger tout ce qui suit — c'est bien plus sévère qu'une comparaison
// situation par situation, et c'est pour cela qu'on procède ainsi.
//
// **IL SAIT AUSSI DIRE NON.** Le second argument nomme le joueur JavaScript
// opposé au portage : « reflechi » par défaut, « hasard » pour éprouver le juge
// lui-même. Un banc de concordance vert quoi qu'on lui présente ne prouve rien ;
// celui-ci doit rapporter des désaccords et sortir en échec quand on lui oppose
// le joueur qui tire au sort.
//
// **ET IL SAIT ALLER SUR LE TERRAIN OÙ LES DÉFAUTS SE CACHENT.** Par défaut, les
// deux sièges portent l'étalon : c'est le duel le plus rapide, donc celui qu'on
// peut lancer sur vingt graines. Mais un duel où PERSONNE ne saisit d'occasion
// de vente avant la question laisse dormir toute une moitié de la mécanique —
// les désaccords nos 4, 5 et 6 du carnet n'y étaient pas visibles, et ils ont
// pourtant fait rouge le duel dont dépendent tous les chiffres du projet.
// `--adversaire=apprenti` assoit le joueur artificiel en face, aux deux côtés à
// la fois, et `--echange` intervertit les sièges. C'est LENT (le joueur
// artificiel rejoue la manche à chaque décision) : deux graines suffisent.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { fournisseurReflechi } from "../joueurs/reflechi.js";
import { fournisseurApprenti } from "../joueurs/apprenti.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(RACINE, "../..");

// OÙ EST LA BALANCE NATIVE. Le chantier peut compiler dans un dossier à lui pour
// ne pas se disputer le verrou du compilateur avec un autre : on regarde aux
// deux endroits, en commençant par celui que les bancs du dépôt utilisent.
const CANDIDATS = [
  resolve(DEPOT, "engine/target/release/duel"),
  "/home/alexis/.agentic-workspace/target-etalon/release/duel",
];
const BIN = CANDIDATS.find((p) => existsSync(p));
if (!BIN) {
  console.log(`l'exécutable « duel » est introuvable : ${CANDIDATS.join(", ")}`);
  process.exit(2);
}

// Les joueurs JavaScript que le juge sait opposer au portage. Table écrite à la
// main, jamais découverte en parcourant un dossier : un banc dont la liste
// dépend du contenu d'un répertoire ne se répète pas d'une machine à l'autre.
const JOUEURS = {
  reflechi: (graine, nom) => fournisseurReflechi(graine, nom),
  hasard: (graine, nom) => fournisseurAleatoire(graine, nom),
  // L'apprenti a besoin du pont et de la liste des boîtes : il DÉCIDE en
  // rejouant la manche, donc il lui faut un moteur sous la main. Il n'est jamais
  // le joueur JUGÉ — c'est l'étalon qu'on juge — mais il peut être l'adversaire.
  apprenti: (graine, nom) => fournisseurApprenti(graine, nom, undefined, pont, boites),
};

// LES DRAPEAUX SONT RETIRÉS AVANT DE LIRE LES ARGUMENTS DE POSITION, sans quoi
// un `--echange` glissé en troisième place serait pris pour un nom de joueur et
// le banc dirait « joueur inconnu » au lieu de faire son travail.
const argv = process.argv.slice(2);
const echange = argv.includes("--echange");
const drapeauAdv = argv.find((a) => a.startsWith("--adversaire="));
const adversaire = drapeauAdv ? drapeauAdv.slice("--adversaire=".length) : "reflechi";
const positions = argv.filter((a) => !a.startsWith("--"));

const graines = Number(positions[0] ?? 20);
const joueur = positions[1] ?? "reflechi";
const boites = positions[2] ?? "base,decouverte";

if (!Number.isInteger(graines) || graines < 1) {
  console.log(`nombre de graines invalide : « ${process.argv[2]} »`);
  process.exit(2);
}
if (!JOUEURS[joueur]) {
  console.log(
    `joueur inconnu : « ${joueur} » — joueurs connus : ${Object.keys(JOUEURS).join(", ")}`,
  );
  process.exit(2);
}
if (!JOUEURS[adversaire]) {
  console.log(
    `adversaire inconnu : « ${adversaire} » — joueurs connus : ${Object.keys(JOUEURS).join(", ")}`,
  );
  process.exit(2);
}

/**
 * La graine du camp, comme dans la balance (`duel.mjs`, `graineDuCamp`). L'étalon
 * n'en consomme pas une goutte ; le joueur qui tire au sort, si — et il doit la
 * recevoir par le même mélange que partout ailleurs, faute de quoi le juge ne
 * comparerait pas ce que la balance compare.
 */
function graineDuCamp(graine, camp) {
  const sel = camp === 0 ? 0x9e3779b9 : 0x85ebca6b;
  let x = (graine * 2654435761) ^ sel;
  x = Math.imul(x ^ (x >>> 15), 0x2545f491) >>> 0;
  return x || 1;
}

/**
 * Une écriture JSON dont l'ordre des clefs ne dépend pas de qui a construit
 * l'objet.
 *
 * **Ce n'est PAS un assouplissement du juge.** Deux valeurs différentes gardent
 * deux écritures différentes : trier les clefs est une bijection sur les valeurs
 * comparées, elle ne peut rien effacer. Ce qu'elle retire est le seul détail que
 * personne ne veut mesurer ici : `serde_json` écrit les clefs par ordre
 * alphabétique, tandis qu'un objet littéral JavaScript garde l'ordre de sa
 * source — l'étalon rend `{joueur, cartes}` là où le Rust rend
 * `{cartes, joueur}`. Sans ce tri, le juge crierait au désaccord sur deux ventes
 * identiques.
 */
function canonique(v) {
  if (Array.isArray(v)) return `[${v.map(canonique).join(",")}]`;
  if (v && typeof v === "object") {
    const clefs = Object.keys(v).sort();
    return `{${clefs.map((k) => `${JSON.stringify(k)}:${canonique(v[k])}`).join(",")}}`;
  }
  return JSON.stringify(v ?? null);
}

const pont = await ouvrirPontDepuis(RACINE);
let decisions = 0;
let accords = 0;
let parties = 0;
let scoresDifferents = 0;
const desaccords = [];

for (let g = 1; g <= graines; g++) {
  // ---- l'étalon Rust joue la partie entière, aux deux sièges
  let rust;
  try {
    rust = JSON.parse(
      execFileSync(
        BIN,
        [
          "--journal",
          "reflechi",
          adversaire,
          "--graine",
          String(g),
          "--boites",
          boites,
          ...(echange ? ["--echange"] : []),
        ],
        { cwd: DEPOT, maxBuffer: 1e9 },
      )
        .toString()
        .trim()
        .split("\n")
        .pop(),
    );
  } catch (e) {
    console.log(`✗ graine ${g} : la balance native a échoué — ${String(e.message).split("\n")[0]}`);
    process.exit(1);
  }

  // ---- le joueur JavaScript joue la même, aux deux sièges
  // LE CAMP A EST CELUI QU'ON JUGE — l'étalon, ou son remplaçant quand on
  // éprouve le juge lui-même. Le camp B est l'adversaire, le même des deux
  // côtés. Les graines de camp et l'ordre des sièges suivent la balance à la
  // lettre (`duel.mjs`) : comparer autre chose que ce que la balance joue ne
  // prouverait rien sur la balance.
  const a = JOUEURS[joueur](graineDuCamp(g, 0), joueur);
  const b = JOUEURS[adversaire](graineDuCamp(g, 1), adversaire);
  const partie = creerPartie(pont, { graine: g, boites });
  // LA FORME DE CHAQUE QUESTION, relevée au moment où elle est posée. Elle ne
  // sert pas à juger — le verdict ne porte que sur les réponses — mais à rendre
  // un désaccord LISIBLE : « décision 412 » ne dit rien, « décision 412,
  // choose_build, joueur 1 » envoie droit au bon endroit du portage.
  const formes = [];
  await jouerJusquAuBout(partie, echange ? [b, a] : [a, b], (p) => {
    formes[p.decisions.length] = { type: p.decision.type, joueur: p.decision.joueur };
  });
  const js = partie.decisions;
  parties++;

  const n = Math.max(js.length, rust.decisions.length);
  let premier = -1;
  for (let i = 0; i < n; i++) {
    decisions++;
    const x = canonique(js[i] ?? null);
    const y = canonique(rust.decisions[i] ?? null);
    if (x === y) {
      accords++;
      continue;
    }
    if (premier < 0) premier = i;
    if (desaccords.length < 10) {
      const f = formes[i];
      const forme = f ? `${f.type}, joueur ${f.joueur}` : "vente saisie hors question";
      desaccords.push(`graine ${g}, décision ${i} (${forme}) : natif ${y}, javascript ${x}`);
    }
  }
  const memeScore = canonique(partie.scores) === canonique(rust.scores);
  if (!memeScore) scoresDifferents++;
  console.log(
    `graine ${g} : ${js.length} décisions côté javascript, ${rust.decisions.length} côté natif — ` +
      `${memeScore ? "mêmes scores" : "SCORES DIFFÉRENTS"} ` +
      `(natif ${JSON.stringify(rust.scores)}, javascript ${JSON.stringify(partie.scores)})` +
      `${premier < 0 ? "" : ` — premier désaccord à la décision ${premier}`}`,
  );
}

console.log(
  `${parties} partie(s) « reflechi » contre « ${adversaire} »` +
    `${echange ? ", sièges échangés" : ""}, ` +
    `${decisions} décision(s) comparées, ${accords} accord(s)`,
);
for (const d of desaccords) console.log(`  ✗ ${d}`);

// UN BANC QUI N'A RIEN JOUÉ NE PROUVE RIEN, et « 0 désaccord sur 0 décision »
// est le vert le plus facile du monde. Le juge dont celui-ci s'inspire
// (`juge-meme-option.mjs`) porte le même plancher, et pour la même raison : le
// contrôle scellé qui l'appelle en exige davantage, mais le juge doit savoir
// dire non tout seul, sans compter sur qui l'appelle.
const PLANCHER = 200;
if (decisions < PLANCHER) {
  console.log(
    `KO seulement ${decisions} décision(s) comparées : il en faut au moins ${PLANCHER} pour conclure`,
  );
  process.exit(1);
}
if (accords !== decisions) {
  console.log(`KO ${decisions - accords} désaccord(s) sur ${decisions} décisions comparées`);
  process.exit(1);
}
if (scoresDifferents > 0) {
  // Impossible en principe — mêmes réponses, même moteur, mêmes scores — mais un
  // banc qui ne le vérifierait pas laisserait passer une divergence de comptage
  // final sans un mot.
  console.log(`KO ${scoresDifferents} partie(s) aux scores différents malgré des réponses égales`);
  process.exit(1);
}
console.log(
  `OK aucun désaccord : sur ${parties} parties entières et ${decisions} décisions, ` +
    `l'étalon natif répond comme l'étalon javascript`,
);

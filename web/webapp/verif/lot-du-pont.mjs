#!/usr/bin/env node
// **LE BANC DU LOT « LE PONT NE TRICHE PLUS ».**
//
//   node web/webapp/verif/lot-du-pont.mjs
//
// Une vérification par défaut corrigé, et une par invariant que ce lot installe.
// Les grandes campagnes vivent ailleurs — `juge-l-avenir-cache.mjs` compare le
// navigateur au natif sur vingt-quatre parties, `occasion-dans-les-deux-sens.mjs`
// éprouve des milliers de ventes, `le-binaire-est-a-jour.mjs` recompile la
// source. Ce banc-ci est le FILET SERRÉ : il tient en quelques secondes, il
// nomme précisément ce qui casse, et il couvre les cinq territoires du lot —
// la graine d'essais, le rebattage de l'avenir, l'occasion de vente numérotée,
// le fichier `cards.json` partagé, et le binaire `terra.wasm` livré.
//
// Les trois défauts que ce lot ferme :
//
//   V1 — LA VOYANCE. Essayer un coup rejouait la vraie partie depuis sa vraie
//        graine : le joueur lisait les cartes qu'il allait piocher, le bonus des
//        tuiles Océan face cachée, l'ordre du paquet des corporations.
//   V2 — LA VENTE QUI REMONTE LE TEMPS. Plusieurs occasions de vente peuvent
//        être ouvertes au même point d'arrêt ; une vente décidée en regardant la
//        seconde était consommée à la première, donc appliquée à un instant que
//        le joueur n'avait pas devant les yeux.
//   D23/D25 — LES DEUX FICHIERS QUI DÉRIVENT. `data/cards.json` contre
//        `web/webapp/assets/cards.json`, et `terra.wasm` contre sa source.

import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { readFileSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { ouvrirPontDepuis } from "../pont.js";
import { creerPartie, jouerJusquAuBout, offrirLesOccasions } from "../partie.js";
import { fournisseurAleatoire } from "../fournisseurs.js";
import { fournisseurApprenti } from "../joueurs/apprenti.js";

const LIVRAISON = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(LIVRAISON, "..", "..");
const BOITES = "base,decouverte";
const POIDS = join(DEPOT, "data/poids/apprenti.txt");
const BIN = join(DEPOT, "engine/target/release/jouer");

const pont = await ouvrirPontDepuis(LIVRAISON);

// ── le carnet de vérifications ──────────────────────────────────────────────
const carnet = [];
function test(nom, fn) {
  carnet.push({ nom, fn });
}
const jsonDe = (x) => JSON.stringify(x);
const md5 = (o) => createHash("md5").update(o).digest("hex");
const lire = (chemin) => readFileSync(join(DEPOT, chemin), "utf8");
const mainDe = (etat, s) => (((etat && etat.players) || [])[s] || {}).hand || [];
const idsDe = (main) => main.map((c) => c && c.id).join(",");

/** Une partie témoin, rejouable, arrêtée à un point utile. */
function partieTemoin(graine, pas) {
  const p = creerPartie(pont, { graine, boites: BOITES });
  const h = [fournisseurAleatoire(graine * 31, "a"), fournisseurAleatoire(graine * 37 + 1, "b")];
  let garde = 0;
  while (!p.termine && p.decisions.length < pas) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const d = p.decision;
    if (!d) break;
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  return p;
}
const TEMOIN = partieTemoin(4242, 30);
const D30 = TEMOIN.decisions.slice();

/**
 * **UN POINT OÙ L'ESSAI SE VOIT.** Tous les points d'une partie ne sont pas
 * sensibles au rebattage : à un rang où le moteur ne consomme rien d'inconnu
 * avant de reposer sa question, l'essai rend légitimement le même écran que le
 * rejeu nu, et la réponse visible ne bouge pas. Un banc calé sur un rang fixe
 * n'éprouve donc rien du tout — il peut rester vert sur un pont qui ne rebat
 * jamais. On CHERCHE donc le point : le premier rang où l'essai diffère du
 * rejeu nu ET où deux graines d'essais diffèrent entre elles.
 */
function pointRevelateur(graine, depart, fin) {
  for (let r = depart; r <= fin; r++) {
    const d = TEMOIN.decisions.slice(0, r + 1);
    const nu = jsonDe(pont.pas(graine, BOITES, d));
    const a = jsonDe(pont.pas(graine, BOITES, d, { graine: 11, rang: r }));
    const b = jsonDe(pont.pas(graine, BOITES, d, { graine: 12, rang: r }));
    if (nu !== a && a !== b) return { decisions: d, rang: r };
  }
  throw new Error(`aucun rang entre ${depart} et ${fin} ne montre le rebattage : le pont ne rebat rien`);
}
// La recherche a lieu au chargement, mais elle ne doit PAS faire exploser le
// banc : un pont qui ne rebat rien ferait alors mourir le module avant le
// premier verdict, et le contrôle lirait une trace de pile au lieu de « ROUGE ».
// On garde donc l'échec de côté et on le rend par une vérification à part.
let REVELATEUR = null;
let ECHEC_REVELATEUR = null;
try {
  REVELATEUR = pointRevelateur(4242, 4, D30.length - 1);
} catch (e) {
  ECHEC_REVELATEUR = e.message;
  REVELATEUR = { decisions: D30, rang: D30.length - 1 };
}
const DR = REVELATEUR.decisions;
const RR = REVELATEUR.rang;

test("il existe un rang de la partie témoin où l'essai se voit", () => {
  if (ECHEC_REVELATEUR) throw new Error(ECHEC_REVELATEUR);
  return `rang ${RR} sur ${D30.length} décisions`;
});

// ════════════════════════════════════════════════════════════════════════════
// I. LA GRAINE D'ESSAIS — le quatrième argument de `pont.pas`
// ════════════════════════════════════════════════════════════════════════════

test("sans quatrième argument, `pas` fait exactement ce qu'il faisait avant le lot", () => {
  const a = jsonDe(pont.pas(4242, BOITES, D30));
  const b = jsonDe(pont.pas(4242, BOITES, D30, undefined));
  if (a !== b) throw new Error("passer `undefined` ne rend pas la même chose que ne rien passer");
  return `${a.length} octets de réponse identiques`;
});

test("sans quatrième argument, le rejeu est déterministe", () => {
  const a = jsonDe(pont.pas(4242, BOITES, D30));
  const b = jsonDe(pont.pas(4242, BOITES, D30));
  if (a !== b) throw new Error("deux rejeux nus de la même liste divergent");
  return null;
});

test("une graine d'essais présente change le résultat : l'essai n'est plus le rejeu nu", () => {
  const nu = jsonDe(pont.pas(4242, BOITES, DR));
  const essai = jsonDe(pont.pas(4242, BOITES, DR, { graine: 7, rang: RR }));
  if (nu === essai) throw new Error("l'essai rend le rejeu nu : la voyance V1 est intacte");
  return `rang ${RR}`;
});

test("zéro est une VALEUR de graine d'essais, pas une absence", () => {
  // `GRAINE_ESSAIS_DEFAUT = 0` côté natif : c'est la PRÉSENCE de la clef qui
  // allume le rebattage, jamais sa valeur. Un pont qui testerait `graine != 0`
  // laisserait le mode par défaut du natif voyant.
  const nu = jsonDe(pont.pas(4242, BOITES, DR));
  const zero = jsonDe(pont.pas(4242, BOITES, DR, { graine: 0, rang: RR }));
  if (nu === zero) throw new Error("la graine d'essais 0 est traitée comme une absence de rebattage");
  return null;
});

test("deux graines d'essais différentes rendent deux essais différents", () => {
  const a = jsonDe(pont.pas(4242, BOITES, DR, { graine: 11, rang: RR }));
  const b = jsonDe(pont.pas(4242, BOITES, DR, { graine: 12, rang: RR }));
  if (a === b) throw new Error("le rebattage est décoratif : la graine d'essais ne change rien");
  return null;
});

test("la même graine d'essais rend deux fois le même essai", () => {
  const a = jsonDe(pont.pas(4242, BOITES, DR, { graine: 11, rang: RR }));
  const b = jsonDe(pont.pas(4242, BOITES, DR, { graine: 11, rang: RR }));
  if (a !== b) throw new Error("l'essai n'est pas reproductible : quelque chose n'est pas ensemencé");
  return null;
});

test("le rang de l'essai entre dans la graine dérivée", () => {
  // `joueur::graine_du_rejeu` mêle la graine d'essais, la graine de partie, le
  // rang au journal et le compte d'occasions. Deux rangs voisins ne peuvent donc
  // pas partager le même paquet rebattu.
  const a = jsonDe(pont.pas(4242, BOITES, D30.slice(0, 20), { graine: 11, rang: 19 }));
  const b = jsonDe(pont.pas(4242, BOITES, D30.slice(0, 20), { graine: 11, rang: 18 }));
  if (a === b) throw new Error("changer le rang de l'essai ne change rien au rebattage");
  return null;
});

test("la graine de la PARTIE entre elle aussi dans la graine dérivée", () => {
  const a = jsonDe(pont.pas(4242, BOITES, [0, 0], { graine: 11, rang: 1 }));
  const b = jsonDe(pont.pas(4243, BOITES, [0, 0], { graine: 11, rang: 1 }));
  if (a === b) throw new Error("deux parties différentes rendent le même essai");
  return null;
});

test("une graine d'essais sans rang est REFUSÉE, pas devinée", () => {
  let refus = null;
  try {
    pont.appeler({ op: "pas", seed: 4242, boites: BOITES, decisions: D30, graine_essais: 5 });
  } catch (e) {
    refus = e.message;
  }
  const r = pont.appeler({ op: "pas", seed: 4242, boites: BOITES, decisions: D30, graine_essais: 5 });
  if (refus === null && r.ok !== false) {
    throw new Error("le pont accepte une graine d'essais sans rang et devine le moment de l'essai");
  }
  return String(refus || r.erreur).slice(0, 80);
});

test("un essai ne salit pas la vraie partie : le rejeu nu ne bouge pas après", () => {
  const avant = jsonDe(pont.pas(4242, BOITES, D30));
  pont.pas(4242, BOITES, D30, { graine: 99, rang: D30.length - 1 });
  pont.pas(4242, BOITES, D30, { graine: 100, rang: 3 });
  const apres = jsonDe(pont.pas(4242, BOITES, D30));
  if (avant !== apres) throw new Error("le rejeu nu a changé après un essai : le harnais garde un état");
  return null;
});

// ════════════════════════════════════════════════════════════════════════════
// II. LE REBATTAGE DE L'AVENIR — ce qui change, et surtout ce qui ne change pas
// ════════════════════════════════════════════════════════════════════════════

const RANG = RR;
const nuEtat = pont.pas(4242, BOITES, DR).etat;
const essaiEtat = pont.pas(4242, BOITES, DR, { graine: 11, rang: RANG }).etat;
const siegeEssai = pont.pas(4242, BOITES, DR).decision?.joueur ?? 0;

test("le rebattage laisse la planète intacte : le passé ne se rejoue pas", () => {
  if (jsonDe(nuEtat.planet) !== jsonDe(essaiEtat.planet)) {
    throw new Error(`planète ${jsonDe(nuEtat.planet)} ≠ ${jsonDe(essaiEtat.planet)}`);
  }
  return null;
});

/**
 * **LE BALAYAGE DU DÉJÀ-VU.** Les cinq « invariants » qui tenaient ici avant —
 * paquet au même compte, corporations au même compte, tuiles révélées
 * identiques, mains de même longueur — étaient mesurés en UN point choisi pour
 * être révélateur, et ils y sont vrais par accident : plus loin dans la partie
 * ils tombent LÉGITIMEMENT (une carte piochée après la dernière observation a
 * parfaitement le droit de changer, c'est même tout le sujet du lot). Un banc
 * qui les affirme mesure donc le point, pas la propriété.
 *
 * La vraie propriété, celle que `DejaVu` promet, est plus étroite et se vérifie
 * partout : **ce que le joueur a DÉJÀ VU ne bouge pas.** Les cartes qu'il avait
 * en main au rang précédent sont, dans le même ordre, encore là au rang suivant
 * quelle que soit la graine d'essais ; les tuiles Océan déjà retournées restent
 * en tête de la liste, à l'identique. Seule la QUEUE — ce qui vient d'être
 * pioché ou retourné — a le droit de changer.
 *
 * Et la propriété se contrôle DANS LES DEUX SENS : le balayage compte aussi les
 * points où l'essai change réellement quelque chose. Un pont qui ne rebat rien
 * satisferait le déjà-vu trivialement ; il tombe sur ce second compte.
 */
function balayerLeDejaVu(graines, profondeur, pasDeRang, grainesEssais) {
  let compares = 0;
  let mordus = 0;
  let points = 0;
  let oceans = 0;
  // **LES FAUTES SONT COLLECTÉES, PAS LANCÉES.** Le balayage tourne au
  // chargement du module : une exception ici tuerait le banc avant son premier
  // verdict, et le contrôle lirait une trace de pile au lieu de « ROUGE ».
  const fautesMain = [];
  const fautesOcean = [];
  const idl = (m) => m.map((c) => c && c.id);
  for (const graine of graines) {
    const p = partieTemoin(graine, profondeur);
    const D = p.decisions;
    for (let r = 6; r < D.length; r += pasDeRang) {
      if (D[r] && D[r].vendre !== undefined) continue; // une vente n'est pas une décision
      const nu = pont.pas(graine, BOITES, D.slice(0, r + 1));
      const avant = pont.pas(graine, BOITES, D.slice(0, r));
      if (!nu.etat || !avant.etat) continue;
      points++;
      const nuJson = jsonDe(nu);
      const ocAvant = (avant.etat.planet && avant.etat.planet.oceans_revealed_tiles) || [];
      for (const ge of grainesEssais) {
        let e;
        try {
          e = pont.pas(graine, BOITES, D.slice(0, r + 1), { graine: ge, rang: r });
        } catch (err) {
          fautesMain.push(`graine ${graine} rang ${r} essais ${ge} : le moteur refuse l'essai — ${err.message}`);
          continue;
        }
        if (!e.etat) {
          fautesMain.push(`graine ${graine} rang ${r} essais ${ge} : l'essai ne rend aucun état`);
          continue;
        }
        compares++;
        if (jsonDe(e) !== nuJson) mordus++;
        for (const s of [0, 1]) {
          const dejaVu = new Set(idl(mainDe(avant.etat, s)));
          const gardeNu = idl(mainDe(nu.etat, s)).filter((i) => dejaVu.has(i)).join(",");
          const gardeEssai = idl(mainDe(e.etat, s)).filter((i) => dejaVu.has(i)).join(",");
          if (gardeNu !== gardeEssai && fautesMain.length < 200) {
            fautesMain.push(
              `graine ${graine} rang ${r} essais ${ge} siège ${s} : le rebattage a touché du DÉJÀ-VU — ` +
                `[${gardeNu}] devient [${gardeEssai}]`,
            );
          } else if (gardeNu !== gardeEssai) {
            fautesMain.push("…");
          }
        }
        const ocEssai = (e.etat.planet && e.etat.planet.oceans_revealed_tiles) || [];
        if (ocAvant.length > 0) {
          oceans++;
          const intact =
            ocAvant.length <= ocEssai.length && ocAvant.every((t, i) => jsonDe(t) === jsonDe(ocEssai[i]));
          if (!intact) {
            fautesOcean.push(
              `graine ${graine} rang ${r} essais ${ge} : une tuile Océan DÉJÀ RETOURNÉE a changé — ` +
                `${jsonDe(ocAvant)} n'est plus en tête de ${jsonDe(ocEssai)}`,
            );
          }
        }
      }
    }
  }
  return { compares, mordus, points, oceans, fautesMain, fautesOcean };
}

let BALAYAGE = { compares: 0, mordus: 0, points: 0, oceans: 0, fautesMain: [], fautesOcean: [] };
let ECHEC_BALAYAGE = null;
try {
  BALAYAGE = balayerLeDejaVu([4242, 7, 13, 101, 2026, 55], 150, 3, [11, 12, 77]);
} catch (e) {
  ECHEC_BALAYAGE = e.message;
}

test("le rebattage ne touche JAMAIS une carte que le joueur avait déjà en main", () => {
  if (ECHEC_BALAYAGE) throw new Error(`le balayage n'a pas pu aller au bout : ${ECHEC_BALAYAGE}`);
  if (BALAYAGE.fautesMain.length > 0) {
    throw new Error(
      `${BALAYAGE.fautesMain.length} faute(s) sur ${BALAYAGE.compares} essais — ${BALAYAGE.fautesMain[0]}`,
    );
  }
  if (BALAYAGE.compares < 500) {
    throw new Error(`balayage trop maigre : ${BALAYAGE.compares} comparaisons, il en faut plus de 500`);
  }
  return `${BALAYAGE.compares} essais sur ${BALAYAGE.points} points, deux sièges à chaque fois`;
});

test("le rebattage ne retourne pas autrement une tuile Océan DÉJÀ révélée", () => {
  if (ECHEC_BALAYAGE) throw new Error(`le balayage n'a pas pu aller au bout : ${ECHEC_BALAYAGE}`);
  if (BALAYAGE.fautesOcean.length > 0) {
    throw new Error(
      `${BALAYAGE.fautesOcean.length} faute(s) sur ${BALAYAGE.oceans} points — ${BALAYAGE.fautesOcean[0]}`,
    );
  }
  if (BALAYAGE.oceans < 100) {
    throw new Error(
      `${BALAYAGE.oceans} points seulement avec une tuile déjà retournée : le balayage ne prouve rien`,
    );
  }
  return `${BALAYAGE.oceans} points où au moins une tuile était déjà retournée`;
});

test("et le rebattage MORD : sur une large part des points, l'essai change l'écran", () => {
  if (ECHEC_BALAYAGE) throw new Error(`le balayage n'a pas pu aller au bout : ${ECHEC_BALAYAGE}`);
  // Le contre-sens du test précédent. Un pont qui ne rebat rien garde le déjà-vu
  // intact par construction : c'est ici qu'il tombe.
  const part = BALAYAGE.compares === 0 ? 0 : BALAYAGE.mordus / BALAYAGE.compares;
  if (BALAYAGE.mordus === 0) throw new Error("aucun essai ne change quoi que ce soit : le pont ne rebat rien");
  // Le seuil est bas À DESSEIN. La part exacte dépend de combien, parmi les
  // rangs balayés, sont des points SENSIBLES — c'est-à-dire des points où le
  // moteur consomme quelque chose d'inconnu avant de reposer sa question. Elle
  // vaut 20,6 % sur la livraison ; en exiger 20 % ferait de ce banc un
  // détecteur de rangs, pas de rebattage. Ce qui se contrôle ici, c'est qu'une
  // part FRANCHE des essais mord, pas un taux.
  if (part < 0.05) {
    throw new Error(
      `seulement ${BALAYAGE.mordus} essais sur ${BALAYAGE.compares} changent l'écran (${(part * 100).toFixed(1)} %)`,
    );
  }
  return `${BALAYAGE.mordus} essais sur ${BALAYAGE.compares} diffèrent du rejeu nu (${(part * 100).toFixed(1)} %)`;
});

test("le rebattage laisse les scores courants intacts", () => {
  const a = (nuEtat.players || []).map((j) => j.score).join(",");
  const b = (essaiEtat.players || []).map((j) => j.score).join(",");
  if (a !== b) throw new Error(`scores ${a} ≠ ${b}`);
  return null;
});

test("le rebattage laisse les ressources des deux joueurs intactes", () => {
  const res = (e) => (e.players || []).map((j) => [j.mc, j.plants, j.heat, j.steel, j.titanium].join("/")).join(" ");
  if (res(nuEtat) !== res(essaiEtat)) throw new Error(`${res(nuEtat)} ≠ ${res(essaiEtat)}`);
  return null;
});

test("le rebattage change bien l'AVENIR : plusieurs graines, plusieurs avenirs", () => {
  const vus = new Set();
  for (let s = 1; s <= 8; s++) vus.add(jsonDe(pont.pas(4242, BOITES, DR, { graine: s, rang: RANG })));
  if (vus.size < 4) {
    throw new Error(`8 graines d'essais ne produisent que ${vus.size} avenir(s) distinct(s)`);
  }
  return `${vus.size} avenirs distincts sur 8 graines`;
});

test("le rebattage n'est écrit NULLE PART en JavaScript", () => {
  // Le critère B du lot : un Fisher-Yates recopié en JS dériverait du Rust au
  // premier changement, et le natif et le navigateur ne joueraient plus la même
  // partie. Le pont doit APPELER le Rust, pas le refaire.
  const interdits = /gen_range|fisher|yates|deck\.swap|\.swap\([ij]|rebattre|brasser/i;
  const fichiers = ["pont.js", "partie.js", "fournisseurs.js", "interface.js", "joueurs/apprenti.js"];
  const coupables = [];
  for (const f of fichiers) {
    const chemin = join(LIVRAISON, f);
    if (!existsSync(chemin)) continue;
    for (const [i, l] of readFileSync(chemin, "utf8").split("\n").entries()) {
      if (/^\s*(\/\/|\*|\/\*)/.test(l)) continue; // les commentaires ont le droit d'en parler
      if (interdits.test(l)) coupables.push(`${f}:${i + 1} ${l.trim().slice(0, 70)}`);
    }
  }
  if (coupables.length > 0) throw new Error(`brassage recopié en JavaScript : ${coupables.join(" | ")}`);
  return `${fichiers.length} fichiers relus`;
});

test("le pont APPELLE le rebattage Rust du moteur", () => {
  const src = lire("web/webapp/wasm/src/lib.rs");
  const manque = [];
  if (!/joueur::rebattre_le_reste|rebattre_le_reste\(/.test(src)) manque.push("rebattre_le_reste");
  if (!/joueur::ecarter_les_cartes_du_futur|ecarter_les_cartes_du_futur\(/.test(src)) {
    manque.push("ecarter_les_cartes_du_futur");
  }
  if (manque.length > 0) throw new Error(`le wasm n'appelle pas ${manque.join(" ni ")}`);
  return null;
});

test("le moteur natif et le navigateur essaient sur le MÊME paquet rebattu", () => {
  // La preuve courte de ce que `juge-l-avenir-cache.mjs` mesure en grand : à
  // graine de partie ET graine d'essais égales, les deux joueurs choisissent la
  // même option. Un seul essai vu sur un autre paquet ferait diverger la suite.
  // **UN BINAIRE ABSENT EST UNE FAUTE, PAS UNE EXCUSE.** Rendre ici un texte
  // d'explication comptait la vérification comme RÉUSSIE alors qu'elle n'avait
  // pas eu lieu : le banc se déclarait vert sur ce qu'il n'avait pas mesuré.
  if (!existsSync(BIN)) {
    throw new Error(
      `binaire natif absent (${BIN}) : la comparaison natif/navigateur ne peut pas avoir lieu — ` +
        `compiler d'abord (cargo build --release --manifest-path engine/Cargo.toml)`,
    );
  }
  const G = 5;
  const E = 4321;
  const rust = JSON.parse(
    execFileSync(BIN, ["--graine", String(G), "--poids", POIDS, "--boites", BOITES, "--graine-essais", String(E)], {
      cwd: DEPOT,
      maxBuffer: 1e9,
      stdio: ["ignore", "pipe", "ignore"],
    })
      .toString()
      .trim()
      .split("\n")
      .pop(),
  );
  const a = fournisseurApprenti(1, "apprenti", POIDS, pont, BOITES, undefined, E);
  const b = fournisseurApprenti(2, "apprenti", POIDS, pont, BOITES, undefined, E);
  const p = creerPartie(pont, { graine: G, boites: BOITES });
  const N = 60;
  let garde = 0;
  while (!p.termine && p.decisions.length < N) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    offrirLesOccasionsSync(p, [a, b]);
    if (p.termine || p.decisions.length >= N) break;
    const d = p.decision;
    if (!d) break;
    p.repondre([a, b][d.joueur].decider(d, p.etat));
  }
  for (let i = 0; i < Math.min(N, p.decisions.length); i++) {
    if (jsonDe(p.decisions[i]) !== jsonDe(rust.decisions[i])) {
      throw new Error(
        `décision ${i} : natif ${jsonDe(rust.decisions[i])}, navigateur ${jsonDe(p.decisions[i])}`,
      );
    }
  }
  return `${Math.min(N, p.decisions.length)} décisions identiques (graine ${G}, essais ${E})`;
});

/** `offrirLesOccasions` sans `await` : les fournisseurs de ce banc sont synchrones. */
function offrirLesOccasionsSync(p, fournisseurs) {
  let garde = 0;
  let encore = true;
  while (encore) {
    encore = false;
    if (++garde > 5000) throw new Error("boucle d'occasions anormalement longue");
    for (const occ of p.occasions) {
      const f = fournisseurs[occ.joueur];
      if (!f || typeof f.vendre !== "function") continue;
      const cartes = f.vendre(occ, p.etat);
      if (Array.isArray(cartes) && cartes.length > 0) {
        p.vendre({ cartes, joueur: occ.joueur, occasion: occ.numero });
        encore = true;
        break;
      }
    }
  }
}

// ════════════════════════════════════════════════════════════════════════════
// III. L'OCCASION DE VENTE NUMÉROTÉE
// ════════════════════════════════════════════════════════════════════════════

/** Un point de la partie témoin où au moins une occasion est ouverte. */
function pointAOccasion(graine, minimum = 1) {
  const p = creerPartie(pont, { graine, boites: BOITES });
  const h = [fournisseurAleatoire(graine * 31, "a"), fournisseurAleatoire(graine * 37 + 1, "b")];
  let garde = 0;
  while (!p.termine) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const parSiege = new Map();
    for (const o of p.occasions) {
      if (!parSiege.has(o.joueur)) parSiege.set(o.joueur, []);
      parSiege.get(o.joueur).push(o);
    }
    for (const [s, liste] of parSiege) {
      if (liste.length >= minimum && liste[0].main.length > 0) return { partie: p, siege: s, liste };
    }
    const d = p.decision;
    if (!d) break;
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  return null;
}

/**
 * La première occasion de vente d'une partie où deux graines d'essais donnent
 * deux avenirs. C'est la preuve que le champ `occasion_essais` est lu : sans
 * lui, le pont dériverait la même graine aux deux appels.
 */
function occasionRevelatrice(graine, plafond = 400) {
  const p = creerPartie(pont, { graine, boites: BOITES });
  const h = [fournisseurAleatoire(graine * 31, "a"), fournisseurAleatoire(graine * 37 + 1, "b")];
  let garde = 0;
  while (!p.termine && p.decisions.length < plafond) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    for (const o of p.occasions) {
      if (!o.main || o.main.length === 0) continue;
      const D = [...p.decisions, { vendre: { cartes: [0], joueur: o.joueur, occasion: o.numero } }];
      const e = (g) => jsonDe(pont.pas(graine, BOITES, D, { graine: g, rang: p.decisions.length, occasion: o.numero }));
      if (e(5) !== e(6)) return { numero: o.numero, rang: p.decisions.length };
    }
    const d = p.decision;
    if (!d) break;
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  return null;
}

/**
 * Un instant où les DEUX sièges ont une occasion de vente ouverte, chacun avec
 * des cartes en main. C'est le seul point où « une vente adressée à l'autre
 * siège » veut dire quelque chose.
 */
function pointADeuxSieges(graines) {
  for (const graine of graines) {
    const p = creerPartie(pont, { graine, boites: BOITES });
    const h = [fournisseurAleatoire(graine * 31, "a"), fournisseurAleatoire(graine * 37 + 1, "b")];
    let garde = 0;
    while (!p.termine) {
      if (++garde > 100000) throw new Error("boucle anormalement longue");
      const utiles = p.occasions.filter((o) => o.main && o.main.length > 0);
      const s0 = utiles.find((o) => o.joueur === 0);
      const s1 = utiles.find((o) => o.joueur === 1);
      if (s0 && s1) return { partie: p, graine, siege: 0, autre: 1, occAutre: s1 };
      const d = p.decision;
      if (!d) break;
      p.repondre(h[d.joueur].decider(d, p.etat));
    }
  }
  return null;
}

const UNE = pointAOccasion(4242, 1);
const DEUX = pointAOccasion(1, 2);
const DEUX_SIEGES = pointADeuxSieges([1, 4242, 7, 13, 101, 2026, 55, 3, 9, 21]);

test("le pont publie le compteur d'occasions et la liste des occasions ouvertes", () => {
  if (!UNE) throw new Error("aucune occasion ouverte trouvée dans une partie entière");
  const p = UNE.partie;
  if (!Number.isInteger(p.occasionsOuvertes)) throw new Error("`occasions` n'est pas un entier");
  if (!Array.isArray(p.occasions)) throw new Error("`occasions_ouvertes` n'est pas une liste");
  for (const o of p.occasions) {
    if (!Number.isInteger(o.numero) || !Number.isInteger(o.joueur) || !Array.isArray(o.main)) {
      throw new Error(`occasion mal formée : ${jsonDe(o)}`);
    }
  }
  return `${p.occasions.length} occasion(s) ouverte(s), compteur ${p.occasionsOuvertes}`;
});

test("le numéro d'une occasion est toujours strictement inférieur au compteur", () => {
  const p = UNE.partie;
  for (const o of p.occasions) {
    if (o.numero >= p.occasionsOuvertes) {
      throw new Error(`occasion ${o.numero} publiée avec un compteur de ${p.occasionsOuvertes}`);
    }
  }
  return null;
});

test("les numéros d'occasion se suivent sans reculer sur toute une partie", () => {
  const p = creerPartie(pont, { graine: 3, boites: BOITES });
  const h = [fournisseurAleatoire(93, "a"), fournisseurAleatoire(112, "b")];
  let precedent = -1;
  let compteur = 0;
  let garde = 0;
  while (!p.termine) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    if (p.occasionsOuvertes < compteur) throw new Error("le compteur d'occasions a reculé");
    compteur = p.occasionsOuvertes;
    for (const o of p.occasions) {
      if (o.numero <= precedent) throw new Error(`occasion ${o.numero} republiée après ${precedent}`);
      precedent = o.numero;
    }
    const d = p.decision;
    if (!d) break;
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  return `${compteur} occasions sur la partie`;
});

test("une entrée de vente numérotée est honorée à SON occasion", () => {
  const { partie: p, siege, liste } = UNE;
  const o = liste[0];
  const i = o.main.length - 1;
  const cible = o.main[i].id;
  const r = pont.pas(p.graine, BOITES, [...p.decisions, { vendre: { cartes: [i], joueur: siege, occasion: o.numero } }]);
  const apres = mainDe(r.etat, siege);
  if (apres.some((c) => c && c.id === cible) && apres.length !== o.main.length) {
    throw new Error(`la carte ${cible} est restée et une autre est partie`);
  }
  return `occasion ${o.numero}, carte ${cible}`;
});

test("une entrée de vente SANS numéro reste acceptée : l'écran de jeu n'a pas bougé", () => {
  const { partie: p, siege, liste } = UNE;
  const r = pont.pas(p.graine, BOITES, [...p.decisions, { vendre: { cartes: [0], joueur: siege } }]);
  if (r && r.ok === false) throw new Error(`une vente sans numéro est refusée : ${r.erreur}`);
  return null;
});

test("une entrée numérotée dans le FUTUR est refusée, et la partie n'en garde rien", () => {
  const { partie: p, siege } = UNE;
  const avant = jsonDe(p.etat);
  const n = p.decisions.length;
  let refusee = false;
  try {
    p.vendre({ cartes: [0], joueur: siege, occasion: p.occasionsOuvertes + 1000 });
  } catch (e) {
    refusee = true;
  }
  if (!refusee) throw new Error("une vente numérotée pour une occasion inexistante a été consommée");
  if (p.decisions.length !== n) throw new Error("l'entrée refusée est restée dans la liste des décisions");
  if (jsonDe(p.etat) !== avant) throw new Error("le refus a modifié la partie");
  return "rejet net, partie intacte";
});

test("un numéro d'occasion MAL FORMÉ est refusé, pas ignoré", () => {
  // Le trou serré ici : `as_u64` rend « rien » sur `"3"`, `1.5`, `-1`, `true`.
  // S'en tenir là sautait la garde, et la vente retombait à la PREMIÈRE
  // occasion du siège — c'est-à-dire le défaut V2, rouvert par une valeur qui a
  // transité par un relais ou une concaténation.
  const { partie: p, siege } = UNE;
  const bancal = ["3", 1.5, -1, true, [2], { n: 2 }];
  const passes = [];
  for (const v of bancal) {
    const r = pont.appeler({
      op: "pas",
      seed: p.graine,
      boites: BOITES,
      decisions: [...p.decisions, { vendre: { cartes: [0], joueur: siege, occasion: v } }],
    });
    if (!r || r.ok !== false) passes.push(jsonDe(v));
  }
  if (passes.length > 0) {
    throw new Error(`numéros mal formés acceptés en silence : ${passes.join(", ")}`);
  }
  return `${bancal.length} formes refusées`;
});

test("un rang d'essai INATTEIGNABLE est refusé, pas replié sur la dernière manche", () => {
  // Sans cette garde, la passe 1 ne trouve jamais son moment, `reprise` porte la
  // DERNIÈRE manche jouée, et le pont rend un écran sans rapport avec l'essai
  // demandé — sans que personne ne le sache.
  const { partie: p } = UNE;
  const r = pont.appeler({
    op: "pas",
    seed: p.graine,
    boites: BOITES,
    decisions: p.decisions,
    graine_essais: 7,
    rang_essais: p.decisions.length + 5000,
  });
  if (!r || r.ok !== false) throw new Error("un rang d'essai hors de la partie a été accepté en silence");
  const o = pont.appeler({
    op: "pas",
    seed: p.graine,
    boites: BOITES,
    decisions: p.decisions,
    graine_essais: 7,
    rang_essais: p.decisions.length,
    occasion_essais: p.occasionsOuvertes + 100000,
  });
  if (!o || o.ok !== false) throw new Error("une occasion d'essai qui ne s'ouvre jamais a été acceptée en silence");
  return `${r.erreur}`;
});

test("le 4e argument de `pas` doit être un objet : la graine seule est refusée", () => {
  // `if (essais)` ferait taire l'essai sur `0` — une graine d'essais valable,
  // celle du natif par défaut — et le coup serait essayé sur la VRAIE partie.
  // C'est le défaut V1 par la porte de service.
  const { partie: p } = UNE;
  const refuse = (x) => {
    try {
      pont.pas(p.graine, BOITES, p.decisions, x);
      return false;
    } catch (e) {
      return true;
    }
  };
  const laisses = [0, 7, "7", true].filter((x) => !refuse(x));
  if (laisses.length > 0) throw new Error(`4e argument non-objet accepté : ${laisses.join(", ")}`);
  if (refuse(undefined)) throw new Error("l'absence du 4e argument doit rester le comportement d'avant");
  if (!refuse({ graine: 3 })) throw new Error("un descripteur d'essai sans rang doit être refusé");
  return "0, 7, \"7\", true refusés ; absent accepté ; sans rang refusé";
});

test("le numéro n'est PAS décoratif : deux numéros, deux parties (défaut V2)", () => {
  if (!DEUX) throw new Error("aucun point à deux occasions du même siège trouvé");
  const { partie: p, siege, liste } = DEUX;
  const v = (n) => jsonDe(pont.pas(p.graine, BOITES, [...p.decisions, { vendre: { cartes: [0], joueur: siege, occasion: n } }]));
  if (v(liste[0].numero) === v(liste[1].numero)) {
    throw new Error(`les occasions ${liste[0].numero} et ${liste[1].numero} rendent la même partie`);
  }
  return `occasions ${liste[0].numero} et ${liste[1].numero} du siège ${siege}`;
});

test("sans numéro, la vente retombe à la PREMIÈRE occasion — le comportement d'avant", () => {
  const { partie: p, siege, liste } = DEUX;
  const v = (n) => {
    const x = { cartes: [0], joueur: siege };
    if (n !== undefined) x.occasion = n;
    return jsonDe(pont.pas(p.graine, BOITES, [...p.decisions, { vendre: x }]));
  };
  if (v(undefined) !== v(liste[0].numero)) {
    throw new Error("une entrée sans numéro ne tombe plus à la première occasion : la compatibilité est cassée");
  }
  return "c'est précisément le trou que le numéro bouche";
});

test("une vente adressée à l'AUTRE siège n'est pas consommée par le premier", () => {
  // Le point d'épreuve est CHERCHÉ : il faut un instant où les DEUX sièges ont
  // une occasion ouverte avec des cartes à vendre. Se contenter du point de
  // `DEUX` et rendre « pas d'occasion de l'autre siège » comptait la
  // vérification comme réussie sans qu'elle ait eu lieu.
  if (!DEUX_SIEGES) {
    throw new Error(
      "aucun instant, dans les parties témoins, où les deux sièges ont une occasion ouverte : " +
        "la vérification n'a pas pu avoir lieu",
    );
  }
  const { partie: p, siege, autre, occAutre } = DEUX_SIEGES;
  const r = pont.pas(p.graine, BOITES, [
    ...p.decisions,
    { vendre: { cartes: [0], joueur: autre, occasion: occAutre.numero } },
  ]);
  const moi = idsDe(mainDe(p.etat, siege));
  const moiApres = idsDe(mainDe(r.etat, siege));
  if (moi !== moiApres) throw new Error("la vente de l'autre siège a touché ma main");
  // Et elle a bien MORDU chez l'autre : sinon on aurait prouvé qu'une vente ne
  // fait rien du tout, ce qui n'est pas la propriété visée.
  const luiAvant = idsDe(mainDe(p.etat, autre));
  const luiApres = idsDe(mainDe(r.etat, autre));
  if (luiAvant === luiApres) {
    throw new Error(`la vente adressée au siège ${autre} ne lui a rien retiré : [${luiAvant}]`);
  }
  return `siège ${siege} intact, siège ${autre} passe de ${luiAvant.split(",").length} à ${luiApres.split(",").length} cartes`;
});

test("une entrée de vente sans « cartes » est REFUSÉE, pas devinée", () => {
  const { partie: p, siege } = UNE;
  const r = pont.appeler({
    op: "pas",
    seed: p.graine,
    boites: BOITES,
    decisions: [...p.decisions, { vendre: { joueur: siege } }],
  });
  if (r.ok !== false) throw new Error("une vente sans liste de cartes a été acceptée");
  return String(r.erreur).slice(0, 80);
});

test("un indice de vente hors bornes est REFUSÉ, pas rogné", () => {
  const { partie: p, siege, liste } = UNE;
  const r = pont.appeler({
    op: "pas",
    seed: p.graine,
    boites: BOITES,
    decisions: [...p.decisions, { vendre: { cartes: [liste[0].main.length + 50], joueur: siege } }],
  });
  if (r.ok !== false) throw new Error("un indice hors bornes a été accepté");
  return String(r.erreur).slice(0, 80);
});

test("un indice de vente en double est REFUSÉ", () => {
  const { partie: p, siege } = UNE;
  const r = pont.appeler({
    op: "pas",
    seed: p.graine,
    boites: BOITES,
    decisions: [...p.decisions, { vendre: { cartes: [0, 0], joueur: siege } }],
  });
  if (r.ok !== false) throw new Error("un indice en double a été accepté");
  return String(r.erreur).slice(0, 80);
});

test("une occasion saisie n'est plus republiée", () => {
  const { partie: p, siege, liste } = UNE;
  const o = liste[0];
  const r = pont.pas(p.graine, BOITES, [...p.decisions, { vendre: { cartes: [0], joueur: siege, occasion: o.numero } }]);
  const encore = (r.occasions_ouvertes || []).some((x) => x.numero === o.numero && x.joueur === siege);
  if (encore) throw new Error(`l'occasion ${o.numero} est republiée après avoir été saisie`);
  return null;
});

test("un essai peut porter sur une OCCASION de vente, pas seulement sur une décision", () => {
  // Là encore, toutes les occasions ne sont pas sensibles : celle de la mise en
  // place précède le premier tirage inconnu, et l'essai y rend légitimement la
  // même chose pour toute graine. On CHERCHE donc une occasion révélatrice,
  // plutôt que de parier sur la première venue — un banc calé sur une occasion
  // sourde resterait vert sur un pont qui ignore `occasion_essais`.
  const trouvee = occasionRevelatrice(4242);
  if (!trouvee) throw new Error("aucune occasion de vente ne montre le rebattage : `occasion_essais` est ignorée");
  return `occasion ${trouvee.numero} au rang ${trouvee.rang}`;
});

test("un fournisseur SANS méthode `vendre` traverse la partie sans rien voir changer", () => {
  // C'est l'écran de jeu, et c'est un joueur distant : ils n'ont jamais entendu
  // parler des occasions numérotées, et le lot ne doit rien leur imposer.
  const p = creerPartie(pont, { graine: 8, boites: BOITES });
  const h = [fournisseurAleatoire(21, "a"), fournisseurAleatoire(22, "b")];
  for (const f of h) if (typeof f.vendre === "function") throw new Error("le fournisseur témoin sait vendre");
  let garde = 0;
  while (!p.termine) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const d = p.decision;
    if (!d) break;
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  if (!p.termine) throw new Error("la partie ne s'est pas terminée");
  return `partie complète en ${p.decisions.length} décisions, ${p.occasionsOuvertes} occasions déclinées`;
});

// ════════════════════════════════════════════════════════════════════════════
// IV. LE FICHIER DE CARTES — data/cards.json contre web/webapp/assets/cards.json
// ════════════════════════════════════════════════════════════════════════════
//
// Le territoire couvert ici est celui du fichier de cartes.json : le moteur lit
// `data/cards.json`, la page lit `web/webapp/assets/cards.json`, et rien dans
// la chaîne de construction ne garantissait jusqu'ici qu'ils soient le même
// fichier. Deux copies qui dérivent, et le natif et le navigateur ne jouent
// plus avec le même paquet — sans qu'aucune partie ne plante pour le dire.

test("le moteur et le navigateur lisent le même cards.json, à l'octet", () => {
  const a = readFileSync(join(DEPOT, "data/cards.json"));
  const b = readFileSync(join(DEPOT, "web/webapp/assets/cards.json"));
  if (md5(a) !== md5(b)) throw new Error(`md5 ${md5(a)} ≠ ${md5(b)} (${a.length} contre ${b.length} octets)`);
  return `${a.length} octets, md5 ${md5(a)}`;
});

test("le cards.json de la livraison est un JSON valide et fourni", () => {
  const j = JSON.parse(lire("web/webapp/assets/cards.json"));
  const n = Array.isArray(j) ? j.length : Object.keys(j).length;
  if (n < 50) throw new Error(`seulement ${n} entrées dans cards.json`);
  return `${n} entrées`;
});

test("le wasm sert bien le cards.json de la livraison", () => {
  const lignes = pont.lignes({ op: "dump_deck", boites: BOITES });
  if (lignes.length < 100) throw new Error(`le recensement du pont ne rend que ${lignes.length} cartes`);
  const noms = new Set(lignes.map((l) => JSON.parse(l).name));
  const texte = lire("web/webapp/assets/cards.json");
  let absents = 0;
  for (const n of noms) if (!texte.includes(JSON.stringify(n).slice(1, -1))) absents++;
  if (absents > 0) throw new Error(`${absents} carte(s) du pont sont absentes du cards.json servi`);
  return `${noms.size} cartes recensées, toutes présentes dans le fichier servi`;
});

// ════════════════════════════════════════════════════════════════════════════
// V. LE BINAIRE LIVRÉ — web/webapp/terra.wasm
// ════════════════════════════════════════════════════════════════════════════

const OCTETS_WASM = readFileSync(join(LIVRAISON, "terra.wasm"));

test("terra.wasm existe et porte la signature WebAssembly", () => {
  const magie = [...OCTETS_WASM.subarray(0, 4)];
  if (jsonDe(magie) !== jsonDe([0x00, 0x61, 0x73, 0x6d])) {
    throw new Error(`en-tête ${magie.map((x) => x.toString(16)).join(" ")} : ce n'est pas un module WebAssembly`);
  }
  return `${OCTETS_WASM.length} octets, md5 ${md5(OCTETS_WASM)}`;
});

test("terra.wasm expose l'interface C que le pont appelle", () => {
  const texte = OCTETS_WASM.toString("latin1");
  const manquants = ["terra_call", "terra_alloc", "terra_free", "terra_result_ptr"].filter(
    (n) => !texte.includes(n),
  );
  if (manquants.length > 0) throw new Error(`exports absents : ${manquants.join(", ")}`);
  return null;
});

test("terra.wasm répond à une interrogation simple", () => {
  const l = pont.lignes({ op: "dump_corporations", boites: BOITES });
  if (!Array.isArray(l) || l.length === 0) throw new Error("dump_corporations ne rend rien");
  return `${l.length} corporations`;
});

test("terra.wasm CONNAÎT la graine d'essais : ce n'est pas un binaire d'avant le lot", () => {
  const a = jsonDe(pont.pas(4242, BOITES, [0, 0], { graine: 1, rang: 1 }));
  const b = jsonDe(pont.pas(4242, BOITES, [0, 0], { graine: 2, rang: 1 }));
  if (a === b) {
    throw new Error("le binaire livré ignore `graine_essais` — relance web/construire.sh");
  }
  return null;
});

test("terra.wasm refuse une op inconnue au lieu de la deviner", () => {
  const r = pont.appeler({ op: "op-qui-n-existe-pas" });
  if (r.ok !== false) throw new Error("une op inconnue a été acceptée");
  return String(r.erreur).slice(0, 60);
});

test("terra.wasm refuse une boîte inconnue", () => {
  const r = pont.appeler({ op: "dump_deck", boites: "boite-imaginaire" });
  if (r.ok !== false) throw new Error("une boîte inconnue a été acceptée");
  return String(r.erreur).slice(0, 60);
});

// ════════════════════════════════════════════════════════════════════════════
// VI. CE QUE LE LOT NE DOIT PAS AVOIR CASSÉ
// ════════════════════════════════════════════════════════════════════════════

test("une partie entière se joue toujours de bout en bout", async () => {
  const p = creerPartie(pont, { graine: 5150001, boites: BOITES });
  await jouerJusquAuBout(p, [fournisseurAleatoire(1), fournisseurAleatoire(2)]);
  if (!p.termine) throw new Error("la partie ne s'est pas terminée");
  if (!p.partieComplete) throw new Error("la partie s'est arrêtée sur le plafond de manches");
  return `${p.decisions.length} décisions, ${p.manches} manches, scores ${jsonDe(p.scores)}`;
});

test("la même graine et les mêmes décisions rendent la même partie", () => {
  const p = partieTemoin(777, 40);
  const q = creerPartie(pont, { graine: 777, boites: BOITES });
  for (const d of p.decisions) {
    if (d && d.vendre) q.vendre(d.vendre);
    else q.repondre(d);
  }
  if (jsonDe(q.etat) !== jsonDe(p.etat)) throw new Error("le rejeu de la même liste ne rend pas le même état");
  return `${p.decisions.length} décisions rejouées`;
});

test("le rang annoncé par le moteur reste le nombre de décisions déjà prises", () => {
  const p = creerPartie(pont, { graine: 99, boites: BOITES });
  const h = [fournisseurAleatoire(5, "a"), fournisseurAleatoire(6, "b")];
  let garde = 0;
  while (!p.termine && p.decisions.length < 40) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const d = p.decision;
    if (!d) break;
    if (d.rang !== p.decisions.length) throw new Error(`rang ${d.rang} pour ${p.decisions.length} décisions prises`);
    p.repondre(h[d.joueur].decider(d, p.etat));
  }
  return `${p.decisions.length} rangs vérifiés`;
});

test("l'état rendu reste celui du moteur : les deux mains sont visibles", () => {
  const e = TEMOIN.etat;
  if (!Array.isArray(e.players) || e.players.length !== 2) throw new Error("l'état n'a pas deux joueurs");
  if (!Array.isArray(e.players[0].hand) || !Array.isArray(e.players[1].hand)) {
    throw new Error("les mains ne sont pas publiées");
  }
  return null;
});

test("la fiche de situation n'a pas bougé : 1 630 cases des deux côtés", async () => {
  // Le lot NE DOIT PAS toucher la description. Si ce compte change, les poids
  // appris ne veulent plus rien dire et le juge des fiches tombe.
  const { decrire } = await import("../joueurs/description.js");
  const f = decrire(TEMOIN.etat, 0);
  if (!Array.isArray(f) || f.length !== 1630) throw new Error(`fiche de ${Array.isArray(f) ? f.length : "?"} cases`);
  return "1630 cases";
});

test("les modules de la livraison s'importent tous sans erreur", async () => {
  for (const m of ["../pont.js", "../partie.js", "../fournisseurs.js", "../joueurs/apprenti.js"]) {
    await import(m);
  }
  return "4 modules";
});

// ════════════════════════════════════════════════════════════════════════════
// Le verdict
// ════════════════════════════════════════════════════════════════════════════

const fautes = [];
let faits = 0;
const debut = Date.now();
for (const { nom, fn } of carnet) {
  faits++;
  try {
    const detail = await fn();
    console.log(`  ✓ ${nom}${detail ? " — " + detail : ""}`);
  } catch (e) {
    fautes.push(`${nom} : ${e.message}`);
    console.log(`  ✗ ${nom} : ${e.message}`);
  }
}
const secondes = ((Date.now() - debut) / 1000).toFixed(1);
console.log(`${faits} verifications, ${fautes.length} faute(s), ${secondes} s`);
if (fautes.length > 0) {
  console.log(`ROUGE ${fautes.length} faute(s) sur ${faits} verifications du lot « le pont ne triche plus »`);
  process.exit(1);
}
console.log(`VERT ${faits} verifications du lot « le pont ne triche plus », toutes vertes`);

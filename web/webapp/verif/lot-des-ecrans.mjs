#!/usr/bin/env node
// **LE BANC DU LOT « LES-ECRANS-MANQUANTS ».**
//
//   node web/webapp/verif/lot-des-ecrans.mjs
//
// Les controles du contrat prouvent que les defauts sont partis ; ils ne restent
// pas dans le depot. Ce banc-ci y reste : c'est lui qui empechera, dans six mois,
// qu'un changement d'ecran remette en place ce qu'on vient d'oter.
//
// CE QU'IL EPROUVE, ET COMMENT. Neuf defauts, un par critere A a I. Chaque fois
// que la PROPRIETE est mesurable — dans le moteur, dans un module pur — c'est la
// propriete qui est mesuree, jamais la forme du code : le contrat de ce depot
// dit qu'un hold-out sur la forme punit une meilleure solution que la sienne.
// Les quelques verifications de texte qui restent portent sur des choses qu'on
// ne peut pas mesurer autrement (un mot qui doit etre UNIQUE dans tout le
// dossier, un chemin `data-valeur` qui doit exister dans le document), et elles
// le disent.
//
// CE QU'IL N'EPROUVE PAS. La mise en page. Ce banc ne pilote aucun navigateur :
// les neuf controles scelles le font, et ils mesurent la visibilite reelle des
// elements. Ici on garde ce qui survit sans ecran — les nombres du moteur, les
// modules purs, l'unicite des marques.
//
// Les neuf defauts :
//
//   A — le moteur ne disait pas qui gagne, ni le total de departage du livret.
//   B — l'ecran final ne nommait pas le vainqueur.
//   C — le mode en ligne transmettait la mise en place au fil de l'eau : le
//       second a repondre voyait le choix du premier.
//   D — la zone de l'adversaire ne connaissait que trois questions sur cinq.
//   E — `first_player` etait publie et lu nulle part.
//   F — la boite de base annoncait des points d'Objectif et de Recompense qui
//       ne valent rien.
//   G — le classement des Recompenses n'etait lu que par l'IA.
//   H — la vente faite a l'ecran ne portait pas son numero d'occasion.
//   I — la vente de l'adversaire ne se voyait nulle part.

import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { readFileSync, readdirSync, statSync } from "node:fs";

// UN FAUX DOCUMENT, POUR UN SEUL FICHIER. `distant.js` s'adresse au navigateur
// des sa premiere ligne ; on ne peut pas l'importer sans lui en donner un. Ce
// faux-la ne sert qu'a l'importer : aucune verification ci-dessous n'appelle une
// fonction qui le touche.
globalThis.location = new URL("http://127.0.0.1/index.html");
globalThis.window = { addEventListener() {} };
globalThis.requestAnimationFrame = () => {};
const fauxElement = () => ({
  dataset: {}, style: { setProperty() {} }, children: [], innerHTML: "",
  classList: { add() {}, remove() {}, toggle() {} },
  appendChild() {}, removeChild() {}, remove() {},
  setAttribute() {}, addEventListener() {},
});
globalThis.document = {
  documentElement: { dataset: {} },
  body: fauxElement(),
  getElementById: () => null,
  querySelector: () => null,
  querySelectorAll: () => [],
  addEventListener: () => {},
  createElement: fauxElement,
};

const { ouvrirPontDepuis } = await import("../pont.js");
const { creerPartie, jouerJusquAuBout } = await import("../partie.js");
const { fournisseurAleatoire } = await import("../fournisseurs.js");
const {
  mesurerQuestionsSimultanees, reglerQuestionsSimultanees, estSimultanee,
  oublierQuestionsSimultanees,
} = await import("../questions-simultanees.js");
const { MOT, actionAdverse } = await import("../vue/mots.js");
const { reglerBoites, honneursComptent } = await import("../vue/boites.js");
const { reponsesPossibles } = await import("../distant.js");
const { ecranFinal } = await import("../vue/annonce.js");

const LIVRAISON = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const DEPOT = resolve(LIVRAISON, "..", "..");
const TOUT = "base,decouverte";
const BASE = "base";
// Le livret p. 16 : une carte en main vaut 3 MC dans le total de departage.
const CARTE_EN_MC = 3;

const pont = await ouvrirPontDepuis(LIVRAISON);

// ── le carnet ───────────────────────────────────────────────────────────────
const carnet = [];
function test(nom, critere, fn) {
  carnet.push({ nom, critere, fn });
}
function exige(condition, dire) {
  if (!condition) throw new Error(dire);
}
const source = (chemin) => readFileSync(join(LIVRAISON, chemin), "utf8");

/**
 * Le corps d'une fonction, isole de son fichier.
 *
 * Certaines proprietes ne se mesurent qu'en lisant le code — « le nom d'une
 * question ne doit decider de rien ». Les lire sur le FICHIER ENTIER rendrait un
 * faux rouge des qu'un nom parait ailleurs pour une raison legitime : la table
 * des phrases d'attente de `distant.js` nomme les dix-huit types du moteur, et
 * c'est un vocabulaire, pas une decision. On lit donc la fonction qui decide, et
 * elle seule.
 */
function corps(texte, nom) {
  const i = texte.indexOf(`function ${nom}(`);
  if (i < 0) throw new Error(`la fonction ${nom} n'existe plus`);
  // ⚠️ CORRIGE LE 28-08 (les-sept-bancs-rouges). La delimitation cherchait la
  // premiere accolade fermante EN DEBUT DE LIGNE. Elle suffisait aux deux
  // fonctions lues ici, et se trompait des qu'une accolade non indentee traine
  // dans le corps — un objet aligne a la marge, une chaine, un commentaire :
  // elle rendait alors un corps TRONQUE, et un motif cherche dedans manquait
  // sans que rien ne le dise. On compte donc les accolades, en sautant ce qui
  // n'est pas du code.
  const debut = texte.indexOf("{", i);
  if (debut < 0) throw new Error(`la fonction ${nom} n'a pas de corps lisible`);
  let profondeur = 0;
  for (let k = debut; k < texte.length; k++) {
    const c = texte[k], d = texte[k + 1];
    if (c === "/" && d === "/") {
      const fin = texte.indexOf("\n", k);
      if (fin < 0) break;
      k = fin;
      continue;
    }
    if (c === "/" && d === "*") {
      const fin = texte.indexOf("*/", k + 2);
      if (fin < 0) break;
      k = fin + 1;
      continue;
    }
    if (c === '"' || c === "'" || c === "`") {
      for (k++; k < texte.length; k++) {
        if (texte[k] === "\\") { k++; continue; }
        if (texte[k] === c) break;
      }
      continue;
    }
    if (c === "{") profondeur++;
    else if (c === "}" && --profondeur === 0) return texte.slice(i, k + 1);
  }
  throw new Error(`la fonction ${nom} n'a pas de fin lisible`);
}

/** Tous les fichiers de la livraison, hors binaire, hors bancs, hors sources Rust. */
function fichiersDeLaPage() {
  const out = [];
  const marcher = (d) => {
    for (const n of readdirSync(d)) {
      if (n === "node_modules" || n === "wasm" || n === "verif" || n === "assets") continue;
      const c = join(d, n);
      if (statSync(c).isDirectory()) marcher(c);
      else if (/\.(js|mjs|css|html)$/.test(n)) out.push(c);
    }
  };
  marcher(LIVRAISON);
  return out;
}

// ── les parties temoins ─────────────────────────────────────────────────────
//
// On joue quelques parties ENTIERES, une fois pour toutes, et l'on retient de
// chaque instant le strict necessaire. Rejouer par verification couterait des
// minutes pour rien.

function extrait(etat) {
  return {
    // LA PRESENCE, RELEVEE SUR L'ETAT BRUT. Relire la clef sur la copie ci-
    // dessous ne prouverait rien : cette copie la pose toujours, meme absente de
    // l'etat. Un sabotage qui effacait `winner` du pont a laisse passer la
    // premiere version de cette verification — c'est ce qu'elle mesure ici.
    vainqueurPublie: "winner" in etat,
    departagePublie: etat.players.map((p) => "tiebreak_total" in p),
    over: etat.game_over === true,
    gen: etat.generation,
    premier: etat.first_player,
    vainqueur: etat.winner,
    recompenses: etat.awards.slice(),
    jalons: etat.milestones.map((m) => m.kind),
    prises: etat.milestones.map((m) => m.achieved_by.slice()),
    joueurs: etat.players.map((p) => ({
      score: p.score,
      departage: p.tiebreak_total,
      heat: p.heat, mc: p.mc, plants: p.plants,
      main: (p.hand || []).length,
      parts: { ...p.score_parts },
      valeurs: { ...p.valeurs_recompenses },
    })),
  };
}

const parcours = new Map();
async function partieTemoin(graine, boites) {
  const cle = `${graine}/${boites}`;
  if (parcours.has(cle)) return parcours.get(cle);
  const p = creerPartie(pont, { graine, boites });
  const instants = [];
  await jouerJusquAuBout(
    p,
    [fournisseurAleatoire(graine * 3 + 1, "a"), fournisseurAleatoire(graine * 5 + 2, "b")],
    (partie) => instants.push(extrait(partie.etat)),
  );
  const r = {
    instants,
    fin: extrait(p.etat),
    // L'etat brut de la derniere image, recopie : l'ecran final le prend tel
    // quel, et certaines verifications en modifient une clef pour voir ou tombe
    // la marque du vainqueur.
    etatFinal: JSON.parse(JSON.stringify(p.etat)),
    decisions: p.decisions.length,
  };
  parcours.set(cle, r);
  return r;
}

const GRAINES = [7301, 7302, 7303];

// ═══════════════════════════════════════════════ A — LE MOTEUR DIT QUI GAGNE

test("A1 — la clef « winner » est publiee a la racine de la vue d'etat, toujours", "A", async () => {
  for (const g of GRAINES) {
    const { instants, fin } = await partieTemoin(g, TOUT);
    for (const i of [...instants, fin]) {
      exige(i.vainqueurPublie, `graine ${g} : la clef « winner » manque a l'etat`);
    }
  }
});

test("A2 — « tiebreak_total » est publie pour CHAQUE joueur, toujours", "A", async () => {
  for (const g of GRAINES) {
    const { instants, fin } = await partieTemoin(g, TOUT);
    for (const i of [...instants, fin]) {
      for (const j of [0, 1]) {
        exige(i.departagePublie[j],
          `graine ${g}, joueur ${j} : la clef « tiebreak_total » manque a l'etat`);
      }
    }
  }
});

test("A3 — EN COURS DE PARTIE, le total de departage reste tu : il compte les cartes en main", "A", async () => {
  for (const g of GRAINES) {
    const { instants } = await partieTemoin(g, TOUT);
    for (const i of instants) {
      if (i.over) continue;
      for (const j of [0, 1]) {
        exige(i.joueurs[j].departage === null,
          `graine ${g}, joueur ${j} : le total vaut ${i.joueurs[j].departage} avant la fin — ` +
          `il compte les cartes en main, donc il donnerait la taille de la main d'en face`);
      }
    }
  }
});

test("A4 — en cours de partie, le vainqueur reste tu lui aussi", "A", async () => {
  for (const g of GRAINES) {
    const { instants } = await partieTemoin(g, TOUT);
    for (const i of instants) {
      if (!i.over) exige(i.vainqueur === null, `graine ${g} : vainqueur annonce avant la fin`);
    }
  }
});

test("A5 — a la fin, le total de departage est celui du livret : chaleur + MC + plantes + 3 MC par carte en main", "A", async () => {
  for (const g of GRAINES) {
    const { fin } = await partieTemoin(g, TOUT);
    for (const j of [0, 1]) {
      const p = fin.joueurs[j];
      const attendu = p.heat + p.mc + p.plants + p.main * CARTE_EN_MC;
      exige(p.departage === attendu,
        `graine ${g}, joueur ${j} : ${p.departage} publie, ${attendu} selon le livret ` +
        `(chaleur ${p.heat} + MC ${p.mc} + plantes ${p.plants} + ${p.main} carte(s))`);
    }
  }
});

test("A6 — le vainqueur publie est celui que les points, PUIS le total, designent", "A", async () => {
  for (const g of GRAINES) {
    const { fin } = await partieTemoin(g, TOUT);
    const [a, b] = fin.joueurs;
    let attendu = null;
    if (a.score !== b.score) attendu = a.score > b.score ? 0 : 1;
    else if (a.departage !== b.departage) attendu = a.departage > b.departage ? 0 : 1;
    exige(fin.vainqueur === attendu,
      `graine ${g} : « ${fin.vainqueur} » publie, « ${attendu} » attendu ` +
      `(scores ${a.score}/${b.score}, departages ${a.departage}/${b.departage})`);
  }
});

test("A7 — le vainqueur n'est jamais un siege qui n'existe pas", "A", async () => {
  for (const g of GRAINES) {
    const { fin } = await partieTemoin(g, TOUT);
    exige(fin.vainqueur === null || fin.vainqueur === 0 || fin.vainqueur === 1,
      `graine ${g} : « ${fin.vainqueur} »`);
  }
});

// ═══════════════════════════════════════ B — L'ECRAN FINAL NOMME LE VAINQUEUR
//
// Ces verifications-ci ne lisent pas la source : elles FABRIQUENT l'ecran final
// et lisent ce qu'il ecrit. Un etat de fin de partie reel sert de moule ; on n'en
// change que la clef `winner`, et l'on regarde ou tombe la marque. C'est la
// propriete qui compte — l'ecran suit le moteur — et non la facon dont la marque
// est posee.

/** Le fragment de la colonne du siege j, depuis son ouverture jusqu'a son score. */
function colonne(html, j) {
  const i = html.indexOf(`data-score-final="${j}"`);
  exige(i >= 0, `la colonne du siege ${j} n'existe pas sur l'ecran final`);
  return html.slice(html.lastIndexOf("<div", i), i);
}

/** L'ecran final rendu, en texte, pour l'etat donne. */
function ecranPour(etat) {
  return ecranFinal(etat).innerHTML || "";
}

/** Un etat de fin de partie reel, recopie pour etre modifiable. */
let MOULE = null;
async function moule() {
  if (!MOULE) MOULE = (await partieTemoin(GRAINES[0], TOUT)).etatFinal;
  return JSON.parse(JSON.stringify(MOULE));
}

test("B1 — l'ecran final montre les DEUX totaux de departage, sous le chemin qui les designe", "B", async () => {
  const e = await moule();
  const h = ecranPour(e);
  for (const j of [0, 1]) {
    const m = h.match(new RegExp(`data-valeur="players\\.${j}\\.tiebreak_total">([^<]*)<`));
    exige(m, `le total du siege ${j} n'est pas publie sous son chemin`);
    exige(Number(m[1]) === e.players[j].tiebreak_total,
      `siege ${j} : « ${m[1]} » a l'ecran, ${e.players[j].tiebreak_total} dans le moteur`);
  }
});

test("B2 — la marque de vainqueur tombe sur le siege que le moteur designe, et sur lui seul", "B", async () => {
  for (const gagnant of [0, 1]) {
    const e = await moule();
    e.winner = gagnant;
    const h = ecranPour(e);
    for (const j of [0, 1]) {
      const marque = /data-vainqueur/.test(colonne(h, j));
      exige(marque === (j === gagnant),
        `winner=${gagnant} : le siege ${j} est ${marque ? "" : "non "}marque`);
    }
  }
});

test("B3 — LA PROPRIETE : quand les points contredisent le moteur, l'ecran suit LE MOTEUR", "B", async () => {
  const e = await moule();
  e.winner = 1;
  e.players[0].score = 99;
  e.players[1].score = 1;
  const h = ecranPour(e);
  exige(!/data-vainqueur/.test(colonne(h, 0)) && /data-vainqueur/.test(colonne(h, 1)),
    "l'ecran a couronne le plus gros score : il rejoue la regle au lieu de lire `winner`. " +
    "Le departage du livret peut donner la partie a un score plus bas, et c'est le moteur " +
    "qui le sait, pas la page");
});

test("B4 — une partie nulle ne couronne personne, et le dit", "B", async () => {
  const e = await moule();
  e.winner = null;
  e.players[0].score = e.players[1].score;
  const h = ecranPour(e);
  for (const j of [0, 1]) {
    exige(!/data-vainqueur/.test(colonne(h, j)), `le siege ${j} est couronne sur une partie nulle`);
  }
  exige(h.includes(MOT.drawn), "la partie nulle n'est annoncee par aucune phrase");
});

test("B5 — a points egaux avec un vainqueur, l'ecran dit ce qui a tranche", "B", async () => {
  const e = await moule();
  e.winner = 0;
  e.players[0].score = e.players[1].score;
  exige(ecranPour(e).includes(MOT.tiebreakWhy),
    "les points sont a egalite et rien ne dit que le total de departage a tranche");
});

test("B6 — les quatre mots de l'ecran final existent et ne sont pas vides", "B", () => {
  for (const cle of ["winnerMark", "tiebreak", "tiebreakWhy", "drawn"]) {
    exige(typeof MOT[cle] === "string" && MOT[cle].length > 0, `MOT.${cle} manque`);
  }
});

test("B7 — la phrase du departage nomme les trois reserves et le prix d'une carte", "B", () => {
  const t = MOT.tiebreakWhy.toLowerCase();
  for (const m of ["heat", "plants", "card"]) {
    exige(t.includes(m), `la phrase ne parle pas de « ${m} » : ${MOT.tiebreakWhy}`);
  }
});

// ═════════════════ C — LE RESEAU NE FUIT PAS SUR LES QUESTIONS SIMULTANEES

let MESUREES = null;
async function mesurees() {
  if (!MESUREES) MESUREES = await mesurerQuestionsSimultanees(pont, TOUT);
  return MESUREES;
}

test("C1 — la mesure trouve les cinq questions que le moteur pose aux deux joueurs", "C", async () => {
  const s = await mesurees();
  const attendus = ["corp_mulligan", "pick_corporation", "project_mulligan",
    "pick_phase", "research_keep"];
  for (const t of attendus) exige(s.has(t), `« ${t} » manque a l'ensemble mesure : ${[...s]}`);
});

test("C2 — la mesure ECARTE les questions que la table donne a voir", "C", async () => {
  const s = await mesurees();
  for (const t of ["action_choice", "choose_build", "discard_down"]) {
    exige(!s.has(t),
      `« ${t} » est compte simultane : le grouper cacherait au second joueur ce que ` +
      `la table lui montre, et le ferait decider sur un etat perime`);
  }
});

test("C3 — la mesure ne depend pas de la boite : elle se refait pour chacune", "C", async () => {
  oublierQuestionsSimultanees();
  const a = await mesurerQuestionsSimultanees(pont, BASE);
  exige(a.size >= 4, `boite de base : seulement ${a.size} question(s) mesurees`);
});

test("C4 — les reponses possibles d'un choix simple sont exactement les indices des options", "C", () => {
  const r = reponsesPossibles({ type: "pick_phase", options: ["a", "b", "c"] });
  exige(JSON.stringify(r) === "[0,1,2]", `obtenu ${JSON.stringify(r)}`);
});

test("C5 — « passer » compte pour une reponse de plus", "C", () => {
  const r = reponsesPossibles({ type: "x", options: ["a", "b"], passer: true });
  exige(r.length === 3, `obtenu ${JSON.stringify(r)} : l'option « passer » n'est pas comptee`);
});

test("C6 — un choix multiple a nombre LIBRE enumere tous ses sous-ensembles", "C", () => {
  const r = reponsesPossibles({ type: "project_mulligan", multiple: true, options: [1, 2, 3] });
  exige(r.length === 8, `${r.length} reponses au lieu de 8 pour trois cartes libres`);
  exige(r.some((x) => x.length === 0) && r.some((x) => x.length === 3),
    "le tout-garder ou le tout-rendre manque");
});

test("C7 — un choix multiple a nombre IMPOSE n'enumere que les tailles permises", "C", () => {
  const r = reponsesPossibles({ type: "research_keep", multiple: true, a_choisir: 2, options: [1, 2, 3, 4] });
  exige(r.length === 6, `${r.length} reponses au lieu de 6 (4 parmi 2)`);
  exige(r.every((x) => x.length === 2), "une reponse ne respecte pas le nombre impose");
});

test("C8 — le plafond d'enumeration tient : juste dessous on enumere, juste au-dessus on refuse", "C", () => {
  const libre = (n) => reponsesPossibles({
    type: "x", multiple: true, options: Array.from({ length: n }, (_, i) => i),
  });
  // 2^8 = 256, sous le plafond : c'est le cas reel de `project_mulligan`, et il
  // DOIT passer, sans quoi la mise en place ne serait jamais groupee.
  const dessous = libre(8);
  exige(Array.isArray(dessous) && dessous.length === 256,
    `huit cartes libres devraient donner 256 reponses, obtenu ${dessous && dessous.length}`);
  // 2^12 = 4096, au-dessus : refuse. Le doute se paie par une attente — la page
  // ne declare alors aucun groupe et pose la question comme les autres.
  exige(libre(12) === null, "4096 sous-ensembles ont ete enumeres au lieu d'etre refuses");
  exige(libre(40) === null, "un millier de milliards de sous-ensembles a ete tente");
});

test("C9 — un montant enumere ses bornes, et refuse une plage sans bornes", "C", () => {
  const r = reponsesPossibles({ type: "x", montant: true, minimum: 2, maximum: 5 });
  exige(JSON.stringify(r) === "[2,3,4,5]", `obtenu ${JSON.stringify(r)}`);
  exige(reponsesPossibles({ type: "x", montant: true }) === null, "une plage sans bornes a ete acceptee");
});

test("C10 — LA PROPRIETE : la ou un groupe face cachee est declare, la question suivante ne depend PAS de la reponse", "C", async () => {
  const s = await mesurees();
  const p = creerPartie(pont, { graine: 7411, boites: TOUT });
  const f = [fournisseurAleatoire(101, "a"), fournisseurAleatoire(202, "b")];
  const decisions = [];
  let eprouvees = 0;
  const types = new Set();
  let garde = 0;
  while (!p.termine && garde++ < 500) {
    const d = p.decision;
    if (!d) break;
    if (s.has(d.type)) {
      // TROIS CONDITIONS AVANT DE JUGER, exactement celles que la page applique
      // avant de declarer un groupe : meme type, rang immediatement suivant,
      // AUTRE siege. Elles CLASSENT — la question forme-t-elle une paire ? —,
      // elles n'accusent pas : la seconde question d'une paire est suivie d'une
      // question toute neuve, et c'est normal, aucun groupe n'est declare la.
      // C'est seulement quand la paire existe que l'invariance devient une
      // obligation : ce que la page montre d'avance ne doit rien apprendre.
      const reponses = reponsesPossibles(d);
      exige(reponses !== null,
        `rang ${d.rang} « ${d.type} » : les reponses ne sont pas enumerables, ` +
        "la page ne pourrait pas prouver l'invariance");
      let empreinte = null;
      let paire = true;
      for (const r of reponses) {
        const pas = pont.pas(7411, TOUT, [...decisions, r]);
        const dd = pas && pas.decision;
        if (!dd) { paire = false; break; }
        if (empreinte === null) {
          paire = dd.type === d.type && dd.rang === d.rang + 1 && dd.joueur !== d.joueur;
          if (!paire) break;
          empreinte = JSON.stringify(dd);
        } else {
          exige(JSON.stringify(dd) === empreinte,
            `rang ${d.rang} « ${d.type} » : la question suivante DEPEND de la reponse ` +
            "donnee — l'afficher d'avance, face cachee, en dirait quelque chose");
        }
      }
      if (paire) { eprouvees++; types.add(d.type); }
    }
    const rep = await f[d.joueur].decider(d, p.etat);
    decisions.push(rep);
    p.repondre(rep);
  }
  // ZERO FAUTE SUR ZERO OCCASION NE PROUVE RIEN. Mesure du 22-08 sur cette
  // graine et ces deux fournisseurs, tous deux fixes : 63 paires en 363
  // decisions — {corp_mulligan 1, project_mulligan 1, pick_corporation 1,
  // pick_phase 46, research_keep 14}. Les planchers sont poses sous la mesure,
  // assez haut pour qu'un groupement retreci a la seule mise en place (3 paires)
  // ou a un unique type tombe ici.
  exige(eprouvees >= 40, `seulement ${eprouvees} paire(s) eprouvee(s) sur les 63 mesurees`);
  exige(types.size >= 4, `${types.size} genre(s) de question eprouve(s) : ${[...types]}`);
});

test("C11 — l'ordre des indices d'une reponse multiple ne change pas la question suivante", "C", () => {
  const p = creerPartie(pont, { graine: 7413, boites: TOUT });
  const prefixe = [];
  let garde = 0;
  while (p.decision && !p.decision.multiple && garde++ < 24) {
    prefixe.push(0);
    p.repondre(0);
  }
  const d = p.decision;
  exige(d && d.multiple && d.options.length >= 2,
    "aucune question a choix multiple dans les vingt-quatre premiers rangs : " +
    "la mesure ne porterait sur rien");
  const a = pont.pas(7413, TOUT, [...prefixe, [0, 1]]);
  const b = pont.pas(7413, TOUT, [...prefixe, [1, 0]]);
  exige(JSON.stringify(a.decision) === JSON.stringify(b.decision),
    `rang ${d.rang} « ${d.type} » : l'ordre des indices change la suite — enumerer ` +
    "les sous-ensembles dans un seul ordre ne prouverait alors rien");
});

test("C12 — aucun nom de question ne DECIDE du groupement dans le mode en ligne", "C", () => {
  const s = source("distant.js");
  exige(!/CHOIX_SIMULTANE/.test(s),
    "`CHOIX_SIMULTANE` est revenu : la liste des questions groupees est ecrite a la main");
  const decide = corps(s, "questionSuivanteInvariante");
  exige(/estSimultanee\(/.test(decide),
    "la fonction qui declare les groupes ne consulte pas l'ensemble mesure");
  for (const t of ["pick_phase", "corp_mulligan", "research_keep", "pick_corporation",
    "project_mulligan", "action_choice"]) {
    exige(!new RegExp(`["'\`]${t}["'\`]`).test(decide),
      `« ${t} » est ecrit en dur dans la fonction qui decide du groupement`);
  }
});

// ══════════════════ D — LA ZONE DE L'ADVERSAIRE DIT CE QU'IL EST EN TRAIN DE FAIRE

test("D1 — chacune des cinq questions simultanees a SON mot, et ils sont tous differents", "D", async () => {
  reglerQuestionsSimultanees(await mesurees());
  const mots = new Map();
  for (const t of await mesurees()) {
    const m = actionAdverse({ type: t });
    exige(typeof m === "string" && m.length > 0, `« ${t} » n'a pas de mot`);
    exige(!mots.has(m), `« ${t} » et « ${mots.get(m)} » partagent le mot « ${m} »`);
    mots.set(m, t);
  }
  exige(mots.size >= 5, `${mots.size} mot(s) distincts seulement`);
});

test("D2 — une question qui n'est PAS simultanee n'allume pas la zone d'en face", "D", async () => {
  reglerQuestionsSimultanees(await mesurees());
  const generique = actionAdverse({ type: "action_choice" });
  for (const t of ["action_choice", "choose_build", "discard_down", "pick_ocean"]) {
    exige(actionAdverse({ type: t }) === generique,
      `« ${t} » recoit un mot particulier alors qu'elle n'est pas posee aux deux`);
  }
});

test("D3 — LA PROPRIETE : sans mesure, la page n'annonce AUCUNE simultaneite", "D", async () => {
  reglerQuestionsSimultanees(new Set());
  const general = actionAdverse({ type: "un_type_qui_n_existe_pas" });
  for (const t of ["pick_phase", "corp_mulligan", "research_keep", "pick_corporation",
    "project_mulligan"]) {
    exige(actionAdverse({ type: t }) === general,
      `« ${t} » est annonce simultane alors que rien ne l'a mesure : la table des mots ` +
      "decide a la place de la mesure, et dira faux le jour ou le moteur changera");
  }
  reglerQuestionsSimultanees(await mesurees());
});

test("D4 — un type inconnu ne fait pas taire l'ecran ni ne le fait mentir", "D", async () => {
  reglerQuestionsSimultanees(new Set(["question_de_demain"]));
  const m = actionAdverse({ type: "question_de_demain" });
  exige(m.includes("question de demain"),
    `un type nouveau devrait etre nomme par defaut, obtenu « ${m} »`);
  exige(actionAdverse(null) && actionAdverse({}), "une decision absente casse le mot");
  reglerQuestionsSimultanees(await mesurees());
});

// ═════════════════════════════════ E — QUI COMMENCE LA MANCHE EST AFFICHE

test("E1 — « first_player » designe toujours un des deux sieges", "E", async () => {
  for (const g of GRAINES) {
    const { instants } = await partieTemoin(g, TOUT);
    for (const i of instants) {
      exige(i.premier === 0 || i.premier === 1, `graine ${g} : « ${i.premier} »`);
    }
  }
});

test("E2 — il ne change JAMAIS au milieu d'une manche", "E", async () => {
  for (const g of GRAINES) {
    const { instants } = await partieTemoin(g, TOUT);
    const par = new Map();
    for (const i of instants) {
      if (!par.has(i.gen)) par.set(i.gen, new Set());
      par.get(i.gen).add(i.premier);
    }
    for (const [gen, v] of par) {
      exige(v.size === 1, `graine ${g}, manche ${gen} : il vaut ${[...v]} au cours de la meme manche`);
    }
  }
});

test("E3 — il change au moins une fois dans la partie", "E", async () => {
  for (const g of GRAINES) {
    const { instants } = await partieTemoin(g, TOUT);
    exige(new Set(instants.map((i) => i.premier)).size >= 2,
      `graine ${g} : toujours le siege ${instants[0].premier} — un affichage fige passerait`);
  }
});

test("E4 — l'element qui porte « qui commence » declare bien ce chemin-la", "E", () => {
  const s = source("vue/monde.js");
  const i = s.indexOf('class="manche__premier"');
  exige(i >= 0, "l'element de « qui commence » a disparu du bandeau de manche");
  // La balise, et elle seule : le nom du chemin parait aussi dans les
  // commentaires et dans la recherche de l'element. Un chemin faux sur la balise
  // ferait afficher un nombre sous le nom d'un autre — c'est ce que `prix-barre`
  // et `ce-que-le-moteur-ne-dit-pas` traquent dans tout le reste de la page.
  const balise = s.slice(s.lastIndexOf("<", i), s.indexOf(">", i));
  exige(balise.includes('data-valeur="first_player"'),
    `la balise ne declare pas le bon chemin : ${balise.replace(/\s+/g, " ")}`);
  exige(/dataset\.premier/.test(s), "le siege brut du moteur n'est recopie nulle part");
  exige(/etat\.first_player/.test(s), "la valeur n'est pas lue dans l'etat");
});

// ════════════════════════ F — LA BOITE DE BASE NE COMPTE PAS LES TUILES

test("F1 — le moteur remplit les Objectifs et les Recompenses dans LES DEUX boites", "F", async () => {
  const a = await partieTemoin(GRAINES[0], BASE);
  const b = await partieTemoin(GRAINES[0], TOUT);
  for (const [nom, r] of [["base", a], ["base+Decouverte", b]]) {
    exige(r.fin.jalons.length === 3, `${nom} : ${r.fin.jalons.length} Objectif(s)`);
    exige(r.fin.recompenses.length === 3, `${nom} : ${r.fin.recompenses.length} Recompense(s)`);
  }
});

test("F2 — en boite de base, ces deux parts du score valent zero du debut a la fin", "F", async () => {
  const { instants, fin } = await partieTemoin(GRAINES[0], BASE);
  for (const i of [...instants, fin]) {
    for (const j of [0, 1]) {
      for (const cle of ["milestones", "awards"]) {
        exige((i.joueurs[j].parts[cle] || 0) === 0,
          `boite de base, joueur ${j} : « ${cle} » vaut ${i.joueurs[j].parts[cle]}`);
      }
    }
  }
});

test("F3 — avec l'extension, l'une d'elles finit non nulle : la mesure porte sur quelque chose", "F", async () => {
  let vues = 0;
  for (const g of GRAINES) {
    const { fin } = await partieTemoin(g, TOUT);
    for (const j of [0, 1]) {
      for (const cle of ["milestones", "awards"]) {
        if ((fin.joueurs[j].parts[cle] || 0) !== 0) vues++;
      }
    }
  }
  exige(vues > 0, "aucune part d'Objectif ni de Recompense n'est non nulle a la fin de trois parties");
});

test("F4 — en boite de base, le moteur marque pourtant des Objectifs comme PRIS", "F", async () => {
  const { instants, fin } = await partieTemoin(GRAINES[0], BASE);
  const pris = [...instants, fin].some((i) => i.prises.some((p) => p.some(Boolean)));
  exige(pris,
    "aucun Objectif pris en boite de base sur cette graine : la verification F5 ne porterait sur rien");
});

test("F5 — l'ecran sait quelle boite est sur la table, et ne le devine pas", "F", () => {
  reglerBoites("base");
  exige(honneursComptent() === false, "la boite de base compte encore les honneurs");
  reglerBoites("base,decouverte");
  exige(honneursComptent() === true, "l'extension ne compte plus les honneurs");
  reglerBoites("");
  exige(honneursComptent() === false, "une composition vide compte les honneurs");
  reglerBoites("BASE,DECOUVERTE");
  exige(honneursComptent() === true, "la composition n'est pas lue sans egard a la casse");
});

test("F6 — les deux cases de score sortent de la mise en page hors extension", "F", () => {
  const s = source("vue/joueurs.js");
  exige(/PARTS_HONNEUR/.test(s), "les deux parts ne sont pas distinguees");
  exige(/honneursComptent\(\)/.test(s), "la barre des joueurs ne consulte pas la boite");
});

test("F7 — la bande des tuiles n'est pas montree hors extension", "F", () => {
  const s = source("vue/monde.js");
  exige(/honneursComptent\(\)/.test(s), "le bandeau ne consulte pas la boite");
});

// ═══════════════════════════ G — LE CLASSEMENT DES RECOMPENSES SE VOIT

test("G1 — le moteur publie une valeur par Recompense et par joueur", "G", async () => {
  for (const g of GRAINES) {
    const { fin } = await partieTemoin(g, TOUT);
    for (const j of [0, 1]) {
      const noms = Object.keys(fin.joueurs[j].valeurs).sort();
      exige(JSON.stringify(noms) === JSON.stringify(fin.recompenses.slice().sort()),
        `graine ${g}, joueur ${j} : ${noms} publie(s) pour les tuiles ${fin.recompenses}`);
    }
  }
});

test("G2 — ces valeurs BOUGENT dans la partie : ce n'est pas un classement fige", "G", async () => {
  const { instants } = await partieTemoin(GRAINES[0], TOUT);
  let mouvantes = 0;
  for (const nom of instants[0].recompenses) {
    for (const j of [0, 1]) {
      const vues = new Set(instants.map((i) => i.joueurs[j].valeurs[nom]));
      if (vues.size >= 2) mouvantes++;
    }
  }
  exige(mouvantes >= 2, `${mouvantes} valeur(s) bougent seulement`);
});

test("G3 — l'ecran declare les six chemins, et les construit depuis les tuiles du moteur", "G", () => {
  const s = source("vue/monde.js");
  exige(/valeurs_recompenses/.test(s), "aucun chemin de Recompense declare a l'ecran");
  exige(/etat\.awards/.test(s), "les tuiles ne viennent pas de l'etat : la liste serait ecrite a la main");
  for (const j of [0, 1]) {
    exige(s.includes(`players.\${j}.valeurs_recompenses`) || s.includes(`players.${j}.valeurs_recompenses`),
      `le chemin du siege ${j} n'est declare nulle part`);
  }
});

test("G4 — le classement n'est PAS pose dans la ventilation provisoire du score", "G", () => {
  const s = source("vue/joueurs.js");
  exige(!/valeurs_recompenses/.test(s),
    "le classement des Recompenses est entre dans la barre des joueurs : `score.py` y exige " +
    "que la seule part provisoire soit « awards »");
});

// ═════════════════════════════ H — LA VENTE PORTE SON NUMERO D'OCCASION

test("H1 — l'entree de vente fabriquee par l'ecran nomme son occasion", "H", () => {
  const s = source("vue/vente.js");
  const constructions = s.match(/vendre\s*:\s*\{[^}]*\}/gs) || [];
  exige(constructions.length > 0, "aucune construction d'entree de vente");
  for (const c of constructions) {
    exige(/\boccasion\b/.test(c), `entree sans numero : ${c.replace(/\s+/g, " ").slice(0, 100)}`);
  }
});

test("H2 — sans numero relevable, l'ecran ne fabrique aucune vente", "H", () => {
  const s = source("vue/vente.js");
  exige(/Number\.isInteger\(occasionDeMonSiege\)/.test(s),
    "rien n'empeche de rendre une vente sans numero, qui tomberait a la premiere occasion du siege");
});

test("H3 — LA PROPRIETE : le moteur refuse une vente dont le numero n'est pas encore venu", "H", () => {
  let essais = 0, refus = 0;
  for (const g of [7501, 7502, 7503]) {
    const p = creerPartie(pont, { graine: g, boites: TOUT });
    let garde = 0;
    while (!p.termine && garde++ < 4000) {
      const occ = (p.occasions || [])[0];
      if (occ && Number.isInteger(occ.numero) && (p.etat.players[occ.joueur].hand || []).length > 0) {
        essais++;
        try { p.vendre({ cartes: [0], joueur: occ.joueur, occasion: occ.numero + 9 }); }
        catch { refus++; }
        break;
      }
      const d = p.decision;
      if (!d) break;
      try { p.repondre(d.multiple ? [] : (d.montant ? (d.minimum ?? 0) : 0)); } catch { break; }
    }
  }
  exige(essais > 0, "aucune occasion de vente rencontree : la mesure ne porte sur rien");
  exige(refus === essais, `${essais - refus} vente(s) mal numerotees acceptees sur ${essais}`);
});

test("H4 — et il ACCEPTE la meme vente au bon numero : le refus ci-dessus n'est pas un refus de tout", "H", () => {
  let acceptees = 0, essais = 0;
  for (const g of [7501, 7502, 7503]) {
    const p = creerPartie(pont, { graine: g, boites: TOUT });
    let garde = 0;
    while (!p.termine && garde++ < 4000) {
      const occ = (p.occasions || [])[0];
      if (occ && Number.isInteger(occ.numero) && (p.etat.players[occ.joueur].hand || []).length > 0) {
        essais++;
        const avant = (p.etat.players[occ.joueur].hand || []).length;
        p.vendre({ cartes: [0], joueur: occ.joueur, occasion: occ.numero });
        if ((p.etat.players[occ.joueur].hand || []).length < avant) acceptees++;
        break;
      }
      const d = p.decision;
      if (!d) break;
      try { p.repondre(d.multiple ? [] : (d.montant ? (d.minimum ?? 0) : 0)); } catch { break; }
    }
  }
  exige(essais > 0, "aucune occasion rencontree");
  exige(acceptees === essais, `${essais - acceptees} vente(s) bien numerotees perdues sur ${essais}`);
});

// ═══════════════════════ I — LA VENTE DE L'ADVERSAIRE SE VOIT SUR MON ECRAN

test("I1 — le mot de l'annonce existe et ne compte aucun nombre", "I", () => {
  exige(typeof MOT.opponentSold === "string" && MOT.opponentSold.length > 0,
    "MOT.opponentSold manque");
  exige(!/[0-9]/.test(MOT.opponentSold),
    `l'annonce porte un nombre : « ${MOT.opponentSold} » — un compteur n'est pas une annonce`);
});

test("I2 — LA PROPRIETE : le mot de l'annonce est UNIQUE dans toute la page", "I", () => {
  const jeton = MOT.opponentSold.toLowerCase().match(/[a-z]{4,}/g)
    .find((m) => !["card", "cards", "a"].includes(m));
  exige(jeton, `aucun mot distinctif dans « ${MOT.opponentSold} »`);
  const porteurs = [];
  for (const f of fichiersDeLaPage()) {
    const t = readFileSync(f, "utf8").toLowerCase();
    if (new RegExp(`\\b${jeton}\\b`).test(t)) porteurs.push(f.slice(LIVRAISON.length + 1));
  }
  exige(porteurs.length === 1 && porteurs[0] === "vue/mots.js",
    `le mot « ${jeton} » parait dans ${porteurs} — il ne distinguerait plus une vente`);
});

test("I3 — l'annonce est declenchee par la REPONSE du siege d'en face, pas par un compteur", "I", () => {
  const s = source("interface.js");
  exige(/venteDuSiege/.test(s), "aucune reconnaissance de la vente dans la reponse de l'autre");
  exige(/guetterSesVentes/.test(s), "le siege d'en face n'est pas ecoute");
  exige(!/ventes_volontaires/.test(corps(s, "venteDuSiege")),
    "l'annonce s'appuie sur le compteur global `ventes_volontaires`, qui dit COMBIEN de " +
    "ventes ont eu lieu et jamais QUI a vendu : deux ventes du meme tour n'en font qu'une");
});

test("I4 — elle se tait pendant un rattrapage", "I", () => {
  const s = source("interface.js");
  exige(/rattrapageEnCours\(\)/.test(s),
    "au rechargement, la page rejoue tout l'historique et annoncerait des ventes vieilles " +
    "d'une demi-heure");
});

test("I5 — elle passe, et sa duree n'est pas zerotable par le reglage d'animations", "I", () => {
  const s = source("vue/mains.js");
  exige(/VENTE_ADVERSE_MS/.test(s), "l'annonce n'a pas de duree");
  exige(/setTimeout\(/.test(s), "l'annonce ne s'efface jamais : elle deviendrait un decor");
  const bloc = s.slice(s.indexOf("export function venteAdverse"), s.indexOf("export function venteAdverse") + 700);
  exige(!/duree\(/.test(bloc),
    "l'annonce passe par `duree()`, que `?animations=non` met a zero : elle disparaitrait " +
    "avant d'etre nee");
});

test("I6 — elle pose deux marques independantes sur la zone d'en face", "I", () => {
  const s = source("vue/mains.js");
  exige(/dataset\.vente = "oui"/.test(s), "aucune marque d'attribut");
  exige(/adverse-vente/.test(s), "aucun element pour porter le mot");
});

// ── le verdict ──────────────────────────────────────────────────────────────
const rouges = [];
let verts = 0;
for (const { nom, critere, fn } of carnet) {
  try {
    await fn();
    verts++;
  } catch (e) {
    rouges.push(`[${critere}] ${nom}\n         ${e.message}`);
  }
}
const criteres = new Set(carnet.map((c) => c.critere));
console.log(`verifications : ${carnet.length}, criteres couverts : ${[...criteres].sort().join(" ")}`);
for (const r of rouges) console.log("   ECHEC " + r);
if (rouges.length) {
  console.log(`ROUGE ${rouges.length} verification(s) en echec sur ${carnet.length}`);
  process.exit(1);
}
console.log(`VERT ${verts} verifications passees sur ${carnet.length}, ` +
  `neuf criteres (${[...criteres].sort().join(" ")}) — vainqueur, departage, first_player, ` +
  `recompense, jalon, vente et questions simultanees`);

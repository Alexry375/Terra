#!/usr/bin/env node
// (jouer-contre-l-apprenti) UNE PARTIE ENTIÈRE CONTRE LE JOUEUR ENTRAÎNÉ, DANS LA
// VRAIE PAGE, DANS UN VRAI NAVIGATEUR.
//
//   node web/webapp/verif/partie-entiere-au-navigateur.mjs [graine] [--liste <fichier>]
//
// La page est servie par `web/servir.py` et ouverte dans Chromium sur
//   index.html?graine=<g>&boites=base,decouverte&decide=programme&adversaire=apprenti
//             &animations=non&sons=non
// c'est-à-dire : le siège 0 joue par le programme de la page (le tireur au hasard,
// graine*2+1), le siège d'en face est l'adversaire entraîné — installé par la page
// elle-même, ses réglages demandés au serveur. Le banc ne joue AUCUN coup : il
// attend l'écran final et le lit.
//
// Ce qu'il imprime en dernière ligne :
//   SCORES <siège 0> <siège 1> DECISIONS <n> REFLEXION-MOYENNE <ms> REFLEXION-PIRE <ms>
//
// · les scores sont ceux de l'écran final (`data-score-final`) ;
// · DECISIONS compte les questions de la partie, les deux sièges réunis : la page
//   enregistre la partie juste avant chacune (`sauverPartie`), le banc compte ces
//   enregistrements ; et il recoupe avec le dernier enregistrement lu ;
// · les deux durées sont celles du SEUL adversaire entraîné, par décision : la
//   page chronomètre chacune de ses réponses dans le navigateur
//   (`performance.measure`, `adversaire-entraine.js`) et le banc les relit là.
//   Ses consultations aux occasions de vente sont mesurées à part et imprimées
//   au-dessus, pas mêlées à la réflexion par décision.
//
// `--liste <fichier>` écrit le dernier enregistrement de la partie (la liste des
// décisions jusqu'à la dernière question), pour le comparer à un autre monde.
import { writeFileSync } from "node:fs";
import { chromium, servir } from "./navigateur-de-controle.mjs";

const argv = process.argv.slice(2);
const iListe = argv.indexOf("--liste");
const fichierListe = iListe >= 0 ? argv[iListe + 1] : null;
const positionnels = argv.filter((_, i) => iListe < 0 || (i !== iListe && i !== iListe + 1));
const graine = Number(positionnels[0] ?? 880002);
const boites = "base,decouverte";
const CLE = "terra.partie-en-cours";
const DELAI = 30 * 60 * 1000;

const { serveur, adresse } = await servir();
const navigateur = await chromium.launch();
try {
  const page = await navigateur.newPage();
  const fautes = [];
  page.on("pageerror", (e) => fautes.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") fautes.push("console : " + m.text()); });
  // Les enregistrements de la page, relevés au passage : leur nombre est celui des
  // questions posées, le dernier porte la liste des décisions.
  await page.addInitScript((cle) => {
    const vu = { n: 0, dernier: null };
    window.__enregistrements = vu;
    const ecrire = Storage.prototype.setItem;
    Storage.prototype.setItem = function (k, v) {
      if (k === cle) { vu.n += 1; vu.dernier = v; }
      return ecrire.call(this, k, v);
    };
  }, CLE);

  const debut = Date.now();
  const url = `${adresse}/index.html?graine=${graine}&boites=${boites}`
    + "&decide=programme&adversaire=apprenti&animations=non&sons=non";
  await page.goto(url);
  await page.waitForFunction(
    () => document.body.dataset.phase === "fin" || document.getElementById("panne"),
    null,
    { timeout: DELAI, polling: 500 },
  );
  const panne = await page.$eval("#panne", (z) => z.textContent).catch(() => null);
  if (panne) throw new Error("la page est en panne : " + panne);

  const r = await page.evaluate(() => {
    const score = (j) => Number(document.querySelector(`[data-score-final="${j}"]`)?.textContent);
    const durees = (nom) => performance.getEntriesByName(nom).map((m) => m.duration);
    return {
      scores: [score(0), score(1)],
      reflexions: durees("terra:reflexion-apprenti"),
      ventes: durees("terra:vente-apprenti"),
      enregistrements: window.__enregistrements.n,
      dernier: window.__enregistrements.dernier,
      agent: navigator.userAgent,
    };
  });
  const secondes = (Date.now() - debut) / 1000;
  if (fautes.length) throw new Error("erreurs dans la page : " + fautes.join(" | "));
  if (!r.scores.every(Number.isFinite)) throw new Error("l'écran final ne montre pas les deux scores");
  if (r.reflexions.length === 0) throw new Error("l'adversaire entraîné n'a pris aucune décision");

  const dernier = JSON.parse(r.dernier);
  if (dernier.adversaire !== "apprenti") {
    throw new Error(`la partie s'est enregistrée contre « ${dernier.adversaire} », pas contre l'apprenti`);
  }
  const estVente = (e) => e && typeof e === "object" && !Array.isArray(e) && e.vendre;
  const ventesFaites = dernier.decisions.filter(estVente).length;
  const questionsDuDernier = dernier.decisions.length - ventesFaites + 1;
  if (questionsDuDernier !== r.enregistrements) {
    throw new Error(
      `${r.enregistrements} enregistrements, mais le dernier dit ${questionsDuDernier} questions`,
    );
  }
  if (fichierListe) writeFileSync(fichierListe, JSON.stringify(dernier.decisions) + "\n");

  const stats = (xs) => ({
    moyenne: xs.reduce((a, b) => a + b, 0) / xs.length,
    pire: Math.max(...xs),
  });
  const d = stats(r.reflexions);
  console.log(`navigateur : ${r.agent}`);
  console.log(`adresse : ${url.replace(adresse, "")}`);
  console.log(`partie jouée en ${secondes.toFixed(1)} s (horloge murale, respirations de l'écran comprises)`);
  console.log(`ventes de l'adversaire entraîné : ${ventesFaites} (sur la liste jusqu'à la dernière question)`);
  if (r.ventes.length) {
    const v = stats(r.ventes);
    console.log(
      `consultations aux occasions de vente : ${r.ventes.length}, ` +
        `moyenne ${v.moyenne.toFixed(1)} ms, pire ${v.pire.toFixed(1)} ms`,
    );
  }
  console.log(`décisions de l'adversaire entraîné : ${r.reflexions.length}`);
  console.log(
    `SCORES ${r.scores[0]} ${r.scores[1]} DECISIONS ${r.enregistrements} ` +
      `REFLEXION-MOYENNE ${d.moyenne.toFixed(1)} REFLEXION-PIRE ${d.pire.toFixed(1)}`,
  );
} finally {
  await navigateur.close();
  serveur.kill();
}

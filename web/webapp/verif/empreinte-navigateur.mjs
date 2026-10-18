#!/usr/bin/env node
// (jouer-contre-l-apprenti) L'EMPREINTE DES DÉCISIONS DU JOUEUR ENTRAÎNÉ, PRISE
// DANS UN VRAI NAVIGATEUR.
//
//   node web/webapp/verif/empreinte-navigateur.mjs [graine] [combien] [boites] [--avant]
//
// Elle se compare, au caractère près, à `inputs/empreinte-node.mjs` (le contrat du
// chantier) : même boucle, même repliage, mêmes champs, même ordre. Ce qui change,
// c'est le MONDE : la page est servie par `web/servir.py`, ouverte dans Chromium,
// et c'est le navigateur qui charge les modules, le moteur compilé et les réglages
// du réseau — par le réseau, à `poids/apprenti.txt`. Le joueur est construit par
// les fonctions mêmes que la page utilise (`adversaire-entraine.js`), sur les
// mêmes graines qu'elle lui donne.
//
// LE MOMENT OÙ LES CERVEAUX SONT CONSTRUITS FAIT PARTIE DE LA RÉFÉRENCE. Celle-ci
// les construit APRÈS `creerPartie` : le joueur entraîné, qui relève la partie en
// observant `pont.pas`, a alors manqué le premier appel, et la décision 0 est
// répondue sans aucun essai (la première option — `MODE_DEGRADE`, que Node
// annonce sur sa sortie d'erreur). Le banc reproduit cet ordre, puisque c'est à
// elle qu'il se compare ; il le DIT sur sa sortie. La page, elle, construit son
// cerveau AVANT la partie (`interface.js`, `cerveauDEnFace`) : `--avant` prend
// l'empreinte dans cet ordre-là, à comparer à la même boucle sous Node construite
// dans le même ordre.
//
// Reproductible au caractère près : aucune horloge, aucun hasard non semé.
import { chromium, servir } from "./navigateur-de-controle.mjs";

const args = process.argv.slice(2).filter((a) => a !== "--avant");
const avant = process.argv.includes("--avant");
const graine = Number(args[0] ?? 880001);
const combien = Number(args[1] ?? 40);
const boites = args[2] ?? "base,decouverte";

const { serveur, adresse } = await servir();
const navigateur = await chromium.launch();
try {
  const page = await navigateur.newPage();
  const fautes = [];
  page.on("pageerror", (e) => fautes.push(String(e)));
  // Les réglages doivent arriver PAR LE RÉSEAU : on regarde passer la réponse.
  const reglages = [];
  const lectures = [];
  page.on("response", (r) => {
    if (new URL(r.url()).pathname === "/poids/apprenti.txt") {
      const vu = { statut: r.status(), reserve: r.headers()["cache-control"] || "(aucun)", octets: 0 };
      reglages.push(vu);
      lectures.push(r.body().then((b) => { vu.octets = b.length; }, () => {}));
    }
  });
  await page.goto(`${adresse}/index.html?animations=non&sons=non`);
  const r = await page.evaluate(
    async ({ graine, combien, boites, avant }) => {
      const { ouvrirPontDepuis } = await import("/pont.js");
      const { creerPartie, offrirLesOccasions } = await import("/partie.js");
      const { chargerReglagesApprenti, cerveauApprenti } = await import("/adversaire-entraine.js");

      const pont = await ouvrirPontDepuis(".");
      const poids = await chargerReglagesApprenti();
      const construire = () => [
        cerveauApprenti(poids, graine * 2 + 1, "apprenti-0", pont, boites),
        cerveauApprenti(poids, graine * 2 + 2, "apprenti-1", pont, boites),
      ];
      let cerveaux = avant ? construire() : null;
      const partie = creerPartie(pont, { graine, boites });
      if (!avant) cerveaux = construire();

      let empreinte = 0;
      let rang = 0;
      const lignes = [];
      while (rang < combien) {
        await offrirLesOccasions(partie, cerveaux);
        if (partie.termine) break;
        const d = partie.decision;
        if (!d) throw new Error("ni décision ni fin de partie");
        const reponse = await cerveaux[d.joueur].decider(d, partie.etat);
        const indice = Array.isArray(reponse) ? reponse.join(".") : String(reponse);
        const options = (d.options ? d.options.length : 0) + (d.passer ? 1 : 0);
        lignes.push(`${rang} ${d.joueur} ${options} ${indice}`);
        for (const c of `${d.joueur}|${options}|${indice};`) {
          empreinte = (empreinte * 31 + c.codePointAt(0)) % 2147483647;
        }
        partie.repondre(reponse);
        rang++;
      }
      const reflexions = performance.getEntriesByName("terra:reflexion-apprenti").map((m) => m.duration);
      return { lignes, empreinte, rang, reflexions, agent: navigator.userAgent };
    },
    { graine, combien, boites, avant },
  );

  await Promise.all(lectures);
  if (fautes.length) throw new Error("erreurs dans la page : " + fautes.join(" | "));
  if (reglages.length === 0) throw new Error("la page n'a jamais demandé poids/apprenti.txt au serveur");
  for (const l of r.lignes) console.log(l);
  console.log(`navigateur : ${r.agent}`);
  for (const g of reglages) {
    console.log(`réglages reçus par le réseau : statut ${g.statut}, ${g.octets} octets, Cache-Control « ${g.reserve} »`);
  }
  if (r.reflexions.length) {
    const moy = r.reflexions.reduce((a, b) => a + b, 0) / r.reflexions.length;
    console.log(
      `réflexion mesurée dans le navigateur : ${r.reflexions.length} consultation(s), ` +
        `moyenne ${moy.toFixed(1)} ms, pire ${Math.max(...r.reflexions).toFixed(1)} ms`,
    );
  }
  console.log(
    avant
      ? "ordre : cerveaux construits AVANT creerPartie (celui de la page)"
      : "ordre : cerveaux construits APRÈS creerPartie, comme la référence — la décision 0 y est répondue sans essai",
  );
  console.log(`graine ${graine} — ${r.rang} décision(s) repliée(s)`);
  console.log(`EMPREINTE ${r.empreinte}`);
} finally {
  await navigateur.close();
  serveur.kill();
}

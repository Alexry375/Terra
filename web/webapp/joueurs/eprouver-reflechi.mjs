#!/usr/bin/env node
// LES ÉPREUVES DU JOUEUR RÉFLÉCHI — ma couverture à moi, au-delà des contrôles
// du contrat.
//
//   node web/webapp/joueurs/eprouver-reflechi.mjs [graines]
//
// Ce fichier est INERTE à l'import : il ne joue rien tant qu'on ne le lance pas
// en ligne de commande. C'est nécessaire — les bancs du contrat importent tout
// ce que contient `joueurs/` pour y chercher les fournisseurs, et un fichier qui
// jouerait une partie au chargement les ferait attendre des minutes.
//
// QUATRE ÉPREUVES, ET LA QUATRIÈME EST CELLE QUI COMPTE.
//
//   1. Les réponses sont RECEVABLES : indice dans les bornes, montant entre
//      minimum et maximum, liste de la bonne longueur et sans doublon. On ne
//      s'en remet pas au refus du moteur pour s'en apercevoir.
//   2. `decider` est une FONCTION PURE : reposée dix fois, la même question
//      dans le même état rend dix fois la même réponse.
//   3. Il ne regarde pas la main d'en face : on remplace le siège adverse en
//      ENTIER par une caricature (main vide, cartes retournées, ressources
//      absurdes) et la réponse ne bouge pas.
//   4. **L'ÉPREUVE 3 A DES DENTS.** Un vert ne prouve rien si le contrôle est
//      incapable de voir un tricheur. On fait donc passer la même épreuve à un
//      joueur SABOTÉ, écrit ici même, qui lit la main d'en face — et on exige
//      qu'il soit PRIS. Si le tricheur passait, c'est l'épreuve qui serait
//      fausse, pas le joueur qui serait honnête.

import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, resolve } from "node:path";
import { formeDeLaReponse, nombreDeChoix } from "../fournisseurs.js";
import { fournisseurReflechi } from "./reflechi.js";

const W = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * LE JOUEUR SABOTÉ, qui existe pour être pris. Il ne s'exporte pas sous un nom
 * commençant par « fournisseur » : les bancs du contrat ne doivent pas le
 * confondre avec un joueur livré.
 */
function tricheurDEpreuve() {
  return {
    nom: "tricheur",
    decider(d, etat) {
      const autre = (etat.players || []).find((p) => p.player !== d.joueur);
      const enFace = autre && Array.isArray(autre.hand) ? autre.hand.length : 0;
      const forme = formeDeLaReponse(d);
      if (forme === "montant") return d.minimum ?? 0;
      if (forme === "multiple") {
        const k = d.a_choisir ?? 1;
        return Array.from({ length: Math.min(k, d.options.length) }, (_, i) => i);
      }
      // Voilà la triche : la réponse dépend de la main d'en face.
      return enFace % Math.max(1, nombreDeChoix(d));
    },
  };
}

/** Le siège adverse, remplacé par une caricature. */
function caricature(etat, moi, mode) {
  const copie = JSON.parse(JSON.stringify(etat));
  for (const p of copie.players || []) {
    const siege = p.player !== undefined ? p.player : copie.players.indexOf(p);
    if (siege === moi) continue;
    if (mode === "vide") p.hand = [];
    else if (mode === "retournee" && Array.isArray(p.hand)) {
      p.hand = p.hand.slice().reverse().map((c, i) => ({ ...c, id: (c.id || 0) + 1000 + i }));
    } else if (mode === "absurde") {
      p.hand = [{ id: 999, nom: "?", prix: 99, pv: 9, couleur: "verte", badges: [] }];
      p.mc = 999;
      p.plants = 99;
      p.production = { mc: 99, heat: 99, plants: 99, cards: 9 };
      p.played = [];
      p.score = 999;
    }
    if (Array.isArray(p.main_payable)) p.main_payable = p.main_payable.map(() => true);
  }
  return copie;
}

function recevable(d, r) {
  const forme = formeDeLaReponse(d);
  if (forme === "montant") {
    return Number.isInteger(r) && r >= (d.minimum ?? 0) && r <= (d.maximum ?? 0);
  }
  if (forme === "multiple") {
    if (!Array.isArray(r)) return false;
    if (new Set(r).size !== r.length) return false;
    if (d.a_choisir !== undefined && r.length !== d.a_choisir) return false;
    return r.every((i) => Number.isInteger(i) && i >= 0 && i < d.options.length);
  }
  return Number.isInteger(r) && r >= 0 && r < nombreDeChoix(d);
}

async function main() {
  const graines = Number(process.argv[2] || 4);
  const { ouvrirPontDepuis } = await import(resolve(W, "pont.js"));
  const { creerPartie, jouerJusquAuBout } = await import(resolve(W, "partie.js"));
  const pont = await ouvrirPontDepuis(W);

  const bilan = {
    decisions: 0,
    occasions: 0, // les questions à plusieurs options, seules probantes
    irrecevables: 0,
    instables: 0, // la même question, une autre réponse
    regarde: 0, // la réponse change quand la main d'en face change
    tricheurPris: 0,
  };

  for (let g = 1; g <= graines; g++) {
    const partie = creerPartie(pont, { graine: g, boites: "base,decouverte" });
    const espion = (siege) => {
      const vrai = fournisseurReflechi(g, "reflechi");
      const faux = tricheurDEpreuve();
      return {
        nom: "epreuve",
        async decider(d, etat) {
          bilan.decisions++;
          const r = await vrai.decider(d, etat);
          if (!recevable(d, r)) bilan.irrecevables++;
          const combien = (d.options || []).length;
          if (combien > 1) {
            const attendu = JSON.stringify(r);
            // 2 — pureté : dix fois la même question, dix fois la même réponse.
            for (let k = 0; k < 10; k++) {
              if (JSON.stringify(await vrai.decider(d, etat)) !== attendu) bilan.instables++;
            }
            // 3 — il ne regarde pas ; 4 — et le tricheur, lui, se fait prendre.
            const refTricheur = JSON.stringify(await faux.decider(d, etat));
            for (const mode of ["vide", "retournee", "absurde"]) {
              const autre = caricature(etat, siege, mode);
              bilan.occasions++;
              if (JSON.stringify(await vrai.decider(d, autre)) !== attendu) bilan.regarde++;
              if (JSON.stringify(await faux.decider(d, autre)) !== refTricheur) bilan.tricheurPris++;
            }
          }
          return r;
        },
      };
    };
    // Le joueur tient les DEUX sièges : le siège 1 est éprouvé comme le siège 0.
    await jouerJusquAuBout(partie, [espion(0), espion(1)]);
  }

  const fautes = [];
  if (bilan.occasions < 200) fautes.push(`seulement ${bilan.occasions} occasion(s) : rien n'a été mesuré`);
  if (bilan.irrecevables) fautes.push(`${bilan.irrecevables} réponse(s) hors bornes`);
  if (bilan.instables) fautes.push(`${bilan.instables} réponse(s) instables : « decider » n'est pas une fonction pure`);
  if (bilan.regarde) fautes.push(`${bilan.regarde} fois, la réponse change quand la main d'en face change`);
  if (!bilan.tricheurPris) {
    fautes.push("le joueur SABOTÉ n'a pas été pris : l'épreuve elle-même est fausse, "
      + "son vert ne prouverait rien");
  }

  console.log(`    ${bilan.decisions} décision(s) jouées, ${bilan.occasions} occasion(s) probante(s)`);
  console.log(`    le joueur saboté a été pris ${bilan.tricheurPris} fois (il doit l'être)`);
  for (const f of fautes) console.log("ECHEC :", f);
  if (fautes.length) process.exit(1);
  console.log("    réponses recevables, décisions pures, main d'en face jamais consultée");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}

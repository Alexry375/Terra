// TERRA — l'écran du jeu.
//
// Le moteur de règles (Rust, WebAssembly) décide de TOUT. Cette page ne connaît
// aucune règle : elle ne sait pas ce qu'une carte coûte, ni si elle est jouable,
// ni combien vaut un point. Elle fait exactement deux choses :
//
//   1. donner à voir `etat` — l'état que le moteur rend lui-même à chaque
//      décision (`engine::observe::state_view`). Chaque nombre affiché porte son
//      chemin exact dans cet objet (`data-valeur`), pour qu'on puisse vérifier à
//      tout instant que l'écran ne ment pas ;
//   2. offrir `decision.options` — et rien d'autre — puis rendre au moteur
//      l'indice choisi.
//
// Le joueur humain est branché par le même « fournisseur de décisions » que
// celui par lequel se brancherait un cerveau artificiel (`adversaire.md`). Ici
// un seul fournisseur tient les DEUX joueurs : même écran, mains face visible.

import { ouvrirPontDepuis } from "./pont.js";
import { creerPartie, jouerJusquAuBout } from "./partie.js";
import { fournisseurHumain } from "./fournisseurs.js";

import { chargerMateriel } from "./vue/materiel.js";
import { construireMonde, majMonde, oublier } from "./vue/monde.js";
import { construireJoueurs, majJoueurs } from "./vue/joueurs.js";
import { construireMains, majMains } from "./vue/mains.js";
import { construireScene, poserDecision, viderScene } from "./vue/scene.js";
import { construireLoupe } from "./vue/loupe.js";
import { oublierRefs } from "./vue/ecrire.js";
import { construireAnnonce, annonceManche, annoncePhases, ecranFinal } from "./vue/annonce.js";
import * as son from "./vue/son.js";

// ------------------------------------------------------------------ l'adresse

/**
 * L'adresse porte la partie : `?graine=<entier>&boites=base` ou
 * `base,decouverte`. Quand elle est là, la partie démarre sans le moindre clic.
 */
function lireAdresse() {
  const p = new URLSearchParams(location.search);
  const g = p.get("graine");
  const b = p.get("boites");
  if (g === null && b === null) return null;
  const graine = Number.parseInt(g ?? "1", 10);
  return {
    graine: Number.isFinite(graine) ? graine : 1,
    boites: b === "base" || b === "base,decouverte" ? b : "base,decouverte",
  };
}

// ------------------------------------------------------------------ le décor

function batir() {
  construireMonde();
  construireMains();
  construireJoueurs();
  construireScene();
  construireAnnonce();
  construireLoupe();
}

function etatDuChargement(texte) {
  let e = document.getElementById("chargement");
  if (!e) {
    e = document.createElement("div");
    e.id = "chargement";
    document.body.appendChild(e);
  }
  e.textContent = texte;
  return e;
}

function panne(e) {
  // Un échec se montre, il ne se jette pas : une exception non rattrapée est une
  // erreur de console, et une erreur de console est un écran cassé.
  const z = document.createElement("div");
  z.id = "panne";
  z.textContent = "Le moteur n'a pas pu continuer : " + (e && e.message ? e.message : e);
  document.body.appendChild(z);
}

// ------------------------------------------------------------------ la partie

// Ce que l'écran a déjà annoncé, pour ne pas répéter la même manche deux fois.
let dejaVu = { manche: null, phases: null };

function theatre(etat) {
  if (etat.generation !== dejaVu.manche) {
    if (dejaVu.manche !== null) {
      annonceManche(etat.generation);
      son.sonManche();
    }
    dejaVu.manche = etat.generation;
    dejaVu.phases = null;
  }
  const paire = etat.players.map((p) => p.chosen_phase).join("-");
  if (paire !== dejaVu.phases && etat.players.every((p) => p.chosen_phase)) {
    if (dejaVu.phases !== null) annoncePhases(etat);
    dejaVu.phases = paire;
  }
}

/** Le rendu complet d'un instant : le monde, les deux équipages, les deux mains. */
function rendre(etat, decision) {
  document.body.dataset.actif = decision ? String(decision.joueur) : "";
  majMonde(etat);
  majJoueurs(etat, decision);
  majMains(etat, decision);
  theatre(etat);
}

async function lancer({ graine, boites }) {
  document.body.dataset.phase = "chargement";
  etatDuChargement("réveil du moteur…");

  const pont = await ouvrirPontDepuis(".");
  document.getElementById("chargement")?.remove();
  document.body.dataset.phase = "partie";
  oublier();
  oublierRefs();
  dejaVu = { manche: null, phases: null };

  const partie = creerPartie(pont, { graine, boites });

  // Le fournisseur : il ne connaît aucune règle, il attend un clic sur l'une des
  // options que le moteur vient d'énumérer.
  const humain = fournisseurHumain(async (d, etat) => {
    rendre(etat, d);
    const reponse = await poserDecision(d, etat);
    son.eveiller();
    son.sonChoix();
    return reponse;
  });

  await jouerJusquAuBout(partie, [humain, humain]);

  viderScene();
  rendre(partie.etat, null);
  document.body.dataset.phase = "fin";
  ecranFinal(partie.etat);
  son.sonFin();
}

// ------------------------------------------------------------- l'écran d'entrée

function ecranEntree() {
  const z = document.createElement("section");
  z.id = "entree";
  z.innerHTML = `
    <h1>Terra</h1>
    <p class="entree__sous">Terraforming Mars · Ares Expedition — deux joueurs, un écran</p>
    <div class="entree__reglages">
      <label>Graine <input id="entree-graine" type="number" value="7"></label>
      <label>Boîtes
        <select id="entree-boites">
          <option value="base">base</option>
          <option value="base,decouverte" selected>base + Découverte</option>
        </select>
      </label>
    </div>
    <button id="entree-go" type="button">Commencer</button>`;
  document.body.appendChild(z);

  document.getElementById("entree-go").addEventListener("click", () => {
    const graine = Number.parseInt(document.getElementById("entree-graine").value, 10) || 1;
    const boites = document.getElementById("entree-boites").value;
    z.remove();
    son.eveiller();
    lancer({ graine, boites }).catch(panne);
  });
}

// ---------------------------------------------------------------- le démarrage

async function demarrer() {
  // Le manifeste d'abord : tout le décor est bâti à partir des images qu'il
  // désigne, il ne peut pas se construire avant d'être lu.
  try {
    await chargerMateriel();
  } catch (e) {
    panne(e);
    return;
  }
  batir();
  const adresse = lireAdresse();
  if (adresse) await lancer(adresse);
  else ecranEntree();
}

demarrer().catch(panne);

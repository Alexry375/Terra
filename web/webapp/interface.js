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
// UN SEUL POINT DE VUE, DEUX RÉGLAGES INDÉPENDANTS. L'écran ne montre plus les
// deux mains en clair : il montre CELLE DU SIÈGE REGARDÉ, en bas, et celle de
// l'adversaire en haut, retournée. Deux réglages, jamais confondus :
//
//   ?siege=0|1               quel joueur occupe le bas de l'écran
//   ?decide=humain|programme qui répond pour ce siège
//
// « Le siège du bas » et « celui qui clique » ne sont pas le même concept : mon
// intelligence artificielle peut tenir mon siège pendant que je la regarde
// jouer, cartes en clair, comme si c'était moi (`docs/INTERFACE_RETOURS_02.md`,
// point 26-bis). L'adversaire, lui, est toujours tenu par un programme
// (`fournisseurs.js`) et reste opaque.

import { ouvrirPontDepuis } from "./pont.js";
import { creerPartie, jouerJusquAuBout } from "./partie.js";
import { fournisseurHumain, fournisseurAleatoire } from "./fournisseurs.js";

import { chargerMateriel } from "./vue/materiel.js";
import { construireMonde, majMonde, oublier } from "./vue/monde.js";
import { construireJoueurs, majJoueurs, replacerBarres } from "./vue/joueurs.js";
import {
  construireMains, majMains, adversaireAgit, replacerMains, oublierMains,
} from "./vue/mains.js";
import {
  construirePhases, majPhases, enPlanification, oublierPhases,
} from "./vue/phases.js";
import {
  construirePlateaux, majPlateaux, replacerPlateaux, oublierPlateaux,
} from "./vue/plateau.js";
import {
  construireScene, poserDecision, viderScene, replacerScene, repondrePourLeSiege,
} from "./vue/scene.js";
import { construireLoupe } from "./vue/loupe.js";
import { oublierRefs } from "./vue/ecrire.js";
import { construireAnnonce, annonceManche, annoncePhases, ecranFinal } from "./vue/annonce.js";
import * as son from "./vue/son.js";
import { MOT, SIMULTANEES, actionAdverse } from "./vue/mots.js";

// Le temps qu'un geste de l'adversaire reste sous les yeux. Sans lui, un
// programme répondrait entre deux images et l'on ne verrait JAMAIS qu'il joue :
// l'écran mentirait par vitesse. Le siège tenu par un programme respire un peu
// plus longtemps — c'est sa partie à lui qu'on regarde.
const PAS_ADVERSE = 180;
const PAS_PROGRAMME = 320;

const pause = (ms) => new Promise((r) => setTimeout(r, ms));

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

/**
 * LE CADRE : de quel siège je regarde, et qui répond pour lui. Ces deux
 * réglages sont lus une seule fois, ici, et tout le reste de la page s'y réfère
 * — l'écran ne suppose jamais que « le joueur 0, c'est moi ».
 */
function lireCadre() {
  const p = new URLSearchParams(location.search);
  return {
    siege: p.get("siege") === "1" ? 1 : 0,
    decide: p.get("decide") === "programme" ? "programme" : "humain",
  };
}

const cadre = lireCadre();

// ------------------------------------------------------------------ le décor

function batir() {
  construireMonde();
  construirePlateaux();
  construireJoueurs();
  construireMains();
  construirePhases();
  construireScene();
  construireAnnonce();
  construireLoupe();
  // Le siège regardé est écrit sur le document : c'est lui qui décide quel
  // plateau se pose en haut et lequel se pose en bas.
  document.body.dataset.siege = String(cadre.siege);
  document.body.dataset.decide = cadre.decide;
  // TOUT ce qui est mesuré en pixels doit être remesuré quand la fenêtre change
  // de taille : les deux plateaux, les deux barres de joueur, et la grille des
  // choix de la décision en cours. On attend la fin du geste plutôt que de tout
  // refaire à chaque pixel de la poignée de redimensionnement.
  let minuteur = null;
  window.addEventListener("resize", () => {
    replacerPlateaux();
    replacerBarres();
    replacerMains();
    clearTimeout(minuteur);
    minuteur = setTimeout(replacerScene, 120);
  }, { passive: true });
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
  z.textContent = MOT.broken + (e && e.message ? e.message : e);
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
  // LA RÉVÉLATION N'A LIEU QU'UNE FOIS LES DEUX CHOIX FAITS. `chosen_phase`
  // garde la valeur de la manche précédente pour qui n'a pas encore rechoisi :
  // annoncer sur « les deux sont non nuls » projetait en grand, une fois par
  // manche, la carte que l'adversaire venait de poser face cachée — et, vu du
  // siège interrogé en second, la lui montrait avant qu'il ait choisi la sienne.
  // C'est `vue/phases.js` qui sait où en est la planification, et lui seul.
  const paire = etat.players.map((p) => p.chosen_phase).join("-");
  if (!enPlanification() && paire !== dejaVu.phases &&
      etat.players.every((p) => p.chosen_phase)) {
    if (dejaVu.phases !== null) annoncePhases(etat);
    dejaVu.phases = paire;
  }
}

/**
 * Le rendu complet d'un instant, DEPUIS LE SIÈGE REGARDÉ : le monde, les deux
 * équipages, les deux plateaux, ma main en clair et celle de l'adversaire
 * retournée. La décision passée ici sert à dire QUI parle ; elle ne choisit
 * jamais ce que l'on montre — c'est le siège qui le décide.
 */
function rendre(etat, decision) {
  document.body.dataset.actif = decision ? String(decision.joueur) : "";
  majMonde(etat);
  majPlateaux(etat, decision, cadre.siege);
  majJoueurs(etat, decision, cadre.siege);
  majMains(etat, decision, cadre.siege);
  majPhases(etat, decision);
  theatre(etat);
}

/**
 * MON SIÈGE, TENU PAR UNE MAIN. La scène se pose et attend le clic.
 *
 * Les trois questions que le moteur pose aux DEUX joueurs (remplacement des
 * corporations, des cartes projet, choix de la phase) se jouent en même temps à
 * la table : dès qu'elle m'est posée, on voit l'adversaire y répondre lui aussi,
 * dans son coin, en petit et retourné. Le reste du temps il attend, et la zone
 * du haut se tait.
 */
function siegeHumain() {
  return fournisseurHumain(async (d, etat) => {
    rendre(etat, d);
    adversaireAgit(SIMULTANEES.has(d.type) ? actionAdverse(d) : null);
    const reponse = await poserDecision(d, etat);
    son.eveiller();
    son.sonChoix();
    adversaireAgit(null);
    return reponse;
  }, "humain à l'écran");
}

/**
 * MON SIÈGE, TENU PAR UN PROGRAMME — « je regarde mon intelligence artificielle
 * jouer à ma place ». Elle voit exactement ce que je verrais : la scène est
 * posée comme pour un humain, ma main reste en clair, et la réponse arrive par
 * le même chemin qu'un clic (`vue/scene.js`).
 */
function siegeProgramme(graine) {
  const cerveau = fournisseurAleatoire(graine * 2 + 1, "programme au siège");
  return {
    nom: cerveau.nom,
    async decider(d, etat) {
      rendre(etat, d);
      adversaireAgit(SIMULTANEES.has(d.type) ? actionAdverse(d) : null);
      const attente = poserDecision(d, etat);
      const reponse = cerveau.decider(d, etat);
      await pause(PAS_PROGRAMME);
      repondrePourLeSiege(reponse);
      adversaireAgit(null);
      return attente;
    },
  };
}

/**
 * L'ADVERSAIRE — un programme qui décide arbitrairement, et qui reste OPAQUE.
 *
 * Sa décision ne redessine JAMAIS la scène : mon écran reste le mien, et lui
 * agit dans un coin, cartes retournées. On voit QU'il agit, jamais QUOI. C'est
 * précisément ce que l'ancien écran ne faisait pas : il donnait la parole — et
 * toute la surface — à celui qui décidait, quel qu'il soit.
 */
function adversaire(graine) {
  const cerveau = fournisseurAleatoire(graine * 2 + 2, "adversaire");
  return {
    nom: cerveau.nom,
    decider(d, etat) {
      rendre(etat, d);

      // UNE QUESTION POSÉE AUX DEUX. Elle va m'être posée à l'instant d'après :
      // on le montre en train d'y répondre, et l'écran PREND LE TEMPS de le
      // montrer. C'est le seul moment où l'attente a un sens — et aucun de ces
      // trois moments n'est jamais le dernier de la partie.
      if (SIMULTANEES.has(d.type)) {
        adversaireAgit(actionAdverse(d));
        return pause(PAS_ADVERSE).then(() => cerveau.decider(d, etat));
      }

      // PARTOUT AILLEURS, IL RÉPOND TOUT DE SUITE, dans le même tour de boucle
      // que ma propre réponse. Attendre ici laisserait l'écran sans question ET
      // sans fin de partie pendant qu'il termine la partie : mesuré sur huit
      // graines, c'est le joueur 0 qui prend la dernière décision sept fois,
      // donc au siège 1 la partie s'achève presque toujours pendant son tour.
      // Un écran qui ne montre alors ni question ni fin est un écran bloqué —
      // pour une main comme pour une machine qui pilote la page. Ce qu'il fait
      // se voit à son plateau, qui change sous nos yeux.
      return cerveau.decider(d, etat);
    },
  };
}

async function lancer({ graine, boites }) {
  document.body.dataset.phase = "chargement";
  etatDuChargement(MOT.waking);

  const pont = await ouvrirPontDepuis(".");
  document.getElementById("chargement")?.remove();
  document.body.dataset.phase = "partie";
  oublier();
  oublierRefs();
  oublierPlateaux();
  oublierMains();
  oublierPhases();
  dejaVu = { manche: null, phases: null };

  const partie = creerPartie(pont, { graine, boites });

  // Un fournisseur par siège, posé à sa place : le siège regardé reçoit celui
  // que `?decide=` désigne, l'autre reçoit toujours le programme adverse. Rien
  // d'autre dans la page ne dépend de « qui est le joueur 0 ».
  const fournisseurs = [];
  fournisseurs[cadre.siege] =
    cadre.decide === "programme" ? siegeProgramme(graine) : siegeHumain();
  fournisseurs[1 - cadre.siege] = adversaire(graine);

  await jouerJusquAuBout(partie, fournisseurs);

  viderScene();
  adversaireAgit(null);
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
    <p class="entree__sous">${MOT.subtitle}</p>
    <div class="entree__reglages">
      <label>${MOT.seed} <input id="entree-graine" type="number" value="7"></label>
      <label>${MOT.boxes}
        <select id="entree-boites">
          <option value="base">base</option>
          <option value="base,decouverte" selected>base + Discovery</option>
        </select>
      </label>
    </div>
    <button id="entree-go" type="button">${MOT.start}</button>`;
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

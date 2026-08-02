// LE CADRE DE JEU — ma main en bas, celle de l'adversaire en haut, retournée.
//
// C'est le seul point de vue que l'écran connaît : celui du SIÈGE regardé
// (`?siege=`). Ce siège n'est pas « le joueur qui décide » — un programme peut
// le tenir pendant qu'on le regarde jouer (`?decide=programme`).
//
// CE QUI EST CACHÉ N'EST PAS DANS LA PAGE. La zone adverse ne reçoit aucun nom
// de carte, aucun identifiant, aucune image de face : uniquement des dos et un
// NOMBRE. Rendre une carte transparente ou la pousser hors de l'écran ne serait
// pas la cacher — il suffirait d'ouvrir les outils du navigateur pour la lire.
//
// CE QUI EST JOUABLE. Une carte de ma main porte `data-jouable="oui"` si et
// seulement si le moteur vient de l'énumérer parmi les options de la décision en
// cours ET que cette décision est celle de mon siège. Recopie d'identifiants,
// pas jugement : la page ne sait pas ce que coûte une carte.

import { carte } from "./cartes.js";
import { dosDeCarte, nomJoueur } from "./materiel.js";
import { survolable } from "./loupe.js";
import { ref } from "./ecrire.js";
import { MOT } from "./mots.js";

// La largeur d'une carte en main. Elle est TENUE : sous 90 px le coût imprimé
// n'est plus lisible (mesuré sur les images du jeu). Quand la main s'allonge,
// les cartes se recouvrent — elles ne rétrécissent pas.
const LARGEUR = 116;
const ECART = 10;
// Deux cartes voisines ne se recouvrent jamais au point de masquer la bande
// gauche de la carte, qui porte les badges et le prix.
const SERRAGE_MAX = 0.62;

// Les dos de l'adversaire : plus petits, ce sont des objets sans rien à lire.
const LARGEUR_DOS = 62;
const ECART_DOS = 6;

export function construireMains() {
  const adverse = document.createElement("aside");
  adverse.className = "main main--adverse";
  adverse.id = "main-adverse";
  adverse.dataset.main = "adverse";
  adverse.dataset.cartes = "0";
  adverse.innerHTML =
    `<div class="main__tete">` +
    `<span class="main__mot" id="adverse-mot"></span>` +
    `<span class="main__agit" id="adverse-agit"></span>` +
    `</div>` +
    `<div class="main__rang" id="adverse-rang"></div>`;
  document.body.appendChild(adverse);

  const mienne = document.createElement("aside");
  mienne.className = "main main--mienne";
  mienne.id = "main-mienne";
  mienne.dataset.main = "mienne";
  mienne.innerHTML =
    `<div class="main__tete"><span class="main__mot" id="mienne-mot"></span></div>` +
    `<div class="main__rang" id="mienne-rang"></div>`;
  document.body.appendChild(mienne);
}

/**
 * Réécrit les deux mains DEPUIS LE SIÈGE REGARDÉ.
 *
 * @param {object} etat      l'état rendu par le moteur
 * @param {object} decision  la décision en cours (`null` en fin de partie)
 * @param {number} siege     le joueur assis en bas de l'écran
 */
export function majMains(etat, decision, siege) {
  const moi = etat.players[siege];
  const lui = etat.players[1 - siege];
  if (!moi || !lui) return;

  // Les identifiants que le moteur vient d'énumérer POUR MON SIÈGE : une
  // décision de l'adversaire ne rend rien de ma main jouable.
  const active = !!decision && decision.joueur === siege;
  const proposees = new Set();
  if (active) {
    for (const o of decision.options || []) {
      const c = o && o.carte ? o.carte : o;
      if (c && c.id !== undefined && c.id !== null) proposees.add(String(c.id));
    }
  }

  maMain(siege, cartesEnMain(moi, decision, active), proposees, active);
  mainAdverse(1 - siege, lui.hand.length);
}

// Les cartes Corporation que le siège tient encore. Elles sont RETENUES parce
// qu'elles ne passent qu'une fois : le descripteur de MA décision me les donne,
// puis le moteur interroge l'adversaire — et pendant ce temps ma main ne doit
// pas se vider sous mes yeux. Elles s'effacent quand j'en ai choisi une.
let corposEnMain = [];

/**
 * CE QUE JE TIENS RÉELLEMENT. L'état rend `hand` — les cartes projet. Mais à la
 * mise en place, les deux cartes Corporation distribuées sont, elles aussi, dans
 * ma main, et l'état ne les porte pas : seul le descripteur de la décision les
 * donne (`corporations`, puis les options de `pick_corporation`). On les prend
 * donc là — et seulement quand la décision est la MIENNE, sans quoi ce seraient
 * les corporations de l'adversaire qu'on afficherait.
 */
function cartesEnMain(p, decision, active) {
  if (active) {
    const offertes =
      decision.type === "corp_mulligan" ? decision.corporations || []
        : decision.type === "pick_corporation" ? decision.options || []
          : null;
    // `null` = cette décision ne parle pas de corporations ; on garde celles
    // qu'on connaît. Une liste, même vide, les remplace.
    if (offertes) {
      corposEnMain = offertes.filter((c) => c && c.id !== undefined && c.id !== null);
    }
  }
  // Le moteur dit lui-même quand elles quittent la main : dès que la
  // corporation est en jeu, il n'y a plus rien à tenir.
  if (p.corporation) corposEnMain = [];

  const cartes = [...p.hand];
  const vues = new Set(cartes.map((c) => String(c.id)));
  for (const c of corposEnMain) {
    if (!vues.has(String(c.id))) {
      vues.add(String(c.id));
      cartes.push(c);
    }
  }
  return cartes;
}

/** Ma main, en bas, en clair. */
function maMain(j, cartes, proposees, active) {
  const z = ref("#mienne-rang");
  if (!z) return;
  ref("#mienne-mot").textContent = `${MOT.hand} · ${nomJoueur(j)} · ${cartes.length}`;

  const signature =
    j + "#" + cartes.map((c) => c.id).join("|") + "#" + [...proposees].sort().join(",") +
    "#" + (active ? "1" : "0");
  if (z.dataset.signature !== signature) {
    z.dataset.signature = signature;
    z.textContent = "";
    for (const c of cartes) {
      const f = carte(c, { classe: "carte--main" });
      f.dataset.carteId = String(c.id);
      if (active) f.dataset.jouable = proposees.has(String(c.id)) ? "oui" : "non";
      survolable(f, c);
      z.appendChild(f);
    }
  }
  serrer(z, cartes.length, LARGEUR, ECART, SERRAGE_MAX);
}

/**
 * La main de l'adversaire, en haut, retournée. La seule chose qui en sorte est
 * son NOMBRE de cartes — la seule information publique d'une main tenue.
 */
function mainAdverse(j, combien) {
  const zone = ref("#main-adverse");
  const z = ref("#adverse-rang");
  if (!zone || !z) return;
  zone.dataset.cartes = String(combien);
  ref("#adverse-mot").textContent =
    `${MOT.opponent} · ${nomJoueur(j)} · ${combien} ${combien === 1 ? MOT.oneCard : MOT.manyCards}`;

  if (z.dataset.combien !== String(combien)) {
    z.dataset.combien = String(combien);
    z.textContent = "";
    for (let i = 0; i < combien; i++) {
      const f = document.createElement("figure");
      f.className = "carte carte--dos carte--adverse";
      const im = document.createElement("img");
      im.src = dosDeCarte();
      // Aucun nom ici : le texte de remplacement d'un dos ne dit que « dos ».
      im.alt = MOT.faceDown;
      im.draggable = false;
      f.appendChild(im);
      z.appendChild(f);
    }
  }
  serrer(z, combien, LARGEUR_DOS, ECART_DOS, SERRAGE_MAX);
}

/**
 * LES CARTES SE RECOUVRENT, ELLES NE RÉTRÉCISSENT PAS. Une main de quinze
 * cartes ne tient pas côte à côte dans la largeur de l'écran ; les réduire
 * rendrait le prix illisible, donc le jeu injouable. On calcule le recouvrement
 * nécessaire, borné pour que la bande gauche de chaque carte reste découverte.
 */
function serrer(z, n, largeur, ecart, maximum) {
  if (n <= 1) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const dispo = z.clientWidth || 1200;
  const naturel = n * largeur + (n - 1) * ecart;
  if (naturel <= dispo) {
    z.style.setProperty("--serrage", ecart + "px");
    return;
  }
  const chevauchement = Math.min(largeur * maximum, (naturel - dispo) / (n - 1) + ecart);
  z.style.setProperty("--serrage", Math.round(-chevauchement) + "px");
}

/**
 * Le recouvrement est calculé en PIXELS, à partir de la largeur disponible : il
 * ne veut plus rien dire dès que la fenêtre change de taille. On le reprend
 * alors, sans réécrire les cartes.
 */
export function replacerMains() {
  const m = ref("#mienne-rang");
  if (m) serrer(m, m.childElementCount, LARGEUR, ECART, SERRAGE_MAX);
  const a = ref("#adverse-rang");
  if (a) serrer(a, a.childElementCount, LARGEUR_DOS, ECART_DOS, SERRAGE_MAX);
}

/**
 * L'ADVERSAIRE AGIT — on voit QU'IL agit, jamais QUOI.
 *
 * @param {string|null} quoi  ce qu'il est en train de faire, en anglais court ;
 *                            `null` éteint l'état.
 */
export function adversaireAgit(quoi) {
  const zone = ref("#main-adverse");
  const mot = ref("#adverse-agit");
  if (!zone || !mot) return;
  if (quoi) {
    zone.dataset.agit = "oui";
    mot.textContent = quoi;
  } else {
    delete zone.dataset.agit;
    mot.textContent = "";
  }
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublierMains() {
  corposEnMain = [];
  for (const s of ["#mienne-rang", "#adverse-rang"]) {
    const z = ref(s);
    if (!z) continue;
    delete z.dataset.signature;
    delete z.dataset.combien;
    z.textContent = "";
  }
  adversaireAgit(null);
}

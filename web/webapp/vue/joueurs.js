// LES DEUX ÉQUIPAGES — une barre par joueur, collée à son plateau.
//
// Tout ce qui s'affiche ici vient de `etat.players[j]` et porte son chemin exact
// dans `data-valeur`. Rien n'est calculé : la barre lit, elle ne compte pas.
//
// La place appartient désormais au plateau de jeu : ces barres sont donc
// compactes et horizontales, l'une sous le plateau du joueur d'en face, l'autre
// au-dessus du sien. Les cartes en jeu, elles, ne sont plus ici — elles sont
// posées sur le plateau (`plateau.js`).

import {
  imageEquipage, imageReserve, imageBadge, nomBadge, ORDRE_BADGES,
  imageForet, EQUIPAGES, nomJoueur,
} from "./materiel.js";
import { carte } from "./cartes.js";
import { survolable } from "./loupe.js";
import { ref, poser, poserValeur } from "./ecrire.js";
import { MOT } from "./mots.js";

const RESERVES = [
  ["mc", MOT.mc],
  ["heat", MOT.heat],
  ["plants", MOT.plants],
];

const PRODUCTIONS = [
  ["mc", MOT.mc],
  ["heat", MOT.heat],
  ["plants", MOT.plants],
  ["cards", MOT.cards],
];

/** Construit les deux barres. Appelé une fois par partie. */
export function construireJoueurs() {
  for (const j of [0, 1]) {
    const a = document.createElement("aside");
    a.className = "equipage";
    a.id = "equipage-" + j;
    a.dataset.joueur = String(j);
    a.style.setProperty("--teinte", EQUIPAGES[j].teinte);

    a.innerHTML = `
      <div class="equipage__rang" id="rang-${j}">
      <div class="equipage__tete">
        <img class="equipage__suit" src="${imageEquipage(j)}" alt="crew ${EQUIPAGES[j].nom}">
        <span class="equipage__jn">${nomJoueur(j)}</span>
      </div>
      <div class="equipage__corpo-carte" id="corpo-carte-${j}"></div>

      <div class="jauge jauge--tr">
        <span class="jauge__mot">${MOT.tr}</span>
        <b class="jauge__n" data-valeur="players.${j}.tr">0</b>
      </div>

      <div class="reserves" id="reserves-${j}"></div>

      <div class="prod">
        <span class="prod__mot">${MOT.production}</span>
        <div class="prod__cases" id="prod-${j}"></div>
      </div>

      <div class="capacites">
        <span class="cap"><i>${MOT.steel}</i><b data-valeur="players.${j}.steel_capacity">0</b></span>
        <span class="cap"><i>${MOT.titanium}</i><b data-valeur="players.${j}.titanium_capacity">0</b></span>
        <span class="cap cap--foret"><img src="${imageForet()}" alt="forests">
          <b data-valeur="players.${j}.forests">0</b></span>
        <span class="cap"><i>stage</i><b data-valeur="players.${j}.chosen_phase">0</b></span>
      </div>

      <div class="badges" id="badges-${j}"></div>

      <div class="jauge jauge--vp" data-role="vp">
        <span class="jauge__mot">${MOT.vp}</span>
        <b class="jauge__n" data-valeur="players.${j}.score">0</b>
      </div>
      </div>`;

    document.body.appendChild(a);

    const zr = a.querySelector("#reserves-" + j);
    for (const [cle, mot] of RESERVES) {
      const d = document.createElement("div");
      d.className = "reserve reserve--" + cle;
      const im = imageReserve(cle);
      d.innerHTML =
        `<img class="reserve__bac" src="${im}" alt="${mot} store">` +
        `<b class="reserve__n" data-valeur="players.${j}.${cle}">0</b>` +
        `<span class="reserve__mot">${mot}</span>`;
      zr.appendChild(d);
    }

    const zp = a.querySelector("#prod-" + j);
    for (const [cle, mot] of PRODUCTIONS) {
      const d = document.createElement("div");
      d.className = "prod__case";
      d.innerHTML =
        `<b data-valeur="players.${j}.production.${cle}">0</b><i>${mot}</i>`;
      zp.appendChild(d);
    }

    const zb = a.querySelector("#badges-" + j);
    for (const cle of ORDRE_BADGES) {
      const d = document.createElement("div");
      d.className = "badge";
      d.dataset.badge = cle;
      d.title = nomBadge(cle);
      d.innerHTML =
        `<img src="${imageBadge(cle)}" alt="${nomBadge(cle)}">` +
        `<b data-valeur="players.${j}.tags.${cle}">0</b>`;
      zb.appendChild(d);
    }
  }
}

/**
 * LA BARRE SE MET À L'ÉCHELLE, ELLE NE SE COUPE PAS. Sur un écran étroit, la
 * ligne d'un joueur (réserves, production, capacités, badges, VP) ne tient plus
 * dans la largeur. Plutôt que d'en rogner la fin — les points de victoire
 * seraient les premiers perdus — on la réduit, comme on réduit le plateau.
 * Tout reste affiché, plus petit.
 */
export function replacerBarres() {
  for (const j of [0, 1]) {
    const rang = ref("#rang-" + j);
    if (!rang || !rang.parentElement) continue;
    const hote = rang.parentElement;
    const l = rang.scrollWidth;
    const h = rang.scrollHeight;
    if (!l || !h) continue;
    const s = Math.min(1, hote.clientWidth / l, hote.clientHeight / h);
    rang.style.setProperty("--echelle", Math.max(0.3, s).toFixed(4));
  }
}

/** Réécrit les deux barres à partir de l'état. */
export function majJoueurs(etat, decision) {
  for (const p of etat.players) {
    const j = p.player;
    const a = ref("#equipage-" + j);
    if (!a) continue;
    a.classList.toggle("equipage--actif", !!decision && decision.joueur === j);

    poserValeur(`players.${j}.tr`, p.tr);
    poserValeur(`players.${j}.score`, p.score);
    poserValeur(`players.${j}.forests`, p.forests);
    poserValeur(`players.${j}.steel_capacity`, p.steel_capacity);
    poserValeur(`players.${j}.titanium_capacity`, p.titanium_capacity);
    poserValeur(`players.${j}.chosen_phase`, p.chosen_phase || 0);
    for (const [cle] of RESERVES) poserValeur(`players.${j}.${cle}`, p[cle]);
    for (const [cle] of PRODUCTIONS) {
      const e = ref(`[data-valeur="players.${j}.production.${cle}"]`);
      if (e) {
        poser(e, p.production[cle]);
        e.parentElement.classList.toggle("prod__case--vide", p.production[cle] === 0);
      }
    }

    // La corporation est montrée par son SCAN, jamais par son nom écrit : six
    // cartes du jeu s'appellent « … Corporation », et l'écran est en anglais.
    const z = ref("#corpo-carte-" + j);
    if (z.dataset.corpo !== (p.corporation || "")) {
      z.dataset.corpo = p.corporation || "";
      z.textContent = "";
      if (p.corporation) {
        const f = carte({ nom: p.corporation }, { classe: "carte--corpo" });
        survolable(f, { nom: p.corporation });
        z.appendChild(f);
      }
    }

    // Les familles de badges sont celles que l'état porte, pas une liste recopiée.
    for (const cle of Object.keys(p.tags)) {
      const e = ref(`[data-valeur="players.${j}.tags.${cle}"]`);
      if (!e) continue;
      const n = p.tags[cle];
      poser(e, n);
      e.parentElement.classList.toggle("badge--vide", n === 0);
    }
  }
  // Les nombres grossissent en cours de partie (3 MC devient 104 MC) : la
  // largeur de la barre change avec eux, donc l'échelle se reprend à chaque fois.
  replacerBarres();
}

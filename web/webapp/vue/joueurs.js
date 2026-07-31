// LES DEUX ÉQUIPAGES — une colonne par joueur, de part et d'autre du monde.
//
// Tout ce qui s'affiche ici vient de `etat.players[j]` et porte son chemin exact
// dans `data-valeur`. Les réserves (MC, chaleur, plantes) sont posées dans les
// zones de stockage réellement imprimées sur le plateau joueur : le nombre n'est
// pas dans une case d'un tableau, il est dans son bac.

import {
  imageEquipage, imageReserve, imageBadge, nomBadge, ORDRE_BADGES,
  imageForet, imagePhase, imageAmelioration, phaseNom, phaseRomain, EQUIPAGES,
} from "./materiel.js";
import { carte } from "./cartes.js";
import { survolable } from "./loupe.js";
import { ref, poser, poserValeur } from "./ecrire.js";

const RESERVES = [
  ["mc", "MC"],
  ["heat", "Chaleur"],
  ["plants", "Plantes"],
];

const PRODUCTIONS = [
  ["mc", "MC"],
  ["heat", "chaleur"],
  ["plants", "plantes"],
  ["cards", "cartes"],
];

/** Construit les deux colonnes. Appelé une fois par partie. */
export function construireJoueurs() {
  for (const j of [0, 1]) {
    const a = document.createElement("aside");
    a.className = "equipage";
    a.id = "equipage-" + j;
    a.dataset.joueur = String(j);
    a.style.setProperty("--teinte", EQUIPAGES[j].teinte);

    a.innerHTML = `
      <div class="equipage__tete">
        <img class="equipage__suit" src="${imageEquipage(j)}" alt="équipage ${EQUIPAGES[j].nom}">
        <div class="equipage__id">
          <span class="equipage__jn">J${j}</span>
          <span class="equipage__corpo" id="corpo-nom-${j}">—</span>
        </div>
      </div>
      <div class="equipage__corpo-carte" id="corpo-carte-${j}"></div>

      <div class="tr">
        <span class="tr__mot">Terraformation</span>
        <b class="tr__n" data-valeur="players.${j}.tr">0</b>
      </div>

      <div class="reserves" id="reserves-${j}"></div>

      <div class="prod">
        <span class="prod__mot">Production</span>
        <div class="prod__cases" id="prod-${j}"></div>
      </div>

      <div class="capacites">
        <span class="cap"><i>acier</i><b data-valeur="players.${j}.steel_capacity">0</b></span>
        <span class="cap"><i>titane</i><b data-valeur="players.${j}.titanium_capacity">0</b></span>
        <span class="cap cap--foret"><img src="${imageForet()}" alt="forêts">
          <b data-valeur="players.${j}.forests">0</b></span>
      </div>

      <div class="badges" id="badges-${j}"></div>

      <div class="phases">
        <div class="phases__courante" id="phase-courante-${j}"></div>
        <div class="phases__ameliorees" id="phase-up-${j}"></div>
      </div>

      <div class="posees">
        <span class="posees__mot">En jeu</span>
        <div class="posees__pile" id="posees-${j}"></div>
      </div>

      <div class="score">
        <span class="score__mot">Score</span>
        <b class="score__n" data-valeur="players.${j}.score">0</b>
      </div>`;

    document.body.appendChild(a);

    const zr = a.querySelector("#reserves-" + j);
    for (const [cle, mot] of RESERVES) {
      const d = document.createElement("div");
      d.className = "reserve reserve--" + cle;
      const im = imageReserve(cle);
      d.innerHTML =
        `<img class="reserve__bac" src="${im}" alt="réserve ${mot}">` +
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

/** Réécrit les deux colonnes à partir de l'état. */
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
    for (const [cle] of RESERVES) poserValeur(`players.${j}.${cle}`, p[cle]);
    for (const [cle] of PRODUCTIONS) {
      const e = ref(`[data-valeur="players.${j}.production.${cle}"]`);
      if (e) {
        poser(e, p.production[cle]);
        e.parentElement.classList.toggle("prod__case--vide", p.production[cle] === 0);
      }
    }

    const nom = ref("#corpo-nom-" + j);
    if (nom.textContent !== (p.corporation || "—")) {
      nom.textContent = p.corporation || "—";
      const z = ref("#corpo-carte-" + j);
      z.textContent = "";
      if (p.corporation) z.appendChild(carte({ nom: p.corporation }, { classe: "carte--corpo" }));
    }

    // Les familles de badges sont celles que l'état porte, pas une liste recopiée.
    for (const cle of Object.keys(p.tags)) {
      const e = ref(`[data-valeur="players.${j}.tags.${cle}"]`);
      if (!e) continue;
      const n = p.tags[cle];
      poser(e, n);
      e.parentElement.classList.toggle("badge--vide", n === 0);
    }

    phases(j, p);
    posees(j, p);
  }
}

/** La carte Phase du joueur, et ses améliorations acquises. */
function phases(j, p) {
  const z = ref("#phase-courante-" + j);
  const choisie = p.chosen_phase || 0;
  if (z.dataset.phase !== String(choisie)) {
    z.dataset.phase = String(choisie);
    z.textContent = "";
    if (choisie) {
      const im = document.createElement("img");
      im.src = imagePhase(choisie);
      im.alt = "carte Phase " + phaseNom(choisie);
      z.appendChild(im);
      const t = document.createElement("span");
      t.className = "phases__nom";
      t.textContent = `${phaseRomain(choisie)} · ${phaseNom(choisie)}`;
      z.appendChild(t);
    }
  }

  const zu = ref("#phase-up-" + j);
  const codes = (p.phase_upgrades || []).join(",");
  if (zu.dataset.codes !== codes) {
    zu.dataset.codes = codes;
    zu.textContent = "";
    for (const c of p.phase_upgrades || []) {
      const src = imageAmelioration(c);
      if (!src) continue;
      const im = document.createElement("img");
      im.src = src;
      im.alt = "phase améliorée " + c;
      im.title = "Phase améliorée " + c;
      zu.appendChild(im);
    }
  }
}

/** Les cartes déjà posées, en pile serrée : on voit la fortune s'empiler. */
function posees(j, p) {
  const z = ref("#posees-" + j);
  const signature = p.played.map((c) => `${c.name}:${c.resources ?? 0}`).join("|");
  if (z.dataset.signature === signature) return;
  z.dataset.signature = signature;
  z.textContent = "";
  p.played.forEach((c, k) => {
    const f = carte(c, {
      classe: "carte--posee",
      chemin: `players.${j}.played.${k}.resources`,
    });
    survolable(f, c);
    z.appendChild(f);
  });
}

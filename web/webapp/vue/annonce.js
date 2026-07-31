// LE THÉÂTRE — les moments qui font qu'une manche a un début et une fin.
//
// RÈGLE ABSOLUE : rien de ce qui est annoncé ne bloque le jeu, et aucun voile ne
// reçoit le clic (`pointer-events: none`). La page est jouable pendant que
// l'annonce s'efface. Un humain met plus d'une demi-seconde à décider : il voit
// tout. Une machine qui clique tout de suite n'est jamais gênée.

import { imagePhase, phaseNom, phaseRomain, EQUIPAGES, imageCarte } from "./materiel.js";

export function construireAnnonce() {
  const d = document.createElement("div");
  d.id = "annonce";
  document.body.appendChild(d);
}

let minuteur = null;

function jouerAnnonce(noeud, ms) {
  const z = document.getElementById("annonce");
  if (!z) return;
  z.textContent = "";
  z.appendChild(noeud);
  z.classList.remove("annonce--vive");
  void z.offsetWidth;
  z.classList.add("annonce--vive");
  clearTimeout(minuteur);
  minuteur = setTimeout(() => z.classList.remove("annonce--vive"), ms);
}

/** Le tour de manche : un chiffre qui traverse l'écran. */
export function annonceManche(n) {
  const d = document.createElement("div");
  d.className = "annonce__manche";
  // Ce chiffre est `generation`, rendu par le moteur : il déclare son chemin
  // comme tous les autres nombres de l'écran.
  d.innerHTML = `<span>manche</span><b data-valeur="generation">${n}</b>`;
  jouerAnnonce(d, 1100);
}

/**
 * Le moment où l'on retourne sa carte Phase : les deux choix se révèlent
 * côte à côte, en grand, avec la couleur de chaque équipage.
 */
export function annoncePhases(etat) {
  const d = document.createElement("div");
  d.className = "annonce__phases";
  for (const p of etat.players) {
    const n = p.chosen_phase;
    if (!n) continue;
    const c = document.createElement("div");
    c.className = "annonce__phase";
    c.style.setProperty("--teinte", EQUIPAGES[p.player].teinte);
    c.innerHTML =
      `<img src="${imagePhase(n)}" alt="carte Phase ${phaseNom(n)}">` +
      `<span class="annonce__phase-qui">J${p.player}</span>` +
      `<span class="annonce__phase-nom">${phaseRomain(n)} · ${phaseNom(n)}</span>`;
    d.appendChild(c);
  }
  if (!d.childElementCount) return;
  jouerAnnonce(d, 1400);
}

/**
 * L'écran final. Les deux scores viennent du moteur (`partie.scores`), et
 * l'élément qui les porte ne contient qu'eux — rien d'autre à lire dedans.
 */
export function ecranFinal(etat) {
  const f = document.createElement("section");
  f.id = "final";
  f.dataset.partieTerminee = "";

  // Les scores affichés sont ceux de l'état rendu par le moteur — exactement ce
  // que `data-valeur="players.j.score"` désigne. `partie.scores` dit la même
  // chose ; on n'affiche qu'une seule de ces deux sources, pour qu'aucun nombre
  // à l'écran ne puisse s'écarter du chemin qu'il déclare.
  //
  // La page ne DÉSIGNE AUCUN VAINQUEUR : le moteur n'en rend pas, et « le plus
  // grand score l'emporte » (avec son départage) est une règle du jeu. Les deux
  // totaux sont posés côte à côte, en grand ; les joueurs lisent eux-mêmes.
  const colonnes = etat.players
    .map((p) => {
      const j = p.player;
      const im = p.corporation ? imageCarte(p.corporation) : null;
      return `
      <div class="final__colonne" style="--teinte:${EQUIPAGES[j].teinte}">
        ${im ? `<img class="final__corpo" src="${im}" alt="${p.corporation}">` : ""}
        <span class="final__qui">J${j} · ${EQUIPAGES[j].nom}</span>
        <span class="final__corpo-nom">${p.corporation || ""}</span>
        <b class="final__score" data-score-final="${j}" data-valeur="players.${j}.score"
           >${p.score}</b>
        <span class="final__mot">points</span>
        <span class="final__detail">terraformation
          <i data-valeur="players.${j}.tr">${p.tr}</i> ·
          forêts <i data-valeur="players.${j}.forests">${p.forests}</i></span>
      </div>`;
    })
    .join("");

  f.innerHTML = `
    <div class="final__titre">
      <span>Mars est terraformée</span>
      <b>Décompte final</b>
    </div>
    <div class="final__colonnes">${colonnes}</div>
    <div class="final__planete">
      <span>température <i data-valeur="planet.temperature">${etat.planet.temperature}</i>
        /<i data-valeur="planet.temperature_max">${etat.planet.temperature_max}</i></span>
      <span>oxygène <i data-valeur="planet.oxygen">${etat.planet.oxygen}</i>
        /<i data-valeur="planet.oxygen_max">${etat.planet.oxygen_max}</i></span>
      <span>océans <i data-valeur="planet.oceans">${etat.planet.oceans}</i>
        /<i data-valeur="planet.oceans_max">${etat.planet.oceans_max}</i></span>
      <span>manche <i data-valeur="generation">${etat.generation}</i></span>
    </div>`;

  document.body.appendChild(f);
  requestAnimationFrame(() => f.classList.add("final--vif"));
  return f;
}

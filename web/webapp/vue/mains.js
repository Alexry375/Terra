// LES DEUX MAINS — visibles en permanence, l'une en haut, l'autre en bas.
//
// C'est un bac à sable : personne ne cache rien. Les deux mains bordent l'écran
// comme deux joueurs assis de part et d'autre d'une table. Elles ne sont jamais
// cliquables — une carte se joue en répondant à la décision, pas en la prenant
// dans la main. Le chevauchement n'est donc gênant pour personne.

import { carte } from "./cartes.js";
import { EQUIPAGES } from "./materiel.js";
import { survolable } from "./loupe.js";
import { ref } from "./ecrire.js";

export function construireMains() {
  for (const j of [0, 1]) {
    const d = document.createElement("div");
    d.className = "main";
    d.id = "main-" + j;
    d.dataset.joueur = String(j);
    d.style.setProperty("--teinte", EQUIPAGES[j].teinte);
    d.innerHTML =
      `<span class="main__etiquette">main de J${j}</span>` +
      `<div class="main__eventail" id="eventail-${j}"></div>`;
    document.body.appendChild(d);
  }
}

export function majMains(etat, decision) {
  for (const p of etat.players) {
    const j = p.player;
    const z = ref("#eventail-" + j);
    if (!z) continue;

    const signature = p.hand.map((c) => c.name).join("|");
    if (z.dataset.signature !== signature) {
      z.dataset.signature = signature;
      z.textContent = "";
      for (const c of p.hand) {
        const f = carte(c, { classe: "carte--main" });
        survolable(f, c);
        z.appendChild(f);
      }
      // L'éventail se resserre quand la main grossit : elle reste dans son bord.
      const n = Math.max(p.hand.length, 1);
      const pas = Math.min(88, Math.max(28, Math.floor(980 / n)));
      z.style.setProperty("--pas", pas + "px");
    }
    ref("#main-" + j).classList.toggle(
      "main--active", !!decision && decision.joueur === j);
  }
}

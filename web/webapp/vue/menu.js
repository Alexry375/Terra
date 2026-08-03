// L'ÉCRAN D'ACCUEIL — la première chose qu'on voit du jeu.
//
// Ce qu'il remplace : un titre, deux champs et un bouton posés sur du noir. Le
// joueur l'a dit deux fois — « le menu est toujours moche ». Ce n'est pas une
// affaire de couleurs : un écran d'accueil de jeu montre LE JEU. Celui-ci pose
// donc la couverture de la boîte sur le sol martien de la table, et range les
// deux réglages (la graine, les boîtes) comme les cadrans d'une console, non
// comme un formulaire.
//
// UNE RÈGLE DE FER TIENT TOUTE LA MISE EN PAGE : aucun élément qu'on lit ou
// qu'on clique n'en recouvre un autre, à aucune des six tailles de fenêtre, et
// rien ne déborde. C'est mesuré (`checks/01`), pas jugé à l'œil. D'où une
// composition en colonnes de flux, sans le moindre élément posé « par-dessus » :
// tout ce qui décore est un fond, une ombre ou un pseudo-élément, jamais une
// boîte de plus.

import { MOT } from "./mots.js";

const COUVERTURE = "./assets/menu/couverture-boite-ares-expedition.jpeg";

// Les deux réglages que l'ancien écran offrait déjà, et qu'on garde : ils sont
// utiles, et l'entrée par l'adresse (`?graine=…&boites=…`) parle la même langue.
const BOITES = [
  { valeur: "base", mot: MOT.menuBoxBase },
  { valeur: "base,decouverte", mot: MOT.menuBoxAll },
];

let ecran = null; // l'écran, bâti une seule fois puis montré / caché
let demarrerLaPartie = null; // ce que l'on fait quand on clique « Start »

function batirAccueil() {
  const z = document.createElement("section");
  z.id = "accueil";
  z.dataset.accueil = "";

  const affiche = document.createElement("div");
  affiche.className = "accueil__affiche";

  // La couverture de la boîte. IMAGE SOUS DROIT D'AUTEUR (FryxGames /
  // Stronghold Games), présente dans le dépôt pour un usage strictement privé :
  // elle est affichée, jamais redistribuée ni détournée.
  const im = document.createElement("img");
  im.className = "accueil__couverture";
  im.src = COUVERTURE;
  im.alt = MOT.menuCoverAlt;
  im.draggable = false;
  affiche.appendChild(im);

  const colonne = document.createElement("div");
  colonne.className = "accueil__colonne";

  const titre = document.createElement("h1");
  titre.className = "accueil__titre";
  titre.dataset.accueilTitre = "";
  titre.textContent = "Terra";
  colonne.appendChild(titre);

  const sous = document.createElement("p");
  sous.className = "accueil__sous";
  sous.textContent = MOT.subtitle;
  colonne.appendChild(sous);

  const reglages = document.createElement("div");
  reglages.className = "accueil__reglages";

  // Chaque réglage est un `<label>` qui porte SON texte en propre (un nœud de
  // texte, pas un élément) : le champ reste le seul objet mesurable de la case,
  // et rien ne peut se poser sur rien.
  const champGraine = document.createElement("label");
  champGraine.className = "accueil__champ";
  champGraine.append(MOT.seed);
  const graine = document.createElement("input");
  graine.id = "accueil-graine";
  graine.dataset.accueilGraine = "";
  graine.type = "number";
  graine.min = "0";
  graine.step = "1";
  graine.value = "7";
  champGraine.appendChild(graine);
  reglages.appendChild(champGraine);

  const champBoites = document.createElement("label");
  champBoites.className = "accueil__champ";
  champBoites.append(MOT.boxes);
  const boites = document.createElement("select");
  boites.id = "accueil-boites";
  boites.dataset.accueilBoites = "";
  for (const b of BOITES) {
    const o = document.createElement("option");
    o.value = b.valeur;
    o.textContent = b.mot;
    if (b.valeur === "base,decouverte") o.selected = true;
    boites.appendChild(o);
  }
  champBoites.appendChild(boites);
  reglages.appendChild(champBoites);

  colonne.appendChild(reglages);

  const go = document.createElement("button");
  go.className = "accueil__go";
  go.id = "accueil-go";
  go.dataset.accueilCommencer = "";
  go.type = "button";
  // Le libellé est un nœud de texte et non un `<span>` : un élément de plus
  // dans le bouton serait un élément de plus à mesurer, exactement au même
  // endroit que le bouton — c'est-à-dire un recouvrement.
  go.textContent = MOT.start;
  go.addEventListener("click", () => {
    const g = Number.parseInt(graine.value, 10);
    demarrerLaPartie?.({
      graine: Number.isFinite(g) ? g : 1,
      boites: boites.value,
    });
  });
  colonne.appendChild(go);

  affiche.appendChild(colonne);
  z.appendChild(affiche);

  // La mention de la photographie du sol : elle est ici parce que le sol est
  // ici. Obligation de licence (`assets/plateau/CREDITS-sol-martien.md`).
  const credit = document.createElement("p");
  credit.className = "accueil__credit";
  credit.textContent = MOT.credit;
  z.appendChild(credit);

  return z;
}

/**
 * Montre l'écran d'accueil et dit ce qu'il faut faire quand on commence.
 * @param {(reglage: {graine: number, boites: string}) => void} commencer
 */
export function montrerAccueil(commencer) {
  demarrerLaPartie = commencer;
  if (!ecran) {
    ecran = batirAccueil();
    document.body.appendChild(ecran);
  }
  ecran.hidden = false;
  // Le marqueur du corps ne s'appelle SURTOUT pas `data-accueil` : cet
  // attribut-là désigne l'écran d'accueil lui-même, et le corps le précède dans
  // le document — qui cherche « l'écran d'accueil » trouverait la page entière.
  document.body.dataset.ecran = "accueil";
  // Le champ de la graine prend la main : on peut jouer au clavier seul.
  ecran.querySelector("#accueil-graine")?.focus({ preventScroll: true });
}

/** Range l'écran d'accueil. Il n'est pas détruit : on y revient. */
export function cacherAccueil() {
  if (ecran) ecran.hidden = true;
  delete document.body.dataset.ecran;
}

// LA RÉVÉLATION DU DESSUS DE LA PIOCHE — ce que le joueur DOIT voir.
//
// Le moteur retourne des cartes face visible (« Révélez les 3 premières cartes
// de la pioche »). Avant ce chantier, la page n'en montrait aucune : elle ne
// recevait que les cartes PRENABLES, et quand aucune ne l'était, elle ne
// recevait rien du tout — les trois cartes partaient à la défausse sans que
// rien ne bouge à l'écran. C'était le défaut relevé le 04-08.
//
// Ce module ne compose QUE de l'information : les cartes révélées, toutes,
// avec pour chacune le fait qu'elle soit prenable ou non. Il ne rend aucun
// élément cliquable (`data-choix` est l'affaire de la scène) et ne juge rien —
// `prenable` et `filtre` sont écrits par le moteur, la page les recopie.
//
// AUCUNE FUITE : les seules cartes qui entrent ici sont celles que le moteur a
// posées dans `decision.revelees`, c'est-à-dire celles que la règle du jeu
// retourne face visible sur la table. Le reste de la pioche n'est pas nommé.

import { carte } from "./cartes.js";
import { survolable } from "./loupe.js";
import { MOT, regleRevelation, etatRevelee } from "./mots.js";

// Les images de cartes, telles qu'elles ont été découpées (même valeur que
// `vue/scene.js`) : une carte révélée reçoit sa HAUTEUR en pixels, et pas
// seulement sa largeur. Sans hauteur posée d'avance, la bande des cartes
// révélées mesure zéro tant que les images n'ont pas fini de charger — la
// scène calcule alors la place des choix comme si elle était vide, et les
// choix viennent se poser SUR les cartes révélées. Mesuré à l'écran.
const RATIO = 569 / 409;

/**
 * La largeur d'une carte révélée, à la place que la fenêtre offre. Quand des
 * choix sont posés en dessous, les cartes révélées se serrent : ce sont les
 * choix qu'on clique, la bande du dessus est un rappel.
 */
function largeurCarte(combien, avecChoix) {
  const dispo = (window.innerWidth || 1280) * 0.62;
  const plafond = avecChoix ? 116 : 190;
  return Math.max(72, Math.min(plafond, Math.floor(dispo / Math.max(combien, 1)) - 16));
}

/**
 * Le bandeau des cartes révélées, ou `null` si la décision n'en porte pas.
 *
 * @param {object} d le descripteur de décision du moteur
 * @returns {HTMLElement|null}
 */
export function contexteRevelation(d) {
  const revelees = d && d.revelees;
  if (!Array.isArray(revelees) || !revelees.length) return null;

  const z = document.createElement("div");
  z.className = "scene__contexte";
  z.dataset.revelation = String(revelees.length);

  const mot = document.createElement("span");
  mot.className = "scene__contexte-mot";
  const regle = regleRevelation(d.filtre);
  mot.textContent = regle ? `${MOT.revealed} — ${regle}` : MOT.revealed;
  z.appendChild(mot);

  const rang = document.createElement("div");
  rang.className = "revelation__rang";
  rang.style.cssText =
    "display:flex;justify-content:center;align-items:flex-start;gap:14px;" +
    "margin-top:6px;flex:0 0 auto;";
  const w = largeurCarte(revelees.length, ((d.options || []).length > 0));

  for (const c of revelees) {
    const prenable = c.prenable === true;
    const casier = document.createElement("div");
    casier.className = "revelation__carte";
    // Les deux attributs qui rendent la distinction LISIBLE DE L'EXTÉRIEUR :
    // un contrôle peut compter les cartes montrées et celles qu'on ne peut pas
    // prendre, sans lire une couleur ni un style.
    casier.dataset.revelee = c.id === undefined || c.id === null ? "" : String(c.id);
    casier.dataset.prenable = prenable ? "oui" : "non";
    casier.style.cssText =
      "display:flex;flex-direction:column;align-items:center;gap:4px;pointer-events:auto;";

    const f = carte(c, { classe: "carte--contexte" });
    const im = f.querySelector("img");
    if (im) {
      im.style.width = w + "px";
      im.style.height = Math.round(w * RATIO) + "px";
    }
    // Ce qui se voit D'UN REGARD : la carte qu'on ne peut pas prendre est
    // éteinte et barrée d'un cadre sourd ; celle qu'on peut prendre garde ses
    // couleurs et porte un liseré clair. Les styles sont posés ici, en ligne :
    // les feuilles de style du dépôt sont écrites par d'autres chantiers en ce
    // moment, et une règle perdue à la fusion rendrait les deux cas identiques.
    f.style.borderRadius = "8px";
    if (prenable) {
      f.style.boxShadow = "0 0 0 2px #86e08a, 0 6px 20px rgba(0,0,0,.5)";
    } else {
      f.style.filter = "grayscale(1) brightness(.55)";
      f.style.opacity = ".72";
      f.style.boxShadow = "0 0 0 2px #4b4f57";
    }
    survolable(f, c);
    casier.appendChild(f);

    const etat = document.createElement("span");
    etat.className = "revelation__etat";
    etat.style.cssText =
      "font-size:10px;letter-spacing:.14em;text-transform:uppercase;text-align:center;" +
      "max-width:" + (w + 20) + "px;color:" + (prenable ? "#86e08a" : "#8b9099") + ";";
    etat.textContent = etatRevelee(prenable);
    casier.appendChild(etat);

    rang.appendChild(casier);
  }
  z.appendChild(rang);
  return z;
}

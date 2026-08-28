// LE THÉÂTRE — les moments qui font qu'une manche a un début et une fin.
//
// RÈGLE ABSOLUE : rien de ce qui est annoncé ne bloque le jeu, et aucun voile ne
// reçoit le clic (`pointer-events: none`). La page est jouable pendant que
// l'annonce s'efface. Un humain met plus d'une demi-seconde à décider : il voit
// tout. Une machine qui clique tout de suite n'est jamais gênée.

import { imagePhase, phaseNom, phaseRomain, EQUIPAGES, imageCarte, nomJoueur } from "./materiel.js";
import { MOT } from "./mots.js";

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
  d.innerHTML = `<span>${MOT.round}</span><b data-valeur="generation">${n}</b>`;
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
      `<img src="${imagePhase(n)}" alt="Phase card ${phaseNom(n)}">` +
      `<span class="annonce__phase-qui">${nomJoueur(p.player)}</span>` +
      `<span class="annonce__phase-nom">${phaseRomain(n)} · ${phaseNom(n)}</span>`;
    d.appendChild(c);
  }
  if (!d.childElementCount) return;
  jouerAnnonce(d, 1400);
}

/**
 * (cartes-qui-bougent, ANI-3) **UNE PHASE COMMENCE.**
 *
 * « On ne sait pas qu'une phase commence — la Production en particulier. » Les
 * cinq phases de la manche se résolvaient l'une après l'autre sans que rien ne
 * le dise : les compteurs changeaient, et c'était tout. La carte Phase qui
 * s'ouvre traverse donc la bande, avec son chiffre romain et son nom.
 *
 * CE QU'ELLE NE RÉVÈLE PAS. La phase EN COURS DE RÉSOLUTION est publique — le
 * moteur l'a révélée (livret l. 272), les deux cartes sont retournées sur la
 * table. Cette annonce ne dit rien du choix SECRET de personne : elle nomme ce
 * que le moteur est en train de résoudre, et rien d'autre. C'est la carte Phase
 * IMPRIMÉE qu'elle montre, pas celle d'un joueur.
 */
export function annoncePhase(n) {
  const d = document.createElement("div");
  d.className = "annonce__debut";
  d.innerHTML =
    `<img src="${imagePhase(n)}" alt="Phase card ${phaseNom(n)}">` +
    `<span class="annonce__debut-nom">${phaseRomain(n)} · ${phaseNom(n)}</span>`;
  jouerAnnonce(d, 1300);
}

/**
 * (cartes-qui-bougent, ANI-2) **LE TOUR PASSE À L'AUTRE.**
 *
 * « On ne comprend pas que son tour est fini et que l'autre doit choisir sa
 * phase. » Une phrase, en clair, au moment où la main change de côté.
 *
 * ELLE NE LAISSE RIEN FILTRER. Elle ne nomme aucune carte, aucune phase, aucun
 * choix : elle dit qui a la parole. La carte Phase de la manche EN COURS de
 * l'adversaire ne doit apparaître nulle part tant que le moteur ne l'a pas
 * révélée, et ce point a déjà coûté deux corrections à ce dépôt.
 */
export function annoncePassage(texte) {
  const d = document.createElement("div");
  d.className = "annonce__passage";
  d.textContent = texte;
  jouerAnnonce(d, 1200);
}

/**
 * (LIS-12, 05-08) **LA DÉFAUSSE EST REVERSÉE DANS LA PIOCHE.**
 *
 * Quand la pioche est vide, le moteur y reverse la défausse et la mélange
 * (`engine/src/flow.rs:32`, livret p. 15). C'est un moment fort de la partie —
 * mesuré graine 77 : la pioche passe de 0 à 168 cartes et la défausse de 171 à
 * 2 d'un seul coup — et rien ne le montrait : deux nombres du bandeau
 * changeaient, et c'était tout.
 *
 * Le moteur ne publie AUCUN signal de remélange et ne doit pas en publier :
 * c'est `vue/anim.js` qui reconnaît l'évènement aux deux nombres déjà publiés,
 * et qui appelle cette annonce-ci en même temps qu'il fait voler les cartes de
 * la pile de défausse vers celle de la pioche.
 *
 * ELLE NE BLOQUE RIEN, comme toutes les autres : `#annonce` est en
 * `pointer-events: none` et se pose sur le bandeau, jamais sur la scène.
 *
 * LE MOT NE SURVIT PAS AU MOMENT. Les autres annonces laissent leur texte dans
 * le document une fois éteintes ; celle-ci l'efface, parce qu'elle est la seule
 * dont le texte périmé serait FAUX et pas seulement vieux : qui relit
 * `#annonce` après coup — un banc, un lecteur d'écran, une capture — y lirait
 * un remélange qui n'a plus lieu. C'est la même raison qui fait vider `#annonce`
 * au retour au menu (`vue/options.js`, `viderTable`).
 */
const TENUE_REMELANGE = 1300;

export function annonceRemelange() {
  const z = document.getElementById("annonce");
  if (!z) return;
  const d = document.createElement("div");
  d.className = "annonce__passage annonce__remelange";
  d.textContent = MOT.reshuffle;
  jouerAnnonce(d, TENUE_REMELANGE);
  setTimeout(() => {
    // Une autre annonce a pu prendre la place entre-temps : on n'efface que la
    // sienne.
    if (z.firstElementChild === d) {
      z.textContent = "";
      z.classList.remove("annonce--vive");
    }
  }, TENUE_REMELANGE);
}

/**
 * L'écran final. Les deux scores viennent du moteur (`partie.scores`), et
 * l'élément qui les porte ne contient qu'eux — rien d'autre à lire dedans.
 *
 * (les-ecrans-manquants) **IL DIT MAINTENANT QUI A GAGNÉ.** Il ne le disait pas,
 * et son commentaire d'alors en donnait la raison : « le moteur n'en rend pas ».
 * Le moteur en rend un depuis ce lot — `etat.winner`, écrit par `flow::winner`,
 * qui applique le départage du livret (chaleur + MC + plantes, cartes en main à
 * 3 MC pièce) quand les points de victoire sont à égalité.
 *
 * **AUCUNE RÈGLE N'EST REJOUÉE ICI.** La page ne compare pas deux scores, ne
 * compare pas deux totaux, ne connaît pas le barème : elle lit `etat.winner`,
 * qui vaut `0`, `1`, ou `null` quand l'égalité est parfaite jusque sur le total
 * de départage. Trois lectures, aucun calcul — c'est l'interdit dur nº 1 du lot,
 * et c'est la règle qui tient tout le projet : un seul point de calcul des
 * règles, et ce n'est jamais le navigateur.
 *
 * **LES DEUX TOTAUX DE DÉPARTAGE SONT MONTRÉS DANS TOUS LES CAS**, chacun sous
 * son chemin d'état (`players.j.tiebreak_total`), et un mot dit ce qu'ils
 * tranchent quand les points sont à égalité. Les montrer toujours n'apprend rien
 * à personne — la partie est finie, plus rien n'est caché — et c'est ce qui
 * permet aux joueurs de vérifier eux-mêmes le verdict au lieu de le croire.
 */
export function ecranFinal(etat) {
  const f = document.createElement("section");
  f.id = "final";
  f.dataset.partieTerminee = "";

  // LE VAINQUEUR, LU ET NON DÉDUIT. `undefined` (un moteur plus ancien) et
  // `null` (égalité parfaite) ne veulent pas dire la même chose : le premier
  // n'autorise à désigner personne, le second désigne une partie nulle. On ne
  // les confond pas.
  const vainqueur = etat.winner === 0 || etat.winner === 1 ? etat.winner : null;
  const nulle = etat.winner === null;
  // « Les points sont à égalité » est un FAIT publié, pas un départage : les deux
  // scores sont sur la table, côte à côte, et personne n'a besoin de la page
  // pour voir qu'ils sont égaux. Ce n'est pas ce fait qui désigne le vainqueur —
  // c'est `etat.winner`, et lui seul.
  const pointsEgaux = etat.players.length === 2
    && etat.players[0].score === etat.players[1].score;

  // Les scores affichés sont ceux de l'état rendu par le moteur — exactement ce
  // que `data-valeur="players.j.score"` désigne. `partie.scores` dit la même
  // chose ; on n'affiche qu'une seule de ces deux sources, pour qu'aucun nombre
  // à l'écran ne puisse s'écarter du chemin qu'il déclare.
  const colonnes = etat.players
    .map((p) => {
      const j = p.player;
      const im = p.corporation ? imageCarte(p.corporation) : null;
      const gagne = vainqueur === j;
      return `
      <div class="final__colonne${gagne ? " final__colonne--vainqueur" : ""}"
           style="--teinte:${EQUIPAGES[j].teinte}"${gagne ? ` data-vainqueur="oui"` : ""}>
        ${gagne ? `<span class="final__couronne">${MOT.winnerMark}</span>` : ""}
        ${im ? `<img class="final__corpo" src="${im}" alt="${p.corporation}">` : ""}
        <span class="final__qui">${nomJoueur(j)} · ${EQUIPAGES[j].nom}</span>
        <b class="final__score" data-score-final="${j}" data-valeur="players.${j}.score"
           >${p.score}</b>
        <span class="final__mot">${MOT.vp}</span>
        <span class="final__detail">${MOT.tr}
          <i data-valeur="players.${j}.tr">${p.tr}</i> ·
          ${MOT.forests} <i data-valeur="players.${j}.forests">${p.forests}</i></span>
        <span class="final__detail final__departage">${MOT.tiebreak}
          <i data-valeur="players.${j}.tiebreak_total">${p.tiebreak_total}</i></span>
      </div>`;
    })
    .join("");

  // LE MOT QUI DIT POURQUOI. Il vit HORS des deux colonnes : il parle de la
  // partie, pas d'un joueur, et rien de ce qui distingue le gagnant du perdant
  // ne doit dépendre de lui.
  const verdict = nulle
    ? `<div class="final__verdict">${MOT.drawn}</div>`
    : (pointsEgaux ? `<div class="final__verdict">${MOT.tiebreakWhy}</div>` : "");

  f.innerHTML = `
    <div class="final__titre">
      <span>${MOT.endTitle}</span>
      <b>${MOT.endSub}</b>
    </div>
    <div class="final__colonnes">${colonnes}</div>
    ${verdict}
    <div class="final__planete">
      <span>${MOT.temp} <i data-valeur="planet.temperature">${etat.planet.temperature}</i>
        /<i data-valeur="planet.temperature_max">${etat.planet.temperature_max}</i></span>
      <span>${MOT.oxygen} <i data-valeur="planet.oxygen">${etat.planet.oxygen}</i>
        /<i data-valeur="planet.oxygen_max">${etat.planet.oxygen_max}</i></span>
      <span>${MOT.ocean} <i data-valeur="planet.oceans">${etat.planet.oceans}</i>
        /<i data-valeur="planet.oceans_max">${etat.planet.oceans_max}</i></span>
      <span>${MOT.round} <i data-valeur="generation">${etat.generation}</i></span>
    </div>`;

  document.body.appendChild(f);
  requestAnimationFrame(() => f.classList.add("final--vif"));
  return f;
}

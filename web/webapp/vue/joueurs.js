// LES DEUX ÉQUIPAGES — une barre par joueur, collée à son plateau.
//
// Tout ce qui s'affiche ici vient de `etat.players[j]` et porte son chemin exact
// dans `data-valeur`. Rien n'est calculé : la barre lit, elle ne compte pas.
//
// La place appartient désormais au plateau de jeu : ces barres sont donc
// compactes et horizontales, l'une sous le plateau du joueur d'en face, l'autre
// au-dessus du sien. Les cartes en jeu, elles, ne sont plus ici — elles sont
// posées sur le plateau (`plateau.js`).

// COUTURE — deux chantiers ont écrit dans ce fichier, sans se rencontrer :
//
//   · `bandeau-et-monde` — la ventilation du score (`PARTS_SCORE`,
//     `ventilation`, et sa mise à jour) : cinq parts lues chez le moteur, plus
//     la mention « provisional » qui s'efface à la fin de la partie.
//   · `table-vivante` — le dos des corporations encore cachées, tout en bas :
//     la mémoire du dessin passe de `data-corpo` à `data-etat-corpo`, pour que
//     `data-corpo` ne porte jamais un nom qu'on n'a pas le droit de lire.
//
// Les deux vivent dans la même barre mais pas dans le même bloc : la
// ventilation est dans la jauge de score, le dos dans la case de corporation.
// Aucun arbitrage n'a été nécessaire ; le seul point à retenir pour la suite est
// que `vue/options.js` doit effacer les DEUX attributs quand il vide la table.

import {
  imageEquipage, imageReserve, imageBadge, nomBadge, ORDRE_BADGES,
  jetonForetDetoure, EQUIPAGES, nomJoueur,
} from "./materiel.js";
import { carte } from "./cartes.js";
import { survolable } from "./loupe.js";
import { ref, poser, poserValeur } from "./ecrire.js";
import { estPlanification } from "./phases.js";
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

/**
 * LES CINQ PARTS DU SCORE, dans l'ordre du décompte du livret (p.16-17) et sous
 * les noms exacts que le moteur publie (`players[].score_parts`).
 *
 * (regles-de-la-vente) UNE SEULE EST PROVISOIRE : les RÉCOMPENSES.
 * `award_points_split` les distribue d'avance, comme si la partie s'arrêtait à
 * l'instant, alors qu'elles ne seront attribuées qu'à la fin — ce sont les
 * treize points qui faisaient afficher 18 et 15 au premier écran d'une partie où
 * personne n'avait rien fait.
 *
 * Les JALONS ne le sont pas, et ils l'étaient à tort : « un jalon atteint l'est
 * pour de bon ». Ils comptent donc dans l'acquis.
 *
 * Le troisième terme n'est plus une nuance de style : il porte `data-provisoire`
 * sur la part elle-même, et c'est de LUI que se déduit le grand nombre — la
 * somme des parts qui ne sont pas provisoires. Une seule liste, deux lectures
 * qui ne peuvent pas diverger.
 */
const PARTS_SCORE = [
  ["tr", MOT.scoreTr, false],
  // LIS-8 (tranché par Alexis le 04-08) — la part « Forests » a été RETIRÉE
  // d'ici. Une forêt vaut un point de victoire : ce nombre était exactement
  // celui de l'hexagone des capacités, quelques lignes plus bas, et le même
  // nombre écrit deux fois dans une même barre se lit comme deux grandeurs.
  // C'est l'hexagone qu'on garde ; la ligne du score, elle, n'est pas masquée,
  // elle n'est plus construite. Le total, lui, ne bouge pas d'un point : il
  // vient du moteur et n'a jamais été la somme de ces cases.
  ["cards", MOT.scoreCards, false],
  ["milestones", MOT.scoreMilestones, false],
  ["awards", MOT.scoreAwards, true],
];

/**
 * La ventilation affichée sous le score d'un joueur. Aucun de ces nombres n'est
 * calculé ici : chacun porte son chemin dans l'état, et leur somme est le score
 * que le moteur publie juste à côté — il n'existe qu'un point de calcul du
 * score, et ce n'est pas la page.
 */
function ventilation(j) {
  const cases = PARTS_SCORE.map(([cle, mot, provisoire]) =>
    `<span class="ventil__part${provisoire ? " ventil__part--provisoire" : ""}">` +
    `<i>${mot}</i><b data-valeur="players.${j}.score_parts.${cle}">0</b></span>`).join("");
  return cases +
    `<span class="ventil__dit" id="provisoire-${j}" data-provisoire ` +
    `title="${MOT.provisionalWhy}">${MOT.provisional}</span>`;
}

/** Construit les deux barres. Appelé une fois par partie. */
export function construireJoueurs() {
  for (const j of [0, 1]) {
    const a = document.createElement("aside");
    a.className = "equipage";
    a.id = "equipage-" + j;
    a.dataset.joueur = String(j);
    a.style.setProperty("--teinte", EQUIPAGES[j].teinte);

    // (MOT-10) La barre porte désormais, sous la piste de production, LE REVENU
    // RÉEL DE LA PROCHAINE PHASE PRODUCTION (« prod__reel »). La piste ne montre
    // que les productions FIXES ; la phase verse en plus le niveau de
    // terraformation et tout ce qui dépend du nombre de badges ou de jetons
    // Forêt — quatorze cartes. Un joueur voyait 5 et touchait 7.
    //
    // Le nombre vient du moteur ENTIER (chemin players.J.production.mc_reel) :
    // la page ne l'additionne pas, elle le recopie. La mention à côté dit ce
    // qu'il laisse dehors, et pourquoi.
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
        <div class="prod__reel" title="${MOT.nextIncomeNote}">
          <span class="prod__reel-mot">${MOT.nextIncome}</span>
          <b data-valeur="players.${j}.production.mc_reel">0</b>
          <i>${MOT.mc}</i>
          <span class="prod__reel-note">${MOT.nextIncomeNote}</span>
        </div>
      </div>

      <div class="capacites">
        <span class="cap"><i>${MOT.steel}</i><b data-valeur="players.${j}.steel_capacity">0</b></span>
        <span class="cap"><i>${MOT.titanium}</i><b data-valeur="players.${j}.titanium_capacity">0</b></span>
        <span class="cap cap--foret"><img src="${jetonForetDetoure()}" alt="forests">
          <b data-valeur="players.${j}.forests">0</b></span>
        <span class="cap"><i>Phase</i><b data-valeur="players.${j}.chosen_phase">0</b></span>
      </div>

      <div class="badges" id="badges-${j}"></div>

      <div class="jauge jauge--vp" data-role="vp">
        <span class="jauge__mot">${MOT.vp}</span>
        <b class="jauge__n" data-valeur="players.${j}.score">0</b>
        <div class="ventil" id="ventil-${j}">${ventilation(j)}</div>
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
 * La corporation du siège `j` est-elle publique pour qui regarde `siege` ?
 *
 * PRÉDICAT PUR, comme `estPlanification` pour les phases : il ne lit que l'état
 * qu'on lui passe, jamais une variable posée par un autre module — sans quoi il
 * dépendrait de l'ordre dans lequel l'écran se réécrit.
 *
 * Le livret (`docs/regles/livret-base.md` l. 211) distribue les Corporations
 * FACE CACHÉE, deux par joueur ; chacun choisit la sienne parmi ses deux, et
 * l'étape 9 (l. 215) les révèle ensemble. Le moteur, lui, pose la question aux
 * deux joueurs L'UN APRÈS L'AUTRE et installe la corporation du premier avant
 * d'interroger le second : celui-ci choisissait donc en connaissant celle d'en
 * face — un avantage que la table ne donne pas.
 *
 * Le moment se lit dans l'état seul, et c'est le plus sûr : ma corporation est
 * posée à l'instant exact où je réponds, et le moteur ne la retire jamais. Tant
 * que je n'ai pas la mienne, celle d'en face n'a pas été révélée. La décision
 * n'apprendrait rien de plus ici — le type de question ne dit pas si j'ai déjà
 * répondu à la mienne.
 */
export function corporationRevelee(etat, siege, j) {
  // La sienne lui appartient : on ne la lui cache jamais.
  if (j === siege) return true;
  const moi = etat.players.find((p) => p.player === siege);
  return !!(moi && moi.corporation);
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

// ------------------------------------------------------- CE QUI VIENT D'ENTRER
//
// 04-08, Alexis : « il faudrait que lors de la phase de production que l'on voie
// au moins ses compteurs de MC et de chaleur et de plantes avoir un +X / +Y / +Z
// et qui dure un peu de temps pour qu'on comprenne que les compteurs ont
// augmenté. Là c'est instantané. »
//
// Rien n'est CALCULÉ ici, et surtout pas une production : on compare l'état
// d'avant et l'état d'après, exactement comme `vue/monde.js` le fait déjà pour
// la planète, et on affiche l'écart que le moteur vient de créer. Jamais un
// nombre inventé, jamais un nombre attendu — celui qui a eu lieu.
//
// Les BAISSES ne s'affichent pas : dépenser est un geste qu'on vient de faire
// soi-même, on sait ce qu'on a payé. C'est ce qui ARRIVE sans qu'on l'ait
// demandé qui a besoin d'être vu.
const avant = new Map(); // "joueur.cle" -> dernière valeur rendue

/** Efface la mémoire des réserves (nouvelle partie, table vidée). */
export function oublierGains() {
  avant.clear();
}

function montrerGain(a, j, cle, valeur) {
  const memoire = `${j}.${cle}`;
  const precedent = avant.get(memoire);
  avant.set(memoire, valeur);
  // Premier rendu : il n'y a pas d'avant, donc pas d'écart. Sans cette garde,
  // toute la mise en place de la partie s'annoncerait comme un gain.
  if (precedent === undefined || valeur <= precedent) return;
  const bac = a.querySelector(".reserve--" + cle);
  if (!bac) return;
  const d = document.createElement("span");
  d.className = "gain";
  d.textContent = "+" + (valeur - precedent);
  // (cartes-qui-bougent, ANI-4) LES GAINS S'EMPILENT AU LIEU DE S'EFFACER.
  //
  // Le gain précédent était RETIRÉ dès qu'un nouveau arrivait — deux nombres
  // superposés sur le même bac ne se lisent ni l'un ni l'autre, et c'était juste.
  // Mais allonger la durée pour qu'on ait le temps de lire (ANI-4) a retourné
  // cette règle contre elle-même : mesuré par mon banc `verif/passages-et-duree.py`
  // sur la graine 4242, le plus court des 131 gains chronométrés ne vivait plus
  // que 6 MILLISECONDES, tué par le suivant. Un « +3 » de vente pouvait donc
  // disparaître avant d'avoir paru — exactement le défaut qu'ANI-4 demande de
  // réparer.
  //
  // Chacun garde donc sa vie entière, et se pose PLUS HAUT que celui qui est
  // déjà là (`--rang`, lu par `style-monde.css`). Ils ne se recouvrent pas, ils
  // se suivent — et comme chacun s'efface tout seul, la colonne se vide d'elle-
  // même.
  d.style.setProperty("--rang", String(bac.querySelectorAll(".gain").length));
  bac.appendChild(d);
  // (cartes-qui-bougent, ANI-4) 3 400 ms, et non plus 1 900.
  //
  // « Le "+3" de la défausse passe trop vite — il doit durer assez longtemps
  // pour être lu. » Ce « +3 » est celui-ci : vendre une carte projet rapporte
  // 3 MC (livret l. 96), et le gain monte du bac de mégacrédits comme tous les
  // autres. À 1 900 ms, l'animation ne le laissait pleinement lisible qu'un peu
  // plus d'une seconde — le reste étant l'apparition et l'effacement — et le
  // joueur qui regardait sa main au moment de vendre le manquait.
  //
  // La durée est celle de l'animation qui l'efface (`style-monde.css`,
  // `gain-monte`) : les deux nombres disent la même chose et changent ensemble,
  // sans quoi le nœud disparaîtrait avant la fin de son propre effacement.
  setTimeout(() => d.remove(), 3400);
}

/**
 * Réécrit les deux barres à partir de l'état.
 *
 * @param {number} siege  le siège regardé — sa phase choisie lui appartient,
 *                        celle d'en face attend la révélation.
 */
export function majJoueurs(etat, decision, siege) {
  const planifie = estPlanification(decision);
  for (const p of etat.players) {
    const j = p.player;
    const a = ref("#equipage-" + j);
    if (!a) continue;
    a.classList.toggle("equipage--actif", !!decision && decision.joueur === j);
    for (const [cle] of RESERVES) montrerGain(a, j, cle, p[cle] ?? 0);

    poserValeur(`players.${j}.tr`, p.tr);
    // (regles-de-la-vente) **LE GRAND NOMBRE NE COMPTE QUE L'ACQUIS.**
    // `score_acquis` = niveau de terraformation + forêts + cartes posées +
    // jalons ; c'est le moteur qui l'additionne (`ScoreBreakdown::acquis`), la
    // page ne recalcule rien. Le TOTAL, lui, ne bouge pas d'un point et reste ce
    // que lisent le classement et le simulateur — quand la partie est finie, les
    // récompenses sont attribuées pour de vrai et le grand nombre vaut le total.
    poserValeur(`players.${j}.score`, etat.game_over ? p.score : p.score_acquis);

    // LA VENTILATION DU SCORE. Elle vient du moteur, part par part
    // (`engine::observe`, lu sur `flow::score_breakdown` — le même parcours qui
    // forme le total affiché au-dessus). La page ne fait que la recopier : un
    // second barème calculé ici finirait par diverger de celui qui compte.
    //
    // ET CHAQUE PART DIT SI ELLE EST PROVISOIRE, sur elle-même. Sans cela on
    // pouvait lire le grand nombre et la ventilation à côté sans jamais savoir
    // lequel des cinq termes il laissait dehors.
    for (const [cle, , provisoire] of PARTS_SCORE) {
      poserValeur(`players.${j}.score_parts.${cle}`, p.score_parts[cle]);
      const e = document.querySelector(
        `[data-valeur="players.${j}.score_parts.${cle}"]`
      );
      if (!e) continue;
      // « Provisoire » cesse de l'être quand la partie est finie : les
      // récompenses sont alors attribuées, et plus rien ne peut basculer.
      if (provisoire && !etat.game_over) e.dataset.provisoire = "";
      else delete e.dataset.provisoire;
    }
    // « Provisoire » ne se dit que tant que ça peut encore basculer. Une
    // étiquette collée en permanence ne dirait plus rien : à la fin de la
    // partie les récompenses sont attribuées pour de bon, la mention s'en va.
    // (regles-de-la-vente : les JALONS, eux, ne sont plus provisoires du tout —
    // un jalon atteint l'est pour de bon, il compte dans l'acquis dès qu'il
    // tombe.)
    const dit = ref("#provisoire-" + j);
    if (dit) dit.hidden = !!etat.game_over;
    poserValeur(`players.${j}.forests`, p.forests);
    poserValeur(`players.${j}.steel_capacity`, p.steel_capacity);
    poserValeur(`players.${j}.titanium_capacity`, p.titanium_capacity);
    // LA PHASE D'EN FACE N'EST PUBLIQUE QU'APRÈS LA RÉVÉLATION, ici comme dans
    // la bande (`vue/phases.js`) et dans l'annonce. Le marqueur `data-valeur`
    // reste posé — il est le contrat —, mais tant que la manche se planifie la
    // case affiche 0, comme avant tout choix. Sans cela, `chosen_phase` étant
    // rémanent, le siège interrogé en SECOND lisait ici la carte que
    // l'adversaire venait de poser face cachée, et choisissait la sienne en la
    // connaissant : mesuré 43 planifications sur 43 (graine 911, siège 1),
    // toutes différentes de la valeur de fin de manche précédente — donc bien
    // le choix du moment, pas une rémanence. La valeur redevient publique dès
    // la révélation, c'est-à-dire pendant presque toute la manche.
    const attendRevelation = planifie && j !== siege;
    poserValeur(`players.${j}.chosen_phase`, attendRevelation ? 0 : (p.chosen_phase || 0));
    for (const [cle] of RESERVES) poserValeur(`players.${j}.${cle}`, p[cle]);
    for (const [cle] of PRODUCTIONS) {
      const e = ref(`[data-valeur="players.${j}.production.${cle}"]`);
      if (e) {
        poser(e, p.production[cle]);
        e.parentElement.classList.toggle("prod__case--vide", p.production[cle] === 0);
      }
    }
    // (MOT-10) Le revenu réel se pose comme le reste : lu chez le moteur, jamais
    // recomposé ici. Il vaut au moins le niveau de terraformation, donc il n'est
    // jamais vide — pas de classe « --vide » à basculer.
    poserValeur(`players.${j}.production.mc_reel`, p.production.mc_reel);

    // La corporation est montrée par son SCAN, jamais par son nom écrit : six
    // cartes du jeu s'appellent « … Corporation », et l'écran est en anglais.
    //
    // CELLE D'EN FACE N'ENTRE PAS DANS LA PAGE TANT QUE JE N'AI PAS CHOISI LA
    // MIENNE. Un nom vide, et la zone reste ce qu'elle était avant tout choix :
    // rien à lire, rien à survoler, `data-corpo` vide. On ne se contente pas de
    // masquer la carte — son nom voyageait par TROIS chemins à la fois
    // (`data-corpo`, l'`alt` de l'image, et le nom de fichier du scan), et une
    // information cachée mais présente reste une information donnée.
    const nom = corporationRevelee(etat, siege, j) ? (p.corporation || "") : "";
    // Tant qu'elle est cachée mais qu'il en a une, on montre le DOS DES
    // CORPORATIONS — la cité sous dôme, et jamais le campement martien, qui est
    // le dos des cartes projet. Un dos ne dit rien de la carte : il dit sa
    // sorte, ce qui est public, et il évite une case vide qui laisserait croire
    // que l'adversaire n'a pas encore de corporation.
    const cache = !nom && !!p.corporation;
    const etiquette = nom || (cache ? "dos" : "");
    const z = ref("#corpo-carte-" + j);
    // La mémoire du dessin est tenue à part : `data-corpo` ne porte JAMAIS un nom
    // qu'on n'a pas le droit de lire, elle ne peut donc pas servir de mémoire.
    if (z.dataset.etatCorpo !== etiquette) {
      z.dataset.etatCorpo = etiquette;
      z.dataset.corpo = nom;
      z.textContent = "";
      if (nom) {
        const f = carte({ nom }, { classe: "carte--corpo" });
        survolable(f, { nom });
        z.appendChild(f);
      } else if (cache) {
        z.appendChild(carte(null, { classe: "carte--corpo", dos: "corporation" }));
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

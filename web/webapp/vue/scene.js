// LA SCÈNE — l'endroit où une décision est posée.
//
// La page n'invente aucun choix : elle n'offre QUE `decision.options`, dans
// l'ordre où le moteur les a énumérées, et rend au moteur l'indice choisi.
// Elle ne juge jamais si un choix est légal — si le moteur l'a énuméré, il l'est.
//
// Deux zones : le CONTEXTE (ce que la décision donne à voir : une main, deux
// corporations, la carte concernée — jamais cliquable) et les CHOIX (un élément
// `data-choix` par option, dans l'ordre des indices, « passer » en dernier).

import { carte, normaliser, cle } from "./cartes.js";
import {
  imagePhase, imageAmelioration, phaseNom, imageBadge, nomBadge, imageCarte,
  imageReserve, dosProjet, EQUIPAGES, nomJoueur,
  jetonForetDetoure, jetonOceanDetoure,
} from "./materiel.js";
import { survolable, survolableImage, cacher as cacherLoupe, figer } from "./loupe.js";
import { contexteRevelation } from "./revelation.js";
import { MOT, question as questionAnglaise, libelleOption, sorteAction } from "./mots.js";
import { decisionDeMain, LARGEUR as LARGEUR_MAIN } from "./mains.js";
import { ouvrirGeste, fermerGeste, poserLaCarte } from "./geste.js";
import { poserPhase } from "./table.js";
import { voler } from "./anim.js";

const RATIO = 569 / 409; // les images de cartes, telles qu'elles ont été découpées
const ECART = 12;
const MAX_CARTE = 340; // au-delà, une carte mange la scène sans rien ajouter
// La ligne de nom sous une carte de choix : 11 px de texte, son interligne et
// l'espace qui la sépare de la carte. Sous-estimée, les choix débordent de leur
// bande et se posent sur les plateaux.
const LEGENDE = 26;

// ---------------------------------------------------------- LE PLANCHER DE 40
//
// Aucun bouton de choix ne descend sous 40 points de côté, à aucune taille de
// fenêtre : c'est la seule mesure qui rende un bouton atteignable par une main
// comme par un programme. Tout ce qui partage la hauteur de la scène s'y plie —
// c'est le contexte qui cède, jamais les choix.
const MINI = 40;
// Ce que la bande des choix demande quand la place le permet : le double du
// plancher. En dessous, on ne dessine pas « petit », on dessine « juste ».
const AISE = 80;
// LA HAUTEUR QUE DEMANDE UN CONTEXTE DE RAPPEL — une vignette de 68 points de
// large, son intitulé et sa marge. Elle est CALCULÉE et non mesurée : la hauteur
// réelle d'une carte dont l'image n'est pas encore décodée vaut zéro, et le
// contexte se verrait alors accorder zéro point pour toujours.
const CTX_VIGNETTE = 68;
const CTX_CHROME = 23;
// Et la hauteur sous laquelle il ne descend pas tant que les choix ont leur
// plancher : un contexte réduit à rien n'informe plus personne.
const CTX_MINI = 56;

// Les moments qui méritent de couvrir la table : on y regarde des cartes en
// grand, et le plateau reste visible dessous. Partout ailleurs la scène est une
// bande, et les deux plateaux gardent leur place.
//
// Les trois choix de BRANCHE en font partie : la proposition qu'on applique est
// écrite sur la carte elle-même — le texte imprimé d'une carte projet pour
// `alternative_carte` et `alternative_action`, la case BONUS de la carte Phase
// pour `bonus_selectionneur`. Dans une bande, cette carte tombe à 90 px de haut
// et n'est plus lisible : la décision se prendrait à l'aveugle.
//
// (regles-de-la-vente) LA VENTE POUR PAYER en faisait partie depuis le 02-08 :
// une phrase, jusqu'à neuf cartes qui allaient partir, un cadran et un bouton.
// Cette décision n'existe plus — le moteur ne vend plus d'office pour compléter
// un paiement, et le joueur vend désormais quand il veut, par le bouton de
// `vue/vente.js`. Le contexte qui la dessinait est retiré avec elle : une vue
// qu'aucune décision ne peut plus atteindre n'est pas du décor, c'est du code
// mort qui prétend que la règle existe encore.
const SUPERPOSITION = new Set([
  "corp_mulligan", "project_mulligan", "pick_corporation",
  "alternative_carte", "alternative_action", "bonus_selectionneur",
  // Une révélation du dessus de la pioche EST un moment où l'on regarde des
  // cartes : les trois retournées doivent se lire, y compris quand aucune n'est
  // prenable et qu'il n'y a rien d'autre à faire que les voir.
  "revelation_pioche",
]);

let resoudre = null; // la réponse attendue par le moteur, une fois cliquée
let enCours = null; // la décision affichée, pour pouvoir la redessiner
// Une carte Phase est en train de se poser : on n'en lance pas une seconde.
let phaseEnVol = false;

/**
 * Le squelette de la scène. Appelé une fois.
 *
 * Elle se pose DANS la table (`#milieu`, écrit en dur dans `index.html`) : c'est
 * la même surface, entre les deux plateaux, qui reçoit la décision et les cartes
 * qu'on y dépose. La table existe donc avant le premier octet de script — elle
 * n'est pas un décor que le jeu fabrique, c'est le meuble.
 */
export function construireScene() {
  const m = document.createElement("main");
  m.id = "scene";
  (document.getElementById("milieu") || document.body).appendChild(m);
}

/**
 * Pose une décision et rend une promesse résolue avec la réponse attendue.
 * @param {object} d     le descripteur du moteur
 * @param {object} etat  l'état au moment de cette décision (contexte visuel)
 */
export function poserDecision(d, etat) {
  return new Promise((ok) => {
    resoudre = ok;
    enCours = { d, etat };
    dessiner(d, etat);
  });
}

/**
 * REDESSINER À LA NOUVELLE TAILLE. La grille des choix est calculée en pixels
 * au moment du dessin, à partir de la place disponible. Si la fenêtre change de
 * taille pendant qu'une décision est posée, ces pixels ne veulent plus rien
 * dire : les choix débordent de leur bande et se posent sur les plateaux. On
 * refait donc le dessin. Une sélection multiple en cours est reprise à zéro —
 * c'est le prix, et il est payé une fois, au moment où l'on redimensionne.
 */
export function replacerScene() {
  if (!resoudre || !enCours) return;
  dessiner(enCours.d, enCours.etat);
}

/**
 * (regles-de-la-vente) **Rendre une VENTE à la place de la réponse attendue.**
 *
 * Une vente n'est pas une réponse à la question posée : c'est une entrée de plus
 * dans la liste des décisions, que le moteur consomme à son point d'occasion
 * avant de reposer la MÊME question sur l'état d'après (`vue/vente.js`). Elle
 * emprunte pourtant le même fil que la réponse — la promesse que la boucle de
 * jeu attend — parce que c'est le seul par où quelque chose peut atteindre le
 * moteur sans que la page invente un second chemin.
 *
 * Rend `false` si aucune question n'est posée à cet instant : la vente est alors
 * gardée par `vue/vente.js` et rendue à la question suivante. Un `false` n'est
 * pas un échec, c'est « pas maintenant ».
 */
export function venteImmediate(entree) {
  if (!resoudre) return false;
  repondre(entree);
  return true;
}

/** Efface la scène : plus aucun `data-choix` ne subsiste (fin de partie). */
export function viderScene() {
  const m = document.getElementById("scene");
  if (m) m.textContent = "";
  enCours = null;
  phaseEnVol = false;
  fermerScene();
}

/** Referme la scène : les attributs de décision doivent disparaître aussitôt. */
export function fermerScene() {
  const m = document.getElementById("scene");
  m.removeAttribute("data-decision-rang");
  m.removeAttribute("data-decision-forme");
  m.removeAttribute("data-decision-type");
  m.removeAttribute("data-a-choisir");
  // Plus aucune question posée : plus aucune carte de la main ne se pose.
  fermerGeste();
  cacherLoupe();
  figer();
}

/**
 * RÉPONDRE SANS CLIC — le siège regardé est tenu par un programme
 * (`?decide=programme`). La scène est posée comme pour un humain : on la voit se
 * remplir, puis la réponse tombe. Le chemin est le MÊME que celui du clic, à
 * l'origine du geste près.
 *
 * « LE MÊME CHEMIN » DOIT RESTER VRAI. Depuis que jouer une carte veut dire la
 * POSER, répondre ici par un simple `repondre()` en ferait un troisième chemin
 * vers le moteur — celui qu'aucun contrôle n'emprunte, et où l'on ne verrait
 * jamais une carte se poser. Or c'est précisément ce mode qu'on ouvre pour
 * REGARDER une intelligence artificielle jouer à sa place : un écran où rien ne
 * bouge n'a plus d'objet. On repasse donc par le geste, exactement comme un
 * clic — la carte de la main que l'indice désigne, ou la carte Phase choisie.
 */
export async function repondrePourLeSiege(reponse) {
  const d = enCours && enCours.d;
  if (d && typeof reponse === "number") {
    // Une carte de la main : on la pose, et `poserLaCarte` répond au bout.
    if (decisionDeMain(d)) {
      const carteDeLaMain = document.querySelector(
        `[data-main-siege] [data-choix="${reponse}"]`
      );
      if (carteDeLaMain) {
        await poserLaCarte(carteDeLaMain);
        return;
      }
    }
    // Une carte Phase : elle s'en va se poser sur la table, puis on répond.
    if (d.type === "pick_phase") {
      const bouton = document.querySelector(`#scene [data-choix="${reponse}"]`);
      const o = (d.options || [])[reponse];
      if (bouton && o && o.phase) {
        bouton.click();
        return;
      }
    }
    // Une corporation : même chose, elle s'en va se poser à sa place.
    if (d.type === "pick_corporation") {
      const bouton = document.querySelector(`#scene [data-choix="${reponse}"]`);
      if (bouton) {
        bouton.click();
        return;
      }
    }
  }
  repondre(reponse);
}

function repondre(r) {
  const f = resoudre;
  resoudre = null;
  enCours = null;
  if (!f) return;
  fermerScene();
  f(r);
}

// ------------------------------------------------------------------- le rendu

function dessiner(d, etat) {
  const m = document.getElementById("scene");
  m.textContent = "";
  m.dataset.decisionRang = String(d.rang);
  m.dataset.joueur = String(d.joueur);

  const forme = d.multiple ? "multiple" : d.montant ? "montant" : "simple";
  m.dataset.decisionForme = forme;
  m.dataset.mode = SUPERPOSITION.has(d.type) ? "superposition" : "bande";
  m.dataset.type = d.type;
  // Le TYPE que le moteur donne, recopié tel quel à côté du rang : de dehors, on
  // peut alors savoir de quelle question il s'agit sans la relire en anglais.
  m.dataset.decisionType = d.type;

  // Nombre libre (remplacement partiel des cartes de départ) : on ne pose pas
  // l'attribut du tout, plutôt que d'annoncer « undefined » à qui nous lit.
  if (forme === "multiple" && d.a_choisir !== undefined && d.a_choisir !== null) {
    m.dataset.aChoisir = String(d.a_choisir);
  }

  // Le fond de scène : un fragment agrandi et noyé de la carte dont il est
  // question. L'image devient l'atmosphère, pas une vignette.
  const fond = document.createElement("div");
  fond.className = "scene__fond";
  const sujet = imageSujet(d);
  if (sujet) fond.style.backgroundImage = `url("${sujet}")`;
  m.appendChild(fond);

  const tete = entete(d);
  m.appendChild(tete);

  // Deux compositions, selon ce que la décision donne à regarder.
  //
  //   • les options SONT des images (cartes, cartes Phase, jetons) : ce sont
  //     elles le sujet, elles prennent toute la place ;
  //   • les options sont des mots (« Garder », « Piocher AVANT de poser ») :
  //     alors le sujet est ailleurs — la main, les deux corporations, la carte
  //     en cours — et c'est LUI qu'il faut montrer en grand.
  const riche = optionsIllustrees(d);
  m.dataset.composition = riche ? "options" : "contexte";

  const ctx = contexte(d);
  if (ctx) m.appendChild(ctx);

  const zone = document.createElement("div");
  zone.className = "scene__choix";
  m.appendChild(zone);

  const barre = document.createElement("div");
  barre.className = "scene__barre";
  // La barre de validation est remplie APRÈS que les choix soient mesurés : on
  // lui réserve sa hauteur tout de suite, sinon les choix la recouvrent.
  if (forme !== "simple") barre.dataset.reservee = "";
  m.appendChild(barre);

  // LA HAUTEUR SE PARTAGE AVANT QUE QUOI QUE CE SOIT SOIT POSÉ.
  //
  // C'est ici que le défaut d'origine vivait : le contexte et les choix étaient
  // tous deux `flex: 1 1 auto` avec `min-height: 0`, donc rien ne garantissait
  // un plancher à la bande des choix. En 1440 × 810, au rang 9 (dix badges), le
  // contexte prenait ses 117 points naturels et il en restait 32 aux choix —
  // d'où des boutons taillés pour une bande qui n'existait pas, et empilés.
  //
  // On mesure donc la place disponible sur la SOMME des deux zones (leur total
  // ne dépend pas du partage, il est donc juste même avant qu'elles soient
  // remplies), puis on l'attribue : les choix d'abord, le contexte ensuite.
  const geste = forme === "simple" && decisionDeMain(d);
  const nChoix = geste || forme === "montant"
    ? 0
    : (d.options || []).length + (forme === "simple" && d.passer ? 1 : 0);
  const dispo = zone.clientHeight + (ctx ? ctx.offsetHeight : 0);
  if (!geste && dispo > 1) {
    const images = choixSontDesImages(d);
    // Le mot sous une carte s'efface dès qu'elle est serrée (`data-dense`) —
    // sauf sous une carte Phase améliorée, seule à garder le sien. C'est donc la
    // seule pour qui la ligne de nom compte encore dans le calcul du plancher.
    const legende = images && d.type === "amelioration_carte_phase";
    const L = zone.clientWidth || 1040;
    const plancher = nChoix
      ? hauteurPourTenir(L, nChoix, images, legende, MINI)
      : CTX_MINI;
    let hChoix;
    if (!riche && ctx) {
      // LE CONTEXTE EST LE SUJET (les options ne sont que des mots) : il garde le
      // gros de la bande. Les choix, eux, ne descendent plus sous leur plancher.
      const voulue = forme === "montant" ? 108 : nChoix <= 4 ? 172 : 312;
      hChoix = Math.max(plancher, Math.min(voulue, Math.floor(dispo * 0.6)));
      hChoix = Math.min(hChoix, Math.max(plancher, dispo - CTX_MINI));
    } else if (ctx) {
      // LE CONTEXTE N'EST QU'UN RAPPEL : il cède la place aux choix.
      const aise = nChoix ? hauteurPourTenir(L, nChoix, images, legende, AISE) : 108;
      hChoix = dispo - partDuContexte(dispo, plancher, aise);
    } else {
      hChoix = dispo;
    }
    hChoix = Math.max(1, Math.min(hChoix, dispo));
    zone.style.flex = "0 0 auto";
    zone.style.height = hChoix + "px";
    if (ctx) {
      ctx.style.flex = "0 0 auto";
      ctx.style.height = (dispo - hChoix) + "px";
    }
  }

  if (ctx) remplirContexte(ctx, !riche);

  if (forme === "montant") montant(d, zone, barre);
  else if (forme === "multiple") multiple(d, zone, barre, etat);
  else simple(d, zone, etat);
}

function entete(d) {
  const e = document.createElement("div");
  e.className = "scene__entete";
  e.innerHTML =
    `<span class="scene__qui" style="--teinte:${EQUIPAGES[d.joueur].teinte}">` +
    `<i>${nomJoueur(d.joueur)}</i>${EQUIPAGES[d.joueur].nom}</span>` +
    `<h1 class="scene__question"></h1>`;
  // La question du moteur est française ; l'écran ne montre que son intitulé
  // anglais, tenu par `mots.js` et indexé sur le `type` de la décision.
  e.querySelector(".scene__question").textContent = questionAnglaise(d);
  return e;
}

/** Les options portent-elles leur propre image ? */
function optionsIllustrees(d) {
  const o = (d.options || [])[0];
  if (!o) return false;
  if (d.type === "pick_phase" || d.type === "pick_joker_tag") return true;
  // LES DEUX CORPORATIONS SONT LE SUJET. Elles ne passent plus par la main : on
  // les présente en grand au milieu, on en désigne une d'un clic, et elle s'en
  // va se poser à sa place — exactement le chemin d'une carte Phase.
  if (d.type === "pick_corporation") return true;
  // Une amélioration de carte Phase se CHOISIT sur l'image de la carte
  // améliorée : ce sont les options le sujet, elles prennent toute la place.
  if (d.type === "amelioration_carte_phase") return !!codeAmelioration(o);
  return !!normaliser(o);
}

/**
 * L'amélioration que CE joueur possède sur CETTE phase, ou `null`.
 *
 * Lue sur `players[].phase_upgrades`, la liste d'étiquettes que le moteur
 * publie (« 2B » = phase 2, amélioration B). On ne calcule pas, on ne devine
 * pas : si l'étiquette n'est pas là, le joueur n'a pas l'amélioration.
 */
function ameliorationPossedee(etat, joueur, phase) {
  if (!etat || joueur === undefined || joueur === null) return null;
  const p = (etat.players || []).find((x) => x.player === joueur);
  const liste = p && p.phase_upgrades;
  if (!Array.isArray(liste)) return null;
  return liste.find((code) => Number(String(code)[0]) === Number(phase)) || null;
}

/**
 * LES BOUTONS SERONT-ILS DES CARTES ? Ce n'est pas la question à laquelle
 * `optionsIllustrees` répond : celle-là dit « l'option est-elle le SUJET de
 * l'écran ? », et c'est elle qui décide de la composition.
 *
 * Les deux réponses divergent sur un seul cas, et il coûtait cher : un badge de
 * joker (`pick_joker_tag`) est bien le sujet, mais il se pose sur une PLAQUE,
 * avec son jeton dedans. `planImages` le taillait donc en carte — 4 points de
 * large en 1440 × 810 au rang 9 — alors qu'une plaque ne se serre pas sous une
 * cinquantaine de points : dix-sept paires de boutons empilées, et la partie
 * bloquée (mesuré le 04-08).
 */
function choixSontDesImages(d) {
  if (d.type === "pick_joker_tag") return false;
  return optionsIllustrees(d);
}

/**
 * Le code d'image d'une amélioration de carte Phase (« 2B »), lu sur les champs
 * `phase` et `variante` que le moteur pose sur l'option — jamais sur son rang.
 */
function codeAmelioration(o) {
  if (!o || o.phase === undefined || o.phase === null || !o.variante) return null;
  return `${o.phase}${o.variante}`;
}

/** L'image qui portera l'atmosphère de la scène. */
function imageSujet(d) {
  const c = normaliser(d.carte);
  if (c) {
    const im = imageCarte(c.nom);
    if (im) return im;
  }
  // Une carte Phase (améliorée ou non) est un sujet à part entière.
  if (d.type === "amelioration_carte_phase") {
    const code = codeAmelioration((d.options || [])[0]);
    if (code) return imageAmelioration(code);
  }
  if (d.type === "bonus_selectionneur") {
    const im = imageSelectionneur(d);
    if (im) return im;
  }
  const p = normaliser((d.options || [])[0]);
  if (p) {
    const im = imageCarte(p.nom);
    if (im) return im;
  }
  if (d.type === "pick_phase") return imagePhase(1);
  return null;
}

/**
 * La carte Phase dont on prend le bonus de sélectionneur. Le moteur donne sa
 * `phase` et, si elle est améliorée, sa `variante` : c'est ce couple qui
 * désigne l'image, comme pour une amélioration.
 */
function imageSelectionneur(d) {
  if (d.phase === undefined || d.phase === null) return null;
  return d.variante ? imageAmelioration(`${d.phase}${d.variante}`) : imagePhase(d.phase);
}

// ---------------------------------------------------------------- le contexte

/** Ce que la décision donne à voir sans qu'on ait à le choisir. */
function contexte(d) {
  // Les cartes RÉVÉLÉES (dessus de pioche) se montrent toutes, prenables ou
  // non : c'est `vue/revelation.js` qui les compose, et lui seul.
  const revelation = contexteRevelation(d);
  if (revelation) return revelation;

  const cartes = [];
  let mot = "";

  // Le bonus du sélectionneur se prend SUR une carte Phase : on la montre,
  // sinon la question n'a rien à quoi se raccrocher.
  if (d.type === "bonus_selectionneur") {
    const im = imageSelectionneur(d);
    if (im) return contextePhase(im, `${MOT.currentCard} · ${phaseNom(d.phase)}`);
  }

  // Choisir sa corporation n'a PLUS de contexte : les deux cartes sont les
  // options elles-mêmes, montrées en grand au milieu. Les répéter ici les
  // afficherait deux fois et volerait la place à celles qu'on doit choisir.
  if (d.type === "pick_corporation") return null;

  if (d.carte) {
    cartes.push(d.carte);
    mot = MOT.currentCard;
  } else if (d.corporations) {
    cartes.push(...d.corporations);
    mot = MOT.yourCorps;
  } else if (d.main && !optionsIllustrees(d)) {
    // La main n'est rappelée que si elle n'est pas DÉJÀ le sujet : au
    // remplacement des cartes de départ, les options SONT la main, et la
    // montrer deux fois ne fait que voler de la place aux cartes.
    cartes.push(...d.main);
    mot = MOT.yourHand;
  }
  if (!cartes.length) return null;

  const z = document.createElement("div");
  z.className = "scene__contexte";
  const l = document.createElement("span");
  l.className = "scene__contexte-mot";
  l.textContent = mot;
  z.appendChild(l);

  const r = document.createElement("div");
  r.className = "scene__contexte-rang";
  r.dataset.combien = String(cartes.length);
  for (const c of cartes) {
    const f = carte(c, { classe: "carte--contexte" });
    survolable(f, c);
    r.appendChild(f);
  }
  z.appendChild(r);
  return z;
}

/** Un contexte fait d'une seule carte Phase imprimée, montrée telle quelle. */
function contextePhase(src, mot) {
  const z = document.createElement("div");
  z.className = "scene__contexte";
  const l = document.createElement("span");
  l.className = "scene__contexte-mot";
  l.textContent = mot;
  z.appendChild(l);

  const r = document.createElement("div");
  r.className = "scene__contexte-rang";
  r.dataset.combien = "1";
  const f = document.createElement("figure");
  f.className = "carte carte--contexte";
  const im = document.createElement("img");
  im.src = src;
  im.alt = mot;
  im.draggable = false;
  f.appendChild(im);
  survolableImage(f, src, mot);
  r.appendChild(f);
  z.appendChild(r);
  return z;
}

/**
 * Le contexte prend la taille que la composition lui laisse. Quand c'est LUI le
 * sujet (les options ne sont que des mots), il occupe la scène.
 *
 * SES CARTES SUIVENT LA HAUTEUR QU'ON LUI A DONNÉE, dans les deux compositions.
 * La vignette de 68 points écrite en dur ne tenait pas dans une bande basse : le
 * contexte gardait ses 117 points quoi qu'il arrive, et c'est la bande des choix
 * qui payait. Une carte de rappel a le droit d'être petite ; un bouton qu'on
 * doit cliquer, non.
 */
function remplirContexte(z, maitre) {
  const r = z.querySelector(".scene__contexte-rang");
  if (!r) return;
  const n = Number(r.dataset.combien) || 1;
  // Sous cette hauteur, l'intitulé posé AU-DESSUS des cartes leur prendrait le
  // quart de la bande : il passe à côté d'elles, et la carte récupère la place.
  // Ce choix est fait EN PREMIER — il change la hauteur qu'aura le rang.
  z.dataset.compact = z.getBoundingClientRect().height < 84 ? "oui" : "non";
  const L = r.clientWidth || 1040;
  const H = r.getBoundingClientRect().height || (maitre ? 300 : CTX_VIGNETTE * RATIO);
  // Les cartes du contexte peuvent se chevaucher : on ne clique pas dessus.
  const pas = n > 6 ? 0.62 : n > 3 ? 0.82 : 1;
  const plafond = maitre ? 300 : CTX_VIGNETTE;
  let w = Math.min((L - 24) / (1 + (n - 1) * pas), H / RATIO, plafond);
  w = Math.max(maitre ? 60 : 18, Math.floor(w));
  r.style.setProperty("--w", w + "px");
  r.style.setProperty("--serrage", Math.round(-w * (1 - pas)) + "px");
}

// ------------------------------------------------------------------ les choix

/** Choix simple : un élément cliquable par option, « passer » en dernier. */
function simple(d, zone, etat) {
  const options = d.options || [];

  // LA DÉCISION SE JOUE DEPUIS LA MAIN. La liste au milieu n'a plus lieu d'être :
  // les cartes à jouer sont celles qu'on tient, en bas, et c'est là qu'on les
  // attrape (`vue/mains.js` pose `data-choix` dessus, `vue/geste.js` les arme).
  // La scène ne garde ici que ce qui n'est pas une carte : la question, et
  // « passer », qui n'est dans aucune main.
  if (decisionDeMain(d)) {
    ouvrirGeste(d.rang, repondre);
    zone.dataset.sorte = "geste";
    zone.style.gridTemplateColumns = "";
    const mot = document.createElement("p");
    mot.className = "scene__geste";
    mot.textContent = MOT.dropHere;
    zone.appendChild(mot);
    if (d.passer) zone.appendChild(boutonPasser(options.length));
    return;
  }

  const total = options.length + (d.passer ? 1 : 0);
  const largeur = mesurer(d, zone, total);
  options.forEach((o, i) => {
    const b = choix(d, o, i, largeur, etat);
    brancherChoix(b, d, o, i);
    zone.appendChild(b);
  });

  if (d.passer) {
    const b = slab(MOT.pass, "passer");
    b.dataset.choix = String(options.length);
    b.classList.add("choix--passer");
    b.style.setProperty("--w", largeur + "px");
    b.setAttribute("aria-label", "pass");
    b.addEventListener("click", () => repondre(options.length));
    zone.appendChild(b);
  }
}

/**
 * LE BOUTON « PASSER » NE DÉPASSE JAMAIS UNE CARTE À JOUER. Le joueur l'a relevé
 * le 02-08 : « il est plus grand que les cartes qu'on nous propose, il dépasse un
 * peu ». Maintenant que les cartes à jouer sont celles de la main, la comparaison
 * est directe — on lui donne la taille d'une carte de main, en un peu plus
 * petit, et il ne la reprend jamais à la grille des choix.
 */
function boutonPasser(indice) {
  const b = slab(MOT.pass, "passer");
  b.dataset.choix = String(indice);
  b.classList.add("choix--passer", "choix--passer-carte");
  b.setAttribute("aria-label", "pass");
  // ON MESURE LA CARTE, ON NE LA SUPPOSE PAS. La largeur d'une carte de main suit
  // la hauteur de la fenêtre : un bouton taillé sur une constante deviendrait plus
  // grand qu'elle dès que l'écran raccourcit — exactement le défaut qu'on répare.
  const im = document.querySelector("[data-main-siege] .carte--main img");
  const r = im ? im.getBoundingClientRect() : null;
  const l = r && r.width > 1 ? r.width : LARGEUR_MAIN;
  const h = r && r.height > 1 ? r.height : LARGEUR_MAIN * RATIO;
  b.style.width = Math.floor(l * 0.94) + "px";
  b.style.height = Math.floor(h * 0.82) + "px";
  b.style.minHeight = "0";
  b.addEventListener("click", () => repondre(indice));
  return b;
}

/**
 * Le clic d'une option qui reste au milieu. Une carte Phase ne se contente pas de
 * répondre : elle se POSE sur la table, en tournant, et la réponse ne part
 * qu'ensuite — sinon l'écran se réécrirait sous la carte en vol.
 */
function brancherChoix(b, d, o, i) {
  if (d.type === "pick_phase" && o.phase) {
    b.addEventListener("click", async () => {
      // UNE SEULE CARTE PART. Le test `resoudre` ne suffit pas : il est relu
      // AVANT le vol, et `repondre` n'éteint la décision qu'APRÈS. Deux clics
      // pendant les 700 ms du vol franchiraient donc tous deux la garde, et le
      // second répondrait à la décision SUIVANTE — celle du choix de phase de la
      // manche d'après, voire une tout autre question. Le rang capturé ici
      // interdit ce report, et le verrou interdit le second vol.
      if (phaseEnVol || !resoudre) return;
      const rang = d.rang;
      phaseEnVol = true;
      const manche = (enCours && enCours.etat.generation) || 0;
      try {
        await poserPhase(b, o.phase, d.joueur, manche);
      } finally {
        phaseEnVol = false;
      }
      if (enCours && enCours.d.rang !== rang) return; // la question a changé
      repondre(i);
    });
    return;
  }

  // LA CORPORATION QU'ON DÉSIGNE S'EN VA SE POSER. Même chemin, même verrou et
  // même garde que la carte Phase ci-dessus — un seul vol, et la réponse ne part
  // qu'une fois la carte arrivée, sinon l'écran se réécrirait sous elle.
  //
  // OÙ ELLE SE POSE : la case de corporation de son propriétaire, dans la barre
  // d'équipage (`vue/joueurs.js`, `#corpo-carte-<siège>`). C'est là que le moteur
  // la fera paraître au rendu suivant : la carte va donc là où elle sera.
  if (d.type === "pick_corporation") {
    b.addEventListener("click", async () => {
      if (phaseEnVol || !resoudre) return;
      const rang = d.rang;
      phaseEnVol = true;
      try {
        const place = document.getElementById("corpo-carte-" + d.joueur);
        if (place) await voler(b, place, { ms: 700, tour: 360, grossir: 1.1 });
      } finally {
        phaseEnVol = false;
      }
      if (enCours && enCours.d.rang !== rang) return; // la question a changé
      repondre(i);
    });
    return;
  }

  b.addEventListener("click", () => repondre(i));
}

/** Choix multiple : on sélectionne, puis on valide. */
function multiple(d, zone, barre, etat) {
  const options = d.options || [];
  // `a_choisir` absent = nombre LIBRE : le remplacement partiel des cartes de
  // départ va de 0 à 8. On accepte alors n'importe quelle quantité, et le
  // compteur ne promet pas un total qui n'existe pas.
  const k = d.a_choisir;
  const libre = k === undefined || k === null;
  const largeur = mesurer(d, zone, options.length);
  const choisis = new Set();

  const compteur = document.createElement("span");
  compteur.className = "barre__compte";

  const valider = document.createElement("button");
  valider.className = "valider";
  valider.type = "button";
  valider.dataset.valider = "";
  valider.textContent = MOT.confirm;

  const rafraichir = () => {
    compteur.textContent = libre
      ? `${choisis.size} / ${options.length} picked`
      : `${choisis.size} / ${k} picked`;
    valider.classList.toggle("valider--prete", libre || choisis.size === k);
  };

  options.forEach((o, i) => {
    const b = choix(d, o, i, largeur, etat);
    b.addEventListener("click", () => {
      if (choisis.has(i)) choisis.delete(i);
      else choisis.add(i);
      b.classList.toggle("choix--pris", choisis.has(i));
      rafraichir();
    });
    zone.appendChild(b);
  });

  // Le bouton n'est jamais `disabled` : un bouton désactivé est un bouton que
  // l'on ne peut pas atteindre. Il refuse simplement tant que le compte n'y est
  // pas, et le dit.
  valider.addEventListener("click", () => {
    if (!libre && choisis.size !== k) {
      valider.classList.remove("valider--refus");
      void valider.offsetWidth;
      valider.classList.add("valider--refus");
      return;
    }
    repondre([...choisis]);
  });

  barre.appendChild(compteur);
  barre.appendChild(valider);
  rafraichir();
}

/** Montant : un entier entre deux bornes que le moteur donne. */
function montant(d, zone, barre) {
  const min = d.minimum ?? 0;
  const max = d.maximum ?? 0;

  const c = document.createElement("div");
  c.className = "cadran";
  c.innerHTML = `
    <button class="cadran__pas" type="button" data-pas="-1">−</button>
    <input class="cadran__champ" data-montant type="number" inputmode="numeric">
    <button class="cadran__pas" type="button" data-pas="1">+</button>
    <span class="cadran__bornes">from ${min} to ${max}</span>`;
  const champ = c.querySelector("[data-montant]");
  champ.min = String(min);
  champ.max = String(max);
  champ.value = String(min);
  zone.appendChild(c);

  const borne = (v) => Math.max(min, Math.min(max, v));
  // (regles-de-la-vente) Le cadran ne montrait ce qu'il COÛTE que pour la vente
  // d'office : les cartes qui allaient partir suivaient le nombre. Cette
  // décision n'existe plus, et le seul montant qui reste (« spend any amount »)
  // ne fait partir aucune carte. Les deux boutons se contentent donc de borner
  // le nombre.
  for (const b of c.querySelectorAll("[data-pas]")) {
    b.addEventListener("click", () => {
      champ.value = String(borne(Number(champ.value || min) + Number(b.dataset.pas)));
    });
  }

  const valider = document.createElement("button");
  valider.className = "valider valider--prete";
  valider.type = "button";
  valider.dataset.valider = "";
  valider.textContent = MOT.confirm;
  valider.addEventListener("click", () => {
    const v = Number(champ.value);
    if (!Number.isInteger(v) || v < min || v > max) {
      champ.classList.remove("cadran__champ--refus");
      void champ.offsetWidth;
      champ.classList.add("cadran__champ--refus");
      return;
    }
    repondre(v);
  });
  barre.appendChild(valider);
}

// ------------------------------------------------- la fabrique d'un choix

/**
 * Un choix, dans la matière la plus parlante que l'option porte :
 * une carte a son image, une phase a sa carte Phase, un badge a son jeton,
 * une action nommée « Action de X » a l'image de la carte X. À défaut, une
 * plaque gravée qui porte le libellé du moteur, mot pour mot.
 */
function choix(d, o, i, largeur, etat) {
  let b;
  const c = normaliser(o);
  const mot = libelleOption(d, o, i, c, etat);

  if (d.type === "pick_phase" && o.phase) {
    b = document.createElement("button");
    b.type = "button";
    b.className = "choix choix--phase";
    // LA CARTE QU'ON CHOISIT EST CELLE QU'ON POSSÈDE. Alexis, le 04-08 :
    // « quand j'améliore une carte phase, parmi les cartes phase que je
    // choisis, j'ai toujours le choix parmi les cartes phases de base (les
    // nouveaux designs ne s'affichent pas) ».
    //
    // Il avait raison, et la cause était ici : l'image était `imagePhase(n)`,
    // la carte de base, sans jamais regarder ce que le joueur avait amélioré.
    // Le moteur le publie pourtant en clair (`players[].phase_upgrades`, une
    // liste d'étiquettes comme « 2B » = phase 2, amélioration B — voir
    // `engine/src/state.rs:499`). On ne DÉDUIT rien : on lit son étiquette et
    // on montre la carte qu'elle nomme.
    const code = ameliorationPossedee(etat, d.joueur, o.phase);
    const src = (code && imageAmelioration(code)) || imagePhase(o.phase);
    const dit = code
      ? `upgraded Phase card ${phaseNom(o.phase)} ${code.slice(1)}`
      : `Phase card ${phaseNom(o.phase)}`;
    const im = document.createElement("img");
    im.src = src;
    im.alt = dit;
    im.draggable = false;
    b.appendChild(im);
    if (code) {
      b.classList.add("choix--amelioration");
      b.dataset.variante = code.slice(1);
    }
    b.dataset.phase = String(o.phase);
    // Une carte Phase est une carte : elle s'agrandit au survol comme les autres.
    survolableImage(b, src, dit);
    const t = document.createElement("span");
    t.className = "choix__mot";
    t.textContent = mot;
    b.appendChild(t);
  } else if (d.type === "amelioration_carte_phase") {
    // ON VOIT LA CARTE. L'image est celle du couple (phase, variante) que
    // l'option porte ; les deux champs sont posés sur le bouton pour que la
    // correspondance soit vérifiable depuis l'extérieur de la page.
    //
    // Cette branche prend TOUTES les options de cette nature, même celle qui
    // n'aurait pas ses deux champs : le repli est une plaque, jamais le chemin
    // des cartes — l'option porte un `nom` FRANÇAIS (« Research (phase
    // améliorée A) ») que `cartes.js` écrirait sous un dos de carte.
    const code = codeAmelioration(o);
    if (!code) {
      b = slab(mot, "amelioration");
    } else {
      b = document.createElement("button");
      b.type = "button";
      b.className = "choix choix--phase choix--amelioration";
      const src = imageAmelioration(code);
      const im = document.createElement("img");
      im.src = src;
      im.alt = `upgraded Phase card ${phaseNom(o.phase)} ${o.variante}`;
      im.draggable = false;
      b.appendChild(im);
      // « Lors d'améliorations de cartes phases, quand on passe le curseur
      // dessus, ça ne fait pas de zoom comme d'habitude » (02-08). Voilà.
      survolableImage(b, src, `upgraded Phase card ${phaseNom(o.phase)} ${o.variante}`);
      const t = document.createElement("span");
      t.className = "choix__mot";
      t.textContent = mot;
      b.appendChild(t);
      b.dataset.phase = String(o.phase);
      b.dataset.variante = String(o.variante);
    }
  } else if (d.type === "pick_joker_tag" && o.badge) {
    b = slab(mot, "badge");
    const im = document.createElement("img");
    im.className = "choix__jeton";
    im.src = imageBadge(o.badge, true);
    im.alt = nomBadge(o.badge);
    b.prepend(im);
  } else if (c) {
    b = document.createElement("button");
    b.type = "button";
    b.className = "choix choix--carte";
    // La corporation est le sujet même de sa question : on la montre GRANDE,
    // pas en vignette, et la loupe n'a plus rien à ajouter par-dessus.
    const grande = d.type === "pick_corporation";
    b.appendChild(carte(o, { classe: grande ? "carte--choix carte--corpo-choix" : "carte--choix" }));
    if (!grande) survolable(b, o);
    const t = document.createElement("span");
    t.className = "choix__mot";
    t.textContent = mot;
    b.appendChild(t);
    // Une option QUI EST une carte porte son identifiant, et se déclare jouable :
    // c'est le moteur qui vient de l'énumérer, la page ne fait que le recopier.
    //
    // LEQUEL DES DEUX IDENTIFIANTS ? `data-carte-id` désigne une carte qu'on
    // TIENT, `data-carte-en-jeu` une carte déjà POSÉE — c'est la répartition que
    // ce dépôt emploie depuis toujours (`vue/mains.js` et `vue/plateau.js`).
    // Toute carte de la main se joue désormais depuis la main : une option-carte
    // qui arrive jusqu'ici, sur une décision simple, est nécessairement une carte
    // en jeu (prendre ou poser une ressource, rejouer une production). Elle porte
    // donc la marque des cartes en jeu. Les décisions à choix multiple, elles,
    // parlent bien de cartes qu'on tient ou qu'on vient de piocher.
    //
    // UNE CORPORATION N'EST NI L'UN NI L'AUTRE : elle n'est pas tenue, elle
    // n'est pas posée, elle est PRÉSENTÉE. Elle porte donc sa propre marque,
    // `data-corpo-choix`, et jamais celle des cartes qu'on tient — sans quoi une
    // machine qui pilote la page confondrait son numéro avec celui d'une carte
    // projet, les deux comptages étant distincts (voir `vue/cartes.js`).
    // COUTURE — le contrôle `23` (banc `verif/identifiants.py`, commun aux trois
    // chantiers) prend l'écran EN DÉFAUT ici, et lui seul : sur la graine 31337,
    // pendant la mise en place, la corporation « Inventrix » est présentée au
    // milieu sous le numéro 7 pendant que la main tient la carte projet
    // « Arctic Algae », numéro 7 elle aussi. Deux cartes, un seul numéro à
    // l'écran — exactement ce que le paragraphe ci-dessus dit qu'il ne faut pas.
    // La règle était écrite, l'attribut ne la portait pas : il reçoit donc le
    // COUPLE que `vue/cartes.js` fabrique déjà pour la main (`data-carte-cle`),
    // au lieu du numéro nu. Rien de neuf n'est ajouté — c'est l'identifiant que
    // ce dépôt emploie partout ailleurs, appliqué au dernier endroit qui l'avait
    // manqué. `verif/corporation.py` (contrôle `22`) lit cet attribut comme une
    // valeur opaque et clique dessus : il s'accommode du couple sans retouche.
    if (c.id !== null && c.id !== undefined) {
      if (grande) b.dataset.corpoChoix = cle(o) ?? String(c.id);
      else if (d.multiple) b.dataset.carteId = String(c.id);
      else b.dataset.carteEnJeu = String(c.id);
      b.dataset.jouable = "oui";
    }
  } else {
    b = slabAction(o, mot);
  }

  // LIS-11 — LE PRIX D'ORIGINE, BARRÉ, À CÔTÉ DU PRIX RÉELLEMENT PAYÉ.
  //
  // Quand une remise s'applique (`reduction_microbes`, `reduction_plantes`),
  // l'option ne disait que le RABAIS — « 10 MC off » — et jamais ce qu'on
  // allait finir par payer. Le joueur devait faire la soustraction de tête,
  // en retrouvant lui-même le prix imprimé de la carte.
  //
  // Les deux nombres viennent du moteur et d'aucun calcul d'écran : le prix
  // imprimé est `d.carte.prix`, le rabais `o.reduction_mc`. La seule opération
  // est leur différence, qui est la définition même d'une remise. On borne à
  // zéro : le moteur ne fait jamais payer un prix négatif.
  const plein = d.carte && typeof d.carte.prix === "number" ? d.carte.prix : null;
  const rabais = typeof o.reduction_mc === "number" ? o.reduction_mc : 0;
  if (plein !== null && rabais > 0) {
    const bloc = document.createElement("span");
    bloc.className = "choix__prix";
    // `<s>` et non un simple style : ce qui est barré doit l'être aussi pour
    // qui lit la page autrement qu'avec les yeux.
    const avant = document.createElement("s");
    avant.className = "choix__prix--plein";
    avant.textContent = `${plein} MC`;
    const apres = document.createElement("b");
    apres.className = "choix__prix--paye";
    apres.textContent = `${Math.max(0, plein - rabais)} MC`;
    // Les deux nombres déclarent d'où ils viennent, comme tout nombre de
    // l'écran : de la carte que le moteur nomme dans cette décision.
    bloc.dataset.prixCarte = String(d.carte.nom ?? d.carte.id ?? "");
    bloc.appendChild(avant);
    bloc.appendChild(apres);
    b.appendChild(bloc);
  }

  b.dataset.choix = String(i);
  b.style.setProperty("--w", largeur + "px");
  return b;
}

/** Une plaque gravée : le libellé du moteur, en grand. */
function slab(texte, sorte) {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "choix choix--plaque choix--" + sorte;
  const t = document.createElement("span");
  t.className = "choix__mot";
  t.textContent = texte;
  b.appendChild(t);
  return b;
}

// Les actions courantes du jeu portent un nom stable : on leur rend leur
// matière. Le libellé ANGLAIS reste écrit en toutes lettres sur la plaque —
// l'image ne le remplace pas, elle l'accompagne. La reconnaissance, elle, se
// fait sur le libellé brut du moteur, par la table explicite de `mots.js`.
function matiereAction(brut) {
  const s = sorteAction(brut || "");
  if (!s) return null;
  if (s.carte) {
    const im = imageCarte(s.carte);
    return im ? { src: im, sorte: "carte" } : null;
  }
  // Les deux jetons hexagonaux sont pris DÉTOURÉS : sur une plaque de décision,
  // le rectangle blanc du scan se voyait (04-08).
  if (s.jeton === "foret") return { src: jetonForetDetoure(), sorte: "jeton" };
  if (s.jeton === "ocean") return { src: jetonOceanDetoure(), sorte: "jeton" };
  if (s.jeton === "chaleur") return { src: imageReserve("heat"), sorte: "jeton" };
  // « Défausser 1 carte pour du MC » : c'est une carte PROJET qui part.
  if (s.jeton === "dos") return { src: dosProjet(), sorte: "carte" };
  return null;
}

function slabAction(o, mot) {
  const b = slab(mot, "action");
  const m = matiereAction(o && (o.libelle ?? o.nom ?? o.name));
  if (m) {
    const im = document.createElement("img");
    im.className = "choix__matiere choix__matiere--" + m.sorte;
    im.src = m.src;
    im.alt = "";
    b.prepend(im);
    b.dataset.matiere = m.sorte;
  }
  return b;
}

// -------------------------------------------------------------- la géométrie

/**
 * LA RÈGLE DE FER : les choix ne se recouvrent jamais et ne débordent jamais de
 * leur zone. Un choix recouvert n'est pas cliquable — Playwright le refuse, et
 * une main humaine le rate. On ne s'en remet donc pas au retour à la ligne d'un
 * flex (qui dépend d'un demi-pixel) : on POSE une grille au nombre de colonnes
 * calculé, et l'on vérifie que les rangs tiennent en hauteur.
 */
function mesurer(d, zone, n) {
  const illustre = choixSontDesImages(d);
  const L = zone.clientWidth || 1040;
  const H = hauteurDeLaZone(zone);
  let plan;
  if (!illustre) {
    plan = planPlaques(L, H, n);
  } else {
    // La ligne de nom est réservée d'abord ; si les cartes tombent malgré tout
    // sous 170 points, `data-dense` efface le mot et la place qu'il retenait
    // revient aux cartes. Une carte Phase améliorée, elle, garde son mot — la
    // variante A ou B n'est écrite nulle part ailleurs — donc sa ligne aussi.
    plan = planImages(L, H, n, true);
    if (plan.w < 170 && d.type !== "amelioration_carte_phase") {
      const sansMot = planImages(L, H, n, false);
      if (sansMot.w < 170) plan = sansMot;
    }
  }

  zone.style.gridTemplateColumns = `repeat(${plan.c}, ${plan.w}px)`;
  zone.style.setProperty("--w", plan.w + "px");
  zone.style.setProperty("--hp", Math.floor(plan.h) + "px");
  zone.dataset.dense = illustre && plan.w < 170 ? "oui" : "non";
  zone.dataset.sorte = illustre ? "images" : "plaques";
  return plan.w;
}

/**
 * (regles-de-la-vente, round 2) **LA HAUTEUR QU'A VRAIMENT LA ZONE DES CHOIX.**
 *
 * On mesure AVANT de poser les choix : la zone est donc vide, et comme elle
 * grandit pour prendre la place libre (`flex: 1 1 auto`), sa hauteur d'alors est
 * exactement la place qui lui reste. Zéro est une réponse JUSTE : elle veut dire
 * que l'entête et le contexte ont déjà tout pris.
 *
 * Ce zéro était remplacé par 470 px. Mesuré à 1280×720, graine 2024, rang 352
 * (`pick_joker_tag`, dix options) : bande de décision de 173 px, contexte de
 * 116, zone des choix de **10 px** — et un plan calculé pour 470, donc des
 * plaques de **229 px** posées deux rangées, débordant de y 207 à y 677. Ma main
 * commence à y 566 : ses trois cartes étaient recouvertes par les boutons de la
 * scène, donc INVENDABLES (contrôle 06), et le clic de Playwright était
 * intercepté. Le nombre inventé faisait dessiner quarante-sept fois la place
 * disponible.
 *
 * On ne garde donc le repli que pour le seul cas où la mesure ne veut vraiment
 * rien dire : une zone qui n'est pas encore dans une page mise en page. Sinon on
 * rend 1 px, et ce sont les branches de DERNIER RECOURS de `planImages` et
 * `planPlaques` qui répondent — des choix serrés, illisibles peut-être, mais
 * DANS leur bande. Un choix trop petit reste cliquable ; un choix qui déborde
 * sur la main rend deux gestes impossibles au lieu d'un.
 */
function hauteurDeLaZone(zone) {
  const h = zone.clientHeight;
  if (h > 0) return h;
  const parent = zone.parentElement;
  return parent && parent.clientHeight > 0 ? 1 : 470;
}

/** La largeur utile d'une colonne, marge de sûreté comprise. */
function colonne(L, c) {
  return Math.floor((L - (c - 1) * ECART - 2) / c);
}

/**
 * LA HAUTEUR DE RANGÉE qu'exige un bouton de `cote` points de côté. Une plaque
 * fait exactement sa hauteur ; une carte y ajoute ses proportions, et sa ligne
 * de nom quand elle en garde une.
 */
function rangeePour(cote, images, legende) {
  return images ? cote * RATIO + (legende ? LEGENDE : 0) : cote;
}

/**
 * LA HAUTEUR MINIMALE d'une bande qui tient `n` boutons de `cote` points de
 * côté sur `L` points de large. On essaie toutes les découpes en colonnes et on
 * garde la moins haute — c'est ce nombre que `dessiner` réserve à la bande des
 * choix AVANT de laisser le contexte prendre le reste.
 */
function hauteurPourTenir(L, n, images, legende, cote) {
  if (n <= 0) return 0;
  const hRang = rangeePour(cote, images, legende);
  let mieux = null;
  for (let c = 1; c <= n; c++) {
    if (colonne(L, c) < cote) continue;
    const r = Math.ceil(n / c);
    const h = r * hRang + (r - 1) * ECART;
    if (mieux === null || h < mieux) mieux = h;
  }
  // Même une colonne unique est trop étroite : la largeur ne se négocie pas
  // ici, on rend la hauteur d'une rangée plutôt qu'un nombre infini.
  return Math.ceil(mieux === null ? hRang : mieux);
}

/**
 * CE QUI RESTE AU CONTEXTE quand il n'est qu'un rappel. Trois règles, dans cet
 * ordre : les choix ont leur plancher quoi qu'il arrive ; ils ont leur aise si
 * la place le permet ; et le contexte ne descend pas sous `CTX_MINI` tant que
 * les deux premières sont tenues.
 */
function partDuContexte(dispo, plancher, aise) {
  const disponible = Math.max(0, dispo - plancher);
  const naturelle = Math.round(CTX_VIGNETTE * RATIO) + CTX_CHROME;
  let h = Math.min(naturelle, Math.max(0, dispo - aise));
  h = Math.max(h, Math.min(naturelle, CTX_MINI, disponible));
  return Math.min(h, disponible);
}

/**
 * DERNIER RECOURS — la bande est plus basse que le plancher du contrat.
 *
 * Il ne doit plus se produire : `dessiner` réserve à la bande la hauteur que
 * `hauteurPourTenir` exige avant de la remplir. S'il se produisait quand même,
 * l'ancienne branche empilait plusieurs rangées dans une bande qui n'en tenait
 * qu'une — d'où les 24 paires de boutons recouvertes en 1450 × 800. On préfère
 * désormais la découpe la MOINS haute qui respecte les 40 points : des boutons
 * qui dépassent un peu de leur bande restent cliquables, des boutons empilés
 * les uns sur les autres, non.
 *
 * CE QU'IL NE PEUT PAS FAIRE, ET POURQUOI. Il prend le plus grand nombre de
 * colonnes qui tiennent encore 40 points de large, donc le plus petit nombre de
 * rangées possible, donc la disposition la moins haute qui respecte le contrat.
 * Si cette hauteur-là dépasse encore la bande, la grille déborde : c'est la
 * situation que la clause ASK du contrat décrit, et aucune mise en page ne s'en
 * sort — « tous les choix visibles », « 40 points minimum » et « dans la bande »
 * sont alors contradictoires. Elle demande beaucoup de choix sur une fenêtre
 * basse : en 1100 × 620, la plus petite du contrat, la bande vaut 104 points et
 * la largeur utile 858, soit jusqu'à 19 colonnes de 45 points ; il faudrait plus
 * de 38 choix sur un seul écran pour que deux rangées ne tiennent plus. Le
 * balayage des quatorze tailles sur des parties entières (2 912 écrans,
 * `outputs/work/balayage-complet.py`) en a rencontré QUINZE au plus. Et comme
 * `dessiner` réserve exactement `hauteurPourTenir`, la boucle ci-dessus trouve
 * toujours au moins la découpe que cette réservation a payée : sous 38 choix,
 * cette branche est inatteignable. Elle reste écrite parce qu'un plan qui rend
 * `null` serait un écran blanc.
 */
function planSerre(L, n, cote, hRang) {
  let c = 1;
  for (let k = n; k >= 1; k--) {
    if (colonne(L, k) >= cote) { c = k; break; }
  }
  return { c, utile: cote, w: cote, h: Math.ceil(hRang) };
}

/** Des cartes : elles gardent leurs proportions, et le rang doit tenir en hauteur. */
function planImages(L, H, n, legende) {
  const leg = legende ? LEGENDE : 0;
  let mieux = null;
  for (let c = 1; c <= n; c++) {
    const r = Math.ceil(n / c);
    const w = Math.min(colonne(L, c), MAX_CARTE);
    // hauteur d'une rangée = la carte + sa ligne de nom
    const h = (H - (r - 1) * ECART) / r - leg;
    if (h <= 0 || w <= 0) continue;
    const utile = Math.min(w, h / RATIO);
    if (utile < MINI) continue; // sous le plancher du contrat : refusé
    if (!mieux || utile > mieux.utile) {
      mieux = { c, utile, w: Math.floor(utile), h: Math.floor(utile * RATIO + leg) };
    }
  }
  return mieux || planSerre(L, n, MINI, rangeePour(MINI, true, legende));
}

/** Des plaques : pas de proportion imposée, on cherche à REMPLIR la scène. */
function planPlaques(L, H, n) {
  let mieux = null;
  for (let c = 1; c <= n; c++) {
    const r = Math.ceil(n / c);
    const w = colonne(L, c);
    const h = Math.min((H - (r - 1) * ECART) / r, 176);
    if (w < MINI || h < MINI) continue; // sous le plancher du contrat : refusé
    const aire = w * h;
    if (!mieux || aire > mieux.aire) mieux = { c, w, h: Math.floor(h), aire };
  }
  return mieux || planSerre(L, n, MINI, MINI);
}

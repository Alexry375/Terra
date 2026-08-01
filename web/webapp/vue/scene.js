// LA SCÈNE — l'endroit où une décision est posée.
//
// La page n'invente aucun choix : elle n'offre QUE `decision.options`, dans
// l'ordre où le moteur les a énumérées, et rend au moteur l'indice choisi.
// Elle ne juge jamais si un choix est légal — si le moteur l'a énuméré, il l'est.
//
// Deux zones : le CONTEXTE (ce que la décision donne à voir : une main, deux
// corporations, la carte concernée — jamais cliquable) et les CHOIX (un élément
// `data-choix` par option, dans l'ordre des indices, « passer » en dernier).

import { carte, normaliser, libelle } from "./cartes.js";
import {
  imagePhase, phaseNom, phaseRomain, imageBadge, nomBadge, imageCarte,
  imageForet, imageOcean, imageReserve, dosDeCarte, EQUIPAGES,
} from "./materiel.js";
import { survolable, cacher as cacherLoupe, figer } from "./loupe.js";

const RATIO = 569 / 409; // les images de cartes, telles qu'elles ont été découpées
const ECART = 12;
const MAX_CARTE = 340; // au-delà, une carte mange la scène sans rien ajouter

let resoudre = null; // la réponse attendue par le moteur, une fois cliquée

/** Le squelette de la scène. Appelé une fois. */
export function construireScene() {
  const m = document.createElement("main");
  m.id = "scene";
  document.body.appendChild(m);
}

/**
 * Pose une décision et rend une promesse résolue avec la réponse attendue.
 * @param {object} d     le descripteur du moteur
 * @param {object} etat  l'état au moment de cette décision (contexte visuel)
 */
export function poserDecision(d, etat) {
  return new Promise((ok) => {
    resoudre = ok;
    dessiner(d, etat);
  });
}

/** Efface la scène : plus aucun `data-choix` ne subsiste (fin de partie). */
export function viderScene() {
  const m = document.getElementById("scene");
  if (m) m.textContent = "";
  fermerScene();
}

/** Referme la scène : les attributs de décision doivent disparaître aussitôt. */
export function fermerScene() {
  const m = document.getElementById("scene");
  m.removeAttribute("data-decision-rang");
  m.removeAttribute("data-decision-forme");
  m.removeAttribute("data-a-choisir");
  cacherLoupe();
  figer();
}

function repondre(r) {
  const f = resoudre;
  resoudre = null;
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

  m.appendChild(entete(d));

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
  m.appendChild(barre);

  // En composition « contexte », les choix ne sont que des mots : on leur
  // réserve leur hauteur AVANT de mesurer le contexte, sinon les deux zones se
  // disputent la place et les cartes débordent.
  if (!riche && ctx) {
    const n = (d.options || []).length + (d.passer ? 1 : 0);
    zone.style.height = (forme === "montant" ? 108 : n <= 4 ? 172 : 312) + "px";
    zone.style.flex = "0 0 auto";
  }

  if (ctx) remplirContexte(ctx, !riche);

  if (forme === "montant") montant(d, zone, barre);
  else if (forme === "multiple") multiple(d, zone, barre);
  else simple(d, zone);
}

function entete(d) {
  const e = document.createElement("div");
  e.className = "scene__entete";
  e.innerHTML =
    `<span class="scene__qui" style="--teinte:${EQUIPAGES[d.joueur].teinte}">` +
    `<i>J${d.joueur}</i>${EQUIPAGES[d.joueur].nom}</span>` +
    `<h1 class="scene__question"></h1>`;
  e.querySelector(".scene__question").textContent = d.question;
  return e;
}

/** Les options portent-elles leur propre image ? */
function optionsIllustrees(d) {
  const o = (d.options || [])[0];
  if (!o) return false;
  if (d.type === "pick_phase" || d.type === "pick_joker_tag") return true;
  return !!normaliser(o);
}

/** L'image qui portera l'atmosphère de la scène. */
function imageSujet(d) {
  const c = normaliser(d.carte) || normaliser((d.options || [])[0]);
  if (c) {
    const im = imageCarte(c.nom);
    if (im) return im;
  }
  if (d.type === "pick_phase") return imagePhase(1);
  return null;
}

// ---------------------------------------------------------------- le contexte

/** Ce que la décision donne à voir sans qu'on ait à le choisir. */
function contexte(d) {
  const cartes = [];
  let mot = "";

  if (d.carte) {
    cartes.push(d.carte);
    mot = "carte en cours";
  } else if (d.corporations) {
    cartes.push(...d.corporations);
    mot = "vos corporations";
  } else if (d.main) {
    cartes.push(...d.main);
    mot = "votre main";
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

  if (d.type === "discard_payment_count") {
    const n = document.createElement("p");
    n.className = "scene__note";
    n.textContent = `coût ${d.cout} MC · vous avez ${d.mc} MC · ${d.taux} MC par carte défaussée`;
    z.appendChild(n);
  }
  return z;
}

/**
 * Le contexte prend la taille que la composition lui laisse. Quand c'est LUI le
 * sujet (les options ne sont que des mots), il occupe la scène.
 */
function remplirContexte(z, maitre) {
  const r = z.querySelector(".scene__contexte-rang");
  if (!r) return;
  const n = Number(r.dataset.combien) || 1;
  if (!maitre) {
    r.style.setProperty("--w", "68px");
    r.style.setProperty("--serrage", "-22px");
    return;
  }
  const L = r.clientWidth || 1040;
  const H = r.clientHeight || 300;
  // Les cartes du contexte peuvent se chevaucher : on ne clique pas dessus.
  const pas = n > 6 ? 0.62 : n > 3 ? 0.82 : 1;
  let w = Math.min((L - 24) / (1 + (n - 1) * pas), H / RATIO, 300);
  w = Math.max(60, Math.floor(w));
  r.style.setProperty("--w", w + "px");
  r.style.setProperty("--serrage", Math.round(-w * (1 - pas)) + "px");
}

// ------------------------------------------------------------------ les choix

/** Choix simple : un élément cliquable par option, « passer » en dernier. */
function simple(d, zone) {
  const options = d.options || [];
  const total = options.length + (d.passer ? 1 : 0);

  const largeur = mesurer(d, zone, total);
  options.forEach((o, i) => {
    const b = choix(d, o, i, largeur);
    b.addEventListener("click", () => repondre(i));
    zone.appendChild(b);
  });

  if (d.passer) {
    const b = slab("Passer", "passer");
    b.dataset.choix = String(options.length);
    b.classList.add("choix--passer");
    b.style.setProperty("--w", largeur + "px");
    b.setAttribute("aria-label", "passer");
    b.addEventListener("click", () => repondre(options.length));
    zone.appendChild(b);
  }
}

/** Choix multiple : on sélectionne, puis on valide. */
function multiple(d, zone, barre) {
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
  valider.textContent = "Valider";

  const rafraichir = () => {
    compteur.textContent = libre
      ? `${choisis.size} / ${options.length} sélectionnée${choisis.size > 1 ? "s" : ""}`
      : `${choisis.size} / ${k} sélectionnée${k > 1 ? "s" : ""}`;
    valider.classList.toggle("valider--prete", libre || choisis.size === k);
  };

  options.forEach((o, i) => {
    const b = choix(d, o, i, largeur);
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
    <span class="cadran__bornes">de ${min} à ${max}</span>`;
  const champ = c.querySelector("[data-montant]");
  champ.min = String(min);
  champ.max = String(max);
  champ.value = String(min);
  zone.appendChild(c);

  const borne = (v) => Math.max(min, Math.min(max, v));
  for (const b of c.querySelectorAll("[data-pas]")) {
    b.addEventListener("click", () => {
      champ.value = String(borne(Number(champ.value || min) + Number(b.dataset.pas)));
    });
  }

  const valider = document.createElement("button");
  valider.className = "valider valider--prete";
  valider.type = "button";
  valider.dataset.valider = "";
  valider.textContent = "Valider";
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
function choix(d, o, i, largeur) {
  let b;
  const c = normaliser(o);

  if (d.type === "pick_phase" && o.phase) {
    b = document.createElement("button");
    b.type = "button";
    b.className = "choix choix--phase";
    const im = document.createElement("img");
    im.src = imagePhase(o.phase);
    im.alt = "carte Phase " + phaseNom(o.phase);
    b.appendChild(im);
    const t = document.createElement("span");
    t.className = "choix__mot";
    t.textContent = `${phaseRomain(o.phase)} · ${phaseNom(o.phase)}`;
    b.appendChild(t);
  } else if (d.type === "pick_joker_tag" && o.badge) {
    b = slab(libelle(o), "badge");
    const im = document.createElement("img");
    im.className = "choix__jeton";
    im.src = imageBadge(o.badge, true);
    im.alt = nomBadge(o.badge);
    b.prepend(im);
  } else if (c) {
    b = document.createElement("button");
    b.type = "button";
    b.className = "choix choix--carte";
    b.appendChild(carte(o, { classe: "carte--choix" }));
    survolable(b, o);
    const t = document.createElement("span");
    t.className = "choix__mot";
    t.textContent = libelle(o, c.nom);
    b.appendChild(t);
  } else {
    b = slabAction(libelle(o, `option ${i + 1}`));
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
// matière. Le libellé du moteur reste écrit en toutes lettres sur la plaque —
// l'image ne le remplace pas, elle l'accompagne.
function matiereAction(texte) {
  if (texte.startsWith("Action de ")) {
    const im = imageCarte(texte.slice("Action de ".length));
    if (im) return { src: im, sorte: "carte" };
  }
  if (texte.startsWith("Forêt")) return { src: imageForet(), sorte: "jeton" };
  if (texte.startsWith("Océan")) return { src: imageOcean(0), sorte: "jeton" };
  if (texte.startsWith("Température")) return { src: imageReserve("heat"), sorte: "jeton" };
  if (texte.startsWith("Défausser")) return { src: dosDeCarte(), sorte: "carte" };
  return null;
}

function slabAction(texte) {
  const b = slab(texte, "action");
  const m = matiereAction(texte);
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
  const illustre = optionsIllustrees(d);
  const L = zone.clientWidth || 1040;
  const H = zone.clientHeight || 470;
  const plan = illustre ? planImages(L, H, n) : planPlaques(L, H, n);

  zone.style.gridTemplateColumns = `repeat(${plan.c}, ${plan.w}px)`;
  zone.style.setProperty("--w", plan.w + "px");
  zone.style.setProperty("--hp", Math.floor(plan.h) + "px");
  zone.dataset.dense = illustre && plan.w < 170 ? "oui" : "non";
  zone.dataset.sorte = illustre ? "images" : "plaques";
  return plan.w;
}

/** La largeur utile d'une colonne, marge de sûreté comprise. */
function colonne(L, c) {
  return Math.floor((L - (c - 1) * ECART - 2) / c);
}

/** Des cartes : elles gardent leurs proportions, et le rang doit tenir en hauteur. */
function planImages(L, H, n) {
  let mieux = { c: n, w: 40, h: 60 };
  for (let c = 1; c <= n; c++) {
    const r = Math.ceil(n / c);
    const w = Math.min(colonne(L, c), MAX_CARTE);
    // hauteur d'une rangée = la carte + la ligne de légende
    const h = (H - (r - 1) * ECART) / r - 20;
    if (h <= 0) continue;
    const utile = Math.min(w, h / RATIO);
    if (utile > mieux.w) mieux = { c, w: Math.floor(utile), h: Math.floor(utile * RATIO + 20) };
  }
  return mieux;
}

/** Des plaques : pas de proportion imposée, on cherche à REMPLIR la scène. */
function planPlaques(L, H, n) {
  let mieux = null;
  for (let c = 1; c <= n; c++) {
    const r = Math.ceil(n / c);
    const w = colonne(L, c);
    const h = Math.min((H - (r - 1) * ECART) / r, 176);
    if (w < 200 || h < 62) continue;
    const aire = w * h;
    if (!mieux || aire > mieux.aire) mieux = { c, w, h: Math.floor(h), aire };
  }
  if (!mieux) {
    // Cas extrême : on serre en autant de colonnes qu'il faut pour tenir.
    const c = Math.max(1, Math.ceil(n / Math.max(1, Math.floor(H / 62))));
    mieux = { c, w: colonne(L, c), h: Math.max(48, Math.floor(H / Math.ceil(n / c)) - ECART) };
  }
  return mieux;
}

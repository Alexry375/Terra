// L'AIDE — les quinze faces des cartes Phase, et rien d'autre.
//
// La demande du joueur, mot pour mot : « une aide pour voir par exemple
// l'ensemble des cartes phases améliorées ». Les cinq faces normales et les dix
// faces améliorées sont PUBLIQUES : elles sont identiques pour les deux joueurs,
// posées au milieu de la table, et connaître leur contenu ne donne aucun
// avantage. C'est la seule raison pour laquelle cette aide a le droit d'exister.
//
// CE QU'ELLE NE MONTRE JAMAIS : la pioche, la main de l'adversaire, une tuile
// Océan non révélée, une corporation non encore posée. Une aide qui montrerait
// « toutes les cartes du jeu » serait une aide qui triche.
//
// Elle ne nomme pas non plus les cinq phases en toutes lettres : « Research »
// est aussi le nom d'une carte projet, et l'écrire ferait entrer dans la page un
// nom de carte que le jeu n'avait pas montré. Les images portent leur propre
// nom, imprimé dessus ; on les désigne par leur chiffre romain.

import { imagePhase, imageAmelioration, phaseRomain } from "./materiel.js";
import { MOT } from "./mots.js";

// Les trois faces d'une même phase, dans l'ordre où on les découvre : la carte
// telle qu'on la reçoit, puis ses deux améliorations.
const FACES = [
  { cle: null, mot: MOT.faceStandard },
  { cle: "A", mot: MOT.faceUpgradeA },
  { cle: "B", mot: MOT.faceUpgradeB },
];

let panneau = null;
let loupe = null;
let loupeImage = null;
let loupeMot = null;

/** Le nom du fichier d'une image, sans son dossier — ce que les contrôles lisent. */
function nomDeFichier(src) {
  return String(src).split("/").pop();
}

/**
 * Une face de carte Phase : sa vignette, son image et sa légende.
 * @param {number} n      le numéro de la phase (1 à 5)
 * @param {object} face   l'une des trois faces
 */
function vignette(n, face) {
  // Les chemins viennent du manifeste du matériel (`vue/materiel.js`) : aucun
  // nom de fichier n'est écrit en dur ici.
  const src = face.cle ? imageAmelioration(`${n}${face.cle}`) : imagePhase(n);
  if (!src) return null;

  const f = document.createElement("figure");
  f.className = "aide__carte";
  f.dataset.aideCarte = nomDeFichier(src);
  f.tabIndex = 0;

  const im = document.createElement("img");
  im.src = src;
  im.alt = `${MOT.helpTitle} ${phaseRomain(n)} — ${face.mot}`;
  im.draggable = false;
  f.appendChild(im);

  const l = document.createElement("figcaption");
  l.textContent = `${phaseRomain(n)} · ${face.mot}`;
  f.appendChild(l);

  const montrer = () => agrandir(f, src, im.alt);
  f.addEventListener("mouseenter", montrer);
  f.addEventListener("focus", montrer);
  f.addEventListener("click", montrer);
  return f;
}

/** Pose une carte dans la loupe de l'aide. */
function agrandir(f, src, mot) {
  if (!loupe) return;
  for (const autre of panneau.querySelectorAll(".aide__carte--pointee")) {
    autre.classList.remove("aide__carte--pointee");
  }
  f.classList.add("aide__carte--pointee");
  loupeImage.src = src;
  loupeImage.alt = mot;
  loupeMot.textContent = f.querySelector("figcaption").textContent;
  loupe.dataset.aideAgrandie = f.dataset.aideCarte;
}

/**
 * Le panneau d'aide, bâti une seule fois. Il vit DANS le panneau d'options :
 * les quatre entrées (reprendre, aide, réglages, retour au menu) restent donc
 * atteignables pendant qu'on le lit.
 */
export function vueAide() {
  if (panneau) return panneau;

  panneau = document.createElement("div");
  panneau.className = "aide";
  panneau.dataset.aidePanneau = "";

  const tete = document.createElement("div");
  tete.className = "aide__tete";
  const h = document.createElement("h2");
  h.textContent = MOT.helpTitle;
  tete.appendChild(h);
  const p = document.createElement("p");
  p.textContent = MOT.helpLead;
  tete.appendChild(p);
  panneau.appendChild(tete);

  const corps = document.createElement("div");
  corps.className = "aide__corps";

  const grille = document.createElement("div");
  grille.className = "aide__grille";
  // Une rangée par face : les cinq cartes telles qu'on les reçoit, puis les
  // cinq premières améliorations, puis les cinq secondes. Quinze fichiers
  // distincts, chacun désigné par le manifeste.
  for (const face of FACES) {
    for (let n = 1; n <= 5; n++) {
      const v = vignette(n, face);
      if (v) grille.appendChild(v);
    }
  }
  corps.appendChild(grille);

  loupe = document.createElement("div");
  loupe.className = "aide__loupe";
  loupeImage = document.createElement("img");
  loupeImage.alt = "";
  loupeImage.draggable = false;
  loupe.appendChild(loupeImage);
  loupeMot = document.createElement("span");
  loupeMot.className = "aide__loupe-mot";
  loupeMot.textContent = MOT.helpHint;
  loupe.appendChild(loupeMot);
  corps.appendChild(loupe);

  panneau.appendChild(corps);
  return panneau;
}

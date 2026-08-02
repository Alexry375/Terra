// LE MONDE — l'état de la planète n'est pas un tableau de chiffres, c'est le décor.
//
// Ce jeu n'a pas de plateau : son état, c'est trois nombres qui ne montent jamais
// deux fois du même cran. Alors ces trois nombres SONT l'écran :
//
//   • `planet.temperature` réchauffe le ciel, du bleu de nuit au minerai chaud ;
//   • `planet.oceans`      fait monter la mer au bas de l'écran ;
//   • `planet.oxygen`      épaissit la brume à l'horizon.
//
// Chaque valeur affichée porte son chemin exact dans l'état rendu par le moteur
// (`data-valeur`). Aucun nombre n'est calculé ici : tout est lu.

import { imageJalon, imageRecompense, titre } from "./materiel.js";
import { ref, poser, poserValeur } from "./ecrire.js";
import { construireMasqueVP } from "./plateau.js";
import { MOT } from "./mots.js";

const CIEL_FROID = [[6, 10, 20], [16, 26, 42]];
const CIEL_CHAUD = [[26, 6, 3], [125, 44, 16]];

const melange = (a, b, t) =>
  `rgb(${a.map((v, i) => Math.round(v + (b[i] - v) * t)).join(",")})`;

let precedent = null; // l'état précédent, pour SENTIR ce qui vient de bouger

/** Le squelette du monde. Appelé une fois. */
export function construireMonde() {
  const frag = document.createDocumentFragment();

  const ciel = document.createElement("div");
  ciel.id = "ciel";
  ciel.innerHTML = `
    <div class="ciel__voute"></div>
    <div class="ciel__brume"></div>
    <div class="ciel__grain"></div>
    <div class="ciel__mer"><div class="ciel__houle"></div></div>
    <div class="ciel__vignette"></div>`;
  frag.appendChild(ciel);

  // LE SOL MARTIEN — la table est posée sur une photographie, pas sur un
  // dégradé. Le décor a été choisi par Alexis le 01-08 parmi six propositions
  // (Granicus Valles, HiRISE). Le VOILE qui le sépare des cartes est un élément
  // à part : sans lui la photo passe sous les cartes et plus rien ne se lit ;
  // trop dense, le décor disparaît. Il s'éteint tout seul si on le retire du
  // document, ce qui rend son effet mesurable de l'extérieur (`data-voile`).
  const sol = document.createElement("div");
  sol.id = "sol";
  frag.appendChild(sol);

  const voile = document.createElement("div");
  voile.id = "voile";
  voile.dataset.voile = "";
  frag.appendChild(voile);

  const h = document.createElement("header");
  h.id = "horizon";
  h.innerHTML = `
    <div class="manche">
      <span class="manche__mot">${MOT.round}</span>
      <b class="manche__n" data-valeur="generation">—</b>
    </div>

    <section class="param param--temp" id="param-temp">
      <div class="param__tete">
        <span class="param__nom">${MOT.temp}</span>
        <b class="param__n" data-valeur="planet.temperature">0</b>
        <span class="param__max">/<i id="temp-max" data-valeur="planet.temperature_max">0</i></span>
      </div>
      <div class="crans" id="crans-temp"></div>
    </section>

    <section class="param param--o2" id="param-o2">
      <div class="param__tete">
        <span class="param__nom">${MOT.oxygen}</span>
        <b class="param__n" data-valeur="planet.oxygen">0</b>
        <span class="param__max">/<i id="o2-max" data-valeur="planet.oxygen_max">0</i></span>
      </div>
      <div class="crans crans--o2" id="crans-o2"></div>
    </section>

    <section class="param param--mer" id="param-mer">
      <div class="param__tete">
        <span class="param__nom">${MOT.ocean}</span>
        <b class="param__n" data-valeur="planet.oceans">0</b>
        <span class="param__max">/<i id="mer-max" data-valeur="planet.oceans_max">0</i></span>
      </div>
      <div class="crans crans--mer" id="crans-mer"></div>
    </section>

    <section class="tuiles-honneur">
      <div class="tuiles-honneur__rang" id="jalons"></div>
      <div class="tuiles-honneur__rang" id="recompenses"></div>
    </section>

    <!-- La mention de la photographie du sol est une CONDITION D'USAGE de
         l'image, pas une décoration : elle se lit à l'écran, en toutes lettres,
         à tout instant de la partie. -->
    <span class="credit" id="credit">${MOT.credit}</span>`;
  frag.appendChild(h);
  // La case qui masque les points de victoire vit dans le bandeau : elle doit
  // rester atteignable à tout instant de la partie.
  construireMasqueVP(h);

  const secousse = document.createElement("div");
  secousse.id = "secousse";
  frag.appendChild(secousse);

  document.body.appendChild(frag);
}

/** Réécrit le monde à partir de l'état rendu par le moteur. */
export function majMonde(etat) {
  const p = etat.planet;
  const t = p.temperature_max ? p.temperature / p.temperature_max : 0;
  const o2 = p.oxygen_max ? p.oxygen / p.oxygen_max : 0;
  const mer = p.oceans_max ? p.oceans / p.oceans_max : 0;

  // Les variables du ciel ne sont posées que si elles bougent : changer une
  // propriété sur :root invalide le style de tout le document.
  variable("--ciel-a", melange(CIEL_FROID[0], CIEL_CHAUD[0], t));
  variable("--ciel-b", melange(CIEL_FROID[1], CIEL_CHAUD[1], t));
  variable("--chaleur", t.toFixed(3));
  variable("--brume", o2.toFixed(3));
  variable("--niveau-mer", (7 + mer * 13).toFixed(2) + "%");

  poserValeur("generation", etat.generation);
  poserValeur("planet.temperature", p.temperature);
  poserValeur("planet.oxygen", p.oxygen);
  poserValeur("planet.oceans", p.oceans);
  poser(ref("#temp-max"), p.temperature_max);
  poser(ref("#o2-max"), p.oxygen_max);
  poser(ref("#mer-max"), p.oceans_max);

  crans("crans-temp", p.temperature, p.temperature_max);
  crans("crans-o2", p.oxygen, p.oxygen_max);
  crans("crans-mer", p.oceans, p.oceans_max);

  honneurs(etat);
  ressentir(etat);
  precedent = etat;
}

const variables = new Map();
function variable(nom, valeur) {
  if (variables.get(nom) === valeur) return;
  variables.set(nom, valeur);
  document.documentElement.style.setProperty(nom, valeur);
}

/**
 * Les crans de température. Un cran gagné est irréversible : il se verrouille et
 * ne s'éteint plus jamais. C'est toute la tension du jeu, rendue physique.
 */
function crans(id, v, max) {
  const z = ref("#" + id);
  if (z.childElementCount !== max) {
    z.textContent = "";
    for (let i = 0; i < max; i++) z.appendChild(document.createElement("span"));
  }
  [...z.children].forEach((c, i) => c.classList.toggle("acquis", i < v));
}

/** Objectifs et récompenses : les tuiles imprimées, éteintes tant qu'à prendre. */
function honneurs(etat) {
  const zj = ref("#jalons");
  if (zj.childElementCount !== etat.milestones.length) {
    zj.textContent = "";
    for (const m of etat.milestones) {
      const d = document.createElement("div");
      d.className = "honneur";
      d.title = MOT.milestone + " " + titre(m.kind);
      const im = document.createElement("img");
      im.src = imageJalon(m.kind);
      im.alt = MOT.milestone + " " + titre(m.kind);
      d.appendChild(im);
      zj.appendChild(d);
    }
  }
  etat.milestones.forEach((m, i) => {
    const d = zj.children[i];
    if (!d) return;
    d.classList.toggle("honneur--pris", m.achieved_by.some(Boolean));
    d.dataset.par = m.achieved_by
      .map((oui, j) => (oui ? j : null))
      .filter((x) => x !== null)
      .join("");
  });

  const zr = ref("#recompenses");
  if (zr.childElementCount !== etat.awards.length) {
    zr.textContent = "";
    for (const a of etat.awards) {
      const d = document.createElement("div");
      d.className = "honneur honneur--recompense";
      d.title = MOT.award + " " + titre(a);
      const im = document.createElement("img");
      im.src = imageRecompense(a);
      im.alt = MOT.award + " " + titre(a);
      d.appendChild(im);
      zr.appendChild(d);
    }
  }
}

/**
 * SENTIR le cran gagné. On compare l'état d'avant et l'état d'après — on ne
 * calcule aucune règle, on remarque seulement ce que le moteur vient de changer,
 * et on n'affiche jamais que la valeur nouvelle, jamais un écart inventé.
 */
function ressentir(etat) {
  if (!precedent) return;
  const a = precedent.planet;
  const b = etat.planet;
  const evenements = [];
  if (b.temperature > a.temperature) evenements.push(["param-temp", "chaud"]);
  if (b.oxygen > a.oxygen) evenements.push(["param-o2", "o2"]);
  if (b.oceans > a.oceans) evenements.push(["param-mer", "mer"]);
  if (!evenements.length) return;

  for (const [id, teinte] of evenements) {
    const e = document.getElementById(id);
    if (!e) continue;
    e.classList.remove("param--gagne");
    void e.offsetWidth; // relance l'animation même si deux crans s'enchaînent
    e.dataset.teinte = teinte;
    e.classList.add("param--gagne");
  }
  const s = ref("#secousse");
  s.dataset.teinte = evenements[0][1];
  s.classList.remove("secousse--active");
  void s.offsetWidth;
  s.classList.add("secousse--active");
}

/** Remet la mémoire à zéro (nouvelle partie). */
export function oublier() {
  precedent = null;
  variables.clear();
}

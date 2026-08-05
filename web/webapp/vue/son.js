// LE SON — cinq bruits, pas de musique, aucun fichier.
//
// Tout est synthétisé dans la page : rien n'est téléchargé, rien n'est stocké.
// Le contexte audio n'existe qu'à partir du PREMIER clic (les navigateurs
// refusent le son avant un geste, et un refus laisserait une erreur dans la
// console). Tout est enveloppé : si le son est impossible, le jeu continue en
// silence, sans une ligne de console.

let ctx = null;
let coupe = false;

// ------------------------------------------------------------------ l'option
//
// (GRO-2, 05-08) L'INTERRUPTEUR DU SON, ET SON UNIQUE POINT D'ÉCRITURE.
//
// Pourquoi maintenant et pas avant : jusqu'ici trois bruits seulement étaient
// branchés, et tous les trois sont RARES — début de manche, réponse à une
// question, fin de partie. En branchant la pose d'une carte et le cran de
// terraformation, on passe à des dizaines de bruits par partie. Sans
// interrupteur, on impose le bruit aux deux joueurs assis devant le même écran.
//
// Il vit ICI, avec les sons, et `vue/options.js` le lit et le pose par ces deux
// fonctions — jamais en écrivant quoi que ce soit lui-même. C'est la leçon du
// réglage des animations, qui avait fini avec deux mémoires et un interrupteur
// qui mentait (en-tête de `vue/anim.js`).
//
// ÉTEINT, AUCUN DES CINQ NE SORT : le verrou est posé sur le CONTEXTE audio,
// c'est-à-dire en amont de tout ce que ce fichier fabrique. Un sixième bruit
// qu'on ajouterait un jour serait éteint lui aussi, sans rien à y penser.
//
// L'attribut sur le document n'est pas la mémoire — c'est sa déclaration, pour
// qu'on puisse lire de l'extérieur de la page ce que le réglage vaut.
let sonnant = true;

/** Le son est-il allumé ? */
export function sonsActifs() {
  return sonnant;
}

/** L'unique écriture du réglage : `?sons=non` comme l'interrupteur du panneau. */
export function reglerSons(oui) {
  sonnant = !!oui;
  if (sonnant) delete document.documentElement.dataset.sons;
  else document.documentElement.dataset.sons = "non";
}

function contexte() {
  if (!sonnant) return null;
  if (coupe) return null;
  if (ctx) return ctx;
  try {
    const C = window.AudioContext || window.webkitAudioContext;
    if (!C) { coupe = true; return null; }
    ctx = new C();
    if (ctx.state === "suspended" && ctx.resume) {
      // `resume()` peut être refusé : on avale le refus, il n'est pas une panne.
      Promise.resolve(ctx.resume()).catch(() => {});
    }
    return ctx;
  } catch (e) {
    coupe = true;
    return null;
  }
}

/** Le son ne s'éveille qu'au premier geste réel du joueur. */
export function eveiller() {
  contexte();
}

function timbre({ f0, f1, duree, volume, forme = "sine", bruit = false }) {
  const c = contexte();
  if (!c) return;
  try {
    const t = c.currentTime;
    const g = c.createGain();
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(volume, t + 0.012);
    g.gain.exponentialRampToValueAtTime(0.0001, t + duree);
    g.connect(c.destination);

    let source;
    if (bruit) {
      const n = Math.floor(c.sampleRate * duree);
      const tampon = c.createBuffer(1, n, c.sampleRate);
      const d = tampon.getChannelData(0);
      for (let i = 0; i < n; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / n);
      source = c.createBufferSource();
      source.buffer = tampon;
      const filtre = c.createBiquadFilter();
      filtre.type = "bandpass";
      filtre.frequency.value = f0;
      source.connect(filtre);
      filtre.connect(g);
    } else {
      source = c.createOscillator();
      source.type = forme;
      source.frequency.setValueAtTime(f0, t);
      if (f1 && f1 !== f0) source.frequency.exponentialRampToValueAtTime(f1, t + duree);
      source.connect(g);
    }
    source.start(t);
    source.stop(t + duree + 0.02);
  } catch (e) {
    /* le silence n'est pas une panne */
  }
}

/** Un choix posé : sec, court, matériel. */
export function sonChoix() {
  timbre({ f0: 1600, duree: 0.06, volume: 0.06, bruit: true });
  timbre({ f0: 180, f1: 90, duree: 0.09, volume: 0.05, forme: "triangle" });
}

/** Une carte qui se pose. */
export function sonCarte() {
  timbre({ f0: 900, duree: 0.11, volume: 0.05, bruit: true });
}

/** Un cran de terraformation : grave, long, irréversible. */
export function sonCran() {
  timbre({ f0: 70, f1: 200, duree: 0.9, volume: 0.1, forme: "sine" });
  timbre({ f0: 320, f1: 640, duree: 0.7, volume: 0.035, forme: "triangle" });
}

/** Le tour de manche. */
export function sonManche() {
  timbre({ f0: 130, f1: 65, duree: 0.7, volume: 0.07, forme: "sine" });
}

/** La fin de la partie. */
export function sonFin() {
  timbre({ f0: 196, duree: 1.6, volume: 0.07, forme: "sine" });
  timbre({ f0: 294, duree: 1.6, volume: 0.05, forme: "sine" });
  timbre({ f0: 392, duree: 1.9, volume: 0.045, forme: "sine" });
}

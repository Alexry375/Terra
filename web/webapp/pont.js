// Le pont entre la page (ou Node) et le moteur Rust compilé en WebAssembly.
//
// Règle intangible du chantier : AUCUNE règle du jeu ici. Ce fichier sérialise
// une requête JSON, appelle `terra_call` du wasm, et rend le JSON que le moteur
// a produit. Tout coût, toute production, tout score, toute option légale, et
// l'état de la partie lui-même, viennent du wasm.

import { creerWasi } from "./wasi-shim.js";

/** Chemin virtuel du fichier de cartes DANS le système de fichiers du wasm. */
export const CARTES = "assets/cards.json";

/**
 * Ouvre le pont.
 * @param {object} o
 * @param {Uint8Array|ArrayBuffer} o.wasm   octets de `terra.wasm`
 * @param {Uint8Array} o.cards              octets du fichier de cartes
 * @param {(flux: string, texte: string) => void} [o.ecrire]  stdout/stderr du wasm
 */
export async function ouvrirPont({ wasm, cards, ecrire }) {
  const wasi = creerWasi({ fichiers: { [CARTES]: cards }, ecrire });
  const octets = wasm instanceof Uint8Array ? wasm : new Uint8Array(wasm);
  const module = await WebAssembly.compile(octets);
  const instance = await WebAssembly.instantiate(module, wasi.imports);
  wasi.lier(instance);
  const ex = instance.exports;

  function appeler(requete) {
    const bytes = new TextEncoder().encode(
      JSON.stringify({ cards: CARTES, ...requete })
    );
    const ptr = ex.terra_alloc(bytes.length);
    new Uint8Array(ex.memory.buffer).set(bytes, ptr);
    let n;
    try {
      n = ex.terra_call(ptr, bytes.length);
    } finally {
      ex.terra_free(ptr, bytes.length);
    }
    if (n < 0) throw new Error("terra_call a refuse la requete");
    const out = ex.terra_result_ptr();
    const texte = new TextDecoder().decode(
      new Uint8Array(ex.memory.buffer).subarray(out, out + n)
    );
    return JSON.parse(texte);
  }

  /** Lève si le moteur a refusé la requête ; rend la réponse sinon. */
  function verifier(r) {
    if (r && r.ok === false) {
      const e = new Error(r.erreur);
      e.moteur = true;
      throw e;
    }
    return r;
  }

  return {
    /**
     * Une interrogation du moteur : rend le TABLEAU DE LIGNES que le binaire
     * natif écrirait sur sa sortie standard, à l'octet près.
     * @param {object} requete  ex. { op: "dump_deck", boites: "base,decouverte" }
     */
    lignes(requete) {
      return verifier(appeler(requete)).lignes;
    },
    /**
     * Un coup de la partie pas-à-pas : rejoue depuis la graine avec les
     * décisions déjà prises et rend la prochaine décision + l'état vivant que
     * le moteur avait sous les yeux au moment de ce choix (`state_view`).
     */
    pas(seed, boites, decisions) {
      return verifier(appeler({ op: "pas", seed, boites, decisions }));
    },
    appeler,
  };
}

/** Chargement des octets, côté Node comme côté navigateur. */
export async function chargerOctets(chemin) {
  if (typeof process !== "undefined" && process.versions && process.versions.node) {
    const { readFile } = await import("node:fs/promises");
    return new Uint8Array(await readFile(chemin));
  }
  const r = await fetch(chemin);
  if (!r.ok) throw new Error("chargement impossible : " + chemin);
  return new Uint8Array(await r.arrayBuffer());
}

/**
 * Ouvre le pont depuis les fichiers de la livraison.
 * @param {string} racine  dossier de la livraison (défaut : le dossier courant)
 * @param {object} [o]     { cartes, ecrire } — `cartes` permet de servir un
 *                         autre fichier de cartes que celui de la livraison
 *                         (équivalent de `--cards` du binaire natif).
 */
export async function ouvrirPontDepuis(racine = ".", o = {}) {
  const [wasm, cards] = await Promise.all([
    chargerOctets(racine + "/terra.wasm"),
    chargerOctets(o.cartes || racine + "/" + CARTES),
  ]);
  return ouvrirPont({ wasm, cards, ecrire: o.ecrire });
}

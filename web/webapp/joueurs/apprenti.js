// **(le-juge-apprend) `apprenti` — LE JOUEUR QUI A APPRIS À JUGER UNE SITUATION.**
//
// Il n'a aucune échelle de valeur écrite à la main, aucune règle du jeu, aucun
// barème : à chaque point de décision il essaie **chaque option que le moteur a
// énumérée**, demande au moteur l'état qui en résulterait, décrit cet état
// (`description.js`) et le fait passer dans le réseau entraîné en Rust
// (`engine/src/bin/entraine.rs`). Il garde l'option dont SA probabilité de
// victoire est la plus haute. Rien d'autre ne décide à sa place — remplacez les
// poids, et le joueur change du tout au tout.
//
// ─────────────────────────────────────────────────────────────────────────────
// **LE VERROU DU §7, ET POURQUOI IL EXISTE.**
//
// Les poids sont appris en Rust et relus ici. Si les deux côtés ne rangeaient pas
// les mêmes nombres dans le même ordre, les poids ne voudraient plus rien dire et
// le joueur serait mauvais sans qu'on comprenne pourquoi. Le fichier de poids
// porte donc le NOM de chacune de ses entrées ; au chargement, ce module
// régénère les siens (`nomsDesEntrees()`) et les compare un par un. Au premier
// écart, il REFUSE DE JOUER et dit lequel.
//
// ─────────────────────────────────────────────────────────────────────────────
// **COMMENT IL OBTIENT « L'ÉTAT QUI EN RÉSULTERAIT ».**
//
// Le pont sait rejouer une partie depuis sa graine avec n'importe quelle liste de
// décisions (`pont.pas`) : « la partie EST la graine plus la liste des décisions »
// (`adversaire.md`). Essayer un coup sans le jouer, c'est donc
// `pont.pas(graine, boites, [...décisions déjà prises, la réponse essayée])`.
//
// Mais un fournisseur ne reçoit que `(decision, etat)` : ni la graine de la
// partie, ni la liste des décisions, ni le pont — et `partie.js` comme `pont.js`
// sont hors territoire. Ce module reçoit donc le pont par la ligne que le prompt
// autorise dans `duel.mjs`, et **enveloppe `pont.pas` en simple observateur** :
// l'enveloppe délègue à la fonction d'origine, ne touche à aucune valeur, et se
// contente de relever la graine, les boîtes et le tableau vivant des décisions
// que `partie.js` tient déjà. Aucune information cachée n'est lue au passage :
// une décision est un indice dans des options que le moteur a publiées.
//
// Sans pont, le joueur ne peut rien essayer : il le dit une fois et répond alors
// la première option (voir `MODE_DEGRADE` plus bas).
//
// ─────────────────────────────────────────────────────────────────────────────
// **IL NE REGARDE PAS LA MAIN D'EN FACE.** Mieux : il ne lit pas du tout l'`etat`
// qu'on lui passe. Il ne juge que des états qu'il obtient lui-même du moteur, et
// `description.js` n'y prend de l'adversaire que le NOMBRE de ses cartes. Un banc
// qui lui repose la même question avec une autre main adverse obtient donc
// exactement la même réponse.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { formeDeLaReponse, nombreDeChoix } from "../fournisseurs.js";
import { decrire, nomsDesEntrees } from "./description.js";

const ICI = dirname(fileURLToPath(import.meta.url));
/** Le chemin est calculé depuis L'EMPLACEMENT DU MODULE, jamais depuis le
 *  répertoire courant : la balance, les bancs et la page ne sont pas lancés du
 *  même endroit (§9). */
export const POIDS_PAR_DEFAUT = resolve(ICI, "../../../data/poids/apprenti.txt");

const CACHES_ATTENDUS = 50;
const SORTIES_ATTENDUES = 2;

// ──────────────────────────────────────────────────────────── le fichier de poids

/**
 * Lit un fichier de poids du §7 et **vérifie le verrou** : la table des noms du
 * fichier doit être exactement celle que ce dépôt régénère.
 * @param {string} chemin
 */
export function lirePoids(chemin) {
  const lignes = readFileSync(chemin, "utf8").split("\n");
  const tete = (lignes[0] || "").trim().split(/\s+/).map(Number);
  const [nEntrees, caches, sorties] = tete;
  if (!Number.isInteger(nEntrees) || !Number.isInteger(caches) || !Number.isInteger(sorties)) {
    throw new Error(`poids illisibles : première ligne « ${lignes[0]} » (§7 : entrées caches sorties)`);
  }
  if (caches !== CACHES_ATTENDUS || sorties !== SORTIES_ATTENDUES) {
    throw new Error(`poids inattendus : ${caches} neurones cachés et ${sorties} sorties (§1 en impose 50 et 2)`);
  }
  const parties = Number((lignes[1] || "").trim());
  const noms = lignes.slice(2, 2 + nEntrees).map((l) => l.replace(/\r$/, ""));

  // LE VERROU. Une divergence entre les deux descriptions devient impossible à
  // ne pas voir : elle ne peut plus se manifester par « le joueur est
  // mystérieusement mauvais ».
  const attendus = nomsDesEntrees();
  if (attendus.length !== nEntrees) {
    throw new Error(
      `le fichier de poids décrit ${nEntrees} entrées, ce dépôt en produit ${attendus.length} : ` +
        `les poids ont été appris sur une AUTRE description (${chemin})`,
    );
  }
  for (let i = 0; i < nEntrees; i++) {
    if (noms[i] !== attendus[i]) {
      throw new Error(
        `divergence de description au rang ${i} : le fichier dit « ${noms[i] ?? "(rien)"} », ` +
          `ce dépôt produit « ${attendus[i]} » — les poids ne veulent plus rien dire (${chemin})`,
      );
    }
  }

  const total = (nEntrees + 1) * caches + (caches + 1) * sorties;
  const nombres = new Float64Array(total);
  let k = 0;
  for (let i = 2 + nEntrees; i < lignes.length && k < total; i++) {
    const t = lignes[i].trim();
    if (t === "") continue;
    nombres[k++] = Number(t);
  }
  if (k !== total) {
    throw new Error(`le fichier de poids porte ${k} nombres, il en faut ${total} (${chemin})`);
  }
  const nCache = (nEntrees + 1) * caches;
  return {
    nEntrees,
    caches,
    sorties,
    parties,
    noms,
    // Rangés PAR ENTRÉE, comme en Rust : `wCache[i * caches + j]`, la ligne
    // `i === nEntrees` portant le biais d'entrée.
    wCache: nombres.subarray(0, nCache),
    wSortie: nombres.subarray(nCache),
  };
}

// ────────────────────────────────────────────────────────────────── le réseau

/**
 * L'évaluation du réseau : somme pondérée, tangente hyperbolique, puis
 * exponentielle normalisée. **La sortie 0 est la probabilité que gagne le joueur
 * du point de vue duquel la situation est décrite** — jamais « le siège 0 ».
 */
export function evaluer(poids, x) {
  const { nEntrees, caches, sorties, wCache, wSortie } = poids;
  const sommes = new Float64Array(caches);
  const biais = nEntrees * caches;
  for (let j = 0; j < caches; j++) sommes[j] = wCache[biais + j];
  for (let i = 0; i < nEntrees; i++) {
    const v = x[i];
    if (v === 0) continue;
    const base = i * caches;
    for (let j = 0; j < caches; j++) sommes[j] += v * wCache[base + j];
  }
  const h = new Float64Array(caches);
  for (let j = 0; j < caches; j++) h[j] = Math.tanh(sommes[j]);
  const s = new Float64Array(sorties);
  for (let k = 0; k < sorties; k++) {
    const base = k * (caches + 1);
    let acc = wSortie[base + caches];
    for (let j = 0; j < caches; j++) acc += h[j] * wSortie[base + j];
    s[k] = acc;
  }
  const s0 = s[0];
  let total = 0;
  const e = new Float64Array(sorties);
  for (let k = 0; k < sorties; k++) {
    e[k] = Math.exp(s[k] - s0);
    total += e[k];
  }
  const p = new Array(sorties);
  for (let k = 0; k < sorties; k++) p[k] = e[k] / total;
  return p;
}

// ──────────────────────────────────────────────────────── l'espion du pont

/**
 * Enveloppe `pont.pas` en OBSERVATEUR : la fonction d'origine fait tout le
 * travail et rend tout ce qu'elle rendait ; on relève seulement avec quels
 * arguments la partie l'appelle. `decisions` est le tableau que `partie.js`
 * tient à jour — on en garde la référence, pas une copie.
 */
function espionner(pont) {
  if (pont.__espionApprenti) return pont.__espionApprenti;
  const origine = pont.pas.bind(pont);
  const espion = { graine: null, boites: null, decisions: null, origine };
  pont.pas = (graine, boites, decisions) => {
    espion.graine = graine;
    espion.boites = boites;
    espion.decisions = decisions;
    return origine(graine, boites, decisions);
  };
  Object.defineProperty(pont, "__espionApprenti", { value: espion, enumerable: false });
  return espion;
}

// ───────────────────────────────────────────────────────────── le fournisseur

let _poidsEnCache = null;
let _cheminEnCache = null;

/**
 * Le fournisseur `apprenti`, de la même forme que `fournisseurReflechi`.
 *
 * @param {number} graine  acceptée pour respecter la signature des autres
 *   fournisseurs ; ce joueur ne tire rien au sort (l'exploration du §5 ne sert
 *   qu'à l'entraînement, jamais à la mesure).
 * @param {string} nom
 * @param {object|string} [poids]  poids déjà lus, ou chemin d'un fichier ;
 *   par défaut `data/poids/apprenti.txt`.
 * @param {object} [pont]   le pont, pour essayer les options (voir l'en-tête).
 * @param {string} [boites] la composition des boîtes de la partie.
 */
export function fournisseurApprenti(graine, nom = "apprenti", poids, pont, boites) {
  let p = poids;
  if (p === undefined || typeof p === "string") {
    const chemin = typeof p === "string" ? p : POIDS_PAR_DEFAUT;
    if (_cheminEnCache !== chemin) {
      _poidsEnCache = lirePoids(chemin);
      _cheminEnCache = chemin;
    }
    p = _poidsEnCache;
  }
  const espion = pont ? espionner(pont) : null;
  let degradeDit = false;

  /** L'état atteint si l'on répondait `reponse` — ou `null` si le moteur refuse. */
  function etatApres(reponse) {
    const decisions = espion.decisions || [];
    try {
      const r = espion.origine(espion.graine, espion.boites ?? boites, [...decisions, reponse]);
      return r && r.etat ? r.etat : null;
    } catch {
      // Le moteur a refusé cette réponse : elle n'est pas jouable, on l'écarte.
      return null;
    }
  }

  /** La note d'une réponse : MA probabilité de victoire dans l'état qui suit. */
  function noter(reponse, siege) {
    const etat = etatApres(reponse);
    if (!etat) return -Infinity;
    // Toujours du point de vue du joueur QUI DÉCIDAIT, jamais de celui à qui la
    // main revient : une inversion ici donne un joueur qui joue contre lui-même.
    return evaluer(p, decrire(etat, siege))[0];
  }

  function meilleure(reponses, siege) {
    let choix = reponses[0];
    let note = -Infinity;
    for (const r of reponses) {
      const n = noter(r, siege);
      if (n > note) {
        note = n;
        choix = r;
      }
    }
    return choix;
  }

  /**
   * **Un choix multiple.** Le moteur n'accepte que les combinaisons de la taille
   * exacte qu'il demande : une liste à moitié construite est refusée, pas
   * évaluée. Chaque candidat essayé doit donc être une réponse complète.
   *
   * - **nombre libre** (le mulligan projets) : toute liste vaut réponse, y
   *   compris la vide ; on part d'elle et on ajoute la carte qui améliore le
   *   plus, tant qu'une addition améliore.
   * - **nombre imposé** : on part des k premières — complète, donc évaluable —
   *   et on essaie de REMPLACER chaque carte retenue par chacune des autres.
   *   Deux tours, ce qui borne le coût.
   *
   * Copie conforme de `Joueur::choisir_liste` (`engine/src/joueur.rs`), même
   * ordre de parcours : c'est ce que vérifie `verif/juge-meme-option.mjs`.
   */
  function meilleureListe(d, siege) {
    const n = d.options ? d.options.length : 0;
    const libre = d.a_choisir === undefined || d.a_choisir === null;
    let pris = [];
    if (libre) {
      let note = noter([...pris], siege);
      while (pris.length < n) {
        let meilleur = null;
        let meilleureNote = -Infinity;
        for (let i = 0; i < n; i++) {
          if (pris.includes(i)) continue;
          const x = noter([...pris, i], siege);
          if (meilleur === null || x > meilleureNote) {
            meilleureNote = x;
            meilleur = i;
          }
        }
        if (meilleur === null || !(meilleureNote > note)) break;
        pris.push(meilleur);
        note = meilleureNote;
      }
      return pris;
    }
    const attendu = Math.min(d.a_choisir, n);
    pris = [...Array(attendu).keys()];
    let note = noter([...pris], siege);
    for (let tour = 0; tour < 2; tour++) {
      let ameliore = false;
      for (let p = 0; p < pris.length; p++) {
        for (let c = 0; c < n; c++) {
          if (pris.includes(c)) continue;
          const ancien = pris[p];
          pris[p] = c;
          const x = noter([...pris], siege);
          if (x > note) {
            note = x;
            ameliore = true;
          } else {
            pris[p] = ancien;
          }
        }
      }
      if (!ameliore) break;
    }
    return pris;
  }

  return {
    nom,
    poids: p,
    decider(d) {
      const siege = d.joueur ?? 0;
      const forme = formeDeLaReponse(d);
      if (!espion || espion.decisions === null) {
        // MODE_DEGRADE : sans pont, on ne peut essayer aucune option. On répond
        // alors la première, de façon parfaitement déterministe — et on le dit,
        // une fois, plutôt que de laisser croire que le réseau a joué.
        if (!degradeDit) {
          degradeDit = true;
          if (globalThis.process) {
            process.stderr.write(
              "apprenti : aucun pont fourni, le joueur ne peut essayer aucune option " +
                "(il répond la première ; voir l'en-tête de joueurs/apprenti.js)\n",
            );
          }
        }
        if (forme === "montant") return d.minimum ?? 0;
        if (forme === "multiple") return [...Array(d.a_choisir ?? 0).keys()];
        return 0;
      }
      if (forme === "multiple") return meilleureListe(d, siege);
      if (forme === "montant") {
        const min = d.minimum ?? 0;
        const max = d.maximum ?? 0;
        const rs = [];
        for (let v = min; v <= max; v++) rs.push(v);
        return meilleure(rs, siege);
      }
      const n = nombreDeChoix(d);
      const rs = [];
      for (let i = 0; i < n; i++) rs.push(i);
      return meilleure(rs, siege);
    },
  };
}

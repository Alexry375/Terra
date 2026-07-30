#!/usr/bin/env node
// `simulate` — la MÊME ligne de commande que le binaire du moteur, mais servie
// par le WebAssembly de cette livraison.
//
// Ce fichier ne connaît AUCUNE règle du jeu et n'a aucune réponse en réserve :
// il traduit des arguments en une requête JSON, la passe au wasm, et imprime
// telles quelles les lignes que le moteur a produites. Toute la sortie —
// recensements, sonde, bilan, vue d'état — est construite dans le wasm par le
// même code de sérialisation que le binaire du dépôt.
//
// Le dossier est autosuffisant : `terra.wasm` et `assets/cards.json` sont
// résolus par rapport à CE fichier, jamais par rapport au dossier courant.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { ouvrirPontDepuis } from "../pont.js";

const RACINE = resolve(dirname(fileURLToPath(import.meta.url)), "..");

function mourir(msg) {
  process.stderr.write(`simulate: ${msg}\n`);
  process.exit(2);
}

// ------------------------------------------------------------- ligne de commande

function lire(args) {
  const o = {
    games: "1000",
    seed: "0",
    cards: null, // null = le fichier de cartes de la livraison
    effects: true,
    boites: "base",
    dumpDeck: false,
    dumpCorporations: false,
    dumpState: false,
    dumpTurnOrder: false,
    observe: false,
    observeState: false,
    sonde: {},
  };
  const s = o.sonde;
  let i = 0;
  const val = () => {
    if (i + 1 >= args.length) mourir(`valeur manquante pour ${args[i]}`);
    return args[i + 1];
  };
  const entier = (nom) => {
    const n = Number(val());
    if (!Number.isInteger(n)) mourir(`${nom} invalide`);
    return n;
  };
  // `--games` et `--seed` sont des `u64` côté moteur. Un nombre JSON est un
  // flottant : au-delà de 2^53 il ne représente plus exactement une graine, et
  // `--seed 18446744073709551615` deviendrait une AUTRE partie sans le dire.
  // Ils voyagent donc en CHAÎNE de chiffres, et sont refusés comme le fait le
  // binaire natif (`u64::parse`) : ni signe, ni virgule, ni débordement.
  // `--probe-filler` et `--probe-phase` sont des entiers NON SIGNÉS côté
  // moteur : le binaire natif refuse `-3`, le pont doit refuser pareil.
  const entierPositif = (nom) => {
    const n = entier(nom);
    if (n < 0) mourir(`${nom} invalide`);
    return n;
  };
  const u64 = (nom) => {
    const t = val().trim();
    if (!/^\+?\d+$/.test(t)) mourir(`${nom} invalide`);
    let n;
    try { n = BigInt(t); } catch { mourir(`${nom} invalide`); }
    if (n > 18446744073709551615n) mourir(`${nom} invalide`);
    return n.toString();
  };
  while (i < args.length) {
    switch (args[i]) {
      case "--games": o.games = u64("--games"); i += 2; break;
      case "--seed": o.seed = u64("--seed"); i += 2; break;
      case "--cards": o.cards = val(); i += 2; break;
      case "--effects": {
        const v = val();
        if (v !== "on" && v !== "off") mourir(`--effects invalide: ${v} (on|off)`);
        o.effects = v === "on";
        i += 2; break;
      }
      case "--boites": o.boites = val(); i += 2; break;
      case "--dump-deck": o.dumpDeck = true; i += 1; break;
      case "--dump-corporations": o.dumpCorporations = true; i += 1; break;
      case "--dump-state": o.dumpState = true; i += 1; break;
      case "--dump-turn-order": o.dumpTurnOrder = true; i += 1; break;
      case "--observe": o.observe = true; i += 1; break;
      case "--observe-state": o.observe = true; o.observeState = true; i += 1; break;
      case "--probe": s.probe = val(); i += 2; break;
      case "--probe-action": s.probe_action = val(); i += 2; break;
      case "--probe-corp": s.probe_corp = val(); i += 2; break;
      case "--probe-produce": s.probe_produce = true; i += 1; break;
      case "--probe-strict": {
        s.probe_strict = true;
        // Les deux formes du binaire : seule, ou suivie de la séquence.
        if (args[i + 1] !== undefined && !args[i + 1].startsWith("--")) {
          s.probe = args[i + 1];
          i += 2;
        } else i += 1;
        break;
      }
      case "--probe-mc": s.probe_mc = entier("--probe-mc"); i += 2; break;
      case "--probe-plants": s.probe_plants = entier("--probe-plants"); i += 2; break;
      case "--probe-filler": s.probe_filler = entierPositif("--probe-filler"); i += 2; break;
      case "--probe-phase": s.probe_phase = entierPositif("--probe-phase"); i += 2; break;
      case "--probe-upgrade":
        (s.probe_upgrade ||= []).push(val()); i += 2; break;
      case "--probe-objectif": s.probe_objectif = val(); i += 2; break;
      case "--probe-joker-tag": s.probe_joker_tag = val(); i += 2; break;
      case "--probe-choice":
        s.probe_choice = val().split(",").map((x) => x.trim()).filter(Boolean)
          .map((x) => {
            const n = Number(x);
            if (!Number.isInteger(n) || n < 0) mourir("--probe-choice invalide");
            return n;
          });
        i += 2; break;
      case "--probe-target":
        s.probe_target = val().split(";").map((x) => x.trim()).filter(Boolean);
        i += 2; break;
      default: mourir(`argument inconnu: ${args[i]}`);
    }
  }
  return o;
}

// ------------------------------------------------------------------------ main

const o = lire(process.argv.slice(2));

// Le wasm écrit sur sa sortie standard (`--observe` en fait une ligne par
// décision) : on la relaie telle quelle, dans l'ordre, avant la réponse.
const relais = (flux, texte) => {
  (flux === "stderr" ? process.stderr : process.stdout).write(texte + "\n");
};

let pont;
try {
  pont = await ouvrirPontDepuis(RACINE, { cartes: o.cards, ecrire: relais });
} catch (e) {
  mourir(String(e.message || e));
}

const commun = { boites: o.boites, effects: o.effects };
let requete;
if (o.dumpDeck) requete = { op: "dump_deck", ...commun };
else if (o.dumpCorporations) requete = { op: "dump_corporations", ...commun };
else if (o.dumpState) requete = { op: "dump_state", seed: o.seed, ...commun };
else if (o.sonde.probe !== undefined || o.sonde.probe_action !== undefined ||
         o.sonde.probe_corp !== undefined)
  requete = { op: "probe", ...o.sonde, ...commun };
else
  requete = {
    op: "bilan",
    games: o.games,
    seed: o.seed,
    observe: o.observe,
    observe_state: o.observeState,
    dump_turn_order: o.dumpTurnOrder,
    ...commun,
  };

const r = pont.appeler(requete);
if (r.ok === false) mourir(r.erreur);
for (const a of r.avertissements || []) process.stderr.write(`boites: ${a}\n`);
if (r.games_per_sec !== undefined) {
  process.stderr.write(`games_per_sec: ${r.games_per_sec.toFixed(1)}\n`);
}
for (const l of r.lignes) process.stdout.write(l + "\n");

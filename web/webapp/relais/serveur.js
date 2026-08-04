// LE SERVEUR DE RENDEZ-VOUS — deux navigateurs, une seule partie.
//
// Il fait DEUX choses, et pas une de plus :
//
//   1. il sert les fichiers de la livraison (`outputs/webapp/`) en HTTP, parce
//      que la page ne fonctionne pas autrement — jamais en `file://` ;
//   2. il tient, pour chaque partie, LA LISTE ORDONNÉE DES DÉCISIONS, et la
//      diffuse aux deux joueurs.
//
// IL NE CONNAÎT AUCUNE RÈGLE DU JEU. Il ne sait pas ce qu'une carte coûte, ni
// combien de choix une question offre, ni ce qu'est un score. Une décision est
// pour lui une valeur opaque, rangée à la suite des autres. L'autorité est le
// moteur, qui vit dans les deux pages — c'est la règle d'or d'`adversaire.md`.
//
// Ce qu'il sait, il le tient des pages :
//   · la GRAINE — la première page qui ouvre la partie la fixe, la seconde la lit ;
//   · le TOUR — chaque page déclare, pour chaque rang, quel siège le moteur vient
//     d'interroger (`POST /relais/tour`). Les deux moteurs disent la même chose,
//     donc les deux déclarations se corroborent ; une déclaration qui en
//     contredirait une autre est refusée. Le serveur ne DEVINE jamais le tour.
//
// AUCUNE DÉPENDANCE EXTERNE : uniquement les modules de Node lui-même. Rien à
// installer, rien à mettre à jour, rien qui puisse manquer demain matin.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------- l'adresse

const ICI = path.dirname(fileURLToPath(import.meta.url));
const LIVRAISON = path.resolve(ICI, ".."); // `outputs/webapp/`
const PORT_PAR_DEFAUT = 8080;

/** `--port 9000` comme `--port=9000`. Rien d'autre à régler. */
function lirePort(args) {
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--port" && args[i + 1] !== undefined) return Number(args[i + 1]);
    if (a.startsWith("--port=")) return Number(a.slice("--port=".length));
  }
  return PORT_PAR_DEFAUT;
}

// ------------------------------------------------------------- ce qu'il dit
//
// Tout ce que le serveur fait, il l'écrit — en clair et en français. C'est ce
// qui permettra de comprendre une panne demain matin, quand personne n'aura le
// temps de lire du code.

function heure() {
  const d = new Date();
  const deux = (n) => String(n).padStart(2, "0");
  return `${deux(d.getHours())}:${deux(d.getMinutes())}:${deux(d.getSeconds())}`;
}

function dire(...morceaux) {
  console.log(`[${heure()}] ` + morceaux.join(" "));
}

// -------------------------------------------------------------- les parties
//
// Une partie : une graine, une liste ordonnée de décisions, le siège attendu
// pour chaque rang (tel que les moteurs l'ont annoncé), et les connexions
// ouvertes de chaque siège.

const parties = new Map();

function nouvelleGraine() {
  // Un nombre lisible, que l'on peut se dicter au téléphone en cas de besoin.
  return 1 + Math.floor(Math.random() * 999999);
}

function partieDe(code, souhaits = {}) {
  let p = parties.get(code);
  if (!p) {
    p = {
      code,
      graine: Number.isInteger(souhaits.graine) ? souhaits.graine : nouvelleGraine(),
      boites: souhaits.boites === "base" ? "base" : "base,decouverte",
      decisions: [],
      // rang -> siège, tel que le moteur des pages l'a annoncé.
      tours: new Map(),
      // siège -> ensemble des flux ouverts. Une connexion ouverte, c'est un
      // joueur présent : rien n'est déduit, rien n'est affirmé d'avance.
      flux: [new Set(), new Set()],
      ouverte: Date.now(),
    };
    parties.set(code, p);
    dire(`partie « ${code} » ouverte — graine ${p.graine}, boîtes ${p.boites}`);
  }
  return p;
}

// LES PARTIES MORT-NÉES S'OUBLIENT. N'importe quelle demande d'état ouvre une
// partie — il le faut, c'est ainsi que la seconde page apprend la graine de la
// première. Mais sur une adresse publique, un aperçu de lien ou une adresse mal
// recopiée en crée une pour rien. Celles qui n'ont jamais reçu ni joueur ni
// décision s'effacent au bout d'une heure ; une partie vivante, ou seulement
// mise en pause parce que les deux joueurs ont fermé leur page un instant, n'est
// JAMAIS touchée — perdre une partie en cours serait bien pire que la garder.
const OUBLI = 3600_000;

function oublierLesPartiesMortNees() {
  const maintenant = Date.now();
  for (const [code, p] of [...parties]) {
    const vide = p.decisions.length === 0 && !p.flux[0].size && !p.flux[1].size;
    if (vide && maintenant - p.ouverte > OUBLI) {
      parties.delete(code);
      dire(`partie « ${code} » oubliée : ouverte il y a plus d'une heure, ` +
           `jamais rejointe, aucune décision`);
    }
  }
}

setInterval(oublierLesPartiesMortNees, 600_000).unref();

/** Pour chacun des deux sièges : est-il connecté à cet instant ? */
function joueursDe(p) {
  return [p.flux[0].size > 0, p.flux[1].size > 0];
}

function etatDe(p) {
  return {
    partie: p.code,
    graine: p.graine,
    boites: p.boites,
    decisions: p.decisions.slice(),
    joueurs: joueursDe(p),
    rang_attendu: p.decisions.length,
    siege_attendu: p.tours.has(p.decisions.length)
      ? p.tours.get(p.decisions.length)
      : null,
  };
}

// ------------------------------------------------------- le flux d'évènements
//
// Comment le serveur prévient les joueurs qu'il y a du nouveau : un flux
// d'évènements HTTP (`text/event-stream`), qui ne coûte aucune dépendance et
// que le navigateur sait lire tout seul (`EventSource`). La connexion reste
// ouverte : tant qu'elle l'est, le siège est là. Quand elle tombe — page
// fermée, réseau coupé — le serveur le voit tout de suite, et le dit à l'autre.

function envoyerA(reponse, sorte, donnees) {
  reponse.write(`event: ${sorte}\ndata: ${JSON.stringify(donnees)}\n\n`);
}

function diffuser(p, sorte, donnees) {
  for (const siege of [0, 1]) {
    for (const r of p.flux[siege]) {
      try {
        envoyerA(r, sorte, donnees);
      } catch {
        /* la connexion est morte : son « close » fera le ménage */
      }
    }
  }
}

function diffuserPresence(p) {
  diffuser(p, "joueurs", { joueurs: joueursDe(p) });
}

function ouvrirFlux(requete, reponse, code, siege) {
  const p = partieDe(code);
  reponse.writeHead(200, {
    "Content-Type": "text/event-stream; charset=utf-8",
    "Cache-Control": "no-cache, no-transform",
    Connection: "keep-alive",
    // Certains relais intermédiaires gardent la réponse en réserve avant de la
    // transmettre : le flux n'arriverait que par paquets, ou jamais.
    "X-Accel-Buffering": "no",
  });
  reponse.write(": rendez-vous ouvert\n\n");

  p.flux[siege].add(reponse);
  dire(`partie « ${code} » : le joueur du siège ${siege} est arrivé ` +
       `(${p.decisions.length} décision(s) déjà données)`);

  // Ce que le nouvel arrivant doit savoir tout de suite : toute la partie.
  envoyerA(reponse, "bonjour", etatDe(p));
  diffuserPresence(p);

  // Une respiration régulière : elle empêche un relais intermédiaire de croire
  // la connexion morte et de la fermer au bout d'une minute de silence.
  const battement = setInterval(() => {
    try {
      reponse.write(": battement\n\n");
    } catch {
      /* voir plus bas : « close » fait le ménage */
    }
  }, 15000);

  const partir = () => {
    clearInterval(battement);
    if (p.flux[siege].delete(reponse)) {
      dire(`partie « ${code} » : le joueur du siège ${siege} est parti`);
      diffuserPresence(p);
    }
  };
  requete.on("close", partir);
  requete.on("aborted", partir);
}

// -------------------------------------------------------------- les réponses

function repondreJson(reponse, code, objet) {
  const corps = Buffer.from(JSON.stringify(objet), "utf-8");
  reponse.writeHead(code, {
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": corps.length,
    "Cache-Control": "no-store",
  });
  reponse.end(corps);
}

/** Un refus se DIT : un code, et une phrase en clair qui explique pourquoi. */
function refuser(reponse, code, phrase, contexte) {
  dire(`décision refusée — ${contexte ? contexte + " : " : ""}${phrase}`);
  repondreJson(reponse, code, { ok: false, erreur: phrase });
}

function lireCorps(requete) {
  return new Promise((ok, non) => {
    const morceaux = [];
    let taille = 0;
    requete.on("data", (m) => {
      taille += m.length;
      if (taille > 1_000_000) {
        non(new Error("corps de requête démesuré"));
        requete.destroy();
        return;
      }
      morceaux.push(m);
    });
    requete.on("end", () => {
      const texte = Buffer.concat(morceaux).toString("utf-8");
      if (!texte.trim()) return ok({});
      try {
        ok(JSON.parse(texte));
      } catch (e) {
        non(new Error("le corps de la requête n'est pas du JSON lisible"));
      }
    });
    requete.on("error", non);
  });
}

/** Le siège doit être 0 ou 1. « 5 », « les deux » ou rien du tout : non. */
function siegeValide(v) {
  return v === 0 || v === 1 ? v : null;
}

// ------------------------------------------------------- recevoir une décision

async function recevoirDecision(requete, reponse) {
  let corps;
  try {
    corps = await lireCorps(requete);
  } catch (e) {
    refuser(reponse, 400, `Requête illisible : ${e.message}.`);
    return;
  }

  const code = typeof corps.partie === "string" ? corps.partie.trim() : "";
  if (!code) {
    refuser(reponse, 400,
      "Aucun code de partie n'accompagne cette décision : le serveur ne sait pas " +
      "à quelle partie la rattacher.");
    return;
  }
  const p = partieDe(code);

  // (1) LE RANG. Une décision porte le numéro de la question à laquelle elle
  // répond. Le serveur n'attend qu'un seul rang : celui qui suit la dernière
  // décision retenue. Trop loin devant, ou déjà donné : refusé.
  const attendu = p.decisions.length;
  if (!Number.isInteger(corps.rang)) {
    refuser(reponse, 400,
      `Le rang « ${corps.rang} » n'est pas un numéro de décision : le serveur ` +
      `attend le rang ${attendu}.`, `partie « ${code} »`);
    return;
  }
  if (corps.rang !== attendu) {
    const raison = corps.rang < attendu
      ? `la décision de rang ${corps.rang} a déjà été donnée`
      : `le serveur n'a pas encore reçu les décisions qui la précèdent`;
    refuser(reponse, 409,
      `Rang inattendu : ${raison}. Le serveur attend la décision de rang ` +
      `${attendu}, pas celle de rang ${corps.rang}.`, `partie « ${code} »`);
    return;
  }

  // (2) LE SIÈGE. Il n'y a que deux sièges à cette table, le 0 et le 1.
  const siege = siegeValide(corps.siege);
  if (siege === null) {
    refuser(reponse, 403,
      `Le siège « ${corps.siege} » n'existe pas : cette partie n'a que deux ` +
      `sièges, le 0 et le 1.`, `partie « ${code} »`);
    return;
  }

  // (3) LE TOUR. À qui revient cette décision ? Le serveur ne le devine pas :
  // il le tient des moteurs, qui le lui ont déclaré (`POST /relais/tour`). Tant
  // qu'aucun moteur ne s'est prononcé, le premier siège à répondre fixe le tour
  // et l'autre est refusé — jamais les deux.
  const proprietaire = p.tours.get(attendu);
  if (proprietaire !== undefined && proprietaire !== siege) {
    refuser(reponse, 403,
      `Ce n'est pas au siège ${siege} de répondre : la décision de rang ` +
      `${attendu} revient au siège ${proprietaire}. Personne ne répond à la ` +
      `place de l'autre.`, `partie « ${code} »`);
    return;
  }
  if (proprietaire === undefined) p.tours.set(attendu, siege);

  if (corps.reponse === undefined || corps.reponse === null) {
    refuser(reponse, 400,
      "Cette décision ne porte aucune réponse : il n'y a rien à retenir.",
      `partie « ${code} »`);
    return;
  }

  p.decisions.push(corps.reponse);
  dire(`partie « ${code} » : décision ${attendu} reçue du siège ${siege} ` +
       `— réponse ${JSON.stringify(corps.reponse)}`);
  diffuser(p, "decision", { rang: attendu, siege, reponse: corps.reponse });
  repondreJson(reponse, 200, { ok: true, rang: attendu, siege });
}

// ------------------------------------------------------------- déclarer le tour
//
// La page dit ce que SON moteur vient de dire : « la décision de rang R est
// posée au joueur J ». Aucune règle n'est recopiée ici — le serveur ne fait que
// retenir l'annonce, et vérifier que les deux pages disent la même chose.

async function recevoirTour(requete, reponse) {
  let corps;
  try {
    corps = await lireCorps(requete);
  } catch (e) {
    repondreJson(reponse, 400, { ok: false, erreur: `Requête illisible : ${e.message}.` });
    return;
  }
  const code = typeof corps.partie === "string" ? corps.partie.trim() : "";
  const siege = siegeValide(corps.siege);
  if (!code || siege === null || !Number.isInteger(corps.rang)) {
    repondreJson(reponse, 400, {
      ok: false,
      erreur: "Pour annoncer un tour il faut un code de partie, un rang entier " +
              "et un siège qui soit 0 ou 1.",
    });
    return;
  }
  const p = partieDe(code);
  const connu = p.tours.get(corps.rang);
  if (connu !== undefined && connu !== siege) {
    dire(`partie « ${code} » : annonce de tour refusée — le rang ${corps.rang} a ` +
         `déjà été annoncé au siège ${connu}, on l'annonce maintenant au siège ${siege}`);
    repondreJson(reponse, 409, {
      ok: false,
      erreur: `Le rang ${corps.rang} a déjà été annoncé comme revenant au siège ` +
              `${connu} : les deux moteurs ne disent pas la même chose.`,
    });
    return;
  }
  if (connu === undefined) {
    p.tours.set(corps.rang, siege);
    dire(`partie « ${code} » : la décision ${corps.rang} revient au siège ${siege}`);
  }
  repondreJson(reponse, 200, { ok: true, rang: corps.rang, siege });
}

// ----------------------------------------------------------- servir un fichier

const TYPES = new Map(Object.entries({
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
  ".md": "text/plain; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
}));

function servirFichier(requete, reponse, cheminUrl, avecCorps) {
  let relatif = decodeURIComponent(cheminUrl);
  if (relatif === "/" || relatif === "") relatif = "/index.html";
  const complet = path.resolve(LIVRAISON, "." + relatif);
  // Rien en dehors de la livraison, jamais : le dossier servi est une frontière.
  if (complet !== LIVRAISON && !complet.startsWith(LIVRAISON + path.sep)) {
    reponse.writeHead(403, { "Content-Type": "text/plain; charset=utf-8" });
    reponse.end("Ce chemin sort du dossier de la livraison.\n");
    return;
  }
  let infos;
  try {
    infos = fs.statSync(complet);
  } catch {
    reponse.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    reponse.end("Fichier introuvable : " + relatif + "\n");
    return;
  }
  if (infos.isDirectory()) {
    reponse.writeHead(403, { "Content-Type": "text/plain; charset=utf-8" });
    reponse.end("Ceci est un dossier, pas un fichier.\n");
    return;
  }
  // LA DATE DU FICHIER, DONNÉE AU NAVIGATEUR. « no-cache » veut dire « demande-moi
  // à chaque fois si c'est encore bon », pas « ne garde rien » : sans date à
  // comparer, le navigateur ne PEUT que tout retélécharger. Les illustrations
  // pèsent une trentaine de méga-octets ; à travers une liaison publique, une
  // page rechargée en pleine partie mettrait alors une minute à revenir. Avec la
  // date, elle revient instantanément, et une livraison modifiée est quand même
  // reprise — la fraîcheur n'est pas sacrifiée.
  const date = infos.mtime.toUTCString();
  const depuis = requete.headers["if-modified-since"];
  if (depuis && depuis === date) {
    reponse.writeHead(304, { "Last-Modified": date, "Cache-Control": "no-cache" });
    return reponse.end();
  }
  const entetes = {
    "Content-Type": TYPES.get(path.extname(complet).toLowerCase())
                    || "application/octet-stream",
    "Content-Length": infos.size,
    "Last-Modified": date,
    "Cache-Control": "no-cache",
  };
  reponse.writeHead(200, entetes);
  if (!avecCorps) return reponse.end();
  fs.createReadStream(complet).pipe(reponse);
}

// -------------------------------------------------------------- l'aiguillage

const serveur = http.createServer(async (requete, reponse) => {
  let adresse;
  try {
    adresse = new URL(requete.url, "http://127.0.0.1");
  } catch {
    reponse.writeHead(400, { "Content-Type": "text/plain; charset=utf-8" });
    reponse.end("Adresse illisible.\n");
    return;
  }
  const chemin = adresse.pathname;

  try {
    if (chemin === "/relais/etat" && requete.method === "GET") {
      const code = (adresse.searchParams.get("partie") || "").trim();
      if (!code) {
        repondreJson(reponse, 400, {
          ok: false,
          erreur: "Il manque le code de la partie : /relais/etat?partie=<code>.",
        });
        return;
      }
      const souhaits = {};
      const g = adresse.searchParams.get("graine");
      if (g !== null && Number.isInteger(Number(g))) souhaits.graine = Number(g);
      const b = adresse.searchParams.get("boites");
      if (b) souhaits.boites = b;
      repondreJson(reponse, 200, etatDe(partieDe(code, souhaits)));
      return;
    }

    if (chemin === "/relais/decision" && requete.method === "POST") {
      await recevoirDecision(requete, reponse);
      return;
    }

    if (chemin === "/relais/tour" && requete.method === "POST") {
      await recevoirTour(requete, reponse);
      return;
    }

    if (chemin === "/relais/flux" && requete.method === "GET") {
      const code = (adresse.searchParams.get("partie") || "").trim();
      // LE SIÈGE SE LIT STRICTEMENT. `Number(null)` vaut 0 : une adresse sans
      // siège du tout ouvrirait alors un flux au nom du siège 0, et l'autre
      // joueur croirait son adversaire arrivé alors que ce n'est qu'un aperçu
      // de lien ou un robot qui est passé. La présence doit rester CONSTATÉE.
      const brut = adresse.searchParams.get("siege");
      const siege = brut === "0" ? 0 : brut === "1" ? 1 : null;
      if (!code || siege === null) {
        repondreJson(reponse, 400, {
          ok: false,
          erreur: "Pour suivre une partie il faut son code et un siège (0 ou 1).",
        });
        return;
      }
      ouvrirFlux(requete, reponse, code, siege);
      return;
    }

    if (chemin.startsWith("/relais/")) {
      repondreJson(reponse, 404, {
        ok: false,
        erreur: `Le serveur de rendez-vous ne connaît pas « ${chemin} ».`,
      });
      return;
    }

    if (requete.method !== "GET" && requete.method !== "HEAD") {
      reponse.writeHead(405, { "Content-Type": "text/plain; charset=utf-8" });
      reponse.end("Seules les demandes de lecture sont acceptées ici.\n");
      return;
    }
    servirFichier(requete, reponse, chemin, requete.method === "GET");
  } catch (e) {
    dire("panne du serveur sur " + chemin + " : " + (e && e.message ? e.message : e));
    if (!reponse.headersSent) {
      repondreJson(reponse, 500, {
        ok: false,
        erreur: "Le serveur de rendez-vous a rencontré une panne : " +
                (e && e.message ? e.message : String(e)),
      });
    } else {
      reponse.end();
    }
  }
});

// Aucune limite de temps sur les flux d'évènements : ils vivent aussi longtemps
// que la partie. Sans cela, Node les couperait après quelques minutes.
serveur.timeout = 0;
serveur.requestTimeout = 0;
// Les EN-TÊTES, eux, arrivent toujours vite : une connexion qui ne les finit pas
// en une minute est morte, et n'a pas à occuper une place.
serveur.headersTimeout = 60_000;
serveur.keepAliveTimeout = 72_000_000;

const port = lirePort(process.argv.slice(2));
if (!Number.isInteger(port) || port < 0 || port > 65535) {
  console.error(`Le numéro de porte « ${process.argv.slice(2).join(" ")} » n'est pas ` +
                `utilisable. Exemple : node relais/serveur.js --port 8080`);
  process.exit(1);
}

serveur.on("error", (e) => {
  if (e && e.code === "EADDRINUSE") {
    console.error(`La porte ${port} est déjà prise par un autre programme. ` +
                  `Fermez l'autre fenêtre, ou choisissez un autre numéro : ` +
                  `node relais/serveur.js --port ${port + 1}`);
  } else {
    console.error("Le serveur n'a pas pu démarrer : " + (e && e.message ? e.message : e));
  }
  process.exit(1);
});

serveur.listen(port, "127.0.0.1", () => {
  const p = serveur.address().port;
  dire(`la livraison servie depuis ${LIVRAISON}`);
  console.log(`PRET http://127.0.0.1:${p}`);
  dire("le rendez-vous attend les deux joueurs. Ctrl-C pour arrêter.");
});

// ------------------------------------------------------------ s'arrêter net
//
// Le port doit être rendu à l'arrêt : un port resté ouvert empêche la prochaine
// mise en route. Les flux d'évènements sont des connexions ouvertes qui, seules,
// retiendraient Node indéfiniment — on les coupe d'abord.

let enFermeture = false;
function arreter(signal) {
  if (enFermeture) return;
  enFermeture = true;
  dire(`arrêt demandé (${signal}) — le rendez-vous se ferme`);
  for (const p of parties.values()) {
    for (const siege of [0, 1]) {
      for (const r of p.flux[siege]) {
        try {
          r.end();
          r.destroy?.();
        } catch { /* déjà morte */ }
      }
      p.flux[siege].clear();
    }
  }
  serveur.closeAllConnections?.();
  serveur.close(() => process.exit(0));
  // Filet : si une connexion s'accroche, on part quand même.
  setTimeout(() => process.exit(0), 1500).unref();
}

process.on("SIGTERM", () => arreter("SIGTERM"));
process.on("SIGINT", () => arreter("SIGINT"));

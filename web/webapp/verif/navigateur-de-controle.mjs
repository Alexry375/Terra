// (jouer-contre-l-apprenti) Ce dont les deux bancs au navigateur ont besoin, une
// seule fois : Chromium (Playwright, installé hors du dépôt) et un serveur de
// contrôle à nous.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
export { chromium } from "/home/alexis/moodle-qui-parle/node_modules/playwright/index.mjs";

const DEPOT = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

/**
 * Un serveur de contrôle À NOUS — `web/servir.py`, sur 127.0.0.1, sur le premier
 * port libre entre 8030 et 8099. Jamais le service de l'utilisateur.
 */
export async function servir() {
  for (let port = 8030; port <= 8099; port++) {
    // `-u` : servir.py annonce son adresse APRÈS avoir pris le port ; sans tampon,
    // cette annonce prouve que c'est bien NOTRE serveur qui répondra.
    const serveur = spawn("python3", ["-u", resolve(DEPOT, "web/servir.py"), String(port), "127.0.0.1"], {
      stdio: ["ignore", "pipe", "ignore"],
    });
    const pris = await new Promise((ok) => {
      serveur.stdout.once("data", () => ok(true));
      serveur.once("exit", () => ok(false));
    });
    if (!pris) continue;
    const adresse = `http://127.0.0.1:${port}`;
    for (let i = 0; i < 40; i++) {
      try {
        if ((await fetch(`${adresse}/index.html`)).ok) return { serveur, adresse };
      } catch { /* pas encore prêt */ }
      await new Promise((r) => setTimeout(r, 250));
    }
    serveur.kill();
  }
  throw new Error("aucun port libre entre 8030 et 8099 pour le serveur de contrôle");
}

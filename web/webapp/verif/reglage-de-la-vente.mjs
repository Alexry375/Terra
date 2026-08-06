// LE RÉGLAGE DE LA VENTE DU JOUEUR `reflechi` — et les graines sur lesquelles
// il a le droit de se faire.
//
// Ce banc mesure plusieurs variantes de (`prixVenteMini`, `gardeMini`) contre le
// joueur au hasard, et n'emploie QUE les graines 7000 et au-delà. Les graines
// 1 à 100 servent à la mesure finale (contrôle 04) et ne doivent jamais servir
// à régler : un joueur réglé sur les parties qu'on va montrer n'a pas appris à
// jouer, il a appris ces parties-là.
//
// Chaque variante joue une COPIE de `joueurs/reflechi.js` où les deux nombres
// ont été substitués — le fichier livré n'est jamais modifié, et la copie est
// bien le joueur livré, pas une réécriture de son cerveau. Les imports relatifs
// de la copie sont réécrits en chemins absolus, puisqu'elle vit ailleurs.
//
//   node web/webapp/verif/reglage-de-la-vente.mjs [nbGraines] [premiereGraine]
//
// Sortie : une ligne par variante — victoires, nuls, ventes, parties arrêtées.

import { resolve, dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";

const ICI = dirname(fileURLToPath(import.meta.url));
const W = resolve(ICI, "..");

const { ouvrirPontDepuis } = await import(join(W, "pont.js"));
const { creerPartie, jouerJusquAuBout } = await import(join(W, "partie.js"));
const { fournisseurAleatoire } = await import(join(W, "fournisseurs.js"));

const GRAINES = Number(process.argv[2] || 60);
const PREMIERE = Number(process.argv[3] || 7000);
if (PREMIERE < 7000 || PREMIERE + GRAINES > 8000) {
  console.error(
    `REFUSÉ : le réglage ne se fait que sur les graines 7000 à 7999 ` +
      `(demandé ${PREMIERE}..${PREMIERE + GRAINES - 1}).`,
  );
  process.exit(2);
}

// Les variantes mesurées. `null` = le joueur qui ne vend jamais (le témoin :
// c'est lui qui gagnait 94,5 % avant ce chantier).
const VARIANTES = [
  null,
  { prixVenteMini: 10, gardeMini: 4 },
  { prixVenteMini: 14, gardeMini: 4 },
  { prixVenteMini: 18, gardeMini: 4 },
  { prixVenteMini: 22, gardeMini: 4 },
  { prixVenteMini: 26, gardeMini: 4 },
  { prixVenteMini: 18, gardeMini: 2 },
  { prixVenteMini: 18, gardeMini: 6 },
  { prixVenteMini: 14, gardeMini: 2 },
  { prixVenteMini: 14, gardeMini: 6 },
];

// Un second tour, sur d'AUTRES graines, ne rejoue que les finalistes : c'est ce
// qui distingue un écart réel du bruit d'un lot de 60 parties (± 5 points).
//   VARIANTES=0,1,7 node … 100 7100
const FILTRE = (process.env.VARIANTES || "")
  .split(",")
  .map((x) => Number(x.trim()))
  .filter((x) => Number.isInteger(x));

const SOURCE = readFileSync(join(W, "joueurs/reflechi.js"), "utf8");
const DOSSIER = mkdtempSync(join(tmpdir(), "reglage-vente-"));

/** Une copie du joueur livré, avec deux nombres substitués. */
async function joueurAvec(variante, rang) {
  if (!variante) {
    // Le témoin : la copie garde le cerveau livré, mais on lui coupe la vente
    // en lui donnant un prix qu'aucune carte n'atteint.
    variante = { prixVenteMini: 1e9, gardeMini: 0 };
  }
  let src = SOURCE.replace(
    /prixVenteMini: [-\d.e+]+/,
    `prixVenteMini: ${variante.prixVenteMini}`,
  )
    .replace(/gardeMini: [-\d.e+]+/, `gardeMini: ${variante.gardeMini}`)
    // La copie vit hors du dossier : ses imports relatifs doivent devenir absolus.
    .replace(/from "\.\.\//g, `from "${pathToFileURL(W + "/").href}`);
  if (!src.includes(`prixVenteMini: ${variante.prixVenteMini}`)) {
    throw new Error("la substitution de `prixVenteMini` a échoué : le fichier a changé de forme");
  }
  const chemin = join(DOSSIER, `reflechi-${rang}.js`);
  writeFileSync(chemin, src);
  const mod = await import(pathToFileURL(chemin).href);
  return mod.fournisseurReflechi;
}

const pont = await ouvrirPontDepuis(W);

console.log(
  `    ${VARIANTES.length} variante(s), graines ${PREMIERE}..${PREMIERE + GRAINES - 1}, ` +
    `${GRAINES * 2} parties chacune`,
);

for (const [rang, variante] of VARIANTES.entries()) {
  if (FILTRE.length && !FILTRE.includes(rang)) continue;
  const fabrique = await joueurAvec(variante, rang);
  let gagnees = 0;
  let nulles = 0;
  let jouees = 0;
  let ventes = 0;
  let cartes = 0;
  const arrets = [];
  for (let g = PREMIERE; g < PREMIERE + GRAINES; g++) {
    for (const siege of [0, 1]) {
      const partie = creerPartie(pont, { graine: g, boites: "base,decouverte" });
      const brut = fabrique(g * 7 + siege, "reflechi");
      const compte = {
        nom: "reflechi",
        async decider(d, etat) {
          const r = await brut.decider(d, etat);
          if (r && typeof r === "object" && r.vendre) {
            ventes++;
            cartes += (r.vendre.cartes || []).length;
          }
          return r;
        },
      };
      const hasard = fournisseurAleatoire(g * 977 + siege, "hasard");
      try {
        await jouerJusquAuBout(partie, siege === 0 ? [compte, hasard] : [hasard, compte]);
        const s = partie.scores || [];
        if (s.length < 2) throw new Error("partie terminee sans scores");
        jouees++;
        if (s[siege] > s[1 - siege]) gagnees++;
        else if (s[siege] === s[1 - siege]) nulles++;
      } catch (e) {
        arrets.push(`graine ${g}, siege ${siege} : ${String((e && e.message) || e).slice(0, 90)}`);
      }
    }
  }
  const nom = variante
    ? `prix mini ${String(variante.prixVenteMini).padStart(3)} garde ${variante.gardeMini}`
    : "temoin (ne vend jamais)   ";
  const part = jouees ? (100 * gagnees) / jouees : 0;
  console.log(
    `    ${nom} : ${String(gagnees).padStart(3)}/${jouees} (${part.toFixed(1)} %), ` +
      `${nulles} nul(s), ${ventes} vente(s) / ${cartes} carte(s), ${arrets.length} arret(s)`,
  );
  for (const a of arrets.slice(0, 2)) console.log("        " + a);
}

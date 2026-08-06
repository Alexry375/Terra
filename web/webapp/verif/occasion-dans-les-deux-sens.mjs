// L'OCCASION DE VENDRE, ÉPROUVÉE DANS LES DEUX SENS, SUR LE VRAI HARNAIS.
//
// `etat.occasion_de_vendre_ouverte[siège]` promet une chose précise : « si ce
// siège rend une entrée de vente à ce point-ci, elle sera acceptée ». Cette
// promesse ne se vérifie pas côté moteur : la consommation d'une entrée vit
// dans le pont (`wasm/src/lib.rs`, `Harnais::vendre_librement` / `prendre`), et
// le rejeu rejoue toute la partie à chaque coup. Les tests Rust du dépôt
// (`engine/tests/occasion_de_vendre_tests.rs`) ne peuvent en éprouver que la
// moitié moteur.
//
// Ce banc-ci éprouve la promesse ENTIÈRE, et dans LES DEUX SENS :
//
//   · là où l'état dit OUVERTE, on vend pour de bon → ce doit être accepté ;
//   · là où l'état dit FERMÉE, on TENTE une vente → ce doit être refusé.
//
// Le second sens est possible sans empoisonner la partie parce que
// `partie.repondre` retire l'entrée refusée avant de relever l'erreur
// (`partie.js`) : on tente, on note, on continue comme si de rien n'était.
//
// Un banc qui ne ferait que le premier sens serait vert sur un état qui
// répondrait « ouverte » partout — c'est-à-dire sur le défaut d'avant ce
// chantier, à peine déguisé.
//
//   node web/webapp/verif/occasion-dans-les-deux-sens.mjs [nbGraines] [premiere]

import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ICI = dirname(fileURLToPath(import.meta.url));
const W = resolve(ICI, "..");

const { ouvrirPontDepuis } = await import(join(W, "pont.js"));
const { creerPartie } = await import(join(W, "partie.js"));
const { fournisseurAleatoire } = await import(join(W, "fournisseurs.js"));

const GRAINES = Number(process.argv[2] || 12);
const PREMIERE = Number(process.argv[3] || 1);

const pont = await ouvrirPontDepuis(W);

let ouvertesAcceptees = 0;
let ouvertesRefusees = 0;
let fermeesTentees = 0;
let fermeesHonorees = 0;
let fermeesDetournees = 0;
let deplacees = 0;
const fautes = [];
const manquees_vues = [];

for (let g = PREMIERE; g < PREMIERE + GRAINES; g++) {
  const partie = creerPartie(pont, { graine: g, boites: "base,decouverte" });
  const hasard = [fournisseurAleatoire(g * 31, "h0"), fournisseurAleatoire(g * 37 + 1, "h1")];
  let garde = 0;
  while (!partie.termine) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const d = partie.decision;
    if (!d) throw new Error("ni decision ni fin de partie");
    const etat = partie.etat;
    const siege = d.joueur;
    const ouvertes = (etat && etat.occasion_de_vendre_ouverte) || [];
    const ouverte = ouvertes[siege] === true;
    const main = ((etat.players || [])[siege] || {}).hand || [];

    if (main.length >= 2) {
      // On vend TOUJOURS la DERNIÈRE carte de la main : c'est l'indice le plus
      // fragile à un décalage. Si l'entrée était consommée à une autre occasion
      // que celle que l'état décrit, la carte partie ne serait pas celle-là.
      const i = main.length - 1;
      const cible = main[i] && main[i].id;
      const entree = { vendre: { joueur: siege, cartes: [i] } };
      let acceptee = true;
      try {
        partie.repondre(entree);
      } catch (e) {
        acceptee = false;
        if (ouverte) {
          ouvertesRefusees++;
          if (fautes.length < 5) {
            fautes.push(
              `graine ${g} : l'etat disait OUVERTE et le moteur a refuse — ` +
                String((e && e.message) || e).slice(0, 90),
            );
          }
        }
      }
      if (acceptee) {
        if (ouverte) {
          ouvertesAcceptees++;
          // La carte partie est-elle celle qu'on avait désignée ?
          const apres = ((partie.etat.players || [])[siege] || {}).hand || [];
          const restait = apres.some((c) => c && c.id === cible);
          const bornee = apres.length === main.length; // rien n'est parti
          if (restait && !bornee) {
            deplacees++;
            if (fautes.length < 5) {
              fautes.push(`graine ${g} : la vente a emporte une AUTRE carte que celle designee`);
            }
          }
        } else {
          fermeesTentees++;
          // ACCEPTÉE ALORS QUE L'ÉTAT DISAIT FERMÉE. « Acceptée » ne veut pas
          // dire « honorée » : l'entrée peut être consommée par une occasion
          // qui ne portait pas la même main, et vendre alors une AUTRE carte
          // que celle désignée. C'est même la raison d'être du garde
          // `mains_a_l_occasion` : le moteur ferme l'occasion PRÉCISÉMENT là où
          // il ne peut pas garantir ce que les indices désignent. On sépare
          // donc les deux, parce qu'ils n'ont pas la même gravité.
          const apres = ((partie.etat.players || [])[siege] || {}).hand || [];
          const partie_juste = !apres.some((c) => c && c.id === cible);
          if (partie_juste) {
            fermeesHonorees++;
            if (manquees_vues.length < 3) {
              manquees_vues.push(
                `graine ${g} : FERMEE, vente acceptee ET honoree (occasion manquee, ` +
                  `vente_offerte=${etat.vente_offerte})`,
              );
            }
          } else {
            fermeesDetournees++;
            if (fautes.length < 5) {
              fautes.push(
                `graine ${g} : FERMEE, entree consommee mais la carte designee est ` +
                  `RESTEE — l'etat avait raison de fermer`,
              );
            }
          }
        }
        continue; // la même question revient : on y répondra au tour suivant
      }
      if (!ouverte) fermeesTentees++;
    }
    partie.repondre(await hasard[siege].decider(d, partie.etat));
  }
}

console.log(
  `    ouverte -> acceptee ${ouvertesAcceptees}, ouverte -> REFUSEE ${ouvertesRefusees} ; ` +
    `fermee tentee ${fermeesTentees}, fermee -> honoree ${fermeesHonorees}, ` +
    `fermee -> detournee ${fermeesDetournees} ; ` +
    `ventes deplacees ${deplacees}`,
);
for (const f of fautes) console.log("      " + f);
for (const m of manquees_vues) console.log("      " + m);

const total = ouvertesAcceptees + fermeesTentees;
if (total < 200) {
  console.log(`KO banc vide : ${total} essai(s) seulement`);
  process.exit(1);
}
if (fermeesTentees === 0) {
  console.log("KO aucune occasion FERMEE n'a ete tentee : le second sens n'est pas eprouve");
  process.exit(1);
}
// UNE OCCASION FERMÉE QUI AURAIT ÉTÉ HONORÉE est une occasion MANQUÉE : le
// joueur perd une vente qu'il aurait pu faire. C'est le sens sûr de l'erreur
// (« au pire un bouton non offert, jamais une vente refusée », `flow::observer`)
// et le moteur l'assume ; on le CHIFFRE au lieu de le taire.
if (ouvertesRefusees || fermeesDetournees || deplacees) {
  console.log("KO l'etat ne dit pas la verite sur l'occasion de vendre");
  process.exit(1);
}
const manquees = fermeesTentees ? (100 * fermeesHonorees) / fermeesTentees : 0;
console.log(
  `    occasions manquees : ${fermeesHonorees}/${fermeesTentees} (${manquees.toFixed(1)} %) — ` +
    `l'etat ferme par prudence la ou \`vente_offerte\` est faux (garde ` +
    `\`mains_a_l_occasion\`, anterieur a ce chantier)`,
);
console.log(
  "OK l'etat ne ment jamais dans le sens dangereux : ouverte => acceptee et honoree, " +
    "fermee => jamais detournee",
);

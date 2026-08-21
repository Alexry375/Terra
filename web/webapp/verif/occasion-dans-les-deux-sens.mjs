#!/usr/bin/env node
// **L'OCCASION DE VENDRE, ÉPROUVÉE DANS LES DEUX SENS, SUR LE VRAI HARNAIS.**
//
//   node web/webapp/verif/occasion-dans-les-deux-sens.mjs [nbGraines] [premiere]
//
// `etat.occasion_de_vendre_ouverte[siège]` promet une chose précise : « si ce
// siège rend une entrée de vente à ce point-ci, elle sera acceptée ». Cette
// promesse ne se vérifie pas côté moteur : la consommation d'une entrée vit
// dans le pont (`wasm/src/lib.rs`, `Harnais::vendre_librement` / `prendre`), et
// le rejeu rejoue toute la partie à chaque coup. Les tests Rust du dépôt
// (`engine/tests/occasion_de_vendre_tests.rs`) ne peuvent en éprouver que la
// moitié moteur.
//
// Ce banc éprouve la promesse ENTIÈRE, et dans LES DEUX SENS :
//
//   · là où l'état dit OUVERTE, on vend pour de bon → ce doit être accepté ;
//   · là où l'état dit FERMÉE, on TENTE une vente → ce ne doit jamais être
//     détourné vers une autre carte.
//
// Le second sens est possible sans empoisonner la partie parce que
// `partie.repondre` retire l'entrée refusée avant de relever l'erreur
// (`partie.js`) : on tente, on note, on continue comme si de rien n'était.
//
// **(le-pont-ne-triche-plus, critère E / défaut V2) LE NUMÉRO DE L'OCCASION.**
//
// Une vente n'est pas une réponse à une question : le moteur ne demande jamais
// « voulez-vous vendre ? ». Il ouvre une OCCASION avant chacun de ses points de
// décision, et l'entrée que la page inscrit est consommée à la première occasion
// que le rejeu rencontre pour ce siège-là. Or plusieurs occasions peuvent être
// ouvertes en même temps : le moteur enchaîne les points d'occasion des deux
// sièges avant de s'arrêter sur une question. Une vente décidée en regardant la
// SECONDE de ces occasions était alors consommée à la PREMIÈRE — c'est-à-dire
// appliquée à un instant que le joueur n'avait pas devant les yeux.
//
// D'où le numéro, et la règle « jamais avant son numéro » que ce banc mesure :
//
//   3. une entrée numérotée n est bien honorée à l'occasion n ;
//   4. le numéro n'est pas décoratif — deux numéros différents donnent deux
//      parties différentes, et une entrée SANS numéro retombe à la PREMIÈRE
//      occasion (le comportement d'avant, conservé pour l'écran de jeu) ;
//   5. une entrée numérotée pour une occasion À VENIR est REFUSÉE au point
//      courant : la faute est déclarée, `partie.vendre` retire l'entrée, et
//      rien n'a bougé ;
//   6. une entrée numérotée qui vise une occasion de l'AUTRE siège attend la
//      première occasion du SIEN qui soit au moins aussi tardive ;
//   7. les numéros d'occasion se suivent sans trou et ne reculent jamais.

import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ICI = dirname(fileURLToPath(import.meta.url));
const W = resolve(ICI, "..");

const { ouvrirPontDepuis } = await import(join(W, "pont.js"));
const { creerPartie } = await import(join(W, "partie.js"));
const { fournisseurAleatoire } = await import(join(W, "fournisseurs.js"));

// Vingt-quatre graines par défaut : c'est ce qui donne assez de PAIRES
// d'occasions ouvertes au même point — la configuration exacte du défaut V2 —
// pour que le compte publié soit une mesure et non une anecdote. Huit graines
// n'en donnaient que cinquante-trois, trois n'en donnaient que dix-sept.
const GRAINES = Number(process.argv[2] || 24);
const PREMIERE = Number(process.argv[3] || 1);
const BOITES = "base,decouverte";

const pont = await ouvrirPontDepuis(W);

const fautes = [];
let cas = 0;
function faute(m) {
  if (fautes.length < 12) fautes.push(m);
  else if (fautes.length === 12) fautes.push("… (autres fautes tues)");
}

// ════════════════════════════════════════════════════════════════════════════
// I. LES DEUX SENS DE LA PROMESSE `occasion_de_vendre_ouverte`
// ════════════════════════════════════════════════════════════════════════════

let ouvertesAcceptees = 0;
let ouvertesRefusees = 0;
let fermeesTentees = 0;
let fermeesHonorees = 0;
let fermeesDetournees = 0;
let deplacees = 0;
const manquees_vues = [];

for (let g = PREMIERE; g < PREMIERE + GRAINES; g++) {
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
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
      cas++;
      try {
        partie.repondre(entree);
      } catch (e) {
        acceptee = false;
        if (ouverte) {
          ouvertesRefusees++;
          faute(
            `graine ${g} : l'etat disait OUVERTE et le moteur a refuse — ` +
              String((e && e.message) || e).slice(0, 90),
          );
        }
      }
      if (acceptee) {
        if (ouverte) {
          ouvertesAcceptees++;
          const apres = ((partie.etat.players || [])[siege] || {}).hand || [];
          const restait = apres.some((c) => c && c.id === cible);
          const bornee = apres.length === main.length; // rien n'est parti
          if (restait && !bornee) {
            deplacees++;
            faute(`graine ${g} : la vente a emporte une AUTRE carte que celle designee`);
          }
        } else {
          fermeesTentees++;
          // ACCEPTÉE ALORS QUE L'ÉTAT DISAIT FERMÉE. « Acceptée » ne veut pas
          // dire « honorée » : l'entrée peut être consommée par une occasion
          // qui ne portait pas la même main, et vendre alors une AUTRE carte
          // que celle désignée. C'est même la raison d'être du garde
          // `mains_a_l_occasion` : le moteur ferme l'occasion PRÉCISÉMENT là où
          // il ne peut pas garantir ce que les indices désignent.
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
            faute(
              `graine ${g} : FERMEE, entree consommee mais la carte designee est ` +
                `RESTEE — l'etat avait raison de fermer`,
            );
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
  `I. ouverte -> acceptee ${ouvertesAcceptees}, ouverte -> REFUSEE ${ouvertesRefusees} ; ` +
    `fermee tentee ${fermeesTentees}, fermee -> honoree ${fermeesHonorees}, ` +
    `fermee -> detournee ${fermeesDetournees} ; ventes deplacees ${deplacees}`,
);
for (const m of manquees_vues) console.log("      " + m);

const totalI = ouvertesAcceptees + fermeesTentees;
if (totalI < 200) faute(`banc vide : ${totalI} essai(s) seulement dans la partie I`);
if (fermeesTentees === 0) faute("aucune occasion FERMEE n'a ete tentee : le second sens n'est pas eprouve");
if (ouvertesRefusees || fermeesDetournees || deplacees) {
  faute("l'etat ne dit pas la verite sur l'occasion de vendre");
}
const manquees = fermeesTentees ? (100 * fermeesHonorees) / fermeesTentees : 0;
console.log(
  `   occasions manquees : ${fermeesHonorees}/${fermeesTentees} (${manquees.toFixed(1)} %) — ` +
    "l'etat ferme par prudence la ou `vente_offerte` est faux (garde " +
    "`mains_a_l_occasion`, anterieur a ce chantier)",
);

// ════════════════════════════════════════════════════════════════════════════
// II. LE NUMÉRO DE L'OCCASION
// ════════════════════════════════════════════════════════════════════════════

// **TOUTES LES GRAINES, PAS TROIS.** La partie II est celle qui éprouve le
// défaut V2 lui-même — deux occasions ouvertes au même point, et la vente qui
// doit tomber à la seconde. Trois graines n'en donnaient que dix-sept paires :
// trop peu pour que le compte veuille dire quelque chose sur le seul défaut que
// ce lot ferme. On balaye les mêmes graines que la partie I.
const GRAINES_II = GRAINES;
let paires = 0; // points où un siège a DEUX occasions ouvertes à la fois
let numeroHonore = 0;
let numeroBorne = 0;
let numeroDecoratif = 0;
let sansNumeroTombePremiere = 0;
let sansNumeroDeplace = 0;
let refusesFutur = 0;
let acceptesFutur = 0;
let partieAbimee = 0;
let attentesAutreSiege = 0;
let attentesAutreSiegeFautives = 0;
let pointsNumerotes = 0;

/** L'identité de la main d'un siège dans un état, sous forme comparable. */
const mainDe = (etat, s) => (((etat && etat.players) || [])[s] || {}).hand || [];
const ids = (main) => main.map((c) => c && c.id).join(",");

for (let g = PREMIERE; g < PREMIERE + GRAINES_II; g++) {
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  const hasard = [fournisseurAleatoire(g * 31, "h0"), fournisseurAleatoire(g * 37 + 1, "h1")];
  let garde = 0;
  let precedent = -1;
  while (!partie.termine) {
    if (++garde > 100000) throw new Error("boucle anormalement longue");
    const D = partie.decisions;
    const occ = partie.occasions;

    // ── 7. les numéros ne reculent jamais et ne sautent pas ────────────────
    cas++;
    for (const o of occ) {
      if (o.numero <= precedent) {
        faute(`graine ${g} : l'occasion ${o.numero} revient apres ${precedent} — les numeros reculent`);
      }
      if (o.numero >= partie.occasionsOuvertes) {
        faute(`graine ${g} : l'occasion ${o.numero} est publiee alors que le compteur vaut ${partie.occasionsOuvertes}`);
      }
      precedent = o.numero;
    }

    if (occ.length > 0) pointsNumerotes++;

    // **UN REFUS DU MOTEUR EST UNE FAUTE, PAS UNE MORT.** Le banc doit rendre
    // son verdict meme quand le pont rejette une entree qu'il aurait du honorer :
    // une trace de pile n'est ni « VERT » ni « ROUGE », et le controle ne saurait
    // pas dire ce qui est tombe. On attrape donc, on nomme, et on continue.
    const pasProtege = (entree, quoi) => {
      try {
        return pont.pas(g, BOITES, [...D, entree]);
      } catch (e) {
        faute(`graine ${g} : ${quoi} — le moteur a refuse l'entree : ${e.message}`);
        return null;
      }
    };

    // ── 3. une entrée numérotée est honorée à SON occasion ─────────────────
    for (const o of occ) {
      if (o.main.length === 0) continue;
      cas++;
      const i = o.main.length - 1;
      const cible = o.main[i].id;
      const r = pasProtege(
        { vendre: { cartes: [i], joueur: o.joueur, occasion: o.numero } },
        `entree numerotee ${o.numero} a son occasion`,
      );
      if (!r) break;
      const apres = mainDe(r.etat, o.joueur);
      const restee = apres.some((c) => c && c.id === cible);
      if (!restee) {
        numeroHonore++;
      } else if (apres.length === o.main.length) {
        // (MOT-13) LA DETTE PASSE AVANT LA VENTE : quand une defausse imposee
        // attend, `flow::borner_la_vente` retire de la vente les cartes qu'elle
        // reserve. L'entree est bien consommee a SON occasion — rien n'est parti,
        // et c'est la regle, pas un decalage. On le CHIFFRE au lieu de le taire.
        numeroBorne++;
      } else {
        faute(
          `graine ${g} : entree numerotee ${o.numero} detournee — la carte ${cible} est ` +
            `restee et une AUTRE est partie (main ${o.main.length} -> ${apres.length})`,
        );
      }
      break; // une seule par point suffit : le banc traverse toute la partie
    }

    // ── 4. le numéro n'est pas décoratif ; sans numéro, on retombe à la
    //       PREMIÈRE occasion — c'est exactement le trou que ce lot bouche ──
    const parSiege = new Map();
    for (const o of occ) {
      if (!parSiege.has(o.joueur)) parSiege.set(o.joueur, []);
      parSiege.get(o.joueur).push(o);
    }
    for (const [s, liste] of parSiege) {
      if (liste.length < 2 || liste[0].main.length === 0) continue;
      paires++;
      cas++;
      const vente = (n) => {
        const v = { cartes: [0], joueur: s };
        if (n !== undefined) v.occasion = n;
        const r = pasProtege({ vendre: v }, `vente numerotee ${n === undefined ? "(sans numero)" : n}`);
        return r === null ? `refus-${n}` : JSON.stringify(r);
      };
      const a = vente(liste[0].numero);
      const b = vente(liste[1].numero);
      const nu = vente(undefined);
      if (a === b) {
        numeroDecoratif++;
        faute(
          `graine ${g} : occasions ${liste[0].numero} et ${liste[1].numero} du siege ${s} ` +
            "rendent la MEME partie — le numero ne sert a rien",
        );
      }
      if (nu === a) sansNumeroTombePremiere++;
      else {
        sansNumeroDeplace++;
        faute(
          `graine ${g} : une entree SANS numero ne tombe pas a la premiere occasion ` +
            `(${liste[0].numero}) — la compatibilite avec l'ecran de jeu est cassee`,
        );
      }
      break;
    }

    // ── 5. JAMAIS AVANT SON NUMÉRO : une entrée numérotée pour une occasion à
    //       venir est REFUSÉE ici, et la partie n'en garde aucune trace ─────
    if (occ.length > 0 && occ[0].main.length > 0) {
      cas++;
      const s = occ[0].joueur;
      const futur = partie.occasionsOuvertes + 1000; // aucune occasion ne l'atteindra
      const avantEtat = JSON.stringify(partie.etat);
      const avantN = D.length;
      let refusee = false;
      try {
        partie.vendre({ cartes: [0], joueur: s, occasion: futur });
      } catch (e) {
        refusee = true;
      }
      if (refusee) refusesFutur++;
      else {
        acceptesFutur++;
        faute(
          `graine ${g} : une vente numerotee ${futur} a ete consommee alors qu'aucune ` +
            "occasion ne porte ce numero — le rejet « jamais avant son numero » ne marche pas",
        );
      }
      if (partie.decisions.length !== avantN || JSON.stringify(partie.etat) !== avantEtat) {
        partieAbimee++;
        faute(`graine ${g} : le refus d'une vente mal numerotee a laisse la partie modifiee`);
      }
    }

    // ── 6. une entrée visant une occasion de l'AUTRE siège attend la
    //       première occasion du SIEN qui soit au moins aussi tardive ───────
    for (const [s, liste] of parSiege) {
      const autre = occ.find((o) => o.joueur !== s && o.numero > liste[0].numero);
      const suivante = liste.find((o) => autre && o.numero > autre.numero);
      if (!autre || !suivante || liste[0].main.length === 0) continue;
      cas++;
      attentesAutreSiege++;
      const rAttendu = pasProtege(
        { vendre: { cartes: [0], joueur: s, occasion: suivante.numero } },
        `vente du siege ${s} numerotee ${suivante.numero}`,
      );
      const rObtenu = pasProtege(
        { vendre: { cartes: [0], joueur: s, occasion: autre.numero } },
        `vente du siege ${s} numerotee ${autre.numero} (occasion de l'autre siege)`,
      );
      if (rAttendu === null || rObtenu === null) break;
      const attendu = JSON.stringify(rAttendu);
      const obtenu = JSON.stringify(rObtenu);
      if (attendu !== obtenu) {
        attentesAutreSiegeFautives++;
        faute(
          `graine ${g} : une vente du siege ${s} numerotee ${autre.numero} (occasion de ` +
            `l'autre siege) n'a pas ete consommee a ${suivante.numero}, la premiere du sien`,
        );
      }
      break;
    }

    const d = partie.decision;
    if (!d) break;
    partie.repondre(await hasard[d.joueur].decider(d, partie.etat));
  }
}

console.log(
  `II. points a occasion ouverte ${pointsNumerotes} ; entrees numerotees honorees ${numeroHonore} ` +
    `, bornees par une defausse imposee ${numeroBorne} (MOT-13) ; ` +
    `paires d'occasions du meme siege ${paires} (numero decoratif ${numeroDecoratif}) ; ` +
    `sans numero -> premiere occasion ${sansNumeroTombePremiere} (deplacees ${sansNumeroDeplace})`,
);
console.log(
  `   ventes numerotees dans le FUTUR : ${refusesFutur} refus, ${acceptesFutur} acceptees a tort, ` +
    `${partieAbimee} partie(s) abimee(s) par le refus ; ` +
    `numero de l'autre siege reporte correctement ${attentesAutreSiege - attentesAutreSiegeFautives}/${attentesAutreSiege}`,
);

if (paires === 0) faute("aucune paire d'occasions du meme siege : le sens dangereux n'est pas eprouve");
if (refusesFutur === 0) faute("aucun rejet d'entree numerotee dans le futur : le refus n'est pas eprouve");
if (numeroHonore === 0) faute("aucune entree numerotee honoree : le numero n'est pas eprouve");

// ── verdict ─────────────────────────────────────────────────────────────────
for (const f of fautes) console.log(`  ✗ ${f}`);
if (fautes.length > 0) {
  console.log(`ROUGE ${fautes.length} faute(s) sur ${cas} cas eprouves — une vente peut encore tomber a la mauvaise occasion`);
  process.exit(1);
}
console.log(
  `VERT ${cas} cas eprouves (${totalI} ventes dans les deux sens, ${paires} paires d'occasions, ` +
    `${refusesFutur} refus d'entree mal numerotee) : une vente ne tombe qu'a son occasion`,
);

// TERRA — l'écran du jeu.
//
// Le moteur de règles (Rust, WebAssembly) décide de TOUT. Cette page ne connaît
// aucune règle : elle ne sait pas ce qu'une carte coûte, ni si elle est jouable,
// ni combien vaut un point. Elle fait exactement deux choses :
//
//   1. donner à voir `etat` — l'état que le moteur rend lui-même à chaque
//      décision (`engine::observe::state_view`). Chaque nombre affiché porte son
//      chemin exact dans cet objet (`data-valeur`), pour qu'on puisse vérifier à
//      tout instant que l'écran ne ment pas ;
//   2. offrir `decision.options` — et rien d'autre — puis rendre au moteur
//      l'indice choisi.
//
// UN SEUL POINT DE VUE, DEUX RÉGLAGES INDÉPENDANTS. L'écran ne montre plus les
// deux mains en clair : il montre CELLE DU SIÈGE REGARDÉ, en bas, et celle de
// l'adversaire en haut, retournée. Deux réglages, jamais confondus :
//
//   ?siege=0|1               quel joueur occupe le bas de l'écran
//   ?decide=humain|programme qui répond pour ce siège
//
// « Le siège du bas » et « celui qui clique » ne sont pas le même concept : mon
// intelligence artificielle peut tenir mon siège pendant que je la regarde
// jouer, cartes en clair, comme si c'était moi (`docs/INTERFACE_RETOURS_02.md`,
// point 26-bis). L'adversaire, lui, est toujours tenu par un programme
// (`fournisseurs.js`) et reste opaque.

// ---------------------------------------------------------------------------
// LA COUTURE DES TROIS CHANTIERS — ce fichier est le seul que deux d'entre eux
// ont réécrit au même endroit. Qui a apporté quoi, et pourquoi cet arrangement :
//
//   · `table-vivante` — la table remplace la liste de cartes Phase
//     (`construireTable` / `majTable` / `oublierTable` au lieu de
//     `construirePhases`), et la pause locale disparaît au profit de celle de
//     `vue/anim.js`, qui sait la mettre à zéro quand `?animations=non` est là.
//     `majPhases` reste appelé AVANT `majTable` : c'est lui qui sait où en est
//     la planification, et la table lit sa réponse.
//   · `menu-et-options` — l'écran d'accueil sort d'ici pour aller dans
//     `vue/menu.js`, et tout le coupe-circuit de la partie (`ABANDON`,
//     `coupeCircuit`, `sousCoupeCircuit`, `retourAuMenu`) s'y ajoute.
//   · `bandeau-et-monde` — n'a pas touché ce fichier.
//
// Les deux chantiers ne se croisent qu'à deux lignes, et sans se contredire :
// dans `lancer`, l'oubli de la table (`table-vivante`) précède la pose du
// coupe-circuit (`menu-et-options`) ; dans les imports, chacun ajoute les siens.
//
// DEUX ARBITRAGES ONT ÉTÉ NÉCESSAIRES, et ils sont commentés là où ils vivent :
//   · le réglage des animations, écrit deux fois, chacun de son côté — note en
//     tête de `vue/anim.js` ;
//   · la respiration de l'adversaire (`PAS_ADVERSE`), que `table-vivante` rend
//     zérotable et dont le contrôle `20` de `menu-et-options` a besoin — note
//     dans `adversaire()`, plus bas. C'est la seule contradiction franche des
//     trois livraisons.
// ---------------------------------------------------------------------------

import { ouvrirPontDepuis } from "./pont.js";
import { creerPartie, jouerJusquAuBout } from "./partie.js";
import { fournisseurHumain, fournisseurAleatoire } from "./fournisseurs.js";
// JOUER À DEUX EN LIGNE — le seul fichier neuf. Il n'apporte qu'un fournisseur
// de décisions de plus, au sens d'`adversaire.md` : hors ligne, rien de ce
// module ne s'exécute et la partie sur le même écran est exactement ce qu'elle
// était.
import {
  ouvrirRendezVous, brancherEnLigne, finDeLaPartieEnLigne,
} from "./distant.js";

import { chargerMateriel } from "./vue/materiel.js";
import { construireMonde, majMonde, oublier } from "./vue/monde.js";
import { reglerBoites } from "./vue/boites.js";
import { construireJoueurs, majJoueurs, replacerBarres } from "./vue/joueurs.js";
import {
  construireMains, majMains, adversaireAgit, replacerMains, oublierMains,
  venteAdverse,
} from "./vue/mains.js";
import { majPhases, enPlanification, oublierPhases } from "./vue/phases.js";
import { construireTable, majTable, oublierTable } from "./vue/table.js";
import { reglerAnimations, pause, rattrapageEnCours } from "./vue/anim.js";
import {
  construirePlateaux, majPlateaux, replacerPlateaux, oublierPlateaux,
} from "./vue/plateau.js";
import {
  construireScene, poserDecision, viderScene, replacerScene, repondrePourLeSiege,
  venteImmediate,
} from "./vue/scene.js";
import {
  construireVente, majVente, venteAEcrire, oublierVente, apresMaReponse,
} from "./vue/vente.js";
import { construireLoupe } from "./vue/loupe.js";
import { oublierRefs } from "./vue/ecrire.js";
import { montrerAccueil, cacherAccueil } from "./vue/menu.js";
// (CNF-6) LA REPRISE D'UNE PARTIE INTERROMPUE. Ce module n'apporte rien au jeu :
// il garde trois valeurs dans le navigateur (graine, boîtes, liste des
// décisions), et sait dire s'il y a quelque chose à reprendre. Le moteur, le
// pont et `partie.js` n'en savent rien et n'ont pas bougé d'une ligne.
import {
  sauverPartie, oublierPartie, partieEnregistree, proposerReprise,
  replierEmpreinte,
} from "./vue/reprise.js";
import {
  installerOptions, montrerBoutonOptions, fermerOptions, viderTable,
} from "./vue/options.js";
import { construireAnnonce, annonceManche, annoncePhases, ecranFinal } from "./vue/annonce.js";
import * as son from "./vue/son.js";
import { MOT, actionAdverse } from "./vue/mots.js";
// (les-ecrans-manquants) LA LISTE DES QUESTIONS POSÉES AUX DEUX JOUEURS N'EST
// PLUS ÉCRITE : elle est mesurée sur le moteur au démarrage de la partie, et les
// deux lecteurs — l'écran et le mode en ligne — lisent le même ensemble.
import {
  mesurerQuestionsSimultanees, reglerQuestionsSimultanees, estSimultanee,
} from "./questions-simultanees.js";

// Le temps qu'un geste de l'adversaire reste sous les yeux. Sans lui, un
// programme répondrait entre deux images et l'on ne verrait JAMAIS qu'il joue :
// l'écran mentirait par vitesse. Le siège tenu par un programme respire un peu
// plus longtemps — c'est sa partie à lui qu'on regarde.
const PAS_ADVERSE = 180;
const PAS_PROGRAMME = 320;

// `PAS_PROGRAMME` est une DURÉE au sens de `table-vivante` : `?animations=non`
// la met à zéro comme le reste (`pause`, importé de `vue/anim.js`), sans changer
// d'un iota ce qui est décidé.
//
/**
 * `PAS_ADVERSE`, LUI, N'EN EST PAS UNE — c'est le temps que met l'adversaire à
 * répondre, la seule chose de ce fichier que `?animations=non` ne doit pas
 * escamoter : à zéro, l'adversaire disparaît de l'écran au lieu d'y agir.
 *
 * COUTURE : c'est le seul endroit où deux livraisons se contredisent vraiment.
 * La note complète est dans `adversaire()`, plus bas, avec l'arbitrage et ce
 * qu'il coûte.
 */
const attendreAdversaire = (ms) => new Promise((r) => setTimeout(r, ms));

// ------------------------------------------------------------------ l'adresse

/**
 * L'adresse porte la partie : `?graine=<entier>&boites=base` ou
 * `base,decouverte`. Quand elle est là, la partie démarre sans le moindre clic.
 */
function lireAdresse() {
  const p = new URLSearchParams(location.search);
  const g = p.get("graine");
  const b = p.get("boites");
  if (g === null && b === null) return null;
  const graine = Number.parseInt(g ?? "1", 10);
  return {
    graine: Number.isFinite(graine) ? graine : 1,
    boites: b === "base" || b === "base,decouverte" ? b : "base,decouverte",
  };
}

/**
 * LE CADRE : de quel siège je regarde, et qui répond pour lui. Ces deux
 * réglages sont lus une seule fois, ici, et tout le reste de la page s'y réfère
 * — l'écran ne suppose jamais que « le joueur 0, c'est moi ».
 */
function lireCadre() {
  const p = new URLSearchParams(location.search);
  return {
    siege: p.get("siege") === "1" ? 1 : 0,
    decide: p.get("decide") === "programme" ? "programme" : "humain",
    // `?animations=non` coupe les DURÉES, jamais les résultats : la carte part
    // du même endroit, arrive au même endroit, et la réponse rendue au moteur est
    // la même. Sans ce réglage, aucun contrôle automatique ne pourrait jouer une
    // partie entière — il mesurerait l'animation au lieu du jeu.
    animations: p.get("animations") !== "non",
    // (GRO-2) `?sons=non` éteint les cinq bruits, exactement comme
    // `?animations=non` éteint les durées — et pour la même raison : sans un
    // réglage lisible dans l'adresse, aucun contrôle automatique ne peut
    // vérifier que l'extinction éteint vraiment. Il ne change RIEN de ce qui
    // est décidé ni affiché.
    sons: p.get("sons") !== "non",
  };
}

const cadre = lireCadre();
// LE CADRE QUE L'ADRESSE DEMANDE, gardé tel quel. Une partie reprise impose le
// sien (c'est le sien qui distribue les tirages), mais si son enregistrement
// finit par être écarté, c'est celui-ci qui doit reprendre la main : on ne joue
// pas au siège d'un enregistrement qu'on vient de déclarer illisible.
const CADRE_DE_L_ADRESSE = { siege: cadre.siege, decide: cadre.decide };

// Le point de rendez-vous, quand l'adresse porte un code de partie ; `null`
// sinon — et alors absolument rien ne change.
let rendezVous = null;

// (les-ecrans-manquants) LA PARTIE EN COURS, vue depuis le module.
//
// `rendre` et `siegeHumain` sont des fonctions de module ; la partie, elle, vit
// dans `lancer`. Le bouton de vente a besoin d'une chose que l'ÉTAT ne porte pas
// — le numéro de l'occasion de vente en cours, qui est une clef du résultat de
// pas et non de la vue d'état (`wasm/src/lib.rs`, `occasions_ouvertes`). C'est
// la seule raison de cette référence, et elle ne sert qu'à LIRE.
let partieEnCours = null;

// ------------------------------------------------------------------ le décor

function batir() {
  construireMonde();
  construirePlateaux();
  construireJoueurs();
  construireMains();
  construireTable();
  construireScene();
  // (regles-de-la-vente) Le bouton de vente est posé APRÈS la scène : il vit
  // au-dessus d'elle, et une main comme un programme doivent pouvoir l'atteindre
  // à tout instant d'une phase où l'on peut dépenser.
  construireVente(venteImmediate);
  construireAnnonce();
  construireLoupe();
  // Le siège regardé est écrit sur le document : c'est lui qui décide quel
  // plateau se pose en haut et lequel se pose en bas.
  document.body.dataset.siege = String(cadre.siege);
  document.body.dataset.decide = cadre.decide;
  reglerAnimations(cadre.animations);
  // (GRO-2) Le son est réglé AVANT la première décision : le premier bruit de
  // la partie est celui du début de manche, et il ne doit pas sortir d'une page
  // ouverte avec `?sons=non`.
  son.reglerSons(cadre.sons);
  // TOUT ce qui est mesuré en pixels doit être remesuré quand la fenêtre change
  // de taille : les deux plateaux, les deux barres de joueur, et la grille des
  // choix de la décision en cours. On attend la fin du geste plutôt que de tout
  // refaire à chaque pixel de la poignée de redimensionnement.
  let minuteur = null;
  window.addEventListener("resize", () => {
    replacerPlateaux();
    replacerBarres();
    replacerMains();
    clearTimeout(minuteur);
    minuteur = setTimeout(replacerScene, 120);
  }, { passive: true });
}

function etatDuChargement(texte) {
  let e = document.getElementById("chargement");
  if (!e) {
    e = document.createElement("div");
    e.id = "chargement";
    document.body.appendChild(e);
  }
  e.textContent = texte;
  return e;
}

/**
 * (les-sept-bancs-rouges) **LE REPLI SE MONTRE.** Quand la mesure des questions
 * simultanées échoue, l'ensemble reste vide — et un ensemble vide ne veut pas
 * dire « prudent » : il veut dire AUCUN groupe de décisions déclaré au
 * rendez-vous, donc les questions de mise en place y repassent une par une, au
 * fil de l'eau. C'est la fuite que la mesure ferme. Elle se dit donc à l'écran,
 * et pas seulement à la console : un repli qu'on ne voit pas est un repli que
 * personne ne répare.
 */
function direSansMesure() {
  let e = document.getElementById("sans-mesure");
  if (!e) {
    e = document.createElement("p");
    e.id = "sans-mesure";
    e.setAttribute("role", "status");
    e.style.cssText = "position:fixed;left:0;right:0;bottom:0;z-index:9999;margin:0;"
      + "padding:.5rem 1rem;background:#7a2018;color:#fff;font:14px/1.4 system-ui";
    document.body.appendChild(e);
  }
  e.textContent = MOT.unmeasured;
  return e;
}

function panne(e) {
  // Un échec se montre, il ne se jette pas : une exception non rattrapée est une
  // erreur de console, et une erreur de console est un écran cassé.
  const z = document.createElement("div");
  z.id = "panne";
  z.textContent = MOT.broken + (e && e.message ? e.message : e);
  document.body.appendChild(z);
}

// ------------------------------------------------------------------ la partie

// Ce que l'écran a déjà annoncé, pour ne pas répéter la même manche deux fois.
let dejaVu = { manche: null, phases: null };

function theatre(etat) {
  if (etat.generation !== dejaVu.manche) {
    if (dejaVu.manche !== null) {
      annonceManche(etat.generation);
      son.sonManche();
    }
    dejaVu.manche = etat.generation;
    dejaVu.phases = null;
  }
  // LA RÉVÉLATION N'A LIEU QU'UNE FOIS LES DEUX CHOIX FAITS. `chosen_phase`
  // garde la valeur de la manche précédente pour qui n'a pas encore rechoisi :
  // annoncer sur « les deux sont non nuls » projetait en grand, une fois par
  // manche, la carte que l'adversaire venait de poser face cachée — et, vu du
  // siège interrogé en second, la lui montrait avant qu'il ait choisi la sienne.
  // C'est `vue/phases.js` qui sait où en est la planification, et lui seul.
  const paire = etat.players.map((p) => p.chosen_phase).join("-");
  if (!enPlanification() && paire !== dejaVu.phases &&
      etat.players.every((p) => p.chosen_phase)) {
    if (dejaVu.phases !== null) annoncePhases(etat);
    dejaVu.phases = paire;
  }
}

/**
 * Le rendu complet d'un instant, DEPUIS LE SIÈGE REGARDÉ : le monde, les deux
 * équipages, les deux plateaux, ma main en clair et celle de l'adversaire
 * retournée. La décision passée ici sert à dire QUI parle ; elle ne choisit
 * jamais ce que l'on montre — c'est le siège qui le décide.
 */
function rendre(etat, decision) {
  document.body.dataset.actif = decision ? String(decision.joueur) : "";
  // (regles-de-la-vente) **LA PAGE DÉCLARE LA FIN AU MÊME INSTANT QUE LE
  // MOTEUR.** Elle ne le déclarait nulle part : `data-phase="fin"` n'est posé
  // qu'après la boucle de jeu, une fois l'écran final dessiné, alors que le
  // moteur, lui, a rendu `game_over` au dernier coup. Entre les deux, l'écran
  // savait la partie finie et ne le disait pas — et c'est précisément l'instant
  // où les récompenses cessent d'être provisoires (`vue/joueurs.js`, qui lit le
  // même `etat.game_over`). Un seul fait, déclaré une fois, lu par les deux.
  if (etat.game_over) document.body.dataset.fin = "oui";
  majMonde(etat);
  majPlateaux(etat, decision, cadre.siege);
  majJoueurs(etat, decision, cadre.siege);
  majMains(etat, decision, cadre.siege);
  // `majPhases` d'abord : c'est lui qui sait où en est la planification et quelle
  // phase se résout, et la table lit ces deux réponses.
  majPhases(etat, decision);
  majTable(etat, decision, cadre.siege);
  // (regles-de-la-vente) EN DERNIER : le bouton de vente lit la phase que le
  // moteur résout (`etat.phase_en_cours`), la même source que la table
  // ci-dessus — les deux ne peuvent donc pas se contredire — et c'est ici que
  // le mode de vente se referme, une fois l'écran refait sur l'état d'APRÈS la
  // vente.
  // (les-ecrans-manquants) Le bouton reçoit AUSSI de quoi numéroter la vente
  // qu'il fabriquera : les occasions ouvertes à cet instant et le compte de
  // celles déjà ouvertes. Sans numéro, une vente retombe sur la première
  // occasion du siège — sur une main que le joueur n'avait pas encore.
  majVente(etat, cadre.siege, partieEnCours
    ? { ouvertes: partieEnCours.occasions, compte: partieEnCours.occasionsOuvertes }
    : null);
  theatre(etat);
}

/**
 * MON SIÈGE, TENU PAR UNE MAIN. La scène se pose et attend le clic.
 *
 * Les CINQ questions que le moteur pose aux DEUX joueurs — et c'est la mesure
 * qui les compte, pas ce commentaire (`questions-simultanees.js`) : le
 * remplacement des corporations, celui des cartes projet, le choix de la
 * corporation, le choix de la phase et les cartes gardées à la recherche — se
 * jouent en même temps à la table : dès qu'elle m'est posée, on voit l'adversaire y répondre lui aussi,
 * dans son coin, en petit et retourné. Le reste du temps il attend, et la zone
 * du haut se tait.
 */
function siegeHumain() {
  return fournisseurHumain(async (d, etat) => {
    rendre(etat, d);
    // (regles-de-la-vente) UNE VENTE VALIDÉE PENDANT QUE L'ADVERSAIRE JOUAIT.
    // Elle n'avait alors aucune question où se glisser ; elle prend sa place
    // ici, AVANT la réponse à celle-ci. Le moteur la consomme à son point
    // d'occasion et repose la même question sur l'état d'après — c'est le
    // rendu suivant qui allumera les cartes devenues payables.
    const vente = venteAEcrire();
    if (vente) return vente;
    adversaireAgit(estSimultanee(d.type) ? actionAdverse(d) : null);
    const reponse = await poserDecision(d, etat);
    // (K1, 04-08) MA RÉPONSE ROUVRE LE DROIT DE VENDRE, et elle seule : le
    // moteur n'ouvrira une nouvelle occasion qu'au point de décision suivant.
    // Ce n'est PAS le cas quand la réponse est une vente (retour anticipé
    // ci-dessus) : celle-là consomme l'occasion en cours.
    apresMaReponse();
    son.eveiller();
    son.sonChoix();
    adversaireAgit(null);
    return reponse;
  }, "humain à l'écran");
}

/**
 * MON SIÈGE, TENU PAR UN PROGRAMME — « je regarde mon intelligence artificielle
 * jouer à ma place ». Elle voit exactement ce que je verrais : la scène est
 * posée comme pour un humain, ma main reste en clair, et la réponse arrive par
 * le même chemin qu'un clic (`vue/scene.js`).
 */
function siegeProgramme(cerveau, arret) {
  return {
    nom: cerveau.nom,
    async decider(d, etat) {
      rendre(etat, d);
      adversaireAgit(estSimultanee(d.type) ? actionAdverse(d) : null);
      const attente = poserDecision(d, etat);
      const reponse = cerveau.decider(d, etat);
      await pause(PAS_PROGRAMME);
      // La partie a pu être abandonnée pendant cette pause. Répondre malgré
      // tout résoudrait la décision de la partie SUIVANTE — c'est exactement la
      // réponse fantôme que le retour au menu doit rendre impossible.
      if (arret.abandonne) return attente;
      repondrePourLeSiege(reponse);
      adversaireAgit(null);
      return attente;
    },
  };
}

/**
 * L'ADVERSAIRE — un programme qui décide arbitrairement, et qui reste OPAQUE.
 *
 * Sa décision ne redessine JAMAIS la scène : mon écran reste le mien, et lui
 * agit dans un coin, cartes retournées. On voit QU'il agit, jamais QUOI. C'est
 * précisément ce que l'ancien écran ne faisait pas : il donnait la parole — et
 * toute la surface — à celui qui décidait, quel qu'il soit.
 */
function adversaire(cerveau) {
  return {
    nom: cerveau.nom,
    decider(d, etat) {
      rendre(etat, d);

      // UNE QUESTION POSÉE AUX DEUX. Elle va m'être posée à l'instant d'après :
      // on le montre en train d'y répondre, et l'écran PREND LE TEMPS de le
      // montrer. C'est le seul moment où l'attente a un sens — et aucun de ces
      // trois moments n'est jamais le dernier de la partie.
      if (estSimultanee(d.type)) {
        adversaireAgit(actionAdverse(d));
        // COUTURE — LE SEUL ENDROIT OÙ DEUX LIVRAISONS SE CONTREDISENT.
        //
        // `table-vivante` a remplacé la pause locale par celle de `vue/anim.js`,
        // qui met TOUTE durée à zéro sous `?animations=non` (sa note en tête de
        // fichier : « les deux respirations ci-dessous sont des DURÉES »).
        // `menu-et-options`, lui, a écrit son contrôle `20` contre l'écran de
        // départ, où cette respiration-ci durait vraiment : sa boucle de mise en
        // route avance décision par décision et ne sait sortir que sur un écran
        // momentanément sans question — celui, précisément, que l'adversaire
        // laisse pendant qu'il répond. À zéro, la boucle atteint la deuxième
        // décision de la partie, qui est un choix MULTIPLE, et un clic sur une
        // seule carte ne la résout pas : le contrôle `20` s'y bloque.
        //
        // L'arrangement retenu : `attendreAdversaire` ne passe PAS par `duree`.
        // Ce n'est pas une animation, c'est le temps qu'un adversaire met à
        // répondre — la seule chose que `?animations=non` ne doit pas escamoter,
        // sous peine de faire disparaître l'adversaire de l'écran. Tout le reste
        // de la règle de `table-vivante` est intact : les vols de cartes, la
        // pose des phases et la respiration du siège tenu par un programme
        // (`PAS_PROGRAMME`, ligne plus haut) restent zérotables.
        //
        // Ce que ça coûte, RECOMPTÉ le 28-08 : l'ensemble mesuré ne contient
        // CINQ types de décision, et non les 3 d'alors
        // (`questions-simultanees.js` en mesure cinq : corp_mulligan,
        // project_mulligan, pick_corporation, pick_phase et research_keep) ;
        // la respiration, elle, ne se prend que sur les décisions
        // du siège d'en face. Mesure du 28-08 sur une partie entière (graine
        // 424242, base + Découverte, 349 décisions) : 82 `pick_phase`,
        // 40 `research_keep`, 2 de chacune des trois autres, dont 64 au siège
        // d'en face — soit 64 x 180 ms, environ 11,5 s ajoutées à une partie
        // complète, et non les 2,5 s écrites ici quand on n'en comptait que
        // trois. Sans effet sur les contrôles `01`, `02` et `24`, qui jouent
        // des parties entières et qui sont verts.
        return attendreAdversaire(PAS_ADVERSE).then(() => cerveau.decider(d, etat));
      }

      // PARTOUT AILLEURS, IL RÉPOND TOUT DE SUITE, dans le même tour de boucle
      // que ma propre réponse. Attendre ici laisserait l'écran sans question ET
      // sans fin de partie pendant qu'il termine la partie : mesuré sur huit
      // graines, c'est le joueur 0 qui prend la dernière décision sept fois,
      // donc au siège 1 la partie s'achève presque toujours pendant son tour.
      // Un écran qui ne montre alors ni question ni fin est un écran bloqué —
      // pour une main comme pour une machine qui pilote la page. Ce qu'il fait
      // se voit à son plateau, qui change sous nos yeux.
      return cerveau.decider(d, etat);
    },
  };
}

// ------------------------------------------------- la vente de l'autre se voit

/**
 * Cette réponse est-elle une vente faite par ce siège-là ?
 *
 * (les-ecrans-manquants) Vendre n'est pas répondre à une question : le moteur ne
 * demande jamais « voulez-vous vendre ? ». Une vente voyage comme une réponse
 * ordinaire, `{ vendre: { joueur, occasion, cartes } }`, rendue par le
 * fournisseur du siège qui vend (`vue/vente.js`, `venteAEcrire`). C'est donc là
 * qu'on la reconnaît, et nulle part ailleurs.
 *
 * POURQUOI PAS LE COMPTEUR DU MOTEUR. `ventes_volontaires` est un compteur
 * GLOBAL : il dit qu'une vente a eu lieu, jamais QUI l'a faite. Deux ventes de
 * sièges différents s'y confondent.
 */
function venteDuSiege(reponse, siege) {
  const v = reponse && typeof reponse === "object" ? reponse.vendre : null;
  return !!v && v.joueur === siege
    && Array.isArray(v.cartes) && v.cartes.length > 0;
}

/**
 * **ENVELOPPE LE SIÈGE D'EN FACE POUR VOIR SES VENTES PASSER.**
 *
 * Un seul point, et il couvre les deux vendeurs possibles : l'humain distant,
 * qui rend sa vente comme réponse, et un programme vendeur — il n'en existe pas
 * aujourd'hui à l'écran, les deux sièges tenus par un programme étant branchés
 * sur le joueur au hasard, mais le jour où il en existera un, sa vente passera
 * par `vendre()` et sera annoncée sans qu'une ligne d'ici ne bouge.
 *
 * PENDANT UN RATTRAPAGE, ON SE TAIT. Au rechargement d'une partie en ligne, la
 * page rejoue tout l'historique : annoncer chaque vente passée comme neuve
 * ferait clignoter des nouvelles vieilles d'une demi-heure.
 */
function guetterSesVentes(f, sonSiege) {
  const suivi = {
    nom: f.nom,
    async decider(d, etat) {
      const reponse = await f.decider(d, etat);
      if (venteDuSiege(reponse, sonSiege) && !rattrapageEnCours()) venteAdverse();
      return reponse;
    },
  };
  // Un fournisseur qui SAISIT les occasions de vente en expose une (`partie.js`,
  // `offrirLesOccasions`). On la garde, et on l'écoute aussi.
  if (typeof f.vendre === "function") {
    suivi.vendre = async (occ, etat) => {
      const cartes = await f.vendre(occ, etat);
      if (Array.isArray(cartes) && cartes.length > 0 && !rattrapageEnCours()) {
        venteAdverse();
      }
      return cartes;
    };
  }
  return suivi;
}

// ------------------------------------------------------- interrompre la partie
//
// LE POINT DÉLICAT DU RETOUR AU MENU. La boucle de jeu (`jouerJusquAuBout`)
// attend la réponse du joueur : une promesse posée par `vue/scene.js`, qui ne se
// résout QUE sur un clic. Quitter en plein milieu, c'est donc la laisser en
// suspens — et une boucle restée vivante répondrait à la place du joueur dans la
// partie suivante.
//
// On ne peut pas résoudre cette promesse : il faudrait inventer une réponse, et
// le moteur la refuserait. On ne touche pas non plus à `scene.js`. On COURT donc
// les deux : chaque décision est une course entre la réponse du fournisseur et
// un coupe-circuit qui ne sait que rejeter. Au retour au menu, la boucle remonte
// par l'exception sans avoir rien répondu, et meurt pour de bon.
const ABANDON = Symbol("retour au menu");
let arretCourant = null;

function coupeCircuit() {
  const c = { abandonne: false, attente: null, couper: null };
  c.attente = new Promise((_, rejeter) => {
    c.couper = () => {
      c.abandonne = true;
      rejeter(ABANDON);
    };
  });
  // Une promesse rejetée que plus personne n'attend est une erreur de console :
  // le rattrapage muet est posé ici, une fois pour toutes.
  c.attente.catch(() => {});
  return c;
}

/** Le même fournisseur, mais qui rend la main dès que la partie est abandonnée. */
function sousCoupeCircuit(f, arret) {
  return {
    nom: f.nom,
    decider: (d, etat) => Promise.race([
      (async () => f.decider(d, etat))(),
      arret.attente,
    ]),
  };
}

/**
 * (CNF-6) **REJOUER UNE PARTIE ENREGISTRÉE — ou refuser de le faire.**
 *
 * Reprendre, c'est recréer la partie avec la même graine et redonner au moteur
 * la liste des décisions, une par une, exactement comme elles ont été prises. Le
 * moteur rejoue de toute façon la partie depuis la graine à chaque coup : au
 * bout de la liste, il est très précisément là où le joueur l'avait laissé.
 * Mesuré : 355 décisions rejouées en 355 ms, 150 en 87 ms.
 *
 * RIEN N'EST DESSINÉ PENDANT LE REJEU. On n'appelle ni `rendre` ni
 * `poserDecision` : ces évènements ont déjà eu lieu, les rejouer à l'écran
 * annoncerait comme neuf ce qui est vieux (c'est la leçon du « rattrapage » de
 * `vue/anim.js`, apprise en partie à deux). Le premier dessin est celui de la
 * décision qu'on attendait.
 *
 * DEUX FAÇONS DE REFUSER, et c'est le cœur de ce point :
 *
 *   1. **le moteur refuse une réponse.** L'enregistrement est tronqué, bricolé,
 *      ou vient d'un jeu qui ne compte plus les options pareil. `repondre` la
 *      retire de la liste et relève ; on rend `null`.
 *   2. **les questions traversées ne sont plus les mêmes.** Une liste d'indices
 *      peut rester parfaitement valide alors qu'elle ne désigne plus les mêmes
 *      choses : c'est très exactement ce qui arrive si une question du moteur
 *      bouge de place. Le moteur ne lèverait pas, et l'on reprendrait une partie
 *      fausse en croyant la reprendre. L'empreinte, repliée ici question après
 *      question (type, nombre d'options, forme, passage offert) et comparée à
 *      celle qui a été enregistrée, le dit — et elle le dit pour TOUTE la
 *      partie rejouée, pas seulement pour son dernier pas.
 *
 * Dans les deux cas la page ne plante pas : elle écarte l'enregistrement et
 * commence une partie neuve.
 *
 * **ET LE HASARD DES PROGRAMMES AVANCE AVEC ELLE.** C'est le piège de tout ce
 * point, et il ne se voit pas au moment de la reprise : il se voit à la FIN.
 * L'adversaire est un tirage reproductible (`fournisseurAleatoire`,
 * `graine*2+2`), mais c'est un FLUX — sa n-ième réponse dépend du nombre de fois
 * qu'on l'a déjà consulté. Rejouer la liste sans le consulter le remettrait à
 * son premier tirage : l'écran serait juste à l'instant de la reprise (même
 * rang, même planète, même main, tout ce qui vient du moteur), et la partie
 * divergerait ensuite, coup après coup, pour finir sur un autre score. On
 * consulte donc chaque cerveau exactement une fois par décision qui lui
 * revenait, comme la partie l'avait fait, et l'on jette sa réponse : c'est la
 * liste enregistrée qui fait foi, jamais lui. Reprendre au bon rang mais
 * diverger ensuite, ce serait rejouer « à peu près ».
 *
 * @param {Array} cerveaux  un tirage par siège (`null` pour un siège humain)
 * @returns {?number} l'empreinte repliée si la reprise est fidèle, `null` sinon
 */
function rejouerLesDecisions(partie, enregistree, cerveaux) {
  let empreinte = 0;
  try {
    for (const reponse of enregistree.decisions) {
      if (partie.termine) {
        console.warn("terra : l'enregistrement dit plus de décisions que la "
          + "partie n'en a — écarté");
        return null;
      }
      const d = partie.decision;
      empreinte = replierEmpreinte(empreinte, d);
      const cerveau = d ? cerveaux[d.joueur] : null;
      if (cerveau) cerveau.decider(d, partie.etat);
      partie.repondre(reponse);
    }
  } catch (e) {
    console.warn("terra : le moteur refuse l'enregistrement —", e && e.message);
    return null;
  }
  if (partie.termine || !partie.decision) {
    console.warn("terra : l'enregistrement mène à une partie déjà finie — écarté");
    return null;
  }
  // LA QUESTION D'ARRIVÉE COMPTE POUR LA COMPARAISON, PAS POUR LA SUITE.
  //
  // C'est celle que le joueur avait sous les yeux quand tout s'est arrêté : elle
  // doit être vérifiée. Mais elle ne doit PAS rester dans l'empreinte qu'on
  // rend — sinon la boucle de jeu, qui replie elle aussi chaque question avant
  // de sauver, la replierait une seconde fois. L'enregistrement écrit juste
  // après une reprise porterait alors F(d0…dk, dk) là où une partie jamais
  // coupée porte F(d0…dk), et le décalage resterait pour toujours : **la partie
  // ne se reprendrait qu'une seule fois**, la seconde reprise s'accusant
  // elle-même de ne plus vouloir dire la même chose. Défaut trouvé par la
  // relecture adversariale, mesuré (empreintes 3534500047 puis 1818884364 sur
  // la même partie, graine 4242) ; c'était l'angle mort commun des deux bancs,
  // qui ne reprennent qu'une fois.
  if (replierEmpreinte(empreinte, partie.decision) !== enregistree.empreinte) {
    console.warn("terra : l'empreinte de l'enregistrement ne correspond plus — "
      + "les décisions ne veulent plus dire la même chose, il est écarté");
    return null;
  }
  return empreinte;
}

/**
 * @param {object} reglage  graine et boîtes de la partie
 * @param {object} [reprise]  un enregistrement à rejouer d'abord (CNF-6)
 */
async function lancer({ graine, boites }, reprise = null) {
  // (les-ecrans-manquants) **QUELLE BOÎTE EST SUR LA TABLE**, dit à l'écran.
  //
  // Les Objectifs et les Récompenses viennent de l'extension « Découverte » ;
  // sans elle ils ne rapportent rien. Le moteur les tient pourtant en mémoire
  // dans les deux boîtes, et la page les recopiait donc telles quelles : un
  // joueur de la boîte de base voyait un Objectif s'allumer comme PRIS et deux
  // cases de points qui ne comptent pour personne.
  //
  // C'est le SEUL endroit de la page qui connaisse la composition — elle vient
  // de l'adresse ou du rendez-vous, jamais d'une déduction — et c'est ici, et
  // non dans `batir`, parce que le décor est bâti AVANT que le rendez-vous n'ait
  // rendu la sienne.
  reglerBoites(boites);
  document.body.dataset.phase = "chargement";
  etatDuChargement(MOT.waking);

  const pont = await ouvrirPontDepuis(".");

  // (les-ecrans-manquants) **QUELLES QUESTIONS LE MOTEUR POSE-T-IL AUX DEUX
  // JOUEURS EN MÊME TEMPS ?** On ne l'écrit pas, on le MESURE — six parties
  // entières jouées hors écran par le moteur du navigateur, et l'on retient les
  // types dont chaque occurrence est appariée au siège opposé
  // (`questions-simultanees.js`). Deux endroits de la page en dépendent, et ils
  // en dépendaient chacun de son côté avec une liste différente : l'annonce
  // « l'adversaire répond en même temps » et, en ligne, les groupes de décisions
  // face cachée déclarés au rendez-vous.
  //
  // C'EST FAIT AVANT LA PREMIÈRE QUESTION, et il le faut : le tout premier rang
  // de la partie est déjà l'une de ces questions (le remplacement des
  // corporations). Mesuré : 3,0 s avec l'extension, 2,9 s en boîte de base, une
  // seule fois par page et par composition de boîtes.
  //
  // UN ÉCHEC N'ARRÊTE PAS LA PARTIE, MAIS IL NE PASSE PLUS EN SILENCE.
  //
  // (les-sept-bancs-rouges) Le commentaire d'ici jurait qu'un ensemble vide ne
  // laissait rien échapper. C'est l'inverse : un ensemble vide veut dire AUCUN
  // groupe de décisions déclaré au rendez-vous, donc les questions de mise en
  // place y repassent une par une, au fil de l'eau — précisément la brèche que
  // la mesure ferme. Ce qui est vrai, c'est que la page n'ANNONCE alors rien de
  // faux ; ce qui est faux, c'est de croire que rien ne s'échappe.
  //
  // Le repli le DIT donc, à la console et sur la page : mieux vaut un joueur
  // averti qu'une fuite silencieuse.
  etatDuChargement(MOT.measuring);
  try {
    reglerQuestionsSimultanees(await mesurerQuestionsSimultanees(pont, boites));
  } catch (e) {
    console.warn(
      "terra : la mesure des questions simultanées a échoué — aucun groupe de "
      + "décisions ne sera déclaré au rendez-vous, et les questions de mise en "
      + "place y repasseront une par une, au fil de l'eau : c'est une fuite, "
      + "pas une prudence.", e && e.message);
    reglerQuestionsSimultanees(new Set());
    // ET ÇA SE VOIT. L'attribut se lit d'un contrôle, le mot se lit d'un joueur.
    document.body.dataset.sansMesure = "oui";
    direSansMesure();
  }

  document.getElementById("chargement")?.remove();
  document.body.dataset.phase = "partie";
  oublier();
  oublierRefs();
  oublierPlateaux();
  oublierMains();
  oublierPhases();
  oublierTable();
  oublierVente();
  // Une partie neuve n'est pas finie : la déclaration de la précédente ne doit
  // pas lui survivre.
  delete document.body.dataset.fin;
  dejaVu = { manche: null, phases: null };

  let partie = creerPartie(pont, { graine, boites });
  partieEnCours = partie;

  // LES DEUX TIRAGES DE LA PARTIE, créés ICI et une seule fois. Ils l'étaient
  // dans `siegeProgramme` et `adversaire` ; ils en sortent parce qu'une partie
  // reprise doit pouvoir les faire avancer au même point avant de rendre la main
  // (voir `rejouerLesDecisions`). Les graines sont les mêmes qu'avant, au chiffre
  // près : rien de ce qui est tiré ne change.
  const cerveaux = [];
  cerveaux[cadre.siege] = cadre.decide === "programme"
    ? fournisseurAleatoire(graine * 2 + 1, "programme au siège")
    : null;
  cerveaux[1 - cadre.siege] = fournisseurAleatoire(graine * 2 + 2, "adversaire");

  // (CNF-6) LA PARTIE REPRISE. Si le rejeu échoue, l'enregistrement est écarté
  // — et la partie repart de zéro, sur la même graine, plutôt que de laisser le
  // joueur devant une page morte. Ni la liste à moitié rejouée ni les tirages à
  // moitié consommés ne survivent : on refait tout, proprement.
  //
  // L'EMPREINTE CONTINUE DE SE REPLIER APRÈS LA REPRISE : celle que le rejeu a
  // obtenue est exactement celle qu'une partie jamais coupée aurait à cet
  // instant, donc les enregistrements suivants restent comparables au même
  // nombre. Repartir de zéro ici rendrait la partie irreprenable une seconde fois.
  let empreinte = 0;
  if (reprise) {
    const repliee = rejouerLesDecisions(partie, reprise, cerveaux);
    if (repliee === null) {
      oublierPartie();
      // Le cadre de l'enregistrement est rendu avec lui : la partie neuve se
      // joue du siège que l'ADRESSE demandait, pas de celui d'une sauvegarde
      // qu'on vient d'écarter.
      cadre.siege = CADRE_DE_L_ADRESSE.siege;
      cadre.decide = CADRE_DE_L_ADRESSE.decide;
      document.body.dataset.siege = String(cadre.siege);
      document.body.dataset.decide = cadre.decide;
      partie = creerPartie(pont, { graine, boites });
      partieEnCours = partie;
      cerveaux[cadre.siege] = cadre.decide === "programme"
        ? fournisseurAleatoire(graine * 2 + 1, "programme au siège")
        : null;
      cerveaux[1 - cadre.siege] = fournisseurAleatoire(graine * 2 + 2, "adversaire");
    } else {
      empreinte = repliee;
    }
  }

  // Le bouton d'options n'apparaît qu'ici : sur l'accueil, il n'aurait rien à
  // ouvrir. Il est posé AVANT la première décision, pour être là dès la première.
  const arret = coupeCircuit();
  arretCourant = arret;
  montrerBoutonOptions(true);

  // Un fournisseur par siège, posé à sa place : le siège regardé reçoit celui
  // que `?decide=` désigne, l'autre reçoit toujours le programme adverse. Rien
  // d'autre dans la page ne dépend de « qui est le joueur 0 ».
  const fournisseurs = [];
  fournisseurs[cadre.siege] = cadre.decide === "programme"
    ? siegeProgramme(cerveaux[cadre.siege], arret)
    : siegeHumain();
  fournisseurs[1 - cadre.siege] = adversaire(cerveaux[1 - cadre.siege]);
  // LA SEULE LIGNE DE COMPOSITION QUI CHANGE. En ligne, le siège d'en face
  // n'est plus tenu par le programme mais par un humain devant un autre écran,
  // et mes propres réponses passent par le point de rendez-vous. `partie.js`
  // n'en sait rien : ce sont deux fournisseurs de décisions ordinaires.
  if (rendezVous) {
    brancherEnLigne(rendezVous, fournisseurs, cadre.siege, (d, etat) => rendre(etat, d));
  }
  // (les-ecrans-manquants) EN DERNIER, et par-dessus tout le reste : ce que le
  // siège d'en face finit par rendre. Posé APRÈS `brancherEnLigne` pour voir la
  // réponse telle qu'elle arrive du rendez-vous, et non celle qu'un fournisseur
  // local aurait fabriquée.
  fournisseurs[1 - cadre.siege] =
    guetterSesVentes(fournisseurs[1 - cadre.siege], 1 - cadre.siege);

  try {
    // (CNF-6) LA PARTIE S'ENREGISTRE AU FIL DE L'EAU. `jouerJusquAuBout` appelle
    // `avant` juste avant chaque décision : ce qui est écrit à cet instant, c'est
    // la liste de TOUT ce qui a déjà été répondu. Une fermeture brutale ne perd
    // donc au pire que la décision en cours — celle qui n'a pas encore reçu de
    // réponse. En ligne, ce crochet n'est pas posé du tout : la liste fait
    // autorité au relais, pas ici (voir `vue/reprise.js`).
    await jouerJusquAuBout(
      partie,
      fournisseurs.map((f) => sousCoupeCircuit(f, arret)),
      rendezVous ? undefined : (p) => {
        empreinte = replierEmpreinte(empreinte, p.decision);
        sauverPartie(p, cadre, empreinte);
      },
    );
  } catch (e) {
    // La partie a été abandonnée : `retourAuMenu` a déjà vidé la table et remis
    // l'accueil. Il n'y a ni score à montrer ni fin à annoncer.
    if (e === ABANDON) { partieEnCours = null; return; }
    throw e;
  } finally {
    if (arretCourant === arret) arretCourant = null;
  }

  // (CNF-6) LA PARTIE EST FINIE : ELLE NE SE PROPOSE PLUS. C'est le premier
  // geste d'après la boucle, avant même l'écran final — un enregistrement qui
  // survivrait à la fin proposerait de reprendre une partie qui n'a plus rien à
  // jouer.
  oublierPartie();

  viderScene();
  adversaireAgit(null);
  finDeLaPartieEnLigne(rendezVous);
  rendre(partie.etat, null);
  document.body.dataset.phase = "fin";
  ecranFinal(partie.etat);
  son.sonFin();
  // (les-sept-bancs-rouges) LA PARTIE EST FINIE : ON LÂCHE LA POIGNÉE. Sans
  // cela, une page qui enchaîne deux parties garde celle d'avant sous la main —
  // et c'est elle que la vente irait numéroter.
  partieEnCours = null;
}

// ------------------------------------------------------------- l'écran d'entrée

/**
 * L'accueil, et ce qu'on fait quand on le quitte. Le dessin vit dans
 * `vue/menu.js` ; ici ne restent que les deux gestes qui touchent la partie.
 */
function ecranEntree() {
  montrerAccueil((reglage) => {
    cacherAccueil();
    son.eveiller();
    lancer(reglage).catch(panne);
  });
}

/**
 * LE RETOUR AU MENU. Trois gestes, dans cet ordre : on coupe la boucle de jeu,
 * on vide la table, on remontre l'accueil. La partie précédente est alors morte
 * — pas cachée : plus une carte à l'écran, plus une décision en attente, et
 * aucune promesse qui pourrait répondre à la partie suivante.
 */
function retourAuMenu() {
  // EN LIGNE, IL N'Y A PAS DE RETOUR AU MENU. Une partie à distance ne
  // s'abandonne pas d'un côté seulement : l'autre joueur, lui, est toujours
  // devant son écran. Et repartir de l'accueil rebrancherait une partie NEUVE
  // sur le canal de l'ancienne — le moteur se verrait resservir les réponses
  // d'avant comme « déjà connues » et la page attendrait pour toujours une
  // question qui ne viendrait jamais. On recharge donc la page : elle rejoue la
  // liste et se retrouve exactement où elle en était, ce qui est le seul geste
  // utile ici. (Défaut trouvé par la relecture adversariale, éprouvé.)
  if (rendezVous) {
    location.reload();
    return;
  }
  const arret = arretCourant;
  arretCourant = null;
  arret?.couper();
  fermerOptions();
  montrerBoutonOptions(false);
  viderTable();
  document.body.dataset.phase = "accueil";
  ecranEntree();
}

// ---------------------------------------------------------------- le démarrage

async function demarrer() {
  // (CNF-6) LA PROPOSITION DE REPRISE PASSE AVANT TOUT LE RESTE, et elle ne
  // dépend de rien : ni du manifeste, ni du wasm, ni du réseau — seulement de ce
  // que le navigateur a gardé. Elle se pose donc tout de suite, PENDANT que le
  // matériel arrive, au lieu d'attendre derrière lui. Le joueur lit, décide, et
  // le chargement s'est fait sous la question.
  //
  // `partieEnregistree` ne croit rien de ce qu'elle lit et rend `null` au
  // moindre doute : un enregistrement abîmé ne pose donc aucune question et ne
  // retarde rien.
  const enregistree = partieEnregistree();
  // La promesse du matériel est attrapée TOUT DE SUITE. Sans ce `catch`, un
  // manifeste manquant deviendrait un rejet non rattrapé — c'est-à-dire une
  // erreur de console — pendant que le joueur lit la question.
  let echecMateriel = null;
  const materiel = chargerMateriel().catch((e) => { echecMateriel = e; });

  let reprise = null;
  if (enregistree) {
    if (await proposerReprise(enregistree)) reprise = enregistree;
    else oublierPartie(); // refusée : on ne la reproposera pas au chargement suivant
  }
  if (reprise) {
    // ON REPREND LA TABLE, PAS SEULEMENT LA PARTIE. Le siège regardé et celui
    // qui répond pour lui font partie de la partie qu'on reprend : c'est d'eux
    // que dépend quel siège reçoit quel tirage aléatoire. Les rejouer sous le
    // cadre de l'ADRESSE distribuerait le hasard à l'envers — écran juste au
    // bon rang, puis divergence jusqu'à un autre score final (mesuré : `[70,
    // 56]` du même siège, `[58, 61]` du siège d'en face, graine 4242). Le cadre
    // est donc adopté ICI, avant que `batir` ne s'en serve.
    cadre.siege = reprise.siege;
    cadre.decide = reprise.decide;
  }

  // Le manifeste : tout le décor est bâti à partir des images qu'il désigne, il
  // ne peut pas se construire avant d'être lu.
  await materiel;
  if (echecMateriel) {
    panne(echecMateriel);
    return;
  }
  batir();
  installerOptions({ auMenu: retourAuMenu });
  // EN LIGNE. Le rendez-vous s'ouvre AVANT la partie, parce que c'est lui qui
  // porte la graine : le lien envoyé à l'autre joueur n'a pas à la transporter,
  // et deux liens recopiés à un chiffre près ne peuvent pas donner deux parties
  // différentes. La partie démarre alors sans le moindre clic.
  try {
    rendezVous = await ouvrirRendezVous();
  } catch (e) {
    panne(e);
    return;
  }
  if (rendezVous) {
    await lancer({ graine: rendezVous.graine, boites: rendezVous.boites });
    return;
  }
  // (CNF-6) La partie reprise l'emporte sur l'adresse : c'est ce que le joueur
  // vient de demander, et sa graine est celle de la partie qu'il reprend.
  if (reprise) {
    await lancer({ graine: reprise.graine, boites: reprise.boites }, reprise);
    return;
  }
  const adresse = lireAdresse();
  if (adresse) await lancer(adresse);
  else ecranEntree();
}

demarrer().catch(panne);

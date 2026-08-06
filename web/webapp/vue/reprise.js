// REPRENDRE UNE PARTIE INTERROMPUE (CNF-6) — et n'enregistrer que le strict
// nécessaire pour cela.
//
// « Aujourd'hui, une partie coupée est perdue. » C'est arrivé une fois pour de
// vrai, et il a fallu recopier la liste des décisions à la main.
//
// CE QU'ON ENREGISTRE, ET POURQUOI SI PEU. Une partie est ENTIÈREMENT décrite
// par sa graine, ses boîtes et la liste ordonnée des décisions déjà prises —
// `partie.js` le dit en toutes lettres : « rejouables telles quelles ». Le
// moteur rejoue de toute façon la partie depuis la graine à chaque coup. Tout le
// reste — l'état de la planète, les deux mains, ce qui est posé sur la table, le
// score — se RECALCULE. L'enregistrer serait à la fois inutile et faux : inutile
// parce que le moteur le refait mieux, faux parce qu'un état recopié peut mentir
// alors qu'une liste de décisions rejouée ne le peut pas. Et les visuels des
// cartes sont sous droits d'auteur : il n'y en a pas un seul ici.
//
// Il y a UN champ de plus, `attendue`, et il n'est pas du contenu de partie :
// c'est le `type` de la question qui suit la liste (une chaîne comme
// `choose_build`). Il sert à répondre au cas que ce chantier promet de tenir —
// « un enregistrement venu d'une version du jeu où les décisions ne veulent plus
// dire la même chose ». Une liste d'indices reste souvent VALIDE après un
// déplacement de questions : elle désigne simplement autre chose. Le rejeu ne
// lèverait alors pas, et l'on reprendrait une partie fausse en croyant la
// reprendre. En vérifiant qu'on retombe sur la MÊME question, on l'écarte.
//
// CE QU'ON N'ENREGISTRE PAS DU TOUT : les parties à deux, chacun chez soi
// (`?partie=<code>`). Là, la liste des décisions fait autorité AU RELAIS, pas
// dans ce navigateur : `distant.js` la redemande au rendez-vous et la rejoue à
// chaque rechargement. Une reprise locale ferait diverger les deux écrans — ce
// que le contrat de ce chantier dit être pire que pas de reprise du tout. La
// reprise en ligne existe donc déjà, et elle ne passe pas par ici.

import { lireRendezVous } from "../distant.js";

/** La clé, et le numéro de forme de l'enregistrement. */
const CLE = "terra.partie-en-cours";
const FORME = 1;

/** Les seules valeurs de `boites` que le moteur connaisse (`interface.js`). */
const BOITES = new Set(["base", "base,decouverte"]);

/** Sommes-nous dans une partie à deux, chacun chez soi ? */
export function enLigne() {
  try {
    return !!lireRendezVous();
  } catch {
    return false;
  }
}

/**
 * LE NAVIGATEUR PEUT REFUSER D'ÉCRIRE, et ce n'est pas une panne du jeu.
 *
 * `localStorage` lève quand il est plein, et il n'existe pas du tout dans une
 * page ouverte sans origine (`file://` sous certains réglages), ni en navigation
 * privée sur de vieux navigateurs. Une partie qui ne s'enregistre pas reste une
 * partie jouable : tout ce module échoue en silence plutôt que de casser l'écran.
 */
function coffre() {
  try {
    return window.localStorage || null;
  } catch {
    return null;
  }
}

/**
 * Enregistre la partie en cours. Appelé AVANT CHAQUE DÉCISION, par la boucle de
 * jeu (`partie.jouerJusquAuBout`, argument `avant`) : au pire, une fermeture
 * brutale perd la décision en cours, jamais plus.
 *
 * @param {object} partie l'objet rendu par `creerPartie`
 */
export function sauverPartie(partie) {
  if (enLigne()) return;
  const c = coffre();
  if (!c) return;
  try {
    if (partie.termine) {
      oublierPartie();
      return;
    }
    const d = partie.decision;
    c.setItem(CLE, JSON.stringify({
      forme: FORME,
      graine: partie.graine,
      boites: partie.boites,
      decisions: partie.decisions,
      // L'empreinte, et rien d'autre : le nom de la question qui suit la liste.
      attendue: d && typeof d.type === "string" ? d.type : null,
      quand: Date.now(),
    }));
  } catch (e) {
    // Coffre plein, ou refusé : on le dit à la console de développement (jamais
    // en erreur — une page qui n'enregistre pas n'est pas une page cassée) et
    // l'on continue de jouer.
    console.warn("terra : la partie n'a pas pu être enregistrée —", e && e.message);
  }
}

/** Efface l'enregistrement. Une partie finie ne se propose plus. */
export function oublierPartie() {
  const c = coffre();
  if (!c) return;
  try {
    c.removeItem(CLE);
  } catch {
    /* rien à faire : au pire l'enregistrement survit, et le rejeu le jugera */
  }
}

/**
 * LA LECTURE, ET LE PLUS IMPORTANT DE TOUT CE POINT : **rien de ce qui est lu
 * ici n'est cru.**
 *
 * Ce qui est dans le navigateur a pu être écrit par une autre version du jeu,
 * tronqué par une fermeture au mauvais moment, ou remplacé par n'importe quoi.
 * On ne lit donc pas un objet : on VÉRIFIE une forme, champ par champ, et l'on
 * rend `null` au moindre doute. Un `null` n'est pas une panne — c'est « il n'y a
 * rien à reprendre », et la page démarre une partie neuve.
 *
 * Ce que cette fonction ne peut PAS vérifier : que les indices désignent encore
 * les mêmes options. Cela ne se sait qu'en rejouant, et c'est `interface.js` qui
 * le fait, avec l'empreinte `attendue`.
 *
 * @returns {{graine:number, boites:string, decisions:Array, attendue:?string}|null}
 */
export function partieEnregistree() {
  if (enLigne()) return null;
  const c = coffre();
  if (!c) return null;
  let brut;
  try {
    brut = c.getItem(CLE);
  } catch {
    return null;
  }
  if (!brut) return null;

  let o;
  try {
    o = JSON.parse(brut);
  } catch {
    // Ce n'est même pas du JSON : on l'écarte, et on l'efface pour ne pas y
    // revenir à chaque chargement.
    console.warn("terra : enregistrement illisible, écarté");
    oublierPartie();
    return null;
  }

  const faute = formeFautive(o);
  if (faute) {
    console.warn("terra : enregistrement écarté —", faute);
    oublierPartie();
    return null;
  }
  return {
    graine: o.graine,
    boites: o.boites,
    decisions: o.decisions,
    attendue: typeof o.attendue === "string" ? o.attendue : null,
  };
}

/**
 * Ce qui cloche dans l'objet lu, ou `null` s'il a la bonne forme.
 *
 * Une réponse au moteur est soit un entier (choix simple, montant), soit un
 * tableau d'entiers (choix multiple) — `fournisseurs.js` l'énonce. On ne juge
 * jamais si un indice est LÉGAL : c'est le moteur qui le dira, au rejeu. On
 * refuse seulement ce qui n'a pas la forme d'une réponse.
 */
function formeFautive(o) {
  if (!o || typeof o !== "object" || Array.isArray(o)) return "ce n'est pas un objet";
  if (o.forme !== FORME) return `forme ${JSON.stringify(o.forme)} inconnue`;
  if (!Number.isInteger(o.graine)) return "graine absente ou non entière";
  if (typeof o.boites !== "string" || !BOITES.has(o.boites)) return "boîtes inconnues";
  if (!Array.isArray(o.decisions)) return "liste de décisions absente";
  if (o.decisions.length === 0) return "liste de décisions vide";
  for (const r of o.decisions) {
    if (Number.isInteger(r)) continue;
    if (Array.isArray(r) && r.every((x) => Number.isInteger(x))) continue;
    return "une réponse n'a pas la forme d'une réponse";
  }
  return null;
}

// ------------------------------------------------------------ la proposition

let ecran = null;

/**
 * PROPOSER, JAMAIS IMPOSER. Deux boutons, du même poids : reprendre, ou
 * commencer une partie neuve. Les deux sont déclarés (`data-reprendre`,
 * `data-nouvelle-partie`) comme tout ce qui se clique dans cette page, pour
 * qu'une machine puisse les atteindre exactement comme une main.
 *
 * @param {{decisions:Array}} enregistree
 * @returns {Promise<boolean>} vrai si le joueur veut reprendre
 */
export function proposerReprise(enregistree) {
  return new Promise((repondre) => {
    ecran = document.createElement("section");
    ecran.id = "reprise";

    const boite = document.createElement("div");
    boite.className = "reprise__boite";

    const titre = document.createElement("h1");
    titre.className = "reprise__titre";
    titre.textContent = "Unfinished game";
    boite.appendChild(titre);

    const dit = document.createElement("p");
    dit.className = "reprise__dit";
    // On ne promet que ce qu'on sait : le nombre de décisions déjà prises. Pas
    // de « manche 4 », pas de score — rien de tout cela n'est enregistré, et
    // l'inventer serait mentir.
    dit.textContent =
      `A game was interrupted after ${enregistree.decisions.length} decisions. `
      + "Resume it exactly where it stopped, or start a new one.";
    boite.appendChild(dit);

    const boutons = document.createElement("div");
    boutons.className = "reprise__boutons";

    const oui = document.createElement("button");
    oui.type = "button";
    oui.className = "reprise__bouton reprise__bouton--oui";
    oui.dataset.reprendre = "";
    oui.textContent = "Resume";
    oui.addEventListener("click", () => {
      fermer();
      repondre(true);
    });
    boutons.appendChild(oui);

    const non = document.createElement("button");
    non.type = "button";
    non.className = "reprise__bouton";
    non.dataset.nouvellePartie = "";
    non.textContent = "New game";
    non.addEventListener("click", () => {
      fermer();
      repondre(false);
    });
    boutons.appendChild(non);

    boite.appendChild(boutons);
    ecran.appendChild(boite);
    document.body.appendChild(ecran);
    document.body.dataset.ecran = "reprise";
    oui.focus({ preventScroll: true });
  });
}

function fermer() {
  if (ecran) ecran.remove();
  ecran = null;
  if (document.body.dataset.ecran === "reprise") delete document.body.dataset.ecran;
}

// (jouer-contre-l-apprenti) L'ADVERSAIRE ENTRAÎNÉ, TEL QUE LA PAGE L'INSTALLE.
//
// `?adversaire=apprenti` met en face le joueur entraîné (`joueurs/apprenti.js`)
// au lieu du tireur au hasard. Sans ce réglage, rien de ce module ne s'exécute et
// la page est exactement ce qu'elle était.
//
// Trois choses ici, et rien d'autre :
//
//   1. **les réglages du réseau, demandés au serveur** (`poids/apprenti.txt`, lien
//      symbolique vers `data/poids/apprenti.txt`), par le même chemin que le
//      moteur compilé. S'ils n'arrivent pas, ou s'ils ne passent pas le verrou des
//      noms, la page REFUSE DE COMMENCER et dit pourquoi : un « apprenti » qui
//      jouerait au hasard sans le dire serait un mensonge à l'écran ;
//   2. **le cerveau**, construit sur ces poids déjà en mémoire — ses réponses
//      sont donc SYNCHRONES, ce dont la reprise a besoin (`rejouerLesDecisions`
//      consulte chaque cerveau sans l'attendre). Chaque consultation est
//      chronométrée dans le navigateur par l'API de mesure standard
//      (`performance.measure`) : un banc la relit, DevTools l'affiche, et rien de
//      ce qui est décidé n'en dépend ;
//   3. **le rejeu de ses ventes** à la reprise d'une partie interrompue. Ce
//      joueur-là VEND (116 ventes sur une partie mesurée, graine 880002), et une
//      vente n'est pas une réponse à une question : elle est saisie à une
//      occasion, par `offrirLesOccasions`. La partie reprise doit consulter le
//      cerveau aux mêmes occasions que la partie coupée — c'est la règle des
//      flux d'`interface.js` (« consulter chaque cerveau exactement autant de
//      fois, et jeter la réponse »), étendue aux ventes.

import { chargerPoids, fournisseurApprenti } from "./joueurs/apprenti.js";

/** Les deux adversaires que l'adresse sait demander. */
export const ADVERSAIRES = new Set(["hasard", "apprenti"]);

/** Les noms des mesures, relus par `verif/partie-entiere-au-navigateur`. */
export const MESURE_REFLEXION = "terra:reflexion-apprenti";
export const MESURE_VENTE = "terra:vente-apprenti";

/**
 * Les réglages du réseau, demandés au serveur. Une seule demande par page : une
 * seconde partie dans la même page ne retélécharge rien (le serveur, lui,
 * interdit toute réserve sur ce fichier — un rechargement redemande donc le
 * champion en service).
 *
 * @returns {Promise<object>} les poids, verrou des noms vérifié
 */
export async function chargerReglagesApprenti() {
  try {
    return await chargerPoids();
  } catch (e) {
    console.error("terra : les réglages de l'adversaire entraîné n'ont pas pu être chargés —", e);
    throw new Error(
      "the trained opponent cannot play: its settings (poids/apprenti.txt) could not be loaded — "
        + (e && e.message ? e.message : e),
    );
  }
}

/**
 * Le cerveau entraîné, sur des poids déjà chargés. Même forme que
 * `fournisseurAleatoire` (`decider`), plus `vendre` — que les enveloppes de la
 * page laissent passer quand le cerveau en a une.
 *
 * @param {object} poids  rendus par `chargerReglagesApprenti`
 * @param {number} graine
 * @param {string} nom
 * @param {object} pont   le pont de LA partie — le cerveau doit être construit
 *                        AVANT `creerPartie` (voir l'en-tête de `apprenti.js`)
 * @param {string} boites
 */
export function cerveauApprenti(poids, graine, nom, pont, boites) {
  const f = fournisseurApprenti(graine, nom, poids, pont, boites);
  return {
    nom: f.nom,
    decider: (d, etat) => chronometre(MESURE_REFLEXION, () => f.decider(d, etat)),
    vendre: (occ, etat) => chronometre(MESURE_VENTE, () => f.vendre(occ, etat)),
  };
}

function chronometre(nom, fn) {
  const debut = performance.now();
  const reponse = fn();
  performance.measure(nom, { start: debut, end: performance.now() });
  return reponse;
}

/**
 * Le fournisseur `f`, avec la méthode `vendre` du cerveau s'il en a une. Les
 * enveloppes de la page (`adversaire()`) ne portent que `decider` : sans ce
 * relais, le joueur entraîné ne serait jamais consulté aux occasions de vente,
 * et ne prendrait pas dans la page les décisions qu'il prend sous Node.
 */
export function avecSesVentes(f, cerveau) {
  if (cerveau && typeof cerveau.vendre === "function" && typeof f.vendre !== "function") {
    f.vendre = (occ, etat) => cerveau.vendre(occ, etat);
  }
  return f;
}

/** Une entrée de la liste est-elle une vente `{ vendre: { joueur, occasion, cartes } }` ? */
function venteDe(entree) {
  return entree && typeof entree === "object" && !Array.isArray(entree) && entree.vendre
    ? entree.vendre
    : null;
}

/**
 * **REJOUER LES OCCASIONS DE VENTE, comme `offrirLesOccasions` les a offertes.**
 *
 * Appelé par le rejeu d'une partie enregistrée avant chaque décision, au même
 * point que la boucle de jeu appelle `offrirLesOccasions`. Pour chaque occasion
 * ouverte, dans le même ordre : le cerveau du siège, s'il sait vendre, est
 * CONSULTÉ et sa réponse JETÉE — c'est la liste enregistrée qui fait foi. Si
 * l'entrée suivante est la vente saisie à cette occasion-là, elle est appliquée
 * (`partie.vendre`, le geste même de la boucle de jeu), et l'on repart du début
 * des occasions, comme elle.
 *
 * AU BOUT DE LA LISTE, ON S'ARRÊTE AVANT DE CONSULTER. Les occasions encore
 * ouvertes à cet instant, la boucle de jeu les offrira elle-même en reprenant :
 * les consulter ici les ferait consulter deux fois.
 *
 * Les ventes d'un siège sans `vendre` (l'humain, qui vend par un bouton et dont
 * la vente voyage comme une réponse) ne sont pas touchées ici : elles restent des
 * entrées ordinaires, rejouées comme telles.
 *
 * @param {object} partie
 * @param {Array} entrees  la liste enregistrée
 * @param {number} i       la première entrée pas encore rejouée
 * @param {Array} cerveaux un cerveau par siège (`null` pour un humain)
 * @returns {number} la première entrée pas encore rejouée, après les ventes
 */
export function rejouerLesOccasions(partie, entrees, i, cerveaux) {
  let garde = 0;
  let encore = true;
  while (encore) {
    encore = false;
    if (++garde > 5000) throw new Error("boucle d'occasions de vente anormalement longue");
    for (const occ of partie.occasions) {
      if (i >= entrees.length) return i;
      const cerveau = cerveaux[occ.joueur];
      if (!cerveau || typeof cerveau.vendre !== "function") continue;
      cerveau.vendre(occ, partie.etat);
      const v = venteDe(entrees[i]);
      if (v && v.joueur === occ.joueur && v.occasion === occ.numero) {
        partie.vendre(v);
        i += 1;
        encore = true;
        break;
      }
    }
  }
  return i;
}

// La partie pas-à-pas, côté hôte.
//
// AUCUNE règle du jeu ici non plus. Ce module ne fait que trois choses :
//   1. tenir la liste des décisions déjà prises,
//   2. la redonner au moteur (`pont.pas`) pour obtenir la décision suivante,
//   3. faire tourner la boucle « le moteur demande / un fournisseur répond ».
//
// Le moteur rejoue la partie depuis la graine à chaque coup. C'est le chemin
// recommandé par le contrat : une panique n'est pas rattrapable en
// WebAssembly, le moteur ne peut donc pas être suspendu au milieu d'une manche.

/**
 * Ouvre une partie pas-à-pas.
 * @param {object} pont     le pont wasm (`pont.js`)
 * @param {object} o
 * @param {number} o.graine graine de la partie
 * @param {string} o.boites « base » ou « base,decouverte »
 */
export function creerPartie(pont, { graine, boites }) {
  const decisions = [];
  let dernier = pont.pas(graine, boites, decisions);

  return {
    graine,
    boites,
    /** Les décisions déjà prises, dans l'ordre (rejouables telles quelles). */
    decisions,
    /** L'état vivant que le moteur avait sous les yeux (`observe::state_view`). */
    get etat() {
      return dernier.etat;
    },
    /** La décision attendue : joueur, question, options. `null` si fini. */
    get decision() {
      return dernier.decision;
    },
    get termine() {
      return dernier.termine === true;
    },
    /** Scores finaux, une fois la partie terminée. */
    get scores() {
      return dernier.scores;
    },
    /** Nombre de manches jouées, une fois la partie terminée. */
    get manches() {
      return dernier.manches;
    },
    /** La partie s'est-elle arrêtée d'elle-même (et non sur le plafond) ? */
    get partieComplete() {
      return dernier.partie_complete === true;
    },
    /**
     * **(le-pont-ne-triche-plus) LES OCCASIONS DE VENTE OUVERTES.**
     *
     * Vendre une carte n'est pas une réponse à une question : le moteur ne
     * demande jamais « voulez-vous vendre ? ». Il fait savoir, avant chacun de
     * ses points de décision, qu'ici une vente serait recevable. Le pont rend
     * donc la liste des occasions ouvertes que personne n'a encore saisies, avec
     * pour chacune son NUMÉRO — un rang dans la partie —, le siège concerné et
     * la main de ce siège à cet instant.
     *
     * Chaque élément : `{ numero, joueur, main: [...] }`.
     */
    get occasions() {
      return dernier.occasions_ouvertes || [];
    },
    /** Combien d'occasions de vente le moteur a ouvertes depuis le début. */
    get occasionsOuvertes() {
      return dernier.occasions || 0;
    },
    /**
     * Saisit une occasion de vente : l'entrée s'ajoute à la liste des décisions,
     * comme une réponse, mais elle porte le numéro de l'occasion à laquelle elle
     * a été décidée. Le moteur refuse de la consommer AVANT ce numéro : sans
     * cela, une vente décidée à l'occasion n s'appliquerait à une main que le
     * joueur n'avait pas encore, plus tôt dans la partie.
     *
     * @param {{joueur: number, occasion: number, cartes: number[]}} vente
     */
    vendre(vente) {
      // Pas de garde « la partie est terminée » ici, et c'est délibéré : une
      // occasion de vente peut rester OUVERTE alors que le moteur a fini de
      // jouer — c'est le cas de la dernière occasion de la dernière manche, où
      // le joueur Rust vend encore (et cela change son score). Saisir une
      // occasion n'est pas répondre à une question posée après la fin : c'est
      // insérer un geste à l'instant que son numéro désigne, en pleine partie.
      // Une entrée qui ne correspond à aucune occasion est refusée par le
      // moteur, et retirée ci-dessous.
      decisions.push({ vendre: vente });
      try {
        dernier = pont.pas(graine, boites, decisions);
      } catch (e) {
        decisions.pop();
        throw e;
      }
      return dernier;
    },
    /**
     * Répond à la décision en cours et avance jusqu'à la suivante.
     *
     * Une réponse que le moteur REFUSE ne reste pas dans la liste : elle est
     * retirée avant que l'erreur ne remonte. Sans cela, la partie serait
     * empoisonnée — chaque coup suivant rejouerait la mauvaise réponse et
     * échouerait à son tour. Le joueur humain n'y est pas exposé (la page
     * n'offre que les options énumérées), mais un cerveau artificiel ou un
     * joueur distant, si : c'est-à-dire le point d'entrée que ce module existe
     * pour tenir (`adversaire.md`).
     */
    repondre(reponse) {
      if (this.termine) throw new Error("la partie est terminée");
      decisions.push(reponse);
      try {
        dernier = pont.pas(graine, boites, decisions);
      } catch (e) {
        decisions.pop();
        throw e;
      }
      return dernier;
    },
  };
}

/**
 * Fait tourner la partie jusqu'au bout : tant que le moteur demande quelque
 * chose, le fournisseur du joueur concerné répond.
 *
 * C'est LE point d'entrée unique du livrable 3 : brancher un cerveau artificiel
 * ou un joueur distant, c'est fournir un autre objet dans `fournisseurs` — rien
 * d'autre ne bouge (voir `adversaire.md`).
 *
 * @param {object} partie
 * @param {Array}  fournisseurs  un fournisseur par joueur (indice = numéro)
 * @param {(partie: object) => void} [avant]  appelé avant chaque décision
 */
export async function jouerJusquAuBout(partie, fournisseurs, avant) {
  let garde = 0;
  for (;;) {
    if (++garde > 200000) throw new Error("boucle de décisions anormalement longue");
    // **(le-pont-ne-triche-plus) LES OCCASIONS DE VENTE VIENNENT AVANT LA
    // DÉCISION**, exactement comme dans le moteur (`flow::avant_decision` ouvre
    // l'occasion, puis observe, puis interroge). Un fournisseur qui n'expose pas
    // de méthode `vendre` — l'écran de jeu, un joueur distant — ne voit rien
    // changer : ses occasions passent, déclinées.
    //
    // Elles viennent aussi avant le test de fin, et non après : les DERNIÈRES
    // occasions de la partie restent ouvertes alors que le moteur a déjà fini
    // de jouer. Les sauter faisait perdre au navigateur la dernière vente de la
    // partie — et le joueur Rust, lui, la faisait.
    await offrirLesOccasions(partie, fournisseurs);
    if (partie.termine) break;
    const d = partie.decision;
    if (!d) throw new Error("le moteur n'a rendu ni décision ni fin de partie");
    if (avant) avant(partie);
    const f = fournisseurs[d.joueur];
    if (!f) throw new Error("aucun fournisseur pour le joueur " + d.joueur);
    partie.repondre(await f.decider(d, partie.etat));
  }
  return partie;
}

/**
 * Offre au fournisseur de chaque siège les occasions de vente ouvertes, dans
 * l'ordre où le moteur les a ouvertes.
 *
 * Une vente saisie change la suite de la partie : on redemande alors la liste au
 * moteur et l'on reprend. Les occasions déclinées ne reviennent pas — le pont ne
 * publie que celles qu'aucune entrée n'a saisies, et une entrée numérotée les
 * enjambe sans les consommer.
 */
export async function offrirLesOccasions(partie, fournisseurs) {
  let garde = 0;
  let encore = true;
  while (encore) {
    encore = false;
    if (++garde > 5000) throw new Error("boucle d'occasions de vente anormalement longue");
    for (const occ of partie.occasions) {
      const f = fournisseurs[occ.joueur];
      if (!f || typeof f.vendre !== "function") continue;
      const cartes = await f.vendre(occ, partie.etat);
      if (Array.isArray(cartes) && cartes.length > 0) {
        // L'ordre des clefs est celui de `serde_json` côté Rust (alphabétique) :
        // les deux journaux sont alors identiques CARACTÈRE POUR CARACTÈRE, ce que
        // `verif/juge-meme-option.mjs` compare sans rien normaliser.
        partie.vendre({ cartes, joueur: occ.joueur, occasion: occ.numero });
        encore = true;
        break;
      }
    }
  }
}

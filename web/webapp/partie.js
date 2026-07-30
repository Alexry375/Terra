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
  while (!partie.termine) {
    if (++garde > 200000) throw new Error("boucle de décisions anormalement longue");
    const d = partie.decision;
    if (!d) throw new Error("le moteur n'a rendu ni décision ni fin de partie");
    if (avant) avant(partie);
    const f = fournisseurs[d.joueur];
    if (!f) throw new Error("aucun fournisseur pour le joueur " + d.joueur);
    partie.repondre(await f.decider(d, partie.etat));
  }
  return partie;
}

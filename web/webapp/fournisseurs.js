// Les « fournisseurs de décisions » — le point d'entrée unique du livrable 3.
//
// Un fournisseur est un objet `{ nom, decider(decision, etat) -> réponse }`.
// `decision` est le descripteur rendu par le moteur ; `etat` est
// `observe::state_view`. La réponse est ce que le moteur attend, et rien de plus :
//
//   • choix simple  → l'indice d'une option (`decision.options`) ;
//                     si `decision.passer`, l'indice `options.length` = passer ;
//   • montant       → un entier de `decision.minimum` à `decision.maximum` ;
//   • choix multiple→ un tableau de `decision.a_choisir` indices distincts.
//
// Un fournisseur ne connaît AUCUNE règle : il ne fait que choisir parmi ce que
// le moteur a lui-même énuméré. C'est ce qui rend interchangeables l'humain à
// l'écran, un cerveau artificiel et un joueur distant (voir `adversaire.md`).

/** La forme de la réponse attendue, lue sur le descripteur du moteur. */
export function formeDeLaReponse(d) {
  if (d.multiple) return "multiple";
  if (d.montant) return "montant";
  return "simple";
}

/** Nombre de réponses simples possibles (options + « passer » s'il est offert). */
export function nombreDeChoix(d) {
  return (d.options ? d.options.length : 0) + (d.passer ? 1 : 0);
}

// --------------------------------------------------------------- aléatoire

/** Générateur reproductible (xorshift32) : mêmes graines, mêmes parties. */
export function alea(graine) {
  let x = (graine >>> 0) || 0x9e3779b9;
  return () => {
    x ^= x << 13; x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5; x >>>= 0;
    return x / 4294967296;
  };
}

/**
 * Fournisseur aléatoire : sert la preuve scriptée du check 03 et le banc de
 * tests. Il ne décide rien « selon les règles » — il tire au sort parmi les
 * options que le moteur vient d'énumérer.
 */
export function fournisseurAleatoire(graine, nom = "aléatoire") {
  const r = alea(graine);
  const entre = (min, max) => min + Math.floor(r() * (max - min + 1));
  return {
    nom,
    decider(d) {
      switch (formeDeLaReponse(d)) {
        case "montant":
          return entre(d.minimum ?? 0, d.maximum ?? 0);
        case "multiple": {
          const n = d.options.length;
          const indices = [...Array(n).keys()];
          for (let i = n - 1; i > 0; i--) {
            const j = entre(0, i);
            [indices[i], indices[j]] = [indices[j], indices[i]];
          }
          // `a_choisir` absent = nombre LIBRE (le mulligan projets, de 0 à 8) :
          // on tire alors aussi la quantité, sinon on remplacerait toujours tout.
          const combien = d.a_choisir ?? entre(0, n);
          return indices.slice(0, combien);
        }
        default:
          return entre(0, nombreDeChoix(d) - 1);
      }
    },
  };
}

// ------------------------------------------------------------------ humain

/**
 * Fournisseur « humain à l'écran ». Il ne sait pas dessiner : il délègue à
 * `demander(decision, etat)`, que l'interface fournit et qui rend une promesse
 * résolue quand le joueur a cliqué. Les deux joueurs du bac à sable partagent
 * le même écran, donc le même fournisseur.
 */
export function fournisseurHumain(demander, nom = "humain à l'écran") {
  return { nom, decider: (d, etat) => demander(d, etat) };
}

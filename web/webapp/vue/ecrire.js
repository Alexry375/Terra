// Écrire le moins possible.
//
// L'écran se réécrit à chaque décision — 633 fois dans les deux parties que le
// pilote enchaîne. Or réécrire un texte identique remplace quand même le nœud et
// force le navigateur à refaire sa mise en page. Ces deux aides ne touchent le
// document que lorsqu'une valeur a VRAIMENT changé, et retiennent les éléments
// une fois pour toutes plutôt que de balayer le document à chaque coup.

const memoire = new WeakMap(); // élément -> dernier texte posé
const refs = new Map(); // sélecteur -> élément

/** L'élément d'un sélecteur, retenu après la première recherche. */
export function ref(selecteur) {
  let e = refs.get(selecteur);
  if (e && e.isConnected) return e;
  e = document.querySelector(selecteur);
  if (e) refs.set(selecteur, e);
  return e;
}

/** Pose un texte, seulement s'il a changé. */
export function poser(element, valeur) {
  if (!element) return;
  const t = String(valeur);
  if (memoire.get(element) === t) return;
  memoire.set(element, t);
  element.textContent = t;
}

/** Pose la valeur d'un chemin de l'état sur l'élément qui le déclare. */
export function poserValeur(chemin, valeur) {
  poser(ref(`[data-valeur="${chemin}"]`), valeur);
}

/** Vide le cache de sélecteurs (nouvelle partie, DOM reconstruit). */
export function oublierRefs() {
  refs.clear();
}

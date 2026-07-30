# La place des futurs modes — brancher un adversaire

Ce document décrit **le seul point d'entrée** par lequel un cerveau artificiel
ou un joueur distant se branchera. Rien d'autre n'aura à bouger : ni le pont, ni
le moteur, ni la page.

Ce soir, seul l'humain-à-l'écran est branché — et il tient les **deux** joueurs.

## Le contrat, en une phrase

> Un **fournisseur de décisions** est un objet
> `{ nom, decider(decision, etat) -> réponse | Promise<réponse> }`.

`partie.js` en prend un par joueur :

```js
const partie = creerPartie(pont, { graine: 1, boites: "base,decouverte" });
await jouerJusquAuBout(partie, [fournisseurJ0, fournisseurJ1]);
```

`jouerJusquAuBout` boucle : le moteur dit qui doit décider et quoi, le
fournisseur de ce joueur répond, le moteur avance. Il n'y a pas d'autre chemin —
la page (`interface.js`) et la preuve scriptée
(`verif/partie-pas-a-pas.mjs`) empruntent tous les deux celui-là.

## Ce que le fournisseur reçoit

**`decision`** — ce que le moteur demande, rien de plus :

| clef | sens |
|---|---|
| `type` | le point de décision du moteur (`pick_phase`, `choose_build`, …) |
| `joueur` | 0 ou 1 |
| `rang` | numéro de la décision dans la partie |
| `question` | la question, en clair |
| `options` | les choix **que le moteur a énumérés** (chacun a un `libelle`) |
| `passer` | si présent, « ne rien faire » est permis |
| `montant` | si présent, la réponse est un entier `minimum..maximum` |
| `multiple` | si présent, la réponse est une liste de `a_choisir` indices |

**`etat`** — l'état complet de la partie **au moment exact de cette décision**,
tel que le moteur le rend (`engine::observe::state_view`) : paramètres
planétaires et leurs plafonds, ressources, productions, savoir-faire, badges,
corporation, phase choisie, cartes Phase améliorées, les **deux** mains, cartes
posées et leurs ressources, repères, récompenses, score courant.

## Ce que le fournisseur rend

- **choix simple** : l'indice d'une option. Si `decision.passer` est vrai,
  l'indice `options.length` signifie « passer ».
- **montant** : un entier entre `minimum` et `maximum`.
- **choix multiple** : un tableau de `a_choisir` indices distincts.

`fournisseurs.js` expose deux aides qui évitent de relire cette table :
`formeDeLaReponse(d)` (`"simple" | "montant" | "multiple"`) et
`nombreDeChoix(d)`.

Une réponse hors bornes n'est pas devinée : le moteur la refuse et le pont
remonte l'erreur. C'est voulu — un fournisseur qui triche doit être vu.

## Règle d'or

**Un fournisseur ne connaît aucune règle du jeu.** Il choisit parmi ce que le
moteur vient d'énumérer. Il n'a pas à savoir ce qu'une carte coûte, si un
prérequis est rempli, ni combien vaut un point : si une option est offerte, elle
est légale ; si elle n'est pas offerte, elle n'existe pas.

C'est cette règle qui rend les trois modes interchangeables.

## Brancher un cerveau artificiel

Il évalue `etat` et rend un indice. Il peut prendre son temps : `decider` peut
rendre une promesse.

```js
// cerveau.js
export function fournisseurCerveau(evaluer) {
  return {
    nom: "cerveau",
    async decider(d, etat) {
      const scores = d.options.map((o, i) => evaluer(etat, d, o, i));
      return scores.indexOf(Math.max(...scores));   // + montant/multiple
    },
  };
}
```

```js
await jouerJusquAuBout(partie, [humain, fournisseurCerveau(monEvaluation)]);
```

**Explorer sans jouer.** `partie.decisions` est la liste complète des réponses
déjà données ; `pont.pas(graine, boites, decisions)` rejoue la partie depuis la
graine avec n'importe quelle liste. Un cerveau peut donc essayer un coup dans le
vide — `pont.pas(graine, boites, [...partie.decisions, coupEssaye])` — sans
toucher à la partie en cours. Aucun état caché ne l'en empêche : **la partie EST
la graine plus la liste des décisions.**

## Brancher un joueur distant

Le fournisseur devient un aller-retour réseau. La partie ne change pas de
nature : elle reste « graine + liste de décisions », ce qui se transmet en
quelques octets et se rejoue à l'identique des deux côtés.

```js
// distant.js
export function fournisseurDistant(canal) {
  return {
    nom: "distant",
    decider(d, etat) {
      canal.envoyer({ decision: d, etat });
      return canal.attendreReponse();   // Promise<réponse>
    },
  };
}
```

Côté serveur, l'autorité reste le moteur : on rejoue
`pont.pas(graine, boites, decisions)` et l'on vérifie que la réponse reçue est
bien dans les bornes que le moteur annonce. Un client qui ment est rejeté par le
moteur, pas par une règle recopiée.

## Ce qui n'a PAS à bouger

- `pont.js`, `wasi-shim.js`, `terra.wasm`, `wasm/` — le moteur et son accès ;
- `partie.js` — la boucle est déjà générique (elle ne cite aucun mode) ;
- `interface.js` — il ne fait qu'appeler `fournisseurHumain`.

Le seul fichier à écrire est le nouveau fournisseur, et la seule ligne à changer
est celle qui compose le tableau `[fournisseurJ0, fournisseurJ1]`.

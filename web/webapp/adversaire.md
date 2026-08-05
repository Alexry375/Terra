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

## Vendre des cartes — une entrée, jamais une réponse

Vendre une carte projet de sa main est une décision que le moteur pose, et elle
ne ressemble à aucune autre : **le moteur ne la demande pas**. Il fait savoir,
avant chacun de ses points de décision, qu'ici une vente serait recevable
(`flow::occasion_de_vendre`), et c'est au fournisseur de la proposer s'il la
veut.

**Ce que le fournisseur reçoit** : rien dans `decision` — la question posée est
toujours une autre (« quelle carte poser ? », le plus souvent). C'est **l'état**
qui porte l'information, dans `etat.vente_offerte` : vrai exactement là où une
vente sera reçue. `etat.ventes_volontaires` compte celles déjà faites, tous
joueurs confondus. La main à vendre, elle, est `etat.players[monSiège].hand`, et
les indices de vente sont ceux de cette liste-là.

**Ce que le fournisseur rend** : au lieu d'un indice d'option, une **entrée de
vente**, qui prend sa place dans la liste des décisions —

```js
return { vendre: { joueur: monSiège, cartes: [0, 3] } };   // indices dans `hand`
```

Le moteur consomme cette entrée au point d'occasion qui précède la question,
puis **repose la même question** sur l'état d'après : les cartes payables sont
ré-énumérées avec l'argent d'après la vente. Répondre à la question vient donc
ensuite, à l'appel suivant.

**Le piège, et il est sérieux.** Une occasion ne se dépense qu'une fois : après
une vente, la même question revient et `etat.vente_offerte` vaut **encore
vrai** — le drapeau a été armé avant la vente. Une seconde vente rendue là est
refusée par le moteur (« aucune occasion de vendre n'est ouverte à ce point ») et
la partie s'arrête. Une nouvelle occasion ne s'ouvre qu'après une vraie réponse
de ce siège. Un fournisseur qui vend doit donc se souvenir qu'il vient de
vendre : l'état ne le lui dira pas.

## La règle des deux mains

L'état publie **les deux mains** : `etat.players[0].hand` et
`etat.players[1].hand` existent toutes les deux, et c'est assumé côté moteur
(« Mode bac à sable : les DEUX mains sont visibles », `engine/src/observe.rs`).
La page en a besoin — elle sert les deux joueurs sur le même écran — et un
simulateur aussi.

**Un joueur artificiel honnête ne lit pas celle d'en face.** Des deux mains
publiées, il ne lit que la sienne : ni la `hand` de l'autre siège, ni son
`main_payable`, ni sa phase tant que le moteur ne l'a pas révélée. Il ne regarde
pas non plus les cartes de son adversaire par un détour — un score courant
recalculé sur cette main en serait un. Rien dans le code ne l'en empêche : c'est
une règle de conduite, et
elle a deux raisons. La première est que ce qu'on mesure alors ne veut rien
dire — un joueur qui voit le jeu d'en face paraît brillant sans l'être. La
seconde est plus lourde : ce joueur-là ne serait **pas transposable** à une
partie contre un humain, où ces cartes ne seront pas connues. Il faudrait le
réécrire, et le chiffre de victoires obtenu ne dirait rien de ce qu'il vaudrait
une fois aveugle.

Ce qu'un fournisseur peut lire sans mentir : **son propre siège** en entier, ce
qui est **public** (paramètres de la planète, objectifs et récompenses, tailles
de pioche et de défausse, scores affichés, cartes **posées** de l'adversaire et
leurs ressources), et **la décision elle-même** (`question`, `options`, `passer`,
`montant`, `multiple`). C'est déjà beaucoup : les options énumérées portent le
prix, les points de victoire, les badges et la couleur de chaque carte.

Cette règle s'éprouve de l'extérieur, sans lire le code : on repose au
fournisseur exactement la même question dans deux états qui ne diffèrent que par
la main d'en face, et l'on compare ses réponses. C'est ce que fait
`inputs/checks/03-il-ne-regarde-pas-les-cartes-d-en-face.sh`.

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

## La balance — `verif/duel.mjs`

```
node web/webapp/verif/duel.mjs <joueurA> <joueurB> [graines] [boites]
```

Elle fait jouer deux fournisseurs **nommés** l'un contre l'autre et dit qui
gagne — et surtout **si l'écart veut dire quelque chose**. Chaque graine est
jouée deux fois, sièges échangés, pour ne pas confondre la valeur d'un joueur
avec l'avantage d'un siège ; chaque camp reçoit sa propre graine de hasard, sans
quoi deux joueurs aléatoires feraient le même tirage ; rien n'y consulte
l'horloge, donc deux exécutions impriment exactement les mêmes lignes. Elle
imprime le nombre de décisions jouées, et conclut par « écart significatif » ou
« dans le bruit » selon qu'on est à plus ou moins de deux écarts typiques de
l'équilibre (le calcul est écrit en clair dans le fichier).

Ce qu'elle prouve : que deux joueurs au hasard sont à l'équilibre — c'est le
verdict qu'elle rend sur `duel.mjs hasard hasard 100` — et donc qu'un écart
qu'elle déclare significatif est un vrai écart.

## Le joueur qui réfléchit — `joueurs/reflechi.js`

```
node web/webapp/verif/duel.mjs reflechi hasard 100
```

`fournisseurReflechi(graine, nom)` est le premier fournisseur du dépôt qui ne
tire pas au sort. Il ne connaît aucune règle : il note les options que le moteur
vient d'énumérer (prix, points de victoire, badges, couleur, production) et son
propre côté de la table, et prend la mieux notée. `decider` est une **fonction
pure** — pas de mémoire, pas de tirage, donc pas de partie apprise par cœur — et
il ne lit que son propre siège : une seule fonction du fichier touche
`etat.players`, et elle prend le siège qui décide.

Ses chiffres, sur les cent graines du contrat jouées aux deux sièges
(200 parties) : **189 victoires sur 200 contre le hasard** (94,5 %, écart
déclaré significatif) et **142 sur 200 contre le joueur témoin** — celui qui
choisit l'option au libellé le plus long et ne comprend rien, mais gagne déjà
73 % contre le hasard. C'est cette seconde mesure qui compte : battre le hasard
ne prouve rien.

Ce qu'il prouve : qu'on peut battre le hasard **nettement** sans explorer l'arbre
des possibles, et qu'on a désormais une référence à laquelle comparer le
prochain adversaire.

## Ce qui n'a PAS à bouger

- `pont.js`, `wasi-shim.js`, `terra.wasm`, `wasm/` — le moteur et son accès ;
- `partie.js` — la boucle est déjà générique (elle ne cite aucun mode) ;
- `interface.js` — il ne fait qu'appeler `fournisseurHumain`.

Le seul fichier à écrire est le nouveau fournisseur, et la seule ligne à changer
est celle qui compose le tableau `[fournisseurJ0, fournisseurJ1]`.

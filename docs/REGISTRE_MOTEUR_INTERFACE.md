# REGISTRE MOTEUR → INTERFACE

> **À quoi sert ce fichier.** Le moteur de règles et l'interface de jeu sont deux
> programmes séparés qui doivent raconter exactement la même partie. Chaque fois
> qu'un lot modifie le moteur, quelque chose peut devoir changer dans l'interface —
> parfois une ligne recopiée, parfois un écran entier. Ce registre tient la trace
> de **chaque** modification du moteur et de ce qu'elle impose côté écran, pour
> qu'aucune ne soit découverte trois semaines plus tard devant un affichage faux.
>
> **Règle** : aucun lot moteur n'est déclaré fini tant que sa ligne n'est pas
> remplie ici, colonne « interface » comprise, même quand la réponse est « rien ».
>
> Créé le 19-08-2026.

---

## 0. LES QUATRE FAÇONS DONT L'INTERFACE DÉPEND DU MOTEUR

1. **Le moteur compilé** (`web/webapp/terra.wasm`, reconstruit par
   `web/construire.sh`). Toute modification du moteur exige une reconstruction,
   sans exception. Un `terra.wasm` périmé fait jouer deux jeux différents en
   silence.
2. **Le code recopié à la main.** `web/webapp/joueurs/description.js` (361 lignes)
   reproduit `engine/src/description.rs` case par case. Si l'ordre ou le nombre
   des cases diverge, le réseau lit des nombres qui ne veulent plus rien dire, et
   **rien ne le signale** — le joueur devient simplement mauvais. Même chose pour
   `web/webapp/assets/cards.json`, copie à l'octet de `data/cards.json` (défaut
   D23 : aucun contrôle ne compare les deux aujourd'hui).
3. **Les indices d'options.** Une décision est transmise sous forme de **numéro
   dans une liste**. Ajouter, retirer ou déplacer une option, ou déplacer un point
   de décision, rend **injouables toutes les parties déjà enregistrées** et
   déplace ce que l'écran doit afficher.
4. **Le déroulement visible.** Une règle qui devient simultanée, une question
   nouvelle posée au joueur, un départage affiché en fin de partie : ce sont des
   écrans, pas des lignes.

---

## 1. LE REGISTRE

Colonnes : le changement moteur, ce que l'interface doit faire, l'ampleur, l'état.

### Lot L1 — Le secret et l'ordre  *(LIVRÉ le 19-08, commit `3d14d25`, audité `ok`)*

Le détail complet, écrit par l'agent du chantier, est dans
`workspaces/le-secret-et-l-ordre/outputs/interface.md`. Résumé ici.

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **D1** — la carte Phase choisie n'est plus publiée avant que les deux joueurs aient répondu. Champ nouveau `phase_revelee` (ce que la table voit) ; `previous_phase` reste, privé au joueur | `description.js` : **fait** — les six cases `previous_phase_*` lisent `phase_revelee`, des deux côtés, comme `description.rs`. **Reste** : le mode en ligne (`distant.js`, `relais/serveur.js`) transmet toujours au fil de l'eau et doit recueillir les **deux** réponses avant d'en révéler une seule ; écran d'attente à prévoir | **écran + réseau** | recopie **faite**, réseau à faire |
| **D1 bis** — la vue d'état (`observe.rs`) publie une clef de plus, `phase_revelee` | Tout lecteur de la vue d'état qui montre « la phase de l'adversaire » doit basculer sur cette clef. `vue/table.js:18-25` s'en protège aujourd'hui à la main ; ce garde-fou peut être remplacé par une simple lecture | petite | à faire |
| **D10** — Objectifs et Récompenses comptés seulement avec l'extension | L'écran de score ne doit plus afficher de ligne Objectifs/Récompenses en boîte de base. Les tuiles restent **tirées** : ce sont les points qui valent zéro | petite | à faire |
| **D11** — départage d'égalité du livret | Nouvel affichage de fin de partie : total chaleur + MC + plantes des deux joueurs, cartes en main converties à 3 MC. Le moteur expose `flow::tiebreak_total` et `flow::winner` — l'écran ne recalcule pas le barème | **écran neuf** | à faire |
| **D14** — mise en place simultanée | Les cartes rendues et la corporation installée par l'adversaire ne s'affichent qu'une fois les deux joueurs ayant répondu, aux **trois** étapes. Écran d'attente à prévoir. Attention : le premier interrogé n'est plus toujours le siège 0 | **écran** | à faire |
| **D15** — extension seule refusée | Le menu de choix des boîtes doit refuser la combinaison, avec le message du moteur (l'écran ne réécrit pas la règle) | petite | à faire |
| **D16** — phase IV à l'ordre du tour | Aucun changement visible, mais **les parties enregistrées divergent**. `terra.wasm` **reconstruit**, banc de concordance Rust/JavaScript vert (201 situations, 1472 cases) | recompilation + regénération | **wasm fait**, parties à regénérer |
| **Premier joueur tiré au sort** | Afficher qui commence (clef `first_player` déjà publiée). Conséquence plus lourde : la **place d'une réponse** dans la liste des décisions ne désigne plus le siège 0 mais le premier joueur de la manche | petite à l'écran, **structurante** pour le rejeu | à faire |

**Piège rencontré, à ne pas oublier pour les lots suivants** : `terra.wasm` était
resté périmé un moment, pendant que la recopie JavaScript lisait déjà
`phase_revelee`. Douze cases de la fiche seraient restées figées à une constante,
**en silence** — le garde-fou des noms ne compare que des noms, et ils
concordaient. Seul le banc de concordance `web/webapp/verif/juge-descriptions.mjs`
voit ce genre de divergence, et il n'est vert qu'après `bash web/construire.sh`.

### Lot L2 — Les règles de cartes et de phases

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **D2** — *Mining Guild* gagne 1 NT par acier | Afficher le gain de niveau de terraformation au bon moment (le journal de partie doit le nommer) | petite | à faire |
| **D5** — le badge joker se choisit au moment de jouer la carte | **Nouvelle question posée au joueur** : nouveau point de décision, donc nouvel écran et décalage des indices | **écran neuf** | à faire |
| **D6** — le bonus de phase III n'est plus attaché de force | Nouvelle question : à quelle action appliquer le bonus | **écran neuf** | à faire |
| **D7** — trois activations ramenées à deux | Aucun écran, mais les listes d'options changent | recompilation | à faire |
| **D8** — le basculement A→B est demandé | **Nouvelle question posée au joueur** | **écran neuf** | à faire |
| **D9** — pas de destruction de ressources pour un paramètre au maximum | L'option disparaît de la liste : décalage d'indices | recompilation | à faire |
| **D17** — Objectif « Terraformeur » attribué à l'instant où la condition est remplie | La tuile peut être prise au milieu d'une phase : l'animation doit suivre | petite | à faire |
| **D18** — phase I-B : remise correcte et couleur libre sans première carte | Les options affichées changent | recompilation | à faire |
| **D19, D20** — comptage des badges | Aucun effet observable aujourd'hui | rien | à faire |
| **D21** — deux cartes retirées du drapeau « dans la pioche » | `assets/cards.json` à regénérer | petite | à faire |

### Lot L3 — La description que voit l'IA

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **D3** — les corporations tenues apparaissent dans la fiche | `description.js` : recopier les nouvelles cases, **dans le même ordre** | moyenne, silencieuse si ratée | à faire |
| **2.8, 2.9, 2.10** — compteurs de main, écarts entre joueurs, ressources posées | idem : chaque case ajoutée doit l'être des deux côtés, au même rang | moyenne | à faire |
| **2.12** — 44 entrées mortes retirées | idem, plus la régénération de la table des cartes du JavaScript | moyenne | à faire |

> **Le risque numéro un du projet est ici.** Les poids sont appris en Rust et
> relus en JavaScript. Le fichier de poids porte le nom de chacune de ses entrées :
> le côté qui relit doit les regénérer et **refuser de jouer au premier écart**.
> Ce garde-fou existe (`reseau.rs:588-620`) ; il doit rester actif après chaque
> changement de description.

### Lot L4 — Le joueur

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **V1** — le joueur cesse de voir le hasard futur | `apprenti.js:349-354,482` espionne la graine vivante (`espion.origine`) : à retirer, et à remplacer par un tirage indépendant | moyenne | à faire |
| **2.11** — 256 combinaisons essayées à l'échange de départ | Aucun écran, mais l'IA met plus de temps à répondre : vérifier que l'attente reste tenable | petite | à faire |
| **2.15** — l'IA peut vendre une carte | La vente doit s'animer correctement quand c'est l'IA qui la déclenche | petite | à faire |

### Lot L5 — Vitesse et réglages

Aucune répercussion sur l'interface, **sauf** : si la couche cachée passe de 50 à
100 ou 200 neurones (chantier 2.16), le fichier de poids change de forme et le
lecteur JavaScript doit suivre.

---

## 2. LES CONTRÔLES QUI DOIVENT EXISTER À LA FIN

1. **Un contrôle qui compare `data/cards.json` et `web/webapp/assets/cards.json`**
   à l'octet, et qui échoue si elles divergent (défaut D23 : il n'existe pas
   aujourd'hui).
2. **Un contrôle qui compare la fiche de situation produite par le Rust et celle
   produite par le JavaScript**, case par case, sur au moins 200 parties tirées
   au sort — et non sur les graines 1, 2 et 3 de la mise au point (défaut D25).
3. **Un contrôle qui reconstruit `terra.wasm` et vérifie qu'il correspond au
   moteur du dépôt**, pour qu'un binaire périmé ne puisse pas survivre.
4. **Un contrôle de secret** : à information cachée différente, la fiche de
   l'adversaire doit être identique case pour case.

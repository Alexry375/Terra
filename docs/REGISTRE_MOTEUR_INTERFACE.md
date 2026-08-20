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

### Lot L1 — Le secret et l'ordre  *(LIVRÉ le 19-08, commit `46109dc`, audité `ok`)*

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

### Lot L2 — Les règles de cartes et de phases  *(LIVRÉ le 19-08, commit `c28b307`, audité `ok`)*

**Le détail complet, ligne par ligne, est dans
`workspaces/les-regles-des-cartes/outputs/interface.md`.** Ce qui suit est le
résumé, avec l'état réel après audit.

**Le binaire du navigateur a été reconstruit par la main le 19-08**
(`bash web/construire.sh`), et la concordance des fiches est verte après
reconstruction : 185 situations, 1 472 cases, aucune divergence. Le déroulement
de la boîte de **base** a changé — l'empreinte de référence passe de
`47030e306f1006cd` à `15cd9db748878cec` — donc un binaire périmé aurait fait
jouer deux jeux différents en silence.

**Aucune case de la fiche vue par l'IA n'a changé** : `engine/src/description.rs`
n'est pas touché par ce lot, donc `web/webapp/joueurs/description.js` n'a rien à
recopier. (Le lot L3, lui, la changera de fond en comble.)

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **D2** — *Mining Guild* gagne 1 NT par acier gagné (deux aciers, deux niveaux) | Afficher le gain au bon moment ; le journal de partie doit le nommer | petite | à faire |
| **D5** — le badge joker se choisit au moment de POSER la carte, borné aux badges qui la laissent payable | **Nouvelle question posée au joueur** : écran neuf, et la place des réponses suivantes se décale | **écran neuf** | à faire |
| **D6** — l'activation supplémentaire de la phase III est choisie, plus imposée | Nouvelle question : à quelle carte appliquer la répétition | **écran neuf** | à faire |
| **D7** — avec la carte Phase III améliorée B, les deux répétitions vont à deux cartes DISTINCTES | Aucun écran neuf, mais les listes d'options changent | recompilation | **wasm fait**, écran à revoir |
| **D8** — sur une phase déjà améliorée, les deux variantes sont toujours proposées (la liste passe de 9 à 10 candidates) | **Nouvelle question**, et une option de plus : « je garde celle que j'ai » | **écran neuf** | à faire |
| **D9** — une action dont le seul gain porte sur un paramètre au maximum n'est plus offerte | L'option disparaît de la liste : les indices se décalent | recompilation | **wasm fait** |
| **D17** — l'Objectif est pris à l'instant où la condition est remplie, et la fenêtre « même phase » du livret reste ouverte le temps de la phase | La tuile peut être prise au milieu d'une phase : l'animation doit suivre, et l'adversaire qui franchit le seuil dans la même phase reçoit 3 PV à montrer | petite | à faire |
| **D18** — la seconde carte verte n'est accordée qu'après une première ; sans première, c'est la pose ordinaire qui est versée | Les options affichées changent | recompilation | **wasm fait** |
| **D19, D20** — les effets déclenchés se résolvent une fois par badge, et les réductions comptent les badges | Aucun effet observable aujourd'hui : aucune carte de la pioche ne porte deux fois le badge concerné | rien | sans objet |
| **D21** — deux cartes fantômes quittent le drapeau « dans la pioche » | `web/webapp/assets/cards.json` a été resynchronisé à l'octet dans le même geste | petite | **fait** |
| **Régression corrigée en chemin** — `flow::reveal_top` publie de nouveau les drapeaux de vente quand rien n'est prenable | L'écran cesse de recevoir les drapeaux du point de décision précédent : il n'offrira plus le bouton « vendre » là où l'occasion n'existe pas | petite | **fait côté moteur**, à vérifier à l'écran |
| **Six compteurs neufs dans la sortie de `simulate`** (`joker_badges_reposes`, `activations_bonus_libres`, `cartes_activees_trois_fois`, `ameliorations_imposees_sans_choix`, `branches_impossibles_offertes`, `secondes_poses_sans_premiere`, plus `branches_a_parametre_prises` et `premieres_poses_substituees`) | Rien à l'écran. Aucune clef existante n'a changé de nom ni de sens | rien | sans objet |
| **Les parties enregistrées d'avant ce lot sont périmées** | D5, D6 et D8 ajoutent des questions : toute partie de référence antérieure au 19-08 est injouable au rejeu, et les empreintes correspondantes sont fausses | regénération | à faire |

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

### Lot L4 — Le joueur  *(LIVRÉ le 19-08, commit `e5050b9`, audité `ok`)*

Détail complet : `workspaces/le-joueur-sans-voyance/outputs/interface.md`.

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **V1** — les essais de coups ne rejouent plus l'avenir réel : les trois tas cachés (paquet de projets, tuiles Océan face cachée, paquet de corporations) sont rebattus par une graine d'essais | **PRIORITÉ 1 DU LOT INTERFACE.** Le pont doit accepter une graine d'essais : `pont.pas(graine, boîtes, décisions, graineEssais)`. **Tant que ce n'est pas fait, l'intelligence artificielle du navigateur voit l'avenir** — elle lit le dessus de la pioche pendant qu'elle réfléchit, et aucun contrôle ne le signale : elle est simplement meilleure qu'elle ne devrait l'être. Le rebattage doit refaire à l'identique `joueur::rebattre_le_reste` (Fisher-Yates sur les trois tas, en épargnant ce qui est déjà sorti) et `joueur::ecarter_les_cartes_du_futur` | **pont + recompilation `terra.wasm`** | **à faire** |
| **2.11** — l'échange des cartes de départ essaie les 256 sous-ensembles | `apprenti.js` : **fait**, même ordre qu'en Rust. Reste à l'écran : 256 appels au pont par siège, l'attente du mulligan est le seul moment où elle se verra ; prévoir un voyant d'attente. Et l'animation doit tenir avec **huit cartes rendues d'un coup** (le joueur rend 4,16 cartes en moyenne contre 2,12) | recopie **faite**, attente à vérifier | **fait côté joueur** |
| **2.15 bis** — l'entrée de vente porte un champ nouveau : `{"vendre": {"joueur": j, "occasion": n, "cartes": [i]}}` | Le harnais du pont doit **compter les occasions de vente** et refuser de consommer une entrée avant son numéro. Sans cela, une vente décidée à une occasion s'applique à une occasion antérieure | **pont** | à faire |
| **2.15** — l'IA peut vendre une carte | L'écran doit **animer une vente déclenchée par l'IA** (la main perd une carte, le compte de MC monte de 3). Et le miroir `apprenti.js` **ne vend pas encore** : l'API du fournisseur doit exposer l'occasion de vente | **écran + API du fournisseur** | à faire |
| **Sortie de `jouer`** — neuf clefs nouvelles (`essais`, `essais_mulligan`, `ventes_volontaires`, `graine_essais`, …) | Rien à l'écran ; aucune clef existante n'a changé de nom ni de sens | rien | sans objet |
| **Les parties enregistrées d'avant le 19-08 sont périmées** | Une partie enregistrée est une graine plus une liste d'indices ; l'énumération du mulligan et les entrées de vente changent la liste. À regénérer avant tout banc de concordance | regénération | à faire |

**Le banc `web/webapp/verif/juge-meme-option.mjs` est rouge, et c'est attendu** :
le joueur Rust essaie ses coups sur un paquet rebattu, le joueur JavaScript sur
la vraie partie. Il redeviendra vert quand le pont acceptera une graine d'essais.
Ne pas le « réparer » autrement.

### Lot L3 — La fiche que l'intelligence artificielle regarde  *(LIVRÉ le 20-08, commit `2691b0b`, audité `ok`)*

Détail complet : `workspaces/la-fiche-que-l-ia-regarde/outputs/interface.md`.

> **L'avertissement qui commande tout le reste : tous les fichiers de poids
> d'avant ce lot sont devenus illisibles.** La fiche passe de **1 472 à 1 630
> cases** — 216 noms neufs, 58 disparus, et le premier écart de rang est au 79e.
> Le fichier de poids porte la table des noms (§ 3.7) et `reseau::Reseau::lire`
> refuse au premier nom qui ne correspond pas. Ce n'est pas une panne : c'est le
> verrou qui refuse au lieu de réinterpréter. Un fichier mal réinterprété
> donnerait un joueur mauvais sans que rien ne le signale.
>
> **Réparé à l'audit** : `data/poids/apprenti.txt`, le nom canonique que six
> outils chargent par défaut, porte désormais les poids d'amorçage du lot
> (30 000 parties, fiche neuve). Les six outils remarchent — dont
> `juge-main-cachee.mjs`, qui est le contrôle de secret du navigateur.

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **La fiche passe de 1 472 à 1 630 cases** (six familles neuves, une amaigrie) | `web/webapp/joueurs/description.js` : **fait**, recopié case par case, même ordre. Banc de concordance vert sur **390 situations**, 1 630 cases, 0 divergence | recopie | **faite** |
| **D3 — les corporations tenues en main entrent dans l'état** (`PlayerState::corps_en_main`), et la vue publie `corps_en_main` (les noms) par joueur | `description.js` : **fait** — 16 cases `corpo_…_ma_main`, **du seul côté du joueur qui regarde**. **Écran** : l'échange de corporations de la mise en place peut maintenant montrer les deux cartes tenues ; la donnée existe, l'écran ne s'en sert pas | petite à l'écran | recopie **faite**, écran à faire |
| **2.8 — la vue d'état publie `tags` et `vp` sur chaque carte** (main et cartes posées) | `description.js` : **fait** (79 cases de résumé de ma main). **Écran** : les badges d'une carte sont lisibles sans ouvrir `cards.json` ; `vue/*.js` peut cesser de croiser deux sources | petite | recopie **faite**, simplification possible |
| **2.10 — la vue d'état publie `valeurs_recompenses`** par joueur, calculé par `flow::award_value` | `description.js` : **fait** (21 cases de classement). **Écran** : l'affichage des Récompenses peut montrer qui mène sans recopier le barème | petite | recopie **faite**, écran à faire |
| **2.9 — 46 cases d'écart entre les deux joueurs, échelle de score de 8 à 25 paliers** (elle monte à 147 : s'arrêter à 83 laissait 15 situations indiscernables sur mille) | `description.js` : **fait**. Rien à l'écran | recopie | **faite** |
| **2.12 — la table des cartes suit la composition** : elle est bâtie sur `in_deck` et non plus sur l'appartenance à une boîte | `web/webapp/joueurs/paquet.js` : **fait**, réengendré, 257 → **246** cartes. **Piège** : la table n'est plus la même pour `base` seule et pour `base,decouverte`. La page joue `base,decouverte` ; si elle offre un jour la boîte de base seule, il lui faudra un `paquet.js` **et un fichier de poids** par composition | recopie + **piège de composition** | recopie **faite**, à surveiller |
| **`web/webapp/terra.wasm` reconstruit** | Rien à faire, mais **obligatoire** : un binaire périmé publierait une vue sans `tags`, sans `vp`, sans `corps_en_main` ni `valeurs_recompenses`, et la recopie lirait des champs absents — des dizaines de cases figées à −1, **en silence**, puisque les noms concorderaient | recompilation | **faite** |
| **Aucun point de décision n'a bougé** | Rien. Les parties enregistrées restent rejouables : les quatre empreintes d'état sont inchangées (`f781479fc2bce873`, `20fa65e8b81b3b39`, `7b5beb0c04da3776`, `42c1f72ad53c9264`), relevées par la main sur 1 200 parties | néant | néant |

**Réserve d'audit portée au lot L5** : **10,6 %** des paliers de la fiche sortent
de la bande 2 %–98 % du § 3.5, contre **5,4 %** avant le lot — mesuré par le même
programme sur 164 550 situations, avec pour chaque côté les poids de son époque.
Vingt-deux de ces paliers sont assumés et démontrés (fermer la case ouverte du
haut de l'échelle de score oblige à poser des paliers dans la queue de la
distribution) ; treize ne le sont pas. La cause est déclarée par l'agent : les
seuils ont été relevés sur l'intelligence artificielle de l'**ancienne** fiche,
faute d'en avoir une entraînée sur la neuve. **À re-poser dans le lot L5, juste
avant le dernier entraînement** — c'est le seul moment où les deux conditions
peuvent être vraies ensemble.

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

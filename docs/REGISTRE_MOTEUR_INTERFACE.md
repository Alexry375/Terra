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

### Lot L1 — Le secret et l'ordre

| Changement moteur | Ce que l'interface doit faire | Ampleur | État |
|---|---|---|---|
| **D1** — la carte Phase du siège 0 cesse d'être écrite dans l'état avant que le siège 1 ait répondu | `description.js` : retirer la publication de la phase adverse (recopie de `description.rs:389-403`, aujourd'hui en `description.js:263-268`). **Et surtout** : le mode en ligne (`distant.js`, `relais/serveur.js`) doit collecter les **deux** réponses avant d'en révéler une seule | **écran + réseau** | à faire |
| **D10** — Objectifs et Récompenses comptés seulement avec l'extension | L'écran de score ne doit plus afficher de ligne Objectifs/Récompenses en boîte de base | petite | à faire |
| **D11** — départage d'égalité du livret | Nouvel affichage de fin de partie : montrer le total chaleur + MC + plantes des deux joueurs, et les cartes en main converties à 3 MC | **écran neuf** | à faire |
| **D14** — mise en place simultanée | Les cartes rendues et la corporation installée par l'adversaire ne s'affichent qu'une fois les deux joueurs ayant répondu. Écran d'attente à prévoir | **écran** | à faire |
| **D15** — extension seule refusée | Le menu de choix des boîtes doit refuser la combinaison, avec un message | petite | à faire |
| **D16** — phase IV à l'ordre du tour | Aucun changement visible, mais **les parties enregistrées divergent** : à graine égale les cartes reçues ne sont plus les mêmes | recompilation + regénération | à faire |
| **Premier joueur tiré au sort** | Afficher qui commence, puisque ce n'est plus toujours le même siège | petite | à faire |

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

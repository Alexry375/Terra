# Retours d'Alexis sur la première interface — 31 juillet 2026

Ce document reformule les huit retours d'Alexis après avoir joué lui-même sur la
première version de l'interface (livraison `workspaces/interface-visuelle`), et y
ajoute ce que la lecture du code confirme ou infirme. Il sert de cahier des
charges au chantier suivant.

Convention de ce dépôt : **[VÉRIFIÉ 31-07]** = relu à la source, fichier et ligne
cités. **[DÉCLARÉ]** = affirmé, pas encore prouvé.

## Où en est chaque point, au 31-07 au soir

| # | Sujet | État |
|---|---|---|
| — | Tout le jeu en anglais | à faire (interface) |
| 1 | Mulligan projets de 0 à 8 cartes | **FAIT dans le moteur** |
| 2/6 | « branche 1, branche 2… » | moteur à enrichir, puis interface |
| 3 | Pas de loupe au choix des corporations | à faire (interface) |
| 4 | Onglet paramètres + rappels | à faire (interface) |
| 5 | Prix réduit affiché | à vérifier, puis interface |
| 7/11 | Plateau toujours visible, cartes empilées | à faire (interface) |
| 8 | Masquer les points de victoire | à faire (interface) |
| 9 | Pose de carte animée, 3 secondes | à faire (interface) |
| 10 | Glisser-déposer avec tangage | à faire (interface) |
| 12 | Visuels des cartes Phase améliorées | confirmé absent, à faire |
| 13 | Décor du plateau, ressources libres | direction proposée, à valider |
| 14 | Sauvegarde de partie | possible et bon marché, à faire |
| 15 | Carte des océans + animation de retournement | à faire (interface) |
| 16a | Choisir la carte vendue pour 3 MC | **FAIT dans le moteur** |
| 16b | Vendre « à tout moment » | écart connu, chantier moteur à part |
| 17 | Jauges style jeu, sur les côtés | à faire (interface) |
| 18 | Écran de lancement | à faire (interface) |

---

## Décision transverse — tout le jeu est en anglais

L'habillage de l'interface (titres, boutons, questions posées au joueur,
étiquettes des ressources) passe **en anglais**, comme les cartes. Cela comprend
**les instructions** adressées au joueur. Mon contrat de la première livraison
n'avait rien dit sur la langue : c'est mon omission, pas une faute de l'agent.

La documentation interne du dépôt, elle, reste en français.

---

## 1. Le remplacement des cartes projet de départ n'est pas « tout ou rien »

**Règle voulue.** À la mise en place, chaque joueur peut remplacer **entre 0 et 8**
de ses 8 cartes projet de départ, carte par carte. Le « tout ou rien » ne
s'applique qu'aux **corporations** (les 2 ou aucune).

**Ce que fait le code aujourd'hui — c'est le moteur qui est faux, pas l'interface.**
[VÉRIFIÉ 31-07]

- `engine/src/policy.rs:76-78` : la méthode `project_mulligan` rend un **oui/non**
  (`-> bool`). Elle ne peut structurellement pas exprimer « je remplace ces
  trois-là ».
- `engine/src/flow.rs:192-201` : si la réponse est « oui », **toute** la main part
  à la défausse et huit nouvelles cartes sont piochées.
- `engine/src/flow.rs:58` le dit noir sur blanc : « mulligan projets (les 8 ou
  aucune, en une fois) ».

**Conséquence à assumer.** Corriger cette règle modifie le moteur, donc modifie
l'ordre dans lequel les cartes sortent du paquet, donc **change les trois
empreintes de référence** qui servent aujourd'hui à prouver qu'on n'a rien cassé
(`cee020cda9db283b`, `981bb47e336034cc`, `c20dd5be100de393`). Il faudra les
recalculer et les réinscrire dans la carte d'état après la correction. C'est
normal et sans danger tant que la correction est faite avant, et la nouvelle
empreinte relevée après.

**Travail à faire.**
1. ~~Moteur : remplacer `project_mulligan(-> bool)` par un choix d'un
   **sous-ensemble** de la main~~ **FAIT le 31-07** : la méthode rend désormais
   `Vec<usize>`, les indices des cartes à remplacer, comme `discard_down`.
2. ~~Moteur : ne défausser que les cartes désignées et n'en repiocher
   qu'autant~~ **FAIT le 31-07** (`engine/src/flow.rs`, étape 4 de
   `setup_game`). Le moteur assainit ce que la politique lui rend : indices hors
   bornes ou répétés ignorés, jamais deux fois la même carte.
3. Interface : écran de mise en place où l'on **coche/décoche** chaque carte des
   huit, avec un compteur « 3 / 8 selected » et un bouton de validation. Le pont
   web pose déjà la question sous forme de choix multiple à nombre libre.
4. Corporations : inchangé, tout ou rien, deux boutons.

**Ce que ça a coûté, mesuré** [VÉRIFIÉ 31-07] : **813 tests verts**, aucune
régression. Trois tests qui figeaient l'empreinte ont été recalés, et deux tests
de la phase Recherche ont dû être rendus indépendants de la corporation tirée au
sort — ils mesuraient sans le dire l'apport de la corporation en plus de celui
des cartes, et ne tenaient que par chance. Nouvelles empreintes de référence :

| Partie de référence | Empreinte au 31-07 |
|---|---|
| `--seed 2024 --boites base` | `d6a7267472501b13` |
| `--seed 4242 --boites base` | `51e7966094e225cb` |
| `--seed 4242 --boites base,decouverte` | `2b5235e31f71c812` |

Mille parties dans chaque configuration, **zéro invariant cassé**, **zéro effet
de carte non traité**, mille parties achevées sur mille.

---

## 2. et 6. « Choisissez une branche du texte de la carte » — le joueur ne peut pas savoir de quoi on parle

Alexis a buté deux fois sur le même écran : une liste de boutons gris nommés
« branche 1 », « branche 2 »… sans rien d'autre.

**D'où vient ce libellé.** [VÉRIFIÉ 31-07]
`workspaces/interface-visuelle/outputs/webapp/wasm/src/lib.rs:1080-1086` fabrique
la question « Choisissez une branche du texte de la carte » et numérote les
options, avec ce commentaire honnête : « Le moteur ne donne QUE le nombre de
branches jouables ». C'est exact : `engine/src/policy.rs:163-170`, la méthode
`choose_option(rng, joueur, n)` ne reçoit **qu'un nombre**. Ni la carte
concernée, ni le texte des options, ni la nature du choix.

**Ce que ces boutons recouvrent en réalité — deux choses très différentes.**

- **Image 1 (2 branches)** : une vraie alternative « … ou … » imprimée sur une
  carte projet. Le moteur numérote les possibilités dans l'ordre du texte
  imprimé, après avoir écarté celles qui sont injouables
  (`engine/src/effects.rs:555-556`).
- **Image 2 (9 branches)** : **oui, Alexis avait raison, c'est bien le choix
  d'une amélioration de carte Phase.** [VÉRIFIÉ 31-07]
  `engine/src/flow.rs:983-1009` : le moteur dresse la liste des améliorations
  possibles — **5 phases × 2 variantes A et B = 10**, moins la variante déjà en
  place chez le joueur, d'où **9** exactement dans la situation de la capture —
  puis appelle le même `choose_option` générique. L'interface, qui ne reçoit
  qu'un nombre, ne peut que dire « branche ».

**Bonne nouvelle pour la réparation.** Les dix visuels des cartes Phase
améliorées sont **déjà présents** dans les fichiers livrés [VÉRIFIÉ 31-07] :
`assets/plateau/carte-phase-{1..5}-{development,construction,action,production,research}-amelioree-{a,b}.webp`.
Il n'y a rien à photographier, seulement à afficher.

**Travail à faire.**
1. **Moteur** : enrichir le choix pour qu'il transporte le **sens** de la
   question — de quelle carte il s'agit, et le texte de chaque option. Sans cela
   l'interface ne pourra jamais faire mieux que « branche 1 ». C'est le
   changement structurant de ce chantier.
2. **Interface, alternative de carte** : montrer la carte concernée en grand, et
   chaque option avec son texte imprimé.
3. **Interface, amélioration de phase** : montrer les **dix cartes Phase
   améliorées en image**, celle déjà possédée signalée comme telle, et l'on
   choisit en cliquant sur une carte, pas sur un bouton gris.

---

## 3. Choix des corporations — pas de loupe au survol

La carte de corporation est déjà affichée en très grand pendant ce choix : la
loupe au survol n'apporte rien et gêne. **Supprimer l'agrandissement au survol
sur cet écran**, **garder** le léger soulèvement de la carte qui indique quelle
carte est visée par le curseur.

---

## 4. Un onglet « Settings » (paramètres)

Un onglet accessible à tout moment pendant la partie, contenant au moins :

- **Retour au menu principal** (donc il faut aussi un menu principal, qui n'existe
  pas encore : lancer une partie, régler les boîtes utilisées, quitter).
- **Reminders** (rappels) : de quoi revoir sans quitter la partie les informations
  qu'on oublie, à commencer par **la liste des cartes Phase améliorées** de
  chacun, avec leur visuel et leur texte.

---

## 5. Afficher le prix réduit sur les cartes qu'on s'apprête à jouer

Aujourd'hui la carte affiche son **prix imprimé**. Or beaucoup d'effets le
réduisent (corporation, cartes déjà en jeu, acier, titane). Le joueur doit voir
**ce qu'il va réellement payer**, sur la carte, au moment du choix.

Forme retenue : prix imprimé **barré** et prix effectif à côté, mis en valeur,
uniquement quand les deux diffèrent.

**Point à vérifier avant de chiffrer ce travail** [DÉCLARÉ] : je n'ai pas encore
vérifié si le moteur expose déjà le prix effectif d'une carte jouable ou s'il se
contente de refuser les cartes trop chères. Si l'information n'est pas exposée,
c'est un second ajout au moteur.

---

## 7. La vue plateau — ce qui manque le plus

> **Corrigé plus bas par le point 11.** Alexis a précisé ensuite que le plateau
> doit être **toujours visible**, et non ouvert par la touche Espace. Tout ce qui
> suit sur la **disposition des cartes** reste valable ; seule la façon
> d'accéder au plateau change.


Aujourd'hui on ne voit pas les cartes en jeu ; elles sont réduites dans un coin.
Ce n'est pas jouable.

**Ouverture.** La touche **Espace** ouvre la vue plateau, qui montre les cartes en
jeu **des deux joueurs**. Une fois cette vue en place, **les mini-cartes des coins
de l'écran disparaissent** : elles n'ont plus de raison d'être.

**Disposition des cartes — le point délicat.** Comme sur la photo de la vraie
table : les cartes sont **empilées les unes sur les autres, chacune décalée vers
le haut et vers la droite**, de sorte que restent visibles sur chaque carte :

- la **colonne de badges**, qui se trouve sur la **partie haute** de la carte ;
- le **rectangle en bas à gauche**, qui indique une production, un effet ou une
  action.

C'est ce double impératif qui donne l'escalier montant vers la droite.

**Méthode imposée à l'agent qui fera ce travail** : commencer par poser
**une seule** carte sur **une seule** autre, vérifier à l'image que les deux
zones sont bien visibles et non recouvertes, et **seulement ensuite** passer à
une pile entière. Ne pas attaquer directement l'affichage complet.

**Regroupement en piles.** Une pile par **couleur de carte** : une (ou plusieurs)
pile de vertes, une de bleues, une de rouges. Sur la photo d'Alexis les bleues et
les rouges sont mêlées : c'est une erreur de sa part au moment de la photo, il
faut bien **trois familles séparées**.

**Débordement.** Au-delà d'un certain nombre de cartes dans une même pile, on
**commence une nouvelle pile** de la même couleur à côté, au lieu d'allonger
indéfiniment l'escalier.

**Lecture.** Passer le curseur sur une carte l'**agrandit** pour qu'on puisse la
lire. C'est le seul moyen de lecture prévu, y compris pour le plateau d'en face.

**Mise à l'échelle.** Quand le plateau devient trop grand pour l'écran, un
**dézoom automatique** s'applique pour que l'ensemble tienne en entier. Rien ne
doit sortir de l'écran.

**Plateau adverse.** Il est disposé **en vis-à-vis**, tête-bêche, comme sur une
vraie table. On le lit au curseur, l'agrandissement au survol présentant la
carte à l'endroit.

---

## 8. Bouton pour masquer les points de victoire

Une case à cocher, qui **se coche et se décoche en cours de partie**, masquant
ou révélant les scores en points de victoire des deux joueurs. Utile pour ne pas
s'influencer.

---

## Récapitulatif : ce qui touche au moteur

Trois des huit retours ne peuvent pas être réglés dans la seule interface :

| Retour | Ce qu'il faut changer dans le moteur |
|---|---|
| 1. Remplacement partiel des cartes de départ | `project_mulligan` doit rendre une liste de cartes, plus un oui/non |
| 2. et 6. « branche 1, branche 2… » | le choix doit transporter la carte concernée et le texte de chaque option |
| 5. Prix réduit affiché | le prix effectif doit être exposé (à vérifier : peut-être déjà le cas) |

Le moteur est aujourd'hui **gelé** par un contrôle automatique de la livraison
d'interface (empreinte
`f55a20c708ab0ca46741481a292a308b2a43a5d6ff234b1e14b30f007119c7af`). Ce gel devra
être levé pour ce chantier, puis reposé sur la nouvelle version, et les trois
empreintes de partie de référence recalculées.

---

# Deuxième série de retours — même jour

## 9. La pose d'une carte doit prendre du temps et se voir

Aujourd'hui une carte jouée disparaît sans qu'on ait le temps de la regarder.
C'est un vrai problème de jeu à deux sur le même écran : l'adversaire doit
pouvoir voir ce qui vient d'être joué.

**Comportement voulu.** Quand un joueur clique sur une carte pour la jouer :

1. la **vue plateau s'ouvre** ;
2. on **voit la carte se déplacer** depuis la main jusqu'à sa pile ;
3. elle **se pose** sur la bonne pile, à sa place définitive dans l'escalier ;
4. l'ensemble dure **environ 3 secondes**, assez pour lire la carte.

Référence assumée : le rythme de pose de *Hearthstone*.

## 10. Glisser-déposer « physique » à la manière de Hearthstone

Plutôt que de cliquer, on **attrape** la carte et on la **déplace** jusqu'au
plateau. Pendant le déplacement, la carte :

- **tangue** légèrement, comme si elle avait un poids et une inertie —
  l'inclinaison suit la vitesse et la direction du curseur ;
- est **mise en lumière**, plus claire et détachée du fond que les autres.

**Ma réponse à la question « est-ce jouable ? » : oui.** Ce n'est pas une
difficulté technique, c'est un travail de réglage. L'inclinaison se calcule à
partir du déplacement du curseur entre deux images, et la mise en lumière est un
effet d'affichage courant. Ce qui coûte, c'est le **soin du réglage** : trop de
tangage donne le mal de mer, trop peu ne se voit pas. Il faudra plusieurs
allers-retours avec des captures.

**Pourquoi ça vaut le coup.** Alexis le dit lui-même : l'interface est sobre, il
faut **un élément vraiment soigné** pour qu'elle ne soit pas terne. La carte
qu'on tient en main est le bon candidat : c'est ce que le joueur regarde le plus.

## 11. Disposition permanente de l'écran

Changement de fond par rapport à la première version : **le plateau de jeu est
toujours visible**, il n'est plus une vue qu'on ouvre.

- Les **cartes projet en main** sont décalées **à droite**, disposées en **arc de
  cercle** (comme une main de cartes tenue en éventail).
- Les **cartes Phase** forment une **seconde main, à gauche**.
- Une carte projet **jouable** se reconnaît à son **contour lumineux vert**,
  comme dans *Hearthstone*. Une carte non jouable n'a pas ce contour.
- Les moments particuliers — remplacement des cartes de départ, choix de
  corporation, choix d'amélioration de phase — restent des **écrans en
  superposition**, en grand, par-dessus le plateau qui reste visible dessous.

Cela remplace la disposition actuelle, où les deux mains sont en haut et en bas
et où les cartes en jeu sont reléguées dans les coins.

## 12. Confirmation demandée sur les cartes Phase améliorées

**Oui, confirmé : au moment de choisir une amélioration, aucun visuel n'est
affiché.** [VÉRIFIÉ 31-07]

- La fonction qui fabrique l'image d'une carte Phase améliorée existe bien :
  `vue/materiel.js:62-68` (`imageAmelioration`).
- Elle n'est appelée **qu'à un seul endroit** : `vue/joueurs.js:190`, pour
  afficher dans le panneau d'un joueur les améliorations **qu'il possède déjà**.
- L'écran de décision (`vue/scene.js:138-142`) n'illustre une option que si
  celle-ci porte une image. Or l'option envoyée est `{ libelle: "branche 3" }` :
  aucune image, donc aucun visuel.

Les dix images existent, elles ne sont simplement jamais montrées au bon moment.

## 13. Habillage du plateau — ressources libres proposées

Alexis doute qu'on trouve un décor aussi propre que celui de *Hearthstone*, et il
a raison : ce décor est peint à la main par un studio. Mais il existe des
matériaux libres de droits de très bonne qualité, et surtout **une ressource que
ce jeu-ci est le seul à pouvoir exploiter**.

**a) Les photographies réelles de Mars — domaine public, résolution énorme.**
La caméra HiRISE de la sonde *Mars Reconnaissance Orbiter* photographie la
surface de Mars depuis 2006 à 30 centimètres par point d'image, et **toutes ces
images sont dans le domaine public**, sans restriction d'usage ; l'université
d'Arizona demande simplement de créditer « NASA/JPL/University of Arizona ».
Plus de 100 000 images sont disponibles. C'est de quoi faire un fond de plateau
qui est *réellement* la surface de Mars, ce qu'aucun autre jeu ne peut se
permettre gratuitement.
→ https://www.uahirise.org/ et https://marsoweb.nas.nasa.gov/hirise/hirise_images/

**b) Les matériaux — Poly Haven et ambientCG, licence CC0.**
Métal brossé, béton, verre dépoli, en très haute résolution, avec toutes les
cartes de relief et de brillance. Licence CC0 : usage libre, y compris
commercial, sans obligation de crédit. C'est ce qui servira à fabriquer le
**cadre** du plateau, les bordures, les emplacements de piles — la partie
« fabriquée » du décor, par-dessus le sol martien.
→ https://polyhaven.com/license et https://docs.ambientcg.com/license/

**c) Les pictogrammes — Kenney, licence CC0.**
Trois lots de jeu de société (250, 490 et 280 éléments) : jetons, emplacements,
flèches, marqueurs. Utile pour les petits éléments d'interface, pas pour le
décor.
→ https://kenney.nl/assets/boardgame-pack et https://kenney.nl/assets/board-game-icons

**Ma proposition de direction, à valider par Alexis** : sol = photographie
HiRISE réelle, assombrie et désaturée pour ne pas concurrencer les cartes ;
par-dessus, un cadre en métal et verre fabriqué à partir des matériaux CC0, avec
les emplacements de piles gravés dedans. Les cartes restent l'élément le plus
lumineux de l'écran.

---

# Troisième série de retours — même soirée

## 14. La sauvegarde de partie

**Question d'Alexis** : aujourd'hui, rafraîchir la page ou fermer l'onglet perd la
partie. Est-ce une contrainte inhérente à une page HTML seule ?

**Réponse : non, pas du tout — et chez nous c'est même presque gratuit.**
[VÉRIFIÉ 31-07]

Le pont entre la page et le moteur fonctionne déjà par **rejeu** : il reprend la
partie depuis sa **graine** (le nombre qui détermine tout le hasard) en lui
réappliquant **la liste des décisions déjà prises**, et s'arrête à la première
décision non encore enregistrée
(`web/webapp/wasm/src/lib.rs`, service `pas`, documenté en tête de fichier).

Autrement dit, une partie **est déjà** entièrement décrite par deux choses :
un nombre, et une liste de nombres. Sauvegarder, c'est écrire ces quelques
centaines d'octets ; charger, c'est les relire. Aucun besoin de serveur.

**Ce qu'il faut ajouter au chantier :**
1. **Reprise automatique** : à chaque décision, la partie en cours est écrite
   dans la mémoire locale du navigateur. On rafraîchit, on retrouve sa partie.
2. **Sauvegarde et chargement de fichier** : un bouton qui exporte la partie en
   un petit fichier, un autre qui la recharge. C'est ce qui permet de reprendre
   une partie sur un autre ordinateur, ou de la transmettre.
3. **Retour en arrière** : puisque la partie est une liste de décisions, revenir
   d'un coup en arrière consiste à retirer la dernière. À prévoir, au moins pour
   corriger un clic malheureux.

Pour le jeu **en ligne** (deux ordinateurs distants), il faudra un serveur — mais
c'est un tout autre sujet, et la sauvegarde locale n'en dépend pas.

## 15. La carte des océans

Il manque à l'écran la **carte des tuiles Océan**, avec les océans **déjà
révélés** visibles.

- Elle est affichée **en petit, à côté des cartes en jeu**.
- On peut **zoomer dessus** pour voir précisément lesquelles sont retournées.
- **Révéler un océan doit être une vraie animation** : on voit la tuile se
  retourner et on **découvre** ce qu'elle était. C'est un moment de jeu, pas une
  ligne de compteur qui s'incrémente.

## 16. « Défausser 1 carte pour du MC » — deux défauts, un corrigé cette nuit

Alexis a relevé deux choses sur cette action de la phase Action.

**a) On ne peut pas choisir la carte défaussée. → CORRIGÉ le 31-07.**
[VÉRIFIÉ 31-07] Le moteur tirait la carte **au hasard** :
`engine/src/flow.rs`, branche `ActionOpt::SellCard`, faisait
`game.rng.gen_range(0..n)`. Une méthode `Policy::sell_card` a été ajoutée : le
moteur **demande** désormais quelle carte, en présentant la main entière. Son
comportement par défaut reproduit l'ancien tirage à l'identique, si bien que les
empreintes de référence n'ont pas bougé de ce fait. Le pont web pose la question
au joueur (« Quelle carte vendez-vous pour 3 MC ? »). Un test en partie réelle
le prouve (`engine/tests/engine_tests.rs`,
`selling_a_card_asks_the_policy_which_one`).

**b) L'action devrait être possible à tout moment. → écart connu, NON corrigé.**
[VÉRIFIÉ 31-07] Alexis a raison, et le livret est formel : « à tout moment, vous
pouvez défausser une carte Projet de votre main pour gagner 3 MC »
(`docs/regles/livret-base.md:96` et `:310`). Cet écart était **déjà recensé** :
`docs/regles/notes/conformite-moteur-24-07.md` §E4 — l'action n'existe
aujourd'hui qu'en phase Action et à l'étape de fin de ronde.

**Ma recommandation : ne pas le corriger dans le même mouvement.** Rendre cette
action possible « à tout moment » ouvre un point de décision à chaque instant du
jeu, ce qui pèse lourd sur la future intelligence artificielle (l'arbre des coups
possibles gonfle énormément) et demande de définir précisément les moments où on
l'autorise. C'est un chantier de moteur à part entière, à traiter posément, pas
un réglage d'interface.

## 17. Les jauges de terraformation — style du jeu, et sur les côtés

Les jauges de **température** et d'**oxygène** doivent :

- **ressembler à celles du jeu imprimé**, reconstruites en numérique — pas des
  barres génériques ;
- être placées **de part et d'autre de l'écran** : température **à gauche**,
  oxygène **à droite** ;
- être en **surimpression**, dans un esprit d'affichage lumineux de poste de
  pilotage ;
- rendre lisibles **les seuils de couleur** : beaucoup de cartes exigent « au
  moins tel niveau », repéré par une couleur sur la piste. On doit voir d'un coup
  d'œil où l'on en est par rapport à ces seuils.

## 18. L'écran de lancement est fade

Il faut le reprendre en s'appuyant sur le **matériel officiel** : visuels de
boîte, illustrations, éléments graphiques présents dans les modules *Tabletop
Simulator* déjà récupérés. Objectif : un écran d'accueil qui soit beau **et**
fidèle au jeu de société.

À vérifier au moment du chantier : quels visuels de boîte et d'ambiance sont
réellement disponibles dans ce qui a été récupéré.

---

# Le plan retenu, et ce qui a été lancé dans la nuit du 31 juillet

Les dix-huit points ne tiennent pas dans un seul chantier. Ils ont été répartis
en quatre, dont deux sont **déjà lancés**.

| Chantier | Contenu | État au 01-08 au matin |
|---|---|---|
| Moteur (fait par le CTO) | 1 · remplacement partiel des cartes de départ ; 16a · choisir la carte vendue | **livré et enregistré** |
| `choix-parlants` | 2 et 6 · le moteur doit dire de quoi il parle, et le pont poser de vraies questions | **en cours** |
| `plateau-vivant` | tout en anglais ; 3 · pas de loupe au choix des corporations ; 7 et 11 · plateau permanent, cartes empilées, mains à droite et à gauche ; 8 · masquer les points ; 12 · cartes Phase améliorées visibles ; 15 · carte des océans | **en cours** |
| à lancer ensuite | 5 · prix réduit ; 9 · pose animée ; 10 · glisser-déposer physique ; 13 · décor du plateau ; 14 · sauvegarde ; 15 · retournement animé des tuiles ; 17 · jauges de style poste de pilotage ; 18 · écran d'accueil | pas commencé |

Les deux chantiers en cours travaillent sur des fichiers **disjoints** — l'un
sur le moteur et le pont, l'autre sur la page — pour pouvoir avancer en même
temps sans se marcher dessus.

## Ce que la préparation a révélé au passage

**Une régression que j'avais introduite le soir même.** En rendant le
remplacement des cartes de départ partiel, j'ai fait poser au moteur une
question d'un genre nouveau : « cochez entre 0 et 8 cartes », sans nombre imposé.
L'écran, lui, exigeait un compte exact : il annonçait `data-a-choisir="undefined"`
et **la partie se bloquait à la mise en place**. Corrigé : attribut absent quand
le nombre est libre, validation possible dès zéro carte cochée.

C'est exactement le genre de défaut qu'un contrôle automatique attrape et qu'une
relecture ne voit pas — il n'est apparu qu'en faisant jouer une machine.

---

## 19. Les tuiles Océan : le moteur sait, mais ne le dit pas — dette ouverte

Découvert le 01-08 en construisant la carte des océans. [VÉRIFIÉ 01-08]

Le moteur **modélise bien les neuf tuiles Océan une à une** : `flow::reveal_ocean`
tire la tuile suivante de `game.oceans`, incrémente `oceans_revealed`, et applique
ses bonus propres (`tile.mc`, `tile.plants`, `tile.cards`). La règle est donc
juste.

Mais `observe::state_view` n'expose qu'un **compte** — `planet.oceans` — jamais
l'identité des tuiles retournées. L'écran ne peut donc pas dire *lesquelles* le
sont : il affiche les premières de la planche, ce qui est faux dans le détail.

**Ce qu'il faut faire** : exposer, dans l'état rendu, la liste des tuiles déjà
révélées (leur rang dans la planche, ou leur identifiant). C'est un ajout
d'affichage, pas une règle : quelques lignes dans `engine/src/observe.rs` et dans
le pont. À faire avant l'animation de retournement (point 15), qui n'aurait aucun
sens sur une tuile inventée.

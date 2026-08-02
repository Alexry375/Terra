# Carte d'état — Projet Terra

> Source de vérité du projet. Ancrée au code (`fichier:ligne`) dès qu'il y aura du
> code. [VÉRIFIÉ JJ-MM] = relu à la source ce jour-là. [DÉCLARÉ] = non re-vérifié.

Dernière mise à jour : 2026-08-02

## 🚧 TROIS CHANTIERS EN COURS (02-08) — `table-vivante`, `bandeau-et-monde`, `menu-et-options`

### `menu-et-options` — scellé et lancé le 02-08 après-midi

7 contrôles visibles, 3 cachés. L'écran d'accueil est refait (Alexis, deux fois :
« le menu est toujours moche »), un bouton d'options devient atteignable à tout
instant — reprendre, aide, réglages, retour au menu principal — et l'aide montre
les quinze faces des cartes Phase, dont les dix améliorées que rien ne permettait
de consulter. Chantier purement d'affichage : `engine/` et le pont lui sont
interdits, et le contrôle 07 le vérifie octet pour octet. Trois contrôles cachés :
la main adverse ne fuit pas, le bouton d'options ne recouvre jamais rien de
jouable (à trois tailles), et trois allers-retours menu/partie — dont un depuis
l'écran de fin — sans qu'une partie abandonnée réponde encore ni que le document
enfle.

Volontairement **hors** de ce chantier, pour ne pas mélanger affichage et règles :
la sauvegarde de partie, le réglage « ne jamais me proposer de vendre des cartes
pour payer », les effets sonores.

### Les deux premiers, lancés le matin

Lancés en parallèle après la troisième série de retours d'Alexis. Zones
volontairement disjointes : le premier tient les mains, les cartes et le milieu
de l'écran ; le second tient le bandeau du haut, les deux bords et le score.
Chacun écrit ses styles dans un fichier à lui (`style-table.css`,
`style-monde.css`) : `style.css` est interdit aux deux, pour que la fusion soit
possible.

- `table-vivante` — 10 contrôles visibles, 6 cachés (dont 4 repris de
  `cadre-de-jeu` : l'opacité de la main adverse doit tenir). On pose les cartes
  en les glissant, le clic reste équivalent ; les cartes Phase se posent par un
  clic avec rotation ; la carte de la manche précédente est couchée de côté
  **avant** le choix ; deux cartes si les deux joueurs prennent la même phase.
- `bandeau-et-monde` — 7 contrôles visibles, 2 cachés. Deux arcs de cercle
  gradués comme le plateau imprimé, la planche des neuf tuiles Océan qui se
  retournent, et un score qui se décompose. Une seule modification du moteur
  autorisée : publier la ventilation du score, sans toucher au calcul.

### Trois faits mesurés ce matin, avant d'écrire les contrats

1. **Le score de 17 en début de partie est juste.** `score_parts`
   (`engine/src/flow.rs:4339`) compte les récompenses comme si la partie
   s'arrêtait : au départ les deux joueurs sont à égalité sur les trois, et une
   égalité vaut 4 points à chacun. 5 de note de terraformation + 12 = 17.
   Mesuré en lançant le moteur sur une partie neuve, graine 5150.
   [VÉRIFIÉ 02-08] Ce n'est donc pas un défaut de calcul mais un défaut
   d'explication : la ventilation manque.
2. **Le bouton « pass » dépasse bien les cartes.** Le conteneur du bouton et
   celui d'une carte font la même hauteur (194 px), mais l'image de la carte
   n'en fait que 167 : le bouton est peint sur 27 px de plus.
   [VÉRIFIÉ 02-08 — mesure Playwright, graine 5150]
3. **Mon constat du chevauchement du bandeau était FAUX.** J'avais annoncé à
   Alexis que « OCEANS » était recouvert par « ROUND ». Mesure à six tailles de
   fenêtre : aucun texte n'en recouvre un autre. Les vrais défauts sont
   ailleurs — sous 1440 px de large, les blocs débordent du bandeau de 6 à
   12 px et les pastilles de récompense sont coupées.
   [VÉRIFIÉ 02-08] Le contrat a été corrigé en conséquence, et le contrôle
   mesure le défaut réel.

### Quatre faits mesurés l'après-midi, sur questions d'Alexis

4. **Vendre des cartes pour payer est une règle officielle, pas un défaut.**
   Défausser une carte de sa main rapporte 3 MC pour compléter un paiement
   (`engine/src/state.rs:32` `SELL_CARD_MC = 3`, appliqué en
   `engine/src/flow.rs:2342-2369`, le commentaire cite le livret p. 13).
   [VÉRIFIÉ 02-08] Alexis préférerait qu'une action trop chère soit simplement
   indisponible. Je le lui ai déconseillé : son ami jouera la vraie règle, et
   une intelligence artificielle entraînée sans elle ne saurait pas la voir
   venir. Ma proposition, qui couvre sa demande sans amputer le moteur : vente
   **volontaire** (on peut refuser et renoncer à la construction), choix des
   cartes par le joueur, et un réglage « ne jamais me proposer de vendre » qui
   la fait disparaître pour de bon. En attente de sa décision.
5. **Le joueur ne choisit pas quelles cartes partent.** Le moteur prend les
   **dernières de la main** (`engine/src/flow.rs:2354`, choix assumé pour la
   sonde séquence). [VÉRIFIÉ 02-08]
6. **Le prix réduit n'est affiché nulle part, et n'est même pas transmis.** Le
   module qui dessine une carte lit un prix mais ne l'écrit jamais
   (`web/webapp/vue/cartes.js:20`), et le pont ne publie que le prix **imprimé**
   (`web/webapp/wasm/src/lib.rs:693`) : aucune des cinq sources de réduction
   (badges, carte suivante, microbes, plantes, titane) n'arrive à l'écran.
   [VÉRIFIÉ 02-08] Chantier 4 : publier le prix effectif depuis le **même**
   service que celui qui prélève l'argent, jamais un second calcul dans la page.
7. **La température est affichée en crans, pas en degrés.** Le moteur compte des
   niveaux 0 à 19 (`engine/src/state.rs:19`, « index 19 == +8 °C »), et l'écran
   recopie ce numéro tel quel (`web/webapp/vue/monde.js:66`). Le joueur lit
   « 3 / 19 » là où le jeu dit « −24 °C ». [VÉRIFIÉ 02-08] L'oxygène (0-14 %) et
   les océans (0-9) sont, eux, dans leur vraie unité. À vérifier à l'audit de
   `bandeau-et-monde` : le contrat impose l'arc gradué de −30 à +8, mais pas la
   correction du chiffre écrit à côté.

### Une faiblesse de mes propres contrôles, trouvée sur question d'Alexis

Aucun des 7 contrôles visibles de `bandeau-et-monde` ne vérifiait **de quel
côté** chaque arc se trouve : deux arcs empilés du même bord auraient été
acceptés. Contrôle caché ajouté le 02-08 après le scellement,
`03-chacun-son-bord.sh` : température dans la moitié gauche, oxygène dans la
moitié droite, sans recouvrement mutuel ni recouvrement de la main, à trois
tailles. Le contrat, lui, le disait déjà noir sur blanc — c'est le contrôle qui
manquait, pas la consigne.

## 🎭 LE CADRE DE JEU EXISTE (02-08) — `cadre-de-jeu`, audité OK (6/6 et 4/4) et promu

L'écran montrait les DEUX mains en clair : il servait à vérifier le moteur, pas à
jouer. C'est fini. **Ma main est en bas et lisible, celle de l'adversaire en haut
et retournée**, sur le sol martien choisi le 01-08. C'est le cadre définitif :
le même contre un programme et contre un humain à distance.

- `?siege=0|1` place le siège regardé, `?decide=humain|programme` dit qui répond
  pour lui. Deux réglages **vraiment** indépendants : on peut regarder son propre
  programme jouer, cartes en clair, comme si c'était soi.
  [VÉRIFIÉ 02-08 — `web/webapp/interface.js`, `siegeHumain` / `siegeProgramme`]
- La zone adverse ne reçoit que des dos et un NOMBRE. Aucun nom, aucun
  identifiant. [VÉRIFIÉ 02-08 — `web/webapp/vue/mains.js:155`]
- Les cartes Phase montrées sont celles choisies dans la manche, celle en cours
  allumée (déduite du type de la décision : l'état ne la rend pas).
  [VÉRIFIÉ 02-08 — `web/webapp/vue/phases.js`]
- Le moteur est intact d'un octet, et le pont n'a pas été modifié : le nombre de
  cartes adverses était déjà déductible. [VÉRIFIÉ 02-08 — contrôle 06]

### Deux fuites réelles, trouvées par l'audit et corrigées

1. **La phase choisie.** Le moteur interroge toujours le joueur 0 en premier :
   vu du siège 1, la barre d'équipage affichait la carte que l'adversaire venait
   de poser **face cachée**. Mesure avant correction : **43 planifications sur
   43**, toutes prouvées — la valeur avait changé depuis la manche précédente,
   ce n'était donc pas une rémanence. On choisissait sa phase en connaissant
   celle d'en face, à chaque manche. **C'était une faute de mon contrat** : le
   contrôle 04 exigeait d'afficher cette valeur. Contrainte levée par écrit,
   puis corrigée. [VÉRIFIÉ 02-08 — contrôle caché 03, trois configurations]
2. **La corporation.** Même défaut un cran plus tôt. Le livret l'interdit
   (l. 211 : distribution face cachée ; l. 215 : révélation commune). Le nom
   voyageait par trois chemins à la fois — l'attribut, le texte de remplacement
   de l'image, et le nom de fichier du scan.
   [VÉRIFIÉ 02-08 — contrôle caché 04]

### Mon propre contrôle caché était faux — le fait le plus important de l'audit

Le contrôle censé prouver l'opacité de la main rejouait la partie **deux fois
séparément**, en espérant que ce soit la même. Ce n'en était pas une : dans la
page, l'adversaire est joué par un programme du navigateur, et le siège regardé
ne répond qu'à ses propres questions. **159 décisions dans la page contre 345
dans ma référence, 135 désaccords de forme.** Il cherchait donc les cartes d'une
main qui n'existait pas : sans valeur, ni pour accuser ni pour disculper — et il
avait pourtant servi à déclarer 852 fuites sur l'ancien écran.

Refait sur une base saine : **on joue d'abord** en relevant tout ce que la page
livre, **le moteur rejoue ensuite** la même partie avec les réponses réellement
données. Éprouvé dans les deux sens : vert sur la livraison (137 et 143
occasions), rouge sur deux versions volontairement fautives (513 et 762 fautes).
[VÉRIFIÉ 02-08]

**Leçon générale, à appliquer à tous les contrôles à venir :** un contrôle qui
reconstruit une référence par un chemin parallèle doit d'abord prouver que les
deux chemins produisent le même objet. Sinon il mesure autre chose que ce qu'il
croit.

### Ce qui reste ouvert sur cet écran, déclaré et accepté

- La phase en cours est **déduite** du type de la décision, pas lue : l'état du
  moteur ne la rend pas. 0 à 2 % des écrans n'allument alors rien plutôt que de
  deviner. [DÉCLARÉ par l'agent, mesuré par son banc]
- Sous ~640 px de hauteur de fenêtre, les deux plateaux manquent de place. Les
  six contrôles mesurent en 1600 × 1000, et le contrôle caché sur six tailles
  descend à 1600 × 720. [DÉCLARÉ]
- `data-cartes` compte la main de projets : à la mise en place, les deux cartes
  Corporation que l'adversaire tient ne sont pas dans l'état, la zone affiche
  donc 0 à cet instant. [DÉCLARÉ]

## 🌊 LES TUILES OCÉAN ONT UNE IDENTITÉ (02-08)

`OceanTile` porte son rang sur la planche imprimée, et `state_view` publie la
liste des tuiles déjà retournées (`oceans_revealed_tiles`). L'écran connaissait
le NOMBRE d'océans, jamais lesquels : il en montrait de fausses. Le champ ne
participe à aucune règle. **821 tests verts, les trois empreintes de référence
inchangées.** [VÉRIFIÉ 02-08 — `engine/src/state.rs:48`, `engine/src/observe.rs:122`]

## 🔵 62 CARTES BLEUES SUR 101 NE PORTENT AUCUNE ACTION (02-08)

Relevé refait en interrogeant le moteur, non plus en lisant le texte des cartes :
39 cartes bleues portent une action, 62 n'en portent pas. Mon premier relevé
annonçait « au moins huit ». La correction du 01-08 les couvre toutes — le filtre
porte sur la fiche d'effet, pas sur une liste de noms.
[VÉRIFIÉ 02-08 — `engine/src/flow.rs:2867`]

## 🇬🇧 L'ÉCRAN PARLE ANGLAIS ET MONTRE LES CARTES (01-08) — `fusion-parlante`, audité OK et promu

Réunion des deux chantiers parallèles. **Audit sans faute du premier coup :
4/4 visibles, 2/2 cachés, zéro altération.** [VÉRIFIÉ 01-08]

- **26 natures de question** ont leur intitulé anglais, bâti sur les champs du
  descripteur. L'agent est remonté à `ChoiceContext::kind` pour établir la liste
  **close**, au lieu de se fier aux trois exemples de mon contrat.
- **Vocabulaire du jeu rétabli** : Score, Corporation, Temperature, Oceans,
  Hand, Phase card. Les abréviations (VP, Temp, Corp, « stage card ») venaient
  d'un défaut de **mon** contrôle de langue, qui prenait ces mots anglais pour du
  français.
- **Les dix cartes Phase améliorées s'affichent en image** au moment du choix,
  désignées par le couple phase/variante de l'option, jamais par son rang.
  Vérifié de mes yeux sur capture : « Upgrade a Phase card: which one, and which
  variant? » avec les dix cartes imprimées.
- Trois écrans de choix passent en superposition : la carte concernée passait de
  90 px illisibles à 410 px.
- Défaut d'affichage réel corrigé : sans `aspect-ratio`, une image de carte
  mesure 0 px tant qu'elle n'est pas décodée.

**Vérification propre de l'agent, au-delà des contrôles** : 60 parties,
**24 501 décisions**, 26 natures, zéro accent, zéro mot français, zéro repli.

**Limite déclarée** : pour **4 natures sur 26**, le pont ne décrit une
proposition qu'en français (`describe_branch`, `describe_selector_grant`), sans
champ exploitable. L'écran dit « Printed option N » et montre la carte en grand.
À traiter côté pont — c'est la prochaine dette d'interface.

## 🃏 LE PLATEAU EST PERMANENT (01-08) — `plateau-vivant`, audité OK et promu

**On voit enfin la partie.** L'écran est refait autour d'un plateau permanent :
les cartes des deux joueurs posées en escalier, le plateau d'en face en
vis-à-vis, la scène de décision en bande centrale, la main projet en arc de
cercle à droite, les cartes Phase à gauche.

- **L'empilement** : décalage de 40 % de la largeur vers la droite, 9 % vers le
  haut. Le chiffre vient d'un **regard sur une capture**, pas du seuil du
  contrôle : sur les scans, le rectangle imprimé en bas à gauche va jusqu'à 37 %
  de la carte et se trouvait coupé à 36 %. [VÉRIFIÉ 01-08]
- Piles par couleur, nouvelle pile au-delà d'un seuil, **réduction automatique**
  quand le plateau déborde — rien n'est masqué, rien ne défile.
- Contour vert des cartes jouables : **recopie** des options que le moteur
  énumère, aucun prix comparé à aucune bourse.
- Loupe au survol : **485 px, à l'endroit, sur les deux plateaux**.
- Carte des tuiles Océan, bouton pour masquer les points de victoire, tout
  l'habillage en anglais.

**Audit** [VÉRIFIÉ 01-08] : **5/5 visibles, 2/2 cachés**, zéro altération. Le
contrôle caché décisif confronte chaque valeur affichée à l'état réel du moteur
sur deux parties entières : **3243 valeurs, zéro écart**, mêmes décisions et
mêmes scores qu'en ligne de commande. Moteur et pont **intacts octet pour
octet**.

**Les deux échecs initiaux venaient de MES contrôles, pas de la livraison** : le
pilote secret ne savait pas répondre à un choix multiple à nombre libre (règle
née la veille), et le contrôle du survol ignorait la loupe. Corrigés, tous deux
passent. Leçon : un contrôle hérité d'un chantier précédent doit être relu à
l'aune des règles qui ont changé depuis.

**Deux limites connues, déclarées par l'agent** :
1. Le moteur ne rend qu'un **compte** d'océans, jamais l'identité des tuiles
   retournées — alors qu'il les modélise bien une à une (`flow::reveal_ocean`
   lit `tile.mc`, `tile.plants`, `tile.cards`). Ce n'est donc pas une lacune de
   règle, seulement un manque d'exposition dans `observe::state_view`. L'écran
   révèle les *n* premières tuiles de la planche, ce qui est faux dans le détail.
2. L'écran évite six mots anglais corrects (`score`, `phase`, `corporation`,
   `points`, `temperature`, `oceans`) parce que **mon** contrôle de langue les
   prenait pour du français : il dit VP, Temp, Corp, « stage card ». Défaut de
   mon contrôle, réparé au chantier `fusion-parlante`.

## 🗣️ LES CHOIX PARLENT (01-08) — `choix-parlants`, audité OK et promu

**Le moteur transporte désormais le SENS de chaque alternative jusqu'à celui qui
décide.** Il n'annonçait qu'un nombre d'options ; l'écran ne pouvait afficher que
« branche 1 », « branche 2 »… — le défaut le plus grave relevé par Alexis.

- Nouveau `engine/src/choice.rs` : `ChoiceContext`, **onze variantes pour les
  onze sites d'appel** de `flow.rs`, aucune fourre-tout. [VÉRIFIÉ 01-08]
- Seconde voie `Policy::choose_option_ctx`, **corps par défaut retombant sur
  `choose_option`** avec le même nombre d'options : aucune politique existante
  modifiée, **empreintes intactes**. `ProbePolicy` et `ObservingPolicy`
  délèguent.
- Le pont WebAssembly a perdu ses libellés « branche N ». Une amélioration de
  carte Phase annonce phase, variante et **nom imprimé**, lus dans
  `effects::PHASE_UPGRADED` — la table que le moteur consulte pour appliquer la
  règle. Le pont ne recalcule rien.

**Chiffres** [VÉRIFIÉ 01-08] : **818 tests verts** ; empreintes `d6a7267472501b13`,
`51e7966094e225cb`, `2b5235e31f71c812` **inchangées** ; audit **4/4 visibles,
2/2 cachés**, zéro altération du contrat ; 11 547 décisions écoutées au travers
du pont recompilé, **11 natures sur 11** rencontrées.

**Le contrôle caché qui comptait** : confronter ce que le moteur ANNONCE à ce
qu'il APPLIQUE ensuite, avec une règle de choix que l'agent n'a jamais vue —
**93 améliorations confrontées, 93 conformes**.

**Deux choses honnêtes à noter.**
1. Ce contrôle caché a d'abord été compté en échec **deux fois** : `aw` coupe
   chaque contrôle à 120 secondes et le mien en demandait dix minutes. Défaut de
   mon contrôle, pas de la livraison — allégé, puis relancé avec
   `AW_CHECK_TIMEOUT=1500`.
2. L'agent a signalé de lui-même que son test de rétrocompatibilité était
   **tautologique** (il comparait une valeur à elle-même) après relecture
   adversariale, et l'a remplacé par un second calcul disjoint vérifié par
   mutation. C'est exactement le comportement attendu.

**Reste à faire** : `vue/scene.js` n'exploite pas encore ce que le moteur dit —
c'est là que l'amélioration de carte Phase deviendra un visuel. Cela revient au
chantier d'interface.

## ⚖️ LE MOTEUR APPREND DEUX RÈGLES (31-07) — mulligan partiel, vente choisie

Alexis a joué lui-même sur l'interface livrée la veille et a relevé dix-huit
points. **Deux d'entre eux n'étaient pas des défauts d'affichage mais des règles
fausses dans le moteur**, corrigées le soir même. Les seize autres sont des
travaux d'interface, réunis dans `docs/INTERFACE_RETOURS_01.md`, qui sert de
cahier des charges au chantier de refonte.

**1. Le remplacement des cartes projet de départ n'était pas partiel.**
[VÉRIFIÉ 31-07] `Policy::project_mulligan` rendait un oui/non et `setup_game`
défaussait la main entière. La règle est « entre 0 et 8 cartes, au choix ». La
méthode rend désormais `Vec<usize>` — les indices des cartes rendues, comme
`discard_down` — et `engine/src/flow.rs` ne défausse que celles-là, en assainissant
les indices hors bornes ou répétés. Le tout ou rien ne concerne plus que les
corporations.

**2. La carte vendue pour 3 MC était tirée au hasard par le moteur.**
[VÉRIFIÉ 31-07] La branche `ActionOpt::SellCard` faisait `game.rng.gen_range`.
Une méthode `Policy::sell_card` a été ajoutée : le moteur demande, en présentant
la main entière. Son corps par défaut reproduit l'ancien tirage **à l'identique**,
donc les politiques existantes jouent exactement comme avant et cette correction
seule ne déplace aucune empreinte. `ProbePolicy` et `ObservingPolicy` la
délèguent toutes deux.

**Reste connu et NON corrigé** : vendre une carte devrait être possible « à tout
moment » (`docs/regles/livret-base.md:96`), pas seulement en phase Action et en
fin de ronde. Écart déjà recensé (`docs/regles/notes/conformite-moteur-24-07.md`
§E4). Ma recommandation : chantier moteur à part, car cela ouvre un point de
décision permanent qui pèsera lourd sur la future intelligence artificielle.

**Les chiffres** [VÉRIFIÉ 31-07] :

| Mesure | Valeur |
|---|---|
| Tests | **813 verts** (810 + 3 neufs sur le mulligan et la vente), 0 échec |
| Empreinte `--seed 2024 --boites base` | `d6a7267472501b13` (était `cee020cda9db283b`) |
| Empreinte `--seed 4242 --boites base` | `51e7966094e225cb` (était `981bb47e336034cc`) |
| Empreinte `--seed 4242 --boites base,decouverte` | `2b5235e31f71c812` (était `c20dd5be100de393`) |
| Invariants cassés sur 3 × 1000 parties | **0** |
| Effets de carte non traités | **0** |

Les trois empreintes ont changé, et **c'est attendu** : remplacer trois cartes au
lieu de huit modifie l'ordre de sortie du paquet. Elles ont été recalculées et
réinscrites dans les trois tests qui les figent.

**Deux tests fragiles démasqués au passage.** `lot7_tests` mesurait le bonus de
recherche d'*Interns* et d'*Extended Resources* sans neutraliser la corporation,
elle-même tirée au sort par la graine ; ils ne passaient que parce que le tirage
tombait bien. Ils retranchent désormais l'apport de la corporation, mesuré avant
de poser les cartes.

**Dette réparée en passant** : `web/webapp/wasm/Cargo.toml` pointait vers un
moteur inexistant (`../../../../../engine`, chemin du workspace, jamais recalculé
à la promotion). Le dossier promu n'était donc pas reconstructible. Corrigé,
`web/construire.sh` refabrique `terra.wasm` depuis la racine du dépôt.

## 🖥️ L'ÉCRAN DE JEU EXISTE (30-07) — `interface-visuelle`, audité PARTIEL et promu

**Deux personnes peuvent jouer une partie entière d'Ares Expedition sur le même
écran, dans `web/webapp/`.** Concept « L'HORIZON » : le ciel se réchauffe avec la
température, la mer monte avec les océans, la brume s'épaissit avec l'oxygène.
Les cartes sont les scans réels, jusqu'à 1252 px de large.

- **Contrôles visibles : 4/4** (harnais intact, partie entière cliquée jusqu'aux
  scores, cartes réellement peintes, page autosuffisante hors dépôt).
- **Contrôles cachés : 2/2** [VÉRIFIÉ 30-07]. Le plus important : la même partie
  jouée deux fois, une fois à l'écran et une fois en ligne de commande, avec une
  règle de choix que l'agent n'a jamais vue. **3115 valeurs affichées confrontées
  à l'état réel du moteur, zéro écart**, comptes de décisions et scores finaux
  identiques sur deux configurations. L'écran ne recalcule rien.
- **Un défaut trouvé par moi, pas par les contrôles** : `style.css:545` masquait
  le libellé des options en mode dense ; la plaque « Passer », qui n'a pas
  d'image, devenait un rectangle noir vide — y compris sur « quelle carte
  poser ? », la décision la plus fréquente. Corrigé après promotion
  (`style.css:546-548`), vérifié dans le navigateur (largeur rendue 0 → 89 px) et
  les contrôles 02, 03 et le hold-out vérité rejoués verts sur la version promue.
- **Fausse alerte dans les limites déclarées par l'agent** : « deux cartes sans
  illustration sur 264 ». `--dump-deck --boites base,decouverte` donne **262**
  cartes (246 projets + 16 corporations) et le catalogue d'images les couvre
  **exactement**. `Microbiology Patents` et `Project Inspection` portent le vieux
  drapeau `in_deck_v1` mais ne sont pas dans les boîtes physiques
  (`engine/src/cards.rs:204` : ce drapeau ne compose plus la pioche) — le moteur
  ne les distribue jamais.
- **Décision en attente d'Alexis** : la langue. Mon contrat ne la précisait pas.
  L'habillage est en français (MANCHE, TERRAFORMATION, « Quelle carte poser ? »)
  autour de cartes en anglais.

## 🏷️ LES BADGES SONT TOUS JUSTES (30-07) — 262/262 [VÉRIFIÉ 30-07]

**Les badges (petites icônes rondes qui classent une carte : bâtiment, science,
espace…) de `data/cards.json` ont été confrontés un par un aux images imprimées.
Aucune erreur.** C'était le dernier doute qui restait sur l'héritage du moteur
Java dont ce projet est parti.

- Méthode : découpe de la colonne d'icônes de chaque carte (bord gauche pour les
  246 cartes projet, coin haut gauche pour les 16 corporations), montage en 22
  planches numérotées, lecture **à l'aveugle** par 11 agents qui n'avaient pas
  accès à `data/cards.json`, puis confrontation automatique.
- Premier passage : 239/262. Les 23 écarts étaient **tous** des défauts de ma
  méthode, aucun du dépôt :
  - 11 corporations : leur badge n'est pas dans la colonne de gauche mais en haut
    à gauche — ma découpe le manquait. Recoupées et relues par moi : 16/16 justes,
    y compris *Mining Guild* qui porte bien **deux** badges bâtiment.
  - 12 cartes projet : le badge « espace » est une **étoile jaune sur pastille
    sombre**, que ma consigne décrivait à tort comme une fusée ; les agents l'ont
    donc nommé « ville », « joker » ou « plante ». Relues par moi : 12/12 justes.
- Vocabulaire des icônes établi et vérifié : maison marron = BÂTIMENT, atome =
  SCIENCE, globe = TERRE, éclair magenta = ÉNERGIE, feuille = PLANTE, bactérie =
  MICROBE, empreinte = ANIMAL, flèche noire vers le bas = ÉVÉNEMENT, **étoile
  jaune = ESPACE**, planète rayée = JUPITER, pastille joker = DYNAMIQUE.
- Complète la confrontation partielle faite le même jour contre la transcription
  française de `data/cartes-imprimees/projets-decouverte/` (17 cartes appariables
  sans ambiguïté, 17/17 concordantes).

## 🖼️ CHAQUE CARTE A SON IMAGE (30-07) — `harnais-images`, audité OK et promu

**Les 262 cartes jouables ont chacune une image nette, découpée dans les planches
du module Tabletop, et une entrée de catalogue qui dit d'où elle vient.** Le
chantier 1 de l'interface est donc terminé : le moteur tourne dans le navigateur
ET l'interface a de quoi montrer des cartes.

- `web/webapp/assets/cartes/` : **262 images** (220 boîte de base + 42
  Découverte), une par carte du deck moteur. Format WebP, 409x569 environ,
  20 Mo au total. Noms de fichiers dérivés de la sortie exacte de `--dump-deck`.
- `web/webapp/assets/plateau/` : **112 éléments** de table (tuiles océan,
  compteurs forêt, jetons, astronautes, cartes Phase normales et améliorées,
  dos de cartes, repères, récompenses), 11 Mo.
- `web/webapp/assets/manifeste.json` : pour chaque image, la **planche, la ligne
  et la colonne** d'origine.
- `data/correspondance-decouverte.json` : les 42 cartes Découverte localisées sur
  la planche, **plus les 39 cases surnuméraires** (autres extensions) consignées
  à part pour prouver qu'aucune n'a été prise pour une carte de Découverte.
  C'était la partie sans oracle : aucune donnée du dépôt ne disait où se trouvait
  quelle carte de Découverte.

**Ce que j'ai vérifié moi-même** [VÉRIFIÉ 30-07] :

- les 262 noms du catalogue sont **exactement** le deck rendu par `--dump-deck`
  (ni manquant, ni surnuméraire, ni doublon) ;
- j'ai **lu de mes yeux** les 42 titres imprimés des cartes Découverte sur un
  montage de bandeaux : 42/42 concordent avec le nom de fichier ;
- les 42 **coûts imprimés** concordent avec `data/cards.json` — oracle
  indépendant que ni l'agent ni moi n'avions utilisé pour construire la
  correspondance ;
- contrôle caché n° 1 (couleur du liseré gauche) : **246/246** cartes colorées
  concordent avec la couleur encodée dans le moteur, 0 liseré illisible ;
- contrôle caché n° 2 (redécoupe de la planche aux coordonnées annoncées) :
  **44/44** après correction d'un défaut **de mon propre outil** ; éprouvé dans
  le sens rouge, un décalage d'une seule colonne fait tomber 44/44.

**Divergences déclarées par l'agent, toutes vérifiées justes** : aucune tuile
« cité » n'existe dans les scans (le jeu de cartes n'en a pas — les 9 hexagones
de la base sont 8 tuiles océan et 1 compteur forêt) ; la géométrie annoncée dans
mon contrat était fausse pour trois planches sur quatre ; la planche des cartes
Phase porte 10 faces améliorées et 5 faces normales, pas 15 améliorées — piège
attrapé par l'agent en rouvrant ses propres découpes.

## 👁️ LE MOTEUR EST DEVENU OBSERVABLE (30-07) — `moteur-observe`, audité OK et promu

**Jusqu'à aujourd'hui, celui qui décide dans le moteur ne voyait RIEN de la
partie.** Le trait `Policy` — le point de passage unique de toutes les décisions
d'un joueur — ne recevait que le tirage aléatoire et la liste des options. Ni son
argent, ni ses badges, ni la température, ni sa main. Conséquence : une
intelligence artificielle aurait été aveugle, et une interface n'aurait pas pu
afficher l'état exact au moment d'une décision. [VÉRIFIÉ 30-07 aux 33 sites
d'appel de `flow.rs`]

Ce chantier ouvre la vue, **sans changer une seule décision** :

- `Policy::observe(&mut self, &GameState, player)` — **corps par défaut vide**
  (`engine/src/policy.rs:52`). Les politiques existantes l'héritent, ne
  consomment pas le tirage aléatoire, décident exactement comme avant.
- **33 sites de décision recensés, 33 équipés** dans `flow.rs`. Le diff est
  **purement additif** : 33 lignes ajoutées, 0 retirée, 0 modifiée. Aucun site
  structurellement inéquipable. Aucun `unsafe`, aucun `RefCell`.
  [VÉRIFIÉ 30-07 par diff et par recensement automatique]
- `engine/src/observe.rs` (neuf, 420 lignes) : `ObservingPolicy` enveloppe une
  politique, l'observe et **délègue les 15 méthodes** du trait ; `state_view()`
  rend `GameState` en JSON. Le score vient de `flow::score_parts`, le point de
  calcul unique du moteur — pas de barème parallèle.
- Exposé par `--observe` (une ligne JSON par décision) et `--dump-state`.
  C'est ce que consommera le pont navigateur de l'interface.

**Les chiffres** [VÉRIFIÉ 30-07 après promotion, dans le dépôt] :

| Mesure | Valeur |
|---|---|
| Empreintes de référence | `cee020cda9db283b`, `981bb47e336034cc`, `c20dd5be100de393` — **les 3 inchangées** |
| Tests | **810 verts** (793 avant + 17 neufs), 0 ignoré, aucun test existant touché |
| Observations sur une partie | ~335 à 383 selon la graine |

### Ce que mes contrôles cachés ont prouvé, et ce qu'ils ont raté

Les deux contrôles cachés sont passés. Le premier **recompile le moteur d'origine**
et compare sur trois graines écrites nulle part dans le contrat, avec et sans
observation : identiques. Le second recoupe le flux d'observations contre le
compteur de manches que le moteur publie lui-même : 34 manches vues / 34 jouées,
43 / 43.

**Mon erreur** : mon deuxième contrôle caché exigeait un TR strictement
croissant. Faux — `state.rs:509` (`spend_tr`) le fait reculer légitimement, car
des cartes exigent d'en dépenser (`flow.rs:1807`, `flow.rs:3188`). J'ai corrigé
le contrôle, pas la livraison. [VÉRIFIÉ 30-07]

**Deux défauts de mon contrat, trouvés avant scellement** : une expression de
recherche qui ne reconnaissait pas le type écrit sous sa forme complète ; et
surtout quatre tests du moteur qui lisent `../data/cards.json` par un chemin
relatif inexistant dès qu'on copie le moteur — l'agent aurait vu quatre tests
rouges avant d'écrire une ligne. Remède documenté dans le contrat.

### Sabotage rejoué par ma main [VÉRIFIÉ 30-07]

J'ai retiré une observation sur les 33 et relancé les tests de l'agent :
`une_observation_avant_chaque_decision` passe au rouge (« 2 décisions prises sans
observation préalable »). Ses tests mordent réellement.

### Un test fragile, préexistant, à surveiller

`tests/lot7_tests.rs:1645` exige plus de 3 000 parties/s. Sous charge machine il
échoue par intermittence — **reproduit à l'identique sur le moteur d'origine**
(2 572 parties/s sur la première mesure), donc ce n'est **pas** une régression du
chantier. Machine au repos : ~6 500 à 7 000 parties/s. [VÉRIFIÉ 30-07]

## 🌐 LE MOTEUR TOURNE DANS LE NAVIGATEUR (30-07) — `interface-harnais` manche 2, audité OK et promu

**Une partie complète se joue dans une page internet, à deux joueurs sur le même
écran, et c'est le VRAI moteur Rust qui décide de tout.** Promu dans `web/`
(1,6 Mo : `web/webapp/` servi, `web/vendor/` pour la recompilation).
[VÉRIFIÉ 30-07 après promotion]

### Le fait qui compte : les empreintes coïncident, et j'avais tort

J'avais écrit dans le contrat : « ne cherche pas à faire coïncider les
empreintes, c'est une impasse, elle est démontrée ». **C'était faux.** Le
diagnostic « 32 bits contre 64 bits » était incomplet : la vraie cause est que la
bibliothèque de tirage au sort `rand` 0.8 échantillonne un entier de taille
machine **sur sa propre largeur**. L'agent l'a mesurée là où le contrat croyait
le hasard absent — sur `--probe`, où **19 cartes sur 262** répondaient
différemment.

Correction à la racine : une copie de `rand` 0.8.7 avec **un seul écart**, que
j'ai relu ligne à ligne (`web/vendor/rand-usize64/src/distributions/uniform.rs`,
24 lignes de commentaire + 3 de code) : sous 64 bits, un entier de taille machine
s'échantillonne comme un entier 64 bits ; l'instanciation d'amont pour les
machines 64 bits est conservée telle quelle.

Résultat mesuré par ma main sur **1000 parties** :

| Configuration | Empreinte du navigateur | Empreinte du moteur natif |
|---|---|---|
| `--seed 2024 --boites base` | `cee020cda9db283b` | identique |
| `--seed 4242 --boites base` | `981bb47e336034cc` | identique |
| `--seed 4242 --boites base,decouverte` | `c20dd5be100de393` | identique |

### Le reste de la livraison

- Cible `wasm32-wasip1`, interface C minimale, couche d'adaptation maison
  identique dans Node et dans le navigateur.
- L'état affiché vient de `engine::observe::state_view` capté dans
  `Policy::observe` — l'état **vivant** au moment du choix. Le chantier
  `moteur-observe` a donc servi immédiatement.
- Point d'entrée unique des futurs modes : `web/webapp/fournisseurs.js` +
  `adversaire.md`. Un « fournisseur de décisions » est un objet
  `{ nom, decider(decision, etat) }` ; brancher un cerveau artificiel ou un
  joueur distant = remplacer un élément d'un tableau de deux.
- 55 tests propres à la livraison, 0 échec.
- Contrôle caché passé : la livraison recopiée **hors du dépôt** répond encore
  à l'identique sur 14 cartes tirées au sort. Un imposteur que j'avais écrit
  moi-même (faux moteur appelant le binaire natif par un chemin reconstitué) y
  meurt.

### Cinq défauts que l'agent a trouvés dans son PROPRE travail

Sa relecture adversariale a corrigé : un pont qui captait l'état de la mauvaise
décision (14 discordances sur 383) ; une graine trop grande rendant en silence
la partie d'une autre graine ; `--games -3` tuant l'instance ; une réponse
refusée qui empoisonnait la partie ; un commentaire qui mentait sur le
comportement réel.

### Écarts assumés, déclarés

`--games` plafonné à 1 000 000 par appel (en WebAssembly un débordement est une
panique irrattrapable qui tue la page) ; l'intervalle plein d'un entier de taille
machine tirerait 32 bits au lieu de 64 dans le `rand` corrigé (le moteur ne
l'emploie jamais) ; `web/webapp/` est autosuffisant à l'exécution mais pas à la
recompilation.

## 🎴 LES VISUELS DE DÉCOUVERTE EXISTENT EN NET (30-07) — trouvés sur intuition d'Alexis

Alexis a demandé si un module Tabletop Simulator contenait Découverte. **Oui.**
Ma recherche précédente ne portait que sur un module en **français** (il n'y en a
pas) ; je n'avais jamais cherché Découverte toutes langues confondues. Lacune de
ma part. [VÉRIFIÉ 30-07]

Module Steam Workshop **3159480208**, « Ares Expedition Terraforming Mars
Discovery and Foundation », 12-02-2024, **anglais**, non scripté.

Récupéré dans `data/scans/decouverte-tabletop/` (55 Mo, 53 images + 4 PDF ;
dossier **non versionné**, `.gitignore:4` — il n'existe que sur ce disque) :

| Planche | Taille | Contenu |
|---|---|---|
| `img_82e08cb04661.jpg` | 4096 x 3986 | **projets Découverte**, grille 10 x 7 |
| `img_988c9278d90e.jpg` | 3684 x 4096 | **cartes Phase améliorées**, 5 phases x 3 versions |
| `img_d80e604470e5.jpg` | 4096 x 2136 | **corporations** Découverte + mode Crise |
| `img_b7d95969a643.jpg` | 2948 x 4096 | projets **Fondations** (extension non modélisée) |

Titres lus sur la planche et retrouvés dans `data/cards.json` (42 cartes
`discovery`) : Communications Streamlining, Drone Assisted Construction,
Experimental Technology, Impact Analysis, Volcanic Soil, Dandelions, Blast
Furnaces, Political Influence, Martian Museum. [VÉRIFIÉ 30-07]

**Deux pièges techniques notés** : l'hôte `cloud-3.steamusercontent.com` inscrit
dans les modules répond désormais 403 — les mêmes fichiers répondent sur
`cdn.steamusercontent.com`, même chemin ; et les requêtes d'entête HTTP y
mentent (92 octets annoncés pour un fichier de 15 Mo). **Conséquence pour
l'interface : ne jamais pointer ces adresses, toujours servir les images du
disque.**

**Décision en attente d'Alexis** : anglais net partout, ou français moins net sur
Découverte, ou les deux avec choix de langue. Les photos françaises d'Alexis
(`data/cartes-imprimees/projets-decouverte/`) restent la seule source française.

## Infrastructure du dépôt

- **Dépôt distant créé le 26-07 : `github.com/Alexry375/Terra`, PRIVÉ.**
  Branche `main` poussée. [VÉRIFIÉ 26-07 par `gh api repos/Alexry375/Terra`]
- Avant mise en ligne : `engine/target/` (307 fichiers, 94 Mo d'artefacts de
  compilation) était versionné par erreur depuis le début. Retiré du suivi ET
  purgé de tout l'historique (`git filter-repo`) : le dépôt passe de 71 Mo à
  19,4 Mo, dont ~20 Mo de photos du livret conservées volontairement.
  Sauvegarde de l'ancien `.git` dans le dossier temporaire de la session.
  Traces `.playwright-mcp/` retirées aussi. [VÉRIFIÉ 26-07]
- **`workspaces/` n'est PAS sauvegardé en ligne** : il est exclu par
  `.git/info/exclude:8`, posé par l'outil `aw`. Conséquence à connaître : les
  221 cartes transcrites de `textes-cartes` n'existent que sur le disque local
  tant qu'elles ne sont pas auditées et promues. [VÉRIFIÉ 26-07]

## 🏁 LE CHIFFRE QUI COMPTE (29-07 soir) — TOUT LE CONTENU IMPRIMÉ EST ENCODÉ

**246 cartes projet sur 246 et 16 corporations sur 16 agissent.** Zéro carte
muette dans la configuration cible `base,decouverte`. [VÉRIFIÉ 29-07 par ma
main après promotion, `simulate --dump-deck --boites base,decouverte`]

Oracle disjoint du recensement : `cards_effects_unhandled` mesuré en partie
réelle passe de **991 à 0** sur 1000 parties, graine 4242. [VÉRIFIÉ 29-07]

Trajectoire : 62 muettes le 27-07 → 29 → 18 → 14 → **0 en boîte de base**
(28-07) → 31 en `base,decouverte` → 3 (`decouverte-projets`) → **0**
(`decouverte-jokers-corpos`, 29-07 soir).

**Et plus aucun prérequis imprimé ne manque**, ni en base ni dans les 38 cartes
de Découverte. [VÉRIFIÉ 29-07]

**Le prochain chantier n'est plus du contenu : c'est l'intelligence
artificielle**, qui n'existe pas du tout à ce jour.

## LES 7 DERNIÈRES CARTES (29-07 soir) — `decouverte-jokers-corpos`, audité OK et promu

**793 tests verts** (765 avant), aucun désactivé. Empreinte de la boîte de base
**inchangée** (`cee020cda9db283b`, graine 2024), 1000/1000 parties, 0 violation
d'invariant. [VÉRIFIÉ 29-07]

### Le badge joker

Le badge est choisi **dès que la carte est en main**, avant que le moteur ne
juge si le joueur peut se l'offrir — c'est ce que veut le livret, qui donne
l'exemple d'un joker déclaré Espace faisant baisser le prix de sa propre carte.
Mesuré : *Political Influence* derrière *Metallurgy*, déclarée ESPACE, coûte
**7 au lieu de 10** ; déclarée BÂTIMENT, elle coûte 10. [VÉRIFIÉ 29-07]

Un jeton par carte (`PlayerState::joker_tags`), posé par un nouveau point de
décision `Policy::pick_joker_tag` — donc **interceptable par la future
intelligence artificielle**, au même titre que le choix de phase. Le comptage
passe par le **point de passage unique** `put_in_play` → `tag_counts` : les
prérequis, les productions par badge, les points de victoire, les Objectifs et
les Récompenses en découlent sans code dispersé.

`TAG_COUNT` reste à **10** et `Tag::Dynamic` reste hors décompte : le joker
n'est pas devenu un onzième badge. **Zéro catégorie d'effet neuve.**

### Les 4 corporations

Chacune améliore sa carte Phase à la mise en place (Apollo II, Exocorp V,
Hyperion III, Sultira I), depuis la table d'effets, sans aucun nom de
corporation dans le code du déroulement. [VÉRIFIÉ 29-07]

**Écart de source tranché en faveur du carton** : *Sultira* porte « chaque
badge énergie, **y compris celui-ci** » — soit 2 chaleurs dès la mise en place.
`cards.json` omettait la clause. Mesuré : `corp.start_heat = 2`. [VÉRIFIÉ 29-07]

### Mes deux erreurs de ce chantier

1. **Mon contrat affirmait faux** : « les productions et l'amélioration de phase
   sont déjà encodées, la seule chose neuve est le badge joker ». Les 3 cartes
   n'avaient **aucune entrée** dans la table d'effets (mesuré : 245 → 248
   entrées `card!`). J'ai confondu « le mécanisme existe » et « la carte est
   encodée ». Trouvé par l'agent.
2. **Mes deux contrôles cachés accusaient la livraison de mes propres oublis** :
   l'un n'attendait que les 3 projets dans la liste des cartes devenues actives,
   oubliant les 4 corporations ; l'autre exigeait qu'un badge soit posé au moins
   aussi souvent qu'il est choisi, alors que le choix se fait **en main** et que
   toute carte en main n'est pas posée (540 poses pour 1220 choix est le rapport
   normal). Corrigés, puis les 3 contrôles cachés passent.

### Dette laissée par ce chantier

- **Plus aucun test ne prouve que `cards_effects_unhandled` sait encore
  compter** : il vaut 0 partout et tous les tests épinglent 0. Si son
  incrémentation était supprimée, rien ne virerait au rouge. [VÉRIFIÉ 29-07]
- Le badge joker n'est pas choisi **à la révélation depuis la pioche**, comme le
  prévoit le livret, mais à l'entrée en main. Écart assumé et documenté.
- Le badge n'est pas revu après une défausse suivie d'une repioche.

## L'HISTORIQUE (28-07 soir) — 194 cartes sur 208

**14 des 208 cartes projets de la boîte de base n'ont aucun encodage** (62 le
27-07, 29 après `moteur-cartes-5`, 18 après `moteur-cartes-6`, 14 après
`moteur-acier-titane`). En configuration cible `base,decouverte` : **199 / 246**,
donc **47 muettes**. [VÉRIFIÉ 28-07 par ma main après promotion]

**Et plus aucun prérequis imprimé ne manque** dans la boîte de base : le compte
est passé de 2 à **0**. [VÉRIFIÉ 28-07 par ma main]

Les 14 restantes réclament des mécanismes absents : cartes supplémentaires
jouées et réductions (5), phase de recherche modifiée (3), assouplissement de
prérequis (2), divers (4). Liste nominative :
`workspaces/moteur-acier-titane/inputs/checks/02-les-14-restantes.sh`.

**Ne JAMAIS citer le « 7 » de `docs/cartes/moteur-vs-imprime.md` comme une
couverture de la boîte de base** : ce rapport n'échantillonne que 66 cartes.

## LES 28 DERNIERS PROJETS DE DÉCOUVERTE (29-07) — `decouverte-projets`, audité OK et promu

**Il ne reste plus que 3 cartes projet muettes sur 246** : les trois badges
jokers. Les 28 autres agissent. [VÉRIFIÉ 29-07 par ma main, `--dump-deck`]

**765 tests verts** (716 avant), aucun désactivé. Empreinte de la boîte de base
**inchangée** (`cee020cda9db283b`), 0 violation d'invariant sur 1000 parties,
tous les compteurs neufs nuls en `--effects off`.

**Sept couleurs fausses corrigées.** J'ai trouvé, avant de sceller le contrat,
que `cards.json` classait **vertes** six cartes rouges (événements) et une
bleue : D05, D14, D16, D17, D18, D19, D20. La couleur décide de la phase de
pose, du fait que la carte reste en jeu ou parte à la défausse, et du décompte
des Récompenses. Corrigé dans la donnée elle-même. [VÉRIFIÉ 29-07]

### Modélisation : zéro catégorie d'effet neuve

19 catégories avant, **19 après**, pour 28 cartes de plus. La phase imposée est
un **paramètre** — `ResEff::PhaseUpgrade(Option<u8>)`, `None` = au choix,
`Some(phase)` = imposée (`effects.rs:548`) — et non trois cas particuliers dans
le flux. Un seul chemin d'octroi subsiste.

### Les quatre contradictions de l'agent — les quatre étaient justes [VÉRIFIÉ 29-07]

1. **Trois prérequis imprimés manquaient à mon contrat** : D12 « 3 badges
   science » (+ 1 PV), D17 « température jaune ou plus chaud », D19 « un
   Objectif ». **Mon erreur** : mon extraction des textes imprimés omettait le
   champ `reqs_fr` du fichier de transcription. Encodés et câblés dans
   `requirements_met` (`flow.rs:1222`).
2. **`cards_effects_unhandled` comptait faux en `--effects off`** : il valait
   750 alors qu'en squelette intégral AUCUN pouvoir n'est appliqué, pour les
   388 cartes — y désigner sept coupables, c'était compter « cartes sans entrée
   de table » en prétendant compter « pouvoirs sautés ». Corrigé. Remesuré par
   ma main : 565 en effets actifs (3 jokers + 4 corporations), **0** en
   `--effects off`.
3. **Un message d'échec de mon contrôle 08 disait « 29 catégories »** alors que
   le contrat et la mesure disent 19. Sans conséquence, mais trompeur.
4. **D23, D29 et D34 créent des savoir-faire** acier et titane, donc entrent
   dans la Récompense INDUSTRIEL. Vérifié : c'est la dérivation du lot
   acier-titane (`flow::capacities`, `flow.rs:362`) — toute réduction par badge
   portée par une carte **verte** dérive un savoir-faire. Cohérent avec les
   annotations du carton (D25 −4 bâtiment = acier ×2, donc −2 = acier ×1).

**Mes sept couleurs étaient justes** : première fois en sept chantiers qu'aucune
de mes mesures n'est prise en défaut. La seule erreur de ma part est l'omission
du champ `reqs_fr` — dont la leçon : le fichier de transcription porte aussi un
champ `name_en`, que j'aurais dû utiliser pour la correspondance des noms au
lieu de l'établir à la main.

### Ce que la relecture adversariale de l'agent a trouvé

Son sous-agent a relevé que l'invariant I2 (« affordabilité et paiement ne
divergent jamais ») n'était prouvé que du côté **paiement** : toutes les sondes
tournaient à 400 MC, donc `flow::affordable` n'était jamais mis en cause. Un
test à budget décisif (prix imprimé − 1 MC) a été ajouté, qui éprouve les quatre
réductions dans les deux sens.

## LE VERROU DE DÉCOUVERTE A SAUTÉ (28-07) — `decouverte-phases`, audité OK et promu

**Le mécanisme des cartes Phase améliorées existe et agit en partie réelle.**
Mesuré sur 1000 parties `base,decouverte` (graine 2024) [VÉRIFIÉ 28-07 par ma
main] : **497 améliorations accordées**, 23 bascules A↔B, 2065 bonus améliorés
appliqués, 558 poses supplémentaires, 3378 points distribués par VISIONNAIRE — et
surtout **`phase_upgrades_skipped` passe de 510 à 0**. Plus un seul « améliorez
une carte Phase » n'est sauté. Les 26 cartes projet de Découverte qui en
dépendaient sont maintenant encodables.

**716 tests verts** (640 avant), aucun désactivé. 1000/1000 dans les deux boîtes,
invariants 0. Tous les compteurs neufs **nuls en `--effects off`**. **Empreinte de
la boîte de base INCHANGÉE** (`cee020cda9db283b`) : rien de la base n'a bougé.

### La conception livrée — et pourquoi le cumul est désormais impossible

Trois pièces : les onze cartes Phase sont une **table de données**
(`effects::PHASE_BASE` / `PHASE_UPGRADED`) ; un **point de calcul unique**
(`flow::selector_bonus`, `flow.rs:2902`) que les cinq phases consomment ; les
poses supplémentaires de I-B, II-A et II-B empruntent le `BuildGrant` et la file
`pending_builds` du lot cartes-8 — **aucun second mécanisme**.

Le non-cumul n'est pas « absent », il est **inexprimable** : `selector_bonus` lit
**une seule** entrée de table, `PHASE_UPGRADED[phase][variante]` **ou**
`PHASE_BASE[phase]` (`flow.rs:2914-2917`). Les deux constantes de bonus
`DEV_SELECTOR_DISCOUNT` et `PRODUCTION_SELECTOR_MC` ne sont plus lues **nulle
part** dans `flow.rs`. [VÉRIFIÉ 28-07]

### QUATRE contrôles rouges, QUATRE erreurs de ma part [VÉRIFIÉ 28-07]

Le pire score de mes contrôles sur ce projet. Chacune vérifiée à la source avant
de conclure, comme la règle l'exige — et à chaque fois l'agent avait raison.

1. **Mon check 01 §5 contredisait mon propre done-when 05.** J'exigeais 33
   projets Découverte muets ET `skipped = 0`. Or *Cryogenic Shipment*
   (`effects.rs:1704`) et *Fibrous Composite Material* (`:1508`) étaient **déjà
   encodées** ; elles n'étaient déclarées non gérées que parce que leur
   `ResEff::PhaseUpgrade` comptait comme un effet sauté. Faire tomber `skipped`
   à 0 les rend mécaniquement gérées. **Les deux exigences ne peuvent pas être
   vraies ensemble.** Périmètre vérifié autrement : diff des tables d'encodage,
   **217 cartes avant, 217 après, aucune ajoutée ni retirée.**
2. **Je lisais un objet d'ANNONCE comme un objet d'APPLICATION.**
   `selector_bonus` agrège les branches par maximum (`flow.rs:2923-2931`) et
   porte un champ `alternative` ; la partie réelle n'en applique **qu'une**,
   tranchée par la politique (`flow::selector_branch`, `flow.rs:2955`). Le « ou »
   de II-B est correct, mon témoin y voyait un « et ».
3. **Mon témoin de IV-A tombait sur la seule carte où l'effet est invisible.**
   *Tall Station* : le bonus MC descend de 4 à 1 (−3) et sa production rejouée
   vaut exactement +3. Contre-mesuré sur deux autres cartes : *Power Supply
   Consortium* 11 → 10, *Mine* 9 → 6. La production est bien rejouée.
4. **Mon motif de recherche attrapait des COMPTEURS**, pas des grandeurs :
   `upgrad\w*\s*\+=` correspond à `phase_upgrades_granted += 1`. **Troisième
   fois** que je confonds un compteur avec ce qu'il compte (lot acier-titane,
   lot cartes-8, ici).

### Ce que l'agent a trouvé de son côté

Sa relecture adversariale a corrigé **deux vrais défauts** : le budget
d'activations de la phase III était copié dans une variable locale, laissant le
champ `extra_blue_activations` en **état mort** — une mutation qui jetait la
seconde activation de III-B passait les 714 tests ; et `visionary_award_points`
était recalculé par un second parcours au lieu de sortir du parcours de score.

Et un effet de bord que mon contrat n'avait pas vu : passer la réserve de
récompenses de 6 à 7 entrées **changeait l'empreinte de la boîte de base** (un
tableau de 7 consomme un tirage de mélange de plus). VISIONNAIRE n'entre donc
dans la réserve que là où le mécanisme peut jouer — Découverte **et** effets
actifs (`flow::award_pool`, `flow.rs:3468`). Le raisonnement tient : sans le
mécanisme, la tuile serait une égalité à zéro dans toutes les parties, le défaut
exact que COLLECTIONNEUR a traîné jusqu'à ce matin.

**Dette notée** : le livret fait des Récompenses **et** des Objectifs des modules
de Découverte ; le moteur les applique aussi en boîte de base. Approximation
antérieure à ce chantier, à trancher un jour.

## ~~EN COURS (28-07 nuit)~~ — `decouverte-phases`, contrat scellé et lancé

**Le verrou de l'extension Découverte.** 26 des 38 cartes projet de Découverte
reposent sur l'amélioration de carte Phase, mécanisme qui n'existe pas du tout :
`PlayerState::phase_upgrades` est un tableau que rien ne lit, et
`ResEff::PhaseUpgrade` se contente d'incrémenter un compteur de renoncement
(`phase_upgrades_skipped = 510` sur 1000 parties en `base,decouverte`,
mesuré le 28-07). Tant qu'il n'existe pas, les 26 cartes sont inencodables.

**Périmètre** : le mécanisme, les **dix** cartes Phase améliorées, et la
récompense **VISIONNAIRE** (7e tuile, absente du moteur). Les 26 cartes projet
et les 4 corporations de Découverte sont hors périmètre — chantier suivant.

### La bonne nouvelle : le vocabulaire existe déjà, à deux exceptions près

En transcrivant les dix bonus, j'ai constaté que **huit sur dix se disent avec
des briques déjà écrites** : `DEV_SELECTOR_DISCOUNT`, `extra_blue_activations`,
`PRODUCTION_SELECTOR_MC`, `ResearchBonus` (lot 4), `Reveal` (lot 6),
`ConstructionBonus`. Et surtout — le lot cartes-8 fini une heure plus tôt tombe
pile : **I-B dit « une seconde carte verte dont le coût imprimé est de 12 MC ou
moins »**, c'est-à-dire mot pour mot le `BuildGrant` que je venais de construire.
Seules deux moitiés sont neuves : le « ou » de II-B et le « rejouer la production
d'une carte verte » de IV-A. [VÉRIFIÉ 28-07]

### Une erreur de mon contrat, trouvée AVANT scellement

J'avais écrit que `--probe-phase` suffisait à démontrer les dix bonus. **Faux, et
mesuré** : la sonde appelle `build_card_with(…, discount = 0, …)` et n'exécute
aucune phase. `--probe-phase 1` laisse *Lichen* à 5 MC payés, remise du
sélectionneur comprise ou non ; `--probe-phase 5` ne change rien au champ
`research`. **Aucun bonus de sélectionneur n'est observable de l'extérieur
aujourd'hui.** J'ai remplacé l'interface par un objet `selector_bonus` que la
sonde doit rendre **tel que le service unique le calcule** — ce qui rend le
mécanisme observable ET force la conception exigée.

### Deux faux positifs de mes contrôles, attrapés au calibrage

1. Mon contrôle « une seule file de poses supplémentaires » comptait
   `extra_builds_granted` et `extra_builds_used` — des **compteurs**, pas des
   files. Le type fait désormais partie du critère (`Vec<BuildGrant>`).
2. Mon contrôle « le chantier a écrit ses propres tests » était **vert avant le
   chantier** : trois fichiers mentionnent déjà `phase_upgrades_skipped`. Le
   critère porte désormais sur des tests qui **installent** réellement une
   amélioration (`PhaseUpgrade::Variant`), ce que zéro test fait aujourd'hui.

### Et une erreur de MESURE, la plus bête de la journée

En vérifiant l'état initial des huit contrôles, je les ai tous vus à zéro — donc
tous verts, donc un scellement invalide. C'était ma commande qui mentait :
`echo "$(basename $f) : $?"` exécute `basename` **avant** de lire `$?`, et écrase
donc le code de retour à zéro. Même famille que le piège du tuyau
(`cmd | tail` puis `echo $?`) noté au lot 5. Remesuré proprement : **8 contrôles
rouges sur 8**. [VÉRIFIÉ 28-07]

**Bidirectionnalité prouvée** : 8 contrôles rouges pour la bonne raison (vérifié
sortie par sortie), sauf `07-non-regression.sh` vert dès aujourd'hui car c'est un
garde-fou (208 cartes de base enregistrées dans `inputs/sondes-reference.json`).
Trois hold-outs cachés rouges, dont les parties garde-fou (base intacte,
déterminisme) vertes dès aujourd'hui.

## LES DEUX DÉFAUTS DES TUILES SONT CORRIGÉS (28-07) — fait en direct

Les deux défauts trouvés en faisant le décompte d'avancement, réparés et
**verrouillés par des contrôles** (`engine/tests/tuiles_tests.rs`, 6 tests).

1. **BARON SPATIAL : seuil 7 → 6.** La tuile imprimée dit « 6 badges espace » ;
   le moteur en exigeait 7, chiffre venu du squelette et d'aucune source.
   `flow.rs:3063`. Le test vérifie les DEUX bords : rien à 5, acquis à 6.
2. **COLLECTIONNEUR ressuscitée.** « Le plus de ressources sur les cartes »
   renvoyait **0 pour tout le monde** depuis la création du squelette, alors que
   les ressources posées sur les cartes existent depuis le lot 3 : la récompense
   distribuait une égalité systématique dans toutes les parties où elle sortait.
   `flow.rs:3089`. Vérifiée dans les deux sens (le classement s'inverse quand les
   ressources s'inversent) ET sur cinq parties entières.

**Les onze seuils d'objectifs sont désormais épinglés un par un**, chacun
confronté au texte de sa tuile, chacun vérifié juste en dessous puis pile
dessus. Il y a maintenant un contrôle en face de chaque chiffre.

**Dette assumée et épinglée** : la 7e tuile de récompense, *VISIONNAIRE* (« le
plus de cartes Phase améliorées »), n'a toujours pas de variante dans le moteur —
elle ne peut pas en avoir tant que les améliorations de phase ne sont pas
implantées (`state.rs` `phase_upgrades` n'est lu nulle part). Un test épingle
l'écart 6 contre 7 et devra être RETOURNÉ le jour où la variante existera.

**640 tests verts**, 1000/1000 parties dans les deux boîtes, aucun invariant
violé. [VÉRIFIÉ 28-07]

## 🏁 LA BOÎTE DE BASE EST TERMINÉE (28-07) — `cartes-8`, fait EN DIRECT par le CTO

**Les 208 projets de la boîte de base sont encodés. Zéro carte muette.**
Mesuré deux fois, par deux chemins indépendants [VÉRIFIÉ 28-07] :
`--dump-deck --boites base` ne rend plus une seule carte à `effets_geres: false`,
et 1000 parties réelles donnent `cards_effects_unhandled = 0` — plus un seul
pouvoir imprimé n'est sauté en cours de partie.

**Décision d'Alexis** : ce lot a été fait **en direct, sans chantier séparé**,
sur sa demande explicite après que je lui aie recommandé l'inverse. Le compromis
est inscrit ici : correction évidente et localisée → en direct ; ajout qui touche
au déroulement d'une partie → chantier séparé. Ce lot tombait du second côté ;
Alexis a tranché autrement en connaissance de cause.

### Les cinq cartes et les deux mécanismes qu'elles ont imposés

*Asset Liquidation*, *Special Design*, *Work Crews* (phase II) ·
*Automated Factories*, *Tall Station* (phase I).

1. **La pose supplémentaire.** Le tour de jeu ne savait pas se rouvrir : chaque
   phase proposait une pose, une seule, et le seul cas de « deuxième pose » était
   écrit en dur pour le sélectionneur de phase. Généralisé par
   `effects::BuildGrant` (`effects.rs:846`) — couleurs autorisées, plafond de
   prix **imprimé**, gratuité — exercé par le seul `flow::drain_pending_builds`
   (`flow.rs:1246`). **Les poses ordinaires des phases I et II sont elles-mêmes
   des permissions** (`GRANT_DEVELOPMENT`, `GRANT_CONSTRUCTION`) : il n'existe
   plus qu'un seul chemin de pose dans tout le moteur (I1).
2. **L'effet à DURÉE.** Le moteur n'avait que du permanent et de l'instantané.
   *Work Crews* (« 11 MC de moins pour la **prochaine** carte de cette phase »)
   et *Special Design* (souplesse d'un palier, même portée) ont imposé un
   troisième genre : `effects::NextCardMod`, armé à la pose, consommé par la
   pose suivante, effacé en début de phase même s'il n'a jamais servi.

**Ajout de vocabulaire** : `ActionCost::Tr(n)`, premier coût en note de
terraformation du moteur (*Asset Liquidation* : « Spend 1 TR to draw three
cards »). Il emprunte le service unique `PlayerState::spend_tr`, donc il est
compté par l'invariant du NT.

### Un défaut de la sonde, trouvé et corrigé au passage [VÉRIFIÉ 28-07]

`probe.rs` recalculait le prix payé pour son compte, à partir des seules
réductions permanentes — dette connue depuis le lot corporations. La réduction
armée par *Work Crews* lui échappait donc : `paid` mentait, et son garde-fou de
payabilité aurait refusé une carte que la partie réelle propose. Corrigé
(`probe.rs:735`) : la sonde lit le même service que le paiement (I2).

### Les chiffres, mesurés

- **634 tests verts** (599 avant), aucun désactivé — dont **35 neufs**
  (`tests/lot8_tests.rs`), chaque mécanisme vérifié dans les DEUX sens.
- **203 cartes hors lot sondées avant/après : aucune n'a bougé** (comparaison au
  binaire du lot 7, champ par champ).
- 1000 parties menées à terme dans les deux boîtes, `invariant_violations = 0`,
  `truncated = 0`. Empreinte base : `13dd0cfeb7532dde` → `1edd85ff035a8767`
  (le déroulement a bien changé), déterministe au rejeu.
- Compteurs neufs en 1000 parties (boîte de base) : 1042 permissions accordées,
  **432 exercées**, 94 cartes posées sans payer, 462 modificateurs armés dont
  **250 consommés**. Tous **nuls en `--effects off`** (I7).
- ~8 850 parties/s : aucune dégradation.

### Onze attentes de tests retournées, aucune affaiblie

Le succès du lot a rendu faux tous les témoins qui disaient « il reste des cartes
muettes en boîte de base ». Chacun a été **retourné et rendu plus exigeant**,
jamais neutralisé — par exemple `le_compteur_grossit_quand_la_pioche_s_elargit`
(`lot_boites_tests.rs`) affirmait « la base a des cartes muettes » ; il affirme
désormais « la base n'en a **aucune** », ce qui est strictement plus fort. Deux
témoins nommant des cartes de base devenues encodées ont été reportés sur des
cartes de Découverte, toujours muettes.

## NEUF CARTES DE PLUS (28-07) — `moteur-cartes-7`, audité OK et promu

**Résultat mesuré après promotion** [VÉRIFIÉ 28-07] : muettes **14 → 5** en boîte
de base (203/208 encodées), **47 → 38** en `base,decouverte`. **599 tests verts**
(509 avant), aucun désactivé. 1000/1000 parties menées à terme,
`invariant_violations = 0`, empreinte `13dd0cfeb7532dde` (graine 2024, base).
Compteurs : `standard_action_discounts = 1500`, `action_mc_bonuses = 1578`,
`research_extra_draws` 3 888 → **9 467**, `cards_effects_unhandled` 3 154 →
**1 054**. Vitesse ~8 500 parties/s (le lot coûte ~12 %).

### DEUX FAUX POSITIFS DE MES CONTRÔLES — la leçon se répète [VÉRIFIÉ 28-07]

Les deux hold-outs rouges étaient **mes** erreurs, pas celles de l'agent.
Vérification faite à la source avant de conclure, comme la règle l'exige.

1. **Hold-out 01, point 4 — *Mars University*.** Mon témoin exigeait que
   `delta.hand` bouge autrement que de −1. Le texte imprimé
   (`textes-cartes.json`) dit : « vous **pouvez défausser une carte** ; si elle
   portait un badge plante, **piochez-en deux**, sinon **piochez-en une** ».
   Une défausse suivie d'une pioche fait un bilan **net nul** sur la main :
   −1 est donc la valeur correcte, et mon témoin ne pouvait rien distinguer.
   L'effet est réellement présent : `effects.rs:1976`
   (`TrigGain::MayDiscardDraw`, `include_self: true`) et trois tests le prouvent
   branche par branche (`lot7_tests.rs:1241-1300`).
2. **Hold-out 02 — le taux de défausse.** Mon commentaire disait « deux lectures
   au plus peuvent subsister », mon code testait « une au plus ». Les deux qui
   restent sont **la définition du service unique lui-même**,
   `flow::discard_mc_rate` (`flow.rs:1104-1114`), appelé aux quatre sites
   (`flow.rs:1200, 1566, 2729, 3055`) plus la politique. I1 est respecté.

### MON ASK N°4 ÉTAIT FAUX — l'agent m'a corrigé, livret en main

Je supposais que la défausse de fin de manche « ne rapporte rien » et sortait
donc du texte de *Composting Factory*. Le livret dit l'inverse, deux fois mot
pour mot : `docs/regles/livret-base.md` **l. 437 et l. 654** — « Pour chaque
carte ainsi défaussée, le joueur gagne 3 MC, **comme toujours** », renvoyant à
la règle générale l. 96. Vérifié par ma main. *Composting Factory* couvre bien
les **quatre** sites.

### UN CONTRÔLE VISIBLE CREUX, SIGNALÉ PAR L'AGENT AU LIEU D'ÊTRE EXPLOITÉ

Mon check scellé `08-rapport.sh:25` écrit ses deux motifs en syntaxe simple
(`recherche\|research`) mais les passe à `grep -E`, qui y voit une barre
verticale **littérale**. Mesuré : `printf 'la recherche\n' | grep -qiE
"recherche\|research"` ne trouve rien. Le contrôle cherchait la chaîne
`recherche|research`, pas les mots. **C'est le deuxième contrôle visible creux
que j'écris** (le premier au lot acier-titane, une corporation mal orthographiée
qui rendait `found=false`). Règle à appliquer désormais : *tout motif de
recherche textuel doit être prouvé sur un exemple positif ET un exemple négatif
avant scellement.*

### Un changement de sémantique assumé et vérifié

L'agent a déplacé le relevé de `prereq_ok` de la sonde : il se faisait sur
**l'état de départ**, il se fait désormais **juste avant la pose de la dernière
carte**, comme `flow::affordable` en partie réelle. Sans ce déplacement,
l'interface que j'imposais était inatteignable. Vérifié : sur une sonde à une
seule carte — mes 237 références de non-régression — la valeur est **inchangée**
(check `07-non-regression.sh` vert).

### Question laissée ouverte par l'agent, à arbitrer plus tard

La commande littérale de mon interface n°3 rend `delta.plants = 0` pour
*Restructured Resources*, parce que la politique aléatoire **décline** le
« vous pouvez ». Le chemin existe et se voit avec `--probe-choice 0` (mesuré :
`delta.plants = -1`). Ce n'est pas un défaut : câbler ce choix violerait mon
propre interdit n°4 (les choix appartiennent à la politique, pas au moteur).

## ~~EN COURS (28-07 soir)~~ — `moteur-cartes-7`, contrat scellé et lancé

**Découpage décidé par moi** : les 14 muettes ne font pas un lot, elles font
deux. Ce lot en prend **9**, celles qui modifient un chemin déjà existant
(« modificateurs permanents ») ; les **5** autres partagent le seul mécanisme
vraiment neuf — « jouer une carte de plus dans cette phase » — et feront le lot
suivant. Cible : 14 → **5** muettes en base, 47 → 38 en `base,decouverte`.

Les neuf : *Interns*, *Extended Resources*, *United Planetary Alliance*
(recherche) · *Composting Factory*, *Standard Technology*,
*Restructured Resources* (prix) · *Adaptation Technology*, *Assembly Lines*,
*Mars University* (déclencheurs).

**Le lot est plus abordable qu'il n'y paraît** : 5 des 9 réutilisent un
mécanisme déjà écrit. `ResearchBonus` existe (lot 4, *Interplanetary
Relations*) ; l'assouplissement de prérequis existe (`req_color_flex`
d'*Inventrix*) ; la réduction payée en ressource existe (`PayResources`,
*Anaerobic Microorganisms*). [VÉRIFIÉ 28-07]

### Cinq erreurs de mon contrat, trouvées AVANT scellement

C'est le point important de ce cadrage. En calibrant, j'ai corrigé :

1. **`research_extra_draws` valait 3 888, pas 4 266.** Le chiffre de la carte
   d'état datait du 27-07 et était devenu faux. J'allais le sceller.
2. **`--effects off` rend `prereq_ok = true` PARTOUT** (la couche d'effets étant
   coupée, les prérequis ne s'appliquent plus). Ma ligne de contrôle exigeait
   `false` : elle aurait échoué quoi que fasse l'agent. Remplacée par deux
   témoins réellement discriminants (*Great Dam* pour les océans, *Fusion Power*
   pour les badges — la souplesse ne doit toucher ni l'un ni l'autre).
3. **Le surplus rendu d'un paiement par défausse se REPORTE sur le paiement
   suivant.** Mon arithmétique donnait −16, la mesure donne **−15**.
4. **`delta.mc` de `--probe` ne contient PAS le prix de la carte** (le prix vit
   dans `paid[]`). Mon témoin d'*Assembly Lines* attendait −24 ; la bonne valeur
   est **−11** contre −12 aujourd'hui.
5. **Mon hold-out comptait « illisible » comme un succès** : quand l'option de
   sonde n'existe pas, la sortie est vide et ma condition passait au vert. Garde
   ajoutée : seul un entier non nul compte.

### Deux choses ajoutées parce que la preuve était impossible sans elles

- **`--probe-plants <n>`** imposée à l'interface : sans plante, *Restructured
  Resources* est improuvable de l'extérieur. Leçon du lot précédent, où j'avais
  failli sceller une preuve impossible.
- **Une question ASK sur le périmètre de « discard for MC »** : la constante
  `SELL_CARD_MC` est lue à **quatre** endroits de `flow.rs` (affordabilité,
  paiement à la pose, vente de carte, défausse de fin de tour) et le texte
  imprimé ne dit pas lesquels il vise. L'agent doit trancher en le déclarant,
  pas en silence. [VÉRIFIÉ 28-07]

**Bidirectionnalité prouvée** : 7 contrôles rouges pour la bonne raison (vérifié
sortie par sortie), le 8e vert dès aujourd'hui car c'est un garde-fou de
non-régression (237 cartes hors périmètre enregistrées dans
`inputs/sondes-reference.json`). Les deux contrôles les plus risqués (03
recherche, 05 compteurs) ont été passés au vert contre un **faux moteur**
simulant l'état-cible. Trois hold-outs cachés rouges, dont les parties
garde-fou (40 témoins inchangés, déterminisme) sont vertes dès aujourd'hui.

## DÉCISIONS D'ALEXIS DU 28-07

- **Les règles maison ne sont PAS traitées pour le moment.** Aucun chantier ne
  s'ouvre dessus ; ne pas relancer Alexis sur le sujet. À noter : deux règles
  maison sont **déjà** dans le moteur depuis `moteur-conformite-1` (24-07) —
  alternance J1/J2 action par action en phase III, et égalité sèche. Cette
  décision ne les défait pas, elle interdit d'en ajouter d'autres pour l'instant.
  [VÉRIFIÉ — son message du 28-07]
- **Point de règle « phase Action : toutes les cartes ou seulement les bleues ? »
  clos comme SANS OBJET**, par mesure et non par arbitrage : sur les 242 entrées
  de `textes-cartes.json`, les **38** cartes portant « Action: » sont **toutes
  bleues**. Les deux lectures du livret donnent le même jeu. Détail :
  `docs/regles/notes/cas-tranches.md`. [VÉRIFIÉ 28-07]

## ÉTAT D'AVANCEMENT MESURÉ (28-07 soir) — réponse à « on en est où ? »

Mesuré par `--dump-deck` et lecture du code, pas de mémoire. [VÉRIFIÉ 28-07]

| Brique | État |
|---|---|
| Déroulement d'une partie (phases I-V, production, score, fin) | fait |
| Projets boîte de base | **208 / 208** encodés — **BOÎTE DE BASE TERMINÉE** |
| Corporations boîte de base | 12 / 12 |
| Projets en configuration cible `base,decouverte` | **246 / 246**, zéro muette [VÉRIFIÉ 29-07 soir] |
| Corporations Découverte | **4 / 4** encodées [VÉRIFIÉ 29-07 soir] |
| Objectifs (tuiles) | **11 / 11 encodés, 11 seuils vérifiés à la tuile** |
| Récompenses (tuiles) | **7 / 7** fonctionnelles |
| Cartes Phase améliorées | **10 / 10 appliquées** (chantier `decouverte-phases`, 28-07) |
| Badges jokers de Découverte | **3 / 3 implantés** [VÉRIFIÉ 29-07 soir] |
| Interface de jeu | rien |
| IA | rien — **c'est le prochain chantier, et le dernier grand** |

### Deux défauts trouvés en faisant ce décompte [VÉRIFIÉ 28-07]

- **Objectif BARON SPATIAL : seuil faux.** La tuile imprimée dit **6 badges
  espace** (`data/cartes-imprimees/objectifs-recompenses/objectifs-recompenses.json`,
  lue à la photo le 27-07) ; `flow.rs:2254` exige **7**. Les dix autres seuils
  concordent exactement avec les tuiles.
- **Récompense VISIONNAIRE absente du moteur** (« le plus de cartes Phase
  améliorées ») : `AwardKind` (`state.rs:116`) n'a que **6 variantes** pour
  **7 tuiles imprimées**. C'est le conflit « 7 awards Discovery vs 6 dans le
  Java » noté au squelette le 24-07, tranché ici : **le carton dit 7**. La
  septième dépend de l'amélioration de phase, donc du même chantier.
- Rappel : `AwardKind::Collector => 0` (`flow.rs:2281`) est encore un stub
  **alors que les ressources posées sur les cartes existent depuis
  `moteur-cartes-3`**. Deuxième récompense morte, réparable en une ligne.

## L'ACIER ET LE TITANE EXISTENT (28-07) — `moteur-acier-titane`, audité OK et promu

**Mesuré par ma main après promotion** : **509 tests verts**, 1 000 parties graine
2024 en `base,decouverte` → `completed: 1000`, `invariant_violations: 0`,
`truncated: 0`, empreinte `162e50432a84a517`. Graine inédite 616161 sur
800 parties : 800/800, 0 violation. **Muettes 18 → 14** (base) et **51 → 47**
(`base,decouverte`), exactement la cible du contrat. `cards_effects_unhandled`
en base : 4 084 → **3 154** (−22 %). [VÉRIFIÉ 28-07]

- **Le compte est DÉRIVÉ, jamais ressaisi.** `flow::capacities` lit les
  `Reduction::Tag(Building|Space, n)` déjà encodées sur les cartes **vertes** en
  jeu et sur la corporation, et divise par le taux du livret porté en un seul
  endroit (`effects::Capacity`, `capacity_units`). `steel_capacity` /
  `titanium_capacity` deviennent vrais mais comme **cache** : seule écriture
  `flow::refresh_capacities`, et `sim::check_invariants` recompare le cache à la
  dérivation **à chaque manche de chaque partie** — 2 000 parties à 0 violation
  prouvent qu'ils ne divergent pas. [VÉRIFIÉ 28-07 par lecture du code]
- Garde I3 réelle : `capacity_units` **panique** si un montant n'est pas un
  multiple exact du taux, plutôt que d'arrondir en silence ; le garde-fou de
  `CardsDb::load` rend le cas impossible en amont. [VÉRIFIÉ 28-07]
- Briques neuves : `Reduction::PerCapacity` (résolue **au paiement**, rien de figé
  à la pose) et `ActionCost::McPerCapacity` ; `ActionEff::Ocean`/`Forest`
  empruntent les chemins uniques `reveal_ocean` / `gain_forest`.
- **La récompense *Industrialist* n'est plus morte** : elle compte désormais une
  grandeur réellement disputée. L'agent ne l'a pas retouchée (ASK 6), il l'a
  déclarée. Conforme au carton.

### Les TROIS contradictions de l'agent, toutes exactes [VÉRIFIÉ 28-07 par ma main]

1. **Mon census « 27 réductions / 21 / 6 » était mal libellé.** Mesure réelle :
   **27** entrées `Reduction::Tag(Building|Space, …)`, dont **23 portées par des
   cartes projet et 4 par des corporations**. Ma phrase « les 21 sont toutes
   vertes ou corporations à encart gris » mélangeait deux populations. La
   conclusion tient, le décompte non.
2. **Mon check `03-le-compte.sh` testait une corporation jamais installée** : il
   écrivait `--probe-corp CrediCor` alors que le nom canonique est `Credicor`.
   Vérifié : `corp.found = false`, la sonde se déroulait **sans corporation**. Le
   contrôle passait pour une mauvaise raison. L'agent l'a remplacé dans sa
   couverture par un test installant réellement les 8 corporations sans
   savoir-faire.
3. **Deux `notes` de `textes-cartes.json` contredisaient leur `text`** :
   *Aquifer Pumping* (« -2 / [building] ») et *Solarpunk* (« -2 / [event] » — un
   badge que la carte ne porte même pas, ses badges sont space et plant). Le
   moteur a suivi `text`, comme ordonné. **Les deux notes sont corrigées à la
   source le 28-07**, dans `data/cartes-imprimees/` ET dans la copie
   `engine/data/`, avec mention explicite de la correction.

### Mon hold-out 02 : deux FAUX POSITIFS, vérifiés à la source

Il criait « 5 endroits divisent par 2 ou 3 » — les cinq sont des lignes de
**commentaire** — et « la couleur n'apparaît pas près du calcul », alors que
`flow::capacities` contient littéralement `if card.color != Color::Green
{ continue; }`. Détecteur trop grossier, fenêtre de recherche trop étroite.

**LE COMPTE À TENIR : mes contrôles cachés se sont trompés aux QUATRE derniers
lots, et l'agent avait raison à chaque fois. S'y ajoute désormais un contrôle
VISIBLE faux (le `CrediCor` ci-dessus) — un contrôle vert qui ne testait rien.**
Conséquence à appliquer : **tout contrôle qui installe une corporation ou nomme
une carte doit d'abord prouver que la sonde l'a TROUVÉE** (`corp.found` /
`found`), avant de juger la moindre valeur.

## ~~EN COURS (28-07)~~ — `moteur-acier-titane`, contrat scellé et lancé

Encode les **4 cartes muettes qui parlent d'acier ou de titane** (*Advanced
Alloys*, *Aquifer Pumping*, *Solarpunk*, *Water Import from Europa*) plus
l'effet manquant de la corporation *PhoboLog*. Après lui : **14** muettes.

**La trouvaille qui débloque le chantier** : aucune de nos sources de données ne
dit combien d'aciers ou de titanes une carte donne — mais le compte est
**dérivable** de ce que le moteur encode déjà. Chaque acier vaut 2 MC de
réduction sur les cartes bâtiment, chaque titane 3 MC sur les cartes espace, et
les 21 réductions `Reduction::Tag(Building|Space, n)` du moteur sont **toutes**
des multiples exacts de 2 et de 3. [VÉRIFIÉ 28-07 par ma main]

Trois vérifications indépendantes, faites AVANT le scellement :
1. **À l'image** (`data/scans/base/img_917b063334cb.png`, planche CORP) : le
   savoir-faire se reconnaît à un encart **gris hachuré** (icône acier = outils
   bruns ; titane = étoile jaune). *Mining Guild* et *Interplanetary Cinematics*
   portent 1 acier ; *PhoboLog* et *Saturn Systems* 1 titane. *CrediCor* et
   *ThorGate* n'en ont pas : leurs réductions vivent dans l'encart **rose**.
2. **Transcription Découverte** : D25 « Savoir-faire acier ×2 » (réduction 4),
   D31 « Savoir-faire titane ×2 » (réduction 6), D34 « ×1 » (réduction 3).
3. **Contre-épreuve** sur les 27 réductions encodées : les 21 qui portent sur
   bâtiment ou espace sont toutes vertes ou corporations à encart gris ; les 6
   autres (n'importe quelle carte, événement, énergie, Terre, Jupiter, prix
   minimum, microbes) n'en sont pas.

`engine/src/state.rs:162-163` : `steel_capacity` / `titanium_capacity` existaient
en **stub figé à 0** depuis leur création, lus uniquement par la récompense
*Industrialist* (`flow.rs:2283`) — qui comptait donc toujours zéro pour tout le
monde. [VÉRIFIÉ 28-07]

## 11 CARTES DE PLUS (28-07) — `moteur-cartes-6`, audité OK et promu

- Actions bleues et manipulation de la main : bonus « si vous avez choisi la
  phase Action », coûts payés en défaussant, « dépensez jusqu'à n », piocher
  puis défausser, révéler le dessus de la pioche, prérequis d'oxygène maximum.
  `effects::LOT1` : 188 → 199 entrées. **453 tests verts**, 0 violation,
  0 partie tronquée. [VÉRIFIÉ 28-07 par ma main]
- `cards_effects_unhandled` en boîte de base : **6 706 → 4 084** sur 1000
  parties. [VÉRIFIÉ 28-07]
- Nouvelle option de sonde imposée au contrat : **`--probe-phase <1..5>`**, qui
  fixe la phase choisie par le joueur sondé. Sans elle, un bonus conditionnel à
  la phase n'était pas prouvable de l'extérieur. [VÉRIFIÉ 28-07]
- **L'agent a trouvé seul une infidélité au texte imprimé** : *Invention Contest*
  dit « Keep **one of them** » — le texte compte les cartes GARDÉES, pas les
  défaussées. Pioche épuisée, le premier encodage n'aurait rien laissé au joueur.
  Corrigé, avec un test qui échoue sur l'ancien code. [DÉCLARÉ par l'agent,
  code relu par ma main : `flow.rs`, branche `Eff::DrawDiscard`]
- **Non-régression prouvée par oracle disjoint** : les 262 cartes hors périmètre
  sondées sur les deux binaires, **0 différence**. [DÉCLARÉ par l'agent]
- Deux arbitrages assumés : `ActionEff::Heat`/`Temperature` jugés mécaniques et
  non « septième brique » ; `Action::SpendUpTo` n'offre pas le montant 0, ce qui
  est sans conséquence de jeu puisque ne pas activer l'action est déjà possible.
  [VÉRIFIÉ 28-07 par lecture du code]

## ⚠️ LEÇON CTO RÉPÉTÉE TROIS FOIS (25 au 28-07)

**Mes témoins cachés se sont trompés à chaque lot, et l'agent avait raison à
chaque fois** : témoin de planche faux (`moteur-verite-1`), bonus de tuile océan
ignoré (`moteur-cartes-5`), main vide de la sonde et cartes sans effet à la pose
(`moteur-cartes-6`). Cause commune : **j'écris les valeurs attendues sans
exécuter le chemin réel sur une carte déjà gérée.**

Règle à appliquer désormais : **tout témoin caché doit être calibré en
l'exécutant sur une carte du même genre déjà encodée, AVANT le scellement.**
Ce que j'ai fait pour la sémantique des deltas, jamais pour l'état de départ de
la sonde (main vide, température violette, effet à la pose ou en action).

## 33 CARTES RENDUES VIVANTES (28-07) — `moteur-cartes-5`, audité OK et promu

- **20 productions, 9 effets immédiats, 4 gains de forêt** encodés depuis le
  texte imprimé. `effects::LOT1` passe de 155 à 188 entrées. **396 tests verts**,
  0 violation d'invariant, 0 partie tronquée sur 1000 parties graine 2024.
  [VÉRIFIÉ 28-07 par ma main]
- `cards_effects_unhandled` en boîte de base : **14 037 → 6 706** sur 1000
  parties (−52 %). [VÉRIFIÉ 28-07]
- **Deux règles de forêt tranchées avant le seal, confirmées par le livret** :
  (R1) « gagnez 1 PV forêt **et** +1 oxygène » décrit ce que fait la forêt, il ne
  s'y ajoute pas — *Plantation* donne 2 forêts et 2 pas d'oxygène, jamais 4 ; le
  livret p.14 l.379 emploie la formule exacte pour l'action standard, à un seul
  pas. (R2) le gain de forêt **déclenche** *Small Animals*, qui imprime « When
  you **gain a forest VP** » — `cards.json` écrit « **Build** a forest », et
  c'est le verbe qui décide. [VÉRIFIÉ 28-07]
- **Chemin unique** : `flow::gain_forest` est la seule écriture de
  `PlayerState::forests` du moteur ; l'action standard payante paie puis appelle
  la même fonction. [VÉRIFIÉ 28-07, hold-out 02]
- **Divergence déclarée par l'agent et vérifiée** : mon contrat affirmait que le
  vocabulaire des prérequis suffisait. Faux — *Energy Storage* porte « Requires
  you to have 7 or more TR » dans le champ **`requirement`**, que je n'avais pas
  lu (je n'avais lu que `text`). `Req::TrMin` ajouté. **Leçon : une carte a
  plusieurs champs de texte, les lire tous.** [VÉRIFIÉ 28-07]
- Réserve consignée : *Quantum Extractor* porte `phase: "I-II"` dans la
  transcription alors que son `text` décrit une production. Le moteur ne lit
  nulle part ce champ ; l'encodage suit le texte. Risque résiduel dans la
  donnée, pas dans le code. [DÉCLARÉ par l'agent]

## LA PIOCHE EST ASSAINIE (27-07) — `moteur-boites-1`, audité OK et promu

- **Point unique de composition : `engine/src/boites.rs`.** L'appartenance de
  boîte vient des planches physiques (`engine/data/textes-cartes.json`, copie
  verbatim des transcriptions), critère POSITIF : une carte entre parce qu'une
  planche la nomme. Le drapeau `in_deck_v1` de `cards.json` ne décide plus rien.
  [VÉRIFIÉ 27-07]
- Option `--boites base|promo|decouverte` (défaut : `base`), recensement
  `--dump-deck` (une ligne JSON par carte : `name`, `kind`, `boite`, `planche`,
  `effets_geres`), compteur de fin de simulation `cards_effects_unhandled`.
- **Composition mesurée après promotion** : `base` 208/12 · `base,promo` 219/12 ·
  `base,decouverte` **246/16** · tout 257/16. **336 tests verts.**
  [VÉRIFIÉ 27-07 par ma main]
- Les 2 cartes qui n'existent sur aucune planche (*Microbiology Patents*,
  *Project Inspection*) ne sont plus distribuées. `phase_upgrades_skipped` tombe
  à 0 en boîte de base (il valait 599 sur 1000 parties avant). [VÉRIFIÉ 27-07]
- Réserves consignées par l'agent, non traitées : `--effects off` ne change ni
  `effets_geres` ni `cards_effects_unhandled` (ils décrivent la table, pas le
  réglage) ; les combinaisons sans `base` sont acceptées sans être testées ;
  *Microbiology Patents* reste encodée dans `LOT1` sans être distribuée ; le
  garde-fou de doublons ne regarde qu'à l'intérieur d'une même boîte.
  [DÉCLARÉ par l'agent — `workspaces/moteur-boites-1/outputs/boites.md` §5]

## EXTENSION DÉCOUVERTE — transcrite, décidée, pas encore implantée

- **Décision d'Alexis (27-07) : Découverte se joue EN ENTIER**, les quatre
  modules (Objectifs, Récompenses, cartes Phase améliorées, badges jokers).
  **Configuration cible de l'entraînement : `--boites base,decouverte`.**
  [VÉRIFIÉ — son message du jour]
- **Cartes promotionnelles : NON possédées.** Les planches `PROMO`/`PROMOCORP`
  viennent de l'adaptation Tabletop Simulator, pas de la boîte d'Alexis, et
  forment le pack Kickstarter 2021 dont l'absence est tranchée depuis le 24-07.
  `--boites promo` existe et est testé, mais ne correspond à aucune partie
  réelle. [VÉRIFIÉ 27-07]
- Sources physiques transcrites et promues dans `data/cartes-imprimees/` :
  `corporations-discovery/` (4), `projets-decouverte/` (38 entrées, **toutes
  lues à l'image**), `objectifs-recompenses/` (11 objectifs, 7 récompenses),
  `phases-ameliorees/`. [VÉRIFIÉ 28-07]
- **`D37` = « Production de Perfluorocarbone » — VÉRIFIÉE À L'IMAGE.** Elle
  manquait au scan du 27-07 et n'était déduite que par élimination ; Alexis a
  fourni son scan le 28-07 (`data/cartes-imprimees/projets-decouverte/
  scan-D37-28-07.pdf`). Coût 10, verte, badge bâtiment unique, « Améliorez votre
  carte Phase I. », production 1 chaleur, encart IV, marqueur U. La déduction
  était juste. Une seule correction : `effect_phases` portait `I` (la phase
  **améliorée**) alors que ce champ désigne le chiffre de l'encart, donc `IV`
  comme toutes les cartes de production. [VÉRIFIÉ 28-07]
- **Écart carton / `cards.json`** : le carton de *Sultira* dit « y compris
  celui-ci » (2 chaleurs dès la mise en place), `cards.json` omet la clause. Le
  carton fait foi. [VÉRIFIÉ 27-07]
- **Aucun effet de Découverte n'est implanté.** Les 4 corporations et les
  38 projets entrent en jeu en stub et sont comptés dans
  `cards_effects_unhandled`. [VÉRIFIÉ 27-07]

## LE MOTEUR EST FIABLE (27-07) — `moteur-verite-1`, audité OK et promu

**La question qui bloquait le projet est tranchée : le moteur n'a PAS hérité en
masse des erreurs de la paraphrase.**

- Périmètre : les **66 cartes** nommées en §G1/§G2 de `docs/cartes/divergences.md`.
  Résultat : **35 encodées, dont 33 CONFORMES au texte imprimé et 2 fausses
  (corrigées)** ; les 31 autres ne sont pas encodées du tout.
  Rapport complet : `docs/cartes/moteur-vs-imprime.md`. [VÉRIFIÉ 27-07]
- **Le régime `Action:` était déjà bon** — les 4 cartes concernées sont prouvées
  **répétables par le flux réel** `play_round` (deux activations dans la même
  partie), avec un test nommé chacune. C'était le risque n° 1 : il n'existe pas.
  [VÉRIFIÉ 27-07]
- **283 tests verts** (271 + 12), 1 000 parties graine 2024, 0 violation,
  0 tronquée, déterministe, effets OFF neutre, **11 377 parties/s**.
  Re-mesuré par la main après promotion : 283 tests, 1000/1000, 0 violation.
  [VÉRIFIÉ 27-07]

### Le défaut corrigé, et sa cause profonde

*Viral Enhancers* et *Decomposers* résolvaient leur effet déclenché **une seule
fois**, quelle que soit la carte jouée. Le livret dit l'inverse
(`docs/regles/livret-base.md:106`) : « Si la condition d'un effet est remplie
plusieurs fois lorsqu'une carte est jouée, résolvez l'effet correspondant
plusieurs fois. » Le moteur appliquait ce principe partout **sauf** pour la
variante « … ou … » (`TrigGain::Choose`), câblée à une résolution unique **en
suivant le moteur Java, pas le carton** — l'inversion d'oracle exacte que ce
chantier existe pour corriger.

**Vérifié par ma main, test A/B contre le binaire d'avant** : sur
`--probe "Decomposers;Adapted Lichen"` (badges microbe ET plante), avant = 0
microbe sur Decomposers, après = 1 microbe. L'effet est désormais résolu deux
fois. [VÉRIFIÉ 27-07]

**Cause profonde à traiter** : cette clause du livret est **absente de
`docs/regles/notes/regles-condensees.md`**. Tant qu'elle n'y est pas, l'erreur se
reproduira. [VÉRIFIÉ 27-07]

### Mes contre-vérifications indépendantes (sondes rejouées moi-même)

- *Windmills*, motif « including this » : `--probe-produce` donne
  `derived_prod.heat = 1` **avec la carte seule en jeu** — elle compte bien son
  propre badge Énergie. [VÉRIFIÉ 27-07]
- *Earth Catapult*, régime `Effect:` permanent : *Media Group* coûte **11** seule
  et **9** jouée après — la réduction s'applique à une carte posée **ensuite**.
  C'est la preuve de régime exigée par le contrat. [VÉRIFIÉ 27-07]

### Trouvaille non demandée, réelle et grave

**`Oxidation Byproducts` est irrécupérable en l'état.** Sa description dans
`cards.json` est « During the production phase, this produces 2 **руфе**. » — le
mot désignant la **ressource produite** est détruit par la corruption cyrillique.
La carte est `in_deck_v1: true` et **absente de `textes-cartes.json`** (jamais
imprimée sur les planches). Le moteur ne peut pas savoir ce qu'elle produit.
Homoglyphes : **18 entrées** de `cards.json` au total, 17 dans la pioche v1,
16 dans la pioche de base. [VÉRIFIÉ 27-07]

### Réserves consignées (aucune bloquante)

- **Défaut de l'outil d'audit, signalé et non corrigé** : `probe.rs` recalcule le
  prix pour son compte, donc le champ `paid[]` de la sonde **ment** quand une
  réduction payée en microbes s'applique. Le moteur lui-même est correct (prouvé
  par `delta.mc`). **À traiter : cela affecte la fiabilité de mes propres
  audits.** [DÉCLARÉ par l'agent, plausible]
- *Interplanetary Conference* : verdict `CONFORME` **contingent** d'un arbitrage
  d'ambiguïté déclaré dans `blocked.md` (lecture conservatrice, argumentée au
  livret). [VÉRIFIÉ 27-07]
- **Le lot suivant coûtera plus cher que prévu** : reclassement honnête après
  relecture adversariale en **7 ABSENT / 24 HORS-PORTEE** (au lieu de 13/18),
  l'agent ayant constaté que son propre rapport se contredisait — la structure
  `Corporation` n'a **aucun champ d'effet**. Les 12 corporations ne sont pas
  muettes par oubli : la table n'existe pas pour elles. [DÉCLARÉ, cohérent]
- **Mon hold-out 01 était fautif** : témoin *Comet* choisi hors périmètre, et
  attendu `ABSENT` pour *Hydro-Electric Energy* là où `HORS-PORTEE` est mieux
  justifié — vérifié à la source, `ActionEff` (`effects.rs:394`) n'a **aucune
  variante Heat**, l'action imprimée est littéralement inexprimable.
  [VÉRIFIÉ 27-07]

## LES CORPORATIONS SONT VIVANTES (27-07) — `moteur-corporations-1`, audité OK et promu

**Les 12 corporations de la boîte de base appliquent leurs pouvoirs.** 10 verdicts
`ENCODÉE`, 2 `PARTIELLE` (*Phobolog* et *Mining Guild* — titane et acier non
modélisés, cadrage que j'avais tranché avant le lot).
Rapport : `docs/cartes/corporations.md`. [VÉRIFIÉ 27-07]

### Mesuré par ma main après promotion

- **317 tests verts** (283 + 34 neufs), 0 échec. 5 tests existants renforcés
  — la limite contractuelle exacte —, **aucun supprimé** (221 → 255 fonctions).
- 1 000 parties graine 2024 : `completed: 1000`, `invariant_violations: 0`,
  `truncated: 0`, empreinte `21c7cdd6a342ca0c` **identique sur deux exécutions**.
- Débit **7 404 à 8 422 parties/s** contre 7 400 à 8 900 avant le lot : aucune
  régression de vitesse malgré tous les mécanismes ajoutés.
- `--dump-corporations` rend **exactement les 12** corporations de la boîte ;
  les 4 intruses Découverte sont absentes.
- Le MC de départ est **assigné** (`engine/src/flow.rs:204`) : donné, jamais payé.
  Le piège signalé le 26-07 n'existe pas. [VÉRIFIÉ 27-07]

### Preuve d'exécution en PARTIE RÉELLE, pas seulement en sonde

Quatre compteurs neufs, incrémentés à l'endroit exact du mécanisme et nuls en
`--effects off`, relevés sur 1 000 parties :

| Compteur | Valeur | Ce qu'il prouve |
|---|---|---|
| `corp_heat_as_mc` | 5 510 | la chaleur d'*Helion* sert de monnaie |
| `corp_forest_rebates` | 883 | la forêt d'*Ecoline* coûte 1 plante de moins |
| `corp_tr_boosts` | 797 | le pas de terraformation acheté d'*Unmi* |
| `corp_trigger_tr` | 242 | le TR déclenché de *Saturn Systems* |

`research_extra_draws` passe de 1 293 à **4 266** : le +1/+1 de *Tharsis Republic*
s'applique bien en phase V. [VÉRIFIÉ 27-07]

### Les deux défauts corrigés

- **La pioche distribuait 16 corporations pour 12 dans la boîte.** Les intruses
  — *Apollo Industries*, *Exocorp*, *Hyperion Systems*, *Sultira* — sont des
  corporations de **Découverte** marquées `in_deck_v1: true` à tort, toutes
  porteuses de « Upgrade your phase N card ». **L'agent a refusé le filtre par
  noms que je proposais** et a posé le critère inverse : une table déclarée
  `effects::CORPS` des 12 planches réelles, `CardsDb::load` ne retenant que ce
  qui y figure, avec garde-fou « exactement une entrée v1 par nom » (piège des
  deux « Teractor Corporation ») et « exactement 12 ». **Quand Découverte
  s'ouvrira, il suffira d'ajouter les 4 entrées à cette table.** [VÉRIFIÉ 27-07]
- **Les productions de départ ne s'appliquaient pas.** *Ecoline* (1 plante),
  *Helion* (3 chaleur), *Thorgate* (1 chaleur) les reçoivent désormais sur les
  pistes fixes que la phase IV consomme — donc à chaque phase, pas une fois.
  [VÉRIFIÉ 27-07]

### Ma réserve principale, levée par l'agent lui-même

J'avais signalé au lancement que le « may » d'*Helion* (« tu **peux** utiliser la
chaleur comme MC ») risquait d'être figé en convention codée — donc **jamais
apprenable par l'IA**. C'était bien le cas dans sa première version.

**Sa relecture adversariale l'a trouvé et corrigé** : `engine/src/flow.rs:1139`
offre désormais le choix par `Policy::choose_option`, à la pose d'une carte —
seul site où le livret propose une alternative (payer en défaussant à 3 MC).
Ailleurs, renoncer à la chaleur reviendrait à renoncer à l'action : ce n'est pas
une branche jouable. [VÉRIFIÉ 27-07 par lecture du code]

**Bug trouvé par exécution et non par lecture** (journal D14) : avec *Helion*, la
conversion pouvait consommer la chaleur qu'un prérequis « Requires you to spend N
heat » engageait à dépenser à la pose, rendant la carte impayable.
`flow::heat_reserved_by` met cette chaleur hors d'atteinte, **à l'affordabilité
comme au paiement** — les deux ne peuvent donc pas diverger. [VÉRIFIÉ 27-07]

### Réserves consignées (aucune bloquante)

- **Défaut de MON contrat** : j'exigeais une preuve par sonde, mais la sonde
  n'exécute ni la phase III ni la phase V. La forêt d'*Ecoline* et le +1/+1 de
  *Tharsis* sont donc prouvés par partie réelle scriptée et par compteurs — plus
  fort qu'une sonde, mais pas ce que le contrat demandait à la lettre.
- **Défaut de MON contrôle caché n° 2** : il exigeait ≥ 8 sorties JSON
  analysables collées au rapport ; l'agent les a abrégées par des « … » pour la
  lisibilité. Vérifié à la main : les 12 corporations sondées existent et les 12
  sondes se rejouent correctement. Aucun mensonge. [VÉRIFIÉ 27-07]
- *Inventrix* : la souplesse de palier s'applique à chaque prérequis de couleur
  au lieu d'un seul. **Vérifié par ma main** : sur les 155 entrées de la table,
  les 3 cartes citant à la fois température et oxygène (*Regolith Eaters*,
  *Small Animals*, *Herbivores*) ne le font que dans leurs **effets**, jamais
  dans leurs `reqs`. L'écart est inobservable. [VÉRIFIÉ 27-07]
- Commentaire périmé corrigé par ma main à la promotion : l'en-tête de
  `flow.rs` décrivait encore la convention en dur, contredisant le code.

### Ce que Découverte devra reprendre

Les 4 corporations écartées reviennent en ajoutant leurs entrées à
`effects::CORPS`, une fois l'amélioration de carte Phase modélisée
(`phase_upgrades_skipped` compte toujours ces sauts). Alexis a confirmé le 27-07
qu'on jouera avec l'extension : **c'est désormais du périmètre obligatoire.**

## Acquis : textes imprimés des cartes (26-07) — NOUVELLE SOURCE DE VÉRITÉ

- **`data/cartes-imprimees/textes-cartes.json`** : **242 cartes** transcrites
  depuis les images des cartes imprimées, dont **220 de la pioche de base**
  (+ 12 corporations, 5 cartes de phase, 5 corporations promo). Remplace le
  champ `description` de `cards.json` comme référence de texte. [VÉRIFIÉ 26-07]
- **`docs/cartes/divergences.md`** : 247 écarts entre le texte imprimé et
  `cards.json`, classés par gravité (§G1 = 62 entrées nominatives qui changent
  une règle, §G2 = 20, §G3 = 163) + **6 motifs systémiques (§A)**.
  `docs/cartes/methode.md` : méthode et cartes non lues. [VÉRIFIÉ 26-07]

### Ce que l'audit a établi de MA propre main

- **La boîte contient 220 cartes, pas 222.** Les numéros imprimés couvrent
  1 à 220 sans un seul trou, sans doublon. *Microbiology Patents* et *Project
  Inspection* sont dans `cards.json` mais **sur aucune planche** — deux entrées
  probablement de trop dans la pioche v1. **Décision de conception à prendre.**
  Preuve que la numérotation n'est pas fabriquée : corrélation ordre-des-cellules
  / numéro-imprimé = **+0,114** sur la planche P1 (quasi nulle). [VÉRIFIÉ 26-07]
- **6 cartes contre-vérifiées par ma lecture des images** : *Advanced Ecosystems*
  n°65 (11 champs sur 11), *Energy Subsidies* n°25, *Surface Mines* n°192,
  *Biothermal Power* n°118, *Asteroid Mining* n°110 — exactes ;
  *Ganymede Shipyard* n°138 — **défaut trouvé et corrigé** (voir ci-dessous).
  [VÉRIFIÉ 26-07]
- **`vp_printed` de Ganymede Shipyard corrigé de 2 à 0 par la main.** L'encart
  gris à deux étoiles jaunes est un **savoir-faire de 2 titane**, pas des points
  de victoire. Règle du corpus : 1 étoile grise → « pay 3 MC less for [space] » ;
  2 étoiles → « pay 6 MC less » — soit 3 MC par titane, exactement le livret
  p. 18. Sur *Asteroid Mining* les deux marquages **coexistent et sont
  distincts** (encart gris + pastille brune ronde séparée). **Conséquence :
  après correction, ZÉRO écart de points de victoire entre le texte imprimé et
  `cards.json` sur 220 cartes — sur ce champ, la référence est fiable.**
  [VÉRIFIÉ 26-07]
- **Trouvaille confirmée exactement : 16 cartes de la pioche de base écrivent
  « MC » avec les lettres CYRILLIQUES « МС » dans `cards.json`** (Energy
  Subsidies, Power Grid, Trading Post, Tall Station…). Invisible à toute
  recherche textuelle sur « MC ». [VÉRIFIÉ 26-07 par mesure indépendante]
- **La correction de badge Espace/Énergie est réelle** : *Energy Subsidies*
  porte bien le soleil doré (Espace) ; l'éclair magenta n'est que dans son
  texte. 73 cartes revérifiées, 6 corrections au total. [VÉRIFIÉ 26-07]

### Réserves consignées à l'audit

- **Chiffre corrigé par la main** : §A annonçait « 47 cartes » où le mot-clé
  `Action:`/`Effect:` est imprimé mais absent de la paraphrase ; ma mesure donne
  **25** (mot-clé en début de texte) à **35** (sans ancrage). Le fond reste
  massif : **64 cartes de la pioche portent le mot-clé imprimé contre 29 dans la
  paraphrase**. Corrigé dans le document. [VÉRIFIÉ 26-07]
- **Deux de mes propres contrôles cachés étaient fautifs** : le seuil « ≤5 noms
  inconnus » (dépassé parce que j'ai moi-même élargi le périmètre en cours de
  route aux cartes de phase et corporations promo) et le critère « densité des
  numéros < 98 % » (mauvaise heuristique : la numérotation dense est
  authentique). [VÉRIFIÉ 26-07]
- Les `notes` de certaines cartes empilent **deux lectures non réconciliées**
  et peuvent se contredire (ex. *Advanced Ecosystems*). Les CHAMPS sont bons ;
  seules les notes sont à lire avec prudence. [VÉRIFIÉ 26-07]
- Verdict `aw report` : **partial**, promu après mes deux corrections.

## Ce qui marche

- **`engine/` : moteur Rust 2 joueurs, effets lots 1+2** — état, phases I-V,
  mulligans maison branchés au flux réel, production, conversions forcées,
  fin de partie, score AVEC VP des cartes (fixes + dynamiques calculables :
  tags Jupiter/Terre, forêts, cartes bleues/toutes — `flow::score`,
  `card_points`). **Couche d'effets déclarative** `engine/src/effects.rs`
  (table `LOT1` : 63 cartes complètes, prérequis paliers/tags/dépenses,
  chemin unique de pose `flow::build_card`). Sonde d'audit
  `simulate --probe "<nom>"` (JSON de deltas) ; `--effects on|off`.
  **Lot 2 (110 cartes couvertes au total)** : réductions de coût (service
  unique `flow::card_discount`, affordabilité + paiement, plancher 0),
  déclencheurs « when you play … » et température/océan
  (`fire_play_triggers`, `fire_global_trigger`), actions bleues réelles en
  phase III (`apply_blue_action`, compteur `blue_actions`), sondes v2
  (`--probe "A;B"` + `paid[]`, `--probe-action`). **Lot conformité** :
  prérequis de paramètres sur l'instantané de début de phase, pioche avant
  ou après en phase II, paiement d'une carte par défausse (3 MC/carte,
  surplus rendu), règles maison J1/J2 alterné et égalité sèche.
  **Lot 3** : ressources posées sur les cartes (microbes / animaux / science),
  28 cartes, VP dynamiques ANIMAL/MICROBE/SCIENCE réels, choix délégués à la
  politique. **231 tests verts** ; revalidé après promotion : 300/300 graine
  2024, 0 violation, ~11 750 parties/s. [VÉRIFIÉ 25-07]
- **`data/cards.json`** : 388 cartes, pioche v1 = 264 (248 projets +
  16 corporations), **+ champs `vp` (74 cartes > 0) et `vp_dynamic` (22)**
  extraits du Java par script reproductible
  (`workspaces/moteur-cartes-1/outputs/work/extract_vp.py`). [VÉRIFIÉ 24-07]

## Étude du terrain (2026-07-23) — voir `docs/ETUDE_TERRAIN.md`

- Meilleur simulateur existant : `nikitinalexx/ares-expedition` (GPL-3.0, Java,
  Discovery couvert, embryon d'IA, dormant depuis déc. 2025). [VÉRIFIÉ 23-07 —
  vérification contradictoire 3-0 par le harnais de recherche]
- Précision (vérifiée par clone du dépôt le 23-07) : les cartes SONT toutes
  répertoriées dans le code de nikitinalexx — 403 fichiers Java, un par carte,
  chacun portant nom, texte, coût, tags, effets ET son extension d'origine
  (interfaces `BaseExpansion...Card`, `DiscoveryExpansion...Card`, etc. ;
  `Card.java:37` expose `getExpansion()`). Ce qui n'existe pas, c'est un fichier
  de données autonome (JSON) réutilisable hors de ce code ; l'extraction est un
  travail mécanique de conversion. [VÉRIFIÉ 23-07]
- Point à auditer : le décompte des cartes Discovery dans le code (52 classes
  projet Discovery) dépasse les 38 cartes projet officielles — comprendre
  pourquoi (cartes de phase ? doublons ? contenu non officiel ?). [VÉRIFIÉ 23-07
  pour les chiffres, cause inconnue]
- Précédent IA le plus pertinent : `bnordli/rftg` (Race for the Galaxy,
  mécaniques quasi identiques, IA forte sur matériel modeste). [VÉRIFIÉ 23-07]
- Recommandation CTO issue de l'étude : réutiliser nikitinalexx comme référence
  de règles et source de cartes + construire un simulateur rapide dédié à
  l'entraînement. [DÉCLARÉ — jugement, pas un fait]

## Décisions ouvertes (attendent Alexis ou une étude)

1. ~~Valider la recommandation « nikitinalexx comme référence + simulateur
   d'entraînement maison »~~ → **VALIDÉE par Alexis le 23-07**. [VÉRIFIÉ 23-07]
2. **Approche d'apprentissage** : pari validé par Alexis le 23-07 — plancher
   type Keldon (réseau léger auto-entraîné, faisable même sans carte graphique)
   puis montée vers les méthodes modernes (recherche arborescente + réseau,
   actions simultanées, information cachée ; ReBeL exclu). Architecture précise
   à étudier. [VÉRIFIÉ 23-07 pour la décision, architecture À FAIRE]
3. **Entraînement local (RTX 3060) ou machines louées en ligne** : les
   références publiées dépassent une 3060 seule ; arbitrage après conception du
   simulateur rapide. [DÉCLARÉ]
4. ~~**Interfaces de jeu** : en ligne, et/ou plateau physique par caméra~~ →
   **TRANCHÉ par Alexis le 25-07** : la lecture du plateau physique par caméra
   est **abandonnée**. Le projet livrera un **jeu numérique avec interface
   propre** (glisser-déposer à la souris, ressenti d'un jeu de cartes en ligne
   du commerce), dans lequel l'IA jouera. Ordre retenu : moteur de règles →
   interface → IA. Conséquence : le chantier « vision par ordinateur » sort du
   périmètre ; un chantier « interface de jeu » y entre. Visuels de cartes :
   chaîne Tabletop Simulator VALIDÉE (voir §Acquis scans), réutilisable pour
   l'interface en usage privé. [VÉRIFIÉ 25-07 — message d'Alexis]

## Acquis (audités)

- **Workspace `audit-nikitinalexx` : livré et audité OK le 23-07** (contrôles
  3/3, contrôle caché 1/1, aucun trafiquage, chemin critique sondé).
  Livrables dans `workspaces/audit-nikitinalexx/outputs/` : [VÉRIFIÉ 23-07]
  - `cards.json` : **388 cartes** extraites du code Java par script
    (`outputs/work/extract_cards.py`), données non inventées (3 cartes témoins
    cachées exactes).
  - `rapport.md` : backend **lancé localement** (PostgreSQL Docker port 5455 +
    JDK 11 + Maven ; README faux sur POSTGRES_DB → `ares_db` ; lancer depuis la
    racine du dépôt sinon FileNotFoundException) ; **partie Discovery créée et
    ~2 rondes jouées via l'API REST**.
  - **Anomalie 38 vs 52 élucidée** : 52 = 38 officielles Discovery (dont
    Oxidation Byproducts codée mais jamais enregistrée dans `CardFactory` —
    typo « руфе » dans sa description) + 12 cartes du Promo Pack Kickstarter
    officiel (IDs 369-380) + 2 rebalances maison « buffed ». Hypothèse
    « cartes étoilées tutoriel » réfutée. Le champ `expansion` du code encode
    le deck de mélange, PAS la boîte d'origine → retagage nécessaire.
  - **Fidélité des règles** : 15 cartes auditées, 12 conformes, 2 écarts réels
    (Advanced Screening Technology prend toutes les cartes au lieu d'une ;
    Celestior non implémentée) + 1 carte absente du deck. **Zéro test dans le
    dépôt** → oracle utile mais pas parole d'évangile.
  - **IA embarquée** : infrastructure de self-play sans interface
    (`/simulations`, datasets, encodage d'état ~321 entrées) réutilisable ;
    le réseau lui-même (MLP figé, 1 coup de profondeur) non.
  - GPL-3.0 : consommer via API ou données, ne pas lier le code de Terra.

## Acquis : reconnaissance scans de cartes (24-07)

- **Chaîne d'extraction Tabletop Simulator validée de bout en bout sans
  posséder le jeu** : API Steam `GetPublishedFileDetails` → `file_url` du
  save → `strings` + grep des URLs `steamusercontent` → réécrire l'hôte
  mort `cloud-3.steamusercontent.com` en `images.steamusercontent.com` →
  planches 4096×4000 (10×7, ~40 cartes), qualité impression, texte net.
  Mod base anglais : `2831959805` (60 images). AE confirmé (phases I-V,
  12 corporations). Échantillons vus par la main (zoom 4 corporations
  lisible). Téléchargement complet dans `data/scans/base/` (hors git).
  [VÉRIFIÉ 24-07]
- **Discovery : seul un mod ESPAGNOL confirmé** (`3009184792`, 155 images,
  toutes extensions) ; le mod anglais `2793118592` = base seule (700 URLs,
  0 occurrence « Discovery »). Piste anglaise Discovery À TROUVER ; repli :
  photos d'Alexis + planches espagnoles pour les visuels. [VÉRIFIÉ 24-07]
- **DÉCOUVERTE DE RÈGLES (scan lu par la main)** : le titane et l'acier
  EXISTENT dans Ares Expedition — PhoboLog : « Each titanium you have
  reduces the cost of [space] cards an additional 1 MC » ; Mining Guild :
  « Each time you play steel production… ». Mon souvenir « pas de titane
  en AE » était FAUX (Alexis avait raison). Conséquence : le champ
  `description` de cards.json (« pay 6 MC less ») peut être une PARAPHRASE
  du dev russe et non le texte imprimé (Asteroid Mining imprimée donne
  peut-être « 2 titane »). TRANCHÉ le 24-07 par le livret (voir Acquis
  règles ci-dessous). Sources texte complémentaires trouvées :
  `alrusdi/tmae-content` (xlsx 219 cartes base, 2021),
  `sebwieser/ares-expedition` (Python, 2023). [VÉRIFIÉ 24-07]

## Acquis : règles officielles transcrites (24-07, photos d'Alexis)

- **`docs/regles/` = référence unique sur les règles** : 23 photos du livret
  FRANÇAIS (19 pages base + 4 pages Découverte, livret complet), transcrites
  mot à mot par 5 sous-agents Opus 4.8 (`transcription-brute/photo-NN.md`),
  assemblées en `livret-base.md` (pages 2-20) et `livret-decouverte.md`
  (pages 1-4). Audit par échantillon : pages 2, 18 et Découverte 3 relues
  par la main contre les photos — fidèles (1 correction : icônes de la
  Récompense Industriel = acier/titane). Originaux pleine taille hors git
  (`data/regles/photos-originales/`). [VÉRIFIÉ 24-07]
- **Aide-mémoire moteur** : `docs/regles/notes/regles-condensees.md`
  (~215 lignes, valeurs chiffrées, dérivé des livrets). Registre des points
  ambigus : `docs/regles/notes/cas-tranches.md` (4 tranchés, 3 en attente).
  [VÉRIFIÉ 24-07]
- **TITANE/ACIER TRANCHÉ par le livret p. 18** (« Savoir-faire — acier et
  titane ») : ce sont des SAVOIR-FAIRE, compteurs permanents de réduction
  (2 MC/acier sur badge Construction, 3 MC/titane sur badge Espace), PAS
  des ressources dépensées — exactement le modèle du Java. L'encodage
  actuel (réductions fixes) est équivalent tant qu'aucune carte ne
  multiplie le savoir-faire ; migration vers 2 compteurs/joueur à prévoir
  pour Advanced Alloys, Phobolog, Aquifer Pumping, Solarpunk. [VÉRIFIÉ 24-07]
- Points restant ouverts (détail dans cas-tranches.md) : listes complètes
  des 11 Objectifs / 7 Récompenses et des 10 cartes Phase améliorées
  (à tirer des scans/tuiles physiques), portée exacte de la phase Actions
  (« cartes » p. 14 vs « cartes bleues » p. 20). [VÉRIFIÉ 24-07]

## Travaux en cours

- **`moteur-cartes-3` LIVRÉ, AUDITÉ OK ET PROMU le 25-07** (2 rounds) :
  ressources posées sur les cartes. Périmètre arrêté à **28 cartes**
  après inventaire à la source — 14 conteneurs (Tardigrades, Birds, Fish,
  Livestock, Herbivores, Physics Complex, Ecological Zone, Anaerobic
  Microorganisms, Nitrite Reducting Bacteria, Fibrous Composite Material,
  Decomposing Fungus, GHG Production Bacteria, Regolith Eaters, Decomposers)
  et 14 cartes qui posent des ressources ailleurs. Le chiffre « ~41 cartes »
  du 24-07 était une estimation par motif textuel : il incluait ~24 cartes
  « production par tag » (Cartel, Satellites, Worms, Microbiology Patents…)
  qui relèvent d'un mécanisme distinct — **lot 4 « productions et VP variables
  par tag » à prévoir**. [VÉRIFIÉ 25-07]
- Nouveau vocabulaire imposé au lot 3 : type de ressource porté par carte,
  ajout/retrait par service unique, **alternatives (`ou`) et cibles exposées
  au trait `Policy`** (`choose_option`, `choose_res_target`,
  `choose_res_source`, à implémentation par défaut) — décision d'architecture :
  ces choix doivent être APPRENABLES par l'IA, donc jamais câblés.
  Stockage à ordre déterministe imposé (`HashMap` interdit). [VÉRIFIÉ 25-07]
- Hors périmètre déclaré du lot 3 : l'**amélioration de carte Phase**
  (Cryogenic Shipment, Fibrous Composite Material) reste non gérée, comptée
  par `phase_upgrades_skipped`, sans compensation inventée. [VÉRIFIÉ 25-07]

## Acquis : workspace `moteur-conformite-1` (livré et audité OK le 24-07)

- Audit 8/8 (4 checks + 4 hold-out, dont les deux témoins d'instantané
  OPPOSÉS : `Giant Ice Asteroid;Great Dam` doit être bloquée / `Crater`
  après 3 événements doit passer — piège si les tags basculent à tort sur
  l'instantané) ; tampering néant. Contre-vérifications de la main :
  **264 cartes sondées identiques** à l'ancien moteur (rétro-compatibilité
  totale du mode sans option), graine inédite 555777 (1000/1000, 0
  violation), arithmétique de défausse-paiement recalculée à la main
  (Ice Asteroid 21 MC avec 4 MC + 6 cartes, surplus rendu ; les 2 océans
  rapportent 4 MC — identique à l'ancien moteur), instantané entre joueurs
  confirmé, ordre du tour lu sur l'état réel (`play_round` :1157, pas une
  formule). Verdict : ok. [VÉRIFIÉ 24-07]
- **Corrections livrées** (`engine/`, 185 tests verts, ~13 000 parties/s) :
  C1 prérequis de PARAMÈTRES sur l'instantané `snap_*` (`flow.rs:252`,
  prédicat commun `reqs_satisfied` :216 ; tags et dépenses restent à l'état
  courant) ; C2 pioche AVANT ou après en phase II (`DrawCardBefore`) ;
  C3 défausse-paiement 3 MC/carte, minimum nécessaire, surplus rendu,
  prédicat unique `payable` (`flow.rs:283`) + `build_card_with` ;
  C4 règle maison J1/J2 alterné avec alternance ACTION PAR ACTION en
  phase III (`phase_action`) ; C5 égalité sèche (compteur `draws`) +
  conversions obligatoires sur l'instantané. Compteurs d'audit :
  `prereq_snapshot_blocks` (rare : 2-10 par millier de parties),
  `draw_before_build`/`draw_after_build`, `discard_payments`, `draws`,
  `turn_order_switches`. Sonde : `--probe-mc`, `--probe-filler`,
  `--probe-strict`, champs `discarded[]` et `prereq_ok_now`. [VÉRIFIÉ 24-07]
- **Erreur de MON contrat, relevée par l'agent** : le cas imposé
  « Lichen --probe-mc 5 --probe-filler 5 → delta.hand = -1 » est
  incompatible avec la convention `delta.hand` du lot 2 (qui donne 0).
  L'agent a fait basculer la base de calcul sur la présence de
  `--probe-filler` et l'a déclaré. Conséquence : `delta.hand` a deux sens
  selon les options — cosmétique (outil d'audit), à unifier si la sonde est
  retouchée. [VÉRIFIÉ 24-07]
- Deux tests existants adaptés (limite contractuelle : 3), tous deux au
  niveau du HARNAIS, aucune assertion affaiblie : `snapshot_planet()` ajouté
  dans un test lot 1 qui fabriquait un état hors flux de phase ; script
  d'actions de `TestPolicy` réservé au joueur 0 (la phase III alterne
  désormais). Relus par la main. [VÉRIFIÉ 24-07]

- **`moteur-cartes-4` LIVRÉ, AUDITÉ OK ET PROMU le 25-07** : productions
  **dérivées** — les cartes dont la production dépend du nombre de badges
  possédés, **recalculée à chaque phase de production**. Périmètre : **17
  cartes** (et non ~24 : les points de victoire variables par badge étaient
  **déjà** calculés par `flow::card_points` via `vp_dynamic`, vérifié
  `flow.rs:1551-1571`). 14 productions dérivées + Immigration Shuttles
  (production fixe) + Terraforming Ganymede (NT par badge Jupiter) +
  Interplanetary Relations (bonus permanent de phase Recherche).
  **271 tests verts**, table à 155 entrées, ~8 500 parties/s (machine chargée ;
  ~11 750/s au repos). [VÉRIFIÉ 25-07]
- Vocabulaire du lot 4 : `ProdRes`, `ProdCount { Tag, Forests }`,
  `DerivedProd { res, count, per }`, `ResearchBonus { draw, keep }`,
  `Eff::TrPerTag`. **Services uniques** `flow::derived_production` (`flow.rs:750`)
  et `flow::research_extra` (`flow.rs:786`), consommés par la phase de jeu ET
  par la sonde. Rien n'est jamais inscrit sur les pistes `*_prod` : c'est
  l'interdit central du lot. Sonde : `--probe-produce` exécute la VRAIE
  `phase_production` et relève la variation des compteurs ; champ `vp_total`
  (somme de `card_points` sur toutes les cartes en jeu). [VÉRIFIÉ 25-07]
- **Règle tranchée au livret** : la production « 1 MC par badge X » n'est PAS
  figée à la pose. `docs/regles/livret-base.md:180`. [VÉRIFIÉ 25-07]
- Vérification OCR intégrée à la préparation du contrat : **Windmills**
  (n° 206) porte « including this » que `cards.json` omet ; **Insects**
  (n° 152) compte les badges **Plante**, qu'elle ne possède pas ; **Zeppelins**
  (n° 208) compte les **jetons Forêt**. Conclusion : « including this » est un
  rappel, pas une règle à part. [VÉRIFIÉ 25-07 par lecture des scans]
- **Signalé par l'agent, non traité (périmètre)** : trois cartes du deck v1
  portent le même bonus de recherche et restent inertes — *United Planetary
  Alliance* (11 MC, texte identique à Interplanetary Relations à 35 MC),
  *Interns*, *Extended Resources*. Le vocabulaire est en place : trois lignes
  de table. À verser au prochain lot. [DÉCLARÉ par l'agent, plausible]
- **Le seuil de vitesse des contrôles (8 000 parties/s) n'a plus de marge** :
  mesuré entre 7 460 et 8 800 selon la charge de la machine. Mesures alternées
  avant/après le lot par la main : aucune régression (le lot est marginalement
  plus rapide). À relever ou à mesurer sur 10 000 parties dans les prochains
  contrats. [VÉRIFIÉ 25-07]
- **Incident de harnais** : le premier agent lancé sur ce lot s'est figé au
  démarrage sans rien produire (surveillance : 600 s sans activité). Relancé à
  neuf, `outputs/` était vide — aucune reprise bancale. [VÉRIFIÉ 25-07]
- Bidirectionnalité du contrat prouvée avant scellement : 4 contrôles visibles
  rouges à l'état actuel pour la bonne raison, verts sur un faux moteur
  simulant l'état-cible ; hold-outs 01/02/03 idem, hold-out 04
  (non-régression) vert dès le départ ; **7 contre-tests de falsification, 7
  détectés** (production figée à la pose, annoncée sans être créditée, inscrite
  sur la piste de production, division arrondie au-dessus, mauvais badge
  compté, compteur forfaitaire, effets coupés non neutres). [VÉRIFIÉ 25-07]

## Acquis : workspace `moteur-cartes-3` (livré, audité OK et promu le 25-07)

- **Audit 8/8 aux deux rounds** (4 checks visibles + 4 hold-outs cachés),
  tampering néant. Promu dans `engine/` : **231 tests verts** (27+72+53+46+33),
  300/300 parties graine 2024, 0 violation, ~11 750 parties/s. [VÉRIFIÉ 25-07]
- **Livré** : mécanisme complet des ressources posées sur les cartes.
  Vocabulaire déclaratif (`ResKind`, champ `holds`, `ResPut`/`ResEff`/
  `ResStep`, `TrigGain::ResSelf`/`Choose`, `TrigCond::AnyOfTags`,
  `GlobalTrigger::OnRaiseOxygen`/`OnBuildForest`, `Action::Res`,
  `Reduction::PayResources`). Stockage `BTreeMap<u16,u32>` dans `PlayerState`
  (aucune table de hachage dans `src/`). **Service unique**
  `flow::add_resources`/`remove_resources` (seuls points d'écriture, avec
  assertions défensives). Score dans `flow::card_points` (retourne
  `(total, from_resources)`), chemin unique partagé par le score de partie et
  la sonde. 28 cartes encodées, table à 138 entrées. [VÉRIFIÉ 25-07]
- **Choix délégués à la politique** (décision d'architecture) : `choose_option`,
  `choose_res_target`, `choose_res_source`, à implémentation par défaut.
  Branches injouables filtrées AVANT le choix ; une seule branche jouable = pas
  de choix demandé (déclaré, journal D3). [VÉRIFIÉ 25-07]
- **MON CONTRAT ÉTAIT FAUX sur 3 cartes**, l'agent l'a signalé au round 1 et
  j'ai tranché **au scan des cartes imprimées** : Symbiotic Fungus,
  Extreme-Cold Fungus et Conserved Biome portent « Action: » → actions
  RÉPÉTABLES, pas effets de pose. Corrigé au round 2 (+ Large Convoy
  « ANOTHER » et non « ANY »). Tests des 3 cartes passant par le flux réel
  (`build_card` + `play_round`), prouvant la répétabilité. [VÉRIFIÉ 25-07]
- **Bogue préexistant attrapé par l'agent** : la sonde et les tests par nom
  résolvaient le PREMIER homonyme de `cards.json`, souvent la version
  rééquilibrée « Buffed » hors pioche. `CardsDb::resolve_card` (préfère la
  carte `in_deck_v1` quand elle est unique) corrige 5 cartes du deck v1 :
  Community Gardens 10→20, Drone Assisted Construction 7→15, Extreme-Cold
  Fungus 6→10, Farming Co-ops 7→15, Wood Burning Stoves 9→13. **Les PARTIES
  n'étaient pas faussées** (la pioche filtre sur `in_deck_v1`, `flow.rs:63`) :
  le défaut était limité à la sonde et aux tests par nom — dont 2 tests du
  lot 2. [VÉRIFIÉ 25-07 par comparaison des 264 cartes ancien/nouveau moteur]
- **MON hold-out 02 avait une attente fausse** : j'attendais 3 plantes pour
  Imported Hydrogen branche « plantes », sans compter le bonus de la tuile
  Océan révélée (2 plantes) — déjà vrai dans l'ancien moteur. Corrigé.
  [VÉRIFIÉ 25-07]
- **Non géré et déclaré** : amélioration de carte Phase (Cryogenic Shipment,
  Fibrous Composite Material), sautée et comptée par `phase_upgrades_skipped`,
  **sans compensation** (vérifié par hold-out : tous les deltas à 0).
  [VÉRIFIÉ 25-07]
- Compteurs d'audit : `res_added`, `res_removed`, `res_targets_missing`,
  `phase_upgrades_skipped`, `vp_from_resources` — tous nuls en `--effects off`.
  Sonde : `resources[]`, `target_error`, `--probe-choice`, `--probe-target`,
  sur `--probe` et `--probe-action`. [VÉRIFIÉ 25-07]

## Acquis : workspace `moteur-cartes-2` (livré et audité OK le 24-07)

- Audit 8/8 (4 checks + 4 hold-out : réductions, déclencheurs, actions,
  graine 662607), tampering néant ; chemin critique lu (`card_discount`
  service unique affordabilité+paiement, `fire_play_triggers`,
  `apply_blue_action`) ; 19/19 réductions contre-vérifiées au texte par la
  main ; sondages indépendants graine 909090 (600/600, 0 violation) et
  6 sondes hors hold-out exactes. Verdict : ok. Promu dans `engine/`
  (152 tests re-vérifiés verts, 300/300 graine 2024). [VÉRIFIÉ 24-07]
- 47 cartes neuves (A=26 réductions, B=9 déclencheurs, C=14 actions).
  Enquête titane/acier ÉLUCIDÉE : le Java `DiscountService` consomme
  `steelIncome`/`titaniumIncome` comme réductions (×2 MC/Building,
  ×3 MC/Space) — cohérent avec les scans (PhoboLog) : en AE le titane est
  un « compteur de réduction », pas une ressource dépensée. Encodage suivi :
  texte de description (réductions fixes), titane non modélisé ; cartes qui
  le suivent vraiment (Aquifer Pumping, Solarpunk, Advanced Alloys) hors
  lot. Conflits déclarés : Titanium Mine (tag imprimé Building, réduction
  Space). Exclues pour nom dupliqué « Buffed » : Greenhouses, Community
  Gardens. Imprécision mineure du journal de l'agent : montants variables
  de `--probe-action` = tirage aléatoire déterministe, pas « montant max ».
  [VÉRIFIÉ 24-07]
- Reste stubbé (lots suivants) : ressources sur cartes (lot 3),
  améliorations de phases, suivi acier/titane réel (si les photos du livret
  le confirment comme ressource), corporations, 7e award. [VÉRIFIÉ 24-07]

## Acquis : workspace `moteur-cartes-1` (livré et audité OK le 24-07)

- Contrôles 4/4 + hold-out 3/3 (5 témoins VP, 5 témoins d'effets vérifiés à
  la source Java, graine inédite), sondage indépendant graine 424242
  (600/600, 0 violation), 10 encodages contre-vérifiés au texte imprimé par
  la main. Verdict : ok. Promu dans `engine/` + `data/cards.json` (chemins
  adaptés, 99 tests re-vérifiés). [VÉRIFIÉ 24-07]
- Conflit texte/Java tranché texte : Nitrogen-Rich Asteroid (`== 3` Java vs
  « 3 or more » imprimé). Cas Livestock (`//TODO` dans le code VP). Erreur de
  MON contrat : Grain Silos imposée alors que `in_deck_v1=false` → la base
  charge désormais les 331 cartes projets (pioche inchangée = 248), piste
  infrastructure minimale ajoutée (+1 TR +1 carte par pas, hors fin de
  partie). Invariant TR étendu (`tr == 5 + incr − decr`, cartes
  « spend 1 TR »). [VÉRIFIÉ 24-07]
- Reste stubbé (lots suivants) : réductions de coût, « when you play … »,
  ressources sur cartes (vp_dynamic ANIMAL/MICROBE = 0 au score), actions
  bleues, améliorations de phases, 7e award. [VÉRIFIÉ 24-07]

## Acquis : workspace `moteur-squelette` (livré et audité OK le 24-07)

- Contrôles 4/4, hold-out graine inédite PASS, re-sondage indépendant
  (graine 314159 : 500/500, 0 violation). Journal : 18 décisions sourcées
  (livret base seulement disponible en suédois — traduit et croisé avec le
  moteur Java ; conflit « 7 awards Discovery vs 6 dans le Java » noté).
  Moteur promu dans `engine/` (chemins adaptés, 27 tests re-vérifiés).
  [VÉRIFIÉ 24-07]
- Trous relevés à l'audit : VP imprimés absents et effets stubbés (comblés
  par `moteur-cartes-1` — voir section dédiée) ; restent : améliorations de
  phases neutres, 7e award à élucider, revendication des milestones
  simplifiée. [VÉRIFIÉ 24-07]

## Acquis : workspace `retag-cartes` (livré et audité OK le 24-07)

- `outputs/cards_v1.json` : 388 cartes ré-étiquetées par boîte réelle
  (base 239, discovery 42, promo 15, fan 69, crysis 22, none 1) ;
  pioche v1 = 281 cartes ; Oxidation Byproducts réintégrée ; origine des
  données intacte ; hold-out 7 témoins exact. [VÉRIFIÉ 24-07]
- `outputs/divergences.md` : croisement Mylaana (317 paires) — 31 divergences
  de nom, 4 de coût, 4 de tags, arbitrées et sourcées. [VÉRIFIÉ 24-07]
- Bogues du moteur Java attrapés : `BuffedBirds`/`BuffedCommunityGardens`
  marquées base ; la classe `MayNiProductionsCorporation` porte le `name`
  erroné « Teractor Corporation » (doublon). [VÉRIFIÉ 24-07 par sondage]
- Correctif à mon contrat, déclaré par l'agent : les 20 cartes
  « infrastructure » = très probablement l'extension OFFICIELLE Foundations
  (20 cartes appariées Mylaana `foundations`), pas une extension maison ;
  livrées `fan` (enum scellé), sans impact pioche v1. [DÉCLARÉ par l'agent,
  cohérent]
- Décision pack 2021 **CLOSE (24-07)** : Alexis certain de ne pas posséder
  Synthetic Catastrophe (carte témoin la plus reconnaissable du pack) → il
  ne possède PAS le pack KS 2021. Les 17 cartes restent `box: promo2021`,
  HORS pioche — réglage par défaut confirmé définitif. `data/cards.json`
  (pioche v1 = 264) inchangé. [VÉRIFIÉ 24-07]
- (Historique) 17 cartes d'un SECOND pack promo
  (Kickstarter 2021 : ArkLight, Celestior, DevTechs, LaunchStar, Mai-Ni,
  Zetacell + 11 projets dont Self-Replicating Bacteria, Synthetic Catastrophe,
  Processing Plant) sont marquées `base` par le moteur et comptent
  actuellement DANS la pioche v1. Mylaana les classe promo. Résolu : Alexis
  ne les possède pas (voir décision close ci-dessus), aucun round 2 requis.
  [VÉRIFIÉ 24-07]

## Décisions de périmètre tranchées par Alexis (23-07)

- **2 joueurs sur tout le projet** (« on jouera toujours à 2 joueurs »).
- Cartes promo : exclues de la pioche (non possédées). Cartes étoilées
  tutoriel : incluses. Voir `docs/CONCEPTION_SIMULATEUR.md`. [VÉRIFIÉ 23-07]

## Verrous et risques connus

- L'IA a besoin d'un **simulateur complet et fidèle des règles** (extension
  Découverte et règles maison comprises) avant tout apprentissage : c'est la
  dépendance numéro un du projet. [DÉCLARÉ]
- Droits d'auteur : le jeu est une propriété commerciale (FryxGames / Intrafin) ;
  un usage privé d'un simulateur maison est défendable, une diffusion publique
  des textes/images de cartes ne l'est pas forcément. À garder en tête. [DÉCLARÉ]

## Sources à relire pour régénérer cette carte

- `docs/CTO_PROJET.md` (objectif et périmètre)
- Le message initial d'Alexis du 2026-07-23 (repris dans `docs/JOURNAL.md`)

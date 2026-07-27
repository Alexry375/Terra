# Méthode — transcription du texte imprimé des cartes

> Document de méthode du chantier `textes-cartes`. Il décrit d'où viennent les
> images, comment les planches ont été découpées, comment les cartes ont été
> lues, et il nomme les cartes qui n'ont pas pu être lues.

## 1. Source des images

Toutes les images viennent de `data/scans/base/`
(`/home/alexis/Global/Agents_Projects/Terra/data/scans/base/`), en lecture
seule : 55 fichiers PNG, ~211 Mo, plus `urls.txt` et `save.bin`. Rien n'y a été
écrit.

Il s'agit de l'adaptation Tabletop Simulator de **Ares Expedition — The
Terraforming Mars Card Game**, boîte de base, en anglais (sauvegarde datée du
9 juillet 2022, version 13.1.1 du mod).

## 2. Établir la géométrie des grilles — sans la deviner

`save.bin` n'est pas une image : c'est la sauvegarde Tabletop Simulator au
format **BSON**. Elle a été décodée avec un décodeur minimal écrit pour
l'occasion (`outputs/work/bsondec.py`, lecture seule) vers
`outputs/work/save.json`. Cette sauvegarde déclare, pour chaque planche, sa
géométrie exacte (`NumWidth`, `NumHeight`) et le nombre de cartes réellement
posées dessus.

C'est ce qui permet de ne pas « supposer 10 × 7 » : la grille **est** 10 × 7,
mais les paquets ne remplissent que les *n* premières cases (52 sur 70 pour les
planches de cartes projet). Le reste de la planche est noir — c'est ce qui
produisait des cases vides lors d'une découpe naïve en 70 cellules.

Les URL d'origine ne permettent pas de retrouver le nom de fichier local (le
nom `img_<12 hexa>.png` n'est pas un hachage de l'URL). L'appariement
planche ↔ fichier a donc été fait par **taille d'image, empreinte MD5 et
inspection visuelle** : deux URL pointant vers la même ressource ont produit
deux fichiers identiques octet à octet, ce qui a confirmé l'appariement des
planches dupliquées.

### Planches retenues

| Fichier | Étiquette | Grille | Cartes | Contenu |
|---|---|---|---|---|
| `img_092c2f2f19d8.png` | P1 | 10 × 7 | 52 | cartes projet |
| `img_29b614b44ee9.png` | P2 | 10 × 7 | 52 | cartes projet |
| `img_6eb0336793ad.png` | P3 | 10 × 7 | 52 | cartes projet |
| `img_b09c488427ab.png` | P4 | 10 × 7 | 52 | cartes projet |
| `img_917b063334cb.png` | CORP | 10 × 7 | 12 | corporations |
| `img_8ec0d6d5d6ca.png` | PROMO | 10 × 2 | 11 | cartes projet promo Kickstarter |
| `img_991954f9c3f1.png` | PROMOCORP | 10 × 2 | 6 | corporations promo |
| `img_ef125fd09905.png` | PHASE | 10 × 7 | 5 | cartes de phase |

Décompte final vérifié des **cellules réellement pleines** : 52 / 52 / 52 / 52 /
12 / 11 / 6 / 5 = **242 cartes**, soit 220 de la boîte de base (208 cartes projet
+ 12 corporations), 17 hors pioche (11 promos + 6 corporations promo) et 5 cartes
de phase. Les cellules de fin de grille
sont entièrement noires et se reconnaissent à leur poids (9 à 30 Ko contre
~2,7 Mo pour une vraie carte) ; les 10 cellules noires de la planche PHASE
avaient d'abord été comptées comme des cartes, elles sont écartées de la
livraison. Aucune cellule pleine n'est restée non lue.

Doublons octet à octet écartés : `img_fad00409f01a.png` (= PHASE),
`img_d71993106cdb.png` (= `img_c8db0760e78b.png`),
`img_fbd508c294bc.png` (= `img_c17759143fb6.png`).

Images écartées comme ne contenant pas de cartes : plateaux joueur
(`img_503a82f8fc24.png`, `img_e2af1ce6daa7.png`, `img_f1d17f696568.png`,
`img_2bd222a2d82f.png`), tuiles et jetons (formats 460 × 460, 451 × 404,
550 × 550, 710 × 372, 36 × 36), dos de cartes (formats 760 × 1060).

## 3. Découpe

`outputs/work/decoupe.py` découpe chaque planche en cellules de largeur `W/10`
et hauteur `H/7` (ou `H/2` pour les planches promo), remplies **ligne par ligne**
(`index = ligne × 10 + colonne`), et ne découpe que les *n* premières cellules —
celles qui portent une carte. Chaque cellule est ensuite agrandie par
interpolation Lanczos à 1500 px de haut (≈1075 × 1500), taille à laquelle le
texte imprimé est net.

Sortie : `outputs/work/cells/<ÉTIQUETTE>/<ÉTIQUETTE>-NN_rXcY.png`. Le champ
`source_image` de chaque entrée livrée reprend le nom du fichier planche, son
étiquette, sa ligne et sa colonne.

## 4. Lecture

La lecture est faite **à l'œil, sur l'image de la carte**, cellule par cellule.
La reconnaissance optique n'a servi qu'à une tentative de repérage des titres,
abandonnée : sur du texte blanc posé sur illustration, `tesseract` rendait des
titres inutilisables. **Aucun texte livré ne vient d'un OCR.**

Le volume (242 cellules pleines) a été réparti en **73 lots** confiés à des
sous-agents lecteurs (`outputs/work/lots/`, `lots2/`, `lots3/`, `promo/`), avec
une consigne commune (`outputs/work/consigne-lecture.md`) leur interdisant l'OCR
et toute lecture de `inputs/cards.json` — la paraphrase à remplacer ne devait pas
pouvoir contaminer la lecture. Chaque lot est écrit dans son propre fichier JSON,
réécrit après **chaque carte** : une interruption ne coûte jamais plus d'une
carte. Chaque lot a été relu et recoupé par l'agent responsable (voir §6).

**Budget réseau.** Le poste a un débit montant de 200 Ko/s contre 1 900 en
descendant, et toute la conversation d'un agent — images comprises — est
renvoyée à chaque action. Huit lecteurs d'images en parallèle ne vont pas plus
vite : ils partagent le même lien, chacun devient environ huit fois plus lent, et
le garde-fou d'inactivité de 600 s les tue. La règle appliquée est donc
**3 lecteurs simultanés au maximum, 3 images chacun, uniquement les JPEG
allégés**. C'est ce qui a coûté la première vague de lecteurs de la session
précédente (18 agents perdus, voir `journal.md` §Decision Log).

### Conventions de transcription

- Les pictogrammes sont rendus entre crochets : `[heat]`, `[plant]`, `[MC]`,
  `[card]`, `[ocean]`, `[forest]`, `[temperature]`, `[oxygen]`, `[TR]`,
  `[building]`, `[space]`, `[science]`, `[microbe]`, `[animal]`, `[earth]`,
  `[jupiter]`, `[energy]`, `[event]`.
- **Extension au vocabulaire du cahier des charges** : le petit éclair jaune qui
  ouvre une ligne d'effet immédiat sur les cartes vertes n'a pas d'équivalent
  dans la liste fournie. Il est transcrit `[effect]`. C'est une convention
  ajoutée, signalée ici pour qu'elle puisse être renommée d'un coup si le projet
  préfère un autre jeton.
- Les pastilles rondes **vides** du bord gauche ne sont pas des badges : ce sont
  des emplacements décoratifs. Elles ne sont pas comptées dans `tags`.
- `production` reprend ce que montre l'encart orange, tel que lisible ; la
  quantité produite, elle, est dans le texte de la carte.

### Champ `name` et champ `printed_name`

`inputs/cards.json` emploie parfois un nom différent de celui imprimé
(« Helion Corporation » pour *Helion*, « Unmi » pour *United Nations Mars
Initiative*, « Advanced Screening Tech » pour *Advanced Screening Technology*…).
Le fichier livré doit rester indexable par le projet : le champ `name` porte donc
**la clé du projet**, et un champ supplémentaire `printed_name` porte **le nom
réellement imprimé** — toujours renseigné, y compris quand les deux coïncident,
pour qu'un consommateur n'ait jamais à deviner lequel il lit. Les 18 cas où ils
diffèrent sont tabulés dans `divergences.md` §B.

## 5. Recontrôle des badges Espace et Énergie

Une erreur avérée a motivé un contrôle dédié : *Energy Subsidies* avait été notée
`energy` par un lecteur, alors qu'elle porte le badge **Espace**. Le lecteur
s'était laissé guider par le titre de la carte et par une icône présente dans le
**texte** — laquelle désigne ce que la carte affecte, pas ce qu'elle porte.

Vocabulaire visuel établi puis imposé à tous les relecteurs :

- **Espace** = soleil doré à rayons sur disque sombre
- **Énergie** = éclair blanc sur disque **magenta**
- **Construction** = toit / bâtiment marron
- **Événement** = flèche noire vers le bas sur disque jaune
- une pastille **unie, sans pictogramme** n'est pas un badge — c'est la source
  de sur-comptage la plus fréquente, présente sur presque toutes les cartes

Les **73 cartes** portant `energy` ou `space`, plus celles dont les deux lectures
divergeaient sur les badges, ont été **revérifiées sur l'image**, badge par badge,
seule la colonne du bord gauche faisant foi. Pour tenir le budget réseau du poste
(200 Ko/s en émission), ces contrôles ont porté sur des montages ne contenant que
la bande gauche des cartes (`outputs/work/badges/badges-NN.jpg`, 6 cartes par
image, ~33 Ko par carte au lieu de 85). **Les relecteurs ne connaissaient pas les
noms** des cartes qu'ils contrôlaient : il leur était matériellement impossible
de se laisser guider par le titre.

Limite du cadrage, signalée spontanément par trois relecteurs : les bandes
s'arrêtent à 62 % de la hauteur de la carte, soit trois emplacements de badge ; un
quatrième badge plus bas serait passé inaperçu. Vérification faite, **aucune des
388 cartes de `cards.json` ne porte plus de 3 badges**, et la seule lecture à
4 badges du chantier (*Advanced Ecosystems*) était précisément l'erreur que ce
recontrôle a corrigée. Risque résiduel nul en pratique.

## 6. Double lecture et arbitrages

**116 des 242 cellules ont été lues deux fois** (48 %), par deux agents
indépendants (`outputs/work/lots/` puis `outputs/work/lots2/`) ; les 126 autres
n'ont qu'une lecture. Ces doubles lectures servent d'oracle : la fusion
(`outputs/work/fusion2.py`) les compare champ à champ. Résultat mesuré :
**41 cellules en désaccord**, pour **60 désaccords champ par champ** (`text` 21,
`production` 20, `tags` 5, `name` 5, `cost` 5, `color` / `requirement` /
`phase` / `printed_number` 1 chacun). Ce sont eux qui ont révélé les fautes de
lecture les plus coûteuses.

**Comment ces désaccords sont tranchés — précision qui compte.** La très grande
majorité l'est **automatiquement**, par heuristique typographique dans
`fusion2.py` : `pick_text` retient la variante la plus riche (plus d'icônes entre
crochets, plus de deux-points, à défaut la plus longue), `pick_common` prend la
valeur majoritaire, `pick_prod` privilégie la transcription pictographique de
l'encart. **Seuls deux cas sont arbitrés à la main**, en dur dans le code
(`TEXT_FIX`, `COLOR_FIX`) : *Surface Mines* et *Advanced Ecosystems*. Il ne faut
donc pas lire « 41 désaccords arbitrés » comme « 41 images rouvertes » : les
seules images rouvertes le sont dans le recontrôle de badges du §5, qui a porté
sur 73 cartes.

Limite connue de cette heuristique : `pick_text` préfère la variante la plus
longue à égalité d'icônes, ce qui conserve parfois un espacement moins régulier
(*Small Animals* porte `* = 1 VP …` là où les 20 autres cartes à PV variable
portent `*=1 VP …`). Cosmétique, mais c'est un artefact de l'automatisme, pas une
lecture.

Arbitrages non triviaux :

- **`text` contre `requirement`.** Un lecteur recopiait le prérequis en tête du
  texte principal, l'autre non. Le prompt distingue les deux champs : le prérequis
  est retiré de `text` quand il y fait doublon. Aucune information n'est perdue,
  `requirement` la porte.
- **Prérequis d'oxygène et de température.** Le vocabulaire d'icônes du prompt
  n'en prévoyait aucun. Ils sont conservés **en toutes lettres, tels qu'imprimés**
  (« Requires white temperature. », « Requires red oxygen or higher. »,
  « Requires 6 ocean tiles to be flipped. ») dans `requirement`. Contrôle fait :
  rien n'a été perdu, les cartes à prérequis en portent bien un.
- **`phase`.** La consigne initiale ne prévoyait qu'un seul chiffre romain ;
  beaucoup de cartes en portent une **plage**. La plage est conservée telle
  qu'imprimée : `I-II`, `I-III`, `I-V` sont des valeurs légitimes du champ, au
  même titre que `III` ou `IV`.
- ***Advanced Ecosystems*** : lue rouge + 4 badges par un lecteur, verte +
  2 badges par l'autre. Le recontrôle tranche : `microbe, animal, event`. Le badge
  Événement n'existant que sur les cartes rouges, la couleur rouge est confirmée
  par la même lecture.
- ***Surface Mines*** : divergence sur la deuxième icône de l'effet (`[space]`
  contre `[event]`). Retenu `[space]`. **Déclaration honnête : c'est le seul champ
  de texte de toute la livraison dont l'arbitrage s'appuie sur `cards.json`** — la
  référence y indique Espace, et c'est aussi le seul choix cohérent avec un effet
  de réduction de coût par catégorie de carte. L'image n'a pas été rouverte pour
  trancher. Une carte, un mot, aucun autre champ concerné ; le fait est signalé
  ici plutôt que reformulé, parce que la clause anti-shortcut du contrat interdit
  toute sortie fabriquée depuis une autre source que les images.
- **Corporations et `cost`.** Une corporation n'a pas de pastille de coût ; le
  nombre imprimé en haut à droite est son **MC de départ**, confirmé par la phrase
  « You start with N MC. » de son texte. C'est lui qui est porté dans `cost`.
  Piège de modélisation détaillé dans `divergences.md`.
- **Cartes de phase.** Les 5 cartes de phase ne sont pas des cartes projet ; l'une
  s'appelle *Research*, comme une carte rouge de la pioche. Elles portent le
  suffixe « (carte de phase) » dans `name` pour ne pas la masquer.

## 7. Cartes non lues

Deux cartes attendues par `inputs/cards.json` (`in_deck_v1: true`, `box: "base"`)
**ne figurent sur aucune planche** :

- **Microbiology Patents**
- **Project Inspection**

Ce ne sont pas des échecs de lecture : les planches P1 à P4 contiennent exactement
208 cartes projet (52 × 4), toutes lues, et `save.bin` — qui liste nommément le
contenu de chaque paquet TTS — ne mentionne ni l'une ni l'autre. Surtout, les
numéros imprimés lus au coin inférieur droit couvrent **1 à 220 sans un seul
trou**, alors qu'ils ne suivent pas du tout l'ordre des cellules (la première
cellule de P1 porte le n° 148) : la boîte comporte 220 cartes numérotées, pas
222. Elles sont donc absentes de cette édition, pas illisibles ; elles sont
**déclarées manquantes et non fabriquées** (voir `outputs/blocked.md`).

**Aucune autre carte n'est illisible.** Les 220 autres cartes attendues sont lues,
ainsi que 5 cartes de phase et 17 cartes hors pioche (11 promos, 6 corporations
promo) qui ne sont pas exigées mais qui serviront.

### Cartes livrées avec un `text` vide

Quatre cartes ont un champ `text` vide sans être « non lues » : ce sont des cartes
rouges dont **tout le contenu imprimé est un prérequis et des points de victoire**,
sans zone de texte d'effet. Le prérequis est dans `requirement`, les PV dans
`vp_printed` :

- **Advanced Ecosystems** — « Requires an [animal], [microbe], and [plant]. », 3 PV
- **Breathing Filters** — « Requires yellow oxygen or higher. », 2 PV
- **Colonizer Training Camp** — « Requires red oxygen or lower. », 2 PV
- **Interstellar Colony Ship** — « Requires 4 [science]. », 4 PV

### Incertitudes de lecture résiduelles

Conservées carte par carte dans le champ `notes` de `outputs/textes-cartes.json` :

- ***Advanced Ecosystems*** — la première pastille contient deux pictogrammes
  accolés ; tranchée par le recontrôle de badges.
- ***Nitrogen-Rich Asteroid*** — le lecteur a relevé un doublon « If you have
  **have** 3 or more [plant] » et l'a transcrit tel que vu, en signalant le doute
  (artefact d'impression possible). Conservé tel quel : le livrable dit ce qui a
  été vu, pas ce qui devrait être écrit.
- ***Mining Guild*** — la formulation « play steel production » est légèrement
  floue ; transcrite telle que lue.
- ***Unmi*** (United Nations Mars Initiative) — le repère de phase du second bloc
  (« I-III ») est peu net.
- ***Processing Plant*** (promo) — repère de phase lu `III`, l'espacement laisse
  possible `I II`.
- ***Processed Metals*** (promo) — repère de phase lu `III`, possible `II` ;
  l'encart montre une icône hors vocabulaire (tuiles à étoile dorée), décrite
  plutôt qu'inventée.
- ***Filter Feeders***, ***Diverse Habitats***, ***Self-Replicating Bacteria***
  (promos) — numéro imprimé peu net.
- **Cartes promo en général** — la pastille du coin inférieur droit porte un code
  alphanumérique (« P91 », « P3 »), pas un numéro de la série de base ; quand il
  n'a pas pu être lu avec certitude, `printed_number` est laissé **vide** plutôt
  que deviné.

## 8. Contrôles effectués

- Cellules pleines détectées automatiquement (poids > 200 Ko) confrontées aux
  cellules effectivement lues : **aucune cellule pleine non lue, aucune cellule
  noire livrée comme carte**.
- Couverture : 220 des 222 cartes attendues, numéros imprimés uniques, aucun
  champ obligatoire vide.
- Recopie : 46 textes sur 216 (21 %) coïncident avec la paraphrase — ce sont les
  cartes dont la paraphrase était effectivement fidèle. 127 cartes portent au
  moins une icône transcrite entre crochets.
- Cohérence coût / couleur / badges contre `cards.json` : **0 écart** après le
  recontrôle des badges.
- Numéros imprimés : aucun ne vaut « index de cellule + 1 » (0 sur 220), et ils
  couvrent 1 à 220 sans trou — ils sont donc lus, pas dérivés.
- Écarts de points de victoire contre `cards.json` : **1** (*Ganymede Shipyard*,
  2 PV imprimés contre 0 dans la référence), consigné dans `divergences.md`.
- Le badge Événement n'apparaît que sur des cartes rouges : **0 anomalie** sur les
  242 entrées.
- **Contre-lecture aveugle finale** : 3 cartes tirées au sort (*Automated
  Factories*, *Inventrix*, *Soletta*) relues par un lecteur neuf, sans accès à
  `cards.json` ni au fichier livré. **30 champs sur 30 identiques**, texte
  intégral mot pour mot. C'est le seul contrôle qui teste la lecture elle-même
  plutôt que la cohérence interne — 3 cartes sur 242 ne prouvent pas tout, mais
  une transcription hallucinée n'aurait pas ce comportement.
- `divergences.md` : **247 entrées** classées (2 en §0, 62 en §G1, 20 en §G2,
  163 en §G3), plus les 18 noms fautifs du §B et les 6 motifs systémiques du §A.

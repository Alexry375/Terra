# Les 12 corporations de la boîte de base — diagnostic

Chantier **moteur-corporations-1**. Source du texte : `inputs/textes-cartes.json`,
champ `text` (transcription des planches imprimées). Le champ `description` de
`cards.json` **n'a jamais servi de source** : il est faux de bout en bout sur
quatre des douze corporations (§Le champ `description` ment).

Toutes les commandes de ce document se lancent depuis la racine du workspace
(`workspaces/moteur-corporations-1/`) et leurs sorties sont **collées telles
qu'exécutées**. Raccourci employé ci-dessous :

```
S="outputs/engine/target/release/simulate --cards inputs/cards.json"
```

---

## Comment lire les preuves

| Type de preuve | Ce qu'elle vaut |
|---|---|
| **Sonde** (`--probe-corp …`) | Exécution réelle : la sonde emprunte `flow::build_card_with`, le chemin de `simulate`. C'est la preuve principale. |
| **Test** (`cargo test --release …`) | Exécution réelle sur une PARTIE complète (`setup_game` + `play_round`), politique scriptée. Employée quand l'effet vit dans une phase que la sonde n'exécute pas (phase III forêt, phase V recherche). |
| **Compteur de partie réelle** | 1 000 parties aléatoires : le mécanisme se déclenche-t-il pour de vrai, hors sonde ? |

Trois conventions à connaître avant de lire les sorties :

1. **`--probe-corp` n'applique pas le MC de départ.** L'état de départ fixe de la
   sonde (100 MC, 20 chaleur, 20 plantes) reste son contrat depuis le lot 1, et
   `--probe-mc` reste maître du MC. Le champ `corp.starting_mc` **déclare** la
   valeur imprimée sans l'appliquer. (journal D8)
2. **La corporation est installée avant l'évaluation des prérequis**, sans quoi
   le palier ±1 d'Inventrix serait invisible ; `delta.hand` est calculé sur la
   main d'AVANT installation, pour que la pioche de départ d'Inventrix
   apparaisse. Sans `--probe-corp`, tout est bit à bit comme avant. (journal D9)
3. **`corp` n'est émis que si `--probe-corp` est donné.** Les sondes existantes
   gardent exactement leur sortie d'aujourd'hui. (journal D10)

---

## Tableau récapitulatif

| Corporation | Verdict | Appliqué par le moteur | Manquant (mécanisme absent) |
|---|---|---|---|
| Credicor | `ENCODÉE` | −4 MC sur toute carte de prix imprimé ≥ 20 | — |
| Ecoline | `ENCODÉE` | 1 production de plante ; forêt à 7 plantes au lieu de 8 | — |
| Helion Corporation | `ENCODÉE` | 3 productions de chaleur ; chaleur dépensable comme MC partout, le « may » offert au joueur | — |
| Interplanetary Cinematics | `ENCODÉE` | −2 MC [building] **et** −2 MC [event] | — |
| Inventrix | `ENCODÉE` | 3 cartes piochées à la mise en place ; prérequis d'oxygène/température jugés à ±1 palier de couleur | — |
| Mining Guild | `PARTIELLE` | −2 MC [building] | « each time you play steel production, gain 1 TR » : **l'acier n'est pas modélisé** |
| Phobolog | `PARTIELLE` | −3 MC [space] | « each titanium reduces [space] by 1 MC de plus » : **le titane n'est pas modélisé** |
| Saturn Systems | `ENCODÉE` | −3 MC [space] ; +1 NT par badge [jupiter] joué | — |
| Teractor Corporation | `ENCODÉE` | −3 MC [earth] | — |
| Tharsis Republic | `ENCODÉE` | phase V : +1 carte piochée et +1 gardée | — |
| Thorgate Corporation | `ENCODÉE` | 1 production de chaleur ; −3 MC [energy] | — |
| Unmi | `ENCODÉE` | premier pas de NT de chaque phase : 6 MC → +1 NT, au choix du joueur | — |

**10 `ENCODÉE`, 2 `PARTIELLE`, 0 `HORS-PORTEE`.** Les deux `PARTIELLE` sont les
deux cadrages tranchés par la main (titane, acier) : ils n'ont pas été rouverts.

---

## La pioche : 12, et non 16

Le chargement ne retient dans la pioche de corporations que les entrées
`in_deck_v1` de `cards.json` **dont le nom figure dans la table déclarée
`effects::CORPS`** — laquelle décrit les 12 planches de la boîte de base
(`cards.rs`, `effects.rs`). Deux critères, tous deux nécessaires :

- `in_deck_v1` écarte le jumeau hors-pioche « Teractor Corporation » (48 MC) de
  l'entrée officielle (51 MC) : `cards.json` porte **deux** entrées de ce nom
  exact, et un appariement naïf par nom en retiendrait la mauvaise ou les deux ;
- la présence dans `CORPS` écarte **Apollo Industries, Exocorp, Hyperion Systems
  et Sultira**, `in_deck_v1: true` mais sur aucune planche de la boîte de base
  (aucune n'existe dans `textes-cartes.json`). Leur texte repose entièrement sur
  l'**amélioration de carte Phase**, mécanisme que le moteur saute
  (`phase_upgrades_skipped`, 599 déclenchements sur 1 000 parties de graine 2024
  après ce lot).

**Pourquoi un critère positif et non un filtre par noms en dur** : le filtre
négatif ne dirait pas quoi faire le jour où l'amélioration de phase sera
implémentée. Avec la table, il suffira d'ajouter les quatre entrées à `CORPS` :
elles rentreront dans la pioche par le même chemin, sans toucher au chargement.
Deux garde-fous de chargement le protègent — chaque entrée de `CORPS` doit
résoudre vers **exactement une** corporation `in_deck_v1`, et le compte final
doit valoir `CORPS.len()`.

L'invariant de conservation des corporations (`sim.rs`) porte sur
`db.corporations.len()` : il vaut désormais 12 sans modification. Les badges
rendus par `--dump-corporations` sont écrits **exactement comme dans
`cards.json`** (`Tag::as_str` est l'inverse de `Tag::from_str`), pour qu'ils se
comparent au fichier source sans transformation.

```
$ $S --dump-corporations
{"encoded":true,"name":"Credicor","starting_mc":48,"tags":[]}
{"encoded":true,"name":"Ecoline","starting_mc":27,"tags":["PLANT"]}
{"encoded":true,"name":"Helion Corporation","starting_mc":28,"tags":["SPACE"]}
{"encoded":true,"name":"Interplanetary Cinematics","starting_mc":46,"tags":["BUILDING"]}
{"encoded":true,"name":"Inventrix","starting_mc":33,"tags":["SCIENCE"]}
{"encoded":true,"name":"Mining Guild","starting_mc":27,"tags":["BUILDING","BUILDING"]}
{"encoded":true,"name":"Phobolog","starting_mc":20,"tags":["SPACE"]}
{"encoded":true,"name":"Saturn Systems","starting_mc":24,"tags":["JUPITER"]}
{"encoded":true,"name":"Teractor Corporation","starting_mc":51,"tags":["EARTH"]}
{"encoded":true,"name":"Tharsis Republic","starting_mc":40,"tags":[]}
{"encoded":true,"name":"Thorgate Corporation","starting_mc":45,"tags":["ENERGY"]}
{"encoded":true,"name":"Unmi","starting_mc":35,"tags":[]}
```

---

## Le champ `description` ment — quatre fois sur douze

| Corporation | `description` de `cards.json` | Texte imprimé (`textes-cartes.json`) | Ce que le moteur applique |
|---|---|---|---|
| Interplanetary Cinematics | « 1 steel production. When you play an Event, you pay 2 MC less » | « When you play a **[building]**, you pay 2 MC less. EFFECT: When you play an **[event]**, you pay 2 MC less » | les **deux** réductions ; aucune production d'acier |
| Mining Guild | « 1 Steel income. Whenever you play a card that increases Steel income, gain 1 TR » | « When you play a **[building]**, you pay 2 MC less. EFFECT: Each time you play steel production… » | la réduction [building] ; aucune production d'acier |
| Phobolog | « 1 Titanium income. Each Titanium you have is worth 1 MC extra » | « When you play a **[space]**, you pay 3 MC less. EFFECT: Each titanium… » | la réduction [space] ; aucune production de titane |
| Saturn Systems | « 1 Titanium income. Whenever you play a Jupiter tag… » | « When you play a **[space]**, you pay 3 MC less. EFFECT: Each time you play a [jupiter]… » | la réduction [space] **et** le déclencheur [jupiter] |

Dans les quatre cas la paraphrase **supprime une réduction de coût imprimée** et
**invente une production d'acier ou de titane** que la planche ne porte pas. Le
texte imprimé gagne, et c'est déclaré ici comme le contrat l'exige.

Un cinquième écart, mineur : `cards.json` donne 48 MC à Teractor, la planche
imprimée 51 — c'est l'entrée `in_deck_v1` (51) qui est la bonne, l'autre est le
jumeau hors pioche.

---

## Les services uniques alimentés

Aucune corporation n'a d'implémentation propre. Chaque contribution entre dans
le service que le moteur possédait déjà, ou dans un service neuf **unique** :

| Effet de corporation | Service | Consommé par |
|---|---|---|
| Réduction de coût | `flow::card_discount` | `affordable` (affordabilité) + `build_card_with` (paiement) |
| Production de départ | pistes fixes `*_prod` via `flow::install_corporation` | `flow::phase_production`, à chaque génération |
| Pioche de départ | `flow::install_corporation` | `setup_game` et la sonde |
| Cartes de la phase V | `flow::research_extra` → `research_draw_keep` | `flow::phase_research` |
| Coût en plantes d'une forêt | `flow::forest_plant_cost` (neuf) | `action_options`, `build_forest`, conversion obligatoire de fin de phase III |
| Chaleur comme MC | `flow::spendable_mc` + `flow::top_up_mc_with_heat` (neufs) | `affordable`, `build_card_with`, actions standard, actions de cartes bleues, bonus Unmi |
| Palier de couleur ±1 | `flow::reqs_satisfied` | `requirements_met` et `requirements_met_now` |
| « quand tu joues … » | `flow::fire_play_triggers` (déclencheurs du lot 2) | `build_card_with` |
| Pas de NT bonus | `flow::gain_tr` (neuf, enveloppe `PlayerState::gain_tr`) | les 7 sites de hausse de NT de `flow.rs` |

`install_corporation` est le **seul** point de mise en place d'une corporation :
`setup_game` et `--probe-corp` l'empruntent tous deux.

---

## Credicor — `ENCODÉE`

**Imprimé** : « You start with 48 MC. EFFECT: When you play a card with a printed
cost of 20 MC or more, you pay 4 MC less for it. »

**Moteur** : `Reduction::MinPrice { min: 20, amount: 4 }`, servie par
`flow::card_discount`. Le seuil porte sur le **prix imprimé** (`ProjectCard::price`),
jamais sur un coût déjà réduit — sinon deux réductions se conditionneraient l'une
l'autre selon leur ordre d'application.

**Preuve** — Commercial District, 25 MC imprimés, **payée 21** ; l'effet agit sur
une carte jouée APRÈS la mise en place :

```
$ $S --probe-corp "Credicor" --probe "Commercial District"
{"card":"Commercial District","corp":{"encoded":true,"found":true,"name":"Credicor","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":48},"delta":{...,"mc_prod":4,...},"paid":[21],"played":true,...}
```

Témoin sans corporation — plein tarif :

```
$ $S --probe "Commercial District"
{"card":"Commercial District",...,"paid":[25],"played":true,...}
```

Témoin sous le seuil — Grass, 9 MC, aucune réduction :

```
$ $S --probe-corp "Credicor" --probe "Grass"
{"card":"Grass","corp":{...,"name":"Credicor",...},"delta":{...,"plant_prod":1,"plants":3,...},"paid":[9],"played":true,...}
```

Test : `credicor_reduit_de_4_mc_les_cartes_a_20_mc_ou_plus`.

---

## Ecoline — `ENCODÉE`

**Imprimé** : « You start with 1 plant production and 27 MC. EFFECT: When you
spend plants to gain a forest VP token and raise oxygen, you spend one less
plant. »

**Moteur** : `start_prod.plants = 1` sur la piste FIXE `plant_prod` (donc
re-produite à chaque phase IV) ; `forest_plant_rebate = 1`, servi par
`flow::forest_plant_cost` — le service unique qu'interrogent l'offre de l'action
(`action_options`), le paiement (`build_forest`) et la conversion **obligatoire**
de fin de phase III. Les trois ne peuvent donc pas diverger.

**Preuve 1, production de départ** — une VRAIE phase IV créditée :

```
$ $S --probe-corp "Ecoline" --probe-produce
{"card":"","corp":{"encoded":true,"found":true,"name":"Ecoline","start_prod":{"heat":0,"mc":0,"plants":1},"starting_mc":27},"delta":{...,"plants":1,...},"produced":true,"found":false,"played":false,...}
```

**Preuve 2, la forêt à 7 plantes** — la sonde n'exécute pas la phase III : la
preuve est une PARTIE réelle. Le joueur reçoit 7 plantes ; la conversion
obligatoire de fin de phase III bâtit la forêt avec Ecoline, et ne peut pas la
bâtir avec Credicor (7 < 8) :

```
$ cd outputs/engine && cargo test --release --test lot_corp_tests ecoline_paie_une_foret
test ecoline_paie_une_foret_une_plante_de_moins ... ok
```

Le test assère `forests == 1`, `plants == 0` et `corp_forest_rebates == 1` pour
Ecoline, contre `forests == 0`, `plants == 7`, `corp_forest_rebates == 0` pour le
témoin.

**Preuve 3, en partie réelle** : sur 1 000 parties de graine 2024,
`corp_forest_rebates: 883` — la remise se déclenche pour de vrai, hors sonde.

---

## Helion Corporation — `ENCODÉE`

**Imprimé** : « You start with 3 heat production and 28 MC. EFFECT: You may use
heat as MC. You may not use MC as heat. »

**Moteur** : `start_prod.heat = 3` ; `heat_as_mc = true`. La chaleur est
convertie 1:1 en MC **partout où des MC sont dépensés** — pose de carte, forêt /
température / océan standard, action de carte bleue, pas de NT d'Unmi — par le
couple de services uniques `flow::spendable_mc` (affordabilité) et
`flow::top_up_mc_with_heat` (conversion juste avant la dépense). Il n'existe
aucune dépense de MC qui contourne ce chemin, et l'affordabilité et le paiement
lisent la même fonction.

**Le « may » est un vrai choix du joueur** — là où il en est un. À la pose d'une
carte, le livret offre déjà une alternative (payer en défaussant des cartes à
3 MC) : le moteur présente donc les deux branches par `Policy::choose_option`
(branche 0 = employer la chaleur, l'option imprimée ; branche 1 = y renoncer), et
seulement si la carte reste payable sans la chaleur — convention du lot 3, une
alternative à une seule branche jouable n'est pas une alternative. Partout
ailleurs (actions standard, actions de cartes bleues, pas de NT d'Unmi), aucune
défausse n'est offerte : renoncer à la chaleur y reviendrait à renoncer à
l'action, ce n'est pas une branche jouable, et la chaleur comble alors ce qui
manque. C'est le même chemin de décision que le « may » d'Unmi et que la
réduction payée en microbes du lot 3 — il n'y a pas de « may » à part dans ce
moteur.

**Preuve 1, la chaleur paie une carte** — 0 MC, 20 chaleur, Mohole Area à 18 :

```
$ $S --probe-corp "Helion Corporation" --probe-mc 0 --probe "Mohole Area"
{"card":"Mohole Area","corp":{"encoded":true,"found":true,"name":"Helion Corporation","start_prod":{"heat":3,"mc":0,"plants":0},"starting_mc":28},"delta":{...,"heat":-18,"heat_prod":4,"mc":18,...},"discarded":[0],"paid":[18],"played":true,...}
```

Témoin sans corporation — la même carte n'est pas payable :

```
$ $S --probe-mc 0 --probe "Mohole Area"
{"card":"Mohole Area",...,"paid":[],"played":false,...}
```

**Preuve 2, la chaleur ne sert qu'à combler** — avec 10 MC en poche, seules
**8** chaleur partent (et il n'y a pas de choix à faire : sans monnaie de
défausse, renoncer n'est pas jouable) :

```
$ $S --probe-corp "Helion Corporation" --probe-mc 10 --probe "Mohole Area"
{"card":"Mohole Area",...,"delta":{...,"heat":-8,...,"mc":8,...},"paid":[18],"played":true,...}
```

**Preuve 3, le choix existe et il est réel** — avec 6 cartes de monnaie en main,
la carte est payable des deux façons, donc les deux branches sont offertes.
Branche 0 (l'option imprimée) : 18 chaleur, 0 carte défaussée. Branche 1
(renoncer) : 0 chaleur, 6 cartes défaussées. Même carte, même prix payé.

```
$ $S --probe-corp "Helion Corporation" --probe-mc 0 --probe-filler 6 --probe-choice "0" --probe "Mohole Area"
{"card":"Mohole Area",...,"delta":{...,"heat":-18,...},"discarded":[0],"paid":[18],"played":true,...}

$ $S --probe-corp "Helion Corporation" --probe-mc 0 --probe-filler 6 --probe-choice "1" --probe "Mohole Area"
{"card":"Mohole Area",...,"delta":{...,"heat":0,...},"discarded":[6],"paid":[18],"played":true,...}
```

**Preuve 4, production de départ** :

```
$ $S --probe-corp "Helion Corporation" --probe-produce
{"card":"","corp":{...,"start_prod":{"heat":3,"mc":0,"plants":0},...},"delta":{...,"heat":3,...},"produced":true,...}
```

**Limite nommée et gardée** : la chaleur qu'un prérequis « Requires you to spend
N heat » engage n'est **pas** convertible (`flow::heat_reserved_by`). Sans cette
réserve, Helion payait une carte avec la chaleur que la carte devait ensuite
dépenser, et le moteur cassait — bug trouvé par exécution, corrigé, et couvert
par `helion_ne_convertit_pas_la_chaleur_promise_a_un_prerequis` (journal D14).

**Preuve 5, en partie réelle** : `corp_heat_as_mc: 5510` sur 1 000 parties.

---

## Interplanetary Cinematics — `ENCODÉE`

**Imprimé** : « You start with 46 MC. When you play a [building], you pay 2 MC
less for it. EFFECT: When you play an [event], you pay 2 MC less for it. »

**Moteur** : `Reduction::Tag(Building, 2)` **et** `Reduction::Tag(Event, 2)`,
cumulables sur une carte portant les deux badges. `cards.json` ne décrivait que
la seconde et inventait une production d'acier : le texte imprimé gagne.

**Preuve** — Coal Imports (13 MC, [building]) payée 11 ; Lava Flows (17 MC,
[event]) payée 15 :

```
$ $S --probe-corp "Interplanetary Cinematics" --probe "Coal Imports"
{"card":"Coal Imports","corp":{"encoded":true,"found":true,"name":"Interplanetary Cinematics","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":46},"delta":{...,"heat_prod":3,...},"paid":[11],"played":true,...}

$ $S --probe-corp "Interplanetary Cinematics" --probe "Lava Flows"
{"card":"Lava Flows",...,"delta":{...,"temperature":2,"tr":2},"paid":[15],"played":true,...}
```

Test : `interplanetary_cinematics_reduit_les_building_et_les_event` (il vérifie
en plus le cumul sur Comet, 25 MC [space]+[event] → 23).

---

## Inventrix — `ENCODÉE`

**Imprimé** : « At the start of the game, draw 3 cards. You start with 33 MC.
EFFECT: When playing a card with requirements, you may consider the oxygen or
temperature one color higher or lower. This cannot be modified further by other
effects. »

**Moteur** : `start_draw = 3` (pioche réelle par `flow::draw_card` à la mise en
place) ; `req_color_flex = true`, lu par `flow::reqs_satisfied`. Le prérequis
porte sur un **palier de couleur** (violet / rouge / jaune / blanc, bornes du
module `effects`) : la souplesse est donc de **±1 palier**, jamais de ±1 niveau.
Elle ne touche ni les océans (le texte ne les nomme pas), ni les badges, ni les
dépenses de pose. La souplesse s'ajoute au test exact par un OU logique : sans
Inventrix, le prédicat est bit à bit celui d'avant ce lot.

**Deux clauses du texte sont sans objet dans ce moteur, et c'est déclaré plutôt
que masqué :**

- « This cannot be modified further by other effects » : aucun autre effet ne
  déplace un palier de couleur.
- « the oxygen **or** temperature » (un seul paramètre par carte) : le moteur
  applique la souplesse à chaque prérequis de couleur indépendamment. **Aucune
  carte du deck v1 ne porte à la fois un prérequis de température et un
  prérequis d'oxygène** — l'écart est donc inobservable. Le jour où une telle
  carte existera, Inventrix sera trop permissive d'un cran (journal D16).

**Preuve 1, le palier ±1** — la sonde part de température VIOLETTE ; Bushes exige
le ROUGE. `prereq_ok` passe à `true`, et `delta.hand: 3` montre la pioche de
départ :

```
$ $S --probe-corp "Inventrix" --probe "Bushes"
{"card":"Bushes","corp":{"encoded":true,"found":true,"name":"Inventrix","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":33},"delta":{...,"hand":3,...,"plant_prod":2,"plants":2,...},"paid":[13],"played":true,"prereq_ok":true,"prereq_ok_now":true,...}
```

Témoin sans corporation — `prereq_ok: false` :

```
$ $S --probe "Bushes"
{"card":"Bushes",...,"prereq_ok":false,"prereq_ok_now":false,...}
```

**Preuve 2, la souplesse est d'UN palier** — Trees exige le JAUNE, deux paliers
au-dessus du violet : `prereq_ok` reste `false` même avec Inventrix.

```
$ $S --probe-corp "Inventrix" --probe "Trees"
{"card":"Trees","corp":{...,"name":"Inventrix",...},"delta":{...,"hand":3,...},"paid":[17],"played":true,"prereq_ok":false,"prereq_ok_now":false,...}
```

**Preuve 3, la souplesse change la POSE, pas seulement un drapeau** — en mode
`--probe-strict` la sonde cesse de forcer la pose et n'accepte la carte que si
la règle l'autorise. Avec Inventrix, Bushes est réellement posée ; sans elle,
non :

```
$ $S --probe-corp "Inventrix" --probe-strict "Bushes"
… "played":true,"prereq_ok":true …

$ $S --probe-strict "Bushes"
… "played":false,"prereq_ok":false …
```

**Preuve 4, les 3 cartes en partie réelle** : le test
`inventrix_pioche_3_cartes_a_la_mise_en_place` lance un `setup_game` complet et
assère 11 cartes en main contre 8 pour le témoin.

---

## Mining Guild — `PARTIELLE`

**Imprimé** : « You start with 27 MC. When you play a [building], you pay 2 MC
less for it. EFFECT: Each time you play steel production, excluding this, gain
1 TR. »

**Appliqué** : `Reduction::Tag(Building, 2)`.

**Manquant, nommé** : « each time you play steel production … gain 1 TR ».
L'**acier n'existe pas dans le moteur** — `PlayerState::steel_capacity` est
initialisé à 0 et n'est jamais alimenté ; aucune carte ne déclare de production
d'acier, le lot 2 ayant encodé le TEXTE IMPRIMÉ des cartes à réduction (« pay N
less on <tag> ») plutôt que le revenu d'acier du moteur Java. Il n'existe donc
aucun événement « tu joues une production d'acier » à écouter. Cadrage imposé par
le contrat, non rouvert.

**Ce que ça coûterait** : modéliser l'acier comme ressource — une piste
`steel_prod` par joueur, un `Eff::SteelProd(n)` dans le vocabulaire, la
ré-lecture des ~20 cartes du deck v1 qui portent un revenu d'acier dans le Java,
et le branchement de `DiscountService` (×2 MC par acier sur les [building]), ce
qui **rouvrirait toutes les réductions déjà encodées du lot 2**. C'est un
chantier à part entière, pas un ajout.

**Preuve de la partie appliquée** — Coal Imports (13 MC, [building]) payée 11,
et **aucun** NT inventé :

```
$ $S --probe-corp "Mining Guild" --probe "Coal Imports"
{"card":"Coal Imports","corp":{"encoded":true,"found":true,"name":"Mining Guild","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":27},"delta":{...,"heat_prod":3,...,"tr":0},"paid":[11],"played":true,...}
```

---

## Phobolog — `PARTIELLE`

**Imprimé** : « You start with 20 MC. When you play a [space], you pay 3 MC less
for it. EFFECT: Each titanium you have reduces the cost of [space] cards an
additional 1 MC. »

**Appliqué** : `Reduction::Tag(Space, 3)`.

**Manquant, nommé** : « each titanium you have reduces … 1 MC ». Le **titane
n'est pas modélisé** — `PlayerState::titanium_capacity` est initialisé à 0
(`state.rs`) et n'est jamais alimenté ; son unique lecteur est la récompense
`Industrialist`, qui vaut donc 0. C'était une décision assumée du lot 2 (voir le
commentaire d'en-tête d'`effects.rs` sur Asteroid Mining, encodée « −6 MC Space »
d'après son texte imprimé plutôt que par un revenu de titane). Cadrage imposé par
le contrat, non rouvert.

**Ce que ça coûterait** : mêmes conséquences que pour l'acier ci-dessus, plus la
reprise de la récompense `Industrialist` et du score des parties déjà mesurées.

**Preuve de la partie appliquée** — Ice Asteroid (21 MC, [space]) payée 18 :

```
$ $S --probe-corp "Phobolog" --probe "Ice Asteroid"
{"card":"Ice Asteroid","corp":{"encoded":true,"found":true,"name":"Phobolog","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":20},"delta":{...,"mc":4,"oceans":2,"plants":2,"tr":2},"paid":[18],"played":true,...}
```

---

## Saturn Systems — `ENCODÉE`

**Imprimé** : « You start with 24 MC. When you play a [space], you pay 3 MC less
for it. EFFECT: Each time you play a [jupiter], excluding this, gain 1 TR. »

**Moteur** : `Reduction::Tag(Space, 3)` **et** un `PlayTrigger` du vocabulaire du
lot 2 — `cond: Tag(Jupiter)`, `gains: [TrigGain::Tr(1)]`,
`scale_by_matched_tags: true`. La corporation devient une **source de
déclencheurs** dans `flow::fire_play_triggers`, au même titre qu'une carte en
jeu. « excluding this » ne demande aucun traitement : la corporation n'est jamais
« jouée », donc son propre badge [jupiter] ne déclenche rien. `scale_by_matched_tags`
suit le livret p.9 l.106 (« si la condition est remplie plusieurs fois, résolvez
l'effet autant de fois »). Le pas de NT passe par le service unique
`flow::gain_tr`, donc reste comptabilisé pour l'invariant TR.

**Preuve** — Water Import from Europa, 22 MC, [space] + [jupiter] : payée **19**
(−3) et **+1 NT** :

```
$ $S --probe-corp "Saturn Systems" --probe "Water Import from Europa"
{"card":"Water Import from Europa","corp":{"encoded":true,"found":true,"name":"Saturn Systems","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":24},"delta":{...,"tr":1},"paid":[19],"played":true,...}
```

Le test `saturn_systems_reduit_les_space_et_gagne_1_nt_par_jupiter` ajoute le
témoin sans badge [jupiter] (Ice Asteroid : −3 mais aucun NT du déclencheur).

**Preuve en partie réelle** : `corp_trigger_tr: 242` sur 1 000 parties.

---

## Teractor Corporation — `ENCODÉE`

**Imprimé** : « You start with 51 MC. EFFECT: When you play an [earth], you pay
3 MC less for it. »

**Moteur** : `Reduction::Tag(Earth, 3)`. Le MC de départ est bien **51** (entrée
`in_deck_v1`), pas les 48 du jumeau hors pioche.

**Preuve** — Bribed Comittee (5 MC, [earth] + [event]) payée 2 :

```
$ $S --probe-corp "Teractor Corporation" --probe "Bribed Comittee"
{"card":"Bribed Comittee","corp":{"encoded":true,"found":true,"name":"Teractor Corporation","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":51},"delta":{...,"tr":2},"paid":[2],"played":true,...}
```

---

## Tharsis Republic — `ENCODÉE`

**Imprimé** : « You start with 40 MC. EFFECT: When you draw cards during the
research phase, draw one additional card and keep one additional card. »

**Moteur** : `ResearchBonus { draw: 1, keep: 1 }` — le vocabulaire existait déjà
(Interplanetary Relations, lot 4, texte identique). La corporation alimente le
**même** cumul `flow::research_extra`, consommé par `research_draw_keep` puis par
la seule phase V. Un joueur qui aurait la corporation ET la carte gagnerait 2/2,
comme deux cartes identiques.

**Preuve** — la sonde n'exécute pas la phase V : la preuve est une PARTIE réelle.
Le joueur 0 n'est pas sélectionneur de la phase V (base 2 piochées / 1 gardée) ;
avec Tharsis il pioche 3 et garde **2** :

```
$ cd outputs/engine && cargo test --release --test lot_corp_tests tharsis_republic
test tharsis_republic_pioche_et_garde_une_carte_de_plus_en_phase_v ... ok
```

Le test assère `research_extra == (1, 1)`, `hand == avant + 2` et
`research_extra_draws == 1` (compteur relevé au site de pioche), contre
`(0, 0)`, `avant + 1` et `0` pour le témoin Credicor.

Mise en place vérifiée par sonde :

```
$ $S --probe-corp "Tharsis Republic" --probe-produce
{"card":"","corp":{"encoded":true,"found":true,"name":"Tharsis Republic","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":40},"delta":{...,"mc":5,...},"produced":true,...}
```

**Preuve en partie réelle** : `research_extra_draws` passe de **1 293** (avant ce
lot, seule Interplanetary Relations le nourrissait) à **4 266** sur les mêmes
1 000 parties de graine 2024.

---

## Thorgate Corporation — `ENCODÉE`

**Imprimé** : « You start with 1 heat production and 45 MC. EFFECT: When you play
a [energy], you pay 3 MC less for it. »

**Moteur** : `start_prod.heat = 1` (piste fixe) et `Reduction::Tag(Energy, 3)`.

**Preuve** — Geothermal Power (8 MC, [building] + [energy]) payée 5, et
`start_prod.heat: 1` déclaré :

```
$ $S --probe-corp "Thorgate Corporation" --probe "Geothermal Power"
{"card":"Geothermal Power","corp":{"encoded":true,"found":true,"name":"Thorgate Corporation","start_prod":{"heat":1,"mc":0,"plants":0},"starting_mc":45},"delta":{...,"heat_prod":2,...},"paid":[5],"played":true,...}
```

Production de départ réellement créditée par une phase IV : c'est ce que vérifie
`inputs/checks/05-prod-depart.sh`, et le test
`ecoline_helion_thorgate_produisent_des_la_premiere_phase_iv`.

---

## Unmi — `ENCODÉE`

**Imprimé** : « You start with 35 MC. EFFECT: The first time your TR is raised
each phase, you may pay 6 MC to raise your TR 1 step. »

**Moteur** : `TrBoost { cost_mc: 6, steps: 1 }`, servi par le service unique
`flow::gain_tr(game, db, p, policy)` — une enveloppe de `PlayerState::gain_tr`
(qui garde la comptabilité de l'invariant TR) posée sur les **7 sites de hausse
de NT** de `flow.rs` : oxygène, température, océan, infrastructure, `Eff::Tr`,
`Eff::TrPerTag`, `ActionEff::Tr`. Le drapeau `tr_raised_this_phase` est remis à
zéro au début de chaque phase réellement exécutée, à côté de l'instantané
planétaire — le seul marqueur de début de phase du moteur.

**Le « may » est un vrai choix du joueur**, servi par `Policy::choose_option`
(branche 0 = payer, l'option imprimée ; branche 1 = renoncer), et il n'est
proposé que si les 6 MC sont payables. C'est le même mécanisme que la réduction
payée en microbes du lot 3, donc scriptable par `--probe-choice`.

**Pas de récursion possible** : le drapeau est posé AVANT d'accorder le pas
bonus, et le pas bonus passe par `PlayerState::gain_tr` et non par le service
enveloppant.

**Preuve** — Bribed Comittee donne 2 NT ; le joueur paie 6 MC pour un troisième :

```
$ $S --probe-corp "Unmi" --probe-choice "0" --probe "Bribed Comittee"
{"card":"Bribed Comittee","corp":{"encoded":true,"found":true,"name":"Unmi","start_prod":{"heat":0,"mc":0,"plants":0},"starting_mc":35},"delta":{...,"mc":-6,...,"tr":3},"paid":[5],"played":true,...}
```

Il peut refuser — la branche 1 ne coûte rien et ne rapporte rien :

```
$ $S --probe-corp "Unmi" --probe-choice "1" --probe "Bribed Comittee"
{"card":"Bribed Comittee",...,"delta":{...,"mc":0,...,"tr":2},"paid":[5],"played":true,...}
```

**Une seule fois par phase** — deux cartes à 2 NT chacune, choix « payer » deux
fois : 4 NT des cartes + **un seul** pas acheté, **6 MC une seule fois** :

```
$ $S --probe-corp "Unmi" --probe-choice "0,0" --probe "Bribed Comittee;Release of Inert Gases"
{"card":"Release of Inert Gases",...,"delta":{...,"mc":-6,...,"tr":5},"paid":[5,16],"played":true,...}
```

Le test `unmi_achete_un_pas_de_nt_pour_6_mc_une_fois_par_phase` couvre les trois
cas plus celui du joueur qui n'a pas les 6 MC (aucune offre, rien ne casse).

**Preuve en partie réelle** : `corp_tr_boosts: 797` sur 1 000 parties.

---

## Ce que les corporations font dans une vraie partie

Quatre compteurs neufs, incrémentés à l'endroit exact du mécanisme, remontés dans
la ligne JSON de `simulate`. Ils prouvent que les effets se déclenchent **hors
sonde**, dans le flux aléatoire :

```
$ $S --games 1000 --seed 2024 | tail -1
… "completed":1000, "invariant_violations":0, "truncated":0,
  "corp_forest_rebates":883, "corp_heat_as_mc":5510, "corp_tr_boosts":797,
  "corp_trigger_tr":242 …
```

Les quatre valent **0** en `--effects off` : les effets de corporation sont
coupés comme les effets de cartes (journal D5). La **composition de la pioche**,
elle, reste celle de la boîte de base dans les deux modes — composer la boîte
n'est pas un effet.

---

## Limites déclarées

1. **Acier et titane** ne sont pas modélisés (Mining Guild, Phobolog) — cadrage
   imposé par le contrat, non rouvert.
2. **`--probe-corp` n'applique pas le MC de départ** de la corporation : l'état
   de départ fixe de la sonde reste son contrat, `corp.starting_mc` déclare la
   valeur sans l'appliquer.
3. **Le « may » de Helion** n'est offert en choix qu'à la pose d'une carte, seul
   site où le livret propose une alternative (la défausse-paiement). Ailleurs, la
   chaleur comble ce qui manque sans question posée — renoncer y reviendrait à
   renoncer à l'action.
4. **La chaleur promise à un prérequis « spend N heat » n'est pas convertible**
   par Helion — nécessaire à la cohérence, et couvert par un test.
5. Les corporations **promotionnelles** (6), les cartes projets `ABSENT` (7),
   l'amélioration de carte Phase : hors périmètre, non touchées.

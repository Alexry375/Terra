# Carte d'état — Projet Terra

> Source de vérité du projet. Ancrée au code (`fichier:ligne`) dès qu'il y aura du
> code. [VÉRIFIÉ JJ-MM] = relu à la source ce jour-là. [DÉCLARÉ] = non re-vérifié.

Dernière mise à jour : 2026-07-28 (soir)

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

## ⚠️ LE CHIFFRE QUI COMPTE (28-07 soir) — 194 cartes sur 208 sont encodées

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
| Projets boîte de base | **203 / 208** encodés (5 muettes) |
| Corporations boîte de base | 12 / 12 |
| Projets en configuration cible `base,decouverte` | **208 / 246** (38 muettes) |
| Corporations Découverte | 0 / 4 (écartées, table `effects::CORPS`) |
| Objectifs (tuiles) | 11 / 11 encodés — **1 seuil faux** |
| Récompenses (tuiles) | **5 / 7** fonctionnelles (*Industrialist* ressuscitée le 28-07) |
| Cartes Phase améliorées | **transcrites (10), jamais appliquées** — `state.rs:181` `phase_upgrades` n'est lu nulle part dans `flow.rs` |
| Badges jokers de Découverte | non implantés |
| Interface de jeu | rien |
| IA | rien |

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

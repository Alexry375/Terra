# Architecture — moteur de simulation Ares Expedition (v1, 2 joueurs)

Simulateur d'entraînement du projet Terra : état de jeu, boucle de phases,
production, paramètres globaux, fin de partie, score, règles maison de
mulligan, **couche d'effets déclarative** (chantier moteur-cartes-1 : lot 1 de
63 cartes aux effets complets, VP des 388 cartes extraits, sonde d'audit
`--probe`, interrupteur `--effects on|off`) **étendue au lot 2** (chantier
moteur-cartes-2 : +47 cartes portant réductions de coût, effets déclenchés et
actions de cartes bleues — voir §Lot 2) puis **au lot RESSOURCES** (chantier
moteur-cartes-3 : +28 cartes portant les jetons microbe / animal / science
posés sur les cartes — voir §Ressources posées sur les cartes). Les cartes hors
lot restent des **stubs neutres** (voir §Stubbé). Parties complètes en
politique aléatoire, déterministes à graine fixée.

**Lot 3 — conformité au livret + règles maison** (chantier
moteur-conformite-1) : aucune carte ajoutée, quatre règles corrigées d'après le
livret officiel (prérequis de paramètres sur l'instantané de début de phase ;
bonus construction « piocher avant OU après » ; paiement d'une carte par
défausse de cartes à 3 MC ; conversion obligatoire jugée sur l'instantané) et
deux règles maison appliquées (ordre du tour J1/J2 alterné, égalité sèche sans
départage). Chaque correction est **observable** : compteurs de conformité dans
la ligne JSON, sonde étendue, `--dump-turn-order`. Voir §Compteurs de
conformité, §Ordre du tour, §Sonde.

## Couche d'effets (chantier cartes-1, `src/effects.rs` + `src/probe.rs`)

- **Encodage déclaratif** : chaque carte du lot 1 est une entrée
  `(nom, CardEffects { reqs, effects })` de la table statique `LOT1`
  (`src/effects.rs`) — données pures, interprétées par le moteur, zéro logique
  par carte. Table justifiée au journal (B1) : vérifiée à la compilation, le
  chargement échoue si un nom ne résout pas exactement une carte de la base.
- **Vocabulaire v1** — prérequis `Req` : palier de couleur d'un paramètre
  global (bornes en niveaux, sémantique Java `PlanetFactory`, journal B5 :
  temp P=0-5/R=6-10/Y=11-15/W=16-19, oxy P=0-2/R=3-6/Y=7-11/W=12-14, océans
  min), n tags d'un type, dépenses à la pose (`SpendHeat/SpendPlants/SpendTr`) ;
  effets `Eff` : gains immédiats (MC, chaleur, plantes, pioche), hausses de
  production (MC, chaleur, plantes, cartes), température/oxygène/océan +n
  (réutilisent `raise_*`/`reveal_ocean` du squelette : TR + caps sur
  l'instantané de phase), TR +n, infrastructure +n (Grain Silos), gain
  conditionnel de plantes sur tags (Nitrogen-Rich Asteroid).
- **Branchement réel** : prérequis vérifiés AVANT de jouer (`requirements_met`
  filtre `affordable`) ; dépenses + effets appliqués à la pose dans
  `flow::build_card` — **chemin unique** pour `simulate`, la sonde et les
  tests ; productions comptées par la phase de production existante
  (champs `*_prod`).
- **Interrupteur** : `simulate --effects on|off` (défaut `on`) →
  `CardsDb.effects_on`. `off` = squelette intégral : ni prérequis, ni effets,
  ni VP de cartes au score (non-régression, vérifiée par le check 03).
- **Le texte imprimé gagne** : conflits texte/Java au journal et dans
  `outputs/lot1.md` (Nitrogen-Rich Asteroid : `== 3` en Java vs « 3 or more »
  imprimé → `>= 3` implémenté).

## Lot 2 — réductions, déclencheurs, actions (chantier cartes-2)

Le vocabulaire de `src/effects.rs` gagne quatre champs sur `CardEffects`
(`reductions`, `play_triggers`, `global_triggers`, `action`) et 47 entrées
NEUVES dans la table `LOT1` (les 10 imposées incluses ; correspondances,
encodages et conflits texte/Java dans `outputs/lot2.md`). Le texte imprimé
gagne toujours.

- **(A) Réductions de coût** (`Reduction::AnyCard(n)` / `Tag(tag, n)`) :
  service UNIQUE `flow::card_discount(game, db, p, card_id)` = somme des
  réductions de TOUTES les cartes persistantes déjà en jeu du joueur (calculée
  AVANT la mise en jeu, donc une carte ne se réduit jamais elle-même). Consommé
  par `affordable` (filtre d'affordabilité) ET par `build_card` (paiement) —
  chemin unique, pas de logique parallèle. `paid = max(0, prix − remise_phase −
  remise_cartes)`, `assert paid >= 0`. **Titane/acier** : le Java
  `DiscountService` consomme `steelIncome`/`titaniumIncome` COMME réductions
  (×2 MC/BUILDING, ×3 MC/SPACE) ; Asteroid Mining (Java = titane 2 → −6 MC
  Space) est encodé selon son TEXTE IMPRIMÉ « −6 MC Space » sans modéliser le
  titane ; les cartes qui SUIVENT l'acier/titane (AquiferPumping, Solarpunk)
  sont hors lot.
- **(B) Déclencheurs de pose** (`PlayTrigger { cond, gains,
  scale_by_matched_tags, include_self }`) : évalués dans `build_card` APRÈS la
  mise en jeu, sur les tags de la carte POSÉE, pour toutes les cartes
  persistantes en jeu du joueur. `include_self` = Java
  `onBuiltEffectApplicableToItself` (Olympus « including this » = true, défaut
  false) ; `scale_by_matched_tags` = Java `countCardTags` (pioche par tag).
  **Déclencheurs globaux** (`GlobalTrigger::OnRaiseTemperature/OnFlipOcean`)
  fixés dans `raise_temperature`/`reveal_ocean` au moment où le TR est accordé,
  pour le seul joueur agissant (Volcanic Soil +2 plantes/pas de température,
  Arctic Algae +4 plantes/océan).
- **(C) Actions de cartes bleues** : le stub neutre `ActionOpt::BlueAction` de
  `phase_action` devient un vrai moteur (`flow::apply_blue_action`). Chaque
  carte bleue en jeu offre 1 activation par phase III (INCHANGÉ) ; à
  l'activation, si les effets sont ON et l'action définie et payable, coût et
  effet sont appliqués et le compteur `blue_actions` incrémenté ; sinon no-op
  (`--effects off` = squelette « à blanc », `blue_actions: 0`). Coûts (chaleur/
  MC/plantes), effets (pioche/plantes/MC/TR/oxygène), variantes variables
  « up to X » (`HeatToMc`, `DiscardDraw`, coût réduit par tags Energy ou par
  cartes bleues) dont le montant est tiré par `Policy::action_amount` (méthode
  par défaut du trait → aucune impl existante modifiée). Le compteur vit dans
  `GameState.blue_actions`, agrégé en `SimSummary.blue_actions` (champ JSON
  `blue_actions` de `simulate`).

## VP des cartes et score

`cards_v2.json` (Mission A, script `outputs/work/extract_vp.py`) ajoute aux
388 cartes : `vp` (entier, `getWinningPoints()` Java, 0 par défaut) et
`vp_dynamic` (`null` ou `{type, resources, points}` recopié du builder
`WinPointsInfo`). Le score (effets ON) ajoute par carte jouée : `vp` fixes +
VP dynamiques calculables en v1 — formule Java `floor(n / resources) × points`
avec n = tags Jupiter (`JUPITER`), tags Terre (`EARTH`), forêts (`FOREST`),
cartes bleues jouées (`BLUE_CARD`), cartes jouées (`ANY_CARD`). Depuis le
chantier cartes-3, les types portant sur des **ressources posées sur les
cartes** (`ANIMAL`, `MICROBE`, `SCIENCE`) sont RÉELS : n = ressources posées sur
CETTE carte (voir §Ressources posées sur les cartes). Tout se calcule dans
`flow::card_points`, qui renvoie `(total, part venant des ressources)` — il n'y
a pas de second chemin de score. `VpKind::Unsupported` reste 0.

## Sonde d'audit v2 (`--probe` séquence, `--probe-action`)

État de départ fixe (`src/probe.rs`) : joueur 1 sans corporation, 100 MC,
20 chaleur, 20 plantes, productions 0, TR 5, paramètres globaux au départ, les
cartes nommées seules en main (dans l'ordre) ; pioche = cartes v1 restantes en
ordre d'index, tuiles océan **non mélangées** (1re = +2 plantes, 2e = +4 MC).
Même chemin de pose que `simulate` (`flow::build_card`).

- **`--probe "<A>;<B>;…"`** : pose FORCÉE des cartes DANS L'ORDRE (séparateur
  `;`). Rétro-compatible — une seule carte = comportement exact du lot 1.
  Nouveau champ `"paid": [int, …]` = prix effectivement payé de chaque carte
  (après réductions, ≥ 0). `delta` = cumul depuis l'état de départ, HORS prix
  payés (`delta.mc` réintègre le total réellement déboursé — identique au lot 1
  quand une seule carte est en jeu, remise 0) ; `found`/`in_lot`/`prereq_ok`/
  `played`/`vp` portent sur la DERNIÈRE carte. Les réductions et déclencheurs
  des cartes posées plus tôt s'appliquent aux poses suivantes — c'est le but de
  la séquence (observer réductions et « when you play … »).
- **`--probe-action "<nom>"`** : pose la carte puis active son action UNE fois
  si elle est payable. JSON `{card, found, in_lot, has_action, action_applied,
  delta{…}}` où `delta` isole L'ACTION SEULE (état après pose → après action,
  dépenses de l'action comprises, ex. Development Center : `heat: -2,
  hand: +1`). Carte sans action : `has_action:false, action_applied:false`,
  delta nul.

`found:false` si le nom est inconnu ; carte hors lot ou effets coupés :
`in_lot:false`, réductions/déclencheurs/actions inertes.

### Extensions du lot 3

Deux champs s'ajoutent au JSON de `--probe`, trois options à la CLI. Le
comportement SANS option est celui du lot 2, à l'identique.

```
{"card":"<nom>","found":true,"in_lot":true,"prereq_ok":true,
 "prereq_ok_now":true,"played":true,
 "paid":[<int>,…],"discarded":[<int>,…],"delta":{…},"vp":<int>}
```

- **`discarded`** : nombre de cartes défaussées pour payer **chaque** carte de
  la séquence, dans l'ordre. Valeur renvoyée par `flow::build_card_with` — la
  sonde ne recalcule rien.
- **`prereq_ok_now`** : prérequis de la **dernière** carte évalués à l'état
  **courant**, juste avant sa pose (`flow::requirements_met_now`).
  `prereq_ok` garde son sens du lot 1 : évalué sur l'état de DÉPART de la
  sonde — qui est aussi son instantané, donc la lecture « règle du jeu ». Les
  deux diffèrent dès qu'une carte de la séquence a fait bouger un paramètre
  (ex. `--probe "Ice Asteroid;Great Dam"` → `prereq_ok:false`,
  `prereq_ok_now:true`).
- **`--probe-mc <n>`** : MC de départ du joueur sondé (défaut 100).
- **`--probe-filler <n>`** : `n` cartes supplémentaires en main, prises en tête
  de pioche, servant uniquement de monnaie de défausse (défaut 0). En présence
  de monnaie, `delta.hand` compte TOUT ce qui quitte la main (cartes posées +
  cartes défaussées pour payer) ; sans monnaie, la convention du lot 1/2 est
  conservée (`delta.hand` exclut la carte jouée).
- **`--probe-strict`** : la sonde cesse de forcer la pose. Chaque carte n'est
  posée que si ses prérequis sont remplis SELON LA RÈGLE (paramètres sur
  l'instantané = l'état de départ ; tags et dépenses à l'état courant) ET si
  elle est payable (MC + défausse, prédicat `flow::payable`). Premier refus =
  la séquence s'arrête et `played` vaut `false`. C'est le seul moyen d'observer
  la règle de l'instantané carte par carte, et il emprunte le chemin réel.

### Extensions du chantier cartes-3

Deux champs et deux options s'ajoutent, aux DEUX sondes (`--probe` **et**
`--probe-action` — sans quoi les actions à ressources ne seraient pas
observables). Sans option nouvelle, la sortie est celle du lot précédent, à
l'identique.

- **`resources`** : `[{"card","kind":"microbe|animal|science","n"}]`, toutes les
  cartes PORTEUSES du joueur sondé après la séquence, celles à 0 comprises,
  **triées par nom de carte**. Lu sur `PlayerState::card_resources` ; la sonde
  n'écrit jamais de ressource.
- **`target_error`** : première cible imposée absente des candidats (ou nom de
  carte inconnu), `null` sinon. Une cible imposée introuvable n'est jamais
  remplacée en silence : l'effet est sauté et l'erreur remonte ici.
- **`--probe-choice "1,0,2"`** : pile de réponses imposées à
  `Policy::choose_option`, consommée dans l'ordre ; épuisée → comportement par
  défaut.
- **`--probe-target "Tardigrades;Birds"`** : pile de cartes imposées à
  `Policy::choose_res_target` puis `Policy::choose_res_source`, consommée dans
  l'ordre d'appel.

Les deux options imposent les réponses de la **politique**, pas des valeurs au
moteur : la sonde emprunte donc exactement les mêmes points de décision que
`simulate` (`probe::ProbePolicy` délègue à `RandomPolicy` dès qu'une pile est
épuisée).

Le champ **`vp`** vaut désormais : VP fixes de la dernière carte **+** points de
victoire venant des ressources de toutes les cartes en jeu, lus sur
`flow::card_points`. Les VP dynamiques non liés aux ressources (JUPITER,
BLUE_CARD…) restent hors de ce champ, comme au lot 2.

## Ressources posées sur les cartes (chantier cartes-3)

Les jetons **microbe / animal / science** empilés sur une carte en jeu. 28
cartes neuves dans la table `LOT1` (110 + 28 = 138 entrées) ; correspondances,
encodages, choix exposés et limites dans `outputs/lot3.md`. Le texte imprimé
gagne toujours.

- **Stockage** : `PlayerState::card_resources: BTreeMap<u16, u32>` (identifiant
  de carte → quantité). Pas de table de hachage : l'ordre d'itération ordonne
  la liste de candidats présentée à la politique, donc les tirages du RNG, donc
  la reproductibilité à graine fixe. Une carte n'y entre que si elle **porte**
  un type (`CardEffects::holds`), à sa pose et **à 0** (`Player.initResources`
  du Java) : une carte porteuse vide est déjà une cible valide, une carte non
  porteuse n'est jamais un réceptacle.
- **Service unique** : `flow::add_resources` / `flow::remove_resources`, seuls
  points d'écriture, empruntés par la pose, les déclencheurs, les actions et la
  sonde — même discipline que `card_discount` au lot 2. Ils incrémentent
  `res_added` / `res_removed` au moment exact de l'opération, et assèrent que
  la carte est bien une porteuse en jeu du joueur.
- **Vocabulaire** (`effects.rs`) : `ResKind`, champ `holds`, `ResPut { target:
  SelfCard | Another | Any, kinds, amount: Fixed | ByKind }`, `ResEff { Gain,
  Put, RemoveSelf, RemoveAny, PhaseUpgrade }`, `ResStep { Do, Choose }` et le
  champ `on_build` ; `TrigGain::ResSelf`/`Choose` et `TrigCond::AnyOfTags` pour
  les déclencheurs de pose ; `GlobalTrigger::OnRaiseOxygen`/`OnBuildForest`
  (Herbivores, Small Animals) ; `Action::Res` pour les actions à ressources
  (dix cartes, dont Symbiotic Fungus, Extreme-Cold Fungus et Conserved Biome,
  reclassées d'effet de pose en action au round 2 d'après le scan des cartes
  imprimées) ;
  `Reduction::PayResources` pour la réduction payée en microbes.
- **Choix du joueur** : trois méthodes **à implémentation par défaut** sur le
  trait `Policy` — `choose_option` (branches d'une alternative, numérotées dans
  l'ordre du TEXTE IMPRIMÉ après filtrage des branches injouables),
  `choose_res_target` (carte qui reçoit), `choose_res_source` (carte sur
  laquelle retirer). Aucune politique existante n'est modifiée. Conventions :
  `choose_option` n'est appelée qu'à partir de 2 branches jouables ;
  `choose_res_target` est appelée même à un seul candidat ; un indice hors
  bornes vaut **renoncement explicite** (l'effet est sauté), ce dont seule la
  sonde se sert pour signaler une cible imposée introuvable.
- **Absence de cible** = effet de POSE sauté, **sans compensation**, compté dans
  `res_targets_missing`. Une ACTION dont la seule branche est injouable faute de
  cible ne s'applique simplement pas (`action_applied` faux) et n'est pas
  comptée : elle reste offerte à la génération suivante, exactement comme une
  action dont le coût n'est pas payable.
- **Amélioration de carte Phase** (Cryogenic Shipment, action de Fibrous
  Composite Material) : l'effet était perdu et compté dans
  `phase_upgrades_skipped` jusqu'au chantier `decouverte-phases`. Il est
  désormais appliqué par `flow::apply_phase_upgrade` — voir §Cartes Phase
  améliorées. Ces deux cartes sont donc intégralement gérées.
- **Score** : `VpKind::Animal/Microbe/Science` ne valent plus 0. Ils comptent
  les ressources posées sur CETTE carte, dans `flow::card_points` — unique
  implémentation, qui renvoie `(total, part venant des ressources)` et que
  consomment à la fois `flow::score_parts` (score de partie, compteur
  `vp_from_resources`) et la sonde. Pas de second chemin de score.
- **Résolution des noms de cartes** : `CardsDb::resolve_card`. `cards.json`
  contient des homonymes `Buffed…` (variantes maison, `in_deck_v1: false`),
  parfois AVANT la carte officielle et à un prix différent. À nom multiple,
  seule l'entrée du deck v1 est canonique ; elle seule reçoit l'effet, le
  jumeau reste un stub. Chemin unique (table d'effets, sonde, tests).

## Lot 5 — les 33 muettes de la boîte de base (chantier cartes-5)

33 cartes projets de la boîte de base qui n'avaient **aucun** encodage (62 avant
le lot, **29** après) rejoignent la table `LOT1` : 20 productions pures,
9 effets immédiats éventuellement suivis d'une production, 4 gains de forêt.
Source du texte : `inputs/textes-cartes.json` champ `text` (transcription des
cartons), jamais le champ `description` de `cards.json`. Correspondances carte
par carte, texte imprimé cité et traces de sonde : `outputs/cartes5.md`.

Deux briques de vocabulaire seulement, et **aucune** ligne de logique par carte :

- **`Eff::Forest(n)`** — gain de n jetons PV Forêt **sans paiement**. Appliqué
  par `flow::gain_forest`, qui est désormais **l'unique** écriture de
  `PlayerState::forests` du moteur : `forests += 1`, un pas d'oxygène par
  `raise_oxygen` (donc +1 NT, cap sur l'instantané de phase, déclencheur « when
  you raise oxygen »), puis l'événement `GlobalEvent::Forest`. L'action standard
  payée (`flow::build_forest`) paie d'abord, puis appelle la même fonction — le
  **paiement reste dehors**, parce que la remise d'Ecoline porte sur « lorsque
  vous DÉPENSEZ DES PLANTES pour gagner un jeton PV Forêt ».
  - **R1** : « Gain a forest VP **and** raise oxygen 1 step » n'est pas
    l'addition de deux effets, c'est la description d'un gain de forêt — même
    formule que l'action standard du livret (p. 14, l. 379), pour UN pas
    d'oxygène. *Plantation* = `Forest(2)` : 2 forêts, 2 pas d'oxygène, jamais 4.
  - **R2** : `GlobalTrigger::OnBuildForest` porte en réalité « **when you gain a
    forest VP** » (texte imprimé de *Small Animals*) ; sa doc, qui disait
    « build », est corrigée. L'événement se lève **une fois par forêt gagnée**
    (livret l. 106), quelle qu'en soit l'origine : *Plantation* pose 2 animaux.
- **`Req::TrMin(n)`** — « Requires you to have N or more TR » (*Energy
  Storage*). Le NT est une ressource de JOUEUR : le seuil est évalué à l'état
  COURANT dans `flow::reqs_satisfied`, avec `Tags` et `Spend*`, jamais sur
  l'instantané de début de phase. `TrMin` **teste** le NT ; `SpendTr` le dépense.

Ni `src/probe.rs`, ni `src/bin/simulate.rs`, ni le recensement n'ont été
touchés : `--probe` exposait déjà `delta.forests` / `delta.oxygen` /
`delta.*_prod` / `resources`, et `effets_geres` est **dérivé** de l'encodage par
`cards::encodage_integral`.

`tests/lot5_tests.rs` — 60 tests : un par carte du lot (33, état de jeu comparé
au texte imprimé via la sonde), R1 (dont le témoin de l'action standard payée en
partie réelle), R2 (dont un témoin négatif), le contrôle STRUCTUREL du chemin
unique (`forests += 1` n'apparaît qu'une fois dans `flow.rs`, et dans
`gain_forest`), le seuil de NT en mode strict, l'encaissement par la VRAIE phase
IV à chaque génération, `--effects off`, le recensement des 29 restantes, le
coût imprimé des 33, l'absence de tout nom de carte dans `src/*.rs` hors table
d'effets, et le déterminisme.

## Lot 6 — actions bleues et manipulation de la main (chantier cartes-6)

11 cartes projets de la boîte de base rejoignent la table `LOT1` (29 muettes
avant le lot, **18** après ; 66 → **55** en `base,decouverte`). Source du texte :
`inputs/textes-cartes.json`, champs `text`, `requirement`, `production` et
`vp_printed` — jamais `description` de `cards.json`. Correspondances carte par
carte, traces de sonde et divergences : `outputs/cartes6.md`.

**Six briques de vocabulaire**, aucune ligne de logique par carte :

- **`Req::OxyMax(n)`** — « Requires red oxygen or lower » (*Colonizer Training
  Camp*). Prérequis de PARAMÈTRE : jugé sur l'instantané de début de phase comme
  les autres, souplesse Inventrix de ±1 palier comme `TempMax`. Constante
  `OXY_R_MAX = 6`.
- **`CardEffects::phase_bonus: Option<PhaseBonus>`** — « *If **you** chose the
  action phase this round … ». `PhaseBonus { phase, cost, extra }` : `extra` =
  effets ajoutés (*Community Gardens*, *Hydro-Electric Energy*), `cost` = coût de
  REMPLACEMENT (*Wood Burning Stoves*, « spend 3 plants instead »). Lu sur
  `PlayerState::chosen_phase` du joueur QUI ACTIVE, au moment de l'activation —
  jamais celle de l'adversaire. N'a de sens que sur `Action::Fixed` ; un test
  structurel le garantit.
- **`ActionCost::DiscardCard(n)`** — coût d'action payé en CARTES (*Farming
  Co-ops*). Payable ssi la main en porte assez ; les cartes sont choisies par
  `Policy::discard_down` et rejoignent la défausse commune.
- **`Action::SpendUpTo { spend, gain, cap }`** — « Spend up to N <res> to gain
  that amount of <res> » (*Greenhouses*). Le plafond IMPRIMÉ rend les montants
  énumérables : ce sont des branches (1..N), filtrées par ce que le joueur
  possède, tranchées par `Policy::choose_option` (convention du lot 3). Un
  montant nul n'est pas une branche du texte imprimé. `Action::HeatToMc` (« spend
  ANY amount », *Power Infrastructure*) reste inchangée : sans plafond, le
  montant ne s'énumère pas.
- **`Eff::DrawDiscard { draw, discard, from_drawn }`** — « piochez n puis
  défaussez d », brique UNIQUE des trois cartes du groupe C (*Business
  Contracts* 4/2, *Invention Contest* 3/2, *Microprocessors* 2/1).
  `from_drawn` porte la seule différence de texte : « Keep one **of them** »
  restreint la défausse aux cartes piochées, « Then, discard N cards » porte sur
  la main entière. Le choix passe par `Policy::discard_down`.
- **`ActionEff::Reveal(Reveal { n, keep, take, mc_per_discarded })`** —
  révélation du dessus de la pioche (*Advanced Screening Tech* :
  `AnyOfTags([Science, Plant])`, take 1 ; *Brainstorming Session* :
  `ColorIsNot(Green)`, take 1, 1 MC par révélée non gardée). Les cartes sortent
  réellement de la pioche par `flow::draw_card` (remélange compris) et les non
  gardées rejoignent la défausse : la conservation des cartes reste vraie. Le
  choix de la gardée passe par `Policy::research_keep`.

Deux ajouts **déclarés, non mécaniques** : `ActionEff::Heat(n)` et
`ActionEff::Temperature(n)`, valeurs de plus dans l'énumération d'effets
d'action, qui empruntent les services existants (réserve de chaleur,
`raise_temperature`).

`flow::apply_action_eff` est extrait de `apply_blue_action` pour que les effets
de l'action et ceux ajoutés par le bonus de phase empruntent un chemin unique ;
`flow::discard_from_hand` est l'unique point d'écriture des deux défausses
neuves.

### Sonde et compteurs du lot 6

- **`--probe-phase <1..5>`** : fixe la phase choisie par le joueur sondé dans
  l'état de départ, avant la pose et avant l'action, et n'écrit rien d'autre.
  Sans l'option, la sortie est celle des lots précédents à l'identique
  (`ProbeOptions::phase = 0`). `--probe-filler` et `--probe-choice` s'appliquent
  désormais aussi à `--probe-action` (`probe::run_probe_action_opts`), sans quoi
  un coût payé en cartes et un montant « jusqu'à n » ne seraient pas observables.
- Quatre compteurs d'audit dans la ligne JSON de `simulate`, incrémentés au site
  exact du mécanisme, tous nuls en `--effects off` : `action_phase_bonuses`,
  `action_discard_costs`, `draw_discard_discards`, `cards_revealed`.

`tests/lot6_tests.rs` — 57 tests : un ou plusieurs par carte du lot, le bonus de
phase des deux côtés (et le témoin « c'est l'adversaire qui a choisi »), la
révélation en flux réel sur un dessus de pioche composé, la différence
`from_drawn` entre les trois cartes du groupe C, la conservation des cartes, les
quatre compteurs en partie réelle et à zéro effets coupés, le recensement (18 et
55), le déterminisme, l'inertie de *Power Infrastructure* et l'absence de tout
nom de carte du lot dans `src/`.

## Corporations (chantier corpo-1)

Les **12 planches de corporation de la boîte de base** ont leurs effets. Source
du texte : `inputs/textes-cartes.json` champ `text` — surtout PAS le champ
`description` de `cards.json`, faux de bout en bout sur quatre d'entre elles
(Interplanetary Cinematics, Mining Guild, Phobolog, Saturn Systems : la
paraphrase supprime une réduction imprimée et invente une production d'acier ou
de titane). Diagnostic complet, verdicts et preuves : `outputs/corporations.md`.

- **Table déclarative `effects::CORPS`** (12 entrées `(nom, CorpEffects)`, macro
  `corp!`), même discipline que `LOT1` : des données interprétées par `flow`,
  zéro exception codée par corporation. `CorpEffects` réemploie le vocabulaire
  des cartes (`Reduction`, `PlayTrigger`, `ResearchBonus`, productions fixes) et
  l'étend de quatre champs que le texte imprimé exige : `forest_plant_rebate`
  (Ecoline), `heat_as_mc` (Helion), `req_color_flex` (Inventrix), `tr_boost`
  (Unmi). Deux ajouts au vocabulaire des cartes : `Reduction::MinPrice { min,
  amount }` (Credicor, seuil sur le prix IMPRIMÉ) et `TrigGain::Tr(n)`
  (Saturn Systems).
- **La pioche de corporations, c'est cette table.** `CardsDb::load` ne retient
  que les entrées `in_deck_v1` de `cards.json` dont le nom y figure : le double
  critère écarte à la fois le jumeau hors-pioche « Teractor Corporation » (48 MC
  contre 51) et les quatre corporations `in_deck_v1` sans planche imprimée
  (Apollo Industries, Exocorp, Hyperion Systems, Sultira), toutes bâties sur
  l'amélioration de carte Phase que le moteur saute. Garde-fous : chaque entrée
  de `CORPS` doit résoudre vers exactement une corporation `in_deck_v1`, et le
  compte final doit valoir `CORPS.len()`. *Branchement du chantier « améliorations
  de phase »* : ajouter les quatre entrées à `CORPS` suffit à les remettre dans
  la pioche. L'invariant de conservation (`sim.rs`) porte sur
  `db.corporations.len()` : il vaut 12 sans modification.
- **Mise en place** : service unique `flow::install_corporation` (MC de départ,
  badges, production de départ sur les pistes FIXES `*_prod`, pioche de départ),
  emprunté par `setup_game` ET par `--probe-corp`. Les productions de départ
  d'Ecoline (1 plante), Helion (3 chaleur) et Thorgate (1 chaleur) sont donc
  reprises par `phase_production` à CHAQUE génération, jamais une seule fois.
- **Services alimentés, jamais dupliqués** : `card_discount` (réductions),
  `research_extra` (Tharsis Republic, même cumul qu'Interplanetary Relations),
  `fire_play_triggers` (la corporation est une source de déclencheurs, `src =
  None` puisqu'elle ne porte pas de ressources), `phase_production`. Trois
  services NEUFS, chacun unique : `forest_plant_cost` (offre de l'action,
  paiement et conversion obligatoire lisent le même coût), `spendable_mc` /
  `top_up_mc_with_heat` (Helion : la chaleur est du MC partout où des MC se
  dépensent — pose, actions standard, actions bleues, bonus Unmi), et `gain_tr`,
  enveloppe de `PlayerState::gain_tr` posée sur les 7 sites de hausse de NT, qui
  porte le `TrBoost` d'Unmi.
- **Chaleur réservée** (`heat_reserved_by`) : la chaleur qu'un prérequis
  « Requires you to spend N heat » engage n'est pas convertible en MC par Helion,
  à l'affordabilité comme au paiement — sans quoi la dépense de pose deviendrait
  impayable.
- **Choix du joueur** : le `TrBoost` d'Unmi passe par `Policy::choose_option`
  (branche 0 = payer, l'option imprimée ; branche 1 = renoncer), offert seulement
  si les 6 MC sont payables — même mécanique que la réduction payée en microbes
  du lot 3, donc scriptable par `--probe-choice`. Le drapeau
  `PlayerState::tr_raised_this_phase` (« the FIRST TIME each phase ») est remis à
  zéro au début de chaque phase exécutée, à côté de `snapshot_planet`.
- **`--effects off`** coupe TOUS les effets de corporation, productions de départ
  comprises. Le MC de départ et les badges restent (c'est la planche), et la
  composition de la pioche aussi (composer la boîte n'est pas un effet).
- **Quatre compteurs d'audit** dans la ligne JSON, incrémentés au site du
  mécanisme, nuls en `--effects off` : `corp_heat_as_mc`, `corp_forest_rebates`,
  `corp_tr_boosts`, `corp_trigger_tr`.

### Options de sonde du chantier corpo-1

- **`--dump-corporations`** : une ligne JSON par corporation de la pioche, dans
  l'ordre de chargement — `{"name","starting_mc","tags","encoded"}`. `encoded` =
  la corporation porte un effet dans `CORPS`.
- **`--probe-corp "<nom>"`** : impose la corporation au joueur sondé, à la place
  du tirage, par `install_corporation`. S'ajoute à `--probe`, `--probe-produce`
  et `--probe-action`. La sortie gagne un objet `corp`
  (`{"name","found","encoded","starting_mc","start_prod"}`) — **émis uniquement
  avec cette option**, pour que les sondes existantes gardent exactement leur
  sortie d'avant. Nom inconnu → `found: false`, la sonde ne s'interrompt pas.
  Employée sans `--probe` mais avec `--probe-produce`, elle met la corporation en
  place et exécute une phase IV : `card` est vide, `played` faux, et `delta`
  porte ce que la production a crédité.
  Deux conventions : le MC du joueur sondé reste celui de `--probe-mc` (100 par
  défaut), `corp.starting_mc` DÉCLARE la valeur imprimée sans l'appliquer ; la
  corporation est installée avant l'évaluation des prérequis (sans quoi le palier
  ±1 d'Inventrix serait invisible) et `delta.hand` est calculé sur la main
  d'avant installation (pour que la pioche de départ d'Inventrix y apparaisse).

## Cartes Phase améliorées (chantier decouverte-phases)

L'extension Découverte double chacune des cinq cartes Phase : chaque joueur
reçoit **dix** cartes Phase améliorées (deux options, A et B, par phase) et
certaines cartes Projet ou Corporation lui permettent d'en échanger une contre
la carte Phase correspondante de sa main. La carte améliorée donne un **BONUS de
sélectionneur** meilleur ; sa COMPÉTENCE est identique mot pour mot.

- **Les onze cartes Phase sont des DONNÉES** — `effects::PHASE_BASE` (les cinq
  de la boîte de base) et `effects::PHASE_UPGRADED[phase][variante]` (les dix
  améliorées). Chaque entrée porte le nom imprimé et des **branches** de bonus ;
  plusieurs branches = un « ou » du texte imprimé, tranché par `Policy`.
- **Un point de calcul unique** — `flow::selector_bonus(db, pl, phase)`, fonction
  pure. Elle rend zéro hors sélectionneur, ignore les améliorations en
  `--effects off`, et lit **une seule** entrée de table : le bonus amélioré
  REMPLACE celui de base (livret l. 64), le cumul n'est pas exprimable. Les cinq
  phases y passent (`phase_development`, `phase_construction`, `phase_action`,
  `phase_production`, `research_base`) ; plus aucune constante de bonus n'est
  lue dans le flux.
- **L'octroi** — `flow::apply_phase_upgrade`, seul chemin, appelé par
  `ResEff::PhaseUpgrade`. Les dix cartes moins la variante déjà en place sont
  proposées à `Policy::choose_option` : améliorer une phase déjà améliorée
  bascule A ↔ B (l. 66) et n'est jamais un gaspillage. `phase_upgrades_skipped`
  ne peut donc plus bouger.
- **Les poses supplémentaires** (I-B « une seconde verte à 12 MC imprimés ou
  moins », II-A et II-B « une seconde bleue ou rouge ») empruntent le
  `BuildGrant` et la file `pending_builds` du lot cartes-8 : aucune seconde
  file, aucun second drainage.
- **Quand le bonus prend effet** : il est LU au moment où la phase s'exécute.
  Une amélioration gagnée en phase II vaut donc dès la phase IV de la même
  manche. C'est pourquoi `extra_blue_activations` est écrit au début de la
  phase III et non à la planification.
- **VISIONNAIRE** — septième variante d'`AwardKind` (« le plus de cartes Phase
  améliorées », valeur `PlayerState::phase_upgrades_count`).
  `flow::award_pool(db)` ne la fait entrer dans la réserve que là où le
  mécanisme peut jouer : boîte Découverte ET couche d'effets active. Sans cela
  elle serait une égalité à zéro dans toutes les parties — et la réserve de la
  boîte de base changerait de taille, donc de tirage, donc d'empreinte.
- **Observabilité** — `simulate --probe-upgrade <phase><variante>` (répétable,
  cumulable, argument mal formé refusé), champs `upgrades` et `selector_bonus`
  de `--probe`, et cinq compteurs de bilan : `phase_upgrades_granted`,
  `phase_upgrades_reupgraded`, `upgraded_bonus_applied`,
  `upgraded_extra_builds`, `visionary_award_points`. Tous nuls en
  `--effects off` et en boîte de base seule.


## Représentation de l'état

Tout l'état d'une partie tient dans `GameState` (`src/state.rs`) :

- **Planète** : `temperature: u8` (niveau 0..=19, soit −30 °C à +8 °C par pas
  de 2), `oxygen: u8` (0..=14 %), `oceans: [OceanTile; 9]` mélangées à la mise
  en place + `oceans_revealed: u8`. Chaque `OceanTile` porte son bonus
  (cartes/MC/plantes) repris de `PlanetFactory.generateOceans` du moteur Java.
  Un **instantané de début de phase** (`snap_*`) porte la règle du livret :
  les caps de hausse s'évaluent sur l'état planétaire au début de la phase en
  cours, ce qui permet aux deux joueurs de finir un paramètre dans la même
  phase en touchant leur TR.
- **Joueurs** (`PlayerState` × 2) : ressources (`mc`, `heat`, `plants` en
  `i64`), `tr`, `forests`, productions (`mc_prod`, `heat_prod`, `plant_prod`,
  `card_prod` — alimentées par les cartes du lot 1, effets ON),
  capacités acier/titane (idem, stub), `hand`/`played` (indices `u16` dans la
  base de cartes), compteurs de tags et de couleurs, corporation choisie,
  phase choisie / phase précédente, activations bonus de la phase action,
  `phase_upgrades: [Option<PhaseUpgrade>; 5]` (structure Discovery, toujours
  `None` en v1), `card_resources` (ressources posées sur ses cartes, chantier
  cartes-3), et un compteur d'audit `tr_increments` pour l'invariant TR.
- **Pioches** : `deck`/`discard` projets (248 cartes `in_deck_v1`),
  `corp_deck`/`corp_discard` (16 corporations). Pioche vide → la défausse est
  remélangée (livret p.15). Les cartes sont des indices dans `CardsDb`,
  chargée une fois depuis `--cards` : TOUTES les cartes projets green/blue/red
  (331) y figurent — pour que la sonde trouve aussi les cartes hors pioche
  (Grain Silos, journal B2) — mais seules les 248 v1 entrent dans la pioche.
- **Infrastructure** : niveau 0..=14 (extension minimale pour Grain Silos,
  journal B2) ; chaque pas donne +1 TR et pioche 1 carte (sémantique Java
  `increaseInfrastructure`), cap sur l'instantané de phase. Hors condition de
  fin de partie ; jamais montée par la pioche v1.
- **Milestones/awards** (structure Discovery) : 3 milestones + 3 awards tirés
  à la mise en place des pools du moteur Java (11 milestones, 6 awards),
  drapeaux de revendication par joueur.
- **RNG** : `StdRng` embarqué dans `GameState`. Graine unique : `simulate`
  seed un RNG maître (`StdRng::seed_from_u64(seed)`) qui produit la graine de
  chaque partie ; mélanges ET décisions des deux joueurs consomment le même
  RNG de partie.

## Actions / flux (`src/flow.rs`)

Une ronde (`play_round`) suit le livret :

1. **Planification** : chaque joueur choisit une phase 1-5, **jamais celle de
   sa ronde précédente** (livret p.10 ; `allowed_phases`). Le sélectionneur de
   la phase 3 reçoit son activation bonus ici (comme `PickPhaseProcessor`).
2. **Exécution** : seules les phases choisies, dans l'ordre I→V, résolues pour
   les deux joueurs **dans l'ordre du tour de la manche** (§Ordre du tour) :
   - **I Développement** : 1 carte verte payée en MC et/ou en cartes ;
     sélectionneur : −3 MC. Un passage chacun, dans l'ordre du tour.
   - **II Construction** : 1 carte bleue/rouge ; sélectionneur : piocher
     1 carte **AVANT ou APRÈS** avoir joué, OU en jouer une 2e. Le moment de
     la pioche est un choix de la politique (`ConstructionBonus`) ; quand il
     est « avant », la carte piochée entre en main avant le calcul
     d'affordabilité et peut donc être posée dans la foulée (livret l.336).
     Un passage chacun, dans l'ordre du tour.
   - **III Action** : actions des cartes bleues (une activation par carte et
     par phase ; sélectionneur : une répétition), actions standard à volonté —
     forêt 8 plantes ou 20 MC (+1 forêt, +1 oxygène), température 8 chaleur ou
     14 MC, océan 15 MC (bonus de tuile), vente de carte 3 MC. Les joueurs
     jouent **action par action, en alternance** à partir du premier joueur ;
     un joueur qui passe est retiré du tour, la phase s'arrête quand les deux
     ont passé. Après la boucle, la règle obligatoire du livret p.14 : en fin
     de phase, conversion forcée des plantes (8→forêt) et de la chaleur
     (8→température) tant que possible, sauf paramètre déjà au max **sur
     l'instantané de début de phase** (même lecture que les hausses
     individuelles). Cette règle garantit la progression des parties
     aléatoires.
   - **IV Production** : MC += production MC + TR (+4 sélectionneur) ;
     chaleur/plantes/cartes selon production.
   - **V Recherche** : 2 piochées / 1 gardée ; sélectionneur 5 / 2. Un passage
     chacun, dans l'ordre du tour.
   Après chaque phase : revendication des milestones, puis test de fin de
   partie. La phase IV (collecte automatique) et l'étape de fin de manche
   gardent l'ordre fixe 0 puis 1 : aucun joueur n'y « agit ».
3. **Étape de fin** : défausse au-delà de 10 cartes en main, +3 MC par carte
   (livret p.16), génération suivante.

**Mise en place** (`setup_game`), avec les règles maison d'Alexis :
2 corporations chacun → **mulligan corporations** (remplacer les 2 ou aucune,
AVANT les cartes projets) → 8 cartes projets chacun → **mulligan projets**
(les 8 ou aucune, en une fois) → choix final de corporation (1 parmi 2, cartes
projets en main) → MC de départ de la corporation.

**Fin de partie** : les trois paramètres au max → on finit la phase en cours
puis décompte, les phases restantes de la ronde ne sont pas jouées (livret
« spelets slut »). **Score** : TR + 1 VP/forêt + VP des cartes jouées (fixes +
dynamiques, effets ON — voir §VP des cartes et score) + 3 VP/milestone +
awards (5/2 ; égalité au 1er rang : 4 chacun, pas de 2e — Discovery p.3).

**Politiques** (`src/policy.rs`) : le moteur appelle un `trait Policy` à chaque
point de décision (mulligans, choix de corporation, phase, constructions,
actions, recherche, défausse). `RandomPolicy` = politique uniforme pour
`simulate` ; les tests injectent des politiques scriptées **dans le même
flux** (aucun chemin de test parallèle).

## Ordre du tour (règle maison, lot 3)

`GameState::first_player` porte le premier joueur de la manche en cours :
**manche 1 = joueur 0**, puis **alternance à chaque manche menée à son terme**
(`play_round`). `GameState::turn_order: Vec<u8>` enregistre ce premier joueur
au DÉBUT de chaque manche réellement jouée — c'est la valeur que les phases
lisent ensuite via `players_in_turn_order()`, jamais une formule sur le numéro
de manche. Il y a donc exactement une entrée par manche jouée.

`GameState::turn_order_switches()` compte les alternances observées dans cette
liste (`= manches − 1` tant que l'alternance est stricte). `GameOutcome` et
`SimSummary` remontent la liste et la somme des alternances ;
`simulate --dump-turn-order` imprime une ligne `turn_order:<p0>,<p1>,…` par
partie, sur stdout, AVANT la ligne JSON finale (qui reste la dernière ligne).

## Compteurs de conformité (lot 3)

Cinq compteurs rendent les corrections observables sans lire le code. Chacun
est incrémenté à l'endroit exact où le mécanisme a lieu — jamais dans une
fonction de résumé, jamais depuis la sonde. Ils remontent de `GameState` à
`GameOutcome` puis à `SimSummary`, et figurent dans la ligne JSON de
`simulate`.

| Champ JSON | Sens | Incrémenté dans | Effets OFF |
|---|---|---|---|
| `prereq_snapshot_blocks` | Cartes payables exclues des options parce que leurs prérequis de paramètres n'étaient pas remplis **au début de la phase**, alors que l'état courant les aurait autorisées | `flow::affordable` | 0 (pas de prérequis sans couche d'effets) |
| `draw_before_build` | Pioches du bonus construction prises **avant** la pose | `flow::phase_construction` | > 0 (règle) |
| `draw_after_build` | Pioches du bonus construction prises **après** la pose | `flow::phase_construction` | > 0 (règle) |
| `discard_payments` | Cartes défaussées pour payer des cartes Projet (3 MC/carte) | `flow::build_card_with` | > 0 (règle) |
| `draws` | Parties terminées sur une égalité de PV (aucun départage : règle maison) | `sim::run_simulation` (sur `GameOutcome::draw`) | > 0 |

`turn_order_switches` complète la liste (voir §Ordre du tour).

Le chantier cartes-3 en ajoute cinq, mêmes règles (JSON de `simulate`, tous à 0
avec `--effects off`) :

| Champ JSON | Sens | Incrémenté dans |
|---|---|---|
| `res_added` | ressources posées sur des cartes (en unités) | `flow::add_resources` |
| `res_removed` | ressources retirées | `flow::remove_resources` |
| `res_targets_missing` | poses sautées faute de carte cible valide | `flow::apply_res_eff` / `apply_choice` |
| `phase_upgrades_skipped` | améliorations de carte Phase demandées et non gérées — **vaut 0 depuis le chantier `decouverte-phases`**, plus aucun site ne l'incrémente | (plus aucun) |
| `phase_upgrades_granted` | améliorations de carte Phase accordées | `flow::apply_phase_upgrade` |
| `phase_upgrades_reupgraded` | parmi elles, les bascules A ↔ B | `flow::apply_phase_upgrade` |
| `upgraded_bonus_applied` | bonus de sélectionneur AMÉLIORÉS réellement lus | `flow::selector_bonus_applied` |
| `upgraded_extra_builds` | permissions de pose versées par une carte Phase améliorée | `flow::grant_selector_builds` |
| `visionary_award_points` | points distribués par la tuile VISIONNAIRE | `flow::award_points_split`, agrégé par `sim::play_game` |
| `vp_from_resources` | points de victoire venant des ressources, tous joueurs | `flow::score_parts`, depuis `flow::card_points` |

## Paiement d'une carte (lot 3)

Livret p.13, l.348 : le coût se paie en cubes MC **et/ou** en défaussant
d'autres cartes de sa main à raison de 3 MC par carte, le surplus étant rendu.
Un service unique porte la règle des deux côtés :

- `flow::payable(mc, hand_len, cost)` — `mc + 3 × (hand_len − 1) ≥ cost`. La
  carte à poser ne peut pas se payer elle-même, d'où le `− 1`. Appelé par
  `flow::affordable` (énumération des options) ET par la sonde.
- `flow::build_card_with(...)` — retire la carte de la main, paie d'abord avec
  les MC, puis défausse le nombre de cartes donné par la méthode **par
  défaut** du trait `Policy::discard_payment_count` (= minimum,
  `ceil((cost − mc) / 3)`, plafonné à la main), encaisse 3 MC par carte et
  garde le surplus. Renvoie le nombre de cartes défaussées.
- `flow::build_card(...)` reste la façade historique (même signature qu'au
  lot 2) : elle délègue à `build_card_with` avec la règle par défaut. Il n'y a
  qu'un seul chemin de paiement.

## Invariants vérifiés

À chaque ronde de chaque partie simulée (`check_invariants`, `src/sim.rs`) :

1. Ressources jamais négatives (MC, chaleur, plantes, les deux joueurs).
2. Paramètres globaux dans leurs bornes (temp ≤ 19, oxy ≤ 14, océans ≤ 9) et
   **monotones croissants** d'une ronde à l'autre.
3. TR cohérent : `tr == 5 + tr_increments − tr_decrements` (compteurs
   alimentés uniquement par `gain_tr`/`spend_tr`), TR ≥ 0, compteurs
   monotones croissants. La monotonie brute du TR est remplacée par cette
   comptabilité depuis que le lot 1 autorise « Requires you to spend 1 TR »
   (journal B3) — toute baisse de TR non comptabilisée est une violation.
4. Conservation des cartes : pioche + défausse + mains + en-jeu =
   `v1_project_count` (248), et corporations : paquet + écartées + choisies
   = 16.

Toute violation est comptée dans `invariant_violations` de la sortie JSON.
Le plafond de sécurité (1 000 générations) classe la partie en `truncated`,
jamais en `completed`.

`state_hash` : FNV-1a 64 bits (implémentation locale, zéro dépendance) sur une
sérialisation canonique de chaque état final — génération, paramètres globaux,
et par joueur : TR, MC, chaleur, plantes, forêts, score, cartes jouées et en
main (triées) — agrégée sur les parties dans l'ordre d'exécution.

## STUBBÉ en v1 (limites du lot 1, et branchement des chantiers suivants)

Explicitement hors périmètre, structure prête :

- **Cartes hors lot 1** (les ~200 cartes projets v1 restantes + toutes les
  bleues) : stubs neutres jouables — payer le `price`, entrer en jeu avec tags
  et couleur, aucun effet. Leurs `vp`/`vp_dynamic` sont néanmoins comptés au
  score (effets ON) : les VP sont des données de `cards_v2.json`,
  indépendantes de la table d'effets. *Branchement des lots suivants* :
  ajouter des entrées à `LOT1` (et au vocabulaire si besoin).
- **Traité au lot 2** (n'est PLUS stub) : réductions de coût (« pay X less »),
  effets déclenchés (« when you play a … », « when you raise the temperature /
  flip an ocean »), actions de cartes bleues en phase III (le no-op
  `ActionOpt::BlueAction` est devenu `flow::apply_blue_action`). Voir §Lot 2.
- **Traité au chantier cartes-3** (n'est PLUS stub) : ressources posées sur les
  cartes (microbes/animaux/science) et leurs points de victoire.
- **Hors vocabulaire (restent stubs)** : productions par tag (« 1 MC per Earth
  tag »), pioche avec défausse à la pose (« draw 4 then discard 2 »), jeu
  gratuit d'une carte, tag wild
  (DYNAMIC). Le « choix du joueur à la pose » (« gain X OR … ») est traité
  depuis le chantier cartes-3 (`ResStep::Choose` + `Policy::choose_option`). Une carte
  mêlant un mécanisme du lot 2 ET un de ceux-ci reste HORS lot 2 (fidélité
  totale ou rien) ; idem une carte dont le nom se dédouble avec une variante
  « Buffed » (Greenhouses, Community Gardens : ambiguës par nom, exclues).
- **Corporations** : traité au chantier corpo-1, ce n'est PLUS un stub — voir
  §Corporations (chantier corpo-1).
- **Traité au chantier `decouverte-phases`** (n'est PLUS stub) : les
  améliorations de cartes Phase (« Upgrade a Phase card ») et la récompense
  VISIONNAIRE. Voir §Cartes Phase améliorées (chantier decouverte-phases).
- **Tag DYNAMIC (wild)** : compté comme aucun tag (le choix du tag est un
  effet de carte).
- **Award Collector** : les ressources posées existent depuis le chantier
  cartes-3, mais `flow::award_value` renvoie encore 0 pour cet award — le
  corriger changerait le score des parties hors du périmètre de ce lot. Reste
  donc 0-0 (égalité 4/4). *Branchement* : `pl.card_resources.values().sum()`.
- **Capacités acier/titane** : champs présents, toujours 0. Le lot 2 encode le
  TEXTE IMPRIMÉ des cartes à réduction (« pay N less on <tag> ») en réductions
  fixes, sans modéliser l'acier/titane comme ressource (le Java le fait via
  `steelIncome`/`titaniumIncome` dans `DiscountService`) ; les cartes qui
  SUIVENT réellement l'acier/titane restent hors lot.
- **Paiement par défausse de cartes** : la défausse à 3 MC existe comme action
  de la phase III et à l'étape de fin, pas comme moyen de paiement d'une
  carte (simplification de politique documentée, journal D9).
- **Revendication des milestones, simplifiée** (autorisée par le prompt) : à
  chaque transition de phase, tout joueur remplissant l'objectif d'un
  milestone non revendiqué le revendique ; revendications simultanées :
  les deux scorent 3 VP (équivalent du jeton 3 VP de Discovery).

## Sources des règles

- **Livret officiel du jeu de base** (fryxgames.se, section téléchargements —
  seule version publiée : suédoise, `TM_ARES_RULES_SEi.pdf`, traduite et
  croisée avec le moteur Java) : structure de ronde et interdiction de répéter
  sa phase (p.10), phases I-V et bonus du sélectionneur (p.11-15), actions
  standard et règle de conversion obligatoire (p.14), pioche remélangée
  (p.15), étape de fin et limite de main (p.16), fin de partie et décompte
  (p.16-17).
- **Livret Discovery** (fryxgames.se, `TM-AE-Discovery-rulebook-11-15-2021.pdf`) :
  milestones/awards (p.2-3 : 3 tuiles de chaque en jeu, 3 VP/milestone,
  awards 5/2 avec égalités 4/-/1), améliorations de phases (stub), wild tag
  (stub).
- **Moteur Java de référence**
  (`workspaces/audit-nikitinalexx/repo/src/main/java/com/terraforming/ares/`) :
  gradations des paramètres et bonus des tuiles océan
  (`factories/PlanetFactory.java`), constantes de coûts et tailles de main
  (`model/Constants.java`), flux de phases (`services/StateTransitionService.java`),
  revenu de production (`processors/turn/CollectIncomeTurnProcessor.java`),
  recherche 2/1 et 5/2 (`processors/turn/DraftCardsTurnProcessor.java`),
  bonus sélectionneur phase 3 (`processors/turn/PickPhaseProcessor.java`),
  caps sur instantané de phase (`services/TerraformingService.java`,
  `MarsGame.planetAtTheStartOfThePhase`), pools et logique milestones/awards
  (`model/milestones/*`, `model/awards/*`, `MarsGame.assignMilestones`),
  vente de cartes à 3 MC (`SellCardsGenericTurnProcessor`), seuils des
  milestones vérifiés un à un dans `model/milestones/` (Builder 8 tags
  bâtiment, Diversifier 9 tags distincts, Energizer 10 prod chaleur, Farmer 5
  prod plantes, Legend 6 rouges, Magnate 8 vertes, Planner 12 cartes,
  SpaceBaron 7 espace, Terraformer TR 15, Tycoon 6 bleues, Gardener 3 forêts)
  et extracteurs d'awards dans `model/awards/` (Celebrity prod MC, Collector
  ressources sur cartes, Generator prod chaleur, Industrialist acier+titane,
  ProjectManager cartes jouées, Researcher tags science).
- **Conflits notés** (le livret gagne) : le livret Discovery annonce 7 tuiles
  award, le moteur Java n'en implémente que 6 — pool de 6 retenu, la 7e
  n'étant pas nommée dans le texte extractible du PDF. Le champ
  `games_per_sec` (non déterministe) est émis sur stderr pour préserver
  « même graine → sortie strictement identique » (journal D17).

## Fichiers

- `src/cards.rs` — chargement de `cards_v2.json`, base de cartes (VP inclus),
  résolution CANONIQUE par nom (`resolve_card`, filtrée sur `in_deck_v1` en cas
  d'homonyme « Buffed ») et rattachement des effets à la carte canonique.
- `src/effects.rs` — vocabulaire d'effets (`Req`/`Eff` + lot 2 : `Reduction`,
  `PlayTrigger`/`TrigCond`/`TrigGain`, `GlobalTrigger`, `Action`/`ActionCost`/
  `ActionEff`), table `LOT1` (63 + 47 = 110 cartes), constantes de paliers.
- `src/state.rs` — état de jeu (dont `PlayerState::card_resources`, `BTreeMap`),
  constantes sourcées, pools milestones/awards,
  compteurs d'audit TR (`tr_increments`/`tr_decrements`), ordre du tour
  (`first_player`, `turn_order`, `players_in_turn_order`,
  `turn_order_switches`) et compteurs de conformité du lot 3.
- `src/flow.rs` — mise en place (mulligans maison), ronde, phases, ordre du
  tour, `requirements_met`/`requirements_met_now` (prérequis : instantané vs
  état courant), `payable`/`build_card`/`build_card_with` (paiement MC +
  défausse), `card_discount` (A), `fire_play_triggers`/`fire_global_trigger`
  (B), `apply_blue_action` (C), score (VP cartes).
- `src/policy.rs` — `trait Policy`, `RandomPolicy` ; `action_amount` (montants
  « up to X ») et `discard_payment_count` (nombre de cartes défaussées pour
  payer), tous deux méthodes par défaut ; `ConstructionBonus` à trois choix
  (`DrawCardBefore`, `DrawCard`, `SecondBuild`).
- `src/probe.rs` — état fixe, sonde séquence (`run_probe`/`run_probe_seq`/
  `run_probe_seq_opts`, `paid`, `discarded`, `prereq_ok_now`, `ProbeOptions`)
  et sonde action (`run_probe_action`).
- `src/sim.rs` — invariants, empreinte FNV-1a, boucle de simulation, compteurs
  `blue_actions` et compteurs de conformité du lot 3 (dont `draws` et
  `turn_order_switches`).
- `src/bin/simulate.rs` — CLI `--games N --seed S [--cards …]
  [--effects on|off] [--dump-turn-order] [--probe "<A>;<B>;…"]
  [--probe-action "<nom>"] [--probe-strict ["<A>;<B>;…"]] [--probe-mc <n>]
  [--probe-filler <n>] [--probe-choice "1,0,2"]
  [--probe-target "<A>;<B>"]`, une ligne JSON sur stdout (champs
  `blue_actions`, `prereq_snapshot_blocks`, `draw_before_build`,
  `draw_after_build`, `discard_payments`, `draws`, `turn_order_switches`,
  `res_added`, `res_removed`, `res_targets_missing`, `phase_upgrades_skipped`,
  `vp_from_resources` en mode simulation).
- `tests/engine_tests.rs` — 27 tests du squelette (mulligans, production,
  contrainte de phase, fin de partie, score, invariants, déterminisme…).
- `tests/lot1_tests.rs` — 72 tests du lot 1.
- `tests/lot3_tests.rs` — 33 tests du lot 3 : un groupe par correction (C1
  instantané, C2 pioche avant/après, C3 défausse-paiement, C4 ordre du tour,
  C5 égalité + conversion obligatoire) plus la sonde étendue. Chaque règle
  corrigée a au moins un test qui ÉCHOUE sur l'ancien comportement.
- `tests/lot3_res_tests.rs` — 43 tests du chantier cartes-3 : un par carte du
  lot (28, état de jeu comparé au texte imprimé via la sonde) + intégration
  (tri et complétude de `resources`, cible imposée absente, absence de cible,
  filtrage des branches, VP de ressources au score RÉEL, compteurs d'audit en
  flux réel, `--effects off`, déterminisme, piège des classes « Buffed »).
- `tests/lot_corp_tests.rs` — 34 tests du chantier corpo-1 : la pioche à 12
  (dont le piège Teractor), les productions de départ et leur RÉPÉTITION à la
  seconde phase IV, un test nommant chacune des 12 corporations, les services
  uniques, `--effects off`, l'interface de sonde et le déterminisme. La
  corporation d'un joueur n'y est jamais posée à la main : un utilitaire cherche
  la première graine à laquelle le TIRAGE RÉEL la donne au joueur 0.
- `tests/lot2_tests.rs` — 53 tests du lot 2 : un par carte (47, sonde →
  état de jeu comparé au texte imprimé : réductions via `paid`, déclencheurs et
  actions via delta) + intégration (réduction dans l'affordabilité, compteur
  `blue_actions` en flux réel, prix payé plafonné à 0, interrupteur
  `--effects off`, intégrité du lot, déterminisme de la sonde séquence).

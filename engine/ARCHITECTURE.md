# Architecture — moteur de simulation Ares Expedition (v1, 2 joueurs)

Simulateur d'entraînement du projet Terra : état de jeu, boucle de phases,
production, paramètres globaux, fin de partie, score, règles maison de
mulligan, **couche d'effets déclarative** (chantier moteur-cartes-1 : lot 1 de
63 cartes aux effets complets, VP des 388 cartes extraits, sonde d'audit
`--probe`, interrupteur `--effects on|off`). Les cartes hors lot restent des
**stubs neutres** (voir §Stubbé). Parties complètes en politique aléatoire,
déterministes à graine fixée.

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

## VP des cartes et score

`cards_v2.json` (Mission A, script `outputs/work/extract_vp.py`) ajoute aux
388 cartes : `vp` (entier, `getWinningPoints()` Java, 0 par défaut) et
`vp_dynamic` (`null` ou `{type, resources, points}` recopié du builder
`WinPointsInfo`). Le score (effets ON) ajoute par carte jouée : `vp` fixes +
VP dynamiques calculables en v1 — formule Java `floor(n / resources) × points`
avec n = tags Jupiter (`JUPITER`), tags Terre (`EARTH`), forêts (`FOREST`),
cartes bleues jouées (`BLUE_CARD`), cartes jouées (`ANY_CARD`). Les types
portant sur des **ressources posées sur les cartes** (ANIMAL, MICROBE,
SCIENCE…) valent **0 en v1** (ressources non modélisées).

## Sonde d'audit (`simulate --probe "<nom exact>"`)

État de départ fixe (`src/probe.rs`) : joueur 1 sans corporation, 100 MC,
20 chaleur, 20 plantes, productions 0, TR 5, paramètres globaux au départ, la
carte nommée seule en main ; pioche = cartes v1 restantes en ordre d'index,
tuiles océan **non mélangées** (1re = +2 plantes, 2e = +4 MC). La sonde force
la pose par `flow::build_card` (même code que `simulate`) : les prérequis non
satisfaits ne bloquent pas (`prereq_ok` les rapporte), les dépenses « spend »
sont réellement payées. Sortie : une ligne JSON `{card, found, in_lot,
prereq_ok, played, delta{…}, vp}` ; `delta.mc` exclut le prix payé,
`delta.hand` exclut la carte jouée (journal B6). `found:false` si le nom est
inconnu ; carte hors lot : `in_lot:false`, delta nul.

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
  `None` en v1), et un compteur d'audit `tr_increments` pour l'invariant TR.
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
   les deux joueurs (joueur 0 puis 1 — le jeu réel est simultané, l'ordre est
   un choix d'implémentation documenté, sans impact hors partage
   pioche/océans) :
   - **I Développement** : 1 carte verte payée en MC ; sélectionneur : −3 MC.
   - **II Construction** : 1 carte bleue/rouge ; sélectionneur : piocher
     1 carte OU en jouer une 2e.
   - **III Action** : actions des cartes bleues (stubs neutres, une activation
     par carte et par phase ; sélectionneur : une répétition), actions
     standard à volonté — forêt 8 plantes ou 20 MC (+1 forêt, +1 oxygène),
     température 8 chaleur ou 14 MC, océan 15 MC (bonus de tuile),
     vente de carte 3 MC — puis la règle obligatoire du livret p.14 : en fin
     de phase, conversion forcée des plantes (8→forêt) et de la chaleur
     (8→température) tant que possible, sauf paramètre au max. Cette règle
     garantit la progression des parties aléatoires.
   - **IV Production** : MC += production MC + TR (+4 sélectionneur) ;
     chaleur/plantes/cartes selon production.
   - **V Recherche** : 2 piochées / 1 gardée ; sélectionneur 5 / 2.
   Après chaque phase : revendication des milestones, puis test de fin de
   partie.
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
  ajouter des entrées à `LOT1` (et au vocabulaire si besoin) ; l'activation
  bleue (`ActionOpt::BlueAction`, no-op qui consomme l'activation) et les
  hooks d'événements (`raise_*`/`build_forest`) restent les points d'entrée
  identifiés pour les effets récurrents.
- **Hors vocabulaire v1** (restent stubs même si la carte est simple par
  ailleurs) : réductions de coût (« pay X less »), effets déclenchés
  récurrents (« when you play a … »), ressources posées sur les cartes,
  productions par tag (« 1 MC per Earth tag »), pioche avec défausse
  (« draw 4 then discard 2 »), améliorations de phases, choix du joueur à la
  pose (« gain X OR … »), tag wild (DYNAMIC).
- **Corporations** : entrent en jeu avec leurs tags ; MC de départ = champ
  `price` du JSON (vérifié contre le Java : Credicor 48). Productions de
  départ et pouvoirs : stubbés.
- **Améliorations de phases (Discovery)** : `phase_upgrades` par joueur,
  toujours `None` ; les bonus de sélectionneur utilisent les valeurs de base.
  *Branchement* : `PhaseUpgrade::VariantA/B` par phase, consommé aux mêmes
  endroits que les bonus de base (valeurs alternatives du Java :
  `Constants.PHASE_*_UPGRADE_*`).
- **Tag DYNAMIC (wild)** : compté comme aucun tag (le choix du tag est un
  effet de carte).
- **Ressources posées sur cartes** (animaux, microbes…) : inexistantes en v1 —
  l'award Collector vaut donc toujours 0-0 (égalité 4/4).
- **Capacités acier/titane** : champs présents, toujours 0 (aucune réduction
  de coût).
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
  résolution des effets par nom.
- `src/effects.rs` — vocabulaire d'effets v1 (`Req`/`Eff`), table `LOT1`
  (63 cartes), constantes de paliers de couleur.
- `src/state.rs` — état de jeu, constantes sourcées, pools milestones/awards,
  compteurs d'audit TR (`tr_increments`/`tr_decrements`).
- `src/flow.rs` — mise en place (mulligans maison), ronde, phases,
  `requirements_met`/`build_card` (couche d'effets), score (VP cartes).
- `src/policy.rs` — `trait Policy`, `RandomPolicy`.
- `src/probe.rs` — état fixe et exécution de la sonde `--probe`.
- `src/sim.rs` — invariants, empreinte FNV-1a, boucle de simulation.
- `src/bin/simulate.rs` — CLI `--games N --seed S [--cards …]
  [--effects on|off] [--probe "<nom>"]`, une ligne JSON sur stdout.
- `tests/engine_tests.rs` — 27 tests du squelette (mulligans, production,
  contrainte de phase, fin de partie, score, invariants, déterminisme…).
- `tests/lot1_tests.rs` — 72 tests du lot 1 : un par carte (63, sonde → état
  de jeu comparé au texte imprimé) + intégration (prérequis dans le flux réel,
  paliers de couleur, dépense de TR, interrupteur, score VP fixes/dynamiques,
  déterminisme de la sonde, intégrité de la table).

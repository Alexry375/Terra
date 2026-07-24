# Audit de conformité — moteur Rust vs livret « Expédition Arès »

> Date : 2026-07-24. Auditeur : sous-agent Terra.
> Sources : `docs/regles/notes/regles-condensees.md` (+ `docs/regles/livret-base.md` mot à mot) contre `engine/src/{state,flow,effects,probe}.rs` et `ARCHITECTURE.md`.
> Méthode : chaque ligne relue dans le code au moment de l'audit. Aucune affirmation de mémoire.
> Rappel de périmètre (NON comptés comme écarts) : 110 cartes / 248 aux effets complets, pas de corporations réelles, pas de ressources posées sur cartes (microbes/animaux/science), pas d'améliorations de phases, extension Découverte non couverte.

---

## 1. Paramètres globaux — bornes, pas, max, NT

| Règle du livret (valeur exacte) | État dans le moteur (fichier:ligne) | Verdict |
|---|---|---|
| Température −30 °C → +8 °C par pas de 2 (20 niveaux) | `TEMPERATURE_MAX = 19`, niveau `u8` 0..=19 (`state.rs:17`, doc `:16`) | CONFORME |
| Oxygène 0 → 14 % par pas de 1 | `OXYGEN_MAX = 14` (`state.rs:19`) | CONFORME |
| Océans 0 → 9 tuiles | `NUM_OCEANS = 9` (`state.rs:21`) | CONFORME |
| +1 NT à chaque hausse d'un niveau (température, oxygène, océan) | `gain_tr()` appelé une fois par pas dans `raise_temperature` (`flow.rs:464`), `raise_oxygen` (`flow.rs:438`), `reveal_ocean` (`flow.rs:489`) | CONFORME |
| Dépassement du max = pas d'avantage (pas d'erreur) | Guards `snap_* >= MAX` → retour sans TR ni hausse (`flow.rs:432,445,458,472`) | CONFORME |
| Pendant la phase où un paramètre atteint son max : tous peuvent continuer et reçoivent PV/NT | Cap évalué sur l'**instantané de début de phase** `snap_*` (`state.rs:294-299`, `flow.rs:1012`) | CONFORME |
| Après cette phase : plus de récompense liée au paramètre | `snap_*` vaut alors le max → hausses inertes | CONFORME |
| Exception oxygène : PV Forêt encore possibles après le max, mais sans NT | `build_forest` incrémente `forests` PUIS appelle `raise_oxygen` qui no-ope si `snap_oxygen` au max (`flow.rs:504-505`, `431-439`) | CONFORME |
| Océans : au-delà du 9e dans la phase du max, récompense de la dernière tuile révélée + 1 NT | Fallback `game.oceans[NUM_OCEANS-1]` (index 8 = 9e révélée) + `gain_tr` (`flow.rs:479-489`) | CONFORME |
| Zones de couleur (violet/rouge/jaune/blanc) comme prérequis | Bornes de paliers `TEMP_*` / `OXY_*` (`effects.rs:25-31`), sourcées du Java `PlanetFactory` (non chiffrées dans le livret) | CONFORME (bornes hors livret, sourcées Java) |

## 2. NT de départ et double rôle

| Règle | État moteur | Verdict |
|---|---|---|
| NT de départ = 5 | `STARTING_TR = 5` (`state.rs:27`), `tr: STARTING_TR` (`state.rs:192`) | CONFORME |
| NT = revenu MC en production (« Ajoutez votre NT à votre production de MC ») | `pl.mc += pl.mc_prod + pl.tr + bonus` (`flow.rs:810`) | CONFORME |
| NT = 1 PV/NT en fin de partie | `s = pl.tr + pl.forests` (`flow.rs:965`) | CONFORME |

## 3. Conversions et actions standard (phase III)

| Règle du livret | État moteur | Verdict |
|---|---|---|
| 8 chaleurs → +1 température (+1 NT) | `TEMPERATURE_HEAT_COST = 8` (`state.rs:35`), `flow.rs:761-764` | CONFORME |
| 14 MC → +1 température | `TEMPERATURE_MC_COST = 14` (`state.rs:36`), `flow.rs:765-768` | CONFORME |
| 8 plantes → 1 PV Forêt + oxygène +1 (+1 NT) | `FOREST_PLANT_COST = 8` (`state.rs:33`), `build_forest` (`flow.rs:496-506`) | CONFORME |
| 20 MC → 1 PV Forêt + oxygène +1 | `FOREST_MC_COST = 20` (`state.rs:34`), `flow.rs:760` | CONFORME |
| 15 MC → révéler 1 océan (récompense immédiate au verso) | `OCEAN_MC_COST = 15` (`state.rs:37`), `flow.rs:769-772`, bonus tuile `reveal_ocean` (`flow.rs:482-488`) | CONFORME |
| Défausser 1 carte de sa main → +3 MC | `SELL_CARD_MC = 3` (`state.rs:30`) ; action de phase III (`flow.rs:773-779`) et étape de fin (`flow.rs:1041`) | CONFORME sur la valeur / SIMPLIFICATION sur la portée (voir Écart §E4 : « à tout moment » non implémenté) |
| Actions standard en nombre illimité | Boucle sans limite jusqu'à choix `None` (`flow.rs:734-781`) | CONFORME |
| Règle importante fin de phase III : convertir chaleur/plantes de force, sauf paramètre au max | Boucles obligatoires (`flow.rs:787-796`) | CONFORME (nuance mineure §E5) |

## 4. Les 5 phases — structure, ordre, bonus

| Règle du livret | État moteur | Verdict |
|---|---|---|
| Résolution dans l'ordre I → V | Boucle `for phase in 1u8..=5` (`flow.rs:1008`) | CONFORME |
| Seules les phases choisies ≥1 fois sont résolues, une fois chacune | Filtre `picked[phase]` (`flow.rs:1004,1009`) | CONFORME |
| Phase non choisie = ignorée | idem (skip si `!picked`) | CONFORME |
| Compétence pour tous / bonus au seul choisissant | Bonus conditionnés à `chosen_phase == n` (`flow.rs:673,696,728,804,827`) | CONFORME |
| Interdit de rejouer la même phase deux manches de suite | `allowed_phases` exclut `previous_phase` (`flow.rs:174-178`), assert (`flow.rs:994`) | CONFORME |
| I Développement — compétence : 1 carte verte ; bonus : −3 MC | `affordable(&[Green])`, 1 pose ; `DEV_SELECTOR_DISCOUNT = 3` (`state.rs:40`, `flow.rs:671-684`) | CONFORME |
| II Construction — compétence : 1 carte bleue OU rouge ; bonus : piocher 1 carte OU jouer une 2e | `affordable(&[Blue,Red])`, `ConstructionBonus::{DrawCard,SecondBuild}` (`flow.rs:688-713`) | CONFORME (nuance §E2 : « avant ou après ») |
| III Action — compétence : chaque « Action : » une fois + actions standard illimitées ; bonus : une répétition | `remaining_blue` une activation/carte, `extra_blue_activations` (`flow.rs:718-757`) | CONFORME |
| IV Production — compétence : ressources + MC = NT ; bonus : +4 MC | `PRODUCTION_SELECTOR_MC = 4` (`state.rs:41`, `flow.rs:802-820`) | CONFORME |
| V Recherche — compétence : pioche 2 garde 1 ; bonus : pioche 5 garde 2 | `(2,1)` / `(5,2)` (`flow.rs:827-831`) | CONFORME |

## 5. Phase de production — détail

| Règle du livret (p.15) | État moteur (`flow.rs:802-820`) | Verdict |
|---|---|---|
| « Ajoutez votre NT à votre production de MC » (MC = prod MC + NT) | `pl.mc += pl.mc_prod + pl.tr + bonus` (`:810`) | CONFORME |
| Gagner chaleur = production de chaleur | `pl.heat += pl.heat_prod` (`:811`) | CONFORME |
| Gagner plantes = production de plantes | `pl.plants += pl.plant_prod` (`:812`) | CONFORME |
| Chaque production de carte → piocher 1 carte cette phase | Boucle `card_prod` × `draw_card` (`:813-818`) | CONFORME |
| Bonus choisissant +4 MC | `bonus` ajouté au MC (`:810`) | CONFORME |

## 6. Phase de recherche — nombres exacts

| Règle du livret (p.15) | État moteur | Verdict |
|---|---|---|
| Compétence : pioche 2, garde 1, défausse 1 | `(2,1)` (`flow.rs:829-830`) | CONFORME |
| Bonus : +3 piochées, +1 gardée → total pioche 5, garde 2, défausse 3 | `(5,2)` (`flow.rs:827-828`) | CONFORME |
| Pioche vide → remélanger la défausse | `draw_card` swap+shuffle (`flow.rs:28-36`) | CONFORME |

## 7. Fin de partie et score

| Règle du livret (p.16-17) | État moteur | Verdict |
|---|---|---|
| Déclencheur : 9 océans ET température +8 °C ET oxygène 14 % | `all_parameters_maxed` (`state.rs:287-291`) | CONFORME |
| On termine la phase en cours, le reste de la manche est ignoré | Test après CHAQUE phase, `return` (`flow.rs:1021-1024`) | CONFORME |
| PV = 1 PV/NT | `pl.tr` (`flow.rs:965`) | CONFORME |
| PV = jetons Forêt (1 PV chacun) | `pl.forests` (`flow.rs:965`) | CONFORME |
| PV = PV des cartes jouées (fixes + variables `*`) | `card_points` fixes + dynamiques (`flow.rs:938-955,966-968`) ; ressources sur cartes = 0 (connu) | CONFORME (VP « ressources sur cartes » = NON IMPLÉMENTÉ connu) |
| Égalité : plus grand cumul chaleur + MC + plantes (cartes en main converties en MC) | Aucun départage dans `score` (`flow.rs:960-980`) | NON IMPLÉMENTÉ (voir §E3) |
| PV milestones/awards | 3 VP/milestone + awards 5/2 TOUJOURS ajoutés (`flow.rs:971-975,914-931`) — Discovery | SIMPLIFICATION ASSUMÉE / hors livret de base (voir §E1) |

## 8. Coût des cartes, réductions, prérequis

| Règle du livret | État moteur | Verdict |
|---|---|---|
| Coût jamais < 0 | `effective_cost = (price-discount).max(0)` (`flow.rs:180-182`), assert (`flow.rs:340`) | CONFORME |
| Acier −2 MC/badge Construction, titane −3 MC/badge Espace | Savoir-faire acier/titane = 0 (`state.rs:200-201`) ; le lot 2 encode le TEXTE IMPRIMÉ des cartes à réduction en `Reduction::{AnyCard,Tag}` (`effects.rs:104-128`, table `:361-548`) ; service unique `card_discount` (`flow.rs:189-203`) | SIMPLIFICATION ASSUMÉE (réductions fixes équivalentes, conforme à la consigne) |
| Réductions cumulables | Somme sur toutes les cartes en jeu (`flow.rs:195-201`) | CONFORME |
| Prérequis océans/oxygène/température remplis **au début de la phase** | `requirements_met` teste `game.temperature/oxygen/oceans_revealed` COURANTS, pas `snap_*` (`flow.rs:216-221`) | ÉCART (voir §E6) |
| Prérequis non remplis → carte injouable | `requirements_met` filtre `affordable` (`flow.rs:250`) | CONFORME (au niveau courant) |
| Paiement par MC et/ou défausse de cartes (3 MC/carte) | Paiement en MC seul ; défausse-paiement non modélisée (`ARCHITECTURE.md` D9) | SIMPLIFICATION ASSUMÉE (voir §E4) |

## 9. Mise en place, main, limites

| Règle du livret | État moteur | Verdict |
|---|---|---|
| Cube NT de départ sur 5 ; productions à 0 | `PlayerState::new` : `tr:5`, prod 0 (`state.rs:189-213`) | CONFORME |
| Température −30 / oxygène 0 au départ | `temperature:0, oxygen:0` (`flow.rs:79-80`) | CONFORME |
| 8 cartes Projet à chaque joueur (gardées) | `STARTING_HAND = 8` (`state.rs:28`, `flow.rs:133-137`) | CONFORME |
| 2 corporations, en garder 1 | `pick_corporation` parmi 2 (`flow.rs:150-166`) | CONFORME (pouvoirs/prod de départ stubbés — connu) |
| Tuiles océan mélangées, 9 emplacements | `shuffle(&mut oceans)` (`flow.rs:68-69`) | CONFORME |
| Limite de main : max 10 en fin de manche, +3 MC/carte défaussée | `HAND_LIMIT = 10` (`state.rs:29`), `flow.rs:1029-1044` | CONFORME |
| Après révélation, reprise en main de la carte Phase précédente (seule la dernière est interdite) | `allowed_phases` n'exclut QUE `previous_phase` (`flow.rs:174-178`) | CONFORME |
| Bonus tuiles océan (cartes/MC/plantes) | `OCEAN_TILES` (`state.rs:52-62`), sourcés Java `PlanetFactory` | CONFORME (valeurs hors livret, sourcées Java) |

## 10. Règles maison d'Alexis (statut, non comptées comme écarts)

| Règle attendue | État moteur | Statut |
|---|---|---|
| Mulligan corporations (les 2 ou aucune, avant les projets) | `corp_mulligan` (`flow.rs:120-130`) | IMPLÉMENTÉ |
| Mulligan projets (les 8 ou aucune, en une fois) | `project_mulligan` (`flow.rs:139-148`) | IMPLÉMENTÉ |

## 11. Éléments hors livret de base / hors périmètre (pour mémoire)

| Élément | État moteur | Statut |
|---|---|---|
| Piste « Infrastructure » (+1 TR + pioche 1/pas) | `raise_infrastructure` (`flow.rs:444-455`), `INFRASTRUCTURE_MAX=14` (`state.rs:25`) | Hors livret, sourcé Java ; atteinte uniquement par Grain Silos (hors pioche v1) → jamais en simulation. Gravité négligeable. |
| Milestones / awards | pools + attribution (`state.rs:71-137`, `flow.rs:851-931`) | Structure Discovery, hors périmètre annoncé mais TOUJOURS active au score (voir §E1) |

---

# Écarts

## E1 — Milestones et awards (Discovery) toujours comptés au score, sans bascule jeu de base
- **Livret (base)** : le score est NT + jetons Forêt + PV des cartes. Aucune notion de milestone/award (contenu Découverte, hors périmètre annoncé de l'audit).
- **Code** : `assign_milestones` est appelé après chaque phase (`flow.rs:1020`) et `score` ajoute 3 VP par milestone revendiqué (`flow.rs:971-975`) plus les awards 5/2/4 (`flow.rs:914-931`), **inconditionnellement** — même avec `--effects off` (le seul interrupteur ne coupe que les effets/VP de cartes, `flow.rs:966`).
- **Gravité** : mineure à modérée. Pour une IA censée jouer le **jeu de base**, ces PV supplémentaires modifient l'ordre des scores et donc la politique optimale (course aux milestones). Ce n'est pas une contradiction du livret de base *stricto sensu* (c'est du contenu Découverte greffé), mais il n'existe aucun moyen d'obtenir un score « base pure ».
- **Correction suggérée** : gate les milestones/awards derrière un drapeau `discovery_on` (ou les couper quand `--effects off`), pour produire un score fidèle au livret de base.

## E2 — Phase II : « piocher une carte avant OU après avoir joué » réduit à « après »
- **Livret (p.12, l.336)** : bonus du choisissant = « piocher une carte **avant ou après** avoir joué une carte ».
- **Code** : la carte est toujours construite d'abord (`flow.rs:692-695`), puis le bonus `DrawCard` pioche (`flow.rs:698-702`). La carte piochée ne peut donc jamais servir à la pose unique de la phase.
- **Gravité** : mineure. Perte d'une option tactique (piocher puis jouer la carte piochée).
- **Correction suggérée** : proposer à la politique le moment de la pioche (avant/après), ou piocher avant si le joueur n'a rien de jouable.

## E3 — Départage d'égalité (chaleur + MC + plantes) non implémenté
- **Livret (p.17, l.461)** : à égalité de PV, gagne le joueur au plus grand cumul chaleur + MC + plantes (cartes en main d'abord converties en MC).
- **Code** : `score` renvoie les PV bruts sans départage (`flow.rs:960-980`) ; aucune conversion des cartes en main.
- **Gravité** : mineure (ne change pas le score, seulement la désignation du vainqueur en cas d'ex æquo exact).
- **Correction suggérée** : ajouter un critère secondaire `heat + mc + plants + hand.len()` (1 carte = 3 MC) au comparateur de vainqueur.

## E4 — « Défausser une carte pour +3 MC à tout moment » restreint à la phase III et à l'étape de fin
- **Livret (p.7, l.96 ; p.13, l.348)** : à tout moment, on peut défausser une carte pour 3 MC ; on peut aussi payer une carte en défaussant des cartes (3 MC/carte, surplus rendu).
- **Code** : la vente à 3 MC n'existe que comme action de phase III (`flow.rs:773-779`) et à l'étape de fin (`flow.rs:1041`) ; le paiement d'une carte se fait uniquement en MC (`flow.rs:339-342`).
- **Gravité** : mineure, documentée (`ARCHITECTURE.md` D9). L'affordabilité d'une carte est légèrement sous-estimée (on ne peut pas compléter le paiement par des cartes).
- **Correction suggérée** : autoriser la défausse-paiement dans `affordable`/`build_card` (borne : cartes en main × 3 MC), et/ou la vente hors phase III.

## E5 — Conversion obligatoire de fin de phase III : garde sur le paramètre COURANT, pas sur l'instantané
- **Livret (p.14, l.393)** : à la fin de la phase d'actions, il **faut** convertir chaleur/plantes disponibles, « sauf si le paramètre associé a déjà atteint son maximum ».
- **Code** : les boucles obligatoires testent `game.oxygen < OXYGEN_MAX` / `game.temperature < TEMPERATURE_MAX` (valeurs **courantes**, `flow.rs:788,791`), alors que les hausses individuelles plafonnent sur `snap_*`. Si un paramètre atteint son max **pendant cette même phase III** (donc `snap_* < max`), l'obligation s'arrête dès que la valeur courante touche le max, même si le livret autorise (sans forcer) à continuer pour des PV Forêt.
- **Gravité** : très mineure / défendable. Le livret dit « **sauf si** le paramètre a atteint son max » — s'arrêter au max courant respecte la lettre de la règle d'obligation ; seul le côté volontaire (continuer pour des PV) n'est pas déclenché ici, mais il reste ouvert via l'action volontaire `ForestWithPlants` plus tôt dans la phase. À noter par cohérence avec le reste du moteur (qui raisonne partout sur `snap_*`).
- **Correction suggérée** : aucune impérative ; si l'on veut l'uniformité, tester `snap_* < MAX` comme ailleurs.

## E6 — Prérequis de paramètres évalués à l'état courant, pas « au début de la phase »
- **Livret (p.13, l.352)** : « Si une carte dispose d'un prérequis relatif aux océans, à l'oxygène ou à la température, ce prérequis doit être rempli **au début de la phase** ».
- **Code** : `requirements_met` compare aux valeurs **courantes** `game.temperature` / `game.oxygen` / `game.oceans_revealed` (`flow.rs:216-221`), et non à l'instantané `snap_temperature/oxygen/oceans` pris au début de la phase (`state.rs:294-299`). Or le moteur prend déjà ce `snap_*` et l'utilise partout ailleurs pour les caps.
- **Cas concret** : phase II, choisissant avec le bonus `SecondBuild` — la 1re carte posée hausse un paramètre (ex. `Ice Asteroid` → 2 océans, `Lava Flows` → température), puis l'affordabilité de la 2e carte est recalculée (`flow.rs:704`) sur l'état **post-1re-carte**. Une 2e carte dont le prérequis n'était PAS rempli au début de la phase devient jouable — contraire au livret. (En phase I une seule carte verte est jouée, l'écart ne s'y manifeste pas.)
- **Gravité** : mineure mais réelle et facile à corriger. Rare (nécessite le bonus SecondBuild + une 1re carte qui change un paramètre + une 2e carte à prérequis de paramètre).
- **Correction suggérée** : dans `requirements_met`, comparer les prérequis `TempMin/TempMax/OxyMin/OceanMin/OceanMax` à `game.snap_temperature/snap_oxygen/snap_oceans` plutôt qu'aux valeurs courantes.

---

## Synthèse des verdicts
- Points vérifiés : ~55 lignes de règles.
- CONFORME : ~44.
- SIMPLIFICATION ASSUMÉE : 4 (acier/titane en réductions fixes, défausse-paiement, portée de la vente 3 MC, milestones/awards Discovery).
- NON IMPLÉMENTÉ (connu) : départage d'égalité, VP « ressources sur cartes », plus les absences de périmètre annoncées (corporations, ressources sur cartes, améliorations de phases).
- ÉCARTS réels : 6 (E1 à E6), tous mineurs ; le plus net est E6 (prérequis évalués à l'état courant au lieu du début de phase).

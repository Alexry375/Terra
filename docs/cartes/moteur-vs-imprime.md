# moteur-vs-imprime.md — le moteur simule-t-il les cartes IMPRIMÉES ?

Diagnostic carte par carte des **66 cartes distinctes** nommées en `inputs/divergences.md`
§G1 (51) et §G2 (20), 5 étant communes aux deux sections. Le texte de référence est
`inputs/textes-cartes.json` (champ `text`), jamais la paraphrase de `cards.json`.

## Verdict global

**Le moteur n'a PAS hérité en masse des erreurs de la paraphrase.** Sur les 35 cartes du
périmètre réellement encodées, **33 appliquent la règle imprimée** et **2 sont fausses**.
Les 31 autres ne sont pas encodées du tout : le moteur ne fait rien pour elles.

| Verdict | Nombre | Sens |
|---|---|---|
| CONFORME | 33 | prouvé par sonde exécutée ; régime `Action:` prouvé RÉPÉTABLE par le flux réel |
| FAUX | 2 | corrigé dans `outputs/engine/`, test de non-régression à l'appui |
| ABSENT | 7 | pas dans la table d'effets ; le mécanisme nécessaire existe déjà |
| HORS-PORTEE | 24 | pas dans la table ET la règle imprimée dépend d'un mécanisme non modélisé |

**Re-mesures contredisant les chiffres fournis** (le contrat demandait de ne pas les croire) :

- §G1 compte bien **51 cartes distinctes**, dont **26 dans la table et 25 absentes** — la mesure
  de la main est exacte, y compris le détail : les 12 corporations et *Hydro-Electric Energy*,
  la carte-témoin, sont bien parmi les absentes.
- Mais le périmètre **entier** (§G1 ∪ §G2) est de **66 cartes**, dont **35 dans la table et 31 absentes**.
- Les homoglyphes cyrilliques touchent **18 entrées de `cards.json`, pas 16** (§4.1).

## Tableau récapitulatif

Trié par gravité : ce qui est faux d'abord, puis ce qui est muet, puis ce qui est juste.

| Carte | Verdict | Gravité |
|---|---|---|
| **Viral Enhancers** | FAUX | haute — corrigée |
| **Decomposers** | FAUX | haute — corrigée |
| **CrediCor** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Interplanetary Cinematics** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Saturn Systems** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Teractor Corporation** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Tharsis Republic** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Thorgate Corporation** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Adaptation Technology** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Advanced Alloys** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Assembly Lines** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Asset Liquidation** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Composting Factory** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Ecoline** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Helion** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Hydro-Electric Energy** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Inventrix** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Mars University** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Mining Guild** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **PhoboLog** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Restructured Resources** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Solarpunk** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Standard Technology** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **United Nations Mars Initiative** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Water Import from Europa** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Wood Burning Stoves** | HORS-PORTEE | moyenne — le moteur est muet, et le mécanisme manque |
| **Biothermal Power** | ABSENT | moyenne — le moteur est muet |
| **Extended Resources** | ABSENT | moyenne — le moteur est muet |
| **Interns** | ABSENT | moyenne — le moteur est muet |
| **Mangrove** | ABSENT | moyenne — le moteur est muet |
| **Plantation** | ABSENT | moyenne — le moteur est muet |
| **Protected Valley** | ABSENT | moyenne — le moteur est muet |
| **United Planetary Alliance** | ABSENT | moyenne — le moteur est muet |
| **Anaerobic Microorganisms** | CONFORME | nulle |
| **Anti-Gravity Technology** | CONFORME | nulle |
| **Arctic Algae** | CONFORME | nulle |
| **Birds** | CONFORME | nulle |
| **Conserved Biome** | CONFORME | nulle |
| **Earth Catapult** | CONFORME | nulle |
| **Ecological Zone** | CONFORME | nulle |
| **Energy Subsidies** | CONFORME | nulle |
| **Extreme-Cold Fungus** | CONFORME | nulle |
| **Fish** | CONFORME | nulle |
| **Fusion Power** | CONFORME | nulle |
| **Herbivores** | CONFORME | nulle |
| **Insects** | CONFORME | nulle |
| **Interplanetary Conference** | CONFORME | nulle — mais **contingent** : valide sous la lecture du livret retenue en §5 / `blocked.md` |
| **Interplanetary Relations** | CONFORME | nulle |
| **Large Convoy** | CONFORME | nulle |
| **Livestock** | CONFORME | nulle |
| **Media Group** | CONFORME | nulle |
| **Medical Lab** | CONFORME | nulle |
| **Nitrogen-Rich Asteroid** | CONFORME | nulle |
| **Olympus Conference** | CONFORME | nulle |
| **Optimal Aerobraking** | CONFORME | nulle |
| **Physics Complex** | CONFORME | nulle |
| **Power Grid** | CONFORME | nulle |
| **Recycled Detritus** | CONFORME | nulle |
| **Research Outpost** | CONFORME | nulle |
| **Small Animals** | CONFORME | nulle |
| **Symbiotic Fungus** | CONFORME | nulle |
| **Terraforming Ganymede** | CONFORME | nulle |
| **Titanium Mine** | CONFORME | nulle |
| **Windmills** | CONFORME | nulle |
| **Worms** | CONFORME | nulle |
| **Zeppelins** | CONFORME | nulle |

---

## 1. Les deux cartes FAUSSES — et le motif qui les explique

### Le motif : un effet déclenché ne se résolvait qu'UNE fois

Le livret officiel tranche, et aucun lot précédent ne l'avait vu —
`inputs/regles/livret-base.md` ligne 106 :

> **EFFET** (Violet) — […] **Si la condition d'un effet est remplie plusieurs fois lorsqu'une**
> **carte est jouée, résolvez l'effet correspondant plusieurs fois.**

Une carte à deux badges satisfaisants remplit donc deux fois la condition. Le moteur
appliquait déjà ce principe pour les gains simples (chaleur, plantes, pioche, ressources)
via le drapeau `scale_by_matched_tags` — mais **jamais** pour `TrigGain::Choose`, la
variante « … ou … », qui était appliquée une seule fois **par construction**. Le commentaire
du code l'assumait : « *les deux cartes concernées sont au forfait* », avec pour justification
le moteur Java, et non le texte imprimé. C'est exactement l'inversion d'oracle que ce
chantier existe pour corriger.

Cette clause décisive est **absente de `inputs/regles/regles-condensees.md`** — origine
probable de l'erreur pour quiconque n'a lu que le condensé.

### Viral Enhancers — FAUX

**Ce que la carte imprime** : « Effect: When you play a [animal], [microbe], or [plant], including these, gain 1 plant or add 1 animal or microbe to ANOTHER* card. »

**Ce que le moteur faisait** : une seule résolution par carte jouée, quel que soit le nombre
de badges satisfaisants. La carte porte elle-même [microbe] **et** [plant] : sur sa propre
pose (« including these ») elle devait donner 2 plantes, elle n'en donnait qu'1.

**Conséquence en partie** : sous-production silencieuse de la moitié pendant toute la partie,
sur chaque carte à deux badges bio jouée ensuite. Sur une carte à 8 MC censée être un moteur
économique, c'est la moitié de sa valeur qui disparaît sans que rien ne le signale.

**Preuve, après correction** :

```
$ simulate --cards inputs/cards.json --probe "Viral Enhancers" --probe-choice 0,0,0,0
{"card":"Viral Enhancers","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[8],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

`plants: 2` — deux résolutions pour ses deux badges. Avant correction, cette même commande
renvoyait `plants: 1`.

**Correction** : `flow.rs::apply_trig_gain` résout désormais `TrigGain::Choose` `mult` fois
(chaque résolution rappelant la politique, donc avec un choix indépendant), et
`effects.rs` passe `scale_by_matched_tags` à `true`.
**Test** : `verite_tests.rs::viral_enhancers_resolves_once_per_matching_tag_of_the_played_card`,
qui échoue sur l'ancien comportement (`left: 1, right: 2`) et passe sur le nouveau.

### Decomposers — FAUX

**Ce que la carte imprime** : « Effect: When you play an [animal], [microbe], or [plant], including this, add a microbe here or remove a microbe from here to draw a card. »

**Ce que le moteur faisait** : même défaut. Une carte à deux badges bio (Adapted Lichen,
[microbe]+[plant]) ne déclenchait qu'une résolution au lieu de deux.

**Conséquence en partie** : la carte accumule moitié moins de microbes, donc pioche moitié
moins de cartes sur toute la partie.

**Preuve, après correction** :

```
$ simulate --cards inputs/cards.json --probe "Decomposers;Adapted Lichen" --probe-choice 0,0,0,0
{"card":"Adapted Lichen","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":1,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[7,6],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Decomposers","kind":"microbe","n":3}],"target_error":null,"vp":0,"vp_total":1}
```

3 microbes = 1 (sa propre pose, un seul badge [microbe]) + 2 (Adapted Lichen, deux badges).
Avant correction : 2.

**Test** : `verite_tests.rs::decomposers_resolves_once_per_matching_tag_of_the_played_card`,
qui échoue sur l'ancien comportement (`left: 2, right: 3`).

### Un test existant a été RENFORCÉ, pas supprimé

`tests/lot3_res_tests.rs::viral_enhancers_is_a_flat_one_not_multiplied_by_tags` encodait
explicitement l'ancien comportement fautif. Il a été renommé
`viral_enhancers_resolves_once_per_matching_tag` et ses assertions relevées vers le texte
imprimé (1 → 2). C'est le seul test existant modifié (limite du contrat : 5).

---

## 2. Les cartes que le moteur ignore — listées, non implémentées

Aucune de ces 31 cartes n'a été ajoutée à la table d'effets : c'est explicitement hors
périmètre. Elles sont des **stubs neutres** — le moteur les fait payer, compte leurs badges
et leurs PV fixes, et n'applique rien d'autre.

### 2.1 — ABSENT (7) : simple omission de la table

Pour celles-ci, le **concept de jeu** existe déjà dans l'état du moteur. Trois sont
littéralement à une ligne de table (`ResearchBonus`, mécanisme déjà utilisé par
*Interplanetary Relations*). Les quatre autres demandent en plus **une variante de**
**vocabulaire** — `Eff` ne sait pas « gagner un PV forêt » — mais aucune architecture
nouvelle : le compteur `players.forests` et la fonction `build_forest` existent et
fonctionnent (mesuré en §3, *Small Animals* et *Zeppelins*).

| Carte | Ce qu'il faudrait |
|---|---|
| **Biothermal Power** | **Variante de vocabulaire** : `Eff` n'a pas de « gagner un PV forêt ». Le compteur `players.forests` et la production de chaleur, eux, existent déjà. |
| **Extended Resources** | **Une ligne de table** : `ResearchBonus { draw: 0, keep: 1 }`, mécanisme déjà utilisé par Interplanetary Relations. |
| **Interns** | **Une ligne de table** : `ResearchBonus { draw: 2, keep: 0 }`. |
| **Mangrove** | Comme Biothermal Power : « Gain a forest VP and raise oxygen 1 step ». |
| **Plantation** | Comme Biothermal Power, en double (« Gain 2 forest VPs and raise oxygen 2 steps »). |
| **Protected Valley** | Comme Biothermal Power, plus une production de 2 MC. |
| **United Planetary Alliance** | **Une ligne de table** : `ResearchBonus { draw: 1, keep: 1 }` — encodage strictement identique à Interplanetary Relations, déjà encodée. |

### 2.2 — HORS-PORTEE (24) : le mécanisme lui-même manque

Ici, ajouter une ligne à la table ne suffirait pas : la règle imprimée demande un concept
que le moteur ne possède pas. C'est le vrai coût du lot suivant.

| Carte | Mécanisme manquant |
|---|---|
| **CrediCor** | Corporation : `Corporation` n'a AUCUN champ d'effet — il faut d'abord créer le mécanisme. Et `Reduction` n'a pas de variante conditionnée au coût imprimé (« 20 MC or more »). |
| **Interplanetary Cinematics** | Corporation : aucun champ d'effet sur la structure. Les deux réductions ([building] −2, [event] −2) sont exprimables, la table qui les porterait n'existe pas. |
| **Saturn Systems** | Corporation : aucun champ d'effet. De plus « Each time you play a [jupiter] … **gain 1 TR** » est inexprimable : `TrigGain` n'offre que Heat, Plants, Draw, ResSelf et Choose — aucun gain de TR sur un déclencheur de pose. |
| **Teractor Corporation** | Corporation : aucun champ d'effet sur la structure. La réduction [earth] −3 serait exprimable, la table qui la porterait n'existe pas. |
| **Tharsis Republic** | Corporation : aucun champ d'effet sur la structure. Le bonus de recherche serait exprimable, la table qui le porterait n'existe pas. |
| **Thorgate Corporation** | Corporation : aucun champ d'effet sur la structure. La réduction [energy] −3 et la production de départ seraient exprimables, la table qui les porterait n'existe pas. |
| **Adaptation Technology** | Assouplissement des PRÉREQUIS (« consider the oxygen or temperature one color higher or lower »). Aucun mécanisme de modification de prérequis n'existe dans le moteur. |
| **Advanced Alloys** | Le titane et l'acier comme RESSOURCES dépensables. `steel_capacity` / `titanium_capacity` existent dans `PlayerState` mais ne sont jamais alimentés (initialisés à 0, lus par une seule récompense). |
| **Assembly Lines** | Déclencheur « when you use an "Action:" effect ». Le moteur n'émet aucun événement à l'activation d'une action de carte. |
| **Asset Liquidation** | « You may play an additional blue or red card this phase » : une carte ne peut pas accorder de pose supplémentaire (seul le sélectionneur de phase II le peut, via `ConstructionBonus`). |
| **Composting Factory** | Modifie la valeur de la défausse pour MC. `SELL_CARD_MC` est une constante du moteur, non modifiable par une carte. |
| **Ecoline** | Corporation. Réduit le coût en plantes de l'ACTION STANDARD forêt. Les coûts des actions standard sont des constantes. |
| **Helion** | Corporation. « You may use heat as MC » : aucune conversion chaleur→MC n'existe au moment du paiement. |
| **Hydro-Electric Energy** | La carte-témoin du chantier. `ActionEff` ne sait pas gagner de la CHALEUR (variantes : Draw, Plants, Mc, Tr, Oxygen). L'action imprimée « Spend 1 MC to gain 2 heat » est donc inexprimable en l'état. |
| **Inventrix** | Corporation. Même assouplissement de prérequis qu'Adaptation Technology, plus une pioche à la mise en place. |
| **Mars University** | « you may discard a card. If that card had a [plant], draw two cards » : défausse avec choix DANS un déclencheur de pose, puis lecture du badge de la carte défaussée. Rien de tel n'existe. |
| **Mining Guild** | Corporation. « Each time you play steel production, excluding this, gain 1 TR » : la production d'acier n'est pas modélisée. |
| **PhoboLog** | Corporation. Réduction par TITANE possédé — même mécanisme manquant qu'Advanced Alloys. |
| **Restructured Resources** | « you may spend 1 plant to reduce that card's cost by 5 MC ». `Reduction::PayResources` ne sait payer qu'avec des ressources POSÉES SUR UNE CARTE, pas avec les plantes du joueur. |
| **Solarpunk** | Action qui accorde un PV forêt (`ActionEff` n'a pas de forêt) ET réduction par titane. |
| **Standard Technology** | « You pay 4 MC less for standard actions that cost MC ». Les coûts des actions standard sont des constantes du moteur. |
| **United Nations Mars Initiative** | Corporation. « The first time your TR is raised each phase » : aucun déclencheur sur la hausse de TR, ni compteur « première fois de la phase ». |
| **Water Import from Europa** | Action qui retourne un océan (`ActionEff` n'a pas d'océan) ET réduction par titane possédé. |
| **Wood Burning Stoves** | Action dont le COÛT change selon la phase choisie (« spend 3 plants instead »). `Action::Fixed` n'a pas de coût conditionnel. |

**Les cinq familles de mécanismes manquants**, par ordre de portée :

1. **Les corporations n'ont pas d'effets du tout** — 12 cartes, de loin le plus gros poste.
   La structure `Corporation` (`cards.rs`) ne porte que `name`, `starting_mc` et `tags` :
   elle **n'a aucun champ d'effet**. Les 12 corporations du périmètre ne sont donc pas
   muettes par oubli de table — **la table n'existe pas pour elles**. C'est la raison pour
   laquelle elles figurent toutes ici et non en §2.1, même celles dont la règle imprimée
   se dirait avec le vocabulaire existant.
2. **Acier et titane comme ressources** — 5 cartes. `steel_capacity` / `titanium_capacity`
   existent dans `PlayerState` mais restent à 0 : rien ne les alimente, rien ne les dépense.
3. **Vocabulaire d'action incomplet** — 4 cartes. `ActionEff` ne sait ni gagner de la chaleur,
   ni retourner un océan, ni accorder un PV forêt, ni changer de coût selon la phase choisie.
   *Hydro-Electric Energy, la carte-témoin de tout ce chantier, tombe précisément ici.*
4. **Modificateurs de coûts fixes du moteur** — 3 cartes. Actions standard et valeur de
   défausse sont des constantes qu'aucune carte ne peut toucher.
5. **Prérequis assouplissables** — 2 cartes.

À quoi s'ajoute un manque de vocabulaire repéré en chemin, qui touche aussi une carte
de §2.1 : `TrigGain` (déclencheurs de pose) n'offre que `Heat`, `Plants`, `Draw`,
`ResSelf` et `Choose` — **aucun gain de TR**, alors que Saturn Systems et Mining Guild
en demandent un ; et `Eff` n'a pas de « gagner un PV forêt », dont quatre cartes
dépendent.

### 2.3 — Preuve exécutée que le moteur ne fait RIEN pour ces 31 cartes

Le verdict « le moteur est muet » n'est pas une lecture de code : les 31 cartes ont été
sondées une par une (`outputs/work/absents.log`). **Aucune n'est dans le lot d'effets.**

| Cas | Nombre | Ce que dit la sonde |
|---|---|---|
| Cartes projet hors lot | 19 | `found: true, in_lot: false` — la carte est jouable et payée, mais son encodage n'existe pas : stub neutre |
| Corporations | 12 | `found: false` — elles ne sont même pas résolvables par la sonde, qui n'interroge que `db.projects` |

Le second cas est le plus parlant : une corporation **ne peut pas être sondée du tout**.
Ce n'est pas une carte oubliée dans une table, c'est une catégorie d'objets sans effets.

*la carte-témoin de tout ce chantier : trouvée et jouable, mais hors lot — delta entièrement nul :*

```
$ simulate --cards inputs/cards.json --probe "Hydro-Electric Energy"
{"card":"Hydro-Electric Energy","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":false,"paid":[11],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

*une corporation : la sonde ne la résout même pas, elle n'interroge que `db.projects` :*

```
$ simulate --cards inputs/cards.json --probe Credicor
{"card":"Credicor","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[],"found":false,"in_lot":false,"paid":[],"played":false,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```


---

## 3. Les 33 cartes CONFORMES — preuve exécutée pour chacune

Chaque verdict ci-dessous porte une commande de sonde et sa sortie réelle, collée telle
quelle. Les commandes se relancent depuis la racine du workspace.

**Sur le régime `Action:`** — le piège central de ce chantier. Une sonde `--probe-action`
ne prouve qu'UNE activation : elle ne distingue donc pas une capacité répétable d'un gain
unique à la pose. Les quatre cartes à capacité `Action:` du périmètre (**Birds**,
**Conserved Biome**, **Extreme-Cold Fungus**, **Symbiotic Fungus**) sont donc prouvées
répétables **par le flux réel** : un test qui les active **deux fois dans la même partie**
via `play_round`, en phase III, le joueur étant sélectionneur de phase (le livret p.14 lui
accorde une activation supplémentaire). Sans cette seconde activation, le verdict serait
`NON PROUVÉ`.

**Portée exacte de cette preuve, pour ne pas la surinterpréter** : elle établit deux
activations dans la MÊME PARTIE — ce qu'exige le contrat — en l'occurrence deux fois dans
la même phase III. Elle n'établit pas que la capacité revient à chaque manche : le livret
p.10 interdit de rejouer la phase de la manche précédente, et je n'ai pas mesuré ce second
point. Ce qui est réfuté, et c'était tout l'enjeu, c'est le régime « gain unique à la pose ».

### Anaerobic Microorganisms — CONFORME

**Imprimé** : « Effect: When you play a card, you may remove 2 microbes from this card to pay 10 MC less for that card. Effect: When you play an [animal], [microbe], or [plant], including this, add a microbe to this card. »

```
$ simulate --cards inputs/cards.json --probe "Anaerobic Microorganisms;Adapted Lichen;Moss;Grass" --probe-choice 0,0,0,0
{"card":"Grass","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":12,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":3,"plants":2,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0,0],"found":true,"in_lot":true,"paid":[10,6,3,9],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Anaerobic Microorganisms","kind":"microbe","n":1}],"target_error":null,"vp":0,"vp_total":0}
```

La réduction payée en microbes est bien un effet PERMANENT et un CHOIX : `delta.mc` = 12 (le joueur a économisé 12 MC sur les poses suivantes) contre 0 quand on refuse la branche (`--probe-choice "1,1,1,1"`, 5 microbes conservés). Les microbes s'accumulent à chaque carte [animal]/[microbe]/[plant] jouée, pas seulement à la pose.

*Point de contrôle — réduction REFUSÉE : delta.mc reste à 0, les 5 microbes sont conservés :*

```
$ simulate --cards inputs/cards.json --probe "Anaerobic Microorganisms;Adapted Lichen;Moss;Grass" --probe-choice 1,1,1,1
{"card":"Grass","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":3,"plants":2,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0,0],"found":true,"in_lot":true,"paid":[10,6,3,9],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Anaerobic Microorganisms","kind":"microbe","n":5}],"target_error":null,"vp":0,"vp_total":0}
```

### Anti-Gravity Technology — CONFORME

**Imprimé** : *Requires 5 [science].* — « Effect: When you play a card, gain 2 heat and 2 plants. »

```
$ simulate --cards inputs/cards.json --probe "Anti-Gravity Technology;Cartel;Building Industries"
{"card":"Building Industries","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":4,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[18,6,6],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":3}
```

`Effect:` armé en permanence : +2 plantes à CHAQUE carte jouée ensuite (`plants` = 4 pour 2 poses ; la chaleur gagnée est reprise par la dépense de Building Industries). Prérequis 5 badges [science] encodé.

*Point de contrôle — seule, elle ne se déclenche PAS sur sa propre pose (delta entièrement nul) — le texte ne dit pas « including this » :*

```
$ simulate --cards inputs/cards.json --probe "Anti-Gravity Technology"
{"card":"Anti-Gravity Technology","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[18],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":3,"vp_total":3}
```

*Point de contrôle — les deux mêmes cartes SANS elle : `heat: -4` (Building Industries coûte 4 chaleur). Avec elle, le delta chaleur est nul — donc elle a bien rendu +4, soit 2 chaleur par carte jouée, deux fois :*

```
$ simulate --cards inputs/cards.json --probe "Cartel;Building Industries"
{"card":"Building Industries","delta":{"card_prod":0,"forests":0,"hand":0,"heat":-4,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[6,6],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Arctic Algae — CONFORME

**Imprimé** : *Requires red temperature or warmer.* — « Effect: When you flip an ocean tile, gain 4 plants. »

```
$ simulate --cards inputs/cards.json --probe "Arctic Algae;Artificial Lake;Ice Asteroid"
{"card":"Ice Asteroid","delta":{"card_prod":0,"forests":0,"hand":1,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":3,"oxygen":0,"plant_prod":0,"plants":14,"temperature":0,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[19,13,21],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":3}
```

Déclencheur global permanent sur CHAQUE océan retourné : 3 océans → 12 plantes + 2 du bonus de tuile = 14. Un effet unique à la pose en aurait donné 4.

*Point de contrôle — témoin : seule, aucun océan retourné, aucune plante gagnée :*

```
$ simulate --cards inputs/cards.json --probe "Arctic Algae"
{"card":"Arctic Algae","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[19],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":2,"vp_total":2}
```

### Birds — CONFORME

**Imprimé** : *Requires white oxygen.* — « *=1 VP per animal on this card. Action: Add an animal to this card. »

```
$ simulate --cards inputs/cards.json --probe-action Birds
{"action_applied":true,"card":"Birds","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"found":true,"has_action":true,"in_lot":true,"resources":[{"card":"Birds","kind":"animal","n":1}],"target_error":null}
```

Capacité `Action:` présente et applicable. **Répétabilité prouvée séparément par le flux réel** : test `birds_action_is_repeatable_within_the_same_game` — deux activations dans la même partie via `play_round` donnent 2 animaux (et `blue_actions == 2`).

### Conserved Biome — CONFORME

**Imprimé** : « *=1 VP per 2 forest VPs you have. Action: Add a microbe to ANOTHER* card or add an animal to ANOTHER* card. »

```
$ simulate --cards inputs/cards.json --probe-action "Conserved Biome"
{"action_applied":false,"card":"Conserved Biome","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"found":true,"has_action":true,"in_lot":true,"resources":[],"target_error":null}
```

`action_applied:false` ici est correct : sans AUTRE carte porteuse en jeu, « ANOTHER* card » n'a pas de cible. **Répétabilité prouvée par le flux réel** : test `conserved_biome_action_is_repeatable_within_the_same_game` (2 activations → 2 microbes sur Tardigrades, 0 sur elle-même).

Sur le §G2 de cette carte (« 1 VP per 2 **forest VPs** » et non « per 2 forests ») : l'encodage lit `VpKind::Forest` → `players.forests` (`flow.rs`), et `players.forests` n'est incrémenté qu'en un seul endroit du moteur, `build_forest` — c'est-à-dire au gain d'un PV forêt. Les deux formulations désignent donc le même compteur. **C'est une lecture de code, pas une sonde** ; ce qui est mesuré, lui, c'est que ce compteur est bien celui que le jeu alimente (test `zeppelins_counts_the_same_forest_vp_counter_that_small_animals_watches`).

### Earth Catapult — CONFORME

**Imprimé** : « Effect: When you play a card, you pay 2 MC less for it. »

```
$ simulate --cards inputs/cards.json --probe "Earth Catapult;Cartel;Building Industries"
{"card":"Building Industries","delta":{"card_prod":0,"forests":0,"hand":0,"heat":-4,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[24,4,4],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":2}
```

Réduction permanente −2 MC sur TOUTE carte : Cartel (6) payée 4, Building Industries (6) payée 4. Les deux poses ultérieures en bénéficient, donc l'effet n'est pas consommé.

*Point de contrôle — témoin : elle-même se paie PLEIN TARIF (24) — une carte ne se réduit jamais elle-même :*

```
$ simulate --cards inputs/cards.json --probe "Earth Catapult"
{"card":"Earth Catapult","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[24],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":2,"vp_total":2}
```

### Ecological Zone — CONFORME

**Imprimé** : « *=1 VP per 2 animals on this card. Effect: When you play a [animal] or [plant], including these, add an animal to this card. »

```
$ simulate --cards inputs/cards.json --probe "Ecological Zone;Algae;Grass"
{"card":"Grass","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":3,"plants":3,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[11,9,9],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Ecological Zone","kind":"animal","n":4}],"target_error":null,"vp":2,"vp_total":2}
```

Déclencheur permanent : 4 animaux (2 pour sa propre pose — elle porte [animal] ET [plant], « including these », deux conditions remplies — puis +1 par carte [plant] jouée).

*Point de contrôle — témoin : seule, elle se pose déjà 2 animaux (ses deux badges, « including these ») :*

```
$ simulate --cards inputs/cards.json --probe "Ecological Zone"
{"card":"Ecological Zone","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[11],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Ecological Zone","kind":"animal","n":2}],"target_error":null,"vp":1,"vp_total":1}
```

### Energy Subsidies — CONFORME

**Imprimé** : « Effect: When you play an [energy], you pay 4 MC less for it and you draw a card. »

```
$ simulate --cards inputs/cards.json --probe "Energy Subsidies;Fueled Generators;Geothermal Power"
{"card":"Geothermal Power","delta":{"card_prod":0,"forests":0,"hand":2,"heat":0,"heat_prod":4,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":-1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[5,0,4],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

Réduction permanente −4 MC sur [energy] : Fueled Generators (4) payée 0, Geothermal Power (8) payée 4 ; et `hand` = +2, soit une pioche par carte [energy]. Deux déclenchements.

*Point de contrôle — témoin : sans elle, Fueled Generators se paie plein tarif (4) et ne fait pas piocher :*

```
$ simulate --cards inputs/cards.json --probe "Fueled Generators"
{"card":"Fueled Generators","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":2,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":-1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[4],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Extreme-Cold Fungus — CONFORME

**Imprimé** : *Requires purple temperature.* — « Action: Gain 1 plant or add a microbe to ANOTHER* card. »

```
$ simulate --cards inputs/cards.json --probe-action "Extreme-Cold Fungus" --probe-choice 0
{"action_applied":true,"card":"Extreme-Cold Fungus","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":1,"temperature":0,"tr":0},"found":true,"has_action":true,"in_lot":true,"resources":[],"target_error":null}
```

Branche 0 du texte imprimé (« Gain 1 plant ») appliquée. **Répétabilité prouvée par le flux réel** : test `extreme_cold_fungus_action_is_repeatable_within_the_same_game` — 1 activation = 1 plante, 2 activations = 2 plantes.

### Fish — CONFORME

**Imprimé** : *Requires red temperature or warmer.* — « *=1 VP per animal on this card. Effect: When you flip an ocean tile, add 1 animal to this card. »

```
$ simulate --cards inputs/cards.json --probe "Fish;Artificial Lake;Ice Asteroid"
{"card":"Ice Asteroid","delta":{"card_prod":0,"forests":0,"hand":1,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":3,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[11,13,21],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Fish","kind":"animal","n":3}],"target_error":null,"vp":3,"vp_total":4}
```

Déclencheur permanent : 3 animaux pour 3 océans retournés. Les PV variables (1 PV par animal) montent donc réellement pendant la partie.

*Point de contrôle — témoin : seule, elle entre en jeu VIDE (0 animal) — rien n'est accordé à la pose :*

```
$ simulate --cards inputs/cards.json --probe Fish
{"card":"Fish","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[11],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Fish","kind":"animal","n":0}],"target_error":null,"vp":0,"vp_total":0}
```

### Fusion Power — CONFORME

**Imprimé** : *Requires 2 [energy].* — « During the production phase, draw a card. »

```
$ simulate --cards inputs/cards.json --probe-strict "Fueled Generators;Geothermal Power;Fusion Power"
{"card":"Fusion Power","delta":{"card_prod":1,"forests":0,"hand":0,"heat":0,"heat_prod":4,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":-1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[4,8,7],"played":true,"prereq_ok":false,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

§G2 tranchée : « Requires 2 [energy] » est évalué sur les BADGES. Sans badge [energy], `--probe-strict "Fusion Power"` refuse la pose (`played:false`) ; après deux cartes [energy], elle passe. Le moteur ne teste pas une ressource énergie (qui n'existe pas dans ce jeu).

*Point de contrôle — témoin : sans badge [energy], le mode strict REFUSE la pose (played:false) :*

```
$ simulate --cards inputs/cards.json --probe-strict "Fusion Power"
{"card":"Fusion Power","delta":{"card_prod":0,"forests":0,"hand":1,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[],"found":true,"in_lot":true,"paid":[],"played":false,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Herbivores — CONFORME

**Imprimé** : *Requires 5 ocean tiles to be flipped.* — « *=1 VP per 2 animals on this card. Effect: When you raise oxygen, flip an ocean tile, or raise the temperature, add 1 animal to this card. »

```
$ simulate --cards inputs/cards.json --probe "Herbivores;Airborne Radiation;Artificial Lake;Deep Well Heating"
{"card":"Deep Well Heating","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":3,"mc":0,"mc_prod":0,"oceans":1,"oxygen":1,"plant_prod":0,"plants":2,"temperature":1,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0,0],"found":true,"in_lot":true,"paid":[25,15,13,14],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Herbivores","kind":"animal","n":3}],"target_error":null,"vp":1,"vp_total":2}
```

Les TROIS déclencheurs imprimés sont armés en permanence : hausse d'oxygène, océan retourné, hausse de température → 3 animaux. Prérequis « 5 ocean tiles » encodé.

*Point de contrôle — témoin : seule, elle entre en jeu vide :*

```
$ simulate --cards inputs/cards.json --probe Herbivores
{"card":"Herbivores","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[25],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Herbivores","kind":"animal","n":0}],"target_error":null,"vp":0,"vp_total":0}
```

### Insects — CONFORME

**Imprimé** : « During the production phase, this produces 1 plant per [plant] you have. »

```
$ simulate --cards inputs/cards.json --probe "Algae;Grass;Insects" --probe-produce
{"card":"Insects","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":3,"plants":8,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":2},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[9,9,10],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

§G2 tranchée : le décompte porte sur les BADGES [plant], pas sur les plantes-ressources. `derived.plants` = 2 avec deux cartes à badge [plant] en jeu, alors que le joueur détient 8 plantes-ressources. Recalculé à chaque phase IV, jamais inscrit sur `plant_prod`.

*Point de contrôle — témoin : seule (elle porte [microbe], pas [plant]), elle ne produit rien :*

```
$ simulate --cards inputs/cards.json --probe Insects --probe-produce
{"card":"Insects","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[10],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Interplanetary Conference — CONFORME

**Imprimé** : « Effect: When you play an [earth] or [jupiter], excluding this, you pay 3 MC less and draw a card. »

```
$ simulate --cards inputs/cards.json --probe "Interplanetary Conference;Cartel;Ganymede Shipyard"
{"card":"Ganymede Shipyard","delta":{"card_prod":0,"forests":0,"hand":2,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[6,3,14],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

Réduction permanente −3 MC + pioche sur [earth]/[jupiter] : Cartel (6) payée 3, Ganymede Shipyard (17) payée 14, `hand` = +2. « excluding this » respecté : elle porte [earth] et se paie plein tarif (6).

*Point de contrôle — témoin : seule, hand = 0 malgré son badge [earth] — « excluding this » respecté :*

```
$ simulate --cards inputs/cards.json --probe "Interplanetary Conference"
{"card":"Interplanetary Conference","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[6],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Interplanetary Relations — CONFORME

**Imprimé** : « *= 1 VP per four cards you have played. Effect: When you draw cards during the research phase, draw one additional card and keep one additional card. »

```
$ simulate --cards inputs/cards.json --probe "Interplanetary Relations"
{"card":"Interplanetary Relations","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[35],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

La sonde CLI ne peut pas atteindre la phase V. **Preuve par le flux réel** : test `interplanetary_relations_research_bonus_stays_armed_for_the_whole_game` — le bonus fait réellement piocher en plus sur PLUSIEURS manches de phase Recherche (compteur d'audit `research_extra_draws`), et non une seule fois à la pose.

### Large Convoy — CONFORME

**Imprimé** : « [effect] Flip an ocean tile. [effect] Draw two cards. [effect] Gain 5 plants or add 3 animals to ANOTHER card. »

```
$ simulate --cards inputs/cards.json --probe "Birds;Large Convoy" --probe-choice 1 --probe-target Birds
{"card":"Large Convoy","delta":{"card_prod":0,"forests":0,"hand":2,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[15,36],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Birds","kind":"animal","n":3}],"target_error":null,"vp":5,"vp_total":5}
```

Effet immédiat conforme : océan retourné, 2 cartes piochées, et la branche 2 pose 3 animaux sur une AUTRE carte (Birds) — c'est la SECONDE branche du texte, indice 1.

**Sur §G1 (« ANY » dans la paraphrase, « ANOTHER » sur le carton), je suis honnête : la sonde ne DISTINGUE pas les deux lectures.** L'encodage porte bien `ResTarget::Another` (`effects.rs`, `put_another(K_ANIMAL, 3)`), mais Large Convoy est déclarée `holds: None` — elle ne porte aucune ressource, donc elle ne serait de toute façon jamais une cible valide, même encodée en `Any`. L'écart de la paraphrase est **sans conséquence observable ici** ; le verdict CONFORME porte sur l'effet mesuré (océan, 2 cartes, 3 animaux sur une autre carte), pas sur une distinction que rien ne peut départager.

*Point de contrôle — PREMIÈRE branche du texte imprimé (« Gain 5 plants », indice 0) — 5 plantes + 2 du bonus de tuile océan :*

```
$ simulate --cards inputs/cards.json --probe "Large Convoy" --probe-choice 0
{"card":"Large Convoy","delta":{"card_prod":0,"forests":0,"hand":2,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":0,"plant_prod":0,"plants":7,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[36],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":2,"vp_total":2}
```

### Livestock — CONFORME

**Imprimé** : *Requires yellow oxygen or higher.* — « *=1 VP per animal on this card. Effect: When you raise the temperature, add 1 animal to this card. »

```
$ simulate --cards inputs/cards.json --probe "Livestock;Deep Well Heating;Soil Warming"
{"card":"Soil Warming","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":1,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":2,"plants":0,"temperature":2,"tr":2},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[15,14,24],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Livestock","kind":"animal","n":2}],"target_error":null,"vp":2,"vp_total":2}
```

Déclencheur permanent sur la température : 2 hausses → 2 animaux. Prérequis « yellow oxygen or higher » encodé.

*Point de contrôle — témoin : seule, elle entre en jeu vide :*

```
$ simulate --cards inputs/cards.json --probe Livestock
{"card":"Livestock","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[15],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Livestock","kind":"animal","n":0}],"target_error":null,"vp":0,"vp_total":0}
```

### Media Group — CONFORME

**Imprimé** : « Effect: When you play an [event], you pay 5 MC less for it. »

```
$ simulate --cards inputs/cards.json --probe "Media Group;Artificial Lake;Atmosphere Filtering"
{"card":"Atmosphere Filtering","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":1,"plant_prod":0,"plants":2,"temperature":0,"tr":2},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[11,8,1],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

Réduction permanente −5 MC sur [event] : Artificial Lake (13) payée 8, Atmosphere Filtering (6) payée 1. Deux poses ultérieures réduites.

*Point de contrôle — témoin : sans elle, Artificial Lake se paie plein tarif (13) :*

```
$ simulate --cards inputs/cards.json --probe "Artificial Lake"
{"card":"Artificial Lake","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[13],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Medical Lab — CONFORME

**Imprimé** : « During the production phase, produce 1 MC per 2 [building] you have, including this. »

```
$ simulate --cards inputs/cards.json --probe "Building Industries;Medical Lab" --probe-produce
{"card":"Medical Lab","delta":{"card_prod":0,"forests":0,"hand":0,"heat":-4,"heat_prod":0,"mc":6,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":1,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[6,11],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

§G2 tranchée : décompte sur les BADGES [building], division entière, « including this ». Seule, 1 badge → 0 MC ; avec une seconde carte [building], 2 badges → 1 MC.

*Point de contrôle — témoin : seule (1 badge [building]), la division entière donne 0 — pas de derived_prod :*

```
$ simulate --cards inputs/cards.json --probe "Medical Lab" --probe-produce
{"card":"Medical Lab","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[15],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Nitrogen-Rich Asteroid — CONFORME

**Imprimé** : « [effect] Raise your TR 2 steps. [effect] Raise the temperature 1 step. [effect] Gain 2 plants. If you have have 3 or more [plant], gain 4 additional plants. »

```
$ simulate --cards inputs/cards.json --probe "Algae;Grass;Lichen;Nitrogen-Rich Asteroid"
{"card":"Nitrogen-Rich Asteroid","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":4,"plants":9,"temperature":1,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0,0],"found":true,"in_lot":true,"paid":[9,9,5,30],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

§G2 tranchée : le seuil « 3 or more [plant] » porte sur les BADGES. Seule, la carte donne 2 plantes ; après trois cartes à badge [plant], le bonus de 4 plantes s'ajoute. TR +2 et température +1 conformes.

*Point de contrôle — témoin : sans badge [plant], le bonus de 4 plantes ne se déclenche pas :*

```
$ simulate --cards inputs/cards.json --probe "Nitrogen-Rich Asteroid"
{"card":"Nitrogen-Rich Asteroid","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":2,"temperature":1,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[30],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Olympus Conference — CONFORME

**Imprimé** : « Effect: When you play a [science], including this, draw a card. »

```
$ simulate --cards inputs/cards.json --probe "Olympus Conference;Development Center;Physics Complex"
{"card":"Physics Complex","delta":{"card_prod":0,"forests":0,"hand":3,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[15,7,5],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Physics Complex","kind":"science","n":0}],"target_error":null,"vp":0,"vp_total":1}
```

Déclencheur permanent, « including this » : +1 carte à sa propre pose (elle porte [science]) puis +1 par carte [science] jouée ensuite → `hand` = +3.

*Point de contrôle — témoin : seule, elle pioche déjà 1 carte — « including this », elle porte [science] :*

```
$ simulate --cards inputs/cards.json --probe "Olympus Conference"
{"card":"Olympus Conference","delta":{"card_prod":0,"forests":0,"hand":1,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[15],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Optimal Aerobraking — CONFORME

**Imprimé** : « Effect: When you play an [event], you gain 2 heat and 2 plants. »

```
$ simulate --cards inputs/cards.json --probe "Optimal Aerobraking;Artificial Lake;Atmosphere Filtering"
{"card":"Atmosphere Filtering","delta":{"card_prod":0,"forests":0,"hand":0,"heat":4,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":1,"plant_prod":0,"plants":6,"temperature":0,"tr":2},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[10,13,6],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

Déclencheur permanent sur [event] : 2 events → +4 chaleur et +4 plantes (les 2 plantes de plus viennent du bonus de tuile océan).

*Point de contrôle — témoin : sans elle, Artificial Lake ne donne que le bonus de tuile (2 plantes) :*

```
$ simulate --cards inputs/cards.json --probe "Artificial Lake"
{"card":"Artificial Lake","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[13],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Physics Complex — CONFORME

**Imprimé** : *Requires 4 [science].* — « *= 1 VP per 2 science resources on this card. Effect: When you raise the temperature, add 1 science resource to this card. »

```
$ simulate --cards inputs/cards.json --probe "Physics Complex;Deep Well Heating;Soil Warming"
{"card":"Soil Warming","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":1,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":2,"plants":0,"temperature":2,"tr":2},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[5,14,24],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[{"card":"Physics Complex","kind":"science","n":2}],"target_error":null,"vp":1,"vp_total":1}
```

Déclencheur permanent sur la température : 2 hausses → 2 ressources science posées. La carte accumule donc bien ses PV variables. Prérequis 4 badges [science] encodé.

*Point de contrôle — témoin : seule, elle entre en jeu vide :*

```
$ simulate --cards inputs/cards.json --probe "Physics Complex"
{"card":"Physics Complex","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[5],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Physics Complex","kind":"science","n":0}],"target_error":null,"vp":0,"vp_total":0}
```

### Power Grid — CONFORME

**Imprimé** : « During the production phase, this produces 1 MC per [energy] you have, including this. »

```
$ simulate --cards inputs/cards.json --probe "Fueled Generators;Power Grid" --probe-produce
{"card":"Power Grid","delta":{"card_prod":0,"forests":0,"hand":0,"heat":2,"heat_prod":2,"mc":6,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":-1},"derived_prod":{"heat":0,"mc":2,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[4,8],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

§G2 tranchée : décompte sur les BADGES [energy], « including this ». Seule → 1 MC ; avec une seconde carte [energy] → 2 MC. Recalculé à chaque phase IV.

*Point de contrôle — témoin : seule en jeu, elle produit déjà 1 MC :*

```
$ simulate --cards inputs/cards.json --probe "Power Grid" --probe-produce
{"card":"Power Grid","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":6,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":1,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[8],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Recycled Detritus — CONFORME

**Imprimé** : « Effect: When you play an [event], draw two cards. »

```
$ simulate --cards inputs/cards.json --probe "Recycled Detritus;Artificial Lake;Atmosphere Filtering"
{"card":"Atmosphere Filtering","delta":{"card_prod":0,"forests":0,"hand":4,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":1,"plant_prod":0,"plants":2,"temperature":0,"tr":2},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[24,13,6],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":2}
```

Déclencheur permanent sur [event] : `hand` = +4, soit deux cartes piochées par event, deux fois. Pas une pioche unique à la pose.

*Point de contrôle — témoin : sans elle, Artificial Lake ne fait piocher aucune carte :*

```
$ simulate --cards inputs/cards.json --probe "Artificial Lake"
{"card":"Artificial Lake","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":1,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[13],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Research Outpost — CONFORME

**Imprimé** : « Effect: When you play a card, you pay 1 MC less for it. »

```
$ simulate --cards inputs/cards.json --probe "Research Outpost;Cartel;Building Industries"
{"card":"Building Industries","delta":{"card_prod":0,"forests":0,"hand":0,"heat":-4,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[6,5,5],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

Réduction permanente −1 MC sur toute carte : Cartel (6) payée 5, Building Industries (6) payée 5.

*Point de contrôle — témoin : sans elle, Cartel se paie plein tarif (6) :*

```
$ simulate --cards inputs/cards.json --probe Cartel
{"card":"Cartel","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[6],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Small Animals — CONFORME

**Imprimé** : *Requires red temperature or warmer.* — « * = 1 VP per 2 animals on this card. Effect: When you gain a forest VP, add 1 animal to this card. »

```
$ simulate --cards inputs/cards.json --probe "Small Animals"
{"card":"Small Animals","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[9],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Small Animals","kind":"animal","n":0}],"target_error":null,"vp":0,"vp_total":0}
```

La sonde CLI ne peut pas bâtir de forêt (action de phase III). **Preuve par le flux réel** : test `small_animals_gains_one_animal_per_forest_vp_over_several_rounds` — l'invariant « autant d'animaux que de PV forêt » tient à chaque manche, sur au moins deux forêts gagnées sur des manches DISTINCTES. §G1 tranchée : le déclencheur du moteur est branché sur le compteur de PV forêt (`players.forests`), le seul qui existe.

### Symbiotic Fungus — CONFORME

**Imprimé** : *Requires red temperature or warmer.* — « Action: Add a microbe to ANOTHER* card. »

```
$ simulate --cards inputs/cards.json --probe-action "Symbiotic Fungus"
{"action_applied":false,"card":"Symbiotic Fungus","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"found":true,"has_action":true,"in_lot":true,"resources":[],"target_error":null}
```

`action_applied:false` sans autre carte porteuse est correct (« ANOTHER* card »). **Répétabilité prouvée par le flux réel** : test `symbiotic_fungus_action_is_repeatable_within_the_same_game` (2 activations → 2 microbes).

*Point de contrôle — cette sonde n'établit QUE la présence d'une carte porteuse (Decomposers, 0 microbe) ; `--probe-action` n'accepte pas de séquence, donc l'action elle-même n'est prouvée QUE par le test en flux réel :*

```
$ simulate --cards inputs/cards.json --probe "Decomposers;Symbiotic Fungus"
{"card":"Symbiotic Fungus","delta":{"card_prod":0,"forests":0,"hand":1,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[7,3],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":false,"resources":[{"card":"Decomposers","kind":"microbe","n":0}],"target_error":null,"vp":0,"vp_total":1}
```

### Terraforming Ganymede — CONFORME

**Imprimé** : « [effect] Raise your TR 1 step per [jupiter] you have, including this. »

```
$ simulate --cards inputs/cards.json --probe "Ganymede Shipyard;Vesta Shipyard;Terraforming Ganymede"
{"card":"Terraforming Ganymede","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":3},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[17,10,19],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":2,"vp_total":3}
```

§G2 tranchée : TR +1 par BADGE [jupiter], « including this ». Seule → +1 ; après deux autres cartes [jupiter] → +3.

*Point de contrôle — témoin : seule, un seul badge [jupiter] — le sien — donc TR +1 :*

```
$ simulate --cards inputs/cards.json --probe "Terraforming Ganymede"
{"card":"Terraforming Ganymede","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":1},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[28],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":2,"vp_total":2}
```

### Titanium Mine — CONFORME

**Imprimé** : « When you play a [space], you pay 3 MC less for it. »

```
$ simulate --cards inputs/cards.json --probe "Titanium Mine;Atmospheric Insulators;Optimal Aerobraking"
{"card":"Optimal Aerobraking","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[7,7,7],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

§G2 tranchée : la réduction est bien de 3 **MC** sur les cartes [space], appliquée en permanence — les deux cartes [space] suivantes (10) sont payées 7.

*Point de contrôle — témoin : sans elle, Atmospheric Insulators se paie plein tarif (10) :*

```
$ simulate --cards inputs/cards.json --probe "Atmospheric Insulators"
{"card":"Atmospheric Insulators","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":0,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[10],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":false,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Windmills — CONFORME

**Imprimé** : « During the production phase, this produces 1 heat per [energy] you have, including this. »

```
$ simulate --cards inputs/cards.json --probe "Fueled Generators;Windmills" --probe-produce
{"card":"Windmills","delta":{"card_prod":0,"forests":0,"hand":0,"heat":4,"heat_prod":2,"mc":4,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":-1},"derived_prod":{"heat":2,"mc":0,"plants":0},"discarded":[0,0],"found":true,"in_lot":true,"paid":[4,10],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":1,"vp_total":2}
```

§G1 tranchée : « including this » est bien pris en compte. Seule → 1 chaleur (son propre badge [energy]) ; avec une seconde carte [energy] → 2. La production n'était PAS inférieure de 1.

*Point de contrôle — témoin : seule en jeu, elle produit déjà 1 chaleur — c'est son propre badge :*

```
$ simulate --cards inputs/cards.json --probe Windmills --probe-produce
{"card":"Windmills","delta":{"card_prod":0,"forests":0,"hand":0,"heat":1,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":1,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[10],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

### Worms — CONFORME

**Imprimé** : *Requires red oxygen or higher.* — « During the production phase, this produces 1 plant per [microbe] you have, including this. »

```
$ simulate --cards inputs/cards.json --probe "Symbiotic Fungus;Worms" --probe-produce
{"card":"Worms","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":2,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":2},"discarded":[0,0],"found":true,"in_lot":true,"paid":[3,11],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

§G2 tranchée : décompte sur les BADGES [microbe], « including this ». Seule → 1 plante ; avec une seconde carte [microbe] → 2.

*Point de contrôle — témoin : seule en jeu, elle produit déjà 1 plante :*

```
$ simulate --cards inputs/cards.json --probe Worms --probe-produce
{"card":"Worms","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":1,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":1},"discarded":[0],"found":true,"in_lot":true,"paid":[11],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":0}
```

### Zeppelins — CONFORME

**Imprimé** : *Requires red oxygen or higher.* — « During the production phase, this produces 1 MC per forest VP you have. »

```
$ simulate --cards inputs/cards.json --probe Zeppelins --probe-produce
{"card":"Zeppelins","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":5,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":0,"plants":0},"discarded":[0],"found":true,"in_lot":true,"paid":[10],"played":true,"prereq_ok":false,"prereq_ok_now":false,"produced":true,"resources":[],"target_error":null,"vp":1,"vp_total":1}
```

§G2 tranchée : le décompte porte sur les jetons de PV forêt (`ProdCount::Forests`), pas sur des badges ni des cartes. Sans forêt → 0. **Preuve avec forêts par le flux réel** : test `zeppelins_counts_the_same_forest_vp_counter_that_small_animals_watches` — après des forêts réellement bâties en phase III, la production dérivée vaut exactement le nombre de PV forêt.

*Point de contrôle — le 1 MC produit ici vient de Satellites (1 badge [space]), PAS de Zeppelins : ajouter des badges ne fait rien produire à Zeppelins, qui ne compte que les forêts :*

```
$ simulate --cards inputs/cards.json --probe "Zeppelins;Media Group;Satellites" --probe-produce
{"card":"Satellites","delta":{"card_prod":0,"forests":0,"hand":0,"heat":0,"heat_prod":0,"mc":6,"mc_prod":0,"oceans":0,"oxygen":0,"plant_prod":0,"plants":0,"temperature":0,"tr":0},"derived_prod":{"heat":0,"mc":1,"plants":0},"discarded":[0,0,0],"found":true,"in_lot":true,"paid":[10,11,14],"played":true,"prereq_ok":true,"prereq_ok_now":true,"produced":true,"resources":[],"target_error":null,"vp":0,"vp_total":1}
```

---

## 4. Les deux pièges de données (PARTIE 3)

### 4.1 — Homoglyphes cyrilliques : 18 entrées, pas 16 — sans effet sur le moteur

**Re-mesure.** Un balayage de tous les champs des 388 entrées de `inputs/cards.json` trouve
**18 entrées** portant un caractère cyrillique, **toutes dans le seul champ `description`**.
Les 16 annoncées sont exactes mais incomplètes. Les deux oubliées :

- **Progressive Policies** — même motif « МС » (U+041C U+0421), simplement omise de la liste.
- **Oxidation Byproducts** — cas **différent et plus grave** : `"During the production phase,
  this produces 2 руфе."` Ce n'est pas un homoglyphe « MC » mais un **mot entier corrompu**
  à l'emplacement du nom de la ressource produite. L'information est **détruite** : la carte
  est jouable (`in_deck_v1: true`) mais `cards.json` ne dit plus ce qu'elle produit, et elle
  est **absente de `inputs/textes-cartes.json`** — la source de vérité imprimée ne comble pas
  le trou. Il faut retourner au scan ou à la carte physique.

**Le moteur en dépend-il ? Non, et c'est prouvé.** Le désérialiseur `RawCard` (`cards.rs`)
ne déclare que `name`, `category`, `tags`, `price`, `in_deck_v1`, `vp`, `vp_dynamic` :
**`description` n'est jamais lu**. Le champ dont le moteur dépend vraiment est `name` —
la table d'effets y est appariée par égalité stricte de chaîne — et **aucun `name` ne
contient de caractère non-ASCII** (vérifié sur les 388 entrées, et verrouillé désormais par
le test `no_cyrillic_homoglyph_reaches_a_field_the_engine_depends_on`).

**C'est néanmoins une bombe amorcée.** Un homoglyphe injecté dans un `name` fait échouer le
chargement avec un message qui accuse la mauvaise chose (« carte 'Comet' résolue 0 fois »),
et aucun lecteur humain ne distingue `Comet` de `Сomet`.

**Correction de données livrée, `inputs/` non modifié** : `outputs/cards-homoglyphes-corriges.json`
— 17 entrées dont `МС` → `MC` dans `description`. `Oxidation Byproducts` est laissée en
l'état, avec une note ajoutée dans son champ `notes_retag` : la translittérer mécaniquement
serait inventer une donnée.

### 4.2 — Le `price` d'une corporation : le moteur fait bien REPRENDRE, pas payer

**Aucun défaut.** `flow.rs` affecte `game.players[p].mc = corp.starting_mc` au choix de la
corporation : c'est une affectation, jamais une soustraction, et `starting_mc` n'apparaît
nulle part dans les chemins de paiement ou d'affordabilité (qui travaillent tous sur
`db.projects`, jamais sur `db.corporations`).

Prouvé par exécution sur **les 16 corporations, 120 mises en place réelles** :
test `verite_tests.rs::corporation_price_is_starting_mc_and_is_granted_never_paid`.
Un joueur CrediCor démarre bien avec **48 MC en poche**, il ne les paie pas.

**En revanche — et ce n'est pas dans mon périmètre de correction — les productions de**
**départ des corporations ne sont pas accordées** : Ecoline (1 plante), Thorgate (1 chaleur),
Helion (3 chaleur), et les productions d'acier/titane d'Interplanetary Cinematics, Mining
Guild, PhoboLog et Saturn Systems. C'est un stub assumé et documenté du lot précédent,
mais le déséquilibre est réel : Helion perd 3 chaleur par génération sur une partie qui en
compte une soixantaine.

---

## 5. Ce que je n'ai pas prouvé, et les risques latents

### Un défaut de la SONDE elle-même, trouvé en chemin

Le champ `paid[]` de `--probe` **ment** lorsqu'une réduction payée en microbes s'applique
(Anaerobic Microorganisms). `probe.rs` recalcule le prix pour son compte
(`price - card_discount`) **avant** d'appeler `build_card_with`, et ignore donc la réduction
décidée à l'intérieur. C'est un second chemin de calcul, exactement ce que l'architecture
prétend ne pas avoir.

**Le moteur, lui, est correct** : sur la séquence `Anaerobic Microorganisms;Adapted Lichen;
Moss;Grass`, `delta.mc` vaut **12** quand la réduction est utilisée et **0** quand elle est
refusée (`--probe-choice "1,1,1,1"`), et les microbes sont bien consommés. C'est sur ces
champs-là — qui ne mentent pas — que repose le verdict CONFORME d'Anaerobic Microorganisms.
Une seule carte du moteur utilise `Reduction::PayResources` : `paid[]` est exact partout
ailleurs. **Correction non exécutée** (hors périmètre) — reportée en `result.md` §Adjacent work.

### Trois réglages formellement discutables mais INOBSERVABLES

`Optimal Aerobraking`, `Recycled Detritus` et `Energy Subsidies` portent des drapeaux
`scale_by_matched_tags` qui, au regard strict de la règle du livret l.106, pourraient être
faux. Mais **aucune carte de la pioche v1 ne porte deux badges [event] ni deux badges**
**[energy]** (mesuré : 0 sur 248). L'écart ne peut donc être ni observé ni testé. Je ne les
ai pas touchés : une correction qu'aucun test ne distingue serait de la configuration
inutilisée. **Risque latent** : le jour où une extension ajoute une carte à deux badges
[event] ou [energy], ces trois cartes deviendront fausses en silence.

### Une ambiguïté de lecture, tranchée par le livret — et un angle mort qu'elle révèle

`Interplanetary Conference` imprime « you pay 3 MC less » au singulier, mais accorde
**−6 MC** à une carte portant [earth] **et** [jupiter] (Miranda Resort, coût 15, payée 9).
Les deux lectures et la raison du choix sont détaillées dans `outputs/blocked.md`. J'ai
retenu celle du livret (l'effet entier se résout deux fois), conformément à la clause ALWAYS
du contrat. Le moteur n'a pas été modifié — le verdict CONFORME de cette carte est donc
**contingent de cet arbitrage**, ce que le tableau récapitulatif signale.

**Angle mort découvert en vérifiant cet arbitrage** : réduction et pioche ne suivent PAS le
même chemin. La pioche passe par `PlayTrigger` et se multiplie réellement par le nombre de
badges ; la réduction passe par `Reduction::amount_for`, qui teste `tags.contains(&t)` —
un **booléen, pas un compte**. Le −6 vient de deux entrées `Reduction::Tag` indépendantes,
pas d'un effet résolu deux fois. Sur une carte à **deux badges [earth]**, le moteur
piocherait 2 cartes mais ne réduirait que de 3. Aucune carte de la pioche v1 n'est dans ce
cas (mesuré : 0 sur 248), donc rien n'est faux aujourd'hui — mais c'est un **quatrième
risque latent**, de la même famille que les trois ci-dessus.

### Limites de méthode, assumées

- Le passage à l'échelle des résolutions multiples n'est testé que sur **2** badges
  satisfaisants. Aucune carte du jeu n'en porte 3 : la linéarité au-delà est une
  extrapolation, pas une mesure.
- La répartition entre les deux verdicts de non-encodage porte sur des cartes dont
  l'inaction est, elle, MESURÉE (§2.3) — mais le mécanisme manquant que je nomme pour
  chacune est une lecture du code, pas une expérience.
- Le partage entre les deux verdicts de non-encodage est un **jugement d'ingénierie** sur le coût
  d'implémentation, pas une mesure. La frontière retenue : « le concept de jeu existe-t-il
  déjà dans l'état du moteur ? »

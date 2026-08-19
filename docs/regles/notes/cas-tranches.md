# Cas de règle tranchés

Registre des points ambigus rencontrés pendant la construction du moteur, et de
la façon dont ils ont été tranchés. Un cas = une entrée. Statuts : **TRANCHÉ**
(source citée) ou **EN ATTENTE** (arbitrage à venir).

---

## Titane et acier : ressources ou compteurs de réduction ? — TRANCHÉ (24-07, livret p. 18)

- **Verdict du livret** (« SAVOIR-FAIRE - ACIER ET TITANE », p. 18, transcription
  `transcription-brute/photo-17.md`) : l'acier et le titane sont des
  **savoir-faire**, pas des ressources dépensables. « Chaque savoir-faire dans le
  secteur de l'acier réduit de 2 MC le coût des cartes Projet ayant un badge
  Construction » ; « chaque savoir-faire dans le secteur du titane réduit de
  3 MC le coût des cartes […] badge Espace ». Compteurs permanents de réduction,
  acquis par certaines cartes vertes — exactement le modèle du Java
  (`steelIncome`/`titaniumIncome` dans `DiscountService`).
- **Conséquence moteur** : modéliser deux compteurs par joueur (savoir-faire
  acier / titane) appliqués dans `card_discount`. L'encodage actuel d'Asteroid
  Mining (réduction fixe de 6 MC Espace) est équivalent tant qu'aucune carte ne
  multiplie le savoir-faire (Advanced Alloys, Phobolog) ; à migrer vers les
  compteurs quand ces cartes entreront dans un lot. Débloque Aquifer Pumping,
  Solarpunk, Advanced Alloys, Phobolog.
- Vocabulaire français officiel : badge « Construction » (= BUILDING).

## Cartes au nom dupliqué (« Buffed ») — TRANCHÉ (24-07, contrat moteur-cartes-2)

- Greenhouses et Community Gardens ont chacune un jumeau « Buffed » portant le
  MÊME nom dans `cards.json` (`in_deck_v1=false`). Résolution par nom ambiguë →
  cartes exclues du moteur tant que la résolution se fait par nom. Source :
  audit moteur-cartes-2, `workspaces/moteur-cartes-2/outputs/journal.md`.

## Titanium Mine : tag imprimé ≠ effet Java — TRANCHÉ (24-07, règle « le texte gagne »)

- Le Java donne une réduction sur les cartes Espace ; la carte imprimée porte le
  badge Bâtiment. Encodée selon le texte imprimé. Conflit déclaré dans
  `workspaces/moteur-cartes-2/outputs/lot2.md`.

## Champ `description` de cards.json = paraphrase possible — TRANCHÉ (24-07)

- Démontré par Asteroid Mining (description « −6 MC Espace » vs Java « +2
  titane »). Conséquence : toute carte douteuse s'arbitre sur le scan de la
  carte physique (`data/scans/`), pas sur la description.

## Phase Actions : toutes les cartes ou seulement les bleues ? — SANS OBJET (28-07)

- Contradiction interne du livret : p. 14 (règle détaillée) dit « chacune de ses
  cartes en jeu » ; p. 20 (aperçu) dit « chacune de ses cartes **bleues** en
  jeu ».
- **Mesuré le 28-07 sur les 242 entrées de `textes-cartes.json`** (base +
  Découverte + corporations + cartes de phase) : **38 cartes portent
  « Action: », et les 38 sont BLEUES.** Aucune carte verte, rouge ou
  corporation n'en porte. La seule autre occurrence est la carte de phase
  « Action » elle-même, qui n'est pas une carte projet.
- **Les deux lectures donnent donc rigoureusement le même jeu.** La contradiction
  est réelle dans le texte, sans objet dans la pratique. Ne consommer aucun temps
  d'Alexis dessus.
- Le moteur applique la lecture restrictive : `flow::phase_action` (`flow.rs:2282`)
  filtre sur `color == Color::Blue`. [VÉRIFIÉ 28-07]
- **Amélioration à faire un jour, pas urgente** : filtrer sur « la carte a une
  action » plutôt que sur sa couleur ferait coïncider les deux lectures
  automatiquement et supprimerait le piège si une carte non bleue à action
  apparaissait un jour.

## Tuiles Objectifs et Récompenses : listes incomplètes dans le livret — EN ATTENTE

- Le livret Découverte ne détaille que 3 Objectifs (Diversificateur 9 badges
  différents, Magnat 8 cartes vertes, Terraformeur NT 15) et 3 Récompenses
  (Industriel acier+titane, Générateur production de chaleur, Chercheur badges
  Science) sur respectivement 11 et 7 tuiles. Le reste devra venir des scans ou
  de photos des tuiles physiques.

## Améliorations de phases : 2 options par phase, non toutes détaillées — TRANCHÉ (19-08, transcription des scans)

- Le livret montre des exemples (Développement amélioré : −6 MC au lieu de −3 ;
  Recherche améliorée : +2 piochées +1 gardée) mais ne liste pas les 10 cartes
  Phase améliorées.
- **Le manque est comblé** : les dix cartes sont transcrites dans
  `data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`. C'est cette
  transcription qui fait foi pour le moteur tant qu'aucun livret ne les détaille,
  et c'est elle que citent les tests du lot « les règles des cartes ».

## Améliorer une phase déjà améliorée : peut-on garder la sienne ? — TRANCHÉ (19-08, arbitrage moteur)

- **Le texte** (`livret-decouverte.md:66`) : « Lors de la résolution d'un effet
  "Améliorez une carte Phase", vous **pouvez** choisir d'améliorer en une
  amélioration **différente** une carte Phase que vous avez déjà améliorée. »
- **La question.** Trois cartes IMPOSENT la phase à améliorer. Si cette phase est
  déjà améliorée, il ne reste qu'une variante différente : le moteur l'appliquait
  sans rien demander, et le joueur qui avait bâti sa manche sur sa variante la
  perdait de force (audit D8). Le livret dit « vous pouvez », donc il ne peut pas
  s'agir d'une obligation — mais il n'écrit nulle part qu'on peut rechoisir la
  sienne.
- **Verdict** : la variante DÉJÀ EN PLACE reste candidate, et rechoisir la sienne
  vaut « je ne change rien ». Raison : le moteur n'a aucun « je renonce » pour une
  amélioration imposée ; sans cette candidate, la seule autre issue serait de
  sauter l'effet de la carte en silence, ce qui est pire. La même règle vaut pour
  la phase libre — il n'y a pas deux façons d'améliorer.
- **Ce que ce verdict DÉBORDE** : le contrat du lot ne visait que la phase
  imposée. L'appliquer aussi à la phase libre fait passer la liste de 9 à 10
  candidates dans ce cas. Divergence déclarée dans
  `workspaces/les-regles-des-cartes/outputs/result.md`.
- **Ce qu'il faudrait pour trancher mieux** : le texte des trois cartes à phase
  imposée, ou une réponse d'Alexis sur « rechoisir sa propre amélioration ».

## Phase III améliorée B : « deux de vos effets », combien de répétitions et sur quoi ? — TRANCHÉ (19-08, transcription + Alexis)

- **Le texte** (`phases-ameliorees.json`, carte III-B) : « Vous pouvez activer
  **deux de vos effets** "Action :" une fois de plus. » À comparer à III-A :
  « Vous pouvez activer **un de vos effets** "Action :" une fois de plus. »
- **La question.** Le moteur ouvrait un budget de deux répétitions sans jamais
  retirer une carte de la liste : la même carte pouvait encaisser les deux, et se
  retrouvait activée **trois** fois dans la phase (une fois normalement, deux fois
  en rappel). Fallait-il lire « deux répétitions, à répartir librement » ou
  « deux effets distincts, une répétition chacun » ?
- **Verdict** : deux effets **DISTINCTS**, une répétition chacun. Le carton
  compte des *effets*, pas des *répétitions* — « deux de vos effets », comme
  III-A dit « un de vos effets ». Une carte qui a consommé sa répétition ne
  revient donc jamais dans la liste des activables, et aucune carte ne peut être
  activée trois fois dans une phase.
- **Confirmé par Alexis le 18-08** : le comportement d'avant est bien un défaut
  (audit D7). Corrigé dans `flow::phase_action`, tenu par la sentinelle
  `state::GameState::cartes_activees_trois_fois`, qui doit rester nulle.
- **Ce que le verdict ne tranche pas** : le budget est-il perdu s'il n'y a qu'une
  seule carte activable ? Le moteur répond oui — une répétition non dépensée ne
  survit pas à la phase, comme toute permission « lors de cette phase ». Aucune
  source ne le contredit, mais aucune ne l'écrit non plus.

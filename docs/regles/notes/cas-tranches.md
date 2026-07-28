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

## Améliorations de phases : 2 options par phase, non toutes détaillées — EN ATTENTE

- Le livret montre des exemples (Développement amélioré : −6 MC au lieu de −3 ;
  Recherche améliorée : +2 piochées +1 gardée) mais ne liste pas les 10 cartes
  Phase améliorées. À compléter par les scans des cartes.

# divergences.md — ce que la paraphrase de `cards.json` dit de faux

Ce document compare, carte par carte, **le texte réellement imprimé** (lu sur les
images, `outputs/textes-cartes.json`) au champ `description` de
`inputs/cards.json` — une paraphrase écrite par le développeur d'un moteur Java
tiers, dont ce chantier existe pour établir la fiabilité.

Périmètre : les **220 cartes de la pioche de base effectivement lues** (sur 222
attendues ; voir §0). Classement par gravité : ce qui change une règle d'abord,
la ponctuation en dernier.

| Section | Ce que c'est | Entrées |
|---|---|---|
| §0 | cartes absentes des planches | 2 |
| §A | écarts structurels, à lire avant tout | 6 motifs |
| §B | noms fautifs dans `cards.json` | 18 |
| §G1 | écarts qui **changent une règle** | 62 |
| §G2 | écarts qui changent une **précision** (cible d'un décompte, prérequis, valeur) | 20 |
| §G3 | formulation, orthographe, ponctuation | 163 |

---

## §0 — Cartes attendues absentes des planches

Ces deux cartes figurent dans `inputs/cards.json` (`in_deck_v1: true`,
`box: "base"`) mais **sur aucune planche** de l'adaptation TTS. Elles ne sont pas
illisibles : les quatre planches de cartes projet contiennent exactement 208
cartes (52 × 4), toutes lues, et `save.bin` — qui liste nommément le contenu de
chaque paquet — ne les mentionne pas.

- **Microbiology Patents** — absente. `cards.json` la décrit verte, coût 6, badge
  Microbe. **Conséquence :** aucune vérification n'est possible sur cette carte ;
  le projet doit la considérer comme non validée.
- **Project Inspection** — absente. `cards.json` la décrit rouge, événement,
  coût 0. **Conséquence :** idem. Sa paraphrase (« Next 3rd phase you may use
  additional action twice. ») est de surcroît la plus suspecte du fichier, car
  elle ne suit aucune des formulations imprimées du jeu.

L'argument décisif est numérique : les numéros imprimés lus sur les 220 cartes
couvrent **1 à 220 sans un seul trou**. La boîte comporte donc 220 cartes
numérotées, et non 222. **Conséquence pour le projet :** ces deux entrées de
`cards.json` sont probablement de trop dans la pioche v1 — c'est une décision de
conception à prendre, détaillée dans `outputs/blocked.md`.

---

## §A — Les six motifs systémiques

Ce ne sont pas des cas isolés : ce sont des défauts de la paraphrase qui se
répètent sur des dizaines de cartes. Les corriger un par un serait une erreur ;
il faut corriger la source.

1. **Le mot-clé `Action:` / `Effect:` est presque toujours absent** — **35 cartes au plus
   mesurées** (mesure du CTO à l'audit : 25 avec le mot-clé en début de texte, 35 sans ancrage ;
   le chiffre 47 initialement annoncé n'est pas reproductible). Le fond reste massif :
   64 cartes de la pioche portent le mot-clé imprimé contre 29 dans la paraphrase.
   C'est l'essentiel de §G1. La paraphrase transcrit le *contenu* de l'effet sans son
   *régime*. **Conséquence :** un moteur qui encode depuis `cards.json` transforme
   une capacité permanente ou répétable en gain unique à la pose. Cas de
   référence, connu avant le chantier : *Hydro-Electric Energy* (n° 34) — la
   paraphrase dit « Spend 1 MC to get 2 heat », la carte imprime « **Action:**
   Spend 1 MC to gain 2 heat. » et toute une seconde phrase (« ⚡ If you chose the
   action phase this round, gain 1 additional heat. ») que la paraphrase omet.
   Sans le mot `Action:`, la capacité serait jouée une seule fois au lieu d'être
   activable à chaque phase III.
2. **« Build a forest » au lieu de « Gain a forest VP »** — 5 cartes mesurées :
   Biothermal Power, Mangrove, Protected Valley, Small Animals, Solarpunk. *Ares Expedition* n'a ni
   plateau ni tuiles. **Conséquence :** un moteur qui « construit » une forêt
   hérite de la logique de *Terraforming Mars* classique — placement, adjacence,
   production de plantes — qui n'existe pas dans ce jeu.
3. **« including this » omis.** Un seul cas strict dans la pioche, mais il est
   coûteux. Cas de référence connu : *Windmills* (n° 206) —
   la paraphrase dit « this produces 1 heat per Energy tag you have », la carte
   dit « …per [energy] you have, **including this**. » **Conséquence :** la
   production est inférieure de 1 à chaque phase de production, silencieusement,
   toute la partie. Même défaut sur *Medical Lab* et plusieurs cartes à décompte.
4. **Les corporations sont paraphrasées comme des cartes différentes.** *Mining
   Guild*, *PhoboLog*, *Saturn Systems*, *Interplanetary Cinematics* : la
   paraphrase invente des revenus (« 1 Steel income », « 1 Titanium income »)
   qui ne sont imprimés nulle part, et omet des réductions de coût permanentes
   qui, elles, sont imprimées. **Conséquence :** quatre corporations sur douze
   sont fausses de bout en bout.
5. **Homoglyphes cyrilliques dans `cards.json`.** **16 entrées de la pioche de
   base** écrivent « MC » avec les lettres cyrilliques « МС » : Developed
   Infrastructure, Energy Subsidies, Industrial Center, Io Mining Industries,
   Natural Preserve, New Portfolios, Power Grid, Protected Valley, Rad Suits,
   Sponsors, Tall Station, Trading Post, Tropical Resort, Tundra Farming,
   Underground City, Venture Capitalism.
   **Conséquence :** toute recherche, comparaison ou parsing sur la chaîne « MC »
   les rate silencieusement — un bug d'outillage, pas de règle, mais invisible.
6. **Casse et ordre des badges.** `cards.json` écrit les badges en capitales et
   dans un ordre qui n'est pas celui de la carte. Ce n'est pas une faute de
   contenu, mais **toute comparaison de listes par égalité stricte échouera**
   sans normalisation préalable.

### Sur le coût des corporations

Une corporation n'a **pas** de pastille de coût. Le nombre imprimé en haut à
droite est son **MC de départ** (confirmé par la phrase « You start with N MC. »
imprimée dans son texte). C'est cette valeur que porte le champ `cost` des douze
corporations livrées, faute d'un champ mieux nommé. `cards.json` la range dans
`price`, au même endroit que le coût d'une carte projet. **Conséquence :** un
moteur qui traite `price` uniformément ferait *payer* 48 MC pour jouer CrediCor
au lieu de lui en *donner* 48. Ce n'est pas un écart de lecture, c'est un piège
de modélisation à signaler.

---

## §B — Noms fautifs dans `cards.json`

Le nom imprimé fait foi. Pour rester indexable par le projet, chaque entrée
livrée porte `name` (la clé actuelle du projet, inchangée) **et** `printed_name`
(le nom réellement imprimé). Vérifié une par une sur l'image :

| `cards.json` (fautif) | imprimé sur la carte |
|---|---|
| Advanced Screening Tech | Advanced Screening Technology |
| Ai Central | AI Central |
| Bribed Comittee | Bribed Committee |
| Business Contracts | Business **Contacts** |
| Credicor | CrediCor |
| GHG Production Bacteria | GHG **Producing** Bacteria |
| Helion Corporation | Helion |
| Lake Mariners | Lake **Marineris** |
| Matter Manufactoring | Matter Manufacturing |
| Nitrite Reducting Bacteria | Nitrite Reducing Bacteria |
| Nitropholic Moss | Nitrophilic Moss |
| Phobolog | PhoboLog |
| Sattellite Farms | Satellite Farms |
| Space Heater | Space Heaters |
| Tall Station | **Toll** Station |
| Teractor Corporation | Teractor |
| Thorgate Corporation | Thorgate |
| Unmi | United Nations Mars Initiative |

Trois d'entre eux ne sont pas de simples coquilles mais **changent le sens** :
*Business Contacts* (et non « Contracts »), *Toll Station* (et non « Tall »),
*Lake Marineris* (référence à Valles Marineris, et non « Lake Mariners »).

**Faux écart écarté après vérification.** *Artificial Lake* et *Artificial
Jungle* avaient été soupçonnés d'être une seule carte mal orthographiée. Ce sont
**deux cartes distinctes**, toutes deux présentes et toutes deux lues :
*Artificial Lake*, rouge, n° 66, coût 13, badge Événement, 1 PV, « Requires
yellow temperature or warmer. » ; *Artificial Jungle*, bleue, coût 5, « Action:
Spend 1 plant to draw a card. » Aucune correction à faire de ce côté.

---
## §G1 — écarts qui changent une règle
- **Adaptation Technology** (n° 1) — paraphrase : « When playing a card with requirements, you may consider… » ; carte : « **Effect:** When playing a card with requirements, you may consider… ». **Conséquence :** le mot-clé `Effect:` (capacité permanente d'une carte bleue) disparaît ; le moteur peut appliquer la modification une seule fois à la pose au lieu de la rendre permanente.
- **Advanced Alloys** (n° 2) — paraphrase : « Each titanium you have is worth 1 MC extra. Each steel you have is worth 1 MC extra. » ; carte : « Effect: Each titanium you have reduces the cost of [space] cards an additional 1 MC. Each steel you have reduces the cost of [building] cards an additional 1 MC. » Le mot `Effect:` est absent de la paraphrase. **Conséquence :** le moteur peut traiter la capacité comme un gain unique à la pose au lieu d'un effet permanent.
- **Advanced Alloys** (n° 2) — paraphrase : « is worth 1 MC extra » (sans cible) ; carte : « reduces the cost of [space] cards » / « reduces the cost of [building] cards ». **Conséquence :** la réduction s'appliquerait à n'importe quelle carte au lieu d'être restreinte aux cartes [space] pour le titane et [building] pour l'acier.
- **Anaerobic Microorganisms** (n° 5) — paraphrase : « When you play an Animal, Microbe, or Plant, including this, add a microbe to this card. When you play a card, you may remove 2 microbes… » ; carte : « **Effect:** When you play a card, you may remove 2 microbes from this card to pay 10 MC less for that card. **Effect:** When you play an [animal], [microbe], or [plant], including this, add a microbe to this card. ». **Conséquence :** les deux capacités sont des effets permanents déclenchés ; sans le mot-clé `Effect:` le moteur peut les exécuter une seule fois à la pose au lieu de les armer pour toute la partie.
- **Anti-Gravity Technology** (n° 6) — paraphrase : « Requires 5 SCT. When you play a card, gain 2 heat and 2 plants. » ; carte : « Effect: When you play a card, gain 2 heat and 2 plants. » Le mot `Effect:` est absent. **Conséquence :** effet permanent transformé en gain unique à la pose.
- **Arctic Algae** (n° 8) — paraphrase : « When you flip an ocean tile, gain 4 plants. » ; carte : « **Effect:** When you flip an ocean tile, gain 4 plants. ». **Conséquence :** gain unique à la pose au lieu d'un déclencheur permanent sur chaque océan retourné.
- **Assembly Lines** (n° 10) — paraphrase : « When you use an "Action:" effect… » ; carte : « **Effect:** When you use an "Action:" effect on one of your cards, gain 1 MC. ». **Conséquence :** le revenu par action ne se déclenche jamais, ou une seule fois.
- **Birds** (n° 12) — paraphrase : « Add an animal to this card. » ; carte : « **Action:** Add an animal to this card. ». **Conséquence :** capacité répétable transformée en ajout unique à la pose ; les PV variables (1 VP par animal) restent bloqués à 1.
- **Composting Factory** (n° 17) — paraphrase : « Cards you discard for MC are worth an additional MC. » ; carte : « **Effect:** Cards you discard for MC are worth an additional 1 MC. ». **Conséquence :** sans le marqueur `Effect:`, le moteur traite un effet permanent (phase I-V, encart magenta) comme un bonus ponctuel à la pose ; les défausses ultérieures ne sont plus majorées.
- **Conserved Biome** (n° 18) — paraphrase : « Add a microbe to ANOTHER* card or add an animal to ANOTHER* card. » ; carte : « **Action:** Add a microbe to ANOTHER* card or add an animal to ANOTHER* card. ». **Conséquence :** une action répétable en phase III devient un effet unique à la pose ; le joueur perd toute accumulation de microbes/animaux.
- **CrediCor** (n° 209) — paraphrase : « 48 Mc. When you play a card with a printed cost of 20 or more, get a 4 MC discount. » ; carte : « You start with 48 MC. **EFFECT:** When you play a card with a printed cost of 20 MC or more, you pay 4 MC less for it. ». **Conséquence :** `EFFECT:` absent — la réduction peut être appliquée une seule fois au lieu de rester active toute la partie.
- **Decomposers** (n° 19) — paraphrase : « When you play an Animal, Microbe, or Plant, including this… » ; carte : « **Effect:** When you play an [animal], [microbe], or [plant], including this… ». **Conséquence :** l'effet déclenché permanent est traité comme un effet ponctuel à la pose ; plus aucune pioche sur les cartes jouées ensuite.
- **Earth Catapult** (n° 23) — paraphrase : « When you play a card, you pay 2 MC less for it. » ; carte : « **Effect:** When you play a card, you pay 2 MC less for it. ». **Conséquence :** la réduction permanente de 2 MC n'est pas armée.
- **Ecoline** (n° ★210) — paraphrase : « When you exchange plants for forest, pay 1 plant less. » ; carte : « EFFECT: When you spend plants to gain a forest VP token and raise oxygen, you spend one less plant. » Le mot `EFFECT:` est absent. **Conséquence :** réduction permanente traitée comme un bonus ponctuel.
- **Ecological Zone** (n° 24) — paraphrase : « When you play a Animal or Plant, including these, add an animal to this card. » ; carte : « **Effect:** When you play a [animal] or [plant], including these, add an animal to this card. ». **Conséquence :** `Effect:` absent — le déclencheur permanent devient un ajout unique d'animal à la pose, et la carte ne marque plus jamais de PV supplémentaire.
- **Energy Subsidies** (n° 25) — paraphrase : « When you play an Energy tag, you pay 4 МС less for it and you draw a card. » ; carte : « Effect: When you play an [energy], you pay 4 MC less for it and you draw a card. » Le mot `Effect:` est absent. **Conséquence :** réduction permanente traitée comme un effet à la pose.
- **Extended Resources** (n° 26) — paraphrase : « When you keep cards during the research phase, keep one additional card. » ; carte : « **Effect:** When you keep cards during the research phase, keep one additional card. ». **Conséquence :** le bonus permanent de phase V (recherche) devient un événement unique ; la limite de conservation n'est jamais augmentée.
- **Extreme-Cold Fungus** (n° 27) — paraphrase : « Gain 1 plant or add a microbe to ANOTHER* card. » ; carte : « **Action:** Gain 1 plant or add a microbe to ANOTHER* card. ». **Conséquence :** action répétable chaque tour réduite à un gain unique à la pose.
- **Fish** (n° 30) — paraphrase : « When you flip an ocean tile, add 1 animal to this card. » ; carte : « **Effect:** When you flip an ocean tile, add 1 animal to this card. ». **Conséquence :** l'accumulation d'animaux (et donc les PV) ne se déclenche pas.
- ~~**Ganymede Shipyard** (n° 138) — référence : `vp = 0` ; carte : pastille de PV = **2**~~
  → **ÉCART RETIRÉ À L'AUDIT (CTO, 26-07). C'était une erreur de lecture, pas un écart.**
  L'encart gris à deux étoiles jaunes est un **savoir-faire de 2 titane**, pas des points
  de victoire. Preuve arithmétique sur tout le corpus : 1 étoile grise → « pay 3 MC less
  for [space] » (*Titanium Mine*, *Space Station*, *Vesta Shipyard*, *Asteroid Mining
  Consortium*) ; 2 étoiles → « pay 6 MC less » (*Asteroid Mining*, *Ganymede Shipyard*,
  *Io Mining Industries*). 3 MC par titane est exactement la règle du livret p. 18.
  Preuve visuelle : sur *Asteroid Mining* (n° 110) les deux marquages **coexistent et sont
  distincts** — encart gris à 2 étoiles **et** pastille brune ronde « 2 » séparée ; sur
  *Ganymede Shipyard* il n'y a aucune pastille brune. `vp_printed` corrigé à 0.
  **Conséquence pour le projet : après cette correction, il n'existe AUCUN écart de points
  de victoire entre le texte imprimé et `cards.json` sur les 220 cartes.** Sur ce champ
  précis, la référence du projet est fiable.
- **Helion** (n° ★211) — paraphrase : « You may use Heat as Mc, but not vise versa. » ; carte : « **EFFECT:** You may use heat as MC. You may not use MC as heat. ». **Conséquence :** capacité corporation permanente non armée.
- **Herbivores** (n° 33) — paraphrase : « When you raise oxygen, flip an ocean tile, or raise temperature, add 1 animal to this card. » ; carte : « Effect: When you raise oxygen, flip an ocean tile, or raise the temperature, add 1 animal to this card. » Le mot `Effect:` est absent. **Conséquence :** le déclencheur permanent deviendrait un ajout unique d'animal à la pose.
- **Hydro-Electric Energy** (n° 34) — paraphrase : « Spend 1 MC to get 2 heat. *if you chose the action phase this round… » ; carte : « **Action:** Spend 1 MC to gain 2 heat. *If you chose the action phase this round, gain 1 additional heat. ». **Conséquence :** `Action:` absent — une action répétable à chaque tour (phase III) devient un gain unique à la pose.
- **Interns** (n° 36) — paraphrase : « When you draw cards during the research phase, draw two additional cards. » ; carte : « **Effect:** When you draw cards… ». **Conséquence :** `Effect:` absent — le bonus de pioche permanent (phase V) est appliqué une seule fois.
- **Interplanetary Cinematics** (n° ★212) — paraphrase : (rien) ; carte : « When you play a [building], you pay 2 MC less for it. ». **Conséquence :** la réduction de 2 MC sur toutes les cartes [building] est totalement absente ; toutes les cartes bâtiment sont surfacturées de 2 MC pendant la partie.
- **Interplanetary Cinematics** (n° ★212) — paraphrase : « 46 Mc. **1 steel production.** When you play an Event… » ; carte : « You start with 46 MC. When you play a [building], you pay 2 MC less for it. EFFECT: When you play an [event], you pay 2 MC less for it. ». **Conséquence :** la paraphrase invente une production d'acier inexistante sur la carte ; le moteur accorde une ressource de départ fantôme.
- **Interplanetary Cinematics** (n° ★212) — paraphrase : « When you play an Event, you pay 2 MC less for it. » ; carte : « **EFFECT:** When you play an [event], you pay 2 MC less for it. ». **Conséquence :** le mot-clé d'effet permanent manque ; la réduction event peut être appliquée une seule fois.
- **Interplanetary Conference** (n° 37) — paraphrase : « When you play an Earth or Jupiter tag, excluding this… » ; carte : « **Effect:** When you play an [earth] or [jupiter], excluding this, you pay 3 MC less and draw a card. ». **Conséquence :** la réduction + pioche permanente n'est pas armée.
- **Interplanetary Relations** (n° 35) — paraphrase : « When you draw cards during the research phase, draw one additional card and keep one additional card. » ; carte : « Effect: When you draw cards during the research phase… ». Le mot `Effect:` est absent. **Conséquence :** bonus de phase V permanent traité comme unique.
- **Inventrix** (n° 213) — paraphrase : « When playing a card with requirements, you may consider… » ; carte : « **EFFECT:** When playing a card with requirements, you may consider the oxygen or temperature one color higher or lower. ». **Conséquence :** l'assouplissement permanent des prérequis n'est pas armé.
- **Large Convoy** (n° 87) — paraphrase : « add 3 animals to **ANY** card. » ; carte : « add 3 animals to **ANOTHER** card. ». **Conséquence :** le moteur autorise une cible interdite (la carte elle-même), condition de placement fausse.
- **Livestock** (n° 39) — paraphrase : « When you raise the temperature, add 1 animal to this card. » ; carte : « **Effect:** When you raise the temperature, add 1 animal to this card. ». **Conséquence :** le déclencheur permanent devient un ajout unique d'animal à la pose ; les PV variables de la carte n'augmentent plus jamais.
- **Mars University** (n° 40) — paraphrase : « When you play a Science tag, including this, you may discard a card… » ; carte : « **Effect:** When you play a [science], including this, you may discard a card… ». **Conséquence :** le déclencheur de défausse/pioche n'est pas armé.
- **Media Group** (n° 42) — paraphrase : « When you play an Event, you pay 5 MC less for it. » ; carte : « **Effect:** When you play an [event], you pay 5 MC less for it. ». **Conséquence :** la réduction permanente sur les events peut être traitée comme un effet ponctuel à la pose.
- **Mining Guild** (n° 214) — paraphrase : aucune mention de réduction ; carte : « When you play a [building], you pay 2 MC less for it. » **Conséquence :** la réduction permanente de 2 MC sur les cartes [building] serait purement absente du moteur.
- **Mining Guild** (n° 214) — paraphrase : « 1 Steel income. » ; carte : rien de tel n'est imprimé (« You start with 27 MC. When you play a [building], you pay 2 MC less for it. »). **Conséquence :** le moteur donnerait une production d'acier de départ qui n'existe pas sur la carte.
- **Mining Guild** (n° 214) — paraphrase : « Whenever you play a card that increases Steel income, gain 1 TR. » ; carte : « EFFECT: Each time you play steel production, excluding this, gain 1 TR. » Le mot `EFFECT:` et la clause « excluding this » sont absents. **Conséquence :** effet traité comme unique, et la corporation se déclencherait sur elle-même (1 TR de trop).
- **Olympus Conference** (n° 44) — paraphrase : « When you play a Science tag, including this, draw a card. » ; carte : « Effect: When you play a [science], including this, draw a card. » Le mot `Effect:` est absent. **Conséquence :** pioche permanente réduite à une pioche unique à la pose.
- **Optimal Aerobraking** (n° 45) — paraphrase : « When you play an Event tag, you gain 2 heat and 2 plants. » ; carte : « **Effect:** When you play an [event], you gain 2 heat and 2 plants. ». **Conséquence :** `Effect:` absent — le déclencheur permanent sur chaque événement devient un gain unique.
- **PhoboLog** (n° 215) — paraphrase : rien sur une réduction fixe ; carte : « When you play a [space], you pay 3 MC less for it. ». **Conséquence :** la réduction de base de 3 MC sur toutes les cartes espace est purement absente du moteur.
- **PhoboLog** (n° 215) — paraphrase : « 1 Titanium income. » ; carte : aucun revenu de titane imprimé (texte lu : « You start with 20 MC. »). **Conséquence :** le moteur accorde une production de titane que la carte ne donne pas.
- **PhoboLog** (n° 215) — paraphrase : « Each Titanium you have is worth 1 MC extra. » ; carte : « **EFFECT:** Each titanium you have reduces the cost of [space] cards an additional 1 MC. ». **Conséquence :** le moteur donnerait un bonus de valeur générique au titane au lieu d'une réduction ciblée sur les cartes [space] — deux économies différentes.
- **Physics Complex** (n° 46) — paraphrase : « When you raise the temperature, add 1 science resource to this card. » ; carte : « **Effect:** When you raise the temperature, add 1 science resource to this card. ». **Conséquence :** `Effect:` absent — la carte n'accumule plus de ressources science, donc plus aucun PV variable.
- **Protected Valley** (n° 177) — paraphrase : « Build a forest and raise oxygen 1 step. » ; carte : « Gain a forest VP and raise oxygen 1 step. ». **Conséquence :** le moteur poserait/compterait une tuile forêt au lieu d'accorder un point de victoire forêt.
- **Recycled Detritus** (n° 48) — paraphrase : « When you play an Event, draw 2 cards. » ; carte : « **Effect:** When you play an [event], draw two cards. ». **Conséquence :** l'effet déclenché permanent devient une pioche unique à la pose.
- **Research Outpost** (n° 51) — paraphrase : « When you play a card, you pay 1 MC less for it. » ; carte : « **Effect:** When you play a card, you pay 1 MC less for it. ». **Conséquence :** réduction permanente non armée.
- **Restructured Resources** (n° 52) — paraphrase : « When you play a card, you may spend 1 plant to reduce that card's cost by 5 MC. » ; carte : « Effect: When you play a card… ». Le mot `Effect:` est absent. **Conséquence :** réduction permanente traitée comme unique.
- **Saturn Systems** (n° 216) — paraphrase : rien ; carte : « When you play a [space], you pay 3 MC less for it. ». **Conséquence :** la réduction de 3 MC sur les cartes espace est absente du moteur.
- **Saturn Systems** (n° 216) — paraphrase : « 1 Titanium income. » ; carte : aucun revenu de titane imprimé (texte lu : « You start with 24 MC. »). **Conséquence :** production de titane accordée à tort.
- **Saturn Systems** (n° 216) — paraphrase : « Whenever you play a Jupiter tag, excluding this, gain 1 TR. » ; carte : « **EFFECT:** Each time you play a [jupiter], excluding this, gain 1 TR. ». **Conséquence :** déclencheur permanent non armé.
- **Small Animals** (n° 53) — paraphrase : déclencheur « When you **build** a forest » ; carte : « When you **gain a forest VP** ». **Conséquence :** déclencheur différent — le moteur ne compte que les forêts construites et rate toutes les autres sources de PV forêt, donc sous-compte les animaux et les PV de la carte.
- **Small Animals** (n° 53) — paraphrase : « When you build a forest, add 1 animal to this card. » ; carte : « **Effect:** When you gain a forest VP, add 1 animal to this card. ». **Conséquence :** `Effect:` absent — le déclencheur permanent devient un ajout unique.
- **Standard Technology** (n° 55) — paraphrase : « You pay 4 MC less for standard actions that cost MC. » ; carte : « Effect: You pay 4 MC less for standard actions that cost MC. » Le mot `Effect:` est absent. **Conséquence :** rabais permanent sur les actions standard potentiellement appliqué une seule fois.
- **Symbiotic Fungus** (n° 57) — paraphrase : « Add a microbe to ANOTHER* card. » ; carte : « **Action:** Add a microbe to ANOTHER* card. ». **Conséquence :** `Action:` absent — une action répétable (phase III) devient un ajout unique à la pose ; toutes les cartes à microbes sont sous-alimentées.
- **Teractor Corporation** (n° ★217) — paraphrase : « 51 Mc. When you play an Earth tag get 3MC discount. » ; carte : « You start with 51 MC. **EFFECT:** When you play an [earth], you pay 3 MC less for it. ». **Conséquence :** le mot-clé d'effet permanent manque ; la réduction earth de la corporation peut n'être appliquée qu'une fois.
- **Tharsis Republic** (n° 218) — paraphrase : « 40 Mc. When you draw cards during the research phase… » ; carte : « You start with 40 MC. EFFECT: When you draw cards during the research phase… ». Le mot `EFFECT:` est absent. **Conséquence :** bonus de recherche permanent traité comme unique.
- **Thorgate Corporation** (n° 219) — paraphrase : « 45 Mc. 1 Heat income. When you play Energy tag get 3MC discount. » ; carte : « You start with 1 heat production and 45 MC. **EFFECT:** When you play a [energy], you pay 3 MC less for it. ». **Conséquence :** le mot-clé d'effet permanent manque ; la réduction energy de la corporation peut n'être appliquée qu'une fois.
- **United Nations Mars Initiative** (n° 220) — paraphrase : « 35 Mc. When you first raise RT during the phase, you may spend 6 MC to get 1 extra RT. » ; carte : « You start with 35 MC. **EFFECT:** The first time your TR is raised each phase, you may pay 6 MC to raise your TR 1 step. ». **Conséquence :** `EFFECT:` absent — la capacité de corporation, active à chaque phase, est traitée comme un effet unique.
- **United Planetary Alliance** (n° 60) — paraphrase : « When you draw cards during the research phase, draw one additional card and keep one additional card. » ; carte : « Effect: When you draw cards during the research phase… ». Le mot `Effect:` est absent. **Conséquence :** bonus de phase V permanent traité comme unique.
- **Viral Enhancers** (n° 61) — paraphrase : « When you play a Plant, Microbe, or Animal tags, including these, gain 1 plant… » ; carte : « Effect: When you play a [animal], [microbe], or [plant], including these… ». Le mot `Effect:` est absent. **Conséquence :** déclencheur permanent réduit à un gain unique à la pose.
- **Water Import from Europa** (n° 63) — paraphrase : « Spend 12 MC to flip an ocean tile. » ; carte : « **Action:** Spend 12 MC to flip an ocean tile. ». **Conséquence :** l'action répétable de phase III devient un effet unique à la pose ; la carte perd sa fonction principale.
- **Windmills** (n° 206) — paraphrase : « this produces 1 heat per Energy tag you have. » ; carte : « this produces 1 heat per [energy] you have, including this. » La clause « including this » est omise. **Conséquence :** production de chaleur inférieure de 1 à chaque phase de production.
---

## §G2 — écarts qui changent une précision
- **Asset Liquidation** (n° 11) — paraphrase : « You may play an additional blue or red card this phase. » ; carte : « [effect] You may play an additional blue or red card this phase. » (icône d'effet immédiat). **Conséquence :** sur une carte bleue, l'absence du marqueur laisse le moteur en faire une capacité permanente au lieu d'un bonus unique à la pose.
- **Biothermal Power** (n° 118) — paraphrase : « **Build a forest** and raise oxygen 1 step. » ; carte : « Gain a **forest VP** and raise oxygen 1 step. ». **Conséquence :** le moteur peut simuler la pose d'une tuile forêt (avec adjacence/production de plantes héritée de TM classique) au lieu d'accorder un simple PV forêt, la seule chose que fait Ares Expedition.
- **Composting Factory** (n° 17) — paraphrase : « worth an additional MC » ; carte : « worth an additional **1** MC ». **Conséquence :** valeur non chiffrée dans la source ; un parseur peut lire 0 ou un montant indéterminé au lieu de +1 MC par carte défaussée.
- **Conserved Biome** (n° 18) — paraphrase : « 1 VP per 2 **forests** you have. » ; carte : « *=1 VP per 2 **forest VPs** you have. ». **Conséquence :** le décompte porte sur les PV forêt possédés, pas sur un nombre de tuiles/forêts ; toute divergence entre les deux compteurs fausse le score final.
- **Ecoline** (n° ★210) — paraphrase : « When you exchange plants for forest » ; carte : « When you spend plants to gain a forest VP token and raise oxygen ». **Conséquence :** le moteur pourrait appliquer la réduction hors de l'action standard forêt (qui inclut la hausse d'oxygène), ou ne pas la reconnaître.
- **Fusion Power** (n° 137) — paraphrase : « Requires 2 Energy tags. » ; carte : « Requires 2 [energy]. » (icône seule, badge ou ressource non désambiguïsé à la lecture). **Conséquence :** si l'icône désigne la ressource énergie et non le badge, le prérequis est évalué sur la mauvaise grandeur.
- **Insects** (n° 152) — paraphrase : « produces 1 plant per Plant you have » ; carte : « produces 1 plant per [plant] you have » (badge plante, confirmé par l'encart de production). **Conséquence :** le moteur pourrait compter les ressources plantes détenues au lieu des badges [plant], donnant une production complètement fausse.
- **Mangrove** (n° 90) — paraphrase : « **Build a Forest** and raise oxygen 1 step. » ; carte : « Gain a **forest VP** and raise oxygen 1 step. ». **Conséquence :** même risque de tuile fantôme et de bonus de placement inexistant.
- **Medical Lab** (n° 160) — paraphrase : « per 2 Building you have » ; carte : « per 2 [building] you have ». **Conséquence :** cible du décompte nommée en toutes lettres — risque de compter des cartes bâtiment plutôt que des badges [building].
- **Nitrogen-Rich Asteroid** (n° 91) — paraphrase : « If you have 3 or more Plant, gain 4 additional plants. » ; carte : « If you have 3 or more [plant], gain 4 additional plants. » **Conséquence :** condition évaluée sur les ressources plantes au lieu des badges [plant] : bonus de 4 plantes accordé ou refusé à tort.
- **Plantation** (n° 94) — paraphrase : « **Build 2 forests** and raise oxygen 2 steps. » ; carte : « Gain **2 forest VPs** and raise oxygen 2 steps. ». **Conséquence :** deux tuiles simulées au lieu de 2 PV forêt.
- **Power Grid** (n° 174) — paraphrase : « per Energy you have » ; carte : « per [energy] you have ». **Conséquence :** même ambiguïté badge/ressource sur la base du décompte de production.
- **Solarpunk** (n° 54) — paraphrase : « Spend 15 MC to **build a Forest** and raise oxygen 1 step. » ; carte : « Spend 15 MC to gain a **forest VP** and raise oxygen 1 step. ». **Conséquence :** même risque de tuile fantôme sur une action répétable, donc erreur cumulée à chaque activation.
- **Terraforming Ganymede** (n° 100) — paraphrase : « per Jupiter tag you have » ; carte : « per [jupiter] you have ». **Conséquence :** cible du décompte de TR exprimée différemment ; à vérifier que le moteur compte bien les badges.
- **Titanium Mine** (n° 194) — paraphrase : « you pay 3 less for it » ; carte : « you pay 3 MC less for it ». **Conséquence :** unité de la réduction absente ; une implémentation pourrait l'appliquer en titane ou en pas de coût au lieu de MC.
- **United Nations Mars Initiative** (n° 220) — paraphrase : « When you **first raise** RT during the phase » (le joueur agit) ; carte : « The first time your TR **is raised** each phase » (voix passive). **Conséquence :** portée du déclencheur rétrécie — les hausses de TR non provoquées directement par le joueur ne déclenchent plus l'achat, la corporation devient moins rentable que la vraie.
- **Water Import from Europa** (n° 63) — paraphrase : « Reduce this by 1 MC per **titanium income** you have. » ; carte : « Reduce this by 1 MC per **titanium** you have. ». **Conséquence :** la réduction est calculée sur la production de titane au lieu du titane possédé — coût de l'action faux à presque chaque activation.
- **Wood Burning Stoves** (n° 64) — paraphrase : « *if you chose the action phase this round, spend 3 plants. » ; carte : « …spend 3 plants **instead**. ». **Conséquence :** sans « instead », le coût de 3 plantes peut être cumulé avec les 4 plantes de base au lieu de les remplacer.
- **Worms** (n° 207) — paraphrase : « per Microbe tag you have » ; carte : « per [microbe] you have ». **Conséquence :** idem — risque de compter des microbes-ressources au lieu des badges.
- **Zeppelins** (n° 208) — paraphrase : « produces 1 MC per Forest you have » ; carte : « produces 1 MC per forest VP you have ». **Conséquence :** décompte sur des tuiles/cartes forêt au lieu des jetons de PV forêt.
---

## §G3 — formulation, orthographe, ponctuation
- **Adaptation Technology** (n° 1) — paraphrase : « cannot be modified **futher** by other effects » ; carte : « cannot be modified **further** by other effects ». **Conséquence :** coquille de la référence, aucune règle changée.
- **Adapted Lichen** (n° 104) — paraphrase : « During the production phase this produces… » ; carte : « During the production phase, this produces… ». Virgule absente.
- **Advanced Ecosystems** (n° 65) — paraphrase : « Requires an Animal, Microbe, and Plant tags. » ; carte : « Requires an [animal], [microbe], and [plant]. ». **Conséquence :** icônes rendues en toutes lettres et « tags » ajouté au pluriel après « an » ; aucune règle changée. (Aucun texte d'effet imprimé sur la carte, conforme à la référence.)
- **Advanced Screening Tech** (n° 3) — clé du projet : « Advanced Screening **Tech** » ; nom imprimé : « Advanced Screening **Technology** ». **Conséquence :** la clé de référence ne correspond pas au nom imprimé (recherche/appariement par nom en échec).
- **Advanced Screening Tech** (n° 3) — paraphrase : « top **3** cards… a card with a **Science or Plant** » ; carte : « top **three** cards… a card with a **[science] or [plant]** ». **Conséquence :** rendu en toutes lettres des icônes de badge, sémantique identique.
- **Aerated Magma** (n° 105) — paraphrase : « During the production phase you draw… » ; carte : « During the production phase, you draw… ». Virgule absente.
- **AI Central** (n° 4) — paraphrase : « Requires 5 Science. […] Draw 2 cards. » ; carte : « Requires 5 [science]. […] Draw two cards. » **Conséquence :** aucune, rendu d'icône et de numéral.
- **AI Central** (n° 4) — référence : `name` = « Ai Central » ; carte : « AI Central ». **Conséquence :** clé du projet mal orthographiée.
- **Airborne Radiation** (n° 106) — paraphrase : « Requires red oxygen or higher. Raise oxygen 1 step. During the production phase this produces 2 heat. » ; carte : « [effect] Raise oxygen 1 step. During the production phase**,** this produces 2 heat. ». **Conséquence :** marqueur d'effet immédiat non rendu et virgule manquante ; rien de fonctionnel.
- **Algae** (n° 107) — paraphrase : « During the production phase this produces 2 plants. » ; carte : « During the production phase, this produces 2 plants. ». **Conséquence :** virgule manquante.
- **Anaerobic Microorganisms** (n° 5) — paraphrase : ordre inversé (ajout de microbe d'abord) et badges écrits « Animal, Microbe, or Plant » ; carte : réduction d'abord, badges en icônes « [animal], [microbe], or [plant] ».
- **Anti-Gravity Technology** (n° 6) — paraphrase : « Requires 5 SCT. » ; carte : « Requires 5 [science]. » **Conséquence :** aucune, abréviation maison du badge.
- **Archaebacteria** (n° 108) — paraphrase : « During the production phase this produces 1 plant. » ; carte : « During the production phase, this produces 1 plant. ». **Conséquence :** virgule manquante.
- **Artificial Lake** (n° 66) — carte : « [effect] Flip an ocean tile. » ; paraphrase sans le chevron d'effet immédiat. **Conséquence :** rendu d'icône seulement.
- **Asset Liquidation** (n° 11) — paraphrase : « ACTION: Action: Spend 1 TR… » ; carte : « Action: Spend 1 TR… ». **Conséquence :** aucune, doublon d'étiquette dans la référence.
- **Asset Liquidation** (n° 11) — paraphrase : « draw 3 cards » ; carte : « draw three cards ». **Conséquence :** aucune.
- **Asteroid Mining Consortium** (n° ★111) — paraphrase : « When you play a **Space** » ; carte : « When you play a **[space]** ». **Conséquence :** icône de badge rendue en toutes lettres.
- **Asteroid Mining** (n° 110) — paraphrase : « When you play a Space » ; carte : « When you play a [space] ». **Conséquence :** icône de badge rendue en toutes lettres.
- **Astrofarm** (n° 112) — paraphrase : « Add 2 microbes to ANOTHER card. » ; carte : « [effect] Add 2 microbes to ANOTHER card. ». Éclair d'effet immédiat non rendu.
- **Atmosphere Filtering** (n° 67) — paraphrase : « Requires 2 Science tags. Raise oxygen 1 step. » ; carte : « Requires 2 [science]. [effect] Raise oxygen 1 step. » **Conséquence :** aucune (événement rouge, effet immédiat par nature).
- **Atmospheric Insulators** (n° 113) — paraphrase : « per Earth you have » ; carte : « per [earth] you have ». **Conséquence :** rendu d'icône ; le « including this » est bien présent des deux côtés.
- **Automated Factories** (n° 114) — paraphrase : (pas de marqueur) ; carte : « **[effect]** You may play a green card… ». **Conséquence :** le pictogramme d'effet immédiat n'est pas rendu ; le déclenchement à la pose reste correct.
- **Beam from a Thorium Asteroid** (n° 116) — paraphrase : « Requires a Jupiter tag. » ; carte : « Requires a [jupiter]. » **Conséquence :** aucune.
- **Biothermal Power** (n° 118) — paraphrase : (pas de marqueur) ; carte : « **[effect]** Gain a forest VP… ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Birds** (n° 12) — paraphrase : « … 1 VP per animal on this card. » en fin ; carte : « *=1 VP per animal on this card. » en tête, avec l'astérisque de PV variables.
- **Blueprints** (n° 119) — paraphrase : « During the production phase you draw » ; carte : « During the production phase, you draw ». **Conséquence :** aucune.
- **Breathing Filters** (n° 68) — paraphrase : « Requires yellow oxygen or higher. » (avec espace finale) ; carte : aucun texte imprimé lu hors ce prérequis. **Conséquence :** aucune règle divergente ; la référence ne contient que le prérequis, ce qui est conforme.
- **Bribed Comittee** (n° 69) — clé du projet : « Bribed Comittee » ; nom imprimé : « Bribed Committee ». Faute d'orthographe dans le fichier de référence.
- **Bribed Committee** (n° 69) — paraphrase : « Raise your TR 2 steps. » ; carte : « [effect] Raise your TR 2 steps. ». Éclair non rendu.
- **Building Industries** (n° 120) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ». **Conséquence :** aucune.
- **Bushes** (n° 121) — carte : « [effect] Gain 2 plants. During the production phase**,** this produces 2 plants. » ; paraphrase sans chevron ni virgule. **Conséquence :** rendu et ponctuation.
- **Business Contracts** (n° 70) — clé du projet : « Business **Contracts** » ; nom imprimé : « Business **Contacts** ». **Conséquence :** la clé de référence ne correspond pas au nom imprimé.
- **Business Contracts** (n° 70) — paraphrase : « Draw **4** cards. Then discard **2** cards. » ; carte : « [effect] Draw **four** cards. Then, discard **two** cards. ». **Conséquence :** chiffres/lettres et virgule, valeurs identiques.
- **Cartel** (n° 123) — paraphrase : « per **Earth tag** you have » ; carte : « per **[earth]** you have ». **Conséquence :** icône de badge rendue en toutes lettres, décompte identique.
- **Cartes portant « МС » en cyrillique dans la référence** (Industrial Center n° 149, Io Mining Industries n° 153, Natural Preserve n° 169, Rad Suits n° 179, Trading Post n° 196, Tundra Farming n° 200) — paraphrase : « produces 2 **МС** » (U+041C/U+0421) ; carte : « produces 2 **MC** » (latin). **Conséquence :** toute recherche ou tout parsing sur la chaîne « MC » rate ces six cartes.
- **CEO's Favorite Project** (n° 71) — paraphrase : « Add 2 resources… » ; carte : « [effect] Add 2 resources… ». Éclair non rendu.
- **Circuit Board Factory** (n° 15) — paraphrase : « Action: Draw a card » ; carte : « Action: Draw a card. ». Point final absent.
- **Comet** (n° 73) — paraphrase : « Raise the temperature 1 step. Flip an ocean tile. » ; carte : « **[effect]** Raise the temperature 1 step. **[effect]** Flip an ocean tile. ». **Conséquence :** les deux pictogrammes d'effet ne sont pas rendus ; effets identiques.
- **Community Gardens** (n° 16) — paraphrase : « *if you chose » ; carte : « *If you chose ». **Conséquence :** aucune.
- **Convoy from Europa** (n° 74) — paraphrase : « Draw a card. Flip an ocean tile. » ; carte : « [effect] Draw a card. [effect] Flip an ocean tile. ». Deux éclairs non rendus.
- **Crater** (n° 75) — paraphrase : « Requires 3 EVT. Flip an ocean tile. » ; carte : « Requires 3 [event]. [effect] Flip an ocean tile. » **Conséquence :** aucune.
- **CrediCor** (n° 209) — paraphrase : « 48 Mc. » et « printed cost of 20 or more » ; carte : « You start with 48 MC. » et « printed cost of 20 **MC** or more ». **Conséquence :** casse de « MC » et unité omise sur le seuil ; le seuil reste 20.
- **CrediCor** (n° 209) — référence : `name` = « Credicor » ; carte : « **CrediCor** ». **Conséquence :** clé du projet mal capitalisée ; risque d'échec d'appariement avec les sources externes.
- **Decomposers** (n° 19) — paraphrase : « an **Animal, Microbe, or Plant** » ; carte : « an **[animal], [microbe], or [plant]** ». **Conséquence :** icônes de badge rendues en toutes lettres.
- **Decomposing Fungus** (n° 20) — paraphrase : « ACTION: Action: Remove 1 animal… » ; carte : « Action: Remove 1 animal… ». Mot-clé dupliqué dans la référence.
- **Decomposing Fungus** (n° 20) — paraphrase : « Place 2 microbes on this card. » ; carte : « [effect] Place 2 microbes on this card. ». Éclair non rendu.
- **Deep Well Heating** (n° 126) — paraphrase : « Raise the temperature 1 step. » ; carte : « [effect] Raise the temperature 1 step. » **Conséquence :** aucune, l'éclair marque l'effet immédiat déjà implicite.
- **Deimos Down** (n° 76) — carte : deux marqueurs d'effet immédiat (« [effect] Raise the temperature 3 steps. [effect] Gain 7 MC. ») ; paraphrase sans marqueurs. **Conséquence :** rendu d'icône.
- **Designed Microorganisms** (n° 127) — paraphrase : « During the production phase this produces 2 plants. » ; carte : « During the production phase, this produces 2 plants. ». **Conséquence :** virgule manquante.
- **Developed Infrastructure** (n° 21) — paraphrase : « Reduce this by 5 МС if you have 5 or more blue cards » (МС en caractères cyrilliques) ; carte : « Reduce this by 5 MC if you have five or more blue cards ». Caractères non latins et chiffre vs mot.
- **Diversified Interests** (n° 128) — carte : « [effect] Gain 3 plants and 3 heat… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Dusty Quarry** (n° 129) — paraphrase : « When you play a **Building** » ; carte : « When you play a **[building]** ». **Conséquence :** icône de badge rendue en toutes lettres.
- **Ecoline** (n° ★210) — paraphrase : « 27 Mc. 1 plant production. » ; carte : « You start with 1 plant production and 27 MC. » **Conséquence :** aucune, casse et ordre.
- **Ecological Zone** (n° 24) — paraphrase : la ligne de PV est placée en fin (« 1 VP per 2 animals… ») ; carte : elle est imprimée en tête, préfixée par l'astérisque (« *=1 VP per 2 animals on this card. »). **Conséquence :** ordre et perte du lien explicite avec la pastille de PV variable (*).
- **Energy Storage** (n° 131) — paraphrase : « draw 2 cards » ; carte : « draw two cards ».
- **Energy Subsidies** (n° 25) — paraphrase : « an Energy tag », « 4 МС » (M et C cyrilliques) ; carte : « an [energy] », « 4 MC ». **Conséquence :** aucune sur la règle, mais le « МС » cyrillique casse toute comparaison textuelle automatique.
- **Eos Chasma National Park** (n° 132) — carte : « [effect] Add 1 animal to ANOTHER card… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Farming Co-ops** (n° 29) — paraphrase : « Gain 3 plants **ACTION: Action:** Discard a card… » ; carte : « [effect] Gain 3 plants. **Action:** Discard a card… ». **Conséquence :** ponctuation manquante et mot-clé dupliqué dans la référence ; un parseur naïf peut lire « 3 plants ACTION » comme un seul segment.
- **Farming** (n° 133) — carte : « [effect] Gain 2 plants… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Fish** (n° 30) — paraphrase : « … 1 VP per animal on this card. » en fin, sans astérisque ; carte : « *=1 VP per animal on this card. » en tête.
- **Ganymede Shipyard** (n° 138) — paraphrase : « When you play a Space » ; carte : « When you play a [space] ». **Conséquence :** rendu d'icône.
- **Gene Repair** (n° 139) — paraphrase : « Requires 3 **Science tags**. » ; carte : « Requires 3 **[science]**. ». **Conséquence :** icône rendue en toutes lettres, seuil identique.
- **GHG Producing Bacteria** (n° 31) — référence : `name` = « GHG Production Bacteria » ; carte : « GHG Producing Bacteria ». **Conséquence :** clé du projet mal orthographiée.
- **Giant Ice Asteroid** (n° 77) — paraphrase : « Raise the temperature 2 steps. Flip 2 ocean tiles. » ; carte : « [effect] Raise the temperature 2 steps. [effect] Flip 2 ocean tiles. » **Conséquence :** aucune.
- **Grass** (n° 142) — paraphrase : « Gain 3 plants. » ; carte : « **[effect]** Gain 3 plants. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Great Escarpment Consortium** (n° ★144) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ». **Conséquence :** aucune.
- **Heather** (n° 145) — paraphrase : « Gain 1 plant. » ; carte : « **[effect]** Gain 1 plant. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Helion** (n° ★211) — clé du projet : « Helion Corporation » ; nom imprimé : « Helion ».
- **Helion** (n° ★211) — paraphrase : « 28 Mc. 3 Heat income. … but not vise versa. » ; carte : « You start with 3 heat production and 28 MC. … You may not use MC as heat. ». Télégraphie + faute « vise versa ».
- **Herbivores** (n° 33) — paraphrase : « Requires 5 oceans to be flipped. » et clause de PV rejetée en fin ; carte : « Requires 5 ocean tiles to be flipped. » et « *=1 VP per 2 animals on this card. » en tête. **Conséquence :** aucune.
- **Ice Asteroid** (n° 78) — paraphrase : « Flip 2 ocean tiles. » ; carte : « **[effect]** Flip 2 ocean tiles. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Ice Cap Melting** (n° 79) — paraphrase : « Flip an ocean tile. » ; carte : « [effect] Flip an ocean tile. ». Éclair non rendu.
- **Immigration Shuttles** (n° 146) — paraphrase : « 1 VP per 2 Earth tags you have. » placé en fin ; carte : « *=1 VP per 2 [earth] you have. » placé en tête. **Conséquence :** aucune.
- **Imported GHG** (n° 148) — paraphrase : « Gain 5 heat. » ; carte : « **[effect]** Gain 5 heat. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Imported Hydrogen** (n° 80) — paraphrase : « Flip an ocean tile. Gain 3 plants… » ; carte : « [effect] Flip an ocean tile. [effect] Gain 3 plants… ». Deux éclairs non rendus.
- **Imported Nitrogen** (n° 81) — paraphrase : quatre phrases nues ; carte : chaque phrase précédée de « [effect] ». **Conséquence :** aucune.
- **Industrial Center** (n° 149) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ». **Conséquence :** rendu d'icône.
- **Industrial Microbes** (n° 151) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ». Icône rendue en toutes lettres.
- **Interplanetary Conference** (n° 37) — paraphrase : « an Earth or Jupiter tag » ; carte : « an [earth] or [jupiter] ».
- **Interplanetary Relations** (n° 35) — paraphrase : « 1 VP per 4 cards you have played. » en fin ; carte : « *= 1 VP per four cards you have played. » en tête. **Conséquence :** aucune.
- **Interstellar Colony Ship** (n° 82) — paraphrase : « Requires 4 Science tag. » ; carte : « Requires 4 [science]. ». **Conséquence :** icône en toutes lettres et pluriel manquant ; le seuil 4 est identique. (Aucun texte d'effet imprimé, conforme.)
- **Invention Contest** (n° 83) — paraphrase : « Draw **3** cards. » ; carte : « **[effect]** Draw **three** cards. ». **Conséquence :** chiffre en lettres et pictogramme non rendu.
- **Inventrix** (n° 213) — paraphrase : « 33 Mc. Take 3 cards. … modified futher » ; carte : « At the start of the game, draw 3 cards. You start with 33 MC. … modified further ». Faute « futher » et formulation télégraphique.
- **Investment Loan** (n° 84) — paraphrase : « Gain 10 MC. » ; carte : « [effect] Gain 10 MC. » **Conséquence :** aucune.
- **Io Mining Industries** (n° 153) — paraphrase : ligne de PV en fin, « 1 VP per Jupiter tag you have » ; carte : en tête et préfixée de l'astérisque, « *=1 VP per [jupiter] you have ». Aussi « When you play Space » (article manquant) vs « When you play a [space] ». **Conséquence :** ordre, icônes et article ; valeurs identiques.
- **Kelp Farming** (n° 154) — paraphrase : « Gain 2 plants. » ; carte : « [effect] Gain 2 plants. ». Éclair non rendu.
- **Lagrange Observatory** (n° 85) — paraphrase : « Draw a card. » ; carte : « [effect] Draw a card. » **Conséquence :** aucune.
- **Lake Marineris** (n° 86) — référence : `name` = « **Lake Mariners** » ; carte : « **Lake Marineris** ». **Conséquence :** nom fautif dans la clé du projet. Par ailleurs le marqueur d'effet immédiat n'est pas rendu.
- **Large Convoy** (n° 87) — paraphrase : « Draw **2** cards. » ; carte : « **[effect]** Draw **two** cards. » (trois segments précédés de [effect]). **Conséquence :** chiffres en lettres, pictogrammes non rendus.
- **Lava Flows** (n° 88) — paraphrase : « Raise the temperature 2 steps. » ; carte : « [effect] Raise the temperature 2 steps. ». Éclair non rendu.
- **Lightning Harvest** (n° 156) — paraphrase : « per Science tag you have » ; carte : « per [science] you have ». **Conséquence :** rendu d'icône ; « including this » présent des deux côtés.
- **Local Heat Trapping** (n° 89) — paraphrase : « Gain 4 plants and add… » ; carte : « [effect] Gain 4 plants and add… ». Éclair non rendu.
- **Mars University** (n° 40) — paraphrase : « a Science tag », « a Plant tag » ; carte : « a [science] », « a [plant] » (icône verte petite, [microbe] non totalement exclu à la lecture).
- **Mass Converter** (n° 159) — paraphrase : « Requires 4 Science tags. […] When you play Space » ; carte : « Requires 4 [science]. […] When you play a [space] ». **Conséquence :** aucune.
- **Matter Manufacturing** (n° 41) — référence : `name` = « **Matter Manufactoring** » ; carte : « **Matter Manufacturing** ». **Conséquence :** faute d'orthographe dans la clé du projet.
- **Media Group** (n° 42) — paraphrase : « an **Event** » ; carte : « an **[event]** ». **Conséquence :** icône de badge rendue en toutes lettres.
- **Medical Lab** (n° 160) — paraphrase : « this produces 1 MC » ; carte : « produce 1 MC ». Formulation.
- **Micro-Mills** (n° 162) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ». **Conséquence :** rendu d'icône.
- **Microprocessors** (n° ★163) — paraphrase : « Draw **2** cards. » ; carte : « **[effect]** Draw **two** cards. ». **Conséquence :** chiffre en lettres, pictogramme non rendu.
- **Mine** (n° 164) — paraphrase : « When you play a Building » ; carte : « When you play a [building] ».
- **Miranda Resort** (n° 165) — paraphrase : « per Earth tag you have » ; carte : « per [earth] you have ». **Conséquence :** rendu d'icône.
- **Monocultures** (n° 167) — paraphrase : « Requires you to spend 1 TR. » ; carte : « [effect] Requires you to spend 1 TR. ». Éclair non rendu.
- **New Portfolios** (n° ★170) — paraphrase : « produces 1 **МС**, 1 plant and 1 heat » (М et С cyrilliques) ; carte : « produces 1 **MC**, 1 plant, and 1 heat ». **Conséquence :** caractères cyrilliques homoglyphes dans la référence : toute recherche/parsing sur « MC » rate cette carte.
- **Nitrite Reducing Bacteria** (n° 43) — paraphrase : « Add 3 microbes to this card. ACTION: Action: Add 1 microbe to this card or remove… » ; carte : « [effect] Add 3 microbes to this card. Action: Add 1 microbe to this card, or remove… ». Mot-clé dupliqué, éclair non rendu, virgule absente.
- **Nitrite Reducting Bacteria** (n° 43) — clé du projet : « Nitrite Reducting Bacteria » ; nom imprimé : « Nitrite Reducing Bacteria ». Faute d'orthographe dans le fichier de référence.
- **Nitrogen-Rich Asteroid** (n° 91) — paraphrase : phrases nues ; carte : « [effect] » devant chacune, et « If you have have 3 or more » (doublon d'impression relevé à la lecture). **Conséquence :** aucune.
- **Nitrophilic Moss** (n° ★171) — référence : `name` = « **Nitropholic Moss** » ; carte : « **Nitrophilic Moss** ». **Conséquence :** faute d'orthographe dans la clé du projet.
- **Noctis Farming** (n° 172) — paraphrase : « Gain 2 plants. » ; carte : « **[effect]** Gain 2 plants. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Olympus Conference** (n° 44) — paraphrase : « a Science tag » ; carte : « a [science] ». **Conséquence :** aucune.
- **Optimal Aerobraking** (n° 45) — paraphrase : « an Event tag » ; carte : « an [event] ». **Conséquence :** rendu d'icône.
- **Permafrost Extraction** (n° 92) — paraphrase : « Flip an ocean tile. » ; carte : « **[effect]** Flip an ocean tile. ». **Conséquence :** pictogramme d'effet immédiat non rendu.
- **Phobolog** (n° 215) — clé du projet : « Phobolog » ; nom imprimé : « PhoboLog ». Casse.
- **Phobos Falls** (n° 93) — paraphrase : trois phrases nues ; carte : « [effect] » devant chacune. **Conséquence :** aucune.
- **Physics Complex** (n° 46) — paraphrase : « 1 VP per 2 science res on this card » et « Requires 4 Science tags » ; carte : « *= 1 VP per 2 science **resources** on this card » et « Requires 4 [science] ». **Conséquence :** abréviation et icône ; valeurs identiques.
- **Plantation** (n° 94) — paraphrase : « Requires 4 **science tags**. » ; carte : « Requires 4 **[science]**. ». **Conséquence :** icône rendue en toutes lettres, seuil identique.
- **Power Grid** (n° 174) — paraphrase : « 1 МС » (caractères cyrilliques) ; carte : « 1 MC ».
- **Protected Valley** (n° 177) — paraphrase : « produces 2 МС » (caractères cyrilliques) ; carte : « produces 2 MC ».
- **Quantum Extractor** (n° 178) — paraphrase : « Requires 3 Science tags. » ; carte : « Requires 3 [science]. » **Conséquence :** aucune.
- **Recycled Detritus** (n° 48) — paraphrase : « an **Event**, draw **2** cards » ; carte : « an **[event]**, draw **two** cards ». **Conséquence :** icône et chiffre en toutes lettres.
- **Release of Inert Gases** (n° 95) — carte : « [effect] Raise your TR 2 steps. » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Research** (n° 96) — paraphrase : « Draw **2** cards. » ; carte : « **[effect]** Draw **two** cards. ». **Conséquence :** chiffre en lettres, pictogramme non rendu.
- **Satellites** (n° 181) — paraphrase : « per Space you have » ; carte : « per [space] you have ». **Conséquence :** rendu d'icône.
- **Sattellite Farms** (n° 180) — clé du projet : « **Sattellite** Farms » ; nom imprimé : « **Satellite** Farms ». **Conséquence :** faute d'orthographe dans la clé de référence.
- **Sattellite Farms** (n° 180) — paraphrase : « per **Space** you have » ; carte : « per **[space]** you have ». **Conséquence :** icône de badge rendue en toutes lettres.
- **Saturn Systems** (n° 216) — paraphrase : « 24 Mc. » et « a Jupiter tag » ; carte : « You start with 24 MC. » et « a [jupiter] ».
- **Smelting** (n° 183) — paraphrase : « Draw **2** cards. » ; carte : « **[effect]** Draw **two** cards. ». **Conséquence :** chiffre en lettres, pictogramme non rendu.
- **Soil Warming** (n° ★184) — paraphrase : « Raise the temperature 1 step. » ; carte : « [effect] Raise the temperature 1 step. ». Éclair non rendu.
- **Solar Trapping** (n° 186) — carte : « [effect] Draw a card and gain 3 heat… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Space Heaters** (n° ★188) — paraphrase : « Draw a card. » ; carte : « [effect] Draw a card. » **Conséquence :** aucune.
- **Space Heaters** (n° ★188) — référence : `name` = « Space Heater » ; carte : « Space Heaters ». **Conséquence :** clé du projet au mauvais nombre.
- **Space Station** (n° 189) — paraphrase : « When you play a Space tag » ; carte : « When you play a [space] ». **Conséquence :** rendu d'icône.
- **Special Design** (n° 97) — paraphrase : (pas de marqueurs) ; carte : « **[effect]** … **[effect]** … ». **Conséquence :** pictogrammes d'effet non rendus, texte sinon identique.
- **Sponsors** (n° ★190) — paraphrase : « produces 2 МС » (caractères cyrilliques) ; carte : « produces 2 MC ».
- **Strip Mine** (n° 191) — paraphrase : « When you play a **Building**… When you play a **Space**, you pay 3 **less** » ; carte : « … a **[building]** … a **[space]**, you pay 3 **MC** less ». **Conséquence :** icônes en toutes lettres et unité « MC » omise dans la référence.
- **Subterranean Reservoir** (n° 98) — paraphrase : « Flip an ocean tile. » ; carte : « [effect] Flip an ocean tile. ». Éclair non rendu.
- **Surface Mines** (n° 192) — paraphrase : « you pay 2 less », « you pay 3 less », « a Building », « Space » ; carte : « you pay 2 MC less », « you pay 3 MC less », « a [building] », « a [space] ». **Conséquence :** aucune sur la valeur, mais l'unité MC est omise dans la référence.
- **Tall Station** (n° 195) — clé du projet : « **Tall** Station » ; nom imprimé : « **Toll** Station ». **Conséquence :** la clé de référence ne correspond pas au nom imprimé (mot différent, pas seulement une coquille de casse).
- **Tall Station** (n° 195) — paraphrase : « without **payind** its MC cost… produces 3 **МС** » (М et С cyrilliques) ; carte : « without **paying** its MC cost… produces 3 **MC** ». **Conséquence :** coquille + homoglyphes cyrilliques ; parsing sur « MC » en échec. Le pictogramme [effect] initial n'est pas rendu non plus.
- **Tardigrades** (n° 58) — paraphrase : « Action: Add 1 microbe to this card. 1 VP per 3 microbes on this card. » ; carte : « *=1 VP per 3 microbes on this card. Action: Add 1 microbe to this card. ». Ordre inversé, astérisque de PV variables absent.
- **Technology Demonstration** (n° 99) — paraphrase : « Flip an ocean tile. Draw 2 cards » (sans point final) ; carte : « [effect] Flip an ocean tile. [effect] Draw two cards. » **Conséquence :** aucune.
- **Teractor Corporation** (n° ★217) — clé du projet : « Teractor **Corporation** » ; nom imprimé : « Teractor ». **Conséquence :** la clé de référence ajoute un mot absent de la carte.
- **Teractor Corporation** (n° ★217) — paraphrase : « 51 **Mc**… get 3MC **discount** » ; carte : « You start with 51 **MC**… you pay 3 MC less for it ». **Conséquence :** casse et formulation non canoniques.
- **Terraforming Ganymede** (n° 100) — paraphrase : « Raise your TR 1 step… » ; carte : « [effect] Raise your TR 1 step… ». Éclair non rendu.
- **Tharsis Republic** (n° 218) — paraphrase : « 40 Mc. » et pas de point final ; carte : « You start with 40 MC. » **Conséquence :** aucune.
- **Think Tank** (n° 59) — paraphrase : « Action: Spend 2 MC to draw a card. 1 VP per 3 blue cards you have in play. » ; carte : « *= 1 VP per **three** blue cards you have in play. Action: Spend 2 MC to draw a card. ». **Conséquence :** ordre inversé et nombre en toutes lettres ; règle identique.
- **Thorgate Corporation** (n° 219) — clé du projet : « Thorgate **Corporation** » ; nom imprimé : « Thorgate ». **Conséquence :** la clé de référence ajoute un mot absent de la carte.
- **Thorgate Corporation** (n° 219) — paraphrase : « 45 **Mc**. 1 Heat **income**… get 3MC **discount** » ; carte : « You start with 1 heat **production** and 45 **MC**… you pay 3 MC less for it ». **Conséquence :** vocabulaire non canonique (« income » pour « production »), casse ; valeurs identiques.
- **Titanium Mine** (n° 194) — paraphrase : « When you play a Space » ; carte : « When you play a [space] ».
- **Towing a Comet** (n° 101) — paraphrase : phrases nues, pas de point final ; carte : « [effect] » devant chaque phrase, point final présent. **Conséquence :** aucune.
- **Trading Post** (n° ★196) — carte : « [effect] Gain 3 plants… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Trees** (n° 198) — paraphrase : « Gain 1 plant. » ; carte : « [effect] Gain 1 plant. ». Éclair non rendu.
- **Tropical Resort** (n° 199) — paraphrase : « produces 4 МС » (M et C cyrilliques) ; carte : « produces 4 MC ». **Conséquence :** aucune sur la règle, mais casse toute comparaison textuelle automatique.
- **Tundra Farming** (n° 200) — carte : « [effect] Gain 1 plant… » ; paraphrase sans marqueur. **Conséquence :** rendu d'icône.
- **Underground City** (n° 201) — paraphrase : « produces 1 **МС** » (cyrillique) « … a **Building** » ; carte : « produces 1 **MC** … a **[building]** ». **Conséquence :** homoglyphes cyrilliques + icône en toutes lettres.
- **United Nations Mars Initiative** (n° 220) — référence : `name` = « **Unmi** » et « RT » ; carte : « **United Nations Mars Initiative** » et « TR ». **Conséquence :** clé du projet réduite à un sigle non imprimé et sigle du terraforming rating inversé.
- **Venture Capitalism** (n° 203) — paraphrase : « produces 1 **МС** per **Event** you have » (cyrillique) ; carte : « produces 1 **MC** per **[event]** you have ». **Conséquence :** homoglyphes cyrilliques + icône en toutes lettres.
- **Vesta Shipyard** (n° 204) — paraphrase : « When you play Space » ; carte : « When you play a [space] ». Article manquant, icône rendue en toutes lettres.
- **Viral Enhancers** (n° 61) — paraphrase : « a Plant, Microbe, or Animal tags » (ordre différent, accord fautif, double espace avant « 1 animal ») ; carte : « a [animal], [microbe], or [plant] ». **Conséquence :** aucune, l'ensemble des trois badges est identique.
- **Volcanic Pools** (n° 62) — paraphrase : « per Energy tag you have » ; carte : « per [energy] you have ». **Conséquence :** rendu d'icône.
- **Water Import from Europa** (n° 63) — paraphrase : « 1 VP per **Jupiter** you have. » ; carte : « ***=1 VP per [jupiter]** you have. ». **Conséquence :** icône rendue en toutes lettres et astérisque de PV variable non rendue.
- **Windmills** (n° 206) — paraphrase : « per Energy tag » ; carte : « per [energy] ». **Conséquence :** aucune (l'écart de règle est traité en G1).
- **Wood Burning Stoves** (n° 64) — paraphrase : « Gain 4 plants. **ACTION: Action:** Spend 4 plants to raise temperature 1 step. » ; carte : « [effect] Gain 4 plants. **Action:** Spend 4 plants to raise **the** temperature 1 step. ». **Conséquence :** marqueur `Action:` dupliqué dans la paraphrase et article manquant ; un parseur naïf peut couper le texte au mauvais endroit.
- **Work Crews** (n° 102) — paraphrase : (pas de marqueurs) ; carte : « **[effect]** … **[effect]** … ». **Conséquence :** pictogrammes d'effet non rendus, texte sinon identique.
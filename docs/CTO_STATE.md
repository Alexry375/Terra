# Carte d'état — Projet Terra

> Source de vérité du projet. Ancrée au code (`fichier:ligne`) dès qu'il y aura du
> code. [VÉRIFIÉ JJ-MM] = relu à la source ce jour-là. [DÉCLARÉ] = non re-vérifié.

Dernière mise à jour : 2026-07-24

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
  (`--probe "A;B"` + `paid[]`, `--probe-action`). **152 tests verts** ;
  revalidé après promotion : 300/300 graine 2024, 0 violation,
  4302 actions bleues ; 67 générations ON vs 114 OFF. [VÉRIFIÉ 24-07]
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
4. **Interfaces de jeu** : en ligne, et/ou plateau physique par caméra. Reporté à
   après le moteur et l'IA. Visuels de cartes : chaîne Tabletop Simulator
   VALIDÉE (voir §Acquis scans). [VÉRIFIÉ 24-07]

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
  peut-être « 2 titane »). À arbitrer à l'audit de `moteur-cartes-2`
  (point d'enquête déjà au contrat) + photos du livret par Alexis
  (annoncées). Sources texte complémentaires trouvées :
  `alrusdi/tmae-content` (xlsx 219 cartes base, 2021),
  `sebwieser/ares-expedition` (Python, 2023). [VÉRIFIÉ 24-07]

## Travaux en cours

- Aucun sous-agent en cours. Prochain chantier : `moteur-cartes-3`
  (ressources posées sur les cartes : microbes/animaux/science — dernière
  grande famille de mécanismes projets ; puis VP dynamiques ANIMAL/MICROBE
  réels au score). [VÉRIFIÉ 24-07]

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

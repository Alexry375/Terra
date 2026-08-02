# Journal de bord — Projet Terra

> Une entrée par journée où il s'est passé quelque chose. Fuseau : America/Martinique.
> [VÉRIFIÉ] = relu à la source. [DÉCLARÉ] = rapporté sans re-vérification.

## 2026-07-23 — Naissance du projet

- Alexis a exposé l'objectif : une IA **imbattable par des humains** à
  *Terraforming Mars : Expédition Arès*, avec l'extension « Découverte », les
  règles maison de mulligan (cartes projets : les 8 ou rien ; corporations : les
  2 ou aucune), de l'interprétabilité (probabilité de victoire, meilleures
  cartes), et à terme un jeu en ligne et/ou sur plateau physique. [VÉRIFIÉ —
  message d'Alexis de ce jour, repris dans `docs/CTO_PROJET.md`]
- Installation de l'agent CTO : création du dépôt git, de la carte d'état
  (`docs/CTO_STATE.md`), de la configuration projet (`docs/CTO_PROJET.md`) et de
  ce journal. Aucun code écrit. [VÉRIFIÉ 23-07]
- Alexis a validé le lancement de l'étude du terrain (objectif n°1), fixé le
  fuseau America/Martinique, pas d'échéance. [VÉRIFIÉ — son message du jour]
- Étude du terrain menée par le harnais de recherche approfondie (106 agents,
  ~20 min, 22 affirmations confirmées / 3 réfutées) → `docs/ETUDE_TERRAIN.md`.
  Conclusion : `nikitinalexx/ares-expedition` est le seul simulateur libre
  couvrant Discovery ; aucune base de cartes JSON n'existe ; précédent IA le
  plus proche : Race for the Galaxy (`bnordli/rftg`). [VÉRIFIÉ 23-07]
- Alexis a **validé la stratégie à deux moteurs** (nikitinalexx comme référence
  de règles et source de cartes + simulateur rapide maison pour
  l'entraînement). [VÉRIFIÉ — son message du jour]
- Correction d'une formulation trompeuse du rapport : les cartes sont bien
  toutes répertoriées dans le code de nikitinalexx (403 classes Java, extension
  d'origine marquée par carte) — ce qui manque, c'est un fichier de données
  autonome réutilisable. Vérifié par clone du dépôt. Anomalie relevée :
  52 classes projet Discovery pour 38 cartes officielles, à élucider.
  [VÉRIFIÉ 23-07]
- Complément sur Race for the Galaxy : l'IA de Keldon Jones (niveau difficile)
  se classe environ dans le 1 % des meilleurs joueurs (~100e sur ~10 000 au
  classement historique), mais les experts la battent régulièrement — forte,
  pas surhumaine façon moteurs d'échecs. Sources BGG/Temple Gates. [VÉRIFIÉ
  23-07 — sources secondaires, pas de mesure formelle publiée]
- **Workspace `audit-nikitinalexx` lancé puis livré et audité OK le jour même**
  (sous-agent Claude, ~32 min). Contrat scellé (3 contrôles visibles + 1 caché
  sur 3 cartes témoins) ; audit : 3/3 verts, hold-out exact, pas de trafiquage,
  divergences déclarées (BGG en 403 → liste Mylaana substituée ; partie non
  jouée jusqu'au bout). Sondage du chemin critique : les affirmations
  surprenantes (typo cyrillique « руфе », carte non enregistrée) re-vérifiées
  vraies à la source. Détail des acquis → `docs/CTO_STATE.md` §Acquis.
  [VÉRIFIÉ 23-07]
- Erreur de ma part corrigée en cours de route : j'avais présenté « aucune base
  de cartes » de façon trompeuse ; et l'audit a aussi trouvé une implémentation
  indépendante supplémentaire (Mylaana/AresExpedition, avec
  `data/cards_data.json`) que l'étude de terrain avait manquée. [VÉRIFIÉ 23-07]
- Limites honnêtes de l'étude : volet « implémentations officielles » (appli
  Asmodee, Board Game Arena, Tabletop Simulator) non couvert par des
  affirmations vérifiées ; fidélité des règles de nikitinalexx non auditée
  carte par carte ; volet légal non sourcé en droit français. [VÉRIFIÉ 23-07]

## 2026-07-24 — Retag des cartes livré et audité

- Workspace `retag-cartes` livré par sous-agent (~17 min) et audité OK :
  contrôles 3/3, hold-out exact, pas de trafiquage. La base de cartes du projet
  existe : `cards_v1.json` (388 cartes, boîte d'origine, pioche v1 = 281).
  [VÉRIFIÉ 24-07]
- L'agent a dépassé le contrat en bien : 2 bogues du moteur Java découverts
  (Buffed* en base ; Mai-Ni nommée « Teractor »), et une erreur factuelle de
  MON contrat déclarée (les cartes « infrastructure » sont très probablement
  l'extension officielle Foundations, pas du contenu maison). Leçon CTO :
  mes contrats doivent laisser une sortie propre aux contradictions
  factuelles (enum extensible ou champ « contestation »). [VÉRIFIÉ 24-07]
- Découverte structurante : il existe DEUX packs promo officiels. Le second
  (KS 2021, 17 cartes) est fondu dans la « base » du moteur → décision Alexis
  requise (possède-t-il ces cartes ?) avant un round 2. [VÉRIFIÉ 24-07]
- Workspace `moteur-squelette` livré (~32 min) et audité OK : squelette Rust
  complet, 27 tests, 1000/1000 parties sans violation, déterminisme et
  hold-out à graine inédite verts. Promu dans `engine/` — un bogue de chemin
  relatif attrapé à la promotion (tests pointant vers le workspace), corrigé
  et revalidé. Déjà ~6 500 parties/s avec effets stubbés. [VÉRIFIÉ 24-07]
- Chantier `moteur-cartes-1` cadré, scellé et lancé (sous-agent en cours) :
  extraction des VP des 388 cartes + couche d'effets + lot ≥ 50 cartes
  fidèles au texte imprimé. Cadrage payant : trois pièges désamorcés AVANT le
  seal — (1) bogue oracle supplémentaire (`NitrogenRichAsteroid.java` teste
  `== 3` tags Plante au lieu de « 3 ou plus ») → règle contractuelle « le
  texte imprimé gagne » ; (2) la sonde d'audit `--probe` aurait échoué sur
  les cartes à prérequis (Farming exige la température blanche, départ =
  violet) → spec corrigée (pose forcée + champ `prereq_ok` séparé) ;
  (3) logique jq des contrôles testée dans les deux sens sur un état-cible
  simulé + contre-test négatif (altération d'un champ d'origine détectée).
  Hold-out : 5 témoins VP + 5 témoins d'effets vérifiés à la source Java +
  graine inédite. [VÉRIFIÉ 24-07]
- Chantier `moteur-cartes-1` livré (~34 min de sous-agent) et audité OK :
  7/7 contrôles (dont hold-out : témoins VP et effets vérifiés à la source
  Java), 99 tests, 63 cartes aux effets complets, VP des 388 cartes extraits,
  sonde `--probe`, `--effects on|off`. Contre-vérification de la main :
  10 encodages relus contre le texte imprimé (10/10), sondage graine 424242.
  L'agent a déclaré proprement mes deux contradictions de contrat (Grain
  Silos hors pioche v1 ; relecture par sous-agent impossible dans son
  environnement). Promu dans `engine/` + `data/cards.json` ; chemins
  re-adaptés (leçon du squelette appliquée), 99 tests verts après promotion.
  Signal encourageant : les effets raccourcissent les parties aléatoires
  (73 générations contre 114 sans effets). [VÉRIFIÉ 24-07]
- Décision pack KS 2021 CLOSE : Alexis certain de ne pas posséder Synthetic
  Catastrophe (témoin le plus reconnaissable) → pack non possédé, les
  17 cartes `promo2021` restent hors pioche (réglage déjà en place, aucun
  changement de données). [VÉRIFIÉ 24-07]
- Directive Alexis : les sous-agents de chantier passent sur Opus 4.8
  (le CTO reste sur Fable 5) ; exception possible si justifiée. [VÉRIFIÉ 24-07]
- Chantier `moteur-cartes-2` cadré, scellé et lancé (sous-agent Opus 4.8 en
  cours) : réductions de coût, effets déclenchés, actions bleues réelles.
  Cadrage vérifié à la source : 10 imposées relues dans le Java (dont
  découverte d'une bizarrerie à élucider : `AsteroidMining.java` porte un
  revenu `GainType.TITANIUM` alors que le texte imprimé dit « pay 6 MC
  less » → point d'enquête contractuel) ; une erreur de témoin attrapée
  AVANT le seal (Lichen coûte 5, pas 7 — vérification carte par carte).
  Checks testés dans les deux sens + 3 contre-tests de falsification
  détectés. Hold-out : 4 scripts cachés. [VÉRIFIÉ 24-07]
- Reconnaissance scans (sous-agent Opus, 2 gels d'infrastructure surmontés) :
  chaîne Tabletop Simulator validée sans posséder le jeu (API Steam →
  save → URLs → hôte réécrit) ; planches 4096×4000 qualité impression ;
  base anglaise complète (mod 2831959805), Discovery confirmée seulement
  en espagnol (3009184792). Téléchargement base lancé dans data/scans/
  (hors git). [VÉRIFIÉ 24-07]
- CORRECTION DE FOND grâce au premier scan lu : le titane et l'acier
  existent bien dans Ares Expedition (PhoboLog, Mining Guild) — mon
  affirmation inverse à Alexis était fausse, il avait raison. Le champ
  description de cards.json est donc possiblement une paraphrase, pas le
  texte imprimé : impact à arbitrer à l'audit du lot 2 (Asteroid Mining
  et consorts) puis avec les photos du livret d'Alexis. [VÉRIFIÉ 24-07]
- Chantier `moteur-cartes-2` livré (~73 min de sous-agent Opus 4.8, 1 gel
  d'infrastructure surmonté) et audité OK : 8/8 contrôles, 152 tests,
  47 cartes neuves (26 réductions, 9 déclencheurs, 14 actions bleues),
  sondes v2, compteur blue_actions (13523/1000 parties). Contre-vérifications
  de la main : 19/19 réductions relues au texte, graine indépendante 909090,
  6 sondes hors hold-out. Enquête titane élucidée : en AE le titane du Java
  est un compteur de réduction (×3 MC/Espace), pas une ressource dépensée —
  cohérent avec le scan PhoboLog. Promu dans engine/ (152 tests verts,
  67 générations ON vs 114 OFF). Verdict ok. [VÉRIFIÉ 24-07]
- RÈGLES OFFICIELLES AU DÉPÔT : Alexis a photographié ses deux livrets FR
  (23 photos : base 19 pages + Découverte 4 pages, complet). Créé
  docs/regles/ (référence unique, hiérarchie des sources écrite) :
  transcription mot à mot par 5 sous-agents Opus 4.8 en parallèle,
  assemblée en livret-base.md + livret-decouverte.md ; règles condensées
  (~215 l.) ; registre cas-tranches.md. Audit par échantillon de la main :
  3 pages relues contre photos, fidèles (1 correction d'icône Récompense
  Industriel). TITANE/ACIER TRANCHÉ par le livret p. 18 : « savoir-faire »,
  compteurs permanents de réduction (2 MC acier/Construction, 3 MC
  titane/Espace), pas des ressources dépensées — le modèle Java et notre
  encodage sont bons ; migration vers compteurs à prévoir pour Advanced
  Alloys/Phobolog/Aquifer Pumping/Solarpunk. Points ouverts consignés :
  listes complètes Objectifs/Récompenses/Phases améliorées (scans),
  contradiction p. 14/p. 20 sur la phase Actions. [VÉRIFIÉ 24-07]
- AUDIT DE CONFORMITÉ moteur ↔ livret officiel (demandé par Alexis) :
  sous-agent Opus 4.8, ~55 points de règles confrontés au code, rapport
  docs/regles/notes/conformite-moteur-24-07.md. ~44 conformes (bornes,
  NT, actions standard, bonus de phase, production, fin de partie, main
  10, mulligans maison). 6 écarts réels, tous mineurs ; les 3 principaux
  contre-vérifiés par la main dans flow.rs : E6 prérequis évalués à
  l'état courant au lieu de l'instantané de début de phase (flow.rs:216),
  E1 milestones/awards Découverte toujours comptés sans bascule « base »
  (flow.rs:960-977 — mineur : nos parties réelles incluent Découverte),
  E2 bonus phase II « piocher avant OU après » réduit à après
  (flow.rs:696-702). E3 départage d'égalité absent, E4 défausse +3 MC
  restreinte à la phase III, E5 conversion fin de phase sur état courant.
  Corrections à intégrer avant/avec moteur-cartes-3. [VÉRIFIÉ 24-07]
- Décisions d'Alexis sur les écarts et règles (24-07 après-midi) :
  (1) règle maison CONFIRMÉE « J1/J2 alterné à chaque manche, actions en
  alternance jusqu'à double passe » — même en connaissant la règle p. 14 ;
  (2) règle maison « égalité au score = égalité », pas de départage p. 16 ;
  (3) correction E2 (bonus Construction : pioche AVANT ou après la pose)
  jugée importante par Alexis — prioritaire ; (4) E4 expliqué (défausse
  +3 MC « à tout moment » p. 5 + paiement de cartes par défausse p. 13,
  non modélisés — affordabilité sous-estimée). Règles maison consignées
  dans docs/regles/README.md. Corrections E2/E4/E6 + règles maison à
  intégrer en tête de moteur-cartes-3 ; E3 devient sans objet (égalité
  maison) ; E1 sans objet pratique (parties avec Découverte). [VÉRIFIÉ 24-07]
- Chantier `moteur-conformite-1` cadré et lancé (sous-agent Opus 4.8). Choix
  de méthode : le lot 3 annoncé est SCINDÉ en 3-A (conformité + règles
  maison, ce lot) et 3-B (ressources sur cartes), car 3-B pose des cartes
  qui se paient par défausse et déclenchent des effets — il doit s'appuyer
  sur un cœur déjà corrigé. Contrat scellé : C1 prérequis de PARAMÈTRES sur
  l'instantané de début de phase (tags/dépenses restent à l'état courant),
  C2 pioche avant OU après en phase II, C3 défausse-paiement (3 MC/carte,
  minimum nécessaire, surplus rendu) dans affordable ET build_card, C4 ordre
  J1/J2 alterné avec alternance action par action en phase III, C5 égalité
  sèche + conversion obligatoire sur l'instantané, C6 >= 170 tests, C7
  conformite.md. Sonde étendue : --probe-mc, --probe-filler, --probe-strict,
  champs discarded[] et prereq_ok_now (le mode strict est le seul moyen
  d'observer C1 carte par carte — la sonde du lot 2 force la pose).
  4 checks testés dans les DEUX sens (rouges sur l'état actuel, verts sur
  état-cible simulé) + 6 contre-tests de falsification tous détectés.
  4 hold-outs cachés à valeurs relevées à la source : témoins d'instantané
  opposés (Giant Ice Asteroid;Great Dam doit bloquer / Crater après 3
  événements doit passer — piège si l'agent bascule les tags sur
  l'instantané), arithmétique de défausse avec surplus rendu, ordre du tour
  sur graines inédites, non-régression lots 1-2 (vert sur le moteur actuel,
  7782 parties/s). [VÉRIFIÉ 24-07]
- Chantier `moteur-conformite-1` livré (~53 min de sous-agent Opus 4.8,
  aucun gel) et audité OK : 8/8 dont les 4 hold-outs cachés — y compris les
  deux témoins d'instantané OPPOSÉS, que l'agent a passés sans les voir
  (la règle n'a donc pas été sur-appliquée aux badges). Contre-vérifications
  de la main : 264 cartes sondées rigoureusement identiques à l'ancien
  moteur (rétro-compatibilité totale), graine inédite 555777 saine,
  arithmétique de défausse-paiement recalculée à la main, ordre du tour lu
  sur l'état réel. Promu : 185 tests verts, ~13 000 parties/s (le double
  d'avant), 95 manches alternées sur la partie témoin. Verdict ok.
  MON ERREUR DE CONTRAT, relevée par l'agent : un cas imposé de sonde
  (delta.hand = -1) était incompatible avec la convention du lot 2 ;
  l'agent a tranché en basculant la base de calcul selon les options et
  l'a déclaré — cosmétique, consigné dans la carte. Deux tests existants
  adaptés au niveau du harnais, relus, aucune assertion affaiblie.
  [VÉRIFIÉ 24-07]

## 2026-07-25 — Lot 3 cadré et lancé : les ressources posées sur les cartes

- **Inventaire du périmètre repris à la source, et correction d'un chiffre de la
  carte d'état.** Le 24-07 j'avais annoncé « ~41 cartes concernées » pour les
  ressources sur cartes. C'était une estimation obtenue par recherche de mots
  dans les descriptions : elle mélangeait deux mécanismes différents. Le
  décompte réel, carte par carte, donne **28 cartes** pour les ressources
  (14 conteneurs + 14 poseuses), et **~24 autres cartes** relèvent d'un
  mécanisme distinct — la production ou les points variables **par tag**
  (Cartel, Satellites, Lightning Harvest, Worms, Microbiology Patents…), qui
  fera l'objet d'un lot 4. [VÉRIFIÉ 25-07 — comptage sur `data/cards.json`
  filtré sur `in_deck_v1`]
- **Lecture des 28 classes Java à la source** avant d'écrire le contrat (clone
  neuf du dépôt nikitinalexx, l'ancien clone n'existait plus sur le disque).
  Points relevés qui ont servi à fixer des valeurs témoins : le modèle de
  référence est une simple table « carte → nombre », initialisée à 0 à la pose
  (`Player.initResources`) ; Cryogenic Shipment et Imported Hydrogen posent
  **3 ressources si la carte visée porte des microbes, 2 si elle porte des
  animaux** ; Ecological Zone se déclenche sur sa propre pose et compte
  **2 tags** (Plante + Animal), donc 2 animaux d'entrée. [VÉRIFIÉ 25-07]
- **PIÈGE TROUVÉ ET DÉSAMORCÉ — les classes « Buffed ».** Le dépôt Java contient
  des versions rééquilibrées maison de certaines cartes. Quatre cartes du lot en
  ont une : Birds, Extreme-Cold Fungus, GHG Production Bacteria, Regolith
  Eaters. `BuffedBirds.java` exige un oxygène **jaune ou blanc** là où la vraie
  `Birds.java` exige **blanc**. Un agent qui lit la mauvaise classe encode une
  carte trop facile à jouer. Le piège est écrit dans le contrat ET vérifié par
  un contrôle caché qui relit l'encodage livré. Je me suis moi-même fait avoir
  au premier essai d'extraction (dictionnaire indexé par nom, la version Buffed
  écrasant l'officielle) — d'où la consigne de filtrer sur `in_deck_v1`.
  [VÉRIFIÉ 25-07]
- **Décision d'architecture : les choix des cartes passent par la politique.**
  Toute alternative (« gagne 1 plante **ou** pose 1 microbe ») et tout choix de
  carte cible devient une méthode du trait `Policy`, avec une implémentation par
  défaut pour ne casser aucune politique existante. Raison : l'IA future doit
  pouvoir **apprendre** ces choix ; un choix câblé dans le moteur serait un
  plafond de compétence définitif. Le contrat l'impose et l'interdit inverse
  (« cible câblée sur le premier candidat ») est activement recherché.
  [VÉRIFIÉ 25-07]
- **Déterminisme imposé** : interdiction d'utiliser une table de hachage pour
  stocker les ressources — son ordre de parcours n'est pas reproductible, ce qui
  casserait les parties à graine fixe sur lesquelles repose tout l'audit.
  Contrôlé par lecture du code livré. [VÉRIFIÉ 25-07]
- **Contrat scellé** (`workspaces/moteur-cartes-3`) : 4 contrôles visibles,
  testés **dans les deux sens** (rouges sur le moteur actuel, verts contre un
  faux moteur simulant l'état-cible) et soumis à **7 tentatives de falsification
  — 7 détectées** : compteur figé, compteur forfaitaire, effets « désactivés »
  non neutres, cible imposée ignorée, points de victoire sans division entière,
  toute carte rendue réceptacle, liste de ressources non triée. [VÉRIFIÉ 25-07]
- **4 contrôles cachés** déposés hors du dépôt : fidélité des prérequis (piège
  Buffed), arithmétique exacte des ressources (dont l'absence de compensation
  quand l'amélioration de phase est sautée), santé sur graines inédites,
  non-régression des lots 1/2/conformité. Les trois premiers sont rouges sur
  l'état actuel et verts sur l'état-cible ; le quatrième est vert dès
  maintenant — c'est son rôle. [VÉRIFIÉ 25-07]
- Agent Opus 4.8 lancé sur le workspace. Audit à venir. [VÉRIFIÉ 25-07]

## 2026-07-25 (suite) — Lot 3 livré en deux rounds, et une erreur de contrat tranchée par les scans

- **Round 1 livré** : mécanisme complet des ressources sur cartes, 28 cartes,
  230 tests verts. Audit : 7/8 au premier passage. Le contrôle caché en échec
  était **le mien** : j'attendais 3 plantes pour la branche « plantes » d'Imported
  Hydrogen, en oubliant que la tuile Océan révélée en rapporte 2 de plus — un
  comportement déjà présent dans l'ancien moteur. Attente corrigée → 8/8.
  [VÉRIFIÉ 25-07]
- **L'agent a levé un doute que j'ai dû trancher moi-même.** Il signalait que
  l'oracle Java fait de Symbiotic Fungus, Extreme-Cold Fungus et Conserved Biome
  des **actions répétables**, là où mon contrat en faisait des effets de pose
  jouant une seule fois — en relevant que Birds, que le contrat donnait bien en
  action, a une signature Java identique. Il a suivi le contrat, comme demandé,
  et déclaré le conflit.
- **Chaîne de vérification par les scans mise en place** (nouvelle capacité) :
  découpe des 7 planches 4096×4000 de `data/scans/base/` en 490 vignettes, puis
  reconnaissance de texte (tesseract). Résultat sans ambiguïté : les trois cartes
  portent « **Action:** ». **Mon contrat était faux, l'agent avait raison.**
  Relevé au passage : Large Convoy dit « ANOTHER card » et non « ANY card »
  comme le prétendait le Java. [VÉRIFIÉ 25-07]
- **Round 2** : addendum écrit avec le texte des scans en source, 2 checks
  visibles mis à jour (ils testaient l'ancien comportement), resceau
  `aw seal --round 2`, agent repris avec son contexte. Livré : 4 entrées de
  table modifiées, zéro ligne de logique nouvelle (le vocabulaire `Action::Res`
  existait déjà). 231 tests verts. Audit **8/8**. [VÉRIFIÉ 25-07]
- **Un bogue préexistant attrapé par l'agent, que MON audit du lot 2 avait
  manqué.** `cards.json` contient des homonymes : la version rééquilibrée maison
  « Buffed » d'une carte porte le même nom que l'officielle, et se trouve parfois
  AVANT elle. La sonde et les tests cherchaient « la première carte de ce nom » →
  ils mesuraient la mauvaise. Cinq cartes du deck étaient concernées (Community
  Gardens 10 au lieu de 20, Drone Assisted Construction 7 au lieu de 15,
  Extreme-Cold Fungus 6 au lieu de 10, Farming Co-ops 7 au lieu de 15, Wood
  Burning Stoves 9 au lieu de 13). **Point rassurant vérifié par moi : les
  parties n'étaient pas faussées** — la pioche filtre sur `in_deck_v1`
  (`flow.rs:63`), donc elle a toujours distribué les vraies cartes. Le défaut
  était limité à l'outil d'audit et à deux tests du lot 2. [VÉRIFIÉ 25-07]
- **Contre-vérifications de la main avant promotion** : les 110 cartes des lots
  1-2 resondées → 0 divergence avec l'ancien moteur ; les 264 cartes du deck v1
  comparées → 5 différences, toutes des corrections du bogue ci-dessus ; graine
  inédite 828282, 1000 parties, 0 violation ; lecture du chemin critique (service
  unique avec assertions, `card_points` chemin unique, choix réellement demandés
  à la politique et non câblés). Verdict : **ok**, promu.
  231 tests verts, ~11 750 parties/s. [VÉRIFIÉ 25-07]
- **Reste non géré et déclaré** : l'amélioration des cartes Phase (2 cartes du
  lot la demandent), sautée sans compensation inventée et comptée par
  `phase_upgrades_skipped`. **Prochain chantier proposé : lot 4 — productions et
  points variables par tag**, ~24 cartes. [VÉRIFIÉ 25-07]

## 2026-07-25 (suite) — Lot 4 cadré et lancé : les productions qui dépendent des badges

- **Périmètre repris à la source, et le chiffre annoncé était encore faux — dans
  l'autre sens.** J'annonçais « ~24 cartes : productions ET points de victoire
  variables par badge ». Vérification faite : les **points** variables par badge
  sont **déjà** calculés depuis le lot 1 (`flow.rs:1551-1571`, `card_points` lit
  le champ `vp_dynamic` de `cards.json` et sait compter les badges Terre et
  Jupiter, les forêts, les cartes bleues et les cartes jouées). Il ne restait
  donc que les **productions**. Périmètre réel du lot : **17 cartes**.
  [VÉRIFIÉ 25-07]
- **La règle centrale tranchée au livret** : `docs/regles/livret-base.md:180` —
  « Certaines cartes de production augmentent leur production lorsque vous avez
  plus d'un badge spécifique. Vous devrez mettre à jour votre plateau Joueur
  chaque fois que vous jouez ce badge. » La production n'est donc PAS figée au
  moment où la carte est posée. C'est devenu l'interdit n° 1 du contrat, et le
  contrôle principal (une carte posée d'abord doit produire davantage quand un
  badge arrive après). [VÉRIFIÉ 25-07]
- **La chaîne de lecture des scans a servi dès la préparation** (elle avait été
  montée en urgence au lot 3) : *Windmills* (n° 206) porte « including this »
  que la base de données omet ; *Insects* (n° 152) compte les badges **Plante**,
  qu'elle ne possède pas ; *Zeppelins* (n° 208) compte les **jetons Forêt**, pas
  des badges. Conclusion versée au contrat : « including this » est un rappel de
  jeu, pas une règle à part — le calcul est uniforme (compter les badges au
  moment de la production). [VÉRIFIÉ 25-07 par lecture directe des images]
- **Découverte en préparant les contrôles** : le champ `vp` de la sonde ne
  rapporte que les points de la DERNIÈRE carte de la séquence (`probe.rs:534`),
  pas le total du joueur. Deux de mes contrôles cachés reposaient dessus à tort.
  Corrigé en demandant un champ nouveau `vp_total` (somme de `card_points` sur
  toutes les cartes en jeu), sans toucher au champ historique. [VÉRIFIÉ 25-07]
- **Mon erreur, deuxième lot de suite** : un contrôle caché attendait 0 MC pour
  *Miranda Resort* accompagnée de *Terraforming Ganymede*, en oubliant que
  Miranda porte elle-même un badge Terre. Attrapée par la vérification dans les
  deux sens, avant scellement. C'est la deuxième fois qu'un de mes contrôles
  cachés est faux : la vérification bidirectionnelle n'est pas une formalité.
  [VÉRIFIÉ 25-07]
- **Contrat scellé** : 17 cartes, vocabulaire imposé (`DerivedProd`,
  `ProdRes`, `ProdCount`), **service unique** `derived_production`, drapeau de
  sonde `--probe-produce` qui exécute la **vraie** phase de production, 5
  compteurs d'audit, 8 interdits, 5 exemples de contournement nommés.
  4 contrôles visibles rouges pour la bonne raison, verts sur un faux moteur
  simulant l'état-cible ; 4 contrôles cachés (3 rouges maintenant, 1 de
  non-régression vert dès le départ) ; **7 contre-tests de falsification, 7
  détectés**. Sous-agent Opus 4.8 lancé. [VÉRIFIÉ 25-07]

## 2026-07-25 (suite 2) — Lot 4 livré et promu, et deux décisions de périmètre d'Alexis

- **Incident de harnais** : le premier agent lancé sur le lot 4 s'est figé au
  démarrage, avant d'avoir lu son cahier des charges ; la surveillance l'a
  déclaré perdu après 600 secondes sans activité. `outputs/` était resté vide —
  aucun demi-travail à reprendre. Relancé à neuf. [VÉRIFIÉ 25-07]
- **Lot 4 livré** (240 000 unités de texte consommées par l'agent, ~38 minutes) :
  17 cartes, **271 tests verts** (231 + 40 neufs), 4/4 contrôles visibles.
  Vocabulaire `DerivedProd`/`ProdCount`/`ResearchBonus`/`Eff::TrPerTag`,
  services uniques `derived_production` (`flow.rs:750`) et `research_extra`
  (`flow.rs:786`), sonde `--probe-produce` qui exécute la VRAIE phase de
  production et relève la variation des compteurs, champ `vp_total`.
  [VÉRIFIÉ 25-07]
- **Audit** : 4/4 visibles, 3/4 cachés. L'unique échec porte sur le **seuil de
  vitesse** (8 000 parties/s) : mesuré entre 7 460 et 8 800 selon la charge de
  la machine. Mesures alternées avant/après par la main : aucune régression, le
  lot est marginalement plus rapide. Ce n'est pas un défaut de livraison, c'est
  mon seuil qui n'a plus de marge — à mesurer sur 10 000 parties dans les
  prochains contrats. [VÉRIFIÉ 25-07]
- **Contre-vérifications indépendantes avant promotion** : les 138 cartes des
  lots précédents resondées et comparées à l'ancien binaire → **0 divergence** ;
  les 155 cartes toutes reconnues ; graine inédite 424242 → 1000/1000, 0
  violation ; effets coupés strictement neutres ; lecture du chemin critique
  (rien n'est écrit sur les pistes de production, compteurs incrémentés au site
  du crédit, un seul chemin de calcul). Verdict **ok**, promu. [VÉRIFIÉ 25-07]
- **L'agent a signalé trois cartes hors contrat** portant le même bonus de
  recherche et restant inertes : *United Planetary Alliance* (11 MC, texte
  identique à Interplanetary Relations à 35 MC), *Interns*, *Extended
  Resources*. Il ne les a pas encodées (respect du périmètre) et les a nommées.
  À verser au prochain lot. [DÉCLARÉ par l'agent, plausible]
- **Une relecture adversariale menée par l'agent lui-même a trouvé deux trous de
  couverture réels** : casser la moitié « garde » du bonus de recherche, ou
  priver le joueur 1 de production dérivée, laissait 269 tests et 4 contrôles
  verts. Il a ajouté les tests manquants. C'est exactement le réflexe attendu.
  [DÉCLARÉ par l'agent, cohérent avec les tests livrés]
- **DÉCISION D'ALEXIS — la lecture du jeu physique par caméra est abandonnée.**
  À la place : un **jeu numérique avec interface propre** (glisser-déposer à la
  souris, ressenti d'un jeu de cartes en ligne du commerce), dans lequel l'IA
  jouera. Ordre retenu : moteur de règles → interface → IA. Consigné dans
  `docs/CTO_PROJET.md` et `docs/CTO_STATE.md`. [VÉRIFIÉ 25-07 — son message]
- **DÉCISION D'ALEXIS — dimensionnement des chantiers délégués** : viser
  ~200 000 unités de texte par workspace, regrouper plusieurs sujets si besoin,
  et paralléliser plusieurs workspaces quand les territoires sont disjoints. Le
  lot 4 a consommé 240 000 : le calibre est le bon, mes lots précédents étaient
  sous-dimensionnés. [VÉRIFIÉ 25-07 — son message]
- **Workspace `textes-cartes` scellé et lancé le 25-07** (autorisé par Alexis) :
  transcrire, **en lisant les images**, le texte imprimé des 222 cartes de la
  boîte de base présentes dans la pioche, avec leur **numéro imprimé** — une
  donnée qui n'existe nulle part ailleurs dans le projet, donc impossible à
  inventer sans lire la carte. Livrables : `textes-cartes.json`,
  `divergences.md` (le plus précieux : chaque écart avec la paraphrase et ce
  qu'il change dans la règle), `methode.md`. 4 contrôles visibles, 3 cachés,
  **6 contre-tests de falsification, 6 détectés** (recopie de la paraphrase,
  numéros séquentiels, numéro faux sur un témoin, texte de reconnaissance
  optique brut, textes vides, cartes de l'extension inventées). Motif : mes
  trois erreurs de contrat des lots 2, 3 et 4 viennent toutes de cette
  paraphrase. [VÉRIFIÉ 25-07]
- Trouvé en préparant ce contrat, et qui justifie le chantier à lui seul :
  *Hydro-Electric Energy* est imprimée « **Action:** Spend 1 MC to gain 2 heat »
  alors que la paraphrase dit « Spend 1 MC to get 2 heat » — le mot « Action: »
  fait la différence entre une capacité répétable chaque tour et un effet unique
  à la pose. Exactement l'erreur qui a coûté un second tour au lot 3.
  [VÉRIFIÉ 25-07 par lecture de l'image, carte n° 34]

## 2026-07-25 (suite) — Transcription des cartes : ~205/222, et un diagnostic d'infrastructure

- **Transcription lancée puis relancée trois fois.** Les lecteurs se faisaient
  couper par le garde-fou des 600 s de silence. Trois corrections successives :
  vignettes recompressées (2,3 Mo → 262 Ko), lots ramenés de 13 à 5 cartes,
  consigne d'annoncer chaque carte pendant le travail. [VÉRIFIÉ 25-07]
- **DIAGNOSTIC (le vrai)** : débit **montant mesuré à 200 Ko/s** contre
  1 900 Ko/s en descendant. Comme la conversation entière est renvoyée à chaque
  action, un agent qui accumule des images paie ~9 s d'envoi par action après
  5 cartes. **Faire tourner 8 agents-images en parallèle les rend 8× plus lents**
  (ils partagent le tuyau) : la parallélisation est contre-productive sur
  images, alors qu'elle reste bonne sur du texte. Réglages retenus pour la
  suite : 3 agents, 3 images chacun, vignettes à ~120 Ko (660 px q78, lisibilité
  vérifiée). [VÉRIFIÉ 25-07]
- **Apport d'Alexis** : un agent coupé se ranime avec « continue » et garde
  toute sa mémoire de travail (5 min au lieu de 25 pour refaire). Limite
  découverte : on ne peut ranimer que **ses propres** agents, pas ceux d'un
  sous-agent. [VÉRIFIÉ 25-07]
- **Erreur de lecture attrapée** : le badge **Espace** (soleil doré sur disque
  sombre) confondu avec **Énergie** (éclair blanc sur disque magenta) sur
  *Energy Subsidies*. Le lecteur s'est laissé guider par le titre de la carte.
  Les 25 cartes « Énergie » et 41 « Espace » sont à revérifier par comparaison
  de pictogramme à la fusion. [VÉRIFIÉ 25-07 par recadrage comparatif]
- **Les cartes Phase améliorées ne sont PAS dans les scans** : les 10 cellules
  correspondantes sont du noir pur (luminosité mesurée 0,0). Seules les photos
  d'Alexis pourront les fournir — elles passent de « souhaitable » à
  **bloquantes**. [VÉRIFIÉ 25-07]
- **Trouvaille majeure confirmée** : *Hydro-Electric Energy* (n° 34) est
  imprimée « **Action:** Spend 1 MC to gain 2 heat. *If you chose the action
  phase this round, gain 1 additional heat.* » — `cards.json` dit seulement
  « Spend 1 MC to get 2 heat ». Deux erreurs : le mot « Action: » (facteur dix
  sur la valeur de la carte) et toute la seconde phrase. [VÉRIFIÉ 25-07]
- **Trois cartes de la famille du lot 4 découvertes hors de mon recensement**
  (*Atmospheric Insulators*, *Worms*, *Satellite Farms*) : mon périmètre de 14
  cartes était incomplet. Compte exact à faire à la fusion. [VÉRIFIÉ 25-07]
- **Décision d'Alexis consignée** : après le moteur, deux chefs de projet en
  parallèle (interface, IA). Ma recommandation, non encore tranchée par lui :
  interface + IA-qui-calcule maintenant, IA-qui-apprend seulement quand la
  boîte est complète. [DÉCLARÉ]
- **Mes propres erreurs de la journée** : compte de cartes annoncé en dérive
  (199 annoncé, 189 réel) faute d'avoir déduplique ; consigne trop étroite sur
  la phase (une seule alors que beaucoup de cartes en portent plusieurs) ;
  aucun symbole prévu pour les prérequis d'oxygène et de température ;
  corporations annoncées en paysage alors qu'elles sont en portrait.


## 2026-07-26 — La transcription est finie, et elle condamne notre fichier de référence

Journée courte en actions, lourde en conséquences. Deux dépôts créés, la
transcription des cartes achevée et auditée, et la découverte que le fichier de
données sur lequel quatre lots du moteur ont été construits est gravement
infidèle au carton imprimé.

### Mise en ligne du projet

- **Il n'existait aucun dépôt distant.** Tout le projet ne vivait que sur le
  disque d'Alexis depuis le 23-07. Corrigé : **`github.com/Alexry375/Terra`**,
  privé. [VÉRIFIÉ 26-07]
- **Défaut ancien découvert à cette occasion** : `engine/target/` — 307 fichiers,
  94 Mo d'artefacts de compilation entièrement reconstructibles — était versionné
  depuis le premier commit. Retiré du suivi ET purgé de tout l'historique
  (`git filter-repo`). Le dépôt passe de **71 Mo à 19,4 Mo**, dont ~20 Mo de
  photos du livret conservées volontairement. Sans cette purge, chaque envoi
  aurait coûté 6 minutes à 200 Ko/s. Sauvegarde de l'ancien `.git` faite avant
  l'opération. [VÉRIFIÉ 26-07]
- **Second dépôt, décidé par Alexis** : `github.com/Alexry375/Terra-ateliers`,
  privé — 1 504 fichiers, 12 Mo. Il sauvegarde les 9 contrats scellés, les
  journaux de décisions des sous-agents et **les 65 fichiers de lots de
  transcription**, que `.git/info/exclude` (posé par l'outil `aw`) empêchait
  d'atteindre le dépôt principal. Motif : ce travail n'existait nulle part
  ailleurs que sur le disque local. [VÉRIFIÉ 26-07]

### Décisions d'Alexis

- **Ne PAS ouvrir les deux chantiers (interface, IA) en parallèle pour
  l'instant** — « ça va tout compliquer ». Ma recommandation était : oui pour
  l'interface et pour l'IA-qui-calcule, non pour l'IA-qui-apprend tant que le
  simulateur n'est pas fiable. Alexis va plus loin et repousse les deux.
  [VÉRIFIÉ 26-07]
- **Visuels des cartes : pas de collage du texte français sur le scan.** Retenu :
  vraie carte scannée + texte français dans un panneau à côté. [VÉRIFIÉ 26-07]

### Transcription : mon inventaire de reprise valait mieux que mon estimation

J'annonçais « une quinzaine de cartes restantes ». Mesure réelle : **9**.
Sur les 25 cartes que le comptage automatique déclarait absentes, **16 étaient
déjà lues** — c'est `cards.json` qui écrit leur nom de travers (*Toll Station*
lu « Tall Station », *Nitrophilic Moss* lu « Nitropholic », *United Nations Mars
Initiative* réduit à « Unmi »…). Leçon : un écart de nom n'est pas une carte
manquante, et mon estimation à la louche valait moins qu'une mesure de deux
minutes. [VÉRIFIÉ 26-07]

Réglages appliqués depuis le diagnostic du 25-07 : vignettes recompressées à
**85 Ko** (660 px, qualité 78), 3 lecteurs simultanés maximum, 3 images chacun.
Lisibilité vérifiée par la main avant lancement sur *Biothermal Power*.
**Aucun agent n'a calé.** Le diagnostic de débit était le bon.

### Trouvaille de méthode de l'agent

Pour le recontrôle des badges, il a monté **6 bandes gauches de cartes par
image** (33 Ko par carte au lieu de 85). Effet de bord décisif qu'il a lui-même
identifié : **les relecteurs ne voyaient plus le titre**, donc ne pouvaient plus
se laisser guider par lui — ce qui était exactement la cause de l'erreur
*Energy Subsidies*. 73 cartes revérifiées, 6 corrections. [DÉCLARÉ par l'agent,
résultat vérifié sur 2 cartes par la main]

### Audit : 6 cartes relues par ma main, 1 défaut trouvé

Verdict `aw report` : **partial**, promu après deux corrections de ma main.
Contrat intact, 3/4 contrôles visibles, 2/3 hold-outs.

- **DÉFAUT RÉEL — `Ganymede Shipyard` (n° 138) livrée avec `vp_printed: 2`.**
  Faux : l'encart gris à deux étoiles jaunes est un **savoir-faire de 2 titane**,
  pas des points de victoire. Preuve arithmétique sur tout le corpus : 1 étoile
  → « pay 3 MC less for [space] » (*Titanium Mine*, *Space Station*, *Vesta
  Shipyard*, *Asteroid Mining Consortium*) ; 2 étoiles → « pay 6 MC less »
  (*Asteroid Mining*, *Ganymede Shipyard*, *Io Mining Industries*) — soit 3 MC
  par titane, exactement le livret p. 18. Preuve visuelle : sur *Asteroid Mining*
  (n° 110) les deux marquages **coexistent et sont distincts** (encart gris à
  2 étoiles + pastille brune ronde « 2 » séparée) ; *Ganymede Shipyard* n'a
  aucune pastille brune. Corrigé à 0. **Conséquence : après correction, ZÉRO
  écart de points de victoire sur les 220 cartes — sur ce champ, `cards.json`
  est fiable.** [VÉRIFIÉ 26-07 par lecture de deux images]
- **Chiffre corrigé** : §A annonçait « 47 cartes » sans le mot-clé
  `Action:`/`Effect:` ; ma mesure donne 25 à 35 selon l'ancrage. Le fond reste
  massif : **64 cartes portent le mot-clé imprimé contre 29 dans la paraphrase**.
  [VÉRIFIÉ 26-07]
- Les 5 autres cartes relues (*Advanced Ecosystems* 11 champs sur 11,
  *Energy Subsidies*, *Surface Mines*, *Biothermal Power*, *Asteroid Mining*)
  sont exactes sur tous leurs champs. [VÉRIFIÉ 26-07]

### Ce que la transcription établit, et qui change le projet

- **`cards.json` perd le régime des cartes.** 64 cartes de la pioche portent
  « Action: » ou « Effect: » imprimé, 29 seulement dans la paraphrase. Une carte
  privée de ce mot devient un gain unique à la pose au lieu d'une capacité
  permanente ou répétable. C'est le motif n° 1 de `divergences.md`, et
  l'essentiel des 62 écarts de règle. [VÉRIFIÉ 26-07]
- **Quatre corporations sur douze sont fausses de bout en bout** : la paraphrase
  leur invente des revenus (« 1 Steel income ») non imprimés et omet des
  réductions imprimées. [DÉCLARÉ par l'agent, cohérent avec les scans lus]
- **Piège de modélisation signalé** : le nombre en haut d'une corporation est son
  **MC de départ**, pas un coût. `cards.json` le range dans `price`. Un moteur
  qui traiterait ce champ uniformément ferait *payer* 48 MC pour jouer CrediCor
  au lieu de lui en *donner* 48. [VÉRIFIÉ 26-07 par lecture des cartes]
- **16 cartes de la pioche écrivent « MC » en lettres CYRILLIQUES dans
  `cards.json`.** Identique à l'œil, différent pour une machine : toute recherche
  textuelle sur « MC » les rate en silence. [VÉRIFIÉ 26-07 par mesure
  indépendante — compte exact confirmé]
- **La boîte contient 220 cartes, pas 222.** Numéros imprimés 1 à 220, aucun
  trou, aucun doublon. *Microbiology Patents* et *Project Inspection* ne sont sur
  aucune planche : deux entrées probablement de trop dans la pioche v1 —
  **décision de conception à prendre**. Preuve que la numérotation n'est pas
  fabriquée : corrélation ordre-des-cellules / numéro-imprimé = **+0,114** sur
  P1. [VÉRIFIÉ 26-07]

### Mes propres erreurs de la journée

- **Deux de mes contrôles cachés étaient fautifs.** Le seuil « ≤ 5 noms inconnus
  du projet » a rejeté la livraison parce que **j'avais moi-même élargi le
  périmètre en cours de route** en demandant les cartes de phase et les
  corporations promo — un contrat scellé ne se complète pas par messages sans
  que ses contrôles suivent. Le critère « densité des numéros < 98 % »
  supposait qu'une numérotation sans trou trahit une fabrication : mauvaise
  heuristique, la vraie boîte numérote sans trou.
- **Mon hypothèse « Artificial Lake = Artificial Jungle mal orthographié » était
  fausse** : ce sont deux cartes distinctes. L'agent l'a vérifiée et corrigée —
  je lui avais explicitement demandé de ne pas me croire sur parole, ce qui a
  servi.
- **Estimation à la louche** (« une quinzaine de cartes ») là où une mesure de
  deux minutes donnait 9. Même défaut que le compte en dérive du 25-07.

### Restes et prochaine étape

- **Question posée à Alexis, en attente** : croiser les 62 écarts de règle avec
  ce que le moteur fait réellement, pour sortir la liste des cartes qu'il simule
  mal aujourd'hui. Les 4 lots du moteur ont encodé les cartes à la main depuis
  plusieurs sources — une partie de ces écarts est peut-être déjà rattrapée. Tant
  que ce croisement n'est pas fait, **on ne sait pas si le moteur est fiable**.
- Les **cartes Phase améliorées** restent absentes de toute source : seules les
  photos d'Alexis peuvent les fournir. Toujours bloquantes.
- Les `notes` de certaines cartes empilent deux lectures non réconciliées et
  peuvent se contredire (ex. *Advanced Ecosystems*) : les champs sont bons, les
  notes sont à lire avec prudence.

## 2026-07-27 — Le moteur est déclaré fiable, et le chantier des corporations est lancé

Deux temps. La nuit : la livraison de `moteur-verite-1`, auditée et promue —
c'est elle qui répond à la question qui bloquait tout depuis le 26-07. Le matin :
le cadrage du lot suivant, qui a fait tomber une erreur que personne ne
cherchait.

### `moteur-verite-1` — livré, audité OK, promu

Détail complet dans `docs/CTO_STATE.md` §« LE MOTEUR EST FIABLE (27-07) ».
En résumé : sur les 66 cartes dont le texte était trahi par `cards.json`,
**35 étaient encodées et 33 d'entre elles sont conformes au carton imprimé**.
Le régime `Action:` — le risque numéro un — **était déjà bon**, prouvé répétable
par le flux réel `play_round`. [VÉRIFIÉ 27-07]

Un seul défaut réel, sur *Viral Enhancers* et *Decomposers* : la variante
« … ou … » d'un effet déclenché n'était résolue **qu'une fois**, en suivant le
moteur Java plutôt que le livret. Corrigé, vérifié par ma main en comparant
l'ancien et le nouveau binaire côte à côte. **Cause profonde restée ouverte** :
la clause du livret p.9 (« condition remplie plusieurs fois → effet résolu
plusieurs fois ») est absente de `docs/regles/notes/regles-condensees.md`.
[VÉRIFIÉ 27-07]

**Mon contrôle caché était fautif, pas l'agent** : témoin choisi hors périmètre,
et verdict attendu trop grossier sur *Hydro-Electric Energy*. Vérification faite
à la source : l'agent avait mieux raisonné que moi. Deuxième journée de suite où
mes propres contrôles sont la partie faible de l'audit. [VÉRIFIÉ 27-07]

### Cadrage de `moteur-corporations-1` — et une erreur trouvée en chemin

Le lot suivant devait être « donner leurs effets aux 12 corporations ». En
mesurant l'état de départ, **le moteur en distribue 16**. [VÉRIFIÉ 27-07,
`engine/src/cards.rs:243` + comptage sur `data/cards.json`]

Les quatre intruses — *Apollo Industries*, *Exocorp*, *Hyperion Systems*,
*Sultira* — n'existent dans `textes-cartes.json` sous aucune orthographe, et
**toutes les quatre portent « Upgrade your phase N card »** : ce sont des
corporations de l'extension **Découverte**, marquées `in_deck_v1: true` à tort.
Leur pouvoir principal repose sur un mécanisme que le moteur saute
(`phase_upgrades_skipped` = 642 déclenchements sur 1 000 parties graine 2024).
Conséquence en partie : un joueur sur deux se voyait proposer une corporation
absente de la boîte, dont le pouvoir ne s'appliquait pas. [VÉRIFIÉ 27-07]

**Second défaut confirmé** : `engine/src/flow.rs:167-183` ne pose qu'`mc` et les
compteurs de badges. Les pistes de production fixes restent à zéro — *Ecoline*
(1 plante), *Helion* (3 chaleur) et *Thorgate* (1 chaleur) démarrent amputées de
leur production imprimée. [VÉRIFIÉ 27-07]

### Ce que le cadrage a établi de bon

**Le moteur est mieux bâti que je ne le disais hier.** J'avais annoncé que le lot
coûterait cher parce que « la structure n'existe pas ». C'est vrai pour la
structure `Corporation` (aucun champ d'effet, `cards.rs:143`), et **faux pour
tout le reste** : chaque règle a un point de calcul unique et documenté —
`card_discount` (`flow.rs:206`), `derived_production` (`flow.rs:750`),
`research_draw_keep` (`flow.rs:816`), `build_forest` (`flow.rs:1136`),
`build_card_with` (`flow.rs:853`). Les corporations n'ont qu'à y verser leur
contribution. Ces six points d'entrée sont inscrits nommément dans le contrat,
avec interdiction de recalculer quoi que ce soit ailleurs. [VÉRIFIÉ 27-07]

**Deux limites tranchées d'avance pour éviter la dérive** : *Phobolog* et
*Mining Guild* parlent de titane et d'acier, ressources non modélisées
(`titanium_capacity` initialisé à 0 en `state.rs:215`, jamais alimenté, unique
lecteur `flow.rs:1632`). Ordre donné : encoder ce qui est possible, déclarer le
reste, ne pas ouvrir le chantier des ressources. [VÉRIFIÉ 27-07]

### Contrat scellé

5 contrôles automatiques, **tous testés dans les deux sens** (ils refusent le
travail bâclé ET acceptent le travail correct), 3 vérifications cachées, dont
13 valeurs que j'ai calculées à la main depuis le texte imprimé. Copie du binaire
d'avant le lot gardée hors du workspace pour comparer.

**Deux de mes cinq contrôles étaient fautifs à la première écriture**, trouvés
par le test en sens vert : dans l'un, un bloc de texte écrasait l'entrée du
programme de lecture, qui ne voyait donc jamais la sortie à vérifier ; dans un
autre, une correction de ma part transformait la valeur `0` en `faux` (piège
classique du langage Python, où zéro et faux se confondent). Corrigés et
revalidés. **Le test en sens vert est ce qui les a trouvés — sans lui, l'agent
aurait buté sur mes erreurs, pas sur les siennes.**

### Décision d'Alexis

- **On jouera avec l'extension Découverte.** Les quatre corporations écartées par
  ce lot devront donc revenir, une fois le mécanisme des améliorations de phase
  modélisé. Les photos des cartes Phases améliorées sont annoncées pour
  aujourd'hui. [VÉRIFIÉ 27-07]

### Restes

- **Cause profonde non traitée** : compléter `docs/regles/notes/regles-condensees.md`
  avec la clause du livret p.9.
- **`probe.rs` ment sur `paid[]`** quand une réduction se paie en microbes —
  affecte la fiabilité de mes propres audits.
- *Microbiology Patents*, *Project Inspection*, *Oxidation Byproducts* : trois
  cartes de la pioche v1 qui n'existent sur aucune planche. Décision de
  conception toujours en attente.

## 2026-07-27 (suite) — Les corporations appliquent leurs pouvoirs

`moteur-corporations-1` livré, audité **ok**, promu. Détail complet dans
`docs/CTO_STATE.md` §« LES CORPORATIONS SONT VIVANTES ». L'agent a calé une fois
en cours de route (arrêt technique de l'outil, deuxième occurrence après le
chantier de transcription) ; relancé depuis son §Reprise, il a repris sans perte.

### Résultat

**10 corporations `ENCODÉE`, 2 `PARTIELLE`** (titane et acier, cadrages que
j'avais tranchés avant le lot). Mesuré par ma main après promotion : **317 tests
verts**, 1 000 parties sans violation ni troncature, empreinte identique sur deux
exécutions, débit inchangé (7 404 à 8 422 parties/s contre 7 400 à 8 900 avant).
[VÉRIFIÉ 27-07]

**Preuve d'exécution en partie réelle**, pas seulement en sonde : 4 compteurs
neufs relevés sur 1 000 parties — chaleur d'*Helion* employée comme monnaie
5 510 fois, remise de forêt d'*Ecoline* 883 fois, pas de terraformation acheté
d'*Unmi* 797 fois, TR déclenché de *Saturn Systems* 242 fois. Et
`research_extra_draws` passe de 1 293 à 4 266 (*Tharsis Republic* en phase V).
[VÉRIFIÉ 27-07]

### Trois choses que l'agent a mieux faites que ce que je demandais

1. **Il a refusé ma solution pour la pioche.** Je proposais d'exclure les 4
   corporations Découverte par leur nom. Son argument : un filtre négatif ne dit
   pas quoi faire quand l'extension arrivera. Il a retourné le critère — une
   table déclarée des 12 planches réelles, dont le chargement ne retient que ce
   qui y figure. **Ajouter les 4 entrées les fera revenir par le même chemin.**
2. **Il a commandé sa propre relecture adversariale** (exigée par la procédure
   des workspaces pour les tâches longues) et en a **corrigé** les 4 trouvailles
   au lieu de les déclarer. La plus importante : le « may » d'*Helion* était figé
   en convention codée, donc jamais apprenable par une IA. C'était **exactement**
   la réserve que j'avais notée au lancement, et il l'a levée avant moi.
   `flow.rs:1139` offre désormais le choix par `Policy::choose_option`.
3. **Un bug trouvé par exécution, pas par lecture** : avec *Helion*, la
   conversion chaleur → argent pouvait consommer la chaleur qu'un prérequis
   « Requires you to spend N heat » engageait à dépenser à la pose. Il l'a
   découvert en lançant 50 parties, et corrigé aux deux endroits à la fois
   (disponibilité et paiement) pour qu'ils ne puissent pas diverger.

### Mes propres défauts, encore

- **Mon contrat exigeait une preuve par sonde, mais la sonde que j'ai imposée
  n'exécute ni la phase III ni la phase V.** La forêt d'*Ecoline* et le +1/+1 de
  *Tharsis* ne pouvaient donc pas être prouvés comme je le demandais. L'agent a
  fourni mieux (partie réelle scriptée + compteurs) et l'a déclaré au lieu de le
  cacher. **L'interface était mal conçue, pas la livraison.**
- **Mon contrôle caché n° 2 a échoué à tort** : il exigeait des sorties de sonde
  analysables collées au rapport, or l'agent les a abrégées pour la lisibilité.
  Vérifié à la main : les 12 corporations sondées existent et se rejouent.
  Troisième journée de suite où un de mes contrôles est la partie faible.

### Ce que j'ai vérifié de ma propre main

Outre les mesures ci-dessus : le MC de départ est **assigné** (`flow.rs:204`),
donc donné et non payé — le piège signalé le 26-07 n'existe pas. Cinq fonctions
de test existantes modifiées, la limite contractuelle exacte, aucune supprimée.
Et la réserve de l'agent sur *Inventrix* est exacte : sur les 155 entrées de la
table, les 3 cartes citant à la fois température et oxygène (*Regolith Eaters*,
*Small Animals*, *Herbivores*) ne le font que dans leurs **effets**, jamais dans
leurs prérequis — l'écart est inobservable. [VÉRIFIÉ 27-07]

### Prochaine étape

**Découverte devient le chantier principal**, Alexis ayant confirmé qu'on joue
avec. Il reste bloqué sur une seule chose : **les photos des cartes Phases
améliorées**, absentes de toute source depuis le 25-07.

## 2026-07-27 (suite 2) — L'extension Découverte transcrite, la pioche assainie, et un chiffre faux que j'avais scellé deux fois

### Transcriptions (sources physiques, promues dans `data/cartes-imprimees/`)

- **4 corporations Découverte** lues sur scan : Apollo Industries (33 MC, espace,
  améliore II), Exocorp (26, science, V), Hyperion Systems (30, Terre, III),
  Sultira (38, énergie, I). Concordance parfaite avec `cards.json` **sauf** la
  clause « y compris celui-ci » de Sultira, absente de `cards.json` : le carton
  fait foi (2 chaleurs dès la mise en place). [VÉRIFIÉ 27-07]
- **38 cartes Projet Découverte** (`D05`–`D42`), 37 photographiées. `D37` manque
  au scan ; par élimination contre les 38 entrées `box: discovery` de
  `cards.json`, ce serait `Perfluorocarbon Production`. **Déduction, pas
  vérification** — la source d'élimination est celle-là même qui a inventé deux
  cartes fantômes. [DÉCLARÉ, à confirmer par photo]
- **11 objectifs et 7 récompenses** transcrits depuis les photos d'Alexis.
  [VÉRIFIÉ 27-07]
- **Carte au coût 28** (*Jardins Hydroponiques*) : l'exemplaire d'Alexis est
  modifié à la main (« une » recouvert d'un « 2 »). Décision d'Alexis : on garde
  le TEXTE D'ORIGINE, une seule amélioration. [VÉRIFIÉ — son message du jour]
- Mon oubli, signalé par Alexis : `docs/regles/livret-decouverte.md` existait
  depuis le 24-07 et je ne l'avais pas relu avant de parler de l'extension.

### Chantier `moteur-boites-1` — cadré, scellé, livré, audité OK, promu

- **Résultat** : la pioche est dérivée des planches physiques
  (`textes-cartes.json`), plus du drapeau `in_deck_v1` hérité du portage Java.
  Point unique de composition : `engine/src/boites.rs`. Option
  `--boites base|promo|decouverte`, recensement `--dump-deck`, compteur
  `cards_effects_unhandled`. **208/12, 219/12, 246/16, 257/16** — les quatre
  nombres exacts. **336 tests verts**, 6/6 contrôles, 3/3 hold-outs.
  [VÉRIFIÉ 27-07 par ma main après promotion]
- Le moteur distribuait **248 cartes au lieu de 208** : 38 de Découverte aux
  pouvoirs sautés (599 fois sur 1000 parties) et 2 inexistantes
  (*Microbiology Patents*, *Project Inspection*). [VÉRIFIÉ 27-07]

### L'erreur de la journée, et elle est à moi

- **Mon contrat affirmait que 7 cartes de la boîte de base ont un pouvoir non
  appliqué. Il y en a 62.** L'agent l'a mesuré, m'a contredit, et l'a prouvé
  trois fois (comptage de `effects::LOT1`, recherche dans les sources, sonde sur
  *Power Plant* dont la production imprimée donne un delta nul). **Re-vérifié
  par ma main, indépendamment, sur le moteur d'AVANT le lot : 146 des 208 noms
  de base figurent dans `src/`, donc 62 muettes.** [VÉRIFIÉ 27-07]
- **Cause** : le « 7 » est le sous-total `ABSENT` de
  `docs/cartes/moteur-vs-imprime.md`, qui n'échantillonne que **66 des 208**
  cartes. Le contrat précédent (`moteur-corporations-1`) citait encore
  correctement « les 7 déclarées ABSENT **par** ce rapport » ; dans le mien, le
  qualificatif de périmètre a sauté et un sous-total de tri est devenu une
  mesure de couverture.
- **Deuxième contrat de suite où je scelle une exigence fausse** (au lot
  précédent : une preuve que la sonde imposée ne pouvait pas produire). Leçon :
  **ne jamais reprendre un nombre d'un document sans re-lire sa phrase de
  périmètre.** Mon propre hold-out 02 portait la preuve — il mesurait
  « 146 / 208 » — et je ne l'ai lu que comme une référence anti-recopie, sans
  faire la soustraction.
- **Ce qui a sauvé la situation** : le contrat exigeait la vérité (I4, « aucun
  pouvoir sauté en silence ») plutôt que la conformité aux nombres. L'agent a
  refusé les deux options complaisantes, levé l'ASK 3 prévu, et documenté trois
  options chiffrées. C'est la deuxième fois qu'un agent corrige une erreur
  factuelle de mon contrat : la clause « déclare et demande » vaut son coût.
- **Corrections apportées par ma main** : contrôles 02 et 03 réécrits sur la
  mesure réelle et **retestés dans les deux sens** (la clause 02 ne compare plus
  à une liste écrite à la main, elle DÉRIVE les muettes de l'absence du nom dans
  le code source, et exige l'équivalence exacte — plus sévère que l'ancienne) ;
  avertissement de périmètre ajouté en tête de `moteur-vs-imprime.md`.
  [VÉRIFIÉ 27-07]

### Deux décisions d'Alexis

- **Les cartes promotionnelles ne sont PAS possédées.** J'avais affirmé le
  contraire en séance ; Alexis a demandé d'où je le tenais. Vérification : les
  planches `PROMO` (11 projets) et `PROMOCORP` (6 corporations) viennent de
  l'adaptation Tabletop Simulator, pas de sa boîte, et forment exactement le
  pack Kickstarter 2021 — dont l'absence était **déjà tranchée et écrite le
  24-07**. J'ai parlé de mémoire au lieu de relire la carte d'état. Commentaire
  inverse corrigé dans `boites.rs`. [VÉRIFIÉ 27-07]
- **Découverte se joue EN ENTIER** — les quatre modules (Objectifs, Récompenses,
  cartes Phase améliorées, badges jokers). Configuration cible de
  l'entraînement : **`--boites base,decouverte`, soit 246 projets et
  16 corporations.** [VÉRIFIÉ — son message du jour]

### Conséquence sur l'ordre des chantiers

L'IA apprend en jouant contre elle-même. Si 62 cartes sur 208 ne font rien, elle
apprendra qu'elles sont mauvaises et les évitera dans la vraie partie. **Finir la
boîte de base passe donc avant d'implanter les effets de Découverte** — ce sont
plusieurs lots, certaines des 62 réclamant des mécanismes absents (acier et
titane comme monnaies, actions standard, cartes jouées en supplément).
Recommandation posée à Alexis, réponse en attente.

## 2026-07-28 → 2026-07-31 — Découverte complète, le moteur observable, l'écran de jeu, les choix qui parlent

*[RECONSTITUÉ le 02-08 à partir des enregistrements du dépôt : ces journées
n'avaient pas été écrites au jour le jour. Les faits cités sont ceux des messages
d'enregistrement, non re-vérifiés un par un.]*

- `decouverte-phases`, `decouverte-projets`, `decouverte-jokers-corpos` :
  livrés, audités, promus. Tout le contenu imprimé est encodé — 246 projets,
  16 corporations, 793 tests. Deux erreurs de mes propres contrôles cachés et
  une erreur de mon contrat relevées à cette occasion. [DÉCLARÉ — messages
  `6ce5b26`, `42d7c65`, `09bd20f`]
- `moteur-observe` : le moteur est enfin observable de l'extérieur — 33 sites sur
  33, 810 tests, trois empreintes de référence inchangées. C'est ce qui a rendu
  l'écran possible. [DÉCLARÉ — `0f0e0f1`]
- `harnais-images` : les 262 cartes ont leur image, et les 262 badges ont été
  confrontés aux images imprimées, sans une seule erreur dans `data/cards.json`.
  [DÉCLARÉ — `c0699c1`, `a2f33e4`]
- Le moteur tourne dans le navigateur (`5446886`), puis une partie à deux sur le
  même écran devient jouable, avec 3115 valeurs affichées confrontées à l'état du
  moteur (`6f6f2c9`).
- Le remplacement des cartes de départ devient partiel — de 0 à 8, au lieu de
  tout ou rien (`973a656`, `fccef86`), et chaque alternative proposée dit enfin
  de quoi elle parle (`73cb9cf`).

## 2026-08-01 — Deuxième série de retours d'Alexis, une vraie erreur de règle, et le décor choisi

### Ce qui a été corrigé et enregistré

- **Une carte bleue sans action n'est plus proposée à l'activation**
  (`4eb57fe`). C'est une vraie erreur de règle, relevée par Alexis à l'écran sur
  *United Planetary Alliance*. Le moteur filtrait sur la COULEUR ; il filtre
  désormais sur l'existence d'une action (`flow::activable_blue`). Le joueur
  perdait son unique activation de la manche, et l'intelligence artificielle à
  venir aurait dû apprendre à éviter un coup qui n'existe pas.
  **Les trois empreintes de référence ont bougé** — recalculées et réécrites :
  `--seed 2024 --boites base` vaut maintenant `c1c52fcbe4e057b0`.
  [VÉRIFIÉ 01-08]
- **La loupe ne se pose plus sur une carte déjà lisible** (`9a95718`) : en
  dessous de 80 % de la largeur de la loupe, agrandir apporte quelque chose ;
  au-dessus, on posait un doublon par-dessus le jeu. Mesuré dans les deux sens :
  corporation à 300 px, pas de loupe ; carte de main à 159 px, loupe.
  [VÉRIFIÉ 01-08]
- **Le décor du plateau est choisi.** Six propositions montrées avec de vraies
  cartes (`312861b`) ; Alexis a tranché : sol martien de Granicus Valles, voile
  sombre léger (`f0a710d`). Domaine public NASA/JPL/University of Arizona, avec
  mention obligatoire à l'écran.

### Mes erreurs de la journée

- **J'ai affirmé qu'un serveur servait le projet sans le vérifier.** Il servait
  le brouillon d'un ancien chantier, vieux de la veille. Alexis a commenté
  pendant un moment une version périmée. Leçon retenue et appliquée depuis :
  vérifier ce qu'un serveur SERT, pas seulement qu'il répond.
- **J'ai eu tort sur les tuiles Océan.** J'avais objecté que choisir sa tuile
  changerait les probabilités ; Alexis a demandé en quoi choisir FACE CACHÉE les
  changerait. Il a raison : neuf tuiles indiscernables donnent la même loi.
- **J'avais tort sur la vente de cartes** : le livret dit bien « à tout moment »
  (l. 96 et l. 348).
- **Mon relevé des cartes bleues sans action était faux** : « au moins huit ».
  Refait le 02-08 en interrogeant le moteur : **62 sur 101**.

### Ce qui a été préparé sans être lancé

Le chantier `cadre-de-jeu` — un seul point de vue, l'adversaire opaque : contrat
écrit, six contrôles visibles tous vérifiés rouges, deux contrôles cachés écrits
et éprouvés. Lancé la nuit suivante.

## 2026-08-02 (nuit) — Le cadre de jeu livré, deux fuites réelles trouvées, et mon propre contrôle pris en défaut

Alexis est allé se coucher en me laissant sceller le contrat, lancer l'agent et
auditer. Voici ce qui s'est passé.

### Le chantier

`cadre-de-jeu` scellé, lancé, livré en une passe : six contrôles visibles verts,
le moteur intact d'un octet, le pont jamais modifié. Puis deux corrections
ciblées, demandées après audit. **Bilan final : 6/6 visibles, 4/4 cachés,
promu.** [VÉRIFIÉ 02-08]

### Mon contrôle caché était faux — et c'est le fait le plus important

Le contrôle censé prouver que la main de l'adversaire ne fuit nulle part
rejouait la partie **deux fois séparément**, en espérant que ce soit la même.
Ce n'en était pas une : dans la page, l'adversaire est joué par un programme du
navigateur, et le siège regardé ne répond qu'à ses propres questions. **159
décisions dans la page contre 345 dans ma référence, 135 désaccords de forme.**

Il cherchait donc les cartes d'une main qui n'existait pas. Sans valeur dans les
deux sens — et il avait pourtant servi, la veille, à déclarer « 852 fuites »
sur l'ancien écran. Ce chiffre ne valait rien.

Refait : on joue d'abord en relevant tout ce que la page livre, le moteur rejoue
ensuite la même partie avec les réponses réellement données. Éprouvé dans les
deux sens — vert sur la livraison, rouge sur deux versions que j'ai
volontairement abîmées (513 et 762 fautes).

**Leçon à appliquer partout : un contrôle qui reconstruit une référence par un
chemin parallèle doit d'abord prouver que les deux chemins produisent le même
objet.**

### Deux fuites réelles, et la première vient de mon contrat

1. **La phase choisie**, 43 planifications sur 43. Vu du siège interrogé en
   second, la barre d'équipage montrait la carte que l'adversaire venait de
   poser face cachée. L'agent l'avait **déclaré lui-même** en disant que mon
   contrôle 04 l'y obligeait : il avait raison sur les deux points. Contrainte
   levée par écrit, correction demandée, vérifiée par un contrôle caché écrit
   pour l'occasion.
2. **La corporation**, une fois par partie. Trouvée en cherchant, après la
   première, s'il existait d'autres moments de la partie où le moteur révèle
   avant l'heure. Le livret tranche (l. 211 et l. 215).

### Le reste

- Dette du moteur réglée : les tuiles Océan portent leur identité, l'état dit
  lesquelles sont retournées. 821 tests verts, empreintes inchangées.
- Comptage des cartes bleues sans action refait en interrogeant le moteur :
  **62 sur 101**, et non « au moins huit ».
- Le journal (cinq jours de retard) et `engine/ARCHITECTURE.md` remis à jour.

### Ce qui attend Alexis

La couverture de boîte sous droit d'auteur, la portée de la vente de cartes, et
le lancement du chantier des jauges en arc de cercle.

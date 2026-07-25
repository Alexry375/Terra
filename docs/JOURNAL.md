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

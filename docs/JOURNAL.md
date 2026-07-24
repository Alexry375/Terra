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

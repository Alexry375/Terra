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

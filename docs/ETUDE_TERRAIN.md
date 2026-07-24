# Étude du terrain — simulateurs, données de cartes, IA, faisabilité (2026-07-23)

> Produite par le harnais de recherche approfondie (106 agents, 23 sources lues,
> 110 affirmations extraites, 25 vérifiées contradictoirement : 22 confirmées,
> 3 réfutées). Chaque affirmation confirmée a survécu à 3 vérificateurs
> indépendants chargés de la réfuter.

## Synthèse

Le paysage libre pour *Ares Expedition* (le jeu de cartes) se réduit à **deux
dépôts dédiés** :

| Dépôt | Verdict |
|---|---|
| `nikitinalexx/ares-expedition` | **Le candidat.** GPL-3.0, Java Spring Boot + Angular + PostgreSQL, ~465 commits (dernier : 2025-12-27), déployé et jouable en ligne (Heroku), couvre **Discovery** (objectifs, récompenses, améliorations de phases, corporations renforcées) plus Crisis et Infrastructure, et contient déjà un **embryon d'IA adverse** (paquet `services/ai/`, branche `ai5`, collecte de jeux de données, commits fin 2023). |
| `sebwieser/ares-expedition` | À écarter : Python/Flask, abandonné depuis juillet 2022, 51 commits, Discovery non documentée. |

Le célèbre `terraforming-mars/terraforming-mars` implémente **le jeu de plateau
de base**, pas Ares Expedition — hors périmètre sauf comme référence
d'architecture.

**Aucune base de cartes JSON couvrant base + Discovery n'existe** :
`alrusdi/tmae-content` est un tableur Excel figé en juillet 2021, antérieur à
Discovery. Les données de cartes devront être **extraites du code Java de
nikitinalexx** ou ressaisies (listes communautaires BGG en appui).

## Recommandation (confiance moyenne — jugement de synthèse, prémisses vérifiées)

**Réutiliser sans reconstruire de zéro, mais en deux moteurs :**

1. `nikitinalexx/ares-expedition` comme **moteur de règles de référence** et
   **source de données de cartes** (seul projet couvrant Discovery, licence
   GPL-3.0 compatible).
2. Construire en parallèle un **simulateur léger et très rapide dédié à
   l'entraînement** : le moteur Java web n'est pas conçu pour des millions de
   parties d'auto-apprentissage.

## Côté IA (vérifié, avec 3 affirmations réfutées — voir plus bas)

- **ReBeL** (Facebook/Meta, poker) n'est PAS directement applicable : ses
  réseaux croissent linéairement avec le nombre d'états d'information par état
  public — intraitable avec des mains cachées tirées d'un grand paquet
  (limitation reconnue par les auteurs, arXiv 2007.13544).
- Le **choix simultané de phases** correspond au cadre « Simultaneous
  AlphaZero » (arXiv 2512.12486, préprint) — transposition **non triviale**
  (Ares Expedition est stochastique et multijoueur).
- **AlphaZe\*\*** (AlphaZero + information imparfaite) perd >80 % contre les
  méthodes spécialisées sur Barrage Stratego ; entraînement de référence :
  ~16 jours sur 3 V100 — bien au-delà d'une RTX 3060 seule. Plaide pour un
  moteur de simulation très rapide et/ou de la location ponctuelle de GPU.
- **Précédent le plus pertinent** : l'IA de *Race for the Galaxy*
  (`bnordli/rftg`, GPLv2, base de Keldon Jones) — jeu aux mécaniques quasi
  identiques (main cachée, choix simultané et secret de phases avec bonus au
  sélectionneur). Preuve qu'une IA forte est atteignable avec des réseaux
  légers auto-entraînés, avant l'ère GPU. Modèle juridique aussi : code libre,
  visuels de cartes sous droits d'auteur séparés.

## Périmètre Discovery à couvrir (vérifié sur FryxGames + livret officiel)

38 cartes projet, 40 cartes de phase améliorées (copies d'un petit ensemble ;
2 options d'amélioration divergentes par phase), 4 corporations, 18 tuiles
Objectifs/Récompenses. Discovery réintroduit Objectifs et Récompenses et ajoute
le remplacement d'une carte de phase par une version améliorée.

## Légal (principes généraux, PAS d'analyse juridique sourcée — voir réserves)

La GPL couvre le code seulement ; textes et visuels des cartes restent la
propriété de FryxGames. Usage privé acceptable, diffusion des visuels à
proscrire. Suivre le modèle rftg.

## Affirmations réfutées (à ne PAS croire)

1. « ReBeL est l'approche de référence directement applicable » — réfuté 0-3.
2. « Le solveur de jeux matriciels de Simultaneous AlphaZero est directement
   transposable » — réfuté 1-2.
3. « AlphaZe** bat les bots heuristiques de Stratego à 68-74 % » — réfuté 0-3.

## Réserves de l'étude

1. `nikitinalexx` est **dormant depuis ~7 mois** — « récemment maintenu », pas
   « en développement continu ».
2. « Jouable » vérifié par réponse du site, **pas par une partie complète** ; la
   fidélité des règles n'a pas été auditée carte par carte.
3. Le volet **implémentations officielles** (application Asmodee, Board Game
   Arena, module Tabletop Simulator) n'a produit **aucune affirmation
   vérifiée** — reste à couvrir. (Repéré en recherche, non vérifié : module
   Tabletop Simulator de « Rabid Pickle », 2021, sans automatisation des
   règles ; un second module scripté existe.)
4. Le volet légal repose sur des principes généraux, pas sur du droit
   français/européen sourcé.

## Questions ouvertes (prochaines études possibles)

1. Implémentations officielles/semi-officielles : que contiennent-elles et
   qu'est-il extractible légalement ?
2. Fidélité réelle des règles du moteur nikitinalexx + coût de conversion de
   ses données de cartes Java en JSON.
3. Architecture d'IA hybride entraînable sur RTX 3060 + budget location GPU
   pour un niveau compétitif.
4. Statut juridique en droit français/européen de la reproduction des textes
   d'effets de cartes.

## Sources principales

- https://github.com/nikitinalexx/ares-expedition (primaire)
- https://expedition-ares-fe.herokuapp.com/ (instance jouable)
- https://github.com/sebwieser/ares-expedition (primaire)
- https://github.com/terraforming-mars/terraforming-mars (primaire, hors périmètre)
- https://github.com/alrusdi/tmae-content (données figées 2021)
- https://github.com/bnordli/rftg (précédent Race for the Galaxy)
- https://fryxgames.se/product/ares-expedition-discovery-expansion/ + livret PDF officiel
- arXiv 2007.13544 (ReBeL), arXiv 2512.12486 (Simultaneous AlphaZero, préprint),
  Frontiers in AI 2023 (AlphaZe**)
- Listes de cartes communautaires BGG : filepage/236719, filepage/226524

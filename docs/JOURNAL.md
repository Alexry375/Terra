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
- Limites honnêtes de l'étude : volet « implémentations officielles » (appli
  Asmodee, Board Game Arena, Tabletop Simulator) non couvert par des
  affirmations vérifiées ; fidélité des règles de nikitinalexx non auditée
  carte par carte ; volet légal non sourcé en droit français. [VÉRIFIÉ 23-07]

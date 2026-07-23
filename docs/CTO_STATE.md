# Carte d'état — Projet Terra

> Source de vérité du projet. Ancrée au code (`fichier:ligne`) dès qu'il y aura du
> code. [VÉRIFIÉ JJ-MM] = relu à la source ce jour-là. [DÉCLARÉ] = non re-vérifié.

Dernière mise à jour : 2026-07-23

## Ce qui marche

- Rien encore : le dépôt vient d'être créé, il ne contient que les documents de
  pilotage. [VÉRIFIÉ 23-07]

## Décisions ouvertes (attendent Alexis ou une étude)

1. **Choix du moteur de règles** : réutiliser un simulateur existant (à auditer)
   ou écrire le nôtre. Aucune étude faite. [DÉCLARÉ]
2. **Approche d'apprentissage** (auto-apprentissage par parties contre soi-même
   façon AlphaZero, ou autre). Aucune étude faite. [DÉCLARÉ]
3. **Entraînement local (RTX 3060) ou machines louées en ligne.** [DÉCLARÉ]
4. **Interfaces de jeu** : en ligne, et/ou plateau physique par caméra. Reporté à
   après le moteur et l'IA. [DÉCLARÉ]

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

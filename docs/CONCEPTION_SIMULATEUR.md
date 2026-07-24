# Conception du simulateur rapide « moteur d'entraînement » (v1)

> Statut : PROPOSITION du CTO, 2026-07-23 — en attente de validation d'Alexis
> sur les points marqués ❓. Les choix techniques sont des recommandations
> argumentées ; les prémisses factuelles viennent de `docs/ETUDE_TERRAIN.md` et
> de l'audit `audit-nikitinalexx` (voir `docs/CTO_STATE.md` §Acquis).

## Rôle dans le projet

Deuxième moteur de la stratégie validée le 23-07 : un simulateur minimaliste et
très rapide, sans interface, dont le seul client est l'entraînement de l'IA
(des millions de parties jouées contre elle-même). Le moteur Java de
nikitinalexx reste l'**oracle de référence** : même partie rejouée sur les deux
moteurs → même résultat, sinon bogue chez nous (ou chez lui — 2 écarts déjà
connus).

## Choix recommandés

1. **Langage : Rust, avec liaison Python (PyO3/maturin).**
   Vitesse de C, sûreté mémoire (un moteur de règles est un nid à erreurs
   d'état), parallélisme sans peur, et le monde de l'apprentissage
   (PyTorch/JAX) reste accessible depuis Python via la liaison. Alternative
   écartée : C++ (même vitesse, plus dangereux) ; Python pur (50-100× trop
   lent pour le self-play).

2. **Cartes = données + petits effets codés.**
   Base de cartes générée depuis `cards.json` (audit), **ré-étiquetée par boîte
   d'origine** (base / tutoriel-étoilées / Discovery / promo / non-officiel) et
   croisée avec `Mylaana/AresExpedition/data/cards_data.json` pour détecter les
   divergences. Les effets des cartes : petites fonctions Rust par carte (ou
   par motif d'effet), écrites par lots et **testées carte par carte contre
   l'oracle Java**.

3. **État de jeu compact et copiable** (tableaux fixes, pas d'allocations en
   cours de partie) : indispensable pour la recherche arborescente (des
   milliers de copies d'état par décision). Information cachée modélisée
   proprement dès le départ : mains, pioche mélangée, vues par joueur.

4. **Vitesse cible mesurable : ≥ 10 000 parties complètes/seconde/cœur** en
   politique aléatoire (point de comparaison : le moteur Java en joue ~une
   poignée par seconde via REST). C'est LA métrique du chantier — elle
   conditionne tout l'entraînement sur RTX 3060 ou sans carte graphique.

5. **Vérification à trois étages** : (a) tests unitaires par carte ;
   (b) parties croisées scriptées contre l'oracle Java via son API REST ;
   (c) invariants globaux (conservation des ressources, bornes des paramètres
   globaux, fin de partie correcte).

## Périmètre v1 (TRANCHÉ par Alexis le 23-07)

- **2 joueurs — sur tout le projet** (« on jouera toujours à 2 joueurs »).
  Le multijoueur 3-4 sort du périmètre, pas seulement de la v1.
- Boîtes : **base + Découverte complète** (objectifs, récompenses,
  améliorations de phases, corporations). **Cartes promo : EXCLUES de la
  pioche** (Alexis ne les possède pas) mais conservées désactivées dans les
  données. **Cartes étoilées tutoriel : INCLUSES**, mélangées au reste.
- Important : *Oxidation Byproducts* (officielle Discovery, absente du deck du
  moteur Java pour cause de bogue) DOIT être dans notre pioche — la boîte
  physique d'Alexis la contient.
- **Règles maison d'Alexis incluses d'emblée** : mulligan des 8 cartes projets
  (tout ou rien) et mulligan des 2 corporations (les 2 ou aucune, avant les
  cartes projets, choix final après). Ce sont des règles de départ de partie,
  triviales à implémenter, mais l'IA doit apprendre AVEC.
- Hors périmètre v1 : mode solo/Crisis, Infrastructure, cartes « buffed »,
  toute interface graphique.

## Découpage en chantiers (workspaces à contrats scellés)

1. **`retag-cartes`** : ré-étiquetage de `cards.json` par boîte d'origine +
   croisement avec la liste Mylaana + repérage des cartes étoilées tutoriel.
   Petit chantier, débloque tout.
2. **`moteur-squelette`** : état de jeu, phases, production, fin de partie,
   mulligans maison — sans les effets de cartes uniques. Contrôles : invariants
   + parties aléatoires qui se terminent.
3. **`moteur-cartes-N`** (série) : effets des cartes par lots de ~50, chaque
   lot verrouillé par tests contre l'oracle Java.
4. **`banc-vitesse`** : mesure officielle parties/seconde, profilage.

## Décisions en attente d'Alexis

Aucune — les trois questions de périmètre ont été tranchées le 23-07 (voir
§Périmètre v1).

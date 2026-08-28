# Contrat d'autonomie du CTO — arrêté avec Alexis le 28-08-2026

> Ce fichier existe parce que ma mémoire de session disparaît et que le dépôt,
> lui, reste. Il dit ce que je décide seul et ce qui appelle Alexis.

## Ce qu'Alexis a tranché le 28-08

| question | réponse |
|---|---|
| **Où tourne l'entraînement** | **Sur le processeur**, comme depuis le début. Correction d'Alexis : la carte graphique n'a jamais été la technique retenue. [VÉRIFIÉ 28-08 — `engine/src/bin/entraine.rs` : `--ouvriers 4`, aucune trace de CUDA ni d'appel à une carte graphique] |
| **Critère de réussite** | **Battre les stratégies à règles écrites avec au moins 98 % de victoires.** La comparaison avec l'IA de référence extérieure est écartée par Alexis : elle trichait et jouait sur un autre moteur. |
| **Bruit et chaleur de la machine** | **Aucune limite.** Je lance ce que je veux, quand je veux. |
| **Comptes rendus** | **Uniquement en cas de problème qui exige son intervention.** Sinon j'enchaîne le chantier suivant ou je répare moi-même. |

## Ce que je décide seul

- L'ordre, le découpage et le contenu des chantiers restants (L7, L8, L9).
- Les contrats confiés aux agents, leurs contrôles visibles, leurs contrôles
  cachés, et le verdict de chaque audit.
- Tout arbitrage technique que la mesure tranche — la largeur du réseau comprise.
- Les enregistrements dans le dépôt et leur publication.
- Relancer un agent tombé ; réparer mes propres contrôles.
- Purger les dettes déclarées quand elles bloquent un chantier.

## Ce sur quoi j'interromps Alexis, toujours

1. **Une dépense d'argent**, quel qu'en soit le montant.
2. **Une exposition publique des visuels du jeu**, qui sont sous droits d'auteur.
3. **Un résultat qui remet en cause l'objectif** — par exemple une force qui
   plafonne sous ce qu'il faut pour battre un bon joueur humain.
4. **Une panne que je ne sais pas réparer en deux tentatives.**
5. **Une décision de règle du jeu** dont le livret ne tranche pas.

## Le critère de réussite, écrit précisément

**Seuil de passage** : au moins **98 % de victoires** contre `reflechi`, la
stratégie à règles écrites, mesurées sur au moins **80 donnes jouées aux deux
sièges** — jamais moins, un duel de 40 donnes ne prouve rien.

**Ce seuil a déjà été atteint une fois**, le 16-08, au palier d'un million de
parties d'entraînement : **99,4 % contre `reflechi` et 99,4 % contre le hasard**
[VÉRIFIÉ 28-08 — `docs/CTO_STATE.md`, courbe du 16-08 au soir]. Les lots L1 à L6
ont depuis changé la fiche de situation que l'IA lit (1 472 → 1 630 cases), ce
qui rend **tous les poids d'avant illisibles** (commit `f33a6e6`) : le chiffre
devra être refait après le dernier entraînement.

**Pourquoi ce seuil ne suffit pas à lui seul, et ce que j'y ajoute.** À 99 %,
l'instrument sature : il ne distingue plus une IA forte d'une IA très forte.
J'ajoute donc deux mesures de progression, déjà employées et sensibles :

- l'**écart de score moyen** contre un adversaire fixe (le seul instrument encore
  sensible au-delà du seuil) ;
- le **duel entre deux versions successives** de l'IA, qui dit si la dernière
  étape a vraiment apporté quelque chose.

**Un piège déjà payé, à ne pas repayer** : le « plafond de 88,8 % » annoncé le
15-08 était faux — cette part n'est pas une propriété du jeu mais du couple de
joueurs, et elle a été dépassée dès le lendemain (91,3 %). Aucun plafond ne sera
plus annoncé sans être mesuré contre deux forces différentes.

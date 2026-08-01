# Retours interface — deuxième série (01-08-2026)

Ce fichier prolonge `INTERFACE_RETOURS_01.md`. Il note les demandes formulées par
Alexis le 1er août, plus une **erreur de règle** que ces retours ont fait
apparaître.

La numérotation continue celle du premier fichier (qui s'arrête à 19).

---

## Décision de cadre : on abandonne le bac à sable

Alexis, mot pour mot : « le fait de commencer par une interface bac à sable avec
cartes face visible ça t'embrouille pour proposer une interface cohérente. Je
pense que le plus simple c'est qu'on travaille tout de suite sur une interface
avec l'adversaire qui joue avec cartes faces cachées. »

Conséquence : **l'écran ne montre plus qu'un seul point de vue**, celui du joueur
humain. Tout ce que l'adversaire tient reste caché. L'adversaire est un
programme qui décide arbitrairement, en attendant l'intelligence artificielle.

C'est un changement de fond, pas un réglage : aujourd'hui l'écran affiche les
deux mains en clair parce qu'il servait à vérifier le moteur.

---

## 20 — Un seul point de vue, deux mains, haut et bas

- Ma main : **en bas** de l'écran, cartes lisibles.
- La main de l'adversaire : **en haut**, cartes **retournées** (dos visible), le
  nombre de cartes restant la seule information publique.
- Plus de main « en éventail sur le côté droit » : Alexis la juge illisible et
  déroutante. [VÉRIFIÉ 01-08 — mesuré : les cartes de cet éventail descendent
  jusqu'à 120 px de large et se recouvrent.]
- L'adversaire est un programme qui joue des coups arbitraires. Il n'a pas
  besoin d'être bon : il doit être **présent et opaque**.

## 21 — Les animations de pose (rappel des points 9 et 10)

Toujours attendues, telles que décrites dans le premier fichier : trois secondes
d'animation qui ouvrent la vue du plateau, la carte qui voyage jusqu'à sa pile,
et la prise en main de la carte à la souris avec une oscillation physique et une
zone d'accueil mise en valeur.

## 22 — ERREUR DE RÈGLE : des cartes sans action sont proposées à l'activation

[VÉRIFIÉ 01-08 — `engine/src/flow.rs:3802-3807`]

En phase III, le moteur retient les cartes activables ainsi :

```rust
remaining_blue[p] = game.players[p].played.iter().copied()
    .filter(|&c| db.projects[c as usize].color == Color::Blue)
    .collect();
```

Le filtre ne regarde que **la couleur**. Or une carte bleue peut porter un effet
permanent et **aucune action déclenchable**. Le moteur les propose quand même
(`flow.rs:2879`), et l'activation est consommée pour rien (`flow.rs:3866-3872` :
l'activation est retirée « dans tous les cas »).

Exemple relevé par Alexis à l'écran : *United Planetary Alliance*, dont le texte
est « When you draw cards during the research phase, draw one additional card and
keep one additional card » — un effet, jamais une action. Sa définition dans le
moteur est pourtant correcte : `atrig: []`, c'est-à-dire aucune action
déclenchable (`engine/src/effects.rs:2223`).

**Au moins huit cartes** sont concernées : Adaptation Technology, Composting
Factory, Extended Resources, Interns, Mars University, Restructured Resources,
Standard Technology, United Planetary Alliance. [DÉCLARÉ — mon relevé n'a su
analyser que 73 des 90 cartes bleues, le compte réel est probablement plus
élevé ; à refaire proprement.]

Gravité : ce n'est pas cosmétique. Un joueur peut gâcher son unique activation de
la manche, et surtout l'intelligence artificielle à venir devra apprendre à
éviter des coups qui ne devraient pas exister.

Correction : ajouter au filtre la condition « cette carte porte une action ».

## 23 — Vendre une carte à tout moment, et plusieurs à la fois

Déjà signalé (point 16b), reformulé et élargi :

- La vente d'une carte contre des mégacrédits doit être possible **à n'importe
  quel moment du tour, y compris pendant le tour de l'adversaire**, par un bouton
  toujours disponible.
- On doit pouvoir en **sélectionner plusieurs** d'un coup.

Rappel : c'est un chantier **moteur**, pas seulement d'écran. Aujourd'hui la
vente n'existe que comme une action de la phase III
(`engine/src/policy.rs:38`), et la rendre permanente ouvre un point de décision
nouveau à chaque instant de la partie — ce qui pèsera lourd sur l'apprentissage
de l'intelligence artificielle. À traiter à part, avec sa propre réflexion.

## 24 — Les jauges de température et d'oxygène, façon jeu de société

Reprise du point 17, précisée : **en arc de cercle**, avec les montants colorés
selon la zone atteinte, comme sur la planche imprimée. Placées de part et
d'autre de l'écran (température à gauche, oxygène à droite), en surimpression,
avec un rendu de tableau de bord lumineux.

Elles doivent permettre de voir d'un coup d'œil **où l'on se situe par rapport
aux exigences de couleur** que portent certaines cartes.

## 25 — La planche des tuiles océan

L'affichage actuel est jugé laid : la tuile posée dessus n'est pas détourée.

À la place :

- Une **petite planche des océans** montrant les vraies tuiles **déjà
  retournées** et celles qui restent face cachée.
- Au survol, la planche **s'agrandit**.
- Quand une carte demande de retourner un océan, la planche apparaît et on
  **choisit soi-même** la tuile à retourner.
- Le retournement joue une **animation** où l'on découvre ce que la tuile
  portait.

Dépendance : ceci exige que le moteur dise **lesquelles** des neuf tuiles sont
retournées. C'est la dette notée au point 19 ; des modifications allant dans ce
sens existent dans le dossier de travail depuis le 01-08 à 9h52
(`engine/src/state.rs`, `engine/src/observe.rs`) mais ne sont **ni vérifiées ni
enregistrées**.

Question ouverte : le choix de la tuile à retourner est-il conforme au livret ?
Le moteur les retourne aujourd'hui dans l'ordre d'un mélange. Si le joueur
choisit, ce n'est plus la même règle. **À trancher avant de coder.**

## 26 — Des effets sonores

« Il faudra du sound effect aussi, ça fait trop la différence. » Sons de pose de
carte, de retournement de tuile, de validation. Portée et style à préciser.

## 27 — L'écran de menu (rappel du point 18, non fait)

Toujours identique et jugé fade.

## 28 — L'onglet de réglages (rappel du point 4, non fait)

Toujours absent. Doit contenir un retour au menu principal et un bouton d'aide
rappelant notamment la liste des cartes Phase améliorées.

---

## État des points antérieurs traités le 01-08

- Point 3 (remplacer entre 0 et 8 cartes projet) — **fait et vérifié à l'écran**.
- Point 3-bis (zoom au survol des corporations) — **fait et vérifié**
  (`9a95718`) : plus de loupe dès qu'une carte est déjà affichée à 80 % ou plus
  de la taille de la loupe.

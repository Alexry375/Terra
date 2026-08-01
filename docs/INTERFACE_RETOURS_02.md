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

## 20-bis — Les choix simultanés se voient simultanément

Précision d'Alexis : « les choix de corporations de mulligan et tout, ce serait
bien que les 2 joueurs puissent les faire en même temps et qu'on voit juste
l'adversaire faire ses choix avec des cartes retournées et en petits formats
(pour que la majorité de l'écran soit toujours prise par nos choix et nos
cartes). »

Donc, quand une phase demande la même chose aux deux joueurs :

- **Notre** choix occupe la majorité de l'écran, en grand.
- Le choix de l'adversaire se déroule **au même moment**, dans un coin, en
  **petit format et cartes retournées** — on voit qu'il agit, jamais quoi.
- Interdit : reprendre l'écran actuel en se contentant de retourner les cartes de
  l'adversaire à chacun de ses tours de parole.

Contrainte technique à ne pas sous-estimer : le moteur pose aujourd'hui les
décisions **l'une après l'autre** (une décision pour le joueur 0, puis une pour
le joueur 1). [VÉRIFIÉ 01-08 — relevé à l'écran : décision 0 = joueur 0, décision
1 = joueur 1 pour la même question.] L'affichage simultané suppose donc que
l'écran laisse le programme adverse répondre pendant que nous réfléchissons, puis
raconte les deux réponses ensemble. C'est un travail d'écran, pas de moteur, mais
il demande d'être conçu exprès.

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

**CORRIGÉ le 01-08** (`4eb57fe`) : nouveau contrôle `flow::activable_blue`, qui
exige que la carte porte réellement une action. Trois tests, dont le contrôle
inverse — vérifié qu'en revenant au filtre par couleur seule, le test redevient
rouge. 821 tests verts, 0 rouge. Les trois empreintes de référence changent, ce
qui est attendu (la suite des tirages n'est plus la même) : boîte de base, graine
2024, `d6a7267472501b13` → **`c1c52fcbe4e057b0`**.

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

**Le livret donne raison à Alexis** [VÉRIFIÉ 01-08, `docs/regles/livret-base.md`
l. 96] : « gardez à l'esprit qu'à tout moment, **vous pouvez défausser une carte
Projet de votre main pour gagner 3 MC** ». Et l. 348 : le coût d'une carte peut
être payé « en défaussant d'autres cartes Projet dans votre main à raison de 3 MC
par carte », le surplus étant rendu.

**Portée retenue** (proposition d'Alexis, à confirmer une dernière fois) :
autoriser la vente pendant les phases **I (développement), II (construction) et
III (actions)**, et pas pendant IV (production) ni V (recherche), où l'on ne
dépense rien. [VÉRIFIÉ 01-08 : la phase V ne fait payer aucune carte gardée —
livret l. 425.] Cela couvre tous les moments utiles sans créer un point de
décision à chaque respiration.

**Ma recommandation, en plus** : implémenter aussi le **paiement par défausse**
(payer une carte en défaussant d'autres cartes à 3 MC pièce). C'est la seconde
moitié de l'écart E4, c'est ce que le livret décrit, et c'est en pratique ce à
quoi sert la vente permanente — cela évite au joueur d'avoir à vendre puis
acheter en deux gestes.

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

**Tranché le 01-08.** Alexis choisit une tuile **face cachée** parmi celles qui
restent. J'avais objecté que cela changeait les probabilités : **j'avais tort**.
Neuf tuiles mélangées face cachée sont indiscernables ; désigner la troisième
plutôt que la première donne exactement la même loi de tirage. Le choix est donc
purement une sensation de jeu, sans effet sur les règles ni sur l'équité. À
implémenter tel quel.

## 26 — Des effets sonores

« Il faudra du sound effect aussi, ça fait trop la différence. » Sons de pose de
carte, de retournement de tuile, de validation. Portée et style à préciser.

## 26-bis — Trois rôles à prévoir dès maintenant, pas seulement deux

Réponse d'Alexis à ma question sur le jeu en ligne : « Oui si c'est moins cher,
prévois le maintenant. Et je pourrais faire jouer mon IA contre mon pote, en
ligne (moi je vois l'IA jouer cartes visible comme si c'était moi et je ne vois
pas les cartes de l'adversaire). »

Il y a donc **trois** situations, et non deux, et elles se ramènent à une seule
question : **qui décide** et **qui regarde**.

| Situation | Qui décide pour moi | Ce que je vois |
|---|---|---|
| Moi contre un programme | moi | ma main, l'adversaire caché |
| Moi contre un humain à distance | moi | ma main, l'adversaire caché |
| Mon intelligence artificielle contre son humain | l'IA | **la main de l'IA en clair**, comme si c'était la mienne ; l'adversaire caché |

Conclusion de conception : l'écran ne doit **jamais** supposer que le joueur du
bas est celui qui décide. Un point de vue (« de quel siège je regarde ») et une
source de décisions (« qui répond aux questions ») doivent être deux réglages
séparés. C'est le seul choix d'architecture réellement structurant de toute cette
liste, et il ne coûte presque rien si on le fait maintenant.

## 26-ter — Voir quelles phases ont été choisies dans la manche

Aujourd'hui les cinq cartes Phase sont affichées en permanence sur le côté
gauche. À la place :

- montrer **les deux phases choisies** par les joueurs pour cette manche — une
  seule si les deux joueurs ont choisi la même ;
- **celle en cours est allumée**, les autres éteintes.

Le moteur expose déjà le nécessaire : `players.N.chosen_phase` et
`players.N.previous_phase`. [VÉRIFIÉ 31-07, vocabulaire d'état.]

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

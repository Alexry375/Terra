# Dernière ligne droite

Liste dictée par Alexis le **04-08 vers 05h00**. Elle fait foi : tant qu'une
ligne n'est pas cochée ET vérifiée en jouant, elle n'est pas faite.

Convention : `[VÉRIFIÉ JJ-MM]` = relu à la source ou mesuré. `[DÉCLARÉ]` = dit
par quelqu'un, pas encore prouvé.

---

## A. Animations de pose de carte

### A1 — ✅ FAIT [VÉRIFIÉ 04-08] Les cartes en suspension sont de travers
Pendant l'animation de pose, la carte reste quelques secondes en l'air, comme
prévu, mais **inclinée** au lieu d'être droite.
État : signalé plusieurs fois, jamais corrigé.
Capture fournie par Alexis — **je ne l'ai pas reçue de mon côté**, à redemander.

### A2 — ✅ FAIT [VÉRIFIÉ 04-08] Il manque la transition entre la grande carte et la carte posée
La grande carte en l'air disparaît, la petite carte apparaît sur le plateau. Il
manque le mouvement qui relie les deux : le dépôt.
État : signalé plusieurs fois, jamais corrigé.

---

## B. Les jauges de température et d'oxygène

### B1 — QUESTION TRANCHÉE : le moteur est juste, seul l'écran est faux
Alexis a rectifié le 04-08 : il avait oublié les cases **rouges** de la
température entre le violet et le jaune. Le découpage réel du plateau est donc :

| Jauge | Découpage du plateau | Total |
|---|---|---|
| Température | 6 violettes, 5 rouges, 5 jaunes, 4 blanches | **20** |
| Oxygène | 3 violettes, 4 rouges, 5 jaunes, 3 blanches | **15** |

Ce que le moteur dit, [VÉRIFIÉ 04-08] :

```
engine/src/state.rs:19-21   TEMPERATURE_MAX = 19  →  20 positions (0 à 19)
                            OXYGEN_MAX      = 14  →  15 positions (0 à 14)
engine/src/effects.rs:26-36 TEMP_R_MIN=6  TEMP_Y_MIN=11  TEMP_W_MIN=16
                            OXY_R_MIN=3   OXY_Y_MIN=7    OXY_W_MIN=12
```

Soit, cran par cran : température violet 0-5 (**6**), rouge 6-10 (**5**), jaune
11-15 (**5**), blanc 16-19 (**4**). Oxygène violet 0-2 (**3**), rouge 3-6
(**4**), jaune 7-11 (**5**), blanc 12-14 (**3**).

**Concordance parfaite sur les deux jauges.** Le moteur compte les cases et
change de couleur exactement aux bons crans.

### B2 — Les requis sont-ils débloqués aux bons moments ? OUI
[VÉRIFIÉ 04-08] `engine/src/flow.rs:1462-1471` : un requis de température se
teste par `temp_color(...)`, c'est-à-dire par le **niveau de couleur**, pas par
le numéro de case. La souplesse d'un cran (`flex`) travaille elle aussi sur la
couleur. Même chose pour l'oxygène avec `oxy_color`.

**Conclusion : il n'y a rien à changer dans les règles.** Le travail se limite à
l'affichage des jauges — nombre de cases dessinées et couleurs.

---

## C. ✅ FAIT [VÉRIFIÉ 04-08] La phase de production ne se voit pas

Les compteurs de MC, de chaleur et de plantes augmentent **instantanément** : on
ne comprend pas qu'il s'est passé quelque chose.
Demandé : un `+X` visible qui dure assez longtemps pour être lu. La forme exacte
est laissée libre.

---

## D. Objectifs et récompenses

### D1 — ✅ FAIT (à regarder de vos yeux) Zoom au survol
Passer le curseur sur un objectif ou une récompense doit l'agrandir pour qu'on
puisse le lire.

### D2 — ✅ FAIT Retirer une mention
Supprimer le texte « Mars surface · NASA / JPL / University of Arizona ».

---

## E. Les tuiles océan

### E1 — Aucune tuile n'est face visible
Même quand une tuile est retournée, elle ne se révèle pas. Défaut ancien,
plusieurs fois signalé.

### E2 — Le joueur ne choisit pas quelle tuile retourner
Aujourd'hui le moteur choisit au hasard. Alexis veut choisir.
**Facilité explicitement autorisée par lui** : si toutes les tuiles donnent le
même résultat, le choix peut être purement visuel. À confirmer contre le livret.

### E3 — Il manque l'animation de retournement
La tuile doit se retourner à l'écran.

---

## F. Les cartes Phase améliorées

### F1 — ✅ FAIT [VÉRIFIÉ 04-08] Les nouveaux visuels ne s'affichent pas au moment du choix
Quand on améliore une carte Phase, la liste proposée montre encore les visuels
**de base**.
Précision d'Alexis : la phase **Recherche** améliorée et la carte
**Développement** s'affichent, elles, correctement. Le défaut ne frappe donc pas
partout.

### F2 — ⚠️ PAS UN DÉFAUT [VÉRIFIÉ 04-08] La production améliorée demande bien
La question EXISTE et le moteur la pose. Elle est simplement **rare** : mesurée
sur **cinq parties entières** (1 047 décisions au total, graines 2024, 5150, 77,
31337, 909), la décision « quelle carte verte rejoue sa production » est apparue
**2 fois**. Il faut, dans la même manche, avoir choisi la carte Production
améliorée A ET posséder au moins **deux** cartes vertes qui produisent quelque
chose — sinon le moteur double la seule carte possible sans rien demander
(`engine/src/flow.rs:4324`, `replay_green_production`).

Alexis n'a donc « pas eu de chance », au sens propre. Ce qui change quand même
pour lui : depuis le point C, le gain apparaît maintenant en « +X » sur ses
compteurs — il verra qu'il s'est passé quelque chose, même sans question posée.

### F3 — L'Action améliorée ne montre pas les cartes tirées
Elle est censée montrer 3 cartes de la pioche et permettre d'en prendre une
bleue ou une rouge. Alexis a l'impression de ne rien récupérer.
Demandé au minimum : **montrer les trois cartes tirées**, même quand aucune
n'est prenable.
Précision donnée le 04-08 : **quand plusieurs cartes bleues ou rouges sont
tirées, le joueur doit choisir laquelle prendre.** C'est peut-être déjà le cas,
Alexis n'a jamais rencontré l'exemple. À reproduire et à prouver.

---

## G. Le paquet de cartes projet

### G1 — ✅ FAIT [VÉRIFIÉ 04-08] Afficher combien de cartes restent dans le paquet
Le bandeau écrit « DECK 246 +0 » : ce qui reste à piocher, puis ce qui attend
dans la défausse. Mesuré sur une partie entière : 246 → 26, défausse 0 → 172.

### G2 — Remélanger la défausse quand le paquet est vide
**Déjà fait** [VÉRIFIÉ 04-08] `engine/src/flow.rs:32-42` : `draw_card`
intervertit pioche et défausse, remélange, puis pioche. Le commentaire cite le
livret p. 15.
Reste à vérifier : que l'écran le montre au joueur.

---

## H. Validé par Alexis, ne plus y toucher

- **Le score.** « Le score c'est bon je valide. » (04-08)

---

## I. Ce qu'Alexis n'a PAS listé et qui reste ouvert

Il a demandé : « J'avais pas mentionné d'autres choses ? » Voici ce que je tiens
au catalogue et qu'il n'a pas cité ce matin.

### I1 — 🔴 BLOQUANT : la partie se bloque à plusieurs tailles de fenêtre
[VÉRIFIÉ 04-08] Balayage de **quatorze** tailles, même partie, même graine.
**Quatre bloquent pour de bon** — plus aucun bouton de choix n'est atteignable
au 7ᵉ écran :

| Fenêtre | Bande des choix | Résultat |
|---|---|---|
| 1536 × 864 | 45 points de haut | **bloquée** |
| 1450 × 800 | 29 points | **bloquée** |
| 1440 × 810 | 32 points | **bloquée** |
| 1280 × 800 | 29 points | **bloquée** |

Et **treize sur quatorze** présentent au moins un écran où des boutons se
chevauchent. Seule 1920 × 1200 est saine de bout en bout.

C'est donc un défaut général de mise en page, pas le cas particulier d'une
taille. **C'est le seul défaut connu qui empêche purement et simplement de
jouer.** Chantier `workspaces/la-bande-des-choix` prêt, contrat écrit, pas encore
scellé.

### I2 — L'état du moteur recule parfois
20 reculs sur 183 lectures, graine 5150. Non expliqué.

### I3 — Le prix effectif barré
Quand une remise s'applique, le prix d'origine devrait être barré à côté du prix
payé.

### I4 — Effets sonores
Jamais commencés.

### I5 — Sauvegarde de partie
Impossible de reprendre une partie interrompue.

### I6 — Trois décisions gardent leur liste au milieu de l'écran
Défaut d'affichage isolé, jamais reproduit proprement.

### I7 — La main déborde en 1280 × 640

### I8 — La vente à distance : un panneau sur dix-huit reste ouvert
[VÉRIFIÉ 04-08] Mesuré ce matin : sur 18 ventes conclues pendant une partie à
deux, **17 se referment en moins d'une seconde**, une est restée ouverte plus de
30 secondes. La partie va au bout et les deux écrans restent d'accord sur le
score : ce n'est pas un blocage. Cause du cas résiduel inconnue.

### I9 — L'intelligence artificielle
Le grand chantier final. Non commencé. C'est l'objectif du projet.

---

## Questions ouvertes — état au 04-08 vers 05h30

1. ~~Température : 15 crans ou 20 ?~~ **TRANCHÉE : 20.** Voir B1 et B2. Le
   moteur est juste, il n'y a que l'écran à corriger.
2. **La capture de la carte de travers** ne m'est **toujours** pas parvenue.
   Alexis l'a envoyée deux fois, elle n'arrive pas jusqu'à moi. Je corrigerai
   l'inclinaison sans la voir, en relisant le code de l'animation.
3. ~~La phrase coupée~~ **RÉPONDUE** : quand plusieurs cartes bleues ou rouges
   sont tirées, il faut pouvoir choisir. Voir F3.
4. ~~L'heure de la partie~~ **RÉPONDUE : 9h30 le 04-08, maintenue.** Consigne
   d'Alexis : « fais de ton mieux, ne bâcle pas juste pour finir. »

**Autonomie totale accordée le 04-08 vers 05h30.** Plus aucune question ne
bloque : je travaille jusqu'au bout sans rien lui redemander.

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

### F3 — ✅ FAIT [VÉRIFIÉ 04-08] L'Action améliorée ne montre pas les cartes tirées
Le défaut était dans le **moteur** : il tirait bien trois cartes, mais ne
présentait que les prenables — et quand aucune n'était bleue ou rouge, aucune
décision n'était posée du tout, donc rien ne s'affichait. Les trois cartes sont
maintenant montrées à chaque fois ; celle qu'on ne peut pas prendre est éteinte
et cerclée de gris avec « CANNOT BE TAKEN », celle qu'on peut prendre garde ses
couleurs et porte un liseré vert. Le choix entre plusieurs prenables existait
déjà dans le moteur, il est maintenant visible.
Mesure : cinq parties entières, **13 révélations vues, 33 cartes montrées dont
19 NON prenables** — c'est précisément ce qui n'apparaissait jamais. Moteur :
830 tests, 830 verts. Aucune règle ne change.

### F3 (énoncé d'origine)
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

### I1 — ✅ RÉGLÉ [VÉRIFIÉ 04-08] La partie se bloquait à plusieurs tailles
Après correction : **dix-neuf tailles balayées, 3 780 écrans mesurés, zéro écran
fautif**, et la partie entière (233 décisions, mêmes scores) à chacune d'elles —
y compris les quatre qui bloquaient. Énoncé d'origine ci-dessous.

### I1 (énoncé d'origine) — la partie se bloquait à plusieurs tailles de fenêtre
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

---

## Deuxième liste, dictée par Alexis le 04-08 vers 09h00 (écran de jeu ouvert)

### J1 — Les cartes Phase améliorées III et IV : les DEUX IMAGES ÉTAIENT INVERSÉES
[VÉRIFIÉ 04-08 · CORRIGÉ] Alexis a choisi « l'amélioration de production qui
double une carte verte », et n'a rien vu se doubler ; une autre partie lui a
donné +13 MC là où il en attendait +8.

**Le moteur n'avait aucun tort.** Les deux découpes d'images étaient permutées :
le fichier nommé `carte-phase-4-production-amelioree-a` portait le texte de la
variante B (« Gain 7 MC »), et réciproquement. En cliquant sur l'image qui
promet le doublement, Alexis installait donc la carte à 7 MC. Cela explique
**les deux** observations d'un seul coup : aucun doublement (la variante B n'en
accorde pas), et 5 de terraformation + 7 du bonus + 1 de production = **13 MC**.

Les dix images ont été relues une par une et comparées au moteur :
phases I, II et V justes ; **phases III et IV inversées**, toutes deux
corrigées. Les fichiers ont été échangés et la trace de découpe du manifeste
avec eux.

Cause première : les découpes ont été nommées d'après leur position dans la
planche scannée, en supposant partout un ordre A puis B — faux pour ces deux
phases. **Aucune de mes mesures ne pouvait le voir** : elles vérifient qu'une
image s'affiche, jamais ce qu'elle raconte. Deuxième défaut de la journée trouvé
à l'œil et non par un contrôle.

### J2 — La grande tuile océan se retourne et montre encore son dos
[EN COURS 04-08] Sur la planche des neuf océans, les tuiles se retournent
correctement. C'est la grande tuile rejouée au milieu de l'écran qui montre son
dos des deux côtés.

### J3 — Les logos Océan et achat de jeton Forêt ne sont pas détourés
[DÉCLARÉ 04-08] Dans les décisions, ces deux jetons s'affichent sur un carré
blanc, alors que le logo de défausse, lui, est proprement détouré.

### J4 — Une liste de musiques en fond de partie
[DÉCLARÉ 04-08] Demandé : les titres dans l'ordre, le nom du morceau affiché, un
bouton pour passer au suivant, reprise au début à la fin de la liste, et de quoi
tout couper. **Le lien de la liste n'est pas arrivé jusqu'à moi** — le message
disait « cette playlist » sans que rien ne me parvienne. À redemander.
Réserve déclarée par Alexis lui-même : si cela oblige à télécharger tous les
morceaux, on laisse tomber pour aujourd'hui. C'est bien le cas — un navigateur
ne peut pas lire une liste hébergée ailleurs sans les fichiers.

## Troisieme liste, dictee par Alexis le 04-08 pendant la partie a deux

### K1 — Deux ventes de suite arretent la partie
[CORRIGE 04-08 COTE ECRAN, VERIFIE — verif_vente.py, vert sur la livraison et
ROUGE sur une copie sabotee]

**Cause racine, lue dans le code.** `flow::occasion_de_vendre` ARME
`game.occasion_ouverte` avant de consommer la vente ; `flow::observer` le publie
ensuite dans `vente_offerte`. Le moteur repose alors la MEME question et
republie donc `vente_offerte = true`, alors que l'occasion vient d'etre
depensee. L'ecran offrait le bouton une seconde fois, sur un point ou aucune
occasion n'attendait plus. Le garde qui existait (`if (soumise) return`) ne
couvrait pas ce cas : une vente livree TOUT DE SUITE remet `soumise` a null.

**Correctif** (`vue/vente.js`, `interface.js`) : un verrou `venduIci`, pose des
qu'une vente est validee, leve UNIQUEMENT quand mon siege repond a une question
(`apresMaReponse`) — c'est exactement la regle du moteur, une occasion par point
de decision. Le bouton reste dans la page, desarme, et DIT pourquoi
(« Sale sent — play or pass first »).

Vecu en partie reelle : le siege 0
vend au rang 108 (accepte), revend au rang 109 (refuse). Le moteur n'ouvre
**qu'une occasion de vendre par point de decision** ; l'ecran, lui, laisse le
bouton en place. La partie s'est arretee des deux cotes sur
« aucune occasion de vendre n'est ouverte a ce point ». Sauvee en recopiant les
109 premieres decisions dans une partie neuve (meme graine), sans la fautive.

### K2 — Pouvoir vendre plusieurs fois d'affilee
[CORRIGE 04-08 COTE ECRAN, VERIFIE] La forme retenue est celle decrite plus bas :
l'ecran accumule autant de cartes qu'on veut dans UNE seule vente, on peut
ajouter et retirer librement, et rien ne quitte la main avant confirmation. Le
panneau le dit maintenant (« Pick as many as you want — nothing leaves your hand
until you confirm »). Mesure : 3 cartes designees, 1 reprise, 2 parties d'un coup
(8 -> 6 cartes en main).

**Ce qui reste impossible sans toucher au moteur** : vendre, VOIR le resultat,
puis revendre au meme point. Verifie ligne a ligne — `occasion_de_vendre`
n'appelle `vendre_librement` qu'une fois par point, et le harnais de rejeu
(`wasm/src/lib.rs:1400`) ne consomme qu'une entree de vente par curseur. K1
transforme donc ce cas en refus lisible au lieu d'un arret de partie.

Motif d'Alexis : « pour le cas ou on se tromperait ». Il a
raison sur le besoin — son ami a vendu la seule carte qu'il pouvait poser et
s'est retrouve sans rien a faire.

**A FAIRE SANS TOUCHER AU MOTEUR.** Alexis a lui-meme pose la contrainte : « ca
va pas rendre l'IA moins performante, ce genre de trucs qui multiplie les
options ? » — et c'est juste. Vendre deux cartes d'un coup ou deux fois de suite
mene EXACTEMENT au meme etat : ce sont deux chemins pour un seul resultat, ce
qui gonfle l'arbre de recherche sans rien apporter. C'est le pire cas pour une
IA.

La bonne forme est donc : UNE seule occasion de vente pour le moteur, mais
REVISABLE tant que le joueur n'a pas repondu a la question principale. L'ecran
accumule les cartes designees et n'envoie qu'une seule reponse de vente. Le
joueur peut vendre, voir le resultat, vendre encore ; le moteur ne voit qu'une
vente, et l'arbre de l'IA ne grossit pas d'un noeud.

### K3 — Rien n'avertit qu'on vend la carte qu'on pouvait poser
[PARTIELLEMENT CORRIGE 04-08, VERIFIE] L'avertissement est en place : une carte
designee qui porte `data-choix` (donc que la question en cours propose de poser)
prend un contour OR par-dessus le rouge de la vente, le panneau passe en or et
ecrit « ⚠ that card can be played right now — selling it loses that play ».
On ne l'interdit pas — le livret l'autorise — on le montre.

**RESTE A FAIRE** : dire « aucune carte constructible cette phase » quand la
question de pose n'offre aucune option, au lieu de passer sans un mot.

Rang 103 : une seule option, « poser Special Design » (3 MC),
avec 7 MC en poche. Le joueur a vendu cette carte-la. Plus aucune carte rouge
abordable ensuite, donc la phase s'arrete — a juste titre, mais en silence.
A faire : marquer, pendant la designation de vente, les cartes qui figurent
parmi les choix de pose en cours ; et dire en clair « aucune carte constructible
cette phase » au lieu de passer sans un mot.

### K4 — Voir la defausse
[DEMANDE 04-08] Pouvoir consulter la pile des cartes defaussees.

### K5 — Une action de carte impossible reste proposée
[VERIFIE 04-08, A CORRIGER APRES LA PARTIE] Les neuf oceans sont reveles et
« Aquifer Pumping » est toujours offerte. Ce que fait le moteur, verifie ligne a
ligne : `flow.rs:3291` (`action_effs_possible`) rend faux des que l'effet pose un
ocean et que `snap_oceans >= NUM_OCEANS`, donc `apply_blue_action` sort par
`return false` AVANT tout paiement — aucun MC perdu. Mais la boucle de la phase
Action consomme l'activation « dans tous les cas » (flow.rs:4198) : le joueur
perd son droit d'action du tour pour rien.

A corriger dans `action_options` (flow.rs:3123) : ne pas proposer une carte
bleue dont l'action ne peut rien produire, exactement comme l'action standard
Ocean l'est deja par `game.snap_oceans < NUM_OCEANS` (flow.rs:3146).

**PAS PENDANT UNE PARTIE EN COURS** : les decisions enregistrees sont des INDICES
dans la liste des options. Retirer une option change tous les indices suivants et
detruirait la partie au rejeu.

Gain double : le joueur ne se piege plus, et l'IA n'explore plus une branche
morte — meme raisonnement que K2.

### K6 — Le bonus de la phase Construction est tranché trop tôt
[VÉRIFIÉ 04-08 contre le livret ET contre le code, À CORRIGER APRÈS LA PARTIE]

Signalé par Alexis : on est obligé de choisir dès le début de la phase entre
« piocher une carte » et « jouer une 2e carte », alors qu'on voudrait poser
d'abord une carte qui fait piocher, voir ce qui arrive, puis décider.

**Le livret lui donne raison.** Texte exact, `docs/regles/livret-base.md:336` :
« Bonus : Si vous avez choisi cette phase, vous pouvez au choix : piocher une
carte AVANT OU APRÈS avoir joué une carte lors de cette phase OU vous pouvez
jouer une carte bleue ou rouge supplémentaire lors de cette phase. » Aucune
phrase n'impose d'annoncer la branche à l'avance ; la branche « après avoir
joué » est explicitement prévue. Le moteur est donc plus restrictif que la règle.

**Ce que fait le code** (`engine/src/flow.rs:3994-4005`, `phase_construction`) :
`policy.construction_bonus(...)` est appelé AVANT le calcul des options de pose
et avant `policy.choose_build`. Les trois issues (pioche avant, pioche après,
seconde pose) sont donc arrêtées alors que le joueur n'a encore rien posé.
Les cartes améliorées II-A / II-B passent par `selector_branch`, appelé au même
endroit — même défaut.

**Correction visée** : garder au début une question réduite (« piocher tout de
suite, avant de poser ? » — c'est la seule branche qui doit être décidée tôt,
puisque la carte piochée peut servir à la pose), puis, une fois la première
carte posée, poser la vraie question entre « piocher » et « poser une seconde ».

**PAS PENDANT UNE PARTIE EN COURS** : même raison que K5 — cela déplace et
modifie des points de décision, donc tous les indices enregistrés.

Effet sur l'IA : neutre à positif. Le nombre d'issues finales ne change pas
(pioche ou seconde pose) ; c'est de l'information gagnée avant de trancher, ce
qui rend chaque branche plus facile à évaluer, pas plus nombreuse.

### K7 — Un correctif de style qui n'existait que dans le fichier
[TROUVE ET CORRIGE 04-08 — trouve parce qu'Alexis a demande de verifier les
correctifs poses en direct, sans workspace ni controle]

Le correctif du bouton de vente pose le matin meme (fond ambre, texte or, pour
qu'il cesse d'etre noir sur noir) **n'avait aucun effet**. Le bloc etait ecrit
`#vente-ouvrir` — specificite (1,0,0) — alors que `#vente button`, quinze lignes
plus haut dans la meme feuille, pese (1,0,1) et fixe deja `background`, `border`
et `color`. Le plus specifique gagne, pas le dernier ecrit.

Mesure qui l'a revele : la couleur calculee du bouton valait `rgb(239,228,212)`
(`var(--os)`) au lieu de `rgb(237,181,78)` (`var(--or)`) annonce dans le fichier.
Corrige en `#vente button#vente-ouvrir`, et le controle mesure DESORMAIS la
couleur calculee — pas la presence de la regle.

**Lecon a retenir** : un correctif de style n'est pas verifie tant qu'on n'a pas
lu la valeur CALCULEE dans un vrai navigateur. Relire la feuille ne prouve rien.
Au passage, `#vente button:hover` a recu `:not(:disabled)` : le panneau se replie
apres une vente et le bouton passe sous le curseur tout seul, donc un bouton
desarme s'allumait sans que la main ait bouge.

### K8 — Une question sautee quand rien n'est payable, donc AUCUNE occasion de vendre
[VERIFIE 04-08 sur la partie en cours, A CORRIGER APRES LA PARTIE]

Signale par Alexis en direct : son ami a choisi le bonus « poser une carte
bleue/rouge supplementaire » et n'a pose qu'une seule carte.

**Ce que le rejeu montre** (partie mars2, graine 210055, rangs 144 a 146) :
rang 144 il choisit bien « Poser une carte bleue/rouge supplementaire » ; rang
145 il pose Business Contracts (une seule option offerte) ; rang 146 le moteur
passe DIRECTEMENT a la limite de main. Aucune question de seconde pose.

**Pourquoi.** Le moteur fait tout ce qu'il faut : `flow.rs:4079-4095`, branche
`ConstructionBonus::SecondBuild`, appelle `occasion_de_vendre` PUIS `affordable`
PUIS `observer` PUIS `choose_build`. Le droit est accorde et l'occasion de vendre
est ouverte. C'est le harnais de l'ecran qui l'escamote :
`web/webapp/wasm/src/lib.rs:1269` — `if affordable.is_empty() { return None; }`.
Aucun point de decision n'est cree, donc la page n'a jamais l'occasion ni de
poser la question, ni d'offrir le bouton de vente que le moteur venait pourtant
d'ouvrir.

**Etat mesure a ce moment-la** : 8 MC, 10 cartes en main, trois bleues/rouges —
Solarpunk 15 MC, Plantation 22 MC, Interplanetary Relations 35 MC. Aucune
payable a 8 MC. Mais vendre 3 cartes (+9 MC = 17 MC) mettait **Solarpunk a
portee**. Il pouvait donc bel et bien poser une seconde carte. Alexis a raison.

**Correctif** : poser la question meme quand la liste est vide (uniquement
« passer »), pour que le point de decision existe et que la vente y soit
offerte. C'est le MEME defaut que la seconde moitie de K3 (« aucune carte
constructible cette phase » dit en silence) — un seul correctif reglera les deux.

**PAS PENDANT UNE PARTIE EN COURS** : cela AJOUTE des entrees dans la liste des
decisions, ce qui decale toutes les reponses enregistrees. Destruction certaine
au rejeu.

**Contournement utilisable tout de suite** : vendre PENDANT la question de la
premiere pose, ou le bouton existe. Les MC gagnes comptent pour la seconde pose,
qui sera alors proposee.

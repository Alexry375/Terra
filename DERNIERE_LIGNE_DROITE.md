# Dernière ligne droite — version du 04-08 au soir

Cette liste **remplace** la précédente. Tout ce qui était fait et vérifié en a
été retiré : la trace des travaux terminés reste dans `docs/JOURNAL.md` et dans
`docs/CTO_STATE.md`. Ici ne figure que **ce qui reste à faire**.

Trois sources :
- la partie à deux du 04-08 (défauts relevés en direct, anciens repères K1 à K10) ;
- les notes prises par **Corentin** pendant cette même partie
  (`~/Téléchargements/temp/Terra.txt`) ;
- les défauts anciens jamais traités (anciens repères I, E, J).

Convention : `[VÉRIFIÉ JJ-MM]` = relu à la source ou mesuré, avec le
`fichier:ligne`. `[DÉCLARÉ]` = dit par quelqu'un, pas encore prouvé.
`[À VÉRIFIER]` = je n'ai pas encore regardé.

Les identifiants sont neufs et parlants. L'ancien repère est rappelé entre
parenthèses quand il existe.

## 0. Questions — état après les réponses d'Alexis du 04-08 au soir

| Repère | Réponse |
|---|---|
| Q1 — « pas clair jauge temp » | **Tranché le 05-08 par Corentin** : retirer le nombre porté par chaque jauge — les degrés à gauche, la valeur d'oxygène à droite. Voir LIS-1 réécrit. |
| Q2 — « retirer interface au milieu » | **Abandonné le 05-08.** Corentin a dit de laisser tomber. Rien à faire, LIS-9 est supprimé. |
| Q3 — défausser pour 3 MC | **Tranché**, voir MOT-7 réécrit. |
| Q4 — qualité des objectifs | Pas de meilleure source. Décision à prendre, voir LIS-4. |
| Q5 — croix ou coche | **On désigne les cartes qu'on JETTE.** La croix est donc juste. |
| Q6 — choix de phase | **Aucune fuite d'information** aujourd'hui. À corriger côté écran seulement, voir MOT-9 réécrit. |
| Q7 — doublon des forêts | Garder l'hexagone, retirer la ligne du score, **et poser le jeton détouré**. |

Plus aucune question n'est ouverte.

## 1. MOTEUR — les règles elles-mêmes

Ce lot **recompile le moteur** et casse la compatibilité des parties
enregistrées : les réponses d'une partie sont des **numéros de position** dans
la liste des choix, pas des noms. Ajouter, retirer ou déplacer un choix décale
tout ce qui suit. **Ces travaux se font donc en un seul lot, hors partie**, avec
une seule campagne de contrôles.

### MOT-1 (ancien K8) — Une question sautée quand rien n'est payable — FAIT
**[FAIT 05-08, fusionné dans `main` (`ff40503`), audité.]** La question est
désormais posée même sans carte payable, avec « passer » pour seule issue et
l'occasion de vendre ouverte. Mesuré : 4 232 questions de ce type sur douze
graines — le cas était fréquent, pas rare.

[VÉRIFIÉ 04-08] `web/webapp/wasm/src/lib.rs:1269` — `if affordable.is_empty()
{ return None; }`. Quand aucune carte n'est payable, aucun point de décision
n'est créé : la seconde pose du bonus Construction n'est jamais proposée, et
l'occasion de vendre que le moteur venait pourtant d'ouvrir est perdue avec elle.

Vécu en partie (mars2, graine 210055, rangs 144-146) : 8 MC, trois cartes
bleues/rouges en main dont Solarpunk à 15 MC. Vendre 3 cartes (+9 MC) la mettait
à portée. La question n'a jamais été posée.

**Correctif** : poser la question même quand la liste est vide — avec la seule
réponse « passer ». Règle du même coup MOT-4.

### MOT-2 (ancien K5) — Une action de carte impossible reste proposée — ✅ RÉPARÉ 05-08
**[VÉRIFIÉ 05-08 · réparé, fusionné, rejoué par moi]** Correctif dans le lot
« les choix se posent au bon moment » (`c071409`). `action_peut_produire` est
le miroir exact des sorties par refus d'`apply_action_spec`, écrit une seule
fois et lu par `action_options`. **Mesure d'après, rejouée par moi : 1419
options d'action essayées sur 5 parties entières, 0 stérile** (avant : 2133
options, 340 stériles). Le prédicat n'est pas placé au sommet
d'`apply_action_spec` parce que ce corps-là offre l'occasion de vendre avant de
juger la payabilité — un prédicat ne doit pas déclencher d'effet de bord.
Contre-témoin gardé : une action qui pose une ressource **sur une carte**
produit quelque chose et reste proposée (*Birds*).

**Reste ouvert, déclaré par l'agent, autre lot** : le prédicat ne garde que
l'océan au maximum, pas la température ni l'oxygène. Une action « 4 chaleurs →
+1 oxygène » à oxygène plein prend les chaleurs et ne rend rien.

Ce qui suit est l'état d'avant, gardé pour mémoire.

[VÉRIFIÉ 04-08] Les neuf océans sont révélés et « Aquifer Pumping » est encore
offerte. `engine/src/flow.rs:3291` refuse l'effet avant tout paiement — aucun MC
n'est perdu — mais la boucle de la phase Action consomme l'activation « dans
tous les cas » (`flow.rs:4198`) : le joueur perd son droit d'action pour rien.

**Correctif** : dans `action_options` (`flow.rs:3123`), ne pas proposer une carte
bleue dont l'action ne peut rien produire, exactement comme l'action standard
Océan l'est déjà (`flow.rs:3146`).

⚠️ **[VÉRIFIÉ 05-08 · mesuré par moi] LE DÉFAUT EST BIEN PLUS LARGE QUE CETTE
FICHE NE LE DISAIT.** *Aquifer Pumping* n'est qu'un cas sur seize. En rejouant
chaque option d'action possible à chaque point de décision — l'état du moteur est
reproductible, on peut donc essayer toutes les branches sans jouer la partie
plusieurs fois — sur 5 parties entières :

```
2133 options d'action essayées
 340 ne changent RIEN du tout (16 %) et n'ouvrent aucune question de suite
```

Réparties sur **seize cartes** : *Wood Burning Stoves* (66), *Volcanic Pools*
(64), *Steelworks* (49), *Aquifer Pumping* (37), *Symbiotic Fungus* (32),
*Decomposing Fungus* (29), *Ironworks* (23), *Solarpunk* (18), *Water Import from
Europa* (13), *Think Tank* (5), *Farmers Market* (4), et quelques autres. La
cause commune n'est pas seulement « les neuf océans sont sortis » : c'est aussi
« le joueur n'a pas la ressource qu'il faut dépenser ».

**Un piège que ma propre mesure a d'abord raté**, et qui vaut pour qui écrira le
correctif : une action qui pose un microbe sur une carte ne change ni les
mégacrédits, ni la planète, ni le nombre de cartes en jeu. Ma première mesure la
comptait donc comme stérile, à tort — *Tardigrades* apparaissait avec 27 cas qui
ont tous disparu une fois les ressources posées sur les cartes prises en compte.
Toute vérification doit regarder **les cartes posées elles-mêmes**, pas seulement
leur nombre.

### MOT-3 (ancien K6) — Le bonus de la phase Construction est tranché trop tôt — ✅ RÉPARÉ 05-08
**[VÉRIFIÉ 05-08 · réparé, fusionné, rejoué par moi]** Deux temps, deux
questions. Premier temps, avant toute pose : « piocher tout de suite, avant de
poser ? », deux issues. Puis, la première carte posée : « piocher, ou poser une
seconde carte ? », deux issues. **Mesure d'après, rejouée par moi : 108
questions de bonus sur 6 parties, dont 17 posées APRÈS une carte posée pendant
la phase, et 2 listes d'options distinctes** (avant : 70 questions, 0 après une
pose, 1 seule liste). L'ancienne question à trois issues est conservée pour les
chemins qui n'ont pas de moment — la sonde et les tests.

**Un défaut trouvé par la relecture adversariale de l'agent, pas par mes
contrôles** : le second temps était demandé aussi à qui avait déjà pioché au
premier.

**La fiche se trompait sur un point**, vérifié par l'agent dans la table des
cartes : la carte améliorée II-A n'a qu'une seule branche et ne pose aucune
question. Seule II-B était concernée.

Ce qui suit est l'état d'avant, gardé pour mémoire.

[VÉRIFIÉ 04-08 contre le livret et contre le code] Livret,
`docs/regles/livret-base.md:336` : « piocher une carte **avant ou après** avoir
joué une carte ». Le moteur, lui, appelle `policy.construction_bonus(...)` avant
le calcul des options de pose (`engine/src/flow.rs:3994-4005`) : les trois issues
sont arrêtées alors que le joueur n'a rien posé. Les cartes améliorées II-A et
II-B ont le même défaut (`selector_branch`, même endroit).

**Correctif visé** : au début, une question réduite (« piocher tout de suite,
avant de poser ? ») ; puis, la première carte posée, la vraie question entre
« piocher » et « poser une seconde ».

### MOT-4 (ancien K3, seconde moitié) — La phase s'arrête sans un mot — FAIT CÔTÉ MOTEUR
**[FAIT 05-08 dans le moteur (`ff40503`) — PAS ENCORE À L'ÉCRAN.]** La phrase
publiée est « No card can be built this phase. You may still sell cards from
your hand. », lue à la graine 4242, rang 17. Mais `web/webapp/vue/mots.js:268`
rédige la question de `choose_build` **en dur** et ne lit jamais `d.question` :
la phrase n'atteindra l'écran que lorsque ce fichier traitera le cas « aucune
option ». **C'est un travail du chantier « ce qu'on voit ».** Tant qu'il n'est
pas fait, le joueur voit toujours « Which card do you play? » sans aucune carte.
Au passage, le libellé `sell_card` de `mots.js:275` est devenu du code mort.

[VÉRIFIÉ 04-08] Quand la question de pose n'offre aucune option, la phase passe
en silence. Il faut dire en clair « aucune carte constructible cette phase ».
Réglé par MOT-1.

### MOT-5 (Corentin, ligne 17) — Défausser pendant la Construction donne la main à l'autre — RÉFUTÉ
[VÉRIFIÉ 05-08 · mesuré] **Ce défaut n'existe plus.** La vente ne consomme rien
et rend la main à personne : en Construction, l'occasion de vendre est posée
avant le calcul des cartes payables, puis la question de pose revient **au même
joueur** (`engine/src/flow.rs:4051-4058`). La vente a cessé d'être une action
(`engine/src/policy.rs:38-48`) et l'écran ne fabrique jamais de « passer »
(`web/webapp/vue/vente.js:211-238`).

**Mesure** : 12 graines, parties entières, on vend **partout où c'est offert** —
**1 003 ventes, dont environ 270 en phase Construction, zéro passage de main**.

Ce que Corentin a vu décrivait très probablement l'état d'avant le lot moteur
(`ff40503`, la vente quitte la phase Action).

### MOT-13 (trouvé par l'enquête du 05-08, personne ne l'avait signalé) — Vendre sa dernière carte fait disparaître une défausse imposée
[VÉRIFIÉ 05-08 · mesuré] Quand un effet de carte **impose** une défausse
(`engine/src/flow.rs:2230-2247`) et que le joueur en profite pour **vendre sa
dernière carte**, il ne reste plus rien à défausser : le moteur sort sans poser
la question (`return`, l. 2246). Conséquence — la défausse imposée s'évapore, le
joueur encaisse les 3 mégacrédits de la vente **et** échappe au coût de l'effet,
et la main passe à l'adversaire.

**Fréquence** : 1 cas sur 1 003 ventes, 1 graine sur 12 (graine 5150). Rare, mais
c'est un gain injuste, donc exploitable par une intelligence artificielle.

**Correctif** : `engine/src/flow.rs` seul, vers la ligne 2236. **Aucune partie
enregistrée cassée** — on ne déplace aucun point de décision.

⚠️ **NON REPRODUIT INDÉPENDAMMENT — retiré du lot moteur du 05-08.** J'ai écrit
un contrôle qui compare deux futurs de la même partie au même point (vendre
toute sa main devant une défausse imposée, ou ne pas vendre) et je n'ai **pas
réussi à ouvrir la situation** : sur 18 tentatives, le moteur **refuse** la
vente à ce point précis (« aucune occasion de vendre n'est ouverte à ce
point »), alors que le champ `vente_offerte` de l'état vaut pourtant *vrai*.

Mon contrôle était donc **vert par accident** : il avalait le refus et comptait
la tentative comme une réussite — exactement la faute que j'interdis aux
agents. Deux choses restent à établir avant d'en faire un lot :

1. Pourquoi `vente_offerte` vaut *vrai* alors qu'aucune occasion n'est ouverte.
   C'est peut-être **cela**, le vrai défaut : l'état annonce une possibilité que
   le moteur refuse ensuite.
2. La suite exacte de réponses qui a produit le cas pendant l'enquête (graine
   5150, vente n° 103, entrée n° 417), qui n'a pas été conservée.

### MOT-6 (Corentin, ligne 19 · recoupe MOT-1) — Vendre quand on ne peut rien payer — FAIT
**[FAIT 05-08 (`ff40503`), audité.]** L'occasion de vendre est ouverte à chaque
point de décision des phases où l'on dépense, y compris quand rien n'est
payable.

[DÉCLARÉ 04-08] Quand on n'a pas de quoi acheter une carte ou payer une action,
la seule issue offerte est « passer ». Il faut que « vendre des cartes » soit
toujours une issue possible à ces moments-là. C'est le même correctif que MOT-1,
étendu à la phase Action.

### MOT-7 (Corentin, ligne 23) — Vendre ne doit pas coûter un échange de la phase Action — FAIT
**[FAIT 05-08 (`ff40503`), audité — le plus vérifié des quatre.]** L'action
« Défausser 1 carte pour du MC » n'existe plus dans le moteur. Mesures : zéro
occurrence sur trois parties entières (466, 390 et 485 décisions) là où il y en
avait 281 ; un contrôle caché, écrit sur un chemin totalement disjoint (une
politique espionne à l'intérieur du moteur, qui compte ce que le moteur propose
sans passer par l'écran), confirme zéro sur cinq graines. Le taux n'a pas bougé :
c'est le même service unique qui crédite. Mille parties automatiques s'achèvent
sans casser aucun invariant.

[TRANCHÉ PAR ALEXIS 04-08 · Q3] Livret `docs/regles/livret-base.md:96` : « à tout
moment, vous pouvez défausser une carte Projet de votre main pour gagner 3 MC ».

**Décision d'Alexis, mot pour mot** : on garde la possibilité, mais on la retire
des **options de la phase Action**. Tout doit passer par le bouton de vente, qui
doit :
1. permettre de vendre **autant de cartes qu'on veut d'un coup** (déjà fait
   côté écran le 04-08) ;
2. **ne consommer aucun échange de la phase Action**.

Précision d'Alexis sur ce dernier point : la phase Action est un aller-retour
entre les deux joueurs, chacun agissant à son tour, et elle ne s'arrête que
lorsque les deux veulent passer. Vendre ne doit pas compter comme l'un de ces
allers-retours.

**Reste à vérifier avant de coder** : le taux de la défausse n'est pas fixe —
certaines cartes le modifient (`engine/src/flow.rs:1555`, `discard_mc_rate`).
Il faut s'assurer que le bouton de vente applique bien le **même** taux que
l'option qu'on retire, sinon on supprimerait un chemin plus rémunérateur que
l'autre.

Gain pour l'intelligence artificielle : deux chemins qui mènent au même état
gonflent l'arbre de recherche sans rien apporter. Ici on en supprime un.

### MOT-8 (Corentin, ligne 8) — Le badge « ? » se choisit trop tôt — ⚠️ DEMI-CORRECTIF 05-08, UNE DÉCISION ATTEND ALEXIS
**[VÉRIFIÉ 05-08 · demi-correctif fusionné, rejoué par moi]** Le périmètre de la
question a changé, **pas son moment** : le badge reste choisi avant le calcul du
prix, parce qu'il EST un facteur du prix. **Mesure d'après, rejouée par moi : 10
questions de badge sur 12 parties, 10 confrontées à la question de pose qui
suit, 0 inutile** (avant : 18 questions, 10 inutiles).

Le code a été amélioré au passage, et c'est ce qui rend le défaut impossible à
revenir : `verdict_de_pose` est extrait du corps de boucle d'`affordable`, et les
**deux** lecteurs l'appellent — celui qui énumère les cartes posables
(`flow.rs:2160`) et celui qui décide s'il faut demander le badge
(`flow.rs:629`). Ils ne peuvent donc plus juger la même carte différemment, ce
qui était exactement le défaut.

**⛔ LA DÉCISION QUI ATTEND ALEXIS — c'est une règle de jeu, pas de la
technique.** Le contrat que j'avais écrit se contredisait, l'agent l'a mesuré au
lieu de le supposer, et a retenu la lecture la plus stricte : la carte est jugée
sur les **dix** badges, et la question n'est posée que si les dix la laissent
posable. Conséquence assumée : **une carte payable seulement sous certains
badges ne reçoit plus de question, donc plus de jeton, donc est jugée plein tarif
et n'est pas offerte** — le joueur perd ce coup-là pour ce tour, et ne le
retrouve que lorsqu'il devient assez riche pour la payer quel que soit le badge.

L'autre lecture, « juger au badge le plus favorable », rend le coup au joueur mais
rouvre la question inutile : mesuré, graine 808 rang 39, *Topographic Mapping* à
10 MC devant un joueur à 8 MC et 1 savoir-faire Acier — deux badges sur dix la
rendent payable, huit non ; le joueur en choisit un des huit et la carte n'est
pas offerte. Et le jeton, une fois posé, **reste sur la carte pour toute la
partie** : un mauvais badge la dégrade définitivement.

Se renverse en changeant `all` en `any` dans `posable_quel_que_soit_le_badge`
(`engine/src/flow.rs:628`), le coût étant écrit dans le code à cet endroit.

**Le correctif complet — demander le badge au moment de poser, ce que Corentin
demandait — n'est PAS fait, délibérément.** Il pose le même problème de règles,
en pire : le prix annoncé ne serait plus le prix payé.

**Un défaut trouvé par la relecture adversariale de l'agent, pas par mes
contrôles** : le badge pouvait encore être demandé à la pose, l'interdit
explicite du contrat, parce que la vente enrichissait le joueur entre la
résolution et l'énumération. Reproduit avant d'être corrigé : 4 cas sur 263
offres, en 300 parties, avec une politique qui vend à chaque occasion. Mes
contrôles ne vendent jamais — c'est pourquoi leurs chiffres sont identiques
avant et après ce correctif-là.

Ce qui suit est l'état d'avant, gardé pour mémoire.

[VÉRIFIÉ 05-08 · mesuré] Corentin a raison, et le titre de la fiche était à
l'envers : le choix se fait trop **tôt**, pas trop tard. Le moteur parcourt
**toute la main** et fait choisir le badge de chaque carte joker avant même de
savoir si le joueur veut la jouer (`engine/src/flow.rs:489-502`, point de
décision l. 466).

**Mesure** : 14 graines, parties entières. Les trois cartes concernées sont
*Local Market*, *Political Influence* et *Topographic Mapping*. **17 choix de
badge, 17 sur 17 posés alors que la carte est encore en main, et 7 sur 17
(41 %) portent sur une carte qui ne sera jamais jouée de la partie.**

**Pourquoi c'est écrit ainsi**, et c'est une vraie raison (`flow.rs:420-427`) :
le badge change le **prix** de la carte, par les savoir-faire Titane et Acier. Le
moteur doit donc connaître le badge pour savoir si la carte est payable — et
payer le même prix que celui qu'il a annoncé.

**Correctif** : `engine/src/flow.rs`. Deux chemins possibles, à trancher — soit
calculer le prix avec le meilleur badge possible puis poser la question à la
pose, soit poser la question à la pose et recalculer le prix avec un chemin de
renoncement si le joueur ne peut plus payer. Un demi-correctif à coût moindre
existe : ne faire choisir le badge que des cartes **payables**, ce qui supprime
les 41 % inutiles.

⚠️ **Ce correctif casse toutes les parties enregistrées** : supprimer ou déplacer
des points de décision décale la liste ordonnée des réponses. À grouper dans le
même lot que MOT-3, qui a le même effet, pour ne refixer les empreintes qu'une
seule fois.

### MOT-9 (Corentin, ligne 14) — Les deux joueurs doivent choisir leur phase en même temps
[TRANCHÉ PAR ALEXIS 04-08 · Q6] Le livret demande un choix **simultané et face
cachée** (`livret-base.md:268` et `:629`). Notre écran fait attendre le second.

**Alexis a écarté ma crainte** : dans l'état actuel, le choix du premier n'est
pas révélé au second — il n'y a donc **aucune fuite d'information**, seulement
une attente inutile. Ce n'est pas un défaut de règle mais de confort.

**Correctif retenu** : les deux joueurs choisissent en même temps ; si le moteur
exige de recevoir les réponses l'une après l'autre, l'écran garde la seconde de
côté et l'envoie ensuite. Le travail est donc dans l'écran et le relais, **pas
dans le moteur** — cela peut se faire hors du gros lot.

### MOT-10 (Corentin, lignes 18 et 20) — La production affichée ignore les cartes à badges — ✅ RÉPARÉ 05-08
[VÉRIFIÉ 05-08 · fusionné] `production.mc_reel` publie le revenu réel de la
prochaine phase Production, depuis le service unique `flow::derived_production`.
L'écran l'affiche à côté de la production de base.

**Ce que l'enquête a appris en plus, et qui n'était pas dans la fiche** : le
bonus du joueur qui choisit la phase Production n'a pas un montant mais
**trois**, selon la carte Phase IV qu'il détient — **+4** avec la carte de base,
**+1** avec la version améliorée dite IV-A (plus le rejeu de la production d'une
carte verte, dont le montant dépend d'un troisième choix), **+7** avec IV-B
(`effects.rs:3029-3042`, `3188-3202`, `3204-3217`, versés par `flow.rs:4284`).
Tous les trois sont **hors du nombre annoncé**, pour la raison écrite plus bas :
ils dépendent d'une phase pas encore choisie. Deux champs déjà publiés
permettent à un observateur de les retrouver : `chosen_phase` et
`phase_upgrades`.

**Preuve de complétude** : dans toute la phase Production il n'existe que
**deux** endroits qui versent des mégacrédits (`flow.rs:4291` et le rejeu de
IV-A en `flow.rs:4309`). Le nombre annoncé couvre le premier moins le bonus.
Rien d'autre ne peut manquer.

[VÉRIFIÉ 05-08 · mesuré, constat d'origine] Confirmé **dans les deux moitiés** : les badges ET les
jetons Forêt. Le moteur ne publie que la piste de production de base
(`engine/src/observe.rs:77-82`), alors qu'il verse en plus tout ce qui dépend des
badges et des forêts (`engine/src/flow.rs:4289-4293`, calculé par
`derived_production`, `flow.rs:2309-2321`). Ces montants ne touchent jamais la
piste, donc rien ne les affiche. Quatorze cartes sont concernées.

**Mesure**, sonde sur la vraie phase Production :

| carte | annoncé | réellement versé | écart |
|---|---|---|---|
| *Cartel* (1 MC par badge Terre) | 0 | 1 | −1 |
| *Cartel* × 2 | 0 | 4 | −4 |
| *Lightning Harvest* (1 MC par badge Science) | 0 | 1 | −1 |
| *Zeppelins* (1 MC par jeton Forêt), 2 forêts | 2 | 2 de plus | −2 |
| *Immigration Shuttles*, production fixe — **témoin** | 3 | 3 | **0, correct** |

Le témoin prouve que le compteur est juste pour les productions fixes : ce n'est
donc pas le compteur qui est cassé, c'est une moitié du calcul qui n'est jamais
publiée. Sur une partie réelle (graine 5150), un joueur voit **5** alors que la
phase suivante lui versera **7**.

**Correctif : peu coûteux.** Le nombre existe déjà et se lit
(`flow::derived_production` est publique) : un champ à ajouter dans
`engine/src/observe.rs`, puis `web/webapp/vue/joueurs.js`. **Aucune partie
enregistrée cassée** — publier un nombre ne pose aucun point de décision.

**Une seule chose à trancher** : le bonus de la phase Production dépend d'une
phase que les joueurs n'ont pas encore choisie (`flow.rs:4282-4284`). Le nombre
annoncé sera donc juste **hors bonus**. C'est le seul choix honnête : annoncer un
bonus qui dépend d'un choix non fait serait deviner.

**[DÉCLARÉ, texte d'origine]** Le compteur de production de MC affiché ne comprend
pas les cartes qui produisent des MC **selon le nombre de badges** — ni,
probablement, celles qui dépendent du nombre de jetons Forêt.

Demandé : une case supplémentaire, sous la production de MC, donnant **le revenu
réel de la prochaine phase Production** — production de base, plus points de
terraformation, plus tout ce qui dépend des badges et des jetons.

C'est un travail d'affichage, mais le nombre doit venir du **moteur** : il n'y a
qu'un seul endroit qui a le droit de calculer, et ce n'est pas la page.

### MOT-11 (ancien E2) — Le joueur ne choisit pas quelle tuile océan retourner — CONFIRMÉ, mais le correctif moteur est DÉCONSEILLÉ
[VÉRIFIÉ 05-08 · lu dans le livret et dans le code] Le moteur mélange les neuf
tuiles une seule fois, à la mise en place (`engine/src/flow.rs:75-76`), puis les
prend **dans l'ordre** de ce mélange (`flow.rs:2984-2994`). Aucun point de
décision océan n'existe. Les tuiles ne sont pas interchangeables : sept
récompenses différentes sur neuf (`engine/src/state.rs:59-68`).

**Mais le livret tranche la question, et pas dans le sens attendu.** Il dit bien
« choisissez une tuile Océan » (`docs/regles/livret-base.md:72`) — seulement il
dit aussi, ligne 203 : « Mélangez les tuiles **face bleue cachée** et placez-les
**aléatoirement** ». **On choisit à l'aveugle**, et aucun emplacement ne porte la
moindre information. Le tirage du moteur reproduit donc **exactement** la loi du
livret : le choix du joueur ne change rien, ni au résultat, ni à son espérance.

**Recommandation : ne toucher au moteur pour aucun prix.** Le choix visuel — le
joueur désigne une tuile parmi celles qui restent, et l'on retourne celle que le
moteur avait déjà tirée — coûte `web/webapp/vue/plateau.js` et une feuille de
style, **zéro ligne de moteur, zéro partie enregistrée cassée**, et donne au
joueur exactement le geste du livret. Un vrai point de décision, lui, casserait
toutes les parties enregistrées pour un choix qui ne décide de rien.

L'écran a déjà tout ce qu'il lui faut : l'identité des tuiles révélées est
publiée (`engine/src/observe.rs:170`).

### ⚠️ Trouvaille hors fiches (05-08) — le document du cerveau distant ne parle pas de la vente
[VÉRIFIÉ 05-08] Un joueur ne peut vendre **qu'une seule fois par point de
décision** ; une seconde tentative au même point est refusée
(`web/webapp/wasm/src/lib.rs:830`). L'écran s'en garde par un verrou
(`vue/vente.js:122` et `:269`), mais **`web/webapp/adversaire.md` — le document
qui explique à un cerveau distant comment jouer — ne dit pas un mot de la
vente**. Une intelligence artificielle branchée dessus empoisonnerait la partie
sans le savoir. À corriger avant GRO-1.

### MOT-14 (05-08) — Le badge choisi pour un joker n'est jamais publié — ✅ RÉPARÉ 05-08
[VÉRIFIÉ 05-08 · fusionné] Une carte posée publie `joker`, la chaîne de
`Tag::as_str()` — exactement celle que le point de décision met dans les options
qu'il propose, donc une seule source de noms. L'écran pose un jeton **en haut à
gauche** de la carte posée : le coin haut-droit est recouvert par la carte
suivante de la pile, leçon tirée de LIS-3. **LIS-6 est donc débloqué et fait.**
Contrôle : 18 choix de badge observés, 12 cartes posées ensuite, aucune muette.

[VÉRIFIÉ 05-08, constat d'origine] Bloque **LIS-6** (« rien ne dit quel badge a été choisi »),
déclaré tel quel par le chantier d'affichage et vérifié par moi.

Le moteur **sait** quel badge a été choisi pour chaque carte joker : il le range
carte par carte (`engine/src/state.rs:357`, `joker_tags`, une entrée par carte et
jamais par joueur, jamais réécrite une fois posée). Mais il ne le publie nulle
part : une carte posée sort avec **cinq champs et pas un de plus** — couleur,
identifiant, nom, prix, ressources (`engine/src/observe.rs:103-108`).

L'écran ne peut donc rien afficher, ni pour soi ni pour l'adversaire, et il
aurait tort de le deviner : mémoriser sa propre réponse ne dirait rien du badge
de l'autre joueur, ni d'une partie reprise en cours de route.

**Correctif : une ligne de publication.** `engine/src/observe.rs`, puis
`web/webapp/vue/`. **Aucune partie enregistrée cassée** — publier un champ ne
pose aucun point de décision. À faire dans le même lot que MOT-10, qui est
exactement le même geste.

### MOT-15 (05-08) — Les points déjà rapportés par les ressources posées ne sont pas calculables à l'écran — ✅ RÉPARÉ 05-08
[VÉRIFIÉ 05-08 · fusionné] Une carte posée publie `pv_ressources`, pris dans
`flow::card_points` — la fonction que le décompte général appelle déjà carte par
carte. Aucun barème recopié nulle part : le contrôle caché « un seul endroit
compte » le vérifie et il est vert. L'écran l'affiche sur la **carte agrandie**,
là où il y a la place de l'écrire en toutes lettres. **La seconde moitié de
LIS-3 est donc faite** ; la première (les pastilles recouvertes) reste ouverte.

Un défaut trouvé en chemin et réparé : l'adaptateur des cartes perdait le compte
de ressources au **second** passage, et la loupe repasse par lui — la carte
agrandie, celle qu'on ouvre justement pour lire, était donc la seule à ne rien
dire.

[VÉRIFIÉ 05-08, constat d'origine] Bloque la **seconde moitié de LIS-3** (« quand on agrandit une
carte, afficher les points de victoire que ses ressources rapportent déjà »).

Le moteur publie `resources` : un **compte** (« 4 »), et rien d'autre. Ni la
nature de ces ressources (microbe, animal, jeton Science), ni le barème propre à
la carte (« 1 point par animal », « 1 point pour 2 microbes »), ni le total qui
en découle. L'écran ne peut pas le calculer sans recopier le barème de chaque
carte — ce qui créerait un **second endroit qui compte les points**, à côté de
`flow::score_breakdown`. Deux comptes finissent toujours par diverger.

**Correctif** : publier le nombre depuis le service unique de comptage,
`engine/src/observe.rs`. **Aucune partie enregistrée cassée.** Même lot que
MOT-10 et MOT-14.

### MOT-16 (05-08) — Un badge joker s'affiche alors que personne ne l'a choisi à l'écran — ✅ ÉLUCIDÉ 05-08, ce n'est pas un mensonge
**[VÉRIFIÉ 05-08 · chaîne de code relue par moi]** **Le badge a bien été choisi
par quelqu'un : par l'adversaire tenu par le programme.** L'écran n'invente
aucun badge. Ce qu'il fait, c'est **ne rien dire** : la question posée au siège
d'en face n'apparaît jamais dans la bande de décision, et pourtant le jeton
qu'elle produit se dessine sur la table adverse.

Chaîne de preuve, relue à la source :

- `engine/src/flow.rs:451-483` — `ensure_joker_tag` est la **seule** écriture de
  `joker_tags`, et elle passe **toujours** par `policy.pick_joker_tag`. Aucune
  valeur par défaut, aucun autre chemin. Le badge ne peut donc pas apparaître
  sans avoir été demandé à une politique.
- `engine/src/observe.rs:134` — le champ `joker` n'est publié que si l'entrée
  existe. Rien n'est fabriqué à la publication.
- `web/webapp/interface.js:470` et `:343` — le siège d'en face est tenu par
  `adversaire(graine)`, un fournisseur de décisions qui répond **sans** passer
  par la bande de décision ; `:347` il redessine pourtant la scène.
- `web/webapp/vue/cartes.js:151-172` — le jeton est dessiné pour **les deux**
  tables.

**Occasions comptées** : 400 parties hors écran, 148 783 décisions, 469 questions
de badge, 282 jetons posés, **0 orphelin** — dont **137 jetons (49 %) dont la
question ne fut jamais affichée au siège 0**. Plus 13 parties au navigateur
(2 126 décisions), 6 jetons vus dont 2 sur le plateau adverse. Preuve que rien de
l'adversaire n'atteint la bande : `#scene[data-joueur]` vaut toujours `"0"`.

**Reproduction exacte** : graine 123, siège 0, rang 274 — jeton `Joker tag :
Building` sur la carte 263 du plateau d'en face, alors que le compteur
`pick_joker_tag` affiché vaut 0 sur toute la partie. Second cas : graine 10,
rang 76 (carte 231).

**Ce qui reste, et c'est un point de confort, pas de règle** : aucun événement à
l'écran n'explique l'apparition du jeton adverse. À rattacher à la liste des
animations (« on ne voit pas ce que fait l'autre »), **pas** au moteur. Aucun
correctif appliqué, dépôt intact.

### MOT-12 (ancien I2) — L'état du moteur recule parfois
[DÉCLARÉ] 20 reculs sur 183 lectures, graine 5150. Jamais expliqué. À reprendre
après le lot moteur, car les changements ci-dessus peuvent le déplacer.

## 2. ANIMATIONS — voir ce qui se passe

Demande générale d'Alexis et de Corentin, formulée plusieurs fois : **on ne voit
pas ce que fait l'adversaire, ni ce qu'on fait soi-même.** Les nombres changent,
rien ne bouge. C'est le plus gros manque de confort restant.

### ANI-1 — Les actions doivent se voir, les siennes comme celles de l'autre — ✅ FAIT 05-08
**[VÉRIFIÉ 05-08 · rejoué par moi]** Lot « les cartes qui bougent » (`cfec28a`).
Les six entrées de la liste sont traitées. **Mesure d'après, rejouée par moi sur
une partie entière jouée à l'écran : 199 événements observés, 0 sans que rien ne
bouge** — au scellement, 95 sur 199 étaient muets, dont l'oxygène (14 sur 14) et
les jetons Forêt (15 sur 15) à 100 %.

Le banc de l'agent est plus strict que le mien : il exige que le mouvement
**dise** l'événement, pas seulement qu'il y en ait un. Il a trouvé une septième
famille que personne ne mesurait — le gain de ressources posées sur une carte.

Ce qui suit est la demande d'origine.

[DEMANDÉ 04-08] Liste dictée : pose de carte, hausse de la température, hausse
de l'oxygène, dépense de MC, gain de jetons Forêt, gain de ressources sur une
carte. Chaque événement doit produire un mouvement visible, du côté du joueur
qui agit **et** du côté de celui qui regarde.

### ANI-2 (Corentin, ligne 10) — Le changement de tour ne se voit pas — ✅ FAIT 05-08
**[DÉCLARÉ par l'agent, mesuré par son banc, pas par le mien]** La barre de celui
qui prend la parole s'allume à chaque passage, et la bande d'annonce parle au
passage dont il est question. Son banc mesure **212 passages de main, 0 muet**.

⚠️ **Mon propre contrôle ne mesurait PAS ce point**, et je l'ai écrit dans le
banc plutôt que de le laisser croire : la page ne publie rien qui désigne le
joueur à qui la question s'adresse, et ma première version comptait **zéro
occasion** sur 242 décisions. Un demi-banc muet vaut moins que pas de banc.

Ce qui suit est la demande d'origine.

[DEMANDÉ] On ne comprend pas que son tour est fini et que l'autre doit choisir
sa phase.

### ANI-3 (Corentin, ligne 11) — Le début de phase ne se voit pas — ✅ FAIT 05-08
**[VÉRIFIÉ 05-08 · rejoué par moi]** La carte Phase qui s'ouvre traverse la bande
d'annonce, Production comprise. **Mesure d'après, rejouée par moi : 57 débuts de
phase, 0 muet** (32 muets au scellement).

Ce qui suit est la demande d'origine.

[DEMANDÉ] En particulier la phase de Production : on ne sait pas qu'elle commence.

### ANI-4 (Corentin, ligne 24) — Le « +3 » de la défausse passe trop vite — ✅ FAIT 05-08
**[DÉCLARÉ par l'agent, mesuré par son banc]** Le gain dure **3 392 millisecondes
au plus court** sur 130 gains chronométrés. Le vrai défaut n'était pas la durée
réglée mais l'**empilement** : deux gains rapprochés se chassaient l'un l'autre,
et le plus court durait **6 millisecondes**. Ils s'empilent désormais.

Déclaré par l'agent, et c'est juste : la correction vaut pour **tous** les « +N »
de tous les bacs, pas seulement celui de la défausse — il n'y a qu'un seul point
d'affichage.

Ce qui suit est la demande d'origine.

[DEMANDÉ] Rallonger la durée d'affichage du gain quand on défausse une carte.

### ANI-6 (Alexis, 04-08) — Les pioches et les défausses doivent se voir — ✅ FAIT 05-08
**[VÉRIFIÉ 05-08 · rejoué par moi]** Un porte-cartes de deux paquets se pose dans
la colonne de droite. Une carte piochée en vient, une carte défaussée y va, des
deux côtés de la table. **Mesure : 40 pioches et 49 défausses, aucune muette**,
le trajet faisant plus de 200 points d'écran.

**Vérifié à la source par moi** : il n'existe qu'**une seule** fabrique qui pose
un objet dans la couche de vol (`web/webapp/vue/anim.js:195`), et elle pose la
marque à chaque fois. Aucun second mécanisme de vol n'a été écrit à côté.

Ce qui suit est la demande d'origine.

[DEMANDÉ] Deux mouvements, visibles **chez soi et chez l'adversaire** :
- **pioche** : la carte arrive par la **droite de l'écran** et rejoint la main ;
- **défausse** : la carte quitte la main et rejoint la pile de défausse — c'est
  le même mouvement en sens inverse, et il désigne du même coup l'endroit où
  CNF-2 va poser la dernière carte défaussée face découverte.

Ces deux animations et la fenêtre de défausse se tiennent : à faire ensemble.

### ANI-5 (Corentin, ligne 9 · anciens E1, E3, J2) — Les océans — ✅ EN GRANDE PARTIE FAIT, LA FICHE ÉTAIT PÉRIMÉE
**[VÉRIFIÉ 05-08 · relu à la source et mesuré par moi]** Cette fiche disait
« TOUJOURS PAS RÉGLÉ ». **C'était faux depuis le 04-08** : le commit `4f7ebdb`
(« le retournement se voit en grand, et le joueur choisit quelle tuile ouvrir »)
avait déjà traité les points 2 et 3, et le côté écran de MOT-11 avec.

- **Point 2 — la grande tuile montrait son dos des deux côtés : RÉGLÉ.** Elle
  paraît de dos, se retourne, dit son bonus en toutes lettres, puis s'efface
  (`web/webapp/vue/plateau.js:600-650`). L'annonce ne peut rien bloquer : elle
  laisse passer le clic.
- **Point 3 — il manquait l'animation de retournement : RÉGLÉE**, et le vrai
  problème était ailleurs. La planche fait cent dix points de large dans un coin
  de l'écran : un demi-tour y passe inaperçu quoi qu'on fasse. D'où le
  retournement **aussi en grand, au milieu de l'écran**.
- **Le côté écran de MOT-11 est fait** : le joueur désigne la tuile qu'il veut
  ouvrir (`data-ocean-choisissable`), et le moteur n'a pas été touché.
- **Point 1 — les tuiles qui se retournent à nouveau au rechargement : le
  garde-fou est en place, mais JE NE L'AI PAS MESURÉ.** `enRattrapage()` existe
  et est lu aux deux endroits qui mettent en scène (`plateau.js:531` et `:577`) :
  pendant le rattrapage les tuiles sont **posées, sans un mot**. [DÉCLARÉ] tant
  qu'une partie en ligne rechargée ne l'aura pas confirmé.

**Mesuré par moi le 05-08** : le banc `web/webapp/verif/oceans.py` rejoué, **129
planches lues, 610 identités vérifiées** contre la table du moteur, jusqu'à neuf
tuiles révélées, aucun défaut.

⚠️ **Ce banc est un quatrième banc cassé du dépôt** : il cherche la page à
`outputs/web/webapp`, un chemin de chantier qui n'existe pas dans le dépôt. Il ne
tourne qu'avec un lien de circonstance. À réparer avec `arcs.py`, `score.py` et
`corporation.py`.

Ce qui suit est la fiche d'origine, périmée.

[CONFIRMÉ PAR ALEXIS 04-08 · TOUJOURS PAS RÉGLÉ] Trois choses, liées :
1. au rechargement de la page, les tuiles océan **déjà retournées se
   retournent à nouveau** — l'animation rejoue tout l'historique ;
2. la grande tuile montrée au milieu de l'écran affiche **son dos des deux
   côtés** (ancien J2) ;
3. il manque l'animation de retournement elle-même au moment de la révélation.

Le point 1 est le plus gênant : c'est le seul défaut visible à chaque
rechargement. Cause probable, à confirmer : la page rejoue toute la partie au
chargement et déclenche les animations du passé au lieu de partir de l'état final.

## 3. LISIBILITÉ — comprendre ce qu'on voit

### LIS-1 (Corentin, ligne 5) — Retirer le nombre porté par les deux jauges
[VÉRIFIÉ 05-08 · Q1 tranché] Corentin veut qu'on **retire la valeur en degrés de
la jauge de température, et la valeur d'oxygène de la jauge d'oxygène**.

Ce qui existe aujourd'hui, relu dans le code : chaque jauge courbe porte un
nombre dans son creux, écrit par `web/webapp/vue/arcs.js:246-250` (l'élément de
classe `arc__n`, mis en forme par `web/webapp/style-monde.css:138-146`). La
température y écrit ses degrés (« -22 »), l'oxygène sa valeur (« 5 »).

**Le même nombre est déjà écrit ailleurs**, dans la barre du haut
(`web/webapp/vue/monde.js:81-91`) : « TEMPERATURE -22 °C / +8 » et
« OXYGEN 5 / 14 ». Le retrait ne fait donc **perdre aucune information** : il
supprime un doublon, qui est très probablement la cause du « pas clair » —
deux nombres pour une seule grandeur, à deux endroits de l'écran.

Travail : retirer l'élément `arc__n` des deux jauges (sa création, sa mise à
jour et son style), et retirer l'unité devenue inutile du nom de la jauge —
`arcTemp: "Temperature °C"` et `arcOxygen: "Oxygen %"`
(`web/webapp/vue/mots.js:110-111`) deviennent « Temperature » et « Oxygen ».
Garder les cases de couleur et le marqueur : c'est tout ce qui reste pour lire
la jauge.

**Conséquence à ne pas manquer** : le marqueur devient le **seul** repère de la
jauge. LIS-2 (marqueur blanc sur fond blanc, invisible en haut de piste) cesse
d'être un confort et devient obligatoire. **Les deux se font ensemble.**

Rappel de ce qui est acquis [VÉRIFIÉ 04-08] : le moteur est juste. Température
20 crans (6 violets, 5 rouges, 5 jaunes, 4 blancs), oxygène 15 crans (3, 4, 5,
3), et les prérequis se testent bien par **couleur** et non par numéro de case
(`engine/src/flow.rs:1462-1471`). Il n'y a donc rien à corriger dans les règles :
tout se joue à l'affichage.

### LIS-2 (Corentin, ligne 21) — Le marqueur des jauges est blanc sur blanc
[DEMANDÉ] Les cases hautes des deux jauges sont blanches, le marqueur aussi : on
ne le voit plus. Le passer en noir. Et Corentin préfère **un simple point** au
point cerclé actuel — il présente cela comme un avis, pas comme une exigence.

### LIS-3 (Corentin, ligne 22) — On ne voit pas les ressources posées sur les cartes — EN GRANDE PARTIE FAIT, PAS FINI
[DEMANDÉ] Les microbes, animaux et jetons Science accumulés sur une carte ne se
voient pas. Demandé en plus : quand on agrandit une carte, afficher **le nombre
de points de victoire que ses ressources rapportent déjà**, pour les cartes dont
les ressources valent des points.

**Première moitié — la cause était la place, pas la taille** [VÉRIFIÉ 05-08]. La
pastille existait déjà et le moteur publiait déjà le nombre. Elle était posée au
coin **haut-droit** de la carte : exactement la partie que la carte suivante de
la pile recouvre. Aucun réglage de superposition ne pouvait la sauver — elle vit
dans le plan de sa propre carte. Elle est passée en bas à gauche.

⚠️ **Mais ce n'est pas fini, et le rapport de livraison disait le contraire.**
Le banc écrit pour ce point, `web/webapp/verif/ressources-visibles.py`, rend
**18 pastilles encore recouvertes sur 330** (graine 4242, décisions 174, 209 et
237, cartes 33 et 67, recouvertes par une image). Le rapport annonçait « aucune
recouverte ». [VÉRIFIÉ 05-08] J'ai mesuré **des deux côtés** — dans la copie de
travail du chantier et sur `main` après fusion — et j'obtiens **exactement les
mêmes 18**, aux mêmes décisions : ce n'est ni la fusion, ni la taille de la
fenêtre, ni un aléa. L'explication la plus probable est que la mesure a été faite
avant le dernier réglage d'échelle et n'a pas été rejouée après.

**Ce qui reste à faire** : les cas où les piles sont serrées. La bande de carte
qui reste découverte n'y fait qu'environ treize points d'écran, et repousser
l'échelle d'agrandissement au-delà de 1,8 remet dix-huit pastilles sous leur
voisine. Ce n'est donc pas un réglage à pousser mais une disposition à revoir.

**La seconde moitié est FAITE 05-08** : MOT-15 réparé, la carte agrandie dit
maintenant ce que ses ressources rapportent déjà, en points de victoire, sans
qu'aucun barème soit recopié à l'écran. **La première moitié reste ouverte** :
`verif/ressources-visibles.py` rend toujours **18 pastilles recouvertes**,
exactement le même chiffre qu'avant ce lot — non régressé, non réparé.

### LIS-4 (Corentin, ligne 15) — Les objectifs et récompenses sont flous
⚠️ **[VÉRIFIÉ 05-08 · mesuré à l'écran par moi — la conclusion du 04-08 était
fausse, et elle nous envoyait chercher une solution coûteuse pour rien]**

**Il n'y a aucun plafond atteint. On peut agrandir presque cinq fois plus qu'à
présent sans le moindre flou.** Mesure faite sur une partie réelle (graine 7,
six tuiles à l'écran) :

| | au repos | au survol | définition disponible | part utilisée |
|---|---|---|---|---|
| un objectif | 29 × 29 points | **151 × 151** | 900 × 293 | **17 %** |
| une récompense | 29 × 29 points | **151 × 151** | 745 × 583 | **20 %** |

L'agrandissement au survol vaut `scale(5,2)` (`style-monde.css:290`). Le plafond
sans perte est de **×31 pour un objectif** (900 ÷ 29) et **×25 pour une
récompense** (745 ÷ 29). On est donc à moins du quart du possible.

**Un second défaut, plus gênant que le premier, que personne n'avait vu** : la
pastille est un **carré** de 29 points (`style.css:284`), alors qu'un objectif
imprimé est un **rectangle très allongé, trois fois plus large que haut**
(900 × 293). Avec l'ajustement « tout faire tenir dedans » (`object-fit:
contain`), l'image d'un objectif n'occupe donc que **29 × 9 points au repos, et
151 × 49 au survol** : les deux tiers de la place sont perdus en haut et en bas,
et c'est cela qui rend le texte illisible bien avant que la définition ne
manque.

**Conséquence sur les trois voies écrites le 04-08 : les voies 1 et 3 sont
inutiles.** Ni ressource de meilleure qualité à chercher, ni agrandissement par
intelligence artificielle. Il suffit d'agrandir davantage et de donner à
l'agrandissement le **bon rapport largeur-hauteur**. La voie 2 (réécrire le
texte net par-dessus) reste souhaitable mais devient un confort, plus un
sauvetage.

**Attention si l'on réécrit le texte** : le jeu s'affiche **en anglais**, comme
les cartes imprimées d'Alexis et Corentin. Le fichier
`objectifs-recompenses.json` est en **français** — il ne peut servir que de
source de sens, pas de source d'affichage. Les noms anglais sont dans les noms
des ressources (`jalon-space-baron-…`, `recompense-industrialist-…`).

Ce qui suit est l'analyse du 04-08, conservée pour mémoire ; **son plafond de
745 points est exact mais on en est très loin, donc sa conclusion ne tient
pas.**

**D'où viennent vraiment ces images** [VÉRIFIÉ 04-08 · Alexis avait raison,
je m'étais trompé de source]. Le manifeste le dit noir sur blanc :
`web/webapp/assets/manifeste.json` → `"source":
"data/scans/decouverte-tabletop/img_9c88384b7936.png"`. Ce ne sont **pas** les
photos d'Alexis, ce sont les ressources du module de plateau virtuel. Ce fichier
mesure **745 × 583** et notre image livrée mesure **745 × 583** : il n'y a eu
**aucun agrandissement**, c'est la taille d'origine de la ressource.

Le plafond est donc net : **745 points de large**. Au-delà, quoi qu'on fasse,
c'est flou. C'est peu pour un agrandissement plein écran, et cela explique
exactement ce que Corentin voit.

**Trois voies, mon avis sur chacune :**
1. **Chercher une ressource de meilleure définition** — les modules de plateau
   virtuel existent souvent en plusieurs qualités, et les planches complètes
   sont parfois bien plus fines que les découpes. **À tenter en premier : c'est
   gratuit et sans risque.**
2. **Réécrire le texte par-dessus** — on connaît déjà le texte exact de chaque
   tuile (`data/cartes-imprimees/objectifs-recompenses/objectifs-recompenses.json`).
   Le nom et la condition seraient alors parfaitement nets à n'importe quelle
   taille, l'image ne servant plus que de fond. **La solution la plus sûre**, et
   elle se cumule avec les deux autres.
3. **Agrandissement par intelligence artificielle** — déconseillé seul : ces
   outils ne récupèrent pas le détail perdu, ils l'**inventent**. Sur un dessin,
   sans conséquence ; sur un chiffre de règle (« 6 badges espace »), un chiffre
   inventé affiche une règle fausse. Acceptable **uniquement** sur le fond, avec
   le texte réécrit net par-dessus (voie 2).

Les photos d'Alexis
(`data/cartes-imprimees/objectifs-recompenses/photo-objectifs-27-07.jpeg`,
1 200 × 1 600 pour toutes les tuiles à la fois) ne servent que de référence de
lecture du texte — elles ne sont pas assez propres pour l'affichage, il l'a dit
lui-même.

### LIS-5 (Corentin, ligne 13) — La disposition des tuiles océan change toute seule
[DEMANDÉ] Quand une tuile est révélée, la planche de droite passe de trois
lignes de trois à deux lignes de quatre et cinq, puis revient. Corentin trouve
la disposition en 4 et 5 plus lisible et voudrait qu'elle soit **la seule**.

### LIS-6 (Corentin, ligne 7) — Rien ne dit quel badge a été choisi — ✅ FAIT 05-08
[VÉRIFIÉ 05-08 · fusionné] Le badge choisi est posé en jeton sur la carte posée,
pour les deux joueurs, une fois MOT-14 réparé. **Un écart assumé avec la demande
d'origine** : le jeton est en **haut à gauche** et non à l'emplacement du « ? ».
Raison mesurée : dans une pile, la carte suivante recouvre le coin haut-droit —
c'est exactement le défaut que LIS-3 a mis trois mesures à faire admettre. Un
badge posé au bon endroit mais invisible ne répond pas à la demande.

**Deux limites que le banc d'écran dit lui-même** : les 725 jetons observés sont
tous du côté du joueur 1 (aucune partie mesurée n'en a posé des deux côtés), et
l'accord « badge montré = badge choisi » n'a pas pu être vérifié **à l'écran**,
parce que le badge se choisit depuis la main et que la page ne pose jamais cette
question dans les parties du banc. Cet accord est prouvé au niveau du moteur.

[DEMANDÉ, texte d'origine] Les cartes à badge « ? » ne montrent pas le badge
retenu. Demandé, idéalement : **poser le badge choisi à l'emplacement du « ? »**
sur la carte.

### LIS-7 (Corentin, ligne 4) — Une croix, pas une coche, pour le premier tri
[CONFIRMÉ PAR ALEXIS 04-08 · Q5] À cet écran, **on désigne les cartes qu'on
JETTE**. La coche actuelle dit donc le contraire de ce qui se passe. Mettre une
croix.

### LIS-8 (Alexis, 04-08) — Le compteur de jetons Forêt est affiché deux fois
[VÉRIFIÉ 04-08] Il apparaît bel et bien deux fois dans la même barre de joueur :
`web/webapp/vue/joueurs.js:121` (l'hexagone avec le nombre de forêts) et
`joueurs.js:68` (la ventilation du score, ligne « Forests »). Les deux nombres
sont égaux, puisqu'une forêt vaut un point de victoire — d'où l'impression de
doublon.

**Tranché par Alexis 04-08 (Q7)** : on **garde l'hexagone**, on retire la ligne
« Forests » de la ventilation du score, et on remplace l'image par le **jeton
détouré** — celui-ci existe déjà (`web/webapp/vue/materiel.js:281`,
`jeton-foret-detoure`), alors que la barre utilise aujourd'hui
`tuile-foret-compteur-hexagone-arbre` (`materiel.js:260`).

### LIS-9 (Corentin, ligne 12) — « retirer interface au milieu » — ABANDONNÉ
[TRANCHÉ 05-08 · Q2] Corentin a dit de **laisser tomber**. Rien à faire.

La trace est gardée ici pour une seule raison : ne pas ressortir cette demande
d'une vieille note et défaire par erreur le travail du 04-08, où l'Action
améliorée s'est mise à montrer les trois cartes tirées, y compris celles qu'on
ne peut pas prendre, éteintes et marquées « CANNOT BE TAKEN ». C'était une
demande explicite d'Alexis. **Ce comportement reste.**

### LIS-13 (Alexis, 04-08) — Ne pas pouvoir agrandir le dos des cartes adverses
[DEMANDÉ] La main de l'adversaire est faite de dos de cartes, tous identiques.
Les agrandir ne montre rien et n'a aucun intérêt : retirer la loupe sur ces
cartes-là.

### LIS-14 (trouvé par la machine le 05-08, personne ne l'avait signalé) — Le contour vert ment parfois
[VÉRIFIÉ 05-08] Le banc `web/webapp/verif/jouable.py` compare, à chaque
décision, les cartes de ma main marquées « jouable » (contour vert) et les
cartes que le moteur offre réellement. Il relève **cinq désaccords** sur une
partie entière, tous du même sens : des cartes **marquées jouables que le moteur
n'offre pas**. Exemple : décision 4, huit cartes marquées à tort.

**Ce n'est pas une régression du lot moteur** : mesuré à l'identique sur le dépôt
d'avant la fusion (même cinq désaccords, même décision 4).

Personne ne l'a signalé en partie — le contour vert trompeur ne se remarque que
si l'on essaie de jouer la carte. À traiter dans un lot d'affichage ultérieur ;
pas glissé dans le chantier en cours, qui est déjà chargé.

### LIS-10 (ancien J3) — Les logos Océan et Forêt ne sont pas détourés — ✅ FAIT 04-08, fiche périmée corrigée le 05-08
**[VÉRIFIÉ 05-08 · mesuré par moi]** Le carré blanc a disparu. Les deux images
détourées existent et sont bien celles que la plaque de décision affiche :

- `web/webapp/assets/plateau/jeton-ocean-detoure.webp` — 447 × 393, **24 % de
  pixels entièrement transparents, les quatre coins transparents** ;
- `web/webapp/assets/plateau/jeton-foret-detoure.webp` — 263 × 231, mêmes
  chiffres.

À comparer aux deux découpes d'origine, `tuile-ocean-dos-orange` et
`tuile-foret-compteur-hexagone-arbre` : **0 % de transparent, aucun coin
transparent** — c'est le rectangle blanc du reproche. Elles restent dans les
ressources, mais la plaque de décision ne s'en sert plus.

Chaîne d'affichage relue : `vue/scene.js:1030-1031` (`matiereAction`, la plaque
de décision) appelle `jetonForetDetoure()` / `jetonOceanDetoure()`, définies en
`vue/materiel.js:277-283`. Le jeton qui s'envole (`vue/anim.js:675`) et
l'hexagone des capacités (`vue/joueurs.js:145`) prennent la même image détourée.

**Pourquoi la fiche disait le contraire** : elle datait du 04-08 au matin, et le
détourage a été fait le 04-08 dans la journée. Deuxième fiche périmée trouvée en
deux jours — c'est pourquoi je remesure au lieu de recopier.

### LIS-11 (ancien I3) — Le prix d'origine n'est pas barré
[DEMANDÉ] Quand une remise s'applique, afficher le prix d'origine barré à côté
du prix réellement payé.

### LIS-12 (ancien G2) — Le remélange de la défausse ne se voit pas
[VÉRIFIÉ 04-08] Le moteur le fait bien (`engine/src/flow.rs:32-42`, livret p. 15).
Reste seulement à le **montrer** au joueur quand cela arrive.

## 4. CONFORT DE JEU

### CNF-1 (Corentin, ligne 6) — Trier sa main en déplaçant les cartes
[DEMANDÉ] Pouvoir réordonner les cartes de sa main en les faisant glisser.

### CNF-2 (ancien K4) — Voir la défausse — ✅ FAIT 05-08
**[VÉRIFIÉ 05-08 · rejoué par moi]** Les trois exigences et les précisions sont
tenues. **Mesure d'après, rejouée par moi : 49 dessus de pile différents relevés
sur une partie entière, aucun caché ; la fenêtre s'ouvre au clic, montre 152
cartes, défile, et se lit du plus récent au plus ancien.** L'option de partie
éteint la pile, la fenêtre, **et** la face de la carte qui s'envole.

**Vérifié à la source par moi** : la fenêtre lit `etat.defausse`, la liste que le
moteur publie, sans la trier ni la renverser (`web/webapp/vue/defausse.js:151`).
C'est le point qui compte : une liste tenue par la page divergerait au premier
remélange de la défausse dans la pioche.

⚠️ **Mon contrôle caché a cru trouver un écart ici, et il avait tort.** Il
comparait le nombre de cartes de la fenêtre à celui d'une partie de référence
qui jouait **466 décisions** là où l'écran en joue **172** — deux parties
différentes. Et la défausse n'est pas une pile qui ne fait que grandir : le
remélange la reverse dans la pioche, donc même le sens de l'écart ne prouvait
rien.

Ce qui suit est la spécification d'origine.

[SPÉCIFIÉ 04-08, mot pour mot] Trois exigences, dans cet ordre :

1. **La dernière carte défaussée est toujours visible, face découverte**, posée
   sur la pile de défausse.
2. **Cliquer dessus ouvre une fenêtre** montrant toutes les cartes défaussées,
   avec un défilement.
3. **L'ordre est le plus récent d'abord** : la dernière défaussée en haut à
   gauche de la grille (première ligne, première colonne), l'avant-dernière juste
   à sa droite (première ligne, deuxième colonne), et ainsi de suite. On voit
   donc immédiatement ce qui vient de partir.

**Précisions d'Alexis, 04-08 (secondes réponses) :**
- la défausse est **commune** aux deux joueurs, et c'est **voulu** : le but est
  justement de voir ce que l'adversaire a jeté ;
- **cinq cartes par ligne**, à peu près la taille d'une carte Phase au moment où
  l'on en choisit une — donc **assez grandes pour être lues sans agrandissement**.
  Pas de loupe à prévoir dans cette fenêtre ;
- ce n'est **pas une règle officielle** : ce sera une **option de partie**,
  activable ou non.

Conséquence à ne pas oublier quand viendra l'intelligence artificielle : cette
option lui profite bien plus qu'à un humain. Voir GRO-1.

### CNF-3 (Corentin, ligne 34 · optionnel) — Un bouton « passer définitivement »
[DEMANDÉ] En plus du bouton qui passe une fois pendant la phase Action, un
bouton qui passe en boucle, pour accélérer quand on est sûr de ne plus rien
faire.

### CNF-4 (Corentin, ligne 35 · optionnel) — Des messages d'attente précis
[DEMANDÉ] Au lieu de « Waiting for the other player », dire ce qu'on attend :
qu'il choisisse ses cartes, qu'il joue une carte, etc.

### CNF-5 (Corentin, ligne 36 · optionnel) — Fermer le zoom d'un clic n'importe où
[DEMANDÉ] Aujourd'hui il faut recliquer sur la tuile elle-même.

### CNF-6 (ancien I5) — Reprendre une partie interrompue
[DEMANDÉ] Aucune sauvegarde n'existe. Une partie coupée est perdue — sauf à
recopier à la main la liste des décisions, ce qu'on a dû faire une fois le 04-08.

## 5. GROS CHANTIERS

### GRO-1 (ancien I9) — L'intelligence artificielle
**C'est l'objectif du projet.** Non commencé. Tout ce qui précède existe pour que
le jeu soit jouable et juste ; l'adversaire artificiel, lui, reste entièrement à
construire.

**Point à retenir dès maintenant — la défausse visible (CNF-2).** Alexis a
demandé si cette option avantagerait la machine. Réponse : **oui, nettement**,
et pour une raison simple. Voir la défausse ne dit rien de la main adverse, mais
elle dit ce qui **reste dans le paquet**. Un humain ne mémorise pas cent
soixante-dix cartes ; une machine, si — et elle le fait sans effort ni erreur.
Cette option ne crée donc pas un avantage partagé : elle en crée un pour celui
qui sait compter.

Deux conséquences pratiques :
1. l'option doit rester **désactivable**, et l'être par défaut quand on compare
   la machine à un humain, sinon la comparaison ne veut rien dire ;
2. si on l'active, il faudra décider explicitement si la machine y a droit. Ce
   n'est pas une décision d'affichage, c'est une décision de règle du jeu.

### GRO-2 (ancien I4) — Les effets sonores
Jamais commencés.

### GRO-3 (ancien J4) — La musique de fond
[REPORTÉ] Liste demandée par Alexis :
`https://music.youtube.com/playlist?list=PLx1xajSbL3ZFd40MlNo4icK25RMMPaEsH`.
Un navigateur ne peut pas lire une liste hébergée ailleurs sans les fichiers ;
Alexis a lui-même dit qu'on abandonne si cela oblige à tout télécharger.

## 6. DÉFAUTS ANCIENS, JAMAIS REPRODUITS PROPREMENT

À reprendre seulement s'ils réapparaissent — ou à fermer si le lot moteur les
fait disparaître.

- **VIE-1** (ancien I6) — trois décisions gardent leur liste au milieu de
  l'écran. Jamais reproduit.
- **VIE-2** (ancien I7) — la main déborde en 1280 × 640.
- **VIE-3** (ancien I8) — la vente à distance : sur 18 ventes mesurées pendant
  une partie à deux, **17 se referment en moins d'une seconde, une est restée
  ouverte plus de 30 secondes** [VÉRIFIÉ 04-08]. Ce n'est pas un blocage : la
  partie va au bout et les deux écrans restent d'accord sur le score. Cause
  inconnue.

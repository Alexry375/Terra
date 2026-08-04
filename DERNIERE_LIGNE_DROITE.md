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

## 0. Ce qui attend une réponse d'Alexis

Rien de ce qui suit ne peut être fait juste : ce sont des choix, pas des défauts.

| Repère | Ce que j'ai besoin de savoir |
|---|---|
| Q1 | « pas clair jauge temp » — qu'est-ce qui n'est pas clair, au juste ? |
| Q2 | « retirer interface au milieu plutôt que griser » — quelle interface, dans quelle situation ? |
| Q3 | Faut-il retirer des phases le choix « défausser une carte pour 3 MC » ? |
| Q4 | Les objectifs et récompenses sont de mauvaise qualité d'image : a-t-on une meilleure source ? |

Le détail de chaque question est écrit dans la section correspondante.

## 1. MOTEUR — les règles elles-mêmes

Ce lot **recompile le moteur** et casse la compatibilité des parties
enregistrées : les réponses d'une partie sont des **numéros de position** dans
la liste des choix, pas des noms. Ajouter, retirer ou déplacer un choix décale
tout ce qui suit. **Ces travaux se font donc en un seul lot, hors partie**, avec
une seule campagne de contrôles.

### MOT-1 (ancien K8) — Une question sautée quand rien n'est payable
[VÉRIFIÉ 04-08] `web/webapp/wasm/src/lib.rs:1269` — `if affordable.is_empty()
{ return None; }`. Quand aucune carte n'est payable, aucun point de décision
n'est créé : la seconde pose du bonus Construction n'est jamais proposée, et
l'occasion de vendre que le moteur venait pourtant d'ouvrir est perdue avec elle.

Vécu en partie (mars2, graine 210055, rangs 144-146) : 8 MC, trois cartes
bleues/rouges en main dont Solarpunk à 15 MC. Vendre 3 cartes (+9 MC) la mettait
à portée. La question n'a jamais été posée.

**Correctif** : poser la question même quand la liste est vide — avec la seule
réponse « passer ». Règle du même coup MOT-4.

### MOT-2 (ancien K5) — Une action de carte impossible reste proposée
[VÉRIFIÉ 04-08] Les neuf océans sont révélés et « Aquifer Pumping » est encore
offerte. `engine/src/flow.rs:3291` refuse l'effet avant tout paiement — aucun MC
n'est perdu — mais la boucle de la phase Action consomme l'activation « dans
tous les cas » (`flow.rs:4198`) : le joueur perd son droit d'action pour rien.

**Correctif** : dans `action_options` (`flow.rs:3123`), ne pas proposer une carte
bleue dont l'action ne peut rien produire, exactement comme l'action standard
Océan l'est déjà (`flow.rs:3146`).

### MOT-3 (ancien K6) — Le bonus de la phase Construction est tranché trop tôt
[VÉRIFIÉ 04-08 contre le livret et contre le code] Livret,
`docs/regles/livret-base.md:336` : « piocher une carte **avant ou après** avoir
joué une carte ». Le moteur, lui, appelle `policy.construction_bonus(...)` avant
le calcul des options de pose (`engine/src/flow.rs:3994-4005`) : les trois issues
sont arrêtées alors que le joueur n'a rien posé. Les cartes améliorées II-A et
II-B ont le même défaut (`selector_branch`, même endroit).

**Correctif visé** : au début, une question réduite (« piocher tout de suite,
avant de poser ? ») ; puis, la première carte posée, la vraie question entre
« piocher » et « poser une seconde ».

### MOT-4 (ancien K3, seconde moitié) — La phase s'arrête sans un mot
[VÉRIFIÉ 04-08] Quand la question de pose n'offre aucune option, la phase passe
en silence. Il faut dire en clair « aucune carte constructible cette phase ».
Réglé par MOT-1.

### MOT-5 (Corentin, ligne 17) — Défausser pendant la Construction donne la main à l'autre
[DÉCLARÉ 04-08 · À VÉRIFIER] Défausser des cartes pendant la phase Construction
ferait passer la main à l'adversaire, **sans avoir joué de carte ni passé son
tour**. Si c'est exact, c'est un défaut grave : un joueur perd son tour de
construction pour avoir vendu.

À reproduire d'abord, puis à situer : est-ce la vente qui consomme le tour, ou
l'écran qui envoie une réponse « passer » à sa place ?

### MOT-6 (Corentin, ligne 19 · recoupe MOT-1) — Vendre quand on ne peut rien payer
[DÉCLARÉ 04-08] Quand on n'a pas de quoi acheter une carte ou payer une action,
la seule issue offerte est « passer ». Il faut que « vendre des cartes » soit
toujours une issue possible à ces moments-là. C'est le même correctif que MOT-1,
étendu à la phase Action.

### MOT-7 (Corentin, ligne 23) — Le choix « défausser une carte pour des MC » dans les phases
[QUESTION Q3 · À VÉRIFIER] Livret `docs/regles/livret-base.md:96` : « à tout
moment, vous pouvez défausser une carte Projet de votre main pour gagner 3 MC ».
Corentin fait remarquer que ce choix, proposé explicitement dans plusieurs
phases, fait double emploi avec le bouton de vente qui, lui, est censé être
disponible en permanence.

Enjeu pour l'intelligence artificielle : deux chemins qui mènent au même état
gonflent l'arbre de recherche sans rien apporter — exactement ce qu'on a évité
sur la vente multiple. **Mais** il faut d'abord vérifier que le taux est le même
partout : certaines cartes modifient le gain de la défausse
(`flow.rs:1555`, `discard_mc_rate`). Si les deux chemins ne rapportent pas la
même chose, on ne peut pas en supprimer un.

### MOT-8 (Corentin, ligne 8) — Le badge « ? » se choisit trop tard
[DÉCLARÉ 04-08 · À VÉRIFIER] Corentin croit qu'on doit choisir le badge d'une
carte à badge « ? » **avant même de la jouer**, et voudrait que le choix se fasse
**au moment où l'on décide de la jouer**. À vérifier dans le moteur : où le point
de décision est-il posé par rapport à la pose ?

### MOT-9 (Corentin, ligne 14) — Toujours le même joueur qui choisit sa phase en premier
[VÉRIFIÉ 04-08 contre le livret, code à vérifier] Le livret est formel
(`livret-base.md:268` et `:629`) : « chaque joueur choisit **simultanément** une
carte Phase et la place **face cachée** devant lui ». Notre écran fait choisir
l'un puis l'autre, et toujours dans le même ordre.

Deux conséquences : c'est contraire à la règle, et le second joueur peut déduire
quelque chose du temps de réflexion du premier. Le moteur fait pourtant tourner
le premier joueur à chaque manche (`engine/src/flow.rs:4881`) — le défaut est
donc probablement dans l'ordre d'interrogation, pas dans la règle.

### MOT-10 (Corentin, lignes 18 et 20) — La production affichée ignore les cartes à badges
[DÉCLARÉ 04-08 · À VÉRIFIER] Le compteur de production de MC affiché ne comprend
pas les cartes qui produisent des MC **selon le nombre de badges** — ni,
probablement, celles qui dépendent du nombre de jetons Forêt.

Demandé : une case supplémentaire, sous la production de MC, donnant **le revenu
réel de la prochaine phase Production** — production de base, plus points de
terraformation, plus tout ce qui dépend des badges et des jetons.

C'est un travail d'affichage, mais le nombre doit venir du **moteur** : il n'y a
qu'un seul endroit qui a le droit de calculer, et ce n'est pas la page.

### MOT-11 (ancien E2) — Le joueur ne choisit pas quelle tuile océan retourner
[DÉCLARÉ] Aujourd'hui le moteur tire au hasard. Alexis veut choisir. Facilité
qu'il a lui-même autorisée : si toutes les tuiles restantes donnent le même
résultat, le choix peut n'être que visuel. À confirmer contre le livret.

### MOT-12 (ancien I2) — L'état du moteur recule parfois
[DÉCLARÉ] 20 reculs sur 183 lectures, graine 5150. Jamais expliqué. À reprendre
après le lot moteur, car les changements ci-dessus peuvent le déplacer.

## 2. ANIMATIONS — voir ce qui se passe

Demande générale d'Alexis et de Corentin, formulée plusieurs fois : **on ne voit
pas ce que fait l'adversaire, ni ce qu'on fait soi-même.** Les nombres changent,
rien ne bouge. C'est le plus gros manque de confort restant.

### ANI-1 — Les actions doivent se voir, les siennes comme celles de l'autre
[DEMANDÉ 04-08] Liste dictée : pose de carte, hausse de la température, hausse
de l'oxygène, dépense de MC, gain de jetons Forêt, gain de ressources sur une
carte. Chaque événement doit produire un mouvement visible, du côté du joueur
qui agit **et** du côté de celui qui regarde.

### ANI-2 (Corentin, ligne 10) — Le changement de tour ne se voit pas
[DEMANDÉ] On ne comprend pas que son tour est fini et que l'autre doit choisir
sa phase.

### ANI-3 (Corentin, ligne 11) — Le début de phase ne se voit pas
[DEMANDÉ] En particulier la phase de Production : on ne sait pas qu'elle commence.

### ANI-4 (Corentin, ligne 24) — Le « +3 » de la défausse passe trop vite
[DEMANDÉ] Rallonger la durée d'affichage du gain quand on défausse une carte.

### ANI-5 (Corentin, ligne 9 · anciens E1, E3, J2) — Les océans
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

### LIS-1 (Corentin, ligne 5) — « pas clair jauge temp » — QUESTION Q1
Corentin trouve la jauge de température peu claire. Ni lui ni Alexis n'ont
précisé en quoi. Trois lectures possibles, il faut trancher :
- on ne voit pas **où en est** le marqueur (rejoint LIS-2, blanc sur blanc) ;
- on ne comprend pas **ce que débloque** chaque palier de couleur ;
- on ne voit pas **de combien** elle vient de monter (rejoint ANI-1).

Rappel de ce qui est acquis [VÉRIFIÉ 04-08] : le moteur est juste. Température
20 crans (6 violets, 5 rouges, 5 jaunes, 4 blancs), oxygène 15 crans (3, 4, 5,
3), et les prérequis se testent bien par **couleur** et non par numéro de case
(`engine/src/flow.rs:1462-1471`). Il n'y a donc rien à corriger dans les règles :
tout se joue à l'affichage.

### LIS-2 (Corentin, ligne 21) — Le marqueur des jauges est blanc sur blanc
[DEMANDÉ] Les cases hautes des deux jauges sont blanches, le marqueur aussi : on
ne le voit plus. Le passer en noir. Et Corentin préfère **un simple point** au
point cerclé actuel — il présente cela comme un avis, pas comme une exigence.

### LIS-3 (Corentin, ligne 22) — On ne voit pas les ressources posées sur les cartes
[DEMANDÉ] Les microbes, animaux et jetons Science accumulés sur une carte ne se
voient pas. Demandé en plus : quand on agrandit une carte, afficher **le nombre
de points de victoire que ses ressources rapportent déjà**, pour les cartes dont
les ressources valent des points.

### LIS-4 (Corentin, ligne 15) — Les objectifs et récompenses — QUESTION Q4
[PARTIELLEMENT FAIT] L'agrandissement au survol existe. Corentin demande
**plus gros**, et signale que les images sont alors **de mauvaise qualité**.
Il faut donc savoir si l'on dispose d'une source d'image plus fine ; sinon
l'agrandissement restera flou quoi qu'on fasse.

### LIS-5 (Corentin, ligne 13) — La disposition des tuiles océan change toute seule
[DEMANDÉ] Quand une tuile est révélée, la planche de droite passe de trois
lignes de trois à deux lignes de quatre et cinq, puis revient. Corentin trouve
la disposition en 4 et 5 plus lisible et voudrait qu'elle soit **la seule**.

### LIS-6 (Corentin, ligne 7) — Rien ne dit quel badge a été choisi
[DEMANDÉ] Les cartes à badge « ? » ne montrent pas le badge retenu. Demandé,
idéalement : **poser le badge choisi à l'emplacement du « ? »** sur la carte.

### LIS-7 (Corentin, ligne 4) — Une croix, pas une coche, pour le premier tri
[DEMANDÉ] Au tout début de la partie, quand on choisit les cartes à garder, la
marque affichée est une coche. Une croix serait plus juste — on désigne ce qu'on
écarte.
*(À confirmer en jouant : selon l'écran, on désigne peut-être ce qu'on garde,
auquel cas la coche est correcte et c'est le libellé qui doit être plus clair.)*

### LIS-8 (Alexis, 04-08) — Le compteur de jetons Forêt est affiché deux fois
[VÉRIFIÉ 04-08] Il apparaît bel et bien deux fois dans la même barre de joueur :
`web/webapp/vue/joueurs.js:121` (l'hexagone avec le nombre de forêts) et
`joueurs.js:68` (la ventilation du score, ligne « Forests »). Les deux nombres
sont égaux, puisqu'une forêt vaut un point de victoire — d'où l'impression de
doublon.

À trancher à l'affichage : garder l'hexagone (le matériel) et retirer la ligne
de la ventilation, ou l'inverse. Ma recommandation : garder l'hexagone, qui dit
combien de forêts on possède, et retirer la ligne du score, qui répète le même
nombre sous un autre nom.

### LIS-9 (Corentin, ligne 12) — « retirer interface au milieu » — QUESTION Q2
Formulation d'origine : « retirer interface au milieu plutôt que griser retirer
totalement ». Deux lectures possibles, opposées :
- **(a)** les choix impossibles s'affichent éteints au centre de l'écran ; il
  voudrait qu'ils **disparaissent** au lieu d'être éteints ;
- **(b)** le panneau central reste affiché, éteint, pendant qu'on attend
  l'adversaire ; il voudrait qu'il **s'efface** complètement.

La différence compte : la lecture (a) **annulerait** un travail fait ce matin —
l'Action améliorée montre désormais les trois cartes tirées, dont celles qu'on
ne peut pas prendre, éteintes et marquées « CANNOT BE TAKEN ». C'était une
demande explicite d'Alexis. Il ne faut pas défaire cela par erreur.

### LIS-10 (ancien J3) — Les logos Océan et Forêt ne sont pas détourés
[DÉCLARÉ 04-08] Dans les décisions, ces deux jetons s'affichent sur un carré
blanc, alors que le logo de la défausse est proprement détouré.

### LIS-11 (ancien I3) — Le prix d'origine n'est pas barré
[DEMANDÉ] Quand une remise s'applique, afficher le prix d'origine barré à côté
du prix réellement payé.

### LIS-12 (ancien G2) — Le remélange de la défausse ne se voit pas
[VÉRIFIÉ 04-08] Le moteur le fait bien (`engine/src/flow.rs:32-42`, livret p. 15).
Reste seulement à le **montrer** au joueur quand cela arrive.

## 4. CONFORT DE JEU

### CNF-1 (Corentin, ligne 6) — Trier sa main en déplaçant les cartes
[DEMANDÉ] Pouvoir réordonner les cartes de sa main en les faisant glisser.

### CNF-2 (ancien K4) — Voir la défausse
[DEMANDÉ 04-08] Pouvoir consulter la pile des cartes défaussées.

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

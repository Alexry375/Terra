# RAPPORT DE CONFORMITÉ DU MOTEUR DE RÈGLES

**Jeu audité :** *Terraforming Mars : Ares Expedition* — le jeu de CARTES,
boîte de base + extension *Discovery*. Deux joueurs.
**Ce n'est pas** le jeu de plateau *Terraforming Mars*, dont les règles sont
différentes ; aucune comparaison n'a été faite avec lui.

**Date :** 19 août 2026. **Établi par :** huit relectures de territoire, dont les
constats les plus lourds ont été soumis à un contradicteur, puis toutes les
preuves rejouées à la main sur le code d'aujourd'hui avant d'entrer dans ce
rapport.

**Sources de vérité :** les livrets transcrits du dépôt
(`docs/regles/livret-base.md`, `docs/regles/livret-decouverte.md`) et les
transcriptions des cartons. Aucune recherche sur la toile, aucune image lue,
aucune ligne du dépôt interdit consultée, aucun fichier modifié.

**Deux règles maison d'Alexis, volontairement différentes du livret, ne sont
jamais comptées comme des défauts :** l'ordre de la mise en place (deux
corporations, échange des corporations, huit cartes projets, échange des projets,
puis choix final de la corporation) et l'ordre des cinq phases
(I Développement, II Construction, III Action, IV Production, V Recherche).

**Vocabulaire employé dans ce rapport**
- *le moteur* : le programme qui applique les règles et arbitre la partie.
- *NT* : le niveau de terraformation, la piste qui donne à la fois du revenu à
  chaque phase de production et un point de victoire par cran à la fin.
- *MC* : les mégacrédits, la monnaie du jeu.
- *une fiche de situation* : la liste de 1 472 nombres par laquelle le moteur
  décrit une position au réseau de neurones qui juge les coups.
- *le rejeu* : la capacité de rejouer une partie enregistrée à l'identique en
  redonnant au moteur la suite des numéros de réponses qui avaient été faites.
  Un correctif « casse le rejeu » quand il change l'ordre ou le nombre des
  choix proposés : les parties enregistrées ne se rejouent alors plus.
- *un siège* : la place d'un joueur, siège 0 ou siège 1.

---

## 1. LE VERDICT

Le moteur est fidèle aux règles sur l'essentiel : la matière du jeu — les
262 cartes distribuées, leurs coûts, leurs badges, leurs effets, le déroulement
des cinq phases, les conditions de fin de partie et le décompte des points — a
été confrontée aux livrets et aux cartons transcrits sans qu'aucun écart de
valeur ne subsiste. Les défauts qui restent ne sont donc pas des erreurs de
calcul mais des erreurs de moment et d'information : le moteur pose les
questions dans un ordre qui laisse un joueur voir ce qu'il ne devrait pas voir,
et il oublie d'appliquer une ligne imprimée sur une planche de corporation.

Le défaut le plus grave est la fuite d'information de la planification : le
livret exige que les deux joueurs choisissent leur carte Phase « simultanément »
et « face cachée », or le siège 1 lit dans sa fiche de situation la carte que le
siège 0 vient de poser, à chaque manche de chaque partie, ce qui a été mesuré ce
jour même et non déduit.

Vient ensuite un défaut de contenu, unique mais coûteux : la corporation *Mining
Guild* n'applique jamais la seconde ligne de son carton, si bien qu'elle ne
rapporte jamais le niveau de terraformation qu'elle promet — ce qui explique
qu'elle figure parmi les corporations les plus mal classées dans nos mesures.

Enfin, le contrôle a un angle mort structurel : les 2 294 lignes qui portent la
description des situations et le choix des coups sont, par construction du
programme, hors d'atteinte de tous les tests automatiques du moteur, et c'est
exactement là que vivent les deux défauts découverts avant cet audit.

---

## 2. LES DÉFAUTS CONFIRMÉS

Chaque défaut a été reproduit sur le code d'aujourd'hui. Les commandes citées
rendent la main en moins de deux secondes et ont toutes été relancées.

### 2.1 — DÉFAUTS MAJEURS

---

#### D1. Le siège 1 voit la carte Phase que le siège 0 vient de poser face cachée

**Où :** `engine/src/flow.rs:5313-5325` (l'écriture) et
`engine/src/description.rs:389-403` (la publication). Même défaut recopié dans
l'interface : `web/webapp/joueurs/description.js:263-268`.

**Ce que fait le moteur.** Au début de chaque manche, le moteur demande à chaque
joueur, l'un après l'autre, quelle carte Phase il choisit. Dès que le siège 0 a
répondu, le moteur inscrit sa réponse dans l'état de la partie
(`previous_phase`). Il interroge ensuite le siège 1 — et la fiche de situation
remise au siège 1 contient six cases qui disent, en clair, quelle phase
l'adversaire a retenue. Le siège 0, lui, ne voit jamais rien, puisqu'il répond
toujours en premier. L'avantage ne tourne jamais : la boucle suit le numéro de
siège et non l'ordre du tour.

**Ce que dit la règle.** `docs/regles/livret-base.md`, ligne 268 :
« Chaque joueur choisit **simultanément** une carte Phase de sa main et la place,
**face cachée**, devant lui. » Ligne 272 : « Une fois que **tous** les joueurs
ont fait leur choix, les cartes Phase choisies sont révélées. » La révélation est
donc postérieure au choix des deux joueurs. C'est l'une des rares phrases du
livret où la simultanéité et le secret sont écrits noir sur blanc.

**Preuve, rejouée aujourd'hui.** Même partie, même mise en place ; seule change
la réponse du siège 0 (option 0 = phase I, option 4 = phase V) :

```
$ ./engine/target/release/decrire --graine 700001 \
    --decisions "0,0,[],[],0,0,0" --siege 1
  moi_previous_phase[aucune,1..5] = [+1, -1, -1, -1, -1, -1]
  adv_previous_phase[aucune,1..5] = [-1, +1, -1, -1, -1, -1]

$ ./engine/target/release/decrire --graine 700001 \
    --decisions "0,0,[],[],0,0,4" --siege 1
  moi_previous_phase[aucune,1..5] = [+1, -1, -1, -1, -1, -1]
  adv_previous_phase[aucune,1..5] = [-1, -1, -1, -1, -1, +1]
```

Les six cases « ma phase précédente » restent sur « aucune » : le siège 1 n'a pas
encore répondu. Et les six cases « la phase de l'adversaire » ont déjà bougé,
exactement selon ce que le siège 0 a choisi.

**Ce que le joueur subit en partie.** La planification est le levier le plus fort
du jeu : savoir ce que l'autre a pris permet soit de compléter sa phase pour que
deux phases soient résolues au lieu d'une, soit de la doubler exprès pour toucher
le même bonus. Ce levier est offert gratuitement à un siège sur deux, à chaque
manche. Conséquence sur l'apprentissage : le réseau apprend à jouer des
situations qui n'existent pas dans une vraie partie, et tous les duels siège 0
contre siège 1 mélangent la force réelle et cet avantage d'information. Contre un
humain, l'ordinateur en siège 1 triche de fait.

Il faut noter que l'écran, lui, a été explicitement protégé
(`web/webapp/distant.js:611-615` et `web/webapp/vue/phases.js:73-75` interdisent
d'afficher la phase adverse) : le garde-fou existe pour l'humain, il n'existe pas
pour le réseau, parce que la fuite passe par un autre champ que celui qui a été
protégé.

**Correctif.** Cesser de faire porter deux sens au même champ. Ajouter un champ
`phase_revelee` par joueur, écrit pour les deux joueurs seulement **après** que
les deux ont répondu, et faire lire ce champ-là par la fiche de situation pour
tout ce qui concerne l'adversaire. Pendant la planification, l'adversaire montre
alors la phase de la manche précédente — exactement ce qu'un joueur humain voit
sur la pile de cartes déjà jouées ; dès la résolution, il montre la phase de la
manche en cours, comme aujourd'hui. Environ vingt lignes côté moteur et dix côté
interface. À ne surtout pas corriger en posant les deux questions avant d'écrire
quoi que ce soit : cela déplacerait des points de décision.

**Casse le rejeu ?** Non. Aucun choix n'est déplacé, aucune option ajoutée ni
retirée. En revanche le sens de six cases de la fiche change, donc les poids
appris jusqu'ici ne sont plus comparables — ce qui est sans importance puisque le
dernier entraînement repart de zéro.

---

#### D2. La corporation *Mining Guild* n'applique jamais la seconde ligne de son carton

**Où :** `engine/src/effects.rs:2733-2735`.

**Ce que fait le moteur.** La planche n'encode que sa première ligne, la
réduction de 2 MC sur les cartes à badge Construction. La liste de ses effets
déclenchés est littéralement vide (`ptrig: []`). Aucun autre endroit du programme
ne la nomme. Le joueur qui prend cette corporation ne gagne donc jamais le niveau
de terraformation que son encart rose lui promet, quel que soit le nombre de
mines et de fonderies qu'il pose.

**Ce que dit la règle.** Le texte imprimé de la planche, transcrit deux fois à la
photo (`data/cartes-imprimees/textes-cartes.json`, entrée « Mining Guild ») :
« You start with 27 MC. When you play a [building], you pay 2 MC less for it.
**EFFECT: Each time you play steel production, excluding this, gain 1 TR.** »
Les notes de la même transcription décrivent l'encart : « phase I, vignette
outils : +1 [TR] ». La vignette outils est celle du savoir-faire acier, et le
livret de base la définit ligne 527 : « Chaque savoir-faire dans le secteur de
l'acier réduit de 2 MC le coût des cartes Projet ayant un badge Construction ».

Le commentaire qui justifie ce vide dans le code
(`effects.rs:2725-2732`) affirme que le savoir-faire acier « est une notion que
le moteur ne modélise toujours pas ». C'était vrai autrefois ; c'est devenu faux
depuis le lot acier-titane : le moteur calcule exactement combien d'aciers chaque
carte apporte, et treize cartes de la pioche en apportent.

**Preuve, rejouée aujourd'hui.**

```
$ ./engine/target/release/simulate --probe "Mine" \
    --probe-corp "Mining Guild" --boites base,decouverte
  → corporation trouvée : "Mining Guild", 27 MC
  → acier passé de 1 à 3, coût payé 8
  → variation du niveau de terraformation : 0
```

La carte « Mine » apporte deux aciers ; le moteur les voit arriver (le compte
passe de 1 à 3) et n'accorde rien. Témoin sur la planche voisine, dont l'effet
équivalent **est** encodé :

```
$ ./engine/target/release/simulate --probe "Vesta Shipyard" \
    --probe-corp "Saturn Systems" --boites base,decouverte
  → variation du niveau de terraformation : 1
```

**Ce que le joueur subit en partie.** Il joue avec une planche amputée de la
moitié de son texte imprimé. Sur cinq mille parties, la corporation *Saturn
Systems*, dont la densité de déclencheurs est presque identique (quatorze cartes
sur 246 contre treize), rapporte 1,15 niveau de terraformation par partie où elle
est installée ; *Mining Guild*, installée aussi souvent, en rapporte zéro. Un
niveau de terraformation vaut un MC de revenu à chaque phase de production, plus
un point de victoire à la fin. C'est exactement le genre d'écart qui explique un
classement : *Mining Guild* est l'une des trois corporations mesurées très en
dessous de la moyenne, et le réseau apprend donc à ne pas la choisir pour une
raison qui n'existe pas dans le jeu réel.

**Correctif.** Lui donner le déclencheur qui lui manque, par le chemin qui existe
déjà : une nouvelle condition de déclenchement « la carte posée accorde un
savoir-faire acier », puis l'entrée correspondante sur la planche, avec la clause
« sauf celle-ci » qui est déjà servie pour *Saturn Systems*. Environ quarante
lignes. **Un point reste à arbitrer par Alexis, carton en main :** quatre des
treize cartes apportent deux aciers ; faut-il alors un ou deux niveaux de
terraformation ? Le livret ligne 106 (« Si la condition d'un effet est remplie
plusieurs fois lorsqu'une carte est jouée, résolvez l'effet correspondant
plusieurs fois ») pousserait vers deux ; la lettre « chaque fois que vous jouez de
la production d'acier » se lit aussi « une fois par carte ». La transcription se
déclare elle-même « légèrement floue » sur cette formule : c'est le seul endroit
de ce rapport où je demande une lecture à l'image.

**Casse le rejeu ?** Oui. Le prix des cartes ne change pas, mais le revenu change,
donc les cartes payables changent, donc le nombre d'options offertes à la
construction change. Les parties enregistrées où *Mining Guild* est en jeu ne se
rejouent plus. À faire avant le dernier entraînement, ou pas du tout.

---

#### D3. L'échange des corporations se décide à l'aveugle : les deux options sont indiscernables

**Où :** `engine/src/description.rs:355-361`.

**Ce que fait le moteur.** La fiche de situation ne publie une corporation que
lorsqu'elle est **installée**. Au moment de l'échange des corporations, aucune ne
l'est encore : les deux corporations que le joueur tient en main ne figurent donc
nulle part dans les 1 472 nombres de la fiche. Les deux options — « je garde » et
« je rends les deux » — décrivent des situations rigoureusement identiques.

**Ce que dit la règle.** Aucune règle du livret n'est en cause : l'échange des
corporations est une règle maison d'Alexis, et le moteur l'applique correctement
(les deux ou aucune, sans voir les projets). Le défaut est que la décision existe
sans que rien ne permette de la prendre.

**Preuve, rejouée aujourd'hui.**

```
$ ./engine/target/release/jouer --graine 700001 \
    --poids data/poids/apprenti-1200k.txt --boites base,decouverte --tracer-rang 0
  rang 0 option 0 : note 0.48831350493588893
  rang 0 option 1 : note 0.48831350493588893
```

Deux notes identiques jusqu'à la dix-septième décimale. Conséquence déjà mesurée
avant cet audit : quatre cents gardes sur quatre cents, jamais un remplacement.

**Ce que le joueur subit en partie.** Le premier choix de chaque partie est un
tirage au sort déguisé. L'ordinateur ne peut pas rendre une paire de corporations
faibles, alors que le classement des corporations mesuré montre des écarts très
larges entre elles.

**Correctif.** Publier les corporations **tenues en main** au même titre que les
projets tenus en main : deux séries de seize cases, « corporation X dans ma main »
et « corporation X déjà installée ». Aucun choix n'est déplacé.

**Casse le rejeu ?** Non. La taille de la fiche change, donc un réentraînement est
nécessaire — il est de toute façon prévu.

---

#### D4. La couche qui décrit les situations et choisit les coups n'est couverte par aucun test

**Où :** `engine/src/lib.rs` (la bibliothèque n'expose que dix modules) et
`engine/src/bin/jouer.rs:25-31` (les quatre autres sont déclarés à l'intérieur de
chaque programme).

**Ce que fait le moteur.** Le moteur de règles proprement dit est couvert par
20 444 lignes de tests, 848 vérifications, toutes vertes en 4,6 secondes. Mais
`description.rs` (466 lignes), `joueur.rs` (951 lignes), `rejeu.rs` (715 lignes)
et `espion.rs` (162 lignes) ne font pas partie de la bibliothèque : ils sont
déclarés à l'intérieur de chaque programme exécutable. Un test ne peut donc pas
les atteindre — ce n'est pas un oubli, c'est une impossibilité de construction.
Aucun de ces quatre fichiers ne porte non plus de test interne.

**Ce que dit la règle.** Le livret n'est pas en cause. Ce qui est en cause est le
constat du document de contexte : les deux défauts d'architecture découverts
avant cet audit — l'échange de corporations aveugle et le fait que l'ordinateur
voie le hasard futur quand il essaie une option — vivent tous les deux dans ces
quatre fichiers. La seule zone sans un test est la zone où sont les seuls défauts
connus.

**Preuve.**
```
$ grep -E "description|joueur|rejeu|espion" engine/src/lib.rs   → absents
$ for m in joueur rejeu espion description; do
    grep -rho "\b$m::" engine/tests/ | wc -l; done              → 0 0 0 0
(à comparer : flow:: 126 emplois, effects:: 59, state:: 25, cards:: 37)
```

**Ce que le joueur subit en partie.** L'ordinateur choisit ses coups avec un
programme que rien n'éprouve. Le défaut D3 ci-dessus est là depuis toujours et
aucune des 848 vérifications ne l'a jamais remarqué.

**Correctif.** Remonter ces quatre fichiers dans la bibliothèque en modules
publics, en laissant les programmes exécutables les emprunter. Environ une heure,
sans une seule ligne de logique changée. C'est le préalable obligatoire aux tests
T1 à T3 de la section 6.

**Casse le rejeu ?** Non.

---

### 2.2 — DÉFAUTS MOYENS

---

#### D5. Le badge joker est gelé la première fois qu'on regarde la carte, alors que le livret laisse en choisir un autre au moment de la jouer

**Où :** `engine/src/flow.rs:458-463`.

**Ce que fait le moteur.** Trois cartes de l'extension portent un badge joker,
c'est-à-dire un badge gris que le joueur choisit. Le moteur pose la question la
première fois que la carte se trouve en main lors d'un calcul de ce que le joueur
peut se payer — donc bien avant qu'il la joue — et n'y revient jamais :
« le badge est DÉFINITIF : jamais réécrit ». Quand la carte est réellement posée,
le moteur repasse par la même fonction, qui ressort aussitôt.

**Ce que dit la règle.** `docs/regles/livret-decouverte.md`, lignes 98 à 100 :
« Dès qu'une carte indiquant un badge joker est révélée, le joueur qui l'a révélée
choisit à quel badge équivaut le joker. […] **Si vous jouez (ou défaussez) la
carte plus tard, vous pourrez choisir un badge différent.** » Le livret tranche
donc explicitement, et dans le sens contraire au moteur.

**Preuve.**
```
$ ./engine/target/release/simulate --games 200 --seed 4242 --boites base,decouverte
  "joker_tag_choices": 218
  "joker_tag_hits":    129
```
Deux cent dix-huit questions posées pour cent vingt-neuf cartes réellement
entrées en jeu : quatre-vingt-neuf choix sur deux cent dix-huit, soit 41 %, sont
faits sur une carte qui n'entrera jamais en jeu. Cela prouve que la question est
bien posée en main et non à la pose.

**Ce que le joueur subit en partie.** Une carte joker arrive en main à la manche 3,
où le joueur a deux savoir-faire acier : le moteur lui fait déclarer
« Construction ». Il la joue à la manche 7, où il a entre-temps deux savoir-faire
titane et voudrait « Espace » : la question ne lui est pas reposée, et le titane
ne réduit pas son coût. C'est très exactement l'exemple que le livret donne pour
illustrer le badge joker.

**Correctif.** Distinguer un badge provisoire, posé seulement pour juger de ce que
le joueur peut se payer et réécrivable, d'un badge définitif, posé au moment de la
pose. Attention : un badge moins favorable choisi à la pose pourrait rendre la
carte impayable après coup ; il faut donc limiter le second choix aux badges qui
laissent la carte payable.

**Casse le rejeu ?** Oui : cela ajoute un point de décision à chaque pose de carte
joker.

---

#### D6. Le bonus de la phase III est attaché de force à la première carte activée

**Où :** `engine/src/flow.rs:4755-4764` et `4771-4779`.

**Ce que fait le moteur.** Le joueur qui a choisi la phase Action a droit à une
activation de plus. Le moteur la dépense automatiquement sur la carte que le
joueur vient d'activer : il retire la carte de la liste des cartes activables,
puis, si le budget de répétition est encore ouvert, il la remet aussitôt. Le
joueur ne choisit jamais sur quelle carte porte la répétition, et ne peut pas la
garder pour plus tard dans la phase.

**Ce que dit la règle.** `docs/regles/livret-base.md`, ligne 371 :
« *Bonus : Si vous avez choisi cette phase, vous pouvez résoudre une fois de plus
la capacité "Action :" de **l'une de vos cartes en jeu**.* » « L'une de vos cartes
en jeu » désigne un choix libre parmi les cartes en jeu, et non « celle que vous
activez en premier ».

**Preuve.** Le code, cité mot pour mot :
```
if let Some(pos) = remaining_blue[p].iter().position(|&c| c == card) {
    remaining_blue[p].remove(pos);
}
if game.players[p].extra_blue_activations > 0 {
    game.players[p].extra_blue_activations -= 1;
    remaining_blue[p].push(card);          // toujours la MÊME carte
}
```

**Ce que le joueur subit en partie.** Le plus souvent il s'en tire en activant
d'abord la carte qu'il veut doubler. La perte est réelle quand la carte qu'il veut
doubler n'est pas encore activable au moment de sa première activation —
typiquement une action « dépensez N MC pour… » qu'il ne peut se payer qu'après
avoir activé une autre carte. Le bonus est alors déjà brûlé sur la mauvaise carte.

**Correctif.** Tenir à côté de la liste des cartes activables une liste des cartes
déjà activées, et les proposer en plus tant que le budget de répétition est
ouvert ; ne décompter le budget que lorsque le joueur choisit une carte déjà
activée. Environ trente lignes.

**Casse le rejeu ?** Oui : cela change le nombre et l'ordre des options de la
phase III.

---

#### D7. Avec la carte Phase III améliorée B, la même action peut être activée trois fois

**Où :** `engine/src/flow.rs:4705` et `4755-4764`.

**Ce que fait le moteur.** La carte Phase III améliorée B ouvre un budget de deux
répétitions. Comme rien ne mémorise quelle carte a déjà consommé une répétition,
le joueur peut dépenser les deux sur la même carte et l'activer trois fois dans la
manche : une fois ordinaire et deux rappels.

**Ce que dit la règle.** Le texte imprimé de cette carte, relevé sur le scan
(`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`) :
« Vous pouvez activer **deux de vos effets** "Action :" une fois de plus. »
« Deux de vos effets » désigne deux effets distincts. La carte de base, elle, dit
au singulier « une "Action :" une fois de plus », ce qui montre que le carton
compte des effets et non des jetons d'activation. **Réserve honnête :** le livret
Découverte ne détaille pas les dix cartes Phase améliorées ; ma référence est une
transcription de scan et non le livret. Le cas est d'ailleurs déjà consigné
« EN ATTENTE » dans `docs/regles/notes/cas-tranches.md`.

**Preuve.**
```
$ ./engine/target/release/simulate --probe "Hematite Mining" \
    --probe-phase 3 --probe-upgrade 3B --boites base,decouverte
  → "card": "Action (phase améliorée B)", "extra_activations": 2
```
et le code cité en D6 ne filtre jamais sur l'identité de la carte.

**Ce que le joueur subit en partie.** Un joueur qui possède cette carte et une
seule action forte la déclenche trois fois au lieu de deux. C'est une combinaison
que le carton n'accorde pas, et l'ordinateur apprendra à l'exploiter : il
surévaluera cette carte Phase et les cartes à action unique puissante. Contre un
humain sur le vrai matériel, cette ligne de jeu sera refusée.

**Correctif.** Tenir, pendant la phase, la liste des cartes ayant déjà consommé une
répétition, et ne rendre une carte à la liste des activables que si elle n'y figure
pas encore. Dix lignes.

**Casse le rejeu ?** Oui.

---

#### D8. Sur une phase déjà améliorée, le moteur impose le basculement A vers B sans rien demander

**Où :** `engine/src/flow.rs:1146-1189`.

**Ce que fait le moteur.** Quand une carte impose d'améliorer une phase précise
et que le joueur a déjà amélioré cette phase-là, il ne reste qu'une variante
possible : l'autre. Le moteur l'applique alors sans consulter le joueur, parce
qu'il a pour convention de ne poser une question qu'à partir de deux candidates.
Le joueur subit le basculement, et peut y perdre.

**Ce que dit la règle.** `docs/regles/livret-decouverte.md`, ligne 66 :
« Lors de la résolution d'un effet « Améliorez une carte Phase », vous **pouvez**
choisir d'améliorer en une amélioration différente une carte Phase que vous avez
déjà améliorée. » Le livret ouvre une faculté ; le moteur en fait une obligation.

**Preuve.**
```
$ ./engine/target/release/simulate --probe "Communications Streamlining" \
    --probe-phase 3 --probe-upgrade 3B --boites base,decouverte
  → on part de III-B (deux activations supplémentaires)
  → on ressort avec III-A (une seule)
  → aucun point de décision n'a été ouvert
```

**Ce que le joueur subit en partie.** Il a bâti sa manche sur la variante B, il
pose une carte pour un tout autre effet, et il perd une activation d'action sans
l'avoir voulu ni pu refuser. Sur deux mille parties, 2 243 améliorations à phase
imposée sont accordées : le cas n'est pas rare.

**Correctif.** Le moins cher : garder la variante déjà installée dans la liste des
candidates ; la décision existe alors toujours, et rechoisir ce qui est en place
vaut « je ne change rien ». Trois lignes.

**Casse le rejeu ?** Oui : le nombre des options change.

---

#### D9. Une action peut détruire des ressources pour un paramètre déjà au maximum

**Où :** `engine/src/flow.rs:1088-1099` (le calcul de ce qui est jouable),
`3675-3679` (le garde-fou qui ne couvre pas ce cas) et `4299` (l'exécution).

**Ce que fait le moteur.** Pour une action à alternatives, le moteur juge chaque
branche jouable ou non. Toute branche qui se contente de faire gagner quelque
chose est déclarée jouable sans condition. La branche « retirer trois microbes
pour révéler une tuile Océan » est donc offerte même quand les neuf tuiles sont
déjà sorties. Choisie, elle retire vraiment les microbes, puis la révélation
d'océan sort immédiatement sans rien accorder. Trois cartes sont concernées :
*Nitrite Reducting Bacteria*, *GHG Production Bacteria*, *Regolith Eaters*, plus
une branche de *Biomedical Imports*.

**Ce que dit la règle.** Le livret autorise le gaspillage
(`livret-base.md` ligne 365 : « Vous pouvez jouer des cartes qui augmentent les
paramètres au-delà de leur maximum […] Vous ne recevrez simplement pas les
avantages liés à ces effets »). Ce n'est donc **pas** une infraction au livret,
mais une infraction à la règle que le moteur s'est donnée à lui-même et qu'il
applique partout ailleurs : une action dont l'effet imprimé ne peut plus rien
produire n'est pas proposée, on ne paie jamais pour rien. Le contraste est net :
*Aquifer Pumping*, qui fait exactement la même chose sous une autre forme
technique, est bien protégée.

**Preuve.** `flow.rs:1093` : `ResEff::Gain(_) | ResEff::PhaseUpgrade(_) => true`
— jouable sans condition. `flow.rs:3675-3679` : le garde-fou qui teste le plafond
des océans ne regarde que les actions simples, jamais les branches.

**Ce que le joueur subit en partie.** En fin de partie — c'est-à-dire dans la
situation normale, puisque la partie ne s'arrête que quand les trois paramètres
sont au maximum — l'écran propose « retirer trois microbes pour révéler une tuile
Océan » alors que les neuf tuiles sont sorties. Qui accepte perd ses microbes et
son activation, et ne reçoit rien.

**Correctif.** Étendre le test d'impossibilité aux gains d'une branche : refuser
la branche quand elle ne contient qu'une hausse d'un paramètre déjà au maximum.
Une dizaine de lignes, aucun nom de carte cité.

**Casse le rejeu ?** Oui : quand une seule branche reste jouable, la question n'est
plus posée.

---

#### D10. Les Objectifs et les Récompenses de l'extension sont tirés et comptés même en boîte de base seule

**Où :** `engine/src/flow.rs:157-170` (le tirage) et `flow.rs:5283-5289` (le
comptage).

**Ce que fait le moteur.** La mise en place tire trois Objectifs sur onze et trois
Récompenses sur sept **sans jamais regarder quelles boîtes sont en jeu**, et le
décompte final ajoute ensuite trois points par Objectif obtenu et les points de
Récompense, quelle que soit la configuration. Or la configuration par défaut du
moteur, celle de tout banc de mesure lancé sans préciser les boîtes, est
« base seule ».

**Ce que dit la règle.** Les Objectifs et les Récompenses sont un module de
l'extension : `livret-decouverte.md` ligne 51, « Mélangez les tuiles Récompense
face cachée. Révélez-en 3 […] Faites de même avec les tuiles Objectif. » Le livret
de base ne les mentionne ni dans son matériel (lignes 41 à 57 : plateau,
208 projets, 9 océans, 24 jetons forêt, 4 plateaux joueur, 12 corporations, cubes,
20 cartes Phase, 5 tuiles Phase — aucune tuile Objectif ni Récompense) ni dans sa
mise en place, ni dans son décompte final (lignes 455 à 459 : niveau de
terraformation, jetons forêt, points des cartes jouées, et rien d'autre).

**Preuve, rejouée aujourd'hui.**
```
$ ./engine/target/release/simulate --dump-state --seed 1 --boites base
  awards      : ['Collector', 'Celebrity', 'ProjectManager']
  milestones  : ['Magnate', 'Energizer', 'Tycoon']

$ ./engine/target/release/simulate --observe-state --games 1 --seed 3 --boites base
  score des deux joueurs, dès la mise en place :
  {"awards": 12, "cards": 0, "forests": 0, "milestones": 0, "tr": 5}
```
Douze points de Récompense par joueur, en boîte de base, avant même le premier
coup.

**Ce que le joueur subit en partie.** Aucun effet sur la configuration réellement
jouée (base + extension). Mais toute mesure faite en boîte de base seule —
calibrage de seuils, bancs de comparaison, duels d'apprentissage — score un jeu
qui n'existe pas, avec jusqu'à vingt-quatre points par joueur venant de modules
absents de la table.

**Correctif.** Ne pas toucher au tirage : le retirer décalerait le flux du
générateur de hasard et ferait diverger toutes les parties enregistrées en boîte
de base. Conditionner seulement le **comptage** : rendre zéro quand l'extension
n'est pas en jeu, exactement comme le moteur le fait déjà pour la seule Récompense
« Visionnaire ». Six lignes.

**Casse le rejeu ?** Non.

---

#### D11. Le départage d'égalité prévu par le livret n'existe pas

**Où :** `engine/src/sim.rs:349-350`.

**Ce que fait le moteur.** Deux scores égaux donnent une égalité, et rien d'autre.
Le code l'assume : « (C5) Aucun départage n'est appliqué : deux scores égaux = une
égalité. » L'écran ne désigne pas de vainqueur non plus.

**Ce que dit la règle.** `livret-base.md`, ligne 461 : « Le joueur ayant le plus
grand nombre de PVs remporte la partie. **En cas d'égalité, le joueur à égalité
ayant le plus grand total cumulé de chaleur, de MC et de plantes est déclaré
vainqueur. Veillez à convertir au préalable toutes les cartes Projet en main en
MC.** »

**Preuve.**
```
$ ./engine/target/release/simulate --games 2000 --seed 12345 --boites base,decouverte
  "draws": 56, "games": 2000     → 2,8 % de parties nulles
```
Le cas est déjà signalé comme non implémenté dans
`docs/regles/notes/conformite-moteur-24-07.md`, et un test existant le scelle
comme un choix délibéré : c'est donc une décision à reconfirmer, pas un oubli.

**Ce que le joueur subit en partie.** Une partie serrée contre un humain n'est pas
tranchée par le moteur alors que le livret la tranche. Et l'ordinateur n'a aucune
raison d'apprendre à finir sur un tas de chaleur, de MC et de cartes en main quand
le score est serré, puisque la ressource qui départage ne vaut rien pour lui.

**Correctif.** Calculer, pour chaque joueur, la somme chaleur + MC + plantes +
(nombre de cartes en main × le taux de conversion en MC déjà utilisé par la fin de
manche), et faire trancher l'égalité par cette valeur. **Impératif :** la
calculer, jamais la demander au joueur — le livret parle d'une conversion
automatique, et en faire une question ajouterait un point de décision.

**Casse le rejeu ?** Non, sous cette forme.

---

#### D12. Deux bancs de vérification calculent la faute puis ne tombent pas dessus

**Où :** `web/webapp/verif/juge-descriptions.mjs:135-136` contre `:145`, et
`web/webapp/verif/juge-meme-option.mjs:81-85` contre `:90-98`.

**Ce que fait le moteur.** Le premier banc compte le nombre de groupes de
situations différentes qui reçoivent une fiche identique, l'affiche — et ne
l'ajoute jamais au compteur de fautes. Le verdict final ne le regarde pas. Le banc
peut donc afficher « 400 groupes de situations jumelles » et conclure « tout va
bien ». Le second banc calcule si les scores finaux du moteur et de l'interface
coïncident, l'affiche — et les deux seules conditions d'échec sont « moins de
200 décisions comparées » et « une réponse différente ». Deux parties qui
finiraient sur des scores différents avec la même suite de réponses produiraient
la ligne « SCORES DIFFÉRENTS » suivie de « tout va bien ».

**Ce que dit la règle.** Aucune règle de jeu. Ce sont les contrôles eux-mêmes qui
se contredisent : le second fichier écrit en tête que le vrai critère est que les
deux joueurs tirent la même conclusion, et un score final divergent est
précisément la preuve que ce n'est pas le cas.

**Preuve.**
```
$ grep -n "jumeaux\|if (fautes)" web/webapp/verif/juge-descriptions.mjs
  135, 136  (calcul et affichage)      145  (le verdict, qui ne le lit pas)
$ grep -n "memeScore\|process.exit(1)" web/webapp/verif/juge-meme-option.mjs
  81, 84 (calcul et affichage)         90, 96 (les deux seules sorties en erreur)
```

**Ce que le joueur subit en partie.** Rien directement. Mais c'est le trou exact
par lequel le défaut D3 est passé : le banc chargé de juger les fiches de
situation a mesuré le phénomène et l'a imprimé comme une simple statistique.

**Correctif.** Cinq lignes au total : compter les situations jumelles comme des
fautes (en ne comparant que des situations relevées à des moments de décision
différents), et compter un score divergent comme une faute.

**Casse le rejeu ?** Non.

---

#### D13. Le contrôle « aucun pouvoir sauté en silence » ne peut pas voir une corporation à moitié encodée

**Où :** `engine/src/flow.rs:290-292` et `engine/src/cards.rs:678`.

**Ce que fait le moteur.** Le compteur d'alerte ne se déclenche que si une planche
de corporation n'a **aucun** encodage. Une planche présente mais dont une ligne du
texte imprimé n'est pas encodée passe pour intégralement traitée. Le même angle
mort existe au chargement : pour un projet, le moteur vérifie qu'aucun effet n'est
sauté ; pour une corporation, il se contente de vérifier qu'une entrée existe
(`effets_geres: c.effect.is_some()`).

**Ce que dit la règle.** Aucune règle de jeu. C'est l'invariant que le moteur s'est
donné lui-même, écrit en toutes lettres dans son propre code : « aucun pouvoir
sauté en silence. Une carte dont le texte imprimé n'est pas intégralement appliqué
vient d'entrer en jeu ». *Mining Guild* est précisément cette carte-là, et elle
n'est pas comptée.

**Preuve.**
```
$ ./engine/target/release/simulate --games 5000 --seed 7 --boites base,decouverte
  "cards_effects_unhandled": 0
```
Zéro pouvoir non géré annoncé sur dix mille installations de corporation, alors
que l'effet imprimé de *Mining Guild* n'est appliqué dans aucune (défaut D2).

**Ce que le joueur subit en partie.** Rien : c'est le contrôle, pas le jeu. Mais
c'est ce qui explique que D2 ait survécu à plusieurs audits — le tableau de bord
affirmait « zéro pouvoir non géré » pendant que la moitié d'une planche dormait.

**Correctif.** Une table de recensement explicite à côté de la table des
corporations : pour chacune des seize, la liste de ses lignes imprimées et le
champ qui la sert ; un test vérifie qu'aucune ligne n'est orpheline. Environ
soixante lignes de test, aucun changement dans le jeu.

**Casse le rejeu ?** Non.

---

### 2.3 — DÉFAUTS MINEURS

Chacun est prouvé, aucun ne change une partie réellement jouée en boîte
base + extension, sauf mention contraire.

| # | Défaut | Où | Ce que dit la règle | Casse le rejeu |
|---|---|---|---|---|
| D14 | La mise en place est séquentielle : au moment d'échanger ses projets, le siège 1 voit dans sa fiche les cartes que le siège 0 vient de rendre, et au moment de choisir sa corporation il voit celle que le siège 0 a installée. Mesuré ce jour : `decrire --graine 7 --decisions "0,0,[0,1,2]" --siege 1` fait apparaître `projet7_defausse`, `projet50_defausse`, `projet104_defausse` (Arctic Algae, Greenhouses, Airborne Radiation, les trois cartes du siège 0) ; le siège 0 au même point ne voit rien. Idem pour la corporation : `corpo_Phobolog_adv` positif au choix du siège 1, rien au choix du siège 0. | `flow.rs:200-224` et `227-236` | Le livret ne prescrit **ni** simultanéité **ni** secret pour ces deux moments : contrairement à la planification (ligne 268), la ligne 211 ne dit rien de tel, et les règles maison d'Alexis ont de toute façon réécrit cette étape. Ce n'est donc **pas** une infidélité au livret mais une asymétrie de siège, à arbitrer par Alexis. | Non, si l'on masque l'adversaire pendant la mise en place |
| D15 | `--boites decouverte` : la mise en place s'interrompt brutalement dès qu'un joueur échange ses corporations. Le paquet ne contient que quatre corporations, la distribution en prend quatre, et l'échange trouve le paquet vide. Mesuré : quatre graines sur cinq s'arrêtent sur « paquet corporations épuisé » (`flow.rs:188`). Le garde-fou du chargement laisse passer exactement quatre corporations, c'est-à-dire le cas qui casse. | `flow.rs:188`, `cards.rs:535` | Cas non tranché : l'extension ne se joue pas seule. Le reproche est de laisser une configuration que le moteur accepte de charger interrompre la partie, au lieu de la refuser au chargement. | Non |
| D16 | La phase IV Production est la seule des cinq à ne pas suivre l'ordre du tour : elle parcourt les joueurs par numéro de siège, alors que les quatre autres suivent l'ordre du tour qui alterne. Comme la phase IV fait piocher dans le paquet commun, le siège 0 prend toujours le dessus du paquet. | `flow.rs:4836` contre `4471`, `4511`, `4697`, `4983` | Le livret ne fixe aucun ordre du tour (ligne 633 : « Il n'y a pas de tour de jeu »). C'est la règle maison qui en impose un, et le moteur s'en réclame par écrit dans quatre phases sur cinq. Incohérence interne, pas infidélité. | Oui : à graine égale, les cartes reçues diffèrent |
| D17 | L'Objectif « Terraformeur » est perdu si le niveau de terraformation redescend avant la fin de la phase. Le moteur n'attribue les Objectifs qu'une fois par phase, après la phase entière ; c'est le seul des onze dont la quantité mesurée peut baisser, puisque le niveau de terraformation est dépensable. Un joueur qui atteint 15 puis en dépense un dans la même phase termine à 14 et n'obtient jamais la tuile. | `flow.rs:5055-5066`, appelé seulement en `5370` | `livret-decouverte.md` ligne 72 : « Le **premier** joueur à remplir cette condition prend la tuile Objectif correspondante. » La tuile est prise au moment où la condition est remplie. | Oui : un Objectif obtenu plus tôt change les options des cartes qui s'y réfèrent |
| D18 | Avec la carte Phase I améliorée B, la permission de poser une seconde carte verte est accordée même si le joueur n'a posé aucune première carte ; elle perd alors la remise de trois MC que le carton réserve à « la première carte ». | `flow.rs:4487-4494` | Texte imprimé : « Le coût de la **première** carte que vous jouez […] est réduit de 3 MC. Vous pouvez jouer une **seconde** carte verte… » Sans première carte, il n'y a pas de seconde. | Oui |
| D19 | Deux effets déclenchés (*Optimal Aerobraking*, *Recycled Detritus*) ne se résolvent qu'une fois même si la condition est remplie deux fois, alors que neuf autres effets de la même famille se résolvent bien une fois par badge. | `effects.rs:1465-1470` | `livret-base.md` ligne 106 : « Si la condition d'un effet est remplie plusieurs fois lorsqu'une carte est jouée, résolvez l'effet correspondant plusieurs fois. » | Non. **Aucun effet observable aujourd'hui** : aucune carte de la pioche ne porte deux badges Événement. Risque différé, correctif de deux caractères. |
| D20 | Une réduction de coût par badge compte la présence du badge et non le nombre de badges, alors que la moitié piochée de la même phrase imprimée, elle, les compte. | `effects.rs:405-412` | Même ligne 106 : l'effet **entier**, réduction comprise. | Non. Aucun effet observable aujourd'hui : aucune carte ne porte deux fois le même badge parmi ceux concernés. |
| D21 | Deux cartes qui n'existent sur aucune planche physique portent encore le drapeau « dans la pioche » dans `data/cards.json` (*Microbiology Patents*, *Project Inspection*). Le moteur ne s'y trompe pas, mais toute mesure qui lit ce drapeau compte 248 cartes projets au lieu de 246 — et c'est ce qui est arrivé aux pourcentages de badges du document de contexte. | `data/cards.json:4706` et `:6601` | 208 + 38 = 246 (`livret-base.md` l.43, `livret-decouverte.md` l.34). | Non. Deux caractères. |
| D22 | Le commentaire qui documente la source des 220 cartes de base cite un chemin de fichier inexistant (`inputs/`) et une empreinte numérique fausse. Les deux copies présentes dans le dépôt sont pourtant identiques à l'octet : seule la documentation ment. | `boites.rs:41-43`, recopié en `effects.rs:2730` | — | Non |
| D23 | Le fichier des cartes est dupliqué à l'octet dans l'interface, et aucun contrôle du dépôt ne compare les deux copies. Elles sont identiques aujourd'hui ; le défaut est l'absence de filet. Si elles divergeaient, l'humain et l'ordinateur ne joueraient pas la même partie pour la même graine, en silence. | `data/cards.json` et `web/webapp/assets/cards.json` | — | Non |
| D24 | Deux commentaires du code affirment encore qu'une amélioration de carte Phase « n'est pas gérée » alors qu'elle l'est, et qu'aucune n'est plus sautée (mesuré : zéro sur deux mille parties). C'est exactement le mécanisme qui fait rouvrir un chantier déjà fait. | `effects.rs:1646` et `1840-1842` | — | Non |
| D25 | L'équivalence entre le moteur et l'interface n'est établie que sur une à trois parties, et toujours sur les graines 1, 2 et 3 — celles-là mêmes qui ont servi à mettre le joueur au point. Le seuil minimal du banc est de 200 décisions, soit moins d'une partie. | `juge-meme-option.mjs:32`, `:42`, `:90` | — | Non |

---

## 3. CE QUI EST VÉRIFIÉ ET SAIN

Cette liste est aussi le résultat de l'audit : elle dit ce qu'il est inutile de
rouvrir.

**La matière du jeu.**
- La pioche contient exactement 262 cartes : 208 projets de base + 38 projets
  Découverte + 12 corporations de base + 4 corporations Découverte. C'est mot pour
  mot ce qu'annoncent les deux livrets, dans les quatre cases.
- Sur les 220 cartes de la boîte de base, le coût, la couleur, les badges et les
  points de victoire du fichier de données concordent avec la transcription des
  cartons : **zéro écart sur 220**.
- Les 38 projets de l'extension concordent également, couleur, coût et badges,
  avec la transcription indépendante des cartons.
- Les 248 entrées de la table des effets n'ont aucun nom en double, aucune entrée
  vide, et **les 262 cartes distribuées ont toutes un encodage** : aucune carte
  muette.
- Les 70 cartes de base à prérequis, les productions des 208 cartes de base, les
  effets immédiats des 208 cartes de base, les 22 cartes à points de victoire
  variables : **zéro écart**, contrôlés exhaustivement et non par sondage.
- Le mélange des cartes est mathématiquement correct (pas de biais). Sur
  1 500 mises en place : les seize corporations sortent entre 77 et 108 fois, les
  onze Objectifs entre 384 et 432, les sept Récompenses entre 621 et 680 — des
  écarts normaux. Sur 3 000 mains : toutes de huit cartes, zéro doublon dans une
  main, zéro carte partagée entre les deux mains.

**Les seize corporations.** MC de départ, badges, production de départ,
savoir-faire, effets, moment d'application : les seize ont été confrontées une par
une au carton, et quinze sont fidèles. La seizième est *Mining Guild* (défaut D2).
Deux erreurs du fichier de données sont correctement arbitrées en faveur du carton
(*Interplanetary Cinematics* et *Sultira*).

**Le déroulement.**
- L'ordre des cinq phases, le fait que seules les phases choisies soient résolues,
  qu'une phase choisie par les deux ne soit résolue qu'une fois tout en donnant le
  bonus aux deux, et l'interdiction de rejouer la même phase deux manches de
  suite : conformes.
- Les cinq bonus de base des cartes Phase, vérifiés un par un contre le livret :
  conformes au mot près.
- L'ordre des revenus de la phase Production, niveau de terraformation compris :
  conforme.
- La fin de partie : les trois seuils sont bons, elle est testée après chaque
  phase, la manche s'arrête net et les phases suivantes déjà choisies sont
  ignorées — exactement le texte du livret.
- La limite de dix cartes en main et les trois MC par carte défaussée : conformes.
- Les hausses de paramètres sont bloquées sur l'état de début de phase, si bien
  qu'on continue de gagner pendant la phase où un maximum est atteint et plus
  après : conforme au livret.
- Les jetons Forêt continuent d'être gagnés une fois l'oxygène au maximum :
  conforme.

**Le décompte des points.** Les cinq parts (niveau de terraformation, forêts,
cartes, Objectifs à trois points, Récompenses cinq / deux et quatre-quatre en cas
d'égalité) correspondent au livret et aux tuiles transcrites. Les onze seuils
d'Objectifs et les sept critères de Récompense concordent un à un avec le relevé
photographique.

**Les dix cartes Phase améliorées.** Leurs valeurs correspondent exactement à la
transcription du scan, et les deux que le livret donne effectivement (I-A et V-A)
concordent. Le mécanisme est sain : une seule entrée lue par phase, jamais le
cumul du bonus de base et du bonus amélioré, chaque joueur a ses propres cases,
et les cas limites (pioche vide, aucune carte activable, aucune production verte)
sont tous gérés sans interruption.

**Le moteur de règles proprement dit est bien testé.** Contrairement à ce que
laissait craindre le document de contexte, ce ne sont pas 178 lignes de tests mais
20 444, réparties en 25 fichiers et 839 vérifications d'intégration, plus neuf
vérifications internes — toutes vertes en 4,6 secondes. Sur les grandes fonctions
du déroulement et sur la table des effets, je n'ai trouvé qu'un seul chemin de
règle sans sentinelle.

**Une clarification importante.** Il n'existe **pas** deux moteurs de règles, l'un
en Rust et l'autre en JavaScript. L'interface utilise le moteur Rust compilé ; ce
qui est réellement écrit deux fois, c'est seulement la description des situations
et le joueur. Le moteur compilé qu'utilise l'interface n'est pas périmé : les
fichiers de règles n'ont pas bougé depuis, et les fichiers de cartes sont
identiques des deux côtés.

**Enfin, un point à ne surtout pas « corriger ».** La règle « la production
d'énergie devient de la chaleur » **n'existe pas** dans ce jeu. Le jeu de cartes
n'a pas de ressource Énergie ; il n'y a que les MC, la chaleur, les plantes et les
savoir-faire acier et titane. Cette règle appartient au jeu de plateau. Il n'y a
rien à ajouter au moteur de ce côté-là.

---

## 4. CE QUI N'A PAS PU ÊTRE VÉRIFIÉ

C'est ici que le risque reste. Territoire par territoire.

**Ce qui traverse tous les territoires : les transcriptions.** L'interdiction de
lire des images est absolue dans cet audit. Tout ce que ce rapport dit du « carton »
passe donc par une transcription texte du dépôt. Si une transcription est fausse,
mon verdict l'est aussi, en silence, et il l'est avec assurance. C'est l'angle
mort principal de cet audit. À décharge : la boîte de base a bénéficié d'une
double lecture à l'aveugle et le seul écart jamais trouvé sur les points de
victoire a été tranché contre la transcription, ce qui donne une bonne confiance
sur ce corpus-là.

**Mise en place.**
- Les récompenses portées par les neuf tuiles Océan : le livret n'en donne qu'un
  seul exemple et aucune photo des tuiles n'est transcrite. Les livrets ne
  tranchent pas ; je ne comble pas.
- La longueur exacte de la piste de température. Le moteur en modélise vingt
  crans, de −30 à +8 degrés par pas de deux. La transcription de la photo du
  plateau en énumère dix-neuf, et l'une des valeurs (« +5 ») n'est pas un pas de
  deux. Soit la transcription du haut de la piste est fautive — le plus probable,
  la photo montre une piste courbe —, soit le moteur demande une hausse de
  température de trop pour finir la partie. **À vérifier par Alexis sur le plateau
  physique.** C'est la seule incertitude de ce rapport qui pourrait affecter la
  durée des parties.
- Le premier joueur fixé au siège 0 au départ est annoté « règle maison » dans le
  code mais ne figure pas dans la liste des règles maison. Le livret n'en définit
  aucun. Signalé sans être compté comme défaut, mais mérite une confirmation.

**Phases.**
- Huit des dix cartes Phase améliorées viennent d'une transcription hors livret :
  je ne peux ni les confirmer ni les infirmer. Deux méritent une relecture au
  carton : la V améliorée B, qui affiche le plus gros chiffre du jeu (huit cartes
  vues), et la III améliorée B, dont dépend le défaut D7.
- Je n'ai pas chiffré ce que rapporte la fuite du défaut D1 en points ou en
  pourcentage de victoires : cela demanderait deux bancs de duel d'au moins
  quatre-vingts parties chacun, ce que l'interdiction de calcul long exclut.

**Effets de cartes.**
- La fréquence réelle du défaut D9 : aucun compteur ne distingue « branche à gain
  vide choisie » des autres retraits de ressources. J'affirme le chemin, pas son
  poids en points.
- Les points de victoire imprimés des cartes de l'extension : la transcription ne
  porte ce champ que pour une carte sur trente-huit. Voir la section 5, où ce
  point a été examiné puis écarté comme non prouvé.
- Le comportement au-delà de deux badges satisfaisant un même déclencheur est une
  extrapolation : aucune carte de la boîte n'en porte trois.

**Corporations.**
- La formulation exacte de l'effet de *Mining Guild* : la transcription se signale
  elle-même « légèrement floue », et c'est de cette formulation que dépend le
  choix entre un et deux niveaux de terraformation pour une carte qui apporte deux
  aciers. **Point à trancher par Alexis, carton en main.**
- Les textes des quatre corporations de l'extension reposent sur une lecture de
  scan simple, non sur une double lecture comme les douze planches de base.
- Je n'ai pas mesuré le gain en points du correctif D2 : cela demanderait un banc
  de duels et un entraînement. Le chiffre avancé (au moins un niveau de
  terraformation par partie) est une extrapolation par symétrie avec *Saturn
  Systems*, valable comme plancher et non comme prédiction.

**Améliorations de carte Phase.**
- Je n'ai pas mesuré ce que l'ordinateur entraîné choisit réellement à ce point de
  décision — quelle phase, quelle variante, à quelle fréquence il gaspille un
  basculement. C'est la mesure qui manque le plus dans ce territoire : elle dirait
  si l'ordinateur sait se servir des dix cartes ou s'il prend toujours la première
  de la liste.

**Fin de partie et décompte.**
- La fréquence réelle du défaut D17 (Objectif Terraformeur perdu) : il faudrait un
  compteur dédié, donc modifier le code, ce qui est interdit ici.
- Le taux de parties nulles sous ordinateur entraîné : les 2,8 % mesurés le sont
  sous jeu au hasard. Quatre parties entraînées observées finissent sur des écarts
  larges, ce qui suggère un taux plus bas — non mesuré.

**Données des cartes.**
- Les points de victoire des trente-huit cartes de l'extension n'ont pas fait
  l'objet d'une confrontation carton par carton dédiée, là où la boîte de base en
  a eu une. Voir la section 5.
- Le fichier des cartes ne porte aucun numéro imprimé, alors que les cartons en
  portent. Confronter le fichier au carton exige donc un appariement par
  ressemblance, qui laisse deux cartes indiscernables. Ce n'est pas un défaut de
  jeu mais un surcoût permanent pour toute vérification future.

**Tests et angles morts.**
- Je n'ai pas lancé la reconstruction complète des tests : il faut recompiler, sur
  une machine que l'entraînement sature. J'ai lancé à la place les vingt-six
  programmes déjà construits — tous verts — après avoir établi qu'ils sont valides
  pour les règles.
- Je n'ai pas mesuré la couverture du code à l'instrument : mes affirmations
  d'absence de couverture reposent sur des arguments de structure, pas sur un
  relevé d'exécution.
- Les vingt-sept programmes de vérification d'écran n'ont pas été lancés : ils
  exigent un serveur et un navigateur.
- Le chemin de repli de la dixième révélation d'océan (quand les neuf tuiles sont
  sorties mais que la phase autorise encore une révélation, le moteur rejoue la
  neuvième tuile) n'est couvert par aucun test. La lecture retenue le juge conforme
  au livret ligne 387, mais je le signale comme un chemin non éprouvé.
- Les onze Objectifs et sept Récompenses ne peuvent pas être confrontés au livret,
  qui n'en détaille que trois et trois : le cas est déjà consigné « EN ATTENTE ».
  Ils concordent avec le relevé photographique du dépôt, correction d'un seuil
  comprise.

---

## 5. LES DÉFAUTS RÉFUTÉS

Quatre constats initialement rapportés comme graves ont été démontés à l'examen.
Ils sont consignés ici pour qu'aucun audit futur ne les rouvre.

**R1 — « Le choix final de corporation n'est pas simultané. »** Le fait de code est
exact et je l'ai reproduit : le siège 1 voit bien la corporation du siège 0 avant
de choisir la sienne. Mais la règle invoquée ne dit pas ce qu'on lui faisait dire.
La ligne 211 du livret dit : « Mélangez toutes les cartes Corporation **face
cachée** et distribuez-en deux à chaque joueur. Chaque joueur en choisit une… » —
le « face cachée » porte sur le mélange, pas sur le choix. Et ce livret n'est pas
silencieux quand il veut la simultanéité et le secret : il l'écrit, dans les mêmes
termes à chaque fois, à la ligne 268 pour la planification et à trois autres
endroits. Le silence de la ligne 211 joue donc contre cette lecture, pas pour
elle. S'y ajoute que les règles maison d'Alexis ont réécrit cette étape entière.
Le fait subsiste, reclassé en défaut mineur D14 sous son vrai nom : une asymétrie
de siège que le livret ne tranche pas, à arbitrer par Alexis.

**R2 — « En fin de partie, le score que voit le réseau est saturé, il ne porte plus
aucune information sur l'écart. »** La première moitié est vraie : le thermomètre
du score plafonne à 51 alors que les scores finaux tournent entre 55 et 99. La
conclusion est fausse, et la mesure la contredit sur les quatre parties mêmes
qu'invoquait le constat : les fiches des deux sièges diffèrent par 216 à 254 cases
sur 1 472, et ce sont exactement les composantes du score (niveau de
terraformation, forêts, Objectifs atteints, et une case par carte posée). Le
réseau entraîné en tire d'ailleurs le bon vainqueur avec une forte confiance dans
trois cas sur quatre. Par ailleurs le seuil 51 n'est pas un oubli mais le produit
de la règle de calibrage écrite du projet, et la saturation ne frappe que les
toutes dernières manches, c'est-à-dire le moment où plus aucune décision n'est
prise. **Reclassé en piste d'optimisation** pour le dernier entraînement — allonger
le thermomètre et ajouter une case d'écart signé restent défendables — mais ce
n'est pas un défaut de conformité et le gain est inconnu.

**R3 — « Le score montré au réseau exclut les Récompenses : six à quinze points par
joueur invisibles. »** Les trois faits bruts sont exacts, la conclusion ne tient
pas. Six des sept critères de Récompense sont déjà publiés exactement, pour les
deux joueurs, dans la fiche de situation (production de MC, production de chaleur,
savoir-faire, cartes posées, badges science, améliorations de phase). Il ne manque
que la quantité du septième, « le plus de ressources sur les cartes ». Surtout, le
correctif proposé était démontrablement sans effet : dans les quatre parties
données en preuve, remplacer une valeur par l'autre produirait une fiche identique
case pour case, puisque les deux dépassent le seuil le plus haut. Et l'écart de
Récompenses ne retourne jamais le vainqueur dans ces quatre parties. **Ce qui reste
vrai et utile :** ajouter une case pour la quantité de ressources posées, seul
critère absent.

**R4 — « Les points de victoire des trente-huit cartes de l'extension ne sont
confrontés à aucune source, et le chiffre obtenu est statistiquement
invraisemblable. »** Le mécanisme invoqué est faux : dans le fichier que le moteur
lit, les 388 entrées portent toutes un champ de points de victoire **écrit**, y
compris les zéros — il n'y a pas de silence à interpréter. Ce champ a par ailleurs
été confronté carton par carton sur la boîte de base, avec zéro écart après
correction d'un seul cas. Enfin l'invraisemblance annoncée (une chance sur
vingt-huit mille) est un artefact : elle suppose que les cartes de l'extension
suivent la même économie de conception que celles de base, alors que vingt-six des
trente-huit reposent sur l'amélioration de carte Phase, mécanique absente de la
boîte de base — les cartes chères de l'extension achètent cela, pas des points.
Restreint aux dix cartes comparables, la probabilité tombe à 3 %, c'est-à-dire à un
événement banal. **Rien ici ne justifie de retarder le dernier entraînement.**
Reste une bonne hygiène sans urgence : écrire explicitement « 0 » dans la
transcription à la prochaine occasion où Alexis a les cartons en main.

---

## 6. LA LISTE MINIMALE DE TESTS À ÉCRIRE AVANT LE DERNIER ENTRAÎNEMENT

Sept tests, dans cet ordre. **Aucun ne change l'ordre ni le nombre des options :
le rejeu des parties enregistrées reste intact.** Les trois premiers valent à eux
seuls tout le reste, parce qu'ils attrapent les défauts connus et le composant dont
tout dépend.

**T0 — Rendre la couche de décision atteignable.** *(préalable obligatoire,
environ une heure, zéro logique changée)* Remonter `description`, `joueur`,
`rejeu` et `espion` dans la bibliothèque en modules publics, en laissant les
programmes exécutables les emprunter. Sans cela, T1 à T3 sont impossibles à écrire.
Corrige le défaut D4.

**T1 — Une décision doit distinguer ses options.** Pour chaque point de décision
d'un lot de parties, décrire la situation qui résulterait de chaque option et
exiger qu'au moins deux descriptions diffèrent. Attrape le défaut D3 (échange de
corporations aveugle) et **tout aveuglement futur, y compris ceux que personne
n'a encore imaginés**. C'est le test qui rend le plus de sécurité au mètre de code.

**T2 — Un essai ne doit pas voir le hasard futur.** Au moment de l'échange des
projets, essayer plusieurs ensembles de cartes rendues et exiger que les cartes de
remplacement obtenues ne soient pas les mêmes d'un essai à l'autre. Attrape le
second défaut d'architecture connu — le dessus du paquet lisible par simple essai.
Une vingtaine de lignes.

**T3 — Le rejeu doit reproduire l'état vivant.** Jouer une partie d'un trait en
enregistrant ses réponses, puis rejouer les *n* premières et comparer l'état obtenu
à l'état de référence : mêmes mains, même pioche, même défausse, mêmes paramètres,
même score partiel. Sur une dizaine de graines et plusieurs valeurs de *n*. C'est
le socle de tout le jugement d'options, et il n'est aujourd'hui vérifié que par
ricochet, sur une à trois parties.

**T4 — Le secret de la planification.** Vérifier qu'au point de décision de
planification du siège 1, aucune case de la fiche ne dépend de la réponse du
siège 0 : rejouer la même partie avec chacune des réponses possibles du siège 0 et
exiger des fiches identiques pour le siège 1. Scelle le défaut D1, le plus grave
de ce rapport, et interdit qu'il revienne par un autre champ.

**T5 — Les corporations, ligne imprimée par ligne imprimée.** Une table de
recensement à côté de la table des corporations : pour chacune des seize, la liste
de ses lignes de texte imprimé et le champ du moteur qui la sert ; un test échoue
si une ligne est orpheline. Scelle les défauts D2 et D13, et garantit qu'aucune
autre planche ne sera amputée au prochain lot.

**T6 — Faire tomber les deux bancs qui ne tombent pas.** Deux lignes dans le banc
des réponses (échouer sur un score divergent) et trois dans le banc des
descriptions (échouer sur les situations jumelles). Puis relancer le banc des
réponses sur au moins quarante graines d'une plage **jamais employée pour la mise
au point** — pas les graines 1, 2, 3 — et consigner le résultat. Corrige D12 et
D25. Environ dix-sept minutes de machine.

**Deux décisions à prendre avant de lancer, qui ne sont pas des tests :**
1. *Mining Guild* rapporte-t-elle un ou deux niveaux de terraformation pour une
   carte qui apporte deux aciers ? Lecture du carton nécessaire.
2. Le départage d'égalité du livret (défaut D11) est-il implémenté ou reste-t-il
   volontairement absent ? La décision actuelle est délibérée et scellée par un
   test ; elle mérite d'être reconfirmée, car le dernier entraînement apprendra sur
   des parties où une égalité reste une égalité.


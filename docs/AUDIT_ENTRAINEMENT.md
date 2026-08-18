# Rapport final — audit de l'architecture d'entraînement

> Écrit le 19-08-2026, avant le dernier entraînement.
> Synthèse de huit dimensions d'audit, dont les constats les plus importants ont
> été soumis à un second agent chargé de les contredire.
> Chaque chiffre cité ici vient d'une lecture de code (fichier:ligne) ou d'une
> commande courte et reproductible. Ce qui n'a pas pu être mesuré est écrit
> « inconnu ».

---

## 1. LE VERDICT

L'architecture est saine dans sa charpente : le déroulement d'une partie, la
description d'une situation (le tableau de 1 472 nombres que le réseau de
neurones reçoit en entrée), l'apprentissage par différences temporelles
(méthode où chaque situation est tirée vers la note de la situation suivante) et
l'auto-jeu (l'IA joue contre elle-même) sont corrects, symétriques entre les
deux sièges et reproductibles au dernier chiffre près. Elle est en revanche mal
réglée dans ses nombres et gravement sous-employée dans sa vitesse : sept
constantes sur huit n'ont jamais été mesurées, et le calcul n'occupe qu'un cœur
de processeur sur quatre. Le changement qui compte le plus est le plus petit de
tous : lancer l'entraînement avec `--amplitude-depart 0.045` au lieu de la
valeur livrée 0,1, ce qui coûte zéro ligne de code et rend au réseau environ un
neurone caché sur huit, aujourd'hui figé sur une valeur constante et donc
inutile. Le deuxième point le plus important n'est pas un défaut mais une
occasion : à budget de temps égal, en n'exécutant rien d'autre sur la machine,
en compilant pour le processeur réel et en répartissant les parties sur les
quatre cœurs, on obtient trois à quatre fois plus de parties d'entraînement pour
le même nombre d'heures. Enfin, deux familles d'information manquent à la
description et une seule des deux est corrigeable à coup sûr : ce que contient
la main tenue, et les écarts entre les deux joueurs.

---

## 2. LES CHANGEMENTS À FAIRE

Classés du meilleur rapport « gain obtenu sur effort dépensé » au moins bon.
Rappel valable partout : **le dernier entraînement repart de zéro**, donc aucun
changement listé ici n'est empêché par les fichiers de poids existants. La
colonne « repartir de zéro » indique seulement si le changement rendrait
illisibles les poids déjà enregistrés, ce qui n'a d'importance que si l'on
voulait reprendre un entraînement en cours de route.

### 2.1 — Changer le tirage aléatoire de départ des poids (0,1 → 0,045)

- **Ce qu'on change** : au lancement, ajouter `--amplitude-depart 0.045`.
  L'option existe déjà et est acceptée par le programme.
- **Où** : `engine/src/reseau.rs:241` (`AMPLITUDE_DEPART = 0.1`), option
  d'entrée en ligne de commande à `engine/src/bin/entraine.rs:151-153`.
- **Le problème, en clair** : les 1 472 entrées de la description valent toutes
  soit +1 soit −1, jamais 0 (`description.rs:108` et `:113`). Chaque neurone
  caché additionne 1 472 nombres multipliés par des poids tirés au hasard entre
  −0,1 et +0,1 ; la somme obtenue a donc dès le premier instant une dispersion
  de 2,2. Or la fonction qui écrase cette somme entre −1 et +1 (la tangente
  hyperbolique) est déjà quasiment plate à cette valeur : le neurone rend
  presque toujours −1 ou +1 et ne distingue plus deux situations.
- **Gain chiffré** : mesuré sur les poids livrés à 1 200 000 parties, six
  neurones sur cinquante rendent une valeur constante dans **tous** les états
  testés, et leur contribution totale au verdict du réseau est de 0,00015 sur
  2,948 — c'est-à-dire rien. La pente moyenne qui laisse passer l'apprentissage
  vaut 0,277 ; avec 0,045 elle vaut 0,558. Autrement dit, la couche cachée
  apprend aujourd'hui à 28 % de la vitesse prévue, et passerait à 56 %. Le
  fichier `data/poids/amplitude045.txt` (10 000 parties, tirage 0,045) confirme
  la mesure : 0 neurone constant contre 1 à la même ancienneté avec 0,1.
- **Coût d'écriture** : zéro ligne, zéro seconde de calcul supplémentaire.
- **Risque** : nul.
- **Repartir de zéro** : sans objet, c'est un réglage de départ.

### 2.2 — Éteindre la devinette pour le dernier entraînement

- **Ce qu'on change** : ne pas passer `--sortie-adversaire`. C'est le
  comportement par défaut du programme.
- **Où** : `engine/src/bin/entraine.rs:236-242` (le second réseau n'est même pas
  créé sans cette option).
- **Le problème, en clair** : la « devinette » est un second réseau qui essaie
  de prédire quelle phase l'adversaire va annoncer. Elle coûte 85 547 secondes
  contre 60 530 pour le même million de parties, soit 41 % de temps en plus, et
  elle rend exactement ce qu'elle coûte : l'entraînement qui l'utilise finit à
  égalité avec celui qui ne l'utilise pas (+0,33 point d'écart, soit 0,08 écart
  typique), alors que sans elle il était plus faible de 3,61 points. Sa justesse
  mesurée est de 30,8 % contre 24,9 % pour un tirage au hasard.
- **Gain chiffré** : +41 % de parties pour le même nombre d'heures.
- **Coût d'écriture** : zéro ligne. Ne **pas** supprimer le code : il faudra
  peut-être y revenir un jour.
- **Risque** : nul.
- **Repartir de zéro** : sans objet.

### 2.3 — N'exécuter rien d'autre sur la machine pendant l'entraînement

- **Ce qu'on change** : pendant les heures que dure le dernier entraînement,
  aucun banc de mesure, aucun duel, aucun programme JavaScript. On les rejoue
  après.
- **Où** : règle d'exploitation, aucun fichier.
- **Le problème, en clair** : la machine possède quatre cœurs physiques et non
  huit (`lscpu` : « Thread(s) par cœur : 2 / Cœur(s) par socket : 4 » ; la ligne
  72 de `docs/AUDIT_FINAL_CONTEXTE.md` compte des fils d'exécution, pas des
  cœurs). Pendant l'audit, six calculs lourds tournaient simultanément et une
  partie d'entraînement coûtait 114 millisecondes.
- **Gain chiffré** : ramené au travail réellement fait (le nombre d'essais
  d'option), le surcoût mesuré de la charge est d'un facteur **1,45 à 1,53**, et
  non 3,3 comme un premier constat l'affirmait — ce point a été corrigé par le
  contradicteur. Cela reste 30 à 35 % du temps total, obtenus sans écrire une
  ligne.
- **Coût d'écriture** : zéro.
- **Risque** : nul.
- **Repartir de zéro** : non.

### 2.4 — Compiler pour le processeur réel de la machine

- **Ce qu'on change** : créer `engine/.cargo/config.toml` contenant
  `[build] rustflags = ["-C", "target-cpu=native"]`, puis recompiler.
- **Où** : `engine/Cargo.toml:15-17` fixe bien l'optimisation générale mais ne
  dit jamais pour quel processeur compiler ; aucun répertoire `engine/.cargo`
  n'existe.
- **Le problème, en clair** : par défaut le compilateur produit un programme
  compatible avec n'importe quel processeur de la famille, qui traite deux
  nombres à la fois. Le processeur de cette machine (Intel i5-11300H) sait en
  traiter huit à la fois et sait fusionner une multiplication et une addition en
  une seule opération. Vérifié directement sur le programme livré : en
  désassemblant `engine/target/release/chrono`, la fonction d'évaluation du
  réseau ne contient que des instructions à deux nombres et **aucune**
  instruction de multiplication-addition fusionnée.
- **Gain chiffré** : sur les trois boucles de calcul les plus chargées, mesuré
  deux fois indépendamment : ×1,4 à ×3,2 selon la boucle, soit **×1,12 à ×1,20
  sur le temps total d'une partie**. Autrement dit 60,5 millisecondes par partie
  ramenées à environ 52.
- **Coût d'écriture** : une ligne de configuration, plus une recompilation.
- **Risque** : faible, et vérifié. Le résultat numérique reste identique au
  dernier chiffre près : le compilateur ne réordonne jamais les additions sans
  qu'on le lui demande explicitement. Un contrôle a été fait en compilant les
  quatre boucles concernées dans les deux modes : les empreintes des résultats
  sont identiques. Seule conséquence : le programme ne fonctionnera plus que sur
  cette machine.
- **Repartir de zéro** : non.

### 2.5 — Corriger la plage de scores utilisée pour l'amorçage

- **Ce qu'on change** : `AMORCAGE_SCORE_MAX` de 49 à 120.
- **Où** : `engine/src/reseau.rs:116`.
- **Le problème, en clair** : avant de jouer la première partie, le programme
  fabrique 5 000 fins de partie fictives pour enseigner au réseau neuf que
  « plus de points, c'est mieux ». Il tire les deux scores entre 0 et 49. Or le
  plus haut palier de score de la description est « plus de 51 points », et 94 %
  des fins de partie réelles dépassent ce palier. Pendant tout l'amorçage, ce
  palier n'est donc jamais franchi, et l'amorçage se fait avec un taux
  d'apprentissage dix fois plus élevé que la normale : c'est la toute première
  idée que le réseau reçoit, et elle est hors sujet sur son quart supérieur.
- **Gain chiffré** : le palier concerné passe de « jamais activé » à « activé
  comme dans les parties réelles ». En points de victoire : inconnu.
- **Coût d'écriture** : une constante.
- **Risque** : nul.
- **Repartir de zéro** : sans objet (l'amorçage n'a lieu qu'au départ).

### 2.6 — Protéger le dernier entraînement contre une coupure

- **Ce qu'on change** : trois choses dans le même fichier. (a) enregistrer les
  poids à **chaque** tranche de journal et non seulement aux instantanés
  demandés ; (b) écrire en première ligne du journal la ligne de commande
  complète et la plage de graines consommée ; (c) nommer les instantanés avec le
  nombre **absolu** de parties vues et non le rang dans la session.
- **Où** : `engine/src/bin/entraine.rs:355-363` (instantanés) et `:375`
  (enregistrement final, situé **après** la boucle des parties).
- **Le problème, en clair** : entre deux instantanés, rien n'est écrit sur le
  disque. C'est déjà arrivé : le journal `data/mesures/entrainement-A-2M.log`
  s'arrête net à 200 000 parties alors qu'un million avait été demandé — 800 000
  parties perdues. Le nom de l'instantané est construit avec le rang dans la
  session (`format!("{sortie}.{}", g + 1)`, ligne 357) alors que le compteur
  inscrit dans le fichier est le total absolu : un fichier nommé `.200000` peut
  donc contenir un réseau à 1 200 000 parties, et le prochain entraînement
  réécrira silencieusement le même nom.
- **Gain chiffré** : la perte maximale sur une coupure passe de 100 % de
  l'entraînement à 5 %. Sur un entraînement de quinze heures, cela vaut jusqu'à
  quatorze heures de calcul.
- **Coût d'écriture** : environ 30 lignes.
- **Risque** : faible. Les scripts qui passent des rangs relatifs à
  `--instantanes` devront passer des rangs absolus.
- **Repartir de zéro** : non.

### 2.7 — Répartir les parties sur les quatre cœurs

- **Ce qu'on change** : quatre ouvriers ; au début d'un groupe de quatre
  parties, chaque ouvrier recopie les poids courants, joue sa partie en
  appliquant ses corrections sur sa copie, et rend sa différence ; à la fin du
  groupe on additionne les quatre différences **dans l'ordre des graines**.
- **Où** : `engine/src/bin/entraine.rs:295` (`for g in 0..parties`, séquentiel).
- **Le problème, en clair** : rien dans le code ne s'oppose au partage du
  travail. Vérifié : aucune occurrence de `Rc`, `RefCell`, `Cell`, `static mut`,
  `unsafe`, `Mutex` ou `Arc` dans tout `engine/src`. La base de cartes et la
  description sont des données en lecture seule partagées. Le hasard est
  ré-initialisé à chaque partie depuis la graine, donc aucun état aléatoire ne
  traverse les parties : le découpage par partie est réellement déterministe.
- **Gain chiffré** : ×3,3 mesuré en faisant tourner quatre copies de la boucle
  de calcul réelle sur une machine déjà saturée ; ×3,6 à ×3,8 attendu sur
  machine libre. Mémoire : 590 kilooctets de poids par ouvrier, 2,4 mégaoctets
  en tout.
- **Coût d'écriture** : environ 150 lignes.
- **Risque** : moyen, et c'est le seul de cette liste. Le programme garantit
  aujourd'hui que deux exécutions identiques produisent le même fichier, octet
  pour octet (`reseau.rs:118-120`). L'addition ordonnée des différences préserve
  cette garantie, mais il faut la **vérifier** avant de lancer, en comparant un
  entraînement de 2 000 parties fait à un ouvrier et à quatre. À rejeter
  absolument : la mise à jour concurrente sans verrou des poids partagés (chaque
  correction touche les 73 650 poids, ce qui ferait s'entre-gêner les cœurs), et
  la répartition des options d'une même décision (elle annulerait l'économie de
  calcul qui fait passer une évaluation de 24,8 à 3,1 microsecondes).
- **Repartir de zéro** : non, mais **à décider avant le lancement**, car
  l'apprentissage devient un apprentissage par petits groupes de quatre parties
  au lieu d'une par une.

### 2.8 — Description : décrire le contenu de la main tenue

- **Ce qu'on change** : ajouter environ 73 entrées résumant la main : dix
  paliers « nombre de cartes portant tel symbole en main », trois « nombre de
  cartes de telle couleur en main », un « points de victoire imprimés en main »,
  un « prix total de la main », un « prix de la carte la moins chère ».
- **Où** : `engine/src/description.rs:348-354` (la main n'est publiée que carte
  par carte) à comparer avec `:382-386` (les cartes posées, elles, ont dix
  résumés chiffrés).
- **Le problème, en clair** : la main est décrite par 257 signaux, un par carte
  existante. Les seuls résumés la concernant sont sa taille, le nombre de cartes
  payables et ce nombre réparti par couleur. Il n'existe **aucun** résumé de son
  contenu. Vérifié à la source : sur les 1 472 noms d'entrées rendus par
  `decrire --noms`, exactement 257 se terminent par `_main`, et parmi les 444
  entrées qui ne sont pas des cartes, seules 28 dépendent de la main.
- **Gain chiffré** : une carte donnée est en main dans 4 % des situations
  mesurées. Chacun de ses 50 poids ne reçoit donc un signal qu'une fois sur
  vingt-cinq. La valeur générale d'une main n'est aujourd'hui apprenable qu'à
  travers 12 850 poids qu'il faudrait ajuster ensemble ; les 73 entrées
  proposées la rendent apprenable en 3 650 poids sollicités à chaque situation.
  Coût de calcul : +5 % sur une correction (81 microsecondes mesurées, environ
  85 après). Gain en points de victoire : **inconnu**.
- **Coût d'écriture** : environ 60 lignes en Rust et 60 en JavaScript. Attention
  : `observe::state_view` ne publie aujourd'hui, pour une carte en main, que sa
  couleur, son numéro, son nom et son prix ; il faut y ajouter les symboles et
  les points de victoire pour que le JavaScript puisse tenir une copie
  identique.
- **Risque** : faible sur le fond, réel sur la synchronisation : les deux
  descriptions, Rust et JavaScript, doivent rester identiques, ce que vérifient
  les bancs `juge-descriptions.mjs` et `juge-meme-option.mjs`.
- **Repartir de zéro** : **oui**, tout changement de description rend illisibles
  tous les fichiers de poids existants. Sans importance ici, mais cela veut dire
  que **tous** les changements de description doivent être décidés en une seule
  fois, avant le lancement.

### 2.9 — Description : publier les écarts entre les deux joueurs

- **Ce qu'on change** : (a) prolonger l'échelle de score vers le haut, de
  `[5, 7, 10, 14, 20, 27, 36, 51]` à `[…, 51, 62, 75, 90]`, soit 6 entrées ;
  (b) ajouter des paliers d'**écart signé** pour les six grandeurs décisives :
  score acquis, niveau de terraformation, cartes posées, argent, production
  d'argent, forêts, soit 40 entrées.
- **Où** : `engine/src/description.rs:369-433` et l'échelle `S_SCORE` ligne 58.
- **Le problème, en clair** : toutes les grandeurs sont publiées en valeur
  absolue, joueur par joueur. Aucune entrée n'exprime la différence entre les
  deux, alors que ce que le réseau doit produire est une probabilité de victoire,
  c'est-à-dire une fonction de la seule différence. Et l'échelle de score a été
  relevée sur des parties jouées au hasard, alors que les parties de l'IA
  entraînée dépassent largement son dernier palier.
- **Gain chiffré** : mesuré sur 625 situations issues de 25 parties de l'IA
  entraînée, les huit paliers de score sont **identiques pour les deux joueurs**
  dans 35,4 % des cas ; dans 4,8 % des situations l'écart réel de score est
  pourtant d'au moins 8 points, et il monte jusqu'à 82. Le palier le plus haut
  est saturé 20 % du temps. Après correction, la proportion de situations
  indiscernables tombe à quelques pour cent. Gain en points de victoire :
  **inconnu**.
- **Coût d'écriture** : environ 25 lignes de chaque côté.
- **Risque** : nul sur le calcul, ce sont des entrées dérivées de valeurs déjà
  lues. Même remarque de synchronisation Rust/JavaScript qu'au point précédent.
- **Repartir de zéro** : oui, même remarque.

### 2.10 — Description : les ressources posées et le classement des récompenses

- **Ce qu'on change** : environ 33 entrées, et pas les 103 qu'un premier constat
  proposait. (a) un palier de « ressources posées sur mes cartes » par joueur,
  soit 12 entrées ; (b) pour chacune des sept récompenses révélées, trois
  signaux « je mène / égalité / il mène », soit 21 entrées.
- **Où** : `engine/src/description.rs:283-310`. Vérifié : la recherche de
  `card_resources` et `color_counts` dans ce fichier ne rend **aucune** ligne.
- **Le problème, en clair** : les récompenses valent 6 à 15 points par joueur et
  entrent dans le score final que le réseau doit prédire, mais la seule entrée de
  score de la description les exclut. Le contradicteur a cependant montré que six
  récompenses sur sept sont **déjà** déductibles d'entrées publiées (production
  d'argent, production de chaleur, acier et titane, cartes posées, symboles
  science, améliorations). Une seule est totalement invisible : celle qui compte
  les ressources posées sur les cartes. Il a également montré que les objectifs,
  eux, sont bel et bien lisibles : leurs points sont une fonction exactement
  proportionnelle de l'entrée `reperes_atteints`, publiée pour les deux joueurs
  (`description.rs:429-431`, échelle `S_REPERES` ligne 63) — cette moitié du
  constat initial est donc écartée.
- **Gain chiffré** : mesuré sur 900 tuiles de récompense issues de 300 parties,
  le classement des deux joueurs est déjà déductible dans 72 à 74 % des cas ;
  14,7 % sont indéterminables à cause de la seule récompense « collectionneur »,
  et 11 % à cause de paliers trop grossiers. Les 33 entrées suppriment les deux.
  Effet mesuré de l'ensemble des récompenses sur la valeur à apprendre : 0,107
  en moyenne sur une sortie comprise entre 0 et 1.
- **Coût d'écriture** : environ 30 lignes de chaque côté.
- **Risque** : faible.
- **Repartir de zéro** : oui, même remarque.

### 2.11 — Essayer toutes les combinaisons à l'échange des cartes de départ

- **Ce qu'on change** : à l'échange des cartes de départ (nommé `mulligan` dans
  le code), essayer les 256 sous-ensembles possibles au lieu de construire la
  liste carte par carte.
- **Où** : `engine/src/joueur.rs:478-500`, appelé depuis `:772-774`.
- **Le problème, en clair** : le joueur part de la liste vide, ajoute à chaque
  tour la carte dont l'ajout améliore le plus, et **s'arrête** au premier tour où
  aucune addition seule n'améliore. Il n'essaie jamais de retirer une carte déjà
  ajoutée ni d'en ajouter deux ensemble. Il visite au plus 37 des 256
  possibilités.
- **Gain chiffré** : mesuré sur onze mains réelles, la construction carte par
  carte reste bloquée sur une solution moins bonne **6 fois sur 11**, avec des
  écarts de +0,025 à +0,146 de probabilité de victoire, moyenne +0,028. Effet
  secondaire important : l'énumération complète rend 4,45 cartes en moyenne
  contre 2,82 aujourd'hui — une partie notable de l'écart avec le témoin à règles
  écrites (qui en rend 6) vient donc de là, et pas seulement du défaut n°2 déjà
  connu. Coût de calcul : +476 essais sur environ 4 750 par partie, soit +9 à
  +10 % de temps.
- **Coût d'écriture** : environ 20 lignes, plus le miroir JavaScript.
- **Risque** : faible côté moteur. **Réserve honnête** : ces gains ont été
  mesurés avec le défaut n°2 encore actif, donc partiellement gonflés par la
  lecture du dessus de la pioche. Une fois ce défaut corrigé, choisir le maximum
  parmi 256 notes bruitées favorise systématiquement les grands sous-ensembles ;
  il faudra soit moyenner quelques rejeux par sous-ensemble, soit accepter ce
  biais. Le fait mesuré qui survit dans tous les cas : ajouter deux cartes
  ensemble améliore là où aucune addition seule n'améliore.
- **Repartir de zéro** : non, mais le miroir JavaScript doit recevoir la même
  énumération dans le même ordre, sinon le banc de parité échoue.

### 2.12 — Décrire uniquement les cartes réellement distribuées

- **Ce qu'on change** : construire la liste des cartes décrites à partir de leur
  appartenance à la pioche et non de leur appartenance à une boîte nommée.
- **Où** : `engine/src/description.rs:195-198`.
- **Le problème, en clair** : la description retient les 257 cartes portant un
  nom de boîte, dont 11 cartes promotionnelles qui ne sont jamais distribuées
  avec la composition retenue. Leurs 44 signaux valent donc −1 dans toutes les
  parties possibles. Contrôle indépendant : sur 2 400 situations de 60 parties
  entraînées, exactement 11 cartes n'apparaissent jamais.
- **Gain chiffré** : −44 entrées (−3 %), −2 200 poids, environ −2,4
  microsecondes sur les 81 d'une correction. Gain en force : **nul ou
  négligeable**. C'est du rangement à faire pendant qu'on refait la description,
  pas une raison d'y toucher seule.
- **Coût d'écriture** : une ligne, plus la régénération du fichier de cartes du
  JavaScript.
- **Risque** : faible. Contrepartie assumée : la table des cartes cesse d'être
  indépendante de la composition demandée, ce que le commentaire des lignes
  152-156 revendique comme un principe.
- **Repartir de zéro** : oui, même remarque que les autres changements de
  description.

### 2.13 — Réserver des graines que l'entraînement ne touchera jamais

- **Ce qu'on change** : établir et faire respecter une convention : les graines
  au-dessus de 10 000 000 pour l'entraînement, 1 000 000 à 9 999 999 pour les
  bancs de mesure et les duels, 1 à 999 999 pour la vérification des règles.
- **Où** : `engine/src/bin/entraine.rs:176` (la seule protection actuelle
  interdit de démarrer sous 100 000).
- **Le problème, en clair** : l'entraînement de référence a consommé les graines
  300 000 à 1 299 999. Or les bancs de mesure qui servent de témoin jouent
  500 000, 700 000, 900 000, 1 210 000 — tous à l'intérieur de cette plage. Le
  jeu de contrôle réservé n'en est donc pas un.
- **Gain chiffré** : aucun gain de force. L'exposition réelle est faible (chaque
  graine vue une fois sur un million, avec un taux d'apprentissage de 0,0001 : la
  mémorisation est très improbable), et je n'affirme pas que les chiffres publiés
  soient faussés. Mais sans cela, tout chiffre publié après le dernier
  entraînement restera contestable par la même objection.
- **Coût d'écriture** : une ligne dans l'entraînement, une ligne d'en-tête dans
  chaque banc.
- **Risque** : nul, sauf qu'il faut rejouer les mesures témoins sur des graines
  neuves pour que la comparaison avant/après reste sur la même échelle.
- **Repartir de zéro** : non.

### 2.14 — Décaler le compteur qui décide quand on apprend

- **Ce qu'on change** : soit décaler le compteur d'un tirage aléatoire au début
  de chaque partie, soit tenir un compteur **par siège**.
- **Où** : `engine/src/joueur.rs:308` (remise à zéro par partie) et `:563-566`
  (une correction n'a lieu que si le compteur est un multiple de 8).
- **Le problème, en clair** : le compteur est remis à zéro à chaque partie et
  incrémenté à chaque décision, les deux sièges confondus. La première correction
  tombe donc toujours exactement à la huitième décision. Or la mise en place pose
  6, 7 ou 8 décisions selon les corporations tirées. Mesuré sur 60 graines : 26
  parties à 6 décisions, 26 à 7, 8 à 8. Seules ces dernières, 13 %, voient une
  décision de mise en place recevoir une correction.
- **Gain chiffré** : le taux de correction sur la mise en place passe d'environ
  0,13 par partie à environ 0,75, soit six fois plus. **Correction importante
  apportée par le contradicteur** : ce n'est pas « zéro » aujourd'hui comme un
  premier constat l'affirmait, mais un sous-échantillonnage d'un facteur six. Et
  il ne faut **pas** forcer six corrections par partie sur la mise en place :
  cela lui consacrerait 12 à 15 % de l'apprentissage pour 1,7 % des décisions.
  Second effet, également réel : le compteur étant commun aux deux sièges, les
  deux sièges ne reçoivent pas le même nombre de corrections.
- **Coût d'écriture** : deux lignes.
- **Risque** : faible.
- **Repartir de zéro** : non.

### 2.15 — Permettre à l'IA de vendre une carte

- **Ce qu'on change** : faire de la vente une décision essayée comme les autres,
  d'abord limitée à zéro ou une carte.
- **Où** : `engine/src/joueur.rs:874-876` — la fonction rend une liste toujours
  vide ; `engine/src/rejeu.rs:103` fait de même, donc l'IA ne peut même pas
  essayer.
- **Le problème, en clair** : c'est le seul endroit trouvé où une action légale
  est entièrement interdite à l'IA. Deux conséquences : elle laisse passer
  environ 17,5 occasions de vente par partie (mesuré par un contrôle du dépôt :
  3 496 occasions sur 200 parties), et surtout le réseau n'est jamais entraîné
  sur des situations où un joueur a converti sa main en argent — situations que
  Corentin produira. Le témoin à règles écrites, pourtant plus faible, vend
  4 077 fois en 120 parties.
- **Gain chiffré** : **inconnu**. Coût de calcul estimé : +0,3 % d'essais, très
  loin du « plusieurs fois le prix de tout le reste » qu'annonce le commentaire
  du code — chiffre à re-mesurer avant de le croire.
- **Coût d'écriture** : environ 20 lignes, plus le miroir dans le rejeu et dans
  le JavaScript.
- **Risque** : faible, l'option se coupe par un drapeau si le coût dépasse les
  prévisions.
- **Repartir de zéro** : non, mais à décider avant le lancement puisque cela
  change les situations vues.

### 2.16 — Élargir la couche cachée de 50 à 100 ou 200 neurones

- **Ce qu'on change** : la constante `CACHES`, **et** la constante correspondante
  du JavaScript.
- **Où** : `engine/src/reseau.rs:54` et `web/webapp/joueurs/apprenti.js:90`
  (`const CACHES_ATTENDUS = 50;`) ainsi que le message d'erreur de la ligne 165.
- **Le problème, en clair** : 50 neurones pour 1 472 entrées, dont 1 028 sont des
  signaux de cartes individuelles, est étroit. Le nombre de combinaisons
  « telle corporation avec telle carte en main » qui comptent réellement est de
  l'ordre de 4 100, et une couche de 50 unités ne peut en représenter que 50.
- **Gain chiffré** : **le coût est chiffré, pas le gain**. Coût mesuré : la
  correction passe de 22 à 103 microsecondes à 200 neurones, l'évaluation
  économique de 5,2 à environ 12,9 ; sur une partie, ×1,8 à ×2,2 à 200 neurones,
  ×1,3 à 100 neurones. **Ce qu'il faut retirer d'un premier constat** : son
  argument principal — « le réseau plafonne à 0,79 de justesse depuis 550 000
  parties, donc 50 neurones sont le coupable » — a été réfuté. Sur cet
  intervalle exact, la force mesurée en duel a fortement monté (+34,2 puis +57,7
  points d'écart contre le témoin de référence entre 400 000 et un million ; duel
  direct un million contre 600 000 : 66,9 % contre 30,0 %, soit 4,7 écarts
  typiques). L'indicateur « justes » est saturé, ce n'est pas le réseau. Le
  dépôt avait déjà commis et consigné cette erreur.
- **Coût d'écriture** : deux constantes, mais l'une est en JavaScript et son
  oubli ferait échouer tous les bancs et l'application web.
- **Risque** : moyen. **Impératif** : n'élargir qu'en appliquant en même temps le
  changement 2.1, sinon la largeur est gaspillée — mesuré, à 200 neurones avec le
  tirage 0,1, 45 neurones sur 200 sont déjà saturés.
- **Repartir de zéro** : oui, les poids existants deviennent illisibles (le
  programme le détecte et refuse, il n'y a pas de risque d'erreur silencieuse).
- **Recommandation** : 100 neurones, qui coûtent ×1,3 seulement et doublent la
  capacité, si le partage sur quatre cœurs (2.7) est acquis. 200 sinon
  déconseillé.

### 2.17 — Trois corrections mineures, gratuites, à faire tant qu'on y est

- **Le calcul de correction de la couche de sortie** : il n'applique que la
  partie principale de la formule là où la couche cachée applique la formule
  complète (`reseau.rs:429` contre `:439-457`). Pour le réseau à deux sorties —
  le seul qui sera entraîné — cela **revient exactement** à un taux
  d'apprentissage deux fois plus petit sur les 255 poids de sortie, et non à une
  direction fausse : c'est un réglage, pas un défaut, et le contradicteur a
  montré que la « preuve sur les poids réels » avancée initialement était sans
  rapport. Cinq lignes, à classer comme cohérence gratuite. **Ne pas** passer à
  l'entropie croisée sur le dernier entraînement : c'est un changement bien plus
  vaste et non contrôlé.
- **L'accumulateur de la couche cachée** : il force un parcours de 73 700 poids
  à chaque correction alors qu'aucune évaluation ne s'intercale ; on peut écrire
  directement dans les poids. 34,8 microsecondes économisées sur 214, résultat
  identique au dernier chiffre près, environ 20 lignes.
- **La fin de manche exécutée pour rien** : quand le joueur a posé son point
  d'attente, la manche continue de se dérouler alors que plus rien n'est
  enregistré. Une sortie anticipée dans `flow::play_round` coupe ce travail sans
  toucher aux règles et sans changer l'état rendu. Gain estimé 6 à 10 % du temps
  total — **à mesurer, pas à annoncer** : la première estimation de ce poste
  (25 % du temps) reposait sur une mesure prise sous charge et a été réfutée.
- **Ajouter deux entrées « je suis le sélectionneur de la phase en cours »** :
  l'information existe déjà sous forme de conjonction de deux entrées, mais la
  rendre directement disponible coûte deux entrées et deux lignes.

---

## 3. CE QUI EST DÉJÀ BIEN — À NE PAS CASSER

1. **Le déterminisme au dernier chiffre.** Deux exécutions identiques produisent
   le même fichier de poids, octet pour octet ; aucune horloge n'entre dans le
   calcul. C'est ce qui rend tous les contrôles possibles. Toute modification —
   en particulier le partage sur quatre cœurs — doit préserver cette propriété et
   être vérifiée sur ce point.
2. **La description ne triche pas sur le secret.** Le parcours est unique, il ne
   publie jamais la main de l'adversaire, et les grandeurs sont codées en paliers
   cumulés, ce qui rend les comparaisons entre joueurs faciles à apprendre. Un
   contradicteur a explicitement établi que le codage en paliers rend le
   basculement d'une récompense apprenable par un seul neurone.
3. **Le contrôle strict à la lecture des poids.** Un fichier dont la description
   ne correspond pas est refusé avec un message précis, jamais chargé en silence.
   C'est ce qui évite d'entraîner sur des poids incompatibles sans s'en
   apercevoir. À garder comme comportement par défaut.
4. **Le miroir JavaScript et ses bancs de parité.** Le fait que la même décision
   doive être prise à l'identique en Rust et en JavaScript est une contrainte
   coûteuse, mais c'est le seul contrôle qui attrape une divergence de
   description. Ne jamais modifier un des deux côtés seul.
5. **Le calcul économique de l'évaluation.** Évaluer une option en ne recalculant
   que ce qui a changé fait passer une évaluation de 24,8 à 3,1 microsecondes.
   Aucune modification ne doit annuler cet effet — c'est pourquoi il ne faut pas
   répartir les options d'une même décision sur plusieurs cœurs.
6. **L'auto-jeu symétrique et la pile de traces filtrée par siège.** Chaque
   siège reçoit sa propre cible et ses propres corrections ; rien n'est mélangé
   entre les deux. Cette partie est correcte.
7. **Le choix final de corporation.** C'est la décision la mieux mesurée du
   projet : 96,1 % de bons choix sur les paires que le classement tranche, contre
   45,6 % pour le témoin à règles écrites. Elle fonctionne parce qu'elle est
   jugée sur un état de jeu ordinaire. Ne pas la déranger.
8. **Les options de réglage déjà présentes** : `--amplitude-depart`, `--lambda`,
   `--rythme`, `--exploration`, `--reprise`, `--instantanes`. Elles permettent de
   régler l'essentiel sans recompiler.

---

## 4. LES CONSTATS RÉFUTÉS — POUR MÉMOIRE

Ces affirmations ont été produites par l'audit puis démontées par un second
agent. Elles sont consignées ici pour qu'on ne les redécouvre pas.

1. **« Le réseau plafonne depuis 550 000 parties, il faut l'élargir. »** Faux
   comme raisonnement. Sur cet intervalle exact, la force en duel a fortement
   monté. L'indicateur « justes » (proportion de parties dont le vainqueur est
   correctement désigné à mi-partie) est saturé par le hasard résiduel du jeu ;
   le dépôt avait déjà tiré et consigné cette fausse conclusion une fois.
   Élargir la couche reste défendable, mais sur ses mérites propres.
2. **« La description révèle au second joueur la phase secrète du premier. »**
   Le fait est vrai mais sans conséquence : le réseau qui joue n'est ni évalué ni
   entraîné sur cet état. Seule la devinette l'était, et elle est éteinte. La
   vraie transmission d'information secrète se trouve ailleurs, dans le rejeu
   d'essai — c'est le défaut n°2 déjà connu.
3. **« Les récompenses et les objectifs sont invisibles. »** À moitié faux. Les
   points d'objectifs sont exactement et proportionnellement lisibles sur une
   entrée existante. Six récompenses sur sept sont déductibles d'entrées
   publiées. Seule la récompense « collectionneur » est réellement aveugle. Le
   correctif proposé au départ (103 entrées) était surdimensionné d'un facteur
   trois.
4. **« La mise en place n'est jamais entraînée. »** Faux : elle l'est dans 13 %
   des parties, pas dans 0 %. C'est un sous-échantillonnage d'un facteur six, à
   corriger par un décalage de compteur et non par un forçage.
5. **« Le taux d'apprentissage est trop grand, il faut le faire décroître. »**
   Non étayé. Le diagnostic reposait sur l'indicateur « justes », dont le dépôt
   a mesuré **deux fois** qu'il varie à l'inverse de la force en duel. Les seules
   mesures disponibles sur le pas effectif montrent la force qui **monte** avec
   le pas. La variante proposée contenait de plus une erreur de calcul (elle
   augmentait le pas de 44 % en se présentant comme neutre). Ce qui reste : il
   manque une option `--taux`, c'est un manque d'outillage, pas un défaut.
6. **« Le calcul de correction de la sortie descend dans une mauvaise
   direction. »** Faux pour le réseau à deux sorties : direction identique,
   facteur d'échelle 2. Pour la devinette, la direction reste correcte dans
   100 % des situations réalistes mesurées. La preuve avancée (des poids figés)
   était vraie mais sans rapport : ils resteraient figés après correction.
7. **« Remplacer la construction carte par carte par l'énumération complète rend
   26 à 37 % de vitesse. »** Faux. Sur les tailles réelles — les défausses de fin
   de manche, jusqu'à 19 448 combinaisons — l'énumération est dix à seize fois
   **plus** chère. Le gain réel du garde-fou proposé est de 3 %. L'essentiel du
   gain annoncé venait en réalité d'une réduction de la recherche, c'est-à-dire
   d'une perte de force déguisée en optimisation. L'énumération complète reste
   justifiée **uniquement** pour l'échange des cartes de départ (point 2.11).
8. **« Chaque essai rejoue la manche depuis son début : 25 % du temps
   récupérable. »** Le poste pèse bien 23 à 25 %, mais le mécanisme décrit est
   faux : le déroulement d'une manche n'a pas de sortie anticipée, donc le
   préfixe rejoué n'ajoute aucun travail. Le vrai gaspillage est la **fin** de
   manche exécutée après que plus rien n'est enregistré. La « variante à coût
   nul » proposée (supprimer une copie d'état) casserait le point de référence de
   l'apprentissage.
9. **« Tourner seul sur la machine rend un facteur 3,3. »** Le facteur est de
   1,45 à 1,53 une fois ramené au travail réellement effectué. La référence de
   35 millisecondes n'a jamais été mesurée sur machine libre et portait sur des
   parties deux fois plus légères.

---

## 5. LE PLAN DU DERNIER ENTRAÎNEMENT

### Étape 0 — Corriger les deux défauts déjà connus (préalable absolu)

Le mulligan des corporations aveugle et la lecture du hasard futur pendant les
essais. Ils ne font pas partie de cet audit mais **rien de ce qui suit n'a de
sens sans eux** : enrichir la description ou énumérer les 256 combinaisons ne
ferait qu'exploiter plus finement une information illégitime.
Durée estimée : une à deux journées, non chiffrée par cet audit.

### Étape 1 — Les changements à zéro ligne (immédiat)

Décider et écrire dans le script de lancement : `--amplitude-depart 0.045`, pas
de `--sortie-adversaire`, graine de départ au-dessus de 10 000 000, machine
libérée de tout autre calcul.
**Durée : quelques minutes.**

### Étape 2 — Les changements à une constante ou une ligne

`AMORCAGE_SCORE_MAX` 49 → 120 ; création de `engine/.cargo/config.toml` ;
recompilation.
**Durée : moins d'une heure, dont environ cinq minutes de recompilation.**
Contrôle immédiat : relancer `chrono` et vérifier que les temps baissent ;
relancer un entraînement de 2 000 parties et vérifier que le fichier de poids
produit est identique à celui obtenu avant la recompilation.

### Étape 3 — Les protections

Enregistrement à chaque tranche, ligne de commande et plage de graines en tête de
journal, instantanés nommés en absolu.
**Durée : environ deux heures.**

### Étape 4 — La description, en **un seul lot**

Les quatre changements 2.8, 2.9, 2.10 et 2.12 ensemble : environ +150 entrées
utiles, −44 mortes, pour un total d'environ 1 580 entrées au lieu de 1 472. Côté
Rust puis côté JavaScript, puis régénération du fichier de cartes, puis passage
des bancs de parité.
**Durée : une journée pleine.**
**Ne rien laisser pour après** : chaque retouche de description invalide tout ce
qui précède.

### Étape 5 — Le comportement du joueur

Énumération complète à l'échange des cartes de départ (2.11), décalage du
compteur (2.14), et — si l'on juge le risque acceptable — la vente de cartes
(2.15). Chacun avec son miroir JavaScript.
**Durée : une demi-journée à une journée.**

### Étape 6 — Le partage sur les quatre cœurs

Le seul travail de taille. À faire en dernier parmi les modifications, parce
qu'il est le seul qui peut mettre en péril la reproductibilité.
**Durée : une journée, dont une demi-journée de vérification.**
**Contrôle obligatoire avant de continuer** : un entraînement de 2 000 parties à
un ouvrier et le même à quatre doivent produire le même fichier de poids, octet
pour octet.

### Étape 7 — Répétition générale

Un entraînement de 20 000 parties, complet, avec tous les changements.
**Durée : environ trente minutes.**
On vérifie : que rien ne casse, que le journal contient bien la ligne de
commande, qu'un instantané a été écrit, que la courbe d'erreur descend au début,
et qu'aucun neurone caché n'est saturé (contrôle direct sur le fichier de poids
produit).

### Étape 8 — Décision sur la largeur de la couche cachée

Si et seulement si l'étape 6 a réussi : passer à 100 neurones (coût ×1,3, donc
absorbé par le partage sur quatre cœurs). Sinon rester à 50. Ne pas aller à 200
sans une mesure.
**Durée : dix minutes de décision, deux constantes.**

### Étape 9 — Le dernier entraînement

Budget de temps estimé, en partant des 60,5 millisecondes par partie mesurées
aujourd'hui : machine libre (−30 %), compilation native (−15 %), partage sur
quatre cœurs (÷3,3), description plus riche (+10 %), énumération complète à
l'échange de départ (+9 %). Résultat attendu : **un million de parties en
environ 5 à 7 heures au lieu de 17**, donc deux à trois millions de parties
dans une nuit.
Le nombre de parties à jouer n'est pas tranché par cet audit ; les mesures
disponibles montrent que la force montait encore à un million, sans plafond
visible.

### Ce qu'il faut surveiller pendant qu'il tourne

Par ordre d'utilité :

1. **L'écart de score contre un adversaire fixe.** C'est le **seul** indicateur
   encore sensible du projet ; il doit être mesuré sur les instantanés, sur des
   graines réservées, et jamais sur des parties d'auto-jeu. Un duel doit compter
   au moins 80 parties pour être lu — le dépôt garde le souvenir d'une régression
   annoncée à tort sur 40.
2. **La proportion de neurones cachés saturés.** À imprimer à chaque tranche.
   C'est le contrôle direct du changement 2.1 : elle doit rester proche de zéro.
   Si elle remonte au-dessus de 10 %, l'apprentissage est en train de se dégrader
   et il faut arrêter.
3. **L'écart moyen entre les notes de deux options.** Déjà calculé par le
   programme mais jamais imprimé. S'il s'effondre, le réseau ne départage plus
   rien et le reste des chiffres ne veut plus dire grand-chose.
4. **La norme moyenne des poids cachés.** Une divergence se voit là avant de se
   voir ailleurs.
5. **L'erreur moyenne et la proportion de vainqueurs correctement désignés.** À
   regarder, mais **sans en tirer de conclusion sur la force** : le dépôt a
   mesuré deux fois que le second varie à l'inverse de la force réelle, et le
   premier monte normalement pendant que le réseau progresse, parce que la cible
   se déplace avec lui.
6. **La présence effective des fichiers de poids sur le disque.** Vérifier
   qu'une tranche a bien produit un fichier dans les premières minutes. C'est le
   contrôle du changement 2.6, et c'est déjà arrivé qu'il manque.

---

## 6. CE QUI RESTE INCERTAIN

Honnêtement, et sans arrondir.

1. **Aucun gain en points de victoire n'est chiffré dans tout ce rapport.**
   Absolument aucun des changements proposés n'a été mesuré en duel. Ce qui est
   chiffré, ce sont des coûts, des fréquences et des mécanismes. Le seul
   changement de ce projet qui ait jamais montré un déplacement de force
   mesurable est le tirage de départ à 0,045 (7 victoires sur 50 contre 1, à
   10 000 parties) — et cet écart lui-même n'est que de 1,6 écart typique,
   c'est-à-dire non concluant.
2. **On ne sait pas si enrichir la description rend l'IA plus forte.** Le
   raisonnement est solide (remplacer 12 850 poids rarement sollicités par 3 650
   sollicités partout), mais c'est un raisonnement. Il est possible que le réseau
   apprenne moins bien avec un tableau d'entrées plus grand.
3. **On ne sait pas si 100 neurones valent mieux que 50.** L'argument du plafond
   a été réfuté. Il ne reste qu'un argument de capacité, non mesuré.
4. **Le partage sur quatre cœurs modifie l'apprentissage**, pas seulement la
   vitesse : les poids vus par un ouvrier ont jusqu'à trois parties de retard.
   C'est un régime standard et probablement plus stable, mais ce n'est pas le
   même algorithme, et personne ne l'a mesuré ici.
5. **Les gains mesurés de l'énumération complète à l'échange de départ sont
   partiellement contaminés** par le défaut n°2 encore actif au moment de la
   mesure. Ils seront plus faibles une fois ce défaut corrigé, et l'énumération
   apporte alors un biais nouveau en faveur des grands sous-ensembles.
6. **On ne sait pas ce que coûte de n'avoir qu'un seul adversaire.** L'IA
   n'affronte qu'elle-même, à l'instant présent, sans réserve d'anciennes
   versions. Aucune mesure du dépôt ne chiffre la perte. Ce qui est certain,
   c'est que le coût d'y remédier serait borné (+14 % de parties, 4,7
   mégaoctets, environ 100 lignes) et le risque nul si l'option est éteinte par
   défaut. Ce n'est pas dans le plan ci-dessus faute de preuve de son utilité.
7. **On ne sait pas ce que coûte l'exploration à 5 %** (une décision sur vingt
   prise au hasard, soit 19 à 23 par partie). La règle classique voudrait qu'on
   interrompe la propagation du crédit après un coup pris au hasard ; ce n'est
   pas fait. Aucune mesure ne dit si c'est nuisible ici.
8. **Toute mesure de temps sur cette machine est fragile.** La même commande,
   strictement déterministe, a donné des temps variant d'un facteur 1,8 selon la
   charge. Toutes les durées annoncées dans ce rapport doivent être relues comme
   des ordres de grandeur, et remesurées à machine libre avant d'engager quoi que
   ce soit sur leur foi.
9. **Je n'ai pas pu vérifier** le comportement réel du partage sur quatre cœurs,
   ni celui de la description enrichie, ni celui de l'entraînement à 100
   neurones : cela demandait de modifier le code et de lancer des calculs longs,
   ce que le cadre de cet audit interdit.

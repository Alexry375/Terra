# Plan d'exécution — de la partie du 04-08 à l'intelligence artificielle

Ce plan ordonne les 41 travaux de `DERNIERE_LIGNE_DROITE.md`. Il ne les
redécrit pas : il dit **dans quel ordre**, **pourquoi cet ordre**, et **ce qui
peut avancer en même temps**.

## Les trois règles qui commandent tout l'ordre

**1. Le moteur se change en UNE fois, jamais en plusieurs.**
Les réponses d'une partie enregistrée sont des **numéros de position** dans la
liste des choix, pas des noms. Ajouter, retirer ou déplacer un choix décale tout
ce qui suit et rend illisibles les parties enregistrées. Dix travaux touchent la
liste des choix : ils partent ensemble, avec une seule recompilation et une
seule campagne de contrôles. Les faire un par un multiplierait le travail de
vérification par dix.

**2. Deux chantiers ne tournent en même temps que si leurs fichiers sont
séparés.** Deux agents qui écrivent dans le même fichier se détruisent
mutuellement. Chaque chantier ci-dessous porte donc la liste de ses fichiers, et
deux chantiers simultanés n'en partagent aucun.

**3. Publier plus d'informations ne casse rien ; poser une question de plus,
si.** C'est ce qui permet de sortir la défausse du gros lot : faire dire au
moteur ce qu'il y a dans la pile ne déplace aucun choix.

## Vague 0 — Enquête préalable [EN COURS, c'est moi qui la fais]

Cinq points doivent être établis **avant** d'écrire les contrats, sinon les
agents devineront. Trois sont déjà tranchés.

| Point | État |
|---|---|
| La défausse est-elle publiée à l'écran ? | **NON** [VÉRIFIÉ 04-08]. `engine/src/state.rs:526` tient bien la pile ordonnée (`discard: Vec<u16>`), mais `web/webapp/wasm/src/lib.rs` n'en publie que des compteurs. Il faudra publier la liste — travail moteur **sans** effet sur les parties enregistrées. |
| Le badge « ? » (MOT-8) | **Difficulté trouvée** [VÉRIFIÉ 04-08]. Le badge choisi change le **prix** de la carte (les savoir-faire Titane et Acier s'appliquent selon le badge). Le moteur doit donc le connaître **avant** de savoir si la carte est payable : `resolve_hand_jokers` pose le jeton juste avant le calcul (`engine/src/flow.rs:409-435`). Déplacer le choix « au moment où l'on joue » demande de renverser cet ordre — proposer la carte si elle est payable **pour au moins un badge possible**, puis demander le badge à la pose. C'est faisable, mais c'est le travail le plus délicat du lot. |
| Le taux de la défausse (MOT-7) | À mesurer : le bouton de vente et l'option qu'on retire doivent rapporter la même chose, or certaines cartes modifient le taux (`engine/src/flow.rs:1555`). |
| Défausser pendant la Construction (MOT-5) | À reproduire : passe-t-on vraiment la main ? |
| La production affichée (MOT-10) | À mesurer : quelles cartes le compteur oublie-t-il ? |

## Vague 1 — Trois chantiers, en même temps

Leurs fichiers sont disjoints. C'est le seul endroit du plan où l'on peut
vraiment paralléliser sans risque.

### Chantier A · LE MOTEUR — le plus long, le plus risqué, il commande la suite
Fichiers : `engine/src/`, `web/webapp/wasm/src/lib.rs`.
Contenu : **MOT-1** (question sautée quand rien n'est payable), **MOT-2**
(action impossible encore proposée), **MOT-3** (bonus Construction tranché trop
tôt), **MOT-4** (« aucune carte constructible » dit en silence), **MOT-5**
(défausse qui passe la main), **MOT-6** (vendre quand on ne peut rien payer),
**MOT-7** (la vente ne consomme plus d'échange), **MOT-10** (revenu réel
calculé par le moteur), **MOT-11** (choisir sa tuile océan), plus la
**publication de la défausse** dont dépend la vague 2.

**MOT-8 (le badge « ? ») est mis à part** : c'est le seul dont je ne connais pas
encore le coût réel. Il entre dans le lot **seulement si** l'enquête montre que
le renversement est raisonnable. Sinon il attend un lot à lui.

Pourquoi en premier : tout le reste s'affiche à partir de ce que le moteur
publie, et c'est le seul chantier qui interdit de jouer pendant qu'il tourne.

### Chantier B · CE QU'ON VOIT — beaucoup de petites choses, peu de risque
Fichiers : `web/webapp/vue/joueurs.js`, `materiel.js`, `plateau.js`, `mains.js`,
`loupe.js`, `cartes.js`, et les feuilles de style associées.
Contenu : **LIS-2** (marqueur noir sur les jauges), **LIS-3** (voir les
ressources posées sur les cartes), **LIS-5** (une seule disposition des tuiles),
**LIS-6** (montrer le badge choisi), **LIS-7** (une croix, pas une coche),
**LIS-8** (le doublon des forêts, avec le jeton détouré), **LIS-11** (prix
d'origine barré), **LIS-13** (pas de loupe sur les dos adverses), **CNF-1**
(trier sa main en déplaçant les cartes), **CNF-5** (fermer le zoom d'un clic).

Aucun de ces travaux ne dépend du moteur. C'est aussi le chantier qui se voit le
plus vite à l'écran.

### Chantier C · LE MODE À DEUX — petit, isolé
Fichiers : `web/webapp/distant.js`, `web/webapp/relais/`.
Contenu : **MOT-9** (les deux joueurs choisissent leur phase en même temps),
**CNF-4** (dire ce qu'on attend au lieu de « Waiting for the other player »).

Malgré son nom, MOT-9 ne touche pas au moteur : c'est l'écran qui garde la
seconde réponse et l'envoie ensuite.

## Vague 2 — Après le moteur

### Chantier D · LES CARTES QUI BOUGENT ET LA DÉFAUSSE
Fichiers : `web/webapp/vue/anim.js`, `scene.js`, plus un nouveau
`web/webapp/vue/defausse.js`.
Contenu, dans cet ordre : **ANI-6** (pioche arrivant par la droite, défausse en
sens inverse, vues des deux côtés) puis **CNF-2** (la fenêtre de défausse : cinq
cartes par ligne, la plus récente en haut à gauche, sans loupe) puis **ANI-1 à
ANI-4** (pose de carte, hausse des jauges, dépense de MC, gain de jetons,
changement de tour, début de phase, le « +3 » qui dure plus longtemps).

**Ce chantier ne se coupe pas en deux** : les quatre travaux écrivent tous dans
le même fichier d'animations. Un seul agent, en plusieurs étapes.

Dépend de A pour la publication de la défausse, et de A pour ne pas animer des
événements que le moteur va changer.

### Chantier E · LES OCÉANS
Fichiers : `web/webapp/vue/revelation.js`.
Contenu : **ANI-5** — les trois défauts liés (les tuiles se retournent à nouveau
à chaque rechargement, la grande tuile montre son dos des deux côtés, il manque
l'animation de retournement), plus le côté écran de **MOT-11** (choisir sa
tuile).

Disjoint de D : peut tourner en même temps.

### Chantier F · LES IMAGES
Fichiers : `data/scans/`, `web/webapp/assets/plateau/`, `manifeste.json`.
Contenu : **LIS-4** (objectifs et récompenses nets — chercher une meilleure
ressource, puis réécrire le texte par-dessus), **LIS-10** (détourer les logos
Océan et Forêt), **LIS-12** (montrer le remélange de la défausse).

**Ce chantier tourne SEUL.** Il lit beaucoup d'images, et le débit d'envoi
d'Alexis est limité à 200 kilo-octets par seconde : plusieurs agents qui lisent
des images en même temps saturent sa connexion.

## Vague 3 — Le confort qui peut attendre

- **CNF-3** — un bouton « passer définitivement ».
- **CNF-6** — reprendre une partie interrompue. Le plus utile des trois : c'est
  ce qui a failli coûter leur partie le 04-08.
- **GRO-2** — les effets sonores.
- **MOT-12** — l'état du moteur qui recule parfois. À reprendre **après** A :
  les changements du gros lot peuvent le faire disparaître ou le déplacer.
- **VIE-1, VIE-2, VIE-3** — les trois défauts jamais reproduits. À rouvrir
  seulement s'ils réapparaissent.
*(Plus rien n'est en attente de Corentin : sa réponse est arrivée le 05-08.
**LIS-1** entre dans le chantier B, attelé à LIS-2 — retirer le nombre des deux
jauges fait du marqueur le seul repère, il doit donc devenir visible dans le même
mouvement. **LIS-9** est abandonné, Corentin a dit de laisser tomber.)*

## Vague 4 — L'intelligence artificielle (GRO-1)

C'est l'objectif du projet, et tout ce qui précède existe pour qu'elle joue à un
jeu juste et lisible. Deux décisions l'attendent, notées dès maintenant pour ne
pas les oublier :
- la défausse visible lui profite bien plus qu'à un humain (elle compte le
  paquet restant sans effort) : l'option devra être désactivable, et son droit
  d'accès décidé explicitement ;
- chaque choix qu'on retire du jeu (MOT-2, MOT-7) réduit d'autant l'arbre
  qu'elle doit explorer. Ce n'est pas un effet secondaire : c'est un gain.

## Ce qui pourrait faire dérailler ce plan

Trois risques, nommés d'avance.

1. **Le chantier A est gros.** Dix travaux, un seul lot, et il touche les règles.
   Parade : contrôles écrits **avant** les correctifs, une partie entière rejouée
   à chaque étape, et un contrôle sur une copie volontairement sabotée pour
   prouver que les contrôles voient vraiment quelque chose.
2. **MOT-8 peut coûter beaucoup plus cher que prévu** (le badge change le prix).
   Parade : il est sorti du lot par défaut, et n'y rentre que si l'enquête le
   permet.
3. **Les animations sont invisibles aux mesures.** Une mesure de position ne dit
   pas si un mouvement est joli, ni même s'il a eu lieu. Deux défauts de la
   journée du 04-08 ont été trouvés à l'œil, jamais par un contrôle. Parade :
   captures d'écran à des instants précis, et Alexis regarde avant qu'on ferme
   le chantier.

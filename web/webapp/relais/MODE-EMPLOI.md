# Jouer à deux, chacun chez soi — mode d'emploi

Ce document s'adresse à quelqu'un qui ne programme pas. Suivez-le dans l'ordre,
ligne par ligne. Tout se fait sur **votre** machine : votre adversaire, lui,
n'installe rien et n'a rien à préparer.

---

## 1. Ouvrir une fenêtre de commandes

Ouvrez le terminal (la fenêtre noire où l'on tape des commandes), puis placez-vous
dans le dossier du jeu — celui qui contient le dossier `outputs`.

## 2. Démarrer le point de rendez-vous

Tapez exactement ceci, puis appuyez sur Entrée :

    node outputs/webapp/relais/serveur.js --port 8080

Le point de rendez-vous est le petit programme qui met les deux joueurs en
relation : il donne la page du jeu aux deux ordinateurs, et il transmet à l'un ce
que l'autre vient de décider. Il ne connaît aucune règle du jeu.

**Ce qui doit s'afficher**, en quelques secondes :

    [09:12:03] la livraison servie depuis /…/outputs/webapp
    PRET http://127.0.0.1:8080
    [09:12:03] le rendez-vous attend les deux joueurs. Ctrl-C pour arrêter.

La ligne qui commence par `PRET` est le signal : tout est en place.

**Laissez cette fenêtre ouverte pendant toute la partie.** Si vous la fermez, la
partie s'arrête pour les deux joueurs. Vous pouvez la réduire, mais pas la
fermer.

Le nombre `8080` est le numéro de la porte d'entrée. S'il vous répond que la
porte est déjà prise, recommencez avec un autre nombre, par exemple `8081`, et
utilisez ce nombre partout dans la suite.

## 3. Ouvrir la porte à l'autre joueur

Dans une **deuxième** fenêtre de commandes, tapez :

    tailscale funnel --bg 8080

(le même nombre qu'à l'étape 2). Cette commande donne une adresse publique en
`https://…` — quelque chose comme :

    https://ma-machine.exemple.ts.net/

C'est cette adresse que votre adversaire ouvrira dans son navigateur. Il n'a
**rien** à installer.

## 4. Choisir un nom de partie et envoyer son adresse

Choisissez un mot simple, sans accent ni espace : ce sera le nom de la partie.
Par exemple `dimanche`.

**Un mot NOUVEAU pour chaque nouvelle partie** (`dimanche2`, `dimanche3`…). Le
point de rendez-vous garde chaque partie sous son nom, aussi longtemps qu'il
tourne : reprendre un nom déjà joué, c'est rouvrir l'ancienne partie, déjà
terminée. C'est d'ailleurs ce qui vous sauve si une page se ferme par mégarde —
mais il faut le savoir.

- **Vous** ouvrez, dans votre navigateur :

      http://127.0.0.1:8080/index.html?partie=dimanche&siege=0

- **Votre adversaire** ouvre, dans le sien, l'adresse publique de l'étape 3
  suivie de la même chose, mais avec le siège 1 :

      https://ma-machine.exemple.ts.net/index.html?partie=dimanche&siege=1

Envoyez-lui cette adresse par le moyen que vous voulez (message, courriel).

Deux choses seulement comptent, et il ne faut pas se tromper dessus :

| ce qu'on écrit dans l'adresse | à quoi ça sert |
|---|---|
| `partie=dimanche` | le nom de la partie — **le même mot pour les deux joueurs** |
| `siege=0` ou `siege=1` | qui est qui — **un chiffre différent pour chacun** |

Le premier des deux à ouvrir sa page crée la partie ; le second la rejoint. Vous
n'avez rien d'autre à régler : la partie démarre toute seule dès que la page est
chargée.

## 5. Pendant la partie

En bas à gauche de l'écran, une petite étiquette dit toujours où l'on en est :

- « Your turn. » — c'est à vous de choisir ;
- « Waiting for the other player… » — l'autre réfléchit, patientez ;
- « The other player is away… » — l'autre a fermé sa page ou a perdu le réseau.
  La partie n'est pas perdue : elle repartira dès son retour.

Chacun ne voit que sa propre main. Personne ne peut répondre à la place de
l'autre : le point de rendez-vous refuse toute réponse qui ne vient pas du bon
siège, et écrit pourquoi dans la fenêtre de l'étape 2.

## 6. Quand la partie est finie — tout refermer

C'est **important** : les illustrations des cartes sont protégées par le droit
d'auteur, rien ne doit rester accessible depuis l'extérieur.

1. Refermer l'adresse publique :

       tailscale funnel --https=443 off

2. Vérifier que c'est bien coupé :

       tailscale funnel status

   La réponse attendue est `No serve config`. Tant que ce n'est pas ce qui
   s'affiche, quelque chose est encore ouvert.

3. Revenir à la fenêtre de l'étape 2 et appuyer sur les touches **Ctrl** et
   **C** ensemble. Le programme dit qu'il se ferme, et rend la porte `8080`.

---

## Si quelque chose ne marche pas

**Rien ne s'affiche, ou une erreur mentionne `node`.**
Le programme `node` n'a pas été trouvé. Vérifiez avec `node --version` : un
numéro doit s'afficher.

**« La porte 8080 est déjà prise. »**
Un autre programme l'utilise, ou une partie précédente n'a pas été refermée.
Recommencez l'étape 2 avec `--port 8081`, et employez `8081` partout ensuite,
y compris à l'étape 3.

**La page de l'autre joueur reste blanche, ou dit que le rendez-vous ne répond
pas.**
Trois choses à vérifier, dans cet ordre : la fenêtre de l'étape 2 est-elle
toujours ouverte ? la commande de l'étape 3 a-t-elle bien affiché une adresse en
`https://…` ? l'adresse envoyée à l'autre joueur se termine-t-elle bien par
`?partie=…&siege=1` ?

**L'étiquette reste sur « Waiting for the other player… ».**
C'est que l'autre n'a pas encore choisi, ou que sa page est fermée. Regardez la
fenêtre de l'étape 2 : elle écrit, en français et à l'heure près, chaque arrivée,
chaque départ et chaque décision reçue.

**Vous vous êtes trompé de siège (les deux avec `siege=0`).**
Fermez les deux pages, choisissez un **nouveau** nom de partie, et
recommencez l'étape 4. Une partie déjà commencée ne se corrige pas.

**Un des deux a fermé sa page par mégarde.**
Qu'il rouvre exactement la même adresse : il retrouve la partie à l'endroit
exact où elle en était. Rien n'est perdu tant que la fenêtre de l'étape 2 reste
ouverte.

**Une réponse a été « refusée », ou l'étiquette parle de désaccord entre les deux
écrans.**
Rechargez la page (touche F5) : elle reprend la partie exactement où elle en
était, rien n'est perdu. Cela arrive si deux pages ont été ouvertes par erreur
sur le **même** siège : refermez celle qui est en trop avant de recharger.

## Et si tout échoue : la partie de secours

Le jeu à deux **sur un seul écran** n'a pas changé. Il ne dépend ni de l'adresse
publique de l'étape 3, ni du nom de partie, ni de rien de ce qui précède : ni le
moteur du jeu ni la page n'ont été touchés. Il faut seulement un moyen de
donner la page au navigateur, et l'étape 2 en est un — c'est pourquoi l'adresse
ci-dessous s'en sert, faute de plus simple sous la main :

    http://127.0.0.1:8080/index.html?graine=7

Il n'y a **pas** de nom de partie dans cette adresse, et c'est tout ce qui
compte : sans lui, la page ne cherche aucun adversaire à distance. Les deux
joueurs se passent alors la souris, comme avant. Si même la commande de
l'étape 2 refuse de démarrer, n'importe quel autre moyen de servir le dossier
`outputs/webapp` fait tout aussi bien l'affaire.

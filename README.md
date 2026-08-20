# Terra — une intelligence artificielle pour *Terraforming Mars : Ares Expedition*

> ## ⚠️ Projet de fan, non officiel — Unofficial fan project
>
> **Français.** Ce dépôt n'est **affilié ni approuvé ni soutenu** par FryxGames,
> Asmodee, Intrafin, ni aucun éditeur ou ayant droit de *Terraforming Mars* ou de
> *Terraforming Mars : Ares Expedition*. C'est un projet personnel, non commercial,
> écrit par un joueur pour un joueur. *Terraforming Mars* et *Ares Expedition* sont
> des marques de leurs titulaires respectifs.
>
> **Ce dépôt ne distribue aucune illustration, aucun visuel de carte, aucune photo
> de matériel, aucune page du livret de règles.** Pour utiliser ce programme, il
> faut **posséder le jeu**. Achetez-le : [FryxGames](https://www.fryxgames.se/games/terraforming-mars/).
>
> **English.** This repository is **not affiliated with, endorsed by, or sponsored
> by** FryxGames, Asmodee, or any publisher or rights holder of *Terraforming Mars*
> or *Terraforming Mars: Ares Expedition*. It is a personal, non-commercial fan
> project. All trademarks belong to their respective owners.
>
> **No card art, no component images, and no rulebook pages are distributed here.**
> You need to own the game to use this software.
>
> **Rights holders:** if anything in this repository concerns you, open an issue or
> contact me and I will remove it promptly, no argument.

## Ce que c'est

Un moteur de règles écrit en Rust pour le **jeu de cartes** *Ares Expedition* (pas
le jeu de plateau *Terraforming Mars* d'origine — ce sont deux jeux différents), et
une intelligence artificielle qui apprend à y jouer toute seule, en jouant contre
elle-même.

Il y a trois morceaux :

| Dossier | Ce qu'il contient |
|---|---|
| `engine/` | le moteur de règles et l'apprentissage, en Rust |
| `web/webapp/` | une interface de jeu dans le navigateur, en JavaScript, qui appelle le même moteur compilé en WebAssembly |
| `docs/` | le journal de bord du projet, les audits, les plans — en français |

L'intelligence artificielle est un réseau de neurones simple : il regarde une fiche
de 1 630 cases décrivant la situation, et prédit qui va gagner la partie. Le joueur
essaie chaque coup possible, demande au réseau ce qu'il en pense, et garde le
meilleur. Le réseau n'a jamais lu une partie humaine : il n'apprend que de ses
propres parties.

## Compiler et lancer

Il faut [Rust](https://rustup.rs/) (édition stable).

```sh
cd engine
cargo build --release
cargo test                         # ~1 200 tests
```

Huit programmes sont produits dans `engine/target/release/` :

| Programme | À quoi il sert |
|---|---|
| `simulate` | joue N parties au hasard et vérifie les invariants des règles |
| `entraine` | entraîne le réseau de neurones |
| `predire` | mesure la force d'un fichier de poids |
| `jouer` | joue une partie complète entre deux intelligences artificielles |
| `decrire` | affiche la fiche de situation vue par le réseau |
| `deviner` | outils de la voyance (ce que l'IA sait des cartes cachées) |
| `mesures` | campagnes de mesure |
| `chrono` | mesures de temps |

Exemple — 300 parties aléatoires, avec l'extension *Discovery* :

```sh
./target/release/simulate --games 300 --seed 4242 --boites base,decouverte
```

L'interface du navigateur se reconstruit avec `web/construire.sh`, puis se sert
depuis `web/webapp/` par n'importe quel serveur de fichiers statiques.

## Les images : à vous de les fournir

`web/webapp/assets/manifeste.json` associe chaque carte à un nom de fichier
attendu dans `web/webapp/assets/cartes/`, et note d'où vient l'image d'origine
(une numérisation de vos propres cartes). Les dossiers `assets/cartes/`,
`assets/plateau/` et `assets/menu/` sont **volontairement vides** dans ce dépôt et
exclus du suivi de version. Si vous possédez le jeu, numérisez vos cartes et
remplissez ces dossiers en suivant les noms du manifeste.

Sans images, l'interface se lance et fonctionne : chaque carte porte son nom en
clair sous son emplacement. Mais les emplacements d'images restent vides — le
repli propre, qui dessinerait la carte au lieu de la montrer, n'est pas encore
écrit. C'est une limite connue, pas un défaut de règles.

## Ce qui vient d'ailleurs, et sous quelle licence

- **Les données de cartes** (`data/cards.json` : nom, coût, catégorie, étiquettes,
  points de victoire) sont dérivées de
  [`nikitinalexx/ares-expedition`](https://github.com/nikitinalexx/ares-expedition),
  sous licence GPLv3. Merci à ses auteurs. Le champ de texte de chaque carte n'est
  là que pour l'affichage ; le moteur n'en lit pas un mot, il applique des effets
  codés à la main.
- **L'image du sol martien** (`web/webapp/assets/plateau/`) : NASA / JPL-Caltech /
  University of Arizona, caméra HiRISE, domaine public. Détail et obligation de
  mention dans `assets/plateau/CREDITS-sol-martien.md`.
- **Les polices** `DejaVu`, sous leur licence libre d'origine.

Aucune ligne du code de [`bnordli/rftg`](https://github.com/bnordli/rftg) n'a été
copiée : ce projet a été lu comme référence de conception, jamais recopié.

## Licence

Le code de ce dépôt est publié sous **GPLv3** — voir `LICENSE`. Cela ne s'étend ni
aux marques, ni aux illustrations, ni aux règles du jeu, qui appartiennent à leurs
titulaires et ne sont pas distribuées ici.

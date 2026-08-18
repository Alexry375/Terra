# Contexte d'audit — avant LE DERNIER ENTRAÎNEMENT

> Document de briefing lu par les agents d'audit. Écrit le 18-08-2026 par le CTO.
> Tout ce qui est marqué [VÉRIFIÉ 18-08] a été relu à la source ce jour-là.

## 1. Le projet en trois phrases

Construire une intelligence artificielle imbattable par un humain à
**Terraforming Mars : Ares Expedition** — **le jeu de CARTES**, pas le jeu de
plateau *Terraforming Mars* dont il est tiré. Deux joueurs. Boîte de base +
extension **Discovery**. L'adversaire visé est un humain nommé Corentin.

Le moteur de règles (Rust, `engine/`) et l'interface (JavaScript, `web/webapp/`)
sont écrits par nous. Un réseau de neurones (`engine/src/reseau.rs`) juge les
situations ; le joueur (`engine/src/joueur.rs`) essaie chaque option et retient
celle que le réseau note le mieux.

## 2. Pourquoi cet audit maintenant

Nous avons déjà entraîné plusieurs réseaux jusqu'à 1,5 million de parties. Deux
défauts d'architecture viennent d'être découverts, tous deux invisibles depuis
le code pris isolément et trouvés seulement en mesurant le comportement :

**Défaut n°1 — le mulligan des corporations est structurellement aveugle.**
[VÉRIFIÉ 18-08] La description publie `corpo_<nom>_moi` et `corpo_<nom>_adv`
seulement pour la corporation **installée** (`description.rs:356-360`,
`moi.corporation` est une option vide avant le choix final). Au moment du
mulligan, aucune n'est installée : les deux corporations tenues ne figurent
**nulle part** dans les 1 472 entrées. Preuve : les deux options de cette
décision reçoivent une note identique à 17 décimales
(`jouer --graine 700001 --poids data/poids/apprenti-1200k.txt --boites base,decouverte --tracer-rang 0`).
Conséquence mesurée : 400 gardes sur 400, jamais un remplacement.

**Défaut n°2 — le joueur voit le hasard futur quand il essaie une option.**
[VÉRIFIÉ 18-08] Pour noter une option, `joueur.rs:325-373` reconstruit la partie
avec `setup_game(self.db, self.seed, &mut rejeu)` où `self.seed` est **la graine
de la partie en cours** (`entraine.rs:296-297`). Le JavaScript fait pareil en
espionnant la graine vivante (`web/webapp/joueurs/apprenti.js:349-354, 482`).
Donc en essayant « je rends ces cartes », il obtient **exactement** les cartes de
remplacement que la vraie partie donnera. Démonstration sur la graine 700001 :
quelles que soient les cartes rendues, les cartes reçues sont toujours
*Developed Infrastructure*, puis *Vesta Shipyard*, puis *Aerated Magma* — le
dessus du paquet, lisible par simple essai. Conséquence mesurée : il ne rend que
2 cartes sur 8 au mulligan des projets, là où un témoin à règles écrites en rend 6.

Ces deux défauts n'ont pas été trouvés par relecture de code mais par mesure du
comportement. **C'est le mode de recherche que cet audit doit reproduire.**

## 3. Ce qui va se passer ensuite

Un **dernier entraînement**, lancé depuis zéro, avec toutes les corrections et
optimisations retenues. Il n'y en aura pas d'autre. Tout ce qui doit être décidé
doit l'être maintenant.

Les résultats de l'IA actuelle sont conservés comme témoin de comparaison :
classement des corporations, taux de bons choix, comportement en mise en place,
duels. Ils sont dans `data/mesures/` et résumés dans `docs/CTO_STATE.md`.

## 4. Faits d'architecture, vérifiés le 18-08

| point | valeur | source |
|---|---|---|
| entrées de la description | 1 472 | `description.rs`, `desc.taille` |
| forme du réseau | 1 couche cachée, **50 neurones**, tangente hyperbolique, 2 sorties, exponentielle normalisée | `reseau.rs:54-55` |
| poids | ≈ 73 650 pour la couche cachée | `reseau.rs` en-tête |
| apprentissage | différences temporelles, λ = 0,9, pile de 120 situations | `reseau.rs:107,123` |
| taux d'apprentissage | 0,0001 (second réseau : 0,0005) | `reseau.rs:65,95` |
| rythme des corrections | une situation sur 8 | `reseau.rs:112` |
| amorçage | 5 000 parties, facteur 10 | `reseau.rs:114-116` |
| **parallélisme** | **AUCUN** — pas de `rayon`, pas de `std::thread`, dépendances = `rand`, `serde`, `serde_json` | `engine/Cargo.toml`, grep vide |
| coût mesuré | 60 530 s pour 1 000 000 de parties (≈ 60 ms/partie) ; 85 547 s avec la devinette | `docs/CTO_STATE.md` |
| machine | 8 cœurs, Linux ; **7 cœurs inutilisés pendant l'entraînement** | `nproc` |
| rejeu d'essai | rejoue depuis la mise en place (`Reprise::MiseEnPlace`) ou depuis le début de manche (`Reprise::Manche`) | `joueur.rs:350-364` |
| tests Rust du moteur | `flow_tests.rs` = **178 lignes** pour `flow.rs` = 5 425 lignes | `wc -l` |
| vérifications JS/Python | ≈ 25 programmes dans `web/webapp/verif/` | `ls` |
| cartes | `data/cards.json` ; 16 corporations distribuées (12 base + 4 Discovery) ; 248 cartes projets | `boites::composer` |

## 5. Règles absolues pour tout agent d'audit

1. **Ne jamais lire, chercher, télécharger ou citer le dépôt `bnordli/rftg`
   (Keldon Jones) ni aucun code sous licence GPL.** Le projet ne doit pas être
   contaminé. Cette règle est sans exception.
2. **Ne lire aucune image.** La liaison montante de la machine est limitée à
   200 Ko/s ; lire des visuels de cartes bloque tout le reste. La vérification à
   l'image se fait séparément, par le CTO.
3. **Ne rien modifier.** Cet audit produit un rapport, pas un correctif. Aucune
   écriture dans `engine/`, `web/`, `data/`.
4. **Ne pas lancer de calcul long.** Des entraînements et des bancs de mesure
   tournent déjà et saturent la machine. Toute commande doit rendre la main en
   moins de 60 secondes. Pas de `cargo build --release` sur tout le dépôt sans
   nécessité ; le binaire compilé existe déjà dans `engine/target/release/`.
5. **Chaque constat doit être prouvé**, par `fichier:ligne` relu, ou par une
   commande courte et reproductible avec sa sortie. Un constat non prouvé sera
   rejeté. Les erreurs les plus coûteuses de ce projet sont venues de fiches
   décrivant des défauts déjà corrigés.
6. **Distinguer le certain du probable.** Écrire « je n'ai pas pu vérifier » est
   toujours préférable à une affirmation confortable.

## 6. Ce qu'on attend d'un bon constat

- **Où** : fichier et ligne.
- **Quoi** : ce que le code fait, factuellement.
- **Pourquoi c'est un problème** : effet sur la force de l'IA ou sur la fidélité
  aux règles — chiffré si possible.
- **Preuve** : commande + sortie, ou citation exacte du code.
- **Correctif proposé** : concret, avec son coût approximatif et son risque.
- **Gain attendu** : en points de score, en pourcentage de victoires, ou en
  facteur de vitesse. Dire « inconnu » si inconnu.

## 7. Les règles font foi — et elles sont dans le dépôt

**Source de vérité des règles** : `docs/regles/livret-base.md` et
`docs/regles/livret-decouverte.md` — transcriptions des livrets officiels, faites
à partir des photos par un travail antérieur. Voir aussi
`docs/regles/notes/regles-condensees.md`, `docs/regles/notes/cas-tranches.md`
(les cas déjà arbitrés) et `docs/regles/notes/conformite-moteur-24-07.md`.

**Ne pas chercher les règles sur le web.** La quasi-totalité de ce qu'on y trouve
concerne *Terraforming Mars*, le jeu de plateau, dont les règles sont
DIFFÉRENTES. Une comparaison faite contre les mauvaises règles produirait des
constats faux et coûteux. Si les livrets du dépôt ne tranchent pas un cas, le
dire explicitement au lieu de combler.

**Règles maison d'Alexis, volontairement différentes du livret** — ne pas les
signaler comme des défauts :
- la mise en place suit l'ordre : 2 corporations → mulligan des corporations
  (les DEUX ou aucune, sans voir les projets) → 8 cartes projets → mulligan des
  projets (0 à 8, carte par carte) → choix final de corporation, projets en main
  (`flow.rs:53-60`) ;
- l'ordre des cinq phases est **I Développement, II Construction, III Action,
  IV Production, V Recherche** (`flow.rs:1731-1732`).

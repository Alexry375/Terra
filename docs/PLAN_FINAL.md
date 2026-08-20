# PLAN FINAL — du moteur fautif au dernier entraînement

> **Ce document est l'index maître.** Il consigne, en un seul endroit, tout ce que
> les deux audits du 18-19 août ont remonté, tout ce qu'Alexis a tranché, et
> l'ordre dans lequel les chantiers s'exécutent. Rien de ce qui a été trouvé ne
> doit exister ailleurs sans être référencé ici.
>
> Créé le 19-08-2026. Tenu à jour à chaque livraison de lot.

---

## 0. OÙ TOUT EST CONSIGNÉ

| Document | Contenu | État |
|---|---|---|
| `docs/AUDIT_ENTRAINEMENT.md` | Audit de l'architecture d'entraînement : 17 changements, 9 constats réfutés, 9 incertitudes déclarées | figé, commit `3245bb3` |
| `docs/AUDIT_MOTEUR.md` | Audit du moteur de règles : 25 défauts confirmés, 4 réfutés, ce qui n'a pas pu être vérifié | figé, commit `3245bb3` |
| `docs/TEMOIN_AVANT_AUDIT.md` | **Toutes** les mesures de l'IA « voyante » avant correction, chaque tableau avec ses réserves | figé, commit `8b4776f` |
| `data/temoin/temoin-voyant-1M.txt`, `…-1200k.txt` | Les poids figés de cette IA, pour rejouer les duels après correction | figé |
| `docs/regles/README.md` | Les règles maison d'Alexis **et** les arbitrages du 19-08. **Fait autorité.** | vivant |
| `docs/REGISTRE_MOTEUR_INTERFACE.md` | Chaque changement du moteur et sa répercussion sur l'interface | vivant |
| `docs/CTO_STATE.md` | Carte d'état ancrée code | vivant |
| `docs/JOURNAL.md` | Le récit daté, échecs compris | vivant |
| **ce fichier** | Le plan d'attaque et l'avancement des lots | vivant |

**Règle de consignation** : un lot n'est déclaré fini que lorsque son résultat est
écrit ici **et** dans le registre de l'interface. Un résultat qui n'existe que dans
une conversation est un résultat perdu.

---

## 1. CE QU'ALEXIS A TRANCHÉ (19-08)

| Question | Décision |
|---|---|
| Piste de température | **20 crans** vérifiés sur le plateau physique. Le moteur est juste ; c'est la transcription de la photo qui était fautive |
| *Mining Guild* | La seconde ligne s'applique : **1 NT par acier gagné** (deux aciers d'un coup = 2 NT) |
| Premier joueur | **Tiré au sort**, puis alterné à chaque manche |
| Mise en place | **Simultanée** au mulligan de départ : aucun joueur ne voit les cartes rendues ni la corporation de l'autre avant d'avoir répondu. En cours de partie, la défausse reste publique |
| Extension seule | **Refusée au chargement** |
| Objectifs et Récompenses | Comptés **seulement** si l'extension est en jeu |
| Phase IV Production | Passe à l'ordre du tour comme les quatre autres phases |
| Carte Phase | Choix **secret et simultané** des deux joueurs |
| Départage d'égalité | **Adopté** — le livret p. 16 s'applique (chaleur + MC + plantes, cartes en main converties à 3 MC). Revient sur la règle maison du 24-07, prise sans connaître la règle officielle |

---

## 2. INVENTAIRE COMPLET DES PROBLÈMES

### 2.1 Moteur de règles — 25 défauts confirmés

| # | Défaut | Fichier principal | Lot |
|---|---|---|---|
| D1 | Le siège 1 voit la carte Phase posée face cachée par le siège 0 | `flow.rs:5313`, `description.rs:389` | L1 |
| D2 | *Mining Guild* n'applique jamais la seconde ligne de son carton | `effects.rs:2734` | L2 |
| D3 | Le mulligan des corporations se décide à l'aveugle (notes identiques à 17 décimales) | `description.rs:356` | L3 |
| D4 | `description.rs` n'est couvert par **aucun** test (vérifié : zéro fichier de `engine/tests/` ne le cite) | — | L3 + L7 |
| D5 | Le badge joker est gelé dès qu'on regarde la carte | `flow.rs` | L2 |
| D6 | Le bonus de phase III est attaché de force à la première carte activée | `flow.rs` | L2 |
| D7 | Phase III améliorée B : la même action peut être activée trois fois | `flow.rs` | L2 |
| D8 | Le basculement A→B est imposé sans demander | `flow.rs` | L2 |
| D9 | Une action peut détruire des ressources pour un paramètre déjà au maximum | `flow.rs` | L2 |
| D10 | Objectifs et Récompenses tirés et comptés même en boîte de base seule | `flow.rs:157`, `:5283` | L1 |
| D11 | Le départage d'égalité du livret n'existe pas (2,8 % de parties nulles) | `flow.rs` | L1 |
| D12 | Deux bancs de vérification calculent la faute puis ne tombent pas dessus | bancs | L7 |
| D13 | Le contrôle « aucun pouvoir sauté » ne voit pas une corporation à moitié encodée | bancs | L7 |
| D14 | Mise en place séquentielle : le siège 1 voit les cartes rendues et la corporation de l'autre | `flow.rs:200-236` | L1 |
| D15 | Extension seule : la mise en place s'interrompt, paquet de corporations vide | `flow.rs:188`, `cards.rs:535` | L1 |
| D16 | La phase IV Production ne suit pas l'ordre du tour | `flow.rs:4836` | L1 |
| D17 | L'Objectif « Terraformeur » est perdu si le niveau redescend dans la phase | `flow.rs:5055` | L2 |
| D18 | Phase I améliorée B : seconde carte verte accordée sans première carte, remise de 3 MC perdue | `flow.rs:4487` | L2 |
| D19 | Deux effets déclenchés ne se résolvent qu'une fois (sans effet observable aujourd'hui) | `effects.rs:1465` | L2 |
| D20 | Une réduction de coût compte la présence d'un badge, pas leur nombre (sans effet observable) | `effects.rs:405` | L2 |
| D21 | Deux cartes inexistantes portent le drapeau « dans la pioche » : 248 comptées au lieu de 246 | `data/cards.json` | L2 |
| D22 | Un commentaire cite un chemin de fichier inexistant et une empreinte fausse | `boites.rs:41` | L2 |
| D23 | `cards.json` dupliqué à l'octet entre moteur et interface, sans contrôle qui les compare | — | L6 + L7 |
| D24 | Deux commentaires affirment qu'une amélioration de phase n'est pas gérée alors qu'elle l'est | `effects.rs:1646` | L2 |
| D25 | L'équivalence moteur/interface n'est établie que sur les graines 1, 2 et 3 | `juge-meme-option.mjs` | L7 |

**Faux défaut retiré** : D11 avait d'abord été écarté comme conforme à la règle
maison « égalité sèche » du 24-07 ; Alexis a changé cette règle le 19-08, il
redevient donc un chantier. La cause de la confusion — mon briefing d'audit ne
recopiait que 2 des 5 règles maison — est corrigée : le contrat de chaque
workspace recopiera **le bloc entier** de `docs/regles/README.md`.

### 2.2 Défaut hors audit, trouvé le 18-08

| # | Défaut | Fichier | Lot |
|---|---|---|---|
| V1 | **Le joueur voit le hasard futur** : chaque essai de coup rejoue la partie depuis la graine réelle, donc l'IA connaît les cartes qu'elle recevra. Démontré graine 700001 : les cartes reçues sont les mêmes quoi qu'on rende | `joueur.rs:352`, `apprenti.js:349` | L4 |

### 2.3 Architecture d'entraînement — 17 chantiers

| # | Chantier | Nature | Lot |
|---|---|---|---|
| 2.1 | Tirage de départ des poids 0,1 → **0,045** : un neurone sur huit est figé | réglage, **zéro ligne** | L5 |
| 2.2 | Éteindre la devinette : −41 % de temps, aucun gain mesuré au million | réglage | L5 |
| 2.3 | Machine libre pendant l'entraînement : ×1,45 à 1,53 | hygiène | L5 |
| 2.4 | Compiler pour le processeur réel : ×1,7 à 1,9 | vitesse | L5 |
| 2.5 | `AMORCAGE_SCORE_MAX` 49 → 120 : le palier haut n'est jamais franchi pendant l'amorçage | réglage | L5 |
| 2.6 | Protéger le dernier entraînement contre une coupure | robustesse | L5 |
| 2.7 | Répartir les parties sur les quatre cœurs : ×3,3, ~150 lignes | vitesse, **seul vrai risque** | L5 |
| 2.8 | Décrire le contenu de la main tenue (compteurs, pas seulement des drapeaux) | description | L3 |
| 2.9 | Publier les écarts entre les deux joueurs | description | L3 |
| 2.10 | Ressources posées et classement des Récompenses | description | L3 |
| 2.11 | Énumérer les 256 combinaisons à l'échange des cartes de départ (l'IA n'en essaie que 37) | comportement | L4 |
| 2.12 | Retirer les 44 entrées mortes (11 cartes jamais distribuées) | description | L3 |
| 2.13 | **Convention de graines** : entraînement > 10 000 000, mesures 1-10 M, règles < 1 M | méthode | L0 |
| 2.14 | Décaler le compteur d'apprentissage : la mise en place n'est apprise que dans 13 % des parties | comportement | L4 |
| 2.15 | Permettre à l'IA de vendre une carte (17 occasions par partie, jamais saisies) | comportement | L4 |
| 2.16 | Élargir la couche cachée de 50 à 100 ou 200 neurones | à décider | L8 |
| 2.17 | Trois corrections gratuites : formule de correction de sortie, accumulateur inutile, fin de manche exécutée pour rien | propreté | L5 |

### 2.4 Les treize constats démontés — ne pas les rouvrir

Neuf côté architecture (§4 de `AUDIT_ENTRAINEMENT.md`) et quatre côté moteur
(§5 de `AUDIT_MOTEUR.md`). Le seul rouvert est la fuite de la carte Phase, que
j'ai revérifiée moi-même : le contradicteur avait tort, elle est bien dans l'état
évalué et entraîné.

---

## 3. LES LOTS, LEUR ORDRE, LEURS TERRITOIRES

**Principe de parallélisme** : deux lots ne tournent en même temps que si leurs
listes de fichiers sont **disjointes**. `flow.rs` est touché par presque tout le
moteur : les lots L1 et L2 sont donc **séquentiels**.

| Lot | Titre | Contenu | Fichiers | Dépend de | Parallélisable avec |
|---|---|---|---|---|---|
| **L0** | La convention de graines | 2.13 + rejouer les mesures témoins sur graines neuves | scripts de mesure | — | tout |
| **L1** | Le secret et l'ordre | D1, D10, D11, D14, D15, D16 | `flow.rs`, `cards.rs`, `description.rs` (publication), tests | L0 | L4 |
| **L2** | Les règles de cartes et de phases | D2, D5, D6, D7, D8, D9, D17, D18, D19, D20, D21, D22, D24 | `flow.rs`, `effects.rs`, `data/cards.json`, tests | L1 | — |
| **L3** | La description que voit l'IA | D3, D4, 2.8, 2.9, 2.10, 2.12 | `description.rs`, `description.js`, tests | L1, L2 | L5 |
| **L4** | Le joueur : voyance, essais, vente | V1, 2.11, 2.14, 2.15 | `joueur.rs`, `apprenti.js`, tests | L0 | L1 |
| **L5** | Vitesse et réglages | 2.1 à 2.7, 2.17 | `reseau.rs`, `entraine.rs`, `Cargo.toml` | L4 | L3 |
| **L6** | L'interface remise en phase | tout le registre `REGISTRE_MOTEUR_INTERFACE.md` + les retours d'interface déjà en attente | `web/webapp/**` | L1, L2, L3, L4 | — |
| **L7** | Les tests, en force | D4, D12, D13, D23, D25 + campagne | `engine/tests/**`, `web/webapp/verif/**` | L2 | L6 |
| **L8** | Répétition générale et décision sur la largeur | 2.16 + duels de contrôle | — | L1-L7 | — |
| **L9** | **Le dernier entraînement** | — | — | L8 | — |

### Ce qui reste vrai quoi qu'il arrive

- **Toute partie enregistrée devient injouable au rejeu** dès que L1 ou L2 déplace
  un point de décision : les décisions sont des **indices** dans une liste
  d'options. Il faut donc regénérer les parties de référence après L2, et ne
  jamais modifier le moteur pendant qu'une partie est en cours.
- **Le dernier entraînement repart de zéro.** Aucun fichier de poids existant ne
  contraint un choix de ce plan.

---

## 4. LA STRATÉGIE DE TESTS

**État mesuré au 19-08** : 777 tests d'intégration, 20 444 lignes dans
`engine/tests/`, pour 17 854 lignes de moteur. La couverture du **moteur** est
substantielle. Le trou est ailleurs, et il est exactement au mauvais endroit :

- **`description.rs` : zéro test.** Vérifié — aucun des 24 fichiers de
  `engine/tests/` ne le cite. C'est la couche qui alimente le réseau : une erreur
  y est silencieuse et ruine l'entraînement sans rien casser d'observable.
- **Deux bancs calculent la faute sans tomber dessus** (D12), et un contrôle ne
  peut pas voir ce qu'il est censé attraper (D13).
- **L'équivalence moteur/interface** ne tient que sur trois graines, celles de la
  mise au point (D25).

**Ce que chaque lot doit livrer, sans exception :**
1. un test qui **échoue sur le code d'avant** et passe sur le code d'après, pour
   chaque défaut corrigé ;
2. pour toute propriété de secret (D1, D14) : un test qui compare deux fiches de
   situation et exige qu'elles soient **identiques case pour case** quand
   l'information cachée change ;
3. pour toute règle de livret : la citation `livret-*.md:ligne` dans le test ;
4. un contrôle qui vire au **rouge** sur une copie sabotée — un contrôle qui n'a
   jamais été vu rouge ne prouve rien.

**Cible chiffrée** : `description.rs` passe de 0 à au moins 40 tests ; les deux
bancs faux sont réparés et prouvés rouges sur une copie sabotée ; l'équivalence
moteur/interface passe de 3 graines à au moins 200 parties complètes.

---

## 5. LA MÉTHODE : WORKSPACES SCELLÉS

Chaque lot part en workspace `aw` (`aw new <nom>`), avec :
- un contrat écrit (`inputs/prompt.md`) qui recopie **le bloc entier** des règles
  maison, jamais un résumé de mémoire ;
- des contrôles visibles (`inputs/checks/`) — profil P1, puisqu'un oracle scriptable
  existe pour tout ce qui touche le moteur ;
- un ou trois contrôles **cachés** (hold-out), jamais mentionnés dans le workspace,
  sur des graines que le lot ne peut pas atteindre ;
- interdiction absolue de lire `bnordli/rftg` (licence GPLv2) et de regarder des
  images (débit montant limité).

À la livraison : `aw audit <nom> --mode code`, puis **lecture du chemin critique**
du code — les incidents passés n'étaient visibles que là. Puis `aw report`.

---

## 6. CRITÈRES DE RECETTE DU DERNIER ENTRAÎNEMENT

Il ne se lance que si **tous** ces points sont verts :

1. les 25 défauts et V1 sont corrigés ou explicitement écartés par Alexis ;
2. aucune fiche de situation ne contient d'information secrète de l'adversaire —
   prouvé par un test de comparaison, pas par relecture ;
3. `description.rs` est couvert ; les bancs faux sont réparés et prouvés rouges ;
4. l'interface rejoue à l'identique du moteur sur au moins 200 parties ;
5. la convention de graines est en place et le dernier entraînement tire au-dessus
   de 10 000 000 ;
6. une répétition générale de 10 000 parties tourne de bout en bout sans écart ;
7. les duels contre le témoin figé (`data/temoin/`) sont lancés et consignés.

---

## 7. CE QUE PERSONNE NE SAIT ENCORE

Écrit sans arrondir, repris des deux audits.

- **Aucun gain en points de victoire n'est chiffré nulle part.** Tous les
  changements de ce plan sont justifiés par des mécanismes et des coûts, pas par
  une mesure de force. Le seul écart jamais observé (tirage à 0,045) valait 1,6
  écart typique, c'est-à-dire rien de concluant.
- On ne sait pas si enrichir la description rend l'IA plus forte.
- On ne sait pas si 100 neurones valent mieux que 50.
- Le partage sur quatre cœurs **change l'algorithme d'apprentissage**, pas
  seulement sa vitesse.
- On ne sait pas ce que coûte de n'affronter qu'une seule version de soi-même.
- Toute mesure de temps sur cette machine varie d'un facteur 1,8 selon la charge.

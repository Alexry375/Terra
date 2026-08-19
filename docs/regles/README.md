# Règles du jeu — source de référence

Ce dossier est la **référence unique** sur les règles de « Terraforming Mars :
Expédition Arès » (+ extension « Découverte ») pour tout le projet. La hiérarchie
des sources est :

1. **Le livret physique d'Alexis** (photos ici) — vérité absolue.
2. Les scans des cartes physiques (`data/scans/`).
3. Le simulateur Java de référence (`references/` — code de sebwieser/alrusdi).
4. Le champ `description` de `data/cards.json` (paraphrase possible — le moins fiable).

En cas de conflit entre sources, **le texte imprimé gagne**, et le cas est
consigné dans `notes/cas-tranches.md`.

## Organisation

| Emplacement | Contenu |
|---|---|
| `photos/` | Les 23 photos du livret (réduites à 1600 px, lisibles). 01–19 : jeu de base ; 20–23 : Découverte. Originaux pleine taille hors git : `data/regles/photos-originales/`. |
| `transcription-brute/` | Un fichier markdown par photo, transcription mot à mot (produit par sous-agents, audité par échantillon). |
| `livret-base.md` | Le livret de base assemblé en un document markdown relisible. |
| `livret-decouverte.md` | Le livret Découverte assemblé. |
| `notes/cas-tranches.md` | Registre des points de règle ambigus et de la façon dont ils ont été tranchés (avec la source qui a tranché). |
| `notes/regles-condensees.md` | Les règles réécrites en version ultra-condensée (aide-mémoire ; dérivée, jamais source). |

## Règles maison d'Alexis (au-dessus du livret pour NOS parties)

- Mulligan de la main initiale : 8 cartes, tout ou rien.
- Mulligan des 2 corporations proposées, avant de voir les projets.
- Le paquet exclut les 17 cartes du pack promotionnel 2021 (non possédé).
- **Ordre du tour J1/J2** (décision 24-07) : un premier joueur est désigné et
  alterne à chaque manche ; dans une phase, les joueurs agissent en alternance
  jusqu'à ce que plus personne ne souhaite jouer. (Le livret, lui, fait tout
  résoudre « en simultané » sans ordre — la règle p. 14 « pendant la phase où
  un paramètre atteint son maximum, tous les joueurs peuvent continuer et
  reçoivent tous les avantages » reste appliquée.)
- ~~**Égalité au score final = égalité** (décision 24-07) : pas de départage par
  chaleur + MC + plantes.~~ **CADUQUE — révoquée le 19-08.** Alexis avait pris
  cette décision sans savoir que le livret prévoyait un départage. Le départage
  officiel s'applique désormais (livret p. 16, `livret-base.md:461`) : le plus
  grand total cumulé de chaleur, de MC et de plantes l'emporte, cartes Projet en
  main converties au préalable à 3 MC chacune.

## Arbitrages du 19-08 (Alexis, après les deux audits)

- **Mining Guild** : la seconde ligne du carton s'applique. « Each time you play
  steel production, excluding this, gain 1 TR » = **1 NT par acier gagné** (une
  carte qui apporte deux aciers donne donc 2 NT). L'acier est déjà compté par le
  moteur (`state.rs:269`, `steel_capacity`), il ne manque que l'écouteur.
- **Premier joueur** : **tiré au sort** au départ, puis alterné à chaque manche
  comme le prévoit déjà la règle maison du 24-07.
- **Mise en place** : les deux joueurs reçoivent l'information **simultanément**
  au mulligan de départ (ni les cartes rendues ni la corporation installée par
  l'autre ne sont visibles avant). En cours de partie, la défausse reste publique.
- **Extension seule** : configuration **refusée au chargement** (elle ne se joue
  pas sans la boîte de base).
- **Objectifs et Récompenses** : comptés **seulement** si l'extension est en jeu.
- **Phase IV Production** : passe à l'ordre du tour, comme les quatre autres.
- **Carte Phase** : le choix des deux joueurs est **secret et simultané** ; aucun
  des deux ne voit celui de l'autre avant que les deux aient répondu.

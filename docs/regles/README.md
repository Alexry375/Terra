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

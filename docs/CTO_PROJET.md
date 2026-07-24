# Configuration CTO — Projet « Terra »

> Fichier lu par le rituel `/bienvenue`. Tout ce qui est propre à CE projet vit ici.

## Identité du projet

- **Nom** : Terra
- **Objectif final** : construire une intelligence artificielle **imbattable par des
  humains** au jeu de société *Terraforming Mars : Expédition Arès*.
- **Échéance** : aucune définie à ce jour (ne pas en inventer).
- **Fuseau horaire d'Alexis** : America/Martinique.

## Périmètre souhaité (exprimé par Alexis le 2026-07-23)

1. Jouable **en ligne** et, idéalement, en **version physique** (analyse vidéo du
   plateau en direct, ou à défaut annonce orale des cartes par l'adversaire —
   avec la contrainte que l'IA doit connaître sa propre main).
2. Prise en charge de l'extension **« Découverte »** : objectifs, récompenses,
   améliorations de phases, nouvelles corporations.
3. **Interprétabilité** : statistiques de jeu (meilleures cartes, probabilité de
   victoire selon l'état de la partie, comme aux échecs). La force de jeu prime,
   les statistiques sont un plus.
4. Règles maison : **mulligan des 8 cartes projets** de départ (tout ou rien) et
   **mulligan des 2 corporations** (les 2 ou aucune, avant réception des cartes
   projets ; le choix final de corporation peut se faire cartes projets en main).

## Contraintes matérielles

- Carte graphique locale : **RTX 3060**. L'entraînement lourd se fera peut-être
  sur des machines louées en ligne — à arbitrer.

## Où sont les documents de pilotage

- **Carte d'état** (source de vérité, ancrée code) : `docs/CTO_STATE.md`
- **Feuille de route** : `docs/ROADMAP.md` (à créer quand la direction sera choisie)
- **Journal de bord** : `docs/JOURNAL.md` (une entrée par journée de travail)

## Rituels spécifiques à ce projet

- Aucun pour l'instant.

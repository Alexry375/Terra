#!/usr/bin/env python3
"""CE QUE LE CONTROLE 03 NE VOIT PAS : la planche des oceans dit-elle VRAI ?

Le controle livre verifie trois choses : neuf emplacements, autant de faces
visibles que le moteur en a revele, et rien qui fuite d'une tuile encore
retournee. Il ne verifie PAS que la tuile montree est la bonne — il compare le
nombre, jamais l'identite. Une planche qui retournerait les neuf tuiles dans le
desordre, ou qui afficherait 4 MC la ou le moteur donne 1 carte, passerait.

Ce banc-ci le verifie, contre un oracle EXTERIEUR a la page : la table
`OCEAN_TILES` de `engine/src/state.rs`, recopiee ci-dessous ligne a ligne. Pour
chaque tuile revelee, on exige :

  1. `data-ocean-id` est un rang de cette table (0..8) ;
  2. `data-ocean-bonus` est EXACTEMENT le bonus que la table donne a ce rang ;
  3. le scan affiche est celui de ce bonus-la — le nom de fichier est relu, et
     la table des noms est celle des icones reellement imprimees sur les scans
     (une carte + un MC pour « bonus-1-mc », qui ne dit que le MC) ;
  4. deux emplacements ne montrent jamais la meme tuile ;
  5. un emplacement deja revele une fois remontre TOUJOURS la meme tuile.

Et pour chaque tuile encore retournee : aucun `data-ocean-id`, aucun
`data-ocean-bonus`, et le seul scan present est le dos.

CE QUI N'EST PAS EXIGE ICI, ET POURQUOI. Le nombre de faces visibles n'est pas
tenu de croitre d'une lecture a l'autre : la page rend parfois un etat plus
ancien que le precedent (46 reculs d'etat mesures sur une partie de la page
d'ORIGINE, avant ce chantier — `generation` et `planet.oceans` reculent
ensemble). La planche suit fidelement l'etat qu'on lui donne, y compris quand
il recule ; exiger la croissance ici mesurerait l'ordonnancement des rendus, pas
la planche. Le defaut est signale dans `outputs/journal.md`, non corrige.

Depuis la racine du workspace :

    python3 web/webapp/verif/oceans.py [graine]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer, echec  # noqa: E402

# `engine/src/state.rs`, constante `OCEAN_TILES` : (cards, mc, plants) par rang.
OCEAN_TILES = [
    (0, 0, 2), (0, 4, 0), (1, 1, 0), (0, 2, 1), (1, 0, 1),
    (1, 0, 0), (0, 1, 1), (1, 0, 0), (0, 0, 2),
]

# Le scan attendu pour chaque bonus, releve sur les images elles-memes.
SCANS = {
    (0, 0, 2): "tuile-ocean-bonus-2-plantes",
    (0, 4, 0): "tuile-ocean-bonus-4-mc",
    (1, 1, 0): "tuile-ocean-bonus-1-mc",
    (0, 2, 1): "tuile-ocean-bonus-1-plante-et-2-mc",
    (1, 0, 1): "tuile-ocean-bonus-1-carte-et-1-plante",
    (1, 0, 0): "tuile-ocean-bonus-1-carte",
    (0, 1, 1): "tuile-ocean-bonus-1-plante-et-1-mc",
}
DOS = "tuile-ocean-dos-orange"

LECTURE = """() => {
  const out = [];
  for (const e of document.querySelectorAll('[data-ocean-tuile]')) {
    const im = e.querySelector('img');
    out.push({
      revelee: e.getAttribute('data-ocean-revelee'),
      id: e.getAttribute('data-ocean-id'),
      bonus: e.getAttribute('data-ocean-bonus'),
      src: im ? im.getAttribute('src') : null,
      images: [...e.querySelectorAll('img')].map((i) => i.getAttribute('src')),
    });
  }
  return out;
}"""

fautes = []
vu = {"mesures": 0, "max": 0, "identites": 0}
connu = {}   # emplacement -> identite deja vue


def bonus(txt):
    """`cards=1,mc=1,plants=0` -> (1, 1, 0)."""
    d = dict(p.split("=") for p in (txt or "").split(","))
    return (int(d["cards"]), int(d["mc"]), int(d["plants"]))


def controle(pg, rang):
    tuiles = pg.evaluate(LECTURE)
    if len(tuiles) != 9:
        fautes.append(f"decision {rang} : {len(tuiles)} emplacements")
        return
    vu["mesures"] += 1
    revelees = [i for i, t in enumerate(tuiles) if t["revelee"] == "oui"]
    vu["max"] = max(vu["max"], len(revelees))
    # Les faces visibles sont toujours les PREMIERS emplacements, sans trou :
    # la planche se remplit dans l'ordre ou le moteur revele.
    if revelees != list(range(len(revelees))):
        fautes.append(f"decision {rang} : faces visibles aux places {revelees}, "
                      "la planche devrait se remplir dans l'ordre")

    ids = []
    for i, t in enumerate(tuiles):
        if t["revelee"] != "oui":
            if t["id"] is not None or t["bonus"] is not None:
                fautes.append(f"decision {rang}, place {i} : tuile cachee qui porte "
                              f"id={t['id']} bonus={t['bonus']}")
            if t["images"] != [DOS + ".webp"] and not all(DOS in s for s in t["images"]):
                fautes.append(f"decision {rang}, place {i} : tuile cachee, images "
                              f"{t['images']}")
            continue
        if t["id"] is None or not t["id"].isdigit() or not 0 <= int(t["id"]) <= 8:
            fautes.append(f"decision {rang}, place {i} : identite « {t['id']} »")
            continue
        ident = int(t["id"])
        ids.append(ident)
        try:
            b = bonus(t["bonus"])
        except Exception:
            fautes.append(f"decision {rang}, place {i} : bonus illisible « {t['bonus']} »")
            continue
        vu["identites"] += 1
        if b != OCEAN_TILES[ident]:
            fautes.append(f"decision {rang}, place {i} : la tuile {ident} vaut "
                          f"{OCEAN_TILES[ident]} dans le moteur, l'ecran dit {b}")
        attendu = SCANS.get(b)
        if attendu and (not t["src"] or attendu + ".webp" not in t["src"]):
            fautes.append(f"decision {rang}, place {i} : bonus {b} montre par "
                          f"« {t['src']} », on attend « {attendu} »")
        if i in connu and connu[i] != (ident, b):
            fautes.append(f"decision {rang}, place {i} : etait {connu[i]}, devient "
                          f"{(ident, b)} — une tuile revelee a change d'identite")
        connu[i] = (ident, b)

    if len(set(ids)) != len(ids):
        fautes.append(f"decision {rang} : deux emplacements montrent la meme tuile {ids}")


graine = sys.argv[1] if len(sys.argv) > 1 else "5150"
with serveur() as base:
    with page(f"{base}/?graine={graine}&siege=0") as (pg, erreurs, _):
        controle(pg, 0)
        jouer(pg, apres=controle)
        if erreurs:
            echec(f"{len(erreurs)} erreur(s) de console : {erreurs[0]}")

print(f"{vu['mesures']} planches lues, {vu['identites']} identites verifiees contre "
      f"OCEAN_TILES, jusqu'a {vu['max']} tuiles revelees")
if vu["identites"] < 100:
    echec(f"seulement {vu['identites']} identite(s) verifiee(s)")
if vu["max"] != 9:
    echec(f"{vu['max']} tuiles revelees au plus : la partie devrait aller a neuf")
if fautes:
    for f in fautes[:8]:
        print("  " + f)
    echec(f"{len(fautes)} defaut(s) sur la planche des oceans")
print("OK chaque tuile revelee est celle que le moteur annonce, et rien d'autre")

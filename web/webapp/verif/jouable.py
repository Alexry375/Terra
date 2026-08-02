#!/usr/bin/env python3
"""LE CONTOUR VERT DIT-IL VRAI *DANS LA MAIN* ?

Le controle 04 livre compare l'ensemble des `data-jouable="oui"` a l'ensemble
des options. Comme les options portent elles-memes ce marquage (sans quoi les
decisions dont les options ne sont PAS en main — piocher et garder, choisir sa
corporation, viser une carte en jeu — le feraient echouer), cette comparaison ne
dit rien du marquage de la MAIN. Ce banc-ci le dit :

  pour chaque decision, l'ensemble des cartes DE LA MAIN marquees « oui » doit
  etre exactement l'intersection des cartes de la main et des options offertes,
  et aucune carte de la main ne doit rester sans marquage.

Depuis la racine du workspace :  python3 outputs/verif/jouable.py [graine]
"""
import sys
# Le module de pilotage vit a cote de ce banc dans le depot ; on l'importe par
# le chemin de CE fichier, pour que le banc tourne de n'importe ou.
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import serveur, page, jouer, echec

LECTURE = """
() => {
  // Hors décision (partie terminée), la main ne porte aucun marquage : il n'y a
  // rien à jouer, et un marquage sans décision serait un mensonge.
  if (!document.querySelector('[data-decision-rang]')) return { main: [] };
  // La main du siege regarde, en bas de l'ecran (cadre a un seul point de vue).
  const main = [...document.querySelectorAll('[data-main="mienne"] .carte--main')];
  const opt = [...document.querySelectorAll('[data-choix][data-carte-id]')]
      .filter(e => e.offsetParent !== null).map(e => e.getAttribute('data-carte-id'));
  return {
    main: main.map(e => e.getAttribute('data-carte-id')),
    oui: main.filter(e => e.dataset.jouable === 'oui').map(e => e.getAttribute('data-carte-id')),
    non: main.filter(e => e.dataset.jouable === 'non').map(e => e.getAttribute('data-carte-id')),
    sans: main.filter(e => !e.dataset.jouable).length,
    options: opt,
  };
}
"""

graine = sys.argv[1] if len(sys.argv) > 1 else "77"
ecarts, sans, vus, marquees = [], 0, 0, 0
with serveur(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")) as base:
    with page(f"{base}/?graine={graine}&boites=base,decouverte") as (pg, erreurs, _):
        def controle(p, rang):
            global sans, vus, marquees
            m = p.evaluate(LECTURE)
            if not m["main"]:
                return
            vus += 1
            sans += m["sans"]
            attendu = set(m["main"]) & set(m["options"])
            if attendu:
                marquees += 1
            if set(m["oui"]) != attendu and len(ecarts) < 5:
                ecarts.append((rang, sorted(attendu - set(m["oui"])), sorted(set(m["oui"]) - attendu)))
            # « oui » et « non » partitionnent la main : aucune carte des deux côtés.
            if set(m["oui"]) & set(m["non"]) and len(ecarts) < 5:
                ecarts.append((rang, "une carte marquee oui ET non", []))
        n, scores = jouer(pg, apres=controle)
        print(f"decisions jouees : {n}, ecrans avec une main : {vus}, "
              f"dont {marquees} ou une carte de la main etait offerte")
        if erreurs:
            echec(f"erreur de console : {erreurs[0]}")

if vus < 50:
    echec(f"seulement {vus} ecran(s) avec une main : rien n'a ete confronte")
if sans:
    echec(f"{sans} carte(s) de la main sans `data-jouable` pendant une decision")
if marquees < 10:
    echec(f"seulement {marquees} decision(s) offrant une carte de la main : trop peu pour conclure")
if ecarts:
    r, manque, trop = ecarts[0]
    echec(f"{len(ecarts)} desaccord(s) dans la main — ex. decision {r} : "
          f"offertes mais non marquees {manque}, marquees mais non offertes {trop}")
print("OK dans la main, `data-jouable=oui` est exactement main ∩ options du moteur")

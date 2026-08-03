#!/usr/bin/env python3
"""UN NUMERO NE DESIGNE JAMAIS DEUX CARTES — sur la partie ENTIERE.

Le moteur range ses cartes dans deux tables separees : les projets (numeros 0 a
~330) et les corporations (0 a ~15). Le numero publie est un rang DANS l'une ou
DANS l'autre. Le numero 7 designe donc a la fois la carte projet « Arctic
Algae » et la corporation « Inventrix ».

Le pont publie desormais la SORTE a cote du numero (`wasm/src/lib.rs`), et
l'ecran designe une carte par le COUPLE (`vue/cartes.js`, `cle`). Ce banc verifie
que ce couple tient partout et tout le temps.

Il complete `corporation.py`, qui n'inspecte QUE le choix de la corporation. Le
defaut du 02-08 a traverse seize controles precisement parce qu'ils regardaient
tous la partie en cours et sautaient la mise en place ; celui-ci fait l'inverse
de cette erreur — il regarde CHAQUE decision, du premier au dernier tour.

Quatre exigences, a chaque decision :

  1. aucun numero affiche a l'ecran ne designe deux cartes de noms differents ;
  2. deux cartes de la main n'ont jamais le meme couple ;
  3. aucune carte de la main n'est depourvue de son couple ;
  4. aucune corporation ne se trouve dans la main — elles se choisissent au
     milieu de l'ecran, pas depuis les cartes qu'on tient.

Depuis la racine du workspace :  python3 outputs/verif/identifiants.py [graines...]
Depuis la racine du depot :      python3 web/webapp/verif/identifiants.py [graines...]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import echec, jouer, page, serveur  # noqa: E402

# Les trois premieres sont les graines ou le defaut se manifestait.
GRAINES = [2026, 2043, 2052, 4242, 5150, 31337]

LECTURE = """
() => {
  const nom = e => (e.querySelector('img') || {}).alt || '?';
  const main = [...document.querySelectorAll('#mienne-rang [data-carte-id]')]
      .map(e => ({id: e.dataset.carteId, cle: e.dataset.carteCle || '', nom: nom(e)}));
  const enJeu = [...document.querySelectorAll('[data-carte-en-jeu]')]
      .map(e => ({id: e.dataset.carteEnJeu, nom: nom(e)}));
  const corpo = [...document.querySelectorAll('[data-corpo-choix]')]
      .map(e => ({id: e.dataset.corpoChoix, nom: nom(e)}));
  return {main, enJeu, corpo};
}
"""


def main():
    graines = [int(a) for a in sys.argv[1:]] or GRAINES
    fautes = []
    vus = {"n": 0}

    def controle(pg, rang):
        e = pg.evaluate(LECTURE)
        vus["n"] += 1

        cles = [c["cle"] for c in e["main"]]
        if len(set(cles)) != len(cles):
            fautes.append(f"decision {rang} : deux cartes de la main ont la meme "
                          f"cle — {cles}")
        if any(not c["cle"] for c in e["main"]):
            fautes.append(f"decision {rang} : une carte de la main n'a pas de cle")

        par_numero = {}
        for zone in ("main", "enJeu", "corpo"):
            for c in e[zone]:
                par_numero.setdefault(c["id"], set()).add(c["nom"])
        for numero, noms in par_numero.items():
            if len(noms) > 1:
                fautes.append(f"decision {rang} : le numero {numero} designe "
                              f"{sorted(noms)}")

        corpos = {c["nom"] for c in e["corpo"]}
        intrus = [c["nom"] for c in e["main"] if c["nom"] in corpos]
        if intrus:
            fautes.append(f"decision {rang} : corporation dans la main — {intrus}")

    with serveur() as base:
        for graine in graines:
            with page(f"{base}/?graine={graine}&siege=0&animations=non") as (pg, err, _):
                jouer(pg, apres=controle)
                if err:
                    fautes.append(f"graine {graine} : exception : {err[0]}")

    print(f"{vus['n']} decisions inspectees sur {len(graines)} parties, "
          f"{len(fautes)} faute(s)")
    if vus["n"] < 200:
        echec("trop peu de decisions inspectees : ce banc n'aurait rien prouve")
    if fautes:
        for f in fautes[:8]:
            print("  " + f)
        echec("un numero designe deux cartes, ou une corporation traine dans la main")
    print("OK le couple (sorte, numero) tient sur toute la partie")
    return 0


if __name__ == "__main__":
    sys.exit(main())

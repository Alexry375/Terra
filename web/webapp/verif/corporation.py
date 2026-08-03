#!/usr/bin/env python3
"""LE CHOIX DE LA CORPORATION EST-IL HONNETE ?

Ce banc existe a cause d'un defaut precis, trouve par le joueur le 02-08 en
trois minutes de jeu, et qu'aucun des seize controles du chantier « la table
vivante » n'avait vu.

CE QUI S'ETAIT PASSE. Le moteur range ses cartes dans deux tables separees :
les projets (numeros 0 a ~330) et les corporations (numeros 0 a ~15). Le numero
publie vers l'ecran est un rang DANS l'une ou DANS l'autre — jamais un numero
unique. Le numero 7 designe donc a la fois la carte projet « Arctic Algae » et
la corporation « Inventrix ».

L'ecran, lui, mettait les deux dans le meme sac : il glissait les corporations
dans la main du joueur et ecartait les doublons par leur numero seul. Mesure sur
70 parties : 3 fois, une corporation disparaissait de l'ecran, absorbee par une
carte projet portant le meme numero — et cette carte projet heritait de la
reponse « joue cette corporation ». Cliquer une carte de sa main jouait une
corporation.

CE QUE CE BANC EXIGE, a chaque choix de corporation :

  1. LES DEUX corporations proposees sont AU CENTRE de l'ecran, cliquables,
     porteuses de `data-corpo-choix` — jamais dans la main ;
  2. AUCUNE carte de la main ne se declare jouable a cet instant : le choix ne
     se joue pas depuis la main, et un contour vert y serait un mensonge ;
  3. aucune des deux corporations ne figure dans la main ;
  4. celle qu'on CLIQUE est exactement celle qui se pose : on eprouve les deux
     rangs, pour qu'un ecran qui repondrait toujours « la premiere » soit pris.

Depuis la racine du workspace :  python3 outputs/verif/corporation.py [graines...]
Depuis la racine du depot :      python3 web/webapp/verif/corporation.py [graines...]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import echec, page, serveur  # noqa: E402

# Les trois premieres sont celles ou le defaut se manifestait ; les autres sont
# des parties ordinaires, pour que le banc ne prouve pas seulement le cas connu.
GRAINES = [2026, 2043, 2052, 2024, 4242, 777, 5150, 31337]


def repondre(pg):
    """Une reponse quelconque, juste pour avancer jusqu'a la question voulue."""
    p = pg.query_selector("[data-decision-rang]")
    rang = int(p.get_attribute("data-decision-rang"))
    forme = p.get_attribute("data-decision-forme") or "simple"
    if forme in ("multiple", "montant"):
        pg.click("[data-valider]")
    else:
        visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
        if not visibles:
            raise RuntimeError(f"decision {rang} : aucun choix visible")
        visibles[0].click()
    pg.wait_for_function(
        "r => { const e = document.querySelector('[data-decision-rang]');"
        " return !e || Number(e.getAttribute('data-decision-rang')) !== r; }",
        arg=rang, timeout=15000)


def jusqu_au_choix(pg):
    """Avance jusqu'au choix de corporation du siege regarde."""
    for _ in range(12):
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        t = pg.query_selector("[data-decision-rang]").get_attribute("data-decision-type")
        if t == "pick_corporation":
            return True
        repondre(pg)
    return False


def eprouver(base, graine, rang_clique, fautes):
    with page(f"{base}/?graine={graine}&siege=0&animations=non") as (pg, erreurs, _):
        if not jusqu_au_choix(pg):
            fautes.append(f"graine {graine} : aucun choix de corporation en 12 decisions")
            return

        au_centre = pg.eval_on_selector_all(
            "#scene [data-corpo-choix]",
            "els => els.map(e => [e.dataset.corpoChoix,"
            " (e.querySelector('img') || {}).alt || ''])")
        if len(au_centre) != 2:
            fautes.append(
                f"graine {graine} : {len(au_centre)} corporation(s) au centre de l'ecran, "
                "il en faut deux — le choix n'est pas la ou il doit etre")
            return

        jouables = pg.eval_on_selector_all(
            "#mienne-rang [data-jouable='oui']",
            "els => els.map(e => (e.querySelector('img') || {}).alt || '?')")
        if jouables:
            fautes.append(
                f"graine {graine} : la main propose encore {jouables} alors que la "
                "question porte sur les corporations")

        noms_centre = {n.strip().lower() for _, n in au_centre}
        en_main = pg.eval_on_selector_all(
            "#mienne-rang [data-carte-id]",
            "els => els.map(e => (e.querySelector('img') || {}).alt || '?')")
        intrus = [n for n in en_main if n.strip().lower() in noms_centre]
        if intrus:
            fautes.append(f"graine {graine} : corporation trouvee dans la main : {intrus}")

        numero, nom = au_centre[rang_clique]
        pg.click(f'#scene [data-corpo-choix="{numero}"]')
        pg.wait_for_timeout(900)
        posee = pg.eval_on_selector("#corpo-carte-0", "e => e.dataset.corpo || ''")
        if posee.strip().lower() != nom.strip().lower():
            fautes.append(
                f"graine {graine} : j'ai clique « {nom} », l'ecran a pose « {posee} »")

        if erreurs:
            fautes.append(f"graine {graine} : exception : {erreurs[0]}")


def main():
    graines = [int(a) for a in sys.argv[1:]] or GRAINES
    fautes = []
    vus = 0
    with serveur() as base:
        for graine in graines:
            for rang in (0, 1):
                eprouver(base, graine, rang, fautes)
                vus += 1
    print(f"{vus} choix de corporation eprouves sur {len(graines)} parties, "
          f"{len(fautes)} faute(s)")
    if fautes:
        for f in fautes[:10]:
            print("  " + f)
        echec("le choix de la corporation ne tient pas")
    print("OK les deux corporations sont au centre, la main ne ment pas, "
          "et c'est bien celle qu'on clique qui se pose")
    return 0


if __name__ == "__main__":
    sys.exit(main())

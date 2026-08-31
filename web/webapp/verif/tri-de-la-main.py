#!/usr/bin/env python3
"""TRIER SA MAIN EN DEPLACANT LES CARTES (CNF-1).

Corentin, ligne 6. Le geste passe par les MEMES evenements pointeur que la pose
(`vue/geste.js`) : une machine peut donc l'emprunter exactement comme une main.

Ce banc verifie les trois choses qui peuvent casser, et la derniere est la plus
importante :

  1. la carte deplacee change bien de place dans la rangee ;
  2. AUCUNE CARTE N'EST JOUEE au passage — le rang de la decision ne bouge pas,
     la main garde le meme nombre de cartes, et la console reste muette ;
  3. l'ordre SURVIT au rendu suivant : on rejoue une decision, la main est
     reconstruite, et le rangement du joueur est toujours la.

Le point 2 est ce qui distingue une livraison d'un accident : trier sa main en
posant une carte sur la table serait pire que de ne pas pouvoir trier.

    python3 verif/tri-de-la-main.py <racine-webapp> [graine]
"""
import os
import sys

RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "web/webapp")
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page, choix_simple  # noqa: E402

ORDRE = """() => [...document.querySelectorAll('#mienne-rang > .carte--main')]
  .map((f) => f.dataset.carteCle || '?')"""

fautes = []
# COMBIEN DE GLISSERS ONT REELLEMENT EU LIEU. Ce banc-ci tombait en cinq
# secondes AVANT d'avoir glisse quoi que ce soit, en accusant le tri d'avoir
# repondu au moteur ; un banc vert qui n'aurait toujours rien glisse serait le
# meme mensonge a l'envers. Le nombre est donc compte, imprime, et exige.
glissers = 0


def glisser(pg, depuis, vers):
    """Attrape une carte et la lache ailleurs, par evenements pointeur."""
    global glissers
    glissers += 1
    a = depuis.bounding_box()
    b = vers.bounding_box()
    pg.mouse.move(a["x"] + a["width"] / 2, a["y"] + a["height"] / 2)
    pg.mouse.down()
    # Plusieurs pas : un saut unique ne franchirait pas le seuil de la meme
    # facon qu'une main, et on veut le chemin reel.
    for t in (0.25, 0.5, 0.75, 1.0):
        pg.mouse.move(a["x"] + a["width"] / 2 + (b["x"] - a["x"]) * t,
                      a["y"] + a["height"] / 2)
        pg.wait_for_timeout(20)
    pg.mouse.up()
    pg.wait_for_timeout(250)


with serveur(RACINE) as base:
    with page(f"{base}/?graine={GRAINE}&siege=0&animations=non") as (pg, erreurs, _):
        pg.wait_for_selector("#mienne-rang", timeout=20000)

        # On avance jusqu'a une main d'au moins quatre cartes : au demarrage la
        # main est vide, et il n'y a rien a trier.
        #
        # ⚠️ CORRIGE LE 28-08 (les-sept-bancs-rouges), ET C'EST LA MESURE QUI
        # ETAIT FAUSSE, PAS LA PAGE. Cette boucle sortait des que la main
        # atteignait quatre cartes, SANS regarder si une question etait posee.
        # Or `data-decision-rang` n'existe que pendant qu'une scene est dessinee
        # (`vue/scene.js`, retire par `fermerScene`) : entre deux decisions il
        # n'y en a pas. On sortait donc dans le trou entre deux questions,
        # `rang_avant` valait None, une question s'ouvrait pendant le glisser, et
        # le banc annoncait « la decision a change de rang (None -> 3) : le tri a
        # repondu au moteur » — un rouge permanent sur une mesure qui n'avait pas
        # eu lieu. Le message accusait la page de ce que le banc n'avait pas su
        # observer.
        #
        # Trois conditions pour sortir, et les trois sont necessaires :
        #   1. quatre cartes en main — sinon il n'y a rien a trier ;
        #   2. une question POSEE — sinon le rang n'existe pas et la mesure « le
        #      rang n'a pas bouge » ne veut rien dire ;
        #   3. la scene n'est pas posee EN GRAND par-dessus la table
        #      (`superposition` : mise en place, choix de la corporation), ou la
        #      main est recouverte a dessein et le geste ne l'atteindrait pas.
        def pret(pg):
            if len(pg.query_selector_all("#mienne-rang > .carte--main")) < 4:
                return False
            if pg.query_selector('#scene[data-mode="superposition"]'):
                return False
            return pg.query_selector("[data-decision-rang]") is not None

        for _ in range(200):
            if pret(pg):
                break
            porteur = pg.query_selector("[data-decision-rang]")
            if porteur is None:
                pg.wait_for_timeout(100)
                continue
            rang = int(porteur.get_attribute("data-decision-rang"))
            forme = porteur.get_attribute("data-decision-forme") or "simple"
            visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            if forme == "montant":
                champ = pg.wait_for_selector("[data-montant]", timeout=15000)
                champ.fill(champ.get_attribute("min"))
                pg.click("[data-valider]")
            elif forme == "multiple":
                brut = porteur.get_attribute("data-a-choisir")
                k = int(brut) if (brut or "").isdigit() else 1
                for c in visibles[:min(k, len(visibles))]:
                    c.click()
                pg.click("[data-valider]")
            elif visibles:
                visibles[choix_simple(rang, len(visibles))].click()
            else:
                pg.wait_for_timeout(100)
                continue
            pg.wait_for_function(
                "r => { const e = document.querySelector('[data-decision-rang]');"
                " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                " || document.querySelector('[data-partie-terminee]'); }",
                arg=rang, timeout=15000)

        cartes = pg.query_selector_all("#mienne-rang > .carte--main")
        if len(cartes) < 4:
            print(f"ECHEC : la main n'a jamais atteint quatre cartes "
                  f"({len(cartes)}) — la mesure n'a pas eu lieu")
            sys.exit(1)
        if not pret(pg):
            print("ECHEC : aucune question n'etait posee au moment de trier — "
                  "la mesure n'a pas eu lieu (voir la note de l'amorce)")
            sys.exit(1)

        # DEUX GLISSERS, ET PAS UN. Un seul deplacement ne prouve pas qu'un ORDRE
        # est conserve : il prouve qu'une carte peut bouger. Le second part d'une
        # main deja rangee par le premier — c'est la seule facon de voir que le
        # rangement precedent n'a pas ete efface au passage.
        for numero in (1, 2):
            cartes = pg.query_selector_all("#mienne-rang > .carte--main")
            avant = pg.evaluate(ORDRE)
            porteur = pg.query_selector("[data-decision-rang]")
            rang_avant = porteur.get_attribute("data-decision-rang") if porteur else None
            if rang_avant is None:
                fautes.append(f"glisser {numero} : aucune question posee au moment de "
                              "mesurer — la mesure n'a PAS eu lieu")
                break

            # La PREMIERE carte s'en va a la place de la DERNIERE.
            glisser(pg, cartes[0], cartes[-1])

            apres = pg.evaluate(ORDRE)
            porteur = pg.query_selector("[data-decision-rang]")
            rang_apres = porteur.get_attribute("data-decision-rang") if porteur else None

            print(f"    glisser {numero} — avant : {avant}")
            print(f"    glisser {numero} — apres : {apres}")

            if len(apres) != len(avant):
                fautes.append(f"la main a change de taille ({len(avant)} -> {len(apres)}) : "
                              "une carte a ete JOUEE par le geste de tri")
            elif sorted(apres) != sorted(avant):
                fautes.append("la main ne contient plus les memes cartes")
            elif apres == avant:
                fautes.append("l'ordre de la main n'a pas bouge — le tri n'a rien fait")
            elif apres[-1] != avant[0]:
                fautes.append(f"la carte deplacee ({avant[0]}) n'est pas arrivee au bout "
                              f"(la derniere est {apres[-1]})")
            elif apres[:-1] != avant[1:]:
                # LE RANGEMENT PRECEDENT SURVIT AU SUIVANT. Deplacer une carte ne
                # doit rien deranger d'autre : les autres gardent leur ordre.
                fautes.append(f"le reste de la main a bouge : {avant[1:]} est devenu "
                              f"{apres[:-1]}")
            if rang_avant != rang_apres:
                fautes.append(f"la decision a change de rang ({rang_avant} -> {rang_apres}) : "
                              "le tri a repondu au moteur")

        # L'ORDRE SURVIT-IL A UN RENDU ? On joue une decision de plus, ce qui
        # reconstruit la main, et on regarde si le rangement tient.
        # On joue jusqu'a ce que la main soit REELLEMENT RECONSTRUITE (sa
        # composition change), sinon on ne mesurerait rien : tant que les memes
        # cartes y sont, `vue/mains.js` ne retouche pas la rangee et l'ordre
        # tiendrait tout seul. C'est la RECONSTRUCTION qu'il faut eprouver.
        garde = [k for k in apres]
        mesuree = False
        for _ in range(120):
            porteur = pg.query_selector("[data-decision-rang]")
            if porteur is None or pg.query_selector("[data-partie-terminee]"):
                break
            rang = int(porteur.get_attribute("data-decision-rang"))
            forme = porteur.get_attribute("data-decision-forme") or "simple"
            visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            if forme == "montant":
                champ = pg.wait_for_selector("[data-montant]", timeout=15000)
                champ.fill(champ.get_attribute("min"))
                pg.click("[data-valider]")
            elif forme == "multiple":
                brut = porteur.get_attribute("data-a-choisir")
                k = int(brut) if (brut or "").isdigit() else 1
                for c in visibles[:min(k, len(visibles))]:
                    c.click()
                pg.click("[data-valider]")
            elif visibles:
                visibles[choix_simple(rang, len(visibles))].click()
            else:
                pg.wait_for_timeout(100)
                continue
            pg.wait_for_function(
                "r => { const e = document.querySelector('[data-decision-rang]');"
                " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                " || document.querySelector('[data-partie-terminee]'); }",
                arg=rang, timeout=15000)
            pg.wait_for_timeout(120)
            plus_tard = pg.evaluate(ORDRE)
            if set(plus_tard) == set(garde):
                continue  # meme composition : la rangee n'a pas ete refaite
            # Les cartes encore presentes doivent avoir garde leur ordre
            # relatif. Celles qui sont parties ou arrivees ne comptent pas.
            restantes = [k for k in garde if k in plus_tard]
            vues = [k for k in plus_tard if k in restantes]
            mesuree = True
            if vues != restantes:
                fautes.append(f"l'ordre n'a pas survecu a la reconstruction de la "
                              f"main : {restantes} est devenu {vues}")
            else:
                print(f"    main reconstruite ({len(garde)} -> {len(plus_tard)} cartes) : "
                      f"ordre relatif conserve {vues}")
            break
        if not mesuree:
            fautes.append("la main n'a jamais ete reconstruite apres le tri — "
                          "la survie de l'ordre n'a PAS ete mesuree")

        if erreurs:
            fautes.append(f"erreurs de console : {erreurs[:2]}")

print(f"    {glissers} glissers reellement effectues a l'ecran")
if glissers < 2:
    print(f"ECHEC : {glissers} glisser(s) : la mesure n'a pas vraiment eu lieu")
    sys.exit(1)
if fautes:
    for f in fautes:
        print("ECHEC :", f)
    sys.exit(1)
print("    la main se trie au geste, sans qu'aucune carte soit jouee")

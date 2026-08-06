#!/usr/bin/env python3
"""PASSER DÉFINITIVEMENT — et le prouver autrement que par la longueur d'un saut.

POURQUOI CE BANC EXISTE, alors que `inputs/checks/02` mesure déjà le bouton.

Le contrôle 02 vérifie qu'un clic fait avancer le rang de PLUS D'UNE décision.
C'est une mesure faible : le rang compte les décisions des DEUX joueurs, et
l'adversaire en prend plusieurs entre deux des miennes. Un bouton qui ne
passerait qu'une seule fois afficherait donc lui aussi des sauts de 2, 3 ou 4 —
et passerait le contrôle.

Ce banc mesure la seule chose qui ne peut pas mentir : **le nombre de fois où la
page m'a posé une question sur toute la partie.** On joue deux fois la même
graine, en répondant de la même façon partout ailleurs :

  · partie A — je clique « passer » à la main, chaque fois qu'il est offert ;
  · partie B — je clique « passer définitivement », chaque fois qu'il est offert.

Si le bouton enchaîne vraiment, B me pose STRICTEMENT MOINS de questions que A.
S'il ne passe qu'une fois, il m'en pose exactement autant. Et comme les deux
parties doivent finir sur le même score, il n'a pas pu répondre à autre chose.

Trois propriétés de plus, qui protègent le reste du dépôt :

  · le bouton définitif ne porte AUCUN `data-choix`. Les bancs qui pilotent la
    page tirent leur réponse au sort parmi les `data-choix` visibles
    (`pilote.choix_simple`) : un de plus changerait toutes les parties de
    référence, à commencer par celle du contrôle 03 ;
  · `.choix--passer` ne désigne JAMAIS qu'un seul élément — c'est par cette
    classe que les bancs reconnaissent le bouton « passer » ordinaire ;
  · le bouton définitif n'apparaît jamais là où « passer » n'est pas offert.

    TERRA_WEBAPP=<...>/web/webapp python3 verif/passer-en-boucle.py [graine]
"""
import os
import sys

RACINE = os.environ.get("TERRA_WEBAPP") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
sys.path.insert(0, os.path.join(RACINE, "verif"))

from pilote import serveur, page, choix_simple, echec  # noqa: E402

GRAINE = sys.argv[1] if len(sys.argv) > 1 else "4242"

SELECTEURS_DEFINITIF = ['[data-passer-toujours]', '[data-passer-definitivement]',
                        '[data-passer="toujours"]']


def bouton_definitif(pg):
    for s in SELECTEURS_DEFINITIF:
        e = pg.query_selector(s)
        if e and e.is_visible():
            return e
    return None


def jouer(pg, avec_le_bouton, m):
    """Joue la partie entière. Compte les questions POSÉES, pas les rangs."""
    for _ in range(3000):
        if pg.query_selector("[data-partie-terminee]"):
            break
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        porteur = pg.query_selector("[data-decision-rang]")
        if porteur is None:
            continue
        rang = int(porteur.get_attribute("data-decision-rang"))
        forme = porteur.get_attribute("data-decision-forme") or "simple"
        m["questions"] += 1

        passer = pg.query_selector(".choix--passer")
        passer = passer if (passer and passer.is_visible()) else None
        definitif = bouton_definitif(pg)

        if definitif is not None and passer is None:
            m["definitif_sans_passer"] += 1
        if definitif is not None:
            m["definitif_offert"] += 1
            # Le bouton définitif ne doit pas être une option du moteur.
            if definitif.get_attribute("data-choix") is not None:
                m["definitif_avec_data_choix"] += 1
        if passer is not None:
            m["passer_offert"] += 1
            combien = len(pg.query_selector_all(".choix--passer"))
            if combien != 1:
                m["passer_ambigu"] += 1

        if avec_le_bouton and definitif is not None:
            definitif.click()
        elif passer is not None:
            passer.click()
        elif forme == "montant":
            champ = pg.wait_for_selector("[data-montant]", timeout=20000)
            mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
            champ.fill(str(mini + (rang % (maxi - mini + 1))))
            pg.click("[data-valider]")
        elif forme == "multiple":
            brut = porteur.get_attribute("data-a-choisir")
            visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            k = int(brut) if (brut or "").isdigit() else (rang % max(len(visibles), 1)) + 1
            for c in visibles[:min(k, len(visibles))]:
                c.click()
            pg.click("[data-valider]")
        else:
            visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            if not visibles:
                echec(f"decision {rang} : aucun choix visible")
            visibles[choix_simple(rang, len(visibles))].click()

        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=20000)
    else:
        echec("la partie ne se termine pas")

    scores = []
    for j in (0, 1):
        e = pg.query_selector(f'[data-score-final="{j}"]')
        if e is None:
            echec(f"pas de score final pour le joueur {j}")
        scores.append(int("".join(c for c in e.inner_text() if c.isdigit() or c == "-")))
    return scores


def mesures():
    return {"questions": 0, "passer_offert": 0, "definitif_offert": 0,
            "definitif_sans_passer": 0, "definitif_avec_data_choix": 0,
            "passer_ambigu": 0}


fautes = []
SUFFIXE = f"?graine={GRAINE}&siege=0&animations=non"

with serveur(RACINE) as base:
    mA = mesures()
    with page(f"{base}/{SUFFIXE}") as (pg, err, _):
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        scores_main = jouer(pg, False, mA)
        if err:
            fautes.append(f"{len(err)} erreur(s) de console (a la main) : {err[0]}")

    mB = mesures()
    with page(f"{base}/{SUFFIXE}") as (pg, err, _):
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        scores_bouton = jouer(pg, True, mB)
        if err:
            fautes.append(f"{len(err)} erreur(s) de console (au bouton) : {err[0]}")

print(f"    a la main         : {mA['questions']} questions posees, "
      f"« passer » offert {mA['passer_offert']} fois, scores {scores_main}")
print(f"    au bouton definitif : {mB['questions']} questions posees, "
      f"bouton offert {mB['definitif_offert']} fois, scores {scores_bouton}")

if mB["definitif_offert"] == 0:
    fautes.append("le bouton « passer definitivement » n'apparait jamais")
if mB["definitif_sans_passer"] or mA["definitif_sans_passer"]:
    fautes.append("le bouton definitif est offert la ou passer une fois ne l'est pas")
if mB["definitif_avec_data_choix"] or mA["definitif_avec_data_choix"]:
    fautes.append("le bouton definitif porte data-choix : il se fait prendre pour "
                  "une option du moteur, et change toutes les parties de reference")
if mB["passer_ambigu"] or mA["passer_ambigu"]:
    fautes.append("plusieurs elements portent .choix--passer : les bancs ne savent "
                  "plus lequel est le bouton « passer » ordinaire")
if mB["questions"] >= mA["questions"]:
    fautes.append(f"le bouton definitif ne fait gagner aucune question "
                  f"({mB['questions']} contre {mA['questions']}) : il ne passe "
                  f"qu'une seule fois, c'est le bouton d'avant sous un autre nom")
if scores_main != scores_bouton:
    fautes.append(f"les scores divergent : {scores_main} a la main, "
                  f"{scores_bouton} au bouton — le bouton a repondu a autre chose")

if fautes:
    for f in fautes[:6]:
        print("      " + f)
    echec(f"{len(fautes)} defaut(s) sur le passage definitif")
print(f"OK le bouton epargne {mA['questions'] - mB['questions']} questions sur "
      f"{mA['questions']}, sans changer le score ni se faire passer pour une option")

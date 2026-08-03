#!/usr/bin/env python3
"""BANC D'ESSAI DE LA TABLE VIVANTE — ce que les contrôles du contrat ne voient pas.

Les contrôles livrés vérifient que le geste marche et que les cartes se posent.
Quatre choses les dépassent, et ce sont justement les quatre qui font mal :

  1. LA FUITE. La carte Phase de la manche EN COURS de l'adversaire ne doit
     apparaître NULLE PART tant que le moteur ne l'a pas révélée — ni comme carte
     posée, ni dans le document. On regarde à chaque `pick_phase`, aux deux
     sièges. Le point a coûté deux corrections le 02-08.

  2. LA PHASE PRÉCÉDENTE EST LA BONNE. Un contrôle qui vérifie qu'une carte est
     couchée ne dit pas qu'elle porte la bonne valeur. L'oracle est INDÉPENDANT
     du code mesuré : on relève ce que l'écran a révélé à la fin de la manche
     N-1, et on exige que ce soit exactement ce qu'il couche à la manche N.

  3. `?animations=non` NE CHANGE QUE LA DURÉE. Même graine, même suite de
     réponses, animations allumées puis éteintes : mêmes décisions, mêmes scores.
     Sans quoi le réglage ne serait pas un réglage d'affichage.

  4. UNE CARTE NON JOUABLE NE SE JOUE PAS. On glisse sur la table une carte que
     le moteur n'a pas énumérée : rien ne doit se passer.

Usage : python3 web/webapp/verif/table-vivante.py [racine] [contrôle]
        racine   : le dossier servi (défaut : le `web/webapp` qui contient ce
                   fichier)
        contrôle : `phases`, `animations`, `jouable`, ou `tout` (défaut)

Chaque contrôle se rejoue SEUL, avec sa graine écrite en clair ci-dessous, pour
qu'on puisse reproduire une seule chose sans attendre les autres.
"""
import os
import sys

# Les graines sont fixées ici, et nulle part ailleurs : un contrôle qu'on ne peut
# pas rejouer à l'identique n'est pas un contrôle.
GRAINE_PHASES = 909
GRAINE_ANIMATIONS = 1234
GRAINE_JOUABLE = 4242

RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1
                         else os.path.join(os.path.dirname(__file__), ".."))
LEQUEL = sys.argv[2] if len(sys.argv) > 2 else "tout"

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import serveur, page as ouvrir  # noqa: E402  (voisin de ce fichier)

fautes = []


def faute(m):
    fautes.append(m)
    print("  KO " + m)


# --------------------------------------------------------------- le pilotage

def choix_simple(rang, nb):
    return (rang * 7919 + 13) % nb


def choix_montant(rang, mini, maxi):
    return mini + (rang % (maxi - mini + 1))


def nombre(pg, chemin):
    e = pg.query_selector(f'[data-valeur="{chemin}"]')
    if e is None:
        return None
    t = "".join(c for c in e.inner_text() if c.isdigit())
    return int(t) if t else None


def repondre(pg, rang, forme, porteur, glisser=False):
    """Répond à la décision affichée. Rend l'indice choisi, pour le rejeu."""
    choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if forme == "montant":
        champ = pg.wait_for_selector("[data-montant]", timeout=15000)
        v = choix_montant(rang, int(champ.get_attribute("min")),
                          int(champ.get_attribute("max")))
        champ.fill(str(v))
        pg.click("[data-valider]")
        return ("montant", v)
    if forme == "multiple":
        brut = porteur.get_attribute("data-a-choisir")
        k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
        k = min(k, len(choix))
        for c in choix[:k]:
            c.click()
        pg.click("[data-valider]")
        return ("multiple", k)
    i = choix_simple(rang, len(choix))
    el = choix[i]
    if glisser and el.evaluate("e => !!e.closest('[data-main-siege]')"):
        glisser_vers_la_table(pg, el)
    else:
        el.click()
    return ("simple", el.get_attribute("data-choix"))


def glisser_vers_la_table(pg, el):
    cible = pg.query_selector("[data-table-siege]")
    b, c = el.bounding_box(), cible.bounding_box()
    x0, y0 = b["x"] + b["width"] / 2, b["y"] + b["height"] / 2
    x1, y1 = c["x"] + c["width"] / 2, c["y"] + c["height"] / 2
    pg.mouse.move(x0, y0)
    pg.mouse.down()
    for i in range(1, 11):
        pg.mouse.move(x0 + (x1 - x0) * i / 10, y0 + (y1 - y0) * i / 10)
    pg.mouse.up()


def partie(pg, avant=None, glisser=False, maximum=2000, delai=40000):
    """Joue jusqu'au bout. `avant(pg, rang, type)` voit l'écran de la question."""
    reponses = []
    for _ in range(maximum):
        if pg.query_selector("[data-partie-terminee]"):
            break
        pg.wait_for_selector("[data-decision-rang]", timeout=delai, state="attached")
        p = pg.query_selector("[data-decision-rang]")
        rang = int(p.get_attribute("data-decision-rang"))
        typ = p.get_attribute("data-decision-type") or ""
        if avant:
            avant(pg, rang, typ)
        reponses.append(
            repondre(pg, rang, p.get_attribute("data-decision-forme") or "simple",
                     p, glisser))
        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=delai)
    scores = []
    for j in (0, 1):
        e = pg.query_selector(f'[data-score-final="{j}"]')
        t = "".join(c for c in (e.inner_text() if e else "") if c.isdigit() or c == "-")
        scores.append(int(t) if t else None)
    return reponses, scores


# ------------------------------------------------- 1 & 2 : la fuite et l'oracle

def phases_posees(pg):
    """Ce que l'écran montre : (courantes, precedentes), joueur -> phase."""
    cour, prec = {}, {}
    for e in pg.query_selector_all("[data-phase-posee]"):
        v = e.get_attribute("data-phase-posee") or ""
        if ":" not in v:
            continue
        j, ph = v.split(":")[:2]
        cible = prec if (e.get_attribute("data-phase-precedente") or "") == "oui" else cour
        cible[int(j)] = int(ph)
    return cour, prec


def controle_phases(base, ouvrir_page, siege):
    """
    L'ORACLE, ET POURQUOI IL NE SE FIE PAS AU NUMÉRO DE MANCHE.

    Premier oracle écrit : « la phase précédente de la manche N est celle que
    l'écran a révélée à la manche N-1 ». Il criait à l'erreur six fois sur
    quatre-vingt-dix — et il avait tort. Relevé graine 909 : le compteur
    `generation` change ENTRE la planification et la résolution des phases
    choisies. Les décisions numérotées « manche 10 » résolvent donc les cartes
    choisies à la planification étiquetée « manche 9 ». Indexer sur la manche
    compare des choses décalées d'un cran.

    L'oracle ci-dessous ne compte donc plus les manches : il suit la SUITE des
    planifications. Ce que l'écran révèle après la planification k doit être
    exactement ce qu'il couche de côté à la planification k+1. Quand une manche se
    résout entièrement pendant le tour de l'adversaire, le siège regardé ne voit
    jamais la révélation — la comparaison est alors sautée plutôt que devinée.
    """
    print(f"— fuite et phase précédente, siège {siege}")
    vu = {"planifications": 0, "verifs": 0, "sautees": 0}
    # `choisies` : ce que l'écran a révélé après la planification précédente.
    # `None` = on ne l'a pas vu passer, on ne compare rien.
    etat = {"planifie": False, "choisies": None, "attend": False}

    def regarder(pg, rang, typ):
        manche = nombre(pg, "generation") or 0
        cour, prec = phases_posees(pg)

        if typ == "pick_phase":
            # 1. AUCUNE CARTE COURANTE POUR L'ADVERSAIRE tant que ça planifie.
            if (1 - siege) in cour:
                faute(f"siège {siege}, manche {manche} : la carte Phase EN COURS de "
                      f"l'adversaire est posée pendant la planification "
                      f"(phase {cour[1 - siege]})")
            # ... et rien non plus dans la barre d'équipage.
            n = nombre(pg, f"players.{1 - siege}.chosen_phase")
            if n:
                faute(f"siège {siege}, manche {manche} : la barre annonce la phase "
                      f"{n} de l'adversaire pendant la planification")

            if etat["planifie"]:
                return  # même planification, déjà comptée
            etat["planifie"] = True
            vu["planifications"] += 1

            # 2. LA PHASE PRÉCÉDENTE EST CELLE QUI VIENT D'ÊTRE JOUÉE, pour les
            #    DEUX joueurs, et elle est là AVANT que quiconque ne choisisse.
            if etat["choisies"] is None:
                vu["sautees"] += 1
            else:
                for j in (0, 1):
                    vu["verifs"] += 1
                    if prec.get(j) != etat["choisies"].get(j):
                        faute(f"siège {siege}, manche {manche} : phase précédente du "
                              f"joueur {j} = {prec.get(j)}, alors que la "
                              f"planification d'avant avait révélé "
                              f"{etat['choisies'].get(j)}")
            etat["choisies"] = None
            etat["attend"] = True
            return

        etat["planifie"] = False
        if etat["attend"] and len(cour) == 2:
            etat["choisies"] = dict(cour)
            etat["attend"] = False

    adresse = f"{base}/?graine={GRAINE_PHASES}&siege={siege}&animations=non"
    with ouvrir_page(adresse) as (pg, err, _):
        partie(pg, avant=regarder)
        if err:
            faute(f"siège {siege} : {len(err)} erreur(s) de console : {err[0]}")
    print(f"  {vu['planifications']} planifications, {vu['verifs']} comparaisons, "
          f"{vu['sautees']} sautée(s) faute d'avoir vu la révélation")
    if vu["verifs"] < 10:
        faute(f"siège {siege} : seulement {vu['verifs']} comparaisons — l'oracle "
              "n'a presque rien vérifié")


# --------------------------------------- 3 : le réglage ne change que la durée

def controle_animations(base, ouvrir_page):
    print("— ?animations=non ne change que la durée")
    with ouvrir_page(f"{base}/?graine={GRAINE_ANIMATIONS}&siege=0&animations=non") as (pg, e1, _):
        sans, s_sans = partie(pg, glisser=True)
    with ouvrir_page(f"{base}/?graine={GRAINE_ANIMATIONS}&siege=0") as (pg, e2, _):
        avec, s_avec = partie(pg, glisser=True, delai=60000)
    print(f"  sans : {len(sans)} décisions, scores {s_sans}")
    print(f"  avec : {len(avec)} décisions, scores {s_avec}")
    if e1 or e2:
        faute(f"{len(e1) + len(e2)} erreur(s) de console")
    if sans != avec:
        for i, (a, b) in enumerate(zip(sans, avec)):
            if a != b:
                faute(f"décision {i} : {a} sans animation contre {b} avec")
                break
        faute("les animations changent la partie, pas seulement sa durée")
    if s_sans != s_avec:
        faute(f"scores {s_sans} sans animation contre {s_avec} avec")


# ------------------------------------- 4 : une carte non jouable ne se joue pas

def controle_non_jouable(base, ouvrir_page):
    print("— une carte non jouable ne part pas sur la table")
    vu = {"essais": 0}

    def regarder(pg, rang, typ):
        if vu["essais"] >= 3 or typ != "choose_build":
            return
        muettes = [c for c in pg.query_selector_all(
            '[data-main-siege] [data-carte-id][data-jouable="non"]') if c.is_visible()]
        if not muettes:
            return
        vu["essais"] += 1
        glisser_vers_la_table(pg, muettes[0])
        pg.wait_for_timeout(250)
        p = pg.query_selector("[data-decision-rang]")
        if p is None or int(p.get_attribute("data-decision-rang")) != rang:
            faute(f"décision {rang} : une carte NON jouable a répondu au moteur")

    with ouvrir_page(f"{base}/?graine={GRAINE_JOUABLE}&siege=0&animations=non") as (pg, err, _):
        partie(pg, avant=regarder)
        if err:
            faute(f"{len(err)} erreur(s) de console : {err[0]}")
    print(f"  {vu['essais']} carte(s) non jouable(s) refusée(s)")
    if vu["essais"] == 0:
        faute("aucune carte non jouable rencontrée : le contrôle n'a rien éprouvé")


# ----------------------------------------------------------------------- main

def main():
    if LEQUEL not in ("tout", "phases", "animations", "jouable"):
        print(f"KO contrôle inconnu : {LEQUEL}")
        return 2
    with serveur(RACINE) as base:
        if LEQUEL in ("tout", "phases"):
            for siege in (0, 1):
                controle_phases(base, ouvrir, siege)
        if LEQUEL in ("tout", "animations"):
            controle_animations(base, ouvrir)
        if LEQUEL in ("tout", "jouable"):
            controle_non_jouable(base, ouvrir)
    if fautes:
        print(f"\nKO {len(fautes)} faute(s)")
        return 1
    print("\nOK la table vivante ne fuit rien, dit vrai, et ne se joue qu'aux "
          "cartes que le moteur propose")
    return 0


if __name__ == "__main__":
    sys.exit(main())

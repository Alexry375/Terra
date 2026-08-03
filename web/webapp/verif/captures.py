#!/usr/bin/env python3
"""Les captures d'ecran de la livraison, refaites a la demande.

Elles servent a JUGER A L'OEIL ce qu'aucun controle ne mesure : est-ce que les
deux arcs ressemblent au plateau imprime, est-ce que la planche des oceans
ressemble a un nid d'abeille, est-ce que le bandeau se lit.

La partie est jouee avec les choix deterministes du pilote (`?animations=non`,
graine 5150), donc deux passages rendent la meme image.

Depuis la racine du workspace :

    python3 outputs/web/webapp/verif/captures.py [dossier]

Par defaut : `outputs/captures/`.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, choix_simple, choix_montant  # noqa: E402

SORTIE = sys.argv[1] if len(sys.argv) > 1 else "outputs/captures"
GRAINE = "5150"
# Assez avance pour que les deux pistes soient a moitie remplies et que la
# planche montre des tuiles des deux cotes : retournees et revelees.
DECISIONS = 45

ZONES = {
    "arc-temperature": '[data-arc="temperature"]',
    "arc-oxygen": '[data-arc="oxygen"]',
    "planche-oceans": "[data-oceans]",
    "bandeau": "#horizon",
    "barre-joueur": "#equipage-0",
}


def avance(pg, combien):
    """Les memes choix que `pilote.jouer`, mais bornes a `combien` decisions."""
    for _ in range(combien):
        if pg.query_selector("[data-partie-terminee]"):
            return
        pg.wait_for_selector("[data-decision-rang]", timeout=15000, state="attached")
        p = pg.query_selector("[data-decision-rang]")
        rang = int(p.get_attribute("data-decision-rang"))
        forme = p.get_attribute("data-decision-forme") or "simple"
        visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
        if forme == "montant":
            champ = pg.wait_for_selector("[data-montant]", timeout=15000)
            champ.fill(str(choix_montant(rang, int(champ.get_attribute("min")),
                                         int(champ.get_attribute("max")))))
            pg.click("[data-valider]")
        elif forme == "multiple":
            brut = p.get_attribute("data-a-choisir")
            k = int(brut) if (brut or "").isdigit() else (rang % max(len(visibles), 1)) + 1
            for c in visibles[:min(k, len(visibles))]:
                c.click()
            pg.click("[data-valider]")
        else:
            visibles[choix_simple(rang, len(visibles))].click()
        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=15000)


os.makedirs(SORTIE, exist_ok=True)
with serveur("outputs/web/webapp") as base:
    for (L, H) in [(1600, 1000), (1280, 640)]:
        with page(f"{base}/?graine={GRAINE}&siege=0&animations=non",
                  largeur=L, hauteur=H) as (pg, err, _):
            pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            avance(pg, DECISIONS)
            pg.wait_for_timeout(500)
            pg.screenshot(path=f"{SORTIE}/ecran-{L}x{H}.png")
            print(f"ecran-{L}x{H}.png")
            if (L, H) != (1600, 1000):
                continue
            for nom, sel in ZONES.items():
                e = pg.query_selector(sel)
                if e:
                    e.screenshot(path=f"{SORTIE}/{nom}.png")
                    print(f"{nom}.png")
            etat = pg.evaluate("""() => ({
              temperature: document.querySelector('[data-arc-lecture="temperature"]').textContent,
              oxygen: document.querySelector('[data-arc-lecture="oxygen"]').textContent,
              pas: [document.querySelector('[data-valeur="planet.temperature"]').textContent,
                    document.querySelector('[data-valeur="planet.oxygen"]').textContent],
              revelees: document.querySelectorAll('[data-ocean-revelee="oui"]').length,
            })""")
            print("etat capture :", etat)
            if err:
                print("KO erreurs de console :", err[:2])
                sys.exit(1)
print(f"OK captures dans {SORTIE}/")

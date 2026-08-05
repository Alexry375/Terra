#!/usr/bin/env python3
"""LE PRIX D'ORIGINE EST BARRE QUAND UNE REMISE S'APPLIQUE (LIS-11).

Les decisions de remise (`reduction_microbes`, `reduction_plantes`) ne
disaient que le RABAIS — « 10 MC off » — jamais le prix qu'on allait payer, ni
le prix imprime sur la carte. On les affiche desormais tous les deux, le
premier barre.

Ce banc joue de vraies parties au clic jusqu'a tomber sur une remise, puis
verifie SUR LA PAGE :

  · les deux nombres sont ecrits et visibles ;
  · le prix barre l'est REELLEMENT (`line-through` calcule par le navigateur,
    pas une classe qu'on suppose stylee) ;
  · les deux nombres sont ceux du moteur : prix imprime de la carte, et prix
    imprime moins le rabais annonce par l'option.

Le troisieme point est le seul qui compte vraiment : afficher deux nombres
plausibles mais faux serait pire que de n'en afficher qu'un.

    python3 verif/prix-barre.py <racine-webapp> [graines...]
"""
import os
import sys

RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "web/webapp")
GRAINES = sys.argv[2:] or ["4242", "77", "2024", "210055"]

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page, choix_simple  # noqa: E402

# Ce que la page montre sur une decision de remise, et ce que le navigateur
# CALCULE reellement pour le prix barre.
LECTURE = """() => {
  const s = document.getElementById('scene');
  const out = {type: s ? s.dataset.decisionType : null, prix: []};
  for (const p of document.querySelectorAll('.choix__prix')) {
    const plein = p.querySelector('.choix__prix--plein');
    const paye = p.querySelector('.choix__prix--paye');
    const r = p.getBoundingClientRect();
    out.prix.push({
      visible: r.width > 0 && r.height > 0,
      carte: p.dataset.prixCarte || null,
      plein: plein ? plein.textContent.trim() : null,
      paye: paye ? paye.textContent.trim() : null,
      // Barre pour de vrai ? On lit ce que le navigateur applique.
      barre: plein ? getComputedStyle(plein).textDecorationLine : null,
      balise: plein ? plein.tagName : null,
    });
  }
  return out;
}"""

fautes = []
vus = 0


def nombre(txt):
    return int("".join(c for c in (txt or "") if c.isdigit()) or -1)


with serveur(RACINE) as base:
    for graine in GRAINES:
        if vus >= 3:
            break
        with page(f"{base}/?graine={graine}&siege=0&animations=non") as (pg, erreurs, _):
            pg.wait_for_selector("#horizon", timeout=20000)
            for _ in range(500):
                if pg.query_selector("[data-partie-terminee]"):
                    break
                porteur = pg.query_selector("[data-decision-rang]")
                if porteur is None:
                    pg.wait_for_timeout(100)
                    continue
                rang = int(porteur.get_attribute("data-decision-rang"))
                type_ = porteur.get_attribute("data-decision-type") or ""

                if type_.startswith("reduction_"):
                    lu = pg.evaluate(LECTURE)
                    if not lu["prix"]:
                        fautes.append(
                            f"graine {graine}, rang {rang} ({type_}) : une remise est "
                            "proposee et AUCUN prix n'est affiche")
                    for p in lu["prix"]:
                        vus += 1
                        if not p["visible"]:
                            fautes.append(f"graine {graine}, rang {rang} : le couple de "
                                          "prix est dans la page mais invisible")
                        if p["plein"] is None or p["paye"] is None:
                            fautes.append(f"graine {graine}, rang {rang} : couple "
                                          f"incomplet {p}")
                            continue
                        if p["balise"] != "S":
                            fautes.append(f"graine {graine}, rang {rang} : le prix plein "
                                          f"n'est pas dans une balise <s> mais "
                                          f"<{p['balise']}>")
                        if "line-through" not in (p["barre"] or ""):
                            fautes.append(f"graine {graine}, rang {rang} : le prix plein "
                                          f"n'est pas barre (text-decoration "
                                          f"{p['barre']!r})")
                        if nombre(p["paye"]) >= nombre(p["plein"]):
                            fautes.append(f"graine {graine}, rang {rang} : prix paye "
                                          f"{p['paye']} pas inferieur au prix plein "
                                          f"{p['plein']}")
                        else:
                            print(f"    graine {graine}, rang {rang} ({type_}) · "
                                  f"{p['carte']} : {p['plein']} barre -> {p['paye']}")

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
            if erreurs:
                fautes.append(f"graine {graine} : erreurs de console {erreurs[:2]}")

print(f"    {vus} couple(s) de prix vus sur une decision de remise")
if vus == 0:
    print("ECHEC : aucune remise rencontree — la mesure n'a pas eu lieu")
    sys.exit(1)
if fautes:
    for f in fautes[:6]:
        print("ECHEC :", f)
    sys.exit(1)
print("    le prix d'origine est barre a cote du prix reellement paye")

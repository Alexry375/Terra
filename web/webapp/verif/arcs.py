#!/usr/bin/env python3
"""CE QUE LE CONTROLE 02 NE VOIT PAS : les arcs disent-ils la bonne valeur ?

Le controle livre exige que les crans dessinent un arc, qu'ils couvrent les
valeurs du plateau, et que le repere se soit deplace au moins trois fois. Il ne
verifie JAMAIS que le repere est au bon endroit : un curseur qui bougerait au
hasard, ou qui suivrait l'oxygene sur l'arc de temperature, le passerait.

Ce banc-ci mesure, a chaque decision d'une partie entiere :

  1. LA GRADUATION — les vingt valeurs de temperature sont exactement
     -30, -28, … +8, et les quatorze d'oxygene exactement 1 … 14 (livret
     l. 84 et l. 83) ;
  2. LA LECTURE EN CHIFFRES — pour un compteur du moteur a `n` pas, elle vaut
     la `n`-ieme case de la piste imprimee. La correspondance n'est PAS
     recalculee par une formule : elle est lue dans la table `PISTE_TEMPERATURE`
     ci-dessous, transcrite case par case du livret (l. 84 pour la piste,
     l. 499 pour le pas de 2 °C, l. 201 pour la case de depart). Le compteur,
     lui, est lu sur le BANDEAU, c'est-a-dire par un autre element du document
     que l'arc ;
  3. LES CASES ACQUISES — ce sont exactement celles dont la valeur imprimee est
     atteinte, ni une de plus (une case allumee d'avance annoncerait un palier
     que le joueur n'a pas franchi), ni une de moins ;
  4. LE REPERE — le cran le plus proche du repere est celui de la valeur
     courante. C'est le point que le controle livre laisse passer.

CE QUE CE BANC NE PROUVE PAS, dit franchement. La page place le repere a la
fraction `pas / max` et le cran `i` a la fraction `i / max` : que le repere
tombe sur le bon cran tient donc en partie a la construction, et ce banc ne
peut pas s'en porter garant tout seul. Il attrape neanmoins ce que la
construction ne garantit pas : une inversion des deux arcs, un decalage d'une
case entre les deux fractions, une graduation qui ne serait pas celle du
livret, et une mesure prise pendant que le repere glisse encore (c'est ce
dernier cas qui a impose `?animations=non` ici). L'oracle vraiment exterieur
du banc, ce sont les deux tables de pistes ci-dessous et le compteur du moteur
lu sur le bandeau.

Puis, a six tailles de fenetre : les crans des deux arcs restent DANS leur
panneau, et les deux panneaux ne mordent ni sur le bandeau ni sur la planche
des oceans.

Depuis la racine du workspace :

    python3 outputs/web/webapp/verif/arcs.py [graine]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer, echec  # noqa: E402

# LES DEUX PISTES DU PLATEAU IMPRIME, transcrites case par case du livret
# (l. 84 et l. 83) et non recalculees : c'est l'oracle de ce banc, et il doit
# rester lisible par quelqu'un qui a le carton sous les yeux.
#
# `PISTE_TEMPERATURE[n]` est la case atteinte quand le compteur du moteur vaut
# `n` pas ; la case 0 est celle du cube de depart, -30 °C (livret l. 201).
PISTE_TEMPERATURE = [
    -30, -28, -26, -24, -22, -20, -18, -16, -14, -12,
    -10, -8, -6, -4, -2, 0, 2, 4, 6, 8,
]
# L'oxygene part de 0 % (livret l. 201), case qui n'est pas a gagner : les
# quatorze cases de la piste sont 1 % … 14 %, et le pas `n` du moteur atteint
# la case `n` pour n >= 1.
PISTE_OXYGENE = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]

TEMP = PISTE_TEMPERATURE
O2 = PISTE_OXYGENE
TAILLES = [(1920, 1080), (1600, 1000), (1440, 900), (1366, 768), (1280, 720), (1280, 640)]

LECTURE = """() => {
  const lis = (q) => {
    const out = [];
    for (const e of document.querySelectorAll(`[data-cran="${q}"]`)) {
      const r = e.getBoundingClientRect();
      out.push({v: Number(e.getAttribute('data-cran-valeur')),
                acquis: e.classList.contains('cran--acquis'),
                x: r.x + r.width / 2, y: r.y + r.height / 2});
    }
    return out;
  };
  const rep = (q) => {
    const e = document.querySelector(`[data-repere="${q}"]`);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    return {x: r.x + r.width / 2, y: r.y + r.height / 2};
  };
  const nb = (s) => {
    const e = document.querySelector(s);
    return e ? Number(e.textContent.replace(/[^-0-9]/g, '')) : null;
  };
  return {
    crans: {temperature: lis('temperature'), oxygen: lis('oxygen')},
    repere: {temperature: rep('temperature'), oxygen: rep('oxygen')},
    // LIS-1 (05-08) — l'arc n'écrit plus son nombre : la valeur du plateau se
    // lit maintenant dans la barre du haut, seule à la donner.
    lecture: {temperature: nb('#param-temp [data-valeur="planet.temperature"]'),
              oxygen: nb('#param-o2 [data-valeur="planet.oxygen"]')},
    moteur: {temperature: nb('[data-valeur="planet.temperature"]'),
             oxygen: nb('[data-valeur="planet.oxygen"]')},
  };
}"""

GEOMETRIE = """() => {
  const boite = (s) => {
    const e = document.querySelector(s);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    return {x: r.x, y: r.y, w: r.width, h: r.height};
  };
  const out = {horizon: boite('#horizon'), oceans: boite('[data-oceans]')};
  for (const q of ['temperature', 'oxygen']) {
    out[q] = boite(`[data-arc="${q}"]`);
    const cr = [...document.querySelectorAll(`[data-cran="${q}"]`)]
      .map((e) => e.getBoundingClientRect());
    out[q + '_crans'] = {
      x0: Math.min(...cr.map((r) => r.x)), x1: Math.max(...cr.map((r) => r.x + r.width)),
      y0: Math.min(...cr.map((r) => r.y)), y1: Math.max(...cr.map((r) => r.y + r.height)),
    };
  }
  return out;
}"""

fautes = []
vu = {"mesures": 0, "reperes": 0, "places": {"temperature": set(), "oxygen": set()}}


def valeur_du_plateau(quoi, pas):
    """La case du plateau imprime qu'un compteur de `pas` pas designe.

    LUE dans la table transcrite du livret, jamais recalculee : si la page se
    trompait de formule, ce banc le verrait ; s'il refaisait la formule, non.
    """
    if quoi == "temperature":
        return PISTE_TEMPERATURE[pas]
    return PISTE_OXYGENE[pas - 1] if pas >= 1 else 0


def controle(pg, rang):
    d = pg.evaluate(LECTURE)
    vu["mesures"] += 1
    for quoi, attendu in (("temperature", TEMP), ("oxygen", O2)):
        crans = d["crans"][quoi]
        if [c["v"] for c in crans] != attendu:
            fautes.append(f"decision {rang} : graduation {quoi} = "
                          f"{[c['v'] for c in crans]}")
            continue
        pas = d["moteur"][quoi]
        if pas is None:
            fautes.append(f"decision {rang} : {quoi} absent du bandeau")
            continue
        valeur = valeur_du_plateau(quoi, pas)
        if d["lecture"][quoi] != valeur:
            fautes.append(f"decision {rang} : l'arc {quoi} lit "
                          f"{d['lecture'][quoi]}, le moteur est a {pas} pas "
                          f"soit {valeur} sur le plateau")
        acquis = {c["v"] for c in crans if c["acquis"]}
        devrait = {v for v in attendu if v <= valeur}
        if acquis != devrait:
            fautes.append(f"decision {rang} : cases {quoi} allumees {sorted(acquis)}, "
                          f"attendu {sorted(devrait)}")
        r = d["repere"][quoi]
        if r is None:
            fautes.append(f"decision {rang} : pas de repere {quoi}")
            continue
        vu["places"][quoi].add((round(r["x"]), round(r["y"])))
        if not devrait:
            continue  # oxygene a 0 % : le repere est avant la premiere case
        vu["reperes"] += 1
        proche = min(crans, key=lambda c: (c["x"] - r["x"]) ** 2 + (c["y"] - r["y"]) ** 2)
        if proche["v"] != valeur:
            fautes.append(f"decision {rang} : le repere {quoi} est sur la case "
                          f"{proche['v']}, la valeur est {valeur}")


graine = sys.argv[1] if len(sys.argv) > 1 else "4242"
# `animations=non` : le repere GLISSE d'une case a l'autre en 620 ms. Sans ce
# reglage, une mesure prise juste apres le changement d'etat trouve le repere
# entre deux cases et accuse a tort la page. Le reglage ne change QUE la duree —
# la case visee, elle, est la meme (c'est ce que ce banc verifie, 320 fois).
with serveur("outputs/web/webapp") as base:
    with page(f"{base}/?graine={graine}&siege=0&animations=non") as (pg, erreurs, _):
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        controle(pg, 0)
        jouer(pg, apres=controle)
        if erreurs:
            echec(f"{len(erreurs)} erreur(s) de console : {erreurs[0]}")

    # La geometrie, a six tailles de fenetre.
    for (L, H) in TAILLES:
        with page(f"{base}/?graine={graine}&siege=0&animations=non", largeur=L, hauteur=H) as (pg, err, _):
            pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            pg.wait_for_timeout(300)
            g = pg.evaluate(GEOMETRIE)
            for quoi in ("temperature", "oxygen"):
                b, c = g[quoi], g[quoi + "_crans"]
                if b is None:
                    fautes.append(f"{L}x{H} : arc {quoi} introuvable")
                    continue
                if (c["x0"] < b["x"] - 1 or c["x1"] > b["x"] + b["w"] + 1
                        or c["y0"] < b["y"] - 1 or c["y1"] > b["y"] + b["h"] + 1):
                    fautes.append(f"{L}x{H} : les crans de {quoi} sortent de leur panneau")
                for autre in ("horizon", "oceans"):
                    a = g[autre]
                    if a and (b["x"] < a["x"] + a["w"] - 1 and a["x"] < b["x"] + b["w"] - 1
                              and b["y"] < a["y"] + a["h"] - 1 and a["y"] < b["y"] + b["h"] - 1):
                        fautes.append(f"{L}x{H} : l'arc {quoi} recouvre {autre}")
            if err:
                echec(f"{L}x{H} : {len(err)} erreur(s) de console : {err[0]}")

print(f"{vu['mesures']} lectures, {vu['reperes']} positions de repere verifiees ; "
      f"repere vu a {len(vu['places']['temperature'])} et "
      f"{len(vu['places']['oxygen'])} places")
if vu["reperes"] < 100:
    echec(f"seulement {vu['reperes']} position(s) de repere verifiee(s)")
if fautes:
    for f in fautes[:8]:
        print("  " + f)
    echec(f"{len(fautes)} defaut(s) sur les deux arcs")
print("OK les deux arcs sont gradues comme le plateau et le repere est sur la bonne case")

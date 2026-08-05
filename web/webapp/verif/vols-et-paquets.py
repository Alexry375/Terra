#!/usr/bin/env python3
"""ANI-6 — LA PIOCHE ET LA DÉFAUSSE ONT UN CHEMIN, ET IL SE VOIT.

Ce banc est le mien ; les trois contrôles du contrat sont un minimum, pas ma
couverture. Il mesure quatre choses que ceux-là ne mesurent pas :

  1. LE MOTIF. Tout nœud posé dans la couche `#vol` porte `data-vol`, et le motif
     est l'un des six du contrat. Un nœud sans marque, ou marqué d'un mot
     inconnu, est un défaut.
  2. L'APPARIEMENT. Une carte qui entre dans ma main doit s'accompagner d'un vol
     `pioche` ; une carte qui entre dans la défausse, d'un vol `defausse`.
     L'oracle est DISJOINT du code mesuré : il ne lit ni `vue/anim.js` ni
     `vue/defausse.js`, seulement deux choses que la page publiait déjà avant ce
     chantier — le nombre de cartes dessinées dans ma main, et le compteur
     `decks.discard` du bandeau, écrit par `vue/monde.js`.
  3. LE VOL EST RÉEL. Chaque fac-similé est mesuré à sa naissance puis 180 ms
     plus tard : il doit avoir une taille non nulle ET s'être déplacé. Un nœud
     invisible, de taille nulle ou immobile, posé pour qu'un compteur tombe
     juste, est exactement ce que la clause anti-shortcut interdit.
  4. LE SENS. La pioche arrive PAR LA DROITE : le dock des paquets est à droite
     de la main, et le voyage est long (plus de 200 points). La défausse s'en va
     en sens inverse — même segment, parcouru dans l'autre sens.

⚠️ IL COMPTE SES OCCASIONS AVANT DE JUGER. Sans pioches et sans défausses
observées, un vert ne dirait rien : il échoue plutôt que de rendre un verdict sur
le vide.

Usage : python3 vols-et-paquets.py [racine] [graine] [decisions]
"""
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICI, ".."))
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"
DECISIONS = int(sys.argv[3]) if len(sys.argv) > 3 else 220

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page as ouvrir, choix_simple, choix_montant  # noqa: E402

MOTIFS = {"pioche", "defausse", "pose", "jauge", "mc", "jeton"}

# L'espion est posé AVANT le premier script de la page : les vols de la mise en
# place seraient manqués sinon, et le banc compterait moins d'occasions qu'il n'y
# en a eu.
ESPION = """
window.__vols = [];
function mesurer(n) {
  const r = n.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2, l: r.width, h: r.height };
}
new MutationObserver((ms) => {
  for (const m of ms) {
    for (const n of m.addedNodes || []) {
      if (n.nodeType !== 1) continue;
      if (!n.parentElement || n.parentElement.id !== 'vol') continue;
      const v = { motif: n.dataset.vol === undefined ? null : n.dataset.vol,
                  debut: mesurer(n), fin: null };
      window.__vols.push(v);
      // 180 ms plus tard, le fac-similé doit avoir bougé. Les vols durent de 620
      // a 900 ms : il est encore dans le document a cet instant.
      setTimeout(() => { if (n.isConnected) v.fin = mesurer(n); }, 180);
    }
  }
}).observe(document, { childList: true, subtree: true });
"""

LECTURE = """
() => {
  const n = (s) => { const e = document.querySelector(s); return e ? Number(e.textContent.trim()) : null; };
  return {
    main: document.querySelectorAll('[data-main-siege] [data-carte-cle]').length,
    defausse: n('[data-valeur="decks.discard"]'),
    vols: window.__vols.length,
  };
}
"""

GEOMETRIE = """
() => {
  const b = (s) => { const e = document.querySelector(s); if (!e) return null;
    const r = e.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2, l: r.width, h: r.height }; };
  return { pioche: b('[data-pioche]'), defausse: b('[data-defausse]'),
           main: b('[data-main-siege]') };
}
"""


def repondre(pg, delai=15000):
    """Une décision, par le chemin du joueur. Rend le rang joué, ou None."""
    if pg.query_selector("[data-partie-terminee]"):
        return None
    pg.wait_for_selector("[data-decision-rang]", timeout=delai, state="attached")
    porteur = pg.query_selector("[data-decision-rang]")
    rang = int(porteur.get_attribute("data-decision-rang"))
    forme = porteur.get_attribute("data-decision-forme") or "simple"
    choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if forme == "montant":
        champ = pg.wait_for_selector("[data-montant]", timeout=delai)
        mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
        champ.fill(str(choix_montant(rang, mini, maxi)))
        pg.click("[data-valider]")
    elif forme == "multiple":
        brut = porteur.get_attribute("data-a-choisir")
        k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
        for c in choix[:min(k, len(choix))]:
            c.click()
        pg.click("[data-valider]")
    else:
        if not choix:
            raise RuntimeError(f"decision {rang} : aucun choix visible")
        choix[choix_simple(rang, len(choix))].click()
    pg.wait_for_function(
        "r => { const e = document.querySelector('[data-decision-rang]');"
        " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
        " || document.querySelector('[data-partie-terminee]'); }",
        arg=rang, timeout=delai)
    return rang


def main():
    fautes = []
    with serveur(RACINE) as url:
        page_url = f"{url}/?graine={GRAINE}&siege=0&boites=base,decouverte"
        with ouvrir(page_url) as (pg, erreurs, _):
            pg.add_init_script(ESPION)
            pg.goto(page_url, wait_until="domcontentloaded")
            pg.wait_for_selector("[data-decision-rang]", timeout=20000, state="attached")

            geo = pg.evaluate(GEOMETRIE)
            for nom in ("pioche", "defausse", "main"):
                if not geo[nom]:
                    print(f"ECHEC : la page ne porte aucun [data-{nom}] — "
                          f"le chemin des cartes n'existe pas (ANI-6)")
                    return 1

            avant = pg.evaluate(LECTURE)
            pioches = defausses = 0
            pioches_muettes = defausses_muettes = []
            pioches_muettes, defausses_muettes = [], []
            rangs = 0
            for _ in range(DECISIONS):
                rang = repondre(pg)
                if rang is None:
                    break
                # On laisse au vol le temps d'exister : ce banc mesure ce que le
                # joueur VOIT, pas la vitesse de la machine.
                pg.wait_for_timeout(260)
                apres = pg.evaluate(LECTURE)
                nouveaux = pg.evaluate(
                    "n => window.__vols.slice(n).map(v => v.motif)", avant["vols"])
                if apres["main"] > avant["main"]:
                    pioches += 1
                    if "pioche" not in nouveaux:
                        pioches_muettes.append(rang)
                if (apres["defausse"] is not None and avant["defausse"] is not None
                        and apres["defausse"] > avant["defausse"]):
                    defausses += 1
                    if "defausse" not in nouveaux:
                        defausses_muettes.append(rang)
                avant = apres
                rangs += 1

            vols = pg.evaluate("() => window.__vols")
            if erreurs:
                print(f"ECHEC : la page a leve {len(erreurs)} erreur(s) : {erreurs[0]}")
                return 1

    print(f"    {rangs} decision(s) jouees, graine {GRAINE} ; {len(vols)} vol(s) "
          f"dans la couche #vol")
    print(f"      pioches vues {pioches:4d}, {len(pioches_muettes):4d} sans vol")
    print(f"      defausses vues {defausses:4d}, {len(defausses_muettes):4d} sans vol")

    # 0. La mesure a-t-elle eu lieu ?
    if rangs < 40:
        print(f"ECHEC : {rangs} decision(s) seulement — la partie ne s'est pas jouee")
        return 1
    if pioches < 5 or defausses < 3:
        print(f"ECHEC : {pioches} pioche(s) et {defausses} defausse(s) observees — "
              f"trop peu pour juger, ce banc ne prouve rien en l'etat")
        return 1
    if len(vols) < 10:
        print(f"ECHEC : {len(vols)} vol(s) seulement — ce banc ne prouve rien")
        return 1

    # 1. Le motif.
    sans = [v for v in vols if v["motif"] is None]
    etranges = sorted({v["motif"] for v in vols
                       if v["motif"] is not None and v["motif"] not in MOTIFS})
    if sans:
        fautes.append(f"{len(sans)} noeud(s) de la couche #vol sans data-vol")
    if etranges:
        fautes.append(f"motif(s) hors contrat : {etranges}")

    # 2. L'appariement.
    if pioches_muettes:
        fautes.append(f"{len(pioches_muettes)} pioche(s) sans vol « pioche » "
                      f"(rangs {pioches_muettes[:5]})")
    if defausses_muettes:
        fautes.append(f"{len(defausses_muettes)} defausse(s) sans vol « defausse » "
                      f"(rangs {defausses_muettes[:5]})")

    # 3. Le vol est réel : une taille, et un déplacement.
    plats = [v for v in vols if v["debut"]["l"] < 10 or v["debut"]["h"] < 10]
    if plats:
        fautes.append(f"{len(plats)} fac-simile(s) de taille nulle ou presque — "
                      f"ce qui vole doit reellement traverser l'ecran")
    mesures = [v for v in vols if v["fin"]]
    immobiles = [v for v in mesures
                 if abs(v["fin"]["x"] - v["debut"]["x"]) < 6
                 and abs(v["fin"]["y"] - v["debut"]["y"]) < 6]
    if len(mesures) < len(vols) // 2:
        fautes.append(f"seuls {len(mesures)} vol(s) sur {len(vols)} ont pu etre "
                      f"remesures 180 ms plus tard — ils ne durent pas")
    elif immobiles:
        fautes.append(f"{len(immobiles)} vol(s) sur {len(mesures)} n'ont pas bouge "
                      f"de 180 ms — un fac-simile immobile n'est pas un voyage")

    # 4. Le sens : la pioche arrive par la droite, la defausse repart a l'inverse.
    if geo["pioche"]["x"] <= geo["main"]["x"]:
        fautes.append(f"la pioche (x={geo['pioche']['x']:.0f}) n'est pas a DROITE "
                      f"de la main (x={geo['main']['x']:.0f}) : « la pioche arrive "
                      f"par la droite » n'est pas tenu")
    if geo["defausse"]["x"] <= geo["main"]["x"]:
        fautes.append("la defausse n'est pas a droite de la main : la carte qui "
                      "part ne s'en va pas en sens inverse de celle qui arrive")
    trajet = abs(geo["pioche"]["x"] - geo["main"]["x"]) + abs(geo["pioche"]["y"] - geo["main"]["y"])
    if trajet < 200:
        fautes.append(f"le trajet pioche -> main ne fait que {trajet:.0f} points : "
                      f"ce n'est pas une traversee d'ecran")

    if fautes:
        print(f"ECHEC : {len(fautes)} defaut(s)")
        for f in fautes:
            print(f"      · {f}")
        return 1
    print("    tout ce qui entre en main ou part a la defausse se voit voyager, "
          "marque, et dans le bon sens")
    return 0


if __name__ == "__main__":
    sys.exit(main())

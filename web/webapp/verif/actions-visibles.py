#!/usr/bin/env python3
"""ANI-1 — CHAQUE ACTION DE LA LISTE SE VOIT, ET SE VOIT POUR CE QU'ELLE EST.

Le contrôle 01 du contrat exige qu'un évènement s'accompagne d'AU MOINS UN
mouvement, quel qu'il soit. C'est le minimum, et il laisse passer un défaut réel :
une carte piochée pourrait très bien être « accompagnée » par le jeton de forêt
de l'adversaire, et le joueur ne comprendrait toujours pas ce qui vient
d'arriver. Ce banc-ci exige que le mouvement DISE l'évènement — que la hausse de
température fasse voler une jauge, que la dépense fasse voler des mégacrédits,
que la pose fasse voler une carte.

ORACLE DISJOINT. Il ne lit aucune ligne de `vue/anim.js` ni de `vue/defausse.js`.
Il ne regarde que deux choses :

  · les valeurs que la page PUBLIE (`data-valeur="<chemin>"`), écrites par
    `vue/monde.js` et `vue/joueurs.js`, qui existaient avant ce chantier. Leurs
    variations disent quels évènements ont eu lieu ;
  · la marque `data-vol` des nœuds ajoutés à la couche `#vol`, qui est le contrat
    de forme.

⚠️ IL COMPTE SES OCCASIONS AVANT DE JUGER. Chaque famille déclare combien de fois
elle a été vue ; en dessous du seuil, le banc ÉCHOUE plutôt que de rendre un
verdict sur le vide.

Usage : python3 actions-visibles.py [racine] [graine]
"""
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICI, ".."))
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "909"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page as ouvrir, choix_simple, choix_montant  # noqa: E402

ESPION = """
window.__m = [];
new MutationObserver((ms) => {
  for (const m of ms) for (const n of m.addedNodes || []) {
    if (n.nodeType === 1 && n.parentElement && n.parentElement.id === 'vol')
      window.__m.push(n.dataset.vol === undefined ? '?' : n.dataset.vol);
  }
}).observe(document, { childList: true, subtree: true });
"""

LECTURE = """
() => {
  const n = (s) => { const e = document.querySelector(s); return e ? e.textContent.trim() : null; };
  const v = (c) => n('[data-valeur="' + c + '"]');
  return {
    temperature: v('planet.temperature'),
    oxygene: v('planet.oxygen'),
    mc: [v('players.0.mc'), v('players.1.mc')],
    forets: [v('players.0.forests'), v('players.1.forests')],
    main: document.querySelectorAll('[data-main-siege] [data-carte-cle]').length,
    jeu: document.querySelectorAll('[data-carte-en-jeu]').length,
    defausse: v('decks.discard'),
    vols: window.__m.length,
  };
}
"""


def nombre(x):
    try:
        return int(str(x).strip().replace("+", ""))
    except (TypeError, ValueError):
        return None


def monte(a, b, cle):
    x, y = nombre(a[cle]), nombre(b[cle])
    return x is not None and y is not None and y > x


def monte_joueur(a, b, cle):
    return any(nombre(a[cle][j]) is not None and nombre(b[cle][j]) is not None
               and nombre(b[cle][j]) > nombre(a[cle][j]) for j in (0, 1))


def baisse_joueur(a, b, cle):
    return any(nombre(a[cle][j]) is not None and nombre(b[cle][j]) is not None
               and nombre(b[cle][j]) < nombre(a[cle][j]) for j in (0, 1))


# Famille -> (comment on la reconnaît dans ce que la page publie,
#             le motif que le vol DOIT porter, le minimum d'occasions à voir)
FAMILLES = [
    ("la temperature monte", lambda a, b: monte(a, b, "temperature"), "jauge", 4),
    ("l'oxygene monte", lambda a, b: monte(a, b, "oxygene"), "jauge", 4),
    ("des megacredits depenses", lambda a, b: baisse_joueur(a, b, "mc"), "mc", 10),
    ("un jeton Foret gagne", lambda a, b: monte_joueur(a, b, "forets"), "jeton", 4),
    ("une carte piochee", lambda a, b: b["main"] > a["main"], "pioche", 10),
    ("une carte posee", lambda a, b: b["jeu"] > a["jeu"], "pose", 8),
    ("une carte defaussee", lambda a, b: monte(a, b, "defausse"), "defausse", 8),
]


def repondre(pg, delai=15000):
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
    occasions = {nom: 0 for nom, _, _, _ in FAMILLES}
    manques = {nom: [] for nom, _, _, _ in FAMILLES}
    with serveur(RACINE) as url:
        page_url = f"{url}/?graine={GRAINE}&siege=0&boites=base,decouverte"
        with ouvrir(page_url) as (pg, erreurs, _):
            pg.add_init_script(ESPION)
            pg.goto(page_url, wait_until="domcontentloaded")
            pg.wait_for_selector("[data-decision-rang]", timeout=20000, state="attached")
            avant = pg.evaluate(LECTURE)
            rangs = 0
            for _ in range(2000):
                rang = repondre(pg)
                if rang is None:
                    break
                pg.wait_for_timeout(260)
                apres = pg.evaluate(LECTURE)
                motifs = pg.evaluate("n => window.__m.slice(n)", avant["vols"])
                for nom, reconnait, motif, _ in FAMILLES:
                    try:
                        eu_lieu = reconnait(avant, apres)
                    except (TypeError, KeyError, IndexError):
                        eu_lieu = False
                    if not eu_lieu:
                        continue
                    occasions[nom] += 1
                    if motif not in motifs:
                        manques[nom].append(rang)
                avant = apres
                rangs += 1
            if erreurs:
                print(f"ECHEC : la page a leve {len(erreurs)} erreur(s) : {erreurs[0]}")
                return 1

    print(f"    {rangs} decision(s) jouees a l'ecran, graine {GRAINE}")
    for nom, _, motif, mini in FAMILLES:
        n, m = occasions[nom], len(manques[nom])
        marque = "  <-- SANS SON GESTE" if m else ""
        print(f"      {nom:28s} {n:4d} occasion(s), {m:4d} sans vol « {motif} »{marque}")

    if rangs < 40:
        print(f"ECHEC : {rangs} decision(s) seulement — la partie ne s'est pas jouee")
        return 1
    maigres = [f"{nom} ({occasions[nom]} < {mini})"
               for nom, _, _, mini in FAMILLES if occasions[nom] < mini]
    if maigres:
        print(f"ECHEC : famille(s) trop peu observee(s) : {', '.join(maigres)} — "
              f"ce banc ne prouve rien en l'etat")
        return 1
    total = sum(len(v) for v in manques.values())
    if total:
        detail = ", ".join(f"{nom} ({len(v)}, rangs {v[:4]})"
                           for nom, v in manques.items() if v)
        print(f"ECHEC : {total} evenement(s) sans le geste qui les dit — {detail}")
        return 1
    print("    chaque action de la liste fait voler ce qui la dit")
    return 0


if __name__ == "__main__":
    sys.exit(main())

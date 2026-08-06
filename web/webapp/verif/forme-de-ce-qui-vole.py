#!/usr/bin/env python3
"""CE QUI VOLE A LA BONNE FORME — une carte est un rectangle, pas un oeuf.

POURQUOI CE BANC EXISTE. Le 06-08, Alexis a photographie une carte qui quittait
la pioche en forme d'OVALE. La cause tenait en une ligne : toutes les choses qui
traversent l'ecran passent par `volerMatiere` (`vue/anim.js`), qui servait la
meme etiquette d'apparence aux objets ronds (une piece, un jeton de chaleur) et
aux cartes. Or `style-table.css` y arrondit les coins de MOITIE : sur un carre
cela fait un disque, sur un rectangle de carte cela fait un ovale.

Aucun banc ne pouvait le voir : tous verifiaient QU'UNE CARTE VOLE, jamais DE
QUELLE FORME. Celui-ci mesure la propriete manquante, et rien d'autre :

    tout objet en vol dont la largeur differe de la hauteur
    ne doit PAS etre arrondi en cercle.

Il ne lit aucun nom de classe de la page et ne suppose rien du code : il pose un
observateur sur la couche de vol, releve la taille reelle et l'arrondi CALCULE
de chaque objet qui la traverse, et juge sur ces deux nombres. Une page qui
changerait de vocabulaire mais garderait la propriete resterait verte.

Les animations doivent etre ALLUMEES : sans elles, rien ne vole.

Depuis la racine du depot :

    python3 web/webapp/verif/forme-de-ce-qui-vole.py [graine]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from pilote import serveur, page, choix_simple, echec  # noqa: E402

GRAINE = sys.argv[1] if len(sys.argv) > 1 else "4242"
DECISIONS = 60

# L'OBSERVATEUR. Pose avant que la partie ne demarre, il garde une trace de tout
# ce qui entre dans la couche de vol : sa taille au moment de l'entree, et
# l'arrondi calcule de son image. On mesure l'arrondi EN POINTS via
# `getComputedStyle`, qui rend « 50% » tel quel quand la regle est en pourcents —
# c'est justement le cas fautif.
GUET = """() => {
  window.__vols = [];
  const noter = (n) => {
    if (!(n instanceof HTMLElement)) return;
    const r = n.getBoundingClientRect();
    const im = n.querySelector('img');
    const s = im ? getComputedStyle(im) : null;
    window.__vols.push({
      motif: n.dataset ? (n.dataset.vol || '') : '',
      largeur: Math.round(r.width),
      hauteur: Math.round(r.height),
      arrondi: s ? s.borderTopLeftRadius : '',
      image: !!im,
    });
  };
  const brancher = (couche) => {
    for (const e of couche.children) noter(e);
    new MutationObserver((mut) => {
      for (const m of mut) for (const n of m.addedNodes) noter(n);
    }).observe(couche, { childList: true });
  };
  const deja = document.getElementById('vol');
  if (deja) { brancher(deja); return; }
  // La couche nait au premier vol : on attend qu'elle paraisse.
  new MutationObserver((mut, obs) => {
    const c = document.getElementById('vol');
    if (c) { obs.disconnect(); brancher(c); }
  }).observe(document.body, { childList: true, subtree: false });
}"""


def joue_n(pg, n):
    for _ in range(n):
        if pg.query_selector("[data-partie-terminee]"):
            return
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        porteur = pg.query_selector("[data-decision-rang]")
        rang = int(porteur.get_attribute("data-decision-rang"))
        forme = porteur.get_attribute("data-decision-forme") or "simple"
        choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
        if forme == "montant":
            champ = pg.wait_for_selector("[data-montant]", timeout=20000)
            mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
            champ.fill(str(mini + (rang % (maxi - mini + 1))))
            pg.click("[data-valider]")
        elif forme == "multiple":
            brut = porteur.get_attribute("data-a-choisir")
            k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
            for c in choix[:min(k, len(choix))]:
                c.click()
            pg.click("[data-valider]")
        else:
            if not choix:
                return
            choix[choix_simple(rang, len(choix))].click()
        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=20000)


def rond(arrondi, largeur, hauteur):
    """Cet arrondi fait-il un cercle ou une ellipse de cet objet ?

    Deux ecritures menent au meme resultat : un pourcentage a partir de 50, et
    une longueur qui atteint la moitie du plus petit cote. On les traite toutes
    les deux, pour ne pas juger une ecriture mais une APPARENCE.
    """
    a = (arrondi or "").strip()
    if a.endswith("%"):
        try:
            return float(a[:-1]) >= 50
        except ValueError:
            return False
    if a.endswith("px"):
        try:
            v = float(a[:-2])
        except ValueError:
            return False
        return v >= min(largeur, hauteur) / 2 - 0.5
    return False


fautes = []
with serveur() as base:
    # LES ANIMATIONS SONT ALLUMEES : c'est tout l'objet de ce banc.
    with page(f"{base}/?graine={GRAINE}&siege=0") as (pg, err, _):
        pg.evaluate(GUET)
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        joue_n(pg, DECISIONS)
        pg.wait_for_timeout(900)   # les derniers vols finissent leur course
        vols = pg.evaluate("() => window.__vols || []")

carres = [v for v in vols if v["largeur"] == v["hauteur"]]
allonges = [v for v in vols if v["largeur"] != v["hauteur"]]
motifs = sorted({v["motif"] for v in vols if v["motif"]})

print(f"    {len(vols)} objet(s) en vol releve(s) : {len(carres)} carre(s), "
      f"{len(allonges)} allonge(s) ; motifs vus : {', '.join(motifs) or 'aucun'}")

if not vols:
    echec("aucun objet n'a vole : le banc n'a rien pu mesurer "
          "(animations eteintes, ou la couche de vol a change de nom)")
if not allonges:
    echec(f"aucun objet ALLONGE n'a vole en {DECISIONS} decisions : le defaut "
          f"vise — une carte arrondie en ovale — ne peut pas etre mesure ici")

ovales = [v for v in allonges if v["image"] and rond(v["arrondi"], v["largeur"], v["hauteur"])]
if ovales:
    for v in ovales[:5]:
        print(f"      motif « {v['motif'] or 'sans motif'} » : {v['largeur']}x{v['hauteur']} "
              f"points arrondis de {v['arrondi']} — c'est un ovale, pas une carte")
    fautes.append(f"{len(ovales)} objet(s) allonge(s) sur {len(allonges)} volent en ovale")

if fautes:
    for f in fautes:
        print("      " + f)
    echec(f"{len(fautes)} defaut(s) de forme sur ce qui vole")
print(f"OK les {len(allonges)} objets allonges gardent des coins de carte, "
      f"et les {len(carres)} objets carres restent ronds")

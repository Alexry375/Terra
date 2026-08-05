#!/usr/bin/env python3
"""DEFAUT SIGNALE, NON CORRIGE : l'etat publie par le moteur RECULE.

Ce banc ne juge pas ce chantier — il documente un defaut ANTERIEUR, pour que la
main puisse le reproduire seul en une commande. Il joue une partie entiere, et
compte les fois ou l'etat lu a une decision est plus ANCIEN que celui lu a la
decision precedente : `generation`, `planet.temperature`, `planet.oxygen` ou
`planet.oceans` diminue, puis remonte.

Il se lance indifferemment sur la livraison ou sur le depot d'origine, et rend
le meme verdict sur les deux — c'est tout l'interet :

    python3 web/webapp/verif/recul-etat.py outputs/web/webapp
    python3 web/webapp/verif/recul-etat.py inputs/web/webapp

Le banc du depot le voit deja, hors navigateur, donc du cote du moteur ou du
pont, et non du cote de l'affichage :

    (cd web/webapp && node verif/tests.mjs)   -> KO « les parametres
                                                          planetaires ne
                                                          reculent jamais »
    (cd inputs/web/webapp  && node verif/tests.mjs)   -> le meme KO

CONSEQUENCE SUR CE CHANTIER. Quand l'etat recule, une tuile Ocean deja
retournee se retourne a l'envers, et le repere d'un arc redescend d'une case.
La planche et les arcs SUIVENT l'etat qu'on leur donne : c'est ce que le
controle 03 exige (« autant de tuiles face visible que le moteur en a
revele »), et masquer le symptome ferait mentir l'ecran sur ce que le moteur
pense. Le prompt demande de signaler une regle qui semble fausse, pas de la
corriger.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer  # noqa: E402

LECTURE = """() => {
  const nb = (s) => {
    const e = document.querySelector(s);
    return e ? Number(e.textContent.replace(/[^-0-9]/g, '') || 0) : null;
  };
  return {
    generation: nb('[data-valeur="generation"]'),
    temperature: nb('[data-valeur="planet.temperature"]'),
    oxygen: nb('[data-valeur="planet.oxygen"]'),
    oceans: nb('[data-valeur="planet.oceans"]'),
  };
}"""
CHAMPS = ("generation", "temperature", "oxygen", "oceans")

racine = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
graine = sys.argv[2] if len(sys.argv) > 2 else "5150"

lectures = []
with serveur(racine) as base:
    with page(f"{base}/?graine={graine}&siege=0") as (pg, erreurs, _):
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        jouer(pg, apres=lambda p, r: lectures.append((r, p.evaluate(LECTURE))))

reculs = []
for i in range(1, len(lectures)):
    avant, apres = lectures[i - 1][1], lectures[i][1]
    baisse = [c for c in CHAMPS if apres[c] < avant[c]]
    if baisse:
        reculs.append((lectures[i][0], baisse, avant, apres))

print(f"{racine} graine {graine} : {len(lectures)} lectures, {len(reculs)} recul(s) d'etat")
for rang, baisse, avant, apres in reculs[:5]:
    print(f"  decision {rang} : {'+'.join(baisse)} recule — {avant} puis {apres}")
if not reculs:
    print("OK aucun recul (le defaut a ete corrige depuis)")
    sys.exit(0)
print("DEFAUT ANTERIEUR CONFIRME — a comparer avec la meme commande sur "
      "`inputs/web/webapp`, qui doit en montrer autant")

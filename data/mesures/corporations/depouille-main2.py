#!/usr/bin/env python3
"""L'ACCORD MAIN/CORPORATION, PAR TOUS LES CANAUX QU'ON PEUT TESTER

Le premier test (depouille-main.py) ne regardait que le TAG de la corporation.
C'est le canal le plus evident, pas le seul : Credicor aide les cartes cheres,
Phobolog les cartes spatiales, Thorgate l'energie. Une main peut donc « appeler »
une corporation sans partager son tag.

On teste ici cinq caracteristiques de la main, pour chaque corporation, en
comparant PRISE contre REFUSEE — comparaison toujours appariee, la corporation
face a elle-meme.

Et on refait le tout sur les seules PAIRES AMBIGUES (les deux corporations
separees de moins de 11 points au classement) : c'est la que le classement ne
tranche pas, donc la que la main devrait decider si elle decide quelque part.
"""
import json, sys, math
from collections import defaultdict

RACINE = "/home/alexis/Global/Agents_Projects/Terra"
cartes = json.load(open(f"{RACINE}/data/cards.json"))
INFO = {c["name"]: c for c in cartes}
TAGS_CORPO = {c["name"]: set(c["tags"] or [])
              for c in cartes
              if c["category"] == "corporation" and c["box"] in ("base", "discovery")}
FORCE = {
 "Apollo Industries":14.02,"Tharsis Republic":13.71,"Exocorp":11.88,
 "Teractor Corporation":4.27,"Sultira":1.78,"Helion Corporation":-0.49,
 "Thorgate Corporation":-1.74,"Phobolog":-2.18,"Ecoline":-4.28,
 "Unmi":-4.74,"Credicor":-4.77,"Hyperion Systems":-5.35,
 "Interplanetary Cinematics":-5.78,"Mining Guild":-6.45,
 "Inventrix":-6.76,"Saturn Systems":-6.94,
}

def traits(main, corpo):
    prix = [INFO.get(c, {}).get("price") or 0 for c in main]
    coul = [INFO.get(c, {}).get("category") for c in main]
    tags_corpo = TAGS_CORPO.get(corpo, set())
    return {
        "cartes du tag de la corpo": sum(1 for c in main if set(INFO.get(c, {}).get("tags") or []) & tags_corpo),
        "cout moyen de la main":     sum(prix) / len(prix) if prix else 0,
        "cartes cheres (>=20 MC)":   sum(1 for p in prix if p >= 20),
        "cartes vertes":             sum(1 for c in coul if c == "green"),
        "cartes bleues+rouges":      sum(1 for c in coul if c in ("blue", "red")),
    }

def analyse(lignes, titre):
    pris, refuse = defaultdict(lambda: defaultdict(list)), defaultdict(lambda: defaultdict(list))
    for d in lignes:
        main = d.get("main") or []
        for nom in d.get("proposees") or []:
            if nom not in FORCE:
                continue
            t = traits(main, nom)
            cible = pris if nom == d.get("prise") else refuse
            for k, v in t.items():
                cible[nom][k].append(v)

    print(f"\n{'='*96}\n{titre}\n{'='*96}")
    pires = []
    for critere in ["cartes du tag de la corpo", "cout moyen de la main",
                    "cartes cheres (>=20 MC)", "cartes vertes", "cartes bleues+rouges"]:
        forts = []
        for nom in sorted(FORCE):
            a, b = pris[nom][critere], refuse[nom][critere]
            if len(a) < 5 or len(b) < 5:
                continue
            ma, mb = sum(a)/len(a), sum(b)/len(b)
            va = sum((x-ma)**2 for x in a)/max(1, len(a)-1)
            vb = sum((x-mb)**2 for x in b)/max(1, len(b)-1)
            err = math.sqrt(va/len(a) + vb/len(b))
            z = (ma-mb)/err if err > 0 else 0.0
            forts.append((abs(z), z, nom, ma-mb, len(a), len(b)))
        forts.sort(reverse=True)
        n_signif = sum(1 for f in forts if f[0] >= 2)
        pic = forts[0] if forts else None
        print(f"  {critere:28} corporations testees : {len(forts):2}   "
              f"au-dela de 2 ecarts types : {n_signif}"
              + (f"   |  plus fort : {pic[2]} {pic[1]:+.2f}" if pic else ""))
        pires.extend(forts)
    pires.sort(reverse=True)
    print("\n  les cinq ecarts les plus forts, tous criteres confondus :")
    for z, zs, nom, ec, na, nb in pires[:5]:
        print(f"    {nom:28} {zs:+6.2f} ecarts types   (prise {na}x, refusee {nb}x)")

lignes = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
analyse(lignes, f"TOUTES LES PAIRES  ({len(lignes)} choix)")

amb = [d for d in lignes
       if len(d.get("proposees") or []) == 2
       and all(n in FORCE for n in d["proposees"])
       and abs(FORCE[d["proposees"][0]] - FORCE[d["proposees"][1]]) <= 11]
analyse(amb, f"PAIRES AMBIGUES SEULEMENT  ({len(amb)} choix) — la ou le classement ne tranche pas")

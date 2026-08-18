#!/usr/bin/env python3
"""L'IA ACCORDE-T-ELLE SA CORPORATION A SA MAIN ?

    python3 data/mesures/corporations/depouille-main.py main-au-choix.jsonl

Le classement des corporations est une MOYENNE : il ne peut pas dire « Saturn
Systems cesse d'etre la pire quand vous tenez trois cartes Jupiter ». Mais le
choix se fait cartes en main (flow.rs:227) et la main figure dans la description
transmise au reseau (description.rs:317-350) : l'information est disponible.

LE TEST. Chaque corporation porte un tag (Saturn Systems : Jupiter ; Apollo
Industries : Spatial ; ...). Pour chacune, on compare le nombre de cartes de CE
tag dans la main, entre les fois ou elle a ete PRISE et les fois ou elle a ete
REFUSEE. Si l'IA lit sa main, la difference doit etre positive.

Le test est apparie par construction : la meme corporation est comparee a
elle-meme, donc sa force intrinseque ne joue aucun role. Ce qui reste est
l'effet de la main, et rien d'autre.
"""
import json, sys, math
from collections import defaultdict

RACINE = "/home/alexis/Global/Agents_Projects/Terra"
cartes = json.load(open(f"{RACINE}/data/cards.json"))
TAGS_CARTE = {c["name"]: set(c["tags"] or []) for c in cartes}
TAGS_CORPO = {c["name"]: set(c["tags"] or [])
              for c in cartes
              if c["category"] == "corporation" and c["box"] in ("base", "discovery")}

pris = defaultdict(list)     # corpo -> nb de cartes du bon tag dans la main
refuse = defaultdict(list)

for ligne in open(sys.argv[1]):
    if not ligne.strip():
        continue
    d = json.loads(ligne)
    main = d.get("main") or []
    for nom in d.get("proposees") or []:
        tags = TAGS_CORPO.get(nom)
        if not tags:
            continue                      # Tharsis Republic n'a aucun tag
        n = sum(1 for c in main if TAGS_CARTE.get(c, set()) & tags)
        (pris if nom == d.get("prise") else refuse)[nom].append(n)

def moy(v): return sum(v) / len(v) if v else float("nan")

print(f"{'corporation':28} {'tag':10} {'prise':>6} {'refusee':>8}   "
      f"{'tags en main si prise':>21} {'si refusee':>11}   {'ecart':>7}  {'ecarts types':>12}")
lignes = []
for nom in sorted(TAGS_CORPO):
    a, b = pris[nom], refuse[nom]
    if len(a) < 5 or len(b) < 5:
        continue
    ma, mb = moy(a), moy(b)
    va = sum((x - ma) ** 2 for x in a) / max(1, len(a) - 1)
    vb = sum((x - mb) ** 2 for x in b) / max(1, len(b) - 1)
    err = math.sqrt(va / len(a) + vb / len(b))
    z = (ma - mb) / err if err > 0 else 0.0
    lignes.append((z, nom, list(TAGS_CORPO[nom])[0], len(a), len(b), ma, mb, ma - mb))

for z, nom, tag, na, nb, ma, mb, ec in sorted(lignes, key=lambda x: -x[0]):
    marque = "  <<<" if abs(z) >= 2 else ""
    print(f"{nom:28} {tag:10} {na:6} {nb:8}   {ma:21.2f} {mb:11.2f}   {ec:+7.2f}  {z:+12.2f}{marque}")

print("\n« ecarts types » : au-dela de +2 ou -2, l'ecart n'est pas imputable au hasard.")
print("Positif = l'IA prend cette corporation plus volontiers quand sa main porte le bon tag.")

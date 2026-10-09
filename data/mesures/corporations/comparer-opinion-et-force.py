# -*- coding: utf-8 -*-
"""Compare le classement SELON L'IA (champion 100 neurones, 09-10) au classement
de FORCE REELLE mesure en aout (corporation imposee, ANCIEN reseau)."""
import json, math, statistics
from collections import defaultdict

OPINION = {  # note Bradley-Terry du champion, 09-10
 "Tharsis Republic":393.2,"Exocorp":251.6,"Apollo Industries":162.3,
 "Interplanetary Cinematics":67.5,"Saturn Systems":57.6,"Credicor":47.0,
 "Sultira":-35.3,"Thorgate Corporation":-42.2,"Teractor Corporation":-44.9,
 "Inventrix":-47.2,"Phobolog":-48.5,"Mining Guild":-75.2,
 "Hyperion Systems":-120.0,"Helion Corporation":-142.3,"Unmi":-208.8,
 "Ecoline":-214.9,
}

vic = defaultdict(float); n = defaultdict(int); ecart = defaultdict(float)
for f in ("data/mesures/corporations/tournoi-corpos.jsonl",
          "data/mesures/corporations/tournoi-corpos-2.jsonl"):
    for l in open(f, encoding="utf-8"):
        d = json.loads(l)
        if not d.get("complete"): continue
        for moi, lui in ((0,1),(1,0)):
            c = d[f"corpo{moi}"]; s = d[f"score{moi}"]; a = d[f"score{lui}"]
            n[c] += 1
            ecart[c] += s - a
            vic[c] += 1.0 if s > a else (0.5 if s == a else 0.0)

reel = {c: 100*vic[c]/n[c] for c in n}
print(f"{'corporation':<28}{'rang IA':>8}{'note IA':>9}{'rang reel':>10}{'% vict':>8}{'ecart':>8}{'parties':>8}")
ra = sorted(OPINION, key=lambda c: -OPINION[c])
rr = sorted(reel, key=lambda c: -reel[c])
for c in ra:
    print(f"{c:<28}{ra.index(c)+1:>8}{OPINION[c]:>9.0f}"
          f"{rr.index(c)+1 if c in rr else 0:>10}{reel.get(c,0):>7.1f}%"
          f"{ecart.get(c,0)/max(n.get(c,1),1):>8.1f}{n.get(c,0):>8}")

def pearson(a,b):
    ma,mb=statistics.fmean(a),statistics.fmean(b)
    num=sum((x-ma)*(y-mb) for x,y in zip(a,b))
    da=math.sqrt(sum((x-ma)**2 for x in a)); db=math.sqrt(sum((y-mb)**2 for y in b))
    return num/(da*db) if da and db else 0.0
communs=[c for c in OPINION if c in reel]
xs=[OPINION[c] for c in communs]; ys=[reel[c] for c in communs]
print()
print(f"corporations comparables : {len(communs)}")
print(f"lien note IA (09-10) / force reelle (aout) : {pearson(xs,ys):+.3f}")
rx=[ra.index(c)+1 for c in communs]; ry=[rr.index(c)+1 for c in communs]
print(f"lien des RANGS : {pearson(rx,ry):+.3f}")
# hors trio de tete
h=[c for c in communs if ra.index(c)>2]
print(f"hors trio de tete ({len(h)}) : {pearson([OPINION[c] for c in h],[reel[c] for c in h]):+.3f}")
print()
print(f"parties par corporation : min {min(n.values())}, max {max(n.values())}, "
      f"moyenne {statistics.fmean(n.values()):.0f}")
print("incertitude a ~50 parties : environ +/- 7 points de pourcentage")

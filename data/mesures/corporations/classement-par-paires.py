# -*- coding: utf-8 -*-
"""Classement des corporations SELON L'IA, corrige du biais de voisinage.

Le taux de choix brut ne vaut rien seul : une corporation mediocre tiree contre
une epouvantable est prise souvent. On ajuste donc un classement par
affrontements deux a deux (methode de Bradley-Terry) : on cherche pour chaque
corporation une force f telle que P(X preferee a Y) = f(X)/(f(X)+f(Y)), et on
ajuste les seize forces pour coller au mieux aux 2000 choix observes.
"""
import json, math, sys
from collections import defaultdict

ARGENT = {
 "Teractor Corporation":51,"Credicor":48,"Interplanetary Cinematics":46,
 "Thorgate Corporation":45,"Tharsis Republic":40,"Sultira":38,"Unmi":35,
 "Apollo Industries":33,"Inventrix":33,"Hyperion Systems":30,
 "Helion Corporation":28,"Ecoline":27,"Mining Guild":27,"Exocorp":26,
 "Saturn Systems":24,"Phobolog":20,
}

lignes = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
print(f"observations : {len(lignes)}")

gagne = defaultdict(int)   # (X,Y) -> fois ou X est preferee a Y
vu    = defaultdict(int)
noms  = set()
for d in lignes:
    a, b = d["proposees"]
    pr = d["prise"]
    noms.update([a, b])
    if a == b:            # paire identique : aucune information
        continue
    x, y = (a, b) if a < b else (b, a)
    vu[(x, y)] += 1
    if pr == x:
        gagne[(x, y)] += 1

noms = sorted(noms)
idx = {n: i for i, n in enumerate(noms)}
N = len(noms)
print(f"corporations : {N}   paires distinctes rencontrees : {len(vu)} sur {N*(N-1)//2}")

# --- Bradley-Terry, par montee iterative, avec une regularisation faible ---
# La regularisation (un demi-match nul fictif contre un adversaire moyen) evite
# la divergence pour une corporation jamais refusee, comme Tharsis Republic.
f = [1.0] * N
PRIOR = 0.5
for tour in range(5000):
    num = [PRIOR] * N
    den = [0.0] * N
    for (x, y), n in vu.items():
        i, j = idx[x], idx[y]
        wx = gagne[(x, y)]
        wy = n - wx
        num[i] += wx
        num[j] += wy
        s = f[i] + f[j]
        den[i] += n / s
        den[j] += n / s
    moy = sum(f) / N
    for i in range(N):
        den[i] += PRIOR / (f[i] + moy)
    nf = [num[i] / den[i] if den[i] > 0 else f[i] for i in range(N)]
    g = (math.prod(nf)) ** (1.0 / N)
    nf = [v / g for v in nf]
    bouge = max(abs(nf[i] - f[i]) for i in range(N))
    f = nf
    if bouge < 1e-12:
        break
print(f"ajustement : {tour+1} tours, derniere variation {bouge:.2e}")

# Note en "points" lisibles : logarithme de la force, cale a 0 pour la moyenne.
import statistics
lg = [math.log(v) for v in f]
m = statistics.fmean(lg)
pts = [(v - m) * 100 / math.log(10) for v in lg]   # 100 points = facteur 10

brut = {}
for (x, y), n in vu.items():
    brut.setdefault(x, [0, 0]); brut.setdefault(y, [0, 0])
    brut[x][0] += gagne[(x, y)]; brut[x][1] += n
    brut[y][0] += n - gagne[(x, y)]; brut[y][1] += n

rang = sorted(range(N), key=lambda i: -pts[i])
print()
print(f"{'rang':>4} {'corporation':<28}{'note':>8} {'credits':>8} {'brut':>9} {'occasions':>10}")
for r, i in enumerate(rang, 1):
    n = noms[i]
    w, t = brut[n]
    print(f"{r:>4} {n:<28}{pts[i]:>8.1f} {ARGENT.get(n,0):>8} {100*w/t:>8.1f}% {t:>10}")

# --- Lien avec l'argent de depart ---
xs = [ARGENT[noms[i]] for i in range(N)]
ys = [pts[i] for i in range(N)]
def pearson(a, b):
    ma, mb = statistics.fmean(a), statistics.fmean(b)
    num = sum((x-ma)*(y-mb) for x, y in zip(a, b))
    da = math.sqrt(sum((x-ma)**2 for x in a)); db = math.sqrt(sum((y-mb)**2 for y in b))
    return num/(da*db) if da and db else 0.0
print()
print(f"lien entre l'argent de depart et la note de l'IA : {pearson(xs, ys):+.3f}")

# --- Coherence : l'IA est-elle transitive ? ---
pref = {}
for (x, y), n in vu.items():
    w = gagne[(x, y)]
    if n >= 3:
        if w > n - w: pref[(x, y)] = x
        elif n - w > w: pref[(x, y)] = y
def mieux(a, b):
    k = (a, b) if a < b else (b, a)
    return pref.get(k)
cycles = 0; triplets = 0
for i in range(N):
    for j in range(i+1, N):
        for k in range(j+1, N):
            a, b, c = noms[i], noms[j], noms[k]
            p1, p2, p3 = mieux(a, b), mieux(b, c), mieux(a, c)
            if None in (p1, p2, p3): continue
            triplets += 1
            if p1 == a and p2 == b and p3 == c: cycles += 1
            if p1 == b and p2 == c and p3 == a: cycles += 1
print(f"triplets exploitables : {triplets}, dont incoherents (cycles) : {cycles}")

# LE TÉMOIN — tout ce que l'IA « voyante » savait faire, gelé le 18-08-2026

> Ce document fige l'état de l'IA **avant** le dernier entraînement. Il sert de
> point de comparaison : la nouvelle IA devra faire mieux, et sur les mêmes
> mesures. Rien ici ne doit être modifié ; les chiffres nouveaux vont ailleurs.

## Ce qu'était cette IA, en une phrase

Un réseau de neurones entraîné sur **1 200 000 parties** contre lui-même, qui
**voyait le hasard futur** quand il essayait une option (défaut n°2) et qui était
**structurellement aveugle** au mulligan des corporations (défaut n°1).

## Les poids gelés

| fichier | parties | rôle |
|---|---|---|
| `data/temoin/temoin-voyant-1M.txt` | 1 000 000 | le réseau A du duel final |
| `data/temoin/temoin-voyant-1200k.txt` | 1 200 000 | le plus fort obtenu avant l'audit |

Copies figées. Les originaux vivent dans `data/poids/` et peuvent bouger.

## 1. Le classement des corporations, corporation imposée au hasard

799 parties, 1 598 observations. Écart de score = mon score moins celui de
l'adversaire. Incertitude à deux écarts types.

| corporation | parties | victoires | écart de score |
|---|---|---|---|
| Apollo Industries | 110 | 68,2 % ± 8,9 | **+14,02 ± 5,70** |
| Tharsis Republic | 96 | 75,0 % ± 8,8 | **+13,71 ± 5,80** |
| Exocorp | 112 | 67,9 % ± 8,8 | **+11,88 ± 5,46** |
| Teractor Corporation | 96 | 52,1 % ± 10,2 | +4,27 ± 6,67 |
| Sultira | 95 | 42,1 % ± 10,1 | +1,78 ± 5,48 |
| Helion Corporation | 105 | 45,7 % ± 9,7 | −0,49 ± 5,47 |
| Thorgate Corporation | 109 | 49,5 % ± 9,6 | −1,74 ± 5,43 |
| Phobolog | 109 | 46,8 % ± 9,6 | −2,18 ± 5,46 |
| Ecoline | 102 | 45,1 % ± 9,9 | −4,28 ± 6,14 |
| Unmi | 98 | 40,8 % ± 9,9 | −4,74 ± 5,46 |
| Credicor | 96 | 42,7 % ± 10,1 | −4,77 ± 5,61 |
| Hyperion Systems | 92 | 44,6 % ± 10,4 | −5,35 ± 6,34 |
| Interplanetary Cinematics | 102 | 44,1 % ± 9,8 | −5,78 ± 6,27 |
| Mining Guild | 103 | 40,8 % ± 9,7 | −6,45 ± 5,71 |
| Inventrix | 94 | 40,4 % ± 10,1 | −6,76 ± 5,73 |
| Saturn Systems | 79 | 36,7 % ± 10,8 | −6,94 ± 6,41 |

**Réserve inscrite le 18-08** : le banc du mulligan montre que cette échelle
**exagère** l'effet réel du choix de corporation. Un gain théorique de +9,11 sur
cette échelle n'a produit que +2,64 points de score réel. L'ordre du classement
reste probablement bon ; l'ampleur, non.

## 2. La qualité des choix de corporation

1 000 choix, chacun entre deux corporations proposées. « Tranchées » = les deux
corporations sont séparées de plus de 11 points, donc le bon choix est certain.

| joueur | choix tranchés | bons | perte moyenne |
|---|---|---|---|
| **IA au million** | 285 | **96,1 %** | 0,69 ± 0,41 |
| IA au million (tous) | 1 000 | 66,2 % | 1,18 ± 0,17 |
| témoin à règles écrites | 285 | 45,6 % | 8,95 ± 1,00 |
| joueur au hasard | 1 000 | 50,0 % | 3,81 |
| joueur parfait | 1 000 | 100 % | 0,00 |

Le témoin à règles écrites juge une corporation sur son argent de départ
(corrélation 0,94 entre sa préférence et l'argent) — critère sans lien avec la
victoire (corrélation 0,10 entre argent de départ et force). **Il fait donc pire
que le hasard sur les choix qui comptent.**

## 3. Le comportement réel en mise en place

200 donnes, l'IA décide tout, rien n'est imposé.

| décision | IA au million | témoin à règles écrites |
|---|---|---|
| mulligan des corporations | **400 gardes, 0 remplacement** | 400 gardes, 0 remplacement |
| mulligan des projets | **1,99 carte rendue sur 8** | 6,07 cartes rendues |

Distribution des cartes rendues par l'IA : 0 → 92 fois · 1 → 77 · 2 → 93 ·
3 → 66 · 4 → 36 · 5 → 25 · 6 → 10 · 7 → 1 · 8 → 0.

**Lecture corrigée le 18-08** : les 400 gardes ne sont pas un choix, mais
l'absence de choix (les deux options ont une note identique à 17 décimales). Et
les 2 cartes rendues sur 8 s'expliquent en partie par la vision du hasard futur :
l'IA ne rend une carte qu'après avoir vérifié que le remplacement est meilleur.

## 4. Les améliorations de cartes Phase

526 améliorations relevées sur 50 parties entières.

| phase | choix A | choix B | part de B | A prise/proposée | B prise/proposée |
|---|---|---|---|---|---|
| I Développement | 41 | 69 | 62,7 % | 10,6 % | 21,0 % |
| II Construction | 95 | 2 | 2,1 % | 67,4 % | 0,4 % |
| III Action | 81 | 3 | 3,6 % | 33,3 % | 0,6 % |
| IV Production | 68 | 42 | 38,2 % | 19,5 % | 10,1 % |
| V Recherche | 25 | 100 | 80,0 % | 5,7 % | 62,5 % |

Phase choisie quand elle est libre : IV 24,4 % · V 24,4 % · I 22,3 % ·
II 19,6 % · III 9,3 %.

**Motif observé** : l'IA prend systématiquement la variante qui met des cartes en
jeu plutôt que celle qui donne de l'argent. Elle refuse 7 MC pour rejouer une
production ; elle refuse deux activations pour révéler trois cartes ; en Recherche
elle préfère voir 8 et garder 2 plutôt que voir 4 et garder 3 — le tri prime sur
l'accumulation.

## 5. L'IA n'accorde pas sa corporation à sa main

3 000 choix relevés avec le contenu exact de la main. Comparaison appariée, la
même corporation prise contre refusée, sur cinq caractéristiques.

80 tests au total. Seuil corrigé pour les tests multiples : **3,42 écarts types**.
Écart le plus fort observé : **2,87** (Saturn Systems, cartes bleues et rouges).
Faux positifs attendus par pur hasard : 3,6 ; observés : 6.

**Aucun effet ne survit.** Sur le tag de la corporation — le canal le plus évident
— pas un seul test n'atteint 2 (maximum : Interplanetary Cinematics +1,61 ;
Saturn Systems **−0,86**).

## 6. Le verdict du million : la devinette ne paye pas

Deux réseaux, un million de parties chacun, mêmes 80 donnes × 2 sièges.
A = sans devinette. C = avec devinette (un second réseau qui prédit la carte
Phase de l'adversaire).

| duel | C | A | écart de score | verdict |
|---|---|---|---|---|
| juges seuls, devinette éteinte | 40,0 % | **58,1 %** | −3,61 | 2,31 écarts types → **A meilleur** |
| C avec sa devinette allumée | 49,4 % | 48,8 % | +0,33 | 0,08 → égalité |

Coût : C a demandé **85 547 s** contre **60 530 s** pour A, soit **41 % de calcul
en plus** pour finir à égalité.

**Réserve** : ces duels font jouer l'IA contre l'IA. L'apport de la devinette
contre un humain n'a jamais été mesuré.

## 7. Le mulligan des corporations — la mesure inachevée

Design apparié : la même donne jouée deux fois pour le même siège, une fois en
remplaçant les deux corporations, une fois en gardant.

Seuil théorique calculé sur les 120 paires possibles : l'espérance du meilleur de
deux corporations tirées au hasard vaut **+3,74**. Règle : remplacer si la
meilleure des deux tenues est sous ce seuil — c'est-à-dire toute paire ne
contenant ni Apollo Industries, ni Tharsis Republic, ni Exocorp, ni Teractor
Corporation. Cela arrive dans **55 %** des cas, pour un gain théorique de
**+3,38 points par partie**.

Mesure sur 141 donnes déclenchées (sur 240) :

```
la règle se déclenche                        58,8 %   (théorie 55 %)
le remplacement donne une corporation
  mieux classée                              87 % du temps
gain de force sur le papier                  +9,11
gain de score réellement mesuré              +2,64 ± 6,57   (0,80 écart type)
taux de victoire                             52,8 % en remplaçant, 44,7 % en gardant
test apparié sur les victoires               1,46 écart type
écart à ce que le classement prédisait       −1,97 écart type
```

**Conclusion** : le signe va dans le sens attendu, l'ampleur n'est pas prouvée, et
le classement des corporations exagère l'effet du choix. Des lots
supplémentaires étaient en cours au moment du gel.

## 8. Les coûts de calcul, pour comparaison

| entraînement | parties | durée | rythme |
|---|---|---|---|
| A, sans devinette | 1 000 000 | 60 530 s (16 h 49) | 60,5 ms par partie |
| C, avec devinette | 1 000 000 | 85 547 s (23 h 46) | 85,5 ms par partie |
| A, reprise vers 1,2 M | 200 000 | 14 378 s (4 h 00) | 71,9 ms par partie |

Le tout sur **un seul cœur** : le moteur n'a aucun parallélisme.

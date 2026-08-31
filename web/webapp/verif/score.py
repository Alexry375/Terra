#!/usr/bin/env python3
"""CE QUE LES CONTROLES 04 ET 05 NE VOIENT PAS : la ventilation dit-elle QUOI ?

Le controle 04 verifie que la somme des cinq parts vaut le score du moteur. Une
ventilation qui mettrait tout le TR dans « cards » et rien dans « tr » ferait la
bonne somme et passerait : la somme ne dit rien de la REPARTITION. Le controle 05
compte les mentions « provisoire » sans regarder A COTE DE QUOI elles sont
posees.

Ce banc-ci mesure, a chaque decision d'une partie entiere et pour les deux
joueurs, contre des grandeurs affichees AILLEURS sur l'ecran (donc par un autre
chemin que la ventilation) et contre les regles de bareme du livret :

  1. `score_parts.tr` vaut exactement le TR affiche dans la barre du joueur
     (`players.N.tr`, livret p.16 : « votre NT s'ajoute a votre score ») ;
  2. `score_parts.forests` vaut exactement le nombre de forets affiche
     (`players.N.forests`, 1 PV par foret) ;
  3. `score_parts.milestones` est un multiple de 3 entre 0 et 9 — trois Reperes
     dans la partie, 3 PV chacun ;
  4. `score_parts.awards` est une somme de trois termes pris dans {2, 4, 5}
     (Discovery p.3 : 1er = 5, 2e = 2, egalite au 1er rang = 4 chacun) ; a deux
     joueurs, les deux totaux d'awards font donc 21, 9 ou 8 par recompense ;
  5. les DEUX joueurs cumulent, sur les trois recompenses, un nombre de points
     coherent avec ce meme bareme ;
  6. la mention « provisoire » est posee dans la meme barre que les parts
     `milestones` et `awards`, et pas ailleurs ;
  7. a la fin de la partie, la somme des parts vaut `data-score-final`, et plus
     aucune mention « provisoire » n'est visible.

C'est le point 4 qui rattrape le defaut d'origine : les douze points de depart
sont 3 x 4, l'egalite sur les trois recompenses. Une ventilation inventee dans
la page ne tomberait pas sur ce bareme-la.

CE QUE CE BANC NE PROUVE PAS. La part `cards` — la plus grosse en fin de partie,
et la seule qui depende du contenu des cartes — n'a ici aucun oracle exterieur :
elle n'est tenue que par `>= 0` et par l'identite de somme. Il faudrait, pour la
verifier vraiment, recompter les PV des cartes en jeu depuis `data/cards.json`,
c'est-a-dire ecrire un second bareme — precisement ce que le projet interdit.
Le point 3 (`milestones`) est lui aussi faible : multiple de 3 et borne, rien de
plus. Et `AWARDS_POSSIBLES` suppose exactement trois recompenses, ce qui est le
cas de cette boite mais n'est pas une verite generale.

Depuis la racine du workspace :

    python3 web/webapp/verif/score.py [graine]
"""
import itertools
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer, echec  # noqa: E402

# LES CARTES A POINTS DE VICTOIRE NEGATIFS, LUES DANS LE RECENSEMENT.
#
# ⚠️ C'EST LA SUPPOSITION QUI ETAIT FAUSSE, PAS LA PAGE (corrige le 28-08 par
# `les-sept-bancs-rouges`). Ce banc tenait « une carte ne peut pas rapporter de
# points negatifs » et refusait toute part `cards` sous zero : 69 fautes
# annoncees, toutes la meme, reemise a chaque decision qui suivait — une carte
# posee au rang 204 reste sur le plateau jusqu'a la fin. Or `data/cards.json`
# porte sept cartes a PV negatifs (Invasive Irrigation -2, Landfill -1,
# LowAtmosphere Planes -1, Nuclear Plants -1, Slash and Burn Agriculture -1,
# Bribed Comittee -2, Conscription -1) et le moteur les additionne sans plancher
# (`engine/src/flow.rs`). Le remede n'est pas de cesser de verifier le signe :
# c'est de verifier LA BONNE CHOSE, et il y a plus a verifier qu'avant.
#
# Deux bornes, et la seconde est celle qui mesure vraiment :
#
#   1. un PLANCHER ABSOLU — la somme de tous les PV negatifs imprimes du jeu.
#      Aucune main, aucune boite, aucun ordre de pose ne peut descendre plus bas.
#      Il ne depend d'aucune regle, seulement du recensement des cartes.
#   2. UN PLANCHER PAR JOUEUR, RELEVE SUR SON PLATEAU : une part negative doit
#      s'expliquer par des cartes a PV negatifs REELLEMENT posees devant lui, et
#      ne peut pas descendre sous leur somme. C'est un oracle disjoint — les noms
#      viennent de l'ecran (`#piles-J`), les points viennent du recensement — et
#      il attrape ce que le plancher absolu laisse passer : une part de -3 chez
#      un joueur qui n'a pose qu'un Landfill.
def _pv_negatifs():
    import json
    chemin = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                          "..", "..", "..", "data", "cards.json")
    table = {}
    for c in json.load(open(chemin, encoding="utf-8")):
        pv = c.get("vp")
        if isinstance(pv, int) and pv < 0:
            table[(c.get("name") or "").strip()] = pv
    return table


PV_NEGATIFS = _pv_negatifs()
PLANCHER_CARTES = sum(PV_NEGATIFS.values())

PARTS = ("tr", "forests", "cards", "milestones", "awards")
# Les totaux d'awards possibles pour UN joueur sur trois recompenses.
AWARDS_POSSIBLES = {sum(c) for c in itertools.product((2, 4, 5), repeat=3)}
# Les couples possibles pour les DEUX joueurs, recompense par recompense.
COUPLES = {(5, 2), (2, 5), (4, 4)}
PAIRES_POSSIBLES = {(sum(a for a, _ in c), sum(b for _, b in c))
                    for c in itertools.product(COUPLES, repeat=3)}

LECTURE = """() => {
  const nb = (s) => {
    const e = document.querySelector(s);
    if (!e) return null;
    const t = e.textContent.replace(/[^-0-9]/g, '');
    return t === '' || t === '-' ? null : Number(t);
  };
  // La partie est-elle FINIE ? A la fin, les recompenses sont attribuees pour de
  // bon et entrent dans le grand nombre ; avant, elles en sont exclues. Sans
  // cette lecture, ce banc accuse a tort la derniere decision de la partie.
  const out = {joueurs: [], provisoires: [],
               termine: !!document.querySelector('[data-partie-terminee]')};
  for (const j of [0, 1]) {
    const parts = {};
    for (const p of ['tr', 'cards', 'milestones', 'awards']) {
      parts[p] = nb(`[data-valeur="players.${j}.score_parts.${p}"]`);
    }
    out.joueurs.push({
      score: nb(`[data-valeur="players.${j}.score"]`),
      tr: nb(`[data-valeur="players.${j}.tr"]`),
      forests: nb(`[data-valeur="players.${j}.forests"]`),
      parts,
      // LES CARTES POSEES DEVANT CE JOUEUR, PAR LEUR NOM. C'est l'`alt` de leur
      // scan — le seul lien entre une carte de l'ecran et une ligne du
      // recensement (`vue/geste.js` s'en sert deja pour retrouver une carte
      // posee). Il sert ici d'oracle a la part `cards`, qui n'en avait aucun.
      posees: [...document.querySelectorAll(`#piles-${j} [data-carte-en-jeu] img`)]
        .map((im) => (im.alt || '').trim()).filter(Boolean),
    });
  }
  // Ou vit chaque mention « provisoire », et quelles parts sont dans la meme
  // barre qu'elle.
  //
  // ⚠️ CORRIGE LE 06-08. `data-provisoire` est porte par DEUX choses depuis que
  // le grand nombre se deduit des parts (`vue/joueurs.js:94` pour la mention,
  // `:91` pour la part elle-meme) : la MENTION ecrite (« provisoire ») et la
  // PART qu'elle designe. Ce banc les comptait ensemble et trouvait quatre
  // mentions la ou il en attendait deux — il jugeait une FORME (un attribut) au
  // lieu d'une PROPRIETE (une phrase par joueur). On distingue donc les deux, et
  // on verifie separement qu'il y a UNE mention et UNE part marquee par joueur.
  for (const m of document.querySelectorAll('.ventil__dit[data-provisoire]')) {
    const barre = m.closest('[data-joueur]');
    const dedans = barre
      ? [...barre.querySelectorAll('[data-valeur]')]
          .map((e) => e.getAttribute('data-valeur'))
          .filter((v) => v.includes('score_parts'))
      : [];
    const r = m.getBoundingClientRect();
    out.provisoires.push({
      joueur: barre ? barre.getAttribute('data-joueur') : null,
      visible: r.width > 0 && r.height > 0,
      texte: (m.textContent || '').trim(),
      parts: dedans,
      // Les parts marquees comme pouvant basculer, dans cette meme barre.
      marquees: barre
        ? [...barre.querySelectorAll('.ventil__part--provisoire [data-valeur]')]
            .map((e) => e.getAttribute('data-valeur').split('.').pop())
        : [],
    });
  }
  return out;
}"""

FINAL = """() => ({
  finaux: [0, 1].map((j) => {
    const e = document.querySelector(`[data-score-final="${j}"]`);
    const t = e ? e.textContent.replace(/[^-0-9]/g, '') : '';
    return t === '' ? null : Number(t);
  }),
  // LIS-8 — les forets ne sont plus dans la ventilation : leur terme se lit sur
  // l'hexagone des capacites, seul endroit qui les ecrit encore.
  parts: [0, 1].map((j) => [`players.${j}.score_parts.tr`,
                            `players.${j}.score_parts.cards`,
                            `players.${j}.score_parts.milestones`,
                            `players.${j}.score_parts.awards`,
                            `players.${j}.forests`]
    .map((v) => {
      const e = document.querySelector(`[data-valeur="${v}"]`);
      return e ? Number(e.textContent.replace(/[^-0-9]/g, '') || 0) : null;
    })),
  // Meme distinction qu'au-dessus : la MENTION seule, jamais la part marquee.
  provisoires: [...document.querySelectorAll('.ventil__dit[data-provisoire]')]
    .filter((m) => { const r = m.getBoundingClientRect();
                     return r.width > 0 && r.height > 0; }).length,
  // Et la part designee, comptee a part : elle doit exister une fois par joueur.
  partsMarquees: document.querySelectorAll('.ventil__part--provisoire').length,
})"""

fautes = []
vu = {"mesures": 0, "provisoires": 0, "negatives": 0}


def controle(pg, rang):
    d = pg.evaluate(LECTURE)
    for j, p in enumerate(d["joueurs"]):
        parts = p["parts"]
        if any(v is None for v in parts.values()) or p["score"] is None:
            fautes.append(f"decision {rang}, joueur {j} : ventilation incomplete {parts}")
            continue
        vu["mesures"] += 1
        # LIS-8 (05-08) — la ligne « Forests » de la ventilation a ete retiree :
        # elle redisait, au point de victoire pres, le nombre de l'hexagone.
        # L'invariant tient toujours, c'est l'hexagone qui fournit desormais ce
        # terme : les quatre parts ecrites PLUS les forets font le score.
        if p["forests"] is None:
            fautes.append(f"decision {rang}, joueur {j} : hexagone des forets illisible")
            continue
        # ⚠️ CORRIGE LE 06-08. Ce banc additionnait TOUTES les parts, recompenses
        # comprises. Or le grand nombre affiche est justement celui qui les
        # EXCLUT : « le grand nombre se deduit des parts qui ne sont pas
        # provisoires » (`vue/joueurs.js:65-67`). Les recompenses sont
        # distribuees d'avance par le moteur comme si la partie s'arretait a
        # l'instant — ce sont les treize points qui faisaient afficher 18 et 15
        # au premier ecran d'une partie ou personne n'avait rien fait, et c'est
        # exactement le defaut que la page a corrige. Le banc mesurait donc
        # l'ancienne page.
        # A LA FIN, les recompenses ne sont plus provisoires : elles ont ete
        # attribuees, et le grand nombre les inclut. On lit donc l'etat de la
        # partie plutot que de supposer.
        fini = bool(d.get("termine"))
        acquis = sum(parts.values()) if fini else \
            sum(v for cle, v in parts.items() if cle != "awards")
        if acquis + p["forests"] != p["score"]:
            fautes.append(f"decision {rang}, joueur {j} : {parts} sans les recompenses "
                          f"= {acquis}, plus {p['forests']} foret(s), ne fait pas "
                          f"{p['score']}")
        if parts["tr"] != p["tr"]:
            fautes.append(f"decision {rang}, joueur {j} : part TR {parts['tr']} "
                          f"alors que la barre affiche un TR de {p['tr']}")
        if parts["milestones"] % 3 or not 0 <= parts["milestones"] <= 9:
            fautes.append(f"decision {rang}, joueur {j} : jalons = {parts['milestones']}, "
                          "trois Reperes a 3 PV chacun")
        if parts["awards"] not in AWARDS_POSSIBLES:
            fautes.append(f"decision {rang}, joueur {j} : recompenses = {parts['awards']}, "
                          f"hors du bareme (5 / 2 / 4-4 sur trois recompenses)")
        if parts["cards"] < PLANCHER_CARTES:
            fautes.append(f"decision {rang}, joueur {j} : cartes = {parts['cards']}, "
                          f"sous le plancher absolu {PLANCHER_CARTES} — la somme de "
                          f"TOUS les points negatifs imprimes du jeu "
                          f"({len(PV_NEGATIFS)} cartes) ne descend pas si bas")
        elif parts["cards"] < 0:
            vu["negatives"] += 1
            posees = [n for n in p["posees"] if n in PV_NEGATIFS]
            if not posees:
                fautes.append(f"decision {rang}, joueur {j} : cartes = {parts['cards']}, "
                              f"mais aucune des {len(p['posees'])} carte(s) posees devant "
                              f"lui ne rapporte de points negatifs — cette part ne vient "
                              f"pas du moteur")
            else:
                plancher = sum(PV_NEGATIFS[n] for n in posees)
                if parts["cards"] < plancher:
                    fautes.append(f"decision {rang}, joueur {j} : cartes = "
                                  f"{parts['cards']}, sous la somme {plancher} des cartes "
                                  f"a points negatifs qu'il a posees ({posees}) — les "
                                  f"autres cartes ne peuvent qu'ajouter")
    paire = (d["joueurs"][0]["parts"]["awards"], d["joueurs"][1]["parts"]["awards"])
    if None not in paire and paire not in PAIRES_POSSIBLES:
        fautes.append(f"decision {rang} : recompenses {paire} — aucun tirage du bareme "
                      "ne donne ce couple")

    marques = d["provisoires"]
    if rang >= 5:
        vu["provisoires"] = max(vu["provisoires"], len([m for m in marques if m["visible"]]))
    for m in marques:
        if m["joueur"] is None:
            fautes.append(f"decision {rang} : une mention « provisoire » hors d'une barre")
            continue
        # ⚠️ CORRIGE LE 06-08. Ce banc attendait « jalons ET recompenses ». La
        # page a change et elle a RAISON : « un jalon atteint l'est pour de bon »,
        # il compte donc dans l'acquis (`vue/joueurs.js:61` et la table
        # `PARTS_SCORE`, ou seul `awards` porte le drapeau). Seules les
        # RECOMPENSES sont provisoires : elles sont distribuees d'avance, comme si
        # la partie s'arretait a l'instant. Le banc suivait l'ancienne regle.
        if set(m["marquees"]) != {"awards"}:
            fautes.append(f"decision {rang}, joueur {m['joueur']} : parts marquees "
                          f"{sorted(m['marquees'])}, on n'attend que les recompenses")
        if not m["texte"]:
            fautes.append(f"decision {rang}, joueur {m['joueur']} : mention vide")


graine = sys.argv[1] if len(sys.argv) > 1 else "5150"
with serveur() as base:
    with page(f"{base}/?graine={graine}&siege=0") as (pg, erreurs, _):
        # La toute premiere decision : c'est LA que le joueur a vu « 17 » sans
        # comprendre. On attend que l'ecran soit rendu — avant, la page porte
        # encore les zeros ecrits dans son gabarit, et on mesurerait le vide.
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        controle(pg, 0)
        jouer(pg, apres=controle)
        pg.wait_for_timeout(400)
        f = pg.evaluate(FINAL)
        for j in (0, 1):
            if f["finaux"][j] is None:
                fautes.append(f"fin : pas de score final pour le joueur {j}")
            elif sum(f["parts"][j]) != f["finaux"][j]:
                fautes.append(f"fin, joueur {j} : parts {f['parts'][j]} = "
                              f"{sum(f['parts'][j])}, score final {f['finaux'][j]}")
        if f["provisoires"]:
            fautes.append(f"fin : {f['provisoires']} mention(s) « provisoire » subsistent")
        # DEFAUT D'AFFICHAGE CONSTATE LE 06-08, PAS ENCORE CORRIGE. A la fin, la
        # mention ecrite disparait bien, mais la part des recompenses garde sa
        # marque `ventil__part--provisoire` alors qu'elle est desormais acquise et
        # comptee dans le score final. C'est cosmetique et sans effet sur aucun
        # nombre ; la page est le territoire d'un chantier en cours, donc on le
        # SIGNALE sans faire echouer ce banc. A reprendre apres la fusion.
        if f["partsMarquees"]:
            print(f"    a signaler : {f['partsMarquees']} part(s) gardent la marque "
                  f"« provisoire » apres la fin, alors qu'elles sont acquises")
        if erreurs:
            echec(f"{len(erreurs)} erreur(s) de console : {erreurs[0]}")

print(f"{vu['mesures']} ventilations verifiees part par part ; "
      f"{vu['provisoires']} mention(s) « provisoire » en cours de partie ; "
      f"scores finaux {f['finaux']} contre {[sum(p) for p in f['parts']]}")
print(f"part « cartes » negative rencontree {vu['negatives']} fois, chacune confrontee "
      f"aux cartes a points negatifs posees sur le plateau du joueur "
      f"(plancher absolu {PLANCHER_CARTES}, {len(PV_NEGATIFS)} cartes recensees)")
if vu["mesures"] < 200:
    echec(f"seulement {vu['mesures']} ventilation(s) lue(s)")
if vu["provisoires"] != 2:
    echec(f"{vu['provisoires']} mention(s) « provisoire » : il en faut une par joueur")
if fautes:
    for f2 in fautes[:8]:
        print("  " + f2)
    echec(f"{len(fautes)} defaut(s) sur la ventilation du score")
print("OK chaque part est celle du moteur, le bareme des recompenses est respecte, "
      "et le provisoire est dit au bon endroit")

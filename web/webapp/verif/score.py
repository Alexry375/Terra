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

    python3 outputs/web/webapp/verif/score.py [graine]
"""
import itertools
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer, echec  # noqa: E402

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
  const out = {joueurs: [], provisoires: []};
  for (const j of [0, 1]) {
    const parts = {};
    for (const p of ['tr', 'forests', 'cards', 'milestones', 'awards']) {
      parts[p] = nb(`[data-valeur="players.${j}.score_parts.${p}"]`);
    }
    out.joueurs.push({
      score: nb(`[data-valeur="players.${j}.score"]`),
      tr: nb(`[data-valeur="players.${j}.tr"]`),
      forests: nb(`[data-valeur="players.${j}.forests"]`),
      parts,
    });
  }
  // Ou vit chaque mention « provisoire », et quelles parts sont dans la meme
  // barre qu'elle.
  for (const m of document.querySelectorAll('[data-provisoire]')) {
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
  parts: [0, 1].map((j) => ['tr', 'forests', 'cards', 'milestones', 'awards']
    .map((p) => {
      const e = document.querySelector(`[data-valeur="players.${j}.score_parts.${p}"]`);
      return e ? Number(e.textContent.replace(/[^-0-9]/g, '') || 0) : null;
    })),
  provisoires: [...document.querySelectorAll('[data-provisoire]')]
    .filter((m) => { const r = m.getBoundingClientRect();
                     return r.width > 0 && r.height > 0; }).length,
})"""

fautes = []
vu = {"mesures": 0, "provisoires": 0}


def controle(pg, rang):
    d = pg.evaluate(LECTURE)
    for j, p in enumerate(d["joueurs"]):
        parts = p["parts"]
        if any(v is None for v in parts.values()) or p["score"] is None:
            fautes.append(f"decision {rang}, joueur {j} : ventilation incomplete {parts}")
            continue
        vu["mesures"] += 1
        if sum(parts.values()) != p["score"]:
            fautes.append(f"decision {rang}, joueur {j} : {parts} ne fait pas {p['score']}")
        if parts["tr"] != p["tr"]:
            fautes.append(f"decision {rang}, joueur {j} : part TR {parts['tr']} "
                          f"alors que la barre affiche un TR de {p['tr']}")
        if parts["forests"] != p["forests"]:
            fautes.append(f"decision {rang}, joueur {j} : part forets {parts['forests']} "
                          f"alors que la barre affiche {p['forests']} foret(s)")
        if parts["milestones"] % 3 or not 0 <= parts["milestones"] <= 9:
            fautes.append(f"decision {rang}, joueur {j} : jalons = {parts['milestones']}, "
                          "trois Reperes a 3 PV chacun")
        if parts["awards"] not in AWARDS_POSSIBLES:
            fautes.append(f"decision {rang}, joueur {j} : recompenses = {parts['awards']}, "
                          f"hors du bareme (5 / 2 / 4-4 sur trois recompenses)")
        if parts["cards"] < 0:
            fautes.append(f"decision {rang}, joueur {j} : cartes = {parts['cards']}")
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
        if set(m["marquees"]) != {"milestones", "awards"}:
            fautes.append(f"decision {rang}, joueur {m['joueur']} : parts marquees "
                          f"{sorted(m['marquees'])}, on attend jalons et recompenses")
        if not m["texte"]:
            fautes.append(f"decision {rang}, joueur {m['joueur']} : mention vide")


graine = sys.argv[1] if len(sys.argv) > 1 else "5150"
with serveur("outputs/web/webapp") as base:
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
        if erreurs:
            echec(f"{len(erreurs)} erreur(s) de console : {erreurs[0]}")

print(f"{vu['mesures']} ventilations verifiees part par part ; "
      f"{vu['provisoires']} mention(s) « provisoire » en cours de partie ; "
      f"scores finaux {f['finaux']} contre {[sum(p) for p in f['parts']]}")
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

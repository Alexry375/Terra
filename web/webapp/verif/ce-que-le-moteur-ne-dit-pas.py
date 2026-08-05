#!/usr/bin/env python3
"""CE QUE LE MOTEUR PUBLIE EST-IL SOUS LES YEUX ? (MOT-10, MOT-14, MOT-15)

Les trois bancs du contrat mesurent la PUBLICATION : ils interrogent le moteur
par le pont, hors navigateur. Ils passent au vert sans qu'une seule ligne de
l'ecran ait bouge. Ce banc-ci mesure l'autre moitie — celle que le contrat
demande aussi : « montrer le badge sur la carte posee, pour les deux joueurs »,
« l'afficher quand on agrandit la carte ».

On ne compte donc pas des noeuds : un noeud present mais recouvert par une autre
carte ne se voit pas (c'est le defaut LIS-3, mesure le 04-08). On demande au
navigateur ce qu'il trouve REELLEMENT au point ou chaque chose est posee
(`elementFromPoint`), exactement comme `verif/ressources-visibles.py`.

Trois mesures, trois oracles disjoints :

  1. MOT-10 — la case « next income » existe pour LES DEUX joueurs, elle n'est
     recouverte par rien, et son nombre est superieur ou egal au TR affiche a
     cote (le revenu reel vaut `mc_prod + tr + derivee`, dont chaque terme est
     positif ou nul). Un champ cable sur le mauvais chemin de l'etat le ferait
     tomber : il afficherait la piste de base, plus petite que le TR des la
     deuxieme manche.

  2. MOT-14 — oracle disjoint, entierement dans l'ecran : on releve LA REPONSE
     donnee au point de decision `pick_joker_tag` (le badge du bouton clique, et
     le joueur qui repondait), puis on exige de retrouver ce badge parmi les
     jetons VISIBLES des cartes posees de ce joueur-la. Deux sources qui n'ont
     rien a voir : ce que le pilote a repondu, et ce que la page dessine.

  3. MOT-15 — on agrandit une carte porteuse de ressources et on exige que la
     carte agrandie dise ce qu'elles rapportent. Temoin dans l'autre sens : une
     carte SANS ressource ne doit rien annoncer — sans quoi le banc serait vert
     pour une page qui ecrit « 0 point » partout.

    python3 verif/ce-que-le-moteur-ne-dit-pas.py <racine-webapp> [graine]
"""
import os
import sys

RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "web/webapp")
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page, choix_simple, choix_montant  # noqa: E402

# --------------------------------------------------------------------------
# Ce que le navigateur voit REELLEMENT, au point ou la chose est posee.
# --------------------------------------------------------------------------
VISIBLE = """(sel) => {
  const out = [];
  for (const e of document.querySelectorAll(sel)) {
    const r = e.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) { out.push({texte: null, cache: 'taille nulle'}); continue; }
    let cache = null;
    for (const [x, y] of [[r.x + r.width * .5, r.y + r.height * .5],
                          [r.x + r.width * .3, r.y + r.height * .5],
                          [r.x + r.width * .7, r.y + r.height * .5]]) {
      const dessus = document.elementFromPoint(x, y);
      if (dessus === e || e.contains(dessus) || (dessus && dessus.contains(e))) continue;
      cache = dessus ? (dessus.className || dessus.tagName) : 'rien';
      break;
    }
    out.push({texte: (e.textContent || '').trim(), titre: e.getAttribute('title') || '',
              alt: (e.querySelector('img') || {}).alt || '',
              taille: Math.round(Math.min(r.width, r.height)), cache});
  }
  return out;
}"""

# Les jetons de badge joker VISIBLES, joueur par joueur. Le plateau d'un joueur
# porte son numero (`#piles-J`) : c'est ce qui rattache un jeton a son proprietaire.
JETONS = """() => {
  const out = [];
  for (const j of [0, 1]) {
    const z = document.querySelector('#piles-' + j);
    if (!z) continue;
    for (const e of z.querySelectorAll('.carte__joker')) {
      const r = e.getBoundingClientRect();
      const im = e.querySelector('img');
      const nom = (im && im.alt) || (e.textContent || '').trim();
      let cache = null;
      if (r.width <= 0 || r.height <= 0) cache = 'taille nulle';
      else {
        const dessus = document.elementFromPoint(r.x + r.width * .5, r.y + r.height * .5);
        if (!(dessus === e || e.contains(dessus) || (dessus && dessus.contains(e))))
          cache = dessus ? (dessus.className || dessus.tagName) : 'rien';
      }
      out.push({joueur: j, badge: nom.toUpperCase(),
                titre: (e.getAttribute('title') || '').toUpperCase(),
                taille: Math.round(Math.min(r.width, r.height)), cache});
    }
  }
  return out;
}"""

# La barre d'un joueur : le revenu reel annonce, et le TR affiche a cote.
BARRE = """(j) => {
  const n = (s) => {
    const e = document.querySelector(s);
    if (!e) return null;
    const t = (e.textContent || '').replace(/[^0-9-]/g, '');
    return t === '' ? null : Number(t);
  };
  return {reel: n(`[data-valeur="players.${j}.production.mc_reel"]`),
          tr: n(`[data-valeur="players.${j}.tr"]`),
          base: n(`[data-valeur="players.${j}.production.mc"]`)};
}"""

fautes = []
vu = {"decisions": 0, "choix_joker": 0, "jetons": 0, "loupes": 0, "temoins": 0,
      "revenus": 0}
# joueur -> liste des badges repondus au point de decision `pick_joker_tag`
repondus = {0: [], 1: []}


def erreur(m):
    fautes.append(m)


def controler_revenu(pg, rang):
    """MOT-10 : la case existe, elle se voit, et son nombre tient debout."""
    for j in (0, 1):
        lu = pg.evaluate(VISIBLE, f'[data-valeur="players.{j}.production.mc_reel"]')
        if not lu:
            erreur(f"decision {rang} : aucune case de revenu reel pour le joueur {j} "
                   f"— MOT-10 n'est publie que dans le moteur, pas a l'ecran")
            continue
        vu["revenus"] += 1
        e = lu[0]
        if e["cache"]:
            erreur(f"decision {rang} : le revenu reel du joueur {j} est recouvert "
                   f"par {e['cache']!r}")
        if e["taille"] < 6:
            erreur(f"decision {rang} : le revenu reel du joueur {j} ne fait que "
                   f"{e['taille']} px de cote")
        b = pg.evaluate(BARRE, j)
        if b["reel"] is None:
            erreur(f"decision {rang} : la case de revenu reel du joueur {j} ne "
                   f"contient aucun nombre")
        elif b["tr"] is not None and b["reel"] < b["tr"]:
            erreur(f"decision {rang} : joueur {j}, revenu annonce {b['reel']} "
                   f"INFERIEUR au TR affiche {b['tr']} — la case ne lit pas le bon "
                   f"champ (le revenu reel vaut mc_prod + tr + derivee)")
        elif b["base"] is not None and b["tr"] is not None and b["reel"] < b["base"] + b["tr"]:
            erreur(f"decision {rang} : joueur {j}, revenu annonce {b['reel']} "
                   f"inferieur a la piste de base {b['base']} plus le TR {b['tr']}")


def controler_jetons(pg, rang):
    """MOT-14 : les jetons dessines se voient, et se lisent."""
    for t in pg.evaluate(JETONS):
        vu["jetons"] += 1
        if t["cache"]:
            erreur(f"decision {rang} : le badge joker « {t['badge']} » du joueur "
                   f"{t['joueur']} est recouvert par {t['cache']!r} — il est dans le "
                   f"document sans etre sous les yeux")
        if t["taille"] < 8:
            erreur(f"decision {rang} : un badge joker du joueur {t['joueur']} ne fait "
                   f"que {t['taille']} px de cote — illisible")


def loupe_sur(pg, avec_ressources):
    """Agrandit une carte posee et rend ce que la carte agrandie annonce.

    `avec_ressources` choisit une carte qui porte des ressources, ou au
    contraire une qui n'en porte pas (le temoin). Rend None si aucune carte de
    cette sorte n'est posee.
    """
    cible = pg.evaluate("""(avec) => {
      const cartes = [...document.querySelectorAll('.pile .carte--jeu')];
      const voulue = cartes.filter((c) => !!c.querySelector('.carte__ressources') === avec);
      if (!voulue.length) return null;
      const c = voulue[0];
      const r = c.getBoundingClientRect();
      return {x: r.x + r.width * .5, y: r.y + r.height * .5,
              id: c.getAttribute('data-carte-en-jeu')};
    }""", avec_ressources)
    if not cible:
        return None
    # Le survol doit etre VOLONTAIRE (`loupe.js` ignore un curseur immobile) :
    # on bouge d'abord ailleurs, puis on entre sur la carte.
    pg.mouse.move(5, 5)
    pg.mouse.move(cible["x"], cible["y"])
    try:
        pg.wait_for_selector("#loupe.loupe--visible", timeout=4000)
    except Exception:
        return {"id": cible["id"], "ouverte": False, "pv": None}
    lu = pg.evaluate(VISIBLE, "#loupe .carte__pv")
    pv = None
    if lu and lu[0]["texte"]:
        chiffres = "".join(c for c in lu[0]["texte"] if c.isdigit() or c == "-")
        pv = int(chiffres) if chiffres else None
    return {"id": cible["id"], "ouverte": True, "pv": pv,
            "cache": lu[0]["cache"] if lu else None, "texte": lu[0]["texte"] if lu else None}


with serveur(RACINE) as base:
    with page(f"{base}/?graine={GRAINE}&siege=0&animations=non") as (pg, erreurs, _):
        pg.wait_for_selector("#horizon", timeout=20000)
        for _tour in range(2000):
            if pg.query_selector("[data-partie-terminee]"):
                break
            pg.wait_for_selector("[data-decision-rang]", timeout=15000, state="attached")
            porteur = pg.query_selector("[data-decision-rang]")
            rang = int(porteur.get_attribute("data-decision-rang"))
            forme = porteur.get_attribute("data-decision-forme") or "simple"
            type_ = porteur.get_attribute("data-type") or ""
            joueur = int(porteur.get_attribute("data-joueur") or 0)
            vu["decisions"] += 1

            # Le revenu reel se controle regulierement : a chaque decision ce
            # serait 600 lectures pour rien, jamais ce serait ne rien mesurer.
            if rang % 25 == 0:
                controler_revenu(pg, rang)
            controler_jetons(pg, rang)

            choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            if forme == "montant":
                champ = pg.wait_for_selector("[data-montant]", timeout=15000)
                mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
                champ.fill(str(choix_montant(rang, mini, maxi)))
                pg.click("[data-valider]")
            elif forme == "multiple":
                brut = porteur.get_attribute("data-a-choisir")
                k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
                k = min(k, len(choix))
                for c in choix[:k]:
                    c.click()
                pg.click("[data-valider]")
            else:
                if not choix:
                    raise RuntimeError(f"decision {rang} : aucun choix visible")
                bouton = choix[choix_simple(rang, len(choix))]
                # MOT-14, moitie « reponse » de l'oracle : le badge qu'on va
                # REELLEMENT choisir, lu sur le bouton qu'on s'apprete a cliquer.
                if type_ == "pick_joker_tag":
                    mot = (bouton.inner_text() or "").strip().upper()
                    badge = mot.split("(")[0].strip().split()[0] if mot else ""
                    if badge:
                        repondus[joueur].append(badge)
                        vu["choix_joker"] += 1
                bouton.click()
            pg.wait_for_function(
                "r => { const e = document.querySelector('[data-decision-rang]');"
                " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                " || document.querySelector('[data-partie-terminee]'); }",
                arg=rang, timeout=15000)

        # ------------------------------------------------------------------
        # MOT-14 : ce qui a ete REPONDU se retrouve-t-il sur la table ?
        # ------------------------------------------------------------------
        jetons = pg.evaluate(JETONS)
        for j in (0, 1):
            montres = [t["badge"] for t in jetons if t["joueur"] == j and not t["cache"]]
            titres = " | ".join(t["titre"] for t in jetons if t["joueur"] == j)
            for badge in repondus[j]:
                # Le jeton porte le nom d'affichage du badge (`nomBadge`) et son
                # `title` porte le nom du moteur : l'un des deux doit dire le
                # badge repondu. On ne compare donc pas deux fois la meme source.
                if not any(badge in m for m in montres) and badge not in titres:
                    erreur(f"joueur {j} : le badge « {badge} » a ete choisi au point "
                           f"de decision, aucun jeton VISIBLE ne le dit sur ses cartes "
                           f"posees (jetons vus : {montres or 'aucun'})")

        # ------------------------------------------------------------------
        # MOT-15 : la carte agrandie dit ce que ses ressources rapportent.
        # ------------------------------------------------------------------
        avec = loupe_sur(pg, True)
        if avec is None:
            erreur("aucune carte posee ne porte de ressources en fin de partie — "
                   "la mesure MOT-15 n'a pas eu lieu")
        elif not avec["ouverte"]:
            erreur(f"la loupe ne s'ouvre pas sur la carte {avec['id']}")
        else:
            vu["loupes"] += 1
            if avec["pv"] is None:
                erreur(f"la carte {avec['id']} porte des ressources, mais une fois "
                       f"AGRANDIE elle ne dit pas ce qu'elles rapportent — c'est la "
                       f"moitie visible de MOT-15 (texte lu : {avec['texte']!r})")
            elif avec["cache"]:
                erreur(f"la ligne des points de la carte {avec['id']} est recouverte "
                       f"par {avec['cache']!r}")

        # Temoin en sens inverse : une carte SANS ressource n'annonce rien.
        sans = loupe_sur(pg, False)
        if sans is not None and sans["ouverte"]:
            vu["temoins"] += 1
            if sans["pv"] is not None:
                erreur(f"la carte {sans['id']} ne porte AUCUNE ressource et annonce "
                       f"pourtant {sans['pv']} point(s) : la ligne s'affiche partout, "
                       f"elle ne mesure donc rien")

        for e in erreurs:
            erreur(f"la page a signale une erreur : {e}")

print(f"    graine {GRAINE} : {vu['decisions']} decisions, "
      f"{vu['revenus']} lectures du revenu reel, {vu['choix_joker']} badge(s) joker "
      f"choisi(s), {vu['jetons']} jeton(s) dessine(s), {vu['loupes']} carte(s) "
      f"agrandie(s) a ressources, {vu['temoins']} temoin(s) sans ressource")

# Un banc qui n'a rien mesure doit le DIRE, pas se declarer vert.
if vu["decisions"] < 50:
    print(f"KO {vu['decisions']} decisions seulement — la partie ne s'est pas jouee")
    sys.exit(1)
if vu["revenus"] < 2:
    print("KO le revenu reel n'a jamais ete lu — la mesure MOT-10 n'a pas eu lieu")
    sys.exit(1)
if vu["choix_joker"] == 0 and vu["jetons"] == 0:
    print("KO aucun badge joker dans cette partie — la mesure MOT-14 n'a pas eu lieu "
          "(essaie une autre graine)")
    sys.exit(1)
if vu["loupes"] == 0:
    print("KO aucune carte a ressources agrandie — la mesure MOT-15 n'a pas eu lieu")
    sys.exit(1)

if fautes:
    for f in fautes[:20]:
        print(f"KO {f}")
    print(f"KO {len(fautes)} faute(s)")
    sys.exit(1)
print("OK ce que le moteur publie est sous les yeux, des deux cotes de la table")

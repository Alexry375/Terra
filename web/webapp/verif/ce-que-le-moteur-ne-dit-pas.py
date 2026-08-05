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
# PLUSIEURS GRAINES, et on s'arrete des que les trois mesures ont eu lieu. Une
# partie ne contient pas forcement de carte a badge joker, ni de carte porteuse
# de ressources : une seule graine, et le banc serait vert pour n'avoir rien vu.
GRAINES = (sys.argv[2] if len(sys.argv) > 2 else "4242,7,5150").split(",")

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
      const c = e.closest('.carte--jeu');
      out.push({joueur: j, badge: nom.toUpperCase(),
                carte: c ? c.getAttribute('data-carte-en-jeu') : null,
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
      "revenus": 0, "eclipses": 0, "accords": 0, "derives": 0}
# joueur -> { identifiant de carte : badge repondu au point `pick_joker_tag` }.
# REMIS A ZERO A CHAQUE GRAINE : une reponse donnee dans une partie ne valide
# rien dans la suivante.
repondus = {0: {}, 1: {}}
# (joueur, carte) -> badge deja montre, pour verifier qu'il ne change jamais.
montres = {}
# Les dix familles de badges du moteur (`cards.rs`, JOKER_TAG_CHOICES).
FAMILLES = {"BUILDING", "SPACE", "SCIENCE", "PLANT", "MICROBE", "ANIMAL",
            "EARTH", "JUPITER", "ENERGY", "EVENT"}
graines_jouees = []


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
        elif b["base"] is not None and b["tr"] is not None and b["reel"] > b["base"] + b["tr"]:
            # LE CAS QUI COMPTE. Une page qui additionnerait ELLE-MEME la piste
            # de base et le TR — la « seconde addition a l'ecran » que le contrat
            # interdit — donnerait exactement `base + tr`, et les deux tests
            # ci-dessus la laisseraient passer. Voir au moins une fois un nombre
            # STRICTEMENT superieur, c'est voir la production derivee : elle ne
            # peut venir que du moteur.
            vu["derives"] += 1


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
        # LA PAGE N'INVENTE AUCUN BADGE. Sens de la comparaison : tout jeton
        # MONTRE doit correspondre a un badge REPONDU par ce joueur-la.
        #
        # L'inverse ne se tient pas, et c'est mesure : une carte a badge joker
        # peut etre resolue sans finir sur la table (banc 02 du contrat :
        # 18 choix pour 12 cartes posees), parce que le badge se choisit des la
        # MAIN (`flow::resolve_hand_jokers`).
        #
        # On ne compare QUE si ce joueur a effectivement repondu a un
        # `pick_joker_tag` sous nos yeux : sinon la comparaison serait vide et
        # se declarerait verte pour n'avoir rien vu. Le compteur `accords` dit
        # combien de fois elle a REELLEMENT eu lieu, et le bilan l'affiche.
        # Le badge montre est-il une famille REELLE du jeu ? Un libelle vide, ou
        # un mot qui n'est pas un badge, passerait sinon pour un jeton valide.
        if t["badge"] not in FAMILLES:
            erreur(f"decision {rang} : le joueur {t['joueur']} montre « {t['badge']} », "
                   f"qui n'est pas une famille de badges du jeu")
        # LE BADGE EST DEFINITIF (le moteur ne reecrit jamais une case occupee,
        # `flow::ensure_joker_tag`). Une carte qui change de badge sous l'oeil est
        # donc un defaut d'AFFICHAGE — et rien ne le mesurait.
        cle_carte = (t["joueur"], t["carte"])
        if t["carte"] is not None:
            if cle_carte in montres and montres[cle_carte] != t["badge"]:
                erreur(f"decision {rang} : la carte {t['carte']} du joueur "
                       f"{t['joueur']} montrait « {montres[cle_carte]} » et montre "
                       f"maintenant « {t['badge']} » — le badge est pourtant definitif")
            montres[cle_carte] = t["badge"]
        # LA PAGE N'INVENTE AUCUN BADGE : tout jeton montre doit correspondre a un
        # badge REPONDU par ce joueur-la, pour CETTE carte. On ne compare que si
        # la reponse a ete vue (voir le bilan : le compteur `accords` le dit).
        attendu = repondus[t["joueur"]].get(t["carte"])
        if attendu is not None:
            vu["accords"] += 1
            if t["badge"] != attendu:
                erreur(f"decision {rang} : la carte {t['carte']} du joueur "
                       f"{t['joueur']} montre « {t['badge']} » alors que la reponse "
                       f"donnee au point de decision etait « {attendu} »")


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
      const im = c.querySelector('img');
      return {x: r.x + r.width * .5, y: r.y + r.height * .5,
              id: c.getAttribute('data-carte-en-jeu'),
              nom: (im && im.alt) || ''};
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
    # LA LOUPE EST CLICK-THROUGH PAR CONSTRUCTION (`#loupe { pointer-events:
    # none }`, et c'est tout l'objet de `loupe.js` : elle ne doit jamais
    # recouvrir un choix cliquable). `elementFromPoint` y repond donc toujours
    # ce qui est DESSOUS — on l'a mesure : « recouverte par IMG », l'image d'une
    # carte de la table. Ce test-la ne veut rien dire ici.
    #
    # Ce qu'on exige a la place, et qui a un sens : la ligne a une boite non
    # vide, elle est DEDANS la carte agrandie, et la carte agrandie est dans
    # l'ecran. Une ligne de hauteur nulle, ou posee hors du cadre, ne se lit pas.
    agrandi = pg.evaluate("() => (document.getElementById('loupe')||{}).dataset"
                          " ? document.getElementById('loupe').dataset.agrandi : null")
    lu = pg.evaluate("""() => {
      const l = document.getElementById('loupe');
      const e = l && l.querySelector('.carte__pv');
      if (!e) return null;
      const r = e.getBoundingClientRect(), R = l.getBoundingClientRect();
      return {texte: (e.textContent || '').trim(),
              haut: Math.round(r.height), large: Math.round(r.width),
              dedans: r.top >= R.top - 1 && r.bottom <= R.bottom + 1
                   && r.left >= R.left - 1 && r.right <= R.right + 1,
              ecran: R.top >= 0 && R.left >= 0
                  && R.bottom <= window.innerHeight && R.right <= window.innerWidth};
    }""")
    pv = None
    if lu and lu["texte"]:
        chiffres = "".join(c for c in lu["texte"] if c.isdigit() or c == "-")
        pv = int(chiffres) if chiffres else None
    illisible = None
    if lu:
        if lu["haut"] < 6 or lu["large"] < 10:
            illisible = f"boite de {lu['large']}x{lu['haut']} px"
        elif not lu["dedans"]:
            illisible = "posee HORS de la carte agrandie"
        elif not lu["ecran"]:
            illisible = "la carte agrandie sort de l'ecran"
    # `loupe.js` DECLARE ce qu'elle agrandit (`#loupe[data-agrandi]`), justement
    # « pour qu'un controle exterieur puisse verifier que c'est bien la carte
    # survolee, et pas une autre ». On le verifie : sans cela, on mesurerait une
    # carte au hasard en croyant mesurer celle qu'on a designee.
    if cible["nom"] and agrandi and agrandi != cible["nom"]:
        illisible = (f"la loupe declare agrandir « {agrandi} » alors qu'on a "
                     f"survole « {cible['nom']} »")
    return {"id": cible["id"], "ouverte": True, "pv": pv, "agrandi": agrandi,
            "nom": cible["nom"], "cache": illisible,
            "texte": lu["texte"] if lu else None}


with serveur(RACINE) as base:
  for GRAINE in GRAINES:
    # On s'arrete des que TOUT a ete mesure : inutile de jouer une partie de
    # plus, et le banc dit ce qu'il a vu plutot que de compter les graines.
    if vu["jetons"] and vu["loupes"] and vu["temoins"] and vu["derives"]:
        break
    with page(f"{base}/?graine={GRAINE}&siege=0&animations=non") as (pg, erreurs, _):
        pg.wait_for_selector("#horizon", timeout=20000)
        graines_jouees.append(GRAINE)
        repondus[0].clear()
        repondus[1].clear()
        montres.clear()
        loupe_faite = False
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

            # LA PAGE POSE PARFOIS LE CHOIX EN GRAND PAR-DESSUS LA TABLE
            # (`#scene[data-mode="superposition"]` : remplacement des cartes de
            # depart, choix de la corporation). A cet instant-la, les deux
            # plateaux sont recouverts A DESSEIN — mesurer la visibilite d'un
            # jeton de carte posee n'aurait aucun sens. On compte ces instants
            # plutot que de les taire : un banc qui les sauterait TOUS ne
            # mesurerait plus rien, et on le verrait au compteur.
            eclipse = bool(pg.query_selector('#scene[data-mode="superposition"]'))
            if eclipse:
                vu["eclipses"] += 1

            # Le revenu reel se controle regulierement : a chaque decision ce
            # serait des centaines de lectures pour rien ; jamais, ce serait ne
            # rien mesurer. Un sur huit : assez serre pour attraper le cas qui
            # compte — celui ou la production DERIVEE fait depasser la piste de
            # base plus le TR —, qu'un echantillon d'un sur vingt-cinq ratait.
            if rang % 8 == 0 and not eclipse:
                controler_revenu(pg, rang)
            if not eclipse:
                controler_jetons(pg, rang)

            # MOT-15 se mesure DES QU'une carte posee porte des ressources —
            # pas a la fin : une partie peut finir sans qu'il en reste une.
            if not loupe_faite and not eclipse:
                avec = loupe_sur(pg, True)
                if avec is not None:
                    loupe_faite = True
                    if not avec["ouverte"]:
                        erreur(f"graine {GRAINE} : la loupe ne s'ouvre pas sur la "
                               f"carte {avec['id']}")
                    else:
                        vu["loupes"] += 1
                        if avec["pv"] is None:
                            erreur(f"graine {GRAINE} : la carte {avec['id']} porte des "
                                   f"ressources, mais une fois AGRANDIE elle ne dit pas "
                                   f"ce qu'elles rapportent — c'est la moitie visible "
                                   f"de MOT-15 (texte lu : {avec['texte']!r})")
                        elif avec["cache"]:
                            erreur(f"graine {GRAINE} : la ligne des points de la carte "
                                   f"{avec['id']} est recouverte par {avec['cache']!r}")
                    # Temoin en sens inverse, au meme instant : une carte SANS
                    # ressource ne doit RIEN annoncer. Sans lui, une page qui
                    # ecrirait « 0 point » partout passerait pour juste.
                    sans = loupe_sur(pg, False)
                    if sans is not None and sans["ouverte"]:
                        vu["temoins"] += 1
                        if sans["pv"] is not None:
                            erreur(f"graine {GRAINE} : la carte {sans['id']} ne porte "
                                   f"AUCUNE ressource et annonce pourtant {sans['pv']} "
                                   f"point(s) : la ligne s'affiche partout, elle ne "
                                   f"mesure donc rien")

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
                        # La carte concernee, telle que la page la nomme dans la
                        # question. Sans elle on ne peut pas lier la reponse a
                        # une carte posee : on compte le choix, mais l'accord ne
                        # se fera pas, et le bilan le dira.
                        idc = pg.evaluate("""() => {
                          const s = document.getElementById('scene');
                          const e = s && (s.querySelector('[data-carte-en-jeu]')
                                       || s.querySelector('[data-prix-carte]'));
                          return e ? (e.getAttribute('data-carte-en-jeu')
                                   || e.getAttribute('data-prix-carte')) : null;
                        }""")
                        if idc is not None:
                            repondus[joueur][idc] = badge
                        vu["choix_joker"] += 1
                bouton.click()
            pg.wait_for_function(
                "r => { const e = document.querySelector('[data-decision-rang]');"
                " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                " || document.querySelector('[data-partie-terminee]'); }",
                arg=rang, timeout=15000)

        for e in erreurs:
            erreur(f"graine {GRAINE} : la page a signale une erreur : {e}")

print(f"    graines {'+'.join(graines_jouees)} : {vu['decisions']} decisions, "
      f"{vu['revenus']} lectures du revenu reel, {vu['choix_joker']} badge(s) joker "
      f"choisi(s), {vu['jetons']} jeton(s) dessine(s), {vu['loupes']} carte(s) "
      f"agrandie(s) a ressources, {vu['temoins']} temoin(s) sans ressource, "
      f"{vu['eclipses']} instant(s) ou la table est volontairement recouverte")
# CE QUE CE BANC N'A PAS MESURE SE DIT. L'accord « badge montre = badge choisi »
# ne peut se verifier que si un `pick_joker_tag` s'est presente sous nos yeux :
# mesure, une partie de navigateur entiere (graine 6, 180 decisions) montre un
# jeton sans qu'aucun `pick_joker_tag` ne soit jamais pose. L'accord lui-meme est
# deja prouve a la publication par le banc 02 du contrat ; ici, ce qui est
# mesure, c'est que le jeton SE VOIT.
print(f"    revenu annonce SUPERIEUR a la piste de base plus le TR : {vu['derives']} "
      f"lecture(s) — c'est la production derivee, elle ne peut venir que du moteur")
if vu["accords"]:
    print(f"    accord badge montre / badge choisi : {vu['accords']} confrontation(s)")
else:
    print("    accord badge montre / badge choisi : NON MESURE ici — aucun "
          "`pick_joker_tag` ne s'est presente (le banc 02 du contrat le prouve, lui, "
          "a la publication)")

# Un banc qui n'a rien mesure doit le DIRE, pas se declarer vert.
if vu["decisions"] < 50:
    print(f"KO {vu['decisions']} decisions seulement — la partie ne s'est pas jouee")
    sys.exit(1)
if vu["revenus"] < 2:
    print("KO le revenu reel n'a jamais ete lu — la mesure MOT-10 n'a pas eu lieu")
    sys.exit(1)
if vu["jetons"] == 0:
    print("KO aucun jeton de badge joker dessine — la mesure MOT-14 n'a pas eu lieu "
          "(donne d'autres graines en second argument)")
    sys.exit(1)
if vu["loupes"] == 0:
    print("KO aucune carte a ressources agrandie — la mesure MOT-15 n'a pas eu lieu")
    sys.exit(1)
if vu["temoins"] == 0:
    print("KO aucun temoin sans ressource — le banc ne peut pas distinguer une page "
          "qui compte d'une page qui ecrit « 0 » partout")
    sys.exit(1)
# LE CAS QUI DISTINGUE UNE LECTURE D'UNE ADDITION FAITE A L'ECRAN. Sans une seule
# lecture ou `mc_reel > piste de base + TR`, ce banc serait vert pour une page qui
# additionnerait elle-meme les deux nombres publies — le raccourci que le contrat
# interdit nommement. Il faut donc l'avoir VU.
if vu["derives"] == 0:
    print("KO le revenu annonce n'a jamais depasse la piste de base plus le TR — "
          "ce banc ne peut pas distinguer un nombre venu du moteur d'une addition "
          "faite a l'ecran (donne d'autres graines en second argument)")
    sys.exit(1)
# Des reponses relevees mais jamais confrontees = une mesure annoncee qui n'a pas
# eu lieu. On ne sort pas vert la-dessus.
if vu["choix_joker"] and vu["accords"] == 0:
    print(f"KO {vu['choix_joker']} badge(s) joker choisi(s) sous nos yeux, et AUCUNE "
          f"confrontation avec un jeton montre — l'accord annonce n'a pas ete mesure")
    sys.exit(1)

if fautes:
    for f in fautes[:20]:
        print(f"KO {f}")
    print(f"KO {len(fautes)} faute(s)")
    sys.exit(1)
print("OK ce que le moteur publie est sous les yeux, des deux cotes de la table")

#!/usr/bin/env python3
"""CE QUE LES SIX CONTROLES LIVRES NE VOIENT PAS.

Trois trous, tous mesures ici sur une partie ENTIERE :

  1. Aucun controle ne joue en `?siege=1`. C'est pourtant le seul siege ou le
     moteur interroge l'adversaire EN PREMIER : tout ce qui s'affiche entre sa
     reponse et la mienne est une fuite. Le controle 04, lui, compare l'ecran a
     `data-valeur="players.N.chosen_phase"` — la meme donnee remanente : les
     deux peuvent mentir ensemble et rester verts.

  2. « Celle en cours est allumee » n'est verifie par le controle 04 qu'une
     fois pour toute la partie (`allumee_vue > 0`). On mesure ici la PART des
     decisions ou une carte Phase est montree sans qu'aucune ne soit allumee,
     hors planification.

  3. La zone adverse est verifiee muette au siege 0 seulement (controle 01).

  4. LA BARRE D'EQUIPAGE PORTE `players.N.chosen_phase` EN CLAIR. Le controle 04
     s'en sert d'oracle : il ne peut donc pas voir qu'elle fuit. Vue du siege 1,
     l'adversaire choisit sa phase EN PREMIER — le nombre affiche en face
     donnait sa carte avant que je pose la mienne, a chaque manche. On exige
     ici 0 en face pendant toute la planification, et on compte separement les
     ecrans ou ce nombre PROUVE la fuite (il a change depuis la fin de la manche
     precedente : la remanence ne l'explique pas).

  5. LA CORPORATION D'EN FACE, MEME PIEGE, AUTRE MOMENT. Le livret (l. 211)
     distribue les Corporations FACE CACHEE et l'etape 9 (l. 215) les revele
     ensemble ; le moteur, lui, installe celle du joueur 0 des qu'il a repondu
     puis interroge le joueur 1. Vu du siege 1, le nom d'en face etait donc
     lisible pendant que je choisissais la mienne (mesure : `?graine=5150&
     boites=base`, rang 5, « Teractor Corporation » — via `data-corpo`, via
     l'`alt` de l'image et via le nom de fichier du scan). L'occasion n'arrive
     qu'UNE fois par partie : aucun controle livre ne la regarde. On garde ici
     la page ENTIERE a chaque decision tant que je n'ai pas choisi la mienne,
     et on y cherche, une fois qu'on la connait, la corporation d'en face.

Depuis la racine du workspace :  python3 outputs/web/webapp/verif/cadre.py [graine]
"""
import json
import sys

# Le module de pilotage vit a cote de ce banc dans le depot (il venait de
# `inputs/checks/` du chantier). On l'importe par le chemin de CE fichier, pour
# que le banc tourne depuis n'importe quel repertoire courant.
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import serveur, page, choix_simple, choix_montant, echec

LECTURE = """
() => {
  const montrees = [...document.querySelectorAll('[data-phase-choisie]')];
  const z = document.querySelector('[data-main="adverse"]');
  return {
    montrees: montrees.map(e => Number(e.getAttribute('data-phase-choisie'))),
    allumees: montrees.filter(e => e.getAttribute('data-phase-en-cours') === 'oui').length,
    annonce: document.querySelector('.annonce__phases') !== null
             && document.getElementById('annonce').classList.contains('annonce--vive'),
    adverse: z ? z.innerHTML : null,
    cartes: z ? z.getAttribute('data-cartes') : null,
    mienne: document.querySelectorAll('[data-main="mienne"] [data-carte-id]').length,
    // Ce que la barre d'equipage AFFICHE de la phase choisie, siege par siege.
    // C'est le nombre qu'un joueur assis devant l'ecran peut lire, pas l'etat.
    phaseAffichee: [0, 1].map(j => {
      const e = document.querySelector(`[data-valeur="players.${j}.chosen_phase"]`);
      const t = e ? e.textContent.replace(/[^0-9]/g, '') : '';
      return t ? Number(t) : 0;
    }),
    // La corporation de chaque siege, telle que la page la porte, et la page
    // ENTIERE : c'est en elle qu'on cherchera le nom d'en face, pas seulement
    // dans la zone prevue pour lui.
    corpo: [0, 1].map(j => {
      const e = document.querySelector('#corpo-carte-' + j);
      return e ? (e.getAttribute('data-corpo') || '') : '';
    }),
    // Le scan porte le nom une SECONDE fois, sous forme de nom de fichier
    // (« teractor-corporation.webp ») : chercher le nom en clair ne suffit pas.
    corpoSrc: [0, 1].map(j => {
      const im = document.querySelector('#corpo-carte-' + j + ' img');
      return im ? (im.getAttribute('src') || '') : '';
    }),
    tout: document.body.innerHTML,
  };
}
"""

def jouer_en_regardant(pg, regarder, maximum=3000, delai=30000):
    """Joue la partie en REGARDANT AVANT DE REPONDRE.

    Le `apres=` du pilote livre est appele une fois la reponse donnee : la scene
    est deja refermee, `data-decision-rang` retire, et l'ecran observe est celui
    de la decision SUIVANTE. Pour juger ce que la page montre PENDANT qu'elle
    pose une question, il faut la regarder avant de cliquer — d'ou cette boucle.
    Les choix sont ceux du pilote livre, a l'identique, pour que la partie soit
    la meme.

    On attend `[data-decision-rang]` OU `[data-partie-terminee]` : la partie peut
    s'achever pendant le tour de l'adversaire, et alors aucune question ne vient.
    """
    n = 0
    for _ in range(maximum):
        pg.wait_for_selector("[data-decision-rang],[data-partie-terminee]",
                             timeout=delai, state="attached")
        if pg.query_selector("[data-partie-terminee]"):
            return n
        porteur = pg.query_selector("[data-decision-rang]")
        rang = int(porteur.get_attribute("data-decision-rang"))
        forme = porteur.get_attribute("data-decision-forme") or "simple"
        n += 1
        regarder(pg, rang, porteur.get_attribute("data-type"))
        visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
        if forme == "montant":
            champ = pg.wait_for_selector("[data-montant]", timeout=delai)
            mini = int(champ.get_attribute("min"))
            maxi = int(champ.get_attribute("max"))
            champ.fill(str(choix_montant(rang, mini, maxi)))
            pg.click("[data-valider]")
        elif forme == "multiple":
            brut = porteur.get_attribute("data-a-choisir")
            k = int(brut) if (brut or "").isdigit() else (rang % max(len(visibles), 1)) + 1
            for c in visibles[:min(k, len(visibles))]:
                c.click()
            pg.click("[data-valider]")
        else:
            if not visibles:
                echec(f"decision {rang} : aucun choix visible")
            visibles[choix_simple(rang, len(visibles))].click()
        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=delai)
    echec(f"la partie n'est pas terminee apres {maximum} decisions")


graine = sys.argv[1] if len(sys.argv) > 1 else "1515"

noms = set()
for c in json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),
                            "..", "..", "..", "data", "cards.json"))):
    n = (c.get("name") or "").strip()
    if len(n) >= 6:
        noms.add(n)

SIEGE = 1          # le siege regarde : celui que le moteur interroge en SECOND
EN_FACE = 1 - SIEGE

fautes = []
eteintes_par_type = {}
vides = []
vu = {"decisions": 0, "montrees": 0, "eteintes": 0, "planif": 0, "mainVide": 0,
      "barreMuette": 0, "fuiteProuvee": 0}
# La derniere phase adverse LUE hors planification : c'est la valeur de fin de
# manche precedente. `chosen_phase` etant remanent, un nombre affiche pendant la
# planification ne prouve la fuite que s'il a change depuis celle-la.
fin_de_manche = 0

# La fenetre a risque des corporations : toutes les decisions posees AVANT que
# j'aie choisi la mienne. On y garde la page entiere pour pouvoir la relire une
# fois le nom d'en face connu — il n'est evidemment pas connu sur le moment,
# c'est tout le sujet. Une poignee de decisions, une seule fois par partie.
avant_ma_corpo = []      # [(rang, type, html)]
corpo_en_face = ""       # decouverte APRES coup, quand elle devient publique
corpo_src_en_face = ""   # son scan : le nom de fichier la nomme aussi
corpo_apres = None       # ce que la page montre d'en face juste apres mon choix

with serveur(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")) as base:
    # SIEGE 1 : l'adversaire (le joueur 0) est interroge en premier a chaque
    # question simultanee. C'est la configuration a risque.
    with page(f"{base}/?graine={graine}&siege=1") as (pg, erreurs, externes):

        def controle(p, rang, type_):
            global fin_de_manche, corpo_en_face, corpo_src_en_face, corpo_apres
            m = p.evaluate(LECTURE)
            m["type"] = type_
            vu["decisions"] += 1

            # 5. LA CORPORATION D'EN FACE AVANT LA MIENNE. Tant que ma case est
            #    vide, je n'ai pas choisi : rien de celle d'en face ne doit se
            #    trouver dans la page. On garde la page pour la relire plus tard.
            tout = m.pop("tout")
            if not m["corpo"][SIEGE]:
                avant_ma_corpo.append((rang, type_, tout))
            else:
                if corpo_apres is None:
                    corpo_apres = m["corpo"][EN_FACE]
                if m["corpo"][EN_FACE]:
                    corpo_en_face = m["corpo"][EN_FACE]
                    corpo_src_en_face = m["corpoSrc"][EN_FACE]

            # 0. LA BARRE D'EQUIPAGE NE DOIT PAS DEVANCER LA REVELATION. Tant
            #    que la planification dure, la case Phase d'en face montre 0,
            #    comme avant tout choix ; elle redevient publique ensuite. Le
            #    marqueur `data-valeur`, lui, reste pose (contrat l. 166).
            lue = m["phaseAffichee"][EN_FACE]
            if m["type"] == "pick_phase":
                if lue == 0:
                    vu["barreMuette"] += 1
                else:
                    if lue != fin_de_manche:
                        # Preuve : ce nombre n'est pas la remanence de la manche
                        # precedente, c'est le choix que l'adversaire vient de faire.
                        vu["fuiteProuvee"] += 1
                    fautes.append(
                        f"decision {rang} : la barre d'equipage affiche la phase {lue} "
                        f"du joueur {EN_FACE} pendant la planification "
                        f"(fin de manche precedente : {fin_de_manche})")
            else:
                fin_de_manche = lue

            # 1. Rien des phases pendant la planification : ni bande, ni annonce.
            if m["type"] == "pick_phase":
                vu["planif"] += 1
                if m["montrees"]:
                    fautes.append(
                        f"decision {rang} : la bande montre {m['montrees']} alors que la "
                        "planification est en cours (la carte adverse est posee face cachee)")
                if m["annonce"]:
                    fautes.append(
                        f"decision {rang} : la revelation des phases est annoncee EN GRAND "
                        "pendant la planification")
            else:
                if m["montrees"]:
                    vu["montrees"] += 1
                    if m["allumees"] == 0:
                        vu["eteintes"] += 1
                        eteintes_par_type[type_] = eteintes_par_type.get(type_, 0) + 1
                if m["allumees"] > 1:
                    fautes.append(f"decision {rang} : {m['allumees']} phases allumees")

            # 2. La zone adverse reste muette, vue de l'autre siege aussi.
            if m["adverse"] is None:
                fautes.append(f"decision {rang} : pas de zone adverse")
            else:
                trouves = sorted(n for n in noms if n in m["adverse"])
                if trouves:
                    fautes.append(f"decision {rang} : {trouves[:3]} dans la zone adverse")
                if m["cartes"] is None:
                    fautes.append(f"decision {rang} : la zone adverse ne se compte pas")

            # 3. A LA MISE EN PLACE, je tiens mes deux cartes Corporation : elles
            #    ne sont dans aucun etat rendu par le moteur, seulement dans le
            #    descripteur de MA decision. Ma main ne doit pas se vider pendant
            #    que l'adversaire repond a la meme question.
            #    Plus tard dans la partie, une main VIDE est legitime : on a tout
            #    pose ou tout vendu. On ne l'exige donc qu'a la mise en place.
            if m["type"] in ("corp_mulligan", "pick_corporation") and m["mienne"] < 2:
                fautes.append(
                    f"decision {rang} ({m['type']}) : ma main montre {m['mienne']} carte(s) "
                    "alors que je tiens mes deux Corporations")
            if m["mienne"] == 0:
                vu["mainVide"] += 1
                vides.append((rang, type_))

        n = jouer_en_regardant(pg, controle)
        print(f"siege 1 : {n} decisions jouees")
        if erreurs:
            for e in erreurs[:3]:
                print("  " + e)
            echec(f"{len(erreurs)} erreur(s) de console")
        if externes:
            echec(f"la page sort du dossier : {externes[:3]}")
        if n < 100:
            echec(f"seulement {n} decisions : la partie n'est pas allee au bout")

print(f"planification vue {vu['planif']} fois · "
      f"bande montree {vu['montrees']} fois · "
      f"dont eteinte {vu['eteintes']} · main vide {vu['mainVide']} fois")
print(f"barre d'equipage muette sur la phase d'en face : "
      f"{vu['barreMuette']}/{vu['planif']} planifications · "
      f"fuites PROUVEES (valeur changee depuis la manche precedente) : "
      f"{vu['fuiteProuvee']}")

# --- 5. la corporation d'en face, relue apres coup --------------------------
# Le nom n'est connu qu'une fois public ; on retourne alors le chercher dans les
# pages gardees pendant que je choisissais. Aucune collision possible avec mes
# deux Corporations a moi : le moteur en distribue deux par joueur, disjointes.
rangs_avant = [(r, t) for r, t, _ in avant_ma_corpo]
print(f"corporation d'en face : {corpo_en_face!r} ({corpo_src_en_face}) · "
      f"{len(avant_ma_corpo)} decision(s) avant mon choix {rangs_avant}")

if not corpo_src_en_face:
    echec("le scan de la corporation d'en face n'a jamais ete vu : "
          "le nom de fichier n'a pas pu etre cherche")
if not corpo_en_face:
    echec("la corporation d'en face n'est jamais devenue publique : "
          "la mesure n'a rien eu a confronter")
if not any(t == "pick_corporation" for _, t in rangs_avant):
    echec("`pick_corporation` n'est pas dans la fenetre a risque : "
          "la mesure passe a cote du moment ou je choisis")
# L'OCCASION ETAIT REELLE. Le moteur n'installe une corporation qu'en reponse a
# un `pick_corporation` ; si celle d'en face est deja la a la decision qui SUIT
# la mienne, c'est qu'elle etait installee pendant que je choisissais — donc
# qu'il y avait bien quelque chose a taire.
if not corpo_apres:
    echec("la corporation d'en face n'est pas posee juste apres mon choix : "
          "impossible de prouver qu'il y avait une fuite a fermer")
def trace(html):
    """Les traces de la corporation d'en face : son nom, et le scan qui la nomme."""
    return [x for x in (corpo_en_face, corpo_src_en_face) if x and x in html]


muettes = 0
for rang, type_, html in avant_ma_corpo:
    laissees = trace(html)
    if laissees:
        fautes.append(
            f"decision {rang} ({type_}) : {laissees} dans la page alors que "
            "je n'ai pas encore choisi ma corporation")
    else:
        muettes += 1
print(f"corporation d'en face muette avant mon choix : "
      f"{muettes}/{len(avant_ma_corpo)} decisions")

if eteintes_par_type:
    print("ecrans sans phase allumee, par type :",
          sorted(eteintes_par_type.items(), key=lambda x: -x[1])[:6])

if vu["planif"] < 5:
    echec("la planification n'a presque jamais ete rencontree : rien n'a ete confronte")
if vu["montrees"] < 50:
    echec(f"la bande des phases n'a ete montree qu'a {vu['montrees']} decisions")
if vu["barreMuette"] < vu["planif"]:
    echec(f"la barre d'equipage a montre la phase d'en face sur "
          f"{vu['planif'] - vu['barreMuette']} planification(s) sur {vu['planif']}")

# « Celle en cours allumee » : on tolere quelques ecrans eteints (les moments ou
# le type de la decision ne nomme aucune phase), pas une part importante.
part = vu["eteintes"] / max(vu["montrees"], 1)
if part > 0.05:
    echec(f"{part:.0%} des ecrans montrent une carte Phase sans qu'aucune soit allumee")
if fautes:
    for f in fautes[:5]:
        print("  " + f)
    echec(f"{len(fautes)} faute(s)")
print(f"OK siege 1 : rien ne fuit, la phase en cours est allumee sur "
      f"{1 - part:.0%} des ecrans qui en montrent une")

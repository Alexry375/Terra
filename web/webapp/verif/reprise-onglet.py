#!/usr/bin/env python3
"""REPRENDRE UNE PARTIE INTERROMPUE — l'onglet se ferme, le navigateur reste.

POURQUOI CE BANC EXISTE, alors que `inputs/checks/01` mesure déjà la reprise.

`pilote.page()` relance un NAVIGATEUR à chaque appel, et Playwright donne à
Chrome un profil temporaire neuf à chaque lancement
(`/tmp/playwright_chromiumdev_profile-XXXXXX`). Entre deux `page()`, il ne
survit donc rien : ni `localStorage`, ni cookie, ni `indexedDB`. Ce n'est pas un
onglet qu'on rouvre, c'est un ordinateur neuf — et sur un ordinateur neuf, une
partie interrompue ailleurs ne peut pas exister. La preuve tient en dix lignes :
`outputs/work/preuve-profil-neuf.py`.

Ce banc-ci fait donc la coupure que le contrôle 01 DÉCRIT (« on ferme
brutalement l'onglet, on rouvre ») : **un seul navigateur, un seul contexte, un
onglet fermé puis un autre ouvert**. C'est aussi ce qu'un joueur fait.

Il ne lit AUCUN nom de variable de la page et ne suppose rien de la façon dont la
sauvegarde est faite. Il mesure SIX propriétés :

  1. LA REPRISE EXISTE ET ELLE EST FIDÈLE — même rang, même planète, même main.
  2. ON PEUT REFUSER, et la partie neuve n'est pas l'ancienne déguisée.
  3. UNE PARTIE FINIE NE SE PROPOSE PLUS.
  4. UN ENREGISTREMENT ABÎMÉ NE CASSE RIEN — quatre façons de l'abîmer, dont
     celle qui ne se voit pas : des indices parfaitement valides qui ne
     désignent plus la même chose.
  5. ⚠️ REPRENDRE PUIS JOUER JUSQU'AU BOUT DONNE LE MÊME SCORE FINAL qu'une
     partie jamais coupée. C'est la propriété que le contrôle 01 ne mesure pas,
     et c'est celle qui distingue une vraie reprise d'une reprise « à peu près » :
     retomber au bon rang puis diverger se verrait ici, et nulle part ailleurs.
  6. L'ENREGISTREMENT NE CONTIENT QUE LE NÉCESSAIRE — pas un nom de carte, pas un
     chemin d'image, pas d'état recopié.

    TERRA_WEBAPP=<...>/web/webapp python3 verif/reprise-onglet.py [graine]
"""
import contextlib
import json
import os
import sys

RACINE = os.environ.get("TERRA_WEBAPP") or os.path.abspath(
    os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
sys.path.insert(0, os.path.join(RACINE, "verif"))

from pilote import serveur, choix_simple, echec  # noqa: E402

GRAINE = sys.argv[1] if len(sys.argv) > 1 else "4242"
COUPURE = 40  # décisions jouées avant de fermer l'onglet

# L'ÉTAT VISIBLE, lu par des repères que la page déclarait déjà avant ce
# chantier : si la reprise est juste, ils coïncident tous.
ETAT = """() => {
  const nb = (s) => { const e = document.querySelector(s); return e ? e.textContent.trim() : null; };
  const porteur = document.querySelector('[data-decision-rang]');
  return {
    rang: porteur ? Number(porteur.getAttribute('data-decision-rang')) : null,
    temperature: nb('[data-valeur="planet.temperature"]'),
    oxygene: nb('[data-valeur="planet.oxygen"]'),
    oceans: nb('[data-valeur="planet.oceans"]'),
    main: document.querySelectorAll('[data-carte-main]').length,
  };
}"""


@contextlib.contextmanager
def navigateur(largeur=1600, hauteur=1000):
    """UN navigateur, UN contexte — donc UNE mémoire, comme sur une vraie machine.

    Rend une fonction `ouvrir(url)` qui donne un onglet neuf et la liste de ses
    erreurs de console. Fermer l'onglet ne touche pas à la mémoire du navigateur ;
    c'est très exactement la coupure qu'on veut éprouver.
    """
    from playwright.sync_api import sync_playwright

    with sync_playwright() as p:
        nav = p.chromium.launch(executable_path="/usr/bin/google-chrome")
        ctx = nav.new_context(viewport={"width": largeur, "height": hauteur})

        def ouvrir(url):
            pg = ctx.new_page()
            erreurs = []

            def _console(m):
                if m.type != "error":
                    return
                if "favicon.ico" in (m.location or {}).get("url", ""):
                    return
                erreurs.append(f"console.{m.type} : {m.text}")

            pg.on("pageerror", lambda e: erreurs.append(f"exception : {e}"))
            pg.on("console", _console)
            pg.goto(url, wait_until="domcontentloaded")
            return pg, erreurs

        try:
            yield ouvrir
        finally:
            nav.close()


def repondre_une_fois(pg):
    """Répond à la décision posée, exactement comme `pilote.jouer` le ferait."""
    pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
    porteur = pg.query_selector("[data-decision-rang]")
    if porteur is None:
        return None
    rang = int(porteur.get_attribute("data-decision-rang"))
    forme = porteur.get_attribute("data-decision-forme") or "simple"
    visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if forme == "montant":
        champ = pg.wait_for_selector("[data-montant]", timeout=20000)
        mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
        champ.fill(str(mini + (rang % (maxi - mini + 1))))
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
        arg=rang, timeout=20000)
    return rang


def jouer_n(pg, n):
    for _ in range(n):
        if pg.query_selector("[data-partie-terminee]"):
            return
        repondre_une_fois(pg)


def jouer_jusqu_au_bout(pg, plafond=3000):
    vues = 0
    for _ in range(plafond):
        if pg.query_selector("[data-partie-terminee]"):
            break
        repondre_une_fois(pg)
        vues += 1
    else:
        echec("la partie ne se termine pas")
    scores = []
    for j in (0, 1):
        e = pg.query_selector(f'[data-score-final="{j}"]')
        if e is None:
            echec(f"pas de score final pour le joueur {j}")
        scores.append(int("".join(c for c in e.inner_text() if c.isdigit() or c == "-")))
    return vues, scores


def reprendre(pg):
    """Clique ce qui propose de reprendre. Rend False si rien ne le propose."""
    for sel in ('[data-reprendre]', '[data-reprise="oui"]',
                'button:has-text("Reprendre")', 'button:has-text("Resume")'):
        e = pg.query_selector(sel)
        if e and e.is_visible():
            e.click()
            return True
    return False


def refuser(pg):
    for sel in ('[data-nouvelle-partie]', '[data-reprise="non"]',
                'button:has-text("Nouvelle")', 'button:has-text("New")'):
        e = pg.query_selector(sel)
        if e and e.is_visible():
            e.click()
            return True
    return False


def ce_qui_est_en_trop(brut):
    """CE QUE L'ENREGISTREMENT NE DOIT PAS CONTENIR.

    On ne lit pas les noms de champs de la page — on ne suppose donc pas sa
    forme. On regarde la MATIÈRE : une graine, des boîtes et une liste d'indices
    ne sont que des nombres et deux ou trois mots courts. Une main recopiée, une
    liste de cartes, un visuel ou un état complet se trahissent tous par la même
    chose : beaucoup de chaînes, longues, et des chemins de fichiers. Les visuels
    des cartes sont sous droits d'auteur ; ils n'ont rien à faire là.
    """
    dits = []
    if len(brut) > 8000:
        dits.append(f"l'enregistrement pese {len(brut)} caracteres : c'est plus "
                    f"qu'une graine et une liste d'indices")
    try:
        objets = [json.loads(brut)]
    except ValueError:
        dits.append("l'enregistrement n'est pas du JSON lisible")
        return dits

    chaines = []
    nombres = []

    def parcourir(x):
        if isinstance(x, dict):
            for k, v in x.items():
                chaines.append(k)
                parcourir(v)
        elif isinstance(x, list):
            for v in x:
                parcourir(v)
        elif isinstance(x, str):
            chaines.append(x)
        elif isinstance(x, (int, float)):
            nombres.append(x)

    for o in objets:
        parcourir(o)

    for s in chaines:
        if "/" in s and not s.startswith("base"):
            dits.append(f"l'enregistrement contient un chemin : « {s[:40]} »")
        if any(s.lower().endswith(e) for e in (".webp", ".png", ".jpg", ".jpeg")):
            dits.append(f"l'enregistrement contient une image : « {s[:40]} »")
        if len(s) > 64:
            dits.append(f"l'enregistrement contient une chaine de {len(s)} "
                        f"caracteres : ce n'est ni une graine ni un indice")
    if len(chaines) > 12:
        dits.append(f"l'enregistrement contient {len(chaines)} chaines : une "
                    f"graine, des boites et des indices n'en demandent pas tant")
    return dits


fautes = []

with serveur(RACINE) as base:
    URL = f"{base}/?graine={GRAINE}&siege=0&animations=non"

    # ---- 0. LA PARTIE DE RÉFÉRENCE : jamais coupée, jouée d'un trait.
    with navigateur() as ouvrir:
        pg, err = ouvrir(URL)
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        vues_ref, scores_ref = jouer_jusqu_au_bout(pg)
        if err:
            fautes.append(f"{len(err)} erreur(s) de console sur la partie de "
                          f"reference : {err[0]}")
        pg.close()

    # ---- 1, 2, 5 et 6 : un seul navigateur, l'onglet se ferme et se rouvre.
    with navigateur() as ouvrir:
        pg, err = ouvrir(URL)
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        jouer_n(pg, COUPURE)
        avant = pg.evaluate(ETAT)
        garde = pg.evaluate(
            "() => Object.fromEntries(Object.entries(window.localStorage))")
        if err:
            fautes.append(f"{len(err)} erreur(s) de console avant la coupure : {err[0]}")
        pg.close()  # LA COUPURE : l'onglet meurt, le navigateur se souvient.

        if not garde:
            fautes.append(f"rien n'a ete enregistre apres {COUPURE} decisions")

        # ---- 6. l'enregistrement ne dit que le nécessaire
        brut = "".join(garde.values())
        fautes.extend(ce_qui_est_en_trop(brut))

        # ---- 1. la reprise existe et elle est fidèle
        pg, err = ouvrir(URL)
        pg.wait_for_timeout(400)
        if not reprendre(pg):
            fautes.append("aucune proposition de reprise apres la fermeture de l'onglet")
            pg.close()
        else:
            pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            apres = pg.evaluate(ETAT)
            for c in ("rang", "temperature", "oxygene", "oceans", "main"):
                if apres[c] != avant[c]:
                    fautes.append(f"reprise infidele : {c} = {apres[c]} au lieu "
                                  f"de {avant[c]}")
            # ---- 5. et la suite de la partie ne diverge pas
            vues_apres, scores_apres = jouer_jusqu_au_bout(pg)
            if scores_apres != scores_ref:
                fautes.append(f"la partie reprise finit sur {scores_apres} au lieu "
                              f"de {scores_ref} : elle a divergé apres la reprise")
            if err:
                fautes.append(f"{len(err)} erreur(s) de console a la reprise : {err[0]}")
            pg.close()

        # ---- 3. une partie finie ne se propose plus
        pg, err = ouvrir(URL)
        pg.wait_for_timeout(400)
        if reprendre(pg):
            fautes.append("une partie TERMINEE est encore proposee a la reprise")
        pg.close()

    # ---- 2. on peut refuser, et la partie neuve n'est pas l'ancienne
    with navigateur() as ouvrir:
        pg, err = ouvrir(URL)
        pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
        jouer_n(pg, COUPURE)
        rang_coupe = pg.evaluate(ETAT)["rang"]
        pg.close()

        pg, err = ouvrir(URL)
        pg.wait_for_timeout(400)
        if not refuser(pg):
            fautes.append("impossible de refuser la reprise et de commencer une partie neuve")
        else:
            pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            d = pg.evaluate(ETAT)
            if d["rang"] is None or d["rang"] >= rang_coupe:
                fautes.append(f"la partie « neuve » commence au rang {d['rang']} : "
                              f"c'est l'ancienne deguisee")
        if err:
            fautes.append(f"{len(err)} erreur(s) de console sur le refus : {err[0]}")
        pg.close()

    # ---- 4. UN ENREGISTREMENT ABÎMÉ NE CASSE RIEN.
    #
    # Quatre façons de l'abîmer, de la plus grossière à la plus sournoise. La
    # dernière est celle qui compte : des indices PARFAITEMENT VALIDES, mais qui
    # ne mènent plus à la question qu'on avait laissée — c'est-à-dire ce qui
    # arrive quand une question du moteur change de place. Le rejeu ne lève pas ;
    # seule une empreinte peut le voir.
    CADRE = '"graine":4242,"boites":"base,decouverte","siege":0,"decide":"humain"'
    ABIMES = {
        "des ordures qui ne sont pas du JSON": "]{ ceci n'est pas du json",
        "du JSON qui n'est pas une partie":
            '{"ceci":"n est pas une partie","decisions":[999,-1]}',
        "un enregistrement d'une forme plus ancienne":
            '{"forme":1,' + CADRE + ',"decisions":[0,0],"attendue":"pick_phase"}',
        "des indices hors bornes":
            '{"forme":2,' + CADRE + ',"decisions":[999,-1],"empreinte":0}',
        # LE CAS SOURNOIS, et le seul que le moteur ne peut pas voir : des
        # indices PARFAITEMENT VALIDES, que le moteur accepte sans broncher,
        # derriere une empreinte qui ne correspond plus. C'est ce qui arrive
        # quand une question change de place : la partie serait reprise fausse.
        "une liste valide derriere une empreinte qui ne correspond plus":
            '{"forme":2,' + CADRE + ',"decisions":[0,0],"empreinte":123456789}',
    }
    for dit, poison in ABIMES.items():
        with navigateur() as ouvrir:
            # On sème d'abord la sauvegarde par une vraie partie, puis on
            # remplace son contenu : la clé est celle que la page emploie, pas
            # une clé inventée pour le banc.
            pg, err = ouvrir(URL)
            pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            jouer_n(pg, 6)
            cles = pg.evaluate("() => Object.keys(window.localStorage)")
            if not cles:
                fautes.append("aucune cle a abimer : la page n'enregistre rien")
                pg.close()
                continue
            pg.evaluate("([cles, poison]) => { for (const k of cles) "
                        "window.localStorage.setItem(k, poison); }", [cles, poison])
            pg.close()

            pg, err = ouvrir(URL)
            pg.wait_for_timeout(400)
            reprendre(pg)
            try:
                pg.wait_for_selector("[data-decision-rang]", state="attached", timeout=20000)
            except Exception:
                fautes.append(f"un enregistrement abime ({dit}) empeche la page de demarrer")
            # La page doit être JOUABLE, pas seulement affichée.
            try:
                repondre_une_fois(pg)
            except Exception as e:
                fautes.append(f"un enregistrement abime ({dit}) rend la page "
                              f"injouable : {e}")
            if err:
                fautes.append(f"un enregistrement abime ({dit}) provoque "
                              f"{len(err)} erreur(s) de console : {err[0]}")
            pg.close()

print(f"    partie de reference : {vues_ref} questions, scores {scores_ref}")
print(f"    coupure de l'onglet apres {COUPURE} decisions ; "
      f"enregistrement de {len(brut)} caracteres")
print(f"    {len(ABIMES)} facons d'abimer l'enregistrement eprouvees")
print(f"    {len(fautes)} defaut(s)")
if fautes:
    for f in fautes[:10]:
        print("      " + f)
    echec(f"{len(fautes)} defaut(s) sur la reprise")
print("OK l'onglet se ferme et la partie se reprend fidelement, se refuse, ne se "
      "propose plus finie, ne diverge pas, et aucun enregistrement abime ne casse rien")

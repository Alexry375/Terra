#!/usr/bin/env python3
"""BANC DU RENDEZ-VOUS — ce que les cinq controles d'inputs/ ne regardent pas.

    PYTHONDONTWRITEBYTECODE=1 python3 outputs/webapp/verif/rendez-vous.py

Les cinq controles livres verifient qu'une partie entiere se joue a deux, que le
serveur refuse un rang inattendu et un siege inconnu, qu'on reprend apres une
coupure, que le mode local est intact et qu'il n'y a rien a installer. Ils
laissent QUATRE angles morts, et ce sont eux que ce banc eprouve. Il n'importe
rien d'inputs/ : c'est un oracle independant, avec ses propres choix (ceux du
pilote livre sont « rang * 7919 + 13 » ; ici c'est « rang * 13 + 5 »), pour que
deux parties differentes soient couvertes.

  A. « data-attente » (moi | lui | aucune). Aucun controle livre ne le lit,
     alors que le prompt l'impose. On verifie a CHAQUE decision que la page qui
     porte la question annonce « moi », que l'autre annonce « lui », et que les
     deux ne disent JAMAIS « moi » en meme temps.

     UNE SEULE EXCEPTION, ET ELLE EST EXIGEE, PAS TOLEREE (MOT-9, 04-08). Le
     livret veut que les deux joueurs choisissent leur carte Phase EN MEME
     TEMPS : sur cette question-la — « pick_phase », et sur aucune autre — les
     deux pages DOIVENT dire « moi » ensemble. On ne se contente donc pas de
     lever l'interdit : on verifie que les deux portent bien une question de
     phase, que toutes deux annoncent « moi », et que leurs rangs se suivent
     (chacune la sienne, pas deux fois la meme). Partout ailleurs, deux « moi »
     simultanes restent une faute.

  B. « personne ne repond a la place de l'autre » PENDANT UNE VRAIE PARTIE. Le
     controle 02 parle au serveur sans navigateur : le tour n'y est jamais
     connu, et le premier arrive l'emporte. Ici les deux moteurs tournent, ils
     ont declare a qui revient le rang courant, et un tricheur qui repond avec
     le BON rang mais le MAUVAIS siege doit etre refuse, en toutes lettres.

  C. « data-adversaire » n'est pas deduit de l'existence d'une partie. On ouvre
     UNE seule page : la partie existe cote serveur, et l'attribut doit
     pourtant dire « absent ». Puis on ouvre la seconde et il doit passer a
     « present ».

  D. Le serveur ne sert rien hors de la livraison, ni son propre code.

Toute decision a laquelle le banc ne sait pas repondre l'arrete BRUYAMMENT, en
nommant le type de la question. Et chaque verdict dit sur COMBIEN d'occasions il
porte : zero faute sur zero occasion ne prouve rien.
"""
import json
import os
import re
import signal
import socket
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request

RACINE = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SERVEUR = os.path.join(RACINE, "webapp", "relais", "serveur.js")
CODE = "banc-rendez-vous"
GRAINE = 31337
DECISIONS_A_VOIR = 45

fautes = []
LIRE = "(n) => document.documentElement.getAttribute(n)"


def faute(m):
    fautes.append(m)
    print("   FAUTE " + m)


def echec(m):
    print("KO " + m)
    sys.exit(1)


def port_libre():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def demander(url, corps=None, delai=10):
    donnees, entetes = None, {}
    if corps is not None:
        donnees = json.dumps(corps).encode()
        entetes["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=donnees, headers=entetes)
    try:
        with urllib.request.urlopen(req, timeout=delai) as r:
            brut = r.read().decode("utf-8", "replace")
            try:
                return r.status, json.loads(brut)
            except Exception:
                return r.status, brut
    except urllib.error.HTTPError as e:
        brut = e.read().decode("utf-8", "replace")
        try:
            return e.code, json.loads(brut)
        except Exception:
            return e.code, brut
    except Exception as e:
        return 0, str(e)


# ----------------------------------------------------------------- le serveur

def lancer_serveur(port):
    if not os.path.exists(SERVEUR):
        echec(f"{SERVEUR} n'existe pas")
    sortie = []
    proc = subprocess.Popen(["node", SERVEUR, "--port", str(port)],
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            text=True, bufsize=1, preexec_fn=os.setsid)

    def lire():
        for ligne in proc.stdout:
            sortie.append(ligne.rstrip("\n"))

    threading.Thread(target=lire, daemon=True).start()
    limite = time.time() + 25
    while time.time() < limite:
        for ligne in list(sortie):
            if re.search(r"PRET\s+http://\S+", ligne):
                return proc, sortie
        if proc.poll() is not None:
            echec("le serveur s'est arrete tout de suite : " + "\n".join(sortie[-8:]))
        time.sleep(0.2)
    echec("pas de ligne PRET en 25 s")


def arreter_serveur(proc):
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
        proc.wait(timeout=10)
    except Exception:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except Exception:
            pass


# ------------------------------------------------------- repondre a une page
#
# Un oracle a lui : les choix ne dependent que du rang, mais pas de la meme
# facon que le pilote livre. Deux suites de decisions differentes, donc deux
# parties differentes eprouvees.

def choix_du_banc(rang, nb):
    return (rang * 13 + 5) % nb


def decision_posee(pg):
    e = pg.query_selector("[data-decision-rang]")
    if e is None:
        return None
    rang = e.get_attribute("data-decision-rang")
    if rang is None:
        return None
    return {
        "rang": int(rang),
        "forme": e.get_attribute("data-decision-forme") or "simple",
        "type": e.get_attribute("data-decision-type") or "?",
        "a_choisir": e.get_attribute("data-a-choisir"),
    }


def repondre(pg, d):
    """Repond. Leve BRUYAMMENT, en nommant la question, si elle est injouable."""
    choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if d["forme"] == "montant":
        champ = pg.wait_for_selector("[data-montant]", timeout=15000)
        mini = int(champ.get_attribute("min"))
        maxi = int(champ.get_attribute("max"))
        champ.fill(str(mini + (d["rang"] % (maxi - mini + 1))))
        pg.click("[data-valider]")
        return
    if d["forme"] == "multiple":
        if not choix:
            raise RuntimeError(f"decision {d['rang']} « {d['type']} » : aucun choix "
                               f"visible pour un choix MULTIPLE")
        brut = d["a_choisir"]
        k = int(brut) if (brut or "").isdigit() else max(1, len(choix) // 2)
        k = min(k, len(choix))
        for c in choix[:k]:
            c.click()
        pg.click("[data-valider]")
        return
    if not choix:
        raise RuntimeError(f"decision {d['rang']} « {d['type']} » : aucun "
                           f"[data-choix] visible alors que la page attend une reponse")
    choix[choix_du_banc(d["rang"], len(choix))].click()


def attendre_attribut(pg, nom, valeur, secondes=25):
    limite = time.time() + secondes
    while time.time() < limite:
        if pg.evaluate(LIRE, nom) == valeur:
            return True
        time.sleep(0.2)
    return False


# ------------------------------------------------------------------- le banc

def main():
    from playwright.sync_api import sync_playwright

    port = port_libre()
    proc, journal = lancer_serveur(port)
    base = f"http://127.0.0.1:{port}"
    vus = {"attente": 0, "triche": 0, "simultane": 0}
    try:
        with sync_playwright() as p:
            nav = p.chromium.launch(executable_path="/usr/bin/google-chrome")
            try:
                ctx0 = nav.new_context(viewport={"width": 1600, "height": 1000})
                pg0 = ctx0.new_page()
                erreurs = []
                pg0.on("pageerror", lambda e: erreurs.append(f"siege 0 : {e}"))
                pg0.goto(f"{base}/index.html?partie={CODE}&siege=0&graine={GRAINE}"
                         f"&animations=non", wait_until="domcontentloaded")

                # --- C. l'adversaire absent n'est pas « present »
                time.sleep(4)
                etat_code, etat = demander(f"{base}/relais/etat?partie={CODE}")
                if etat_code != 200:
                    echec(f"/relais/etat rend {etat_code}")
                if pg0.evaluate(LIRE, "data-adversaire") != "absent":
                    faute("la partie existe cote serveur et une seule page est "
                          "ouverte, mais data-adversaire vaut "
                          f"{pg0.evaluate(LIRE, 'data-adversaire')!r} au lieu de "
                          "'absent' : la presence est deduite, pas constatee")
                # L'attente d'une page SEULE doit deja dire la verite : « moi » si
                # elle porte la question, « lui » sinon. On ne suppose pas a qui le
                # moteur pose la premiere : on le lit sur la page.
                attendue = "moi" if decision_posee(pg0) else "lui"
                if pg0.evaluate(LIRE, "data-attente") != attendue:
                    faute(f"une page seule annonce data-attente="
                          f"{pg0.evaluate(LIRE, 'data-attente')!r} alors qu'elle "
                          f"{'porte' if attendue == 'moi' else 'ne porte pas'} la "
                          f"question : attendu {attendue!r}")

                ctx1 = nav.new_context(viewport={"width": 1600, "height": 1000})
                pg1 = ctx1.new_page()
                pg1.on("pageerror", lambda e: erreurs.append(f"siege 1 : {e}"))
                pg1.goto(f"{base}/index.html?partie={CODE}&siege=1&graine={GRAINE}"
                         f"&animations=non", wait_until="domcontentloaded")
                if not attendre_attribut(pg0, "data-adversaire", "present"):
                    faute("le siege 0 ne voit pas arriver le siege 1")
                if not attendre_attribut(pg1, "data-adversaire", "present"):
                    faute("le siege 1 ne voit pas le siege 0 deja la")

                # --- A + B. la partie avance, on regarde l'attente et on triche
                pages = [pg0, pg1]
                donnees = 0
                debut = time.time()
                triche_faite = 0
                while donnees < DECISIONS_A_VOIR and time.time() - debut < 300:
                    porteuses = [i for i, pg in enumerate(pages)
                                 if decision_posee(pg) is not None]
                    if not porteuses:
                        time.sleep(0.2)
                        continue

                    # A. qui attend quoi, vu des deux cotes
                    posees = [decision_posee(pg) for pg in pages]
                    attentes = [pg.evaluate(LIRE, "data-attente") for pg in pages]
                    # La partie avance pendant qu'on la regarde : on ne juge que si
                    # rien n'a bouge entre les deux lectures, sinon on compare deux
                    # instants differents et l'on invente une faute.
                    stable = porteuses == [i for i, pg in enumerate(pages)
                                           if decision_posee(pg) is not None]
                    # LE CHOIX DE PHASE SE FAIT EN MEME TEMPS, et lui seul. Les
                    # deux pages portent alors chacune SA question — deux rangs
                    # qui se suivent — et toutes deux disent « moi ».
                    phases = [d for d in posees
                              if d is not None and d["type"] == "pick_phase"]
                    simultane = len(porteuses) == 2 and len(phases) == 2
                    if simultane:
                        vus["simultane"] += 1
                        rangs = sorted(d["rang"] for d in phases)
                        if rangs[1] - rangs[0] != 1:
                            faute(f"les deux pages portent une question de phase mais "
                                  f"leurs rangs ne se suivent pas : {rangs} — chacune "
                                  f"doit porter la sienne")
                        for i in (0, 1):
                            if attentes[i] != "moi":
                                faute(f"la page {i} porte sa question de phase en meme "
                                      f"temps que l'autre mais annonce data-attente="
                                      f"{attentes[i]!r} au lieu de 'moi' : ce joueur "
                                      f"croit encore qu'il attend")
                    elif attentes.count("moi") > 1:
                        types = [d["type"] if d else None for d in posees]
                        faute(f"les DEUX pages annoncent data-attente='moi' "
                              f"(decision {donnees}, types {types}) alors que seule "
                              f"la question de phase se joue en meme temps")
                    if stable and not simultane:
                        vus["attente"] += 1
                        for i in porteuses:
                            if attentes[i] != "moi":
                                faute(f"la page {i} porte la question mais annonce "
                                      f"data-attente={attentes[i]!r} au lieu de 'moi'")
                            autre = 1 - i
                            if autre not in porteuses and attentes[autre] != "lui":
                                faute(f"la page {autre} attend la reponse de l'autre "
                                      f"mais annonce data-attente={attentes[autre]!r}"
                                      f" au lieu de 'lui'")

                    i = porteuses[0]
                    d = decision_posee(pages[i])

                    # B. un tricheur qui vise le BON rang avec le MAUVAIS siege.
                    # On le fait trois fois, a des moments differents de la partie.
                    if triche_faite < 3 and donnees in (5, 17, 33):
                        triche_faite += 1
                        vus["triche"] += 1
                        c, av = demander(f"{base}/relais/etat?partie={CODE}")
                        # LE RANG ATTENDU SE LIT, IL NE SE COMPTE PLUS. Depuis
                        # que les choix de phase se jouent face cachee, la liste
                        # publiee s'arrete au premier groupe incomplet : sa
                        # longueur n'est plus le rang que le serveur attend, et
                        # « siege_attendu » se rapporte a CE rang-la. Compter les
                        # reponses viserait un autre rang que le siege annonce,
                        # et la triche serait refusee pour la mauvaise raison.
                        rang = av["rang_attendu"]
                        proprio = av.get("siege_attendu")
                        if proprio is None:
                            faute(f"a la decision {rang}, le serveur ne sait toujours "
                                  f"pas a quel siege elle revient : les pages ne le "
                                  f"lui annoncent pas")
                        else:
                            c2, rep = demander(f"{base}/relais/decision", {
                                "partie": CODE, "siege": 1 - proprio,
                                "rang": rang, "reponse": 0})
                            phrase = " ".join(str(v) for v in rep.values()) \
                                if isinstance(rep, dict) else str(rep)
                            if 200 <= c2 < 300:
                                faute(f"le siege {1 - proprio} a pu repondre a la "
                                      f"place du siege {proprio} (rang {rang}) : "
                                      f"le tour n'est pas garde en vraie partie")
                            elif len(phrase.strip()) < 10:
                                faute(f"la triche est refusee ({c2}) sans dire "
                                      f"pourquoi : {phrase!r}")
                            c3, ap = demander(f"{base}/relais/etat?partie={CODE}")
                            if len(ap["decisions"]) != len(av["decisions"]):
                                faute("la liste des decisions a bouge apres une "
                                      "reponse refusee")

                    try:
                        repondre(pages[i], d)
                    except Exception as ex:
                        arreter_serveur(proc)
                        echec(f"page {i} : {ex}")
                    donnees += 1
                    try:
                        pages[i].wait_for_function(
                            "r => { const e = document.querySelector("
                            "'[data-decision-rang]');"
                            " return !e || Number(e.getAttribute('data-decision-rang'))"
                            " !== r || document.querySelector("
                            "'[data-partie-terminee]'); }",
                            arg=d["rang"], timeout=15000)
                    except Exception:
                        arreter_serveur(proc)
                        echec(f"page {i} : la decision {d['rang']} « {d['type']} » "
                              f"est restee posee apres la reponse")

                if donnees < DECISIONS_A_VOIR:
                    faute(f"seulement {donnees} decisions jouees en 5 min")
                # L'exception accordee au choix de phase ne prouve rien si elle
                # n'a jamais servi : une partie de 45 decisions en traverse
                # plusieurs manches, donc plusieurs choix de phase.
                if vus["simultane"] == 0:
                    faute("aucun choix de phase vu sur les DEUX pages a la fois en "
                          f"{donnees} decisions : soit les deux joueurs ne choisissent "
                          "pas en meme temps, soit la mesure n'a pas eu lieu")
                if triche_faite < 3:
                    faute(f"seulement {triche_faite} tentative(s) de triche sur 3 : "
                          f"la partie n'a pas assez avance pour les placer")
                if erreurs:
                    faute(f"{len(erreurs)} erreur(s) de page : {erreurs[:3]}")

                ctx0.close()
                ctx1.close()
            finally:
                nav.close()

        # --- E. quelqu'un repond a MA place, avec MON siege (un second onglet
        # ouvert par megarde sur la meme adresse). Mon clic ne doit PAS etre
        # remplace en silence par le sien : il faut que la page le DISE.
        vus["double"] = 0
        with sync_playwright() as p:
            nav = p.chromium.launch(executable_path="/usr/bin/google-chrome")
            try:
                ctx = nav.new_context(viewport={"width": 1600, "height": 1000})
                pg = ctx.new_page()
                pg.goto(f"{base}/index.html?partie=double-siege&siege=0"
                        f"&graine=99&animations=non", wait_until="domcontentloaded")
                pg.wait_for_selector("[data-decision-rang]", timeout=30000,
                                     state="attached")
                d = decision_posee(pg)
                choix = [c for c in pg.query_selector_all("[data-choix]")
                         if c.is_visible()]
                mien = 1 if len(choix) > 1 else 0
                if d["forme"] != "simple" or mien == 0:
                    print("   (E) premiere decision inexploitable pour cet essai : "
                          f"forme {d['forme']}, {len(choix)} choix — essai non mene")
                else:
                    vus["double"] += 1
                    # L'intrus repond 0 ; moi je clique un autre choix.
                    c, rep = demander(f"{base}/relais/decision", {
                        "partie": "double-siege", "siege": 0,
                        "rang": d["rang"], "reponse": 0})
                    if not (200 <= c < 300):
                        faute(f"(E) le doublon n'a pas pu etre pose : {c} {rep}")
                    else:
                        choix[mien].click()
                        time.sleep(3)
                        bandeau = pg.query_selector("[data-en-ligne-bandeau]")
                        texte = bandeau.inner_text() if bandeau else ""
                        panne = pg.query_selector("#panne")
                        if "answered" not in texte.lower() and panne is None:
                            faute("(E) quelqu'un a repondu a ma place avec une AUTRE "
                                  "reponse que la mienne, et la page n'en dit rien : "
                                  f"bandeau {texte!r}, aucune panne affichee")
                ctx.close()
            finally:
                nav.close()

        # --- D. rien hors de la livraison, ni le code du serveur
        for chemin, quoi in (
            ("/relais/serveur.js", "son propre code"),
            ("/../AGENTS.md", "un fichier au-dessus de la livraison"),
            ("/..%2fAGENTS.md", "un fichier au-dessus, chemin encode"),
            ("/../../inputs/prompt.md", "le contrat de la main"),
        ):
            c, _ = demander(f"{base}{chemin}")
            if 200 <= c < 300:
                faute(f"le serveur sert {quoi} ({chemin}) : code {c}")

        # Le serveur a-t-il ecrit ce qu'il faisait ?
        lignes = list(journal)
        for mot, quoi in (("ouverte", "l'ouverture d'une partie"),
                          ("arrive", "l'arrivee d'un joueur"),
                          ("recue", "une decision recue"),
                          ("refus", "un refus")):
            def sans_accent(t):
                for a, b in (("é", "e"), ("è", "e"), ("ê", "e"), ("à", "a"),
                             ("ç", "c"), ("î", "i"), ("ô", "o"), ("û", "u")):
                    t = t.replace(a, b)
                return t.lower()
            if not any(mot in sans_accent(l) for l in lignes):
                faute(f"le serveur n'ecrit jamais {quoi} dans sa sortie")
    finally:
        arreter_serveur(proc)

    print(f"   {vus['attente']} occasions ou l'attente a ete lue sur les deux pages")
    print(f"   {vus['simultane']} choix de phase portes par les DEUX pages en meme "
          f"temps")
    print(f"   {vus['triche']} tentatives de reponse a la place de l'autre, "
          f"en pleine partie")
    print(f"   {vus.get('double', 0)} essai(s) « quelqu'un repond avec MON siege »")
    if fautes:
        print(f"KO {len(fautes)} faute(s)")
        sys.exit(1)
    print("OK data-attente juste des deux cotes, presence constatee et non deduite, "
          "aucune reponse a la place de l'autre en vraie partie, rien de servi hors "
          "de la livraison.")


if __name__ == "__main__":
    main()

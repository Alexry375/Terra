#!/usr/bin/env python3
"""Pilote de la page : une machine joue une partie entiere en cliquant.

Ce module sert aux checks 02 et 03. Il n'ouvre aucun raccourci : il fait
exactement ce qu'un joueur ferait, en cliquant sur les elements que la page
declare cliquables (`data-choix`, `data-valider`) au moment ou elle attend une
reponse.

Les choix sont DETERMINISTES et ne dependent que du rang de la decision, pour
qu'une partie jouee a l'ecran soit rejouable ailleurs a l'identique :

    choix simple   -> indice (rang * 7919 + 13) % nombre_de_choix
    montant        -> minimum + (rang % (maximum - minimum + 1))
    choix multiple -> les k premiers indices, k = data-a-choisir
"""
import contextlib
import http.server
import os
import socket
import socketserver
import sys
import threading

RACINE = os.path.abspath("outputs/web/webapp")


def choix_simple(rang, nb):
    return (rang * 7919 + 13) % nb


def choix_montant(rang, mini, maxi):
    return mini + (rang % (maxi - mini + 1))


class _Silencieux(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def __init__(self, *a, **kw):
        super().__init__(*a, directory=RACINE, **kw)


@contextlib.contextmanager
def serveur(racine=None):
    """Sert le dossier livre sur un port libre, le temps du bloc."""
    global RACINE
    if racine:
        RACINE = os.path.abspath(racine)
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    socketserver.TCPServer.allow_reuse_address = True
    srv = socketserver.TCPServer(("127.0.0.1", port), _Silencieux)
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    try:
        yield f"http://127.0.0.1:{port}"
    finally:
        srv.shutdown()
        srv.server_close()


@contextlib.contextmanager
def page(url, largeur=1600, hauteur=1000):
    """Ouvre la page dans le navigateur pilotable, en collectant les erreurs."""
    from playwright.sync_api import sync_playwright

    with sync_playwright() as p:
        nav = p.chromium.launch(executable_path="/usr/bin/google-chrome")
        pg = nav.new_page(viewport={"width": largeur, "height": hauteur})
        erreurs = []

        def _console(m):
            # Le navigateur reclame toujours /favicon.ico : c'est son bruit a lui,
            # pas un defaut de la page.
            if m.type != "error":
                return
            if "favicon.ico" in (m.location or {}).get("url", ""):
                return
            erreurs.append(f"console.{m.type} : {m.text}")

        pg.on("pageerror", lambda e: erreurs.append(f"exception : {e}"))
        pg.on("console", _console)
        externes = []
        pg.on("request", lambda r: externes.append(r.url)
              if not r.url.startswith(("http://127.0.0.1", "data:", "blob:")) else None)
        pg.goto(url, wait_until="domcontentloaded")
        try:
            yield pg, erreurs, externes
        finally:
            nav.close()


def jouer(pg, decisions_max=2000, apres=None, delai=15000):
    """Joue la partie jusqu'a la fin. Rend (nb_decisions, scores) ou leve.

    `apres(pg, rang)` est appele apres chaque reponse : sert aux captures.
    """
    vus = []
    for _ in range(decisions_max):
        if pg.query_selector("[data-partie-terminee]"):
            break
        pg.wait_for_selector("[data-decision-rang]", timeout=delai, state="attached")
        porteur = pg.query_selector("[data-decision-rang]")
        rang = int(porteur.get_attribute("data-decision-rang"))
        forme = porteur.get_attribute("data-decision-forme") or "simple"
        vus.append(rang)
        choix = pg.query_selector_all("[data-choix]")
        visibles = [c for c in choix if c.is_visible()]
        if forme == "montant":
            champ = pg.wait_for_selector("[data-montant]", timeout=delai)
            mini = int(champ.get_attribute("min"))
            maxi = int(champ.get_attribute("max"))
            champ.fill(str(choix_montant(rang, mini, maxi)))
            pg.click("[data-valider]")
        elif forme == "multiple":
            # `data-a-choisir` absent = nombre LIBRE (le remplacement partiel
            # des cartes de depart) : on en prend un, deterministe.
            brut = porteur.get_attribute("data-a-choisir")
            k = int(brut) if (brut or "").isdigit() else (rang % max(len(visibles), 1)) + 1
            k = min(k, len(visibles))
            if len(visibles) < k:
                raise RuntimeError(
                    f"decision {rang} : {len(visibles)} choix visibles pour {k} a choisir")
            for c in visibles[:k]:
                c.click()
            pg.click("[data-valider]")
        else:
            if not visibles:
                raise RuntimeError(
                    f"decision {rang} : aucun [data-choix] VISIBLE alors que la page "
                    f"attend une reponse ({len(choix)} presents dans le code)")
            visibles[choix_simple(rang, len(visibles))].click()
        if apres:
            apres(pg, rang)
        pg.wait_for_function(
            "r => { const e = document.querySelector('[data-decision-rang]');"
            " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
            " || document.querySelector('[data-partie-terminee]'); }",
            arg=rang, timeout=delai)
    else:
        raise RuntimeError(f"la partie n'est pas terminee apres {decisions_max} decisions")

    if not pg.query_selector("[data-partie-terminee]"):
        raise RuntimeError("la partie s'est arretee sans [data-partie-terminee]")
    scores = []
    for j in (0, 1):
        e = pg.query_selector(f'[data-score-final="{j}"]')
        if e is None:
            raise RuntimeError(f"pas de [data-score-final=\"{j}\"] a la fin")
        t = "".join(c for c in e.inner_text() if c.isdigit() or c == "-")
        if not t:
            raise RuntimeError(f"[data-score-final=\"{j}\"] ne contient aucun nombre")
        scores.append(int(t))
    return len(vus), scores


def echec(msg):
    print(f"KO {msg}")
    sys.exit(1)

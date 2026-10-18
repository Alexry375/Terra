#!/usr/bin/env python3
"""Sert `web/webapp/` en HTTP, SANS mise en réserve par le navigateur.

Pourquoi ce fichier plutôt que `python3 -m http.server` : le navigateur garde
les fichiers en réserve, et cela nous a déjà coûté une heure à deux — du code
neuf tournait avec un moteur compilé périmé, et l'écran affichait un mélange
incohérent des deux versions. L'en-tête `Cache-Control: no-store` interdit cette
réserve : chaque rechargement redemande tout.

    python3 web/servir.py [port] [adresse]   (défaut : 8020, 127.0.0.1)

Sur le VPS, l'adresse est l'IP Tailscale, jamais 0.0.0.0 : la machine a une IP
publique.

La page ne fonctionne QUE servie en HTTP — jamais par un chemin de fichier, à
cause des modules et du moteur compilé.

(jouer-contre-l-apprenti) UNE SEULE EXCEPTION À LA RÈGLE : LES VISUELS. Les
images de `assets/` ne changent pas d'une version du code à l'autre, et sur une
mauvaise connexion les redemander à chaque rechargement coûtait 2 Mo. Elles, et
elles seules, ont droit à la réserve (`max-age`). Le code, le moteur compilé, le
manifeste des visuels et les réglages du réseau (`poids/apprenti.txt`, qui change
à chaque nouveau champion) restent en `no-store` : une réserve dessus ferait
tourner une version périmée.
"""
import http.server
import os
import posixpath
import socketserver
import sys
import urllib.parse

RACINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "webapp")
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8020
HOTE = sys.argv[2] if len(sys.argv) > 2 else "127.0.0.1"
if HOTE in ("0.0.0.0", "::", ""):
    sys.exit("servir.py : adresse refusée (toutes les interfaces) : " + repr(HOTE))


# Une semaine : assez pour qu'une soirée de parties et ses rechargements ne
# repayent jamais une image ; et au-delà, le navigateur revalide (le serveur
# répond « 304 » sans renvoyer l'image tant qu'elle n'a pas changé).
RESERVE_VISUELS = 7 * 24 * 3600
EXTENSIONS_VISUELLES = (".webp", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".avif")


def est_un_visuel(chemin_demande):
    """Une image sous `assets/`, et rien d'autre (pas son manifeste JSON)."""
    # Normalisé d'abord : « /assets/../x.png » n'est pas sous `assets/`.
    chemin = posixpath.normpath(urllib.parse.unquote(urllib.parse.urlsplit(chemin_demande).path))
    return chemin.startswith("/assets/") and chemin.lower().endswith(EXTENSIONS_VISUELLES)


class Frais(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=RACINE, **kw)

    def send_response(self, code, message=None):
        self.code_envoye = code
        super().send_response(code, message)

    def end_headers(self):
        # Seule une image réellement servie se garde : un « 404 » mis en réserve
        # cacherait pendant une semaine un visuel installé après coup.
        if getattr(self, "code_envoye", None) in (200, 304) and est_un_visuel(self.path):
            self.send_header("Cache-Control", "public, max-age=" + str(RESERVE_VISUELS))
        else:
            self.send_header("Cache-Control", "no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
        super().end_headers()


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer((HOTE, PORT), Frais) as srv:
    print(f"http://{HOTE}:{PORT}/  ({RACINE})")
    srv.serve_forever()

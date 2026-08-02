#!/usr/bin/env python3
"""Sert `web/webapp/` en HTTP, SANS mise en réserve par le navigateur.

Pourquoi ce fichier plutôt que `python3 -m http.server` : le navigateur garde
les fichiers en réserve, et cela nous a déjà coûté une heure à deux — du code
neuf tournait avec un moteur compilé périmé, et l'écran affichait un mélange
incohérent des deux versions. L'en-tête `Cache-Control: no-store` interdit cette
réserve : chaque rechargement redemande tout.

    python3 web/servir.py [port]        (défaut : 8020)

La page ne fonctionne QUE servie en HTTP — jamais par un chemin de fichier, à
cause des modules et du moteur compilé.
"""
import http.server
import os
import socketserver
import sys

RACINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "webapp")
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8020


class Frais(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=RACINE, **kw)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        super().end_headers()


socketserver.TCPServer.allow_reuse_address = True
with socketserver.TCPServer(("127.0.0.1", PORT), Frais) as srv:
    print(f"http://127.0.0.1:{PORT}/  ({RACINE})")
    srv.serve_forever()

#!/usr/bin/env python3
"""LA PLACE EST-ELLE VRAIMENT RÉSERVÉE ? — mesure des défauts B et C du round 2.

Le contrôle 06 conclut une vente à chaque occasion ; chaque vente retire une
carte, et la main n'y dépasse donc jamais huit cartes. Or les défauts qu'on
corrige ici ne se voient QUE sur les mains longues : c'est à onze cartes que la
main atteignait les boutons de vente, à douze et treize que ses premières cartes
passaient dessous.

Cet outil reprend donc le protocole du CTO — ouvrir la vente, tout sonder,
désigner une carte, tout sonder à nouveau, puis RENONCER — qui laisse la main
s'allonger jusqu'à treize. À chaque occasion et aux quatre tailles du contrôle
06 :

  1. chaque carte de la main reçoit le clic en son centre ;
  2. le bouton qui conclut et celui qui renonce le reçoivent aussi, AVANT et
     APRÈS la désignation (désigner déplace les boutons) ;
  3. rien de la scène ni du panneau ne recouvre une carte.

Il ÉCHOUE aussi s'il n'a rien éprouvé de sérieux : zéro faute sur zéro occasion
ne prouve rien.

Usage : PYTHONDONTWRITEBYTECODE=1 python3 outputs/web/webapp/verif/vente-place-reservee.py
        (depuis la racine du workspace)
"""
import sys

sys.path.insert(0, "inputs/checks")
from pilote import serveur, page, jouer  # noqa: E402

TAILLES = [(1280, 720, 2024), (1600, 1000, 2024), (1680, 1050, 4242), (1920, 1080, 5150)]

SONDE = """(sel) => {
  const e = document.querySelector(sel);
  if (!e) return {absent: true};
  const r = e.getBoundingClientRect();
  if (r.width < 1 || r.height < 1) return {absent: false, ok: false, par: 'taille nulle'};
  const d = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
  if (d && (e === d || e.contains(d) || d.contains(e))) return {absent: false, ok: true, par: ''};
  return {absent: false, ok: false, par: d ? (d.tagName.toLowerCase() + '[' +
    (d.getAttribute('alt') || d.getAttribute('data-carte-cle') || d.className || '') + ']') : 'rien'};
}"""

MAIN = """() => {
  const cartes = [...document.querySelectorAll('#mienne-rang [data-carte-cle]')];
  return cartes.map((c, i) => {
    const r = c.getBoundingClientRect();
    const d = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    const ok = d && (c === d || c.contains(d) || d.contains(c));
    return {i, ok: !!ok, par: ok ? '' : (d ? (d.tagName.toLowerCase() + '[' +
      (d.getAttribute('alt') || d.getAttribute('data-carte-cle') || d.className || '') + ']') : 'rien')};
  });
}"""

CLES = """() => [...document.querySelectorAll('#mienne-rang [data-carte-cle]')]
          .map(e => e.getAttribute('data-carte-cle'))"""

fautes = []
compte = {"occasions": 0, "cartes": 0, "max": 0, "max_petite": 0, "longues": 0}


def main():
    with serveur("outputs/web/webapp") as base:
        for largeur, hauteur, graine in TAILLES:
            url = f"{base}/index.html?graine={graine}&siege=0&animations=non"
            with page(url, largeur=largeur, hauteur=hauteur) as (pg, erreurs, _ext):

                def apres(pg, rang, L=largeur, H=hauteur, g=graine):
                    if pg.query_selector('[data-vendre]') is None:
                        return
                    cles = pg.evaluate(CLES)
                    if len(cles) < 2:
                        return
                    pg.click('[data-vendre]')
                    compte["occasions"] += 1
                    compte["cartes"] += len(cles)
                    compte["max"] = max(compte["max"], len(cles))
                    if len(cles) >= 11:
                        compte["longues"] += 1
                    if L == 1280:
                        compte["max_petite"] = max(compte["max_petite"], len(cles))
                    ou = f"{L}x{H} g{g} rang {rang} (main de {len(cles)})"

                    mauvaises = [m for m in pg.evaluate(MAIN) if not m["ok"]]
                    if mauvaises:
                        noms = ", ".join(f"#{m['i']} recouverte par {m['par']}"
                                         for m in mauvaises[:3])
                        fautes.append(f"{ou} : {len(mauvaises)} carte(s) hors "
                                      f"d'atteinte : {noms}")

                    for quand in ("avant", "apres"):
                        for sel, nom in (('[data-vendre-valider]', 'conclure'),
                                         ('[data-vendre-annuler]', 'renoncer')):
                            s = pg.evaluate(SONDE, sel)
                            if s["absent"]:
                                fautes.append(f"{ou} : aucun bouton pour {nom} ({quand})")
                            elif not s["ok"]:
                                fautes.append(f"{ou} : {quand} désignation, le bouton "
                                              f"pour {nom} est recouvert par {s['par']}")
                        if quand == "avant":
                            carte = pg.query_selector('#mienne-rang [data-carte-cle]')
                            try:
                                carte.click(timeout=5000)
                            except Exception as ex:
                                fautes.append(f"{ou} : la première carte refuse le clic "
                                              f"({str(ex).splitlines()[0][:70]})")
                                break

                    # ON RENONCE : rien n'est défaussé, la main garde sa longueur,
                    # et la partie repart dans un état jouable.
                    a = pg.query_selector('[data-vendre-annuler]')
                    if a is None:
                        return
                    try:
                        a.click(timeout=5000)
                        pg.wait_for_selector('html:not([data-mode="vente"])', timeout=8000)
                    except Exception as ex:
                        fautes.append(f"{ou} : on ne peut pas renoncer "
                                      f"({str(ex).splitlines()[0][:70]})")

                try:
                    jouer(pg, apres=apres)
                except Exception as ex:
                    detail = "\n".join("   " + f for f in fautes[-6:]) or "   (aucune)"
                    print(f"KO {largeur}x{hauteur} graine {graine} : partie bloquée après "
                          f"{compte['occasions']} occasion(s) : "
                          f"{str(ex).splitlines()[0][:160]}\n{detail}")
                    return 1

                if erreurs:
                    fautes.append(f"{largeur}x{hauteur} : {len(erreurs)} erreur(s) de "
                                  f"page : {erreurs[:2]}")

    print(f"   {compte['occasions']} occasions, {compte['cartes']} cartes sondées, "
          f"main la plus longue {compte['max']} (dont {compte['max_petite']} en "
          f"1280x720), {compte['longues']} occasions à 11 cartes ou plus")

    if (compte["occasions"] < 120 or compte["max"] < 11 or compte["max_petite"] < 12):
        print("KO cet essai n'a pas éprouvé assez pour prouver quoi que ce soit "
              "(exigé : 120 occasions, une main de 11+, une main de 12+ en 1280x720).")
        return 1

    if fautes:
        print(f"KO {len(fautes)} faute(s) sur {compte['occasions']} occasions :")
        for f in fautes[:15]:
            print("   " + f)
        return 1

    print(f"OK la place est réservée aux 4 tailles : {compte['occasions']} occasions, "
          f"{compte['cartes']} cartes toutes désignables, aucun bouton recouvert, "
          f"mains jusqu'à {compte['max']} cartes.")
    return 0


sys.exit(main())

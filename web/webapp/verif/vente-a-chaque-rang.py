#!/usr/bin/env python3
"""BALAYAGE EXHAUSTIF DE LA VENTE — à CHAQUE rang, pas à quelques-uns.

(regles-de-la-vente) Ce contrôle existe à cause d'un défaut que les contrôles
fournis n'ont pas vu : ils vendaient à des rangs choisis par une formule
(`rang % 7 == 0`), soit six rangs sur plus de deux cents. Un point de décision
où la vente fige la partie pouvait donc passer entre les mailles — et il l'a
fait. La leçon est écrite ici en dur : **on vend PARTOUT où la page dit qu'on
peut vendre**, et on vérifie à chaque fois que la partie continue.

CE QUI EST VÉRIFIÉ, à chaque rang où `[data-vendre]` est offert :
  (a) le mode de vente s'ouvre puis SE REFERME (`data-mode="vente"` retombe) ;
  (b) exactement la carte désignée quitte la main, et elle seule ;
  (c) les MC montent ;
  (d) une décision est de nouveau posée — la partie n'est pas figée ;
  (e) la page n'a signalé aucune erreur.

ET LE RENONCEMENT, à chaque rang aussi : ouvrir la vente puis annuler ne
défausse rien, ne change pas les MC, et laisse la partie continuer.

Deux passages, parce qu'ils n'éprouvent pas la même chose :
  · `--mode vendre`  : on vend à CHAQUE occasion. La main s'épuise, mais on
    traverse tous les TYPES de décision qui offrent la vente.
  · `--mode alterner`: on vend un rang sur deux et on annule l'autre, ce qui
    laisse la partie se dérouler plus loin et éprouve le renoncement autant que
    la vente.

Usage :
    PYTHONDONTWRITEBYTECODE=1 python3 web/webapp/verif/vente-a-chaque-rang.py \
        [--graine 2024] [--mode vendre|alterner|annuler] [--max-decisions 400]

Sortie : une ligne par faute, puis un décompte. Code 0 si tout passe.
Le contrôle ÉCHOUE aussi s'il n'a pas éprouvé assez d'occasions : zéro faute sur
zéro occasion ne prouve rien.
"""
import argparse
import os
import sys

RACINE = os.path.dirname(os.path.abspath(__file__))
# `pilote.py` est fourni scellé dans `inputs/checks/` : on emprunte son serveur
# et son navigateur plutôt que d'en écrire un second qui dériverait.
sys.path.insert(0, os.path.join(os.getcwd(), "inputs", "checks"))
from pilote import serveur, page, choix_simple  # noqa: E402

ETAT = """() => {
  const sc = document.querySelector('[data-decision-rang]');
  const main = [...document.querySelectorAll('#mienne-rang [data-carte-cle]')]
    .map(e => e.getAttribute('data-carte-cle'));
  const mc = document.querySelector('[data-valeur="players.0.mc"]');
  const b = document.querySelector('[data-vendre]');
  return {
    rang: sc ? Number(sc.getAttribute('data-decision-rang')) : null,
    forme: sc ? (sc.getAttribute('data-decision-forme') || 'simple') : null,
    type: sc ? sc.getAttribute('data-decision-type') : null,
    choisir: sc ? sc.getAttribute('data-a-choisir') : null,
    mode: document.documentElement.getAttribute('data-mode'),
    aVendre: document.querySelectorAll('[data-a-vendre="oui"]').length,
    main: main,
    mc: mc ? parseInt((mc.textContent || '').replace(/[^0-9-]/g, ''), 10) : null,
    vendable: !!b && b.getBoundingClientRect().width > 0
              && !b.hasAttribute('disabled'),
    panne: (document.querySelector('#panne') || {}).textContent || null,
    fini: !!document.querySelector('[data-partie-terminee]'),
  };
}"""


def repondre(pg, v):
    """Répond à la décision en cours, exactement comme `pilote.jouer`."""
    rang, forme = v["rang"], v["forme"]
    choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if not choix and forme != "montant":
        raise RuntimeError(f"decision {rang} ({v['type']}) : aucun choix visible")
    if forme == "montant":
        ch = pg.wait_for_selector("[data-montant]", timeout=15000)
        mini, maxi = int(ch.get_attribute("min")), int(ch.get_attribute("max"))
        ch.fill(str(mini + (rang % (maxi - mini + 1))))
        pg.click("[data-valider]")
    elif forme == "multiple":
        brut = v["choisir"]
        k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
        for c in choix[: min(k, len(choix))]:
            c.click()
        pg.click("[data-valider]")
    else:
        choix[choix_simple(rang, len(choix))].click()


def attendre_apres_vente(pg, delai=8000):
    """Attend que le mode retombe ET qu'une décision soit de nouveau posée."""
    pg.wait_for_function(
        "() => document.documentElement.getAttribute('data-mode') !== 'vente'"
        " && (document.querySelector('[data-decision-rang]')"
        "     || document.querySelector('[data-partie-terminee]'))",
        timeout=delai)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--graine", type=int, default=2024)
    ap.add_argument("--mode", default="alterner",
                    choices=["vendre", "alterner", "annuler"])
    ap.add_argument("--max-decisions", type=int, default=600)
    ap.add_argument("--minimum", type=int, default=20,
                    help="occasions minimales pour que le balayage prouve quelque chose")
    a = ap.parse_args()

    fautes = []
    # Les dernieres decisions traversees : quand une vente ne passe pas, savoir
    # CE QUI PRECEDE vaut tout le reste du rapport.
    histoire = []
    ventes = 0
    renoncements = 0
    offertes = 0
    vus = 0

    with serveur() as base:
        url = (f"{base}/index.html?graine={a.graine}&siege=0&animations=non")
        with page(url) as (pg, erreurs, externes):
            for _ in range(a.max_decisions):
                if pg.query_selector("[data-partie-terminee]"):
                    break
                try:
                    pg.wait_for_selector("[data-decision-rang]", timeout=15000,
                                         state="attached")
                except Exception as e:
                    fautes.append(
                        f"apres {vus} decisions ({ventes} ventes, {renoncements} "
                        f"renoncements) : plus aucune decision posee -- {e}")
                    break
                v = pg.evaluate(ETAT)
                vus += 1
                rang = v["rang"]
                histoire.append(f"{rang}:{v['type']}")
                histoire[:] = histoire[-10:]

                if v["vendable"] and len(v["main"]) >= 1:
                    offertes += 1
                    veut_vendre = (a.mode == "vendre"
                                   or (a.mode == "alterner" and offertes % 2 == 1))
                    avant_fautes = len(fautes)
                    ok = (vendre(pg, v, fautes) if veut_vendre
                          else renoncer(pg, v, fautes))
                    if len(fautes) > avant_fautes:
                        fautes.append("   ... decisions precedentes : "
                                      + " -> ".join(histoire))
                    if ok:
                        if veut_vendre:
                            ventes += 1
                        else:
                            renoncements += 1
                    else:
                        # Le geste a echoue : inutile de poursuivre en aveugle,
                        # la partie est probablement figee. On le DIT.
                        break
                    v = pg.evaluate(ETAT)
                    if v["rang"] is None and not v["fini"]:
                        fautes.append(
                            f"rang {rang} ({v['type']}) : plus aucune decision posee "
                            f"apres le geste de vente -- la partie est figee")
                        break
                    rang = v["rang"]
                    if rang is None:
                        break

                try:
                    repondre(pg, v)
                except Exception as e:
                    fautes.append(f"rang {rang} : impossible de repondre -- {e}")
                    break
                try:
                    pg.wait_for_function(
                        "r => { const e = document.querySelector('[data-decision-rang]');"
                        " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                        " || document.querySelector('[data-partie-terminee]'); }",
                        arg=rang, timeout=15000)
                except Exception as e:
                    fautes.append(f"rang {rang} : la decision n'a jamais change -- {e}")
                    break

            if erreurs:
                fautes.append(f"la page a signale {len(erreurs)} erreur(s) : {erreurs[:3]}")
            if externes:
                fautes.append(f"chargement depuis l'exterieur : {externes[:3]}")

    print(f"-- graine {a.graine}, mode {a.mode} : {vus} decisions vues, "
          f"{offertes} occasions offertes, {ventes} ventes, "
          f"{renoncements} renoncements, {len(fautes)} faute(s)")
    if offertes < a.minimum:
        print(f"KO seulement {offertes} occasions eprouvees (minimum {a.minimum}) : "
              f"ce balayage n'aurait rien prouve.")
        for f in fautes[:6]:
            print("   cause : " + f)
        return 1
    if fautes:
        print(f"KO {len(fautes)} faute(s) sur {offertes} occasions :")
        for f in fautes[:15]:
            print("   " + f)
        return 1
    print(f"OK la vente aboutit et la partie continue aux {offertes} occasions "
          f"offertes ({ventes} ventes, {renoncements} renoncements).")
    return 0


def vendre(pg, avant, fautes):
    """Vend la DERNIÈRE carte de la main. Rend False si le geste a échoué."""
    rang, typ = avant["rang"], avant["type"]
    pg.click("[data-vendre]")
    if not pg.query_selector('html[data-mode="vente"]'):
        fautes.append(f"rang {rang} ({typ}) : le clic sur le bouton n'ouvre pas "
                      f"le mode vente")
        return False
    visee = avant["main"][-1]
    cible = pg.query_selector(f'#mienne-rang [data-carte-cle="{visee}"]')
    if cible is None:
        fautes.append(f"rang {rang} ({typ}) : la carte « {visee} » a disparu")
        return False
    cible.click()
    if not pg.query_selector(f'[data-carte-cle="{visee}"][data-a-vendre="oui"]'):
        fautes.append(f"rang {rang} ({typ}) : la carte designee n'est pas marquee")
        return False
    v = pg.query_selector("[data-vendre-valider]")
    if v is None:
        fautes.append(f"rang {rang} ({typ}) : aucun bouton de validation")
        return False
    v.click()
    try:
        attendre_apres_vente(pg)
    except Exception as e:
        etat = pg.evaluate(ETAT)
        fautes.append(
            f"rang {rang} ({typ}) : LA VENTE N'ABOUTIT PAS -- mode={etat['mode']!r}, "
            f"marquees={etat['aVendre']}, main {len(avant['main'])}->"
            f"{len(etat['main'])}, decision={etat['rang']} ({e})"
            f"\n      PANNE : {etat['panne']}")
        return False
    apres = pg.evaluate(ETAT)
    partis = [k for k in avant["main"] if k not in apres["main"]]
    if partis != [visee]:
        fautes.append(f"rang {rang} ({typ}) : designee « {visee} », parties {partis}")
    if avant["mc"] is not None and apres["mc"] is not None and apres["mc"] <= avant["mc"]:
        fautes.append(f"rang {rang} ({typ}) : MC {avant['mc']} -> {apres['mc']}")
    return True


def renoncer(pg, avant, fautes):
    """Ouvre la vente, désigne une carte, puis ANNULE. Rien ne doit bouger."""
    rang, typ = avant["rang"], avant["type"]
    pg.click("[data-vendre]")
    if not pg.query_selector('html[data-mode="vente"]'):
        fautes.append(f"rang {rang} ({typ}) : le mode vente ne s'ouvre pas")
        return False
    cible = pg.query_selector(f'#mienne-rang [data-carte-cle="{avant["main"][-1]}"]')
    if cible is not None:
        cible.click()
    b = pg.query_selector("[data-vendre-annuler]")
    if b is None:
        fautes.append(f"rang {rang} ({typ}) : aucun bouton d'annulation")
        return False
    b.click()
    try:
        pg.wait_for_function(
            "() => document.documentElement.getAttribute('data-mode') !== 'vente'",
            timeout=8000)
    except Exception as e:
        fautes.append(f"rang {rang} ({typ}) : le renoncement ne referme pas le mode ({e})")
        return False
    apres = pg.evaluate(ETAT)
    if apres["main"] != avant["main"]:
        fautes.append(f"rang {rang} ({typ}) : RENONCEMENT mais la main a change "
                      f"{len(avant['main'])} -> {len(apres['main'])}")
    if apres["mc"] != avant["mc"]:
        fautes.append(f"rang {rang} ({typ}) : RENONCEMENT mais les MC ont change "
                      f"{avant['mc']} -> {apres['mc']}")
    if apres["aVendre"]:
        fautes.append(f"rang {rang} ({typ}) : RENONCEMENT mais {apres['aVendre']} "
                      f"carte(s) restent marquees")
    if apres["rang"] is None and not apres["fini"]:
        fautes.append(f"rang {rang} ({typ}) : RENONCEMENT et la partie est figee")
        return False
    return True


if __name__ == "__main__":
    sys.exit(main())

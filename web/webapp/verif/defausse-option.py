#!/usr/bin/env python3
"""CNF-2, DERNIÈRE EXIGENCE — L'OPTION S'ÉTEINT VRAIMENT.

« Ce n'est pas une règle officielle : c'est une option de partie, activable ou
non, comme les autres options du panneau. Quand l'option est éteinte, la pile ne
montre rien et la fenêtre ne s'ouvre pas. »

Le contrôle 02 du contrat ne mesure QUE l'option allumée — c'est son état par
défaut, et il ne touche jamais au panneau. Un réglage qui n'éteint rien est un
mensonge, et personne ne le verrait. Ce banc éprouve donc les deux états, PAR LE
CHEMIN DU JOUEUR : il ouvre le panneau d'options et clique l'interrupteur, comme
une main le ferait. Aucun raccourci, aucun paramètre d'adresse.

Trois états mesurés, dans cet ordre :
  1. allumée (le défaut) — une carte face découverte sur la pile, la fenêtre
     s'ouvre au clic ;
  2. éteinte — plus rien sur la pile, et le clic n'ouvre rien ;
  3. rallumée — la carte revient et la fenêtre s'ouvre de nouveau.

Le troisième état compte autant que le second : un interrupteur qui éteint sans
pouvoir rallumer casserait la partie en cours.

Usage : python3 defausse-option.py [racine] [graine]
"""
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICI, ".."))
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page as ouvrir, choix_simple, choix_montant  # noqa: E402

ETAT = """
() => {
  const pile = document.querySelector('[data-defausse]');
  const dessus = document.querySelector('[data-defausse-dessus], [data-defausse] .carte');
  const im = dessus ? dessus.querySelector('img') : null;
  const b = dessus ? dessus.getBoundingClientRect() : null;
  return {
    pile: !!pile,
    dessus: !!dessus,
    nom: im ? (im.getAttribute('alt') || '').trim() : '',
    visible: !!b && b.width > 20 && b.height > 20,
    fenetre: !!document.querySelector('[data-fenetre-defausse]'),
    reglage: (document.querySelector('[data-reglage="defausse"]') || {})
      .getAttribute ? document.querySelector('[data-reglage="defausse"]')
        .getAttribute('data-reglage-etat') : null,
  };
}
"""


def repondre(pg, delai=15000):
    if pg.query_selector("[data-partie-terminee]"):
        return None
    pg.wait_for_selector("[data-decision-rang]", timeout=delai, state="attached")
    porteur = pg.query_selector("[data-decision-rang]")
    rang = int(porteur.get_attribute("data-decision-rang"))
    forme = porteur.get_attribute("data-decision-forme") or "simple"
    choix = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
    if forme == "montant":
        champ = pg.wait_for_selector("[data-montant]", timeout=delai)
        mini, maxi = int(champ.get_attribute("min")), int(champ.get_attribute("max"))
        champ.fill(str(choix_montant(rang, mini, maxi)))
        pg.click("[data-valider]")
    elif forme == "multiple":
        brut = porteur.get_attribute("data-a-choisir")
        k = int(brut) if (brut or "").isdigit() else (rang % max(len(choix), 1)) + 1
        for c in choix[:min(k, len(choix))]:
            c.click()
        pg.click("[data-valider]")
    else:
        if not choix:
            raise RuntimeError(f"decision {rang} : aucun choix visible")
        choix[choix_simple(rang, len(choix))].click()
    pg.wait_for_function(
        "r => { const e = document.querySelector('[data-decision-rang]');"
        " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
        " || document.querySelector('[data-partie-terminee]'); }",
        arg=rang, timeout=delai)
    return rang


def basculer(pg):
    """Éteint ou rallume l'option, par le panneau — le chemin du joueur."""
    pg.click("[data-options-ouvrir]")
    pg.wait_for_selector('[data-options-action="reglages"]', timeout=5000)
    pg.click('[data-options-action="reglages"]')
    bouton = pg.wait_for_selector('[data-reglage="defausse"]', timeout=5000)
    avant = bouton.get_attribute("data-reglage-etat")
    bouton.click()
    pg.wait_for_timeout(120)
    apres = pg.query_selector('[data-reglage="defausse"]').get_attribute("data-reglage-etat")
    pg.click('[data-options-action="reprendre"]')
    pg.wait_for_timeout(160)
    return avant, apres


def cliquer_la_pile(pg):
    cible = pg.query_selector("[data-defausse-dessus], [data-defausse] .carte") \
        or pg.query_selector("[data-defausse]")
    if cible:
        cible.click()
        pg.wait_for_timeout(300)


def main():
    fautes = []
    with serveur(RACINE) as url:
        page_url = f"{url}/?graine={GRAINE}&siege=0&boites=base,decouverte&animations=non"
        with ouvrir(page_url) as (pg, erreurs, _):
            pg.wait_for_selector("[data-decision-rang]", timeout=20000, state="attached")
            # De quoi remplir la pile : sans cartes défaussées, les trois états
            # seraient indiscernables et ce banc ne prouverait rien.
            joues = 0
            for _ in range(120):
                if repondre(pg) is None:
                    break
                joues += 1
                e = pg.evaluate(ETAT)
                if e["dessus"] and e["nom"]:
                    break

            allume = pg.evaluate(ETAT)
            print(f"    {joues} decision(s) jouees pour remplir la pile ; "
                  f"dessus « {allume['nom']} »")
            if not allume["pile"]:
                print("ECHEC : aucune pile de defausse dans la page")
                return 1
            if not (allume["dessus"] and allume["nom"] and allume["visible"]):
                print("ECHEC : option ALLUMEE (le defaut) et rien de lisible sur la "
                      "pile — ce banc ne prouve rien en l'etat")
                return 1
            cliquer_la_pile(pg)
            if not pg.evaluate(ETAT)["fenetre"]:
                fautes.append("option allumee : cliquer la pile n'ouvre pas la fenetre")
            pg.keyboard.press("Escape")
            pg.wait_for_timeout(150)
            # `Escape` ouvre le panneau d'options quand rien d'autre n'est ouvert :
            # on le referme pour repartir d'un ecran propre.
            if pg.query_selector("[data-fenetre-defausse]"):
                pg.query_selector("[data-fermer-defausse]").click()
                pg.wait_for_timeout(150)
            if pg.query_selector("[data-options-panneau]:not([hidden])"):
                pg.click('[data-options-action="reprendre"]')

            # 2. ÉTEINTE.
            avant, apres = basculer(pg)
            if not (avant == "oui" and apres == "non"):
                fautes.append(f"l'interrupteur du panneau ne bascule pas : "
                              f"{avant} -> {apres}")
            eteint = pg.evaluate(ETAT)
            if eteint["dessus"]:
                fautes.append(f"option eteinte et la pile montre encore « {eteint['nom']} » "
                              f"— « la pile ne montre rien » (CNF-2)")
            cliquer_la_pile(pg)
            if pg.evaluate(ETAT)["fenetre"]:
                fautes.append("option eteinte et la fenetre s'ouvre quand meme — "
                              "« la fenetre ne s'ouvre pas » (CNF-2)")

            # 3. RALLUMÉE.
            avant2, apres2 = basculer(pg)
            if not (avant2 == "non" and apres2 == "oui"):
                fautes.append(f"l'interrupteur ne rallume pas : {avant2} -> {apres2}")
            # La pile se repeint au rendu suivant : on joue une decision.
            repondre(pg)
            pg.wait_for_timeout(200)
            rallume = pg.evaluate(ETAT)
            if not (rallume["dessus"] and rallume["nom"] and rallume["visible"]):
                fautes.append("option rallumee et la pile ne remontre rien — "
                              "l'interrupteur eteint sans savoir rallumer")
            cliquer_la_pile(pg)
            if not pg.evaluate(ETAT)["fenetre"]:
                fautes.append("option rallumee et la fenetre ne s'ouvre plus")

            if erreurs:
                fautes.append(f"la page a leve {len(erreurs)} erreur(s) : {erreurs[0]}")

    if fautes:
        print(f"ECHEC : {len(fautes)} defaut(s)")
        for f in fautes:
            print(f"      · {f}")
        return 1
    print("    l'option eteint la pile ET la fenetre, et sait les rallumer")
    return 0


if __name__ == "__main__":
    sys.exit(main())

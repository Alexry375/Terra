#!/usr/bin/env python3
"""RIEN DE CE QUI EST AJOUTÉ NE VOLE UN CLIC — piège 4 du contrat.

Ce banc garde un défaut RÉEL, trouvé par une relecture adversariale puis mesuré,
et corrigé. Il ne décrit pas une peur : il décrit ce qui s'est produit.

CE QUI S'EST PASSÉ. Le dock des deux paquets (`vue/defausse.js`) partage la
colonne de droite avec la planche des océans. En temps ordinaire la planche
n'occupe que le haut de sa case, et les deux cohabitent. Mais quand une tuile
océan est due, la planche passe à DEUX COLONNES et CINQ RANGÉES
(`style-oceans.css`) et réclame presque toute la colonne. En 1100 × 620 — la plus
petite fenêtre du contrat — elle dépassait alors de 73 points sous le dock, qui
passe au-dessus : QUATRE TUILES SUR NEUF ne recevaient plus le clic, et désigner
un emplacement devenait impossible sur ces quatre-là.

CE QUE CE BANC MESURE, et c'est la seule chose qui compte : pour chaque tuile
qu'on peut désigner, `document.elementFromPoint` au CENTRE de la tuile doit
tomber DANS la planche. C'est la question que se pose le navigateur quand un
doigt ou une souris arrive, et c'est celle que Playwright pose avant de cliquer.
Un panneau qui recouvre sans intercepter n'est pas un défaut ; un panneau qui
intercepte en est un, même s'il est transparent.

DEUX TAILLES DE FENÊTRE, dont la plus petite du contrat : le défaut ne se voyait
QUE là. Un banc qui ne tourne que sur la fenêtre la plus confortable ne prouve
rien du reste.

⚠️ IL COMPTE SES OCCASIONS AVANT DE JUGER : sans avoir vu la planche s'ouvrir
pour un choix, il échoue plutôt que de rendre un verdict sur le vide.

Usage : python3 place-des-panneaux.py [racine] [graine]
"""
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICI, ".."))
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page as ouvrir, choix_simple, choix_montant  # noqa: E402

TAILLES = [(1600, 1000), (1100, 620)]

MESURE = """
() => {
  const o = document.getElementById('oceans');
  if (!o || !o.classList.contains('oceans--choix')) return null;
  const tuiles = [...document.querySelectorAll('.ocean')]
    .filter((d) => d.dataset.oceanChoisissable !== undefined)
    .map((d) => {
      const r = d.getBoundingClientRect();
      const el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      // QUI REÇOIT LE CLIC AU CENTRE DE CETTE TUILE ? Le nom de l'élément est
      // relevé pour que le verdict dise QUI vole, pas seulement QU'ON vole.
      return {
        dans: !!(el && el.closest('#oceans')),
        voleur: el ? (el.closest('#paquets') ? '#paquets'
                     : (el.id || el.className || el.tagName)) : 'rien',
      };
    });
  return { tuiles };
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


def main():
    fautes = []
    total_ouvertures = 0
    with serveur(RACINE) as url:
        page_url = f"{url}/?graine={GRAINE}&siege=0&boites=base,decouverte"
        for largeur, hauteur in TAILLES:
            ouvertures = 0
            volees = 0
            voleurs = set()
            with ouvrir(page_url, largeur=largeur, hauteur=hauteur) as (pg, erreurs, _):
                pg.wait_for_selector("[data-decision-rang]", timeout=20000, state="attached")
                for _ in range(200):
                    if repondre(pg) is None:
                        break
                    # La planche ne s'ouvre que le temps du choix : on regarde
                    # tout de suite après la réponse, pas 260 ms plus tard.
                    m = pg.evaluate(MESURE)
                    if not m or not m["tuiles"]:
                        continue
                    ouvertures += 1
                    for t in m["tuiles"]:
                        if not t["dans"]:
                            volees += 1
                            voleurs.add(t["voleur"])
                    if ouvertures >= 3:
                        break
                if erreurs:
                    fautes.append(f"{largeur}x{hauteur} : la page a leve "
                                  f"{len(erreurs)} erreur(s) : {erreurs[0]}")

            print(f"    {largeur}x{hauteur} : la planche s'ouvre {ouvertures} fois "
                  f"pour un choix ; {volees} tuile(s) designable(s) hors d'atteinte")
            total_ouvertures += ouvertures
            if volees:
                fautes.append(f"{largeur}x{hauteur} : {volees} tuile(s) qu'on doit "
                              f"pouvoir designer ne recoivent pas le clic — "
                              f"recouvertes par {sorted(voleurs)}")

    if total_ouvertures < 2:
        print(f"ECHEC : la planche ne s'est ouverte que {total_ouvertures} fois "
              f"en tout — ce banc n'a rien mesure, il ne prouve rien")
        return 1
    if fautes:
        print(f"ECHEC : {len(fautes)} defaut(s)")
        for f in fautes:
            print(f"      · {f}")
        return 1
    print("    chaque tuile qu'on peut designer recoit son clic, aux deux tailles")
    return 0


if __name__ == "__main__":
    sys.exit(main())

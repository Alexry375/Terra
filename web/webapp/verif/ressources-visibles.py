#!/usr/bin/env python3
"""LES RESSOURCES POSEES SUR UNE CARTE SE VOIENT VRAIMENT (LIS-3, premiere moitie).

Corentin, ligne 22 : « on ne voit pas les microbes / animaux / jetons Science
accumules sur une carte ». La pastille EXISTAIT pourtant, et le moteur publie
bien le nombre (`players[].played[].resources`). Ce qu'elle avait, c'est sa
PLACE : le coin haut-droit, c'est-a-dire exactement la partie de la carte que
la suivante de la pile recouvre (`vue/plateau.js` decale la carte i+1 vers la
droite et vers le haut, et la passe par-dessus).

Ce banc ne regarde donc pas si la pastille est dans le document — elle y a
toujours ete — mais si un OEIL LA VOIT. Pour chaque pastille affichee, on
demande au navigateur ce qui se trouve au point ou elle est posee
(`elementFromPoint`, quatre points pris dans sa boite) : si ce qui repond
n'est pas la pastille elle-meme, c'est qu'une autre carte est passee devant.

C'est la mesure qui manquait : un test qui compte les pastilles aurait ete
vert AVANT la correction comme apres.

    python3 verif/ressources-visibles.py <racine-webapp> [graine] [captures]
"""
import os
import sys

RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "web/webapp")
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"
CAPTURES = sys.argv[3] if len(sys.argv) > 3 else None

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page, choix_simple  # noqa: E402

# Pour chaque pastille VISIBLE, ce que le navigateur trouve reellement au-dessus
# d'elle. On interroge quatre points bien a l'interieur de sa boite : un seul
# point central pourrait tomber dans un trou de bordure arrondie.
LECTURE = """() => {
  const out = [];
  for (const p of document.querySelectorAll('.pile .carte--jeu .carte__ressources')) {
    const r = p.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) continue;
    const carte = p.closest('.carte--jeu');
    const points = [
      [r.x + r.width * 0.5, r.y + r.height * 0.5],
      [r.x + r.width * 0.3, r.y + r.height * 0.5],
      [r.x + r.width * 0.7, r.y + r.height * 0.5],
      [r.x + r.width * 0.5, r.y + r.height * 0.7],
    ];
    let couverte = null;
    for (const [x, y] of points) {
      const dessus = document.elementFromPoint(x, y);
      if (dessus === p || p.contains(dessus)) continue;
      // Ce qui repond n'est pas la pastille : on dit QUI la recouvre, et si
      // c'est une autre carte que la sienne.
      const autre = dessus && dessus.closest ? dessus.closest('.carte--jeu') : null;
      couverte = {
        par: dessus ? (dessus.className || dessus.tagName) : 'rien',
        autreCarte: !!(autre && autre !== carte),
      };
      break;
    }
    out.push({
      carte: carte ? carte.getAttribute('data-carte-en-jeu') : null,
      nombre: (p.textContent || '').trim(),
      taille: Math.round(Math.min(r.width, r.height)),
      couverte,
    });
  }
  return out;
}"""

fautes = []
vu = {"pastilles": 0, "decisions": 0}
plus_grand_lot = []


def controle(pg, rang):
    global plus_grand_lot
    lu = pg.evaluate(LECTURE)
    if not lu:
        return
    vu["decisions"] += 1
    if len(lu) > len(plus_grand_lot):
        plus_grand_lot = lu
    for p in lu:
        vu["pastilles"] += 1
        if p["couverte"]:
            fautes.append(
                f"decision {rang} : la pastille « {p['nombre']} » de la carte "
                f"{p['carte']} est recouverte par {p['couverte']['par']!r}"
                + (" (une AUTRE carte de la pile)" if p["couverte"]["autreCarte"] else ""))
        # Une pastille qu'on ne peut pas lire ne vaut pas mieux qu'une pastille
        # cachee. Les piles sont mises a l'echelle : on exige un plancher.
        #
        # POURQUOI 10 ET PAS 20. Ce plancher n'est pas un souhait, c'est ce que
        # la geometrie des piles autorise. La pastille doit tenir dans la bande
        # que la carte suivante laisse decouverte — 40 % de la largeur d'une
        # carte (`DECALAGE_X`) —, et cette bande se reduit avec la pile. Au pire
        # moment de la graine 4242 elle ne fait qu'une douzaine de pixels a
        # l'ecran : exiger davantage, ce serait exiger que la pastille reponde
        # a cette mesure en se glissant de nouveau sous la carte voisine.
        # Mesure avant / apres correction, meme graine, meme instant : 8 / 11.
        if p["taille"] < 10:
            fautes.append(
                f"decision {rang} : la pastille de la carte {p['carte']} ne fait "
                f"que {p['taille']} px de cote — illisible (plancher 10)")


with serveur(RACINE) as base:
    with page(f"{base}/?graine={GRAINE}&siege=0&animations=non") as (pg, erreurs, _):
        pg.wait_for_selector("#horizon", timeout=20000)
        # Les ressources s'accumulent : il faut jouer un moment avant qu'une
        # carte en porte. On joue au clic, comme le joueur.
        for _ in range(420):
            if pg.query_selector("[data-partie-terminee]"):
                break
            porteur = pg.query_selector("[data-decision-rang]")
            if porteur is None:
                pg.wait_for_timeout(100)
                continue
            rang = int(porteur.get_attribute("data-decision-rang"))
            controle(pg, rang)
            forme = porteur.get_attribute("data-decision-forme") or "simple"
            visibles = [c for c in pg.query_selector_all("[data-choix]") if c.is_visible()]
            if forme == "montant":
                champ = pg.wait_for_selector("[data-montant]", timeout=15000)
                champ.fill(champ.get_attribute("min"))
                pg.click("[data-valider]")
            elif forme == "multiple":
                brut = porteur.get_attribute("data-a-choisir")
                k = int(brut) if (brut or "").isdigit() else 1
                for c in visibles[:min(k, len(visibles))]:
                    c.click()
                pg.click("[data-valider]")
            elif visibles:
                visibles[choix_simple(rang, len(visibles))].click()
            else:
                pg.wait_for_timeout(100)
                continue
            pg.wait_for_function(
                "r => { const e = document.querySelector('[data-decision-rang]');"
                " return !e || Number(e.getAttribute('data-decision-rang')) !== r"
                " || document.querySelector('[data-partie-terminee]'); }",
                arg=rang, timeout=15000)

        if CAPTURES:
            os.makedirs(CAPTURES, exist_ok=True)
            pg.screenshot(path=os.path.join(CAPTURES, "ecran.png"))
            for nom, sel in (("plateau-0", "#plateau-0"), ("plateau-1", "#plateau-1")):
                e = pg.query_selector(sel)
                if e:
                    e.screenshot(path=os.path.join(CAPTURES, nom + ".png"))
            print(f"    captures dans {CAPTURES}/")

        if erreurs:
            fautes.append(f"erreurs de console : {erreurs[:2]}")

print(f"    {vu['pastilles']} pastille(s) de ressources vues sur {vu['decisions']} "
      f"decision(s) ; le plus grand lot en portait {len(plus_grand_lot)}")
if vu["pastilles"] == 0:
    print("ECHEC : aucune ressource posee de toute la partie — la mesure n'a pas eu lieu")
    sys.exit(1)
if fautes:
    for f in fautes[:6]:
        print("ECHEC :", f)
    print(f"ECHEC : {len(fautes)} pastille(s) invisibles")
    sys.exit(1)
print("    toutes les pastilles de ressources sont visibles et lisibles")

#!/usr/bin/env python3
"""LA CARTE PHASE COUCHEE EST-ELLE CELLE DE LA MANCHE PRECEDENTE ?

ETAT AU 02-08 : CE BANC EST ROUGE, ET IL FAUT SAVOIR LIRE SON ROUGE.

  234 comparaisons sur 3 parties : 221 justes, 13 fautes, et ZERO recopie —
  jamais la carte couchee ne vaut la carte de la manche courante. L'ecran se
  souvient donc bel et bien ; il ne recopie pas.

  Les 13 ecarts se concentrent sur les manches tardives et coincident avec le
  RECUL DE L'ETAT DU MOTEUR (defaut distinct, non corrige : voir
  docs/CTO_STATE.md). Ce banc ne pourra etre tenu pour concluant qu'une fois ce
  recul repare — c'est l'ordre a respecter, et non l'inverse.

Point reste en suspens a l'audit du 02-08. Mes deux premiers oracles etaient
faux, et pour la meme raison tous les deux : ils comparaient la carte couchee a
un releve pris DANS le fil du jeu, decision apres decision, alors que l'etat du
moteur RECULE par moments (defaut connu, distinct de celui-ci : 20 reculs sur
183 lectures, graine 5150 ; `verif/tests.mjs` echoue deja dessus). Une lecture
perimee suffisait a fabriquer une fausse faute.

Ce banc-ci ne compare plus rien dans le fil du jeu. Il RELEVE d'abord, il
CONCLUT ensuite :

  1. a chaque decision, on note le numero de manche, la carte DEBOUT de chaque
     joueur et sa carte COUCHEE. Toute lecture dont le numero de manche est
     inferieur au plus haut deja vu est ecartee : elle est perimee ;
  2. la partie finie, on exige, pour chaque manche `m` et chaque joueur : la
     carte couchee en `m` est celle qui etait debout en `m - 1`.

Aucune connaissance du moteur n'entre la-dedans : l'ecran est compare a
lui-meme. Si l'ecran couchait la carte de la manche COURANTE, ou celle d'il y a
deux manches, ou toujours la meme, la comparaison le dirait — et le banc verifie
d'ailleurs explicitement que la couchee n'est pas la carte courante, pour qu'un
ecran qui se contenterait de recopier ne passe pas.

Depuis la racine du workspace :  python3 outputs/verif/carte-couchee.py [graines...]
Depuis la racine du depot :      python3 web/webapp/verif/carte-couchee.py [graines...]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pilote import echec, jouer, page, serveur  # noqa: E402

GRAINES = [2024, 4242, 5150]

LECTURE = """
() => {
  const n = document.querySelector('[data-valeur="generation"]');
  const manche = n ? Number(n.textContent.replace(/[^0-9-]/g, '')) : null;
  const lire = (sel) => {
    const out = {};
    for (const e of document.querySelectorAll(sel)) {
      const v = e.dataset.phasePosee || '';       // « joueur:phase »
      const [j, p] = v.split(':');
      if (j !== undefined && p) out[j] = Number(p);
    }
    return out;
  };
  return {
    manche,
    // Les cartes DEBOUT portent `data-phase-choisie` ; les COUCHEES portent
    // `data-phase-precedente`. Les deux portent `data-phase-posee`.
    debout: lire('[data-phase-posee][data-phase-choisie]'),
    couchee: lire('[data-phase-posee][data-phase-precedente]'),
  };
}
"""


def relever(base, graine, compte):
    """Joue une partie entiere et rend {manche: {'debout': …, 'couchee': …}}."""
    debout, couchee = {}, {}
    plus_haute = [0]

    def controle(pg, _rang):
        e = pg.evaluate(LECTURE)
        m = e["manche"]
        if m is None:
            return
        if m < plus_haute[0]:
            compte["perimees"] += 1
            return
        plus_haute[0] = m
        debout.setdefault(m, {}).update(e["debout"])
        couchee.setdefault(m, {}).update(e["couchee"])

    with page(f"{base}/?graine={graine}&siege=0&animations=non") as (pg, err, _):
        jouer(pg, apres=controle)
        if err:
            raise RuntimeError(f"graine {graine} : exception : {err[0]}")
    return debout, couchee


def main():
    graines = [int(a) for a in sys.argv[1:]] or GRAINES
    fautes = []
    compte = {"comparaisons": 0, "perimees": 0, "recopies": 0}

    with serveur() as base:
        for graine in graines:
            debout, couchee = relever(base, graine, compte)
            for m in sorted(couchee):
                for j, c in couchee[m].items():
                    attendue = debout.get(m - 1, {}).get(j)
                    if attendue is None:
                        continue   # la manche d'avant n'a pas ete relevee
                    compte["comparaisons"] += 1
                    if c != attendue:
                        courante = debout.get(m, {}).get(j)
                        quoi = (" — c'est la carte de la manche COURANTE, "
                                "l'ecran recopie au lieu de se souvenir"
                                if c == courante else "")
                        if c == courante:
                            compte["recopies"] += 1
                        fautes.append(
                            f"graine {graine}, manche {m}, joueur {j} : la carte "
                            f"couchee est la phase {c}, la manche precedente "
                            f"avait choisi la phase {attendue}{quoi}")

    print(f"{compte['comparaisons']} comparaisons, "
          f"{compte['perimees']} lecture(s) perimee(s) ecartee(s), "
          f"{len(fautes)} faute(s) dont {compte['recopies']} recopie(s)")
    if compte["comparaisons"] < 40:
        echec("trop peu de comparaisons : ce banc n'aurait rien prouve")
    if fautes:
        for f in fautes[:8]:
            print("  " + f)
        echec("la carte couchee n'est pas celle de la manche precedente")
    print("OK la carte couchee est bien celle que le joueur avait choisie "
          "a la manche d'avant")
    return 0


if __name__ == "__main__":
    sys.exit(main())

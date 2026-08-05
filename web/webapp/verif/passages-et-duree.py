#!/usr/bin/env python3
"""ANI-2, ANI-3, ANI-4 — LES PASSAGES SE VOIENT, ET LE « +3 » SE LIT.

Ce banc mesure les trois demandes du quatrième travail, dont une que le contrôle
03 du contrat déclare explicitement NE PAS mesurer.

  · ANI-2 — LE PASSAGE DE MAIN. Le contrôle 03 dit : « la page ne publie rien
    qui désigne le siège à qui la question s'adresse », et il a raison pour la
    SCÈNE — elle n'est dessinée que pour le siège regardé, son `data-joueur` ne
    change donc jamais (mesuré : 0 changement sur 172 décisions). Mais
    `interface.js` écrit `body[data-actif]` à CHAQUE décision, celles d'en face
    comprises, et depuis toujours. C'est cette marque-là qu'on suit.

    ET ON LA SUIT EN CONTINU, pas au rythme des réponses : entre deux réponses du
    siège regardé, la main passe à l'adversaire et revient, et un relevé par
    décision ne verrait jamais rien. L'espion horodate donc chaque changement et
    chaque mouvement, et le banc exige qu'un mouvement suive le passage dans les
    400 ms — le temps qu'il faut pour qu'un joueur le voie.
  · ANI-3 — LE DÉBUT D'UNE PHASE, mesuré comme le contrôle 03 le mesure, mais sur
    une AUTRE GRAINE : un banc qui ne tourne que là où il a été écrit ne prouve
    que cet endroit-là.
  · ANI-4 — LA DURÉE DU « +3 ». Le nombre qui monte du bac de mégacrédits est
    chronométré, de son apparition à son retrait. Il doit rester à l'écran assez
    longtemps pour être lu — au moins trois secondes.

ORACLE DISJOINT : aucune ligne de `vue/anim.js`, `vue/annonce.js` ou
`vue/table.js` n'est lue. Le banc ne regarde que ce que la page publie
(`body[data-actif]`, `data-phase-en-cours`, `data-phase-posee`) et ce qui remue.

⚠️ IL COMPTE SES OCCASIONS AVANT DE JUGER, famille par famille.

Usage : python3 passages-et-duree.py [racine] [graine]
"""
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ICI, ".."))
GRAINE = sys.argv[2] if len(sys.argv) > 2 else "4242"

sys.path.insert(0, os.path.join(RACINE, "verif"))
from pilote import serveur, page as ouvrir, choix_simple, choix_montant  # noqa: E402

# L'espion compte ce qui remue, et chronomètre les « +N » du bac.
ESPION = """
window.__p = { remues: 0, gains: [], mains: [], mouvements: [] };
function remuer() { window.__p.remues++; window.__p.mouvements.push(performance.now()); }
new MutationObserver((ms) => {
  for (const m of ms) {
    if (m.attributeName !== 'data-actif') continue;
    const v = document.body.dataset.actif;
    const d = window.__p.mains;
    if (v !== undefined && v !== '' && (!d.length || d[d.length - 1].qui !== v))
      d.push({ qui: v, t: performance.now() });
  }
// On observe `document` et non `document.documentElement` : au moment ou ce
// script tourne — avant le premier script de la page — l'element racine n'existe
// pas encore, et l'observateur refusait de demarrer.
}).observe(document, { attributes: true, subtree: true,
                       attributeFilter: ['data-actif'] });
new MutationObserver((ms) => {
  for (const m of ms) {
    for (const n of m.addedNodes || []) {
      if (n.nodeType !== 1) continue;
      if (n.parentElement && n.parentElement.id === 'vol') remuer();
      if (n.classList && n.classList.contains('gain')) {
        const g = { texte: n.textContent, ne: performance.now(), meurt: null };
        window.__p.gains.push(g);
        n.__suivi = g;
      }
    }
    for (const n of m.removedNodes || []) {
      if (n.nodeType === 1 && n.__suivi && n.__suivi.meurt === null)
        n.__suivi.meurt = performance.now();
    }
    if (m.type === 'attributes') {
      if (m.target.id === 'annonce' && m.target.classList.contains('annonce--vive'))
        remuer();
      if (m.attributeName === 'data-prend-la-main'
          && m.target.dataset.prendLaMain === 'oui') remuer();
    }
  }
}).observe(document, { childList: true, subtree: true, attributes: true,
                       attributeFilter: ['class', 'data-prend-la-main'] });
"""

LECTURE = """
() => {
  const p = document.querySelector('[data-phase-en-cours]');
  return {
    phase: p ? (p.getAttribute('data-phase-posee') || 'oui') : null,
    remues: window.__p.remues,
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


def main():
    fautes = []
    with serveur(RACINE) as url:
        page_url = f"{url}/?graine={GRAINE}&siege=0&boites=base,decouverte"
        with ouvrir(page_url) as (pg, erreurs, _):
            pg.add_init_script(ESPION)
            pg.goto(page_url, wait_until="domcontentloaded")
            pg.wait_for_selector("[data-decision-rang]", timeout=20000, state="attached")
            avant = pg.evaluate(LECTURE)
            phases = 0
            phases_muettes = []
            rangs = 0
            for _ in range(2000):
                rang = repondre(pg)
                if rang is None:
                    break
                pg.wait_for_timeout(260)
                apres = pg.evaluate(LECTURE)
                remue = apres["remues"] - avant["remues"]
                if apres["phase"] is not None and apres["phase"] != avant["phase"]:
                    phases += 1
                    if remue == 0:
                        phases_muettes.append(rang)
                avant = apres
                rangs += 1
            gains = pg.evaluate("() => window.__p.gains")
            mains = pg.evaluate("() => window.__p.mains")
            mouvements = pg.evaluate("() => window.__p.mouvements")
            if erreurs:
                print(f"ECHEC : la page a leve {len(erreurs)} erreur(s) : {erreurs[0]}")
                return 1

    # ANI-2 : chaque passage de main doit etre suivi d'un mouvement dans les
    # 400 ms. Le premier passage de la partie n'en est pas un — personne n'avait
    # la parole avant lui.
    passages = max(0, len(mains) - 1)
    passages_muets = []
    for m in mains[1:]:
        if not any(m["t"] <= t <= m["t"] + 400 for t in mouvements):
            passages_muets.append(round(m["t"]))

    vecus = [g for g in gains if g["meurt"]]
    durees = sorted(round(g["meurt"] - g["ne"]) for g in vecus)
    plus_trois = [g for g in gains if g["texte"].strip() == "+3"]

    print(f"    {rangs} decision(s) jouees a l'ecran, graine {GRAINE}")
    print(f"      passages de main   {passages:4d}, {len(passages_muets):4d} sans que rien ne bouge")
    print(f"      debuts de phase    {phases:4d}, {len(phases_muettes):4d} sans que rien ne bouge")
    print(f"      « +N » du bac      {len(gains):4d} pose(s), dont {len(plus_trois)} « +3 » ; "
          f"{len(vecus)} chronometre(s), la plus courte {durees[0] if durees else '—'} ms")

    if rangs < 40:
        print(f"ECHEC : {rangs} decision(s) seulement — la partie ne s'est pas jouee")
        return 1
    if passages < 20:
        print(f"ECHEC : {passages} passage(s) de main observe(s) — trop peu pour juger")
        return 1
    if phases < 10:
        print(f"ECHEC : {phases} debut(s) de phase observe(s) — trop peu pour juger")
        return 1
    if len(vecus) < 10:
        print(f"ECHEC : {len(vecus)} « +N » chronometre(s) — trop peu pour juger de leur duree")
        return 1

    if passages_muets:
        fautes.append(f"{len(passages_muets)} passage(s) de main sur {passages} sans "
                      f"que rien ne bouge (aux instants {passages_muets[:5]} ms) — ANI-2")
    if phases_muettes:
        fautes.append(f"{len(phases_muettes)} debut(s) de phase sur {phases} sans que "
                      f"rien ne bouge (rangs {phases_muettes[:5]}) — ANI-3")
    # ANI-4 : « il doit durer assez longtemps pour etre lu ».
    courts = [d for d in durees if d < 3000]
    if courts:
        fautes.append(f"{len(courts)} « +N » sur {len(vecus)} durent moins de 3 s "
                      f"(la plus courte {courts[0]} ms) — ANI-4, « il passe trop vite »")
    if not plus_trois:
        fautes.append("aucun « +3 » vu de la partie : la vente d'une carte n'a pas eu "
                      "lieu, ce banc ne prouve rien sur ANI-4")

    if fautes:
        print(f"ECHEC : {len(fautes)} defaut(s)")
        for f in fautes:
            print(f"      · {f}")
        return 1
    print("    le tour qui passe se voit, la phase qui s'ouvre se voit, et le "
          "« +N » reste assez longtemps pour etre lu")
    return 0


if __name__ == "__main__":
    sys.exit(main())

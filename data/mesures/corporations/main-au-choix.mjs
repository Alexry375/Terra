#!/usr/bin/env node
// L'IA LIT-ELLE SA MAIN QUAND ELLE CHOISIT SA CORPORATION ?
//
//   node data/mesures/corporations/main-au-choix.mjs <donnes> > main-au-choix.jsonl
//   (APPRENTI_POIDS designe le niveau)
//
// Le classement des corporations est une MOYENNE : il dit ce que vaut une
// corporation sur une main quelconque. Il ne peut pas dire « Saturn Systems
// cesse d'etre la pire quand vous tenez trois cartes Jupiter ».
//
// Or le choix de corporation tombe A L'ETAPE 5 de la mise en place
// (flow.rs:227), cartes projets DEJA en main, et la main figure dans la
// description transmise au reseau (description.rs:317-350). L'information est
// donc disponible. Ce banc verifie si elle est UTILISEE : on enregistre la
// paire proposee, le choix fait, et le contenu exact de la main a cet instant.
//
// On s'arrete des que les deux sieges ont choisi : jouer la suite ne changerait
// pas le releve, et coute cent fois plus cher.
const RACINE = "/home/alexis/Global/Agents_Projects/Terra/web/webapp";
const DONNES = Number(process.argv[2] || 500);
const BOITES = "base,decouverte";

const { ouvrirPontDepuis } = await import(`${RACINE}/pont.js`);
const { creerPartie } = await import(`${RACINE}/partie.js`);
const { fournisseurApprenti } = await import(`${RACINE}/joueurs/apprenti.js`);
const pont = await ouvrirPontDepuis(RACINE);

const EST_CHOIX = (q) => /choisissez votre corporation/i.test(q || "");

for (let g = 1; g <= DONNES; g++) {
  const f = [
    fournisseurApprenti(g * 7 + 1, "a", undefined, pont, BOITES),
    fournisseurApprenti(g * 13 + 3, "b", undefined, pont, BOITES),
  ];
  const partie = creerPartie(pont, { graine: g, boites: BOITES });
  let faits = 0, garde = 0;
  while (!partie.termine && faits < 2 && ++garde < 60) {
    const d = partie.decision;
    if (!d) break;
    // La main est lue AVANT de repondre : apres, le moteur a deja avance.
    const main = (partie.etat?.players?.[d.joueur]?.hand || []).map((c) => c.name);
    const r = await f[d.joueur].decider(d, partie.etat);
    const i = typeof r === "number" ? r : (r?.indice ?? -1);
    if (EST_CHOIX(d.question)) {
      console.log(JSON.stringify({
        graine: g,
        siege: d.joueur,
        proposees: (d.options || []).map((o) => o.libelle),
        prise: (d.options || [])[i]?.libelle ?? null,
        main,
      }));
      faits++;
    }
    partie.repondre(r);
  }
}

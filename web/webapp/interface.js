// L'interface du bac à sable — fonctionnelle et laide, par contrat.
//
// Elle ne connaît AUCUNE règle. Elle affiche :
//   • l'état, tel que `observe::state_view` le rend (clefs en anglais, ce sont
//     celles du moteur — on ne les renomme pas, on ne les recalcule pas) ;
//   • la décision attendue, telle que le pont la décrit, avec ses options.
// Et elle renvoie au moteur l'indice (ou le montant, ou la liste) choisi.
//
// Le joueur humain est branché par un « fournisseur de décisions »
// (`fournisseurs.js`), exactement comme le sera un cerveau artificiel ou un
// joueur distant. Ici, le même fournisseur tient les DEUX joueurs : c'est le
// mode bac à sable, même écran, mains visibles.

import { ouvrirPontDepuis } from "./pont.js";
import { creerPartie, jouerJusquAuBout } from "./partie.js";
import { fournisseurHumain, formeDeLaReponse, nombreDeChoix } from "./fournisseurs.js";

const $ = (id) => document.getElementById(id);
const zoneDecision = $("decision");
const zoneEtat = $("etat");
const zoneErreur = $("erreur");

function vider(n) {
  while (n.firstChild) n.removeChild(n.firstChild);
  return n;
}

function el(nom, texte, parent) {
  const n = document.createElement(nom);
  if (texte !== undefined && texte !== null) n.textContent = String(texte);
  if (parent) parent.appendChild(n);
  return n;
}

// -------------------------------------------------------------- affichage état

function ligne(table, cle, valeur) {
  const tr = el("tr", null, table);
  el("th", cle, tr).style.textAlign = "left";
  el("td", valeur, tr);
}

function listeCartes(cartes) {
  return cartes
    .map((c) => {
      const r = c.resources ? ` [${c.resources}]` : "";
      return `${c.name} (${c.couleur}, ${c.price})${r}`;
    })
    .join(", ");
}

function afficherEtat(etat, decision) {
  const z = vider(zoneEtat);
  el("h2", "État de la partie (rendu par le moteur)", z);

  const g = el("table", null, z);
  ligne(g, "manche", etat.generation);
  ligne(g, "premier joueur", `J${etat.first_player}`);
  ligne(g, "partie terminée", etat.game_over ? "oui" : "non");
  const p = etat.planet;
  ligne(g, "température", `${p.temperature} / ${p.temperature_max}`);
  ligne(g, "oxygène", `${p.oxygen} / ${p.oxygen_max}`);
  ligne(g, "océans", `${p.oceans} / ${p.oceans_max}`);
  ligne(g, "infrastructure", p.infrastructure);
  ligne(g, "pioche / défausse", `${etat.decks.deck} / ${etat.decks.discard}`);
  ligne(
    g,
    "objectifs",
    // `achieved_by` est un drapeau PAR JOUEUR (`[bool; 2]`) : un Objectif peut
    // être revendiqué par les deux.
    etat.milestones
      .map((m) => {
        const pris = m.achieved_by
          .map((oui, j) => (oui ? `J${j}` : null))
          .filter(Boolean);
        return `${m.kind}${pris.length ? " → " + pris.join(" et ") : ""}`;
      })
      .join(" · ")
  );
  ligne(g, "récompenses", etat.awards.join(" · "));

  const t = el("table", null, z);
  const entete = el("tr", null, t);
  el("th", "", entete);
  for (const j of etat.players) {
    const th = el("th", `Joueur ${j.player}`, entete);
    if (decision && decision.joueur === j.player) th.className = "attention";
  }
  const rang = (cle, lire) => {
    const tr = el("tr", null, t);
    el("th", cle, tr).style.textAlign = "left";
    for (const j of etat.players) el("td", lire(j), tr);
  };
  rang("corporation", (j) => j.corporation || "—");
  rang("MC", (j) => j.mc);
  rang("chaleur", (j) => j.heat);
  rang("plantes", (j) => j.plants);
  rang("NT", (j) => j.tr);
  rang("forêts", (j) => j.forests);
  rang("production", (j) =>
    `MC ${j.production.mc} · chaleur ${j.production.heat} · plantes ${j.production.plants} · cartes ${j.production.cards}`
  );
  rang("savoir-faire", (j) => `acier ${j.steel_capacity} · titane ${j.titanium_capacity}`);
  rang("phase choisie", (j) => `${j.chosen_phase} (précédente : ${j.previous_phase})`);
  rang("phases améliorées", (j) => j.phase_upgrades.join(" ") || "—");
  rang("badges", (j) =>
    Object.entries(j.tags)
      .filter(([, n]) => n > 0)
      .map(([b, n]) => `${b}×${n}`)
      .join(" ") || "—"
  );
  rang("score courant", (j) => j.score);
  rang("main", (j) => listeCartes(j.hand) || "—");
  rang("cartes posées", (j) => listeCartes(j.played) || "—");
}

// ---------------------------------------------------------- affichage décision

/**
 * Dessine la décision et rend une promesse résolue avec la réponse attendue par
 * le moteur. La page ne juge JAMAIS la légalité d'un choix : elle n'offre que
 * les options que le moteur a énumérées.
 */
function demanderAuJoueur(d) {
  return new Promise((resolve) => {
    const z = vider(zoneDecision);
    el("h2", `Joueur ${d.joueur} — ${d.question}`, z).className = "attention";
    el("p", `(décision n°${d.rang} · ${d.type})`, z);

    if (d.carte) el("p", `carte : ${d.carte.nom}`, z);
    if (d.corporations) {
      el("p", "corporations : " + d.corporations.map((c) => c.nom).join(", "), z);
    }
    if (d.main) {
      el("p", "main : " + d.main.map((c) => `${c.nom} (${c.prix})`).join(", "), z);
    }

    const forme = formeDeLaReponse(d);

    if (forme === "montant") {
      const min = d.minimum ?? 0;
      const max = d.maximum ?? 0;
      const entree = el("input", null, z);
      entree.type = "number";
      entree.min = String(min);
      entree.max = String(max);
      entree.value = String(min);
      const b = el("button", `Valider (${min} à ${max})`, z);
      b.onclick = () => {
        const v = Number(entree.value);
        if (!Number.isInteger(v) || v < min || v > max) return;
        resolve(v);
      };
      return;
    }

    if (forme === "multiple") {
      el("p", `à choisir : ${d.a_choisir} parmi ${d.options.length}`, z);
      const cases = [];
      const ul = el("ul", null, z);
      const b = el("button", "Valider", z);
      const choisis = () => cases.map((c, i) => (c.checked ? i : -1)).filter((i) => i >= 0);
      // Le bouton reste inerte tant que le compte n'y est pas : sans cela, un
      // clic sans effet ressemble à une page bloquée.
      const rafraichir = () => { b.disabled = choisis().length !== d.a_choisir; };
      d.options.forEach((o, i) => {
        const li = el("li", null, ul);
        const c = el("input", null, li);
        c.type = "checkbox";
        c.onchange = rafraichir;
        cases.push(c);
        el("span", " " + (o.libelle ?? `option ${i}`), li);
      });
      rafraichir();
      b.onclick = () => {
        if (choisis().length === d.a_choisir) resolve(choisis());
      };
      return;
    }

    // Choix simple : une option = un bouton. « Passer » est l'indice suivant
    // la dernière option, quand le moteur l'autorise.
    const total = nombreDeChoix(d);
    (d.options || []).forEach((o, i) => {
      const b = el("button", o.libelle ?? `option ${i}`, z);
      b.onclick = () => resolve(i);
    });
    if (d.passer) {
      const b = el("button", "Passer", z);
      b.onclick = () => resolve(total - 1);
    }
  });
}

// -------------------------------------------------------------------- lancement

$("commencer").onclick = async () => {
  zoneErreur.textContent = "";
  $("chargement").textContent = " chargement du moteur…";
  try {
    const pont = await ouvrirPontDepuis(".");
    $("chargement").textContent = " moteur chargé.";
    $("commencer").disabled = true;

    const partie = creerPartie(pont, {
      graine: Number($("graine").value) || 0,
      boites: $("boites").value,
    });

    // Le MÊME fournisseur pour les deux joueurs : bac à sable, même écran.
    // Remplacer l'un des deux suffit à brancher un autre mode (adversaire.md).
    const humain = fournisseurHumain(demanderAuJoueur);
    await jouerJusquAuBout(partie, [humain, humain], (p) =>
      afficherEtat(p.etat, p.decision)
    );

    afficherEtat(partie.etat, null);
    const z = vider(zoneDecision);
    el("h2", "Partie terminée", z);
    el(
      "p",
      `Scores : J0 ${partie.scores[0]} — J1 ${partie.scores[1]} · ` +
        `${partie.manches} manches · ${partie.decisions.length} décisions` +
        (partie.partieComplete
          ? ""
          : " · partie ARRÊTÉE par le plafond du moteur (non terminée par les règles)"),
      z
    );
  } catch (e) {
    zoneErreur.textContent = "Erreur : " + (e && e.message ? e.message : e);
    throw e;
  }
};

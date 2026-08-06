// LE PREMIER JOUEUR QUI RÉFLÉCHIT — `reflechi`.
//
// Il ne connaît AUCUNE règle du jeu, comme tout fournisseur (`adversaire.md`) :
// il choisit parmi ce que le moteur vient d'énumérer. Ce qu'il ajoute au hasard,
// c'est une ÉCHELLE DE VALEUR — quelques dizaines de lignes qui disent, à chaque
// point de décision, quelle option vaut mieux qu'une autre.
//
// ─────────────────────────────────────────────────────────────────────────────
// LA RÈGLE QUI DOMINE CE FICHIER : IL NE REGARDE PAS LA MAIN D'EN FACE.
//
// Le moteur publie les DEUX mains dans l'état (`engine/src/observe.rs`, « Mode
// bac à sable : les DEUX mains sont visibles »). Rien ne l'empêcherait donc de
// lire les cartes de son adversaire — et il paraîtrait brillant sans l'être : ce
// qu'il aurait appris serait intransposable à une partie contre un humain, où
// ces cartes ne seront pas connues.
//
// Le parti pris de ce fichier est plus strict encore que le contrat : il ne lit
// **que son propre siège** et **ce qui n'appartient à personne** (les paramètres
// de la planète, la pioche, la manche). Il ne lit pas même les cartes POSÉES de
// l'adversaire, pourtant publiques. Une seule fonction touche à `etat.players`,
// `monSiege()`, et elle prend le siège qui décide, jamais un autre. Tout le
// reste du fichier travaille sur ce qu'elle rend. C'est vérifiable en une
// lecture, et c'est éprouvé de l'extérieur par
// `inputs/checks/03-il-ne-regarde-pas-les-cartes-d-en-face.sh`, qui repose la
// même question avec une autre main d'en face et compare les réponses.
//
// ─────────────────────────────────────────────────────────────────────────────
// IL N'A AUCUNE MÉMOIRE, ET C'EST VOULU.
//
// `decider(d, etat)` est une FONCTION PURE : même décision + même état = même
// réponse, toujours, quel que soit ce qui a été joué avant. Trois conséquences,
// et les trois comptent :
//
//   · il ne peut pas « reconnaître la graine 42 » et réciter une suite de coups
//     qui a marché : il ne sait pas quelle partie il joue, ni la précédente ;
//   · il ne tire rien au sort — la graine reçue à la fabrication est acceptée
//     pour respecter la signature des autres fournisseurs, et ignorée ;
//   · le banc qui lui repose deux fois la même question obtient deux fois la
//     même réponse, ce qui est exactement ce que le contrôle 03 mesure.
//
// Il n'explore pas non plus l'arbre des possibles : il ne rejoue rien, n'appelle
// pas le pont, et ne regarde qu'un coup — celui qu'on lui demande.
//
// ─────────────────────────────────────────────────────────────────────────────
// D'OÙ VIENNENT LES NOMBRES DE `REGLAGES` — ET CE QUI N'A PAS ÉTÉ MESURÉ.
//
// Ce qui a été mesuré, exactement : NEUF variantes, chacune sur 24 parties
// (12 graines × 2 sièges) contre le joueur témoin, sur les **graines 5000 à
// 5011** — disjointes des graines du contrat (1 à 100), pour qu'aucun réglage ne
// soit choisi sur les parties qu'on va montrer. Ces variantes ont fixé les
// treize poids du choix de phase, le sens du prix à la pose et en main, et le
// seuil du mulligan. À 24 parties, le bruit vaut environ ±10 points de taux :
// c'est l'écart de score moyen, moins bruyant, qui a départagé.
//
// Ce qui n'a PAS été mesuré, et qui est donc choisi à la main : la valeur d'un
// point de victoire (`pvEnMain`, `pvALaPose`), celle d'un badge, le barème des
// actions standard (`forêtPlantes` … `actionDeCarte`), les poids de
// `valeurProduction`, et les quelques constantes écrites dans les branches
// (`corp_mulligan`, `reduction_plantes`, `construction_bonus`,
// `bonus_selectionneur`, `valeurDuLibelle`). Elles sont plausibles, elles ne
// sont pas prouvées.
//
// CE QUE ÇA DONNE, sur les cent graines du contrat jouées aux deux sièges :
// 189 victoires sur 200 contre le hasard (94,5 %), 142 sur 200 contre le joueur
// témoin. Les graines 1 à 100 n'ont servi qu'à cette mesure finale, jamais au
// réglage.

import { formeDeLaReponse } from "../fournisseurs.js";

// ────────────────────────────────────────────────────────────── les réglages

/**
 * Les poids de l'échelle de valeur. Mesurés, pas déduits (voir l'en-tête).
 * Chacun est commenté par ce qu'il pèse, pour qu'on puisse le contester.
 */
export const REGLAGES = {
  // — La valeur d'une carte projet, en main comme sur la table.
  // Un point de victoire écrit sur la carte est le seul gain certain qu'on
  // puisse lire sans connaître les règles ; le prix est le meilleur indice
  // disponible de sa puissance, et en même temps ce qu'elle coûtera.
  pvEnMain: 6, // ce que vaut 1 PV imprimé, quand on garde la carte
  prixEnMain: 0.35, // ce que coûte 1 MC de prix, quand on garde la carte
  pvALaPose: 7, // ce que vaut 1 PV imprimé, quand on choisit quoi poser
  prixALaPose: 0.3, // à la pose, on PRÉFÈRE la plus chère des cartes payables
  badgeEnMain: 0.8, // un badge de plus, c'est une synergie de plus

  // — Le choix de la carte Phase (la décision la plus fréquente de la partie).
  devBase: 1.0, // I — Développement
  devParCarte: 5.0, // … par carte VERTE payable en main (au plus 3)
  conBase: 1.0, // II — Construction
  conParCarte: 5.0, // … par carte BLEUE ou ROUGE payable en main (au plus 3)
  actBase: 0, // III — Action : jamais pour elle-même, seulement pour ce qui suit
  actParMc: 0.05, // … l'action standard se paie en MC : plus on est riche, mieux
  actParPlante: 0.8, // … et une forêt payée en plantes ne coûte pas un MC
  proBase: 7.0, // IV — Production
  proDecroissance: 0.3, // … par point de production déjà acquis (elle sature)
  proFinDePartie: 7.0, // … et ne vaut plus rien quand la planète est finie
  recBase: 3.0, // V — Recherche
  recParCarteManquante: 3.5, // … par carte manquante sous `mainVisee`
  mainVisee: 7, // la main qu'on cherche à tenir
  seuilMulligan: 0, // la barre sous laquelle une carte de départ est remplacée

  // — Les actions standard, en phase Action.
  forêtPlantes: 100, // une forêt payée en plantes : PV + oxygène, sans MC
  forêtMc: 62,
  ocean: 52,
  temperature: 48,
  actionDeCarte: 40,
  passer: 0,

  // — La vente (voir « CE JOUEUR VEND », plus bas). Mesurés sur les graines
  // 7000 à 7059, jamais sur les graines 1 à 100 de la mesure finale.
  seuilVente: 0, // on vend une carte dont la valeur en main tombe sous ce seuil
  gardeMini: 4, // … mais jamais au point d'avoir moins de tant de cartes
};

// FIGÉS POUR DE BON. Sans ce gel, n'importe quel importateur pourrait réécrire
// l'échelle de valeur du joueur livré — et la mesure d'hier ne dirait plus rien
// du joueur d'aujourd'hui.
Object.freeze(REGLAGES);

// ───────────────────────────────────────────────── ce que le joueur a le droit de voir

/**
 * MON SIÈGE, ET RIEN QUE LUI. C'est la seule fonction du fichier qui touche à
 * `etat.players` ; elle prend le siège qui décide (`decision.joueur`) et ne
 * cherche jamais un autre joueur. Tout ce qui suit ne travaille que sur ce
 * qu'elle rend : la main d'en face n'entre donc nulle part.
 */
function monSiege(etat, siege) {
  const tous = (etat && etat.players) || [];
  const moi = tous.find((p) => p && p.player === siege) || tous[siege] || {};
  const main = Array.isArray(moi.hand) ? moi.hand : [];
  const payable = Array.isArray(moi.main_payable) ? moi.main_payable : [];
  const prod = moi.production || {};
  return {
    main,
    // « payable » vient de MON `main_payable` : ce que J'AI les moyens de payer.
    // Le défaut est NON payable : un champ absent ou trop court doit retirer une
    // carte du compte, jamais en ajouter — `devParCarte` et `conParCarte` sont
    // les deux plus gros poids de la table, et un défaut permissif ne
    // dégraderait pas le choix de phase, il l'inverserait.
    payable: main.map((_, i) => payable[i] === true),
    mc: moi.mc || 0,
    plantes: moi.plants || 0,
    badges: moi.tags || {},
    production: {
      mc: prod.mc || 0,
      plantes: prod.plants || 0,
      chaleur: prod.heat || 0,
      cartes: prod.cards || 0,
    },
  };
}

/** Ce qui n'appartient à personne : la planète, la pioche, la manche. */
function leMonde(etat) {
  const p = (etat && etat.planet) || {};
  const part = (x, max) => (max ? Math.min(1, (x || 0) / max) : 0);
  const avancement =
    (part(p.oxygen, p.oxygen_max) +
      part(p.temperature, p.temperature_max) +
      part(p.oceans, p.oceans_max)) /
    3;
  return { avancement }; // 0 au départ, 1 quand la planète est terminée
}

// ─────────────────────────────────────────────────────────── l'échelle de valeur

/** La valeur d'une carte projet qu'on GARDE (main, pioche, révélation). */
function valeurEnMain(c) {
  if (!c) return 0;
  const R = REGLAGES;
  return (
    R.pvEnMain * (c.pv || 0) +
    R.badgeEnMain * ((c.badges && c.badges.length) || 0) -
    R.prixEnMain * (c.prix || 0)
  );
}

/**
 * La valeur d'une carte qu'on POSE maintenant. Les points de victoire d'abord —
 * et le prix compte POSITIVEMENT, à l'inverse de la main : le moteur n'énumère
 * que les cartes payables, donc entre deux cartes qu'on peut s'offrir, la plus
 * chère est la plus forte. Mesuré : +36 points d'écart moyen contre le témoin
 * par rapport au choix inverse (24 parties, graines de réglage).
 */
function valeurALaPose(c) {
  if (!c) return 0;
  const R = REGLAGES;
  return R.pvALaPose * (c.pv || 0) + R.prixALaPose * (c.prix || 0);
}

/** La valeur d'un revenu de production, ramenée à une échelle commune. */
function valeurProduction(p) {
  if (!p) return 0;
  return (
    1.0 * (p.mc || 0) +
    0.7 * (p.chaleur || 0) +
    1.2 * (p.plantes || 0) +
    2.0 * (p.cartes || 0)
  );
}

/**
 * DERNIER RECOURS — quand un point de décision n'est pas reconnu par nom, on ne
 * répond pas au hasard : on lit le libellé, qui est écrit pour un humain et dit
 * en clair ce qu'on gagne. On préfère ce qui donne, et on évite ce qui ne fait
 * rien. C'est grossier, et c'est assumé : ce chemin ne sert qu'aux points de
 * décision rares.
 */
function valeurDuLibelle(libelle) {
  const t = String(libelle || "").toLowerCase();
  let v = 0;
  // « ne rien faire » est presque toujours le choix vide.
  if (/^(ne rien|non\b|garder\b|décider après|decider apres)/.test(t)) v -= 6;
  if (/^(oui\b|poser|piocher|améliorer|ameliorer|gagner)/.test(t)) v += 4;
  // Les gains chiffrés : « +2 plantes », « −10 MC », « 1 pas ».
  for (const m of t.matchAll(/([+\-−])\s*(\d+)/g)) {
    const n = Number(m[2]);
    v += (m[1] === "+" ? 1 : -1) * Math.min(n, 12) * 0.8;
  }
  if (/\bpas\b/.test(t)) v += 3; // un pas de terraformation = un point de TR
  if (/carte/.test(t)) v += 1.5;
  return v;
}

// ──────────────────────────────────────────────────────── outils de sélection

/** L'indice du maximum, premier arrivé premier servi (départage déterministe). */
function meilleur(n, note) {
  let iBest = 0;
  let vBest = -Infinity;
  for (let i = 0; i < n; i++) {
    const v = note(i);
    if (v > vBest) {
      vBest = v;
      iBest = i;
    }
  }
  return iBest;
}

/** Les `k` meilleurs indices, dans l'ordre croissant (le moteur veut des indices). */
function lesMeilleurs(n, k, note) {
  const ordre = Array.from({ length: n }, (_, i) => i);
  // Tri stable sur la note décroissante, puis sur l'indice : deux options de
  // même note sortent toujours dans le même ordre.
  ordre.sort((a, b) => note(b) - note(a) || a - b);
  return ordre.slice(0, Math.max(0, Math.min(k, n))).sort((a, b) => a - b);
}

// ───────────────────────────────────────────────────────── les points de décision

/** I à V : quelle carte Phase choisir. C'est la décision la plus fréquente. */
function noterPhase(phase, moi, monde) {
  const R = REGLAGES;
  const payables = moi.main.filter((_, i) => moi.payable[i]);
  const vertes = payables.filter((c) => c.couleur === "verte").length;
  const autres = payables.length - vertes;
  const prodTotale = valeurProduction(moi.production);
  switch (phase) {
    case 1: // Développement — c'est là que les cartes vertes se posent
      return R.devBase + R.devParCarte * Math.min(vertes, 3);
    case 2: // Construction — c'est là que les bleues et les rouges se posent
      return R.conBase + R.conParCarte * Math.min(autres, 3);
    case 3: // Action — les actions standard et celles des cartes posées
      return (
        R.actBase +
        R.actParMc * Math.min(moi.mc, 40) +
        R.actParPlante * Math.min(moi.plantes, 12)
      );
    case 4: // Production — un revenu qui rapporte à chaque manche restante
      return Math.max(
        0,
        R.proBase -
          R.proDecroissance * prodTotale -
          R.proFinDePartie * monde.avancement,
      );
    case 5: // Recherche — on ne joue pas ce qu'on n'a pas en main
      return (
        R.recBase +
        R.recParCarteManquante * Math.max(0, R.mainVisee - moi.main.length)
      );
    default:
      return 0;
  }
}

/** III — Action : ce qu'on active. Les actions standard font le score. */
function noterAction(libelle) {
  const R = REGLAGES;
  const t = String(libelle || "");
  if (/^Forêt/.test(t)) return /plante/i.test(t) ? R.forêtPlantes : R.forêtMc;
  if (/^Océan/.test(t)) return R.ocean;
  if (/^Température/.test(t)) return R.temperature;
  if (/^Action de/.test(t)) return R.actionDeCarte;
  return valeurDuLibelle(t);
}

/**
 * CE JOUEUR VEND — ET C'EST L'ÉTAT, ET RIEN QUE L'ÉTAT, QUI LE LUI PERMET.
 *
 * Vendre est une ENTRÉE de la liste de décisions (`{"vendre":{joueur,cartes}}`,
 * voir `adversaire.md`), pas une réponse : le moteur la consomme au point
 * d'occasion qui précède la question, puis repose la MÊME question sur l'état
 * d'après. Une occasion ne se dépense qu'une fois par siège ; une seconde vente
 * rendue au même point serait refusée et arrêterait la partie.
 *
 * Il fallait donc savoir, sans aucune mémoire, si l'occasion est encore
 * ouverte — et `etat.vente_offerte` ne le disait pas : ce drapeau est armé AVANT
 * la vente et vaut encore vrai après. Ce joueur ne vendait pas pour cette
 * raison-là. Le moteur publie désormais l'occasion elle-même,
 * `etat.occasion_de_vendre_ouverte`, un booléen par siège, faux dès que
 * l'occasion de ce siège est dépensée (`engine/src/flow.rs`, `observer` ;
 * `state::PlayerState::occasion_de_vendre_ouverte`).
 *
 * Ce joueur reste donc SANS MÉMOIRE : il ne retient pas qu'il vient de vendre,
 * il le LIT. Reposez-lui la même question sur le même état, il rend la même
 * chose.
 *
 * CE QU'IL VEND. Les cartes dont la valeur en main tombe sous `seuilVente` —
 * l'échelle est celle de `valeurEnMain`, la même qui sert à défausser et à
 * garder ailleurs dans ce fichier — sans jamais descendre sous `gardeMini`
 * cartes en main. Il vend les PIRES d'abord. Une carte qui ne se posera jamais
 * ne rapporte rien ; 3 MC, si.
 */
function venteEventuelle(d, etat, moi) {
  const siege = d.joueur;
  // L'occasion, par siège, et seulement la mienne. Le nom est celui du moteur ;
  // un état qui ne le publierait pas (ancien moteur) vaut « pas d'occasion »,
  // et ce joueur ne vend alors pas — jamais l'inverse.
  const ouvertes = etat && etat.occasion_de_vendre_ouverte;
  if (!Array.isArray(ouvertes) || ouvertes[siege] !== true) return null;

  const R = REGLAGES;
  const main = moi.main;
  const aVendre = [];
  // Les pires d'abord : on trie les indices par valeur croissante.
  const ordre = main
    .map((c, i) => [i, valeurEnMain(c)])
    .sort((a, b) => a[1] - b[1] || a[0] - b[0]);
  for (const [i, v] of ordre) {
    if (v >= R.seuilVente) break;
    if (main.length - aVendre.length <= R.gardeMini) break;
    aVendre.push(i);
  }
  if (!aVendre.length) return null;
  return { vendre: { joueur: siege, cartes: aVendre.sort((a, b) => a - b) } };
}

// ───────────────────────────────────────────────────────────────── le cerveau

/**
 * La décision, en fonction pure de ce que le moteur vient de dire et de MON
 * côté de la table.
 */
function decider(d, etat) {
  const moi = monSiege(etat, d.joueur);
  const monde = leMonde(etat);

  // VENDRE PASSE AVANT DE RÉPONDRE : le moteur consomme l'entrée au point
  // d'occasion qui précède cette question-ci, puis repose la même question sur
  // l'état d'après — les cartes payables seront ré-énumérées avec l'argent de la
  // vente. On répondra donc à l'appel suivant.
  const vente = venteEventuelle(d, etat, moi);
  if (vente) return vente;

  const options = d.options || [];
  const n = options.length;
  const forme = formeDeLaReponse(d);

  if (forme === "montant") {
    // Le moteur ne propose un montant que pour une dépense qu'il vient
    // d'autoriser, et qui rapporte quelque chose en face : on dépense le
    // maximum. C'est le seul point où ce joueur parie sans pouvoir mesurer
    // l'option par option — les montants sont trop rares (moins d'une décision
    // sur cent) pour qu'une mesure les distingue du bruit.
    return d.maximum ?? d.minimum ?? 0;
  }

  if (forme === "multiple") {
    switch (d.type) {
      case "discard_down": {
        // On jette les MOINS bonnes.
        const k = d.a_choisir ?? 1;
        return lesMeilleurs(n, k, (i) => -valeurEnMain(options[i]));
      }
      case "project_mulligan": {
        // Nombre libre : on remplace tout ce qui est sous la barre. Une carte
        // chère sans point de victoire ne se posera jamais assez tôt.
        const aJeter = [];
        for (let i = 0; i < n; i++) {
          if (valeurEnMain(options[i]) < REGLAGES.seuilMulligan) aJeter.push(i);
        }
        return aJeter;
      }
      default: {
        // `research_keep`, `revelation_pioche`, … : on garde les MEILLEURES.
        const k = d.a_choisir ?? Math.min(1, n);
        return lesMeilleurs(n, k, (i) => valeurEnMain(options[i]));
      }
    }
  }

  // ── choix simple ─────────────────────────────────────────────────────────
  // L'indice de « passer », quand le moteur l'offre : c'est `options.length`
  // (voir `nombreDeChoix` dans `fournisseurs.js`, qui compte cette issue de plus).
  const iPasser = d.passer ? n : -1;

  switch (d.type) {
    case "pick_phase":
      return meilleur(n, (i) => noterPhase(options[i].phase, moi, monde));

    case "amelioration_carte_phase":
      // Améliorer la phase qu'on choisira le plus souvent. Entre la variante A
      // et la B d'une même phase, le libellé ne dit rien qu'on sache lire sans
      // connaître les règles : on tranche pour A, parce qu'il faut trancher et
      // que le départage doit rester déterministe.
      return meilleur(
        n,
        (i) =>
          noterPhase(options[i].phase, moi, monde) +
          (options[i].variante === "A" ? 0.01 : 0),
      );

    case "choose_build": {
      if (n === 0) return iPasser >= 0 ? iPasser : 0;
      // Poser vaut mieux que passer : le moteur n'énumère que ce qui est
      // payable, et une carte en main ne rapporte rien.
      return meilleur(n, (i) => valeurALaPose(options[i].carte));
    }

    case "action_choice": {
      if (n === 0) return iPasser >= 0 ? iPasser : 0;
      const iBest = meilleur(n, (i) => noterAction(options[i].libelle));
      const vBest = noterAction(options[iBest].libelle);
      return iPasser >= 0 && vBest <= REGLAGES.passer ? iPasser : iBest;
    }

    case "pick_corporation":
      // On lit ce que la corporation annonce : son capital de départ et ses
      // badges. Rien d'autre n'est visible à cet instant.
      return meilleur(
        n,
        (i) =>
          (options[i].mc_depart || 0) +
          2 * ((options[i].badges && options[i].badges.length) || 0),
      );

    case "corp_mulligan": {
      // « Garder » ou « Remplacer les 2 » : on garde si la meilleure des deux
      // corporations en main annonce au moins 24 MC de départ (valeur choisie à
      // la main, non mesurée — les corporations vues vont de 20 à 26).
      //
      // Les deux indices sont cherchés PAR LEUR LIBELLÉ, jamais déduits l'un de
      // l'autre : « l'autre option » n'a de sens que s'il y en a exactement
      // deux, et le jour où le moteur en ajoutera une troisième, une
      // soustraction silencieuse répondrait à côté. Si aucun libellé ne parle de
      // garder, on garde l'option 0 — c'est le repli, et il est déclaré.
      let mieux = 0;
      for (const c of d.corporations || []) mieux = Math.max(mieux, c.mc_depart || 0);
      const dit = (motif) => {
        for (let i = 0; i < n; i++) if (motif.test(options[i].libelle || "")) return i;
        return -1;
      };
      const iGarder = dit(/garder/i);
      const iRemplacer = dit(/remplacer/i);
      if (iGarder < 0 || iRemplacer < 0) return 0;
      return mieux >= 24 ? iGarder : iRemplacer;
    }

    case "pick_joker_tag":
      // Le badge dont on a déjà le plus : c'est la synergie la plus probable, et
      // le moteur l'écrit lui-même dans le libellé (« vous en avez N »).
      return meilleur(n, (i) => moi.badges[options[i].badge] || 0);

    case "rejouer_production":
      return meilleur(n, (i) => {
        const p = options[i].production || {};
        return valeurProduction({
          mc: p.mc,
          plantes: p.plantes,
          chaleur: p.chaleur,
          cartes: p.cartes,
        });
      });

    case "construction_bonus":
      // Poser une carte de plus vaut mieux que piocher ; piocher tout de suite
      // vaut mieux que décider après (la carte piochée devient posable).
      return meilleur(n, (i) => {
        const t = String(options[i].libelle || "");
        if (/supplémentaire|supplementaire/.test(t)) return 3;
        if (/tout de suite/.test(t)) return 2;
        if (/^Piocher/.test(t)) return 2;
        return 0;
      });

    case "bonus_selectionneur": {
      // « Poser une carte de plus » ne vaut que si on a de quoi la poser.
      const payables = moi.main.filter((_, i) => moi.payable[i]).length;
      return meilleur(n, (i) => {
        const t = String(options[i].libelle || "");
        if (/poser/.test(t)) return payables > 0 ? 10 : 0;
        return valeurDuLibelle(t);
      });
    }

    case "reduction_plantes":
    case "reduction_microbes": {
      // Payer avec ses ressources plutôt qu'en MC : on accepte quand la remise
      // est franche (au moins 4 MC par ressource dépensée), sinon on garde ses
      // plantes — elles deviennent des forêts, donc des points.
      return meilleur(n, (i) => {
        const o = options[i];
        const ressources = (o.plantes || 0) + (o.microbes || 0);
        if (!ressources) return 0.5; // « Non : payer le prix plein »
        return (o.reduction_mc || 0) / ressources >= 4 ? 1 : 0;
      });
    }

    case "defausser_pour_piocher":
      // Échanger une carte contre une (ou deux) : bon quand la main est fournie.
      return meilleur(n, (i) => {
        const garder = /ne rien/i.test(options[i].libelle);
        return garder ? (moi.main.length > 2 ? 0 : 1) : moi.main.length > 2 ? 1 : 0;
      });

    case "choose_res_target":
      // Poser la ressource sur la carte qui en tirera le plus : à défaut d'en
      // savoir plus, celle qui porte déjà des points.
      return meilleur(n, (i) => valeurEnMain(options[i]));

    case "choose_res_source":
      // En retirer une : sur la carte la moins précieuse.
      return meilleur(n, (i) => -valeurEnMain(options[i]));

    default: {
      // Point de décision non reconnu : on lit le libellé. Jamais le hasard.
      if (n === 0) return iPasser >= 0 ? iPasser : 0;
      const iBest = meilleur(n, (i) => valeurDuLibelle(options[i].libelle));
      const vBest = valeurDuLibelle(options[iBest].libelle);
      if (iPasser >= 0 && vBest < 0) return iPasser;
      return iBest;
    }
  }
}

/**
 * Le fournisseur. `graine` n'est acceptée que pour respecter la signature des
 * autres fournisseurs du dépôt : ce joueur ne tire rien au sort, et n'a aucune
 * mémoire d'une décision à l'autre.
 */
export function fournisseurReflechi(graine = 0, nom = "reflechi") {
  return { nom, decider: (d, etat) => decider(d, etat) };
}

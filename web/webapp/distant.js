// LE FOURNISSEUR DE DÉCISIONS DISTANT — le siège d'en face est tenu par un
// humain, ailleurs, devant son propre écran.
//
// C'est le seul fichier neuf côté page, et il suit à la lettre le contrat
// d'`adversaire.md` : un fournisseur est un objet
// `{ nom, decider(decision, etat) -> réponse | Promise<réponse> }`.
//
// IL NE CONNAÎT AUCUNE RÈGLE DU JEU. Il ne regarde ni les options, ni l'état, ni
// ce qu'une carte coûte : il attend que le rang demandé apparaisse dans la liste
// du serveur de rendez-vous, et rend la réponse telle quelle. Le moteur reste
// l'unique autorité — s'il refuse la réponse reçue, il le dira lui-même.
//
// LA PARTIE EST LA GRAINE PLUS LA LISTE DES DÉCISIONS (`adversaire.md`). Tout
// découle de cette phrase :
//
//   · les deux pages font tourner LEUR moteur, sur la même graine et la même
//     liste : elles voient donc exactement la même partie, sans que rien du jeu
//     n'ait à transiter ;
//   · le serveur ne transporte que des réponses — quelques octets ;
//   · une page rechargée en pleine partie n'a rien à « restaurer » : elle
//     rejoue la liste depuis le début, et se retrouve à l'endroit exact où elle
//     s'était arrêtée. C'est pour cela que le fournisseur du siège LOCAL passe
//     par ici lui aussi : quand le rang demandé est déjà dans la liste, il rend
//     la réponse connue sans rien demander à personne.

// La seule dépendance de ce fichier à la couche d'affichage, et elle ne porte
// que sur le TEMPS : pendant le rattrapage, les durées tombent à zéro.
import { reglerRattrapage } from "./vue/anim.js";
// (les-ecrans-manquants) L'ensemble MESURÉ des questions que le moteur pose aux
// deux joueurs en même temps. Ce fichier ne la connaît pas, il la lit.
import { estSimultanee } from "./questions-simultanees.js";

// ------------------------------------------------------- ce que dit l'adresse

/**
 * `?partie=<code>` allume le mode en ligne. `?siege=0|1` dit quel siège CE
 * navigateur-ci tient. Hors de là, rien de ce fichier ne s'exécute et le mode à
 * deux sur le même écran est exactement ce qu'il était.
 */
export function lireRendezVous() {
  const p = new URLSearchParams(location.search);
  const code = (p.get("partie") || "").trim();
  if (!code) return null;
  const g = Number.parseInt(p.get("graine") ?? "", 10);
  const b = p.get("boites");
  return {
    code,
    siege: p.get("siege") === "1" ? 1 : 0,
    // Une graine dans l'adresse est un SOUHAIT : c'est la première page arrivée
    // qui la fixe pour la partie, et le serveur qui fait foi ensuite. Sans quoi
    // deux liens recopiés à un chiffre près donneraient deux parties.
    graineSouhaitee: Number.isFinite(g) ? g : null,
    boitesSouhaitees: b === "base" || b === "base,decouverte" ? b : null,
  };
}

// ------------------------------------------------------ ce que la page publie
//
// Cinq attributs sur `<html>`, posés DÈS LE CHARGEMENT DU MODULE — avant même
// que le décor soit bâti. Un joueur doit savoir à tout instant s'il est en
// ligne, avec qui, et ce qu'on attend de lui ; et l'on doit pouvoir le vérifier
// de l'extérieur sans lire une ligne de code.

const REGLAGE = lireRendezVous();

/**
 * LE REJEU EST UN FAIT, PAS SEULEMENT UN BANDEAU. (04-08, en partie à deux.)
 *
 * Après un rechargement, la page repasse au moteur toutes les décisions déjà
 * prises pour revenir à l'instant présent. Chacune redéclenchait sa mise en
 * scène : les grandes tuiles océan se retournaient une à une depuis le début de
 * la partie. Le drapeau existait déjà (`canal.rejeu`) mais ne servait qu'à
 * écrire « Catching up… » dans le coin ; personne d'autre ne le lisait.
 *
 * Il est désormais DIT à la couche qui tient les durées, qui les met à zéro tant
 * qu'il dure — sans toucher au réglage d'animations choisi par le joueur, qui
 * reprend la main intact à la fin du rattrapage. Un seul point d'écriture, pour
 * qu'aucun chemin ne puisse lever le drapeau sans éteindre les durées.
 */
function marquerRejeu(canal, oui) {
  canal.rejeu = oui;
  reglerRattrapage(oui);
}

/**
 * L'ÉCRAN EST EN RETARD SUR LE MOTEUR, ET C'EST NORMAL.
 *
 * Quand la première décision inconnue paraît, le moteur a fini de rattraper —
 * mais l'écran, lui, n'a pas encore dessiné l'état qui en découle. Éteindre le
 * rattrapage à cet instant précis rallumait la mise en scène juste à temps pour
 * la dernière révélation d'océan du passé, qui se retournait alors en grand au
 * milieu de l'écran. Mesuré le 04-08 : rattrapage éteint à 327 ms, grande tuile
 * parue à 417 ms.
 *
 * On laisse donc passer deux images avant d'éteindre : le rendu a eu lieu, tout
 * ce qui appartenait au passé est parti sans mise en scène, et ce qui arrivera
 * ensuite — la partie qui reprend — retrouve son théâtre intact.
 */
function finirLeRejeuApresLeRendu(canal) {
  if (!canal.rejeu) {
    // Rien à finir : on n'était pas en train de rejouer. Éteindre quand même
    // serait sans effet, mais le dire est plus clair que de le supposer.
    marquerRejeu(canal, false);
    return;
  }
  requestAnimationFrame(() => requestAnimationFrame(() => marquerRejeu(canal, false)));
}

function poser(nom, valeur) {
  if (valeur === null || valeur === undefined) {
    document.documentElement.removeAttribute(nom);
  } else {
    document.documentElement.setAttribute(nom, String(valeur));
  }
}

if (REGLAGE) {
  poser("data-en-ligne", "oui");
  poser("data-partie", REGLAGE.code);
  poser("data-siege-local", REGLAGE.siege);
  // Tant qu'aucune connexion n'est ouverte, l'autre est absent. On ne le déduit
  // pas de l'existence d'une partie : on le tient d'une connexion réelle.
  poser("data-adversaire", "absent");
  poser("data-attente", "aucune");
}

// ------------------------------------------------------------- le bandeau
//
// Toute latence est AFFICHÉE, jamais masquée. Quand j'attends l'autre, je dois
// voir que j'attends, et pourquoi. Le bandeau est bâti ici, en dur, sans
// toucher à une feuille de style (un autre chantier y travaille) et sans jamais
// intercepter un clic (`pointer-events: none`).

// La page est en anglais, volontairement — comme les cartes. Seul le code, et
// ce que le serveur écrit dans sa fenêtre, sont en français.
// CE QUE L'AUTRE EST EN TRAIN DE FAIRE, question par question (CNF-4, relevé le
// 04-08 en partie à deux). « Waiting for the other player… » ne disait pas si
// l'attente serait d'une seconde ou d'une minute : trier ses cartes de départ
// n'a rien à voir avec choisir une phase. Le texte est une FONCTION du type de
// la décision que le moteur a posée à l'autre, et de rien d'autre — ni compteur,
// ni horloge : deux attentes du même type donnent le même texte.
//
// La liste couvre TOUS les types que le moteur peut poser (`wasm/src/lib.rs` et
// les onze natures de `engine/src/choice.rs`, mêmes clefs que la table des
// questions de `vue/mots.js`). Le message général ne sert plus qu'à un type
// qu'on ne connaîtrait pas — une question ajoutée au moteur après ce jour-ci.
const ATTENTE_PAR_TYPE = {
  corp_mulligan: "The other player is choosing Corporation cards…",
  project_mulligan: "The other player is choosing project cards…",
  pick_corporation: "The other player is picking a Corporation…",
  pick_phase: "The other player is choosing a Phase card…",
  choose_build: "The other player is picking a card to play…",
  construction_bonus: "The other player is taking the Construction bonus…",
  action_choice: "The other player is choosing an action…",
  action_amount: "The other player is deciding how much to spend…",
  choose_res_source: "The other player is taking a resource from a card…",
  choose_res_target: "The other player is placing a resource on a card…",
  pick_joker_tag: "The other player is choosing a tag…",
  research_keep: "The other player is looking through the cards drawn…",
  revelation_pioche: "The other player is looking at a revealed card…",
  discard_down: "The other player is discarding down to the hand limit…",
  sell_card: "The other player is selling a card…",
  choose_option: "The other player is choosing a branch of a card…",
  corp_tr_boost: "The other player is settling a Corporation bonus…",
  amelioration_carte_phase: "The other player is upgrading a Phase card…",
  alternative_carte: "The other player is choosing how a card applies…",
  alternative_action: "The other player is choosing how an action applies…",
  reduction_microbes: "The other player is spending microbes…",
  reduction_plantes: "The other player is spending plants…",
  paiement_chaleur: "The other player is paying with heat…",
  defausser_pour_piocher: "The other player is discarding to draw…",
  montant_depense: "The other player is deciding how much to spend…",
  bonus_selectionneur: "The other player is taking the selector bonus…",
  rejouer_production: "The other player is replaying a production…",
};

/** Le texte d'attente d'un type de décision ; le général pour un inconnu. */
function texteAttente(type) {
  return (type && ATTENTE_PAR_TYPE[type]) || BANDEAU.attente;
}

const BANDEAU = {
  attente: "Waiting for the other player…",
  absent: "The other player is away — the game resumes when they come back.",
  aMoi: "Your turn.",
  reprise: "Catching up with the game…",
  panne: "Lost contact with the meeting point — trying again…",
  desaccord: "The two screens disagree about whose turn it is. Reload this page.",
  double: "Someone else answered for this seat. Reload this page to catch up.",
};

let bandeau = null;

function montrerBandeau(texte) {
  if (!bandeau) {
    bandeau = document.createElement("div");
    bandeau.id = "en-ligne";
    bandeau.setAttribute("data-en-ligne-bandeau", "");
    // (04-08, mesuré en 1920×1080) LE BANDEAU NE S'ASSIED PLUS DANS LE COIN.
    // À `bottom: 12px` il occupait exactement la place du panneau de vente
    // (`style-vente.css` : `left: 12px; bottom: 10px`, mesuré à y 1043→1070) et
    // le recouvrait — avec `z-index: 9999` contre 96, c'est le bandeau qui
    // gagnait. Le bouton restait cliquable (`pointer-events: none`) mais
    // INVISIBLE : signalé en partie à deux, « le bouton pour vendre est toujours
    // caché », y compris après un rechargement complet.
    //
    // La hauteur de départ n'est qu'un repli : `placerBandeau` mesure la bande
    // libre au-dessus de ma barre à chaque affichage. Une première tentative
    // s'était contentée de `--h-mienne` (la hauteur de ma main) et recouvrait
    // alors mes jauges — la barre du joueur est une ligne `auto` de la grille,
    // elle n'a AUCUNE variable de hauteur, donc aucune constante ne peut la
    // décrire. On la mesure.
    bandeau.style.cssText = [
      "position:fixed", "left:12px",
      "bottom:calc(var(--h-mienne, 182px) + 80px)", "z-index:9999",
      "max-width:min(46ch,60vw)", "padding:8px 12px", "border-radius:8px",
      "background:rgba(12,14,20,.86)", "color:#e8eef7",
      "font:500 13px/1.35 system-ui,sans-serif", "letter-spacing:.01em",
      "box-shadow:0 2px 12px rgba(0,0,0,.45)",
      // Un bandeau ne prend JAMAIS un clic : la table reste entièrement jouable.
      "pointer-events:none", "user-select:none",
    ].join(";");
    document.body.appendChild(bandeau);
    // La fenêtre qui change de taille déplace la barre : le bandeau la suit.
    addEventListener("resize", placerBandeau);
  }
  bandeau.textContent = texte;
  bandeau.style.display = texte ? "block" : "none";
  if (texte) placerBandeau();
}

/**
 * Pose le bandeau JUSTE AU-DESSUS de ma barre de jauges.
 *
 * Le bas de l'écran est entièrement pris : ma main tout en bas, ma barre de
 * jauges au-dessus, et le panneau de vente dans le coin. La seule bande libre
 * est celle qui sépare ma barre de la scène où l'on répond — et sa position ne
 * s'écrit pas en dur, la barre étant une ligne `auto` de la grille (`style.css`)
 * dont la hauteur dépend de son contenu. On la MESURE donc, à chaque affichage
 * et à chaque changement de taille de la fenêtre. Si la barre n'est pas encore
 * dessinée, le repli en dur du `cssText` tient jusqu'au prochain affichage.
 */
function placerBandeau() {
  if (!bandeau || !REGLAGE) return;
  const barre = document.querySelector(`.equipage[data-joueur="${REGLAGE.siege}"]`);
  if (!barre) return;
  const r = barre.getBoundingClientRect();
  if (!r.height) return;
  bandeau.style.bottom = `${Math.round(innerHeight - r.top + 10)}px`;
}

function rafraichirBandeau(canal) {
  if (!canal) return;
  const attente = document.documentElement.getAttribute("data-attente");
  const present = canal.joueurs[1 - canal.siege];
  // Une alerte prime sur tout le reste : c'est le seul cas où le joueur doit
  // AGIR. Elle ne s'efface pas toute seule.
  if (canal.alerte) montrerBandeau(canal.alerte);
  else if (!canal.vivant) montrerBandeau(BANDEAU.panne);
  else if (canal.rejeu) montrerBandeau(BANDEAU.reprise);
  else if (!present) montrerBandeau(BANDEAU.absent);
  else if (attente === "lui") montrerBandeau(texteAttente(canal.typeAttendu));
  else if (attente === "moi") montrerBandeau(BANDEAU.aMoi);
  else montrerBandeau("");
}

// ------------------------------------------------------------------- le canal
//
// Un aller-retour réseau, et rien d'autre :
//   · on ÉCOUTE le flux d'évènements du serveur (`/relais/flux`) — chaque
//     décision retenue y arrive, ainsi que la présence des deux sièges ;
//   · on ENVOIE nos propres réponses (`POST /relais/decision`).
//
// La connexion ouverte du flux EST la présence : tant qu'elle tient, le siège
// est là ; dès qu'elle tombe, le serveur le dit à l'autre.

const DELAI_RESYNC = 4000;

function url(chemin, parametres) {
  const u = new URL(chemin, location.href);
  for (const [k, v] of Object.entries(parametres || {})) {
    if (v !== null && v !== undefined) u.searchParams.set(k, String(v));
  }
  return u;
}

/**
 * Un envoi, et jusqu'à deux reprises. Un réseau qui hoquette une seconde — une
 * borne qui bascule, un tunnel qui se rétablit — est un évènement banal sur une
 * liaison publique, et il ne doit pas coûter la partie. Seul l'échec du
 * TRANSPORT est repris : un refus du serveur est une réponse, et une réponse ne
 * se retente pas.
 */
async function envoyerJson(chemin, corps, essais = 3) {
  let derniere = null;
  for (let n = 0; n < essais; n++) {
    try {
      const r = await fetch(url(chemin), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(corps),
      });
      let objet = null;
      try {
        objet = await r.json();
      } catch {
        objet = null;
      }
      return { code: r.status, objet };
    } catch (e) {
      derniere = e;
      if (n + 1 < essais) await new Promise((r) => setTimeout(r, 400 * (n + 1)));
    }
  }
  return { code: 0, objet: null, panne: derniere };
}

function creerCanal(reglage) {
  const canal = {
    code: reglage.code,
    siege: reglage.siege,
    graine: null,
    boites: null,
    /** La liste ordonnée des réponses, telle que le serveur la tient. */
    decisions: [],
    joueurs: [false, false],
    /** Le flux d'évènements est-il ouvert ? */
    vivant: false,
    /** La page est-elle en train de rejouer la liste (après un rechargement) ? */
    rejeu: false,
    /** Un message qui demande une action au joueur ; il ne s'efface pas seul. */
    alerte: null,
    /** La resynchronisation de secours, pour pouvoir l'arrêter à la fin. */
    resync: null,
    attentes: new Map(),
    source: null,
    /** Le type de la décision que l'autre est en train de traiter (bandeau). */
    typeAttendu: null,
    /** Le rang que le rendez-vous attend, tel qu'il l'annonce lui-même. */
    rangAttendu: 0,
    /** Ce qui attend qu'un rang soit annoncé : rang -> résoudre. */
    rendezVousDeRang: new Map(),
    /** Les réponses gardées de côté : rang -> { decision, promesse }. */
    gardees: new Map(),
    /**
     * (les-ecrans-manquants) **LE RANG QU'ON EST EN TRAIN DE ME DEMANDER**, ou
     * `null`. Armé seulement quand le rendez-vous n'avait pas encore dépassé ce
     * rang au moment où la question s'est posée. Voir `noterRangAttendu`.
     */
    monRangEnCours: null,
  };

  /** Une réponse déjà connue pour ce rang, ou `undefined`. */
  canal.reponseConnue = (rang) => canal.decisions[rang];

  /** Réveille tout ce qui attendait un rang désormais connu. */
  function servirLesAttentes() {
    for (const [rang, resoudre] of [...canal.attentes]) {
      if (canal.decisions[rang] !== undefined) {
        canal.attentes.delete(rang);
        resoudre(canal.decisions[rang]);
      }
    }
  }

  /** La liste du serveur fait foi ; elle ne fait que s'allonger. */
  function adopter(liste) {
    if (!Array.isArray(liste)) return;
    if (liste.length >= canal.decisions.length) canal.decisions = liste.slice();
    servirLesAttentes();
  }

  /**
   * LE RANG QUE LE RENDEZ-VOUS ANNONCE ATTENDRE. Une réponse gardée de côté ne
   * DEVINE jamais son rang : elle attend que le point de rendez-vous dise qu'il
   * en est là. C'est la seule chose qui garantisse qu'elle parte au bon moment —
   * et le serveur peut l'annoncer sans rien révéler, puisque savoir QUE l'autre
   * a répondu n'apprend rien de CE qu'il a répondu.
   */
  canal.noterRangAttendu = (n) => {
    if (!Number.isInteger(n) || n <= canal.rangAttendu) return;
    // (les-ecrans-manquants) **QUELQU'UN VIENT DE RÉPONDRE À MA PLACE.**
    //
    // Le rendez-vous annonce qu'il attend désormais un rang PLUS LOIN que celui
    // qu'on est en train de me demander, alors que je n'ai pas encore répondu.
    // Or ce rang-là ne revient qu'à mon siège : le serveur refuse l'autre
    // (`recevoirDecision`, « Personne ne répond à la place de l'autre »). Donc
    // c'est une seconde page ouverte sur MON siège, et mon clic va être remplacé
    // en silence.
    //
    // POURQUOI IL A FALLU L'ÉCRIRE ICI. Avant ce lot, le serveur refusait la
    // seconde réponse et `publier` le disait. Depuis que les questions de mise
    // en place se jouent FACE CACHÉE, ce rang appartient à un groupe, et le
    // serveur autorise alors délibérément une réponse REDONNÉE par son propre
    // siège — c'est ce qui permet à une page fermée entre son envoi et la
    // révélation de reprendre la partie, puisqu'elle ne peut ni relire sa
    // réponse (le rideau est tiré) ni la redonner autrement. Cette permission
    // est juste, et on n'y touche pas : le relais ne change pas d'une ligne.
    // C'est la PAGE qui sait ce que le serveur ne peut pas savoir — qu'elle
    // n'a, elle, rien envoyé à ce rang.
    //
    // ET CE N'EST PAS UNE FUITE : savoir QUE le rendez-vous a avancé est déjà
    // publié (`rang_attendu`), et n'apprend rien de CE qui a été répondu.
    //
    // LA REPRISE LÉGITIME NE DÉCLENCHE RIEN, parce que `monRangEnCours` n'est
    // armé que si le rendez-vous n'avait pas DÉJÀ dépassé ce rang quand la
    // question s'est posée : une page qui reprend voit `rangAttendu` en avance
    // dès le premier instant, et ne s'arme pas.
    if (canal.monRangEnCours !== null && n > canal.monRangEnCours) {
      canal.monRangEnCours = null;
      canal.alerte = BANDEAU.double;
      rafraichirBandeau(canal);
      console.warn("rendez-vous : une autre page a répondu pour ce siège.");
    }
    canal.rangAttendu = n;
    for (const [rang, resoudre] of [...canal.rendezVousDeRang]) {
      if (canal.rangAttendu >= rang) {
        canal.rendezVousDeRang.delete(rang);
        resoudre();
      }
    }
  };

  canal.quandLeRangArrive = (rang) => {
    if (canal.rangAttendu >= rang) return Promise.resolve();
    return new Promise((resoudre) => canal.rendezVousDeRang.set(rang, resoudre));
  };

  canal.presence = (joueurs) => {
    if (!Array.isArray(joueurs)) return;
    canal.joueurs = [joueurs[0] === true, joueurs[1] === true];
    poser("data-adversaire", canal.joueurs[1 - canal.siege] ? "present" : "absent");
    rafraichirBandeau(canal);
  };

  canal.attendre = (nom, type = null) => {
    poser("data-attente", nom);
    // Le bandeau doit dire CE QUE l'autre fait : on retient donc le type de la
    // question qu'il traite, et rien d'autre de son contenu.
    canal.typeAttendu = nom === "lui" ? type : null;
    rafraichirBandeau(canal);
  };

  /**
   * Attend que le rang demandé apparaisse dans la liste du serveur. Aucune
   * limite de temps : un adversaire absent est un adversaire absent, on le DIT
   * (bandeau + `data-adversaire`), on ne le remplace pas par un programme.
   */
  canal.attendreReponse = (rang) => {
    const connue = canal.reponseConnue(rang);
    if (connue !== undefined) return Promise.resolve(connue);
    return new Promise((resoudre) => {
      canal.attentes.set(rang, resoudre);
    });
  };

  /**
   * Dit au serveur ce que le MOTEUR vient de dire : ce rang est à ce siège.
   *
   * `groupe` — `{ debut, taille }` ou `null` — dit en plus que ce rang fait
   * partie d'un choix FACE CACHÉE : le serveur ne publiera aucune des réponses
   * du groupe avant qu'elles y soient toutes. Là non plus il n'apprend aucune
   * règle : ce sont les deux moteurs qui le lui disent, et ils se corroborent.
   *
   * Rend ce que le serveur a retenu, pour que l'appelant sache si le rideau est
   * réellement tiré avant de poser quoi que ce soit à l'écran.
   */
  canal.annoncerTour = async (rang, siege, groupe = null) => {
    const { code, objet } = await envoyerJson(
      "relais/tour", { partie: canal.code, rang, siege, groupe });
    // Une annonce refusée est le SEUL signe que les deux moteurs ne voient pas
    // la même partie. Elle ne doit pas mourir dans la fenêtre de commandes, que
    // personne ne regardera : on la met sous les yeux du joueur.
    if (code === 409) {
      console.warn("rendez-vous : " + ((objet && objet.erreur) || "annonce de tour refusée"));
      canal.alerte = BANDEAU.desaccord;
      rafraichirBandeau(canal);
    }
    return code >= 200 && code < 300 ? objet : null;
  };

  /** Envoie notre réponse. Rend la réponse retenue par le serveur. */
  canal.publier = async (rang, reponse) => {
    const { code, objet } = await envoyerJson("relais/decision", {
      partie: canal.code, siege: canal.siege, rang, reponse,
    });
    if (code >= 200 && code < 300) {
      if (canal.decisions[rang] === undefined) canal.decisions[rang] = reponse;
      canal.noterRangAttendu(rang + 1);
      return reponse;
    }
    // LE SERVEUR A REFUSÉ, ET IL A DIT POURQUOI. Un refus ne s'avale JAMAIS en
    // silence — surtout pas celui-ci. Trois cas, et un seul est bénin :
    //
    //   · le rang porte déjà EXACTEMENT notre réponse : notre envoi est arrivé
    //     deux fois (un réseau qui bégaie, une reprise de `envoyerJson`). Rien
    //     n'est perdu, on continue ;
    //   · le rang porte une AUTRE réponse : quelqu'un a répondu à notre place —
    //     un second onglet ouvert sur le même siège, par exemple. Le clic du
    //     joueur vient d'être remplacé par un autre : c'est exactement ce qu'il
    //     faut DIRE, jamais recouvrir. On l'affiche, et on remonte l'erreur ;
    //   · rien à ce rang : on remonte la phrase du serveur telle quelle.
    const etat = await lireEtat(canal.code);
    const retenue = etat && etat.decisions ? etat.decisions[rang] : undefined;
    if (retenue !== undefined) {
      adopter(etat.decisions);
      if (JSON.stringify(retenue) === JSON.stringify(reponse)) return retenue;
      canal.alerte = BANDEAU.double;
      rafraichirBandeau(canal);
      throw new Error(
        `Le rendez-vous a retenu une autre réponse que la vôtre pour la décision ` +
        `${rang}. Une seconde page est-elle ouverte sur le même siège ? ` +
        `Rechargez cette page : la partie reprendra où elle en est.`);
    }
    const phrase = (objet && objet.erreur)
      || (code === 0 ? "le point de rendez-vous n'a pas répondu" : `code ${code}`);
    throw new Error(
      `Réponse refusée par le rendez-vous : ${phrase} — rechargez la page, ` +
      `la partie reprendra où elle en était.`);
  };

  canal.adopter = adopter;
  return canal;
}

async function lireEtat(code, souhaits) {
  try {
    const r = await fetch(url("relais/etat", { partie: code, ...(souhaits || {}) }),
                          { cache: "no-store" });
    if (!r.ok) return null;
    return await r.json();
  } catch {
    return null;
  }
}

/**
 * Ouvre le rendez-vous : on demande l'état de la partie (ce qui la crée si elle
 * n'existe pas encore), puis on ouvre le flux d'évènements — c'est lui qui nous
 * rend PRÉSENT aux yeux de l'autre.
 *
 * Rend `null` si l'adresse ne porte pas de code de partie : le mode à deux sur
 * le même écran ne dépend alors de rien de neuf, pas même de ce fichier.
 */
export async function ouvrirRendezVous() {
  if (!REGLAGE) return null;

  const souhaits = {};
  if (REGLAGE.graineSouhaitee !== null) souhaits.graine = REGLAGE.graineSouhaitee;
  if (REGLAGE.boitesSouhaitees !== null) souhaits.boites = REGLAGE.boitesSouhaitees;

  const etat = await lireEtat(REGLAGE.code, souhaits);
  if (!etat) {
    throw new Error(
      "Le point de rendez-vous ne répond pas. La fenêtre qui l'a démarré est-elle " +
      "toujours ouverte ?");
  }

  const canal = creerCanal(REGLAGE);
  canal.graine = etat.graine;
  canal.boites = etat.boites;
  canal.adopter(etat.decisions);
  canal.noterRangAttendu(etat.rang_attendu);
  canal.presence(etat.joueurs);

  // Le flux d'évènements : chaque décision retenue y arrive, et la connexion
  // ouverte dit à l'autre que je suis là.
  const source = new EventSource(
    url("relais/flux", { partie: REGLAGE.code, siege: REGLAGE.siege }));
  canal.source = source;

  source.addEventListener("open", () => {
    canal.vivant = true;
    rafraichirBandeau(canal);
  });
  source.addEventListener("error", () => {
    // `EventSource` retente tout seul. On le DIT en attendant.
    canal.vivant = false;
    rafraichirBandeau(canal);
  });
  source.addEventListener("bonjour", (e) => {
    canal.vivant = true;
    const o = JSON.parse(e.data);
    canal.adopter(o.decisions);
    canal.noterRangAttendu(o.rang_attendu);
    canal.presence(o.joueurs);
  });
  // Le rendez-vous a avancé sans rien révéler : une réponse d'un choix face
  // cachée vient d'arriver, et c'est le signal qu'attend une réponse gardée de
  // côté pour partir à son tour.
  source.addEventListener("avancement", (e) => {
    canal.noterRangAttendu(JSON.parse(e.data).rang_attendu);
  });
  source.addEventListener("joueurs", (e) => {
    canal.presence(JSON.parse(e.data).joueurs);
  });
  source.addEventListener("decision", (e) => {
    const o = JSON.parse(e.data);
    const liste = canal.decisions.slice();
    liste[o.rang] = o.reponse;
    canal.adopter(liste);
    canal.noterRangAttendu(o.rang + 1);
  });

  // Filet de sécurité : si un évènement s'est perdu (réseau qui hoquette,
  // relais intermédiaire), on redemande l'état tant qu'on attend quelque chose.
  // C'est ce qui évite qu'une partie reste plantée demain matin.
  canal.resync = setInterval(async () => {
    if (!canal.attentes.size) return;
    const frais = await lireEtat(canal.code);
    if (frais) {
      canal.adopter(frais.decisions);
      canal.noterRangAttendu(frais.rang_attendu);
      canal.presence(frais.joueurs);
    }
  }, DELAI_RESYNC);

  // Quitter la page ferme le flux : l'autre voit le départ tout de suite.
  window.addEventListener("pagehide", () => source.close());

  rafraichirBandeau(canal);
  return canal;
}

// ------------------------------------------- les deux choisissent en même temps
//
// LE DÉFAUT (MOT-9, relevé le 04-08 en partie à deux). Le livret veut un choix
// de phase SIMULTANÉ ET FACE CACHÉE (`docs/regles/livret-base.md`, l. 268 et
// 629). Le moteur, lui, pose ses questions l'une après l'autre : la phase du
// joueur 0 au rang R, celle du joueur 1 au rang R+1. Le second regardait donc
// son écran sans rien pouvoir faire pendant que le premier réfléchissait — à
// chaque manche, dix ou douze fois par partie.
//
// CE QU'ON NE POUVAIT PAS FAIRE. Recopier la question du premier sur l'écran du
// second : les deux listes d'options DIFFÈRENT (chacun ne peut pas reprendre la
// phase qu'il vient de jouer), et une réponse est un numéro de position dans sa
// propre liste. La déduire de l'état, ce serait recopier une règle du jeu dans
// la page — ce que ce fichier n'a pas le droit de faire (`adversaire.md`).
//
// CE QU'ON FAIT. On DEMANDE au moteur. « La partie est la graine plus la liste
// des décisions » : avec la liste des réponses déjà données, on peut lui faire
// rejouer la partie et lui demander la question du rang R+1 — c'est le « essayer
// un coup dans le vide » d'`adversaire.md`. On le fait pour CHACUNE des réponses
// possibles au rang R, et l'on n'affiche la question anticipée que si le moteur
// rend exactement la même des deux, des trois, des cinq fois : ce qu'on montre
// alors ne contient, PAR CONSTRUCTION, aucune information sur le choix de
// l'autre. Si la question suivante dépendait de sa réponse, on ne l'affiche pas
// et le tour reste séquentiel, comme avant.
//
// Mesuré sur trois graines (90210, 1, 7) : 155 paires de `pick_phase` sur 155
// invariantes, ~1,4 ms par sonde — cinq sondes par manche.
//
// L'ÉTAT AFFICHÉ EST CELUI D'AVANT, jamais celui d'après. L'état du rang R+1
// contient `chosen_phase` du joueur 0 en clair : le rendre serait exactement la
// fuite qu'on doit interdire. La question anticipée est donc posée sur l'état
// que les deux pages ont déjà sous les yeux.

// (les-ecrans-manquants) **IL N'Y A PLUS DE TYPE ÉCRIT ICI.** Il y en avait un
// — `pick_phase` — et le moteur en pose CINQ : les trois étapes de la mise en
// place (cartes de départ rendues, projets rendus, corporation retenue) et la
// garde des cartes piochées en phase Recherche passaient toutes au fil de
// l'eau. Le second à répondre pouvait donc lire ce que le premier avait rendu,
// quelle corporation il avait prise et combien de cartes il venait de payer,
// AVANT de choisir.
//
// La liste vient désormais d'une MESURE faite sur le moteur lui-même
// (`questions-simultanees.js`, posée par `interface.js` avant la première
// question). Un lot qui rendrait une sixième question simultanée serait protégé
// sans qu'une ligne de ce fichier ne bouge.
//
// **DEUX CONDITIONS, ET IL FAUT LES DEUX.** La mesure dit QUELLES questions se
// posent aux deux joueurs à la fois ; elle ne dit pas qu'on a le droit d'en
// afficher une d'avance. Ce droit-là se prouve question par question, en
// démontrant que la question suivante est la MÊME quelle que soit la réponse à
// celle-ci (`questionSuivanteInvariante`). Sans la mesure, on grouperait des
// questions que la table donne à voir — mesuré le 22-08 : la seule invariance
// retient aussi `action_choice` (125 occurrences sur 486) et `choose_build`
// (86 sur 437), où le second joueur DOIT voir ce que le premier vient de faire.
// Sans la preuve, on afficherait d'avance une question dont la forme dépend de
// la réponse de l'autre, c'est-à-dire une fuite.

/**
 * COMBIEN DE RÉPONSES ON ACCEPTE D'ESSAYER. Prouver l'invariance, c'est essayer
 * TOUTES les réponses possibles : le prix en est le nombre de rejeux. Mesuré sur
 * cette machine le 22-08 : 0,21 ms par rejeu au rang 2, 0,37 ms au rang 33,
 * 2,4 ms au rang 400. Le plafond couvre les deux questions multiples du moteur
 * — le mulligan des projets (huit cartes, quantité libre : 2^8 = 256 réponses,
 * au rang 2, soit 54 ms) et la garde des cartes de Recherche (au plus 70
 * réponses, soit 170 ms au pire rang). Au-delà, on n'anticipe pas et le tour
 * reste séquentiel : le doute se paie par une attente, jamais par une fuite.
 */
const ESSAIS_MAX = 320;

/** Le nombre de sous-ensembles de `k` éléments parmi `n`. */
function combienDeSousEnsembles(n, k) {
  if (k < 0 || k > n) return 0;
  let r = 1;
  for (let i = 0; i < k; i++) r = (r * (n - i)) / (i + 1);
  return Math.round(r);
}

/** Tous les sous-ensembles d'indices de `n` options dont la taille est permise. */
function sousEnsembles(n, tailles) {
  const out = [];
  const maxi = Math.max(...tailles);
  const permise = new Set(tailles);
  const rec = (debut, acc) => {
    if (permise.has(acc.length)) out.push([...acc]);
    if (acc.length >= maxi) return;
    for (let i = debut; i < n; i++) {
      acc.push(i);
      rec(i + 1, acc);
      acc.pop();
    }
  };
  rec(0, []);
  return out;
}

/**
 * **TOUTES LES RÉPONSES QUE LE MOTEUR ACCEPTERAIT À CETTE QUESTION**, ou `null`
 * quand on ne sait pas les énumérer, ou qu'il y en a plus que le plafond.
 *
 * C'est la SECONDE VOIE que ce lot ajoute. L'ancienne fonction refusait d'agir
 * dès que la décision n'était pas un choix simple (`if (d.passer || d.montant ||
 * d.multiple) return null`) — or deux des cinq questions simultanées sont des
 * choix MULTIPLES : on rend un sous-ensemble de cartes. Elles restaient donc
 * séquentielles, c'est-à-dire à découvert.
 *
 * La forme des réponses n'est pas devinée : c'est celle qu'`adversaire.md` et
 * `fournisseurs.js` décrivent, la seule que le moteur reçoive — l'indice d'une
 * option (plus « passer » s'il est offert), un entier entre deux bornes, ou un
 * tableau d'indices distincts. Rien ici ne connaît une règle du jeu.
 *
 * L'ORDRE DES INDICES N'EST PAS ÉNUMÉRÉ, et c'est mesuré : sur douze réponses
 * multiples de parties réelles, le descripteur de la question suivante est
 * identique que les indices soient rendus croissants ou décroissants. On
 * énumère donc les sous-ensembles une fois chacun, triés.
 */
export function reponsesPossibles(d) {
  const options = d.options || [];
  const n = options.length;

  if (d.montant) {
    const mini = d.minimum;
    const maxi = d.maximum;
    if (!Number.isInteger(mini) || !Number.isInteger(maxi) || maxi < mini) return null;
    if (maxi - mini + 1 > ESSAIS_MAX) return null;
    const out = [];
    for (let v = mini; v <= maxi; v++) out.push(v);
    return out;
  }

  if (d.multiple) {
    if (!n) return null;
    // `a_choisir` absent = quantité LIBRE, de zéro à tout (le mulligan des
    // projets). Le nombre de réponses est alors 2^n, qu'on compte AVANT de les
    // construire : on ne fabrique jamais une liste qu'on va jeter.
    const k = d.a_choisir;
    const tailles = Number.isInteger(k)
      ? [k]
      : Array.from({ length: n + 1 }, (_, i) => i);
    let total = 0;
    for (const t of tailles) {
      total += combienDeSousEnsembles(n, t);
      if (total > ESSAIS_MAX) return null;
    }
    if (!total) return null;
    return sousEnsembles(n, tailles);
  }

  const total = n + (d.passer ? 1 : 0);
  if (!total || total > ESSAIS_MAX) return null;
  return Array.from({ length: total }, (_, i) => i);
}

/**
 * Un moteur À NOUS, pour REGARDER et rien d'autre : on ne lui fait jamais
 * avancer la partie, on lui pose des questions dans le vide. Il n'est ouvert
 * qu'à la première question de phase d'une partie en ligne — une page qui joue
 * en local n'en paie jamais le prix.
 */
let pontDeLecture = null;

function moteurDeLecture() {
  if (!pontDeLecture) {
    pontDeLecture = import("./pont.js")
      .then((m) => m.ouvrirPontDepuis("."))
      .catch((e) => {
        // Une seule tentative ratée ne condamne pas la partie : on retombe sur
        // le tour séquentiel, et l'on pourra réessayer à la manche suivante.
        pontDeLecture = null;
        throw e;
      });
  }
  return pontDeLecture;
}

/**
 * LA QUESTION SUIVANTE, SI ELLE NE DIT RIEN DE LA RÉPONSE À CELLE-CI.
 *
 * Rend le descripteur de la décision de rang `d.rang + 1` si — et seulement si —
 * `d` est d'un type que le moteur pose aux DEUX joueurs (ensemble mesuré), que
 * le descripteur suivant est le MÊME pour toutes les réponses possibles à `d`,
 * qu'il est du même type, et qu'il revient à l'autre joueur. `null` dans tous les
 * autres cas : on ne montre jamais une question dont la forme dépendrait de ce
 * que l'autre vient de choisir.
 */
async function questionSuivanteInvariante(canal, d) {
  // PREMIÈRE CONDITION : le moteur pose-t-il cette question aux deux joueurs en
  // même temps ? La réponse vient de la mesure, pas d'une liste.
  if (!d || !estSimultanee(d.type)) return null;
  if (!Number.isInteger(d.rang) || !Number.isInteger(canal.graine)) return null;
  const avant = canal.decisions.slice(0, d.rang);
  // La liste doit être complète jusqu'ici, sans trou : sinon le moteur ne
  // rejouerait pas la même partie que celle qu'on est en train de jouer.
  if (avant.length !== d.rang || avant.some((r) => r === undefined)) return null;

  // SECONDE CONDITION : toutes les réponses possibles, et on doit savoir les
  // énumérer. Ce qu'on ne peut pas énumérer, on ne peut pas le prouver.
  const reponses = reponsesPossibles(d);
  if (!reponses || !reponses.length) return null;

  let pont;
  try {
    pont = await moteurDeLecture();
  } catch {
    return null;
  }

  let empreinte = null;
  let suivante = null;
  for (const reponse of reponses) {
    let pas;
    try {
      pas = pont.pas(canal.graine, canal.boites, [...avant, reponse]);
    } catch {
      // Le moteur refuse cette réponse : on ne peut plus prouver l'invariance,
      // donc on n'anticipe pas.
      return null;
    }
    const dd = pas && pas.decision;
    if (!dd) return null;
    const vue = JSON.stringify(dd);
    if (empreinte === null) {
      // LE PREMIER ESSAI SERT AUSSI DE FILTRE. Si la question suivante n'est pas
      // la jumelle de celle-ci — même type, siège d'en face, rang qui suit — il
      // n'y a pas de paire à protéger, et payer les 255 essais restants ne
      // changerait rien. Ce raccourci ne peut qu'écarter : il ne fait jamais
      // conclure à l'invariance, seule la boucle entière le fait.
      if (dd.type !== d.type) return null;
      if (dd.rang !== d.rang + 1) return null;
      if (dd.joueur === d.joueur) return null;
      empreinte = vue;
      suivante = dd;
    } else if (vue !== empreinte) {
      // LA QUESTION SUIVANTE DÉPEND DE CETTE RÉPONSE-CI : l'afficher
      // maintenant, ce serait en dire quelque chose. On s'arrête là.
      return null;
    }
  }
  return suivante;
}

/** Deux descripteurs de décision sont-ils la même question, mot pour mot ? */
function memeQuestion(a, b) {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * POSER MA QUESTION EN MÊME TEMPS QUE LA SIENNE, ET GARDER MA RÉPONSE DE CÔTÉ.
 *
 * Le moteur n'acceptera ma réponse qu'à son rang, après celle de l'autre : je
 * la garde donc, et je l'envoie dès que le point de rendez-vous ANNONCE qu'il
 * attend ce rang — je ne le devine pas.
 *
 * Une réponse gardée de côté est FERME : elle est déjà partie du point de vue
 * du joueur, qui a vu sa carte se poser. Si la page se ferme entre le clic et
 * l'envoi, elle est perdue et rien n'est écrit nulle part : la partie reste
 * cohérente, le groupe face cachée reste incomplet, et la page rechargée repose
 * la même question puisqu'elle reprend au même rang.
 */
function garderDeCote(canal, local, d, etat, siege, groupe) {
  if (canal.gardees.has(d.rang)) return;
  const promesse = (async () => {
    // Le rendez-vous doit savoir à qui revient ce rang, et qu'il se joue face
    // cachée, AVANT que quiconque puisse y répondre.
    const retenu = await canal.annoncerTour(d.rang, siege, groupe);
    if (!retenu || !retenu.groupe || retenu.groupe.debut !== groupe.debut) {
      throw new Error("le rendez-vous n'a pas retenu le choix face cachée");
    }
    canal.attendre("moi", d.type);
    const reponse = await local.decider(d, etat);
    canal.attendre("lui", d.type);
    await canal.quandLeRangArrive(d.rang);
    return canal.publier(d.rang, reponse);
  })();
  // L'erreur éventuelle est relevée par celui qui reprendra la réponse ; on
  // l'accroche ici pour qu'elle ne remonte pas en « rejet non traité ».
  promesse.catch(() => {});
  canal.gardees.set(d.rang, { decision: d, promesse });
}

// ------------------------------------------------------------ les fournisseurs

/**
 * LA COMPOSITION EN LIGNE. Les deux sièges passent par le rendez-vous, chacun à
 * sa manière — et `partie.js` n'en sait rien : ce sont deux fournisseurs de
 * décisions ordinaires, au sens d'`adversaire.md`.
 *
 * @param {object}   canal     le rendez-vous ouvert
 * @param {Array}    fournisseurs  le tableau `[fournisseurJ0, fournisseurJ1]`
 * @param {number}   siege     le siège que CE navigateur tient
 * @param {Function} regarder  `(decision, etat) => void` — redessiner l'écran
 *                             pendant que l'autre réfléchit (l'écran ne doit
 *                             jamais se figer sans dire pourquoi)
 */
export function brancherEnLigne(canal, fournisseurs, siege, regarder) {
  const local = fournisseurs[siege];

  // MON SIÈGE. Trois cas, dans cet ordre :
  //   1. la réponse de ce rang est déjà dans la liste du serveur — c'est que je
  //      rejoue la partie après un rechargement : je la rends telle quelle,
  //      sans rien afficher ni demander ;
  //   2. sinon je décide (l'écran pose la question, comme toujours) ;
  //   3. et je publie ma réponse au rendez-vous avant de la rendre au moteur.
  fournisseurs[siege] = {
    nom: local.nom + " (en ligne)",
    async decider(d, etat) {
      // 0. LA QUESTION A DÉJÀ ÉTÉ POSÉE, en même temps que celle de l'autre :
      //    ma réponse est prise depuis longtemps, elle n'attendait que son
      //    rang. Le joueur, lui, a déjà vu sa carte se poser.
      const gardee = canal.gardees.get(d.rang);
      if (gardee) {
        canal.gardees.delete(d.rang);
        if (!memeQuestion(gardee.decision, d)) {
          // Impossible si l'invariance a été prouvée — et c'est justement pour
          // cela qu'on ne l'avale pas en silence : ce serait le signe que les
          // deux moteurs ne voient plus la même partie.
          canal.alerte = BANDEAU.desaccord;
          rafraichirBandeau(canal);
          throw new Error(
            `La question de rang ${d.rang} n'est plus celle à laquelle vous avez ` +
            `répondu. Rechargez cette page : la partie reprendra où elle en est.`);
        }
        const retenue = await gardee.promesse;
        canal.attendre("aucune");
        return retenue;
      }
      const connue = canal.reponseConnue(d.rang);
      if (connue !== undefined) {
        marquerRejeu(canal, true);
        canal.attendre("aucune");
        return connue;
      }
      finirLeRejeuApresLeRendu(canal);
      // Le moteur vient de dire à qui revient ce rang. On le rapporte au
      // serveur : c'est ainsi, et seulement ainsi, qu'il peut refuser qu'un
      // siège réponde à la place de l'autre — sans connaître une seule règle.
      canal.attendre("moi", d.type);
      // MA QUESTION EST-ELLE LA PREMIÈRE D'UN CHOIX FACE CACHÉE ? Si oui, le
      // rendez-vous doit le savoir AVANT que je puisse répondre, sans quoi il
      // publierait ma réponse et l'autre la lirait avant de choisir. C'est
      // pourquoi on l'annonce, et qu'on l'attend, avant de poser la question.
      const suivante = await questionSuivanteInvariante(canal, d);
      const groupe = suivante ? { debut: d.rang, taille: 2 } : null;
      await canal.annoncerTour(d.rang, d.joueur, groupe);
      // (les-ecrans-manquants) ON ARME LA GARDE : à partir d'ici et jusqu'à mon
      // clic, toute avancée du rendez-vous au-delà de ce rang veut dire qu'une
      // autre page a répondu pour moi. On ne l'arme QUE si le rendez-vous n'a
      // pas déjà dépassé ce rang — sans quoi une page qui reprend après une
      // coupure s'accuserait elle-même.
      if (canal.rangAttendu <= d.rang) canal.monRangEnCours = d.rang;
      let reponse;
      try {
        reponse = await local.decider(d, etat);
      } finally {
        if (canal.monRangEnCours === d.rang) canal.monRangEnCours = null;
      }
      const retenue = await canal.publier(d.rang, reponse);
      canal.attendre("aucune");
      return retenue;
    },
  };

  // LE SIÈGE D'EN FACE. Il est tenu par un humain, ailleurs. On ne décide RIEN
  // à sa place : on attend sa réponse, aussi longtemps qu'il le faut, et on
  // affiche l'attente.
  fournisseurs[1 - siege] = {
    nom: "joueur distant",
    async decider(d, etat) {
      const connue = canal.reponseConnue(d.rang);
      if (connue !== undefined) {
        marquerRejeu(canal, true);
        canal.attendre("aucune");
        return connue;
      }
      finirLeRejeuApresLeRendu(canal);
      // L'écran continue de montrer la partie pendant qu'il réfléchit : sans
      // cela, mon écran se figerait sans que rien ne dise pourquoi.
      if (regarder) regarder(d, etat);
      canal.attendre("lui", d.type);
      // SA QUESTION EST-ELLE LA PREMIÈRE D'UN CHOIX FACE CACHÉE, DONT LA MIENNE
      // EST LA SECONDE ? Alors je n'ai aucune raison d'attendre : je pose la
      // mienne tout de suite, et je garde ma réponse de côté.
      const suivante = await questionSuivanteInvariante(canal, d);
      const groupe = suivante ? { debut: d.rang, taille: 2 } : null;
      const retenu = await canal.annoncerTour(d.rang, d.joueur, groupe);
      // On n'anticipe QUE si le rendez-vous a bien tiré le rideau : sans cela il
      // publierait la réponse de l'autre, et je pourrais la lire avant de
      // choisir. Le doute se paie par une attente, jamais par une fuite.
      if (suivante && suivante.joueur === siege
          && retenu && retenu.groupe && retenu.groupe.debut === d.rang) {
        garderDeCote(canal, local, suivante, etat, siege, groupe);
      }
      const reponse = await canal.attendreReponse(d.rang);
      canal.attendre("aucune");
      return reponse;
    },
  };

  return fournisseurs;
}

/**
 * La partie est finie : plus personne n'attend rien, et la resynchronisation de
 * secours n'a plus d'objet — elle interrogerait le serveur jusqu'à l'extinction
 * de l'ordinateur.
 *
 * Le flux d'évènements, LUI, reste ouvert : cette page est toujours là, devant
 * le tableau des scores, et l'autre joueur doit continuer à la voir présente.
 * On ne ment pas sur la présence, dans un sens comme dans l'autre.
 */
export function finDeLaPartieEnLigne(canal) {
  if (!canal) return;
  marquerRejeu(canal, false);
  if (canal.resync !== null) {
    clearInterval(canal.resync);
    canal.resync = null;
  }
  // Plus rien n'attend son rang : une réponse gardée de côté qui survivrait à la
  // partie attendrait une annonce qui ne viendra jamais.
  canal.gardees.clear();
  canal.rendezVousDeRang.clear();
  canal.attendre("aucune");
}

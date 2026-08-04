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
    bandeau.style.cssText = [
      "position:fixed", "left:12px", "bottom:12px", "z-index:9999",
      "max-width:min(46ch,60vw)", "padding:8px 12px", "border-radius:8px",
      "background:rgba(12,14,20,.86)", "color:#e8eef7",
      "font:500 13px/1.35 system-ui,sans-serif", "letter-spacing:.01em",
      "box-shadow:0 2px 12px rgba(0,0,0,.45)",
      // Un bandeau ne prend JAMAIS un clic : la table reste entièrement jouable.
      "pointer-events:none", "user-select:none",
    ].join(";");
    document.body.appendChild(bandeau);
  }
  bandeau.textContent = texte;
  bandeau.style.display = texte ? "block" : "none";
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
  else if (attente === "lui") montrerBandeau(BANDEAU.attente);
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

  canal.presence = (joueurs) => {
    if (!Array.isArray(joueurs)) return;
    canal.joueurs = [joueurs[0] === true, joueurs[1] === true];
    poser("data-adversaire", canal.joueurs[1 - canal.siege] ? "present" : "absent");
    rafraichirBandeau(canal);
  };

  canal.attendre = (nom) => {
    poser("data-attente", nom);
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

  /** Dit au serveur ce que le MOTEUR vient de dire : ce rang est à ce siège. */
  canal.annoncerTour = async (rang, siege) => {
    const { code, objet } = await envoyerJson(
      "relais/tour", { partie: canal.code, rang, siege });
    // Une annonce refusée est le SEUL signe que les deux moteurs ne voient pas
    // la même partie. Elle ne doit pas mourir dans la fenêtre de commandes, que
    // personne ne regardera : on la met sous les yeux du joueur.
    if (code === 409) {
      console.warn("rendez-vous : " + ((objet && objet.erreur) || "annonce de tour refusée"));
      canal.alerte = BANDEAU.desaccord;
      rafraichirBandeau(canal);
    }
  };

  /** Envoie notre réponse. Rend la réponse retenue par le serveur. */
  canal.publier = async (rang, reponse) => {
    const { code, objet } = await envoyerJson("relais/decision", {
      partie: canal.code, siege: canal.siege, rang, reponse,
    });
    if (code >= 200 && code < 300) {
      if (canal.decisions[rang] === undefined) canal.decisions[rang] = reponse;
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
    canal.presence(o.joueurs);
  });
  source.addEventListener("joueurs", (e) => {
    canal.presence(JSON.parse(e.data).joueurs);
  });
  source.addEventListener("decision", (e) => {
    const o = JSON.parse(e.data);
    const liste = canal.decisions.slice();
    liste[o.rang] = o.reponse;
    canal.adopter(liste);
  });

  // Filet de sécurité : si un évènement s'est perdu (réseau qui hoquette,
  // relais intermédiaire), on redemande l'état tant qu'on attend quelque chose.
  // C'est ce qui évite qu'une partie reste plantée demain matin.
  canal.resync = setInterval(async () => {
    if (!canal.attentes.size) return;
    const frais = await lireEtat(canal.code);
    if (frais) {
      canal.adopter(frais.decisions);
      canal.presence(frais.joueurs);
    }
  }, DELAI_RESYNC);

  // Quitter la page ferme le flux : l'autre voit le départ tout de suite.
  window.addEventListener("pagehide", () => source.close());

  rafraichirBandeau(canal);
  return canal;
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
      canal.attendre("moi");
      await canal.annoncerTour(d.rang, d.joueur);
      const reponse = await local.decider(d, etat);
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
      canal.attendre("lui");
      await canal.annoncerTour(d.rang, d.joueur);
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
  canal.attendre("aucune");
}

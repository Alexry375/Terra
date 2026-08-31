//! **LA BALANCE, NATIVE** — jumelle exacte de `web/webapp/verif/duel.mjs`.
//!
//!     duel <joueurA> <joueurB> [graines] [boites]
//!     duel --journal <joueurA> <joueurB> --graine G [--boites B] [--echange]
//!
//! Elle répond à la même question que l'ancienne, et par le même chemin : deux
//! joueurs nommés, chaque graine jouée DEUX fois avec les sièges échangés, une
//! graine propre à chaque camp, puis le calcul qui dit si l'écart veut dire
//! quelque chose. Ce qui change, c'est qu'elle ne sort jamais du moteur compilé :
//! ni pont WebAssembly, ni rejeu de la partie à chaque coup.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **CE QUI DOIT ÊTRE IDENTIQUE, ET POURQUOI CHAQUE POINT EST DÉLICAT.**
//!
//! 1. **Les graines de camp** — même mélange, en arithmétique 32 bits, que
//!    `graineDuCamp`. Un mélange écrit en 64 bits donnerait d'autres nombres.
//! 2. **Le nombre de décisions jouées** — la boucle JavaScript le compte une
//!    fois par question POSÉE. Or le pont escamote certaines questions (une
//!    action sans option, une défausse sans carte) et l'étalon qui vend fait
//!    reposer la sienne : la vente de l'étalon coûte donc une décision de plus.
//!    Ce fichier reproduit les deux règles, faute de quoi le contrôle 06 verrait
//!    deux balances qui rendent le même score sans avoir joué les mêmes parties.
//! 3. **L'arrondi** — `toFixed` du JavaScript arrondit la moitié VERS LE HAUT,
//!    là où Rust arrondit vers le chiffre pair. Sur 8 parties, un écart de score
//!    moyen de 0,125 s'écrit « 0.13 » d'un côté et « 0.12 » de l'autre : deux
//!    balances d'accord sur tout se contrediraient sur la seule ligne qu'un
//!    contrôle compare. [`tofixed`] refait la règle du JavaScript.
//! 4. **Le joueur qui tire au sort** — même générateur (xorshift32), mêmes
//!    tirages, dans le même ordre, y compris là où le pont pose une question
//!    dont l'unique issue est « passer ».

use std::cell::Cell;

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::choice::ChoiceContext;
use engine::description::Description;
use engine::effects::RevealFilter;
use engine::flow::{play_round, score_parts, setup_game};
use engine::joueur::Joueur;
use engine::policy::{ActionOpt, ConstructionBonus, Policy};
use engine::reflechi::Reflechi;
use engine::reseau::{Pile, Reseau};
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;
use rand::rngs::StdRng;
use serde_json::{json, Value};

const BOITES_PAR_DEFAUT: &str = "base,decouverte";
const SEUIL_ECARTS: f64 = 2.0;

/// Les poids du joueur artificiel. Même défaut et même variable d'environnement
/// que `web/webapp/joueurs/apprenti.js` : sans elle, la balance native et
/// l'ancienne pèseraient deux joueurs différents sans le dire.
const POIDS_PAR_DEFAUT: &str = "data/poids/apprenti.txt";

fn mourir(msg: &str) -> ! {
    println!("{msg}");
    std::process::exit(2);
}

// ───────────────────────────────────────────── l'arrondi du JavaScript

/// `Number.prototype.toFixed`, à la lettre : le nombre est écrit avec `f`
/// décimales, et **la moitié va vers le haut** — la norme choisit le plus GRAND
/// des deux entiers également proches. Rust, lui, arrondit vers le chiffre pair.
///
/// On ne peut pas s'en remettre à `{:.2}` : les deux règles ne diffèrent que sur
/// les valeurs exactement à mi-chemin, mais un écart de score moyen tombe
/// dessus dès que le nombre de parties est une puissance de deux (0,125 sur huit
/// parties). La différence porterait alors sur la seule ligne que les contrôles
/// 04 et 06 comparent.
///
/// Le chemin : on écrit d'abord la valeur binaire exacte avec largement assez de
/// décimales — un flottant est un rationnel dyadique, son écriture décimale est
/// finie — puis on arrondit cette écriture-là, à la main.
fn tofixed(x: f64, f: usize) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    let negatif = x < 0.0;
    let exact = format!("{:.*}", 80, x.abs());
    let (entiere, decimales) = match exact.split_once('.') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (exact.clone(), String::new()),
    };
    let mut chiffres: Vec<u8> = entiere.bytes().map(|b| b - b'0').collect();
    let gardees: Vec<u8> = decimales
        .bytes()
        .take(f)
        .map(|b| b - b'0')
        .chain(std::iter::repeat(0))
        .take(f)
        .collect();
    chiffres.extend_from_slice(&gardees);
    // Le reste décide : au-dessus de la moitié on monte, en dessous on descend,
    // et à la moitié EXACTE on monte aussi (« le plus grand des deux »).
    let reste: Vec<u8> = decimales.bytes().skip(f).map(|b| b - b'0').collect();
    let monter = match reste.first() {
        None => false,
        Some(&d) if d > 5 => true,
        Some(&d) if d < 5 => false,
        _ => true, // 5 suivi de n'importe quoi : la moitié ou plus
    };
    if monter {
        let mut i = chiffres.len();
        loop {
            if i == 0 {
                chiffres.insert(0, 1);
                break;
            }
            i -= 1;
            if chiffres[i] == 9 {
                chiffres[i] = 0;
            } else {
                chiffres[i] += 1;
                break;
            }
        }
    }
    let texte: String = chiffres.iter().map(|d| (d + b'0') as char).collect();
    let coupe = texte.len() - f;
    let mut sortie = String::new();
    // LE SIGNE SURVIT À L'ARRONDI. `Number.prototype.toFixed` pose le « - » sur
    // le seul critère `x < 0`, AVANT d'arrondir : `(-0.001).toFixed(2)` s'écrit
    // « -0.00 » et non « 0.00 ». Le zéro négatif, lui, n'est pas `< 0` et
    // s'écrit « 0.00 » des deux côtés. Un écart de score moyen très faiblement
    // négatif tombe exactement dessus, et c'est la ligne que les contrôles 04
    // et 06 comparent caractère par caractère.
    if negatif {
        sortie.push('-');
    }
    sortie.push_str(&texte[..coupe]);
    if f > 0 {
        sortie.push('.');
        sortie.push_str(&texte[coupe..]);
    }
    sortie
}

/// La graine du CAMP, dérivée de la graine de partie par un mélange propre à
/// chaque camp. Terme pour terme `graineDuCamp` (`duel.mjs`, ligne 111), en
/// arithmétique 32 bits : `graine * 2654435761` déborde, et c'est le débordement
/// qui fait le mélange.
///
/// **UNE LIMITE, écrite plutôt que tue.** Le JavaScript fait ce produit en
/// flottant double avant de le tronquer par le `^` : au-delà d'environ 3,4
/// millions de graines, le produit dépasse 2⁵³, le double perd les bits de poids
/// faible, et les deux calculs cessent de coïncider. Le `wrapping_mul` ci-dessous
/// est exact et donc, ce jour-là, DIFFÉRENT. Aucun duel du projet n'approche cet
/// ordre de grandeur — la balance la plus lourde tourne sur quelques centaines de
/// graines — mais la borne existe et ne se devine pas à la lecture.
fn graine_du_camp(graine: u32, camp: usize) -> u32 {
    let sel: u32 = if camp == 0 { 0x9e37_79b9 } else { 0x85eb_ca6b };
    let x = graine.wrapping_mul(2_654_435_761) ^ sel;
    let x = (x ^ (x >> 15)).wrapping_mul(0x2545_f491);
    if x == 0 {
        1
    } else {
        x
    }
}

// ───────────────────────────────────────────────── le joueur qui tire au sort

/// **LE HASARD, BIT POUR BIT.** Jumeau de `fournisseurAleatoire`
/// (`web/webapp/fournisseurs.js`) : même générateur xorshift32, même conversion
/// en flottant, même formule de tirage entre deux bornes — et surtout, les
/// tirages dans le MÊME ORDRE, sans quoi les deux parties divergent dès la
/// première décision.
///
/// Il ne vend jamais : le fournisseur JavaScript n'expose pas de méthode
/// `vendre`, et `partie.js` ne propose l'occasion qu'à ceux qui en ont une.
struct Hasard {
    x: u32,
}

impl Hasard {
    fn new(graine: u32) -> Hasard {
        Hasard {
            x: if graine == 0 { 0x9e37_79b9 } else { graine },
        }
    }

    /// Un flottant de [0, 1[, comme `alea`.
    fn r(&mut self) -> f64 {
        self.x ^= self.x << 13;
        self.x ^= self.x >> 17;
        self.x ^= self.x << 5;
        self.x as f64 / 4_294_967_296.0
    }

    /// Un entier de `min` à `max` inclus, comme `entre`.
    fn entre(&mut self, min: i64, max: i64) -> i64 {
        min + (self.r() * (max - min + 1) as f64).floor() as i64
    }

    /// Le mélange de Fisher-Yates de `fournisseurAleatoire`, parcouru du haut
    /// vers le bas comme lui.
    fn melange(&mut self, n: usize) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..n).collect();
        let mut i = n;
        while i > 1 {
            i -= 1;
            let j = self.entre(0, i as i64) as usize;
            indices.swap(i, j);
        }
        indices
    }

    /// Un choix simple parmi `n` options, `passer` compté en plus s'il est
    /// offert (`nombreDeChoix`).
    fn simple(&mut self, n: usize, passer: bool) -> usize {
        let choix = n + usize::from(passer);
        self.entre(0, choix as i64 - 1) as usize
    }
}

impl Policy for Hasard {
    fn corp_mulligan(&mut self, _rng: &mut StdRng, _player: usize, _corps: &[u16]) -> bool {
        self.simple(2, false) == 1
    }

    fn project_mulligan(&mut self, _rng: &mut StdRng, _player: usize, hand: &[u16]) -> Vec<usize> {
        let n = hand.len();
        let indices = self.melange(n);
        // `a_choisir` absent = nombre LIBRE : la quantité se tire aussi.
        let combien = self.entre(0, n as i64) as usize;
        indices.into_iter().take(combien).collect()
    }

    fn pick_corporation(&mut self, _rng: &mut StdRng, _player: usize, corps: &[u16]) -> usize {
        self.simple(corps.len(), false)
    }

    fn pick_phase(&mut self, _rng: &mut StdRng, _player: usize, allowed: &[u8]) -> u8 {
        allowed[self.simple(allowed.len(), false)]
    }

    fn choose_build(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        // La question est posée MÊME sans option payable : le tirage a lieu, et
        // son unique issue est « passer ». L'oublier décalerait toute la suite du
        // générateur.
        let i = self.simple(affordable.len(), true);
        affordable.get(i).copied()
    }

    fn construction_bonus(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        match self.simple(3, false) {
            0 => ConstructionBonus::DrawCardBefore,
            1 => ConstructionBonus::DrawCard,
            _ => ConstructionBonus::SecondBuild,
        }
    }

    fn construction_bonus_avant(&mut self, _rng: &mut StdRng, _player: usize) -> bool {
        self.simple(2, false) == 0
    }

    fn construction_bonus_apres(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        if self.simple(2, false) == 1 {
            ConstructionBonus::SecondBuild
        } else {
            ConstructionBonus::DrawCard
        }
    }

    fn action_choice(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None; // le pont n'ouvre pas la question
        }
        let i = self.simple(options.len(), true);
        if i < options.len() {
            Some(i)
        } else {
            None
        }
    }

    fn action_amount(&mut self, _rng: &mut StdRng, _player: usize, max: i64) -> i64 {
        self.entre(0, max)
    }

    fn choose_option(&mut self, _rng: &mut StdRng, _player: usize, n: usize) -> usize {
        self.simple(n, false)
    }

    fn choose_option_ctx(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        if n == 0 {
            return 0;
        }
        self.simple(n, false)
    }

    fn choose_res_target(&mut self, _rng: &mut StdRng, _player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        self.simple(candidates.len(), false)
    }

    fn choose_res_source(&mut self, _rng: &mut StdRng, _player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        self.simple(candidates.len(), false)
    }

    fn pick_joker_tag(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _card: u16,
        _tag_counts: &[u32],
    ) -> usize {
        self.simple(engine::cards::JOKER_TAG_CHOICES.len(), false)
    }

    fn research_keep(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        let indices = self.melange(drawn.len());
        indices.into_iter().take(keep).collect()
    }

    fn reveal_pick(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        _filter: RevealFilter,
    ) -> Vec<usize> {
        // Le pont MONTRE la révélation même quand rien n'est prenable : la
        // question est posée, donc le mélange a lieu.
        let indices = self.melange(candidates.len());
        indices.into_iter().take(keep).collect()
    }

    fn discard_down(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        if hand.is_empty() || n == 0 {
            return Vec::new();
        }
        let indices = self.melange(hand.len());
        indices.into_iter().take(n).collect()
    }
}

// ────────────────────────────────────────────────────── les deux camps

/// Un camp de la balance. L'énumération sert à deux choses que le trait ne dit
/// pas : le NOM du joueur, et le fait qu'il vende en RÉPONDANT plutôt qu'en
/// saisissant l'occasion — ce qui change le compte des décisions.
enum Cerveau<'a> {
    Hasard(Hasard),
    Reflechi(Reflechi<'a>),
    /// Le joueur artificiel du dépôt, tel quel : `engine::joueur::Joueur`, le
    /// même code que l'entraînement emploie. Encadré parce qu'il est gros —
    /// réseau, tampons, journal — et qu'une énumération se dimensionne sur sa
    /// plus grande branche.
    Apprenti(Box<Joueur<'a>>),
}

impl Cerveau<'_> {
    fn politique(&mut self) -> &mut dyn Policy {
        match self {
            Cerveau::Hasard(h) => h,
            Cerveau::Reflechi(r) => r,
            Cerveau::Apprenti(j) => j.as_mut(),
        }
    }

    /// **Vend-il en répondant à une question ?** L'étalon n'a pas de méthode
    /// `vendre` : il glisse son geste dans la RÉPONSE, le moteur le consomme à
    /// l'occasion qui précède, puis repose la même question. Sa vente coûte donc
    /// une décision de plus au décompte de la boucle JavaScript. Un joueur qui
    /// saisit l'occasion (l'apprenti) n'en coûte aucune.
    fn vend_en_repondant(&self) -> bool {
        matches!(self, Cerveau::Reflechi(_))
    }

    /// **Saisit-il l'occasion elle-même ?** `partie.js:offrirLesOccasions`
    /// n'interroge que les fournisseurs qui exposent une méthode `vendre` : le
    /// joueur artificiel l'expose, l'étalon et le joueur qui tire au sort non.
    fn saisit_l_occasion(&self) -> bool {
        matches!(self, Cerveau::Apprenti(_))
    }
}

/// Ce qu'un camp apprenti garde d'une partie à l'autre : son réseau et la pile
/// de tampons qui lui évite d'allouer à chaque évaluation. Lus une fois, au
/// démarrage — relire le fichier de poids à chaque partie fausserait la mesure
/// de vitesse sans rien apporter.
struct Cervelle {
    reseau: Reseau,
    pile: Pile,
}

/// **LES DEUX CAMPS À LA TABLE, ET LE GREFFIER.**
///
/// Le moteur ne connaît qu'une politique ; celle-ci aiguille chaque question
/// vers le siège concerné, compte les décisions comme la boucle de `partie.js`
/// les compte, et — sur demande — tient le JOURNAL des réponses au format du
/// pont, celui que le juge de concordance compare.
struct Duo<'a> {
    camps: [Cerveau<'a>; 2],
    /// Le nombre de décisions au sens de `duel.mjs` : une par question posée.
    ///
    /// **IL VIT DEHORS, et ce n'est pas un détail de style.** `duel.mjs` compte
    /// ses décisions dans le rappel `avant`, donc une partie que le moteur
    /// interrompt en cours de route apporte quand même les siennes au total. En
    /// natif, la partie est jouée sous `catch_unwind` : si le compteur vivait
    /// dans le `Duo`, il disparaîtrait avec lui à la panique et les deux
    /// balances ne compteraient plus la même chose. Le cas ne s'est jamais
    /// produit sur les duels mesurés (`parties interrompues : 0`) — raison de
    /// plus pour le régler avant qu'il ne se produise.
    compteur: &'a Cell<u64>,
    /// Le rang de l'occasion de vente en cours, compté comme le pont le compte.
    occasions: u64,
    /// Le siège à qui la question qui suit l'occasion sera posée : c'est le seul
    /// qui puisse vendre EN RÉPONDANT.
    decideur: usize,
    /// Les mains des deux sièges à l'instant où l'occasion s'ouvre, relevées par
    /// `observer_l_occasion`. Le moteur n'interroge que les sièges dont la main
    /// n'est pas vide (`flow::occasion_de_vendre`) : il faut donc savoir, dès le
    /// premier appel, lesquels seront interrogés.
    mains: [Vec<u16>; 2],
    /// Les ventes de l'occasion en cours, une fois l'occasion résolue EN ENTIER.
    /// `None` tant qu'elle ne l'est pas.
    resolu: Option<[Vec<usize>; 2]>,
    /// Les réponses, au format que `partie.decisions` porte. `None` quand la
    /// balance n'a pas besoin du journal (le tenir coûte des allocations).
    journal: Option<Vec<Value>>,
}

impl<'a> Duo<'a> {
    fn new(camps: [Cerveau<'a>; 2], compteur: &'a Cell<u64>, journal: bool) -> Duo<'a> {
        compteur.set(0);
        Duo {
            camps,
            compteur,
            occasions: 0,
            decideur: 0,
            mains: [Vec::new(), Vec::new()],
            resolu: None,
            journal: if journal { Some(Vec::new()) } else { None },
        }
    }

    /// Une question a été posée et répondue : c'est une décision de plus, et une
    /// entrée de journal.
    fn note(&mut self, siege: usize, reponse: Value) {
        self.compteur.set(self.compteur.get() + 1);
        self.dire_a_l_autre(siege, &reponse);
        if let Some(j) = &mut self.journal {
            j.push(reponse);
        }
    }

    /// **CE QUE L'AUTRE SIÈGE VIENT DE RÉPONDRE, DIT À L'APPRENTI.**
    ///
    /// Le joueur artificiel essaie ses coups en REJOUANT la partie ; son rejeu
    /// consomme les réponses par leur rang, sans regarder de qui elles viennent.
    /// Seul contre lui-même, il les inscrit toutes ; face à un autre joueur, il
    /// n'entendrait que la moitié de la partie et rejouerait une partie amputée
    /// — chaque réponse manquante décalant toutes les suivantes.
    fn dire_a_l_autre(&mut self, siege: usize, reponse: &Value) {
        if let Cerveau::Apprenti(j) = &mut self.camps[1 - siege] {
            j.reponse_de_l_autre(reponse.clone());
        }
    }

    /// **L'OCCASION DE VENDRE, RÉSOLUE D'UN SEUL COUP — ET À POINT FIXE.**
    ///
    /// Le moteur offre l'occasion aux sièges par indices croissants, une seule
    /// fois chacun, puis pose la question. Le JavaScript, lui, tourne en rond
    /// jusqu'à ce que plus personne ne vende, et c'est cette boucle-là qu'il faut
    /// reproduire, parce qu'elle change les réponses :
    ///
    /// 1. `partie.js:offrirLesOccasions` interroge les seuls fournisseurs qui
    ///    exposent une méthode `vendre` — ceux qui SAISISSENT l'occasion, comme
    ///    le joueur artificiel — par sièges croissants, et RECOMMENCE la liste
    ///    dès que l'un d'eux vend ;
    /// 2. la question est posée. L'étalon n'expose pas `vendre` : il glisse son
    ///    geste dans sa RÉPONSE, que le pont replace à l'occasion qui précède ;
    /// 3. si l'étalon a vendu, `jouerJusquAuBout` repose la question — donc on
    ///    repart en 1, et le joueur artificiel se voit ré-offrir SON occasion,
    ///    qu'il n'avait pas encore dépensée, sur un état qui a changé.
    ///
    /// Deux règles du moteur bornent la boucle et se lisent dans
    /// `flow::observer` : une occasion ne se dépense qu'une fois par siège, et
    /// `occasion_de_vendre_ouverte` est faux pour un siège dès qu'un siège
    /// POSTÉRIEUR (ou lui-même) a dépensé la sienne — c'est le parcours à rebours
    /// et le drapeau `ferme`. L'étalon lit ce drapeau avant de vendre
    /// (`reflechi.js`, `venteEventuelle`) ; le joueur artificiel, lui, ne le lit
    /// pas : le pont lui ré-offre son occasion tant qu'elle n'est pas dépensée.
    ///
    /// Mesuré le 31-08, graine 1 sièges échangés : sans le point 1 l'étalon
    /// vendait à une occasion que l'adversaire venait de fermer (décision 69) ;
    /// sans le point 3 le joueur artificiel perdait une vente sur deux
    /// (décision 271).
    ///
    /// Le NUMÉRO d'une occasion, lui, ne bouge pas d'un passage à l'autre : c'est
    /// le rang de l'appel du moteur, et le pont le recompte depuis le début à
    /// chaque rejeu. On cale donc le compteur du camp avant chaque question.
    fn resoudre_l_occasion(&mut self, rng: &mut StdRng) {
        // Les sièges que le moteur va interroger : ceux dont la main n'est pas
        // vide (`flow::occasion_de_vendre` passe les autres sans un mot).
        let interroges: Vec<usize> = (0..2).filter(|&s| !self.mains[s].is_empty()).collect();
        let base = self.occasions;
        self.occasions += interroges.len() as u64;
        let mut res: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
        // Une garde, comme la boucle JavaScript en porte une : deux sièges, une
        // vente chacun au plus, mais un banc qui tournerait sans fin est pire
        // qu'un banc qui s'arrête en le disant.
        let mut garde = 0;
        loop {
            garde += 1;
            if garde > 100 {
                mourir("boucle d'occasions de vente anormalement longue");
            }
            let mut encore = true;
            while encore {
                encore = false;
                for &s in &interroges {
                    if !self.camps[s].saisit_l_occasion() || !res[s].is_empty() {
                        continue;
                    }
                    let idx = self.interroger_le_siege(rng, s, base, &interroges);
                    if !idx.is_empty() {
                        self.inscrire_la_vente(s, base, &interroges, &idx);
                        res[s] = idx;
                        encore = true;
                        break;
                    }
                }
            }
            // La question, et la vente que l'étalon glisse dans sa réponse. Un
            // seul siège peut la faire : celui à qui la question est posée.
            let s = self.decideur;
            if !interroges.contains(&s) || !self.camps[s].vend_en_repondant() {
                break;
            }
            let idx = self.interroger_le_siege(rng, s, base, &interroges);
            // `occasion_de_vendre_ouverte` : faux dès qu'un siège postérieur, ou
            // lui-même, a dépensé cette occasion-ci.
            let ferme = (s..2).any(|t| !res[t].is_empty());
            if idx.is_empty() || ferme {
                break;
            }
            self.inscrire_la_vente(s, base, &interroges, &idx);
            res[s] = idx;
        }
        // Le rang d'une occasion est compté par TOUS les camps, y compris celui
        // que le moteur n'a pas interrogé : c'est ainsi que le pont le compte.
        let fin = base + interroges.len() as u64;
        for c in 0..2 {
            if let Cerveau::Apprenti(j) = &mut self.camps[c] {
                j.caler_les_occasions(fin);
            }
        }
        self.resolu = Some(res);
    }

    /// Offre l'occasion à un siège, son compteur calé sur le rang que le moteur
    /// donnera à CET appel-ci.
    fn interroger_le_siege(
        &mut self,
        rng: &mut StdRng,
        s: usize,
        base: u64,
        interroges: &[usize],
    ) -> Vec<usize> {
        let numero = base + Self::rang(s, interroges);
        if let Cerveau::Apprenti(j) = &mut self.camps[s] {
            j.caler_les_occasions(numero);
        }
        let main = self.mains[s].clone();
        self.camps[s].politique().vendre_librement(rng, s, &main)
    }

    /// Inscrit une vente au journal, au format que porte `partie.decisions`.
    fn inscrire_la_vente(&mut self, s: usize, base: u64, interroges: &[usize], idx: &[usize]) {
        let entree = if self.camps[s].vend_en_repondant() {
            // L'étalon a répondu « je vends » à la question qui vient : la boucle
            // JavaScript compte cette réponse-là comme une décision, puis repose
            // la même question. L'ordre des clefs est celui de l'original.
            self.compteur.set(self.compteur.get() + 1);
            json!({ "vendre": { "joueur": s, "cartes": idx } })
        } else {
            // Un joueur qui SAISIT l'occasion inscrit son geste sans qu'aucune
            // question ait été posée : rien à compter, et le numéro d'occasion —
            // son rang d'appel — fait partie de l'entrée.
            json!({
                "vendre": { "cartes": idx, "joueur": s, "occasion": base + Self::rang(s, interroges) }
            })
        };
        self.dire_a_l_autre(s, &entree);
        if let Some(j) = &mut self.journal {
            j.push(entree);
        }
    }

    /// Le rang de l'appel du moteur pour ce siège, parmi les sièges interrogés.
    fn rang(s: usize, interroges: &[usize]) -> u64 {
        interroges.iter().position(|&x| x == s).unwrap_or(0) as u64
    }

    /// Le début d'une manche, pour les camps qui en ont besoin : c'est le point
    /// de reprise de leurs essais de coups (`Joueur::debut_manche`).
    fn debut_manche(&mut self, game: &GameState) {
        for c in self.camps.iter_mut() {
            if let Cerveau::Apprenti(j) = c {
                j.debut_manche(game);
            }
        }
    }
}

impl Policy for Duo<'_> {
    fn observe(&mut self, game: &GameState, player: usize) {
        self.camps[0].politique().observe(game, player);
        self.camps[1].politique().observe(game, player);
    }

    fn observer_l_occasion(&mut self, game: &GameState, decideur: usize, question_posee: bool) {
        // Une occasion neuve commence : rien n'y est encore résolu, et l'on
        // relève les deux mains telles que le moteur va les lire.
        self.resolu = None;
        self.decideur = decideur;
        self.mains = [game.players[0].hand.clone(), game.players[1].hand.clone()];
        self.camps[0]
            .politique()
            .observer_l_occasion(game, decideur, question_posee);
        self.camps[1]
            .politique()
            .observer_l_occasion(game, decideur, question_posee);
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        let r = self.camps[player].politique().corp_mulligan(rng, player, corps);
        self.note(player, json!(usize::from(r)));
        r
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        let r = self.camps[player]
            .politique()
            .project_mulligan(rng, player, hand);
        self.note(player, json!(r));
        r
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        let r = self.camps[player]
            .politique()
            .pick_corporation(rng, player, corps);
        self.note(player, json!(r));
        r
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        let r = self.camps[player].politique().pick_phase(rng, player, allowed);
        // La réponse attendue par le pont est l'INDICE dans les options, pas le
        // numéro de la phase.
        let i = allowed.iter().position(|&p| p == r).unwrap_or(0);
        self.note(player, json!(i));
        r
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        let r = self.camps[player]
            .politique()
            .choose_build(rng, player, affordable);
        // « passer » porte l'indice `options.length`.
        let i = match r {
            Some(x) => affordable
                .iter()
                .position(|&a| a == x)
                .unwrap_or(affordable.len()),
            None => affordable.len(),
        };
        self.note(player, json!(i));
        r
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let r = self.camps[player].politique().construction_bonus(rng, player);
        self.note(player, json!(match r {
            ConstructionBonus::DrawCardBefore => 0,
            ConstructionBonus::DrawCard => 1,
            ConstructionBonus::SecondBuild => 2,
        }));
        r
    }

    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        let r = self.camps[player]
            .politique()
            .construction_bonus_avant(rng, player);
        self.note(player, json!(if r { 0 } else { 1 }));
        r
    }

    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let r = self.camps[player]
            .politique()
            .construction_bonus_apres(rng, player);
        self.note(player, json!(match r {
            ConstructionBonus::SecondBuild => 1,
            _ => 0,
        }));
        r
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        // Sans option à activer, le pont n'ouvre pas la question : rien à
        // compter, rien à journaliser.
        if options.is_empty() {
            return None;
        }
        let r = self.camps[player]
            .politique()
            .action_choice(rng, player, options);
        self.note(player, json!(r.unwrap_or(options.len())));
        r
    }

    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        let r = self.camps[player].politique().action_amount(rng, player, max);
        self.note(player, json!(r));
        r
    }

    /// **L'OCCASION DE VENDRE.** Le moteur l'offre à chaque siège dont la main
    /// n'est pas vide, par indices croissants. On la résout ENTIÈRE au premier
    /// appel — l'ordre du JavaScript n'est pas celui du moteur, et il change la
    /// réponse : voir [`Duo::resoudre_l_occasion`].
    fn vendre_librement(&mut self, rng: &mut StdRng, joueur: usize, _main: &[u16]) -> Vec<usize> {
        if self.resolu.is_none() {
            self.resoudre_l_occasion(rng);
        }
        match &self.resolu {
            Some(r) => r[joueur].clone(),
            None => Vec::new(),
        }
    }

    fn choose_option(&mut self, rng: &mut StdRng, player: usize, n: usize) -> usize {
        let r = self.camps[player].politique().choose_option(rng, player, n);
        self.note(player, json!(r));
        r
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        if ctx.option_count() == 0 {
            return 0;
        }
        let r = self.camps[player]
            .politique()
            .choose_option_ctx(rng, player, ctx);
        self.note(player, json!(r));
        r
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let r = self.camps[player]
            .politique()
            .choose_res_target(rng, player, candidates);
        self.note(player, json!(r));
        r
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let r = self.camps[player]
            .politique()
            .choose_res_source(rng, player, candidates);
        self.note(player, json!(r));
        r
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        card: u16,
        tag_counts: &[u32],
    ) -> usize {
        let r = self.camps[player]
            .politique()
            .pick_joker_tag(rng, player, card, tag_counts);
        self.note(player, json!(r));
        r
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        let r = self.camps[player]
            .politique()
            .research_keep(rng, player, drawn, keep);
        self.note(player, json!(r));
        r
    }

    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        filter: RevealFilter,
    ) -> Vec<usize> {
        let r = self.camps[player]
            .politique()
            .reveal_pick(rng, player, revealed, candidates, keep, filter);
        self.note(player, json!(r));
        r
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        // Une question sans réponse possible n'est pas posée par le pont.
        if hand.is_empty() || n == 0 {
            return Vec::new();
        }
        let r = self.camps[player]
            .politique()
            .discard_down(rng, player, hand, n);
        self.note(player, json!(r));
        r
    }
}

// ─────────────────────────────────────────────────────── une partie entière

/// Ce qu'une partie rend à la balance.
struct Resultat {
    scores: [i64; 2],
    /// La partie s'est-elle arrêtée d'elle-même, et non sur le plafond de
    /// manches ? C'est le `partie_complete` du pont.
    complete: bool,
    decisions: u64,
    journal: Vec<Value>,
}

/// Fabrique le cerveau d'un joueur nommé, à son siège.
///
/// `cervelle` n'est renseignée que pour l'apprenti : c'est son réseau, lu une
/// seule fois au démarrage et réemployé de partie en partie.
fn cerveau<'a>(
    nom: &str,
    db: &'a CardsDb,
    desc: &'a Description,
    graine: u32,
    graine_camp: u32,
    siege: usize,
    cervelle: &'a mut Option<Cervelle>,
) -> Cerveau<'a> {
    match nom {
        "hasard" => Cerveau::Hasard(Hasard::new(graine_camp)),
        // L'étalon ne tire rien au sort : la graine du camp lui est offerte pour
        // respecter la signature des autres joueurs, et ignorée — comme dans
        // l'original.
        "reflechi" => Cerveau::Reflechi(Reflechi::new(db, siege)),
        "apprenti" => {
            let Some(c) = cervelle.as_mut() else {
                mourir("apprenti sans réseau : le fichier de poids n'a pas été lu")
            };
            // Réglé comme `engine/src/bin/jouer.rs`, et pour la même raison : on
            // MESURE une force. Un joueur qui explore pendant sa propre notation
            // se sabote, et un joueur qui apprend pendant la mesure ne serait
            // plus le même d'une partie à l'autre.
            let mut j = Joueur::new(db, desc, &mut c.reseau, &mut c.pile, graine as u64);
            j.exploration = 0.0;
            j.apprendre = false;
            j.nouvelle_partie(graine as u64);
            Cerveau::Apprenti(Box::new(j))
        }
        autre => mourir(&format!(
            "joueur inconnu : « {autre} » — joueurs connus : {}",
            JOUEURS.join(", ")
        )),
    }
}

/// Les joueurs que la balance connaît, écrits à la main comme dans l'original :
/// une liste découverte en parcourant un dossier ne se répéterait pas d'une
/// machine à l'autre.
const JOUEURS: [&str; 3] = ["hasard", "reflechi", "apprenti"];

/// Joue une partie entière. `noms[i]` occupe le siège `i`, et `cervelles.i` est
/// le réseau du camp qui y est assis.
#[allow(clippy::too_many_arguments)]
fn une_partie(
    db: &CardsDb,
    desc: &Description,
    graine: u32,
    noms: [&str; 2],
    graines_camp: [u32; 2],
    cervelle_0: &mut Option<Cervelle>,
    cervelle_1: &mut Option<Cervelle>,
    compteur: &Cell<u64>,
    journal: bool,
) -> Resultat {
    let camps = [
        cerveau(noms[0], db, desc, graine, graines_camp[0], 0, cervelle_0),
        cerveau(noms[1], db, desc, graine, graines_camp[1], 1, cervelle_1),
    ];
    let mut duo = Duo::new(camps, compteur, journal);
    // La boucle du pont, mot pour mot (`wasm/src/lib.rs`, `pas`).
    let mut game = setup_game(db, graine as u64, &mut duo);
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        duo.debut_manche(&game);
        play_round(&mut game, db, &mut duo);
    }
    let (scores, _, _) = score_parts(&game, db);
    Resultat {
        scores: [scores[0], scores[1]],
        complete: game.game_over,
        decisions: duo.compteur.get(),
        journal: duo.journal.unwrap_or_default(),
    }
}

/// Le réseau d'un camp apprenti, ou rien pour les autres joueurs. Le chemin des
/// poids est celui du contrat, et la même variable d'environnement que du côté
/// JavaScript le déplace : les deux balances doivent peser le MÊME joueur.
fn cervelle_de(nom: &str, desc: &Description, db: &CardsDb) -> Option<Cervelle> {
    if nom != "apprenti" {
        return None;
    }
    // **CE QUE CETTE BALANCE NE SAIT PAS PESER, ELLE LE DIT ET S'ARRÊTE.**
    // `APPRENTI_ADVERSAIRE` allume la « devinette » du joueur artificiel côté
    // JavaScript (`web/webapp/joueurs/apprenti.js`) : un second réseau, chargé
    // depuis ce chemin, qui change ses décisions du tout au tout — mesuré sur
    // deux graines, 2 victoires sur 2 sans la variable, 0 sur 2 avec. Le portage
    // ne l'a pas reprise. La lire et l'ignorer ferait mesurer DEUX JOUEURS
    // DIFFÉRENTS aux deux balances sans que personne ne s'en aperçoive : c'est
    // très exactement ce que ce lot devait rendre impossible. On refuse de
    // démarrer plutôt que de rendre un nombre faux.
    if let Ok(v) = std::env::var("APPRENTI_ADVERSAIRE") {
        if !v.is_empty() {
            mourir(
                "APPRENTI_ADVERSAIRE est posée : la balance native ne sait pas encore porter \
                 la devinette du joueur artificiel, et mesurerait un autre joueur que \
                 duel.mjs. Retirer la variable, ou mesurer avec la balance JavaScript.",
            );
        }
    }
    let chemin = std::env::var("APPRENTI_POIDS").unwrap_or_else(|_| POIDS_PAR_DEFAUT.to_string());
    let chemin = if std::path::Path::new(&chemin).exists() {
        chemin
    } else {
        format!("../{chemin}")
    };
    let noms = desc.noms_avec(db);
    match Reseau::lire(&chemin, &noms) {
        Ok(reseau) => Some(Cervelle {
            reseau,
            pile: Pile::new(desc.taille),
        }),
        Err(e) => mourir(&format!("poids de l'apprenti illisibles ({chemin}) : {e}")),
    }
}

// ──────────────────────────────────────────────────────────────── le duel

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Les paniques du moteur sont rattrapées partie par partie ; on ne veut pas
    // que chacune écrive sa trace sur la sortie d'erreur du banc.
    std::panic::set_hook(Box::new(|_| {}));

    if args.first().map(String::as_str) == Some("--journal") {
        journal(&args[1..]);
        return;
    }
    if args.len() < 2 {
        mourir(&format!(
            "il faut deux joueurs : duel <joueurA> <joueurB> [graines] [boites] — \
             joueurs connus : {}",
            JOUEURS.join(", ")
        ));
    }
    let nom_a = args[0].clone();
    let nom_b = args[1].clone();
    let graines: u32 = match args.get(2) {
        None => 100,
        Some(v) => match v.parse::<u32>() {
            Ok(n) if n >= 1 => n,
            _ => mourir(&format!("nombre de graines invalide : « {v} »")),
        },
    };
    let boites_txt = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| BOITES_PAR_DEFAUT.to_string());
    let db = charger(&boites_txt);
    for nom in [&nom_a, &nom_b] {
        if !JOUEURS.contains(&nom.as_str()) {
            mourir(&format!(
                "joueur inconnu : « {nom} » — joueurs connus : {}",
                JOUEURS.join(", ")
            ));
        }
    }

    // Les cartes, la description d'un état et — pour un camp apprenti — son
    // réseau : lus UNE fois. La mesure de vitesse porte sur les parties, pas sur
    // la lecture d'un fichier de poids de plusieurs mégaoctets.
    let desc = Description::new(&db);
    let mut cervelle_a = cervelle_de(&nom_a, &desc, &db);
    let mut cervelle_b = cervelle_de(&nom_b, &desc, &db);

    let mut victoires_a = 0u32;
    let mut victoires_b = 0u32;
    let mut nuls = 0u32;
    let mut decisions = 0u64;
    // Le compteur de décisions vit ICI, hors de la partie : voir `Duo::compteur`.
    let compteur = Cell::new(0u64);
    let mut ecart_total = 0i64;
    // Ce que la balance ne doit pas taire : une partie arrêtée sur le plafond de
    // manches n'a pas fini d'elle-même, et une partie que le moteur a refusé de
    // continuer n'a pas eu lieu du tout.
    let mut plafonnees = 0u32;
    let mut interrompues = 0u32;
    let mut premiere_casse: Vec<String> = Vec::new();

    for g in 1..=graines {
        for echange in [false, true] {
            let noms: [&str; 2] = if echange {
                [nom_b.as_str(), nom_a.as_str()]
            } else {
                [nom_a.as_str(), nom_b.as_str()]
            };
            // La graine d'un camp ne dépend pas du siège où il est assis : c'est
            // bien le joueur qu'on compare, pas sa place.
            let graines_camp: [u32; 2] = if echange {
                [graine_du_camp(g, 1), graine_du_camp(g, 0)]
            } else {
                [graine_du_camp(g, 0), graine_du_camp(g, 1)]
            };
            // `AssertUnwindSafe` : les deux réseaux traversent la frontière en
            // emprunt mutable. Ce qu'une panique pourrait y laisser d'incohérent
            // est effacé à la partie suivante — `Joueur::nouvelle_partie` remet
            // le réseau et le journal à zéro avant le premier coup.
            let (c0, c1) = if echange {
                (&mut cervelle_b, &mut cervelle_a)
            } else {
                (&mut cervelle_a, &mut cervelle_b)
            };
            let joue = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                une_partie(&db, &desc, g, noms, graines_camp, c0, c1, &compteur, false)
            }));
            let r = match joue {
                Ok(r) => r,
                Err(e) => {
                    interrompues += 1;
                    if premiere_casse.len() < 3 {
                        let quoi = match e.downcast_ref::<String>() {
                            Some(s) => s.clone(),
                            None => match e.downcast_ref::<&str>() {
                                Some(s) => (*s).to_string(),
                                None => "panique du moteur".to_string(),
                            },
                        };
                        premiere_casse.push(format!(
                            "graine {g}, {} : {}",
                            if echange {
                                "sièges échangés"
                            } else {
                                "sièges directs"
                            },
                            quoi.split('\n').next().unwrap_or("")
                        ));
                    }
                    // LES DÉCISIONS D'UNE PARTIE INTERROMPUE COMPTENT QUAND
                    // MÊME : c'est ce que fait `duel.mjs`, qui les a déjà
                    // additionnées au fil de la partie quand l'exception
                    // survient.
                    decisions += compteur.get();
                    continue;
                }
            };
            decisions += r.decisions;
            if !r.complete {
                plafonnees += 1;
            }
            let (score_a, score_b) = if echange {
                (r.scores[1], r.scores[0])
            } else {
                (r.scores[0], r.scores[1])
            };
            ecart_total += score_a - score_b;
            if score_a > score_b {
                victoires_a += 1;
            } else if score_b > score_a {
                victoires_b += 1;
            } else {
                nuls += 1;
            }
        }
    }

    // L'écart veut-il dire quelque chose ? Le calcul de `duel.mjs`, terme pour
    // terme : sur `n` parties décisives, une pièce équilibrée donnerait `n / 2`
    // victoires, à `racine(n) / 2` près.
    let parties = graines * 2;
    let decisives = victoires_a + victoires_b;
    let attendu = decisives as f64 / 2.0;
    let ecart_typique = (decisives as f64).sqrt() / 2.0;
    let ecarts = if ecart_typique > 0.0 {
        (victoires_a as f64 - attendu) / ecart_typique
    } else {
        0.0
    };
    let significatif = ecarts.abs() >= SEUIL_ECARTS;
    let pourcent = |x: u32| tofixed(100.0 * x as f64 / parties as f64, 1);

    println!(
        "duel : « {nom_a} » contre « {nom_b} » — {graines} graines × 2 sièges = {parties} \
         parties (boîtes {boites_txt})"
    );
    println!(
        "« {nom_a} » gagne {victoires_a} parties sur {parties} ({} %)",
        pourcent(victoires_a)
    );
    println!(
        "« {nom_b} » gagne {victoires_b} parties sur {parties} ({} %)",
        pourcent(victoires_b)
    );
    println!("nuls : {nuls}");
    println!(
        "écart de score moyen (« {nom_a} » − « {nom_b} ») : {} point(s)",
        tofixed(ecart_total as f64 / parties as f64, 2)
    );
    println!("décisions jouées : {decisions}");
    println!(
        "parties arrêtées sur le plafond de manches : {plafonnees} — leur score est un \
         instantané, il compte quand même dans le total"
    );
    println!("parties interrompues par le moteur : {interrompues}");
    for c in &premiere_casse {
        println!("  interruption — {c}");
    }
    println!(
        "parties décisives : {decisives} — attendu à l'équilibre : {} victoires, écart \
         typique : {}",
        tofixed(attendu, 1),
        tofixed(ecart_typique, 1)
    );
    println!(
        "on est à {} écart(s) typique(s) de l'équilibre (seuil : {})",
        tofixed(ecarts, 2),
        SEUIL_ECARTS as i64
    );
    if significatif {
        let meilleur = if ecarts > 0.0 { &nom_a } else { &nom_b };
        println!("verdict : écart significatif — « {meilleur} » est le meilleur des deux");
    } else {
        println!("verdict : dans le bruit — cet écart ne distingue pas les deux joueurs");
    }
    if interrompues > 0 {
        println!(
            "⚠ {interrompues} partie(s) n'ont pas été jouées jusqu'au bout : le verdict porte \
             sur {} parties, pas sur {parties}",
            parties - interrompues
        );
    }
}

/// **LE JOURNAL D'UNE PARTIE**, pour le juge de concordance.
///
///     duel --journal <joueurA> <joueurB> --graine G [--boites B]
///
/// Une seule ligne JSON : les réponses dans l'ordre, au format que
/// `partie.decisions` porte côté JavaScript, plus les scores. Le juge y compare
/// les deux listes décision par décision.
fn journal(args: &[String]) {
    if args.len() < 2 {
        mourir("duel --journal <joueurA> <joueurB> --graine G [--boites B]");
    }
    let nom_a = args[0].clone();
    let nom_b = args[1].clone();
    let mut graine: u32 = 1;
    let mut boites_txt = BOITES_PAR_DEFAUT.to_string();
    // Les sièges échangés : la seconde des deux parties que la balance joue sur
    // chaque graine. Les deux camps gardent leur propre graine — c'est le joueur
    // qu'on compare, pas sa place — et c'est justement ce que le juge doit
    // pouvoir rejouer, faute de quoi la moitié des parties de la balance ne
    // serait jamais comparée à son jumeau JavaScript.
    //
    // **CE DRAPEAU A ÉTÉ FAUX, et le juge l'a dit.** Il n'échangeait d'abord que
    // les GRAINES de camp, pas les NOMS : le joueur A restait assis au siège 0.
    // Cela ne se voyait pas sur un duel de l'étalon contre lui-même — il ne
    // consomme aucune graine, le journal était identique au caractère près — ni
    // sur un duel contre le hasard, où l'échange des graines suffit à changer la
    // partie. Il fallait un adversaire ASYMÉTRIQUE (`--adversaire=apprenti`)
    // pour que le natif joue une partie et le JavaScript une autre.
    let mut echange = false;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--echange" {
            echange = true;
            i += 1;
            continue;
        }
        let valeur = match args.get(i + 1) {
            Some(v) => v.clone(),
            None => mourir(&format!("valeur manquante pour {}", args[i])),
        };
        match args[i].as_str() {
            "--graine" => {
                graine = valeur
                    .parse()
                    .unwrap_or_else(|_| mourir("--graine attend un entier"))
            }
            "--boites" => boites_txt = valeur,
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += 2;
    }
    let db = charger(&boites_txt);
    let desc = Description::new(&db);
    // LE SIÈGE 0 D'ABORD, quel que soit le camp qui l'occupe — noms, graines et
    // cervelles rangés ensemble, exactement comme la boucle de la balance
    // ci-dessus les range. Les séparer est ce qui avait rendu `--echange` faux.
    let noms: [&str; 2] = if echange {
        [nom_b.as_str(), nom_a.as_str()]
    } else {
        [nom_a.as_str(), nom_b.as_str()]
    };
    let graines_camp = if echange {
        [graine_du_camp(graine, 1), graine_du_camp(graine, 0)]
    } else {
        [graine_du_camp(graine, 0), graine_du_camp(graine, 1)]
    };
    let mut cervelle_0 = cervelle_de(noms[0], &desc, &db);
    let mut cervelle_1 = cervelle_de(noms[1], &desc, &db);
    let r = une_partie(
        &db,
        &desc,
        graine,
        noms,
        graines_camp,
        &mut cervelle_0,
        &mut cervelle_1,
        &Cell::new(0),
        true,
    );
    println!(
        "{}",
        json!({
            "decisions": r.journal,
            "scores": [r.scores[0], r.scores[1]],
            "partie_complete": r.complete,
            "decisions_jouees": r.decisions,
        })
    );
}

/// Les cartes, pour les boîtes demandées. Le chemin est cherché depuis la racine
/// du dépôt puis un cran au-dessus, comme les autres programmes du moteur.
fn charger(boites_txt: &str) -> CardsDb {
    let boites = match BoiteSet::parse(boites_txt) {
        Ok(b) => b,
        Err(e) => mourir(&e),
    };
    let chemin = if std::path::Path::new("data/cards.json").exists() {
        "data/cards.json".to_string()
    } else {
        "../data/cards.json".to_string()
    };
    match CardsDb::load_boites(&chemin, boites) {
        Ok(db) => db,
        Err(e) => mourir(&e),
    }
}

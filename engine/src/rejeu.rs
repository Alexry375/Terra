//! **(le-juge-apprend) Le rejeu d'une partie en natif — le jumeau du pont.**
//!
//! Une partie EST une graine plus une liste de décisions (`web/webapp/
//! adversaire.md`). Le pont WebAssembly sait déjà rejouer une partie depuis sa
//! graine et rendre l'état vivant à la première décision non prise (`op: "pas"`,
//! `web/webapp/wasm/src/lib.rs`). Ce module fait EXACTEMENT la même chose en
//! natif, parce que le contrôle 01 compare les deux descriptions d'une même
//! situation : le binaire `decrire` doit atteindre la situation que le
//! JavaScript a atteinte, décision pour décision.
//!
//! **Le comportement est copié terme pour terme sur le `Harnais` du pont** —
//! consommation des réponses, entrées de vente, repli sur `RandomPolicy` une
//! fois la décision en attente trouvée, et surtout la règle d'écrasement de
//! `observe` (le moteur observe AUSSI les points de décision qu'il finit par ne
//! pas poser ; compter les observations désynchroniserait le curseur).
//!
//! Vérifié le 15-08 avant d'écrire une ligne : le moteur natif et le pont
//! rejouent la MÊME partie (`simulate --dump-state --seed 101` et
//! `--games 20 --seed 3` donnent des sorties identiques au caractère près des
//! deux côtés).

use engine::cards::CardsDb;
use engine::choice::ChoiceContext;
use engine::effects::RevealFilter;
use engine::policy::{ActionOpt, ConstructionBonus, Policy};
use engine::state::GameState;
use rand::rngs::StdRng;
use serde_json::Value;

use crate::description::{Description, Tampons};
use crate::reseau::{phase_la_plus_probable, ReseauPhases};

/// **Le plafond d'avance du §4.1**, et il est impératif : « cette avance ne doit
/// jamais dépasser un nombre de pas fixé (prends soixante), sinon une option qui
/// déclenche une longue cascade ferait boucler le joueur. Au-delà, on évalue là
/// où on en est et on le compte. »
pub const PLAFOND_AVANCE: u32 = 60;

/// **La réponse par défaut, celle qu'on prête à l'AUTRE pendant l'avance du
/// §4.1 — et le seul repli de `Rejeu` quand une réponse manque.**
///
/// Le §4.1 laisse deux voies pour répondre à la place de l'adversaire : le réseau
/// lui-même, ou « un choix simple et fixe — la première option, ou une option
/// tirée d'un hasard semé ». C'est **la première option** qui est retenue, et
/// pour une raison vérifiable : le vrai critère du §4 est que le joueur Rust et
/// le joueur JavaScript choisissent la même option dans la même situation. Le
/// hasard du moteur natif n'est pas reproductible par le pont — il n'expose pas
/// son générateur — alors que « la première option » l'est trivialement des deux
/// côtés. Toute autre voie rendrait le banc `juge-meme-option.mjs` rouge sans
/// qu'aucune faute n'ait été commise.
///
/// **Indice 0 partout, sans exception**, y compris là où le trait `Policy` offre
/// un défaut plus malin : `pick_joker_tag` choisit sinon « le badge le plus
/// possédé », que le JavaScript ne reproduit pas. Une seule divergence de ce
/// genre suffit à faire diverger tout le rejeu qui suit.
pub struct Premiere;

impl Policy for Premiere {
    fn corp_mulligan(&mut self, _rng: &mut StdRng, _player: usize, _corps: &[u16]) -> bool {
        false // indice 0
    }
    fn project_mulligan(&mut self, _rng: &mut StdRng, _player: usize, _hand: &[u16]) -> Vec<usize> {
        Vec::new() // liste libre : la liste vide
    }
    fn pick_corporation(&mut self, _rng: &mut StdRng, _player: usize, _corps: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, _rng: &mut StdRng, _player: usize, allowed: &[u8]) -> u8 {
        allowed[0]
    }
    fn choose_build(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        affordable.first().copied()
    }
    fn construction_bonus(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCardBefore
    }
    fn construction_bonus_avant(&mut self, _rng: &mut StdRng, _player: usize) -> bool {
        true // indice 0
    }
    fn construction_bonus_apres(&mut self, _rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard // indice 0
    }
    fn action_choice(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            None
        } else {
            Some(0)
        }
    }
    fn action_amount(&mut self, _rng: &mut StdRng, _player: usize, _max: i64) -> i64 {
        0
    }
    fn vendre_librement(&mut self, _rng: &mut StdRng, _joueur: usize, _main: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn choose_option(&mut self, _rng: &mut StdRng, _player: usize, _n: usize) -> usize {
        0
    }
    fn choose_option_ctx(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _ctx: &ChoiceContext,
    ) -> usize {
        0
    }
    fn choose_res_target(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _candidates: &[u16],
    ) -> usize {
        0
    }
    fn choose_res_source(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _candidates: &[u16],
    ) -> usize {
        0
    }
    fn pick_joker_tag(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _card: u16,
        _tag_counts: &[u32],
    ) -> usize {
        0
    }
    fn research_keep(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        (0..keep.min(drawn.len())).collect()
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
        (0..keep.min(candidates.len())).collect()
    }
    fn discard_down(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        (0..n.min(hand.len())).collect()
    }
}

// ---------------------------------------------------------------------------
// (il-devine) LA DEVINETTE — §3
// ---------------------------------------------------------------------------

/// **(il-devine) Ce qu'il faut pour prêter à l'autre une intention apprise (§3).**
///
/// Pendant l'avance du §4.1, le joueur rencontre des décisions de l'adversaire et
/// y répond à sa place. **Une seule de ces décisions change de traitement : le
/// choix de la carte Phase.** Toutes les autres continuent de recevoir la première
/// option, par `Premiere`.
///
/// ─────────────────────────────────────────────────────────────────────────────
/// **LE POINT DE CONCEPTION QUI DÉCIDE DE TOUT : LE POINT DE VUE.**
///
/// La description passée au second réseau est celle du joueur **qui devine**
/// ([`Devinette::moi`]), et jamais celle de l'adversaire qu'on cherche à prédire.
/// Il serait tentant de décrire la situation du point de vue de l'autre, puisque
/// c'est lui qu'on prédit : **c'est interdit** (§1). Cette description-là contient
/// sa main, et un joueur qui lit la main d'en face triche. On devine à partir de
/// ce que l'on sait, pas à partir de ce que l'on ne devrait pas savoir.
///
/// Le prix de cette honnêteté est connu et assumé : le second réseau est entraîné
/// sur « ce que choisit celui du point de vue duquel je décris » et interrogé sur
/// « ce que choisira l'autre ». C'est le §7, étape 5, qui dit ce que cela vaut —
/// et un chiffre au-dessus de 60 % y est déclaré **suspect**, précisément parce
/// qu'il trahirait une fuite d'information.
pub struct Devinette<'a> {
    pub db: &'a CardsDb,
    pub desc: &'a Description,
    /// Le second réseau. Il n'est jamais corrigé ici : la devinette est un usage,
    /// l'apprentissage a son seul point de rendez-vous dans `Joueur::pick_phase`
    /// (§2.1).
    pub reseau: &'a mut ReseauPhases,
    pub tampons: &'a mut Tampons,
    /// Tampon de description, réutilisé — aucune allocation dans la boucle chaude.
    pub x: &'a mut Vec<f64>,
    /// **Le point de vue** : le siège du joueur qui est en train de décider, celui
    /// pour qui on avance vers le repère du §4.1.
    pub moi: usize,
}

impl Devinette<'_> {
    /// **La phase qu'on prête à l'adversaire (§3, les quatre pas).**
    ///
    /// 1. décrire l'état **du point de vue du joueur qui décide** ;
    /// 2. le passer dans le second réseau, obtenir cinq probabilités ;
    /// 3. mettre à zéro les phases non autorisées, renormaliser sur le reste ;
    /// 4. rendre la plus probable, la plus petite en cas d'égalité stricte.
    ///
    /// Les pas 3 et 4 sont dans [`phase_la_plus_probable`], partagée par les trois
    /// endroits qui lisent les cinq sorties : ici, le binaire de mesure, et — au
    /// mot près — le miroir JavaScript.
    ///
    /// **`oublier` avant chaque évaluation, et ce n'est pas une précaution
    /// décorative.** Le réseau met normalement ses sommes cachées à jour par
    /// DIFFÉRENCES (l'optimisation du §1.1) : le résultat dépend alors, au dernier
    /// bit, de la situation évaluée juste avant. Le JavaScript, lui, refait chaque
    /// évaluation en entier. Pour le premier réseau, cet écart d'un dernier bit est
    /// absorbé par la marge de départage ; ici il ne le serait pas, parce qu'on
    /// prend un maximum sur cinq valeurs et qu'un maximum n'a pas de marge. On
    /// force donc le calcul complet des deux côtés : même ordre d'addition, et
    /// des sommes de sortie identiques au bit près — la tangente hyperbolique
    /// concorde exactement entre Rust et Node (mesuré : 0 écart sur 2000).
    ///
    /// **Cela ne suffit pourtant pas**, et c'est mesuré aussi : `Math.exp` et
    /// `f64::exp` diffèrent d'un dernier bit sur environ une valeur sur dix (196
    /// sur 2000). Le départage se fait donc à la marge, comme pour le premier
    /// réseau — voir `reseau::MARGE_PHASE`.
    fn phase(&mut self, game: &GameState, autorisees: &[u8]) -> u8 {
        self.desc
            .decrire(game, self.db, self.moi, self.x, self.tampons);
        self.reseau.oublier();
        let p = self.reseau.evaluer(self.x);
        phase_la_plus_probable(&p, autorisees)
    }
}

/// Politique de rejeu : elle répond les décisions déjà prises, puis s'arrête à
/// la première qui manque, en gardant l'état vivant et le siège concerné.
pub struct Rejeu<'a> {
    reponses: Vec<Value>,
    curseur: usize,
    /// Siège de la décision en attente (`None` tant qu'on rejoue).
    pub attente: Option<usize>,
    /// L'état vivant reçu juste avant la décision en attente.
    pub vue: Option<GameState>,
    pub erreur: Option<String>,
    defaut: Premiere,
    /// **LE REPÈRE DU §4.1.** Quand ce champ porte un siège, le rejeu ne s'arrête
    /// plus à la première décision venue : tant que la décision est celle d'un
    /// AUTRE joueur, on répond à sa place (`Premiere`) et on continue. On ne
    /// s'arrête qu'à la prochaine décision **de ce siège-là**, à la fin de la
    /// partie, ou au plafond. Toutes les options sont ainsi jugées au même
    /// instant : « la prochaine fois que j'aurai la main ».
    pub avance: Option<usize>,
    /// Pas d'avance déjà consommés (bornés par [`PLAFOND_AVANCE`]).
    pub pas_avance: u32,
    /// Le plafond a-t-il arrêté cette avance ? Compté et rapporté (§4.1).
    pub plafond_atteint: bool,
    /// **(il-devine, §8) Combien de `pick_phase` adverses cette avance a
    /// rencontrés** — compté que la devinette soit allumée ou non, parce que
    /// c'est le chiffre qui dit si elle peut servir à quelque chose. « Croire
    /// qu'un `pick_phase` adverse est rencontré à chaque avance » est un des
    /// pièges annoncés : il faut le mesurer avant de conclure.
    pub phases_de_l_autre: u32,
    /// **(il-devine, §3/§4) La devinette, quand elle est allumée.** Absente, le
    /// rejeu se comporte exactement comme avant : première option pour l'autre,
    /// carte Phase comprise. C'est l'état par défaut, et le contrôle 10 le
    /// vérifie sur trois parties entières.
    pub devinette: Option<Devinette<'a>>,
    /// **(2.15) LE NUMÉRO DE L'OCCASION DE VENTE EN COURS.**
    ///
    /// Une entrée de vente ne dit pas d'elle-même À QUELLE occasion elle
    /// appartient : le moteur en ouvre une avant chaque point de décision, pour
    /// chacun des deux sièges, et il en ouvre aussi devant des points de décision
    /// qu'il finit par ne pas poser. Sans numéro, une entrée décidée à l'occasion
    /// 277 se faisait consommer à l'occasion 275 — reproduit sur la graine
    /// 1 000 023 — et le rejeu appliquait la vente au mauvais endroit.
    ///
    /// Le numéro est celui de la PARTIE, pas de la manche : un rejeu qui repart du
    /// milieu reçoit donc la valeur qu'il avait à son point de reprise.
    pub occasions: u64,
    /// **Vrai quand le dernier `prendre` a rendu `None` PARCE QU'ON RÉPOND À LA
    /// PLACE DE L'AUTRE pendant l'avance du §4.1** — et non parce que le rejeu
    /// s'arrête ici, ni parce qu'une décision attend déjà.
    ///
    /// La distinction est indispensable : `prendre` rend `None` dans trois cas
    /// bien différents, et la devinette ne doit s'appliquer qu'à celui-là. Aux
    /// deux autres, le repli reste `Premiere`, exactement comme avant.
    pour_l_autre: bool,
}

impl<'a> Rejeu<'a> {
    pub fn new(reponses: Vec<Value>) -> Rejeu<'a> {
        Rejeu {
            reponses,
            curseur: 0,
            attente: None,
            vue: None,
            erreur: None,
            defaut: Premiere,
            avance: None,
            pas_avance: 0,
            plafond_atteint: false,
            phases_de_l_autre: 0,
            devinette: None,
            occasions: 0,
            pour_l_autre: false,
        }
    }

    /// Le même rejeu, mais qui avance jusqu'au prochain point de décision de
    /// `siege` (§4.1).
    pub fn jusqu_a(reponses: Vec<Value>, siege: usize) -> Rejeu<'a> {
        let mut r = Rejeu::new(reponses);
        r.avance = Some(siege);
        r
    }

    /// Le même rejeu, mais qui sait combien d'occasions de vente la partie a déjà
    /// ouvertes avant son point de reprise (§2.15).
    pub fn depuis_occasion(mut self, n: u64) -> Rejeu<'a> {
        self.occasions = n;
        self
    }

    /// Rend la réponse enregistrée pour cette décision, ou `None` s'il faut
    /// répondre par défaut — soit parce qu'on avance vers le repère du §4.1,
    /// soit parce qu'on s'arrête ici (`attente` est alors posé).
    fn prendre(&mut self, joueur: usize) -> Option<Value> {
        self.pour_l_autre = false;
        if self.attente.is_some() {
            return None;
        }
        if self.curseur < self.reponses.len() {
            if self.reponses[self.curseur].get("vendre").is_some() {
                self.faute(
                    "une vente est proposée là où le moteur attend une réponse".to_string(),
                );
                return None;
            }
            let r = self.reponses[self.curseur].clone();
            self.curseur += 1;
            return Some(r);
        }
        // Plus de réponse enregistrée. Sans avance, on s'arrête ici — c'est le
        // comportement du pont. Avec l'avance du §4.1, on ne s'arrête que pour
        // le siège qui choisit ; pour l'autre, on répond à sa place et le rejeu
        // continue, jusqu'au plafond.
        if let Some(moi) = self.avance {
            if joueur != moi {
                if self.pas_avance < PLAFOND_AVANCE {
                    self.pas_avance += 1;
                    // Repli sur `Premiere`, sans poser `attente` — sauf pour la
                    // carte Phase quand la devinette est allumée (§3).
                    self.pour_l_autre = true;
                    return None;
                }
                self.plafond_atteint = true;
            }
        }
        self.attente = Some(joueur);
        None
    }

    fn faute(&mut self, quoi: String) {
        if self.erreur.is_none() {
            self.erreur = Some(format!("décision n°{} : {}", self.curseur, quoi));
        }
    }

    fn indice(&mut self, r: &Value, n: usize) -> Option<usize> {
        match r.as_u64() {
            Some(i) if (i as usize) < n => Some(i as usize),
            _ => {
                self.faute(format!("indice {r} hors de 0..{n}"));
                None
            }
        }
    }

    fn liste_libre(&mut self, r: &Value, n: usize) -> Option<Vec<usize>> {
        let a = r.as_array()?;
        let mut v: Vec<usize> = Vec::with_capacity(a.len());
        for x in a {
            match x.as_u64() {
                Some(i) if (i as usize) < n && !v.contains(&(i as usize)) => v.push(i as usize),
                _ => {
                    self.faute(format!("indice {x} invalide ou en double (0..{n})"));
                    return None;
                }
            }
        }
        Some(v)
    }

    fn liste(&mut self, r: &Value, n: usize, attendu: usize) -> Option<Vec<usize>> {
        let v = self.liste_libre(r, n)?;
        if v.len() != attendu {
            self.faute(format!("{} indices donnés, {attendu} attendus", v.len()));
            return None;
        }
        Some(v)
    }
}

impl Policy for Rejeu<'_> {
    /// Écraser plutôt que compter : le moteur observe aussi les points de
    /// décision qu'il finit par ne pas poser (même raison que le pont).
    fn observe(&mut self, game: &GameState, _player: usize) {
        if self.attente.is_none() && self.curseur == self.reponses.len() {
            self.vue = Some(game.clone());
        }
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        match self.prendre(player) {
            Some(r) => self.indice(&r, 2).map(|i| i == 1).unwrap_or(false),
            None => self.defaut.corp_mulligan(rng, player, corps),
        }
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        match self.prendre(player) {
            Some(r) => match self.liste_libre(&r, hand.len()) {
                Some(v) => v,
                None => self.defaut.project_mulligan(rng, player, hand),
            },
            None => self.defaut.project_mulligan(rng, player, hand),
        }
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        match self.prendre(player) {
            Some(r) => self.indice(&r, corps.len()).unwrap_or(0),
            None => self.defaut.pick_corporation(rng, player, corps),
        }
    }

    /// **(il-devine) LE SEUL POINT DU REJEU QUI CHANGE DE TRAITEMENT (§3).**
    ///
    /// Quand on répond à la place de l'autre pendant l'avance du §4.1 et que la
    /// devinette est allumée, on ne rend plus `allowed[0]` : on rend la phase que
    /// le second réseau juge la plus probable dans cette situation-là. Partout
    /// ailleurs — et pour toutes les autres décisions de l'adversaire — le
    /// comportement est celui d'avant, à la ligne près.
    ///
    /// L'état employé est `self.vue`, celui que `observe` vient d'écrire : le
    /// moteur appelle `avant_decision` juste avant chaque `pick_phase`
    /// (`flow.rs`, la planification), donc `vue` porte bien l'état vivant de
    /// cette décision-ci. Sans état sous la main, on retombe sur `Premiere` : la
    /// devinette n'a alors rien à décrire, et se taire vaut mieux que deviner à
    /// partir de rien.
    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        match self.prendre(player) {
            Some(r) => match self.indice(&r, allowed.len()) {
                Some(i) => allowed[i],
                None => allowed[0],
            },
            None => {
                if self.pour_l_autre && !allowed.is_empty() {
                    self.phases_de_l_autre += 1;
                    if let (Some(d), Some(game)) = (self.devinette.as_mut(), self.vue.as_ref()) {
                        return d.phase(game, allowed);
                    }
                }
                self.defaut.pick_phase(rng, player, allowed)
            }
        }
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        match self.prendre(player) {
            Some(r) => {
                let n = affordable.len();
                match self.indice(&r, n + 1) {
                    Some(i) if i < n => Some(affordable[i]),
                    _ => None,
                }
            }
            None => self.defaut.choose_build(rng, player, affordable),
        }
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        match self.prendre(player) {
            Some(r) => match self.indice(&r, 3) {
                Some(0) => ConstructionBonus::DrawCardBefore,
                Some(1) => ConstructionBonus::DrawCard,
                Some(_) => ConstructionBonus::SecondBuild,
                None => ConstructionBonus::DrawCard,
            },
            None => self.defaut.construction_bonus(rng, player),
        }
    }

    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        match self.prendre(player) {
            Some(r) => self.indice(&r, 2).map(|i| i == 0).unwrap_or(false),
            None => self.defaut.construction_bonus_avant(rng, player),
        }
    }

    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        match self.prendre(player) {
            Some(r) => match self.indice(&r, 2) {
                Some(1) => ConstructionBonus::SecondBuild,
                _ => ConstructionBonus::DrawCard,
            },
            None => self.defaut.construction_bonus_apres(rng, player),
        }
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None;
        }
        match self.prendre(player) {
            Some(r) => {
                let n = options.len();
                match self.indice(&r, n + 1) {
                    Some(i) if i < n => Some(i),
                    _ => None,
                }
            }
            None => self.defaut.action_choice(rng, player, options),
        }
    }

    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        match self.prendre(player) {
            Some(r) => match r.as_i64() {
                Some(x) if x >= 0 && x <= max => x,
                _ => {
                    self.faute(format!("montant {r} hors de 0..={max}"));
                    0
                }
            },
            None => self.defaut.action_amount(rng, player, max),
        }
    }

    /// La vente est une ENTRÉE, pas une réponse : elle se consomme au point
    /// d'occasion, et jamais comme réponse à une question (même règle que le
    /// pont, `Harnais::vendre_librement`).
    fn vendre_librement(&mut self, _rng: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        // Le numéro de CETTE occasion-ci, compté avant toute chose : le moteur
        // interroge chaque siège dont la main n'est pas vide, et le rejeu doit
        // compter exactement comme le joueur a compté.
        let numero = self.occasions;
        self.occasions += 1;
        if self.attente.is_some() || self.curseur >= self.reponses.len() {
            return Vec::new();
        }
        let Some(vente) = self.reponses[self.curseur].get("vendre").cloned() else {
            return Vec::new();
        };
        if vente.get("joueur").and_then(Value::as_u64) != Some(joueur as u64) {
            return Vec::new();
        }
        // **Une entrée numérotée n'est JAMAIS consommée avant son occasion.**
        //
        // Le numéro dit à quelle occasion la vente a été décidée. Le consommer
        // plus tôt appliquait la vente au mauvais endroit — reproduit sur la
        // graine 1 000 023, vente décidée à l'occasion 277 et appliquée à la 275.
        // On ne l'exige pas à l'unité près pour autant : un rejeu d'ESSAI part
        // d'un paquet rebattu et peut sauter une occasion (un siège dont la main
        // s'est vidée n'est pas interrogé). Exiger l'égalité stricte y refusait
        // l'option au lieu de la juger. La règle est donc « jamais avant son
        // heure, au plus tard à la première occasion suivante du même siège ».
        //
        // Une entrée SANS numéro reste acceptée telle quelle : les parties
        // enregistrées d'avant ce lot, et le harnais du pont, n'en portent pas.
        if let Some(n) = vente.get("occasion").and_then(Value::as_u64) {
            if numero < n {
                return Vec::new();
            }
        }
        let Some(cartes) = vente.get("cartes").and_then(Value::as_array) else {
            self.faute("« cartes » attendu : une liste d'indices de main".to_string());
            return Vec::new();
        };
        let mut idx: Vec<usize> = Vec::with_capacity(cartes.len());
        for x in cartes {
            match x.as_u64() {
                Some(i) if (i as usize) < main.len() && !idx.contains(&(i as usize)) => {
                    idx.push(i as usize)
                }
                _ => {
                    self.faute(format!("indice de vente {x} invalide ou en double"));
                    return Vec::new();
                }
            }
        }
        self.curseur += 1;
        idx
    }

    fn choose_option(&mut self, rng: &mut StdRng, player: usize, n: usize) -> usize {
        // Voie anonyme : aucun site du moteur ne l'emprunte plus (le pont la
        // déclare en faute). On la traite pareil, sans consommer de réponse.
        if self.erreur.is_none() {
            self.erreur = Some(format!("voie anonyme `choose_option` ({n} options)"));
        }
        self.defaut.choose_option(rng, player, n)
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        if n == 0 {
            return 0;
        }
        match self.prendre(player) {
            Some(r) => self.indice(&r, n).unwrap_or(0),
            None => self.defaut.choose_option_ctx(rng, player, ctx),
        }
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        match self.prendre(player) {
            Some(r) => self.indice(&r, candidates.len()).unwrap_or(0),
            None => self.defaut.choose_res_target(rng, player, candidates),
        }
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        match self.prendre(player) {
            Some(r) => self.indice(&r, candidates.len()).unwrap_or(0),
            None => self.defaut.choose_res_source(rng, player, candidates),
        }
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        card: u16,
        tag_counts: &[u32],
    ) -> usize {
        let n = engine::cards::JOKER_TAG_CHOICES.len();
        match self.prendre(player) {
            Some(r) => self.indice(&r, n).unwrap_or(0),
            None => self.defaut.pick_joker_tag(rng, player, card, tag_counts),
        }
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        match self.prendre(player) {
            Some(r) => match self.liste(&r, drawn.len(), keep) {
                Some(v) => v,
                None => self.defaut.research_keep(rng, player, drawn, keep),
            },
            None => self.defaut.research_keep(rng, player, drawn, keep),
        }
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
        match self.prendre(player) {
            Some(r) => match self.liste(&r, candidates.len(), keep) {
                Some(v) => v,
                None => self
                    .defaut
                    .reveal_pick(rng, player, revealed, candidates, keep, filter),
            },
            None => self
                .defaut
                .reveal_pick(rng, player, revealed, candidates, keep, filter),
        }
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        match self.prendre(player) {
            Some(r) => match self.liste(&r, hand.len(), n) {
                Some(v) => v,
                None => self.defaut.discard_down(rng, player, hand, n),
            },
            None => self.defaut.discard_down(rng, player, hand, n),
        }
    }
}

/// Rejoue la partie `seed` avec `decisions` et rend l'état vivant à la première
/// décision non prise (ou l'état final si tout a été joué), avec le siège
/// concerné.
pub fn rejouer(
    db: &engine::cards::CardsDb,
    seed: u64,
    decisions: Vec<Value>,
) -> Result<(GameState, Option<usize>), String> {
    let mut pol = Rejeu::new(decisions);
    let mut game = engine::flow::setup_game(db, seed, &mut pol);
    while pol.attente.is_none()
        && !game.game_over
        && game.generation <= engine::sim::MAX_GENERATIONS
    {
        engine::flow::play_round(&mut game, db, &mut pol);
    }
    if let Some(e) = pol.erreur {
        return Err(e);
    }
    match pol.attente {
        Some(joueur) => Ok((pol.vue.unwrap_or(game), Some(joueur))),
        None => Ok((game, None)),
    }
}

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

use engine::choice::ChoiceContext;
use engine::effects::RevealFilter;
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::state::GameState;
use rand::rngs::StdRng;
use serde_json::Value;

/// Politique de rejeu : elle répond les décisions déjà prises, puis s'arrête à
/// la première qui manque, en gardant l'état vivant et le siège concerné.
pub struct Rejeu {
    reponses: Vec<Value>,
    curseur: usize,
    /// Siège de la décision en attente (`None` tant qu'on rejoue).
    pub attente: Option<usize>,
    /// L'état vivant reçu juste avant la décision en attente.
    pub vue: Option<GameState>,
    pub erreur: Option<String>,
    defaut: RandomPolicy,
}

impl Rejeu {
    pub fn new(reponses: Vec<Value>) -> Rejeu {
        Rejeu {
            reponses,
            curseur: 0,
            attente: None,
            vue: None,
            erreur: None,
            defaut: RandomPolicy,
        }
    }

    /// Rend la réponse enregistrée pour cette décision, ou `None` s'il faut
    /// s'arrêter ici.
    fn prendre(&mut self, joueur: usize) -> Option<Value> {
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

impl Policy for Rejeu {
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

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        match self.prendre(player) {
            Some(r) => match self.indice(&r, allowed.len()) {
                Some(i) => allowed[i],
                None => allowed[0],
            },
            None => self.defaut.pick_phase(rng, player, allowed),
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
        if self.attente.is_some() || self.curseur >= self.reponses.len() {
            return Vec::new();
        }
        let Some(vente) = self.reponses[self.curseur].get("vendre").cloned() else {
            return Vec::new();
        };
        if vente.get("joueur").and_then(Value::as_u64) != Some(joueur as u64) {
            return Vec::new();
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

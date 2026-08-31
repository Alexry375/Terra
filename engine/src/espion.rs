//! **(le-juge-apprend) Une politique qui REGARDE une partie sans y toucher.**
//!
//! Jumelle de `crate::observe::ObservingPolicy`, à une différence près : au
//! lieu d'un relevé figé de douze champs, elle appelle une fermeture avec
//! l'état VIVANT et le siège concerné. C'est ce qui permet de relever la
//! distribution réelle des quantités (prompt §3.5) sans écrire une seconde
//! boucle de jeu.
//!
//! La délégation est **intégrale**, pour la même raison qu'elle l'est dans
//! `observe.rs` : une méthode oubliée retomberait sur le corps par défaut du
//! trait au lieu de celui de la politique enveloppée, et changerait donc
//! silencieusement le jeu.
//!
//! Ce module n'écrit jamais dans `GameState` et ne consomme jamais le RNG de la
//! partie.

use crate::choice::ChoiceContext;
use crate::effects::RevealFilter;
use crate::policy::{ActionOpt, ConstructionBonus, Policy};
use crate::state::GameState;
use rand::rngs::StdRng;

/// Enveloppe `inner` et appelle `vu(game, joueur)` avant chaque décision.
pub struct Espion<P: Policy, F: FnMut(&GameState, usize)> {
    inner: P,
    vu: F,
}

impl<P: Policy, F: FnMut(&GameState, usize)> Espion<P, F> {
    pub fn new(inner: P, vu: F) -> Espion<P, F> {
        Espion { inner, vu }
    }

    pub fn into_inner(self) -> P {
        self.inner
    }

    /// La politique enveloppée, empruntée. Sans elle, une politique qui a besoin
    /// qu'on l'appelle ENTRE deux manches — `joueur::Joueur::debut_manche`, qui
    /// fixe le point de reprise des essais — ne peut pas être espionnée : on ne
    /// peut ni la sortir de l'enveloppe ni la retrouver. Aucune décision n'en
    /// dépend : l'enveloppe reste un observateur.
    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }
}

impl<P: Policy, F: FnMut(&GameState, usize)> Policy for Espion<P, F> {
    fn observe(&mut self, game: &GameState, player: usize) {
        (self.vu)(game, player);
        self.inner.observe(game, player);
    }

    /// (l-etalon-natif) Transmise telle quelle : une politique enveloppée doit
    /// entendre qui va décider, comme si rien ne l'enveloppait.
    fn observer_l_occasion(&mut self, game: &GameState, decideur: usize, question_posee: bool) {
        self.inner.observer_l_occasion(game, decideur, question_posee);
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool {
        self.inner.corp_mulligan(rng, player, corps)
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        self.inner.project_mulligan(rng, player, hand)
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        self.inner.pick_corporation(rng, player, corps)
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        self.inner.pick_phase(rng, player, allowed)
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        self.inner.choose_build(rng, player, affordable)
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        self.inner.construction_bonus(rng, player)
    }

    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        self.inner.construction_bonus_avant(rng, player)
    }

    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        self.inner.construction_bonus_apres(rng, player)
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        self.inner.action_choice(rng, player, options)
    }

    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        self.inner.action_amount(rng, player, max)
    }

    fn vendre_librement(&mut self, rng: &mut StdRng, joueur: usize, main: &[u16]) -> Vec<usize> {
        self.inner.vendre_librement(rng, joueur, main)
    }

    fn choose_option(&mut self, rng: &mut StdRng, player: usize, n: usize) -> usize {
        self.inner.choose_option(rng, player, n)
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        self.inner.choose_option_ctx(rng, player, ctx)
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        self.inner.choose_res_target(rng, player, candidates)
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        self.inner.choose_res_source(rng, player, candidates)
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        card: u16,
        tag_counts: &[u32],
    ) -> usize {
        self.inner.pick_joker_tag(rng, player, card, tag_counts)
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        self.inner.research_keep(rng, player, drawn, keep)
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
        self.inner
            .reveal_pick(rng, player, revealed, candidates, keep, filter)
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        self.inner.discard_down(rng, player, hand, n)
    }
}

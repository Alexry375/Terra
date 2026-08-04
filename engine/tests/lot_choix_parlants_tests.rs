//! Tests du chantier `choix-parlants` — **le moteur dit de quoi il parle**.
//!
//! Les quatre contrôles fournis mesurent la livraison de l'extérieur : une
//! regex sur `flow.rs`, trois empreintes, et l'écoute du pont WebAssembly. Ce
//! fichier la mesure de l'intérieur, en PARTIE RÉELLE (`sim::play_game`), et il
//! apporte les oracles que ces contrôles ne peuvent pas fournir :
//!
//! - §1 — les couples (phase, variante) annoncés à celui qui décide sont
//!   exactement ceux que le moteur applique ensuite. L'oracle est double, et
//!   entièrement disjoint du code mesuré : la liste annoncée est recalculée
//!   depuis l'état VIVANT reçu par `Policy::observe` (règle du livret : cinq
//!   phases × deux variantes, moins celles déjà en place, moins le filtre de
//!   phase imposée), et l'amélioration retenue est relue sur le joueur à la
//!   décision SUIVANTE.
//! - §2 — une politique qui n'implémente pas la voie enrichie décide exactement
//!   comme avant : le `n` qu'elle reçoit est celui d'avant, et deux parties
//!   menées l'une par l'ancienne voie, l'autre par la nouvelle, sont
//!   indiscernables.
//! - §3 — les enveloppes (`ObservingPolicy`) ne neutralisent pas la voie
//!   enrichie de la politique qu'elles enveloppent.
//! - §4 — aucune décision anonyme et aucune catégorie fourre-tout : les natures
//!   rencontrées en partie réelle sont nombreuses et toutes distinctes.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::choice::{describe_branch, describe_selector_grant, ChoiceContext};
use engine::effects::PHASE_UPGRADED;
use engine::policy::{Policy, RandomPolicy};
use engine::sim::play_game;
use engine::state::{GameState, PhaseUpgrade, NUM_PLAYERS};
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::BTreeSet;

const CARDS: &str = "../data/cards.json";

fn db_decouverte() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("cards.json doit se charger")
}

/// Les améliorations installées chez chaque joueur, relevées sur l'état vivant.
fn releve(game: &GameState) -> [[Option<PhaseUpgrade>; 5]; NUM_PLAYERS] {
    let mut out = [[None; 5]; NUM_PLAYERS];
    for (p, ligne) in out.iter_mut().enumerate() {
        for (i, case) in ligne.iter_mut().enumerate() {
            *case = game.players[p].phase_upgrade(i as u8 + 1);
        }
    }
    out
}

// ===========================================================================
// 1. L'AMÉLIORATION DE CARTE PHASE EST ANNONCÉE, ET L'ANNONCE EST VRAIE
// ===========================================================================

/// Ce que la politique attend de vérifier à la décision suivante.
struct Attente {
    player: usize,
    phase: u8,
    variant: PhaseUpgrade,
}

/// Politique qui JOUE normalement (elle délègue tout à `RandomPolicy`) mais
/// confronte chaque contexte d'amélioration reçu à l'état de la partie.
struct EspionAmelioration {
    inner: RandomPolicy,
    /// Améliorations en place, relevées au dernier `observe` — c'est-à-dire à
    /// l'instant EXACT qui précède la décision.
    avant: [[Option<PhaseUpgrade>; 5]; NUM_PLAYERS],
    attente: Option<Attente>,
    /// Contextes d'amélioration reçus.
    recus: usize,
    /// Annonces confrontées à l'état de la partie APRÈS application.
    verifies: usize,
}

impl EspionAmelioration {
    fn new() -> EspionAmelioration {
        EspionAmelioration {
            inner: RandomPolicy,
            avant: [[None; 5]; NUM_PLAYERS],
            attente: None,
            recus: 0,
            verifies: 0,
        }
    }
}

impl Policy for EspionAmelioration {
    fn observe(&mut self, game: &GameState, _player: usize) {
        // L'annonce de la décision précédente est-elle devenue vraie ?
        if let Some(a) = self.attente.take() {
            assert_eq!(
                game.players[a.player].phase_upgrade(a.phase),
                Some(a.variant),
                "le moteur avait annoncé la phase {} variante {:?} au joueur {}, \
                 elle n'est pas en place",
                a.phase,
                a.variant,
                a.player
            );
            self.verifies += 1;
        }
        self.avant = releve(game);
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        if let ChoiceContext::PhaseUpgrade {
            candidates,
            imposed_phase,
            ..
        } = ctx
        {
            self.recus += 1;
            // ORACLE INDÉPENDANT : la liste du livret, reconstruite depuis
            // l'état vivant, sans lire une ligne de `apply_phase_upgrade`.
            let mut attendues: Vec<(u8, PhaseUpgrade)> = Vec::new();
            for phase in 1u8..=5 {
                if imposed_phase.is_some_and(|t| t != phase) {
                    continue;
                }
                for v in [PhaseUpgrade::VariantA, PhaseUpgrade::VariantB] {
                    if self.avant[player][phase as usize - 1] != Some(v) {
                        attendues.push((phase, v));
                    }
                }
            }
            let annoncees: Vec<(u8, PhaseUpgrade)> =
                candidates.iter().map(|c| (c.phase, c.variant)).collect();
            assert_eq!(
                annoncees, attendues,
                "les couples (phase, variante) annoncés ne sont pas ceux que le \
                 livret laisse disponibles au joueur {player}"
            );
            // Le nom annoncé est celui de la carte Phase améliorée du moteur.
            for c in candidates.iter() {
                assert_eq!(
                    c.name,
                    PHASE_UPGRADED[c.phase as usize - 1][c.variant.index()].name,
                    "nom d'amélioration inventé pour la phase {} variante {:?}",
                    c.phase,
                    c.variant
                );
            }
            let i = self.inner.choose_option_ctx(rng, player, ctx);
            let choisie = candidates[i.min(candidates.len() - 1)];
            self.attente = Some(Attente {
                player,
                phase: choisie.phase,
                variant: choisie.variant,
            });
            return i;
        }
        self.inner.choose_option_ctx(rng, player, ctx)
    }

    // ------------------------------------------------------------ délégation
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.inner.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.inner.choose_build(r, p, a)
    }
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.inner.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

/// **Le contexte d'amélioration existe, il est complet, et il ne ment pas.**
#[test]
fn l_amelioration_de_carte_phase_est_annoncee_et_appliquee_telle_quelle() {
    let db = db_decouverte();
    let mut espion = EspionAmelioration::new();
    let mut en_suspens = 0usize;
    for seed in 0..40u64 {
        play_game(&db, seed, &mut espion);
        // Une partie peut se terminer sur une amélioration : il n'y a alors plus
        // de décision où relire l'annonce. On la compte, et on repart propre —
        // sans quoi elle serait confrontée à l'état NEUF de la partie suivante,
        // et le test échouerait à tort.
        if espion.attente.take().is_some() {
            en_suspens += 1;
        }
    }
    assert!(
        espion.recus > 20,
        "seulement {} demandes d'amélioration sur 40 parties : l'échantillon ne \
         prouve rien",
        espion.recus
    );
    // Toute annonce est confrontée, sauf celles restées en suspens en fin de
    // partie — et leur compte est connu, pas estimé.
    assert_eq!(
        espion.verifies + en_suspens,
        espion.recus,
        "{} annonces reçues, {} confrontées et {} en suspens : le compte ne tombe pas",
        espion.recus,
        espion.verifies,
        en_suspens
    );
    assert!(espion.verifies > 20, "trop peu d'annonces confrontées");
}

// ===========================================================================
// 2. RÉTROCOMPATIBILITÉ — l'ancienne voie décide exactement comme avant
// ===========================================================================

/// Politique de l'ANCIEN monde : elle ne connaît que `choose_option`, comme
/// toutes les politiques écrites avant ce chantier. Elle enregistre le `n`
/// qu'elle reçoit.
struct AncienneVoie {
    base: RandomPolicy,
    n_recus: Vec<usize>,
}

impl Policy for AncienneVoie {
    fn choose_option(&mut self, rng: &mut StdRng, _p: usize, n: usize) -> usize {
        self.n_recus.push(n);
        rng.gen_range(0..n)
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// **Le nombre d'options, recompté à la main depuis la charge utile du
/// contexte.** C'est un SECOND calcul, écrit ici et non appelé depuis le
/// moteur : si `ChoiceContext::option_count` se mettait à rendre autre chose
/// que le `n` que le site d'appel passait avant ce chantier, les deux
/// divergeraient et le test tomberait. Sans lui, comparer `option_count` à
/// lui-même ne prouverait rien.
fn nombre_d_options_recompte(ctx: &ChoiceContext) -> usize {
    match ctx {
        // Les cinq alternatives binaires du moteur : payer ou non, employer la
        // réduction ou non, défausser ou non.
        ChoiceContext::CorpTrBoost { .. }
        | ChoiceContext::MicrobeDiscount { .. }
        | ChoiceContext::PlantDiscount { .. }
        | ChoiceContext::HeatAsMc { .. }
        | ChoiceContext::DiscardToDraw { .. } => 2,
        ChoiceContext::PhaseUpgrade { candidates, .. } => candidates.len(),
        ChoiceContext::CardAlternative { branches, .. } => branches.len(),
        ChoiceContext::ActionAlternative { branches, .. } => branches.len(),
        // « Spend up to N » : une option par unité, de 1 à N.
        ChoiceContext::SpendAmount { max, .. } => {
            (1..=*max).count()
        }
        ChoiceContext::SelectorBonus { branches, .. } => branches.len(),
        ChoiceContext::ReplayProduction { candidates } => candidates.len(),
    }
}

/// Politique du NOUVEAU monde : elle ne connaît QUE la voie enrichie, et décide
/// avec le même tirage. Elle enregistre le nombre d'options du contexte.
struct NouvelleVoie {
    base: RandomPolicy,
    n_recus: Vec<usize>,
}

impl Policy for NouvelleVoie {
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        _p: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        assert_eq!(
            n,
            nombre_d_options_recompte(ctx),
            "{} : `option_count` ne rend pas le nombre d'options que porte le \
             contexte",
            ctx.kind()
        );
        self.n_recus.push(n);
        rng.gen_range(0..n)
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// **Les deux voies mènent à la même partie, coup pour coup.**
///
/// L'ancienne politique n'a pas été touchée d'une ligne : le moteur n'appelle
/// pourtant plus que la voie enrichie, et elle décide quand même — par le corps
/// par défaut du trait, qui lui repasse la main.
///
/// **Ce que ce test prouve, et ce qu'il ne prouve pas.** Il prouve que le
/// chemin par défaut fonctionne de bout en bout, qu'il consomme le générateur
/// aléatoire au même rythme que la voie enrichie, et que le moteur ne pose
/// jamais de question à moins de deux options. Il NE prouve pas à lui seul que
/// `option_count` vaut le `n` d'avant ce chantier : les deux voies emploient ce
/// même `option_count`, elles divergeraient ensemble. Cette propriété-là a deux
/// autres gardiens, tous deux extérieurs à `option_count` : le recomptage à la
/// main de `nombre_d_options_recompte` ci-dessus, et le contrôle 02, qui
/// compare trois empreintes de mille parties aux valeurs écrites dans le
/// contrat.
#[test]
fn une_politique_qui_ignore_le_contexte_decide_comme_avant() {
    let db = db_decouverte();
    let mut ancienne = AncienneVoie {
        base: RandomPolicy,
        n_recus: Vec::new(),
    };
    let mut nouvelle = NouvelleVoie {
        base: RandomPolicy,
        n_recus: Vec::new(),
    };
    for seed in 0..30u64 {
        let a = play_game(&db, seed, &mut ancienne);
        let n = play_game(&db, seed, &mut nouvelle);
        assert_eq!(a.state_hash, n.state_hash, "graine {seed} : empreinte changée");
        assert_eq!(a.scores, n.scores, "graine {seed} : score changé");
        assert_eq!(a.generations, n.generations, "graine {seed} : durée changée");
    }
    assert!(
        ancienne.n_recus.len() > 200,
        "seulement {} alternatives sur 30 parties : l'échantillon ne prouve rien",
        ancienne.n_recus.len()
    );
    assert_eq!(
        ancienne.n_recus, nouvelle.n_recus,
        "le nombre d'options vu par l'ancienne voie diffère de celui du contexte"
    );
    assert!(
        ancienne.n_recus.iter().all(|&n| n >= 2),
        "le moteur pose une question à moins de deux options"
    );
}

// ===========================================================================
// 3. LES ENVELOPPES NE NEUTRALISENT PAS LA VOIE ENRICHIE
// ===========================================================================

/// Politique qui compte les contextes reçus, pour être enveloppée.
struct CompteurCtx {
    base: RandomPolicy,
    ctx: usize,
}

impl Policy for CompteurCtx {
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        p: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        self.ctx += 1;
        self.base.choose_option_ctx(rng, p, ctx)
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// **`ObservingPolicy` transmet la voie enrichie.** Sans la délégation, elle
/// retomberait sur le corps par défaut du trait, qui appelle `choose_option` :
/// le compteur resterait à zéro et la politique enveloppée ne verrait jamais un
/// contexte — silencieusement, sans qu'aucune empreinte ne bouge.
#[test]
fn l_enveloppe_observatrice_ne_mange_pas_le_contexte() {
    use engine::observe::ObservingPolicy;
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(
        &db,
        CompteurCtx {
            base: RandomPolicy,
            ctx: 0,
        },
    )
    .keeping(false);
    for seed in 0..10u64 {
        play_game(&db, seed, &mut obs);
    }
    let compteur = obs.into_inner();
    assert!(
        compteur.ctx > 50,
        "la politique enveloppée n'a reçu que {} contextes : la délégation manque",
        compteur.ctx
    );
}

// ===========================================================================
// 4. AUCUNE DÉCISION ANONYME, AUCUNE CATÉGORIE FOURRE-TOUT
// ===========================================================================

/// Politique qui relève la NATURE de chaque choix, et vérifie que chaque option
/// annoncée est décrite par autre chose qu'un numéro.
struct Recenseur {
    base: RandomPolicy,
    natures: BTreeSet<&'static str>,
    total: usize,
}

impl Policy for Recenseur {
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        p: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        self.total += 1;
        let nature = ctx.kind();
        assert!(!nature.is_empty(), "nature de choix vide");
        self.natures.insert(nature);
        assert!(
            ctx.option_count() >= 2,
            "{nature} : question posée à moins de deux options"
        );
        // Ce qui doit pouvoir s'afficher au lieu d'un bouton gris.
        match ctx {
            ChoiceContext::PhaseUpgrade { candidates, .. } => {
                for c in candidates.iter() {
                    assert!((1..=5).contains(&c.phase), "phase hors 1..5");
                    assert!(!c.name.is_empty(), "carte Phase améliorée sans nom");
                }
            }
            ChoiceContext::CardAlternative { branches, .. }
            | ChoiceContext::ActionAlternative { branches, .. } => {
                for b in branches.iter() {
                    let d = describe_branch(b.effects);
                    assert!(
                        d.len() > 3 && !d.contains("branche"),
                        "branche décrite par « {d} »"
                    );
                }
            }
            ChoiceContext::SelectorBonus { branches, phase, .. } => {
                assert!((1..=5).contains(phase), "bonus de sélectionneur hors phase");
                for g in branches.iter() {
                    assert!(
                        describe_selector_grant(g).len() > 3,
                        "bonus de sélectionneur sans description"
                    );
                }
            }
            _ => {}
        }
        self.base.choose_option_ctx(rng, p, ctx)
    }
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.base.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.base.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.base.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.base.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.base.choose_build(r, p, a)
    }
    fn construction_bonus(
        &mut self,
        r: &mut StdRng,
        p: usize,
    ) -> engine::policy::ConstructionBonus {
        self.base.construction_bonus(r, p)
    }
    fn action_choice(
        &mut self,
        r: &mut StdRng,
        p: usize,
        o: &[engine::policy::ActionOpt],
    ) -> Option<usize> {
        self.base.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.base.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// **Les natures rencontrées en partie réelle sont multiples.** Une catégorie
/// unique du genre « Alternative » ferait tomber ce test : c'est exactement le
/// raccourci que le contrat interdit.
#[test]
fn les_natures_de_choix_sont_multiples_et_nommees() {
    let db = db_decouverte();
    let mut rec = Recenseur {
        base: RandomPolicy,
        natures: BTreeSet::new(),
        total: 0,
    };
    for seed in 0..60u64 {
        play_game(&db, seed, &mut rec);
    }
    assert!(rec.total > 400, "échantillon trop maigre : {} choix", rec.total);
    // Dix sites d'appel, dix natures — et les soixante parties (déterministes)
    // les atteignent toutes. Le seuil est donc l'exhaustivité, pas un minimum
    // prudent : si un site retombait dans une catégorie déjà employée, ce test
    // le verrait.
    //
    // (regles-de-la-vente) Onze → dix. La nature disparue est
    // `paiement_chaleur` (Helion, « You MAY use heat as MC »). Ce « may »
    // n'était une ALTERNATIVE que tant que le moteur offrait, en face, de payer
    // en défaussant des cartes : renoncer à la chaleur voulait dire « je paierai
    // en vendant ». La vente d'office supprimée, renoncer reviendrait à renoncer
    // à une carte qu'on vient de choisir de poser — une seule branche jouable,
    // et la convention du lot 3 interdit d'interroger la politique sur une
    // branche unique. La variante `ChoiceContext::HeatAsMc` reste dans le
    // vocabulaire de `choice.rs` : c'est le FLUX qui ne l'emprunte plus, pas le
    // langage qui l'a perdue.
    assert_eq!(
        rec.natures.len(),
        10,
        "les dix sites de décision doivent donner dix natures distinctes, \
         rencontrées : {:?}",
        rec.natures
    );
    assert!(
        !rec.natures.contains("paiement_chaleur"),
        "le choix « employer la chaleur ou payer en vendant » ne doit plus être \
         posé : il n'a plus qu'une branche jouable ({:?})",
        rec.natures
    );
    // Les deux natures que le contrat cite nommément.
    assert!(
        rec.natures.contains("amelioration_carte_phase"),
        "l'amélioration de carte Phase n'a pas été annoncée : {:?}",
        rec.natures
    );
    assert!(
        rec.natures.contains("montant_depense"),
        "le choix d'un montant n'a pas été annoncé : {:?}",
        rec.natures
    );
}

// ===========================================================================
// 5. LA SONDE RÉPOND TOUJOURS PAR SON SCRIPT, ET L'ORDRE DES CANDIDATES TIENT
// ===========================================================================

/// **La pile `--probe-choice` arrive intacte au bout de la voie enrichie, et
/// l'indice imposé désigne la candidate que l'on croit.**
///
/// *Hohmann Transfer Shipping* accorde « Améliorez une carte Phase ». Le choix
/// est désormais un `ChoiceContext::PhaseUpgrade` de dix candidates, que le
/// moteur construit dans l'ordre 1A, 1B, 2A, 2B, … 5B. Le test impose l'indice
/// et relit l'amélioration RÉELLEMENT installée : il tient donc à la fois la
/// rétrocompatibilité de la sonde (son script décide encore) et l'ordre annoncé
/// dans le contexte, dont dépend tout ce que l'écran affichera.
///
/// **Ce qu'il ne prouve pas, et pourquoi je le dis** : il ne prouve pas que
/// `ProbePolicy::choose_option_ctx` existe. `ProbePolicy` enveloppe
/// `RandomPolicy`, qui ne surcharge pas la voie enrichie ; sans la délégation,
/// le corps par défaut du trait rappellerait `ProbePolicy::choose_option` et
/// rendrait la même chose. Cette délégation est une garde STRUCTURELLE, exigée
/// par le contrat : elle deviendra observable le jour où la sonde enveloppera
/// une politique qui lit les contextes.
#[test]
fn la_sonde_impose_son_choix_par_la_voie_enrichie() {
    use engine::probe::{run_probe_seq_scripted, ProbeOptions, ProbeScript};
    let db = db_decouverte();
    let impose = |i: usize| {
        run_probe_seq_scripted(
            &db,
            &["Hohmann Transfer Shipping"],
            ProbeOptions::default(),
            &ProbeScript {
                choices: vec![i],
                targets: Vec::new(),
                joker_tag: None,
            },
        )
        .upgrades
    };
    assert_eq!(impose(0), vec!["1A".to_string()], "indice 0 = première candidate");
    assert_eq!(impose(3), vec!["2B".to_string()], "indice 3 = quatrième candidate");
    assert_eq!(impose(9), vec!["5B".to_string()], "indice 9 = dernière candidate");
}

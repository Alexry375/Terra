//! Tests du chantier `moteur-observe` — **la vue de la partie, ouverte à celui
//! qui décide**.
//!
//! Les quatre checks fournis mesurent la livraison de l'EXTÉRIEUR (empreintes,
//! sortie du binaire, regex sur `flow.rs`). Ce fichier la mesure de l'INTÉRIEUR,
//! et surtout il apporte les **oracles indépendants** que la sortie du binaire
//! ne peut pas fournir :
//!
//! - « une observation avant CHAQUE décision » est vérifié en comptant les deux
//!   côtés (§2), pas en faisant confiance à une regex de proximité ;
//! - « l'état RÉEL au moment de l'appel » est vérifié en recoupant l'observation
//!   avec les ARGUMENTS que le moteur passe lui-même à la décision qui suit
//!   (§3) : `sell_card` reçoit la main que `flow.rs` a clonée à cet instant,
//!   `discard_down` reçoit la main à cet instant. Si
//!   l'observation était un instantané périmé, ces valeurs divergeraient. Cet
//!   oracle est entièrement disjoint du code observé ;
//! - « aucune décision changée » est vérifié en rejouant la MÊME partie avec et
//!   sans enveloppe et en comparant l'empreinte (§1).

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score, setup_game};
use engine::observe::{state_view, ObservingPolicy};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::sim::{play_game, run_simulation};
use engine::state::{GameState, NUM_PLAYERS};
use rand::rngs::StdRng;

const CARDS: &str = "../data/cards.json";

fn db_base() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

fn db_decouverte() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("cards.json doit se charger")
}

// ===========================================================================
// 1. NEUTRALITÉ — observer ne change rien
// ===========================================================================

/// La même partie, jouée avec et sans l'enveloppe observatrice, donne le MÊME
/// état final. C'est l'invariant central du chantier, mesuré ici en mémoire (les
/// checks le mesurent sur les empreintes de 1000 parties).
#[test]
fn observer_ne_change_pas_la_partie() {
    let db = db_decouverte();
    for seed in [1u64, 4242, 777, 20240730] {
        let nu = play_game(&db, seed, &mut RandomPolicy);

        let mut obs = ObservingPolicy::new(&db, RandomPolicy);
        let vu = play_game(&db, seed, &mut obs);

        assert_eq!(nu.state_hash, vu.state_hash, "graine {seed} : empreinte changée");
        assert_eq!(nu.scores, vu.scores, "graine {seed} : score changé");
        assert_eq!(nu.generations, vu.generations, "graine {seed} : durée changée");
        assert_eq!(nu.blue_actions, vu.blue_actions, "graine {seed} : actions changées");
        assert!(obs.decisions() > 100, "graine {seed} : partie sans décisions ?");
    }
}

/// Idem sur une simulation entière (l'agrégat de 50 parties), boîte de base.
#[test]
fn observer_ne_change_pas_une_simulation() {
    let db = db_base();
    let nu = run_simulation(&db, 50, 2024, &mut RandomPolicy);
    let mut obs = ObservingPolicy::new(&db, RandomPolicy).keeping(false);
    let vu = run_simulation(&db, 50, 2024, &mut obs);
    assert_eq!(nu.state_hash, vu.state_hash);
    assert_eq!(nu.completed, vu.completed);
    assert_eq!(nu.discard_payments, vu.discard_payments);
    assert!(obs.decisions() > 1000);
}

/// La méthode d'observation a un corps par défaut VIDE : une politique qui ne
/// l'implémente pas ne consomme pas le RNG et ne bouge pas d'un iota. On le
/// prouve en appelant `observe` sur `RandomPolicy` entre deux décisions et en
/// vérifiant que la suite tirée est identique.
#[test]
fn corps_par_defaut_vide_ne_consomme_pas_le_rng() {
    use rand::SeedableRng;
    let db = db_base();
    let game = setup_game(&db, 99, &mut RandomPolicy);

    let mut a = StdRng::seed_from_u64(7);
    let mut b = StdRng::seed_from_u64(7);
    let mut pol = RandomPolicy;

    let mut sans = Vec::new();
    let mut avec = Vec::new();
    for _ in 0..40 {
        sans.push(pol.pick_phase(&mut a, 0, &[1, 2, 3, 4, 5]));
        // Une observation s'intercale ici, à chaque tour.
        pol.observe(&game, 0);
        avec.push(pol.pick_phase(&mut b, 0, &[1, 2, 3, 4, 5]));
    }
    assert_eq!(sans, avec, "le corps par défaut a consommé le RNG");
}

// ===========================================================================
// 2. COUVERTURE — une observation avant chaque décision, mesurée des deux côtés
// ===========================================================================

/// Politique qui compte, d'un côté les observations reçues, de l'autre les
/// décisions demandées — les QUINZE méthodes du trait, sans exception. Elle
/// délègue tout à `RandomPolicy` : la partie se déroule normalement.
struct CountingPolicy {
    inner: RandomPolicy,
    observations: u64,
    decisions: u64,
    /// Nombre de décisions vues alors qu'AUCUNE observation ne les précédait
    /// (compteur d'observations inchangé depuis la décision précédente).
    non_observees: u64,
    vues_a_la_derniere_decision: u64,
    /// Décisions dont le joueur ne correspondait pas à celui de l'observation
    /// qui la précède.
    mauvais_joueur: u64,
    dernier_joueur_observe: usize,
}

impl CountingPolicy {
    fn new() -> CountingPolicy {
        CountingPolicy {
            inner: RandomPolicy,
            observations: 0,
            decisions: 0,
            non_observees: 0,
            vues_a_la_derniere_decision: 0,
            mauvais_joueur: 0,
            dernier_joueur_observe: usize::MAX,
        }
    }

    /// À appeler au début de CHAQUE méthode de décision.
    fn decision(&mut self, player: usize) {
        self.decisions += 1;
        if self.observations == self.vues_a_la_derniere_decision {
            self.non_observees += 1;
        }
        if self.dernier_joueur_observe != player {
            self.mauvais_joueur += 1;
        }
        self.vues_a_la_derniere_decision = self.observations;
    }
}

impl Policy for CountingPolicy {
    fn observe(&mut self, _game: &GameState, player: usize) {
        self.observations += 1;
        self.dernier_joueur_observe = player;
    }

    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.decision(p);
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.decision(p);
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.decision(p);
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.decision(p);
        self.inner.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.decision(p);
        self.inner.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.decision(p);
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.decision(p);
        self.inner.action_choice(r, p, o)
    }
    fn action_amount(&mut self, r: &mut StdRng, p: usize, max: i64) -> i64 {
        self.decision(p);
        self.inner.action_amount(r, p, max)
    }
    // (regles-de-la-vente) `discard_payment_count` a disparu avec la vente
    // d'office qu'elle dosait ; `vendre_librement` est la question qui l'a
    // remplacée. Ce n'est PAS un point de décision mais un point d'OCCASION :
    // il ne compte donc pas dans le recensement des décisions (`self.decision`
    // n'est pas appelé), sans quoi le §2 compterait une occasion par décision
    // en plus de la décision elle-même.
    fn vendre_librement(&mut self, r: &mut StdRng, p: usize, main: &[u16]) -> Vec<usize> {
        self.inner.vendre_librement(r, p, main)
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        self.decision(p);
        self.inner.choose_option(r, p, n)
    }
    fn choose_res_target(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.decision(p);
        self.inner.choose_res_target(r, p, c)
    }
    fn choose_res_source(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.decision(p);
        self.inner.choose_res_source(r, p, c)
    }
    fn pick_joker_tag(&mut self, r: &mut StdRng, p: usize, card: u16, t: &[u32]) -> usize {
        self.decision(p);
        self.inner.pick_joker_tag(r, p, card, t)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.decision(p);
        self.inner.research_keep(r, p, d, k)
    }
    /// **Une révélation du dessus de pioche EST une décision**, même quand elle
    /// n'offre rien à prendre : le moteur pose les cartes devant le joueur et
    /// lui demande ce qu'il en fait — la réponse « rien » reste une réponse, et
    /// c'est l'écran qui en a besoin pour montrer les trois cartes. La question
    /// est donc comptée une fois ici, et déléguée à `inner` (et non à `self`)
    /// pour que le `research_keep` interne du corps par défaut ne la compte pas
    /// une seconde fois.
    fn reveal_pick(
        &mut self,
        r: &mut StdRng,
        p: usize,
        revelees: &[u16],
        candidates: &[u16],
        k: usize,
        f: engine::effects::RevealFilter,
    ) -> Vec<usize> {
        self.decision(p);
        self.inner.reveal_pick(r, p, revelees, candidates, k, f)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.decision(p);
        self.inner.discard_down(r, p, h, n)
    }
    /// (regles-de-la-vente) **`sell_card` entre dans le recensement.** C'était
    /// le SEUL point de décision de `flow.rs` que rien n'observait — l'écran y
    /// affichait donc « quelle carte vends-tu ? » sur l'état d'avant. Il reçoit
    /// désormais son observation, comme les autres, et doit donc être compté ici
    /// : sans cela le test verrait une observation orpheline et tomberait. Le
    /// motif de la correction est la vente libre — le joueur peut vouloir vendre
    /// avant de répondre à cette question-là aussi, et l'occasion se place au
    /// même endroit que l'observation.
    fn sell_card(&mut self, r: &mut StdRng, p: usize, main: &[u16]) -> usize {
        self.decision(p);
        self.inner.sell_card(r, p, main)
    }
}

/// **Exactement une observation par décision, et pas une de plus.**
///
/// Le nombre de parties n'est pas décoratif : il est MESURÉ. À 30 graines, le
/// retrait de l'observation du site `Action::SpendUpTo` (`flow.rs`, branche à
/// deux issues ou plus) passait inaperçu — aucune des 30 parties n'atteignait ce
/// chemin. À 100 graines il est vu ; on en prend 120 pour la marge. Chacun des
/// 33 sites de `flow.rs` est alors détecté individuellement : le retrait d'un
/// seul fait échouer ce test.
#[test]
fn une_observation_avant_chaque_decision() {
    let db = db_decouverte();
    for seed in 0..120u64 {
        let mut pol = CountingPolicy::new();
        play_game(&db, seed, &mut pol);
        assert!(pol.decisions > 100, "graine {seed} : {} décisions", pol.decisions);
        assert_eq!(
            pol.non_observees, 0,
            "graine {seed} : {} décisions prises sans observation préalable",
            pol.non_observees
        );
        assert_eq!(
            pol.observations, pol.decisions,
            "graine {seed} : {} observations pour {} décisions",
            pol.observations, pol.decisions
        );
        assert_eq!(
            pol.mauvais_joueur, 0,
            "graine {seed} : {} décisions observées pour le mauvais joueur",
            pol.mauvais_joueur
        );
    }
}

// ===========================================================================
// 3. FRAÎCHEUR — l'état observé est le vivant, recoupé sur les arguments du
//    moteur lui-même
// ===========================================================================

/// Politique qui retient ce qu'elle a OBSERVÉ, puis le recoupe avec les
/// arguments que `flow.rs` passe à la décision qui suit immédiatement. Ces
/// arguments sont calculés par le moteur au moment de l'appel : ils sont un
/// oracle de fraîcheur totalement disjoint du code d'observation.
struct FreshnessPolicy {
    inner: RandomPolicy,
    vu_mc: i64,
    vu_hand: usize,
    vu_tags: [u32; 10],
    vu_temperature: u8,
    vu_snap_temperature: u8,
    /// Recoupements RÉELLEMENT effectués (le test échoue s'ils sont nuls : un
    /// oracle qui ne se déclenche jamais ne prouve rien).
    recoupes_hand: u64,
    recoupes_tags: u64,
    recoupes_build: u64,
    /// Désaccords, un compteur par recoupement.
    ecarts_hand_paiement: u64,
    ecarts_build: u64,
    ecarts_tags: u64,
    /// Fois où le paramètre VIVANT différait de l'instantané de début de phase
    /// (`snap_temperature`) : preuve que l'observation ne lit pas l'instantané.
    vivant_different_du_snapshot: u64,
}

impl FreshnessPolicy {
    fn new() -> FreshnessPolicy {
        FreshnessPolicy {
            inner: RandomPolicy,
            vu_mc: 0,
            vu_hand: 0,
            vu_tags: [0; 10],
            vu_temperature: 0,
            vu_snap_temperature: 0,
            recoupes_hand: 0,
            recoupes_tags: 0,
            recoupes_build: 0,
            ecarts_hand_paiement: 0,
            ecarts_build: 0,
            ecarts_tags: 0,
            vivant_different_du_snapshot: 0,
        }
    }
}

impl Policy for FreshnessPolicy {
    fn observe(&mut self, game: &GameState, player: usize) {
        let pl = &game.players[player];
        self.vu_mc = pl.mc;
        self.vu_hand = pl.hand.len();
        self.vu_tags = pl.tag_counts;
        self.vu_temperature = game.temperature;
        self.vu_snap_temperature = game.snap_temperature;
        if game.temperature != game.snap_temperature {
            self.vivant_different_du_snapshot += 1;
        }
    }

    /// (regles-de-la-vente) **L'oracle sur les MC a disparu avec la décision qui
    /// les portait.** `discard_payment_count` recevait `game.players[p].mc` en
    /// troisième argument ; cette question n'existe plus, parce que payer en
    /// défaussant d'office est exactement le défaut que cette tâche supprime.
    /// Plus aucune décision du moteur ne reçoit les MC du joueur : l'oracle
    /// n'est pas affaibli, il est devenu sans objet, et `recoupes_mc` est retiré
    /// plutôt que laissé à zéro — un oracle vide qui passe est pire qu'un oracle
    /// absent.
    ///
    /// Il est remplacé, à recouvrement égal, par la MAIN que le moteur soumet à
    /// l'action standard de vente : `flow.rs` appelle
    /// `policy.sell_card(&mut game.rng, p, &main)` où `main` est la main du
    /// joueur à cet instant. Elle doit être celle qu'on vient d'observer.
    fn sell_card(&mut self, r: &mut StdRng, p: usize, main: &[u16]) -> usize {
        self.recoupes_hand += 1;
        if main.len() != self.vu_hand {
            self.ecarts_hand_paiement += 1;
        }
        self.inner.sell_card(r, p, main)
    }

    /// `flow.rs` appelle : `policy.choose_build(&mut game.rng, p, &opts)` où
    /// `opts` est une liste d'INDICES DANS LA MAIN, énumérée par `affordable`
    /// sur l'état courant. Tout indice doit donc désigner une carte de la main
    /// telle qu'on vient de l'observer : une observation périmée d'une manche
    /// (main plus courte après la défausse de fin de manche, plus longue avant
    /// la pose) ferait sortir ces indices des bornes.
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.recoupes_build += 1;
        if a.iter().any(|&i| i >= self.vu_hand) {
            self.ecarts_build += 1;
        }
        self.inner.choose_build(r, p, a)
    }

    /// `flow.rs` appelle : `policy.pick_joker_tag(&mut game.rng, p, card_id,
    /// &counts)` où `counts = game.players[p].tag_counts` à cet instant.
    fn pick_joker_tag(&mut self, r: &mut StdRng, p: usize, card: u16, t: &[u32]) -> usize {
        self.recoupes_tags += 1;
        if t != self.vu_tags {
            self.ecarts_tags += 1;
        }
        self.inner.pick_joker_tag(r, p, card, t)
    }

    // Les huit méthodes sans corps par défaut : déléguées telles quelles, la
    // partie se déroule comme avec `RandomPolicy`.
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
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.inner.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

/// **L'observation porte l'état VIVANT, pas une copie retardée.** Recoupement
/// sur 40 parties contre les arguments que le moteur calcule lui-même.
#[test]
fn l_observation_porte_l_etat_vivant() {
    let db = db_decouverte();
    let mut pol = FreshnessPolicy::new();
    for seed in 0..40u64 {
        play_game(&db, seed, &mut pol);
    }
    println!(
        "recoupements : main-vente={} badges={} indices-de-pose={}",
        pol.recoupes_hand, pol.recoupes_tags, pol.recoupes_build
    );
    assert!(
        pol.recoupes_hand > 0 && pol.recoupes_tags > 0 && pol.recoupes_build > 0,
        "oracle vide : main-vente={} tags={} build={}",
        pol.recoupes_hand,
        pol.recoupes_tags,
        pol.recoupes_build
    );
    assert_eq!(
        (pol.ecarts_hand_paiement, pol.ecarts_tags, pol.ecarts_build),
        (0, 0, 0),
        "écarts entre l'état observé et celui que le moteur passe à la décision \
         (main-vente, badges, indices de pose) sur {} / {} / {} recoupements",
        pol.recoupes_hand, pol.recoupes_tags, pol.recoupes_build
    );
    assert!(
        pol.vivant_different_du_snapshot > 0,
        "le paramètre observé n'a JAMAIS différé de l'instantané de début de \
         phase : rien ne prouve alors que l'observation lit le vivant"
    );
}

/// La température observée bouge À L'INTÉRIEUR d'une manche : c'est le défaut
/// exact que le chantier corrige (une observation par manche donnerait des
/// valeurs constantes). Mesuré ici sur les MC, plus mobiles.
#[test]
fn les_valeurs_observees_bougent_dans_une_manche() {
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(&db, RandomPolicy);
    play_game(&db, 4242, &mut obs);

    let mut bougent = 0usize;
    let mut mesurables = 0usize;
    for gen in 1..=12u32 {
        for p in 0..NUM_PLAYERS {
            let mcs: Vec<i64> = obs
                .records()
                .iter()
                .filter(|o| o.generation == gen && o.player == p)
                .map(|o| o.mc)
                .collect();
            if mcs.len() > 2 {
                mesurables += 1;
                if mcs.iter().any(|m| *m != mcs[0]) {
                    bougent += 1;
                }
            }
        }
    }
    assert!(mesurables > 0, "aucune manche mesurable");
    assert!(
        bougent > 0,
        "les MC observés n'ont jamais varié à l'intérieur d'une manche : \
         l'observation est un instantané périmé"
    );
}

/// La numérotation des décisions est stricte : 0, 1, 2… sans trou ni doublon.
#[test]
fn numerotation_stricte_des_decisions() {
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(&db, RandomPolicy);
    play_game(&db, 12345, &mut obs);
    assert!(obs.records().len() > 100);
    for (i, o) in obs.records().iter().enumerate() {
        assert_eq!(o.decision, i as u64, "trou ou doublon en position {i}");
        assert!(o.player < NUM_PLAYERS);
    }
    assert_eq!(obs.decisions(), obs.records().len() as u64);
}

/// Les DEUX joueurs sont observés (une vue qui ne verrait qu'un joueur serait
/// inutilisable pour une interface).
#[test]
fn les_deux_joueurs_sont_observes() {
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(&db, RandomPolicy);
    play_game(&db, 31337, &mut obs);
    for p in 0..NUM_PLAYERS {
        assert!(
            obs.records().iter().any(|o| o.player == p),
            "joueur {p} jamais observé"
        );
    }
}

// ===========================================================================
// 4. LA VUE SÉRIALISABLE
// ===========================================================================

/// La vue dit ce que le MOTEUR pense, champ par champ — elle ne recalcule rien
/// de son côté. Vérifié sur un état de partie AVANCÉE (pas une mise en place :
/// à la mise en place tout vaut zéro, et une vue en dur passerait).
#[test]
fn la_vue_est_fidele_a_l_etat_du_moteur() {
    let db = db_decouverte();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 4242, &mut pol);
    for _ in 0..6 {
        if game.game_over {
            break;
        }
        play_round(&mut game, &db, &mut pol);
    }

    let v = state_view(&game, &db);
    assert_eq!(v["generation"], game.generation);
    assert_eq!(v["first_player"], game.first_player);
    assert_eq!(v["game_over"], game.game_over);
    assert_eq!(v["planet"]["temperature"], game.temperature);
    assert_eq!(v["planet"]["oxygen"], game.oxygen);
    assert_eq!(v["planet"]["oceans"], game.oceans_revealed);
    assert_eq!(v["planet"]["infrastructure"], game.infrastructure);
    assert_eq!(v["decks"]["deck"], game.deck.len());
    assert_eq!(v["decks"]["discard"], game.discard.len());
    assert_eq!(v["milestones"].as_array().unwrap().len(), 3);
    assert_eq!(v["awards"].as_array().unwrap().len(), 3);

    let scores = score(&game, &db);
    let joueurs = v["players"].as_array().unwrap();
    assert_eq!(joueurs.len(), NUM_PLAYERS);
    for p in 0..NUM_PLAYERS {
        let pl = &game.players[p];
        let j = &joueurs[p];
        assert_eq!(j["player"], p);
        assert_eq!(j["mc"], pl.mc);
        assert_eq!(j["heat"], pl.heat);
        assert_eq!(j["plants"], pl.plants);
        assert_eq!(j["tr"], pl.tr);
        assert_eq!(j["forests"], pl.forests);
        assert_eq!(j["production"]["mc"], pl.mc_prod);
        assert_eq!(j["production"]["heat"], pl.heat_prod);
        assert_eq!(j["steel_capacity"], pl.steel_capacity);
        assert_eq!(j["titanium_capacity"], pl.titanium_capacity);
        assert_eq!(j["chosen_phase"], pl.chosen_phase);
        // Le score de la vue est celui du point de calcul unique du moteur.
        assert_eq!(j["score"], scores[p]);
        // Main et cartes posées : mêmes cartes, même nombre, mêmes identifiants.
        let main = j["hand"].as_array().unwrap();
        assert_eq!(main.len(), pl.hand.len());
        for (k, c) in main.iter().enumerate() {
            assert_eq!(c["id"], pl.hand[k]);
            assert_eq!(c["name"], db.projects[pl.hand[k] as usize].name);
        }
        let posees = j["played"].as_array().unwrap();
        assert_eq!(posees.len(), pl.played.len());
        for (k, c) in posees.iter().enumerate() {
            assert_eq!(c["id"], pl.played[k]);
            assert_eq!(c["resources"], pl.resources_on(pl.played[k]));
        }
        // Corporation : le nom que porte la planche choisie.
        match pl.corporation {
            Some(c) => assert_eq!(j["corporation"], db.corporations[c as usize].name),
            None => assert!(j["corporation"].is_null()),
        }
        // Badges : le décompte du moteur, badge par badge.
        let total_vue: u64 = j["tags"]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_u64().unwrap())
            .sum();
        assert_eq!(total_vue, pl.tag_counts.iter().map(|c| *c as u64).sum::<u64>());
    }
}

/// La vue dépend RÉELLEMENT de la partie : deux graines, deux vues.
#[test]
fn la_vue_depend_de_la_partie() {
    let db = db_decouverte();
    let a = state_view(&setup_game(&db, 4242, &mut RandomPolicy), &db);
    let b = state_view(&setup_game(&db, 777, &mut RandomPolicy), &db);
    assert_ne!(a, b, "deux graines donnent la même vue : elle est écrite en dur");
    // Et elle est stable à graine égale (une vue non déterministe serait
    // inexploitable par une interface).
    let a2 = state_view(&setup_game(&db, 4242, &mut RandomPolicy), &db);
    assert_eq!(a, a2);
}

/// Une mise en place donne une vue déjà complète : deux joueurs, huit cartes en
/// main chacun, une corporation, aucune carte posée.
#[test]
fn la_vue_de_la_mise_en_place_est_complete() {
    let db = db_decouverte();
    let v = state_view(&setup_game(&db, 4242, &mut RandomPolicy), &db);
    assert_eq!(v["generation"], 1);
    let joueurs = v["players"].as_array().unwrap();
    assert_eq!(joueurs.len(), 2);
    for j in joueurs {
        assert_eq!(j["hand"].as_array().unwrap().len(), 8);
        assert_eq!(j["played"].as_array().unwrap().len(), 0);
        assert!(j["corporation"].is_string(), "corporation absente de la vue");
        assert!(j["tr"].as_i64().unwrap() >= 5);
    }
}

/// **La vue sérialisable est accessible DEPUIS `observe`** — c'est ce que le
/// contrat demande, et ce dont le pont navigateur aura besoin. On l'exerce avec
/// une politique qui appelle `state_view` dans son propre `observe`, et l'on
/// vérifie que la vue ainsi obtenue décrit bien l'état de CETTE décision-là.
struct ViewFromObserve<'a> {
    db: &'a CardsDb,
    inner: RandomPolicy,
    vues: usize,
    ecarts: usize,
}

impl Policy for ViewFromObserve<'_> {
    fn observe(&mut self, game: &GameState, player: usize) {
        let v = state_view(game, self.db);
        self.vues += 1;
        // La vue rendue depuis `observe` porte l'état de CE moment.
        if v["planet"]["temperature"] != game.temperature
            || v["players"][player]["mc"] != game.players[player].mc
            || v["generation"] != game.generation
        {
            self.ecarts += 1;
        }
    }
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
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.inner.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

#[test]
fn la_vue_est_accessible_depuis_observe() {
    let db = db_decouverte();
    let mut pol = ViewFromObserve {
        db: &db,
        inner: RandomPolicy,
        vues: 0,
        ecarts: 0,
    };
    let vue = play_game(&db, 4242, &mut pol);
    assert!(pol.vues > 100, "{} vues seulement", pol.vues);
    assert_eq!(pol.ecarts, 0, "{} vues incohérentes avec l'état", pol.ecarts);

    // Et la partie ainsi observée est restée la partie de référence.
    let nu = play_game(&db, 4242, &mut RandomPolicy);
    assert_eq!(nu.state_hash, vue.state_hash);
}

/// `ObservingPolicy::state_view` rend exactement ce que la fonction libre rend.
#[test]
fn state_view_de_l_enveloppe_est_la_meme() {
    let db = db_decouverte();
    let obs = ObservingPolicy::new(&db, RandomPolicy);
    let game = setup_game(&db, 4242, &mut RandomPolicy);
    assert_eq!(obs.state_view(&game), state_view(&game, &db));
}

// ===========================================================================
// 5. LES RÉGLAGES DE L'ENVELOPPE
// ===========================================================================

/// `keeping(false)` cesse de garder les observations sans cesser de les compter
/// — c'est ce que le binaire emploie sur des centaines de parties.
#[test]
fn keeping_false_compte_sans_garder() {
    let db = db_base();
    let mut obs = ObservingPolicy::new(&db, RandomPolicy).keeping(false);
    play_game(&db, 8, &mut obs);
    assert!(obs.decisions() > 100);
    assert!(obs.records().is_empty());
}

/// L'enveloppe rend la politique enveloppée intacte, et une enveloppe
/// d'enveloppe observe deux fois la même chose (la délégation d'`observe` est
/// réelle).
#[test]
fn enveloppe_d_enveloppe() {
    let db = db_base();
    let interne = ObservingPolicy::new(&db, RandomPolicy);
    let mut externe = ObservingPolicy::new(&db, interne);
    play_game(&db, 5, &mut externe);
    let n = externe.decisions();
    let interne = externe.into_inner();
    assert!(n > 100);
    assert_eq!(interne.decisions(), n, "l'observation n'a pas été déléguée");
}

// ===========================================================================
// 6. CE QUE LA REVUE ADVERSARIALE A TROUVÉ NON PINNÉ
//
// Deux sabotages passaient inaperçus des tests ci-dessus (le code livré était
// correct, mais rien ne l'empêchait de régresser) :
//   - remplacer `game.temperature` par `game.snap_temperature` dans
//     `ObservingPolicy::observe` — le shortcut nommé par le contrat ;
//   - retirer la délégation d'une des six méthodes du trait à corps par défaut,
//     que le compilateur n'exige pas.
// Les deux tests suivants les pinnent.
// ===========================================================================

/// Politique qui relève, à chaque observation, l'INSTANTANÉ de début de phase.
/// Enveloppée dans `ObservingPolicy`, elle reçoit l'observation par délégation :
/// on obtient donc, pour la MÊME décision, ce que l'enveloppe a retenu (le
/// vivant) et ce qu'un instantané aurait donné.
struct SnapWatcher {
    inner: RandomPolicy,
    snap_temperature: Vec<u8>,
    snap_oxygen: Vec<u8>,
    snap_oceans: Vec<u8>,
}

impl SnapWatcher {
    fn new() -> SnapWatcher {
        SnapWatcher {
            inner: RandomPolicy,
            snap_temperature: Vec::new(),
            snap_oxygen: Vec::new(),
            snap_oceans: Vec::new(),
        }
    }
}

impl Policy for SnapWatcher {
    fn observe(&mut self, game: &GameState, _player: usize) {
        self.snap_temperature.push(game.snap_temperature);
        self.snap_oxygen.push(game.snap_oxygen);
        self.snap_oceans.push(game.snap_oceans);
    }
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
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.inner.action_choice(r, p, o)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

/// **Les paramètres planétaires observés sont les VIVANTS, pas l'instantané de
/// début de phase.** Chacun des trois est comparé, décision par décision, à son
/// homologue `snap_*` : si `ObservingPolicy` lisait l'instantané (le
/// contournement que le contrat nomme), les deux séries seraient égales partout.
#[test]
fn les_parametres_observes_ne_sont_pas_l_instantane() {
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(&db, SnapWatcher::new());
    for seed in 0..12u64 {
        play_game(&db, seed, &mut obs);
    }
    let vivants: Vec<(u8, u8, u8)> = obs
        .records()
        .iter()
        .map(|o| (o.temperature, o.oxygen, o.oceans))
        .collect();
    let snap = obs.into_inner();
    assert_eq!(vivants.len(), snap.snap_temperature.len());
    assert!(vivants.len() > 1000, "échantillon trop maigre");

    let dt = (0..vivants.len()).filter(|&i| vivants[i].0 != snap.snap_temperature[i]).count();
    let dx = (0..vivants.len()).filter(|&i| vivants[i].1 != snap.snap_oxygen[i]).count();
    let dc = (0..vivants.len()).filter(|&i| vivants[i].2 != snap.snap_oceans[i]).count();
    assert!(
        dt > 0,
        "la température observée n'a JAMAIS différé de snap_temperature : \
         l'observation lit l'instantané de début de phase, pas le vivant"
    );
    assert!(dx > 0, "l'oxygène observé n'a jamais différé de snap_oxygen");
    assert!(dc > 0, "les océans observés n'ont jamais différé de snap_oceans");
}

/// Politique qui COMPTE les appels reçus, méthode par méthode, et délègue tout à
/// `RandomPolicy`. Enveloppée, elle ne doit rien perdre : un compteur resté à
/// zéro signale une délégation manquante dans `ObservingPolicy`.
const NOMS: [&str; 15] = [
    "corp_mulligan",
    "project_mulligan",
    "pick_corporation",
    "pick_phase",
    "choose_build",
    "construction_bonus",
    "action_choice",
    "action_amount",
    "vendre_librement",
    "choose_option",
    "choose_res_target",
    "choose_res_source",
    "pick_joker_tag",
    "research_keep",
    "discard_down",
];

struct TallyPolicy {
    inner: RandomPolicy,
    calls: [u64; 15],
}

impl TallyPolicy {
    fn new() -> TallyPolicy {
        TallyPolicy { inner: RandomPolicy, calls: [0; 15] }
    }
}

impl Policy for TallyPolicy {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.calls[0] += 1;
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> Vec<usize> {
        self.calls[1] += 1;
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.calls[2] += 1;
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, a: &[u8]) -> u8 {
        self.calls[3] += 1;
        self.inner.pick_phase(r, p, a)
    }
    fn choose_build(&mut self, r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        self.calls[4] += 1;
        self.inner.choose_build(r, p, a)
    }
    fn construction_bonus(&mut self, r: &mut StdRng, p: usize) -> ConstructionBonus {
        self.calls[5] += 1;
        self.inner.construction_bonus(r, p)
    }
    fn action_choice(&mut self, r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.calls[6] += 1;
        self.inner.action_choice(r, p, o)
    }
    fn action_amount(&mut self, r: &mut StdRng, p: usize, max: i64) -> i64 {
        self.calls[7] += 1;
        self.inner.action_amount(r, p, max)
    }
    fn vendre_librement(&mut self, r: &mut StdRng, p: usize, main: &[u16]) -> Vec<usize> {
        self.calls[8] += 1;
        self.inner.vendre_librement(r, p, main)
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        self.calls[9] += 1;
        self.inner.choose_option(r, p, n)
    }
    fn choose_res_target(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.calls[10] += 1;
        self.inner.choose_res_target(r, p, c)
    }
    fn choose_res_source(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.calls[11] += 1;
        self.inner.choose_res_source(r, p, c)
    }
    fn pick_joker_tag(&mut self, r: &mut StdRng, p: usize, card: u16, t: &[u32]) -> usize {
        self.calls[12] += 1;
        self.inner.pick_joker_tag(r, p, card, t)
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        self.calls[13] += 1;
        self.inner.research_keep(r, p, d, k)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.calls[14] += 1;
        self.inner.discard_down(r, p, h, n)
    }
}

/// **Les QUINZE méthodes de décision sont réellement déléguées.** Six d'entre
/// elles ont un corps par défaut dans le trait : le compilateur n'exige pas
/// qu'`ObservingPolicy` les redéfinisse, et si l'une manquait, l'enveloppe
/// répondrait à la place de la politique enveloppée — silencieusement, car
/// `RandomPolicy` ne les surcharge pas non plus et l'empreinte ne bougerait pas.
/// Ce test le voit : une méthode non déléguée n'atteint jamais le compteur.
#[test]
fn les_quinze_methodes_sont_deleguees() {
    let db = db_decouverte();
    let mut obs = ObservingPolicy::new(&db, TallyPolicy::new()).keeping(false);
    for seed in 0..25u64 {
        play_game(&db, seed, &mut obs);
    }
    let tally = obs.into_inner();
    for (i, nom) in NOMS.iter().enumerate() {
        assert!(
            tally.calls[i] > 0,
            "`{nom}` n'a jamais atteint la politique enveloppée : \
             ObservingPolicy ne la délègue pas (corps par défaut du trait employé \
             à sa place). Compteurs : {:?}",
            tally.calls
        );
    }
}

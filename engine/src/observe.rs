//! **(moteur-observe) La vue de la partie, ouverte à celui qui décide.**
//!
//! Deux choses, et rien d'autre :
//!
//! 1. [`state_view`] — le rendu JSON complet de `GameState`, celui que le
//!    chantier de l'interface consommera (`--dump-state`) ;
//! 2. [`ObservingPolicy`] — une politique qui ENVELOPPE une politique existante,
//!    reçoit `Policy::observe` avant chaque décision, enregistre ce qu'elle voit,
//!    et délègue **toutes** ses réponses à la politique enveloppée
//!    (`--observe`).
//!
//! Ce module n'écrit jamais dans `GameState` et ne consomme jamais le RNG de la
//! partie : il ne peut pas, structurellement, changer une décision. C'est ce qui
//! rend les trois empreintes de référence insensibles à sa présence.
//!
//! Les noms de champs sont en ANGLAIS, comme le reste du moteur (`probe.rs`,
//! la ligne de bilan de `simulate`) ; les seules exceptions sont les valeurs que
//! le moteur nomme déjà en français, et qui sont reprises telles quelles
//! (`Color::nom_fr`, rendu par `--dump-deck` sous la clef `couleur`).

use crate::cards::{CardsDb, JOKER_TAG_CHOICES};
use crate::choice::ChoiceContext;
use crate::flow::{score_breakdown, ScoreBreakdown};
use crate::policy::{ActionOpt, ConstructionBonus, Policy};
use crate::state::{
    GameState, PlayerState, NUM_OCEANS, NUM_PLAYERS, OXYGEN_MAX, TEMPERATURE_MAX,
};
use rand::rngs::StdRng;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// 1. La vue sérialisable de l'état
// ---------------------------------------------------------------------------

/// Rendu JSON d'un joueur : ressources, productions, TR, badges, corporation,
/// main, cartes posées, score courant.
///
/// `score` et `score_parts` viennent tous deux de
/// [`crate::flow::score_breakdown`], le point de calcul UNIQUE du score : la vue
/// ne tient pas de barème parallèle, elle rapporte ce que le moteur compte, et
/// `score` n'est rien d'autre que la somme des parts publiées à côté. (Un score
/// « courant » est le score que la partie donnerait si elle s'arrêtait
/// maintenant ; en cours de partie il n'est donc pas définitif — c'est
/// précisément ce que la ventilation rend lisible : les parts `milestones` et
/// `awards` peuvent encore basculer.)
fn player_view(game: &GameState, db: &CardsDb, p: usize, parts: &ScoreBreakdown) -> Value {
    let pl: &PlayerState = &game.players[p];

    // Badges : `tag_counts` est indexé par `Tag::index`, c'est-à-dire dans le
    // MÊME ordre que `JOKER_TAG_CHOICES`. On lit donc l'ordre du moteur, on n'en
    // réinvente pas un second.
    let mut tags = serde_json::Map::new();
    for (i, tag) in JOKER_TAG_CHOICES.iter().enumerate() {
        tags.insert(tag.as_str().to_string(), json!(pl.tag_counts[i]));
    }

    let card = |id: u16| -> Value {
        let c = &db.projects[id as usize];
        json!({
            "id": id,
            "name": c.name,
            "couleur": c.color.nom_fr(),
            "price": c.price,
        })
    };

    json!({
        "player": p,
        "corporation": pl.corporation.map(|c| db.corporations[c as usize].name.clone()),
        // Ressources en réserve.
        "mc": pl.mc,
        "heat": pl.heat,
        "plants": pl.plants,
        "tr": pl.tr,
        "forests": pl.forests,
        // Productions.
        "production": {
            "mc": pl.mc_prod,
            "heat": pl.heat_prod,
            "plants": pl.plant_prod,
            "cards": pl.card_prod,
        },
        // Savoir-faire (acier / titane) : ce sont des réductions permanentes,
        // pas des jetons dépensables.
        "steel_capacity": pl.steel_capacity,
        "titanium_capacity": pl.titanium_capacity,
        "tags": Value::Object(tags),
        // Mode bac à sable : les DEUX mains sont visibles. C'est le propre de
        // cette vue — elle sert un moteur, pas encore un joueur humain.
        "hand": pl.hand.iter().map(|&id| card(id)).collect::<Vec<_>>(),
        // (regles-de-la-vente) **CE QUE CE JOUEUR A LES MOYENS DE PAYER**, carte
        // par carte de `hand`, dans le même ordre. C'est le moteur qui répond —
        // `flow::main_payable`, le même point de calcul que l'énumération des
        // options d'une pose — parce que l'écran ne sait ni ce qu'une carte
        // coûte ni quelles réductions s'y appliquent.
        //
        // C'est de là que vient le contour vert. Il annonçait naguère jouable
        // une carte à 17 MC à un joueur qui en avait 13, parce que le moteur
        // comptait d'avance la vente de trois autres cartes de sa main ; il suit
        // désormais les MC RÉELS, et il s'allume tout seul dès qu'une vente en
        // ajoute — la page ne recalcule rien, elle recopie ce champ-ci.
        "main_payable": crate::flow::main_payable(game, db, p),
        "played": pl.played.iter().map(|&id| {
            let mut v = card(id);
            // Ressources posées SUR la carte (lot 3), lues sur le joueur.
            v["resources"] = json!(pl.resources_on(id));
            v
        }).collect::<Vec<_>>(),
        "chosen_phase": pl.chosen_phase,
        "previous_phase": pl.previous_phase,
        "phase_upgrades": pl.phase_upgrade_labels(),
        "score": parts.total(),
        // (regles-de-la-vente) **Le score ACQUIS** : le total moins les
        // récompenses, qui ne seront attribuées qu'à la fin de la partie. C'est
        // ce nombre-là que l'écran met en gros tant que la partie n'est pas
        // finie ; `score` ci-dessus ne bouge pas d'un point, et reste ce que
        // lisent le classement et le simulateur.
        "score_acquis": parts.acquis(),
        // La VENTILATION du score, dans les cinq parts du livret. Rien n'est
        // ajouté au décompte : ce sont les termes que `score_breakdown` vient
        // d'additionner pour former `score` ci-dessus.
        "score_parts": {
            "tr": parts.tr,
            "forests": parts.forests,
            "cards": parts.cards,
            "milestones": parts.milestones,
            "awards": parts.awards,
        },
    })
}

/// **Le rendu JSON complet de l'état d'une partie.**
///
/// C'est la fonction que consommeront `--dump-state`, [`ObservingPolicy`] en
/// mode « état complet », et le futur pont navigateur. Tout ce qu'elle rend est
/// LU sur `game` (ou sur les points de calcul uniques du moteur, comme
/// `score_parts`) : elle ne recalcule aucune valeur pour son propre compte, donc
/// elle ne peut pas afficher juste et mentir sur ce que le moteur pense.
pub fn state_view(game: &GameState, db: &CardsDb) -> Value {
    let (parts, _, _) = score_breakdown(game, db);
    json!({
        "generation": game.generation,
        "first_player": game.first_player,
        "game_over": game.game_over,
        // (regles-de-la-vente) La phase que le moteur résout à cet instant (1 à
        // 5), ou 0 hors phase : mise en place, planification, étape de fin de
        // manche. Écrite par `flow::play_round`, lue telle quelle. L'écran en
        // déduisait naguère la valeur du TYPE de la décision reçue ; les deux
        // doivent en dire la même chose, sinon le bouton de vente s'offre là où
        // le moteur ne l'accepte pas.
        "phase_en_cours": game.phase_en_cours,
        // Une vente volontaire est-elle recevable au point où le moteur se
        // trouve ? (`flow::occasion_de_vendre`.)
        "vente_offerte": game.vente_offerte,
        "ventes_volontaires": game.ventes_volontaires,
        // Paramètres planétaires, avec leurs plafonds : sans eux, « oxygène 7 »
        // ne dit pas à quelle distance on est de la fin de partie.
        "planet": {
            "temperature": game.temperature,
            "temperature_max": TEMPERATURE_MAX,
            "oxygen": game.oxygen,
            "oxygen_max": OXYGEN_MAX,
            "oceans": game.oceans_revealed,
            "oceans_max": NUM_OCEANS,
            // Les tuiles Océan DÉJÀ RETOURNÉES, dans l'ordre où elles l'ont
            // été. Sans cette liste, l'écran connaît le nombre d'océans mais
            // pas lesquels : il en montrait donc de fausses. `id` est le rang
            // de la tuile sur la planche imprimée (`state::OCEAN_TILES`), les
            // trois autres champs sont le bonus qu'elle a versé.
            "oceans_revealed_tiles": game.oceans[..game.oceans_revealed as usize]
                .iter()
                .map(|t| json!({
                    "id": t.id,
                    "cards": t.cards,
                    "mc": t.mc,
                    "plants": t.plants,
                }))
                .collect::<Vec<_>>(),
            "infrastructure": game.infrastructure,
        },
        // Tailles des paquets : le contenu de la pioche n'est pas une
        // information du jeu, son épaisseur l'est.
        "decks": {
            "deck": game.deck.len(),
            "discard": game.discard.len(),
            "corp_deck": game.corp_deck.len(),
            "corp_discard": game.corp_discard.len(),
        },
        // Repères (Objectifs) et Récompenses de CETTE partie : trois de chaque,
        // tirés à la mise en place.
        "milestones": game.milestones.iter().map(|m| json!({
            "kind": m.kind.name(),
            "achieved_by": m.achieved_by,
        })).collect::<Vec<_>>(),
        "awards": game.awards.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>(),
        "players": (0..NUM_PLAYERS)
            .map(|p| player_view(game, db, p, &parts[p]))
            .collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// 2. La politique observatrice
// ---------------------------------------------------------------------------

/// Ce qu'une observation retient d'une décision. Tout est relevé sur l'état
/// VIVANT reçu par `Policy::observe`, au moment exact de l'appel — jamais sur
/// les champs `snap_*` de `GameState`, qui datent du début de la phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// Numéro de la décision, à partir de 0, sans trou : une par appel.
    pub decision: u64,
    /// Joueur à qui la décision va être demandée.
    pub player: usize,
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    pub tr: i64,
    pub temperature: u8,
    pub oxygen: u8,
    pub oceans: u8,
    pub generation: u32,
    /// Phase choisie par ce joueur pour la manche en cours (0 = pas encore).
    pub phase: u8,
    pub hand: usize,
    pub played: usize,
}

impl Observation {
    /// L'observation en objet JSON — une ligne de `--observe`.
    pub fn to_json(&self) -> Value {
        json!({
            "decision": self.decision,
            "player": self.player,
            "mc": self.mc,
            "heat": self.heat,
            "plants": self.plants,
            "tr": self.tr,
            "temperature": self.temperature,
            "oxygen": self.oxygen,
            "oceans": self.oceans,
            "generation": self.generation,
            "phase": self.phase,
            "hand": self.hand,
            "played": self.played,
        })
    }
}

/// **Une politique qui regarde, et ne touche à rien.**
///
/// Elle enveloppe une politique existante `inner` et délègue **chacune** des
/// méthodes de décision du trait, sans en réécrire aucune : pas une seule
/// décision, pas un seul tirage ne change. Son unique apport est
/// `Policy::observe`, où elle enregistre l'état reçu.
///
/// La délégation est exhaustive **par nécessité** : une méthode oubliée ici
/// retomberait sur le corps par défaut du trait au lieu de celui de `inner`, et
/// changerait donc silencieusement le jeu dès que `inner` la surcharge.
pub struct ObservingPolicy<'a, P: Policy> {
    inner: P,
    db: &'a CardsDb,
    decisions: u64,
    records: Vec<Observation>,
    /// Écrire chaque observation sur la sortie standard, au fil de l'eau
    /// (`--observe`).
    emit: bool,
    /// Joindre à chaque observation la vue COMPLÈTE de l'état ([`state_view`]),
    /// sous la clef `state`. Coûteux : hors par défaut.
    full_state: bool,
    /// Garder les observations en mémoire (utile aux tests ; inutile sur des
    /// centaines de parties, où l'on se contente d'émettre).
    keep: bool,
}

impl<'a, P: Policy> ObservingPolicy<'a, P> {
    /// Enveloppe `inner`. Par défaut : on enregistre en mémoire, on n'émet rien.
    pub fn new(db: &'a CardsDb, inner: P) -> ObservingPolicy<'a, P> {
        ObservingPolicy {
            inner,
            db,
            decisions: 0,
            records: Vec::new(),
            emit: false,
            full_state: false,
            keep: true,
        }
    }

    /// Écrire chaque observation sur stdout (une ligne JSON par décision).
    pub fn emitting(mut self, on: bool) -> Self {
        self.emit = on;
        self
    }

    /// Joindre la vue complète de l'état à chaque observation émise.
    pub fn with_full_state(mut self, on: bool) -> Self {
        self.full_state = on;
        self
    }

    /// Garder (ou non) les observations en mémoire.
    pub fn keeping(mut self, on: bool) -> Self {
        self.keep = on;
        self
    }

    /// Les observations enregistrées, dans l'ordre des décisions.
    pub fn records(&self) -> &[Observation] {
        &self.records
    }

    /// Nombre de décisions observées — y compris quand rien n'est gardé.
    pub fn decisions(&self) -> u64 {
        self.decisions
    }

    /// La vue complète de l'état, telle que [`state_view`] la rend, avec la base
    /// de cartes que cette politique porte. C'est par ici que `observe` accède à
    /// la vue sérialisable.
    pub fn state_view(&self, game: &GameState) -> Value {
        state_view(game, self.db)
    }

    /// La politique enveloppée, rendue à l'appelant.
    pub fn into_inner(self) -> P {
        self.inner
    }
}

impl<P: Policy> Policy for ObservingPolicy<'_, P> {
    /// **Le seul endroit où cette politique fait quelque chose de son cru.**
    /// Elle lit `game` — l'état VIVANT au moment de la décision qui suit — et
    /// n'écrit rien dedans.
    fn observe(&mut self, game: &GameState, player: usize) {
        let pl = &game.players[player];
        let o = Observation {
            decision: self.decisions,
            player,
            mc: pl.mc,
            heat: pl.heat,
            plants: pl.plants,
            tr: pl.tr,
            temperature: game.temperature,
            oxygen: game.oxygen,
            oceans: game.oceans_revealed,
            generation: game.generation,
            phase: pl.chosen_phase,
            hand: pl.hand.len(),
            played: pl.played.len(),
        };
        self.decisions += 1;
        if self.emit {
            let mut line = o.to_json();
            if self.full_state {
                line["state"] = self.state_view(game);
            }
            println!("{line}");
        }
        if self.keep {
            self.records.push(o);
        }
        // Une politique enveloppée peut elle-même vouloir observer.
        self.inner.observe(game, player);
    }

    // ------------------------------------------------------------------
    // Délégation intégrale. Aucune de ces méthodes ne consulte l'état
    // observé : elles passent la main, point.
    // ------------------------------------------------------------------

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

    /// (choix-parlants) La voie enrichie est déléguée elle aussi, et il le
    /// FAUT : sans cette ligne, l'enveloppe retomberait sur le corps par défaut
    /// du trait, lequel appelle `choose_option` — la politique enveloppée qui
    /// sait lire un contexte ne le verrait jamais, silencieusement.
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        self.inner.choose_option_ctx(rng, player, ctx)
    }

    fn choose_res_target(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        candidates: &[u16],
    ) -> usize {
        self.inner.choose_res_target(rng, player, candidates)
    }

    fn choose_res_source(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        candidates: &[u16],
    ) -> usize {
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

    /// Transmise TELLE QUELLE, comme toutes les autres : un enrobage qui
    /// oublierait cette méthode laisserait le corps par défaut répondre à la
    /// place de la politique enrobée — l'écran ne verrait plus jamais une
    /// révélation.
    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        filter: crate::effects::RevealFilter,
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

    fn sell_card(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> usize {
        self.inner.sell_card(rng, player, hand)
    }
}

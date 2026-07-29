//! Sonde d'audit. Deux modes, tous deux par le MÊME chemin de pose que
//! `simulate` (`flow::build_card`) depuis un état de départ fixe et documenté :
//!
//! - `--probe "<A>;<B>;…"` : pose FORCÉE des cartes DANS L'ORDRE. Rapporte le
//!   delta d'état CUMULÉ (hors prix payés) et `paid[]` = prix réellement payé de
//!   chaque carte (après réductions, ≥ 0). Rétro-compatible : une seule carte =
//!   comportement du lot 1. Les réductions et déclencheurs des cartes posées
//!   plus tôt s'appliquent aux poses suivantes (c'est le but de la séquence).
//! - `--probe-action "<nom>"` : pose la carte puis active son action UNE fois si
//!   elle est payable ; le delta isole L'ACTION SEULE (état après pose → après
//!   action, dépenses de l'action incluses).
//!
//! Chantier cartes-3 : les deux sondes rapportent en plus `resources` (les
//! cartes porteuses du joueur, celles à 0 comprises, triées par nom) et
//! `target_error` ; `ProbeScript` (`--probe-choice`, `--probe-target`) impose
//! les réponses de la POLITIQUE — pas des valeurs au moteur — via `ProbePolicy`,
//! qui délègue à `RandomPolicy` dès qu'une pile est épuisée. Sans script, le
//! comportement est celui du lot précédent, à l'identique.
//!
//! État de départ (prompt §Sonde) : joueur 1 sans corporation, 100 MC,
//! 20 chaleur, 20 plantes, productions 0, TR 5, paramètres globaux au départ,
//! les cartes nommées seules en main (dans l'ordre) ; pioche = cartes v1
//! restantes en ordre d'index ; tuiles océan NON mélangées.

use crate::cards::{CardsDb, Tag, JOKER_TAG_CHOICES};
use crate::flow::{
    apply_blue_action, apply_corp_action, build_card_with, card_discount, card_points,
    discard_mc_rate, next_card_discount,
    heat_reserved_by, install_corporation_with, payable, phase_production, plant_discount,
    plants_reserved_by, player_capacities, requirements_met, requirements_met_now,
    research_extra, resolve_hand_jokers, selector_bonus, spendable_mc_reserving, SelectorBonus,
};
use crate::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use crate::state::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Delta d'état après − avant. Pour `--probe`, cumulé et hors prix payés (le
/// prix réel est réintégré via `paid`) ; pour `--probe-action`, brut (les
/// dépenses de l'action sont comprises).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProbeDelta {
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    pub hand: i64,
    pub mc_prod: i64,
    pub heat_prod: i64,
    pub plant_prod: i64,
    pub card_prod: i64,
    pub tr: i64,
    pub temperature: i64,
    pub oxygen: i64,
    pub oceans: i64,
    pub forests: i64,
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Dernière carte de la séquence.
    pub card: String,
    pub found: bool,
    pub in_lot: bool,
    /// Prérequis de la dernière carte évalués sur l'état de DÉPART de la sonde
    /// (sens du lot 1 ; l'instantané de la sonde EST l'état de départ, donc
    /// c'est aussi la lecture « règle du jeu » corrigée par C1).
    pub prereq_ok: bool,
    /// (C1) Prérequis de la dernière carte évalués à l'état COURANT, juste
    /// avant sa pose. Diffère de `prereq_ok` quand les cartes précédentes de la
    /// séquence ont fait bouger un paramètre global.
    pub prereq_ok_now: bool,
    pub played: bool,
    pub delta: ProbeDelta,
    pub vp: i64,
    /// Prix effectivement payé de chaque carte posée, dans l'ordre (≥ 0).
    pub paid: Vec<i64>,
    /// (C3) Nombre de cartes défaussées pour payer chaque carte de la séquence,
    /// dans l'ordre. Lu sur le retour de `flow::build_card_with` — jamais
    /// recalculé par la sonde.
    pub discarded: Vec<i64>,
    /// (lot 3) Toutes les cartes PORTEUSES en jeu du joueur sondé après la
    /// séquence, celles à 0 comprises, triées par nom de carte. Lues sur
    /// `PlayerState::card_resources` — la sonde n'écrit jamais de ressource.
    pub resources: Vec<ProbeRes>,
    /// (lot 3) Première cible imposée par `--probe-target` qui ne figurait pas
    /// parmi les candidats (ou nom de carte inconnu). `None` = aucune erreur.
    /// Une cible imposée introuvable n'est JAMAIS remplacée en silence :
    /// l'effet est sauté et l'erreur remonte ici.
    pub target_error: Option<String>,
    /// (lot 4) La VRAIE phase IV de production a-t-elle été exécutée
    /// (`--probe-produce`) ? `false` = comportement des lots précédents, à
    /// l'identique.
    pub produced: bool,
    /// (lot 4) Ce que la PRODUCTION DÉRIVÉE a réellement crédité pendant cette
    /// phase : `(MC, chaleur, plantes)`. Relevé sur les compteurs d'audit
    /// incrémentés à l'endroit du crédit (`flow::phase_production`), jamais
    /// recalculé ici (NEVER 2). `(0,0,0)` sans `--probe-produce`.
    pub derived_prod: (i64, i64, i64),
    /// (lot 4) Somme de `flow::card_points` sur TOUTES les cartes en jeu du
    /// joueur 0 : points imprimés ET points variables (par badge, par carte
    /// jouée, par ressource posée). Lu sur `card_points`, jamais recalculé.
    /// Le champ `vp`, lui, ne rapporte que la dernière carte : il ne change pas.
    pub vp_total: i64,
    /// (corpo-1) Corporation imposée par `--probe-corp` ; `None` sans l'option
    /// (la sortie de la sonde est alors celle des lots précédents, à l'identique).
    pub corp: Option<ProbeCorp>,
    /// (lot acier-titane) Aciers du joueur sondé après la séquence. Lu sur
    /// `PlayerState::steel_capacity`, l'état que la partie réelle emploie pour
    /// calculer ses prix — la sonde ne recalcule rien (clause anti-shortcut 2).
    pub steel: i64,
    /// (lot acier-titane) Idem pour les titanes.
    pub titanium: i64,
    /// **(lot cartes-7) Bonus PERMANENT de phase Recherche du joueur sondé après
    /// la séquence** : `(cartes piochées en plus, cartes gardées en plus)`.
    ///
    /// C'est le résultat du service unique `flow::research_extra` — celui-là
    /// même que la phase V consomme (`flow::research_draw_keep`). La sonde ne
    /// recalcule rien : sans ce champ, le bonus de recherche n'était observable
    /// nulle part de l'extérieur (le contrat le mesure : `--probe
    /// "Interplanetary Relations"` rendait un delta entièrement nul).
    pub research: (usize, usize),
    /// **(Découverte) Les cartes Phase améliorées installées chez le joueur
    /// sondé**, étiquettes triées : `["1B", "5A"]`. Vide par défaut — sans
    /// `--probe-upgrade`, la sortie de la sonde est celle des lots précédents,
    /// à ce champ près.
    pub upgrades: Vec<String>,
    /// **(Découverte) Le bonus du sélectionneur de la phase `--probe-phase`**,
    /// rendu par le point de calcul UNIQUE (`flow::selector_bonus`) — la sonde
    /// ne le recalcule pas, elle le lit. Sans `--probe-phase`, il décrit la
    /// phase 0 : tout est à zéro.
    pub selector_bonus: SelectorBonus,
    /// **(jokers-corpos) Le badge effectivement retenu pour la DERNIÈRE carte de
    /// la séquence**, `None` si elle ne porte pas de badge joker (ou si le
    /// choix n'a pas eu lieu : carte introuvable, couche d'effets coupée).
    ///
    /// Lu sur `PlayerState::joker_tags`, l'état que la partie réelle emploie —
    /// la sonde ne rejoue aucune décision.
    pub joker_tag: Option<&'static str>,
}

/// (corpo-1) Corporation imposée à la sonde par `--probe-corp` : ce que le
/// moteur a réellement mis en place. `found: false` = nom inconnu de la pioche
/// chargée ; la sonde ne s'interrompt pas pour autant, elle rend `found: false`
/// et se déroule sans corporation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCorp {
    pub name: String,
    pub found: bool,
    /// La corporation porte-t-elle un effet dans la table `effects::CORPS` ?
    pub encoded: bool,
    /// MC de départ imprimé (DÉCLARÉ ; l'état de la sonde garde ses `--probe-mc`
    /// MC — voir journal D8).
    pub starting_mc: i64,
    /// Production de départ inscrite sur les pistes fixes du joueur sondé.
    pub start_prod: (i64, i64, i64),
    /// **(jokers-corpos) Les cartes Phase que la MISE EN PLACE de cette
    /// corporation a améliorées**, numéros de phase triés (`[2]` pour Apollo
    /// Industries). Liste VIDE pour les douze planches de la boîte de base.
    ///
    /// Mesuré par différence sur `PlayerState::phase_upgrades` avant/après
    /// l'installation : ce sont les cases que le moteur a réellement écrites, pas
    /// une relecture de la table.
    pub upgrades: Vec<u8>,
    /// **(jokers-corpos) La chaleur que la mise en place a APPORTÉE** — 2 pour
    /// Sultira (« y compris celui-ci »), 0 pour les autres.
    ///
    /// C'est un DELTA, et non la chaleur totale du joueur sondé : l'état de
    /// départ fixe de la sonde en porte déjà 20 en dur (`probe_state_base`), qui
    /// ne doivent rien à la corporation. Distincte de `start_prod.heat`, qui est
    /// une PRODUCTION répétée à chaque génération.
    pub start_heat: i64,
    /// **(jokers-corpos) Ce que rapporte au joueur une carte défaussée**, tel
    /// que le service unique `flow::discard_mc_rate` le rend après la mise en
    /// place : 3 MC du livret, 4 avec Exocorp.
    pub discard_rate: i64,
    /// **(jokers-corpos) La planche porte-t-elle une action activable ?**
    pub has_action: bool,
}

/// Une carte porteuse et son contenu (champ `resources` de la sonde).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRes {
    pub card: String,
    pub kind: &'static str,
    pub n: u32,
}

/// Options de la sonde (`--probe-mc`, `--probe-filler`, `--probe-strict`).
/// `ProbeOptions::default()` = comportement exact du lot 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOptions {
    /// MC de départ du joueur sondé (défaut 100).
    pub mc: i64,
    /// Cartes supplémentaires en main, prises en tête de pioche, servant
    /// uniquement de monnaie de défausse (défaut 0).
    pub filler: usize,
    /// **(lot cartes-7) Plantes de départ du joueur sondé** (`--probe-plants
    /// <n>`), sur le modèle exact de `--probe-mc`. Défaut 20 : la valeur que
    /// l'état de départ de la sonde portait en dur jusqu'ici, donc le
    /// comportement des lots précédents, bit à bit.
    ///
    /// Sans elle, *Restructured Resources* est improuvable de l'extérieur : la
    /// dépense d'une plante ne se voit que dans `delta.plants`, et il faut
    /// pouvoir faire varier la réserve pour la distinguer d'un effet de carte.
    pub plants: i64,
    /// La sonde cesse de forcer la pose : chaque carte n'est posée que si ses
    /// prérequis sont remplis SELON LA RÈGLE (C1 : paramètres sur l'instantané
    /// = l'état de départ pour la sonde ; tags et dépenses à l'état courant) ET
    /// si elle est payable (MC + défausse). Premier refus = arrêt de la
    /// séquence, `played` faux.
    pub strict: bool,
    /// (lot 6) **Phase choisie par le joueur sondé** (`--probe-phase <1..5>`),
    /// écrite dans l'état de départ AVANT la pose et avant l'action. `0` =
    /// aucune phase choisie, c'est-à-dire l'état des lots précédents à
    /// l'identique (`PlayerState::new` initialise déjà `chosen_phase` à 0).
    ///
    /// Elle n'écrit rien d'autre : tout ce qui en découle est ce que le moteur
    /// tire lui-même d'une phase choisie, par le même code qu'en partie réelle
    /// (bonus d'action du lot 6, et — avec `--probe-produce` — le bonus du
    /// sélectionneur de la phase IV).
    pub phase: u8,
    /// (Découverte) **Les cartes Phase améliorées installées chez le joueur
    /// sondé** (`--probe-upgrade <phase><variante>`, répétable et cumulable),
    /// écrites dans l'état de départ AVANT la séquence — là même où
    /// `flow::apply_phase_upgrade` les écrit en partie réelle.
    ///
    /// `[None; 5]` = aucune amélioration, c'est-à-dire l'état des lots
    /// précédents à l'identique. Une seconde installation sur la même phase
    /// REMPLACE la première : un joueur n'a jamais deux cartes Phase pour une
    /// même phase.
    pub upgrades: [Option<PhaseUpgrade>; 5],
    /// (decouverte-projets) **Objectif REVENDIQUÉ par le joueur sondé**
    /// (`--probe-objectif <nom>`), écrit dans l'état de départ AVANT la
    /// séquence — dans le slot `milestones[0]`, là même où
    /// `flow::assign_milestones` l'écrit en partie réelle, et avec le même
    /// drapeau `achieved_by[0]`.
    ///
    /// `None` = aucun Objectif, c'est-à-dire l'état des lots précédents à
    /// l'identique (les trois slots sont `Builder`, non revendiqués).
    /// L'adversaire n'en reçoit jamais : rien n'est partagé (NEVER 7).
    ///
    /// Sans cette option, le gain conditionnel et le prérequis « Objectif » de
    /// la boîte Découverte ne sont observables que dans un seul sens.
    pub objectif: Option<MilestoneKind>,
}

impl Default for ProbeOptions {
    fn default() -> ProbeOptions {
        ProbeOptions {
            mc: 100,
            filler: 0,
            strict: false,
            phase: 0,
            plants: 20,
            upgrades: [None; 5],
            objectif: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeActionResult {
    pub card: String,
    pub found: bool,
    pub in_lot: bool,
    pub has_action: bool,
    pub action_applied: bool,
    /// Delta isolant l'action seule (post-pose → post-action).
    pub delta: ProbeDelta,
    /// (lot 3) Cartes porteuses du joueur APRÈS pose + action (sans quoi les
    /// actions à ressources ne seraient pas observables). Triées par nom.
    pub resources: Vec<ProbeRes>,
    /// (lot 3) Voir `ProbeResult::target_error`.
    pub target_error: Option<String>,
    /// (corpo-1) Voir `ProbeResult::corp`.
    pub corp: Option<ProbeCorp>,
    /// (decouverte-projets) Les cartes Phase améliorées du joueur sondé APRÈS
    /// la pose ET l'activation, étiquettes triées (`["3A"]`). Même source que
    /// `ProbeResult::upgrades` — `PlayerState::phase_upgrade_labels()`, lue et
    /// jamais recalculée.
    ///
    /// Sans ce champ, une action qui améliore une carte Phase ne serait
    /// observable nulle part : `delta` ne porte que des ressources.
    pub upgrades: Vec<String>,
}

// ============================================================ script de sonde
//
// `--probe-choice` et `--probe-target` imposent les réponses de la POLITIQUE,
// pas des valeurs au moteur : la sonde emprunte donc exactement le même chemin
// de décision que `simulate`. Sans script, `ProbePolicy` se comporte
// strictement comme `RandomPolicy` — `--probe` sans option nouvelle est
// inchangé.

/// Réponses imposées à la politique pendant une sonde.
#[derive(Debug, Clone, Default)]
pub struct ProbeScript {
    /// Pile de réponses à `Policy::choose_option`, consommée dans l'ordre.
    pub choices: Vec<usize>,
    /// Pile de noms de cartes imposés à `Policy::choose_res_target` PUIS
    /// `Policy::choose_res_source`, consommée dans l'ordre d'appel.
    pub targets: Vec<String>,
    /// **(jokers-corpos) Badge imposé à `Policy::pick_joker_tag`**
    /// (`--probe-joker-tag <BADGE>`). `None` = la sonde choisit comme la
    /// politique de jeu ordinaire, elle ne plante pas.
    ///
    /// Il vaut pour TOUTES les cartes joker de la séquence : chacune reçoit son
    /// propre jeton, et deux cartes déclarées Terre comptent pour deux badges
    /// Terre. C'est une réponse imposée à la POLITIQUE, comme `choices` et
    /// `targets` — jamais une valeur écrite dans le moteur.
    pub joker_tag: Option<Tag>,
}

impl ProbeScript {
    pub fn is_empty(&self) -> bool {
        self.choices.is_empty() && self.targets.is_empty() && self.joker_tag.is_none()
    }
}

/// Politique de sonde : délègue tout à `RandomPolicy`, sauf les décisions
/// scriptées tant que la pile correspondante n'est pas épuisée.
struct ProbePolicy {
    inner: RandomPolicy,
    choices: Vec<usize>,
    ci: usize,
    /// Cibles imposées, résolues en identifiants de carte (`None` = nom inconnu
    /// de la base) — la résolution est celle du moteur (`CardsDb::resolve_card`,
    /// donc filtrée sur le deck v1, journal D1).
    targets: Vec<(String, Option<u16>)>,
    ti: usize,
    error: Option<String>,
    /// (jokers-corpos) Badge imposé à `pick_joker_tag`, pour toutes les cartes.
    joker_tag: Option<Tag>,
}

impl ProbePolicy {
    fn new(db: &CardsDb, script: &ProbeScript) -> ProbePolicy {
        ProbePolicy {
            inner: RandomPolicy,
            choices: script.choices.clone(),
            ci: 0,
            targets: script
                .targets
                .iter()
                .map(|n| (n.clone(), db.resolve_card(n)))
                .collect(),
            ti: 0,
            error: None,
            joker_tag: script.joker_tag,
        }
    }

    fn fail(&mut self, msg: String) {
        if self.error.is_none() {
            self.error = Some(msg);
        }
    }

    /// Traduit la prochaine cible imposée en indice dans `candidates`.
    /// `None` = pile épuisée (comportement par défaut). `Some(candidates.len())`
    /// = erreur : renoncement explicite, l'effet est sauté (journal D4).
    fn scripted_target(&mut self, candidates: &[u16]) -> Option<usize> {
        if self.ti >= self.targets.len() {
            return None;
        }
        let (name, id) = self.targets[self.ti].clone();
        self.ti += 1;
        match id {
            None => {
                self.fail(format!("cible imposée inconnue de la base : « {name} »"));
                Some(candidates.len())
            }
            Some(id) => match candidates.iter().position(|&c| c == id) {
                Some(i) => Some(i),
                None => {
                    self.fail(format!(
                        "cible imposée « {name} » absente des cartes pouvant recevoir la ressource"
                    ));
                    Some(candidates.len())
                }
            },
        }
    }
}

impl Policy for ProbePolicy {
    fn corp_mulligan(&mut self, rng: &mut StdRng, p: usize, corps: &[u16]) -> bool {
        self.inner.corp_mulligan(rng, p, corps)
    }
    fn project_mulligan(&mut self, rng: &mut StdRng, p: usize, hand: &[u16]) -> bool {
        self.inner.project_mulligan(rng, p, hand)
    }
    fn pick_corporation(&mut self, rng: &mut StdRng, p: usize, corps: &[u16]) -> usize {
        self.inner.pick_corporation(rng, p, corps)
    }
    fn pick_phase(&mut self, rng: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        self.inner.pick_phase(rng, p, allowed)
    }
    fn choose_build(&mut self, rng: &mut StdRng, p: usize, aff: &[usize]) -> Option<usize> {
        self.inner.choose_build(rng, p, aff)
    }
    fn construction_bonus(&mut self, rng: &mut StdRng, p: usize) -> ConstructionBonus {
        self.inner.construction_bonus(rng, p)
    }
    fn action_choice(&mut self, rng: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        self.inner.action_choice(rng, p, o)
    }
    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        p: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        self.inner.research_keep(rng, p, drawn, keep)
    }
    fn discard_down(&mut self, rng: &mut StdRng, p: usize, hand: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(rng, p, hand, n)
    }

    fn choose_option(&mut self, rng: &mut StdRng, p: usize, n: usize) -> usize {
        if self.ci < self.choices.len() {
            let c = self.choices[self.ci];
            self.ci += 1;
            c
        } else {
            self.inner.choose_option(rng, p, n)
        }
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, p: usize, cands: &[u16]) -> usize {
        match self.scripted_target(cands) {
            Some(i) => i,
            None => self.inner.choose_res_target(rng, p, cands),
        }
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, p: usize, cands: &[u16]) -> usize {
        match self.scripted_target(cands) {
            Some(i) => i,
            None => self.inner.choose_res_source(rng, p, cands),
        }
    }

    /// (jokers-corpos) `--probe-joker-tag` impose le badge, sans épuisement :
    /// toutes les cartes joker de la séquence reçoivent le même. Sans l'option,
    /// c'est l'heuristique ordinaire de la politique de jeu qui décide.
    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        p: usize,
        card: u16,
        tag_counts: &[u32],
    ) -> usize {
        match self.joker_tag {
            Some(t) => JOKER_TAG_CHOICES
                .iter()
                .position(|&x| x == t)
                .expect("badge imposé hors des dix choix — refusé par la CLI"),
            None => self.inner.pick_joker_tag(rng, p, card, tag_counts),
        }
    }
}

/// Champ `resources` : toutes les cartes porteuses du joueur 0, triées par nom.
fn probe_resources(game: &GameState, db: &CardsDb) -> Vec<ProbeRes> {
    let mut out: Vec<ProbeRes> = game.players[0]
        .card_resources
        .iter()
        .map(|(&c, &n)| ProbeRes {
            card: db.projects[c as usize].name.clone(),
            kind: db.projects[c as usize]
                .holds()
                .expect("carte sans type de ressource dans card_resources")
                .name(),
            n,
        })
        .collect();
    out.sort_by(|a, b| a.card.cmp(&b.card));
    out
}

/// Points de victoire venant des RESSOURCES posées, toutes cartes en jeu du
/// joueur 0 — lus sur `flow::card_points`, jamais recalculés ici (NEVER 3).
fn probe_resource_vp(game: &GameState, db: &CardsDb) -> i64 {
    let pl = &game.players[0];
    pl.played
        .iter()
        .map(|&c| card_points(db, pl, c).1)
        .sum()
}

/// Construit l'état de départ fixe de la sonde, `ids` en main du joueur 0 dans
/// l'ordre donné (le reste de la pioche v1 en ordre d'index croissant), avec
/// `opts.mc` MC et `opts.filler` cartes de remplissage prises en tête de pioche
/// (elles sont AJOUTÉES APRÈS les cartes de la séquence : celle-ci reste en
/// tête de main, la pose se fait toujours à l'indice 0).
/// (corpo-1) Même état de départ, corporation imposée en plus. Renvoie l'état,
/// le descriptif de la corporation (`None` si `--probe-corp` n'est pas donné) et
/// la TAILLE DE MAIN D'AVANT l'installation — c'est elle qui sert de base au
/// `delta.hand`, pour que la pioche de départ d'une corporation (Inventrix : 3
/// cartes) apparaisse dans le delta sans changer la convention des lots
/// précédents (journal D9).
///
/// La corporation est installée par le service unique `flow::install_corporation`
/// (badges, production de départ, pioche de départ), PUIS le MC du joueur sondé
/// est ramené à `opts.mc` : l'état de départ fixe de la sonde reste son contrat,
/// et `corp.starting_mc` rapporte la valeur imprimée sans l'appliquer (D8).
fn probe_state_corp(
    db: &CardsDb,
    ids: &[u16],
    opts: ProbeOptions,
    corp_name: Option<&str>,
    script: &ProbeScript,
) -> (GameState, Option<ProbeCorp>, usize) {
    let mut game = probe_state_base(db, ids, opts);
    let hand_before = game.players[0].hand.len();
    let Some(name) = corp_name else {
        return (game, None, hand_before);
    };
    let Some(cid) = db.corporations.iter().position(|c| c.name == name) else {
        let rate = discard_mc_rate(db, &game.players[0]);
        return (
            game,
            Some(ProbeCorp {
                name: name.to_string(),
                found: false,
                encoded: false,
                starting_mc: 0,
                start_prod: (0, 0, 0),
                upgrades: Vec::new(),
                start_heat: 0,
                discard_rate: rate,
                has_action: false,
            }),
            hand_before,
        );
    };
    // (jokers-corpos) Ce que la MISE EN PLACE change, relevé avant/après : les
    // cartes Phase qu'elle améliore et la chaleur qu'elle apporte. Mesuré sur
    // l'état, jamais relu dans la table.
    let upg_before = game.players[0].phase_upgrades;
    let heat_before = game.players[0].heat;
    // La mise en place emprunte le chemin unique du moteur, avec la POLITIQUE de
    // sonde : « Améliorez votre carte Phase n » laisse le choix de la variante
    // (A ou B), scriptable par `--probe-choice` comme partout ailleurs.
    //
    // ATTENTION, convention à connaître : cette politique-ci est PROPRE à la
    // mise en place. La séquence de cartes en construit une seconde, qui repart
    // du DÉBUT de la pile `--probe-choice`. Les deux étapes ne se partagent donc
    // pas la pile — c'est ce qui permet de scripter la séquence sans avoir à
    // compter d'abord les choix consommés par la corporation.
    let mut pol = ProbePolicy::new(db, script);
    install_corporation_with(&mut game, db, 0, cid as u16, &mut pol);
    let upgrades: Vec<u8> = (1u8..=5)
        .filter(|&ph| {
            game.players[0].phase_upgrade(ph) != upg_before[ph as usize - 1]
        })
        .collect();
    let start_heat = game.players[0].heat - heat_before;
    game.players[0].mc = opts.mc;
    let corp = &db.corporations[cid];
    let sp = corp
        .effect
        .filter(|_| db.effects_on)
        .map(|s| s.start_prod)
        .unwrap_or_default();
    let has_action = corp
        .effect
        .filter(|_| db.effects_on)
        .and_then(|s| s.action)
        .is_some();
    // Le SERVICE UNIQUE appliqué au joueur sondé, lu et non recalculé.
    let discard_rate = discard_mc_rate(db, &game.players[0]);
    (
        game,
        Some(ProbeCorp {
            name: corp.name.clone(),
            found: true,
            encoded: corp.effect.is_some(),
            starting_mc: corp.starting_mc,
            start_prod: (sp.mc, sp.heat, sp.plants),
            upgrades,
            start_heat,
            discard_rate,
            has_action,
        }),
        hand_before,
    )
}

fn probe_state_base(db: &CardsDb, ids: &[u16], opts: ProbeOptions) -> GameState {
    let mut deck: Vec<u16> = (0..db.projects.len() as u16)
        .filter(|&c| !ids.contains(&c) && db.projects[c as usize].in_deck)
        .collect();
    let mut players = [PlayerState::new(), PlayerState::new()];
    players[0].mc = opts.mc;
    // (lot 6) `--probe-phase` : la phase choisie par le JOUEUR SONDÉ, écrite
    // là où la planification l'écrit en partie réelle. Le joueur 1 n'en reçoit
    // aucune (0) — le bonus ne doit jamais dépendre de la phase de l'adversaire.
    players[0].chosen_phase = opts.phase;
    // (Découverte) Les cartes Phase améliorées du joueur sondé, écrites là où la
    // partie réelle les écrit. Le joueur 1 n'en reçoit aucune : rien n'est
    // partagé entre les deux joueurs (NEVER 7).
    players[0].phase_upgrades = opts.upgrades;
    players[0].heat = 20;
    players[0].plants = opts.plants;
    players[0].hand.extend_from_slice(ids);
    // Monnaie de défausse : le dessus de la pioche (= fin du Vec, comme
    // `flow::draw_card`).
    for _ in 0..opts.filler {
        match deck.pop() {
            Some(c) => players[0].hand.push(c),
            None => break,
        }
    }

    let mut game = GameState {
        rng: StdRng::seed_from_u64(0),
        deck,
        discard: Vec::new(),
        corp_deck: (0..db.corporations.len() as u16).collect(),
        corp_discard: Vec::new(),
        oceans: OCEAN_TILES,
        oceans_revealed: 0,
        temperature: 0,
        oxygen: 0,
        infrastructure: 0,
        players,
        generation: 1,
        // (decouverte-projets) `--probe-objectif` : l'Objectif demandé est
        // posé dans le premier slot et marqué revendiqué PAR LE JOUEUR SONDÉ —
        // exactement l'écriture que `flow::assign_milestones` produit en partie
        // réelle quand le joueur atteint le seuil. Sans l'option, les trois
        // slots sont ceux des lots précédents, bit à bit.
        milestones: {
            let mut m = [MilestoneSlot {
                kind: MilestoneKind::Builder,
                achieved_by: [false; NUM_PLAYERS],
            }; 3];
            if let Some(k) = opts.objectif {
                m[0].kind = k;
                m[0].achieved_by[0] = true;
            }
            m
        },
        awards: [AwardKind::Celebrity; 3],
        game_over: false,
        blue_actions: 0,
        snap_temperature: 0,
        snap_oxygen: 0,
        snap_oceans: 0,
        snap_infrastructure: 0,
        first_player: 0,
        turn_order: Vec::new(),
        prereq_snapshot_blocks: 0,
        draw_before_build: 0,
        draw_after_build: 0,
        discard_payments: 0,
        res_added: 0,
        res_removed: 0,
        res_targets_missing: 0,
        phase_upgrades_skipped: 0,
        phase_upgrades_granted: 0,
        phase_upgrades_reupgraded: 0,
        upgraded_bonus_applied: 0,
        phase_upgrades_targeted: 0,
        phase_upgrades_by_action: 0,
        upgraded_reveal_bonuses: 0,
        objective_condition_hits: 0,
        draw_then_discard_uses: 0,
        upgraded_extra_builds: 0,
        cards_effects_unhandled: 0,
        derived_mc: 0,
        derived_heat: 0,
        derived_plants: 0,
        tr_from_tags: 0,
        research_extra_draws: 0,
        extra_builds_granted: 0,
        extra_builds_used: 0,
        free_builds: 0,
        next_card_mods_armed: 0,
        next_card_mods_used: 0,
        corp_heat_as_mc: 0,
        corp_forest_rebates: 0,
        corp_tr_boosts: 0,
        corp_trigger_tr: 0,
        action_phase_bonuses: 0,
        action_discard_costs: 0,
        draw_discard_discards: 0,
        cards_revealed: 0,
        standard_action_discounts: 0,
        action_mc_bonuses: 0,
        joker_tag_choices: 0,
        joker_tag_hits: 0,
        corp_phase_upgrades_at_setup: 0,
        discard_bonus_mc: 0,
        action_phase_self_bonus: 0,
    };
    game.snapshot_planet();
    game
}

/// Résolution d'un nom de carte : chemin unique du moteur
/// (`CardsDb::resolve_card`), qui écarte les variantes « Buffed » hors deck v1
/// portant le même nom (journal D1).
fn resolve(db: &CardsDb, name: &str) -> Option<u16> {
    db.resolve_card(name)
}

/// Capture des champs suivis pour le delta (ordre : mc, heat, plants, mc_prod,
/// heat_prod, plant_prod, card_prod, tr, forests, temperature, oxygen, oceans).
fn snap(game: &GameState) -> [i64; 12] {
    let pl = &game.players[0];
    [
        pl.mc,
        pl.heat,
        pl.plants,
        pl.mc_prod,
        pl.heat_prod,
        pl.plant_prod,
        pl.card_prod,
        pl.tr,
        pl.forests,
        game.temperature as i64,
        game.oxygen as i64,
        game.oceans_revealed as i64,
    ]
}

fn make_delta(before: &[i64; 12], after: &[i64; 12], hand: i64, mc_extra: i64) -> ProbeDelta {
    ProbeDelta {
        mc: after[0] - before[0] + mc_extra,
        heat: after[1] - before[1],
        plants: after[2] - before[2],
        hand,
        mc_prod: after[3] - before[3],
        heat_prod: after[4] - before[4],
        plant_prod: after[5] - before[5],
        card_prod: after[6] - before[6],
        tr: after[7] - before[7],
        temperature: after[9] - before[9],
        oxygen: after[10] - before[10],
        oceans: after[11] - before[11],
        forests: after[8] - before[8],
    }
}

/// Sonde simple (rétro-compatible) : une carte.
pub fn run_probe(db: &CardsDb, name: &str) -> ProbeResult {
    run_probe_seq(db, &[name])
}

/// Sonde séquence : pose forcée des cartes DANS L'ORDRE, delta cumulé, `paid[]`.
/// Comportement du lot 2, options par défaut.
pub fn run_probe_seq(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_opts(db, names, ProbeOptions::default())
}

/// Sonde séquence complète (lot 3 conformité). `opts` par défaut = lot 2.
///
/// La sonde ne réimplémente aucune règle : elle passe par `requirements_met`
/// (prérequis, C1), `flow::payable` (affordabilité, C3) et
/// `flow::build_card_with` (pose + paiement, C3) — le chemin de `simulate`.
pub fn run_probe_seq_opts(db: &CardsDb, names: &[&str], opts: ProbeOptions) -> ProbeResult {
    run_probe_seq_scripted(db, names, opts, &ProbeScript::default())
}

/// Sonde séquence scriptée (lot 3 ressources) : `script` impose les réponses de
/// la politique (`--probe-choice`, `--probe-target`). Script vide = comportement
/// strictement identique au lot 2.
pub fn run_probe_seq_scripted(
    db: &CardsDb,
    names: &[&str],
    opts: ProbeOptions,
    script: &ProbeScript,
) -> ProbeResult {
    run_probe_seq_full(db, names, opts, script, false)
}

/// Sonde séquence complète du lot 4 (sans corporation imposée). Façade
/// conservée : `--probe-corp` absent = comportement des lots précédents.
pub fn run_probe_seq_full(
    db: &CardsDb,
    names: &[&str],
    opts: ProbeOptions,
    script: &ProbeScript,
    produce: bool,
) -> ProbeResult {
    run_probe_seq_corp(db, names, opts, script, produce, None)
}

/// Sonde séquence complète du lot 4 : `produce = true` (`--probe-produce`)
/// exécute, APRÈS la séquence, la VRAIE phase IV du moteur
/// (`flow::phase_production`) — ni copie ni calcul parallèle.
///
/// `produce` n'appartient pas à `ProbeOptions` : celle-ci décrit l'ÉTAT DE
/// DÉPART de la sonde (MC, monnaie de défausse, mode strict), alors qu'il
/// s'agit ici d'une action jouée après la pose. `produce = false` reproduit à
/// l'identique le comportement des lots précédents.
/// (corpo-1) `corp` = nom imposé par `--probe-corp`. La corporation est mise en
/// place AVANT tout — donc avant l'évaluation des prérequis, sans quoi le palier
/// de couleur ±1 d'Inventrix serait invisible (journal D9).
///
/// La séquence de cartes peut être VIDE (`--probe-corp` employé sans `--probe`,
/// avec `--probe-produce`) : la sonde met alors la corporation en place, exécute
/// la phase de production, et rend `card: ""`, `found: false`, `played: false`.
pub fn run_probe_seq_corp(
    db: &CardsDb,
    names: &[&str],
    opts: ProbeOptions,
    script: &ProbeScript,
    produce: bool,
    corp_name: Option<&str>,
) -> ProbeResult {
    // `names` VIDE = `--probe-corp` employé sans `--probe`. C'est le seul cas
    // nouveau : toute séquence non vide, y compris `["Grass", ""]` que produit
    // `--probe "Grass;"`, suit exactement le chemin des lots précédents (nom
    // final irrésolu → `found:false`, `paid:[]`). La CLI ne fabrique jamais de
    // nom vide pour signaler l'absence de séquence : elle passe une tranche vide.
    let last = *names.last().unwrap_or(&"");

    // Séquence vide (`--probe-corp` seul) : rien à résoudre, la sonde continue.
    // Le test porte sur la TRANCHE, pas sur le nom : `--probe "Grass;"` a bien
    // une séquence (dont le dernier nom est vide) et doit rendre `found:false`,
    // `paid:[]`, comme aux lots précédents.
    let last_id: Option<u16> = if names.is_empty() {
        None
    } else {
        match resolve(db, last) {
            Some(id) => Some(id),
            None => {
                // Dernière carte introuvable : résultat « non trouvée »
                // (comportement du lot 1, inchangé).
                let (g, corp, _) = probe_state_corp(db, &[], opts, corp_name, script);
                let caps = player_capacities(&g.players[0]);
                return ProbeResult {
                    card: last.to_string(),
                    found: false,
                    in_lot: false,
                    prereq_ok: false,
                    prereq_ok_now: false,
                    played: false,
                    delta: ProbeDelta::default(),
                    vp: 0,
                    paid: Vec::new(),
                    discarded: Vec::new(),
                    resources: Vec::new(),
                    target_error: None,
                    produced: false,
                    derived_prod: (0, 0, 0),
                    vp_total: 0,
                    corp,
                    // La corporation est en place même quand la carte est
                    // introuvable : son savoir-faire compte déjà.
                    steel: caps.steel,
                    titanium: caps.titanium,
                    // (lot cartes-7) La corporation est en place : son bonus de
                    // recherche compte déjà (Tharsis Republic).
                    research: research_extra(db, &g.players[0]),
                    // (Découverte) L'état de départ porte déjà les améliorations
                    // demandées, carte trouvée ou non.
                    upgrades: g.players[0].phase_upgrade_labels(),
                    selector_bonus: selector_bonus(db, &g.players[0], opts.phase),
                    // Aucune carte trouvée : aucun badge joker retenu.
                    joker_tag: None,
                };
            }
        }
    };

    // Cartes résolues, dans l'ordre (les noms inconnus sont ignorés à la pose).
    let ids: Vec<u16> = names.iter().filter_map(|n| resolve(db, n)).collect();

    let (mut game, corp, hand_before_corp) = probe_state_corp(db, &ids, opts, corp_name, script);
    // (lot cartes-7, journal D2) Prérequis de la DERNIÈRE carte, relevés JUSTE
    // AVANT SA POSE — c'est-à-dire une fois les cartes qui la précèdent dans la
    // séquence entrées en jeu. Sans cela, un assouplissement porté par une
    // CARTE (*Adaptation Technology*) serait structurellement invisible : celui
    // d'*Inventrix* ne se voyait que parce qu'une corporation, elle, est
    // installée avant l'instantané.
    //
    // La différence de fond avec `prereq_ok_now` est conservée : les prérequis
    // de PARAMÈTRES restent jugés sur l'INSTANTANÉ (`requirements_met`), les
    // autres à l'état courant. Sur une sonde à UNE SEULE carte — le cas de
    // toutes les références de non-régression — l'état est rigoureusement celui
    // du départ : la valeur ne change pas d'un bit.
    //
    // Valeur de repli si la séquence s'arrête avant d'atteindre la dernière
    // carte : l'état de départ, comme aux lots précédents.
    let mut prereq_ok = last_id.map_or(false, |id| requirements_met(&game, db, 0, id));
    // Évalué juste avant la pose de la dernière carte ; valeur de départ si la
    // séquence s'arrête avant d'y arriver.
    let mut prereq_ok_now = last_id.map_or(false, |id| requirements_met_now(&game, db, 0, id));

    let n = ids.len();
    // Base du delta de main : sans monnaie de défausse, convention du lot 1/2
    // (« delta.hand exclut la carte jouée ») ; avec `--probe-filler`, la main
    // initiale COMPLÈTE, pour que le delta compte tout ce qui quitte la main
    // (cartes posées + cartes défaussées pour payer). Voir journal §Decision Log.
    // (corpo-1) `hand_before_corp` = main d'AVANT la mise en place de la
    // corporation : sans `--probe-corp` c'est exactement `hand.len()`, la
    // convention des lots précédents est donc bit à bit conservée ; avec, la
    // pioche de départ (Inventrix, 3 cartes) devient visible dans `delta.hand`.
    let hand0 = if opts.filler > 0 {
        hand_before_corp as i64
    } else {
        (hand_before_corp - n) as i64
    };
    let before = snap(&game);

    // Pose de chaque carte dans l'ordre (toujours à l'indice 0 de la main : les
    // cartes de la séquence sont en tête, monnaie et pioches s'ajoutent en fin).
    let mut pol = ProbePolicy::new(db, script);
    // (jokers-corpos) Les jetons Badge sont posés sur TOUTES les cartes joker de
    // la main AVANT la première pose — donc avant le garde-fou de payabilité
    // ci-dessous et avant tout calcul de prix. C'est exactement ce que fait la
    // partie réelle avant son énumération d'abordabilité, par la même fonction :
    // sans cela, la sonde jugerait une carte joker sur son prix NU et refuserait
    // de poser ce que le jeu, lui, propose.
    resolve_hand_jokers(&mut game, db, 0, &mut pol);
    let mut paid = Vec::with_capacity(n);
    let mut discarded = Vec::with_capacity(n);
    for (k, &id) in ids.iter().enumerate() {
        let price = db.projects[id as usize].price;
        // Prix RAPPORTÉ dans `paid` : prix imprimé moins les réductions FIXES,
        // convention des lots précédents, conservée bit à bit. Il ne tient pas
        // compte des réductions PAYANTES (microbes du lot 3, plantes de ce
        // lot-ci) : celles-ci dépendent d'une décision du joueur, et les
        // rabattre ici ferait mentir `paid` dans l'autre sens dès que le joueur
        // y renonce. Le témoin de ces réductions reste `delta`.
        // (lot cartes-8) La réduction ARMÉE pour la prochaine carte de la phase
        // (*Work Crews*) entre ici comme une réduction fixe : elle ne dépend
        // d'aucune décision du joueur — elle est déjà acquise et sera consommée
        // par cette pose-ci, que le joueur le veuille ou non. La sonde doit donc
        // la voir, sans quoi `paid` mentirait et le garde-fou de payabilité
        // refuserait une carte que la partie réelle propose (I2).
        let disc = card_discount(&game, db, 0, id) + next_card_discount(&game.players[0]);
        let cost = (price - disc).max(0);
        // (lot cartes-7) Prix jugé par le GARDE-FOU de payabilité : celui-ci
        // doit voir ce que `flow::affordable` voit, réduction payable comprise,
        // sinon la sonde refuserait de poser une carte que la partie réelle
        // propose — et la dépense de plante serait indémontrable au budget
        // serré. Rien n'est accordé gratuitement : si la carte n'est payable
        // qu'avec la réduction, `build_card_with` la rend obligatoire (la
        // branche « renoncer » n'y est alors pas jouable).
        let cost_min =
            (cost - plant_discount(&game, db, 0, id).map_or(0, |(_, a)| a)).max(0);
        if k + 1 == n {
            prereq_ok = requirements_met(&game, db, 0, id);
            prereq_ok_now = requirements_met_now(&game, db, 0, id);
        }
        // Prérequis : vérifiés seulement en mode strict (sinon la pose est
        // forcée, comportement du lot 2).
        if opts.strict && !requirements_met(&game, db, 0, id) {
            break;
        }
        // Payabilité : vérifiée dans les DEUX modes — le mode forcé ignore les
        // prérequis, pas le paiement (sans quoi `build_card_with` casserait sur
        // un état volontairement impayable via `--probe-mc`).
        // (corpo-1) Helion : la chaleur compte dans l'affordabilité, par le
        // service unique du moteur.
        if !payable(
            spendable_mc_reserving(db, &game.players[0], heat_reserved_by(db, id)),
            game.players[0].hand.len(),
            cost_min,
            discard_mc_rate(db, &game.players[0]),
        ) {
            break;
        }
        // (lot cartes-7) Une dépense de POSE en plantes (« Requires you to spend
        // N plants ») est un PAIEMENT, pas un prérequis de paramètre : la sonde
        // ne la force donc jamais, exactement comme elle ne force pas le
        // paiement en MC. Sans cette garde, `--probe-plants 0` sur une carte à
        // dépense de plantes ferait sauter l'assertion d'`apply_card_effects` —
        // un plantage, pas un résultat lisible.
        //
        // Aucun effet sur les lots précédents : la réserve de départ valait 20
        // en dur, et aucune carte n'exige plus de 2 plantes.
        if game.players[0].plants < plants_reserved_by(db, id) {
            break;
        }
        paid.push(cost);
        discarded.push(build_card_with(&mut game, db, 0, 0, 0, &mut pol) as i64);
    }

    // (lot 4) `--probe-produce` : la VRAIE phase IV du moteur, pas une copie.
    // Elle traite les deux joueurs — le joueur 1 de l'état de sonde n'a aucune
    // carte en jeu, donc aucune production dérivée : la variation des compteurs
    // d'audit est bien celle du joueur 0, relevée à l'endroit du crédit.
    let mut derived_prod = (0i64, 0i64, 0i64);
    if produce {
        let before_counters = (game.derived_mc, game.derived_heat, game.derived_plants);
        phase_production(&mut game, db, &mut pol);
        derived_prod = (
            (game.derived_mc - before_counters.0) as i64,
            (game.derived_heat - before_counters.1) as i64,
            (game.derived_plants - before_counters.2) as i64,
        );
    }

    let after = snap(&game);
    let total_paid: i64 = paid.iter().sum();
    let played = last_id.map_or(false, |id| game.players[0].played.contains(&id));
    let last_card = last_id.map(|id| &db.projects[id as usize]);
    let hand_delta = game.players[0].hand.len() as i64 - hand0;

    ProbeResult {
        card: last.to_string(),
        found: last_id.is_some(),
        in_lot: db.effects_on && last_card.map_or(false, |c| c.effect.is_some()),
        prereq_ok,
        prereq_ok_now,
        played,
        delta: make_delta(&before, &after, hand_delta, total_paid),
        // VP fixes de la dernière carte (sens du lot 1) + points de victoire
        // venant des RESSOURCES posées sur toutes les cartes en jeu — c'est ce
        // que le lot 3 rend observable (journal D6). Les VP dynamiques non liés
        // aux ressources (JUPITER, BLUE_CARD…) restent hors de ce champ.
        vp: last_card.map_or(0, |c| c.vp) + probe_resource_vp(&game, db),
        paid,
        discarded,
        resources: probe_resources(&game, db),
        target_error: pol.error.clone(),
        produced: produce,
        derived_prod,
        // Points de victoire de TOUTES les cartes en jeu, lus sur `card_points`.
        vp_total: game.players[0]
            .played
            .iter()
            .map(|&c| card_points(db, &game.players[0], c).0)
            .sum(),
        corp,
        // (lot acier-titane) Le compte tel que l'état du joueur le porte —
        // celui-là même que `flow::card_discount` lit pour fixer les prix.
        steel: player_capacities(&game.players[0]).steel,
        titanium: player_capacities(&game.players[0]).titanium,
        // (lot cartes-7) Le SERVICE UNIQUE appliqué au joueur sondé après la
        // pose — celui-là même que consomme la phase V. La sonde ne fait que le
        // lire (clause anti-shortcut n° 1).
        research: research_extra(db, &game.players[0]),
        upgrades: game.players[0].phase_upgrade_labels(),
        // Le point de calcul unique appliqué au joueur sondé : c'est LA valeur
        // que la phase réelle lirait pour lui (clause anti-shortcut n° 1).
        selector_bonus: selector_bonus(db, &game.players[0], opts.phase),
        // (jokers-corpos) Le jeton posé sur la DERNIÈRE carte de la séquence,
        // lu sur l'état du joueur sondé. `None` pour une carte sans badge
        // joker — c'est le contre-témoin du contrôle 02.
        joker_tag: last_id
            .and_then(|id| game.players[0].joker_tag(id))
            .map(|t| t.as_str()),
    }
}

/// Sonde action : pose la carte puis active son action une fois si payable ;
/// le delta isole l'action (état après pose → après action).
pub fn run_probe_action(db: &CardsDb, name: &str) -> ProbeActionResult {
    run_probe_action_scripted(db, name, &ProbeScript::default())
}

/// Sonde action scriptée (lot 3) : le script s'applique à la POSE puis à
/// l'ACTION, dans cet ordre. Script vide = comportement du lot 2.
pub fn run_probe_action_scripted(
    db: &CardsDb,
    name: &str,
    script: &ProbeScript,
) -> ProbeActionResult {
    run_probe_action_corp(db, name, script, None)
}

/// (corpo-1) Sonde action avec corporation imposée (`--probe-corp`), options par
/// défaut. Façade conservée : comportement des lots précédents à l'identique.
pub fn run_probe_action_corp(
    db: &CardsDb,
    name: &str,
    script: &ProbeScript,
    corp_name: Option<&str>,
) -> ProbeActionResult {
    run_probe_action_opts(db, name, script, corp_name, ProbeOptions::default())
}

/// (lot 6) Sonde action complète : `opts` décrit l'ÉTAT DE DÉPART du joueur
/// sondé — dont `phase` (`--probe-phase`), la phase qu'il a choisie ce tour, et
/// `filler`, la monnaie de main sans laquelle une action qui se paie en cartes
/// ne serait pas observable. `ProbeOptions::default()` =
/// comportement des lots précédents, bit à bit.
pub fn run_probe_action_opts(
    db: &CardsDb,
    name: &str,
    script: &ProbeScript,
    corp_name: Option<&str>,
    opts: ProbeOptions,
) -> ProbeActionResult {
    run_probe_action_seq(db, &[name], script, corp_name, opts)
}

/// **(lot acier-titane) Sonde action sur une SÉQUENCE** — `--probe-action
/// "Carte A;Carte B"`.
///
/// Sans elle, rien ne peut prouver « 2 MC par acier » au-delà d'un seul acier :
/// l'action mesurée ne pouvait jamais avoir plus d'un savoir-faire devant elle.
///
/// Elle pose TOUTES les cartes de la séquence, dans l'ordre, exactement comme
/// `--probe` (même chemin `flow::build_card_with`, toujours à l'indice 0 de la
/// main), prend l'instantané de référence APRÈS la dernière pose, puis applique
/// l'action de la DERNIÈRE carte, et d'elle seule.
///
/// **Un seul nom = comportement strictement inchangé** : la boucle ne fait alors
/// qu'un tour, sur la même carte, avec le même état de départ et le même
/// instantané qu'avant ce lot.
pub fn run_probe_action_seq(
    db: &CardsDb,
    names: &[&str],
    script: &ProbeScript,
    corp_name: Option<&str>,
    opts: ProbeOptions,
) -> ProbeActionResult {
    run_probe_action_target(db, names, script, corp_name, opts, None)
}

/// **(jokers-corpos) Ce dont on active l'action.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cible {
    /// Une carte bleue en jeu.
    Carte(u16),
    /// La planche de corporation du joueur sondé.
    Corpo,
}

/// **(jokers-corpos) Sonde action à CIBLE explicite** — `--probe-action <nom>`
/// accepte désormais un nom de CORPORATION en plus d'un nom de carte.
///
/// `target = None` : la cible est la dernière carte de `names`, c'est-à-dire le
/// comportement des lots précédents, bit à bit. `target = Some(nom)` : `names`
/// est la séquence POSÉE (elle vient de `--probe`), et `nom` désigne ce dont on
/// active l'action — une carte de la séquence, ou la corporation installée.
///
/// Un nom qui ne désigne ni une carte de la base ni une corporation de la pioche
/// chargée reste REFUSÉ : `found: false`, rien n'est activé.
pub fn run_probe_action_target(
    db: &CardsDb,
    names: &[&str],
    script: &ProbeScript,
    corp_name: Option<&str>,
    opts: ProbeOptions,
    target: Option<&str>,
) -> ProbeActionResult {
    let name = target.unwrap_or(*names.last().unwrap_or(&""));
    // Toutes les cartes de la séquence, dans l'ordre. Les noms inconnus sont
    // ignorés à la pose, comme dans `run_probe_seq_corp` ; seule la CIBLE décide
    // de `found` — c'est elle que l'on mesure.
    let ids: Vec<u16> = names.iter().filter_map(|n| resolve(db, n)).collect();
    // La cible : une carte d'abord (chemin des lots précédents), une corporation
    // de la pioche chargée ensuite.
    let cible = match resolve(db, name) {
        Some(id) => Some(Cible::Carte(id)),
        None if db.corporations.iter().any(|c| c.name == name) => Some(Cible::Corpo),
        None => None,
    };
    let Some(cible) = cible else {
        let (_, corp, _) = probe_state_corp(db, &[], opts, corp_name, script);
        return ProbeActionResult {
            card: name.to_string(),
            found: false,
            in_lot: false,
            has_action: false,
            action_applied: false,
            delta: ProbeDelta::default(),
            resources: Vec::new(),
            target_error: None,
            corp,
            upgrades: Vec::new(),
        };
    };

    let (mut game, corp, _) = probe_state_corp(db, &ids, opts, corp_name, script);
    // L'action visée est-elle celle d'une planche RÉELLEMENT installée chez le
    // joueur sondé ? Une corporation nommée sans `--probe-corp` n'est pas en jeu :
    // elle ne porte alors aucune action activable.
    let corpo_en_place = matches!(cible, Cible::Corpo)
        && game.players[0]
            .corporation
            .is_some_and(|c| db.corporations[c as usize].name == name);
    let (in_lot, has_action) = match cible {
        Cible::Carte(card_id) => {
            let card = &db.projects[card_id as usize];
            let in_lot = db.effects_on && card.effect.is_some();
            (in_lot, in_lot && card.effect.and_then(|e| e.action).is_some())
        }
        Cible::Corpo => {
            let spec = if corpo_en_place {
                crate::flow::corp_effects(db, &game.players[0])
            } else {
                None
            };
            (spec.is_some(), spec.and_then(|s| s.action).is_some())
        }
    };
    // Pose (état de référence du delta d'action) — même chemin que `simulate`,
    // avec la politique de sonde (identique à RandomPolicy si le script est
    // vide). Les cartes de la séquence sont en tête de main, dans l'ordre : la
    // pose se fait toujours à l'indice 0, comme pour `--probe`.
    let mut pol = ProbePolicy::new(db, script);
    // (jokers-corpos) Les jetons Badge sont posés AVANT la boucle de pose, donc
    // avant le garde-fou de payabilité ci-dessous — exactement comme dans
    // `run_probe_seq_corp`. Sans cet appel, ce garde-fou jugerait une carte à
    // badge joker sur son prix NU alors que `build_card_with`, lui, la poserait
    // au prix réduit : l'affordabilité et le paiement divergeraient dans ce
    // chemin de sonde (I2). Défaut trouvé en relecture adversariale.
    resolve_hand_jokers(&mut game, db, 0, &mut pol);
    for &id in &ids {
        // Payabilité, comme dans `run_probe_seq_corp` : la pose est forcée
        // quant aux PRÉREQUIS, jamais quant au PAIEMENT. Sans ce test,
        // `build_card_with` casse sur un état volontairement impayable
        // (`--probe-mc` bas) au lieu de rendre un résultat lisible. Une
        // séquence interrompue laisse la dernière carte hors jeu : son action
        // n'est alors pas appliquée (voir plus bas).
        // Même garde-fou que `run_probe_seq_corp` : le prix MINIMUM que le
        // joueur peut avoir à payer, réduction payable en plantes comprise.
        let cost_min = (db.projects[id as usize].price
            - card_discount(&game, db, 0, id)
            - plant_discount(&game, db, 0, id).map_or(0, |(_, a)| a))
        .max(0);
        if !payable(
            spendable_mc_reserving(db, &game.players[0], heat_reserved_by(db, id)),
            game.players[0].hand.len(),
            cost_min,
            discard_mc_rate(db, &game.players[0]),
        ) {
            break;
        }
        // (lot cartes-7) Une dépense de POSE en plantes (« Requires you to spend
        // N plants ») est un PAIEMENT, pas un prérequis de paramètre : la sonde
        // ne la force donc jamais, exactement comme elle ne force pas le
        // paiement en MC. Sans cette garde, `--probe-plants 0` sur une carte à
        // dépense de plantes ferait sauter l'assertion d'`apply_card_effects` —
        // un plantage, pas un résultat lisible.
        //
        // Aucun effet sur les lots précédents : la réserve de départ valait 20
        // en dur, et aucune carte n'exige plus de 2 plantes.
        if game.players[0].plants < plants_reserved_by(db, id) {
            break;
        }
        build_card_with(&mut game, db, 0, 0, 0, &mut pol);
    }
    // L'instantané de référence est pris APRÈS la dernière pose : le delta
    // n'isole que l'action de la dernière carte.
    let hand_after_pose = game.players[0].hand.len() as i64;
    let before = snap(&game);

    // L'action ne s'applique qu'à une carte réellement EN JEU : on n'active
    // jamais l'action d'une carte que la séquence n'a pas pu poser. Une
    // corporation, elle, est en place dès la mise en place.
    let action_applied = match cible {
        Cible::Carte(card_id) if has_action && game.players[0].played.contains(&card_id) => {
            // Les actions variables tirent leur montant via la politique sur le
            // RNG déterministe (graine 0) de l'état de sonde.
            apply_blue_action(&mut game, db, 0, card_id, &mut pol)
        }
        // (jokers-corpos) Même chemin d'activation, corps de règle identique.
        Cible::Corpo if has_action => apply_corp_action(&mut game, db, 0, &mut pol),
        _ => false,
    };

    let after = snap(&game);
    let hand_delta = game.players[0].hand.len() as i64 - hand_after_pose;
    ProbeActionResult {
        card: name.to_string(),
        found: true,
        in_lot,
        has_action,
        action_applied,
        delta: make_delta(&before, &after, hand_delta, 0),
        resources: probe_resources(&game, db),
        target_error: pol.error.clone(),
        corp,
        // Lu sur l'état du joueur sondé après l'activation — la sonde ne
        // recalcule rien (clause anti-shortcut n° 1).
        upgrades: game.players[0].phase_upgrade_labels(),
    }
}

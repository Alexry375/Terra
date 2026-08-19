//! **Les treize défauts du chantier `les-regles-des-cartes`.**
//!
//! Un test au moins par défaut, chacun citant en commentaire la ligne exacte de
//! la règle qu'il fait respecter — `docs/regles/livret-base.md:N`,
//! `docs/regles/livret-decouverte.md:N`, ou la transcription des cartons
//! (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`,
//! `data/cartes-imprimees/textes-cartes.json`).
//!
//! Deux disciplines tiennent tout le fichier :
//!
//! 1. **Rien n'est deviné.** Les valeurs attendues viennent des cartons
//!    transcrits et des livrets, jamais du code ; les états de départ sont des
//!    états que la partie réelle produit (une carte en jeu, une bourse, un
//!    paramètre au maximum en fin de partie), jamais des états fabriqués.
//! 2. **Les deux sens.** Chaque correctif est éprouvé sur le cas qu'il change ET
//!    sur le cas voisin qu'il ne doit pas changer — sans quoi un test vert ne
//!    prouverait que l'absence de mesure.

use engine::boites::{Boite, BoiteSet};
use engine::cards::{CardsDb, Color, Tag, JOKER_TAG_CHOICES};
use engine::effects::{Capacity, Reduction, TrigCond, CORPS};
use engine::flow::{
    apply_blue_action, build_card_with, capacites_apportees, ensure_joker_tag, play_round,
    requirements_met, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_corp, ProbeOptions, ProbeResult, ProbeScript};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::{BTreeMap, VecDeque};

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base").unwrap()).expect("boîte de base")
}

fn db_dec() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte")
}

/// Sonde, avec la garde obligatoire : ne jamais juger une valeur avant d'avoir
/// vérifié que la sonde a TROUVÉ et POSÉ la carte.
fn sonde(db: &CardsDb, noms: &[&str], corp: Option<&str>, script: &ProbeScript) -> ProbeResult {
    let r = run_probe_seq_corp(db, noms, ProbeOptions::default(), script, false, corp);
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    assert!(r.played, "sonde : carte non posée « {} »", r.card);
    r
}

fn script_joker(t: Tag) -> ProbeScript {
    ProbeScript { joker_tag: Some(t), ..ProbeScript::default() }
}

/// Partie réelle, mains vidées et bourses à zéro : rien n'arrive qu'on n'ait mis
/// là soi-même.
fn jeu(db: &CardsDb) -> GameState {
    let mut pol = RandomPolicy;
    let mut g = setup_game(db, 11, &mut pol);
    for p in 0..NUM_PLAYERS {
        let h: Vec<u16> = g.players[p].hand.drain(..).collect();
        g.discard.extend(h);
        g.players[p].mc = 0;
        g.players[p].heat = 0;
        g.players[p].plants = 0;
        g.players[p].phase_upgrades = [None; 5];
    }
    g.phase_upgrades_granted = 0;
    g.phase_upgrades_reupgraded = 0;
    g.corp_phase_upgrades_at_setup = 0;
    g
}

fn id_de(db: &CardsDb, nom: &str) -> u16 {
    db.resolve_card(nom).unwrap_or_else(|| panic!("carte introuvable : « {nom} »"))
}

/// Fait entrer une carte NOMMÉE en main du joueur 0.
fn en_main(g: &mut GameState, db: &CardsDb, nom: &str) -> u16 {
    let id = id_de(db, nom);
    if let Some(i) = g.deck.iter().position(|&c| c == id) {
        g.deck.remove(i);
    } else if let Some(i) = g.discard.iter().position(|&c| c == id) {
        g.discard.remove(i);
    } else {
        panic!("« {nom} » n'est ni en pioche ni en défausse");
    }
    g.players[0].hand.push(id);
    id
}

/// Met une carte NOMMÉE en jeu, par le chemin de pose ordinaire du moteur.
fn poser(g: &mut GameState, db: &CardsDb, nom: &str) -> u16 {
    let mut pol = RandomPolicy;
    let id = en_main(g, db, nom);
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    let mc = g.players[0].mc;
    g.players[0].mc = 1000;
    build_card_with(g, db, 0, i, 0, &mut pol);
    g.players[0].mc = mc;
    id
}

/// Une carte de la pioche, de la couleur voulue, dont le prix imprimé est dans
/// l'intervalle donné et dont les prérequis sont remplis sur l'état courant.
fn carte_posable(g: &mut GameState, db: &CardsDb, color: Color, min: i64, max: i64) -> u16 {
    let choix = g
        .deck
        .iter()
        .copied()
        .find(|&c| {
            let card = &db.projects[c as usize];
            card.color == color
                && card.price >= min
                && card.price <= max
                && requirements_met(g, db, 0, c)
        })
        .unwrap_or_else(|| panic!("aucune carte {color:?} posable entre {min} et {max} MC"));
    let i = g.deck.iter().position(|&c| c == choix).unwrap();
    g.deck.remove(i);
    g.players[0].hand.push(choix);
    choix
}

fn prix(db: &CardsDb, id: u16) -> i64 {
    db.projects[id as usize].price
}

fn indice_du_badge(t: Tag) -> usize {
    JOKER_TAG_CHOICES.iter().position(|&x| x == t).expect("badge du jeu")
}

// =========================================================================
// LES POLITIQUES SCRIPTÉES
// =========================================================================

/// Le pilote : il impose la phase, accepte ou refuse chaque pose dans un ordre
/// écrit d'avance, répond aux « ou » dans un ordre écrit d'avance, et RELÈVE ce
/// qu'on lui a proposé. C'est ce relevé qui sert d'oracle : il dit ce que le
/// moteur a offert, pas ce que le moteur a fait.
struct Pilote {
    base: RandomPolicy,
    phase: u8,
    /// Réponses imposées à `choose_build`, dans l'ordre. Épuisée = refus.
    poses: VecDeque<Option<usize>>,
    /// Réponses imposées à `choose_option`, dans l'ordre.
    choix: VecDeque<usize>,
    /// Nombre d'options offertes à chaque appel de `choose_option`.
    offertes: Vec<usize>,
    /// Badge imposé tant que la carte est EN MAIN.
    joker_main: Option<Tag>,
    /// Badge imposé AU MOMENT DE LA POSE, s'il fait partie des candidats.
    joker_pose: Option<Tag>,
    /// Candidats reçus à chaque question du badge reposé.
    candidats_recus: Vec<Vec<usize>>,
    /// Nombre de poses réellement offertes à chaque question de construction.
    poses_offertes: Vec<Vec<usize>>,
}

impl Pilote {
    fn new(phase: u8) -> Pilote {
        Pilote {
            base: RandomPolicy,
            phase,
            poses: VecDeque::new(),
            choix: VecDeque::new(),
            offertes: Vec::new(),
            joker_main: None,
            joker_pose: None,
            candidats_recus: Vec::new(),
            poses_offertes: Vec::new(),
        }
    }
    fn poses(mut self, p: &[Option<usize>]) -> Pilote {
        self.poses = p.iter().copied().collect();
        self
    }
    fn choix(mut self, c: &[usize]) -> Pilote {
        self.choix = c.iter().copied().collect();
        self
    }
    fn joker_main(mut self, t: Tag) -> Pilote {
        self.joker_main = Some(t);
        self
    }
    fn joker_pose(mut self, t: Tag) -> Pilote {
        self.joker_pose = Some(t);
        self
    }
}

impl Policy for Pilote {
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) {
            self.phase
        } else {
            allowed[0]
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, p: usize, a: &[usize]) -> Option<usize> {
        if p != 0 {
            return None;
        }
        self.poses_offertes.push(a.to_vec());
        self.poses.pop_front().flatten()
    }
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, _o: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        self.offertes.push(n);
        match self.choix.pop_front() {
            Some(c) if c < n => c,
            Some(_) | None => self.base.choose_option(r, p, n),
        }
    }
    fn pick_joker_tag(&mut self, r: &mut StdRng, p: usize, c: u16, t: &[u32]) -> usize {
        match self.joker_main {
            Some(tag) => indice_du_badge(tag),
            None => self.base.pick_joker_tag(r, p, c, t),
        }
    }
    fn pick_joker_tag_a_la_pose(
        &mut self,
        r: &mut StdRng,
        p: usize,
        c: u16,
        t: &[u32],
        candidats: &[usize],
    ) -> usize {
        self.candidats_recus.push(candidats.to_vec());
        if let Some(tag) = self.joker_pose {
            if let Some(k) = candidats.iter().position(|&i| i == indice_du_badge(tag)) {
                return k;
            }
        }
        self.base.pick_joker_tag_a_la_pose(r, p, c, t, candidats)
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, d: &[u16], k: usize) -> Vec<usize> {
        (0..k.min(d.len())).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

/// L'activeur : il active TOUJOURS une action de carte bleue quand on lui en
/// propose une, et il RELÈVE, à chaque tour, la liste des cartes qu'on lui
/// offre. Ce relevé est l'oracle du droit de répétition : il dit ce que le
/// joueur pouvait choisir, ce qu'aucune lecture d'état ne prouve.
struct Activeur {
    phase: u8,
    /// Rang de l'option retenue à chaque tour : `derniere` = la dernière offerte.
    derniere: bool,
    /// Cartes bleues offertes à chaque question d'action, dans l'ordre.
    offres: Vec<Vec<u16>>,
    /// Nombre d'activations par carte.
    activations: BTreeMap<u16, u32>,
    total: usize,
}

impl Activeur {
    fn new(phase: u8) -> Activeur {
        Activeur {
            phase,
            derniere: false,
            offres: Vec::new(),
            activations: BTreeMap::new(),
            total: 0,
        }
    }
    fn derniere(mut self) -> Activeur {
        self.derniere = true;
        self
    }
}

impl Policy for Activeur {
    fn corp_mulligan(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> bool {
        false
    }
    fn project_mulligan(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16]) -> Vec<usize> {
        Vec::new()
    }
    fn pick_corporation(&mut self, _r: &mut StdRng, _p: usize, _c: &[u16]) -> usize {
        0
    }
    fn pick_phase(&mut self, _r: &mut StdRng, _p: usize, allowed: &[u8]) -> u8 {
        if allowed.contains(&self.phase) {
            self.phase
        } else {
            allowed[0]
        }
    }
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, _a: &[usize]) -> Option<usize> {
        None
    }
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        ConstructionBonus::DrawCard
    }
    fn action_choice(&mut self, _r: &mut StdRng, p: usize, o: &[ActionOpt]) -> Option<usize> {
        // Le joueur 0 seul active ; l'adversaire passe tout de suite.
        if p != 0 {
            return None;
        }
        let bleues: Vec<u16> = o
            .iter()
            .filter_map(|x| match x {
                ActionOpt::BlueAction(c) => Some(*c),
                _ => None,
            })
            .collect();
        self.offres.push(bleues.clone());
        if bleues.is_empty() {
            return None;
        }
        let carte = if self.derniere { *bleues.last().unwrap() } else { bleues[0] };
        let i = o
            .iter()
            .position(|x| matches!(x, ActionOpt::BlueAction(c) if *c == carte))
            .unwrap();
        *self.activations.entry(carte).or_insert(0) += 1;
        self.total += 1;
        Some(i)
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, d: &[u16], k: usize) -> Vec<usize> {
        (0..k.min(d.len())).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

// =========================================================================
// D2 — MINING GUILD : 1 NT PAR ACIER DE SAVOIR-FAIRE
// =========================================================================

#[test]
fn mining_guild_rend_un_nt_par_acier_de_savoir_faire() {
    // Transcription `data/cartes-imprimees/textes-cartes.json`, Mining Guild :
    // « EFFECT: Each time you play steel production, excluding this, gain 1 TR. »
    // Le « savoir-faire acier » est le mécanisme du livret de base l. 527 :
    // « Le coût des cartes Projet avec un badge Construction […] est réduit de
    // 2 MC pour chaque acier en votre possession. » *Mine* réduit de 4 MC, donc
    // DEUX aciers, donc deux NT — un par acier, jamais un forfait.
    let r = sonde(&db(), &["Mine"], Some("Mining Guild"), &ProbeScript::default());
    assert_eq!(r.delta.tr, 2, "deux aciers apportés, deux NT");
    assert_eq!(r.steel, 3, "les deux aciers de Mine, plus celui de la planche");
}

#[test]
fn mining_guild_ne_rend_aucun_nt_sur_un_savoir_faire_titane() {
    // `data/cartes-imprimees/textes-cartes.json` : le texte imprimé dit
    // « steel production », pas « titanium production ». Le titane est l'autre
    // savoir-faire du livret de base l. 529 (3 MC par titane) : il ne déclenche
    // rien. *Vesta Shipyard* réduit de 3 MC les cartes à badge Espace.
    let r = sonde(&db(), &["Vesta Shipyard"], Some("Mining Guild"), &ProbeScript::default());
    assert_eq!(r.delta.tr, 0, "un titane n'est pas un acier");
    assert_eq!(r.titanium, 1, "le titane est bien arrivé, lui");
}

#[test]
fn sans_mining_guild_aucun_nt_ne_tombe_sur_un_savoir_faire_acier() {
    // Le contre-témoin : la même pose, une autre planche. Sans le texte imprimé
    // de Mining Guild (`data/cartes-imprimees/textes-cartes.json`), aucun NT
    // n'est dû — un test qui ne le vérifierait pas ne saurait pas dire si le NT
    // vient de la corporation ou de la carte.
    let r = sonde(&db(), &["Mine"], Some("Credicor"), &ProbeScript::default());
    assert_eq!(r.delta.tr, 0, "aucune corporation, aucun NT");
    assert_eq!(r.steel, 2, "les deux aciers de Mine sont pourtant bien là");
}

#[test]
fn mining_guild_ne_compte_pas_l_acier_de_sa_propre_planche() {
    // « excluding this » (`data/cartes-imprimees/textes-cartes.json`) : la
    // planche porte elle-même une réduction bâtiment de 2 MC, donc un acier,
    // et elle ne doit pas se payer un NT à elle-même. Une carte sans le moindre
    // savoir-faire ne rend donc rien du tout.
    let r = sonde(&db(), &["Bribed Comittee"], Some("Mining Guild"), &ProbeScript::default());
    assert_eq!(r.delta.tr, 2, "les 2 NT imprimés de la carte, et rien de plus");
    let (_, spec) = CORPS
        .iter()
        .find(|(nom, _)| *nom == "Mining Guild")
        .expect("Mining Guild encodée");
    assert_eq!(spec.play_triggers.len(), 1, "un déclencheur, et un seul");
    assert!(!spec.play_triggers[0].include_self, "« excluding this »");
    assert!(
        spec.play_triggers[0].scale_by_matched_tags,
        "« each time » : un NT par acier, pas un forfait"
    );
    assert_eq!(
        spec.play_triggers[0].cond,
        TrigCond::GrantsCapacity(Capacity::Steel),
        "la condition est le savoir-faire ACIER"
    );
}

#[test]
fn le_compte_des_aciers_d_une_carte_a_un_seul_point_de_lecture() {
    // Le déclencheur de Mining Guild et le décompte du joueur lisent la MÊME
    // fonction : `flow::capacites_apportees`.
    // `docs/regles/livret-base.md:527` pour le taux de l'acier (2 MC = 1 acier)
    // et `docs/regles/livret-base.md:529` pour le titane (3 MC = 1 titane).
    let db = db();
    assert_eq!(capacites_apportees(&db, id_de(&db, "Mine")).steel, 2);
    assert_eq!(capacites_apportees(&db, id_de(&db, "Mine")).titanium, 0);
    assert_eq!(capacites_apportees(&db, id_de(&db, "Vesta Shipyard")).titanium, 1);
    assert_eq!(capacites_apportees(&db, id_de(&db, "Vesta Shipyard")).steel, 0);
    // *Media Group* réduit de 5 MC les cartes à badge Événement : ce n'est ni
    // de l'acier ni du titane, et ce n'est même pas une carte verte.
    assert_eq!(capacites_apportees(&db, id_de(&db, "Media Group")).steel, 0);
    assert_eq!(capacites_apportees(&db, id_de(&db, "Media Group")).titanium, 0);
}

// =========================================================================
// D5 — LE BADGE JOKER : PROVISOIRE EN MAIN, DÉFINITIF À LA POSE
// =========================================================================

/// L'état des tests du badge joker : *Mine* et *Media Group* en jeu,
/// *Topographic Mapping* en main. Prix imprimé 10 MC, badges imprimés
/// [joker, Événement]. Le badge choisi change le prix, et c'est tout l'intérêt :
///   — Événement : deux badges Événement, −10 MC (Media Group), prix 0 ;
///   — Construction : −4 (Mine) −5 (Media Group), prix 1 ;
///   — tout autre badge : −5 (Media Group), prix 5.
fn etat_joker(db: &CardsDb, mc: i64) -> (GameState, u16) {
    let mut g = jeu(db);
    poser(&mut g, db, "Mine");
    poser(&mut g, db, "Media Group");
    let id = en_main(&mut g, db, "Topographic Mapping");
    g.players[0].mc = mc;
    (g, id)
}

#[test]
fn le_badge_joker_est_redemande_a_la_pose_et_peut_changer() {
    // `docs/regles/livret-decouverte.md:100` : « Si vous jouez (ou défaussez) la
    // carte plus tard, vous pourrez choisir un badge différent. » Le jeton posé
    // en main n'engage donc à rien : la question est REPOSÉE au moment de la
    // pose, et la réponse peut différer.
    let db = db_dec();
    let (mut g, id) = etat_joker(&db, 5);
    let mut pol = Pilote::new(1).joker_main(Tag::Earth).joker_pose(Tag::Event);
    ensure_joker_tag(&mut g, &db, 0, id, &mut pol);
    assert_eq!(
        g.players[0].joker_tags.get(&id),
        Some(&Tag::Earth),
        "en main, le jeton est celui qu'on a demandé"
    );
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(
        g.players[0].joker_tags.get(&id),
        Some(&Tag::Event),
        "à la pose, le joueur a changé d'avis — et c'est son droit"
    );
    assert_eq!(g.joker_badges_reposes, 1, "la question a bien été reposée une fois");
}

#[test]
fn le_second_choix_du_badge_joker_est_borne_aux_badges_payables() {
    // `docs/regles/livret-decouverte.md:98` — le badge se choisit « lorsque vous
    // ajoutez la carte à votre main », et de nouveau à la pose (l. 100). Mais la
    // carte a quitté la main : un badge qui la rendrait impayable n'est pas une
    // réponse possible, le paiement du livret de base l. 348 devant rester
    // faisable. À 1 MC, seuls Construction (prix 1) et Événement (prix 0)
    // laissent la carte payable.
    let db = db_dec();
    let (mut g, id) = etat_joker(&db, 1);
    let mut pol = Pilote::new(1).joker_main(Tag::Earth).joker_pose(Tag::Building);
    ensure_joker_tag(&mut g, &db, 0, id, &mut pol);
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(pol.candidats_recus.len(), 1, "une question, une seule");
    let recus: Vec<Tag> =
        pol.candidats_recus[0].iter().map(|&k| JOKER_TAG_CHOICES[k]).collect();
    assert_eq!(
        recus,
        vec![Tag::Building, Tag::Event],
        "les deux seuls badges qui laissent les 10 MC payables avec 1 MC en caisse"
    );
    assert_eq!(g.players[0].joker_tags.get(&id), Some(&Tag::Building));
}

#[test]
fn un_seul_badge_payable_s_impose_sans_qu_on_pose_la_question() {
    // Même règle (`docs/regles/livret-decouverte.md:100`), cas limite : à zéro
    // MC, seul le badge Événement laisse la carte payable (prix 0). Il n'y a
    // plus d'alternative, donc plus de question — et le compteur des questions
    // reposées ne bouge pas.
    let db = db_dec();
    let (mut g, id) = etat_joker(&db, 0);
    let mut pol = Pilote::new(1).joker_main(Tag::Earth);
    ensure_joker_tag(&mut g, &db, 0, id, &mut pol);
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert!(pol.candidats_recus.is_empty(), "aucune question à une seule réponse");
    assert_eq!(g.players[0].joker_tags.get(&id), Some(&Tag::Event));
    assert_eq!(g.joker_badges_reposes, 0);
}

#[test]
fn le_jeton_du_badge_joker_reste_provisoire_tant_que_la_carte_est_en_main() {
    // `docs/regles/livret-decouverte.md:98` : le badge est choisi à l'entrée en
    // main, mais rien n'y est scellé. Le moteur le dit dans son état : la carte
    // n'entre dans la liste des badges DÉFINITIFS qu'à la pose.
    let db = db_dec();
    let (mut g, id) = etat_joker(&db, 5);
    let mut pol = Pilote::new(1).joker_main(Tag::Earth);
    ensure_joker_tag(&mut g, &db, 0, id, &mut pol);
    assert!(
        !g.players[0].joker_tags_definitifs.contains(&id),
        "en main, rien n'est définitif"
    );
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert!(
        g.players[0].joker_tags_definitifs.contains(&id),
        "posée, la carte a son badge pour de bon"
    );
}

#[test]
fn les_badges_jokers_sont_reellement_reposes_en_partie_entiere() {
    // La mesure, et non l'intention (`docs/regles/livret-decouverte.md:100`).
    // Deux cents parties de la boîte Découverte : la question est bel et bien
    // reposée. Et le contre-témoin, la boîte de base, qui ne contient aucune
    // carte à badge joker : le compteur y reste nul.
    let s = run_simulation(&db_dec(), 200, 4242, &mut RandomPolicy);
    assert!(
        s.joker_badges_reposes >= 1,
        "aucun badge joker reposé en 200 parties : la question ne se pose jamais"
    );
    let base = run_simulation(&db(), 200, 4242, &mut RandomPolicy);
    assert_eq!(base.joker_badges_reposes, 0, "pas de badge joker en boîte de base");
}

// =========================================================================
// D6 — LA RÉPÉTITION DE LA PHASE III PORTE SUR N'IMPORTE QUELLE CARTE
// =========================================================================

/// Deux cartes bleues à action GRATUITE en jeu, et la phase III choisie. Leur
/// activation ne dépend d'aucune ressource : le compte mesure le droit de
/// répétition et rien d'autre.
fn etat_deux_bleues(db: &CardsDb, upgrade: Option<PhaseUpgrade>) -> (GameState, u16, u16) {
    let mut g = jeu(db);
    let a = poser(&mut g, db, "Advanced Screening Tech");
    let b = poser(&mut g, db, "Circuit Board Factory");
    if let Some(v) = upgrade {
        g.players[0].upgrade_phase(3, v);
    }
    (g, a, b)
}

#[test]
fn l_activation_bonus_porte_sur_n_importe_quelle_carte_en_jeu() {
    // `docs/regles/livret-base.md:371` : « Bonus : Si vous avez choisi cette
    // phase, vous pouvez résoudre une fois de plus la capacité "Action :" de
    // L'UNE DE VOS CARTES EN JEU. » Le moteur dépensait la répétition d'office
    // sur la carte qu'on venait d'activer ; le joueur ne choisissait jamais.
    // L'oracle est le relevé de ce qui lui est OFFERT.
    let db = db();
    let (mut g, a, b) = etat_deux_bleues(&db, None);
    let mut pol = Activeur::new(3);
    play_round(&mut g, &db, &mut pol);
    // La DERNIÈRE offre non vide est celle de la répétition : les deux cartes y
    // ont déjà été activées une fois, et les deux doivent y figurer. La première
    // offre, elle, les contient aussi — mais parce qu'elles sont neuves : c'est
    // la dernière, et elle seule, qui dit ce que le joueur peut RÉPÉTER.
    let derniere = pol
        .offres
        .iter()
        .filter(|o| !o.is_empty())
        .next_back()
        .cloned()
        .expect("le joueur a bien reçu des offres");
    assert!(
        derniere.contains(&a) && derniere.contains(&b),
        "à la répétition, les deux cartes en jeu doivent être offertes : {:?}",
        pol.offres
    );
}

#[test]
fn l_activation_bonus_peut_etre_gardee_pour_une_autre_carte() {
    // Même ligne (`docs/regles/livret-base.md:371`). Le joueur qui active
    // d'abord A puis B doit pouvoir porter sa répétition sur B — c'est
    // précisément ce que l'ancien câblage lui interdisait, la répétition étant
    // consommée sur A dès la première activation.
    let db = db();
    let (mut g, a, b) = etat_deux_bleues(&db, None);
    let mut pol = Activeur::new(3).derniere();
    play_round(&mut g, &db, &mut pol);
    assert_eq!(pol.total, 3, "deux activations fraîches, plus une répétition");
    assert_eq!(pol.activations.get(&a).copied().unwrap_or(0), 1, "A n'a servi qu'une fois");
    assert_eq!(pol.activations.get(&b).copied().unwrap_or(0), 2, "la répétition est allée sur B");
}

#[test]
fn le_budget_d_activation_bonus_n_est_decompte_qu_a_la_repetition() {
    // `docs/regles/livret-base.md:371` : le bonus vaut « une fois de plus », pas
    // « à la place de ». Les activations fraîches ne l'entament donc pas ; seule
    // la répétition le dépense.
    let db = db();
    let (mut g, _, _) = etat_deux_bleues(&db, None);
    let mut pol = Activeur::new(3);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(pol.total, 3, "deux fraîches et une répétition");
    assert_eq!(
        g.players[0].extra_blue_activations, 0,
        "la répétition, et elle seule, a vidé le budget"
    );
}

#[test]
fn les_activations_bonus_libres_se_comptent_en_partie_entiere() {
    // La mesure (`docs/regles/livret-base.md:371`). Trois cents parties : des
    // répétitions sont réellement CHOISIES par le joueur, au site exact où le
    // budget est décompté.
    let s = run_simulation(&db_dec(), 300, 777, &mut RandomPolicy);
    assert!(
        s.activations_bonus_libres >= 1,
        "aucune répétition librement choisie en 300 parties"
    );
}

// =========================================================================
// D7 — DEUX CARTES DISTINCTES AU PLUS, JAMAIS LA MÊME TROIS FOIS
// =========================================================================

#[test]
fn une_carte_n_est_jamais_activee_trois_fois() {
    // Transcription
    // `data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, III-B :
    // « Vous pouvez activer DEUX DE VOS EFFETS "Action :" une fois de plus. »
    // Deux effets, donc deux cartes distinctes. Avec une seule carte bleue en
    // jeu, la seconde répétition n'a pas de cible : la carte plafonne à deux
    // activations, et ne monte jamais à trois.
    let db = db();
    let mut g = jeu(&db);
    let a = poser(&mut g, &db, "Advanced Screening Tech");
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantB);
    let mut pol = Activeur::new(3);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(pol.activations.get(&a).copied().unwrap_or(0), 2, "une fraîche, une répétition");
    assert_eq!(g.cartes_activees_trois_fois, 0, "jamais trois fois la même carte");
    assert_eq!(
        g.players[0].extra_blue_activations, 1,
        "la seconde répétition reste en caisse, faute d'une seconde carte"
    );
}

#[test]
fn iii_b_vise_deux_cartes_distinctes_au_plus() {
    // Même transcription
    // (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, III-B).
    // L'AUTRE SENS : avec deux cartes bleues, les deux répétitions s'exercent —
    // une sur chacune, et chacune s'arrête à deux activations.
    let db = db();
    let (mut g, a, b) = etat_deux_bleues(&db, Some(PhaseUpgrade::VariantB));
    let mut pol = Activeur::new(3);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(pol.total, 4, "deux fraîches et deux répétitions");
    assert_eq!(pol.activations.get(&a).copied().unwrap_or(0), 2);
    assert_eq!(pol.activations.get(&b).copied().unwrap_or(0), 2);
    assert_eq!(g.cartes_activees_trois_fois, 0);
    assert_eq!(g.players[0].extra_blue_activations, 0, "le budget est à sec");
}

#[test]
fn aucune_carte_activee_trois_fois_sur_trois_cents_parties() {
    // La mesure sur la boîte complète
    // (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, III-B).
    // Le compteur est une sentinelle posée au site exact de l'activation : il
    // compte les activations réellement faites, carte par carte, et monte à la
    // troisième. Il reste nul, et des répétitions ont bien eu lieu — sans quoi
    // le zéro ne prouverait rien.
    let s = run_simulation(&db_dec(), 300, 777, &mut RandomPolicy);
    assert!(s.activations_bonus_libres >= 1, "l'occasion s'est présentée");
    assert_eq!(s.cartes_activees_trois_fois, 0, "aucune carte activée trois fois");
}

// =========================================================================
// D8 — L'AMÉLIORATION EN PLACE RESTE CANDIDATE
// =========================================================================

#[test]
fn l_amelioration_en_place_reste_candidate() {
    // `docs/regles/livret-decouverte.md:66` : « vous pouvez choisir d'améliorer
    // en une amélioration différente une carte Phase que vous avez déjà
    // améliorée. » C'est une PERMISSION, pas une obligation : rien n'oblige à
    // changer, et la carte déjà en place doit donc rester proposée. Le moteur la
    // retirait de la liste, forçant la bascule.
    let db = db_dec();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    let mut pol = Pilote::new(1).choix(&[0]);
    let id = en_main(&mut g, &db, "Topographic Mapping");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(pol.offertes.first().copied(), Some(10), "les dix cartes Phase améliorées");
    assert_eq!(
        g.players[0].phase_upgrade(1),
        Some(PhaseUpgrade::VariantA),
        "l'indice 0 vise 1A : le joueur a gardé la sienne"
    );
}

#[test]
fn rechoisir_l_amelioration_en_place_ne_change_rien() {
    // Même ligne (`docs/regles/livret-decouverte.md:66`). Le joueur qui redésigne
    // sa propre carte n'en gagne pas une seconde et n'en perd pas : il a
    // exercé son droit de ne rien changer.
    let db = db_dec();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    let avant = g.players[0].phase_upgrades_count();
    let mut pol = Pilote::new(1).choix(&[0]);
    let id = en_main(&mut g, &db, "Topographic Mapping");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.players[0].phase_upgrades_count(), avant, "toujours une seule carte Phase I");
    assert_eq!(g.players[0].phase_upgrade(1), Some(PhaseUpgrade::VariantA));
}

#[test]
fn les_dix_ameliorations_sont_offertes_a_un_joueur_qui_n_en_a_aucune() {
    // `docs/regles/livret-decouverte.md:64` : la boîte Découverte apporte DIX
    // cartes Phase améliorées, deux par phase. Le contre-témoin du test
    // précédent : sans rien en place, la liste vaut déjà dix — la correction
    // n'a donc pas ajouté d'option, elle a cessé d'en retirer une.
    let db = db_dec();
    let mut g = jeu(&db);
    let mut pol = Pilote::new(1).choix(&[3]);
    let id = en_main(&mut g, &db, "Topographic Mapping");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(pol.offertes.first().copied(), Some(10));
    assert_eq!(
        g.players[0].phase_upgrade(2),
        Some(PhaseUpgrade::VariantB),
        "l'indice 3 vise la seconde variante de la phase II"
    );
}

#[test]
fn une_amelioration_a_phase_imposee_offre_encore_ses_deux_variantes() {
    // `docs/regles/livret-decouverte.md:64` : la VARIANTE reste un choix du
    // joueur même quand le carton impose la phase. *Perfluorocarbon Production*
    // dit « Améliorez votre carte Phase I » : la phase vient du carton, les
    // deux variantes restent offertes — celle qui est déjà en place comprise.
    let db = db_dec();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    let mut pol = Pilote::new(1).choix(&[0]);
    let id = en_main(&mut g, &db, "Perfluorocarbon Production");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(
        pol.offertes.first().copied(),
        Some(2),
        "phase imposée : les deux variantes, la sienne comprise"
    );
    assert_eq!(g.players[0].phase_upgrade(1), Some(PhaseUpgrade::VariantA));
}

#[test]
fn aucune_amelioration_n_est_imposee_sans_choix_sur_trois_cents_parties() {
    // La mesure (`docs/regles/livret-decouverte.md:66`). La sentinelle compte,
    // à chaque octroi, les fois où le moteur a tranché à la place du joueur
    // faute d'une seconde candidate. Elle reste nulle, et les occasions se
    // présentent bel et bien — les cartes à phase imposée en fournissent des
    // dizaines par centaine de parties.
    let s = run_simulation(&db_dec(), 300, 909, &mut RandomPolicy);
    assert!(s.phase_upgrades_targeted >= 50, "les occasions se présentent");
    assert_eq!(s.ameliorations_imposees_sans_choix, 0);
}

// =========================================================================
// D9 — UNE BRANCHE QUI NE PEUT RIEN PRODUIRE N'EST PLUS OFFERTE
// =========================================================================

/// *Biomedical Imports* : « Augmentez l'oxygène de 1 niveau OU améliorez une
/// carte Phase. » Deux branches, dans l'ordre du carton.
fn etat_biomedical(db: &CardsDb, oxygene: u8) -> GameState {
    let mut g = jeu(db);
    g.oxygen = oxygene;
    g.snap_oxygen = oxygene;
    en_main(&mut g, db, "Biomedical Imports");
    g.players[0].mc = 1000;
    g
}

#[test]
fn une_branche_qui_ne_peut_rien_produire_n_est_plus_offerte() {
    // `docs/regles/livret-base.md:363` : « Vous pouvez jouer des cartes qui
    // augmentent les paramètres au-delà de leur maximum […]. Vous ne recevrez
    // simplement pas les avantages liés à ces effets. » Une BRANCHE dont le seul
    // effet est une telle hausse ne donne donc rien : la proposer, c'est offrir
    // un choix qui n'en est pas un. À oxygène maximal, il ne reste que
    // l'amélioration de carte Phase, et la question ne se pose plus.
    let db = db_dec();
    let mut g = etat_biomedical(&db, OXYGEN_MAX);
    let mut pol = Pilote::new(1).choix(&[0]);
    let i = g.players[0].hand.len() - 1;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(
        pol.offertes.first().copied(),
        Some(10),
        "la seule question posée est celle de l'amélioration, à dix réponses"
    );
    assert_eq!(g.phase_upgrades_granted, 1, "la branche utile a bien été appliquée");
    assert_eq!(g.oxygen, OXYGEN_MAX, "l'oxygène n'a pas bougé, il ne le pouvait pas");
}

#[test]
fn les_deux_branches_restent_offertes_quand_le_parametre_peut_monter() {
    // L'AUTRE SENS, même ligne (`docs/regles/livret-base.md:363`) : tant que
    // l'oxygène peut monter, l'alternative est une vraie alternative et le
    // joueur doit être interrogé sur DEUX branches.
    let db = db_dec();
    let mut g = etat_biomedical(&db, OXYGEN_MAX - 1);
    let mut pol = Pilote::new(1).choix(&[0]);
    let i = g.players[0].hand.len() - 1;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(pol.offertes.first().copied(), Some(2), "les deux branches du carton");
    assert_eq!(g.oxygen, OXYGEN_MAX, "la branche oxygène a été prise et a produit");
    assert_eq!(g.phase_upgrades_granted, 0, "et l'autre branche n'a rien donné");
}

#[test]
fn poser_la_carte_reste_permis_meme_quand_une_branche_disparait() {
    // `docs/regles/livret-base.md:363` encore : le livret autorise EXPRESSÉMENT
    // à jouer la carte, il dit seulement qu'on n'en recevra pas l'avantage. Le
    // correctif retire une BRANCHE de la question, jamais la carte de la main.
    let db = db_dec();
    let mut g = etat_biomedical(&db, OXYGEN_MAX);
    let mut pol = Pilote::new(1).choix(&[0]);
    let id = g.players[0].hand[g.players[0].hand.len() - 1];
    let i = g.players[0].hand.len() - 1;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert!(g.players[0].played.contains(&id), "la carte est bien en jeu");
    // Et la question posée n'a plus qu'UNE réponse : la branche stérile a
    // disparu de l'alternative, pas la carte de la main. C'est la différence
    // entre « ne pas offrir un marché vide » et « interdire la carte ».
    assert_eq!(
        pol.offertes.first().copied(),
        Some(10),
        "l'alternative s'est effacée, la seule question restante est l'amélioration"
    );
    assert!(
        !g.players[0].hand.contains(&id),
        "la carte a quitté la main : elle a bien été jouée, pas refusée"
    );
}

#[test]
fn aucune_branche_impossible_offerte_sur_quatre_cents_parties() {
    // La mesure (`docs/regles/livret-base.md:387`, où le livret redit la règle
    // du maximum pour les cartes Phase).
    //
    // La sentinelle n'appelle PAS le correctif : elle relève l'état du plateau
    // avant et après chaque branche réellement appliquée, et compte celles qui
    // ne promettaient que des hausses de paramètre et n'ont rien bougé. Le
    // correctif pourrait être faux, débranché ou toujours vrai, elle le verrait.
    //
    // Le compteur d'OCCASIONS est là pour que le zéro veuille dire quelque
    // chose : un zéro obtenu parce que le cas ne s'est jamais présenté ne
    // prouverait rien du tout.
    let s = run_simulation(&db_dec(), 400, 31415, &mut RandomPolicy);
    assert!(
        s.branches_a_parametre_prises > 0,
        "aucune branche à gain de paramètre prise : la sentinelle n'a rien mesuré"
    );
    assert_eq!(s.branches_impossibles_offertes, 0);
    assert!(s.completed >= 380, "les parties vont au bout : {}", s.completed);
    assert_eq!(s.invariant_violations, 0);
}

// =========================================================================
// D17 — L'OBJECTIF TERRAFORMEUR EST PRIS AU MOMENT OÙ LA CONDITION EST REMPLIE
// =========================================================================

/// Une partie où la tuile Objectif TERRAFORMEUR est en jeu, et personne ne l'a.
fn etat_terraformeur(db: &CardsDb, tr: i64) -> GameState {
    let mut g = jeu(db);
    g.milestones[0] = MilestoneSlot {
        kind: MilestoneKind::Terraformer,
        achieved_by: [false; NUM_PLAYERS],
    };
    g.players[0].tr = tr;
    g.players[0].mc = 1000;
    g
}

#[test]
fn le_terraformeur_est_pris_au_vol_avant_que_le_niveau_ne_redescende() {
    // `docs/regles/livret-decouverte.md:72` : « Le PREMIER joueur à remplir cette
    // condition prend la tuile Objectif correspondante. » La tuile se prend à
    // l'instant où la condition est remplie, pas au bilan de fin de phase.
    // TERRAFORMEUR demande 15 de note de terraformation. *Bribed Comittee* en
    // donne 2 (13 → 15) ; *Investment Loan* en dépense 1 (15 → 14). Au bilan de
    // fin de phase, le joueur serait à 14 et n'aurait rien — c'est le défaut.
    let db = db();
    let mut g = etat_terraformeur(&db, 13);
    poser(&mut g, &db, "Bribed Comittee");
    assert_eq!(g.players[0].tr, 15, "le seuil est bien franchi en cours de phase");
    assert!(g.milestones[0].achieved_by[0], "la tuile est prise à l'instant du franchissement");
    poser(&mut g, &db, "Investment Loan");
    assert_eq!(g.players[0].tr, 14, "le niveau est redescendu sous le seuil");
    assert!(
        g.milestones[0].achieved_by[0],
        "la tuile reste prise : elle l'a été au moment où la condition était remplie"
    );
}

#[test]
fn le_terraformeur_n_est_pas_pris_avant_le_seuil() {
    // Même ligne (`docs/regles/livret-decouverte.md:72`) : « remplir cette
    // condition ». À 14, la condition n'est pas remplie, et rien n'est pris —
    // sans ce contre-témoin, un moteur qui distribuerait la tuile à tout le
    // monde passerait le test précédent.
    let db = db();
    let mut g = etat_terraformeur(&db, 12);
    poser(&mut g, &db, "Bribed Comittee");
    assert_eq!(g.players[0].tr, 14);
    assert!(!g.milestones[0].achieved_by[0], "14 n'est pas 15");
}

#[test]
fn l_objectif_terraformeur_ne_change_plus_de_main_une_fois_pris() {
    // « Le PREMIER joueur » (`docs/regles/livret-decouverte.md:72`) : la tuile
    // est unique. Une fois qu'un joueur l'a, l'adversaire qui franchit le même
    // seuil À UNE PHASE ULTÉRIEURE ne la lui prend pas et ne reçoit rien.
    let db = db();
    let mut g = etat_terraformeur(&db, 13);
    g.phase_en_cours = 2;
    poser(&mut g, &db, "Bribed Comittee");
    assert!(g.milestones[0].achieved_by[0]);
    g.phase_en_cours = 3;
    g.players[1].tr = 20;
    engine::flow::assign_milestones(&mut g);
    assert!(!g.milestones[0].achieved_by[1], "elle est déjà prise");
    assert!(g.milestones[0].achieved_by[0]);
}

#[test]
fn dans_la_meme_phase_le_second_a_franchir_le_seuil_recoit_son_jeton() {
    // La SECONDE phrase de `docs/regles/livret-decouverte.md:72`, celle qu'on
    // oublie : « Si plusieurs joueurs remplissent la condition durant la même
    // phase, l'un d'entre eux prend la tuile Objectif tandis que les autres
    // reçoivent un jeton 3 PV. »
    //
    // C'est le contre-témoin indispensable de D17. Prendre l'Objectif AU VOL,
    // sans plus, referme cette fenêtre à l'instant : l'adversaire qui franchit
    // le seuil un peu plus tard dans la même phase perdrait 3 PV que le livret
    // lui accorde. Le test emprunte le vrai chemin — une carte posée qui fait
    // monter le niveau de terraformation — et non un appel direct au bilan.
    let db = db();
    let mut g = etat_terraformeur(&db, 13);
    g.phase_en_cours = 2;
    poser(&mut g, &db, "Bribed Comittee");
    assert!(g.milestones[0].achieved_by[0], "le premier prend la tuile au vol");
    assert!(!g.milestones[0].achieved_by[1], "l'adversaire n'a pas encore le seuil");
    // Même phase : l'adversaire franchit le seuil à son tour.
    g.players[1].tr = 15;
    engine::flow::assign_milestones(&mut g);
    assert!(
        g.milestones[0].achieved_by[1],
        "même phase : le second reçoit son jeton 3 PV, il n'est pas spolié"
    );
    assert!(g.milestones[0].achieved_by[0], "et le premier garde la tuile");
}

// =========================================================================
// D18 — PAS DE SECONDE CARTE SANS PREMIÈRE
// =========================================================================

/// Un joueur qui a la carte Phase I améliorée B, la phase I choisie, et une
/// seule carte verte en main.
fn etat_phase_un_b(db: &CardsDb, prix_min: i64, prix_max: i64) -> (GameState, u16) {
    let mut g = jeu(db);
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantB);
    let id = carte_posable(&mut g, db, Color::Green, prix_min, prix_max);
    g.players[0].mc = 1000;
    (g, id)
}

#[test]
fn sans_premiere_carte_posee_la_suivante_recoit_la_remise_de_la_premiere_carte() {
    // Transcription
    // `data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, I-B :
    // « Le coût de la PREMIÈRE carte que vous jouez lors de cette phase est
    // réduit de 3 MC. Vous pouvez jouer une SECONDE carte verte lors de cette
    // phase dont le coût imprimé est de 12 MC ou moins. » Un joueur qui ne pose
    // rien au premier temps n'a pas joué de première carte : celle qu'il joue
    // ensuite EST sa première, et porte donc la remise.
    let db = db_dec();
    let (mut g, id) = etat_phase_un_b(&db, 5, 12);
    let mc_avant = g.players[0].mc;
    let mut pol = Pilote::new(1).poses(&[None, Some(0)]);
    play_round(&mut g, &db, &mut pol);
    assert!(g.players[0].played.contains(&id), "la carte a bien été jouée au second temps");
    assert_eq!(
        mc_avant - g.players[0].mc,
        prix(&db, id) - 3,
        "elle a payé le prix de la PREMIÈRE carte, remise de 3 MC comprise"
    );
    assert_eq!(g.secondes_poses_sans_premiere, 0);
}

#[test]
fn sans_premiere_carte_la_suivante_echappe_au_plafond_de_douze_mc() {
    // Même transcription
    // (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, I-B) :
    // le plafond de 12 MC est écrit sur la SECONDE carte, pas sur la première.
    // Une carte verte à plus de 12 MC doit donc rester jouable au second temps
    // quand le premier n'a rien posé — le moteur la refusait.
    let db = db_dec();
    let (mut g, id) = etat_phase_un_b(&db, 13, 40);
    let mc_avant = g.players[0].mc;
    let mut pol = Pilote::new(1).poses(&[None, Some(0)]);
    play_round(&mut g, &db, &mut pol);
    assert!(prix(&db, id) > 12, "la fixture doit bien dépasser le plafond de la seconde");
    assert!(g.players[0].played.contains(&id), "elle est jouée : c'est une PREMIÈRE carte");
    assert_eq!(mc_avant - g.players[0].mc, prix(&db, id) - 3);
}

#[test]
fn la_seconde_carte_ne_recoit_pas_la_remise_de_trois_mc() {
    // L'AUTRE SENS, même transcription
    // (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, I-B) :
    // « le coût de la PREMIÈRE carte ». Quand une première carte a bien été
    // posée, la seconde se paie plein tarif — le correctif n'a pas transformé
    // la remise en remise permanente.
    let db = db_dec();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantB);
    let a = carte_posable(&mut g, &db, Color::Green, 5, 12);
    let b = carte_posable(&mut g, &db, Color::Green, 5, 12);
    g.players[0].mc = 1000;
    let mc_avant = g.players[0].mc;
    let mut pol = Pilote::new(1).poses(&[Some(0), Some(0)]);
    play_round(&mut g, &db, &mut pol);
    assert!(g.players[0].played.contains(&a) && g.players[0].played.contains(&b), "deux poses");
    assert_eq!(
        mc_avant - g.players[0].mc,
        prix(&db, a) + prix(&db, b) - 3,
        "une seule remise de 3 MC pour deux cartes"
    );
}

#[test]
fn aucune_seconde_pose_sans_premiere_sur_quatre_cents_parties() {
    // La mesure
    // (`data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json`, I-B).
    // La sentinelle ne lit pas la branche empruntée : elle regarde la FILE des
    // permissions et compte celles qui y dorment alors qu'aucune première carte
    // n'a été posée. Elle reste nulle, et les occasions se comptent par
    // dizaines — sans quoi le zéro ne dirait rien.
    let s = run_simulation(&db_dec(), 400, 2718, &mut RandomPolicy);
    assert!(s.upgraded_extra_builds >= 20, "les occasions se présentent : {}", s.upgraded_extra_builds);
    assert_eq!(s.secondes_poses_sans_premiere, 0);
}

// =========================================================================
// D19 — UN EFFET DÉCLENCHÉ SE RÉSOUT AUTANT DE FOIS QUE SA CONDITION
// =========================================================================

#[test]
fn optimal_aerobraking_se_resout_plusieurs_fois_sur_deux_badges() {
    // `docs/regles/livret-base.md:106` : « Si la condition d'un effet est
    // remplie PLUSIEURS FOIS lorsqu'une carte est jouée, résolvez l'effet
    // correspondant plusieurs fois. » *Optimal Aerobraking* donne 2 chaleurs et
    // 2 plantes par badge Événement. *Topographic Mapping* porte un badge
    // Événement imprimé ET un badge joker : déclaré Événement, il en fait deux,
    // donc 4 et 4.
    let db = db_dec();
    let deux = sonde(
        &db,
        &["Optimal Aerobraking", "Topographic Mapping"],
        None,
        &script_joker(Tag::Event),
    );
    assert_eq!(deux.delta.heat, 4, "deux badges Événement, deux résolutions");
    assert_eq!(deux.delta.plants, 4);
}

#[test]
fn un_seul_badge_evenement_ne_declenche_optimal_aerobraking_qu_une_fois() {
    // L'AUTRE SENS, même ligne (`docs/regles/livret-base.md:106`) : la même
    // carte, un badge joker déclaré Terre, donc un seul badge Événement, donc
    // une seule résolution. Sans ce contre-témoin, un moteur qui doublerait
    // tout passerait le test précédent.
    let db = db_dec();
    let un = sonde(
        &db,
        &["Optimal Aerobraking", "Topographic Mapping"],
        None,
        &script_joker(Tag::Earth),
    );
    assert_eq!(un.delta.heat, 2);
    assert_eq!(un.delta.plants, 2);
}

#[test]
fn recycled_detritus_pioche_quatre_cartes_sur_deux_badges() {
    // Même règle (`docs/regles/livret-base.md:106`), l'autre carte que le
    // chantier corrige : *Recycled Detritus* fait piocher 2 cartes par badge
    // Événement.
    let db = db_dec();
    let deux = sonde(
        &db,
        &["Recycled Detritus", "Topographic Mapping"],
        None,
        &script_joker(Tag::Event),
    );
    let un = sonde(
        &db,
        &["Recycled Detritus", "Topographic Mapping"],
        None,
        &script_joker(Tag::Earth),
    );
    assert_eq!(deux.delta.hand - un.delta.hand, 2, "deux cartes de plus pour un badge de plus");
}

#[test]
fn un_declencheur_sans_badge_satisfaisant_ne_se_resout_pas_du_tout() {
    // `docs/regles/livret-base.md:106` : « si la condition est remplie ». Une
    // carte sans badge Événement ne remplit rien, et l'effet ne se résout pas.
    // C'est la borne basse de la même échelle.
    let db = db_dec();
    let r = sonde(&db, &["Optimal Aerobraking", "Mine"], None, &ProbeScript::default());
    assert_eq!(r.delta.heat, 0, "aucun badge Événement, aucune résolution");
    assert_eq!(r.delta.plants, 0);
}

// =========================================================================
// D20 — UNE RÉDUCTION PAR BADGE COMPTE LE NOMBRE DE BADGES
// =========================================================================

#[test]
fn une_reduction_par_badge_compte_le_nombre_de_badges() {
    // `docs/regles/livret-base.md:106` : « résolvez l'effet correspondant
    // plusieurs fois » — la même phrase imprimée que D19, appliquée à une
    // réduction de coût. *Media Group* : « When you play an Event, you pay 5 MC
    // less for it. » *Topographic Mapping* coûte 10 MC et porte un badge
    // Événement imprimé ; son badge joker déclaré Événement en fait deux, donc
    // −10 MC, donc 0 à payer.
    let db = db_dec();
    let deux = sonde(
        &db,
        &["Media Group", "Topographic Mapping"],
        None,
        &script_joker(Tag::Event),
    );
    assert_eq!(deux.paid.last().copied(), Some(0), "10 MC moins deux fois 5 MC");
}

#[test]
fn une_reduction_par_badge_ne_compte_que_les_badges_presents() {
    // L'AUTRE SENS, même ligne (`docs/regles/livret-base.md:106`) : avec un seul
    // badge Événement, la réduction vaut 5 MC et la carte coûte 5. Le prix
    // payé est celui du livret de base l. 348 (« le coût d'une carte Projet doit
    // être payé avec des MC »), lu sur le paiement réel, pas recalculé.
    let db = db_dec();
    let un = sonde(
        &db,
        &["Media Group", "Topographic Mapping"],
        None,
        &script_joker(Tag::Earth),
    );
    assert_eq!(un.paid.last().copied(), Some(5), "10 MC moins une fois 5 MC");
}

#[test]
fn la_table_des_reductions_multiplie_par_le_nombre_de_badges() {
    // Le service lui-même (`docs/regles/livret-base.md:106`). Le savoir-faire
    // acier du livret de base l. 527 s'écrit `Reduction::Tag(Building, 2)` :
    // deux badges Construction doivent valoir 4 MC, pas 2.
    let r = Reduction::Tag(Tag::Building, 2);
    assert_eq!(r.amount_for(&[Tag::Building], 20), 2, "un badge");
    assert_eq!(r.amount_for(&[Tag::Building, Tag::Building], 20), 4, "deux badges");
    assert_eq!(r.amount_for(&[Tag::Building, Tag::Space], 20), 2, "un seul badge compte");
    assert_eq!(r.amount_for(&[Tag::Space], 20), 0, "aucun badge visé");
    assert_eq!(r.amount_for(&[], 20), 0, "aucun badge du tout");
}

// =========================================================================
// D21 — LA PIOCHE COMPTE 246 CARTES PROJET
// =========================================================================

#[test]
fn la_pioche_246_est_le_compte_des_deux_boites() {
    // `docs/regles/livret-base.md:43` : « 208 cartes Projet ».
    // `docs/regles/livret-decouverte.md:34` : « 38 cartes Projet ».
    // 208 + 38 = 246, et c'est ce que la pioche des deux boîtes doit rendre.
    let db = db_dec();
    let n = db.projects.iter().filter(|c| c.in_deck_v1).count();
    assert_eq!(n, 246, "208 cartes de base plus 38 de Découverte");
    assert_eq!(db.deck_project_count, 246, "et la pioche réellement distribuée les compte toutes");
}

#[test]
fn les_deux_cartes_fantomes_ne_portent_plus_le_drapeau_de_la_pioche() {
    // `docs/regles/livret-base.md:43` — le dénombrement imprimé ne laisse la
    // place à aucune carte de plus. *Microbiology Patents* et *Project
    // Inspection* n'existent sur aucune planche : elles restent dans la base de
    // données, mais hors pioche.
    let db = db_dec();
    for nom in ["Microbiology Patents", "Project Inspection"] {
        let c = db
            .projects
            .iter()
            .find(|c| c.name == nom)
            .unwrap_or_else(|| panic!("{nom} doit rester connue de la base"));
        assert!(!c.in_deck_v1, "{nom} ne doit plus porter le drapeau « dans la pioche »");
    }
}

#[test]
fn la_boite_de_base_compte_208_cartes_projet() {
    // `docs/regles/livret-base.md:43` : « **208 cartes Projet** ». Le compte de
    // la boîte seule, sans l'extension.
    let tout = db_dec();
    let n = tout
        .projects
        .iter()
        .filter(|c| c.in_deck_v1 && c.boite == Some(Boite::Base))
        .count();
    assert_eq!(n, 208);
    assert_eq!(db().deck_project_count, 208, "et la pioche de la boîte seule les compte toutes");
    // Et la garde qui va avec (D21) : une carte qui n'existe sur AUCUNE planche
    // ne peut pas porter le drapeau « dans la pioche » — sinon la somme des deux
    // boîtes ne serait plus le dénombrement imprimé.
    assert!(
        tout.projects.iter().filter(|c| c.in_deck_v1).all(|c| c.boite.is_some()),
        "une carte hors planche porte encore le drapeau « dans la pioche »"
    );

}

#[test]
fn la_boite_decouverte_ajoute_38_cartes_projet() {
    // `docs/regles/livret-decouverte.md:34` : « **38 cartes Projet** ». La
    // différence entre les deux configurations, et rien d'autre.
    let tout = db_dec();
    let n = tout
        .projects
        .iter()
        .filter(|c| c.in_deck_v1 && c.boite == Some(Boite::Decouverte))
        .count();
    assert_eq!(n, 38);
    assert_eq!(
        tout.deck_project_count - db().deck_project_count,
        38,
        "et la pioche gagne bien 38 cartes en ajoutant l'extension"
    );
    // Et la garde qui va avec (D21) : une carte qui n'existe sur AUCUNE planche
    // ne peut pas porter le drapeau « dans la pioche » — sinon la somme des deux
    // boîtes ne serait plus le dénombrement imprimé.
    assert!(
        tout.projects.iter().filter(|c| c.in_deck_v1).all(|c| c.boite.is_some()),
        "une carte hors planche porte encore le drapeau « dans la pioche »"
    );

}

// =========================================================================
// D22 ET D24 — LES COMMENTAIRES NE MENTENT PLUS
// =========================================================================

fn source(nom: &str) -> String {
    std::fs::read_to_string(format!("src/{nom}"))
        .unwrap_or_else(|e| panic!("src/{nom} illisible : {e}"))
}

#[test]
fn aucun_commentaire_du_moteur_ne_cite_un_chemin_inexistant() {
    // D22 — les transcriptions citées sont celles de
    // `data/cartes-imprimees/textes-cartes.json` et de ses voisines. Les
    // commentaires les désignaient sous un chemin `inputs/…` qui n'existe nulle
    // part : un lecteur qui voulait vérifier une valeur ne trouvait rien.
    for nom in ["boites.rs", "effects.rs", "flow.rs"] {
        let s = source(nom);
        for (i, ligne) in s.lines().enumerate() {
            assert!(
                !ligne.contains("inputs/"),
                "src/{nom}:{} cite un chemin inexistant : {}",
                i + 1,
                ligne.trim()
            );
        }
    }
    // Et les chemins réellement cités existent.
    for chemin in [
        "../data/cartes-imprimees/textes-cartes.json",
        "../data/cartes-imprimees/projets-decouverte/projets-decouverte.json",
        "../data/cartes-imprimees/corporations-discovery/corporations-discovery.json",
        "../data/cartes-imprimees/phases-ameliorees/phases-ameliorees.json",
    ] {
        assert!(std::path::Path::new(chemin).exists(), "{chemin} doit exister");
    }
}

#[test]
fn l_empreinte_citee_par_le_commentaire_est_celle_du_fichier_transcrit() {
    // D22 — le commentaire de `boites.rs` cite une empreinte SHA-256 de
    // `data/cartes-imprimees/textes-cartes.json` pour qu'on puisse vérifier que
    // la table encodée vient bien de cette transcription-là. L'empreinte citée
    // était celle d'un autre fichier. Le test la recalcule et la compare à ce
    // que le commentaire annonce — un commentaire qui reste faux redevient rouge.
    let s = source("boites.rs");
    let citee = s
        .split(|c: char| !c.is_ascii_hexdigit())
        .find(|m| m.len() == 64)
        .expect("une empreinte SHA-256 doit être citée dans src/boites.rs")
        .to_string();
    let sortie = std::process::Command::new("sha256sum")
        .arg("../data/cartes-imprimees/textes-cartes.json")
        .output()
        .expect("sha256sum doit tourner");
    let reelle = String::from_utf8_lossy(&sortie.stdout)
        .split_whitespace()
        .next()
        .expect("sha256sum rend une empreinte")
        .to_string();
    assert_eq!(citee, reelle, "l'empreinte citée doit être celle du fichier transcrit");
}

#[test]
fn aucun_commentaire_n_affirme_qu_une_amelioration_de_phase_n_est_pas_geree() {
    // D24 — deux commentaires d'`effects.rs` annonçaient qu'une amélioration de
    // carte Phase « n'est pas gérée » alors qu'elle passe par
    // `flow::apply_phase_upgrade` comme toutes les autres
    // (`docs/regles/livret-decouverte.md:64`).
    let s = source("effects.rs");
    for (i, ligne) in s.lines().enumerate() {
        let l = ligne.to_lowercase();
        let dit_non_geree = l.contains("non gérée")
            || l.contains("non geree")
            || l.contains("n'est pas gérée")
            || l.contains("n'est pas geree")
            || l.contains("pas encore gérée");
        assert!(
            !(dit_non_geree && (l.contains("amélioration") || l.contains("amelioration"))),
            "effects.rs:{} affirme encore qu'une amélioration n'est pas gérée : {}",
            i + 1,
            ligne.trim()
        );
    }
}

#[test]
fn les_deux_cartes_mises_en_cause_ameliorent_reellement_une_carte_phase() {
    // D24, l'autre sens : ce que le commentaire dit maintenant doit être vrai.
    // *Cryogenic Shipment* améliore à la pose ; *Fibrous Composite Material*
    // améliore par son action, contre trois jetons science
    // (`docs/regles/livret-decouverte.md:64`).
    let db = db_dec();
    let mut g = jeu(&db);
    let mut pol = Pilote::new(1).choix(&[0]);
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.phase_upgrades_granted, 1, "Cryogenic Shipment améliore bel et bien");
    assert_eq!(g.phase_upgrades_skipped, 0, "et rien n'est sauté");

    let mut g = jeu(&db);
    let mut pol = Pilote::new(1).choix(&[1, 0]);
    let carte = poser(&mut g, &db, "Fibrous Composite Material");
    assert_eq!(
        g.players[0].card_resources.get(&carte).copied().unwrap_or(0),
        3,
        "trois jetons science posés à l'entrée en jeu"
    );
    g.players[0].mc = 1000;
    let agit = apply_blue_action(&mut g, &db, 0, carte, &mut pol);
    assert!(agit, "l'action a bien eu lieu");
    assert_eq!(g.phase_upgrades_granted, 1, "et elle a amélioré une carte Phase");
}

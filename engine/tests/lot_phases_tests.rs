//! Tests du chantier `decouverte-phases` — **les cartes Phase améliorées**.
//!
//! Le mécanisme n'ajoute pas une carte : il ajoute une SECONDE VERSION de
//! chacune des cinq cartes Phase, et le droit d'échanger l'une contre l'autre
//! en cours de partie. Trois propriétés le résument, et chacune est vérifiée
//! ici dans les deux sens :
//!
//! 1. **Remplacement, jamais cumul** (livret l. 64) — le bonus amélioré prend
//!    la place du bonus de base ; -6 MC en phase I, jamais -9.
//! 2. **Un seul point de calcul** (`flow::selector_bonus`) — les cinq phases y
//!    passent, la sonde le lit sans le recalculer.
//! 3. **Rien n'est partagé** — chaque joueur a ses dix cartes ; améliorer chez
//!    l'un ne change rien chez l'autre.
//!
//! Les valeurs attendues viennent de la transcription des dix cartes
//! (`inputs/refs/phases-ameliorees.json`), pas du code.

use engine::boites::BoiteSet;
use engine::cards::{CardsDb, Color};
use engine::effects::{PHASE_BASE, PHASE_UPGRADED};
use engine::flow::{
    award_pool, build_card_with, play_round, requirements_met, research_base, research_draw_keep,
    selector_bonus, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_corp, ProbeOptions, ProbeResult, ProbeScript};
use engine::sim::run_simulation;
use engine::state::*;
use rand::rngs::StdRng;
use std::collections::VecDeque;

const CARDS: &str = "../data/cards.json";

fn db() -> CardsDb {
    CardsDb::load(CARDS).expect("cards.json doit se charger")
}

fn db_off() -> CardsDb {
    let mut d = db();
    d.effects_on = false;
    d
}

fn db_dec() -> CardsDb {
    CardsDb::load_boites(CARDS, BoiteSet::parse("base,decouverte").unwrap())
        .expect("base,decouverte")
}

/// Un joueur nu, avec la phase choisie et, éventuellement, une carte Phase
/// améliorée en main.
fn joueur(phase: u8, upgrade: Option<(u8, PhaseUpgrade)>) -> PlayerState {
    let mut pl = PlayerState::new();
    pl.chosen_phase = phase;
    if let Some((ph, v)) = upgrade {
        pl.upgrade_phase(ph, v);
    }
    pl
}

/// Sonde, avec la garde obligatoire : ne jamais juger une valeur avant d'avoir
/// vérifié que la sonde a TROUVÉ la carte.
fn seq(db: &CardsDb, names: &[&str], o: ProbeOptions) -> ProbeResult {
    let r = run_probe_seq_corp(db, names, o, &ProbeScript::default(), false, None);
    assert!(r.found, "sonde : carte introuvable « {} »", r.card);
    r
}

// =========================================================================
// La politique scriptée : elle choisit la phase, pose la première carte
// possible, et répond aux « ou » dans un ordre imposé.
// =========================================================================

struct Scenario {
    base: RandomPolicy,
    /// Phase imposée aux DEUX joueurs (seule cette phase s'exécutera).
    phase: u8,
    /// Poser dès qu'une carte est posable ?
    poser: bool,
    /// Réponses imposées à `Policy::choose_option`, dans l'ordre.
    choix: VecDeque<usize>,
    /// Branche imposée du bonus de la carte Phase II de BASE.
    constr: ConstructionBonus,
    /// Cartes gardées en phase V : les premières, pour un résultat lisible.
    garder_les_premieres: bool,
}

impl Scenario {
    fn new(phase: u8) -> Scenario {
        Scenario {
            base: RandomPolicy,
            phase,
            poser: true,
            choix: VecDeque::new(),
            constr: ConstructionBonus::SecondBuild,
            garder_les_premieres: true,
        }
    }
    fn choix(mut self, c: &[usize]) -> Scenario {
        self.choix = c.iter().copied().collect();
        self
    }
    fn sans_pose(mut self) -> Scenario {
        self.poser = false;
        self
    }
}

impl Policy for Scenario {
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
    fn choose_build(&mut self, _r: &mut StdRng, _p: usize, a: &[usize]) -> Option<usize> {
        if self.poser {
            a.first().copied()
        } else {
            None
        }
    }
    fn construction_bonus(&mut self, _r: &mut StdRng, _p: usize) -> ConstructionBonus {
        self.constr
    }
    fn action_choice(&mut self, _r: &mut StdRng, _p: usize, _o: &[ActionOpt]) -> Option<usize> {
        None
    }
    fn choose_option(&mut self, r: &mut StdRng, p: usize, n: usize) -> usize {
        match self.choix.pop_front() {
            Some(c) => c,
            None => self.base.choose_option(r, p, n),
        }
    }
    fn research_keep(&mut self, r: &mut StdRng, p: usize, d: &[u16], k: usize) -> Vec<usize> {
        if self.garder_les_premieres {
            (0..k.min(d.len())).collect()
        } else {
            self.base.research_keep(r, p, d, k)
        }
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.base.discard_down(r, p, h, n)
    }
}

/// Partie réelle, mains vidées et bourses à zéro : rien n'arrive qu'on n'ait
/// mis là soi-même.
fn jeu(db: &CardsDb) -> GameState {
    let mut pol = RandomPolicy;
    let mut g = setup_game(db, 11, &mut pol);
    for p in 0..NUM_PLAYERS {
        let h: Vec<u16> = g.players[p].hand.drain(..).collect();
        g.discard.extend(h);
        g.players[p].mc = 0;
        g.players[p].heat = 0;
        g.players[p].plants = 0;
    }
    g
}

/// Fait entrer une carte NOMMÉE en main du joueur 0.
fn en_main(g: &mut GameState, db: &CardsDb, nom: &str) -> u16 {
    let id = db
        .resolve_card(nom)
        .unwrap_or_else(|| panic!("carte introuvable dans la base : « {nom} »"));
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

/// **Fixture MESURÉE, jamais devinée** : une carte de la pioche, de la couleur
/// voulue, dont le prix imprimé est dans l'intervalle donné et dont les
/// prérequis sont remplis sur l'état courant. Elle est mise en main du joueur 0.
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

// =========================================================================
// 1. LES DIX CARTES — la table de données colle à la transcription
// =========================================================================

#[test]
fn il_y_a_bien_dix_cartes_phase_ameliorees() {
    assert_eq!(PHASE_UPGRADED.len(), 5, "cinq phases");
    for (i, paire) in PHASE_UPGRADED.iter().enumerate() {
        assert_eq!(paire.len(), 2, "phase {} : deux options, A et B", i + 1);
        for spec in paire {
            assert!(!spec.name.is_empty(), "chaque carte porte son nom imprimé");
            assert!(!spec.branches.is_empty(), "chaque carte porte un bonus");
        }
    }
}

#[test]
fn il_y_a_cinq_cartes_phase_de_base() {
    assert_eq!(PHASE_BASE.len(), 5);
    for spec in PHASE_BASE.iter() {
        assert!(!spec.name.is_empty());
        assert!(!spec.branches.is_empty());
    }
}

#[test]
fn chaque_carte_amelioree_nomme_sa_phase_et_sa_variante() {
    let phases = ["Development", "Construction", "Action", "Production", "Research"];
    for (i, paire) in PHASE_UPGRADED.iter().enumerate() {
        for (v, spec) in paire.iter().enumerate() {
            let variante = PhaseUpgrade::ALL[v].label();
            assert!(
                spec.name.starts_with(phases[i]) && spec.name.contains(variante),
                "« {} » devrait nommer {} et la variante {variante}",
                spec.name,
                phases[i]
            );
        }
    }
}

#[test]
fn les_noms_des_dix_cartes_sont_tous_distincts() {
    let mut noms: Vec<&str> = PHASE_UPGRADED.iter().flatten().map(|s| s.name).collect();
    assert_eq!(noms.len(), 10);
    noms.sort_unstable();
    noms.dedup();
    assert_eq!(noms.len(), 10, "deux cartes Phase améliorées portent le même nom");
}

#[test]
fn la_variante_a_est_l_indice_zero_et_la_b_l_indice_un() {
    assert_eq!(PhaseUpgrade::VariantA.index(), 0);
    assert_eq!(PhaseUpgrade::VariantB.index(), 1);
    assert_eq!(PhaseUpgrade::VariantA.label(), "A");
    assert_eq!(PhaseUpgrade::VariantB.label(), "B");
    assert_eq!(PhaseUpgrade::ALL, [PhaseUpgrade::VariantA, PhaseUpgrade::VariantB]);
}

#[test]
fn la_designation_des_dix_ameliorations_se_lit() {
    for ph in 1u8..=5 {
        for v in PhaseUpgrade::ALL {
            let s = format!("{ph}{}", v.label());
            assert_eq!(
                parse_phase_upgrade(&s),
                Some((ph, v)),
                "« {s} » doit se lire"
            );
        }
    }
}

#[test]
fn une_designation_mal_formee_est_refusee_pas_ignoree() {
    // Un argument mal formé doit être REFUSÉ : c'est ce refus qui empêche
    // `--probe-upgrade 6A` de passer pour un succès en ne testant rien.
    for mauvais in ["", "1", "A", "6A", "0A", "1C", "X", "11", "1AB", "1a", "b2", " 1A"] {
        assert_eq!(
            parse_phase_upgrade(mauvais),
            None,
            "« {mauvais} » ne devrait pas se lire"
        );
    }
}

// =========================================================================
// 2. LE POINT DE CALCUL UNIQUE — remplacement, jamais cumul
// =========================================================================

#[test]
fn sans_phase_choisie_le_bonus_est_entierement_nul() {
    let db = db();
    let pl = PlayerState::new();
    for ph in 0u8..=5 {
        let b = selector_bonus(&db, &pl, ph);
        assert!(!b.is_selector, "phase {ph} : le joueur n'a rien choisi");
        assert_eq!((b.mc_discount, b.mc, b.draw), (0, 0, 0));
        assert_eq!((b.extra_activations, b.extra_builds), (0, 0));
        assert_eq!((b.research_draw, b.research_keep), (0, 0));
        assert_eq!(b.upgraded, None);
    }
}

#[test]
fn un_joueur_qui_a_choisi_une_autre_phase_n_a_aucun_bonus() {
    let db = db();
    let pl = joueur(2, Some((1, PhaseUpgrade::VariantA)));
    let b = selector_bonus(&db, &pl, 1);
    assert!(!b.is_selector, "il n'a pas choisi la phase I");
    assert_eq!(b.mc_discount, 0, "améliorée ou non, sans la phase il n'a rien");
}

#[test]
fn phase_i_de_base_reduit_de_trois() {
    let db = db();
    let b = selector_bonus(&db, &joueur(1, None), 1);
    assert_eq!(b.mc_discount, DEV_SELECTOR_DISCOUNT);
    assert_eq!(b.mc_discount, 3);
    assert_eq!(b.extra_builds, 0, "la carte de base n'ouvre aucune pose de plus");
    assert_eq!(b.upgraded, None);
}

#[test]
fn i_a_reduit_de_six_et_remplace_les_trois_de_base() {
    let db = db();
    let b = selector_bonus(&db, &joueur(1, Some((1, PhaseUpgrade::VariantA))), 1);
    assert_eq!(b.mc_discount, 6, "I-A : 6 MC");
    assert_ne!(b.mc_discount, 9, "9 MC serait le CUMUL de 3 et de 6");
    assert_eq!(b.upgraded, Some(PhaseUpgrade::VariantA));
    assert_eq!(b.extra_builds, 0, "I-A n'ouvre aucune pose de plus");
}

#[test]
fn i_b_garde_les_trois_mc_et_ouvre_une_seconde_verte() {
    let db = db();
    let b = selector_bonus(&db, &joueur(1, Some((1, PhaseUpgrade::VariantB))), 1);
    assert_eq!(b.mc_discount, 3, "I-B : la réduction reste de 3 MC");
    assert_eq!(b.extra_builds, 1, "I-B : une seconde verte");
    let g = &b.spec.branches[0];
    assert_eq!(g.builds.len(), 1);
    assert_eq!(g.builds[0].colors, &[Color::Green], "une VERTE");
    assert_eq!(g.builds[0].max_printed_cost, Some(12), "12 MC IMPRIMÉS ou moins");
    assert!(!g.builds[0].free, "elle se paie");
}

#[test]
fn phase_ii_de_base_est_une_alternative() {
    let db = db();
    let b = selector_bonus(&db, &joueur(2, None), 2);
    assert!(b.alternative, "« piochez une carte OU jouez-en une de plus »");
    assert_eq!(b.draw, 1);
    assert_eq!(b.extra_builds, 1);
}

#[test]
fn ii_a_donne_les_deux_et_non_l_un_ou_l_autre() {
    let db = db();
    let b = selector_bonus(&db, &joueur(2, Some((2, PhaseUpgrade::VariantA))), 2);
    assert!(!b.alternative, "II-A n'est plus un choix : les deux");
    assert_eq!(b.draw, 1, "piochez une carte");
    assert_eq!(b.extra_builds, 1, "ET une seconde bleue ou rouge");
    assert_eq!(b.spec.branches.len(), 1, "une seule branche");
}

#[test]
fn ii_b_reste_un_ou_entre_une_pose_et_six_mc() {
    let db = db();
    let b = selector_bonus(&db, &joueur(2, Some((2, PhaseUpgrade::VariantB))), 2);
    assert!(b.alternative, "II-B est un « ou »");
    assert_eq!(b.spec.branches.len(), 2);
    assert_eq!(b.spec.branches[0].builds.len(), 1, "branche 1 : une pose de plus");
    assert_eq!(b.spec.branches[1].mc, 6, "branche 2 : 6 MC");
    assert_eq!(b.draw, 0, "l'option « piochez » de la carte de base a disparu");
}

#[test]
fn phase_iii_de_base_donne_une_activation() {
    let db = db();
    let b = selector_bonus(&db, &joueur(3, None), 3);
    assert_eq!(b.extra_activations, 1);
    assert!(b.spec.branches[0].reveal.is_none());
}

#[test]
fn iii_a_garde_une_activation_et_ajoute_la_revelation() {
    let db = db();
    let b = selector_bonus(&db, &joueur(3, Some((3, PhaseUpgrade::VariantA))), 3);
    assert_eq!(b.extra_activations, 1, "III-A : une activation, comme la base");
    let r = b.spec.branches[0].reveal.expect("III-A révèle trois cartes");
    assert_eq!(r.n, 3, "les 3 premières cartes de la pioche");
    assert_eq!(r.take, 1, "une seule carte entre en main");
    assert_eq!(r.mc_per_discarded, 0, "les autres sont défaussées, sans compensation");
}

#[test]
fn iii_b_donne_deux_activations_et_remplace_celle_de_base() {
    let db = db();
    let b = selector_bonus(&db, &joueur(3, Some((3, PhaseUpgrade::VariantB))), 3);
    assert_eq!(b.extra_activations, 2, "III-B : deux");
    assert_ne!(b.extra_activations, 3, "3 serait le cumul du +1 de base et du +2");
}

#[test]
fn phase_iv_de_base_donne_quatre_mc() {
    let db = db();
    let b = selector_bonus(&db, &joueur(4, None), 4);
    assert_eq!(b.mc, PRODUCTION_SELECTOR_MC);
    assert_eq!(b.mc, 4);
    assert!(!b.spec.branches[0].replay_green_prod);
}

#[test]
fn iv_a_tombe_a_un_mc_et_rejoue_une_production_verte() {
    let db = db();
    let b = selector_bonus(&db, &joueur(4, Some((4, PhaseUpgrade::VariantA))), 4);
    assert_eq!(b.mc, 1, "IV-A : 1 MC, pas 4, pas 5");
    assert_ne!(b.mc, 5, "5 MC serait le cumul du +4 de base et du +1");
    assert!(b.spec.branches[0].replay_green_prod, "et une production verte rejouée");
}

#[test]
fn iv_b_donne_sept_mc() {
    let db = db();
    let b = selector_bonus(&db, &joueur(4, Some((4, PhaseUpgrade::VariantB))), 4);
    assert_eq!(b.mc, 7);
    assert_ne!(b.mc, 11, "11 MC serait le cumul du +4 de base et du +7");
}

#[test]
fn phase_v_de_base_vaut_trois_piochees_et_une_gardee() {
    // La COMPÉTENCE imprimée donne 2/1 à tout le monde ; le bonus du
    // sélectionneur vaut donc +3/+1, et le total les 5/2 du livret.
    let db = db();
    let b = selector_bonus(&db, &joueur(5, None), 5);
    assert_eq!((b.research_draw, b.research_keep), (3, 1));
    assert_eq!(research_base(&db, &joueur(5, None)), (5, 2), "5 piochées, 2 gardées");
    assert_eq!(research_base(&db, &PlayerState::new()), (2, 1), "la compétence seule");
}

#[test]
fn v_a_voit_moins_de_cartes_mais_en_garde_plus() {
    // L'arbitrage imprimé, contre-intuitif et voulu : 4 piochées (contre 5 pour
    // la carte de base) mais 3 gardées (contre 2).
    let db = db();
    let pl = joueur(5, Some((5, PhaseUpgrade::VariantA)));
    let b = selector_bonus(&db, &pl, 5);
    assert_eq!((b.research_draw, b.research_keep), (2, 2));
    assert_eq!(research_base(&db, &pl), (4, 3));
    let base = research_base(&db, &joueur(5, None));
    assert!(base.0 > 4, "la carte de base voit PLUS de cartes");
    assert!(base.1 < 3, "et en garde MOINS");
}

#[test]
fn v_b_pioche_huit_et_en_garde_deux() {
    let db = db();
    let pl = joueur(5, Some((5, PhaseUpgrade::VariantB)));
    assert_eq!(selector_bonus(&db, &pl, 5).research_draw, 6);
    assert_eq!(research_base(&db, &pl), (8, 2));
}

#[test]
fn aucun_des_cinq_bonus_ameliores_ne_cumule_celui_de_base() {
    // Contrôle systématique du NEVER 1 : pour chacune des cinq phases et
    // chacune des deux variantes, la valeur rendue est celle de la carte
    // améliorée — jamais la somme des deux cartes.
    let db = db();
    for ph in 1u8..=5 {
        let base = selector_bonus(&db, &joueur(ph, None), ph);
        for v in PhaseUpgrade::ALL {
            let up = selector_bonus(&db, &joueur(ph, Some((ph, v))), ph);
            let attendu = &PHASE_UPGRADED[ph as usize - 1][v.index()];
            let max = |f: fn(&engine::effects::SelectorGrant) -> i64| {
                attendu.branches.iter().map(f).max().unwrap_or(0)
            };
            assert_eq!(up.mc_discount, max(|g| g.mc_discount), "phase {ph}{}", v.label());
            assert_eq!(up.mc, max(|g| g.mc), "phase {ph}{}", v.label());
            // Là où les deux cartes donnent quelque chose, la valeur rendue
            // n'est JAMAIS leur somme.
            for (lu, b, u) in [
                (up.mc_discount, base.mc_discount, max(|g| g.mc_discount)),
                (up.mc, base.mc, max(|g| g.mc)),
                (
                    up.extra_activations as i64,
                    base.extra_activations as i64,
                    attendu
                        .branches
                        .iter()
                        .map(|g| g.extra_activations as i64)
                        .max()
                        .unwrap_or(0),
                ),
                (
                    up.research_draw as i64,
                    base.research_draw as i64,
                    attendu.branches.iter().map(|g| g.research_draw as i64).max().unwrap_or(0),
                ),
            ] {
                if b > 0 && u > 0 {
                    assert_eq!(lu, u, "phase {ph}{} : la carte améliorée", v.label());
                    assert_ne!(lu, b + u, "phase {ph}{} : jamais le cumul", v.label());
                }
            }
        }
    }
}

#[test]
fn une_amelioration_ne_vaut_que_pour_sa_phase() {
    let db = db();
    for ph in 1u8..=5 {
        for autre in 1u8..=5 {
            if autre == ph {
                continue;
            }
            // Le joueur a amélioré `ph` mais choisit `autre` : le bonus de
            // `autre` est celui de sa carte Phase NORMALE.
            let pl = joueur(autre, Some((ph, PhaseUpgrade::VariantB)));
            let b = selector_bonus(&db, &pl, autre);
            assert_eq!(b.upgraded, None, "phase {autre} n'est pas améliorée");
            let nu = selector_bonus(&db, &joueur(autre, None), autre);
            assert_eq!(b.mc_discount, nu.mc_discount);
            assert_eq!(b.mc, nu.mc);
            assert_eq!(b.extra_activations, nu.extra_activations);
            assert_eq!((b.research_draw, b.research_keep), (nu.research_draw, nu.research_keep));
        }
    }
}

#[test]
fn effets_coupes_les_cinq_bonus_retombent_sur_la_carte_de_base() {
    let on = db();
    let off = db_off();
    for ph in 1u8..=5 {
        for v in PhaseUpgrade::ALL {
            let pl = joueur(ph, Some((ph, v)));
            let coupe = selector_bonus(&off, &pl, ph);
            let nu = selector_bonus(&on, &joueur(ph, None), ph);
            assert_eq!(coupe.upgraded, None, "aucune amélioration ne s'applique");
            assert_eq!(coupe.mc_discount, nu.mc_discount, "phase {ph}");
            assert_eq!(coupe.mc, nu.mc, "phase {ph}");
            assert_eq!(coupe.draw, nu.draw, "phase {ph}");
            assert_eq!(coupe.extra_activations, nu.extra_activations, "phase {ph}");
            assert_eq!(coupe.extra_builds, nu.extra_builds, "phase {ph}");
            assert_eq!(coupe.research_draw, nu.research_draw, "phase {ph}");
            assert_eq!(coupe.research_keep, nu.research_keep, "phase {ph}");
        }
    }
}

#[test]
fn le_bonus_permanent_de_recherche_s_ajoute_a_la_carte_amelioree() {
    // Deux mécanismes distincts : le bonus PERMANENT des cartes en jeu
    // (lot 4) et le bonus de la carte Phase. Le premier s'ajoute au second,
    // qui remplace la carte de base — les deux ne se confondent pas.
    let db = db();
    let mut g = jeu(&db);
    g.players[0].chosen_phase = 5;
    let sans = research_draw_keep(&db, &g.players[0]);
    assert_eq!(sans, (5, 2), "carte de base, aucune carte en jeu");
    let mut pol = RandomPolicy;
    let id = en_main(&mut g, &db, "Interplanetary Relations");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    let permanent = research_draw_keep(&db, &g.players[0]);
    assert!(permanent.0 > sans.0, "le bonus permanent ajoute des pioches");
    g.players[0].upgrade_phase(5, PhaseUpgrade::VariantA);
    let avec = research_draw_keep(&db, &g.players[0]);
    assert_eq!(
        avec,
        (4 + (permanent.0 - 5), 3 + (permanent.1 - 2)),
        "V-A remplace la carte de base ; le permanent s'ajoute par-dessus"
    );
}

// =========================================================================
// 3. RIEN N'EST PARTAGÉ ENTRE LES JOUEURS
// =========================================================================

#[test]
fn ameliorer_chez_un_joueur_ne_change_rien_chez_l_autre() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].chosen_phase = 1;
    g.players[1].chosen_phase = 1;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    assert_eq!(selector_bonus(&db, &g.players[0], 1).mc_discount, 6);
    assert_eq!(selector_bonus(&db, &g.players[1], 1).mc_discount, 3, "l'autre garde sa carte");
    assert_eq!(g.players[1].phase_upgrades_count(), 0);
}

#[test]
fn les_deux_joueurs_peuvent_ameliorer_la_meme_phase() {
    // Rien ne l'interdit dans le livret (ASK 5) : chacun a ses dix cartes.
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    g.players[1].upgrade_phase(3, PhaseUpgrade::VariantB);
    g.players[0].chosen_phase = 3;
    g.players[1].chosen_phase = 3;
    assert_eq!(selector_bonus(&db, &g.players[0], 3).extra_activations, 1, "III-A");
    assert_eq!(selector_bonus(&db, &g.players[1], 3).extra_activations, 2, "III-B");
}

#[test]
fn ameliorer_une_phase_deja_amelioree_bascule_a_b() {
    let mut pl = PlayerState::new();
    assert!(!pl.upgrade_phase(2, PhaseUpgrade::VariantA), "première amélioration");
    assert_eq!(pl.phase_upgrade(2), Some(PhaseUpgrade::VariantA));
    assert!(pl.upgrade_phase(2, PhaseUpgrade::VariantB), "la phase était déjà améliorée");
    assert_eq!(pl.phase_upgrade(2), Some(PhaseUpgrade::VariantB), "A a laissé la place à B");
    assert_eq!(pl.phase_upgrades_count(), 1, "toujours UNE carte pour la phase II");
}

#[test]
fn le_compte_de_cartes_ameliorees_suit_les_phases_pas_les_ameliorations() {
    let mut pl = PlayerState::new();
    assert_eq!(pl.phase_upgrades_count(), 0);
    for (i, ph) in (1u8..=5).enumerate() {
        pl.upgrade_phase(ph, PhaseUpgrade::VariantA);
        assert_eq!(pl.phase_upgrades_count(), i as i64 + 1);
    }
    pl.upgrade_phase(1, PhaseUpgrade::VariantB);
    assert_eq!(pl.phase_upgrades_count(), 5, "une bascule ne crée pas de carte");
}

#[test]
fn les_etiquettes_des_ameliorations_sont_triees_par_phase() {
    let mut pl = PlayerState::new();
    pl.upgrade_phase(5, PhaseUpgrade::VariantA);
    pl.upgrade_phase(1, PhaseUpgrade::VariantB);
    assert_eq!(pl.phase_upgrade_labels(), vec!["1B".to_string(), "5A".to_string()]);
    assert!(PlayerState::new().phase_upgrade_labels().is_empty());
}

#[test]
fn hors_bornes_aucune_phase_n_est_amelioree() {
    let mut pl = PlayerState::new();
    pl.upgrade_phase(1, PhaseUpgrade::VariantA);
    assert_eq!(pl.phase_upgrade(0), None);
    assert_eq!(pl.phase_upgrade(6), None);
    assert_eq!(pl.phase_upgrade(1), Some(PhaseUpgrade::VariantA));
}

// =========================================================================
// 4. EN PARTIE RÉELLE — le bonus change le déroulement, pas qu'une sonde
// =========================================================================

#[test]
fn i_a_rend_payable_une_carte_qui_ne_l_etait_pas() {
    // ALWAYS 1 : l'affordabilité et le paiement voient la MÊME remise. Le
    // joueur a exactement le prix moins 6 : sans I-A, la carte reste en main.
    let db = db();
    let mut sans = jeu(&db);
    let id = carte_posable(&mut sans, &db, Color::Green, 10, 20);
    sans.players[0].mc = prix(&db, id) - 6;
    let mut pol = Scenario::new(1);
    play_round(&mut sans, &db, &mut pol);
    assert!(
        sans.players[0].played.is_empty(),
        "sans amélioration, la remise de 3 MC ne suffit pas"
    );

    let mut avec = jeu(&db);
    let id2 = carte_posable(&mut avec, &db, Color::Green, 10, 20);
    assert_eq!(id, id2, "même fixture des deux côtés");
    avec.players[0].mc = prix(&db, id2) - 6;
    avec.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    let mut pol = Scenario::new(1);
    play_round(&mut avec, &db, &mut pol);
    assert!(avec.players[0].played.contains(&id2), "avec I-A, elle est payable");
    assert_eq!(avec.players[0].mc, 0, "elle a coûté son prix moins 6 MC");
}

#[test]
fn i_a_ne_reduit_jamais_de_neuf() {
    // Le contre-témoin du cumul : à prix − 7, même I-A ne suffit pas.
    let db = db();
    let mut g = jeu(&db);
    let id = carte_posable(&mut g, &db, Color::Green, 12, 20);
    g.players[0].mc = prix(&db, id) - 7;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    let mut pol = Scenario::new(1);
    play_round(&mut g, &db, &mut pol);
    assert!(g.players[0].played.is_empty(), "6 MC de remise, pas 9");
}

#[test]
fn i_b_permet_une_seconde_verte_a_douze_mc_imprimes() {
    let db = db();
    let mut g = jeu(&db);
    let a = carte_posable(&mut g, &db, Color::Green, 0, 12);
    let b = carte_posable(&mut g, &db, Color::Green, 0, 12);
    assert_ne!(a, b);
    g.players[0].mc = 1000;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantB);
    let mut pol = Scenario::new(1);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].played.len(), 2, "la pose ordinaire PLUS la seconde verte");
}

#[test]
fn sans_i_b_la_phase_i_ne_pose_qu_une_carte() {
    let db = db();
    let mut g = jeu(&db);
    carte_posable(&mut g, &db, Color::Green, 0, 12);
    carte_posable(&mut g, &db, Color::Green, 0, 12);
    g.players[0].mc = 1000;
    let mut pol = Scenario::new(1);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].played.len(), 1, "la carte Phase de base : une seule pose");
}

#[test]
fn la_seconde_verte_de_i_b_respecte_le_plafond_de_douze() {
    // Deuxième carte à plus de 12 MC IMPRIMÉS : la permission ne l'admet pas,
    // même avec tout l'argent du monde.
    let db = db();
    let mut g = jeu(&db);
    let petite = carte_posable(&mut g, &db, Color::Green, 0, 12);
    let grosse = carte_posable(&mut g, &db, Color::Green, 13, 40);
    g.players[0].mc = 1000;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantB);
    let mut pol = Scenario::new(1);
    play_round(&mut g, &db, &mut pol);
    assert!(g.players[0].played.contains(&petite));
    assert!(
        !g.players[0].played.contains(&grosse),
        "13 MC imprimés : hors du plafond de 12 de I-B"
    );
}

#[test]
fn la_seconde_verte_de_i_b_ne_recoit_pas_la_remise_de_trois() {
    // « Le coût de la PREMIÈRE carte que vous jouez lors de cette phase est
    // réduit de 3 MC » : la seconde se paie plein tarif.
    let db = db();
    let mut g = jeu(&db);
    let a = carte_posable(&mut g, &db, Color::Green, 0, 12);
    let b = carte_posable(&mut g, &db, Color::Green, 0, 12);
    // De quoi payer la première réduite et la seconde plein tarif, à 1 MC près.
    g.players[0].mc = prix(&db, a) - 3 + prix(&db, b) - 1;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantB);
    let mut pol = Scenario::new(1);
    play_round(&mut g, &db, &mut pol);
    assert!(g.players[0].played.contains(&a), "la première passe");
    assert!(
        !g.players[0].played.contains(&b),
        "la seconde n'a pas eu la remise : il manque 1 MC"
    );
}

#[test]
fn ii_a_pioche_une_carte_et_pose_une_seconde_bleue_ou_rouge() {
    let db = db();
    let mut g = jeu(&db);
    carte_posable(&mut g, &db, Color::Blue, 0, 20);
    carte_posable(&mut g, &db, Color::Red, 0, 20);
    g.players[0].mc = 1000;
    g.players[0].upgrade_phase(2, PhaseUpgrade::VariantA);
    let avant = g.draw_before_build;
    let mut pol = Scenario::new(2);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].played.len(), 2, "deux poses");
    assert_eq!(g.draw_before_build, avant + 1, "ET une carte piochée");
}

#[test]
fn ii_b_branche_mc_donne_six_mc_et_aucune_pose_de_plus() {
    let db = db();
    let mut g = jeu(&db);
    carte_posable(&mut g, &db, Color::Blue, 0, 20);
    carte_posable(&mut g, &db, Color::Red, 0, 20);
    g.players[0].mc = 1000;
    g.players[0].upgrade_phase(2, PhaseUpgrade::VariantB);
    let mut pol = Scenario::new(2).choix(&[1]); // branche 2 : « OU gagnez 6 MC »
    let mc_avant = g.players[0].mc;
    play_round(&mut g, &db, &mut pol);
    let depense: i64 = g.players[0]
        .played
        .iter()
        .map(|&c| prix(&db, c))
        .sum();
    assert_eq!(g.players[0].played.len(), 1, "la branche MC ne pose pas de carte");
    assert_eq!(g.players[0].mc, mc_avant - depense + 6, "6 MC gagnés");
}

#[test]
fn ii_b_branche_pose_joue_une_carte_de_plus_et_ne_donne_aucun_mc() {
    let db = db();
    let mut g = jeu(&db);
    carte_posable(&mut g, &db, Color::Blue, 0, 20);
    carte_posable(&mut g, &db, Color::Red, 0, 20);
    g.players[0].mc = 1000;
    g.players[0].upgrade_phase(2, PhaseUpgrade::VariantB);
    let mc_avant = g.players[0].mc;
    let mut pol = Scenario::new(2).choix(&[0]); // branche 1 : une pose de plus
    play_round(&mut g, &db, &mut pol);
    let depense: i64 = g.players[0].played.iter().map(|&c| prix(&db, c)).sum();
    assert_eq!(g.players[0].played.len(), 2, "deux poses");
    assert_eq!(g.players[0].mc, mc_avant - depense, "aucun MC offert sur cette branche");
}

/// Politique qui active TOUJOURS une action de carte bleue quand on lui en
/// propose une, et qui COMPTE ses activations. Elle mesure le budget réel de
/// répétitions de la phase III — ce qu'aucune lecture d'état ne prouve.
struct Activeur {
    phase: u8,
    activations: usize,
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
        let i = o
            .iter()
            .position(|x| matches!(x, ActionOpt::BlueAction(_)))?;
        self.activations += 1;
        Some(i)
    }
    fn research_keep(&mut self, _r: &mut StdRng, _p: usize, d: &[u16], k: usize) -> Vec<usize> {
        (0..k.min(d.len())).collect()
    }
    fn discard_down(&mut self, _r: &mut StdRng, _p: usize, _h: &[u16], n: usize) -> Vec<usize> {
        (0..n).collect()
    }
}

/// Combien de fois le joueur 0 active-t-il RÉELLEMENT l'action de sa carte
/// bleue pendant une phase III complète ?
fn activations_reelles(db: &CardsDb, upgrade: Option<PhaseUpgrade>) -> usize {
    let mut g = jeu(db);
    let mut pose = RandomPolicy;
    // Une carte bleue à action gratuite : son activation ne dépend d'aucune
    // ressource, donc le compte mesure le BUDGET et rien d'autre.
    let id = en_main(&mut g, db, "Advanced Screening Tech");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, db, 0, i, 0, &mut pose);
    if let Some(v) = upgrade {
        g.players[0].upgrade_phase(3, v);
    }
    let mut pol = Activeur { phase: 3, activations: 0 };
    play_round(&mut g, db, &mut pol);
    pol.activations
}

#[test]
fn le_budget_de_repetitions_de_la_phase_iii_est_reellement_exerce() {
    // Le témoin que le câblage — et pas seulement l'état — fait son travail :
    // une carte bleue, une phase III, et on compte les activations obtenues.
    let db = db();
    let base = activations_reelles(&db, None);
    assert_eq!(base, 2, "une activation, plus la répétition du sélectionneur");
    let a = activations_reelles(&db, Some(PhaseUpgrade::VariantA));
    assert_eq!(a, 2, "III-A garde la répétition unique de la carte de base");
    let b = activations_reelles(&db, Some(PhaseUpgrade::VariantB));
    assert_eq!(b, 3, "III-B : deux répétitions, donc trois activations");
    assert!(b > base, "III-B doit rendre PLUS d'activations que la carte de base");
    assert_ne!(b, 4, "4 activations seraient le cumul du +1 de base et du +2");
}

#[test]
fn le_budget_de_repetitions_est_consomme_et_ne_survit_pas_a_la_phase() {
    // Le budget vit dans l'état du joueur : la boucle d'actions le DÉCRÉMENTE
    // à chaque répétition accordée. Une même carte bleue peut absorber les
    // deux répétitions de III-B (elle revient dans les activables à chaque
    // fois) : à la fin de la phase, le budget est à zéro.
    let db = db();
    let mut g = jeu(&db);
    let mut pose = RandomPolicy;
    let id = en_main(&mut g, &db, "Advanced Screening Tech");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pose);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantB);
    let mut pol = Activeur { phase: 3, activations: 0 };
    play_round(&mut g, &db, &mut pol);
    assert_eq!(pol.activations, 3);
    assert_eq!(
        g.players[0].extra_blue_activations, 0,
        "les deux répétitions ont été dépensées : le budget est à sec"
    );
}

#[test]
fn iii_b_porte_deux_activations_supplementaires_dans_la_phase() {
    // Sans carte bleue en jeu, le budget n'est jamais entamé : ce test mesure
    // ce qui est ACCORDÉ. Ce qui est EXERCÉ est mesuré par
    // `le_budget_de_repetitions_de_la_phase_iii_est_reellement_exerce`.
    let db = db();
    let mut g = jeu(&db);
    assert!(g.players[0].played.is_empty(), "aucune carte bleue : rien à activer");
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantB);
    let mut pol = Scenario::new(3).sans_pose();
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].extra_blue_activations, 2, "III-B : deux répétitions");
    assert_eq!(g.players[1].extra_blue_activations, 1, "l'autre garde sa carte de base");
}

#[test]
fn iii_a_revele_trois_cartes_au_debut_de_la_phase() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let avant = g.cards_revealed;
    let main_avant = g.players[0].hand.len();
    let mut pol = Scenario::new(3).sans_pose();
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.cards_revealed, avant + 3, "trois cartes révélées, réellement");
    assert!(
        g.players[0].hand.len() <= main_avant + 1,
        "au plus UNE carte entre en main"
    );
    assert_eq!(g.players[0].extra_blue_activations, 1, "et l'activation de base demeure");
}

#[test]
fn sans_iii_a_aucune_carte_n_est_revelee_en_phase_action() {
    let db = db();
    let mut g = jeu(&db);
    let avant = g.cards_revealed;
    let mut pol = Scenario::new(3).sans_pose();
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.cards_revealed, avant, "la carte Phase III de base ne révèle rien");
}

#[test]
fn iii_a_ne_garde_qu_une_carte_bleue_ou_rouge() {
    // Le filtre imprimé : une carte VERTE révélée ne peut pas entrer en main.
    let db = db();
    let mut g = jeu(&db);
    // Les trois cartes du dessus de la pioche sont vertes : rien n'est gardé.
    let vertes: Vec<u16> = g
        .deck
        .iter()
        .copied()
        .filter(|&c| db.projects[c as usize].color == Color::Green)
        .take(3)
        .collect();
    assert_eq!(vertes.len(), 3);
    for &c in &vertes {
        let i = g.deck.iter().position(|&x| x == c).unwrap();
        g.deck.remove(i);
    }
    g.deck.extend(vertes);
    g.players[0].upgrade_phase(3, PhaseUpgrade::VariantA);
    let main_avant = g.players[0].hand.len();
    let mut pol = Scenario::new(3).sans_pose();
    play_round(&mut g, &db, &mut pol);
    assert_eq!(
        g.players[0].hand.len(),
        main_avant,
        "trois vertes révélées : aucune n'est bleue ou rouge, rien n'entre en main"
    );
}

#[test]
fn iv_b_donne_sept_mc_en_partie_reelle() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(4, PhaseUpgrade::VariantB);
    let (mc0, tr0) = (g.players[0].mc, g.players[0].tr);
    let mut pol = Scenario::new(4);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].mc, mc0 + tr0 + 7, "production + NT + 7 MC");
}

#[test]
fn iv_a_donne_un_mc_et_rejoue_la_production_d_une_verte() {
    let db = db();
    let mut g = jeu(&db);
    let mut pol = RandomPolicy;
    // Une carte verte à production FIXE de MC, posée par le chemin réel.
    let id = en_main(&mut g, &db, "Economic Growth");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    let prod = g.players[0].mc_prod;
    assert!(prod > 0, "la fixture doit produire des MC");

    g.players[0].upgrade_phase(4, PhaseUpgrade::VariantA);
    let (mc0, tr0) = (g.players[0].mc, g.players[0].tr);
    let mut pol = Scenario::new(4);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(
        g.players[0].mc,
        mc0 + prod + tr0 + 1 + prod,
        "production + NT + 1 MC + la production de la verte, rejouée"
    );
}

#[test]
fn iv_a_sans_carte_verte_ne_rejoue_rien() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(4, PhaseUpgrade::VariantA);
    let (mc0, tr0) = (g.players[0].mc, g.players[0].tr);
    let mut pol = Scenario::new(4);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].mc, mc0 + tr0 + 1, "1 MC, et rien à rejouer");
}

#[test]
fn v_a_pioche_quatre_cartes_et_en_garde_trois() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(5, PhaseUpgrade::VariantA);
    let main = g.players[0].hand.len();
    let pioche = g.deck.len();
    let mut pol = Scenario::new(5);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].hand.len(), main + 3, "3 gardées");
    // Le joueur 1 a la carte de base : 5 piochées lui aussi (même phase).
    assert_eq!(g.players[1].hand.len(), 2, "l'autre joueur garde 2 cartes");
    assert_eq!(pioche - g.deck.len(), 4 + 5, "4 piochées pour V-A, 5 pour la base");
}

#[test]
fn v_b_pioche_huit_cartes_et_en_garde_deux() {
    let db = db();
    let mut g = jeu(&db);
    g.players[0].upgrade_phase(5, PhaseUpgrade::VariantB);
    let main = g.players[0].hand.len();
    let pioche = g.deck.len();
    let mut pol = Scenario::new(5);
    play_round(&mut g, &db, &mut pol);
    assert_eq!(g.players[0].hand.len(), main + 2, "2 gardées");
    assert_eq!(pioche - g.deck.len(), 8 + 5, "8 piochées pour V-B, 5 pour la base");
}

// =========================================================================
// 5. « AMÉLIOREZ UNE CARTE PHASE » — l'octroi, en partie réelle
// =========================================================================

#[test]
fn une_carte_a_amelioration_accorde_reellement_une_amelioration() {
    // *Cryogenic Shipment* (Découverte) : « Améliorez une carte Phase. »
    let db = db_dec();
    let mut g = jeu(&db);
    // (jokers-corpos) Les corporations de Découverte améliorent une carte Phase à
    // LA MISE EN PLACE : en boîte `base,decouverte`, le joueur peut donc déjà en
    // porter une avant la moindre pose. Ce test-ci porte sur l'octroi par une
    // CARTE : on repart des cinq cartes Phase normales et d'un compteur remis à
    // zéro, l'état exact que ce test éprouvait avant ce chantier.
    g.players[0].phase_upgrades = [None; 5];
    g.players[1].phase_upgrades = [None; 5];
    g.phase_upgrades_granted = 0;
    g.phase_upgrades_reupgraded = 0;
    g.corp_phase_upgrades_at_setup = 0;
    let mut pol = Scenario::new(1).choix(&[0]); // la première candidate : 1A
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.phase_upgrades_skipped, 0, "plus rien n'est sauté");
    assert_eq!(g.phase_upgrades_granted, 1, "une amélioration accordée");
    assert_eq!(g.players[0].phase_upgrade(1), Some(PhaseUpgrade::VariantA));
    assert_eq!(g.players[1].phase_upgrades_count(), 0, "rien chez l'adversaire");
}

#[test]
fn la_politique_choisit_laquelle_des_dix() {
    // Les candidates sont énumérées phase croissante, A avant B : l'indice 7
    // désigne donc 4B.
    let db = db_dec();
    let mut g = jeu(&db);
    // (jokers-corpos) Les corporations de Découverte améliorent une carte Phase à
    // LA MISE EN PLACE : en boîte `base,decouverte`, le joueur peut donc déjà en
    // porter une avant la moindre pose. Ce test-ci porte sur l'octroi par une
    // CARTE : on repart des cinq cartes Phase normales et d'un compteur remis à
    // zéro, l'état exact que ce test éprouvait avant ce chantier.
    g.players[0].phase_upgrades = [None; 5];
    g.players[1].phase_upgrades = [None; 5];
    g.phase_upgrades_granted = 0;
    g.phase_upgrades_reupgraded = 0;
    g.corp_phase_upgrades_at_setup = 0;
    let mut pol = Scenario::new(1).choix(&[7]);
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.players[0].phase_upgrade(4), Some(PhaseUpgrade::VariantB));
    assert_eq!(g.players[0].phase_upgrades_count(), 1);
}

#[test]
fn ameliorer_deux_fois_la_meme_phase_est_compte_comme_une_bascule() {
    let db = db_dec();
    let mut g = jeu(&db);
    // (jokers-corpos) Les corporations de Découverte améliorent une carte Phase à
    // LA MISE EN PLACE : en boîte `base,decouverte`, le joueur peut donc déjà en
    // porter une avant la moindre pose. Ce test-ci porte sur l'octroi par une
    // CARTE : on repart des cinq cartes Phase normales et d'un compteur remis à
    // zéro, l'état exact que ce test éprouvait avant ce chantier.
    g.players[0].phase_upgrades = [None; 5];
    g.players[1].phase_upgrades = [None; 5];
    g.phase_upgrades_granted = 0;
    g.phase_upgrades_reupgraded = 0;
    g.corp_phase_upgrades_at_setup = 0;
    g.players[0].upgrade_phase(1, PhaseUpgrade::VariantA);
    g.phase_upgrades_granted = 0;
    // 1A étant déjà en place, elle n'est plus candidate : l'indice 0 vise 1B.
    let mut pol = Scenario::new(1).choix(&[0]);
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.players[0].phase_upgrade(1), Some(PhaseUpgrade::VariantB), "A → B");
    assert_eq!(g.phase_upgrades_granted, 1);
    assert_eq!(g.phase_upgrades_reupgraded, 1, "c'était une bascule");
    assert_eq!(g.players[0].phase_upgrades_count(), 1, "toujours une seule carte");
}

#[test]
fn l_amelioration_gagnee_vaut_des_la_phase_suivante_de_la_meme_manche() {
    // ASK 1 : « si cette phase n'a pas encore été résolue, vous bénéficierez du
    // bonus ». Le bonus est LU au moment où la phase s'exécute, jamais figé à
    // la planification — une amélioration gagnée en phase II vaut donc en
    // phase IV de la même manche.
    let db = db_dec();
    let mut g = jeu(&db);
    g.players[0].chosen_phase = 4;
    assert_eq!(selector_bonus(&db, &g.players[0], 4).mc, 4, "carte de base");
    let mut pol = Scenario::new(2).choix(&[7]); // 4B
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(
        selector_bonus(&db, &g.players[0], 4).mc,
        7,
        "la phase IV n'est pas encore résolue : elle lira la carte améliorée"
    );
}

#[test]
fn effets_coupes_aucune_amelioration_n_est_accordee() {
    let mut db = db_dec();
    db.effects_on = false;
    let mut g = jeu(&db);
    let mut pol = Scenario::new(1);
    let id = en_main(&mut g, &db, "Cryogenic Shipment");
    let i = g.players[0].hand.iter().position(|&c| c == id).unwrap();
    g.players[0].mc = 1000;
    build_card_with(&mut g, &db, 0, i, 0, &mut pol);
    assert_eq!(g.phase_upgrades_granted, 0, "la couche d'effets est coupée");
    assert_eq!(g.players[0].phase_upgrades_count(), 0);
}

#[test]
fn sur_mille_parties_plus_rien_n_est_saute_parce_que_tout_est_accorde() {
    // La garde anti-débranchement : `skipped == 0` ne vaut que si `granted > 0`.
    let db = db_dec();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 300, 2024, &mut pol);
    assert_eq!(s.phase_upgrades_skipped, 0);
    assert!(s.phase_upgrades_granted > 0, "des améliorations sont accordées");
    assert!(s.upgraded_bonus_applied > 0, "et des bonus améliorés sont lus");
    assert_eq!(s.invariant_violations, 0);
    assert_eq!(s.truncated, 0);
}

#[test]
fn en_boite_de_base_le_mecanisme_ne_s_eveille_jamais() {
    // Contre-témoin : aucune carte de la boîte de base n'améliore une phase.
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 300, 2024, &mut pol);
    assert_eq!(s.phase_upgrades_granted, 0);
    assert_eq!(s.phase_upgrades_reupgraded, 0);
    assert_eq!(s.upgraded_bonus_applied, 0);
    assert_eq!(s.upgraded_extra_builds, 0);
    assert_eq!(s.visionary_award_points, 0, "VISIONNAIRE n'entre pas dans la réserve");
}

#[test]
fn effets_coupes_les_cinq_compteurs_restent_nuls() {
    let mut db = db_dec();
    db.effects_on = false;
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 200, 2024, &mut pol);
    assert_eq!(s.phase_upgrades_granted, 0);
    assert_eq!(s.phase_upgrades_reupgraded, 0);
    assert_eq!(s.upgraded_bonus_applied, 0);
    assert_eq!(s.upgraded_extra_builds, 0);
    assert_eq!(s.visionary_award_points, 0);
}

#[test]
fn le_deroulement_de_la_boite_de_base_n_a_pas_bouge() {
    // Le témoin de non-régression le plus dur : à graine fixe, la boîte de base
    // rend exactement l'empreinte de référence. Repère REFIXÉ le 04-08
    // (moteur-questions-manquantes) : l'action standard de vente a quitté la
    // phase Action — la liste d'options tirée par `RandomPolicy` change, et le
    // tirage par défaut de `sell_card` disparaît avec la question. Repères
    // précédents : 7dda3ea2e9b2901b (03-08), c1c52fcbe4e057b0 (01-08),
    // d6a7267472501b13 (31-07).
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 2024, &mut pol);
    assert_eq!(
        format!("{:016x}", s.state_hash),
        "bf70799ff3fee1d8",
        "la boîte de base doit se dérouler exactement comme la mesure de référence"
    );
}

#[test]
fn le_deroulement_avec_decouverte_est_deterministe() {
    let db = db_dec();
    let mut a = RandomPolicy;
    let mut b = RandomPolicy;
    let s1 = run_simulation(&db, 200, 7, &mut a);
    let s2 = run_simulation(&db, 200, 7, &mut b);
    assert_eq!(s1.state_hash, s2.state_hash, "même graine, même partie");
    assert_eq!(s1.phase_upgrades_granted, s2.phase_upgrades_granted);
}

#[test]
fn le_mecanisme_change_reellement_le_deroulement() {
    // Sans le mécanisme, base+Découverte rendait `a92abe276c683961`.
    let db = db_dec();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 1000, 2024, &mut pol);
    assert_ne!(
        format!("{:016x}", s.state_hash),
        "a92abe276c683961",
        "le mécanisme doit changer le cours des parties, pas seulement des compteurs"
    );
}

// =========================================================================
// 6. LA RÉCOMPENSE VISIONNAIRE
// =========================================================================

#[test]
fn visionnaire_est_la_septieme_tuile() {
    assert_eq!(AWARD_POOL.len(), 7);
    assert!(AWARD_POOL.contains(&AwardKind::Visionary));
}

#[test]
fn visionnaire_n_entre_en_jeu_qu_avec_la_boite_qui_l_apporte() {
    let base = award_pool(&db());
    let dec = award_pool(&db_dec());
    assert!(!base.contains(&AwardKind::Visionary), "la boîte de base ne l'a pas");
    assert_eq!(base.len(), 6);
    assert!(dec.contains(&AwardKind::Visionary));
    assert_eq!(dec.len(), 7);
}

#[test]
fn visionnaire_ne_sort_pas_quand_les_effets_sont_coupes() {
    // Sans la couche d'effets, aucune carte ne peut améliorer une phase : la
    // tuile serait une égalité à zéro, distribuant 4 PV à chacun pour rien.
    let mut db = db_dec();
    db.effects_on = false;
    assert!(!award_pool(&db).contains(&AwardKind::Visionary));
}

// =========================================================================
// 7. LA SONDE — elle LIT le point de calcul, elle ne le refait pas
// =========================================================================

#[test]
fn la_sonde_rend_les_ameliorations_installees() {
    let db = db();
    let mut o = ProbeOptions::default();
    o.upgrades[0] = Some(PhaseUpgrade::VariantB);
    o.upgrades[4] = Some(PhaseUpgrade::VariantA);
    let r = seq(&db, &["Lichen"], o);
    assert_eq!(r.upgrades, vec!["1B".to_string(), "5A".to_string()]);
}

#[test]
fn sans_option_la_sonde_ne_rend_aucune_amelioration() {
    let db = db();
    let r = seq(&db, &["Lichen"], ProbeOptions::default());
    assert!(r.upgrades.is_empty(), "sortie des lots précédents, à ce champ près");
    assert_eq!(r.selector_bonus.phase, 0);
    assert_eq!(r.selector_bonus.mc_discount, 0);
    assert_eq!(r.selector_bonus.upgraded, None);
}

#[test]
fn la_sonde_rend_le_bonus_de_la_phase_demandee() {
    let db = db();
    let mut o = ProbeOptions::default();
    o.phase = 1;
    let base = seq(&db, &["Lichen"], o);
    assert_eq!(base.selector_bonus.mc_discount, 3);
    o.upgrades[0] = Some(PhaseUpgrade::VariantA);
    let up = seq(&db, &["Lichen"], o);
    assert_eq!(up.selector_bonus.mc_discount, 6);
    assert_eq!(up.selector_bonus.upgraded, Some(PhaseUpgrade::VariantA));
}

#[test]
fn le_bonus_rendu_par_la_sonde_est_celui_du_point_de_calcul() {
    // La sonde ne recalcule rien : pour les cinq phases et les trois états
    // (base, A, B), sa valeur est exactement `flow::selector_bonus`.
    let db = db();
    for ph in 1u8..=5 {
        for up in [None, Some(PhaseUpgrade::VariantA), Some(PhaseUpgrade::VariantB)] {
            let mut o = ProbeOptions::default();
            o.phase = ph;
            if let Some(v) = up {
                o.upgrades[ph as usize - 1] = Some(v);
            }
            let r = seq(&db, &["Lichen"], o);
            let attendu = selector_bonus(&db, &joueur(ph, up.map(|v| (ph, v))), ph);
            assert_eq!(r.selector_bonus.mc_discount, attendu.mc_discount, "phase {ph}");
            assert_eq!(r.selector_bonus.mc, attendu.mc, "phase {ph}");
            assert_eq!(r.selector_bonus.draw, attendu.draw, "phase {ph}");
            assert_eq!(
                r.selector_bonus.extra_activations, attendu.extra_activations,
                "phase {ph}"
            );
            assert_eq!(r.selector_bonus.extra_builds, attendu.extra_builds, "phase {ph}");
            assert_eq!(r.selector_bonus.research_draw, attendu.research_draw, "phase {ph}");
            assert_eq!(r.selector_bonus.research_keep, attendu.research_keep, "phase {ph}");
            assert_eq!(r.selector_bonus.upgraded, attendu.upgraded, "phase {ph}");
        }
    }
}

#[test]
fn la_sonde_ne_compte_jamais_un_bonus_applique() {
    // `upgraded_bonus_applied` est un compteur de PARTIE : la sonde le laisse
    // à zéro, sinon il mesurerait l'observation et non le jeu.
    let db = db_dec();
    let mut pol = RandomPolicy;
    let g = setup_game(&db, 3, &mut pol);
    assert_eq!(g.upgraded_bonus_applied, 0);
    let mut o = ProbeOptions::default();
    o.phase = 1;
    o.upgrades[0] = Some(PhaseUpgrade::VariantA);
    let r = seq(&db, &["Lichen"], o);
    assert_eq!(r.selector_bonus.mc_discount, 6, "la sonde lit, elle ne compte pas");
}

#[test]
fn la_sonde_n_installe_rien_chez_l_adversaire() {
    let db = db();
    let mut o = ProbeOptions::default();
    o.upgrades[2] = Some(PhaseUpgrade::VariantB);
    let r = seq(&db, &["Lichen"], o);
    assert_eq!(r.upgrades, vec!["3B".to_string()]);
    // L'état de départ de la sonde ne donne rien au joueur 1 : c'est le contrat
    // de `probe_state_base`, vérifié par le fait qu'aucun bonus ne dépend de lui.
    let mut o2 = ProbeOptions::default();
    o2.phase = 3;
    assert_eq!(seq(&db, &["Lichen"], o2).selector_bonus.extra_activations, 1);
}

#[test]
fn la_sonde_sans_phase_ne_change_pas_les_sondes_existantes() {
    // Les deux champs neufs mis à part, une sonde sans option nouvelle rend
    // exactement ce qu'elle rendait.
    let db = db();
    let a = seq(&db, &["Lichen"], ProbeOptions::default());
    let mut o = ProbeOptions::default();
    o.upgrades[3] = Some(PhaseUpgrade::VariantA);
    let b = seq(&db, &["Lichen"], o);
    assert_eq!(a.delta, b.delta, "une amélioration de phase IV ne pose pas la carte");
    assert_eq!(a.paid, b.paid);
    assert_eq!(a.vp, b.vp);
}

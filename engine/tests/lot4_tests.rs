//! Tests du lot 4 « productions dérivées » (chantier moteur-cartes-4).
//!
//! Un test par carte du lot (17) vérifiant l'ÉTAT DE JEU résultant contre le
//! TEXTE IMPRIMÉ — pas contre la table d'encodage — plus des tests
//! d'intégration : recalcul à chaque phase (le point central du lot), division
//! entière, non-inscription sur les pistes de production, service unique,
//! bonus permanent de recherche, compteurs d'audit en flux réel, interrupteur
//! `--effects off`, rétro-compatibilité de la sonde et déterminisme.
//!
//! La sonde utilisée est `run_probe_seq_full` avec `produce = true` : elle pose
//! la séquence par le chemin de `simulate` (`flow::build_card_with`) puis
//! exécute la VRAIE phase IV du moteur (`flow::phase_production`). Le champ
//! `derived_prod` est relevé sur les compteurs incrémentés à l'endroit du
//! crédit — aucun test ne recalcule la production dérivée pour son compte.

use engine::cards::{CardsDb, Tag};
use engine::effects::{ProdCount, ProdRes};
use engine::flow::{
    derived_production, install_corporation, play_round, research_base, research_draw_keep,
    research_extra, setup_game,
};
use engine::policy::{ActionOpt, ConstructionBonus, Policy, RandomPolicy};
use engine::probe::{run_probe_seq_full, ProbeOptions, ProbeResult, ProbeScript};
use engine::sim::run_simulation;
use engine::state::PlayerState;
use rand::rngs::StdRng;

fn db() -> CardsDb {
    CardsDb::load("../data/cards.json").expect("cards.json doit se charger")
}

/// Sonde séquence SANS production (comportement des lots précédents).
fn seq(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(db, names, ProbeOptions::default(), &ProbeScript::default(), false)
}

/// Sonde séquence PUIS vraie phase IV de production (`--probe-produce`).
fn produce(db: &CardsDb, names: &[&str]) -> ProbeResult {
    run_probe_seq_full(db, names, ProbeOptions::default(), &ProbeScript::default(), true)
}

/// Production dérivée créditée par la phase IV : (MC, chaleur, plantes).
fn derived(db: &CardsDb, names: &[&str]) -> (i64, i64, i64) {
    let r = produce(db, names);
    assert!(r.produced, "la phase de production doit avoir été exécutée");
    r.derived_prod
}

// ===================================================== 14 productions dérivées
//
// Chaque carte est confrontée à son texte imprimé : la ressource produite, ce
// qui est compté, et le diviseur. Les cartes portant elles-mêmes le badge
// compté produisent 1 dès qu'elles sont seules en jeu — c'est exactement ce que
// veut dire « including this », sans traitement particulier.

#[test]
fn atmospheric_insulators_produces_one_heat_per_earth_tag() {
    let db = db();
    // « produces 1 heat per Earth you have, including this » — la carte porte
    // un badge Terre (et un badge Espace, qui ne compte pas ici).
    assert_eq!(derived(&db, &["Atmospheric Insulators"]), (0, 1, 0));
    // Un badge Terre de plus (Media Group) : 2 chaleur.
    assert_eq!(derived(&db, &["Atmospheric Insulators", "Media Group"]), (0, 2, 0));
}

#[test]
fn cartel_produces_one_mc_per_earth_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Cartel"]), (1, 0, 0));
    assert_eq!(derived(&db, &["Cartel", "Media Group"]), (2, 0, 0));
}

#[test]
fn insects_counts_plant_tags_it_does_not_carry() {
    let db = db();
    // « produces 1 plant per Plant you have » : la carte porte un badge MICROBE,
    // pas Plante. Seule, elle ne produit donc rien.
    assert_eq!(derived(&db, &["Insects"]), (0, 0, 0));
    // Viral Enhancers porte Microbe ET Plante : seul le badge Plante compte.
    assert_eq!(derived(&db, &["Insects", "Viral Enhancers"]), (0, 0, 1));
}

#[test]
fn lightning_harvest_produces_one_mc_per_science_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Lightning Harvest"]), (1, 0, 0));
}

#[test]
fn medical_lab_divides_building_tags_by_two() {
    let db = db();
    // « 1 MC per 2 Building you have » : un seul badge ne rapporte rien
    // (division ENTIÈRE).
    assert_eq!(derived(&db, &["Medical Lab"]).0, 0);
    // Windmills apporte un 2e badge Construction → 1 MC ; elle produit aussi
    // 1 chaleur par badge Énergie (le sien).
    assert_eq!(derived(&db, &["Medical Lab", "Windmills"]), (1, 1, 0));
    // Un 3e badge Construction ne suffit pas à passer à 2 MC.
    let three = derived(&db, &["Medical Lab", "Windmills", "Power Grid"]);
    // Power Grid : Construction + Énergie → 3 badges Construction (1 MC), et
    // elle-même produit 1 MC par badge Énergie (2 en jeu) = 2 MC.
    assert_eq!(three.0, 1 + 2, "3 badges Construction = 1 MC, Power Grid = 2 MC");
}

#[test]
fn microbiology_patents_produces_one_mc_per_microbe_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Microbiology Patents"]), (1, 0, 0));
}

#[test]
fn miranda_resort_produces_one_mc_per_earth_tag() {
    let db = db();
    // La carte porte Espace, Terre et Jupiter : seul le badge Terre compte.
    assert_eq!(derived(&db, &["Miranda Resort"]), (1, 0, 0));
}

#[test]
fn power_grid_produces_one_mc_per_energy_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Power Grid"]), (1, 0, 0));
}

#[test]
fn sattellite_farms_produces_one_heat_per_space_tag() {
    let db = db();
    // Nom orthographié « Sattellite Farms » dans cards.json (faute d'origine).
    assert_eq!(derived(&db, &["Sattellite Farms"]), (0, 1, 0));
}

#[test]
fn satellites_produces_one_mc_per_space_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Satellites"]), (1, 0, 0));
    // Sattellite Farms ajoute un badge Espace : 2 MC pour Satellites, et
    // 2 chaleur pour Sattellite Farms.
    assert_eq!(derived(&db, &["Satellites", "Sattellite Farms"]), (2, 2, 0));
}

#[test]
fn venture_capitalism_counts_event_tags_and_carries_none() {
    let db = db();
    // « 1 MC per Event you have » — la carte ne porte aucun badge.
    assert_eq!(derived(&db, &["Venture Capitalism"]), (0, 0, 0));
    // Comet porte Espace + Événement.
    assert_eq!(derived(&db, &["Venture Capitalism", "Comet"]).0, 1);
}

#[test]
fn windmills_produces_one_heat_per_energy_tag() {
    let db = db();
    assert_eq!(derived(&db, &["Windmills"]), (0, 1, 0));
}

#[test]
fn worms_produces_one_plant_per_microbe_tag_and_requires_red_oxygen() {
    let db = db();
    // Prérequis « oxygène rouge ou plus » : non rempli à l'état de sonde.
    assert!(!seq(&db, &["Worms"]).prereq_ok, "Worms exige l'oxygène rouge");
    // Pose forcée : elle porte un badge Microbe → 1 plante.
    assert_eq!(derived(&db, &["Worms"]), (0, 0, 1));
    assert_eq!(derived(&db, &["Worms", "Microbiology Patents"]).2, 2);
}

#[test]
fn zeppelins_counts_forest_tokens_not_tags() {
    let db = db();
    assert!(!seq(&db, &["Zeppelins"]).prereq_ok, "Zeppelins exige l'oxygène rouge");
    // Aucune forêt à l'état de sonde → aucune production, quels que soient les
    // badges en jeu (elle ne compte pas de badges du tout).
    assert_eq!(derived(&db, &["Zeppelins"]), (0, 0, 0));
    assert_eq!(derived(&db, &["Zeppelins", "Media Group", "Satellites"]).0, 1,
        "seul Satellites produit (1 badge Espace) : Zeppelins ignore les badges");
    // Et la table dit bien « jetons Forêt », pas un badge.
    let spec = db.projects[db.resolve_card("Zeppelins").unwrap() as usize]
        .effect
        .expect("Zeppelins encodée")
        .prod
        .expect("Zeppelins porte une production dérivée");
    assert_eq!(spec.count, ProdCount::Forests);
    assert_eq!(spec.res, ProdRes::Mc);
    assert_eq!(spec.per, 1);
}

#[test]
fn zeppelins_produces_one_mc_per_forest() {
    let db = db();
    // Un joueur avec Zeppelins en jeu et 3 forêts (état parfaitement atteignable
    // en partie réelle : les forêts viennent des actions de la phase III). La
    // carte entre en jeu par `put_in_play`, le chemin du moteur.
    let mut pl = PlayerState::new();
    let id = db.resolve_card("Zeppelins").expect("Zeppelins résolue");
    pl.put_in_play(id, &db);
    assert_eq!(derived_production(&db, &pl), (0, 0, 0), "sans forêt, rien");
    pl.forests = 3;
    assert_eq!(derived_production(&db, &pl), (3, 0, 0), "1 MC par forêt");
}

// ==================================================== les 3 autres cartes

#[test]
fn immigration_shuttles_is_a_fixed_production_of_three_mc() {
    let db = db();
    let r = produce(&db, &["Immigration Shuttles"]);
    // Production FIXE : elle est inscrite sur la piste `mc_prod`…
    assert_eq!(r.delta.mc_prod, 3);
    // …et n'est donc PAS comptée comme production dérivée.
    assert_eq!(r.derived_prod, (0, 0, 0));
    // MC encaissés pendant la phase IV : 3 de production + 5 de NT.
    assert_eq!(r.delta.mc, 8);
}

#[test]
fn immigration_shuttles_victory_points_are_not_recomputed() {
    let db = db();
    // « 1 VP per 2 Earth tags you have » : déjà calculé par `card_points`
    // (vp_dynamic EARTH 1/2). Seule, la carte a 1 badge Terre → 0 point.
    assert_eq!(seq(&db, &["Immigration Shuttles"]).vp_total, 0);
    // Avec Cartel (badge Terre) : 2 badges Terre → 1 point.
    assert_eq!(seq(&db, &["Cartel", "Immigration Shuttles"]).vp_total, 1);
    // Et la carte ne porte aucune production dérivée dans la table.
    let spec = db.projects[db.resolve_card("Immigration Shuttles").unwrap() as usize]
        .effect
        .expect("Immigration Shuttles encodée");
    assert!(spec.prod.is_none(), "sa production est FIXE, pas dérivée");
}

#[test]
fn terraforming_ganymede_raises_tr_one_step_per_jupiter_tag() {
    let db = db();
    // « Raise your TR 1 step per Jupiter tag you have, including this » : la
    // carte est mise en jeu AVANT l'application de ses effets, son badge compte.
    assert_eq!(seq(&db, &["Terraforming Ganymede"]).delta.tr, 1);
    // Io Mining Industries porte un badge Jupiter : 2 pas.
    assert_eq!(
        seq(&db, &["Io Mining Industries", "Terraforming Ganymede"]).delta.tr,
        2
    );
    // Miranda Resort aussi (Espace + Terre + Jupiter) : 3 pas.
    assert_eq!(
        seq(&db, &["Io Mining Industries", "Miranda Resort", "Terraforming Ganymede"])
            .delta
            .tr,
        3
    );
}

#[test]
fn terraforming_ganymede_reads_the_tags_at_application_time_only() {
    let db = db();
    // Un badge Jupiter posé APRÈS la carte n'ajoute rien : le nombre de pas est
    // lu au moment de l'application, pas plus tard (ce n'est pas un effet
    // permanent, contrairement à la production dérivée).
    assert_eq!(
        seq(&db, &["Terraforming Ganymede", "Io Mining Industries"]).delta.tr,
        1
    );
}

#[test]
fn interplanetary_relations_grants_one_extra_draw_and_keep() {
    let db = db();
    let mut pl = PlayerState::new();
    let id = db.resolve_card("Interplanetary Relations").expect("carte résolue");
    // Sans la carte : le livret nu.
    assert_eq!(research_extra(&db, &pl), (0, 0));
    assert_eq!(research_draw_keep(&db, &pl), (2, 1));
    pl.put_in_play(id, &db);
    assert_eq!(research_extra(&db, &pl), (1, 1));
    // Joueur ordinaire : 2/1 → 3/2.
    assert_eq!(research_draw_keep(&db, &pl), (3, 2));
    // Sélectionneur de la phase 5 : 5/2 → 6/3.
    pl.chosen_phase = 5;
    assert_eq!(research_base(&db, &pl), (5, 2));
    assert_eq!(research_draw_keep(&db, &pl), (6, 3));
}

#[test]
fn a_second_research_bonus_adds_again() {
    let db = db();
    // Le service SOMME les cartes en jeu : deux exemplaires donneraient +2/+2.
    let mut pl = PlayerState::new();
    let id = db.resolve_card("Interplanetary Relations").expect("carte résolue");
    pl.put_in_play(id, &db);
    pl.put_in_play(id, &db);
    assert_eq!(research_extra(&db, &pl), (2, 2));
    assert_eq!(research_draw_keep(&db, &pl), (4, 3));
}

// ============================================ LE point du lot : le recalcul

#[test]
fn derived_production_follows_tags_gained_after_the_card_was_played() {
    let db = db();
    // Cartel posée SEULE d'abord, puis un badge Terre : à la production, elle
    // voit 2 badges. Une production figée à la pose en verrait toujours 1.
    assert_eq!(derived(&db, &["Cartel"]).0, 1);
    assert_eq!(derived(&db, &["Cartel", "Media Group"]).0, 2);
    assert_eq!(
        derived(&db, &["Cartel", "Media Group", "Miranda Resort"]).0,
        // 3 badges Terre : Cartel 3 + Miranda Resort 3.
        6
    );
}

#[test]
fn derived_production_never_touches_the_production_tracks() {
    let db = db();
    // NEVER 1 : rien n'est inscrit sur mc_prod / heat_prod / plant_prod.
    for names in [
        &["Cartel"][..],
        &["Windmills"][..],
        &["Insects", "Viral Enhancers"][..],
        &["Medical Lab", "Power Grid"][..],
    ] {
        let r = produce(&db, names);
        assert_eq!(r.delta.mc_prod, 0, "{names:?} : mc_prod modifié");
        assert_eq!(r.delta.heat_prod, 0, "{names:?} : heat_prod modifié");
        assert_eq!(r.delta.plant_prod, 0, "{names:?} : plant_prod modifié");
    }
}

#[test]
fn derived_production_is_really_credited_to_the_player() {
    let db = db();
    // Cartel : 100 MC − 6 payés, puis phase IV = 5 (NT) + 1 (dérivé).
    let r = produce(&db, &["Cartel"]);
    assert_eq!(r.delta.mc, 6, "5 de NT + 1 de production dérivée");
    // Windmills : la chaleur arrive vraiment.
    assert_eq!(produce(&db, &["Windmills"]).delta.heat, 1);
    // Worms : la plante arrive vraiment. (Insects + Viral Enhancers ne
    // conviendrait pas ici : le déclencheur de pose de Viral Enhancers donne
    // lui aussi des plantes, et `delta.plants` cumule toutes les sources —
    // seul `derived_prod` isole la production dérivée, cf. le test dédié.)
    let worms = produce(&db, &["Worms"]);
    assert_eq!(worms.derived_prod, (0, 0, 1));
    assert_eq!(worms.delta.plants, 1);
}

/// Politique qui force une phase donnée pour les deux joueurs et enregistre
/// les paramètres réellement reçus par la phase Recherche (`drawn`, `keep`).
/// Elle délègue tout le reste à `RandomPolicy` : le flux reste celui du moteur.
struct PhaseForcer {
    inner: RandomPolicy,
    /// Phase imposée manche par manche (le livret interdit de rejouer la même
    /// phase deux manches de suite : une séquence est donc nécessaire).
    phases: Vec<u8>,
    calls: usize,
    /// (joueur, cartes piochées, cartes gardées) à chaque passage en phase V.
    research_seen: Vec<(usize, usize, usize)>,
}

impl PhaseForcer {
    fn new(phases: &[u8]) -> PhaseForcer {
        PhaseForcer {
            inner: RandomPolicy,
            phases: phases.to_vec(),
            calls: 0,
            research_seen: Vec::new(),
        }
    }
}

impl Policy for PhaseForcer {
    fn corp_mulligan(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> bool {
        self.inner.corp_mulligan(r, p, c)
    }
    fn project_mulligan(&mut self, r: &mut StdRng, p: usize, h: &[u16]) -> bool {
        self.inner.project_mulligan(r, p, h)
    }
    fn pick_corporation(&mut self, r: &mut StdRng, p: usize, c: &[u16]) -> usize {
        self.inner.pick_corporation(r, p, c)
    }
    fn pick_phase(&mut self, r: &mut StdRng, p: usize, allowed: &[u8]) -> u8 {
        // Une phase par manche, les deux joueurs (2 appels par manche).
        let want = self.phases[(self.calls / 2) % self.phases.len()];
        self.calls += 1;
        if allowed.contains(&want) {
            want
        } else {
            self.inner.pick_phase(r, p, allowed)
        }
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
    fn research_keep(
        &mut self,
        r: &mut StdRng,
        p: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        // Relevé de ce que la VRAIE phase V a décidé de piocher et de garder.
        self.research_seen.push((p, drawn.len(), keep));
        self.inner.research_keep(r, p, drawn, keep)
    }
    fn discard_down(&mut self, r: &mut StdRng, p: usize, h: &[u16], n: usize) -> Vec<usize> {
        self.inner.discard_down(r, p, h, n)
    }
}

#[test]
fn the_research_bonus_reaches_the_real_phase_five() {
    let db = db();
    // Une partie réelle, les deux joueurs forcés sur la phase V (donc tous deux
    // sélectionneurs : base 5/2). Le joueur 0 a Interplanetary Relations en jeu.
    let mut pol = PhaseForcer::new(&[5]);
    let mut game = setup_game(&db, 4242, &mut pol);
    // (boites-1) ATTENTE PRÉSERVÉE, TIRAGE NEUTRALISÉ. Le test mesure le bonus
    // de recherche d'UNE CARTE ; avec la pioche réelle (208 cartes au lieu de
    // 248) la graine 4242 donne à p0 une corporation qui porte elle aussi un
    // bonus de recherche, et les deux s'additionnaient (7/4 au lieu de 6/3).
    // On installe donc aux deux joueurs, par le service réel, une corporation
    // sans bonus de recherche : la carte redevient la seule variable.
    // Deux corporations DIFFÉRENTES (une partie n'en distribue jamais deux
    // fois la même) et toutes deux sans bonus de recherche — critère positif,
    // lu dans l'encodage, aucun nom cité.
    let neutres: Vec<u16> = db
        .corporations
        .iter()
        .enumerate()
        .filter(|(_, c)| c.effect.map_or(false, |e| e.research.is_none()))
        .map(|(i, _)| i as u16)
        .collect();
    assert!(neutres.len() >= 2, "il faut deux corporations sans bonus de recherche");
    for p in 0..2 {
        game.players[p] = PlayerState::new();
        install_corporation(&mut game, &db, p, neutres[p]);
    }
    assert_ne!(neutres[0], neutres[1]);
    let id = db.resolve_card("Interplanetary Relations").expect("carte résolue");
    game.players[0].put_in_play(id, &db);
    play_round(&mut game, &db, &mut pol);

    let p0: Vec<_> = pol.research_seen.iter().filter(|(p, _, _)| *p == 0).collect();
    let p1: Vec<_> = pol.research_seen.iter().filter(|(p, _, _)| *p == 1).collect();
    assert_eq!(p0.len(), 1, "le joueur 0 passe une fois en phase V");
    assert_eq!(p1.len(), 1, "le joueur 1 passe une fois en phase V");
    // Sélectionneur 5/2, + 1/1 du bonus permanent = 6 piochées / 3 gardées.
    assert_eq!((p0[0].1, p0[0].2), (6, 3), "bonus de PIOCHE et de GARDE appliqués");
    // Le joueur 1 n'a pas la carte : il en reste au bonus du sélectionneur.
    assert_eq!((p1[0].1, p1[0].2), (5, 2));
    assert_eq!(game.research_extra_draws, 1, "une seule carte piochée en plus");
}

#[test]
fn the_research_bonus_applies_to_ordinary_players_too() {
    let db = db();
    // Le bonus suit la CARTE, pas le joueur 0 : ici c'est le joueur 1 qui la
    // porte, et lui seul en profite.
    let mut pol = PhaseForcer::new(&[5]);
    let mut game = setup_game(&db, 77, &mut pol);
    let id = db.resolve_card("Interplanetary Relations").expect("carte résolue");
    game.players[1].put_in_play(id, &db);
    play_round(&mut game, &db, &mut pol);
    let p1 = pol.research_seen.iter().find(|(p, _, _)| *p == 1).expect("joueur 1 vu");
    assert_eq!((p1.1, p1.2), (6, 3), "le bonus suit le porteur de la carte");
    let p0 = pol.research_seen.iter().find(|(p, _, _)| *p == 0).expect("joueur 0 vu");
    assert_eq!((p0.1, p0.2), (5, 2), "et lui seul");
}

#[test]
fn production_is_recomputed_at_every_phase_not_accumulated() {
    let db = db();
    // Deux phases IV successives, en partie RÉELLE (`play_round`), sur un état
    // qui ne bouge pas entre les deux : chacune crédite le MÊME montant. Une
    // production figée à la pose, ou cumulée, ne donnerait pas ce profil.
    //
    // Le livret interdit de rejouer la même phase deux manches de suite : la
    // manche intercalée est une phase V (Recherche), qui ne pose aucune carte
    // et laisse donc les badges — donc la production dérivée — inchangés.
    let mut pol = PhaseForcer::new(&[4, 5, 4]);
    let mut game = setup_game(&db, 909, &mut pol);
    let cartel = db.resolve_card("Cartel").expect("Cartel résolue");
    game.players[0].put_in_play(cartel, &db);
    // Le joueur 1 aussi porte une carte dérivée : la phase IV crédite les DEUX
    // joueurs, pas seulement le premier.
    let farms = db.resolve_card("Sattellite Farms").expect("carte résolue");
    game.players[1].put_in_play(farms, &db);

    let expect_mc = derived_production(&db, &game.players[0]).0;
    let expect_heat = derived_production(&db, &game.players[1]).1;
    assert!(expect_mc > 0 && expect_heat > 0, "les deux joueurs produisent");

    play_round(&mut game, &db, &mut pol);
    let after_one = (game.derived_mc, game.derived_heat);
    assert_eq!(after_one, (expect_mc as u64, expect_heat as u64));

    // Manche intercalée : phase V, aucune production.
    play_round(&mut game, &db, &mut pol);
    assert_eq!(
        (game.derived_mc, game.derived_heat),
        after_one,
        "une manche sans phase IV ne crédite aucune production dérivée"
    );

    play_round(&mut game, &db, &mut pol);
    assert_eq!(
        (game.derived_mc, game.derived_heat),
        (after_one.0 * 2, after_one.1 * 2),
        "la 2e phase crédite autant que la 1re — ni cumul, ni gel"
    );
}

#[test]
fn integer_division_never_rounds_up() {
    let db = db();
    let id = db.resolve_card("Medical Lab").expect("carte résolue");
    let mut pl = PlayerState::new();
    pl.put_in_play(id, &db);
    for expected in [0, 1, 1, 2, 2] {
        assert_eq!(derived_production(&db, &pl).0, expected);
        pl.tag_counts[Tag::Building.index().unwrap()] += 1;
    }
}

// ================================================== sonde : nouveaux champs

#[test]
fn probe_produce_is_off_by_default_and_reports_zero() {
    let db = db();
    let r = seq(&db, &["Cartel", "Media Group"]);
    assert!(!r.produced, "sans --probe-produce, aucune phase IV");
    assert_eq!(r.derived_prod, (0, 0, 0));
    // Et l'état n'a pas bougé du fait d'une production : 5 NT non encaissés.
    assert_eq!(r.delta.mc, 0);
}

#[test]
fn vp_total_sums_card_points_over_every_card_in_play() {
    let db = db();
    // Points imprimés seuls : Lightning Harvest 1 + Medical Lab 1.
    assert_eq!(seq(&db, &["Lightning Harvest", "Medical Lab"]).vp_total, 2);
    // Points variables par badge (Immigration Shuttles, 1 PV / 2 badges Terre)
    // et par carte jouée (Interplanetary Relations, 1 PV / 4 cartes jouées).
    let r = seq(&db, &["Cartel", "Media Group", "Immigration Shuttles"]);
    assert_eq!(r.vp_total, 1, "3 badges Terre → 1 PV");
    // Le champ `vp` historique ne porte, lui, que sur la DERNIÈRE carte : il ne
    // change pas de sens (rétro-compatibilité).
    assert_eq!(r.vp, 0, "Immigration Shuttles n'a aucun PV imprimé");
}

#[test]
fn probe_stays_backward_compatible_on_earlier_lots() {
    let db = db();
    // Lot 2 : réduction Média Group sur les Événements.
    let r = seq(&db, &["Media Group", "Lichen"]);
    assert_eq!(r.paid, vec![11, 5]);
    assert!(!r.produced);
    // Lot 1 : effets simples inchangés.
    let comet = seq(&db, &["Comet"]);
    assert_eq!(comet.delta.temperature, 1);
    assert_eq!(comet.delta.oceans, 1);
}

#[test]
fn probe_sequence_stays_deterministic() {
    let db = db();
    for names in [&["Cartel", "Media Group"][..], &["Medical Lab", "Windmills"][..]] {
        let a = produce(&db, names);
        let b = produce(&db, names);
        assert_eq!(a.derived_prod, b.derived_prod);
        assert_eq!(a.delta, b.delta);
        assert_eq!(a.vp_total, b.vp_total);
    }
}

// ================================================== interrupteur --effects off

#[test]
fn effects_off_neutralises_every_lot4_mechanism() {
    let mut db = db();
    db.effects_on = false;
    let mut pl = PlayerState::new();
    for n in ["Cartel", "Windmills", "Interplanetary Relations"] {
        pl.put_in_play(db.resolve_card(n).expect("carte résolue"), &db);
    }
    assert_eq!(derived_production(&db, &pl), (0, 0, 0));
    assert_eq!(research_extra(&db, &pl), (0, 0));
    assert_eq!(research_draw_keep(&db, &pl), (2, 1), "le livret nu subsiste");
    // Sonde : la phase IV a bien lieu (le NT est une règle), mais elle ne
    // crédite aucune production dérivée.
    let r = produce(&db, &["Cartel", "Media Group"]);
    assert_eq!(r.derived_prod, (0, 0, 0));
    assert!(r.produced);
}

// ============================================ compteurs d'audit en flux réel

#[test]
fn audit_counters_grow_in_real_games_and_are_zero_with_effects_off() {
    let mut db = db();
    let mut pol = RandomPolicy;
    let on = run_simulation(&db, 400, 7, &mut pol);
    assert!(on.derived_mc > 0, "derived_mc = {}", on.derived_mc);
    assert!(on.derived_heat > 0, "derived_heat = {}", on.derived_heat);
    assert!(on.derived_plants > 0, "derived_plants = {}", on.derived_plants);
    assert!(on.tr_from_tags > 0, "tr_from_tags = {}", on.tr_from_tags);
    assert!(
        on.research_extra_draws > 0,
        "research_extra_draws = {}",
        on.research_extra_draws
    );
    // Les MC dominent : 10 des 14 cartes produisent des MC.
    assert!(on.derived_mc > on.derived_heat && on.derived_mc > on.derived_plants);

    db.effects_on = false;
    let off = run_simulation(&db, 400, 7, &mut pol);
    assert_eq!(off.derived_mc, 0);
    assert_eq!(off.derived_heat, 0);
    assert_eq!(off.derived_plants, 0);
    assert_eq!(off.tr_from_tags, 0);
    assert_eq!(off.research_extra_draws, 0);
}

#[test]
fn audit_counters_scale_with_the_number_of_games() {
    let db = db();
    let mut pol = RandomPolicy;
    let small = run_simulation(&db, 200, 11, &mut pol);
    let big = run_simulation(&db, 600, 11, &mut pol);
    assert!(big.derived_mc > small.derived_mc);
    assert!(big.tr_from_tags > small.tr_from_tags);
    assert!(big.research_extra_draws > small.research_extra_draws);
}

#[test]
fn full_games_stay_deterministic_and_healthy_with_the_new_mechanisms() {
    let db = db();
    let mut pol = RandomPolicy;
    let a = run_simulation(&db, 300, 2024, &mut pol);
    let b = run_simulation(&db, 300, 2024, &mut pol);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.derived_mc, b.derived_mc);
    assert_eq!(a.research_extra_draws, b.research_extra_draws);
    assert_eq!(a.completed, 300);
    assert_eq!(a.invariant_violations, 0);
    assert_eq!(a.truncated, 0);
}

#[test]
fn earlier_lot_counters_are_untouched() {
    let db = db();
    let mut pol = RandomPolicy;
    let s = run_simulation(&db, 400, 7, &mut pol);
    assert!(s.res_added > 0 && s.res_removed > 0);
    assert!(s.blue_actions > 0);
    assert!(s.discard_payments > 0);
}

// ====================================================== intégrité de la table

#[test]
fn the_seventeen_cards_are_encoded_and_resolve_to_the_v1_deck() {
    let db = db();
    const LOT4: [&str; 17] = [
        "Atmospheric Insulators", "Cartel", "Insects", "Lightning Harvest",
        "Medical Lab", "Microbiology Patents", "Miranda Resort", "Power Grid",
        "Sattellite Farms", "Satellites", "Venture Capitalism", "Windmills",
        "Worms", "Zeppelins", "Immigration Shuttles", "Terraforming Ganymede",
        "Interplanetary Relations",
    ];
    for name in LOT4 {
        let id = db.resolve_card(name).unwrap_or_else(|| panic!("{name} non résolue"));
        let card = &db.projects[id as usize];
        assert!(card.in_deck_v1, "{name} doit venir du deck v1");
        assert!(card.effect.is_some(), "{name} doit être encodée");
    }
    // 110 (lots 1-2) + 28 (ressources) + 17 (lot 4) + 33 (lot 5) + 11 (lot 6)
    // + 4 (lot acier-titane) + 9 (lot cartes-7) + 5 (lot cartes-8)
    // + 28 (decouverte-projets) = 245 entrées.
    // ATTENTE MISE À JOUR par le lot 5 (155 → 188), le lot 6 (188 → 199), le
    // lot acier-titane (199 → 203), le lot cartes-7 (203 → 212), le lot
    // cartes-8 (212 → 217) puis `decouverte-projets` (217 → 245) : taille
    // EXACTE toujours épinglée, aucun test supprimé ni assoupli.
    assert_eq!(engine::effects::LOT1.len(), 245);
}

#[test]
fn exactly_fourteen_cards_carry_a_derived_production() {
    let n = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| e.prod.is_some())
        .count();
    assert_eq!(n, 14, "14 productions dérivées, pas une de plus");
    let r = engine::effects::LOT1
        .iter()
        .filter(|(_, e)| e.research.is_some())
        .count();
    // (lot cartes-7) 1 (Interplanetary Relations, lot 4) + 3 (Interns, Extended
    // Resources, United Planetary Alliance) = 4. ATTENTE MISE À JOUR, pas
    // assouplie : le compte reste EXACT, et le lot n'ajoute aucune production
    // dérivée — le premier assert le prouve, il n'a pas bougé.
    assert_eq!(r, 4, "quatre bonus permanents de recherche, pas un de plus");
}

#[test]
fn no_earlier_lot_card_gained_a_derived_production() {
    let db = db();
    // Les 138 cartes des lots précédents restent sans production dérivée et
    // sans bonus de recherche : aucune régression par effet de bord.
    for name in [
        "Media Group", "Tardigrades", "Birds", "Symbiotic Fungus",
        "Io Mining Industries", "Volcanic Pools", "Lichen", "Comet",
    ] {
        let spec = db.projects[db.resolve_card(name).unwrap() as usize]
            .effect
            .unwrap_or_else(|| panic!("{name} doit rester encodée"));
        assert!(spec.prod.is_none(), "{name} a gagné une production dérivée");
        assert!(spec.research.is_none(), "{name} a gagné un bonus de recherche");
    }
}

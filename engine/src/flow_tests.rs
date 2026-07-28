// =============================================================================
// (lot acier-titane) Tests des CAS LIMITES des deux effets d'action nouveaux.
//
// Ils vivent ici, et pas dans `tests/`, parce qu'ils exigent un état de jeu que
// la sonde ne sait pas fabriquer (océans épuisés, oxygène au maximum) et un
// appel à `apply_blue_action`, qui est `pub(crate)`. L'état est construit par
// `setup_game` — le chemin de mise en place réel — puis poussé à la borne :
// aucune structure fabriquée à la main.
// =============================================================================


use super::*;
use crate::boites::BoiteSet;
use crate::policy::RandomPolicy;

fn db() -> CardsDb {
    CardsDb::load_boites("../data/cards.json", BoiteSet::parse("base").unwrap())
        .expect("cards.json")
}

/// Met la carte nommée en jeu pour le joueur 0, par le chemin réel de mise
/// en jeu (`put_in_play` + `refresh_capacities`), et lui donne de quoi
/// payer.
/// Part d'un joueur NEUF : la corporation tirée par la mise en place peut
/// elle-même porter un savoir-faire, ce qui brouillerait le compte mesuré.
fn joueur_neuf(game: &mut GameState) {
    game.players[0].played.clear();
    game.players[0].corporation = None;
    game.players[0].steel_capacity = 0;
    game.players[0].titanium_capacity = 0;
}

fn poser(game: &mut GameState, db: &CardsDb, nom: &str) -> u16 {
    let id = db.resolve_card(nom).unwrap_or_else(|| panic!("{nom}"));
    game.players[0].put_in_play(id, db);
    refresh_capacities(game, db, 0);
    game.players[0].mc = 100;
    id
}

#[test]
fn une_action_ocean_ne_se_paie_pas_quand_il_n_y_a_plus_d_ocean() {
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 7, &mut pol);
    joueur_neuf(&mut game);
    let id = poser(&mut game, &db, "Aquifer Pumping");

    // Il reste des océans : l'action s'applique et coûte ses 10 MC — moins
    // le bonus en MC de la tuile retournée, qui est un GAIN de l'océan.
    game.oceans_revealed = 0;
    game.snapshot_planet();
    let bonus = game.oceans[0].mc;
    let avant = game.players[0].mc;
    assert!(apply_blue_action(&mut game, &db, 0, id, &mut pol));
    assert_eq!(game.players[0].mc, avant - 10 + bonus);
    assert_eq!(game.oceans_revealed, 1);

    // Plus aucun océan : l'action ne s'applique pas, et surtout elle ne
    // prélève RIEN — on ne paie jamais pour rien.
    game.oceans_revealed = NUM_OCEANS;
    game.snapshot_planet();
    let avant = game.players[0].mc;
    let tr = game.players[0].tr;
    assert!(!apply_blue_action(&mut game, &db, 0, id, &mut pol));
    assert_eq!(game.players[0].mc, avant, "aucun MC prélevé");
    assert_eq!(game.players[0].tr, tr, "aucun NT accordé");
}

#[test]
fn une_action_foret_reste_jouable_oxygene_au_maximum() {
    // Le jeton PV Forêt est gagné même quand l'oxygène ne peut plus monter
    // (livret p. 14, l. 391) : l'action garde son intérêt, elle se paie.
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 11, &mut pol);
    joueur_neuf(&mut game);
    let id = poser(&mut game, &db, "Solarpunk");
    game.oxygen = OXYGEN_MAX;
    game.snapshot_planet();
    let (mc, tr, forets) = (
        game.players[0].mc,
        game.players[0].tr,
        game.players[0].forests,
    );
    assert!(apply_blue_action(&mut game, &db, 0, id, &mut pol));
    assert_eq!(game.players[0].mc, mc - 15);
    assert_eq!(game.players[0].forests, forets + 1, "le jeton est gagné");
    assert_eq!(game.oxygen, OXYGEN_MAX, "l'oxygène ne dépasse pas la borne");
    assert_eq!(game.players[0].tr, tr, "pas d'oxygène, donc pas de NT");
}

#[test]
fn une_action_a_cout_reduit_reste_impayable_sans_les_mc() {
    // La réduction par savoir-faire ne rend pas l'action gratuite : sans les
    // MC, elle ne s'applique pas et ne prélève rien.
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 3, &mut pol);
    joueur_neuf(&mut game);
    let id = poser(&mut game, &db, "Solarpunk");
    poser(&mut game, &db, "Titanium Mine"); // 1 titane → coût 13
    game.snapshot_planet();
    game.players[0].mc = 12;
    assert!(!apply_blue_action(&mut game, &db, 0, id, &mut pol));
    assert_eq!(game.players[0].mc, 12, "rien prélevé");
    game.players[0].mc = 13;
    assert!(apply_blue_action(&mut game, &db, 0, id, &mut pol));
    assert_eq!(game.players[0].mc, 0);
}

#[test]
fn le_compte_suit_la_mise_en_jeu_dans_le_flux_reel() {
    // `refresh_capacities` est bien appelée au bon endroit : le compte est
    // juste sans qu'on ait rien recalculé.
    let db = db();
    let mut pol = RandomPolicy;
    let mut game = setup_game(&db, 5, &mut pol);
    joueur_neuf(&mut game);
    poser(&mut game, &db, "Strip Mine");
    assert_eq!(
        (game.players[0].steel_capacity, game.players[0].titanium_capacity),
        (2, 1)
    );
    poser(&mut game, &db, "Space Station");
    assert_eq!(game.players[0].titanium_capacity, 2);
}

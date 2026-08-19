//! **(il-devine) Point d'accroche n°2 — LA MESURE QUI DÉCIDE (§7, étape 5).**
//!
//!     deviner --poids <premier> --poids-adversaire <second> \
//!             --parties <n> --graine-debut <g> [--boites base,decouverte]
//!
//! Il rejoue `n` parties du joueur contre lui-même et, à **chaque** choix de
//! carte Phase de **chaque** joueur, compare la phase que le second réseau donne
//! comme la plus probable à celle réellement choisie. Il imprime **une seule
//! ligne JSON sur la sortie standard, et rien d'autre** :
//!
//!     {"decisions": 5840, "justes": 2190, "part": 0.375, "hasard": 0.25}
//!
//! - `decisions` : le nombre de choix de phase examinés ;
//! - `justes` : ceux où la phase la plus probable était la bonne ;
//! - `part` : le rapport des deux ;
//! - `hasard` : la part qu'obtiendrait une réponse tirée au sort parmi les phases
//!   autorisées — **moyenne sur les mêmes décisions, pas la valeur théorique
//!   0,25**. Elle n'est pas exactement un quart : à la première manche, aucune
//!   phase n'a été jouée la manche d'avant et les cinq sont autorisées.
//!
//! Tout le reste — le détail, le point de vue de référence, les temps — part sur
//! la sortie d'erreur.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **DE QUEL POINT DE VUE LA QUESTION EST POSÉE, et c'est tout le sujet.**
//!
//! À chaque `pick_phase` du joueur P, la description est prise du point de vue de
//! **l'autre** joueur, celui qui ne choisit pas. C'est le seul point de vue qui
//! mesure ce que le chantier livre : au §3, la devinette prête une intention à
//! l'adversaire, et celui qui devine n'est jamais celui qui choisit. Le §7 le dit
//! par la bande en déclarant suspect tout chiffre au-dessus de 60 % — « vérifie
//! que tu ne décris pas la situation du point de vue de celui que tu prédis ».
//!
//! **Rien n'est appris ici.** `apprendre` est faux, l'exploration est nulle, et la
//! devinette n'est pas allumée : le joueur joue exactement comme il jouerait sans
//! ce chantier, et l'on se contente de regarder par-dessus son épaule. Mesurer un
//! réseau pendant qu'on le corrige serait se mesurer soi-même.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, setup_game};
use engine::sim::MAX_GENERATIONS;
use std::time::Instant;

use engine::{description, joueur, reseau};

use description::Description;
use joueur::Joueur;
use reseau::{Pile, Reseau, ReseauPhases};

fn mourir(msg: &str) -> ! {
    eprintln!("deviner: {msg}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut poids = String::new();
    let mut poids_adversaire = String::new();
    let mut parties: u64 = 150;
    let mut graine_debut: u64 = 820_000;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut devinette = false;
    let mut reference = false;
    let mut i = 1;
    while i < args.len() {
        // Toutes les options de ce binaire prennent une valeur : on avance de deux.
        let avance = 2;
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--poids" => poids = val(i),
            "--poids-adversaire" => poids_adversaire = val(i),
            "--parties" => parties = val(i).parse().unwrap_or_else(|_| mourir("--parties")),
            "--graine-debut" => {
                graine_debut = val(i).parse().unwrap_or_else(|_| mourir("--graine-debut"))
            }
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            // **Le relevé de référence du §8, éteint par défaut.** Il refait la
            // mesure du point de vue du joueur qui choisit — la tâche
            // d'imitation — pour dire ce que coûte le changement de point de vue.
            // Il décrit donc la situation du point de vue de celui qu'on prédit,
            // ce que le §1 interdit au JOUEUR : rien n'en sort qui décide de
            // quoi que ce soit, et il faut le demander pour qu'il ait lieu.
            "--reference-point-de-vue" => {
                reference = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!(
                        "--reference-point-de-vue attend « on » ou « off », pas « {autre} »"
                    )),
                }
            }
            // L'interrupteur du §4, ici aussi : il dit si le joueur SE SERT de la
            // devinette pendant qu'on la mesure. Éteint par défaut — on veut
            // mesurer la devinette sur les parties que le joueur livré produit,
            // pas sur celles qu'elle aurait elle-même infléchies.
            "--devinette" => {
                devinette = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!("--devinette attend « on » ou « off », pas « {autre} »")),
                }
            }
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += avance;
    }
    if poids.is_empty() {
        mourir("--poids est obligatoire");
    }
    if poids_adversaire.is_empty() {
        mourir("--poids-adversaire est obligatoire : c'est le réseau qu'on mesure");
    }

    let boites = match BoiteSet::parse(&boites_txt) {
        Ok(b) => b,
        Err(e) => mourir(&e),
    };
    let chemin = if std::path::Path::new(&cartes).exists() {
        cartes.clone()
    } else {
        format!("../{cartes}")
    };
    let db = match CardsDb::load_boites(&chemin, boites) {
        Ok(db) => db,
        Err(e) => mourir(&e),
    };
    let desc = Description::new(&db);
    let noms = desc.noms_avec(&db);

    // Les deux verrous du §7 jouent : chaque fichier porte les noms de ses
    // entrées, et une divergence de description est refusée sur-le-champ.
    let mut reseau = match Reseau::lire(&poids, &noms) {
        Ok(r) => r,
        Err(e) => mourir(&e),
    };
    let mut adversaire = match ReseauPhases::lire(&poids_adversaire, &noms) {
        Ok(r) => r,
        Err(e) => mourir(&e),
    };
    eprintln!(
        "premier réseau : {poids} ({} parties) — second : {poids_adversaire} ({} parties)",
        reseau.parties, adversaire.parties
    );

    let mut pile = Pile::new(desc.taille);
    let t0 = Instant::now();
    let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, graine_debut);
    // On mesure : on n'apprend pas, on n'explore pas.
    j.apprendre = false;
    j.exploration = 0.0;
    j.adversaire = Some(&mut adversaire);
    j.devinette = devinette;
    j.mesurer_devinette = true;
    j.reference_point_de_vue = reference;

    // Le §8 demande les `pick_phase` adverses rencontrés **par décision**, pas
    // par essai d'option : on compte donc aussi les décisions de la partie, que
    // le joueur tient déjà dans son journal.
    let mut decisions_totales: u64 = 0;
    for g in 0..parties {
        let seed = graine_debut + g;
        j.nouvelle_partie(seed);
        let mut game = setup_game(&db, seed, &mut j);
        while !game.game_over && game.generation <= MAX_GENERATIONS {
            j.debut_manche(&game);
            play_round(&mut game, &db, &mut j);
        }
        decisions_totales += j.journal.len() as u64;
    }

    let decisions = j.devinettes;
    let justes = j.devinettes_justes;
    let justes_soi = j.devinettes_justes_soi;
    let somme_hasard = j.somme_hasard;
    let pas_avance = j.pas_avance;
    let essais = j.essais;
    let rencontres = j.phases_rencontrees;

    if decisions == 0 {
        mourir("aucun choix de carte Phase examiné : la mesure n'a pas eu lieu");
    }
    let part = justes as f64 / decisions as f64;
    let hasard = somme_hasard / decisions as f64;

    // Le détail sur la sortie d'erreur : la sortie standard ne porte QUE la ligne
    // JSON, comme le point d'accroche n°2 l'impose.
    eprintln!(
        "{parties} parties en {:.1} s — {decisions} choix de phase examinés",
        t0.elapsed().as_secs_f64()
    );
    eprintln!(
        "  point de vue de l'autre (celui du §3, le chiffre rapporté) : {:.4}",
        part
    );
    if reference {
        eprintln!(
            "  point de vue du joueur qui choisit (la tâche d'imitation, pour référence) : {:.4}",
            justes_soi as f64 / decisions as f64
        );
    }
    eprintln!("  hasard mesuré sur les mêmes décisions : {hasard:.4}");
    eprintln!(
        "  {decisions_totales} décisions de partie au total ({:.1} par partie)",
        decisions_totales as f64 / parties.max(1) as f64
    );
    eprintln!(
        "  avance vers le repère : {pas_avance} pas pour {essais} essais ({:.2} par essai), \
         {rencontres} `pick_phase` adverses rencontrés ({:.3} par essai, {:.2} par décision)",
        pas_avance as f64 / essais.max(1) as f64,
        rencontres as f64 / essais.max(1) as f64,
        rencontres as f64 / decisions_totales.max(1) as f64
    );

    println!(
        "{{\"decisions\": {decisions}, \"justes\": {justes}, \"part\": {:.6}, \"hasard\": {:.6}}}",
        part, hasard
    );
}

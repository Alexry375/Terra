//! **(le-juge-apprend) Le joueur Rust joue une partie entière, et dit ce qu'il a
//! répondu.**
//!
//!     jouer --graine G --poids data/poids/apprenti.txt [--boites …] [--sieges 0,1]
//!         → {"decisions": [ … ], "scores": [a, b], "generations": n}
//!
//! Il sert au banc du §4 : « le joueur Rust et le joueur JavaScript doivent
//! choisir la même option dans la même situation ». Le banc
//! `web/webapp/verif/juge-meme-option.mjs` fait jouer la même graine au joueur
//! JavaScript et compare les deux listes de réponses, décision par décision. Si
//! les deux listes sont égales, les deux joueurs ont choisi la même option à
//! chacune des décisions de la partie — c'est le critère le plus fort possible,
//! et il ne demande aucune situation fabriquée.
//!
//! Ni exploration ni apprentissage : on mesure une force, et un joueur qui
//! explore pendant sa propre notation se sabote (§5).

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use engine::flow::{play_round, score_parts, setup_game};
use engine::sim::MAX_GENERATIONS;
use serde_json::json;

use engine::{description, joueur, reseau};

use description::Description;
use joueur::Joueur;
use reseau::{Pile, Reseau, ReseauPhases};

fn mourir(msg: &str) -> ! {
    eprintln!("jouer: {msg}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut graine: u64 = 1;
    let mut poids = String::from("data/poids/apprenti.txt");
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut tracer: i64 = -1;
    // **(il-devine §4) L'interrupteur, côté Rust.** Éteint par défaut, et éteint
    // veut dire « exactement comme avant » : sans ces deux options, ce binaire
    // fait très précisément ce qu'il faisait au round 2 — c'est ce que le
    // contrôle 10 vérifie sur trois parties entières, empreinte comprise.
    let mut poids_adversaire = String::new();
    let mut devinette = false;
    // **(le-joueur-sans-voyance, V1) LA GRAINE DES REJEUX D'ESSAI.**
    //
    // Elle ne touche pas au déroulement de la vraie partie : elle fixe le
    // rebattage du paquet que chaque essai de coup subit, pour que le joueur
    // cesse de lire à l'avance les cartes qu'il recevra. C'est ce qui rend la
    // correction vérifiable de l'extérieur : à graine de partie fixée, deux
    // valeurs différentes donnent deux parties différentes, et une même valeur
    // redonne toujours la même.
    let mut graine_essais = joueur::GRAINE_ESSAIS_DEFAUT;
    // (2.11 / 2.15) Les deux interrupteurs de mesure : ils servent à chiffrer le
    // surcoût de l'énumération complète et celui de la vente, et à couper la
    // vente si son coût explose. Allumés par défaut, tous les deux.
    let mut combinaisons_completes = true;
    let mut vente = true;
    let mut i = 1;
    while i < args.len() {
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--graine" => graine = val(i).parse().unwrap_or_else(|_| mourir("--graine")),
            "--poids" => poids = val(i),
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            "--tracer-rang" => tracer = val(i).parse().unwrap_or(-1),
            "--poids-adversaire" => poids_adversaire = val(i),
            "--graine-essais" => {
                graine_essais = val(i).parse().unwrap_or_else(|_| mourir("--graine-essais"))
            }
            "--combinaisons" => {
                combinaisons_completes = match val(i).as_str() {
                    "completes" => true,
                    "carte-par-carte" => false,
                    autre => mourir(&format!(
                        "--combinaisons attend « completes » ou « carte-par-carte », pas « {autre} »"
                    )),
                }
            }
            "--vente" => {
                vente = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!("--vente attend « on » ou « off », pas « {autre} »")),
                }
            }
            "--devinette" => {
                devinette = match val(i).as_str() {
                    "on" => true,
                    "off" => false,
                    autre => mourir(&format!("--devinette attend « on » ou « off », pas « {autre} »")),
                }
            }
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += 2;
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
    let mut reseau = match Reseau::lire(&poids, &noms) {
        Ok(r) => r,
        Err(e) => mourir(&e),
    };
    // (il-devine §4) Le second réseau n'est lu que si on le nomme, et la
    // devinette ne s'allume que si on le demande. « Un joueur à qui on ne donne
    // pas de second réseau doit jouer comme avant, pas planter et pas se dégrader
    // en silence » — d'où l'avertissement plutôt que le silence.
    let mut adversaire = if poids_adversaire.is_empty() {
        None
    } else {
        match ReseauPhases::lire(&poids_adversaire, &noms) {
            Ok(r) => Some(r),
            Err(e) => mourir(&e),
        }
    };
    if devinette && adversaire.is_none() {
        eprintln!(
            "jouer: --devinette on sans --poids-adversaire : aucun second réseau, \
             la devinette reste ÉTEINTE"
        );
        devinette = false;
    }

    let mut pile = Pile::new(desc.taille);
    let mut j = Joueur::new(&db, &desc, &mut reseau, &mut pile, graine);
    j.exploration = 0.0;
    j.tracer_rang = tracer;
    j.apprendre = false;
    j.adversaire = adversaire.as_mut();
    j.devinette = devinette;
    j.graine_essais = graine_essais;
    j.combinaisons_completes = combinaisons_completes;
    j.vente = vente;
    j.nouvelle_partie(graine);

    // Le temps de la PARTIE seule — lecture des cartes et des poids exclues.
    // C'est la grandeur que `result.md` chiffre : un pourcentage sans durée
    // absolue ne vaut rien, et une durée qui compte le démarrage non plus.
    let chrono = std::time::Instant::now();
    let mut game = setup_game(&db, graine, &mut j);
    while !game.game_over && game.generation <= MAX_GENERATIONS {
        j.debut_manche(&game);
        play_round(&mut game, &db, &mut j);
    }
    let secondes = chrono.elapsed().as_secs_f64();
    let (scores, _, _) = score_parts(&game, &db);
    // **L'écart d'évaluation entre les options d'une même décision** (§2.2) : au
    // round 1 il valait 0,016, le niveau du bruit. C'est la mesure qui dit si le
    // réseau départage vraiment, et non s'il prédit bien.
    let ecart = if j.compte_ecart > 0 {
        j.somme_ecart / j.compte_ecart as f64
    } else {
        0.0
    };
    eprintln!(
        "écart moyen entre la meilleure et la pire option : {ecart:.4} ({} décisions à plusieurs options)",
        j.compte_ecart
    );
    eprintln!(
        "avance vers le repère (§4.1) : {} pas, plafond atteint {} fois",
        j.pas_avance, j.plafonds
    );
    println!(
        "{}",
        json!({
            "decisions": j.journal,
            "scores": [scores[0], scores[1]],
            "generations": game.generation,
            "partie_complete": game.game_over,
            "ecart_options": ecart,
            "decisions_multiples": j.compte_ecart,
            // (2.11) Le prix de la partie en essais de coups, et la part que
            // l'échange des cartes de départ y prend : 256 sous-ensembles par
            // siège quand l'énumération est complète.
            "essais": j.essais,
            "essais_mulligan": j.essais_mulligan,
            "essais_vente": j.essais_vente,
            "essais_refuses": j.essais_refuses,
            "rebattages_sautes": j.rebattages_sautes,
            "temps_essais": j.t_essais,
            // (2.15) Combien de fois l'IA a choisi de vendre une carte, et
            // combien d'occasions lui ont été offertes.
            "ventes_volontaires": j.ventes_volontaires,
            "occasions_de_vente": j.occasions_de_vente,
            // (V1) La graine des rejeux d'essai, pour qu'une sortie enregistrée
            // dise avec quoi elle a été produite.
            "graine_essais": graine_essais,
            "secondes": secondes,
        })
    );
}

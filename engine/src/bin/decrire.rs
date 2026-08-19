//! **(le-juge-apprend) Point d'accroche n°1 — la description d'une situation,
//! côté Rust.**
//!
//!     decrire --graine G --decisions "i,j,k" --siege S [--boites base,decouverte]
//!         → {"entrees": [ … ]}
//!     decrire --noms
//!         → {"noms": [ … ]}
//!
//! Il rejoue la partie `G` avec les décisions données (exactement comme le pont
//! WebAssembly le fait pour la page), s'arrête à la première décision non prise,
//! et imprime le vecteur de description de cette situation-là, du point de vue du
//! siège demandé.
//!
//! **La forme de `--decisions`** : le contrôle 01 joint par des virgules les
//! réponses qu'un fournisseur a réellement données, et certaines sont des LISTES
//! (« 3,1,[2, 0],5 » — un choix multiple). On lit donc l'argument comme un
//! tableau JSON, en l'entourant de crochets : c'est du JSON valide pour toutes
//! les formes de réponse que le moteur accepte (indice, montant, liste), y
//! compris une entrée de vente `{"vendre": …}`.

use engine::boites::BoiteSet;
use engine::cards::CardsDb;
use serde_json::{json, Value};

use engine::{description, rejeu, reseau};

use description::{Description, Tampons};

fn mourir(msg: &str) -> ! {
    eprintln!("decrire: {msg}");
    std::process::exit(2);
}

/// Lecture de la liste des décisions. Vide = aucune décision.
fn lire_decisions(txt: &str) -> Vec<Value> {
    let t = txt.trim();
    if t.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Value>(&format!("[{t}]")) {
        Ok(Value::Array(a)) => a,
        _ => mourir(&format!("--decisions illisible : « {t} »")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut graine: u64 = 0;
    let mut decisions_txt = String::new();
    let mut siege: usize = 0;
    let mut boites_txt = "base,decouverte".to_string();
    let mut cartes = "data/cards.json".to_string();
    let mut noms_seuls = false;
    let mut etat_aussi = false;
    let mut table_seule = false;
    let mut poids = String::new();
    let mut i = 1;
    while i < args.len() {
        let mut avance = 2;
        let val = |i: usize| -> String {
            match args.get(i + 1) {
                Some(v) => v.clone(),
                None => mourir(&format!("valeur manquante pour {}", args[i])),
            }
        };
        match args[i].as_str() {
            "--graine" => graine = val(i).parse().unwrap_or_else(|_| mourir("--graine")),
            "--decisions" => decisions_txt = val(i),
            "--siege" => siege = val(i).parse().unwrap_or_else(|_| mourir("--siege")),
            "--boites" => boites_txt = val(i),
            "--cards" => cartes = val(i),
            // Hors contrat : l'évaluation du réseau sur cette situation. Sert à
            // comparer les deux côtés nombre à nombre (le JavaScript calcule la
            // même chose avec le même fichier de poids).
            "--poids" => poids = val(i),
            "--noms" => {
                noms_seuls = true;
                avance = 1;
            }
            // Confort de mise au point, hors contrat : la vue d'état du moteur
            // pour la même situation. Sert au banc d'accord Rust/JavaScript.
            "--etat" => {
                etat_aussi = true;
                avance = 1;
            }
            // Hors contrat, et c'est le point : la table des cartes du vecteur,
            // qui SERT À ENGENDRER `web/webapp/joueurs/paquet.js`. Une seule
            // source de vérité (`data/cards.json`, lu par le moteur) pour les
            // deux côtés — le JavaScript ne redécouvre pas le paquet, il reçoit
            // celui du moteur, et le verrou des noms du §7 le vérifie.
            "--table" => {
                table_seule = true;
                avance = 1;
            }
            autre => mourir(&format!("argument inconnu {autre}")),
        }
        i += avance;
    }
    if siege >= engine::state::NUM_PLAYERS {
        mourir("--siege doit valoir 0 ou 1");
    }

    let boites = match BoiteSet::parse(&boites_txt) {
        Ok(b) => b,
        Err(e) => mourir(&e),
    };
    // Le binaire est lancé depuis la racine du dépôt (le contrôle 01 fixe
    // `cwd=REPO`) ; on accepte aussi d'être lancé depuis `engine/`.
    let chemin = if std::path::Path::new(&cartes).exists() {
        cartes.clone()
    } else {
        format!("../{cartes}")
    };
    let db = match CardsDb::load_boites(&chemin, boites) {
        Ok(db) => db,
        Err(e) => mourir(&e),
    };
    let d = Description::new(&db);

    if noms_seuls {
        println!("{}", json!({ "noms": d.noms_avec(&db) }));
        return;
    }
    if table_seule {
        println!(
            "{}",
            json!({ "projets": d.projets, "corporations": d.corporations })
        );
        return;
    }

    let decisions = lire_decisions(&decisions_txt);
    let (game, _joueur) = match rejeu::rejouer(&db, graine, decisions) {
        Ok(x) => x,
        Err(e) => mourir(&e),
    };
    let mut out: Vec<f64> = Vec::with_capacity(d.taille);
    let mut t = Tampons::new(&d);
    d.decrire(&game, &db, siege, &mut out, &mut t);
    let mut ligne = json!({ "entrees": out });
    if !poids.is_empty() {
        let noms = d.noms_avec(&db);
        match reseau::Reseau::lire(&poids, &noms) {
            Ok(mut r) => {
                let p = r.evaluer(&out);
                ligne["p"] = json!([p[0], p[1]]);
            }
            Err(e) => mourir(&e),
        }
    }
    if etat_aussi {
        ligne["etat"] = engine::observe::state_view(&game, &db);
    }
    println!("{ligne}");
}

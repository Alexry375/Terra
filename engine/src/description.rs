//! **(le-juge-apprend) CE QUE LE RÉSEAU VOIT : la description d'une situation.**
//!
//! C'est la pièce du risque numéro un. Les poids sont APPRIS ici, en Rust, et
//! RELUS par `web/webapp/joueurs/description.js`. Si les deux côtés ne rangent
//! pas les mêmes nombres dans le même ordre, les poids ne veulent plus rien dire
//! une fois relus.
//!
//! Trois précautions, et elles sont toutes les trois structurelles :
//!
//! 1. **Une seule source d'ordre.** Le parcours est écrit UNE fois
//!    ([`Description::parcours`]) et sert aux deux usages : produire les valeurs
//!    (implémentation [`Valeurs`]) et produire la table des noms
//!    (implémentation [`Noms`]). Il est donc impossible qu'un nom se décale d'un
//!    rang par rapport à sa valeur — l'erreur classique de ce genre de code, et
//!    celle qu'aucun contrôle ne voit.
//! 2. **Le même parcours, écrit dans le même ordre, existe en JavaScript**, et
//!    le §7 impose que le fichier de poids porte les noms : au chargement, le
//!    JavaScript régénère les siens et refuse de jouer au premier écart.
//! 3. **Une seule fonction accède aux joueurs**, [`Description::joueur`], et
//!    elle prend le siège en paramètre. De l'adversaire, le parcours ne lit que
//!    ce que le §3.3 autorise : le NOMBRE de cartes en main, jamais leur
//!    identité. La main d'en face est publiée par le moteur (« mode bac à
//!    sable ») ; elle n'est lue nulle part ici, et c'est vérifiable en une
//!    lecture.
//!
//! **Convention de valeurs (§3.1)** : toute entrée vaut +1 ou −1, jamais 0/1,
//! jamais une quantité brute. Les quantités passent par un thermomètre : l'entrée
//! `i` vaut +1 si la quantité est STRICTEMENT supérieure au seuil `i`.
//!
//! **Les seuils viennent d'une mesure**, pas d'une préférence : 1000 parties au
//! hasard sur les graines 100000 à 100999 (`engine/src/bin/mesures.rs`), et un
//! seuil n'est retenu que si la fraction des situations qui le franchissent tombe
//! entre 2 % et 98 % (§3.5). Ils sont recopiés à l'identique dans le JavaScript.

use engine::cards::{CardsDb, JOKER_TAG_CHOICES};
use engine::state::{GameState, AWARD_POOL, MILESTONE_POOL, NUM_PLAYERS};

// ---------------------------------------------------------------------------
// Les seuils, relevés le 15-08 (mesures --parties 1000 --graine-debut 100000)
// ---------------------------------------------------------------------------

pub const S_GENERATION: &[i64] = &[1, 8, 15, 22, 28, 34, 40, 50];
pub const S_TEMPERATURE: &[i64] = &[0, 1, 2, 5, 8, 13, 18];
pub const S_OXYGENE: &[i64] = &[0, 1, 2, 5, 7, 11, 13];
pub const S_OCEANS: &[i64] = &[0, 1, 3, 4, 6, 8];
pub const S_PIOCHE: &[i64] = &[14, 64, 95, 122, 147, 171, 195, 222];
pub const S_DEFAUSSE: &[i64] = &[6, 20, 38, 58, 79, 101, 127, 169];

pub const S_MC: &[i64] = &[0, 3, 8, 14, 22, 33, 53, 144];
pub const S_CHALEUR: &[i64] = &[0, 1, 3, 5, 8, 14, 49];
pub const S_PLANTES: &[i64] = &[0, 1, 2, 4, 5, 8, 18];
pub const S_PROD_MC: &[i64] = &[0, 1, 2, 4, 6, 11];
pub const S_PROD_CHALEUR: &[i64] = &[0, 1, 2, 4, 5, 8, 14];
pub const S_PROD_PLANTES: &[i64] = &[0, 1, 2, 3, 5];
pub const S_PROD_CARTES: &[i64] = &[0, 1, 2];
pub const S_NT: &[i64] = &[5, 6, 8, 10, 13, 17, 22, 30];
pub const S_FORETS: &[i64] = &[0, 1, 2, 3, 5, 9];
pub const S_SCORE: &[i64] = &[5, 7, 10, 14, 20, 27, 36, 51];
pub const S_MAIN: &[i64] = &[6, 8, 9, 10, 12];
pub const S_POSEES: &[i64] = &[0, 3, 6, 8, 11, 15, 18, 26];
pub const S_ACIER: &[i64] = &[0, 1, 2, 3];
pub const S_TITANE: &[i64] = &[0, 1, 3];
pub const S_REPERES: &[i64] = &[0, 1, 2];
pub const S_PAYABLE: &[i64] = &[0, 3, 6, 8, 9, 10, 11];
pub const S_PAYABLE_VERTE: &[i64] = &[0, 1, 2, 3, 4, 5, 7];
pub const S_PAYABLE_BLEUE: &[i64] = &[0, 1, 2, 3, 5];
pub const S_PAYABLE_ROUGE: &[i64] = &[0, 1, 2, 3];

/// Un jeu de seuils par badge, dans l'ordre de `JOKER_TAG_CHOICES`.
pub const S_BADGES: [&[i64]; 10] = [
    &[0, 1, 2, 4, 5, 7, 10], // BUILDING
    &[0, 1, 2, 3, 4, 6],     // SPACE
    &[0, 1, 2, 4, 6],        // SCIENCE
    &[0, 1, 2, 3],           // PLANT
    &[0, 1, 2, 3],           // MICROBE
    &[0, 1],                 // ANIMAL
    &[0, 1, 2, 3, 4],        // EARTH
    &[0, 1, 2],              // JUPITER
    &[0, 1, 2, 4],           // ENERGY
    &[0, 1, 2, 3, 5],        // EVENT
];

/// Les dix améliorations de carte Phase, dans l'ordre imprimé
/// (`PlayerState::phase_upgrade_labels`).
pub const AMELIORATIONS: [&str; 10] = ["1A", "1B", "2A", "2B", "3A", "3B", "4A", "4B", "5A", "5B"];

// ---------------------------------------------------------------------------
// Le collecteur : deux façons de recevoir le même parcours
// ---------------------------------------------------------------------------

/// Ce qui reçoit le parcours. Le nom d'une entrée est la concaténation
/// `a + n + b + c` (le nombre est omis s'il vaut −1) : passer le nom en morceaux
/// évite toute allocation quand on ne collecte que les valeurs, c'est-à-dire des
/// centaines de millions de fois pendant un entraînement.
pub trait Sortie {
    fn drapeau(&mut self, a: &str, n: i64, b: &str, c: &str, v: bool);
    fn thermo(&mut self, a: &str, n: i64, b: &str, c: &str, q: i64, seuils: &[i64]);
}

/// Collecteur de VALEURS : +1 / −1, dans l'ordre du parcours.
pub struct Valeurs<'a> {
    pub out: &'a mut Vec<f64>,
}

impl Sortie for Valeurs<'_> {
    #[inline(always)]
    fn drapeau(&mut self, _a: &str, _n: i64, _b: &str, _c: &str, v: bool) {
        self.out.push(if v { 1.0 } else { -1.0 });
    }
    #[inline(always)]
    fn thermo(&mut self, _a: &str, _n: i64, _b: &str, _c: &str, q: i64, seuils: &[i64]) {
        for s in seuils {
            self.out.push(if q > *s { 1.0 } else { -1.0 });
        }
    }
}

/// Collecteur de NOMS : la table des entrées du §3.4, celle que le fichier de
/// poids porte et que le JavaScript régénère pour la comparer (§7).
pub struct Noms {
    pub out: Vec<String>,
}

impl Noms {
    fn nom(a: &str, n: i64, b: &str, c: &str) -> String {
        if n < 0 {
            format!("{a}{b}{c}")
        } else {
            format!("{a}{n}{b}{c}")
        }
    }
}

impl Sortie for Noms {
    fn drapeau(&mut self, a: &str, n: i64, b: &str, c: &str, _v: bool) {
        self.out.push(Noms::nom(a, n, b, c));
    }
    fn thermo(&mut self, a: &str, n: i64, b: &str, c: &str, _q: i64, seuils: &[i64]) {
        for s in seuils {
            self.out.push(format!("{}>{s}", Noms::nom(a, n, b, c)));
        }
    }
}

// ---------------------------------------------------------------------------
// La description
// ---------------------------------------------------------------------------

/// Les tables qui ne dépendent que de la base de cartes : quelles cartes ont un
/// rang dans le vecteur, et lequel.
pub struct Description {
    /// Identifiants des cartes projets qui appartiennent à une boîte physique —
    /// donc TOUTES celles qui peuvent apparaître dans une partie, quelle que
    /// soit la composition `--boites` de cette partie-ci. La table ne bouge donc
    /// pas d'une partie à l'autre : « le rang d'une entrée désigne toujours la
    /// même chose » (§3.3).
    pub projets: Vec<u16>,
    /// `id de projet -> rang dans `projets``, ou `usize::MAX`.
    rang_projet: Vec<usize>,
    /// Noms des corporations, triés : côté JavaScript l'état ne publie que le
    /// NOM de la corporation (`observe.rs`), jamais son identifiant.
    pub corporations: Vec<String>,
    /// Noms imprimés des récompenses, précalculés : `format!("{kind:?}")` dans
    /// la boucle chaude allouerait sept chaînes par évaluation.
    noms_awards: Vec<String>,
    /// Nombre d'entrées du vecteur.
    pub taille: usize,
}

/// Les tampons de travail d'une description, réutilisés d'une évaluation à
/// l'autre : sans eux, chaque évaluation allouerait quatre vecteurs de la taille
/// du paquet — et il y en a des centaines de millions dans un entraînement.
pub struct Tampons {
    dans_main: Vec<bool>,
    pose_moi: Vec<bool>,
    pose_adv: Vec<bool>,
    defausse: Vec<bool>,
}

impl Tampons {
    pub fn new(d: &Description) -> Tampons {
        let n = d.projets.len();
        Tampons {
            dans_main: vec![false; n],
            pose_moi: vec![false; n],
            pose_adv: vec![false; n],
            defausse: vec![false; n],
        }
    }
}

impl Description {
    pub fn new(db: &CardsDb) -> Description {
        let mut projets: Vec<u16> = Vec::new();
        for (i, c) in db.projects.iter().enumerate() {
            if c.boite.is_some() {
                projets.push(i as u16);
            }
        }
        let mut rang_projet = vec![usize::MAX; db.projects.len()];
        for (rang, id) in projets.iter().enumerate() {
            rang_projet[*id as usize] = rang;
        }
        let mut corporations: Vec<String> = db.corporations.iter().map(|c| c.name.clone()).collect();
        corporations.sort();
        corporations.dedup();
        let noms_awards = AWARD_POOL.iter().map(|k| format!("{k:?}")).collect();
        let mut d = Description {
            projets,
            rang_projet,
            corporations,
            noms_awards,
            taille: 0,
        };
        d.taille = d.noms_avec(db).len();
        d
    }

    /// La table des noms. Elle ne dépend pas de l'état : le parcours est donc
    /// fait sur une partie neuve, dont seules les valeurs — jetées — changent.
    pub fn noms_avec(&self, db: &CardsDb) -> Vec<String> {
        let mut policy = engine::policy::RandomPolicy;
        let game = engine::flow::setup_game(db, 0, &mut policy);
        let mut n = Noms { out: Vec::new() };
        let mut t = Tampons {
            dans_main: vec![false; self.projets.len()],
            pose_moi: vec![false; self.projets.len()],
            pose_adv: vec![false; self.projets.len()],
            defausse: vec![false; self.projets.len()],
        };
        self.parcours(&game, db, 0, &mut n, &mut t);
        n.out
    }

    /// Le vecteur de description de `game`, du point de vue du siège `siege`.
    /// `out` est vidé puis rempli : l'appelant garde son tampon d'une évaluation
    /// à l'autre (aucune allocation dans la boucle chaude).
    pub fn decrire(
        &self,
        game: &GameState,
        db: &CardsDb,
        siege: usize,
        out: &mut Vec<f64>,
        t: &mut Tampons,
    ) {
        out.clear();
        let mut v = Valeurs { out };
        self.parcours(game, db, siege, &mut v, t);
    }

    /// **La seule fonction qui accède aux joueurs**, et elle prend le siège en
    /// paramètre. Rendre la triche impossible par construction plutôt que par
    /// discipline (§3.3).
    #[inline(always)]
    fn joueur<'a>(&self, game: &'a GameState, siege: usize, moi: bool) -> &'a engine::state::PlayerState {
        let p = if moi { siege } else { (siege + 1) % NUM_PLAYERS };
        &game.players[p]
    }

    /// **LE PARCOURS — la source unique de l'ordre des entrées.**
    ///
    /// Il est écrit une fois et sert aux valeurs comme aux noms ; le JavaScript
    /// en tient la copie conforme, dans le même ordre.
    pub fn parcours<S: Sortie>(
        &self,
        game: &GameState,
        db: &CardsDb,
        siege: usize,
        s: &mut S,
        t: &mut Tampons,
    ) {
        // ------------------------------------------------------- a. le global
        s.drapeau("global_", -1, "", "fin_de_partie", game.game_over);
        s.thermo("global_", -1, "", "generation", game.generation as i64, S_GENERATION);
        s.thermo("global_", -1, "", "temperature", game.temperature as i64, S_TEMPERATURE);
        s.thermo("global_", -1, "", "oxygene", game.oxygen as i64, S_OXYGENE);
        s.thermo("global_", -1, "", "oceans", game.oceans_revealed as i64, S_OCEANS);
        s.thermo("global_", -1, "", "pioche", game.deck.len() as i64, S_PIOCHE);
        s.thermo("global_", -1, "", "defausse", game.discard.len() as i64, S_DEFAUSSE);

        // Un rang par TYPE de repère, jamais par position : trois sont tirés au
        // hasard parmi onze à chaque partie (§3.3, le piège annoncé).
        for kind in MILESTONE_POOL.iter() {
            let nom = kind.name();
            let slot = game.milestones.iter().find(|m| m.kind == *kind);
            s.drapeau("repere_", -1, nom, "_present", slot.is_some());
            s.drapeau(
                "repere_",
                -1,
                nom,
                "_atteint",
                slot.map_or(false, |m| m.achieved_by.iter().any(|x| *x)),
            );
            s.drapeau(
                "repere_",
                -1,
                nom,
                "_par_moi",
                slot.map_or(false, |m| m.achieved_by[siege]),
            );
        }
        for (i, kind) in AWARD_POOL.iter().enumerate() {
            s.drapeau(
                "recompense_",
                -1,
                &self.noms_awards[i],
                "_presente",
                game.awards.contains(kind),
            );
        }
        for ph in 0u8..=5 {
            s.drapeau("phase_en_cours_", ph as i64, "", "", game.phase_en_cours == ph);
        }

        // ---------------------------------------------- b. une entrée par carte
        //
        // Quatre drapeaux par carte projet : dans MA main, posée par moi, posée
        // par l'adversaire, dans la défausse. La défausse est une information
        // publique et le propriétaire du projet a accordé le comptage des cartes
        // passées (§3.3, décision du 11-08). La main d'en face, elle, n'est
        // jamais lue.
        let moi = self.joueur(game, siege, true);
        let adv = self.joueur(game, siege, false);
        t.dans_main.fill(false);
        t.pose_moi.fill(false);
        t.pose_adv.fill(false);
        t.defausse.fill(false);
        for id in moi.hand.iter() {
            if let Some(r) = self.rang(*id) {
                t.dans_main[r] = true;
            }
        }
        for id in moi.played.iter() {
            if let Some(r) = self.rang(*id) {
                t.pose_moi[r] = true;
            }
        }
        for id in adv.played.iter() {
            if let Some(r) = self.rang(*id) {
                t.pose_adv[r] = true;
            }
        }
        for id in game.discard.iter() {
            if let Some(r) = self.rang(*id) {
                t.defausse[r] = true;
            }
        }
        for (r, id) in self.projets.iter().enumerate() {
            let id = *id as i64;
            s.drapeau("projet", id, "", "_main", t.dans_main[r]);
            s.drapeau("projet", id, "", "_pose_moi", t.pose_moi[r]);
            s.drapeau("projet", id, "", "_pose_adv", t.pose_adv[r]);
            s.drapeau("projet", id, "", "_defausse", t.defausse[r]);
        }
        // La corporation de l'adversaire est publique une fois installée.
        let corpo_moi = moi.corporation.map(|c| db.corporations[c as usize].name.as_str());
        let corpo_adv = adv.corporation.map(|c| db.corporations[c as usize].name.as_str());
        for nom in self.corporations.iter() {
            s.drapeau("corpo_", -1, nom, "_moi", corpo_moi == Some(nom.as_str()));
            s.drapeau("corpo_", -1, nom, "_adv", corpo_adv == Some(nom.as_str()));
        }

        // -------------------------------------------------- c. par joueur, ×2
        // « Le joueur qui regarde vient toujours en premier, l'adversaire
        // ensuite » (§3.2).
        // Un SEUL passage de score : `score_breakdown` calcule la ventilation des
        // deux joueurs d'un coup, et c'est le point de calcul unique du moteur.
        let (parts, _, _) = engine::flow::score_breakdown(game, db);
        for (prefixe, pl) in [("moi_", moi), ("adv_", adv)] {
            s.thermo(prefixe, -1, "", "mc", pl.mc, S_MC);
            s.thermo(prefixe, -1, "", "chaleur", pl.heat, S_CHALEUR);
            s.thermo(prefixe, -1, "", "plantes", pl.plants, S_PLANTES);
            s.thermo(prefixe, -1, "", "prod_mc", pl.mc_prod, S_PROD_MC);
            s.thermo(prefixe, -1, "", "prod_chaleur", pl.heat_prod, S_PROD_CHALEUR);
            s.thermo(prefixe, -1, "", "prod_plantes", pl.plant_prod, S_PROD_PLANTES);
            s.thermo(prefixe, -1, "", "prod_cartes", pl.card_prod, S_PROD_CARTES);
            s.thermo(prefixe, -1, "", "nt", pl.tr, S_NT);
            s.thermo(prefixe, -1, "", "forets", pl.forests, S_FORETS);
            let p_index = if prefixe == "moi_" { siege } else { (siege + 1) % NUM_PLAYERS };
            s.thermo(prefixe, -1, "", "score_acquis", parts[p_index].acquis(), S_SCORE);
            // De l'adversaire : le NOMBRE de cartes en main, jamais leur identité.
            s.thermo(prefixe, -1, "", "main", pl.hand.len() as i64, S_MAIN);
            s.thermo(prefixe, -1, "", "posees", pl.played.len() as i64, S_POSEES);
            for (i, t) in JOKER_TAG_CHOICES.iter().enumerate() {
                s.thermo(prefixe, -1, "badge_", t.as_str(), pl.tag_counts[i] as i64, S_BADGES[i]);
            }
            s.thermo(prefixe, -1, "", "acier", pl.steel_capacity, S_ACIER);
            s.thermo(prefixe, -1, "", "titane", pl.titanium_capacity, S_TITANE);
            // (D1) LA CARTE PHASE, TELLE QUE LA TABLE LA VOIT.
            //
            // Ces six cases lisent `phase_revelee`, et non plus
            // `previous_phase`. La différence est tout le défaut : le moteur
            // interroge les joueurs l'un après l'autre, et `previous_phase` est
            // écrit à la seconde où chacun répond. Les six cases livraient donc
            // au second interrogé la carte que le premier venait de poser FACE
            // CACHÉE (livret `docs/regles/livret-base.md:268`).
            //
            // `phase_revelee`, lui, n'est écrit qu'une fois les deux réponses
            // données (ligne 272 : « une fois que TOUS les joueurs ont fait leur
            // choix, les cartes Phase choisies sont révélées »). Pendant l'étape
            // de planification, ces cases montrent donc la carte de la manche
            // PRÉCÉDENTE — exactement ce qu'un joueur humain lit sur la pile de
            // cartes Phase déjà jouées ; dès la résolution, celle de la manche en
            // cours, comme avant.
            //
            // Le MÊME champ des deux côtés : une case nommée `previous_phase_3`
            // veut dire la même chose qu'on la lise de soi ou de l'adversaire,
            // à savoir « la carte Phase retournée sur la table ». C'est ce qui
            // rend la fiche du second interrogé insensible au choix caché du
            // premier, quel que soit le siège qui la regarde.
            s.drapeau(prefixe, -1, "previous_phase_", "aucune", pl.phase_revelee.is_none());
            for ph in 1u8..=5 {
                s.drapeau(
                    prefixe,
                    -1,
                    "previous_phase_",
                    match ph {
                        1 => "1",
                        2 => "2",
                        3 => "3",
                        4 => "4",
                        _ => "5",
                    },
                    pl.phase_revelee == Some(ph),
                );
            }
            // Lu sur le tableau du joueur plutôt que sur
            // `phase_upgrade_labels()`, qui allouerait des chaînes à chaque
            // évaluation. Même ordre imprimé que la table `AMELIORATIONS`, donc
            // même rang que côté JavaScript, qui lit les étiquettes publiées.
            for (i, a) in AMELIORATIONS.iter().enumerate() {
                let ph = (i / 2 + 1) as u8;
                let variante = if i % 2 == 0 {
                    engine::state::PhaseUpgrade::VariantA
                } else {
                    engine::state::PhaseUpgrade::VariantB
                };
                s.drapeau(
                    prefixe,
                    -1,
                    "amelioration_",
                    a,
                    pl.phase_upgrade(ph) == Some(variante),
                );
            }
            let p_repere = p_index;
            s.thermo(
                prefixe,
                -1,
                "",
                "reperes_atteints",
                game.milestones.iter().filter(|m| m.achieved_by[p_repere]).count() as i64,
                S_REPERES,
            );
        }

        // ------------------------------------------------- d. la jouabilité
        // Ce que je peux faire MAINTENANT. `main_payable` est publié par le
        // moteur : ni la page ni ce module ne savent ce qu'une carte coûte.
        let payable = engine::flow::main_payable(game, db, siege);
        let n_payable = payable.iter().filter(|x| **x).count() as i64;
        s.thermo("moi_", -1, "", "main_payable", n_payable, S_PAYABLE);
        for (coul, seuils, cle) in [
            ("verte", S_PAYABLE_VERTE, "payable_verte"),
            ("bleue", S_PAYABLE_BLEUE, "payable_bleue"),
            ("rouge", S_PAYABLE_ROUGE, "payable_rouge"),
        ] {
            let n = moi
                .hand
                .iter()
                .enumerate()
                .filter(|(i, id)| {
                    payable.get(*i).copied().unwrap_or(false)
                        && db.projects[**id as usize].color.nom_fr() == coul
                })
                .count() as i64;
            s.thermo("moi_", -1, "", cle, n, seuils);
        }
    }

    #[inline(always)]
    fn rang(&self, id: u16) -> Option<usize> {
        match self.rang_projet.get(id as usize) {
            Some(&r) if r != usize::MAX => Some(r),
            _ => None,
        }
    }
}

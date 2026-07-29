//! Chargement de `cards.json` / `cards_v2.json` et base de cartes en mémoire.
//!
//! Depuis le chantier cartes-1 : TOUTES les cartes projets (green/blue/red)
//! sont chargées, avec leur drapeau `in_deck_v1` — la pioche ne contient que
//! les 248 cartes v1, mais la sonde `--probe` doit trouver aussi les cartes
//! hors pioche (Grain Silos, imposée — journal B2). Corporations : les 16
//! `in_deck_v1`. Les VP (`vp`, `vp_dynamic`) viennent de `cards_v2.json` ;
//! les effets du lot 1 sont résolus par nom dans la table statique
//! `effects::LOT1`.

use crate::boites::{self, Appartenance, Boite, BoiteSet, Kind, Ligne};
use crate::effects::{self, CardEffects};
use serde::Deserialize;

/// Tags du jeu (livret de base p.5 + Discovery « wild tag »).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Building,
    Space,
    Science,
    Plant,
    Microbe,
    Animal,
    Earth,
    Jupiter,
    Energy,
    Event,
    /// Tag wild de Discovery — stub neutre en v1 (compté comme aucun tag, D16).
    Dynamic,
}

pub const TAG_COUNT: usize = 10; // tags comptés (Dynamic exclu)

impl Tag {
    pub fn from_str(s: &str) -> Option<Tag> {
        match s {
            "BUILDING" => Some(Tag::Building),
            "SPACE" => Some(Tag::Space),
            "SCIENCE" => Some(Tag::Science),
            "PLANT" => Some(Tag::Plant),
            "MICROBE" => Some(Tag::Microbe),
            "ANIMAL" => Some(Tag::Animal),
            "EARTH" => Some(Tag::Earth),
            "JUPITER" => Some(Tag::Jupiter),
            "ENERGY" => Some(Tag::Energy),
            "EVENT" => Some(Tag::Event),
            "DYNAMIC" => Some(Tag::Dynamic),
            _ => None,
        }
    }

    /// Nom du tag tel qu'il est écrit dans `cards.json` — l'inverse exact de
    /// [`Tag::from_str`]. Employé par `--dump-corporations`, pour que les badges
    /// rendus se comparent sans transformation au fichier source.
    pub fn as_str(self) -> &'static str {
        match self {
            Tag::Building => "BUILDING",
            Tag::Space => "SPACE",
            Tag::Science => "SCIENCE",
            Tag::Plant => "PLANT",
            Tag::Microbe => "MICROBE",
            Tag::Animal => "ANIMAL",
            Tag::Earth => "EARTH",
            Tag::Jupiter => "JUPITER",
            Tag::Energy => "ENERGY",
            Tag::Event => "EVENT",
            Tag::Dynamic => "DYNAMIC",
        }
    }

    /// Index dans les compteurs de tags ; None pour Dynamic (stub).
    pub fn index(self) -> Option<usize> {
        match self {
            Tag::Building => Some(0),
            Tag::Space => Some(1),
            Tag::Science => Some(2),
            Tag::Plant => Some(3),
            Tag::Microbe => Some(4),
            Tag::Animal => Some(5),
            Tag::Earth => Some(6),
            Tag::Jupiter => Some(7),
            Tag::Energy => Some(8),
            Tag::Event => Some(9),
            Tag::Dynamic => None,
        }
    }
}

/// Couleur d'une carte projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Green,
    Blue,
    Red,
}

impl Color {
    pub fn index(self) -> usize {
        match self {
            Color::Green => 0,
            Color::Blue => 1,
            Color::Red => 2,
        }
    }

    /// (decouverte-projets) Nom FRANÇAIS de la couleur, tel que `--dump-deck`
    /// le rend dans son champ `couleur`.
    ///
    /// La couleur n'est pas cosmétique : elle décide de la phase où la carte se
    /// pose (verte en I, bleue et rouge en II), du fait qu'elle reste en jeu ou
    /// parte à la défausse (une rouge est un événement à usage unique), et du
    /// décompte des Objectifs et Récompenses qui comptent les cartes par
    /// couleur. La rendre observable de l'extérieur est ce qui permet de
    /// vérifier la donnée sans lire le code.
    pub fn nom_fr(self) -> &'static str {
        match self {
            Color::Green => "verte",
            Color::Blue => "bleue",
            Color::Red => "rouge",
        }
    }
}

/// Type de VP dynamiques (décompte du score). Depuis le lot 3, les types
/// portant sur des ressources posées sur les cartes (ANIMAL, MICROBE, SCIENCE)
/// sont RÉELS : ils comptent les ressources posées sur CETTE carte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpKind {
    Jupiter,
    Earth,
    Forest,
    BlueCard,
    AnyCard,
    /// (lot 3) Animaux posés sur cette carte.
    Animal,
    /// (lot 3) Microbes posés sur cette carte.
    Microbe,
    /// (lot 3) Ressources science posées sur cette carte.
    Science,
    /// Type non modélisé → 0 point.
    Unsupported,
}

/// VP dynamiques d'une carte : `points` par tranche de `resources` unités
/// comptées (sémantique Java `WinPointsService.getWinPoints` :
/// floor(n / resources) * points).
#[derive(Debug, Clone, Copy)]
pub struct VpDynamic {
    pub kind: VpKind,
    pub resources: i64,
    pub points: i64,
}

/// Carte projet : prix + tags + couleur + VP + effets du lot 1 (None = stub).
#[derive(Debug, Clone)]
pub struct ProjectCard {
    pub name: String,
    pub color: Color,
    pub price: i64,
    pub tags: Vec<Tag>,
    /// Drapeau brut `in_deck_v1` de `cards.json`. **Ne compose plus la
    /// pioche** depuis le chantier moteur-boites-1 : il vient d'un portage Java
    /// non officiel. Conservé parce qu'il reste la clef de désambiguïsation
    /// historique entre une carte et sa variante « Buffed » de même nom
    /// (`resolve_by_name`), un problème d'IDENTITÉ et non d'appartenance.
    pub in_deck_v1: bool,
    /// (boites-1) Boîte physique d'appartenance, `None` si la carte n'existe
    /// sur aucune planche connue (`Microbiology Patents`, variantes « Buffed »,
    /// cartes `fan`/`crysis`…).
    pub boite: Option<Boite>,
    /// (boites-1) Planche physique (`P1`..`P4`), `None` pour Découverte et pour
    /// les cartes sans boîte.
    pub planche: Option<&'static str>,
    /// (boites-1) Retenue dans la pioche de LA configuration `--boites`
    /// courante. C'est le seul critère de distribution.
    pub in_deck: bool,
    /// VP fixes imprimés (extraction Mission A, 0 par défaut).
    pub vp: i64,
    pub vp_dynamic: Option<VpDynamic>,
    /// Effets déclaratifs (lot 1) ; None = stub neutre.
    pub effect: Option<&'static CardEffects>,
}

/// (boites-1) L'encodage d'une carte applique-t-il TOUT son texte imprimé, ou
/// contient-il un effet que le moteur saute explicitement ?
///
/// `ResEff::PhaseUpgrade` en fut le seul cas, jusqu'au chantier
/// `decouverte-phases` : `flow::apply_res_eff` ne l'appliquait pas et
/// l'incrémentait dans `phase_upgrades_skipped`. Une carte qui en portait un
/// était encodée MAIS pas entièrement gérée — `effets_geres` devait dire
/// `false`, sinon le recensement aurait affirmé qu'un pouvoir est appliqué
/// alors que le moteur reconnaissait lui-même le sauter (I4). **Le moteur ne
/// saute plus aucun effet encodé** ; le prédicat reste, pour que le jour où un
/// texte imprimé demandera un mécanisme absent, le recensement le dise.
///
/// Le prédicat est POSITIF au niveau des données : il lit l'encodage, il ne
/// cite aucun nom de carte.
/// (lot acier-titane, I3) Une réduction portée par un savoir-faire (carte verte
/// ou corporation) est-elle un multiple EXACT du taux du livret ? Renvoie une
/// erreur de chargement sinon — voir le garde-fou de `CardsDb::load_boites`.
///
/// Fonction séparée pour être testable directement : `lot_acier_titane_tests`
/// lui présente une réduction bâtiment de 3 MC et exige qu'elle la refuse.
pub fn verifier_multiple(nom: &str, r: effects::Reduction) -> Result<(), String> {
    let effects::Reduction::Tag(t, n) = r else {
        return Ok(());
    };
    let Some(cap) = effects::Capacity::from_tag(t) else {
        return Ok(());
    };
    if cap.units_from(n).is_none() {
        return Err(format!(
            "savoir-faire non entier : « {nom} » porte une réduction {t:?} de {n} MC, \
             qui n'est pas un multiple de {} — le compte d'aciers/titanes ne \
             s'arrondit pas (I3)",
            cap.mc_per_unit()
        ));
    }
    Ok(())
}

pub fn encodage_integral(e: &CardEffects) -> bool {
    /// Les effets que le moteur reconnaît SAUTER. La liste est vide depuis le
    /// chantier `decouverte-phases` : `ResEff::PhaseUpgrade`, seul cas
    /// jusque-là, est désormais appliqué par `flow::apply_phase_upgrade` et
    /// compté dans `phase_upgrades_granted`. Deux cartes de la boîte Découverte
    /// (*Cryogenic Shipment*, *Fibrous Composite Material*) cessent donc d'être
    /// « muettes » — sans qu'une seule carte ait été encodée par ce chantier.
    fn saute(effs: &[effects::ResEff]) -> bool {
        effs.iter().any(|r| match r {
            effects::ResEff::Gain(_)
            | effects::ResEff::Put(_)
            | effects::ResEff::RemoveSelf(_)
            | effects::ResEff::RemoveAny(_, _)
            | effects::ResEff::PhaseUpgrade(_) => false,
        })
    }
    fn saute_step(steps: &[effects::ResStep]) -> bool {
        steps.iter().any(|s| match s {
            effects::ResStep::Do(r) => saute(std::slice::from_ref(r)),
            effects::ResStep::Choose(branches) => branches.iter().any(|b| saute(b)),
        })
    }
    if saute_step(e.on_build) {
        return false;
    }
    if let Some(effects::Action::Res(branches)) = e.action {
        if branches.iter().any(|b| saute(b)) {
            return false;
        }
    }
    true
}

impl ProjectCard {
    /// (boites-1) Le moteur applique-t-il l'intégralité du texte imprimé ?
    /// Faux si la carte n'a aucun encodage, faux aussi si son encodage porte un
    /// effet que le moteur saute (voir [`encodage_integral`]).
    pub fn effets_geres(&self) -> bool {
        self.effect.map_or(false, encodage_integral)
    }

    /// (lot 3) Type de ressource que la carte PORTE, s'il y en a un. Une carte
    /// sans encodage (stub) ou qui n'en porte pas renvoie `None` : elle n'est
    /// jamais réceptacle.
    pub fn holds(&self) -> Option<effects::ResKind> {
        self.effect.and_then(|e| e.holds)
    }
}

/// Corporation retenue par la configuration `--boites` : tags + MC de départ
/// (champ `price`) + effets déclaratifs (chantier corpo-1).
///
/// `effect` est `Some` pour les 12 corporations de la boîte de base — la table
/// `effects::CORPS` les couvre toutes, et le chargement le vérifie. Il est
/// `None` pour les corporations de Découverte (chantier boites-1) : elles
/// restent à encoder. Leurs pouvoirs reposent sur l'amélioration des cartes
/// Phase — le mécanisme existe depuis le chantier `decouverte-phases`, l'appel
/// depuis `install_corporation` reste à écrire (hors périmètre de ce chantier).
/// Elles sont donc comptées dans `cards_effects_unhandled` à chaque partie où
/// elles sont jouées, jamais appliquées en silence.
#[derive(Debug, Clone)]
pub struct Corporation {
    pub name: String,
    pub starting_mc: i64,
    pub tags: Vec<Tag>,
    pub effect: Option<&'static effects::CorpEffects>,
    /// (boites-1) Boîte physique dont vient la corporation.
    pub boite: Boite,
    /// (boites-1) Planche physique (`CORP`), `None` pour Découverte.
    pub planche: Option<&'static str>,
}

/// (boites-1) Une carte retenue par la configuration `--boites` courante, telle
/// que `--dump-deck` la recense.
#[derive(Debug, Clone, Copy)]
pub struct CarteRetenue<'a> {
    /// Nom anglais de `cards.json` — la clef de jointure de tout le moteur
    /// (I2 bis), y compris pour les cartes de Découverte.
    pub name: &'a str,
    pub kind: Kind,
    pub boite: Boite,
    pub planche: Option<&'static str>,
    /// (decouverte-projets) Couleur de la carte, pour un PROJET seulement
    /// (`None` pour une corporation, qui n'en a pas). Lue sur
    /// `ProjectCard::color`, c'est-à-dire sur le champ `category` de
    /// `cards.json` — la donnée elle-même, jamais une table de rattrapage.
    pub couleur: Option<&'static str>,
    /// Le moteur applique-t-il l'effet imprimé de cette carte ?
    ///
    /// `true` signifie exactement : **un encodage existe** pour cette carte
    /// (`effects::LOT1` pour un projet, `effects::CORPS` pour une corporation)
    /// **et cet encodage ne contient aucun effet que le moteur saute** (voir
    /// [`encodage_integral`]).
    /// `false` couvre deux cas : la carte entre en jeu en stub neutre — payée,
    /// badges et PV comptés, **rien d'autre appliqué** (ni prérequis, ni
    /// production, ni action, ni déclencheur) ; ou elle est encodée mais l'un
    /// de ses effets est une amélioration de carte Phase, que le moteur saute
    /// et compte dans `phase_upgrades_skipped`.
    ///
    /// Ce que `true` n'atteste PAS : que l'encodage soit complet et fidèle au
    /// texte imprimé. Cette fidélité-là n'a été auditée que sur les 66 cartes
    /// de `docs/cartes/moteur-vs-imprime.md`.
    pub effets_geres: bool,
}

/// Base de cartes chargée une fois au démarrage.
pub struct CardsDb {
    pub projects: Vec<ProjectCard>,
    pub corporations: Vec<Corporation>,
    /// Nombre de cartes projets retenues par la configuration `--boites`
    /// (taille de la pioche complète — invariant de conservation).
    pub deck_project_count: usize,
    /// (boites-1) Boîtes actives de cette base de cartes.
    pub boites: BoiteSet,
    /// (boites-1) Ce que la composition a eu à signaler sans le corriger
    /// (cartes physiques absentes de `cards.json`). Remonté sur stderr.
    pub avertissements: Vec<String>,
    /// Interrupteur `--effects on|off` : `false` = squelette intégral
    /// (stubs neutres, ni prérequis ni VP de cartes au score).
    pub effects_on: bool,
}

#[derive(Deserialize)]
struct RawVpDynamic {
    #[serde(rename = "type")]
    kind: String,
    resources: i64,
    points: i64,
}

#[derive(Deserialize)]
struct RawCard {
    name: String,
    category: String,
    tags: Vec<String>,
    price: Option<i64>,
    in_deck_v1: bool,
    /// (boites-1) Champ `box` brut. Ne décide de l'appartenance que pour
    /// Découverte ; ailleurs il ne sert qu'à désambiguïser les homonymes
    /// (voir `boites`).
    #[serde(rename = "box", default)]
    boite_src: String,
    /// Champs Mission A (absents de l'ancien cards.json : défauts neutres).
    #[serde(default)]
    vp: i64,
    #[serde(default)]
    vp_dynamic: Option<RawVpDynamic>,
}

fn vp_kind(s: &str) -> VpKind {
    match s {
        "JUPITER" => VpKind::Jupiter,
        "EARTH" => VpKind::Earth,
        "FOREST" => VpKind::Forest,
        "BLUE_CARD" => VpKind::BlueCard,
        "ANY_CARD" => VpKind::AnyCard,
        // (lot 3) ressources posées sur la carte elle-même.
        "ANIMAL" => VpKind::Animal,
        "MICROBE" => VpKind::Microbe,
        "SCIENCE" => VpKind::Science,
        _ => VpKind::Unsupported,
    }
}

impl CardsDb {
    /// Charge `cards_v2.json` (ou l'ancien `cards.json`, champs VP absents →
    /// défauts neutres) pour la **boîte de base seule** (I3 : le défaut du
    /// moteur). Effets par défaut : ACTIVÉS.
    pub fn load(path: &str) -> Result<CardsDb, String> {
        CardsDb::load_boites(path, BoiteSet::default())
    }

    /// (boites-1) Charge la base de cartes pour une configuration de boîtes
    /// donnée. Projets : toutes les cartes green/blue/red sont chargées (la
    /// sonde et les tests doivent pouvoir atteindre une carte hors pioche),
    /// mais seules celles des boîtes demandées portent `in_deck`.
    /// Corporations : uniquement celles des boîtes demandées — ce sont
    /// exactement les cartes que `setup_game` distribue.
    pub fn load_boites(path: &str, boites: BoiteSet) -> Result<CardsDb, String> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| format!("lecture {path}: {e}"))?;
        let raw: Vec<RawCard> =
            serde_json::from_str(&data).map_err(|e| format!("parse {path}: {e}"))?;

        // Point de composition UNIQUE : `boites::composer` décide, pour chaque
        // ligne du fichier, de quelle boîte physique elle vient — ou d'aucune.
        let lignes: Vec<Ligne> = raw
            .iter()
            .map(|c| Ligne {
                name: &c.name,
                boite_src: &c.boite_src,
                kind: match c.category.as_str() {
                    "corporation" | "buffedCorporation" => Kind::Corporation,
                    _ => Kind::Project,
                },
            })
            .collect();
        let compo = boites::composer(&lignes, boites)?;
        let appartenances = compo.appartenances;

        let mut projects = Vec::new();
        let mut corporations = Vec::new();

        for (i, c) in raw.into_iter().enumerate() {
            let app: Option<Appartenance> = appartenances[i];
            // Retenue dans la pioche ssi sa boîte physique est demandée.
            let retenue = app.map_or(false, |a| boites.contains(a.boite));
            let tags: Vec<Tag> = c
                .tags
                .iter()
                .filter_map(|t| Tag::from_str(t))
                .collect();
            match c.category.as_str() {
                "green" | "blue" | "red" => {
                    let color = match c.category.as_str() {
                        "green" => Color::Green,
                        "blue" => Color::Blue,
                        _ => Color::Red,
                    };
                    projects.push(ProjectCard {
                        // Rattaché plus bas, à la carte CANONIQUE seulement
                        // (piège des variantes « Buffed », journal D1).
                        effect: None,
                        in_deck_v1: c.in_deck_v1,
                        boite: app.map(|a| a.boite),
                        planche: app.and_then(|a| a.planche),
                        in_deck: retenue,
                        vp: c.vp,
                        vp_dynamic: c.vp_dynamic.as_ref().map(|d| VpDynamic {
                            kind: vp_kind(&d.kind),
                            resources: d.resources,
                            points: d.points,
                        }),
                        name: c.name,
                        color,
                        price: c.price.unwrap_or(0),
                        tags,
                    });
                }
                // (boites-1) La pioche de corporations = celles que les boîtes
                // demandées contiennent PHYSIQUEMENT. `in_deck_v1` n'entre plus
                // dans le critère. La planche CORP nomme « Teractor
                // Corporation », et `cards.json` en porte deux lignes : c'est
                // la box déclarée par la famille (`base`) qui retient celle à
                // 51 MC plutôt que le jumeau à 48 MC rangé en `promo2021` —
                // choix d'IDENTITÉ que la planche seule ne pouvait pas faire.
                //
                // Les 4 corporations de Découverte (Apollo Industries, Exocorp,
                // Hyperion Systems, Sultira) entrent ici sans encodage : leurs
                // pouvoirs reposent sur l'amélioration des cartes Phase, que le
                // moteur ne modélise pas. Elles sont comptées dans
                // `cards_effects_unhandled`, pas jouées en silence.
                "corporation" if retenue => {
                    let app = app.expect("corporation retenue sans appartenance");
                    let effect = effects::corp_lookup(&c.name);
                    corporations.push(Corporation {
                        name: c.name,
                        starting_mc: c.price.unwrap_or(0),
                        tags,
                        effect,
                        boite: app.boite,
                        planche: app.planche,
                    });
                }
                // Hors des boîtes demandées : corporations d'une autre boîte,
                // buffedCorporation, crysis.
                _ => {}
            }
        }

        let deck_project_count = projects.iter().filter(|c| c.in_deck).count();
        if deck_project_count == 0 || corporations.len() < 4 {
            return Err(format!(
                "base de cartes suspecte: {} projets en pioche, {} corporations",
                deck_project_count,
                corporations.len()
            ));
        }

        // (corpo-1, conservé) Garde-fou de la pioche de corporations de la
        // BOÎTE DE BASE : la table `CORPS` la décrit entièrement, donc chaque
        // entrée doit résoudre vers EXACTEMENT une corporation chargée, et les
        // 12 planches CORP doivent toutes être encodées. Le piège
        // d'appariement (deux « Teractor Corporation ») serait sinon
        // indécidable. La table de boîtes et `CORPS` se contrôlent ainsi
        // mutuellement : un désaccord fait échouer le chargement.
        if boites.contains(Boite::Base) {
            for (name, _) in effects::CORPS {
                let n = corporations
                    .iter()
                    .filter(|c| c.name == *name && c.boite == Boite::Base)
                    .count();
                if n != 1 {
                    return Err(format!(
                        "pioche de corporations: '{name}' résolue {n} fois dans {path} \
                         (une et une seule planche CORP attendue)"
                    ));
                }
            }
            let base_corps = corporations
                .iter()
                .filter(|c| c.boite == Boite::Base)
                .count();
            if base_corps != effects::CORPS.len() {
                return Err(format!(
                    "pioche de corporations: {base_corps} planches CORP chargées pour {} \
                     déclarées dans la table d'effets",
                    effects::CORPS.len()
                ));
            }
        }

        // Garde-fou : chaque entrée de la table d'effets doit résoudre vers
        // EXACTEMENT une carte canonique (voir `resolve_by_name`) ; l'effet est
        // rattaché à celle-là et à aucune autre — une variante « Buffed » de
        // même nom (hors pioche v1) reste un stub neutre.
        for (name, spec) in effects::LOT1 {
            // Le garde-fou porte sur l'AMBIGUÏTÉ RÉELLE : plusieurs cartes du
            // même nom dans le deck v1 rendraient l'encodage indécidable.
            let v1 = projects
                .iter()
                .filter(|c| c.name == *name && c.in_deck_v1)
                .count();
            let total = projects.iter().filter(|c| c.name == *name).count();
            if total == 0 || v1 > 1 {
                return Err(format!(
                    "table d'effets: carte '{name}' résolue {total} fois ({v1} dans le deck v1) \
                     dans {path}"
                ));
            }
            let i = resolve_by_name(&projects, name).expect("carte du lot résolue");
            projects[i].effect = Some(spec);
        }

        // (lot acier-titane) **Garde-fou I3 : un montant qui n'est pas un
        // multiple doit se voir.**
        //
        // Le compte d'aciers et de titanes se DÉRIVE des réductions déjà
        // encodées (`flow::capacities`) : une réduction bâtiment de 3 MC ou
        // espace de 4 MC sur une carte VERTE (ou sur une corporation) rendrait
        // la dérivation fausse. Elle est refusée ICI, au chargement des tables,
        // avant la première partie — jamais arrondie en silence.
        //
        // Le contrôle est au chargement et pas en `debug_assert!` parce que les
        // contrôles et les simulations tournent en `--release` : un
        // `debug_assert!` y serait muet, ce qui est exactement le contraire de
        // « rendu visible ».
        for c in &projects {
            if c.color != Color::Green {
                continue;
            }
            let Some(spec) = c.effect else { continue };
            for r in spec.reductions {
                verifier_multiple(&c.name, *r)?;
            }
        }
        for (name, spec) in effects::CORPS {
            for r in spec.reductions {
                verifier_multiple(name, *r)?;
            }
        }

        Ok(CardsDb {
            projects,
            corporations,
            deck_project_count,
            boites,
            avertissements: compo.avertissements,
            effects_on: true,
        })
    }

    /// (boites-1) Les cartes réellement retenues par la configuration courante,
    /// projets puis corporations, dans l'ordre du fichier — c'est ce que
    /// `--dump-deck` recense et ce que `setup_game` distribue. Le nom exposé est
    /// TOUJOURS celui de `cards.json` (I2 bis), jamais une traduction.
    pub fn recensement(&self) -> Vec<CarteRetenue<'_>> {
        let mut v: Vec<CarteRetenue<'_>> = Vec::new();
        for c in self.projects.iter().filter(|c| c.in_deck) {
            v.push(CarteRetenue {
                name: &c.name,
                kind: Kind::Project,
                boite: c.boite.expect("carte en pioche sans boîte"),
                planche: c.planche,
                couleur: Some(c.color.nom_fr()),
                effets_geres: c.effets_geres(),
            });
        }
        for c in &self.corporations {
            v.push(CarteRetenue {
                name: &c.name,
                kind: Kind::Corporation,
                boite: c.boite,
                planche: c.planche,
                couleur: None,
                effets_geres: c.effect.is_some(),
            });
        }
        v
    }

    /// Résolution CANONIQUE d'une carte projet par son nom exact — chemin
    /// unique du moteur (table d'effets, sonde, tests).
    ///
    /// `cards.json` contient plusieurs entrées de même nom : les versions
    /// rééquilibrées maison du dépôt Java (classes `Buffed…`) y figurent avec
    /// `in_deck_v1: false`, parfois AVANT la carte officielle et avec un prix
    /// différent. La carte canonique est donc celle du deck v1 dès qu'il y a
    /// ambiguïté ; les noms uniques hors pioche (Grain Silos) restent résolus.
    pub fn resolve_card(&self, name: &str) -> Option<u16> {
        resolve_by_name(&self.projects, name).map(|i| i as u16)
    }
}

/// Voir [`CardsDb::resolve_card`]. `None` uniquement si le nom est inconnu.
///
/// À noms multiples, la carte du deck v1 est la carte canonique — c'est elle
/// qui joue, et donc la seule qu'on veut sonder ou encoder. Si AUCUNE des
/// homonymes n'est du deck v1 (Filter Feeders, Genetically Modified
/// Vegetables : deux variantes hors pioche), il n'y a pas de « bonne » carte à
/// désigner : on garde la PREMIÈRE, c'est-à-dire le comportement historique de
/// la sonde, plutôt que de faire disparaître un nom qu'elle trouvait avant.
fn resolve_by_name(projects: &[ProjectCard], name: &str) -> Option<usize> {
    let mut first: Option<usize> = None;
    let mut v1: Option<usize> = None;
    let mut v1_count = 0usize;
    for (i, c) in projects.iter().enumerate() {
        if c.name != name {
            continue;
        }
        if first.is_none() {
            first = Some(i);
        }
        if c.in_deck_v1 {
            v1_count += 1;
            if v1.is_none() {
                v1 = Some(i);
            }
        }
    }
    if v1_count == 1 {
        v1
    } else {
        first
    }
}

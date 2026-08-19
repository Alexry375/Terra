//! Appartenance de boîte et composition de la pioche — **point unique** du
//! moteur (chantier moteur-boites-1, critère I1).
//!
//! Avant ce chantier, la pioche était le drapeau `in_deck_v1` de `cards.json`,
//! hérité d'un portage Java non officiel : 248 cartes distribuées, dont 38 de
//! l'extension Découverte aux pouvoirs non gérés et 2 cartes qui n'existent sur
//! aucune planche physique. Le drapeau ne décide plus rien de la pioche.
//!
//! # D'où vient l'appartenance
//!
//! * **Boîte de base et cartes promotionnelles** : de `textes-cartes.json`, la
//!   transcription des planches physiques, embarquée dans le binaire par
//!   [`TEXTES`] (copie verbatim de `inputs/textes-cartes.json`). Une carte
//!   appartient à la boîte de base **si et seulement si** une planche `P1`..`P4`
//!   (projet) ou `CORP` (corporation) la nomme ; à la boîte promo si la planche
//!   `PROMO` la nomme. Critère POSITIF : rien n'est exclu par une liste de noms,
//!   les cartes absentes des planches (`Microbiology Patents`,
//!   `Project Inspection`) ne sont simplement jamais nommées.
//! * **Découverte** : de `cards.json`, champ `box == "discovery"`, comme le
//!   contrat l'autorise — ses 42 entrées ont été contre-vérifiées contre les
//!   cartes physiques (38 projets + 4 corporations, cf. [`DECOUVERTE_ATTENDU`]).
//!
//! # Le champ `box` de `cards.json` ne décide JAMAIS de l'appartenance
//!
//! Il vient de la même source défectueuse que `in_deck_v1` — c'est lui qui range
//! deux cartes inexistantes en `base`. Il sert ici à une seule chose, et sur le
//! seul chemin base/promo : **désambiguïser les homonymes**. 22 noms de
//! `cards.json` sont portés par deux entrées, dont une variante « Buffed » du
//! dépôt Java rangée `box: "fan"`. Le couple (nom de planche, box déclarée par
//! la famille) résout exactement une ligne pour chacune des 208 + 12 + 11
//! cartes — l'invariant est vérifié au chargement, pas supposé.
//!
//! # Ajouter une boîte demain
//!
//! Une entrée de plus dans [`FAMILLES`] (si la boîte a des planches
//! transcrites) ou un critère à côté de [`DECOUVERTE_BOX`], plus une variante
//! de [`Boite`]. Aucun autre fichier du moteur ne connaît la notion de boîte.

use serde::Deserialize;

/// Transcription des planches physiques d'Alexis, embarquée dans le binaire.
/// Copie verbatim de `inputs/textes-cartes.json`
/// (sha256 `4951a994e51f6d543fcb93d099aa564d2a287ce8294e9191eb6bf20688010649`).
/// Embarquée plutôt que lue sur disque : le binaire doit composer sa pioche
/// quel que soit le répertoire courant, et aucune interface du contrat ne
/// transporte de chemin vers ce fichier.
const TEXTES: &str = include_str!("../data/textes-cartes.json");

/// Genre d'une carte retenue — le champ `kind` de `--dump-deck`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Project,
    Corporation,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Project => "project",
            Kind::Corporation => "corporation",
        }
    }
}

/// Boîte physique à laquelle une carte appartient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boite {
    Base,
    Promo,
    Decouverte,
}

impl Boite {
    /// Nom exposé par `--dump-deck` et accepté par `--boites`.
    pub fn as_str(self) -> &'static str {
        match self {
            Boite::Base => "base",
            Boite::Promo => "promo",
            Boite::Decouverte => "decouverte",
        }
    }

    pub fn parse(s: &str) -> Option<Boite> {
        match s {
            "base" => Some(Boite::Base),
            "promo" => Some(Boite::Promo),
            "decouverte" => Some(Boite::Decouverte),
            _ => None,
        }
    }
}

/// Les boîtes actives d'une partie — la valeur de `--boites`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoiteSet {
    base: bool,
    promo: bool,
    decouverte: bool,
}

impl Default for BoiteSet {
    /// I3 : par défaut, le moteur joue la boîte de base seule — la seule
    /// configuration sur laquelle les tests ont été validés.
    fn default() -> Self {
        BoiteSet {
            base: true,
            promo: false,
            decouverte: false,
        }
    }
}

impl BoiteSet {
    /// Analyse `--boites <liste>` : énumération séparée par des virgules parmi
    /// `base`, `promo`, `decouverte`. Une liste vide ou un nom inconnu est une
    /// erreur : mieux vaut refuser de démarrer que jouer une pioche que
    /// l'appelant ne croit pas avoir demandée.
    pub fn parse(liste: &str) -> Result<BoiteSet, String> {
        let mut s = BoiteSet {
            base: false,
            promo: false,
            decouverte: false,
        };
        let mut n = 0usize;
        for mot in liste.split(',') {
            let mot = mot.trim();
            if mot.is_empty() {
                continue;
            }
            match Boite::parse(mot) {
                Some(Boite::Base) => s.base = true,
                Some(Boite::Promo) => s.promo = true,
                Some(Boite::Decouverte) => s.decouverte = true,
                None => {
                    return Err(format!(
                        "--boites : '{mot}' inconnue (base, promo, decouverte)"
                    ))
                }
            }
            n += 1;
        }
        if n == 0 {
            return Err("--boites : liste vide".to_string());
        }
        s.valider()?;
        Ok(s)
    }

    /// (D15, décision d'Alexis du 19-08) **L'EXTENSION NE SE JOUE PAS SEULE.**
    ///
    /// La boîte Découverte n'apporte que quatre corporations : la mise en place
    /// en distribue quatre, le mulligan des corporations en réclame deux de
    /// plus, et la partie s'arrêtait alors en plein milieu sur « paquet
    /// corporations épuisé » — quatre graines sur cinq. Le garde-fou du
    /// chargement de `cards.json`, lui, laissait passer exactement quatre
    /// corporations, c'est-à-dire le cas qui casse.
    ///
    /// La configuration est donc REFUSÉE AU CHARGEMENT : aucune partie ne
    /// démarre pour s'interrompre ensuite. Le contrôle vit ici, sur l'ensemble
    /// de boîtes lui-même, parce que c'est le seul endroit que TOUS les chemins
    /// traversent — la ligne de commande comme la composition de la pioche.
    pub fn valider(&self) -> Result<(), String> {
        if self.decouverte && !self.base {
            return Err(
                "--boites : l'extension Découverte ne se joue pas seule ; la boîte de base est \
                 obligatoire (essayez « base,decouverte »)"
                    .to_string(),
            );
        }
        Ok(())
    }

    pub fn contains(&self, b: Boite) -> bool {
        match b {
            Boite::Base => self.base,
            Boite::Promo => self.promo,
            Boite::Decouverte => self.decouverte,
        }
    }

    /// Les boîtes actives, dans l'ordre canonique — pour les rapports.
    pub fn actives(&self) -> Vec<Boite> {
        [Boite::Base, Boite::Promo, Boite::Decouverte]
            .into_iter()
            .filter(|b| self.contains(*b))
            .collect()
    }
}

/// Une famille de planches physiques : ce que la planche contient, dans quelle
/// boîte elle se range, et sous quelle `box` de `cards.json` ses cartes se
/// retrouvent (clef de désambiguïsation des homonymes, jamais d'appartenance).
struct Famille {
    /// Valeur du champ `planche` de `textes-cartes.json`.
    planche: &'static str,
    boite: Boite,
    kind: Kind,
    /// Valeur attendue du champ `box` de `cards.json` pour ces cartes.
    box_cards_json: &'static str,
    /// Famille réellement distribuée ? Voir `PROMOCORP` ci-dessous.
    distribuee: bool,
}

/// Les familles de planches transcrites. La planche `PHASE` (5 cartes de phase)
/// n'y figure pas : ce ne sont ni des projets ni des corporations, elles ne sont
/// pas dans `cards.json` et ne se piochent pas.
static FAMILLES: &[Famille] = &[
    Famille { planche: "P1", boite: Boite::Base, kind: Kind::Project, box_cards_json: "base", distribuee: true },
    Famille { planche: "P2", boite: Boite::Base, kind: Kind::Project, box_cards_json: "base", distribuee: true },
    Famille { planche: "P3", boite: Boite::Base, kind: Kind::Project, box_cards_json: "base", distribuee: true },
    Famille { planche: "P4", boite: Boite::Base, kind: Kind::Project, box_cards_json: "base", distribuee: true },
    Famille { planche: "CORP", boite: Boite::Base, kind: Kind::Corporation, box_cards_json: "base", distribuee: true },
    Famille { planche: "PROMO", boite: Boite::Promo, kind: Kind::Project, box_cards_json: "promo2021", distribuee: true },
    // CORRECTION CTO 27-07 : contrairement à ce qu'affirmait mon contrat,
    // Alexis ne possède PAS ces cartes. Les planches PROMO/PROMOCORP viennent
    // de l'adaptation Tabletop Simulator, pas de sa boîte ; elles forment le
    // pack Kickstarter 2021, dont l'absence est tranchée depuis le 24-07
    // (docs/CTO_STATE.md). `--boites promo` reste construit et testé, mais
    // n'est jamais la configuration de jeu réelle.
    //
    // 5 des 6 corporations promotionnelles sont ABSENTES de `cards.json` : le
    // moteur n'a ni leur
    // prix, ni leurs badges, ni leur texte. Distribuer la seule présente
    // fabriquerait une pioche de corporations promo à 1/6 qui n'existe sur
    // aucune table. La famille est donc déclarée non distribuée, et les
    // manquantes sont remontées en avertissement au chargement — signalées,
    // non corrigées (hors périmètre du lot).
    Famille { planche: "PROMOCORP", boite: Boite::Promo, kind: Kind::Corporation, box_cards_json: "promo2021", distribuee: false },
];

/// Critère d'appartenance à l'extension Découverte : la valeur du champ `box`
/// de `cards.json`. Contre-vérifié le 27-07 contre les cartes physiques.
pub const DECOUVERTE_BOX: &str = "discovery";

/// Dénombrement attendu de Découverte : (projets, corporations). Contre-vérifié
/// par `inputs/decouverte/projets-decouverte.json` (38 entrées, `D05`..`D42`)
/// et `inputs/decouverte/corporations-discovery.json` (4). Ces fichiers sont en
/// français et ne sont pas des clefs de jointure : ils servent de garde-fou de
/// dénombrement, pas de source de noms.
pub const DECOUVERTE_ATTENDU: (usize, usize) = (38, 4);

/// L'appartenance d'une ligne de `cards.json`, une fois résolue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appartenance {
    pub boite: Boite,
    /// Planche physique, ou `None` pour Découverte (pas de transcription).
    pub planche: Option<&'static str>,
    pub kind: Kind,
}

/// Une ligne de `cards.json` telle que la composition a besoin de la voir.
#[derive(Debug, Clone, Copy)]
pub struct Ligne<'a> {
    pub name: &'a str,
    /// Champ `box` brut.
    pub boite_src: &'a str,
    pub kind: Kind,
}

#[derive(Deserialize)]
struct RawTexte {
    name: String,
    planche: Option<String>,
}

/// Résultat de la composition : une appartenance (ou son absence) par ligne de
/// `cards.json`, dans le même ordre, plus les avertissements à remonter.
pub struct Composition {
    pub appartenances: Vec<Option<Appartenance>>,
    pub avertissements: Vec<String>,
}

/// **Le** point de composition. Marque chaque ligne de `cards.json` de sa boîte
/// d'appartenance, ou de `None` si aucune boîte physique ne la contient.
///
/// Échoue — plutôt que de composer une pioche approximative — si une carte
/// nommée par une planche distribuée ne se résout pas à exactement une ligne de
/// `cards.json`, si une ligne se retrouve dans deux boîtes, ou si le
/// dénombrement de Découverte s'écarte de [`DECOUVERTE_ATTENDU`].
pub fn composer(lignes: &[Ligne], demandees: BoiteSet) -> Result<Composition, String> {
    // (D15) Second verrou, sur le chemin que TOUTE composition de pioche
    // emprunte : un ensemble de boîtes fabriqué à la main, sans passer par
    // `BoiteSet::parse`, est refusé ici aussi. Le refus reste AU CHARGEMENT.
    demandees.valider()?;
    let textes: Vec<RawTexte> = serde_json::from_str(TEXTES)
        .map_err(|e| format!("parse de la transcription des planches: {e}"))?;

    let mut app: Vec<Option<Appartenance>> = vec![None; lignes.len()];
    let mut avertissements: Vec<String> = Vec::new();

    for f in FAMILLES {
        let noms: Vec<&str> = textes
            .iter()
            .filter(|t| t.planche.as_deref() == Some(f.planche))
            .map(|t| t.name.as_str())
            .collect();
        if noms.is_empty() {
            return Err(format!(
                "planche '{}' déclarée mais absente de la transcription",
                f.planche
            ));
        }
        let mut absents: Vec<&str> = Vec::new();
        for nom in noms {
            let candidats: Vec<usize> = lignes
                .iter()
                .enumerate()
                .filter(|(_, l)| l.name == nom && l.boite_src == f.box_cards_json)
                .map(|(i, _)| i)
                .collect();
            if !f.distribuee {
                if candidats.is_empty() {
                    absents.push(nom);
                }
                continue;
            }
            match candidats.len() {
                1 => {
                    let i = candidats[0];
                    if lignes[i].kind != f.kind {
                        return Err(format!(
                            "planche {} : '{nom}' est {} dans cards.json, {} attendu",
                            f.planche,
                            lignes[i].kind.as_str(),
                            f.kind.as_str()
                        ));
                    }
                    if let Some(deja) = app[i] {
                        return Err(format!(
                            "'{nom}' revendiquée par deux boîtes ({} et {})",
                            deja.boite.as_str(),
                            f.boite.as_str()
                        ));
                    }
                    app[i] = Some(Appartenance {
                        boite: f.boite,
                        planche: Some(f.planche),
                        kind: f.kind,
                    });
                }
                n => {
                    return Err(format!(
                        "planche {} : '{nom}' résolue {n} fois dans cards.json \
                         (box == '{}') — une et une seule ligne attendue",
                        f.planche, f.box_cards_json
                    ))
                }
            }
        }
        // L'avertissement ne concerne que qui demande la boîte : inutile de
        // parler des corporations promo à qui joue la boîte de base seule.
        if !f.distribuee && demandees.contains(f.boite) {
            absents.sort_unstable();
            avertissements.push(format!(
                "planche {} ({} cartes physiques) non distribuée : {} carte(s) \
                 absente(s) de cards.json ({}) — signalé, non corrigé (hors périmètre)",
                f.planche,
                textes
                    .iter()
                    .filter(|t| t.planche.as_deref() == Some(f.planche))
                    .count(),
                absents.len(),
                absents.join(", ")
            ));
        }
    }

    // Découverte : pas de planche transcrite, critère `box` de cards.json.
    let (mut dp, mut dc) = (0usize, 0usize);
    for (i, l) in lignes.iter().enumerate() {
        if l.boite_src != DECOUVERTE_BOX {
            continue;
        }
        if let Some(deja) = app[i] {
            return Err(format!(
                "'{}' revendiquée par deux boîtes ({} et decouverte)",
                l.name,
                deja.boite.as_str()
            ));
        }
        match l.kind {
            Kind::Project => dp += 1,
            Kind::Corporation => dc += 1,
        }
        app[i] = Some(Appartenance {
            boite: Boite::Decouverte,
            planche: None,
            kind: l.kind,
        });
    }
    if (dp, dc) != DECOUVERTE_ATTENDU {
        return Err(format!(
            "Découverte : {dp} projets / {dc} corporations dans cards.json, \
             {} / {} attendus (contre-vérification inputs/decouverte/)",
            DECOUVERTE_ATTENDU.0, DECOUVERTE_ATTENDU.1
        ));
    }

    // Aucun nom en double dans une même boîte : la pioche ne doit jamais
    // distribuer deux fois le même titre.
    for (i, a) in app.iter().enumerate() {
        let Some(a) = a else { continue };
        for (j, b) in app.iter().enumerate().skip(i + 1) {
            let Some(b) = b else { continue };
            if a.boite == b.boite && lignes[i].name == lignes[j].name {
                return Err(format!(
                    "'{}' retenue deux fois dans la boîte {}",
                    lignes[i].name,
                    a.boite.as_str()
                ));
            }
        }
    }

    Ok(Composition {
        appartenances: app,
        avertissements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_des_boites() {
        assert_eq!(BoiteSet::parse("base").unwrap(), BoiteSet::default());
        let s = BoiteSet::parse("base,promo,decouverte").unwrap();
        assert!(s.contains(Boite::Base) && s.contains(Boite::Promo) && s.contains(Boite::Decouverte));
        let s = BoiteSet::parse(" base , decouverte ").unwrap();
        assert!(s.contains(Boite::Base) && !s.contains(Boite::Promo));
        assert!(BoiteSet::parse("discovery").is_err());
        assert!(BoiteSet::parse("").is_err());
    }

    #[test]
    fn la_transcription_embarquee_porte_les_planches_attendues() {
        let textes: Vec<RawTexte> = serde_json::from_str(TEXTES).unwrap();
        let n = |p: &str| {
            textes
                .iter()
                .filter(|t| t.planche.as_deref() == Some(p))
                .count()
        };
        assert_eq!((n("P1"), n("P2"), n("P3"), n("P4")), (52, 52, 52, 52));
        assert_eq!(n("CORP"), 12);
        assert_eq!(n("PROMO"), 11);
        assert_eq!(n("PROMOCORP"), 6);
    }
}

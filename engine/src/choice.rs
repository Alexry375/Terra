//! **De quoi parle une alternative.**
//!
//! `Policy::choose_option` ne transporte qu'un nombre d'options : celui qui
//! décide sait combien de portes il a devant lui, jamais où elles mènent. Ce
//! module porte le chaînon manquant, [`ChoiceContext`] — le SENS de la question
//! posée, construit par le site d'appel qui le connaît, transmis intact à la
//! politique par `Policy::choose_option_ctx`.
//!
//! Trois propriétés voulues, et la raison de chacune :
//!
//! 1. **Une variante par situation, aucune fourre-tout.** Les onze sites de
//!    `flow.rs` ont onze variantes distinctes. Une catégorie générique
//!    « Alternative » où l'on rangerait ce qu'on n'a pas voulu qualifier ferait
//!    passer les contrôles sans rien apprendre au joueur.
//! 2. **Rien qui ne soit lu sur le moteur.** Les candidats d'une amélioration de
//!    carte Phase sont ceux que `flow::apply_phase_upgrade` va réellement
//!    employer ; le nom de chaque carte Phase améliorée vient de
//!    `effects::PHASE_UPGRADED` ; les branches d'une alternative sont les
//!    `ResEff` que le moteur va appliquer. Aucun consommateur — pont
//!    WebAssembly compris — n'a donc à recalculer une règle : il en existerait
//!    aussitôt deux versions, qui divergeraient au premier changement.
//! 3. **Aucun pouvoir sur le déroulement.** Un contexte se construit à partir de
//!    données que le site d'appel a déjà en main ; il ne touche pas au RNG, ne
//!    réordonne aucune option, ne filtre rien. Le corps par défaut de
//!    `choose_option_ctx` retombe sur `choose_option` avec le même `n` : une
//!    politique qui ignore ce module joue exactement comme avant.
//!
//! Les identifiants sont en anglais comme partout dans le moteur ; les
//! descriptions rendues par [`describe_branch`] et [`describe_selector_grant`]
//! sont en français, parce qu'elles vont sous les yeux d'un joueur.

use crate::cards::Tag;
use crate::effects::{
    ActionRes, Eff, ResAmount, ResEff, ResKind, ResPut, ResTarget, SelectorGrant,
};
use crate::flow::{ActionSource, UpgradeSource};
use crate::state::PhaseUpgrade;

/// Une amélioration de carte Phase encore disponible : la phase visée, la
/// variante, et le nom imprimé de la carte améliorée correspondante.
///
/// `name` est lu dans `effects::PHASE_UPGRADED[phase - 1][variant.index()]`,
/// la table que le moteur consulte lui-même pour appliquer le bonus : l'écran
/// qui affiche ce nom affiche donc bien la carte qui entrera en jeu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseUpgradeOption {
    /// Numéro de phase, 1 à 5.
    pub phase: u8,
    /// Variante A ou B.
    pub variant: PhaseUpgrade,
    /// Nom imprimé de la carte Phase améliorée.
    pub name: &'static str,
}

/// Une branche jouable d'une alternative du texte imprimé.
///
/// `printed_rank` est le rang de la branche **dans le texte imprimé**, avant le
/// filtrage des branches injouables : c'est lui qui permet de dire « la
/// deuxième proposition de la carte », et non « la deuxième de celles qui
/// restent ». `effects` sont les effets que le moteur appliquera si cette
/// branche est retenue — la matière de [`describe_branch`].
#[derive(Debug, Clone, Copy)]
pub struct BranchOption {
    pub printed_rank: usize,
    pub effects: &'static [ResEff],
}

/// Une carte verte candidate au rejeu de production (carte Phase améliorée
/// IV-A), et **ce que son rejeu rapporte**.
///
/// Les quantités sont lues par le service unique `flow::card_production` — le
/// même que la phase de production emploie — au moment où le choix est posé.
/// Sans elles, l'option se réduirait à un nom de carte : celui qui décide ne
/// saurait pas ce qu'il gagne, et un consommateur serait tenté de le
/// recalculer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionOption {
    pub card: u16,
    pub mc: i64,
    pub heat: i64,
    pub plants: i64,
    /// Cartes piochées (production de cartes).
    pub cards: i64,
}

/// **Ce dont parle la décision.** Une variante par situation de `flow.rs`.
#[derive(Debug, Clone, Copy)]
pub enum ChoiceContext<'a> {
    /// Pas de NT supplémentaire offert par la CORPORATION du joueur (Unmi :
    /// « The first time your TR is raised each phase, you may pay 6 MC to raise
    /// your TR 1 step »). Option 0 = payer, l'option imprimée ; option 1 = y
    /// renoncer.
    CorpTrBoost {
        /// Identifiant de la corporation qui offre le pas.
        corporation: Option<u16>,
        cost_mc: i64,
        steps: u8,
    },

    /// « Améliorez une carte Phase » : les couples (phase, variante) encore
    /// disponibles, dans l'ordre où le moteur les a construits (phase
    /// croissante, variante A puis B), moins ceux déjà en place.
    PhaseUpgrade {
        candidates: &'a [PhaseUpgradeOption],
        /// Phase IMPOSÉE par le texte de la carte (`None` = au choix).
        imposed_phase: Option<u8>,
        /// D'où vient l'amélioration : pose, action, ou mise en place d'une
        /// corporation.
        source: UpgradeSource,
    },

    /// Alternative « … ou … » du TEXTE IMPRIMÉ d'une carte, résolue à la pose
    /// ou par un déclencheur. Les branches injouables sont déjà écartées.
    CardAlternative {
        card: u16,
        source: UpgradeSource,
        branches: &'a [BranchOption],
    },

    /// Alternative « … ou … » de l'ACTION d'une carte bleue (phase III). Même
    /// forme que la précédente, autre moment du jeu : le joueur ne décide pas
    /// la même chose en posant une carte et en activant son action.
    ActionAlternative {
        card: u16,
        branches: &'a [BranchOption],
    },

    /// Réduction de prix payée en MICROBES posés sur une carte (*Anaerobic
    /// Microorganisms*) : l'employer coûte des ressources, y renoncer les
    /// garde. Option 0 = employer, option 1 = renoncer.
    MicrobeDiscount {
        /// Carte qu'on est en train de poser.
        card: u16,
        /// Carte qui porte les microbes dépensés.
        holder: u16,
        /// Microbes dépensés.
        count: u32,
        /// MC de réduction obtenus.
        amount: i64,
    },

    /// Réduction de prix payée en PLANTES (*Restructured Resources*). Même
    /// forme, autre monnaie. Option 0 = employer, option 1 = renoncer.
    PlantDiscount {
        card: u16,
        /// Plantes dépensées.
        plants: i64,
        /// MC de réduction obtenus.
        amount: i64,
    },

    /// Payer une carte en CHALEUR (Helion, « You may use heat as MC »), plutôt
    /// que de défausser des cartes. Option 0 = employer la chaleur, option 1 =
    /// y renoncer.
    HeatAsMc {
        card: u16,
        /// Coût effectif de la carte, réductions appliquées.
        cost: i64,
    },

    /// « You may discard a card. If that card had a [badge], draw two cards.
    /// Otherwise, draw a card. » (*Mars University*). Option 0 = défausser,
    /// option 1 = renoncer.
    DiscardToDraw {
        /// Carte porteuse du déclencheur. `None` = déclencheur porté par la
        /// planche de CORPORATION, qui n'est pas une carte en jeu.
        card: Option<u16>,
        /// Badge qui double la pioche.
        tag: Tag,
        draw_if: u8,
        draw_else: u8,
    },

    /// « Spend up to N <ressource> to gain that amount of <ressource> » : les
    /// options sont des QUANTITÉS CROISSANTES, l'option k valant k+1 unités.
    SpendAmount {
        /// Carte bleue, ou planche de corporation, qui porte l'action.
        source: ActionSource,
        spend: ActionRes,
        gain: ActionRes,
        /// Quantité maximale offerte = nombre d'options.
        max: i64,
    },

    /// Bonus du sélectionneur d'une carte Phase (extension Découverte) dont le
    /// texte propose un « ou ».
    SelectorBonus {
        /// Phase sélectionnée, 1 à 5.
        phase: u8,
        /// Variante installée sur cette carte Phase (`None` = carte normale).
        variant: Option<PhaseUpgrade>,
        /// Nom imprimé de la carte Phase lue.
        card_name: &'static str,
        branches: &'a [SelectorGrant],
    },

    /// « Rejouez l'effet de production d'une de vos cartes vertes » (carte
    /// Phase améliorée IV-A) : les options sont des cartes vertes en jeu du
    /// joueur, chacune avec ce que son rejeu rapporte.
    ReplayProduction { candidates: &'a [ProductionOption] },
}

impl ChoiceContext<'_> {
    /// **Le nombre d'options, et l'unique source de vérité sur ce nombre.**
    ///
    /// C'est lui que le corps par défaut de `Policy::choose_option_ctx` passe à
    /// `choose_option` : il doit valoir exactement le `n` que le site d'appel
    /// passait avant ce chantier, sans quoi le RNG serait consommé autrement et
    /// le déroulement des parties changerait.
    pub fn option_count(&self) -> usize {
        match self {
            ChoiceContext::CorpTrBoost { .. }
            | ChoiceContext::MicrobeDiscount { .. }
            | ChoiceContext::PlantDiscount { .. }
            | ChoiceContext::HeatAsMc { .. }
            | ChoiceContext::DiscardToDraw { .. } => 2,
            ChoiceContext::PhaseUpgrade { candidates, .. } => candidates.len(),
            ChoiceContext::CardAlternative { branches, .. }
            | ChoiceContext::ActionAlternative { branches, .. } => branches.len(),
            ChoiceContext::SpendAmount { max, .. } => (*max).max(0) as usize,
            ChoiceContext::SelectorBonus { branches, .. } => branches.len(),
            ChoiceContext::ReplayProduction { candidates } => candidates.len(),
        }
    }

    /// Nom stable de la NATURE du choix, en minuscules sans accent : la clef par
    /// laquelle un consommateur (le pont WebAssembly, un journal, une
    /// intelligence artificielle) distingue les onze situations. Aucune valeur
    /// générique : chaque variante a la sienne.
    pub fn kind(&self) -> &'static str {
        match self {
            ChoiceContext::CorpTrBoost { .. } => "corp_tr_boost",
            ChoiceContext::PhaseUpgrade { .. } => "amelioration_carte_phase",
            ChoiceContext::CardAlternative { .. } => "alternative_carte",
            ChoiceContext::ActionAlternative { .. } => "alternative_action",
            ChoiceContext::MicrobeDiscount { .. } => "reduction_microbes",
            ChoiceContext::PlantDiscount { .. } => "reduction_plantes",
            ChoiceContext::HeatAsMc { .. } => "paiement_chaleur",
            ChoiceContext::DiscardToDraw { .. } => "defausser_pour_piocher",
            ChoiceContext::SpendAmount { .. } => "montant_depense",
            ChoiceContext::SelectorBonus { .. } => "bonus_selectionneur",
            ChoiceContext::ReplayProduction { .. } => "rejouer_production",
        }
    }
}

// ---------------------------------------------------------------------------
// Descriptions en français
// ---------------------------------------------------------------------------
//
// Elles vivent ICI, dans le moteur, et non chez le consommateur : ce sont les
// mêmes données que le moteur applique. Un pont qui rédigerait ses propres
// libellés à partir du nom des cartes en tiendrait une seconde version, qui
// mentirait dès qu'une donnée changerait.

/// Nom français d'un badge.
pub fn tag_label(t: Tag) -> &'static str {
    match t {
        Tag::Building => "Bâtiment",
        Tag::Space => "Espace",
        Tag::Science => "Science",
        Tag::Plant => "Plante",
        Tag::Microbe => "Microbe",
        Tag::Animal => "Animal",
        Tag::Earth => "Terre",
        Tag::Jupiter => "Jupiter",
        Tag::Energy => "Énergie",
        Tag::Event => "Événement",
        Tag::Dynamic => "joker",
    }
}

/// Accord en nombre : « 1 plante », « 3 plantes ». Les libellés vont sous les
/// yeux d'un joueur, ils s'écrivent en français correct.
fn mot(n: i64, singulier: &'static str, pluriel: &'static str) -> &'static str {
    if n <= -2 || n >= 2 {
        pluriel
    } else {
        singulier
    }
}

/// **La quantité que désigne l'option `k` d'un choix de montant.**
///
/// `SpendAmount` offre des quantités CROISSANTES : l'option 0 vaut une unité,
/// l'option 1 en vaut deux, et ainsi de suite jusqu'à `max`. La correspondance
/// vit ici, à côté du site qui l'applique (`flow.rs`, `Action::SpendUpTo` :
/// `amt = k + 1`), et nulle part ailleurs — un consommateur qui la
/// réécrirait en tiendrait une seconde version.
pub fn spend_amount_quantity(k: usize) -> i64 {
    k as i64 + 1
}

/// Nom français d'une ressource de coût ou de gain variable, accordé en nombre.
pub fn action_res_quantity(r: ActionRes, n: i64) -> String {
    let pluriel = n <= -2 || n >= 2;
    match r {
        ActionRes::Heat if pluriel => format!("{n} chaleurs"),
        ActionRes::Heat => format!("{n} chaleur"),
        ActionRes::Mc => format!("{n} MC"),
        ActionRes::Plants if pluriel => format!("{n} plantes"),
        ActionRes::Plants => format!("{n} plante"),
    }
}

/// Nom français d'une ressource de coût ou de gain variable, sans quantité.
pub fn action_res_label(r: ActionRes) -> &'static str {
    match r {
        ActionRes::Heat => "chaleur",
        ActionRes::Mc => "MC",
        ActionRes::Plants => "plantes",
    }
}

/// Nom français d'un type de ressource posée sur une carte.
fn res_kind_label(k: ResKind) -> &'static str {
    match k {
        ResKind::Microbe => "microbe",
        ResKind::Animal => "animal",
        ResKind::Science => "science",
    }
}

fn res_kinds_label(kinds: &[ResKind]) -> String {
    let mots: Vec<&str> = kinds.iter().map(|k| res_kind_label(*k)).collect();
    match mots.len() {
        0 => "ressource".to_string(),
        1 => mots[0].to_string(),
        _ => mots.join(" ou "),
    }
}

/// Signe explicite : « +3 » se lit mieux que « 3 » quand le nombre peut être
/// négatif (une carte peut faire PERDRE des MC).
fn signe(n: i64) -> String {
    if n >= 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

/// Description française d'un effet élémentaire de pose.
fn describe_eff(e: &Eff) -> String {
    match e {
        Eff::Mc(n) => format!("{} MC", signe(*n)),
        Eff::Heat(n) => format!("{} {}", signe(*n), mot(*n, "chaleur", "chaleurs")),
        Eff::Plants(n) => format!("{} {}", signe(*n), mot(*n, "plante", "plantes")),
        Eff::Draw(n) => format!("piocher {n} {}", mot(*n as i64, "carte", "cartes")),
        Eff::McProd(n) => format!("{} production de MC", signe(*n)),
        Eff::HeatProd(n) => format!("{} production de chaleur", signe(*n)),
        Eff::PlantProd(n) => format!("{} production de plantes", signe(*n)),
        Eff::CardProd(n) => format!("{} production de cartes", signe(*n)),
        Eff::Temperature(n) => format!("température +{n} pas"),
        Eff::Oxygen(n) => format!("oxygène +{n} pas"),
        Eff::Ocean(n) => format!("révéler {n} {}", mot(*n as i64, "océan", "océans")),
        Eff::Tr(n) => format!("NT +{n}"),
        Eff::Infrastructure(n) => format!("infrastructure +{n} pas"),
        Eff::PlantsIfTags(t, seuil, n) => format!(
            "{} {} si vous avez au moins {seuil} {} {}",
            signe(*n),
            mot(*n, "plante", "plantes"),
            mot(*seuil as i64, "badge", "badges"),
            tag_label(*t)
        ),
        Eff::TrPerTag(t) => format!("NT +1 par badge {}", tag_label(*t)),
        Eff::Forest(n) => format!("gagner {n} PV Forêt"),
        Eff::DrawDiscard {
            draw,
            discard,
            from_drawn,
        } => {
            if *from_drawn {
                format!("piocher {draw} cartes et n'en garder que {}", draw - discard)
            } else {
                format!("piocher {draw} cartes puis en défausser {discard}")
            }
        }
        Eff::IfObjective(inner) => format!(
            "si vous avez un Objectif : {}",
            inner
                .iter()
                .map(describe_eff)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Description française d'une pose de ressources.
fn describe_put(p: &ResPut) -> String {
    let combien = match p.amount {
        ResAmount::Fixed(n) => n.to_string(),
        ResAmount::ByKind { microbe, other } => format!("{microbe} (microbes) ou {other}"),
    };
    let ou = match p.target {
        ResTarget::SelfCard => "sur cette carte",
        ResTarget::Another => "sur une AUTRE carte",
        ResTarget::Any => "sur une carte",
    };
    format!("poser {combien} {} {ou}", res_kinds_label(p.kinds))
}

/// Description française d'un effet du vocabulaire « ressources ».
fn describe_res_eff(e: &ResEff) -> String {
    match e {
        ResEff::Gain(g) => describe_eff(g),
        ResEff::Put(p) => describe_put(p),
        ResEff::RemoveSelf(n) => format!(
            "retirer {n} {} de cette carte",
            mot(*n as i64, "ressource", "ressources")
        ),
        ResEff::RemoveAny(kinds, n) => format!(
            "retirer {n} {} d'une carte au choix",
            res_kinds_label(kinds)
        ),
        ResEff::PhaseUpgrade(None) => "améliorer une carte Phase".to_string(),
        ResEff::PhaseUpgrade(Some(ph)) => format!("améliorer votre carte Phase {ph}"),
    }
}

/// **Ce qu'une branche d'alternative FAIT**, en une phrase française.
///
/// C'est la matière du libellé qu'un écran affiche à la place d'un numéro. Elle
/// est lue sur les `ResEff` que le moteur va réellement appliquer si la branche
/// est retenue : décrire et faire ne peuvent pas diverger.
pub fn describe_branch(effects: &[ResEff]) -> String {
    if effects.is_empty() {
        return "ne rien faire".to_string();
    }
    effects
        .iter()
        .map(describe_res_eff)
        .collect::<Vec<_>>()
        .join(", puis ")
}

/// **Ce qu'une branche de bonus de sélectionneur FAIT**, en une phrase.
///
/// Même principe que [`describe_branch`], sur la structure que la phase lit
/// elle-même (`flow::selector_branch`).
pub fn describe_selector_grant(g: &SelectorGrant) -> String {
    let mut bouts: Vec<String> = Vec::new();
    if g.mc_discount != 0 {
        bouts.push(format!("{} MC de moins sur la carte posée", g.mc_discount));
    }
    if g.mc != 0 {
        bouts.push(format!("{} MC", signe(g.mc)));
    }
    if g.draw != 0 {
        bouts.push(format!(
            "piocher {} {}",
            g.draw,
            mot(g.draw as i64, "carte", "cartes")
        ));
    }
    if g.extra_activations != 0 {
        bouts.push(format!(
            "{} {} d'action en plus",
            g.extra_activations,
            mot(g.extra_activations as i64, "activation", "activations")
        ));
    }
    for b in g.builds {
        let couleurs: Vec<&str> = b.colors.iter().map(|c| c.nom_fr()).collect();
        let mut m = format!("poser une carte {} de plus", couleurs.join(" ou "));
        if let Some(max) = b.max_printed_cost {
            m.push_str(&format!(" (coût imprimé ≤ {max} MC)"));
        }
        if b.free {
            m.push_str(" sans en payer le coût");
        }
        bouts.push(m);
    }
    if g.research_draw != 0 || g.research_keep != 0 {
        bouts.push(format!(
            "piocher {} {} et en garder {} de plus en Recherche",
            g.research_draw,
            mot(g.research_draw as i64, "carte", "cartes"),
            g.research_keep
        ));
    }
    if g.reveal.is_some() {
        bouts.push("révéler le dessus de la pioche".to_string());
    }
    if g.replay_green_prod {
        bouts.push("rejouer la production d'une de vos cartes vertes".to_string());
    }
    if bouts.is_empty() {
        return "aucun bonus".to_string();
    }
    bouts.join(", ")
}

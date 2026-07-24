//! Politiques de décision. Le moteur appelle la politique à chaque point de
//! choix ; TOUT l'aléatoire passe par le RNG de la partie (D11), fourni en
//! paramètre — la politique elle-même ne possède pas de RNG.

use rand::rngs::StdRng;
use rand::Rng;

/// Bonus du sélectionneur de la phase construction (livret p.12) :
/// piocher 1 carte OU jouer une 2e carte bleue/rouge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionBonus {
    DrawCard,
    SecondBuild,
}

/// Options de la phase action (livret p.14). Les actions bleues sont des
/// stubs neutres (exécutées, sans effet) mais consomment leur activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOpt {
    ForestWithPlants,
    ForestWithMc,
    TemperatureWithHeat,
    TemperatureWithMc,
    OceanWithMc,
    /// Défausser 1 carte de la main pour 3 MC.
    SellCard,
    /// Action de la carte bleue jouée d'indice donné (stub neutre).
    BlueAction(u16),
}

/// Points de décision du flux de jeu. Les tests peuvent fournir une politique
/// scriptée ; `simulate` utilise `RandomPolicy`. Les deux passent par le même
/// flux (`setup_game` / `play_round`).
pub trait Policy {
    /// Mulligan corporations (règle maison n°1) : remplacer SES 2 corporations
    /// par 2 nouvelles — les 2 ou aucune. Avant la donne des cartes projets.
    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool;

    /// Mulligan projets (règle maison n°2) : remplacer ses 8 cartes de départ —
    /// les 8 ou aucune, en une fois.
    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> bool;

    /// Choix final de corporation (1 parmi 2), cartes projets en main.
    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize;

    /// Choix de phase parmi `allowed` (la phase de la ronde précédente est
    /// exclue par le moteur avant l'appel).
    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8;

    /// Choisit une carte à construire parmi `affordable` (indices dans la main),
    /// ou None pour passer.
    fn choose_build(&mut self, rng: &mut StdRng, player: usize, affordable: &[usize])
        -> Option<usize>;

    /// Bonus du sélectionneur en phase construction.
    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus;

    /// Une décision de la phase action : Some(indice dans options) ou None (stop).
    fn action_choice(&mut self, rng: &mut StdRng, player: usize, options: &[ActionOpt])
        -> Option<usize>;

    /// Montant d'une action bleue « up to X » (lot 2 : Power Infrastructure,
    /// Greenhouses, Redrafted Contracts). Tirage uniforme 0..=max via le RNG de
    /// la partie. Méthode par DÉFAUT : les politiques existantes en héritent
    /// (aucune modification de signature).
    fn action_amount(&mut self, rng: &mut StdRng, _player: usize, max: i64) -> i64 {
        if max <= 0 {
            0
        } else {
            rng.gen_range(0..=max)
        }
    }

    /// Recherche : garder `keep` cartes parmi `drawn` — renvoie les indices gardés.
    fn research_keep(&mut self, rng: &mut StdRng, player: usize, drawn: &[u16], keep: usize)
        -> Vec<usize>;

    /// Fin de ronde : défausser `n` cartes (limite de main) — indices à défausser.
    fn discard_down(&mut self, rng: &mut StdRng, player: usize, hand: &[u16], n: usize)
        -> Vec<usize>;
}

/// Politique uniforme aléatoire (toutes décisions tirées du RNG de la partie).
pub struct RandomPolicy;

impl Policy for RandomPolicy {
    fn corp_mulligan(&mut self, rng: &mut StdRng, _player: usize, _corps: &[u16]) -> bool {
        rng.gen_bool(0.5)
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, _player: usize, _hand: &[u16]) -> bool {
        rng.gen_bool(0.5)
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, _player: usize, corps: &[u16]) -> usize {
        rng.gen_range(0..corps.len())
    }

    fn pick_phase(&mut self, rng: &mut StdRng, _player: usize, allowed: &[u8]) -> u8 {
        allowed[rng.gen_range(0..allowed.len())]
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        if affordable.is_empty() || rng.gen_bool(0.25) {
            None
        } else {
            Some(affordable[rng.gen_range(0..affordable.len())])
        }
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        if rng.gen_bool(0.5) {
            ConstructionBonus::DrawCard
        } else {
            ConstructionBonus::SecondBuild
        }
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None;
        }
        // Une chance sur (n+1) de s'arrêter : garantit la terminaison de la
        // phase tout en consommant des ressources la plupart du temps.
        let pick = rng.gen_range(0..=options.len());
        if pick == options.len() {
            None
        } else {
            Some(pick)
        }
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..drawn.len()).collect();
        // Mélange de Fisher-Yates via le RNG de la partie.
        for i in (1..idx.len()).rev() {
            let j = rng.gen_range(0..=i);
            idx.swap(i, j);
        }
        idx.truncate(keep);
        idx
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..hand.len()).collect();
        for i in (1..idx.len()).rev() {
            let j = rng.gen_range(0..=i);
            idx.swap(i, j);
        }
        idx.truncate(n);
        idx
    }
}

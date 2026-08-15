//! **(le-juge-apprend) LE JOUEUR QUI ESSAIE SES OPTIONS, côté Rust.**
//!
//! À chaque point de décision (§4) : pour chaque option proposée par le moteur,
//! construire la réponse correspondante, demander au moteur l'état qui en
//! résulterait, décrire cet état **du point de vue du joueur qui décidait**, le
//! faire passer dans le réseau, et garder l'option dont la probabilité de
//! victoire est la plus haute.
//!
//! **Comment on obtient « l'état qui en résulterait » sans pont.** Une manche du
//! moteur n'est pas reprenable au milieu : `play_round` déroule la planification
//! puis les cinq phases, et rien ne permet d'y rentrer en cours de route. On
//! garde donc un clone de l'état **au début de la manche** et on rejoue la manche
//! depuis là, avec les réponses déjà données plus l'option essayée
//! (`rejeu::Rejeu`, le jumeau natif du harnais du pont). C'est le même principe
//! que le pont — « la partie EST la graine plus la liste des décisions » — mais
//! borné à une manche au lieu d'une partie : mesuré à quelques microsecondes par
//! essai, là où rejouer depuis la graine en coûterait deux cents.
//!
//! Les décisions de la mise en place (mulligans, corporation) précèdent la
//! première manche : leur point de reprise est `setup_game` lui-même.
//!
//! **Les choix multiples** (garder k cartes parmi n, en défausser n) n'ont pas
//! une option par réponse mais une combinaison : on les construit **de proche en
//! proche**, en essayant à chaque tour chacune des cartes qui restent et en
//! gardant la meilleure. Chaque carte ajoutée est donc, elle aussi, essayée.
//!
//! **LE REPÈRE DU §4.1 (corrigé le 15-08).** Appliquer l'option ne suffit pas :
//! l'état qui en résulte ne se trouve pas au même endroit de la partie selon
//! l'option. « Poser une carte » mène à la décision d'après, « passer » mène
//! beaucoup plus loin — la phase se termine, la production est encaissée, la
//! manche suivante commence — et plus tard paraît toujours meilleur. On avance
//! donc, après l'option, **jusqu'au prochain point de décision du joueur qui
//! choisit** : toutes les options sont jugées au même instant, « la prochaine
//! fois que j'aurai la main ». L'avance répond à la place de l'autre (`Premiere`)
//! et ne dépasse jamais soixante pas.
//!
//! **L'apprentissage** (§2) vit ici parce que son point de rendez-vous est une
//! décision : une prise sur K (K = 8), sur l'état du repère de l'option retenue —
//! celui-là même que le joueur vient de juger, et aucun autre.

use engine::cards::CardsDb;
use engine::choice::ChoiceContext;
use engine::effects::RevealFilter;
use engine::flow::{play_round, setup_game};
use engine::policy::{ActionOpt, ConstructionBonus, Policy};
use engine::sim::MAX_GENERATIONS;
use engine::state::GameState;
use rand::rngs::StdRng;
use rand::Rng;
use serde_json::{json, Value};

use crate::description::{Description, Tampons};
use crate::rejeu::Rejeu;
use crate::reseau::{Pile, Reseau};

/// **La marge de départage, et pourquoi elle est indispensable.**
///
/// Deux options peuvent mener à des situations rigoureusement identiques — c'est
/// courant : huit cartes vertes à rejouer dont sept ne changent rien d'observable.
/// Leurs notes sont alors égales *en arithmétique réelle*, mais pas au dernier bit :
/// la mise à jour par différences du §1.1 additionne les sommes cachées dans un
/// ordre qui dépend de l'option évaluée juste avant, et le JavaScript, lui, refait
/// chaque évaluation en entier. Mesuré sur une décision réelle (graine 2, rang 361) :
/// 0,46726923574014501 d'un côté, 0,46726923574014506 de l'autre — cinq
/// dix-millionièmes de milliardième d'écart, et deux joueurs qui ne choisissent plus
/// la même option.
///
/// On ne départage donc une option que si elle est meilleure d'une marge qui dépasse
/// franchement le bruit de calcul ; à égalité, la PREMIÈRE l'emporte. Les deux côtés
/// appliquent la même règle, avec la même marge.
pub const MARGE: f64 = 1e-12;

/// **L'état atteint par un rejeu, et le piège du §4.**
///
/// « Une option peut terminer la partie. L'état rendu porte alors `game_over` et les
/// scores sont définitifs ; c'est un cas normal, pas une erreur. » Il faut donc
/// distinguer les deux issues, exactement comme le pont le fait
/// (`wasm/src/lib.rs`, `let termine = pol.attente.is_none()`) :
///
/// - une décision attend → l'état à décrire est celui que `observe` a retenu juste
///   avant elle ;
/// - plus aucune décision → **l'état FINAL**, et surtout pas `vue`. `Rejeu::observe`
///   écrit `vue` à chaque observation, y compris aux points de décision que le moteur
///   finit par ne pas poser (liste d'options vide) : `vue` peut donc porter un état de
///   milieu de manche alors que la partie est finie. C'est le seul moment où l'entrée
///   `global_fin_de_partie` vaut +1 et où les scores sont définitifs — s'y tromper fait
///   juger la dernière décision de la partie sur un état périmé, et fait diverger le
///   joueur Rust du joueur JavaScript, qui, lui, reçoit l'état final du pont.
fn etat_atteint(rejeu: &mut Rejeu, g: GameState) -> Option<GameState> {
    if rejeu.erreur.is_some() {
        // Le moteur a refusé cette réponse : elle n'est pas jouable, et l'état
        // atteint après un repli n'a rien à voir avec elle.
        return None;
    }
    if rejeu.attente.is_none() {
        return Some(g); // la partie est finie : l'état final, scores compris
    }
    Some(rejeu.vue.take().unwrap_or(g))
}

/// Où reprendre pour rejouer une décision.
enum Reprise {
    /// Avant la première manche : on rejoue `setup_game` depuis la graine.
    MiseEnPlace,
    /// Le clone de l'état au début de la manche en cours.
    Manche(Box<GameState>),
}

pub struct Joueur<'a> {
    pub db: &'a CardsDb,
    pub desc: &'a Description,
    pub reseau: &'a mut Reseau,
    pub pile: &'a mut Pile,
    seed: u64,
    reprise: Reprise,
    /// Réponses déjà données depuis le point de reprise, au format du pont.
    reponses: Vec<Value>,
    /// Probabilité de prendre une option au hasard (§5).
    pub exploration: f64,
    /// Corriger le réseau à chaque carte Phase (faux quand on mesure la force).
    pub apprendre: bool,
    pub taux: f64,
    /// Description de l'état vu par `observe`, et le siège concerné.
    vue: Vec<f64>,
    vue_siege: usize,
    tampons: Tampons,
    /// Tampon d'évaluation, réutilisé (aucune allocation dans la boucle chaude).
    essai: Vec<f64>,
    /// Prédiction de victoire du siège 0, une par génération : elle sert à dire
    /// si le vainqueur était bien celui que le réseau donnait gagnant à
    /// mi-partie.
    pub predictions: Vec<f64>,
    pub generation_vue: u32,
    /// **Toutes les réponses de la partie, dans l'ordre**, au format du pont :
    /// c'est la partie elle-même, rejouable telle quelle par `pont.pas`.
    pub journal: Vec<Value>,
    /// Rang de décision à tracer : le joueur imprime alors la note de chaque
    /// option sur la sortie d'erreur. Sert à comparer les deux côtés quand ils
    /// se départagent différemment.
    pub tracer_rang: i64,
    /// Nombre d'essais faits (mesure de coût).
    pub essais: u64,
    /// Pas d'avance consommés au total, et nombre de fois où le plafond du §4.1
    /// (soixante pas) a arrêté une avance. Les deux sont rapportés dans
    /// `result.md`, comme le §4.1 l'exige.
    pub pas_avance: u64,
    pub plafonds: u64,
    /// **Le rythme du §2.2** : on prend une situation d'entraînement sur K.
    /// Corriger à chacune des 341 décisions d'une partie coûterait tout le temps
    /// de calcul et diluerait l'ancrage sur le résultat réel ; K = 8 donne une
    /// quarantaine de corrections par partie, l'ordre de grandeur de la
    /// référence. Réglable par `--rythme`.
    pub rythme: u64,
    /// Décisions vues depuis le début de la partie (le compteur du rythme).
    compteur: u64,
    /// Description de l'état AU REPÈRE de l'option retenue : c'est elle, et elle
    /// seule, que le réseau apprend (§2.2 corrigé le 15-08).
    repere: Vec<f64>,
    /// Chronomètres de mise au point : où passe le temps d'une partie.
    pub t_essais: f64,
    pub t_apprentissage: f64,
    pub passes: u64,
}

impl<'a> Joueur<'a> {
    pub fn new(
        db: &'a CardsDb,
        desc: &'a Description,
        reseau: &'a mut Reseau,
        pile: &'a mut Pile,
        seed: u64,
    ) -> Joueur<'a> {
        let tampons = Tampons::new(desc);
        Joueur {
            db,
            desc,
            reseau,
            pile,
            seed,
            reprise: Reprise::MiseEnPlace,
            reponses: Vec::new(),
            exploration: 0.0,
            apprendre: false,
            taux: crate::reseau::TAUX,
            vue: Vec::new(),
            vue_siege: 0,
            tampons,
            essai: Vec::new(),
            predictions: Vec::new(),
            generation_vue: 0,
            journal: Vec::new(),
            tracer_rang: -1,
            essais: 0,
            pas_avance: 0,
            plafonds: 0,
            rythme: crate::reseau::RYTHME,
            compteur: 0,
            repere: Vec::new(),
            t_essais: 0.0,
            t_apprentissage: 0.0,
            passes: 0,
        }
    }

    /// À appeler avant chaque `play_round` : le point de reprise devient le
    /// début de cette manche-ci.
    pub fn debut_manche(&mut self, game: &GameState) {
        self.reprise = Reprise::Manche(Box::new(game.clone()));
        self.reponses.clear();
    }

    /// À appeler au début d'une partie neuve.
    pub fn nouvelle_partie(&mut self, seed: u64) {
        self.seed = seed;
        self.reprise = Reprise::MiseEnPlace;
        self.reponses.clear();
        self.journal.clear();
        self.pile.vider();
        self.reseau.oublier();
        self.predictions.clear();
        self.generation_vue = 0;
        self.compteur = 0;
    }

    /// **L'état atteint si l'on répondait `candidate`, AU REPÈRE DU §4.1.**
    ///
    /// Ce n'est pas l'état qui suit immédiatement l'option : c'est celui du
    /// **prochain point de décision de `joueur`**, ou la fin de la partie. La
    /// première version de ce contrat évaluait l'état immédiat, et le défaut est
    /// mesuré : « passer » mène à un état plus lointain — production encaissée,
    /// manche suivante entamée — que « poser une carte », qui mène à la décision
    /// d'après. Le réseau comparait alors un état de maintenant à un état de plus
    /// tard, et plus tard paraît toujours meilleur. Le joueur du round 1 avait
    /// appris à attendre : 1001 générations sans jamais terraformer.
    ///
    /// L'avance est faite **dans le rejeu lui-même** (`Rejeu::jusqu_a`) : tant
    /// que la décision atteinte est celle de l'autre joueur, on répond à sa place
    /// et le moteur continue. Un seul rejeu par option, au lieu d'un par pas.
    fn etat_apres(&mut self, joueur: usize, candidate: &Value) -> Option<GameState> {
        let t0 = std::time::Instant::now();
        let mut reponses = self.reponses.clone();
        reponses.push(candidate.clone());
        let mut rejeu = Rejeu::jusqu_a(reponses, joueur);
        self.essais += 1;
        let r = match &self.reprise {
            Reprise::MiseEnPlace => {
                let mut g = setup_game(self.db, self.seed, &mut rejeu);
                while rejeu.attente.is_none() && !g.game_over && g.generation <= MAX_GENERATIONS {
                    play_round(&mut g, self.db, &mut rejeu);
                }
                etat_atteint(&mut rejeu, g)
            }
            Reprise::Manche(base) => {
                let mut g = (**base).clone();
                while rejeu.attente.is_none() && !g.game_over && g.generation <= MAX_GENERATIONS {
                    play_round(&mut g, self.db, &mut rejeu);
                }
                etat_atteint(&mut rejeu, g)
            }
        };
        self.pas_avance += rejeu.pas_avance as u64;
        if rejeu.plafond_atteint {
            self.plafonds += 1;
        }
        self.t_essais += t0.elapsed().as_secs_f64();
        r
    }

    /// **Le cœur du §4** : essayer chaque candidate, garder celle dont ma
    /// probabilité de victoire est la plus haute. Rend l'indice retenu dans
    /// `candidates`, et enregistre la réponse pour la suite du rejeu.
    fn choisir(&mut self, rng: &mut StdRng, joueur: usize, candidates: &[Value]) -> usize {
        if candidates.is_empty() {
            return 0; // le moteur n'offre rien : il n'y a rien à essayer
        }
        let choix = if candidates.len() == 1 {
            0
        } else if self.exploration > 0.0 && rng.gen::<f64>() < self.exploration {
            // L'exploration du §5 : sans elle, deux joueurs identiques et
            // déterministes rejouent sans cesse des parties très ressemblantes.
            rng.gen_range(0..candidates.len())
        } else {
            let tracer = self.tracer_rang >= 0 && self.journal.len() as i64 == self.tracer_rang;
            let mut meilleur = 0usize;
            let mut meilleure_note = f64::NEG_INFINITY;
            for (i, c) in candidates.iter().enumerate() {
                let note = match self.etat_apres(joueur, c) {
                    Some(g) => {
                        // Toujours MON point de vue : celui du joueur qui
                        // décidait, jamais celui à qui la main revient.
                        self.desc
                            .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                        let p = self.reseau.evaluer(&self.essai);
                        p[0]
                    }
                    None => f64::NEG_INFINITY,
                };
                if tracer {
                    eprintln!("rang {} option {i} : note {note:.17}", self.journal.len());
                }
                if note > meilleure_note + MARGE {
                    meilleure_note = note;
                    meilleur = i;
                }
            }
            meilleur
        };
        self.apprendre_au_repere(joueur, &candidates[choix]);
        self.reponses.push(candidates[choix].clone());
        self.journal.push(candidates[choix].clone());
        choix
    }

    /// **Un choix MULTIPLE.** Une décision multiple n'a pas une option par
    /// réponse mais une COMBINAISON, et le moteur n'accepte que les combinaisons
    /// de la taille exacte qu'il demande : une liste à moitié construite est
    /// refusée, pas évaluée. On ne peut donc pas l'assembler en ajoutant une
    /// carte à la fois — chaque candidat essayé doit être une réponse complète.
    ///
    /// Deux cas, et ils ne se traitent pas pareil :
    ///
    /// - **nombre libre** (le mulligan projets, de 0 à 8) : toute liste est une
    ///   réponse valable, y compris la liste vide. On part d'elle et on ajoute la
    ///   carte qui améliore le plus, tant qu'une addition améliore.
    /// - **nombre imposé** (garder k cartes, en défausser n) : on part des k
    ///   premières — une réponse complète, donc évaluable — et on essaie de
    ///   REMPLACER chaque carte retenue par chacune des autres, en gardant tout
    ///   remplacement qui améliore. Chaque carte est ainsi essayée à chaque
    ///   place. Deux tours suffisent en pratique et bornent le coût.
    ///
    /// Le JavaScript fait exactement la même chose, dans le même ordre : c'est ce
    /// que vérifie `web/webapp/verif/juge-meme-option.mjs`.
    fn choisir_liste(
        &mut self,
        rng: &mut StdRng,
        joueur: usize,
        n: usize,
        attendu: usize,
        libre: bool,
    ) -> Vec<usize> {
        let mut pris: Vec<usize> = Vec::new();
        if self.exploration > 0.0 && rng.gen::<f64>() < self.exploration {
            let mut reste: Vec<usize> = (0..n).collect();
            let combien = if libre { rng.gen_range(0..=n) } else { attendu.min(n) };
            for _ in 0..combien {
                let k = rng.gen_range(0..reste.len());
                pris.push(reste.remove(k));
            }
            self.apprendre_au_repere(joueur, &json!(pris));
            self.reponses.push(json!(pris));
            self.journal.push(json!(pris));
            return pris;
        }
        if libre {
            let mut note = self.noter_liste(joueur, &pris);
            while pris.len() < n {
                let mut meilleur: Option<(usize, f64)> = None;
                for i in 0..n {
                    if pris.contains(&i) {
                        continue;
                    }
                    pris.push(i);
                    let x = self.noter_liste(joueur, &pris);
                    pris.pop();
                    if meilleur.is_none() || x > meilleur.unwrap().1 + MARGE {
                        meilleur = Some((i, x));
                    }
                }
                match meilleur {
                    Some((i, x)) if x > note + MARGE => {
                        pris.push(i);
                        note = x;
                    }
                    _ => break,
                }
            }
        } else {
            pris = (0..attendu.min(n)).collect();
            let mut note = self.noter_liste(joueur, &pris);
            for _tour in 0..2 {
                let mut ameliore = false;
                for p in 0..pris.len() {
                    for c in 0..n {
                        if pris.contains(&c) {
                            continue;
                        }
                        let ancien = pris[p];
                        pris[p] = c;
                        let x = self.noter_liste(joueur, &pris);
                        if x > note + MARGE {
                            note = x;
                            ameliore = true;
                        } else {
                            pris[p] = ancien;
                        }
                    }
                }
                if !ameliore {
                    break;
                }
            }
        }
        self.apprendre_au_repere(joueur, &json!(pris));
        self.reponses.push(json!(pris));
        self.journal.push(json!(pris));
        pris
    }

    fn noter_liste(&mut self, joueur: usize, pris: &[usize]) -> f64 {
        let c = json!(pris);
        match self.etat_apres(joueur, &c) {
            Some(g) => {
                self.desc
                    .decrire(&g, self.db, joueur, &mut self.essai, &mut self.tampons);
                self.reseau.evaluer(&self.essai)[0]
            }
            None => f64::NEG_INFINITY,
        }
    }

    /// **LE POINT DE RENDEZ-VOUS DE L'APPRENTISSAGE — CORRIGÉ LE 15-08 (§2.2).**
    ///
    /// « Le réseau s'entraîne exactement sur les situations qu'il juge, et sur
    /// aucune autre. » La version du round 1 corrigeait au choix de la carte
    /// Phase — un instant où `phase_en_cours` vaut toujours 0, alors que le
    /// joueur évalue des situations en pleine phase. Mesuré : l'écart
    /// d'évaluation entre deux options d'une même décision tombait à 0,016, le
    /// niveau du bruit. Le réseau ne départageait plus rien.
    ///
    /// La situation apprise est donc **celle du repère du §4.1, pour l'option
    /// RETENUE** : très exactement l'état sur lequel le joueur vient de fonder
    /// son choix. Une prise sur `rythme` décisions (K = 8), parce que corriger
    /// aux 341 décisions d'une partie coûterait tout le temps de calcul et
    /// diluerait encore l'ancrage sur le résultat réel.
    ///
    /// Le coût est d'un rejeu de plus par prise — un essai sur K, moins de trois
    /// pour cent des essais — contre la complication d'un tampon retenu au vol
    /// pendant la boucle des candidates.
    fn apprendre_au_repere(&mut self, joueur: usize, retenue: &Value) {
        self.compteur += 1;
        if !self.apprendre || self.rythme == 0 || self.compteur % self.rythme != 0 {
            return;
        }
        let Some(g) = self.etat_apres(joueur, retenue) else {
            return;
        };
        let mut repere = std::mem::take(&mut self.repere);
        self.desc
            .decrire(&g, self.db, joueur, &mut repere, &mut self.tampons);
        // §2.2 : la situation présente passe dans le réseau, ses deux
        // probabilités deviennent LA CIBLE ; puis on remonte la pile.
        // §2.1 : on empile juste avant de corriger.
        self.reseau.oublier();
        let p = self.reseau.evaluer(&repere);
        self.pile.empiler(&repere, joueur);
        let t0 = std::time::Instant::now();
        self.reseau.corriger(self.pile, joueur, p, self.taux);
        self.t_apprentissage += t0.elapsed().as_secs_f64();
        self.passes += 1;
        self.repere = repere;
    }

    /// **Le relevé qui alimente `justes`**, une fois par génération et par
    /// joueur, à la carte Phase : la probabilité que le réseau accorde au siège 0
    /// sur l'état vivant. Il ne corrige rien — c'est une mesure, et elle reste
    /// prise au même endroit qu'au round 1 pour que les deux courbes se
    /// comparent.
    fn relever_prediction(&mut self) {
        if self.vue.is_empty() {
            return;
        }
        self.reseau.oublier();
        let p = self.reseau.evaluer(&self.vue);
        let p0 = if self.vue_siege == 0 { p[0] } else { p[1] };
        if self.predictions.len() < self.generation_vue as usize {
            self.predictions.resize(self.generation_vue as usize, 0.5);
        }
        if self.predictions.len() == self.generation_vue as usize {
            self.predictions.push(p0);
        }
    }
}

impl Policy for Joueur<'_> {
    /// L'état vivant, juste avant chaque décision : on en garde la description
    /// du point de vue du joueur qui va décider.
    fn observe(&mut self, game: &GameState, player: usize) {
        self.desc
            .decrire(game, self.db, player, &mut self.vue, &mut self.tampons);
        self.vue_siege = player;
        self.generation_vue = game.generation.saturating_sub(1);
    }

    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, _corps: &[u16]) -> bool {
        let c = [json!(0), json!(1)];
        self.choisir(rng, player, &c) == 1
    }

    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize> {
        self.choisir_liste(rng, player, hand.len(), 0, true)
    }

    fn pick_corporation(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> usize {
        let c: Vec<Value> = (0..corps.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn pick_phase(&mut self, rng: &mut StdRng, player: usize, allowed: &[u8]) -> u8 {
        self.relever_prediction();
        let c: Vec<Value> = (0..allowed.len()).map(|i| json!(i)).collect();
        allowed[self.choisir(rng, player, &c)]
    }

    fn choose_build(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        affordable: &[usize],
    ) -> Option<usize> {
        // L'indice `affordable.len()` est « passer », comme chez le pont.
        let c: Vec<Value> = (0..=affordable.len()).map(|i| json!(i)).collect();
        let i = self.choisir(rng, player, &c);
        if i < affordable.len() {
            Some(affordable[i])
        } else {
            None
        }
    }

    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let c = [json!(0), json!(1), json!(2)];
        match self.choisir(rng, player, &c) {
            0 => ConstructionBonus::DrawCardBefore,
            1 => ConstructionBonus::DrawCard,
            _ => ConstructionBonus::SecondBuild,
        }
    }

    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        let c = [json!(0), json!(1)];
        self.choisir(rng, player, &c) == 0
    }

    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        let c = [json!(0), json!(1)];
        if self.choisir(rng, player, &c) == 1 {
            ConstructionBonus::SecondBuild
        } else {
            ConstructionBonus::DrawCard
        }
    }

    fn action_choice(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        options: &[ActionOpt],
    ) -> Option<usize> {
        if options.is_empty() {
            return None;
        }
        let c: Vec<Value> = (0..=options.len()).map(|i| json!(i)).collect();
        let i = self.choisir(rng, player, &c);
        if i < options.len() {
            Some(i)
        } else {
            None
        }
    }

    /// **Le montant est TOUJOURS une décision, même quand il n'y a qu'un montant
    /// possible.** Le pont pose la question sans condition (`Harnais::action_amount`)
    /// et la page y répond : une réponse entre donc dans la liste des décisions.
    /// Court-circuiter le cas `max <= 0` ferait consommer une réponse de plus au
    /// JavaScript qu'au Rust, et tout le rejeu se décalerait d'un cran — le genre
    /// de divergence qui ne se voit que sur des milliers de décisions.
    fn action_amount(&mut self, rng: &mut StdRng, player: usize, max: i64) -> i64 {
        let c: Vec<Value> = if max <= 0 {
            vec![json!(0)]
        } else {
            (0..=max).map(|i| json!(i)).collect()
        };
        self.choisir(rng, player, &c) as i64
    }

    /// **La vente n'est pas essayée** : elle n'est pas une option énumérée par le
    /// moteur mais une entrée d'occasion, offerte avant CHAQUE décision des
    /// phases dépensables. L'essayer coûterait un rejeu de manche à chacune de
    /// ces occasions — plusieurs fois le prix de tout le reste — pour un gain que
    /// rien ne mesure encore. Déclaré dans `result.md`.
    fn vendre_librement(&mut self, _rng: &mut StdRng, _joueur: usize, _main: &[u16]) -> Vec<usize> {
        Vec::new()
    }

    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        let n = ctx.option_count();
        if n == 0 {
            return 0;
        }
        let c: Vec<Value> = (0..n).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn choose_res_target(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let c: Vec<Value> = (0..candidates.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn choose_res_source(&mut self, rng: &mut StdRng, player: usize, candidates: &[u16]) -> usize {
        if candidates.is_empty() {
            return 0;
        }
        let c: Vec<Value> = (0..candidates.len()).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn pick_joker_tag(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        _card: u16,
        _tag_counts: &[u32],
    ) -> usize {
        let n = engine::cards::JOKER_TAG_CHOICES.len();
        let c: Vec<Value> = (0..n).map(|i| json!(i)).collect();
        self.choisir(rng, player, &c)
    }

    fn research_keep(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        drawn: &[u16],
        keep: usize,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, drawn.len(), keep, false)
    }

    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        _revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        _filter: RevealFilter,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, candidates.len(), keep, false)
    }

    fn discard_down(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        hand: &[u16],
        n: usize,
    ) -> Vec<usize> {
        self.choisir_liste(rng, player, hand.len(), n, false)
    }
}

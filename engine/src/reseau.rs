//! **(le-juge-apprend) LE RÉSEAU QUI JUGE UNE SITUATION, et comment il apprend.**
//!
//! Forme imposée (§1) : une seule couche cachée, cinquante neurones, deux
//! sorties (une par joueur), un neurone de biais de chaque côté, tangente
//! hyperbolique en couche cachée, exponentielle normalisée en sortie. Environ
//! soixante-treize mille poids pour les 1472 entrées de `description.rs`.
//!
//! Apprentissage par différences temporelles (§2) : on n'attend pas de savoir
//! qui a gagné pour corriger, on corrige en permanence les jugements passés vers
//! le jugement présent, mieux informé ; à la fin de la partie, vers le résultat
//! réel — réparti en douceur selon l'écart de score, parce que gagner de deux
//! points ne s'apprend pas comme gagner de trente.
//!
//! **L'ordre des sorties est relatif au joueur qui regarde** : la sortie 0 est
//! toujours la probabilité que gagne le joueur du point de vue duquel la
//! situation est décrite. C'est ce qui permet à un seul réseau de servir aux deux
//! sièges.
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! **DEUX OPTIMISATIONS, ET SANS ELLES L'ENTRAÎNEMENT NE TIENT PAS DANS LA
//! JOURNÉE.** Toutes deux sont de l'arithmétique réorganisée : aucune formule du
//! §2 n'est changée, et les mesures sont dans `result.md`.
//!
//! 1. **La mise à jour par différences (§1.1)** : `evaluer` garde les entrées du
//!    calcul précédent et ne touche les sommes cachées qu'avec la DIFFÉRENCE des
//!    entrées qui ont changé. Mesuré : 75 µs l'évaluation complète, 1,7 µs la
//!    différentielle — quarante-quatre fois moins. La mémoire se remet à zéro par
//!    [`Reseau::oublier`], appelée aux deux endroits qui changent de fil :
//!    nouvelle partie, et passage à l'entraînement des situations passées.
//!
//! 2. **Le produit externe factorisé**, et c'est celle qui a décidé du chantier.
//!    Une correction remonte la pile : jusqu'à cinquante situations, chacune
//!    corrigeant les 73 650 poids de la couche cachée. La correction du poids
//!    (entrée i, neurone j) vaut `− Σ_k d_k[j] · x_k[i]` : un produit externe,
//!    répété. Or deux situations consécutives de la pile ne diffèrent que par une
//!    poignée d'entrées. En écrivant `x_k = x_{k−1} + δ_k`, la somme devient
//!
//!        Σ_k d_k[j]·x_k[i]  =  S_0[j]·x_0[i]  +  Σ_{m≥1} S_m[j]·δ_m[i]
//!
//!    avec `S_m = Σ_{k≥m} d_k` (sommes suffixes). Un seul parcours complet du
//!    tableau des poids par correction, au lieu de cinquante ; le reste ne touche
//!    que les entrées qui ont bougé. C'est la même idée que le §1.1, appliquée à
//!    la descente au lieu de la montée.
//!
//! **La disposition des poids sert ces deux optimisations** : `w_cache` est rangé
//! PAR ENTRÉE (les cinquante poids d'une même entrée sont contigus), parce que
//! les deux optimisations travaillent entrée par entrée. Rangé par neurone, le
//! même calcul saute de onze kilooctets à chaque pas et la mémoire devient le
//! goulot.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const CACHES: usize = 50;
pub const SORTIES: usize = 2;
/// Taux d'apprentissage (§2.4). Très petit, et délibérément : le réseau voit des
/// centaines de milliers de parties, chaque correction doit être minuscule.
pub const TAUX: f64 = 0.0001;
/// **Facteur d'influence par pas en arrière dans la pile (§2.2) — 0,9 depuis la
/// correction du 15-08.**
///
/// Les parties de la référence durent une douzaine de tours : avec 0,7, sept pas
/// en arrière y couvrent plus de la moitié d'une partie et le résultat réel pèse
/// un douzième des corrections. Les nôtres durent 45 générations : le résultat
/// réel ne pesait plus que 4 % (mesuré au round 1), le réseau s'entraînait
/// presque uniquement vers lui-même, et son jugement s'est effondré vers
/// l'indécision — « 45 points contre 5 » évalué 0,857 après l'amorçage, 0,501
/// après 10 000 parties. À 0,9, l'influence retombe sous 0,09 après vingt-trois
/// pas au lieu de sept.
pub const LAMBDA: f64 = 0.9;
/// **Le rythme des corrections (§2.2)** : une situation sur K. Corriger à chacune
/// des 341 décisions d'une partie coûterait tout le temps de calcul et diluerait
/// l'ancrage sur le résultat réel ; K = 8 donne une quarantaine de corrections
/// par partie, l'ordre de grandeur de la référence.
pub const RYTHME: u64 = 8;
/// Amorçage (§2.7) : cinq mille fins de partie fabriquées, taux ×10.
pub const AMORCAGE_PARTIES: usize = 5000;
pub const AMORCAGE_FACTEUR: f64 = 10.0;
pub const AMORCAGE_SCORE_MAX: i64 = 49;
/// Répartition douce de la cible de fin de partie (§2.3).
pub const DOUCEUR: f64 = 0.3;
/// Graine du tirage des poids de départ. Fixe : « deux entraînements lancés avec
/// les mêmes arguments doivent produire exactement le même fichier » (§1).
pub const GRAINE_POIDS: u64 = 20260815;
/// Hauteur de la pile des situations passées (§2.1).
pub const PILE_MAX: usize = 120;

// ---------------------------------------------------------------------------
// La pile des situations passées
// ---------------------------------------------------------------------------

/// **Un pas de la pile est une PRISE D'ENTRAÎNEMENT, pas une décision** (§2.1) —
/// c'est ce qui donne son sens au facteur d'influence. Avec le rythme K = 8 du
/// §2.2 corrigé, une partie de 341 décisions en produit une quarantaine, soit
/// l'ordre d'une par génération : à 0,9, l'influence retombe sous 0,09 au bout de
/// vingt-trois pas, c'est-à-dire la moitié d'une partie — l'ordre de grandeur de
/// la référence. Plafonnée à cent vingt, vidée à la fin de la partie.
pub struct Pile {
    situations: Vec<Vec<f64>>,
    sieges: Vec<usize>,
    debut: usize,
    taille: usize,
}

impl Pile {
    pub fn new(n_entrees: usize) -> Pile {
        Pile {
            situations: (0..PILE_MAX).map(|_| vec![0.0; n_entrees]).collect(),
            sieges: vec![0; PILE_MAX],
            debut: 0,
            taille: 0,
        }
    }

    pub fn vider(&mut self) {
        self.debut = 0;
        self.taille = 0;
    }

    pub fn empiler(&mut self, x: &[f64], siege: usize) {
        let i = (self.debut + self.taille) % PILE_MAX;
        self.situations[i].copy_from_slice(x);
        self.sieges[i] = siege;
        if self.taille == PILE_MAX {
            self.debut = (self.debut + 1) % PILE_MAX;
        } else {
            self.taille += 1;
        }
    }

    /// Les rangs des situations de ce joueur, de la plus récente à la plus
    /// ancienne.
    pub fn rangs_a_rebours(&self, joueur: usize, out: &mut Vec<usize>) {
        out.clear();
        for k in (0..self.taille).rev() {
            let i = (self.debut + k) % PILE_MAX;
            if self.sieges[i] == joueur {
                out.push(i);
            }
        }
    }

    pub fn situation(&self, i: usize) -> &[f64] {
        &self.situations[i]
    }
}

// ---------------------------------------------------------------------------
// Le réseau
// ---------------------------------------------------------------------------

pub struct Reseau {
    pub n_entrees: usize,
    /// Poids de la couche cachée, **rangés par entrée** :
    /// `w_cache[i * CACHES + j]`. La ligne `i == n_entrees` porte le biais
    /// d'entrée (valeur fixe 1).
    pub w_cache: Vec<f64>,
    /// Poids de sortie : `w_sortie[k * (CACHES + 1) + j]`, `j == CACHES` portant
    /// le biais de couche cachée (valeur fixe 1).
    pub w_sortie: Vec<f64>,
    /// Corrections accumulées, appliquées à la fin de la passe (§2.5).
    acc_cache: Vec<f64>,
    acc_sortie: Vec<f64>,
    /// Nombre de parties d'entraînement déjà vues (écrit dans le fichier, §7).
    pub parties: u64,

    // ---- état du dernier calcul (l'optimisation du §1.1)
    precedentes: Vec<f64>,
    sommes: Vec<f64>,
    pub h: Vec<f64>,
    pub p: [f64; SORTIES],
    e: [f64; SORTIES],
    total_e: f64,

    // ---- tampons de la correction factorisée
    ds: Vec<[f64; CACHES]>,
    rangs: Vec<usize>,

    /// Facteur d'influence par pas en arrière dans la pile. **La version livrée
    /// est celle du §2.2 corrigé le 15-08 : 0,9.** Réglable pour la mesure que le
    /// prompt demande (« tu mesures aussi 0,8 et 0,97 ») — et pour elle seule.
    pub lambda: f64,

    /// Débrancher l'optimisation du §1.1 : chaque évaluation refait le calcul
    /// complet. Sert uniquement à MESURER ce que l'optimisation rapporte
    /// (`entraine --sans-optimisation`).
    pub sans_optimisation: bool,

    // ---- statistiques
    pub somme_erreur2: f64,
    pub compte_erreur: u64,
}

/// **L'amplitude des poids de départ (§1) : ±0,1.** C'est la valeur spécifiée, et
/// c'est celle que `Reseau::neuf` emploie.
pub const AMPLITUDE_DEPART: f64 = 0.1;

impl Reseau {
    /// Un réseau neuf : poids tirés uniformément entre −0,1 et +0,1, générateur
    /// semé (§1).
    pub fn neuf(n_entrees: usize) -> Reseau {
        Reseau::neuf_amplitude(n_entrees, AMPLITUDE_DEPART)
    }

    /// **Le même réseau, avec une autre amplitude de départ — pour la MESURE que
    /// le préambule de la spécification autorise (« tu peux proposer mieux, mais
    /// tu livres d'abord la version spécifiée, tu mesures les deux »).**
    ///
    /// Ce qui la motive, et c'est arithmétique : la description compte 1472
    /// entrées valant toutes ±1. Avec des poids tirés dans ±0,1, la somme
    /// pondérée d'un neurone caché a un écart-type de √(1472/3) × 0,1 ≈ **2,2** —
    /// la tangente hyperbolique y est déjà couchée, et sa dérivée (1 − h²) vaut
    /// moins d'un dixième. Les mille trois cents drapeaux de cartes, qui ne
    /// bougent presque jamais, écrasent ainsi le signal des thermomètres, qui,
    /// eux, portent tout ce qui distingue deux options. La référence, elle, n'a
    /// que 704 entrées : le même 0,1 y donne un écart-type de 1,5.
    ///
    /// `--amplitude-depart 0.045` ramène cet écart-type à 1,0 pour 1472 entrées.
    /// La valeur par défaut reste 0,1, et `Reseau::neuf` est inchangée : sans
    /// l'argument, l'entraînement est celui du §1 au bit près.
    pub fn neuf_amplitude(n_entrees: usize, amplitude: f64) -> Reseau {
        let mut rng = StdRng::seed_from_u64(GRAINE_POIDS);
        let mut w_cache = vec![0.0; (n_entrees + 1) * CACHES];
        for w in w_cache.iter_mut() {
            *w = rng.gen_range(-amplitude..amplitude);
        }
        let mut w_sortie = vec![0.0; (CACHES + 1) * SORTIES];
        for w in w_sortie.iter_mut() {
            *w = rng.gen_range(-amplitude..amplitude);
        }
        Reseau {
            n_entrees,
            acc_cache: vec![0.0; w_cache.len()],
            acc_sortie: vec![0.0; w_sortie.len()],
            w_cache,
            w_sortie,
            parties: 0,
            precedentes: Vec::new(),
            sommes: vec![0.0; CACHES],
            h: vec![0.0; CACHES],
            p: [0.5; SORTIES],
            e: [1.0; SORTIES],
            total_e: 2.0,
            ds: Vec::new(),
            rangs: Vec::new(),
            lambda: LAMBDA,
            sans_optimisation: false,
            somme_erreur2: 0.0,
            compte_erreur: 0,
        }
    }

    /// **Remise à zéro de la mémoire du §1.1.** À appeler chaque fois qu'on
    /// change de fil : nouvelle partie, ou passage à l'entraînement d'une
    /// situation passée. Sans elle, on accumulerait les différences d'états sans
    /// rapport et le réseau calculerait n'importe quoi.
    pub fn oublier(&mut self) {
        self.precedentes.clear();
    }

    /// Évalue une situation. Rend les deux probabilités de victoire, la première
    /// étant celle du joueur du point de vue duquel `entrees` a été écrit.
    pub fn evaluer(&mut self, entrees: &[f64]) -> [f64; SORTIES] {
        debug_assert_eq!(entrees.len(), self.n_entrees);
        let n = self.n_entrees;
        if self.sans_optimisation || self.precedentes.len() != n {
            // Calcul complet : le premier de la partie, ou le premier après un
            // oubli. Boucle entrée par entrée : `w_cache` est rangé pour cela.
            self.sommes
                .copy_from_slice(&self.w_cache[n * CACHES..(n + 1) * CACHES]);
            for i in 0..n {
                let x = entrees[i];
                if x == 0.0 {
                    continue;
                }
                let w = &self.w_cache[i * CACHES..(i + 1) * CACHES];
                for j in 0..CACHES {
                    self.sommes[j] += x * w[j];
                }
            }
            self.precedentes.clear();
            self.precedentes.extend_from_slice(entrees);
        } else {
            // Seules les entrées qui ont changé touchent les sommes.
            for i in 0..n {
                let d = entrees[i] - self.precedentes[i];
                if d != 0.0 {
                    let w = &self.w_cache[i * CACHES..(i + 1) * CACHES];
                    for j in 0..CACHES {
                        self.sommes[j] += d * w[j];
                    }
                    self.precedentes[i] = entrees[i];
                }
            }
        }
        for j in 0..CACHES {
            self.h[j] = self.sommes[j].tanh();
        }
        // Sortie : exponentielle normalisée, mise à l'échelle par la première
        // somme (elle ne change pas le résultat, elle évite les débordements).
        let mut s = [0.0f64; SORTIES];
        for k in 0..SORTIES {
            let base = k * (CACHES + 1);
            let mut x = self.w_sortie[base + CACHES]; // biais de couche cachée
            for j in 0..CACHES {
                x += self.h[j] * self.w_sortie[base + j];
            }
            s[k] = x;
        }
        let s0 = s[0];
        let mut total = 0.0;
        for k in 0..SORTIES {
            self.e[k] = (s[k] - s0).exp();
            total += self.e[k];
        }
        self.total_e = total;
        for k in 0..SORTIES {
            self.p[k] = self.e[k] / total;
        }
        self.p
    }

    /// La moitié « sortie » d'une passe d'entraînement : elle accumule les
    /// corrections des poids de sortie et rend, pour chaque neurone caché, le
    /// facteur `d_j` du produit externe des poids d'entrée (§2.6).
    ///
    /// Le réseau doit venir d'évaluer la situation : `h`, `p` et les
    /// exponentielles en viennent.
    fn accumuler_sortie(&mut self, cible: [f64; SORTIES], lambda: f64, taux: f64) -> [f64; CACHES] {
        let mut erreur = [0.0f64; SORTIES];
        let mut derivee = [0.0f64; SORTIES];
        for k in 0..SORTIES {
            erreur[k] = lambda * (self.p[k] - cible[k]);
            derivee[k] = self.p[k] * (1.0 - self.p[k]);
            self.somme_erreur2 += erreur[k] * erreur[k];
        }
        self.compte_erreur += 1;

        for k in 0..SORTIES {
            let base = k * (CACHES + 1);
            let g = taux * erreur[k] * derivee[k];
            for j in 0..CACHES {
                self.acc_sortie[base + j] -= g * self.h[j];
            }
            self.acc_sortie[base + CACHES] -= g; // biais : h vaut 1
        }

        // Erreur remontée vers chaque neurone caché. Le second terme est le
        // couplage propre à l'exponentielle normalisée : sans lui, on
        // entraînerait deux sorties indépendantes (§2.6).
        let t2 = self.total_e * self.total_e;
        let mut d = [0.0f64; CACHES];
        for j in 0..CACHES {
            let mut eh = 0.0;
            for k in 0..SORTIES {
                let bk = k * (CACHES + 1);
                let mut g = derivee[k] * self.w_sortie[bk + j];
                for l in 0..SORTIES {
                    if l != k {
                        let bl = l * (CACHES + 1);
                        g -= self.w_sortie[bl + j] * (self.e[k] * self.e[l] / t2);
                    }
                }
                eh += erreur[k] * g;
            }
            // Dérivée de la tangente hyperbolique, et le taux : tout ce qui ne
            // dépend pas de l'entrée `i`.
            d[j] = taux * eh * (1.0 - self.h[j] * self.h[j]);
        }
        d
    }

    /// Le produit externe d'une seule situation : `acc[i][j] -= d[j] · x[i]`.
    fn produit_externe(&mut self, x: &[f64], d: &[f64; CACHES]) {
        let n = self.n_entrees;
        for i in 0..n {
            let xi = x[i];
            if xi == 0.0 {
                // Une entrée nulle ne produit aucune correction — on la saute
                // (§2.6). Avec la convention ±1 du §3.1, ce cas ne se présente
                // jamais : le raccourci est écrit, il ne mord pas.
                continue;
            }
            let acc = &mut self.acc_cache[i * CACHES..(i + 1) * CACHES];
            for j in 0..CACHES {
                acc[j] -= d[j] * xi;
            }
        }
        let acc = &mut self.acc_cache[n * CACHES..(n + 1) * CACHES];
        for j in 0..CACHES {
            acc[j] -= d[j]; // biais d'entrée : valeur fixe 1
        }
    }

    /// **Une passe d'entraînement isolée** (l'amorçage du §2.7) : évaluer,
    /// accumuler, appliquer.
    pub fn entrainer_une(&mut self, x: &[f64], cible: [f64; SORTIES], taux: f64) {
        self.oublier();
        self.evaluer(x);
        let d = self.accumuler_sortie(cible, 1.0, taux);
        self.produit_externe(x, &d);
        self.appliquer();
    }

    /// **LA CORRECTION DU §2.2 ET DU §2.3.**
    ///
    /// On remonte la pile, de la plus récente à la plus ancienne, en ne gardant
    /// que les situations où ce joueur décidait, et on les entraîne vers `cible`
    /// avec un poids d'influence qui part de 1 et se multiplie par `lambda`
    /// (0,9 depuis le 15-08) à chaque pas en arrière. Les corrections s'accumulent et ne sont versées dans les
    /// poids qu'à la fin (§2.5) — sinon corriger la situation la plus récente
    /// changerait le réseau avec lequel on évalue la suivante.
    pub fn corriger(&mut self, pile: &Pile, joueur: usize, cible: [f64; SORTIES], taux: f64) {
        let mut rangs = std::mem::take(&mut self.rangs);
        pile.rangs_a_rebours(joueur, &mut rangs);
        if rangs.is_empty() {
            self.rangs = rangs;
            return;
        }
        // Changement de fil (§1.1) : on quitte le jeu pour entraîner des
        // situations passées. On oublie ICI, une fois — et pas entre deux
        // situations de la pile : les poids ne bougent pas avant `appliquer`, la
        // mise à jour par différences reste donc exacte de l'une à l'autre.
        self.oublier();
        let mut ds = std::mem::take(&mut self.ds);
        ds.clear();
        let mut lambda = 1.0;
        for r in rangs.iter() {
            self.evaluer(pile.situation(*r));
            let d = self.accumuler_sortie(cible, lambda, taux);
            ds.push(d);
            lambda *= self.lambda;
        }

        // Sommes suffixes : `S_m = Σ_{k ≥ m} d_k`.
        let k = ds.len();
        for m in (0..k - 1).rev() {
            for j in 0..CACHES {
                ds[m][j] += ds[m + 1][j];
            }
        }
        // La situation la plus récente porte le produit externe complet…
        let s0 = ds[0];
        self.produit_externe(pile.situation(rangs[0]), &s0);
        // … et chaque pas en arrière ne coûte que ce qui a changé.
        let n = self.n_entrees;
        for m in 1..k {
            let x = pile.situation(rangs[m]);
            let precedent = pile.situation(rangs[m - 1]);
            let sm = &ds[m];
            for i in 0..n {
                let d = x[i] - precedent[i];
                if d != 0.0 {
                    let acc = &mut self.acc_cache[i * CACHES..(i + 1) * CACHES];
                    for j in 0..CACHES {
                        acc[j] -= sm[j] * d;
                    }
                }
            }
        }
        self.ds = ds;
        self.rangs = rangs;
        self.appliquer();
    }

    /// Verse les corrections accumulées dans les poids, et remet l'accumulateur
    /// à zéro. Les sommes cachées mémorisées ne valent plus rien après cela : on
    /// oublie.
    pub fn appliquer(&mut self) {
        for (w, a) in self.w_cache.iter_mut().zip(self.acc_cache.iter_mut()) {
            *w += *a;
            *a = 0.0;
        }
        for (w, a) in self.w_sortie.iter_mut().zip(self.acc_sortie.iter_mut()) {
            *w += *a;
            *a = 0.0;
        }
        self.oublier();
    }

    /// **La cible de fin de partie (§2.3)** : une répartition douce selon l'écart
    /// de score, du point de vue du joueur qui regarde. Gagner de deux points ne
    /// s'apprend pas comme gagner de trente.
    pub fn cible_finale(score_moi: i64, score_autre: i64) -> [f64; SORTIES] {
        let meilleur = score_moi.max(score_autre) as f64;
        let a = (DOUCEUR * (score_moi as f64 - meilleur)).exp();
        let b = (DOUCEUR * (score_autre as f64 - meilleur)).exp();
        let t = a + b;
        [a / t, b / t]
    }

    /// **L'écart moyen de prédiction**, racine de la moyenne des carrés des
    /// erreurs accumulées depuis le dernier `raz_stats`.
    pub fn erreur_moyenne(&self) -> f64 {
        if self.compte_erreur == 0 {
            0.0
        } else {
            (self.somme_erreur2 / (self.compte_erreur * SORTIES as u64) as f64).sqrt()
        }
    }

    pub fn raz_stats(&mut self) {
        self.somme_erreur2 = 0.0;
        self.compte_erreur = 0;
    }

    /// **Relecture d'un fichier de poids**, avec le MÊME verrou que côté
    /// JavaScript : la table des noms du fichier doit être exactement celle que
    /// ce dépôt régénère. Au premier écart, on refuse — les poids auraient été
    /// appris sur une autre description.
    pub fn lire(chemin: &str, noms: &[String]) -> Result<Reseau, String> {
        let texte = std::fs::read_to_string(chemin).map_err(|e| format!("{chemin} : {e}"))?;
        let mut lignes = texte.lines();
        let tete: Vec<usize> = lignes
            .next()
            .unwrap_or("")
            .split_whitespace()
            .filter_map(|x| x.parse().ok())
            .collect();
        if tete.len() < 3 {
            return Err(format!("{chemin} : première ligne illisible (§7)"));
        }
        let (n, caches, sorties) = (tete[0], tete[1], tete[2]);
        if caches != CACHES || sorties != SORTIES {
            return Err(format!(
                "{chemin} : {caches} neurones cachés et {sorties} sorties (le §1 en impose {CACHES} et {SORTIES})"
            ));
        }
        let parties: u64 = lignes.next().unwrap_or("0").trim().parse().unwrap_or(0);
        if n != noms.len() {
            return Err(format!(
                "{chemin} : {n} entrées, ce dépôt en produit {} — les poids ont été appris sur une AUTRE description",
                noms.len()
            ));
        }
        for (i, attendu) in noms.iter().enumerate() {
            let lu = lignes.next().unwrap_or("");
            if lu != attendu {
                return Err(format!(
                    "{chemin} : divergence de description au rang {i} (« {lu} » contre « {attendu} »)"
                ));
            }
        }
        let mut r = Reseau::neuf(n);
        r.parties = parties;
        let mut k = 0usize;
        let total = r.w_cache.len() + r.w_sortie.len();
        for l in lignes {
            let t = l.trim();
            if t.is_empty() {
                continue;
            }
            let v: f64 = t.parse().map_err(|_| format!("{chemin} : poids illisible « {t} »"))?;
            if k < r.w_cache.len() {
                r.w_cache[k] = v;
            } else if k < total {
                let j = k - r.w_cache.len();
                r.w_sortie[j] = v;
            }
            k += 1;
        }
        if k != total {
            return Err(format!("{chemin} : {k} poids lus, {total} attendus"));
        }
        Ok(r)
    }

    /// **Le fichier de poids du §7** : la taille, le nombre de parties, **le nom
    /// de chaque entrée** — le verrou anti-divergence — puis les poids de la
    /// couche cachée (entrée par entrée, cinquante par ligne d'entrée, la
    /// dernière entrée étant le biais), puis ceux de la sortie.
    pub fn ecrire(&self, chemin: &str, noms: &[String]) -> std::io::Result<()> {
        use std::io::Write;
        let f = std::fs::File::create(chemin)?;
        let mut w = std::io::BufWriter::new(f);
        writeln!(w, "{} {} {}", self.n_entrees, CACHES, SORTIES)?;
        writeln!(w, "{}", self.parties)?;
        for nom in noms {
            writeln!(w, "{nom}")?;
        }
        for x in self.w_cache.iter() {
            writeln!(w, "{x:.12e}")?;
        }
        for x in self.w_sortie.iter() {
            writeln!(w, "{x:.12e}")?;
        }
        w.flush()
    }
}

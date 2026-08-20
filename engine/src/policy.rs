//! Politiques de décision. Le moteur appelle la politique à chaque point de
//! choix ; TOUT l'aléatoire passe par le RNG de la partie (D11), fourni en
//! paramètre — la politique elle-même ne possède pas de RNG.

use crate::choice::ChoiceContext;
use crate::effects::RevealFilter;
use crate::state::GameState;
use rand::rngs::StdRng;
use rand::Rng;

/// Bonus du sélectionneur de la phase construction (livret p.12, l.336) :
/// « piocher une carte **avant ou après** avoir joué une carte lors de cette
/// phase OU jouer une carte bleue/rouge supplémentaire ».
///
/// Les trois choix du livret (C2 du lot 3 — l'écart E2 était la réduction du
/// bonus « pioche » au seul moment APRÈS) :
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionBonus {
    /// Piocher 1 carte APRÈS avoir joué la carte de la phase (sens historique
    /// du variant, conservé tel quel).
    DrawCard,
    /// Piocher 1 carte AVANT de jouer : la carte piochée peut donc être posée
    /// dans la foulée (l'affordabilité est calculée après la pioche).
    DrawCardBefore,
    /// Jouer une carte bleue/rouge supplémentaire.
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
    // (moteur-questions-manquantes) `SellCard` — « défausser 1 carte de la main
    // pour 3 MC » — N'EST PLUS UNE ACTION de la phase Action, et le variant est
    // retiré plutôt que laissé sans emploi.
    //
    // La phase Action est un aller-retour : chaque option prise y consomme un
    // échange. Vendre coûtait donc un tour de jeu, ne vendait qu'une carte, et
    // faisait doublon avec `flow::occasion_de_vendre`, ouverte avant CHAQUE
    // point de décision des phases dépensables, gratuite, et sans limite de
    // nombre. Le taux est le même des deux côtés (`flow::discard_mc_rate`,
    // service unique) : retirer l'action ne retire aucun MC au joueur, elle ne
    // lui rendait qu'un chemin plus cher vers le même état.
    /// Action de la carte bleue jouée d'indice donné (stub neutre).
    BlueAction(u16),
    /// (jokers-corpos) **Action portée par la CORPORATION du joueur** — une
    /// planche de l'extension Découverte en porte une (« Action : gagnez 1 MC…
    /// »). Elle s'active comme celle d'une carte bleue, une fois par phase III,
    /// et consomme une activation comme elle. Aucun identifiant : un joueur n'a
    /// qu'une corporation.
    CorpAction,
}

/// Points de décision du flux de jeu. Les tests peuvent fournir une politique
/// scriptée ; `simulate` utilise `RandomPolicy`. Les deux passent par le même
/// flux (`setup_game` / `play_round`).
pub trait Policy {
    /// **(moteur-observe) LA VUE DE LA PARTIE, juste avant chaque décision.**
    ///
    /// Appelée par `flow.rs` immédiatement avant CHAQUE appel à l'une des
    /// méthodes de décision ci-dessous, avec l'état de la partie tel qu'il est
    /// **à cet instant précis** — pas l'instantané de début de phase
    /// (`GameState::snap_*`), pas une copie prise plus tôt. `player` est le
    /// joueur à qui la décision qui suit va être demandée.
    ///
    /// **Corps par défaut vide, et c'est le point** : `RandomPolicy`,
    /// `ProbePolicy` et toutes les politiques scriptées des tests l'héritent
    /// sans une ligne de changement, ne consomment pas le RNG de la partie, et
    /// décident donc exactement comme avant ce chantier. C'est ce qui rend les
    /// trois empreintes de référence insensibles au câblage.
    ///
    /// Elle ne rend RIEN : une politique qui observe ne peut pas, par cette
    /// méthode, infléchir le déroulement. Voir `crate::observe::ObservingPolicy`
    /// pour l'usage, et `crate::observe::state_view` pour le rendu JSON de
    /// l'état ainsi reçu.
    fn observe(&mut self, _game: &GameState, _player: usize) {}

    /// **(L5, §2.17.3) LA POLITIQUE A-T-ELLE FINI D'AVOIR BESOIN DE CETTE MANCHE ?**
    ///
    /// `flow::play_round` l'interroge à chaque frontière de phase et avant
    /// l'étape de fin de manche. Une politique qui répond `true` fait sortir la
    /// manche par le haut : le moteur cesse de dérouler des phases dont plus rien
    /// ne sera lu.
    ///
    /// **Corps par défaut `false`, et c'est le point** : `RandomPolicy`,
    /// `ProbePolicy`, `Joueur` et toutes les politiques scriptées des tests
    /// l'héritent sans une ligne de changement et déroulent la manche entière,
    /// exactement comme avant. Les quatre empreintes d'état ne bougent pas.
    ///
    /// Le seul cas où elle vaut `true` est `rejeu::Rejeu` **après** qu'il a posé
    /// son point d'attente : le joueur qui essaie ses coups (`joueur.rs`,
    /// `etat_atteint`) ne lit alors plus que l'état cloné à cette décision-là, et
    /// tout ce que la manche continuait de dérouler était jeté — décisions
    /// répondues par défaut, production encaissée, cartes piochées, rien de quoi
    /// personne n'a plus l'usage.
    fn interrompu(&self) -> bool {
        false
    }

    /// Mulligan corporations (règle maison n°1) : remplacer SES 2 corporations
    /// par 2 nouvelles — les 2 ou aucune. Avant la donne des cartes projets.
    fn corp_mulligan(&mut self, rng: &mut StdRng, player: usize, corps: &[u16]) -> bool;

    /// Mulligan projets (règle maison n°2) : remplacer **entre 0 et 8** de ses
    /// 8 cartes de départ, carte par carte — contrairement au mulligan
    /// corporations qui reste « les 2 ou aucune ».
    ///
    /// Rend les **indices dans `hand`** des cartes à remplacer, comme
    /// `discard_down`. Vide = on garde tout. Les indices hors bornes ou répétés
    /// sont ignorés par le moteur (`flow::setup_game`), qui ne défausse jamais
    /// deux fois la même carte.
    fn project_mulligan(&mut self, rng: &mut StdRng, player: usize, hand: &[u16]) -> Vec<usize>;

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
    ///
    /// **(MOT-3) Ce n'est plus un point de décision du déroulement**, mais le
    /// choix de FOND de la politique — « qu'est-ce que je veux de ce bonus ? ».
    /// Le déroulement, lui, le demande en DEUX TEMPS, par les deux méthodes
    /// ci-dessous. Une politique qui n'a qu'un avis global n'a rien à changer :
    /// les deux temps retombent dessus par leurs corps par défaut.
    fn construction_bonus(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus;

    /// **(MOT-3) PREMIER TEMPS du bonus de Construction : « piocher tout de
    /// suite, avant de poser ? »**
    ///
    /// Livret `docs/regles/livret-base.md:336` : « piocher une carte **avant ou
    /// après** avoir joué une carte lors de cette phase OU jouer une carte
    /// bleue/rouge supplémentaire ». Les trois issues étaient tranchées d'un
    /// coup, avant la moindre pose : le joueur devait choisir « piocher après »
    /// ou « poser une seconde carte » sans savoir ce qu'il pourrait poser — et
    /// le « avant ou après » du livret n'était plus un choix, mais une
    /// formalité.
    ///
    /// Ce premier temps ne pose donc que la question qui DOIT se trancher
    /// avant : la pioche immédiate, celle qui peut encore changer ce qu'on
    /// posera. Vrai = piocher maintenant, le bonus est consommé. Faux = ne rien
    /// décider d'autre pour l'instant ; la vraie question viendra une fois la
    /// carte posée.
    ///
    /// **Aucun réglage de jeu nouveau** : le nombre d'issues du bonus reste
    /// trois, on ne fait que les demander au moment où le joueur peut y
    /// répondre.
    fn construction_bonus_avant(&mut self, rng: &mut StdRng, player: usize) -> bool {
        self.construction_bonus(rng, player) == ConstructionBonus::DrawCardBefore
    }

    /// **(MOT-3) SECOND TEMPS, la carte posée : piocher, ou poser une seconde
    /// carte bleue/rouge ?**
    ///
    /// Demandé seulement aux joueurs qui n'ont pas déjà pioché au premier
    /// temps. C'est ici que le joueur sait enfin ce qu'il a pu poser — et donc
    /// si une seconde pose lui servirait à quelque chose.
    ///
    /// Rendre [`ConstructionBonus::DrawCardBefore`] n'a plus de sens à cet
    /// instant : la pose a eu lieu. Le moteur le lit comme « piocher », le seul
    /// sens qui reste au vœu exprimé.
    fn construction_bonus_apres(&mut self, rng: &mut StdRng, player: usize) -> ConstructionBonus {
        match self.construction_bonus(rng, player) {
            ConstructionBonus::SecondBuild => ConstructionBonus::SecondBuild,
            _ => ConstructionBonus::DrawCard,
        }
    }

    /// Une décision de la phase action : Some(indice dans options) ou None (stop).
    fn action_choice(&mut self, rng: &mut StdRng, player: usize, options: &[ActionOpt])
        -> Option<usize>;

    /// Montant d'une action bleue « spend ANY amount » (lot 2 : Power
    /// Infrastructure, Redrafted Contracts). Tirage uniforme 0..=max via le RNG
    /// de la partie. Méthode par DÉFAUT : les politiques existantes en héritent
    /// (aucune modification de signature).
    ///
    /// (lot 6) Un montant PLAFONNÉ par le texte imprimé (« spend up to N ») ne
    /// passe pas par ici : ses valeurs s'énumèrent, c'est une alternative, donc
    /// `choose_option`.
    fn action_amount(&mut self, rng: &mut StdRng, _player: usize, max: i64) -> i64 {
        if max <= 0 {
            0
        } else {
            rng.gen_range(0..=max)
        }
    }

    /// **Vendre, à tout moment, parce qu'on le veut.**
    ///
    /// Livret l. 96, répété l. 310 : « à tout moment, vous pouvez défausser une
    /// carte Projet de votre main pour gagner 3 MC ». Le moteur pose cette
    /// question — par `flow::occasion_de_vendre` — AVANT chacun de ses points de
    /// décision, aux deux joueurs, et seulement dans les phases où l'on peut
    /// dépenser (I Développement, II Construction, III Action : ni production ni
    /// recherche).
    ///
    /// C'est la question qui a remplacé `discard_payment_count`. Celle-là
    /// demandait COMBIEN de cartes le moteur devait prendre pour compléter un
    /// paiement, et il prenait « les dernières de la main » : le joueur ne
    /// choisissait ni le moment, ni les cartes. Ici il choisit les deux.
    ///
    /// `main` est la main du joueur `joueur` à cet instant. La réponse est une
    /// liste d'indices dans cette main — libre, éventuellement VIDE. Le moteur
    /// la nettoie (bornes, doublons) puis défausse ces cartes-là, au taux du
    /// service unique `flow::discard_mc_rate`.
    ///
    /// Corps par DÉFAUT : **la liste vide**, et pas un tirage. Une politique qui
    /// ne connaît pas cette question ne vend rien, ne consomme pas le RNG de la
    /// partie, et joue donc exactement comme avant : c'est ce qui permet à ce
    /// nouveau point d'occasion d'être posé 34 fois par décision sans déplacer
    /// d'un cran le déroulement des parties simulées.
    fn vendre_librement(
        &mut self,
        _rng: &mut StdRng,
        _joueur: usize,
        _main: &[u16],
    ) -> Vec<usize> {
        Vec::new()
    }

    // ------------------------------------- lot 3 : ressources sur les cartes
    //
    // Les trois décisions du lot 3. Méthodes par DÉFAUT : aucune politique
    // existante n'est modifiée, et tout l'aléatoire passe par le RNG de la
    // partie fourni en paramètre.
    //
    // CONVENTION COMMUNE (journal D4) : un indice `>= n` (resp.
    // `>= candidates.len()`) vaut RENONCEMENT — l'effet concerné est sauté,
    // sans repli sur un autre choix. Les politiques du moteur ne la produisent
    // jamais ; seule la sonde s'en sert pour signaler une cible imposée absente
    // des candidats (`target_error`) au lieu de retomber silencieusement sur
    // une autre carte.

    /// Choisit une branche parmi `n` alternatives (0..n). Les branches sont
    /// numérotées dans l'ordre du TEXTE IMPRIMÉ, après filtrage de celles qui
    /// ne sont pas jouables. Le moteur n'appelle cette méthode que si `n >= 2`
    /// (à une seule branche jouable, il n'y a plus d'alternative — journal D3).
    /// Défaut : tirage uniforme.
    fn choose_option(&mut self, rng: &mut StdRng, _player: usize, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            rng.gen_range(0..n)
        }
    }

    /// **La même question, mais en disant DE QUOI elle parle.**
    ///
    /// `ctx` ([`ChoiceContext`]) porte la nature du choix, la carte concernée
    /// quand le site d'appel la connaît, et ce que désigne chaque option — de
    /// quoi présenter à un joueur humain autre chose qu'un bouton numéroté.
    /// C'est par ici que passent DÉSORMAIS les onze points d'alternative de
    /// `flow.rs` : aucun d'eux ne demande plus « choisis parmi n ».
    ///
    /// **Corps par défaut, et c'est tout le mécanisme de rétrocompatibilité** :
    /// il retombe sur [`Policy::choose_option`] avec
    /// `ctx.option_count()`, qui vaut exactement le `n` d'avant ce chantier.
    /// Une politique qui n'implémente pas cette méthode — `RandomPolicy`,
    /// toutes les politiques scriptées des tests — consomme donc le RNG au même
    /// instant, avec la même borne, et décide à l'identique : les trois
    /// empreintes de référence ne peuvent pas bouger.
    ///
    /// L'ordre des options est celui du contexte, qui est celui du moteur :
    /// une politique ne doit pas le réordonner, l'indice rendu est lu tel quel.
    fn choose_option_ctx(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        ctx: &ChoiceContext,
    ) -> usize {
        self.choose_option(rng, player, ctx.option_count())
    }

    /// Choisit la carte qui REÇOIT la ressource, parmi `candidates`
    /// (identifiants de cartes en jeu, triés par identifiant). Appelée même
    /// avec un seul candidat : c'est le moteur qui demande, jamais lui qui
    /// décide. Défaut : tirage uniforme.
    fn choose_res_target(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        candidates: &[u16],
    ) -> usize {
        if candidates.is_empty() {
            0
        } else {
            rng.gen_range(0..candidates.len())
        }
    }

    /// Choisit la carte sur laquelle RETIRER une ressource (Decomposing
    /// Fungus). Mêmes conventions que `choose_res_target`.
    fn choose_res_source(
        &mut self,
        rng: &mut StdRng,
        _player: usize,
        candidates: &[u16],
    ) -> usize {
        if candidates.is_empty() {
            0
        } else {
            rng.gen_range(0..candidates.len())
        }
    }

    /// **(jokers-corpos) « Choisissez un badge et ajoutez-le à cette carte. »**
    ///
    /// Point de décision au même titre que `pick_phase` ou `pick_corporation` :
    /// le badge d'une carte joker est CHOISI par le joueur, jamais câblé dans le
    /// déroulement — une intelligence artificielle viendra le décider plus tard.
    ///
    /// Renvoie un indice dans [`crate::cards::JOKER_TAG_CHOICES`], les dix
    /// badges du jeu (le joker lui-même n'en fait pas partie). `tag_counts` est
    /// le décompte de badges du joueur À CET INSTANT ; il est indexé par
    /// `Tag::index`, c'est-à-dire dans le MÊME ordre que `JOKER_TAG_CHOICES` —
    /// un indice vaut donc pour les deux.
    ///
    /// **Heuristique par défaut, et sa raison en une phrase** : prendre le badge
    /// que le joueur possède DÉJÀ le plus, parce que c'est celui que ses cartes
    /// en jeu valorisent déjà (réductions de prix par badge, productions et
    /// points de victoire par badge, Objectifs et Récompenses) ; à égalité, le
    /// premier dans l'ordre de l'énumération — déterministe, et sans consommer
    /// le RNG de la partie.
    fn pick_joker_tag(
        &mut self,
        _rng: &mut StdRng,
        _player: usize,
        _card: u16,
        tag_counts: &[u32],
    ) -> usize {
        let mut best = 0usize;
        for i in 1..tag_counts.len() {
            if tag_counts[i] > tag_counts[best] {
                best = i;
            }
        }
        best
    }

    /// **(D5) « Choisissez un badge » — LA QUESTION REPOSÉE AU MOMENT DE LA
    /// POSE.**
    ///
    /// Livret Découverte l. 98-100 : « Si vous jouez (ou défaussez) la carte
    /// plus tard, vous pourrez choisir un badge différent. » Le badge posé
    /// pendant que la carte était en main n'était qu'un badge de travail, servant
    /// à juger de ce que le joueur pouvait se payer ; celui-ci est définitif.
    ///
    /// `candidats` porte les indices, dans [`crate::cards::JOKER_TAG_CHOICES`],
    /// des seuls badges qui laissent la carte PAYABLE — la carte a quitté la
    /// main, un badge moins favorable ne doit pas la rendre impayable après
    /// coup. La réponse est un indice **dans `candidats`**, comme `reveal_pick`
    /// rend des indices dans ses candidates : la forme de la réponse suit la
    /// liste qu'on donne, jamais une liste plus large.
    ///
    /// Le moteur n'appelle cette méthode qu'à partir de DEUX candidats.
    ///
    /// **Le corps par défaut redit l'avis de la politique sans rien inventer** :
    /// il lui demande le badge qu'elle voudrait sans contrainte
    /// ([`Policy::pick_joker_tag`]), le retient s'il est encore permis, et se
    /// rabat sinon sur celui des candidats que le joueur possède déjà le plus —
    /// la même heuristique, restreinte. Une politique qui n'a pas d'avis
    /// particulier sur ce second temps n'a donc rien à écrire.
    fn pick_joker_tag_a_la_pose(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        card: u16,
        tag_counts: &[u32],
        candidats: &[usize],
    ) -> usize {
        let libre = self.pick_joker_tag(rng, player, card, tag_counts);
        if let Some(k) = candidats.iter().position(|&i| i == libre) {
            return k;
        }
        let mut best = 0usize;
        for k in 1..candidats.len() {
            if tag_counts[candidats[k]] > tag_counts[candidats[best]] {
                best = k;
            }
        }
        best
    }

    /// Recherche : garder `keep` cartes parmi `drawn` — renvoie les indices gardés.
    fn research_keep(&mut self, rng: &mut StdRng, player: usize, drawn: &[u16], keep: usize)
        -> Vec<usize>;

    /// **Révélation du dessus de la pioche** (`flow::reveal_top`, brique du lot
    /// 6 : « Révélez les 3 premières cartes… ajoutez-en une bleue ou rouge à
    /// votre main… défaussez les autres »).
    ///
    /// Pourquoi cette méthode existe alors que `research_keep` suffisait à
    /// TRANCHER : parce qu'une révélation n'est pas seulement un choix, c'est
    /// un GESTE PUBLIC. Les trois cartes sont retournées face visible sur la
    /// table ; un joueur humain doit les voir, y compris — surtout — quand
    /// aucune n'est prenable et qu'il n'a rien à choisir. `research_keep` ne
    /// recevait que les candidates, et n'était pas appelée du tout à zéro
    /// candidate : les cartes révélées disparaissaient alors sans que rien ne
    /// paraisse à l'écran. C'est le défaut que cette méthode corrige.
    ///
    /// - `revealed` : les cartes réellement retournées, dans l'ordre où elles
    ///   ont quitté la pioche. **Toutes** sont montrables au joueur : le geste
    ///   est public par les règles du jeu (et elles finiront à la défausse, qui
    ///   l'est aussi).
    /// - `candidates` : la sous-suite de `revealed` que le filtre imprimé rend
    ///   PRENABLE, dans le même ordre.
    /// - `keep` : combien il faut en prendre — **zéro** quand aucune ne l'est.
    /// - `filter` : le filtre IMPRIMÉ, tel quel. C'est lui qui dit *pourquoi*
    ///   une carte révélée n'est pas prenable (« elle est verte », « elle n'a
    ///   ni badge science ni badge plante ») ; une politique qui a un écran doit
    ///   pouvoir l'expliquer sans réinventer la règle.
    ///
    /// Rend des indices **dans `candidates`**, comme `research_keep` : la forme
    /// de la réponse ne change pas, aucune politique existante n'a à être
    /// réécrite, et aucune réponse valide hier ne devient invalide aujourd'hui.
    ///
    /// Le corps par défaut REDIT exactement l'ancien comportement du moteur :
    /// rien à prendre → rien à décider et **pas un tirage** consommé sur le
    /// générateur ; sinon, la question est `research_keep` sur les candidates,
    /// mot pour mot. Les empreintes de parties sont donc inchangées.
    fn reveal_pick(
        &mut self,
        rng: &mut StdRng,
        player: usize,
        _revealed: &[u16],
        candidates: &[u16],
        keep: usize,
        _filter: RevealFilter,
    ) -> Vec<usize> {
        if keep == 0 {
            return Vec::new();
        }
        self.research_keep(rng, player, candidates, keep)
    }

    /// Fin de ronde : défausser `n` cartes (limite de main) — indices à défausser.
    fn discard_down(&mut self, rng: &mut StdRng, player: usize, hand: &[u16], n: usize)
        -> Vec<usize>;

    // (moteur-questions-manquantes) `sell_card` VIVAIT ICI — « quelle carte
    // vendez-vous ? », la seconde moitié de l'action standard `SellCard` que la
    // phase Action offrait. L'action a été retirée (voir `ActionOpt`), et cette
    // question avec elle : la vente passe désormais par `vendre_librement`, qui
    // désigne autant de cartes qu'on veut, à un point d'OCCASION qui ne consomme
    // pas d'échange. Quatre-vingt-une décisions « quelle carte vendre » sur la
    // seule graine 4242 disparaissent ainsi de l'arbre.
}

/// Politique uniforme aléatoire (toutes décisions tirées du RNG de la partie).
pub struct RandomPolicy;

impl Policy for RandomPolicy {
    fn corp_mulligan(&mut self, rng: &mut StdRng, _player: usize, _corps: &[u16]) -> bool {
        rng.gen_bool(0.5)
    }

    /// Chaque carte est remplacée ou non à pile ou face, indépendamment des
    /// autres — le mulligan projets n'est plus « tout ou rien ».
    fn project_mulligan(&mut self, rng: &mut StdRng, _player: usize, hand: &[u16]) -> Vec<usize> {
        (0..hand.len()).filter(|_| rng.gen_bool(0.5)).collect()
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

    /// Les TROIS options du livret sont tirées uniformément : pioche avant,
    /// pioche après, seconde pose.
    fn construction_bonus(&mut self, rng: &mut StdRng, _player: usize) -> ConstructionBonus {
        match rng.gen_range(0..3u8) {
            0 => ConstructionBonus::DrawCardBefore,
            1 => ConstructionBonus::DrawCard,
            _ => ConstructionBonus::SecondBuild,
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

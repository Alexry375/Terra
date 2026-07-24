# data/cards.json — base de cartes du projet Terra

Dérivée de `workspaces/retag-cartes/outputs/cards_v1.json` (audité le
2026-07-24), avec une transformation appliquée par le CTO le 24-07 :
les 17 cartes du pack promo Kickstarter 2021 (identifiées par leur
`notes_retag`) passent en `box: "promo2021"` et `in_deck_v1: false` —
Alexis ne possède pas les corporations de ce pack (certain) ; les 11 projets
sont exclus par défaut en attendant vérification de sa boîte physique
(réglable : il suffit de rebasculer `in_deck_v1` sur ces cartes).
Champ `box` ∈ {base, discovery, promo, promo2021, fan, crysis, none}.

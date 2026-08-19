// **(le-juge-apprend) LE PAQUET, TEL QUE LE MOTEUR LE COMPOSE — table ENGENDRÉE.**
//
// Ne pas modifier à la main : ce fichier est la sortie de
//
//     ./engine/target/release/decrire --table --boites base,decouverte \
//         | python3 outputs/work/engendrer-paquet.py
//
// (2.12) C'est la liste des cartes qui sont RÉELLEMENT DANS LA PIOCHE de la
// composition `base,decouverte` — le drapeau `in_deck` du moteur, et non plus
// l'appartenance à une boîte nommée. Onze cartes portent un nom de boîte sans
// jamais être distribuées : leurs quarante-quatre entrées valaient −1 dans
// toutes les situations de toutes les parties.
//
// **La table dépend donc de la composition.** Une partie jouée avec une autre
// composition n'a pas la même table ni la même fiche ; le verrou du §7 le voit
// et refuse le fichier de poids plutôt que de le mal interpréter.
//
// Le rang d'une carte dans cette liste EST son rang dans le vecteur de
// description : les deux côtés doivent partir de la même table, et non chacun
// de la sienne.
//
// Relevé le 19-08 : 246 cartes projets, 16 corporations.

/** Identifiants des cartes projets, dans l'ordre du vecteur. */
export const PROJETS = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 25, 26, 29,
  30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
  47, 49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 60, 62, 64, 65, 67,
  68, 69, 73, 74, 76, 77, 78, 80, 81, 84, 85, 86, 87, 88, 89, 90,
  91, 92, 93, 94, 95, 96, 97, 98, 100, 101, 103, 104, 105, 106, 108, 109,
  110, 111, 112, 113, 114, 115, 116, 118, 119, 120, 121, 122, 123, 124, 126, 127,
  129, 132, 136, 137, 140, 143, 144, 145, 147, 148, 149, 150, 151, 152, 153, 154,
  156, 157, 158, 159, 160, 161, 163, 164, 168, 169, 170, 171, 172, 173, 175, 176,
  177, 178, 179, 180, 181, 182, 183, 184, 186, 189, 191, 194, 195, 196, 197, 200,
  201, 203, 205, 207, 208, 209, 210, 211, 214, 215, 216, 217, 218, 219, 220, 221,
  222, 223, 224, 225, 228, 229, 230, 231, 233, 235, 236, 237, 240, 242, 243, 244,
  245, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256, 258, 259, 260, 261, 262,
  263, 265, 266, 267, 268, 269, 270, 271, 273, 274, 275, 276, 277, 278, 279, 280,
  281, 283, 284, 285, 286, 287, 288, 289, 292, 293, 294, 296, 300, 302, 303, 304,
  305, 307, 308, 309, 310, 311, 312, 313, 314, 315, 317, 318, 319, 320, 322, 323,
  324, 325, 327, 328, 329, 330,
];

/** Noms des corporations, triés — l'état ne publie que le NOM d'une corporation. */
export const CORPORATIONS = [
  "Apollo Industries",
  "Credicor",
  "Ecoline",
  "Exocorp",
  "Helion Corporation",
  "Hyperion Systems",
  "Interplanetary Cinematics",
  "Inventrix",
  "Mining Guild",
  "Phobolog",
  "Saturn Systems",
  "Sultira",
  "Teractor Corporation",
  "Tharsis Republic",
  "Thorgate Corporation",
  "Unmi",
];

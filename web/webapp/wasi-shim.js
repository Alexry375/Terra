// Shim WASI preview1 minimal — assez pour que le moteur Rust compilé en
// WebAssembly lise `assets/cards.json` et écrive ses messages.
//
// Pourquoi : `CardsDb::load_boites` du moteur lit le fichier de cartes par
// `std::fs::read_to_string`. Le moteur ne bouge pas d'une ligne, donc le wasm
// est compilé pour `wasm32-wasip1` et c'est l'hôte qui fournit un système de
// fichiers. Ce fichier est le MÊME dans le navigateur et dans Node : une seule
// implémentation, aucun drapeau expérimental, aucune dépendance.
//
// Les 12 fonctions implémentées sont exactement celles que le wasm importe
// (vérifiable : WebAssembly.Module.imports).

const ESUCCESS = 0;
const EBADF = 8;
const EINVAL = 28;
const ENOENT = 44;
const ENOTDIR = 54;

const FILETYPE_DIRECTORY = 3;
const FILETYPE_REGULAR_FILE = 4;
const FILETYPE_CHARACTER_DEVICE = 2;

const FD_STDIN = 0;
const FD_STDOUT = 1;
const FD_STDERR = 2;
const FD_PREOPEN = 3;

// Nom du répertoire pré-ouvert : les chemins relatifs du wasm s'y résolvent.
const PREOPEN = ".";

/**
 * @param {object} o
 * @param {Record<string, Uint8Array>} o.fichiers  table « chemin -> octets »
 * @param {(flux: string, texte: string) => void} [o.ecrire]  stdout/stderr
 */
export function creerWasi({ fichiers, ecrire }) {
  const journal = ecrire || ((flux, texte) => {
    const s = texte.replace(/\n$/, "");
    if (s) (flux === "stderr" ? console.error : console.log)(s);
  });

  let memoire = null;
  const ouverts = new Map(); // fd -> { octets, position }
  let prochainFd = 4;
  const tampons = { stdout: "", stderr: "" };

  const vue = () => new DataView(memoire.buffer);
  const octets = () => new Uint8Array(memoire.buffer);

  function lireChaine(ptr, len) {
    return new TextDecoder().decode(octets().subarray(ptr, ptr + len));
  }

  // Normalise un chemin du wasm vers une clef de la table `fichiers`.
  function normaliser(p) {
    let q = p.replace(/\\/g, "/");
    while (q.startsWith("./")) q = q.slice(2);
    if (q.startsWith("/")) q = q.slice(1);
    return q;
  }

  function vider(flux) {
    const t = tampons[flux];
    if (!t) return;
    const morceaux = t.split("\n");
    tampons[flux] = morceaux.pop();
    for (const l of morceaux) journal(flux, l);
  }

  function ecrireIovs(flux, iovs, iovsLen, nptr) {
    const dv = vue();
    let total = 0;
    let texte = "";
    for (let i = 0; i < iovsLen; i++) {
      const base = dv.getUint32(iovs + i * 8, true);
      const len = dv.getUint32(iovs + i * 8 + 4, true);
      texte += new TextDecoder().decode(octets().subarray(base, base + len));
      total += len;
    }
    tampons[flux] += texte;
    vider(flux);
    dv.setUint32(nptr, total, true);
    return ESUCCESS;
  }

  const imports = {
    wasi_snapshot_preview1: {
      environ_sizes_get(nptr, tailleptr) {
        const dv = vue();
        dv.setUint32(nptr, 0, true);
        dv.setUint32(tailleptr, 0, true);
        return ESUCCESS;
      },
      environ_get() {
        return ESUCCESS;
      },
      clock_time_get(_id, _precision, tempsptr) {
        // Nanosecondes. Sert uniquement à `games_per_sec`, que le moteur envoie
        // sur stderr parce qu'il n'est pas déterministe (il ne figure pas dans
        // la ligne de bilan comparée au binaire natif).
        const ns = BigInt(Math.round(Date.now() * 1e6));
        vue().setBigUint64(tempsptr, ns, true);
        return ESUCCESS;
      },
      fd_prestat_get(fd, ptr) {
        if (fd !== FD_PREOPEN) return EBADF;
        const dv = vue();
        dv.setUint8(ptr, 0); // preopentype = dir
        dv.setUint32(ptr + 4, PREOPEN.length, true);
        return ESUCCESS;
      },
      fd_prestat_dir_name(fd, ptr, len) {
        if (fd !== FD_PREOPEN) return EBADF;
        const b = new TextEncoder().encode(PREOPEN);
        if (len < b.length) return EINVAL;
        octets().set(b, ptr);
        return ESUCCESS;
      },
      path_open(dirfd, _dirflags, pathPtr, pathLen, _oflags, _base, _inh, _fdflags, fdOut) {
        if (dirfd !== FD_PREOPEN) return EBADF;
        const chemin = normaliser(lireChaine(pathPtr, pathLen));
        const contenu = fichiers[chemin];
        if (!contenu) return ENOENT;
        const fd = prochainFd++;
        ouverts.set(fd, { octets: contenu, position: 0 });
        vue().setUint32(fdOut, fd, true);
        return ESUCCESS;
      },
      fd_fdstat_get(fd, ptr) {
        const dv = vue();
        let type;
        if (fd === FD_PREOPEN) type = FILETYPE_DIRECTORY;
        else if (fd <= FD_STDERR) type = FILETYPE_CHARACTER_DEVICE;
        else if (ouverts.has(fd)) type = FILETYPE_REGULAR_FILE;
        else return EBADF;
        dv.setUint8(ptr, type);
        dv.setUint16(ptr + 2, 0, true);
        // Droits : tout permis (le shim ne fait pas de contrôle d'accès).
        dv.setBigUint64(ptr + 8, 0xffffffffffffffffn, true);
        dv.setBigUint64(ptr + 16, 0xffffffffffffffffn, true);
        return ESUCCESS;
      },
      fd_filestat_get(fd, ptr) {
        const f = ouverts.get(fd);
        const dv = vue();
        if (fd === FD_PREOPEN) {
          for (let i = 0; i < 64; i++) dv.setUint8(ptr + i, 0);
          dv.setUint8(ptr + 16, FILETYPE_DIRECTORY);
          return ESUCCESS;
        }
        if (!f) return EBADF;
        for (let i = 0; i < 64; i++) dv.setUint8(ptr + i, 0);
        dv.setBigUint64(ptr + 8, BigInt(fd), true); // ino
        dv.setUint8(ptr + 16, FILETYPE_REGULAR_FILE);
        dv.setBigUint64(ptr + 24, 1n, true); // nlink
        dv.setBigUint64(ptr + 32, BigInt(f.octets.length), true); // taille
        return ESUCCESS;
      },
      fd_read(fd, iovs, iovsLen, nptr) {
        const f = ouverts.get(fd);
        const dv = vue();
        if (!f) {
          if (fd === FD_STDIN) {
            dv.setUint32(nptr, 0, true);
            return ESUCCESS;
          }
          return EBADF;
        }
        let lus = 0;
        const dest = octets();
        for (let i = 0; i < iovsLen; i++) {
          const base = dv.getUint32(iovs + i * 8, true);
          const len = dv.getUint32(iovs + i * 8 + 4, true);
          const reste = f.octets.length - f.position;
          if (reste <= 0) break;
          const n = Math.min(len, reste);
          dest.set(f.octets.subarray(f.position, f.position + n), base);
          f.position += n;
          lus += n;
          if (n < len) break;
        }
        dv.setUint32(nptr, lus, true);
        return ESUCCESS;
      },
      fd_write(fd, iovs, iovsLen, nptr) {
        if (fd === FD_STDOUT) return ecrireIovs("stdout", iovs, iovsLen, nptr);
        if (fd === FD_STDERR) return ecrireIovs("stderr", iovs, iovsLen, nptr);
        return EBADF;
      },
      fd_close(fd) {
        if (fd === FD_PREOPEN || fd <= FD_STDERR) return ESUCCESS;
        if (!ouverts.delete(fd)) return EBADF;
        return ESUCCESS;
      },
      fd_seek() {
        return ENOTDIR; // jamais importé par ce wasm ; refus explicite
      },
      proc_exit(code) {
        vider("stdout");
        vider("stderr");
        throw new Error("le wasm a appele proc_exit(" + code + ")");
      },
    },
  };

  return {
    imports,
    /** À appeler juste après l'instanciation. */
    lier(instance) {
      memoire = instance.exports.memory;
      if (typeof instance.exports._initialize === "function") {
        instance.exports._initialize();
      }
    },
  };
}

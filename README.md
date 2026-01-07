# FAT32 Filesystem Implementation

## PROJET 1: FAT32 Reimplementation (Cours Rust 4A)

---

## Fonctionnalités requises

| Commande | Description | Status |
|----------|-------------|--------|
| `ls` | Lister les fichiers selon un chemin donné | OK |
| `cat/more` | Lire les fichiers (chemin absolu ou relatif) | OK |
| `cd` | Naviguer dans les répertoires | OK |
| `pwd` | Afficher le répertoire courant | OK |
| Créer/écrire | Créer et écrire des fichiers (second temps) | En cours |
| Interface CLI | Interface en ligne de commande | OK |

---

## Contraintes

- `no_std` obligatoire
- `alloc` crate autorisé
- Correction sur Linux
- Soumission via **git bundle** sur myges
- **Commits réguliers obligatoires** (peu ou pas de commits = 0)
- Code emprunté non crédité = 0
- Tests obligatoires, miri/mirai/kani/fuzzers = bonus
- Code unsafe documenté avec rustdoc + commentaire safety

---

## Structure du projet

```
fat32-exam/
├── Cargo.toml
├── .cargo/
│   └── config.toml          # Configuration no_std
├── src/
│   ├── lib.rs               # Point d'entrée (no_std)
│   ├── allocator.rs         # Bump allocator pour no_std
│   ├── fat32/
│   │   ├── mod.rs           # Interface FAT32 principale
│   │   ├── boot_sector.rs   # Parsing du boot sector
│   │   ├── fat.rs           # Table FAT et chaînes de clusters
│   │   └── directory.rs     # Entrées de répertoire (8.3 + LFN)
│   └── shell/
│       ├── mod.rs           # Module shell
│       ├── commands.rs      # Implémentation ls, cd, cat, more
│       └── parser.rs        # Parsing des commandes
└── tests/
    └── fat32_tests.rs       # Tests d'intégration
```

---

## Build & Test

```bash
# Installer nightly (requis pour no_std)
rustup install nightly
rustup override set nightly

# Ajouter la target
rustup target add x86_64-unknown-none
rustup component add rust-src

# Lancer les tests
cargo test

# Build no_std (décommenter config dans .cargo/config.toml)
cargo build --target x86_64-unknown-none
```

---


## Ressources

- [Writing an OS in Rust](https://os.phil-opp.com/) (lire jusqu'à memory allocator)
- [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/)
- [Microsoft FAT32 File System Specification](https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc)

---

## Auteur

**Noella IKIREZI** - ESGI 4A

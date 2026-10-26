# Trimbox

Trimbox lit des documents PDF et prépare le travail autour de PDF/X-4. Le noyau est séparé de l'interface, et l'interface Slint est facultative derrière le feature `gui`.

## Artefacts

- Linux : `trimbox`
- Windows : `trimbox.exe`

## Architecture

- `trimbox-core` : logique métier et erreurs typées, sans dépendance Slint.
- `trimbox-cli` : point d'entrée et interface dépendante de `trimbox-core`.
- `trimbox-cli/ui/main_window.slint` : vue traitée par `slint-build` dans `build.rs`.
- Le fichier `.slint` est racine du traité, la logique reste côté Rust.
- `slint::set_xdg_app_id` est appelé après création de la fenêtre.

## Compilation

Compilation par défaut (ligne de commande uniquement) :

```sh
cargo build -p trimbox-cli
```

Compilation avec l'interface graphique :

```sh
cargo build -p trimbox-cli --features gui
```

Compilation complète de la workspace :

```sh
cargo build --workspace --all-features
```

## Installation par Cargo

```sh
cargo install --locked --git https://github.com/Franck-Demongin/trimbox.git --branch master --features gui
```

Cargo installe `trimbox` sur Linux et `trimbox.exe` sur Windows. Sur cette machine, `~/.cargo/bin` n'est pas forcément dans `PATH`.

## Vérification

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p trimbox-cli
cargo check -p trimbox-cli --features gui
```

## Dépendances système pour la carte Linux

Pour Slint 1.18.1 et le backend choisi, winit avec le rendu `femtovg`, installer les paquets vérifiés localement :

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config \
  libfontconfig-dev libfreetype-dev libx11-dev \
  libxkbcommon-dev libwayland-dev libgl-dev
```

## Application Linux

- Identifiant d'application : `io.github.Franck-Demongin.Trimbox`
- Bureau : `packaging/linux/trimbox.desktop`
- La sélection de l'identifiant est faite après la création du composant, avant l'affichage.

## Arborescence

```text
Cargo.toml
crates/core
crates/cli
packaging/linux/trimbox.desktop
README.md
QWEN.md
```

## Capture d'interface

Aucune capture n'est actuellement versionnée. Ajoutez les captures disponibles dans les sections correspondantes sans masquer les dépendances ci-dessus.

## Licence

GPL-3.0-or-later. La licence de Slint open source impose une licence compatible pour une distribution open source. Les licences commerciales de Slint sont nécessaires pour une distribution propriétaire.
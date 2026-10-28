# Trimbox

Trimbox lit des documents PDF et prépare le travail autour de PDF/X-4. Le noyau est séparé de l'interface, et l'interface Slint est facultative derrière le feature `gui`.

## Artefacts

- Linux : `trimbox` et `trimbox-gui`
- Windows : `trimbox.exe` et `trimbox-gui.exe`

## Compilation

Compilation par défaut (ligne de commande uniquement) :

```sh
cargo build -p trimbox-cli
```

Compilation de l'interface graphique :

```sh
cargo build -p trimbox-cli --features gui --bin trimbox-gui
```

Compilation complète de la workspace :

```sh
cargo build --workspace --all-features
```

## Installation depuis GitHub

```sh
cargo install --locked --git https://github.com/Franck-Demongin/trimbox.git --branch master --features gui
```

Cargo installe `trimbox` et `trimbox-gui` sur Linux, `trimbox.exe` et `trimbox-gui.exe` sur Windows. Sur cette machine, `~/.cargo/bin` n'est pas forcément dans `PATH`.

## Vérification locale

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p trimbox-cli
cargo check -p trimbox-cli --features gui
```

## Dépendances système sur Linux

Pour Slint 1.18.1 et le backend winit avec le rendu `femtovg`, les dépendances suivantes ont été vérifiées en compilation locale sur Ubuntu 26.04 :

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config \
  libfontconfig-dev libfreetype-dev libx11-dev \
  libxkbcommon-dev libwayland-dev libgl-dev
```

## Icône

La source se trouve dans `assets/icons/trimbox.svg`.  
Le script `assets/icons/generate.sh` sert de référence pour régénérer les formats PNG et ICO.

La commande `slint::set_xdg_app_id` organise l'application Linux.

## Application Linux

- Identifiant d'application : `io.github.Franck-Demongin.Trimbox`
- Fichier de bureau : `packaging/linux/io.github.Franck-Demongin.Trimbox.desktop`
- Commande graphique : `trimbox-gui`
- `slint::set_xdg_app_id` est appelé après la création du composant, avant son affichage.

## Capture d'interface

Aucune capture d'écran n'est actuellement versionnée. Ajoutez les captures disponibles dans les sections correspondantes, sans masquer les informations techniques ci-dessus.

## Licence

GPL-3.0-or-later. La licence de Slint open source impose une licence compatible pour une distribution open source. Une distribution propriétaire nécessite une licence commerciale Slint.
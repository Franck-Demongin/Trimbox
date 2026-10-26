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

## Documentation Slint (via slint-docs)

- Les modifications de l'interface graphique sont réalisées avec l'agent `slint-ui` et le serveur MCP `slint-docs`.
- Avant toute modification d'un fichier `.slint`, utiliser le serveur MCP `slint-docs` :
  1. `slint_docs_search` avec un mot-clé précis ;
  2. `slint_reference_describe` avant d'utiliser ou de modifier un widget, élément ou type ;
  3. `slint_docs_get_page` pour les détails de syntaxe ou les exemples ;
  4. `slint_docs_list` si le terme exact est inconnu.
- Serveur : `/home/franck/Dev/MCP/slint_docs_mcp`, lancé avec `uv --directory ... python -m slint_docs_mcp --transport stdio`.
- Version Slint documentée : celle de `Cargo.lock` (1.18.1), donc `SLINT_DOCS_VERSION=1`. Ne pas intégrer d'API absente de cette version.
- Vérifier `page_url` en croisant avec la documentation publiée.
- Conserver les règles Slint existantes et exécuter les tests après modification.

## Licence

GPL-3.0-or-later. La licence de Slint open source impose une licence compatible pour une distribution open source. Les licences commerciales de Slint sont nécessaires pour une distribution propriétaire.
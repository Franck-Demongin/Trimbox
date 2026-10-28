# Référence : projet Rust avec interface Slint (type Trimbox)

Ce document est un guide d'amorçage et de maintenance pour un projet Rust dont
l'interface graphique est basée sur Slint, selon le modèle Trimbox : noyau métier
indépendant, interface optionnelle, binaires séparés, toasts réutilisables,
packaging Linux et Windows, CI et release GitHub Actions.

Il complète `doc/toast-integration.md`, qui reste la source de vérité pour
l'intégration et le portage du système de toasts.

## 0. Résumé pour démarrer vite

Pour un nouveau projet de ce type, la chaîne minimale est :

1. `cargo init` dans le répertoire courant (jamais `cargo new`).
2. un workspace avec au moins deux crates : `core` (logique métier) et `cli`
   (binaire et interface).
3. dans `cli`, une feature `gui` qui active Slint, le build Slint et les
   dépendances GTK/natives optionnelles.
4. un `build.rs` qui compile les fichiers `.slint` uniquement si `gui` est activé
   et, sous Windows, intègre l'icône `.ico` avec `winresource`.
5. un point d'entrée GUI fin (`gui_main.rs`) qui appelle simplement
   `crate::gui::run()`.
6. deux workflows GitHub Actions : `ci.yml` et `release.yml`.

## 1. Structure de workspace

Arborescence minimale recommandée :

```text
projet/
├── Cargo.toml                              # workspace
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── ...
│   └── cli/
│       ├── Cargo.toml
│       ├── build.rs
│       └── src/
│           ├── main.rs                     # binaire CLI
│           ├── gui_main.rs                 # binaire GUI, required-features = ["gui"]
│           ├── lib.rs                      # run(), exporté pour tests
│           ├── cli.rs
│           ├── run.rs
│           └── gui/
│               ├── mod.rs                  # include_modules!(), run()
│               └── toast/
│                   ├── mod.rs
│                   └── queue.rs
├── crates/cli/ui/
│   ├── main_window.slint
│   └── toast/
│       └── toast.slint
├── assets/icons/
│   ├── app.svg
│   ├── app-16.png ... app-512.png
│   ├── app.ico
│   └── ui/
│       └── close.svg
├── packaging/linux/
│   ├── io.github.<auteur>.<App>.desktop
│   ├── install-local.sh
│   └── uninstall-local.sh
├── doc/
│   ├── toast-integration.md
│   ├── gui-reference.md
│   └── reference-projet-rust-slint.md
├── .github/workflows/
│   ├── ci.yml
│   └── release.yml
└── .github/dependabot.yml
```

Points d'organisation :

- la logique métier ne dépend jamais de Slint ;
- le crate `core` contient les types d'erreur typés (`thiserror`) ;
- le crate `cli` dépend de `core` et expose l'interface ;
- la feature `gui` est optionnelle : `default = []` ;
- les binaires sont déclarés explicitement dans `cli/Cargo.toml`.

## 2. Manifestes Cargo

### 2.1 Racine du workspace

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.99"
license = "GPL-3.0-or-later"
authors = ["<auteur>"]
description = "<description>"
repository = "https://github.com/<owner>/<repo>"
homepage = "https://github.com/<owner>/<repo>"
readme = "README.md"

[workspace.dependencies]
<proj>-core = { path = "crates/core", version = "0.1.0" }
thiserror = "2.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
clap = { version = "4.6", features = ["derive"] }

slint = { version = "1.18", default-features = false, features = [
    "std",
    "backend-winit",
    "renderer-femtovg",
    "renderer-software",
    "accessibility",
    "compat-1-2",
] }
slint-build = { version = "1.18", default-features = false, features = [
    "compat-1-18",
    "renderer-software",
] }

[workspace.lints.rust]
unused = "warn"
unsafe_code = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"
```

### 2.2 Crate core

```toml
[package]
name = "<proj>-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
description = "Coeur metier, sans dépendance Slint."
repository.workspace = true

[dependencies]
thiserror.workspace = true
```

Le noyau ne doit pas dépendre de Slint, de la GUI ni du système de
fichiers quand ce n'est pas nécessaire. Il expose des types et des fonctions
pures, testables sans interface.

### 2.3 Crate cli

```toml
[package]
name = "<proj>-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
description = "CLI et interface graphique optionnelle pour <Proj>."
repository.workspace = true

[[bin]]
name = "<proj>"
path = "src/main.rs"

[[bin]]
name = "<proj>-gui"
path = "src/gui_main.rs"
required-features = ["gui"]

[features]
default = []
gui = ["dep:slint", "dep:slint-build", "dep:rfd"]

[dependencies]
<proj>-core.workspace = true
clap.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
slint = { workspace = true, optional = true }
rfd = { version = "0.15", default-features = false, features = ["xdg-portal", "tokio"], optional = true }

[build-dependencies]
slint-build = { workspace = true, optional = true }

[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
```

Choix à retenir :

- `gui` est facultatif, pour permettre une compilation purement CLI ;
- `rfd` n'est ajouté qu'avec la GUI ;
- `lopdf`, `serde` ou d'autres dépendances lourdes ne sont activées qu'avec
  `gui` quand elles ne servent qu'à l'interface, comme Trimbox le fait ;
- le binaire GUI utilise `required-features = ["gui"]`.

### 2.4 Points d'entrée

`crates/cli/src/main.rs` :

```rust
fn main() -> anyhow::Result<()> {
    <proj>_cli::run()
}
```

`crates/cli/src/gui_main.rs` :

```rust
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    <proj>_cli::gui::run()
}
```

`crates/cli/src/lib.rs` :

```rust
pub fn run() -> anyhow::Result<()> {
    // Implémentation CLI.
    Ok(())
}
```

`crates/cli/src/gui.rs` reste derrière `#![cfg(feature = "gui")]`.

### 2.5 Build script minimal

```rust
fn main() {
    #[cfg(all(feature = "gui", target_os = "windows"))]
    {
        let mut res = winresource::WindowsResource::new();
        let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/icons/<app>.ico");
        res.set_icon(icon.to_string_lossy().as_ref());
        res.set("FileDescription", "<description>");
        res.set("ProductName", "<Proj>");
        res.set("OriginalFilename", "<proj>-gui.exe");
        res.compile().expect("échec de la compilation des ressources Windows");
    }

    #[cfg(feature = "gui")]
    {
        if let Err(error) = slint_build::compile("ui/main_window.slint") {
            panic!("compilation Slint impossible : {error}");
        }
    }
}
```

Règles :

- les ressources Windows ne sont compilées que pour le binaire GUI ;
- le fichier `.slint` racine est compilé une seule fois ;
- les imports Slint sont résolus relativement au fichier importateur ;
- aucune logique métier ne doit être mise dans `build.rs`.

### 2.6 Fichiers ignorés (proposition)

```gitignore
/target
/.qwen
/data
/output
```

Trimbox ignore aussi `/data` et `/output`, qui servent aux essais locaux et ne
doivent pas être versionnés. Sur ce point, adapter selon les besoins du projet.

## 3. Architecture et séparation des responsabilités

Règles projetées depuis Trimbox :

- `core` : parsing, validation, calculs, rapports ; types d'erreur avec
  `thiserror`, pas de dialogue, pas de toast, pas de chemins UI.
- `cli` : parsing des arguments (clap), orchestration, journalisation (`tracing`),
  appels vers `core`.
- `cli::gui` : création de fenêtre, branchement des callbacks Slint sur les
  fonctions métier, mise à jour des propriétés Slint.
- les fenêtres Slint exposent des propriétés et des callbacks ; la logique
  reste côté Rust.
- l'état applicatif mutable reste en Rust, pas dans les composants Slint.
- les erreurs utilisateur sont transformées en messages lisibles, idéalement via
  le système de toast.
- les tests unitaires visent surtout `core` et les fonctions pures du crate `cli`.

Convention d'import Slint :

```rust
#![cfg(feature = "gui")]

mod toast;
use toast::{ToastLevel, Toaster};

use slint::ComponentHandle;

slint::include_modules!();

pub fn run() -> anyhow::Result<()> {
    let ui = MainWindow::new()?;
    let _ = slint::set_xdg_app_id("io.github.<auteur>.<App>");
    let toaster = Toaster::install(&ui);

    ui.on_browse_source(move || {
        // ...
        toaster.show("Source chargée.", ToastLevel::Success);
    });

    ui.run()?;
    Ok(())
}
```

`slint::set_xdg_app_id` est appelé après la création de la fenêtre, avec
gestion explicite de l'erreur (`let _ = ...` ou traitement). L'identifiant doit
être identique partout sous Linux (fenêtre, `.desktop`, icône).

## 4. Conventions Slint et organisation UI

Règles minimales à conserver :

- un fichier `.slint` par vue ou composant ;
- un global `Theme` centralise les espacements, rayons, tailles et hauteur minimale ;
- les couleurs standard viennent de `Palette`, sauf les états sémantiques
  (succès, avertissement, erreur) ;
- surface publique du composant racine (propriétés, callbacks) minimale et documentée ;
- logique côté Rust ;
- callback sur la fenêtre principale, l'UI ne fait que déclencher.

Extrait de `ui/main_window.slint` :

```slint
global Theme {
    out property <length> spacing: 8px;
    out property <length> window-preferred-width: 860px;
    out property <length> window-preferred-height: 520px;
    out property <length> window-min-width: 760px;
    out property <length> window-min-height: 460px;
    out property <length> corner-radius: 6px;
    out property <length> font-small: 12px;
}

export component MainWindow inherits Window {
    title: root.window-title;
    preferred-width: Theme.window-preferred-width;
    preferred-height: Theme.window-preferred-height;
    min-width: Theme.window-min-width;
    min-height: Theme.window-min-height;

    in property <string> window-title: "<Proj>";
    callback browse-source();
    callback ...();

    // Toasts au-dessus du contenu.
    ToastOverlay {
        available-width: root.width;
        spacing: Theme.spacing;
    }
}
```

La fenêtre minimale et la preferred window sont déclarées avec des tokens, afin
de pouvoir les réutiliser dans un dialogue.

## 5. Notifications toast

Le système de toast est extrait dans `doc/toast-integration.md`. Ce document de
référence ne redécrit pas son implémentation en détail.

Résumé d'usage :

- copier `ui/toast/toast.slint` et le module Rust `gui/toast` (`mod.rs`,
  `queue.rs`) dans le nouveau projet ;
- ajouter l'overlay dans le composant hôte :
  `ToastOverlay { available-width: root.width; spacing: Theme.spacing; }`
- réexporter les globals Slint nécessaires :
  `export { ToastTiming, ToastState } from "toast/toast.slint";`
- déclarer `mod toast;` et appeler `Toaster::install(&ui)` ;
- appeler `toaster.show("message", ToastLevel::Success)` depuis du code thread UI.

Comportement à conserver :

- empilement du plus ancien au plus récent ;
- au plus trois toasts actifs à la fois ;
- éviction du plus ancien quand un quatrième est poussé ;
- fermeture automatique après quatre secondes ;
- transition de sortie de 200 ms ;
- identification par ID, jamais par position.

Voir `doc/toast-integration.md` pour le contrat complet, le bootstrap, les
limitations et la section `Sources in Trimbox`.

## 6. Agent `slint-ui`

### 6.1 Rôle

L'agent `slint-ui` est le sous-agent dédié aux modifications d'interface Slint.
Il consulte la documentation Slint avant toute modification de fichier `.slint`,
limite les dégâts aux fichiers concernés et maintient cohérentes les règles
du projet (version Slint de `Cargo.lock`, palette, tokens, logique côté Rust).

### 6.2 Quand l'appeler

Appeler `slint-ui` dès qu'une modification touche :

- un fichier `.slint` (`main_window.slint`, `toast.slint`, nouveaux composants) ;
- les macros et le glue Rust Slint (`include_modules!`, globals, callbacks)
  quand la propriété ou le callback ajouté doit être vérifié dans la doc ;
- un widget, élément ou type Slint dont l'API n'est pas déjà confirmée.

Ne pas l'appeler pour :

- la logique métier de `core` ;
- les workflows CI/CD et le packaging ;
- l'édition de tests Rust qui ne touchent pas Slint.

### 6.3 Comment l'appeler

Depuis l'assistant Qwen Code, utiliser l'outil `agent` avec le type
`slint-ui` :

```text
Agent(subagent_type="slint-ui", description="...", prompt="...")
```

Le prompt doit indiquer :

- le fichier ou le composant à modifier ;
- ce qui doit être ajouté ou corrigé ;
- les contraintes projet (version Slint, tokens, palette) ;
- la commande de vérification attendue (`cargo fmt --check`, `cargo check --features gui`, tests).

L'agent a accès à :

- `read_file`, `edit`
- MCP `slint-docs` et `slint-docs-material` sous leurs noms qualifiés
- la règle conditionnelle `.qwen/rules/ui-slint.md` pour les fichiers `**/*.slint`.

Les modifications `.slint` imposent toujours, avant toute édition :

1. `mcp__slint-docs__slint_docs_search` (ou `list` si le terme exact est inconnu) ;
2. `mcp__slint-docs__slint_reference_describe` ;
3. `mcp__slint-docs__slint_docs_get_page`.

Pour les composants Material, remplacer `slint-docs` par `slint-docs-material`.
Les noms d'outils doivent rester préfixés (`mcp__slint-docs__...`,
`mcp__slint-docs-material__...`).

Après modification, toujours exécuter les vérifications Rust et Slint et, si
possible, recompiler la GUI.

## 7. Serveurs MCP pour la documentation Slint

### 7.1 Rôles

- `slint-docs` : documentation Slint principale, indexée depuis
  `docs/astro/src/content/docs`, base publique `https://slint.dev/`.
- `slint-docs-material` : composants Material Slint, même helper, source
  `ui-libraries/material/docs/src/content/docs`, base publique
  `https://material.slint.dev`.

Paramètres essentiels :

| Variable | slint-docs | slint-docs-material |
| --- | --- | --- |
| `SLINT_DOCS_VERSION` | `1` | `1` |
| `SLINT_DOCS_SOURCE_PATH` | `docs/astro/src/content/docs` | `ui-libraries/material/docs/src/content/docs` |
| `SLINT_DOCS_INDEX_PATH` | `...` | `.local/slint-docs-material-index.sqlite3` |
| `SLINT_DOCS_CACHE_PATH` | `...` | `.local/cache` |
| `SLINT_DOCS_PUBLIC_BASE_URL` | `https://slint.dev` | `https://material.slint.dev` |

`SLINT_DOCS_VERSION=1` correspond à Slint 1.x. Il faut toujours passer
explicitement la version résolue dans `Cargo.lock` à la documentation, plutôt que
de prendre la dernière version par défaut.

### 7.2 Déclaration dans Qwen Code

Dans `.qwen/settings.json`, déclarer les deux serveurs comme trusted et ajouter
leurs quatre outils qualifiés à `permissions.allow` :

```json
{
  "permissions": {
    "allow": [
      "mcp__slint-docs__slint_docs_search",
      "mcp__slint-docs__slint_docs_get_page",
      "mcp__slint-docs__slint_docs_list",
      "mcp__slint-docs__slint_reference_describe",
      "mcp__slint-docs-material__slint_docs_search",
      "mcp__slint-docs-material__slint_docs_get_page",
      "mcp__slint-docs-material__slint_docs_list",
      "mcp__slint-docs-material__slint_reference_describe"
    ]
  },
  "mcpServers": {
    "slint-docs": {
      "command": "uv",
      "args": ["run", "--directory", "/home/<USER>/Dev/MCP/slint_docs_mcp", "python", "-m", "slint_docs_mcp", "--transport", "stdio"],
      "env": { "SLINT_DOCS_VERSION": "1" },
      "trust": true
    },
    "slint-docs-material": {
      "command": "uv",
      "args": ["run", "--directory", "/home/<USER>/Dev/MCP/slint_docs_mcp", "python", "-m", "slint_docs_mcp", "--transport", "stdio"],
      "env": {
        "SLINT_DOCS_VERSION": "1",
        "SLINT_DOCS_SOURCE_PATH": "ui-libraries/material/docs/src/content/docs",
        "SLINT_DOCS_PUBLIC_BASE_URL": "https://material.slint.dev",
        "SLINT_DOCS_INDEX_PATH": ".local/slint-docs-material-index.sqlite3",
        "SLINT_DOCS_CACHE_PATH": ".local/cache"
      },
      "trust": true
    }
  }
}
```

Les serveurs doivent ensuite être approuvés avec `qwen mcp approve`. Comme
`.qwen/` est typiquement ignoré par Git, cette configuration reste locale et doit
être recopiée manuellement sur chaque machine. L'agent `slint-ui` doit lui aussi
déclarer ces outils dans son fichier `.qwen/agents/slint-ui.md`.

## 8. Icônes du projet

### 8.1 Fichiers attendus

Source unique : `assets/icons/<app>.svg`.

PNG générés pour les tailles suivantes : `16`, `32`, `48`, `64`, `128`, `256`,
`512`. Fichier ICO Windows multi-tailles : `<app>.ico`.

Icônes d'interface (SVG) sous `assets/icons/ui/` : au moins `close.svg`, plus
les icônes spécifiques de l'application si nécessaire. Les assets sont référencés
depuis Slint avec `@image-url("...")`.

### 8.2 Génération

Script de référence (`assets/icons/generate.sh`) :

```sh
#!/usr/bin/env bash
set -euo pipefail

SOURCE="assets/icons/<app>.svg"
OUTPUT_DIR="assets/icons"

mkdir -p "$OUTPUT_DIR"
for SIZE in 16 32 48 64 128 256 512; do
  rsvg-convert -w "$SIZE" -h "$SIZE" "$SOURCE" -o "$OUTPUT_DIR/<app>-${SIZE}.png"
done

# ICO Windows à partir des PNG existants
magick "$OUTPUT_DIR/<app>-256.png" \
  \( "$OUTPUT_DIR/<app>-48.png" \) \
  \( "$OUTPUT_DIR/<app>-32.png" \) \
  \( "$OUTPUT_DIR/<app>-16.png" \) \
  "$OUTPUT_DIR/<app>.ico"
```

Outils possibles : `rsvg-convert` (librsvg), Inkscape ou ImageMagick comme
solution de repli. Les PNG et l'ICO committés rendent les builds indépendants de
ces outils ; le script est une référence de régénération, pas une étape de build.

### 8.3 Icônes système à récupérer

Pour les icônes d'interface déjà fournies par le système, récupérer les SVG
symboliques depuis les thèmes installés, par exemple :

- `/usr/share/icons/Adwaita/symbolic/actions/`
- `/usr/share/icons/Adwaita/symbolic/ui/`
- `/usr/share/icons/Adwaita/scalable/status/`
- `/usr/share/icons/HighContrast/scalable/actions/`

Bonnes pratiques :

- ne jamais dépendre d'un chemin `/usr/share/icons/...` à l'exécution ;
- copier les SVG choisis dans `assets/icons/ui/` et les commiter ;
- vérifier la licence du thème source avant redistribution ;
- préférer des icônes cohérentes avec le thème GTK et éviter de mélanger
  plusieurs familles visuelles.

## 9. Identifiant d'application et intégration Linux

### 9.1 APP_ID

Format conseillé : `io.github.<utilisateur>.<App>`.

L'APP_ID doit être identique dans :

- le titre de fenêtre Slint si pertinent ;
- `slint::set_xdg_app_id("io.github.<utilisateur>.<App>")` ;
- le fichier `.desktop` (`Icon=` et `StartupWMClass=`) ;
- les chemins d'icônes hicolor (`.../apps/<APP_ID>.png`).

Cet identifiant doit aussi être choisi avant le packaging, car il fixe le nom des
icônes et du fichier desktop.

### 9.2 Fichier `.desktop`

`packaging/linux/io.github.<utilisateur>.<App>.desktop` :

```ini
[Desktop Entry]
Type=Application
Version=1.5
Name=<Proj>
Comment=<description courte>
Exec=<proj>-gui
TryExec=<proj>-gui
Icon=io.github.<utilisateur>.<App>
Terminal=false
Categories=Office;
StartupNotify=false
StartupWMClass=io.github.<utilisateur>.<App>
```

`StartupNotify=false` évite un délai de lancement sous GNOME pour ce type
d'application Slint/winit. Ajuster selon le comportement réel de l'application.

### 9.3 Installation locale

Script d'installation (modèle) :

```sh
APP_ID="io.github.<utilisateur>.<App>"
ICON_NAME="$APP_ID"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_HOME/applications"
ICONS_ROOT="$DATA_HOME/icons/hicolor"

for size in 16 32 48 64 128 256 512; do
  target_dir="$ICONS_ROOT/${size}x${size}/apps"
  mkdir -p "$target_dir"
  install -m 0644 "assets/icons/<app>-${size}.png" "$target_dir/${ICON_NAME}.png"
done

mkdir -p "$ICONS_ROOT/scalable/apps"
install -m 0644 "assets/icons/<app>.svg" "$ICONS_ROOT/scalable/apps/${ICON_NAME}.svg"

mkdir -p "$APPS_DIR"
sed -e "s|^Exec=.*|Exec=<chemin-du-binaire>|" \
    -e "s|^TryExec=.*|TryExec=<chemin-du-binaire>|" \
    packaging/linux/"${APP_ID}.desktop" > "$APPS_DIR/${APP_ID}.desktop"

command -v desktop-file-validate >/dev/null 2>&1 && desktop-file-validate "$APPS_DIR/${APP_ID}.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS_DIR"
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$ICONS_ROOT"
```

Le script doit aussi retirer les anciens noms d'icônes et de fichiers `.desktop`
si l'application a été renommée, afin d'éviter qu'un ancien identifiant soit
associé à la nouvelle fenêtre.

## 10. CI (`.github/workflows/ci.yml`)

Objectifs :

- vérifier le formatage ;
- exécuter Clippy ;
- compiler tout le workspace ;
- exécuter les tests ;
- compiler le feature GUI ;
- tourner sur Linux et Windows.

Extrait modèle :

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:"

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  verify:
    name: verify (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest]

    steps:
      - uses: actions/checkout@v7.0.1

      - name: Install Slint Linux dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          PACKAGES=(build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev)
          if apt-cache show libfontconfig-dev >/dev/null 2>&1; then PACKAGES+=(libfontconfig-dev); else PACKAGES+=(libfontconfig1-dev); fi
          if apt-cache show libfreetype-dev >/dev/null 2>&1; then PACKAGES+=(libfreetype-dev); else PACKAGES+=(libfreetype6-dev); fi
          if apt-cache show libgl-dev >/dev/null 2>&1; then PACKAGES+=(libgl-dev); else PACKAGES+=(libgl1-mesa-dev); fi
          sudo apt-get install -y "${PACKAGES[@]}"

      - uses: dtolnay/rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2.9.3
        with:
          key: ${{ runner.os }}-cargo

      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
      - run: cargo build --workspace --all-features --locked
      - run: cargo test --workspace --all-features --locked
```

Points d'attention :

- la toolchain Rust doit être déclarée explicitement avec `dtolnay/rust-toolchain` ;
- `--locked` est utilisé pour reproduire `Cargo.lock` ;
- les dépendances système Slint varient selon la distribution : choisir selon
  la version effectivement utilisée ;
- la compilation croisée aarch64 n'est ajoutée que si réellement nécessaire (voir
  contraintes de portabilité et glibc) ;
- les actions sont épinglées par version, idéalement par SHA, avec le numéro en
  commentaire ;
- `dependabot.yml` gère les mises à jour Cargo et GitHub Actions.
- adapter la branche par défaut selon le dépôt (`main` conseillé pour les
  nouveaux projets ; `master` pour les dépôts existants tels que Trimbox).

## 11. Release (`.github/workflows/release.yml`)

Déclenchement : push d'un tag `v*`.

Étapes minimum :

1. matrice `linux` / `windows` ;
2. installation des dépendances Slint pour Linux ;
3. toolchain stable ;
4. cache Cargo ;
5. build CLI puis GUI :
   ```sh
   cargo build --release -p <proj>-cli --locked --bin <proj>
   cargo build --release -p <proj>-cli --locked --features gui --bin <proj>-gui
   ```
6. packaging Linux en `tar.gz` contenant CLI, GUI, README, LICENSE et `.desktop` ;
7. packaging Windows en `.zip` contenant exe CLI, exe GUI, README et LICENSE ;
8. calcul des SHA256 ;
9. upload des artefacts ;
10. job final `publish` sur Ubuntu qui télécharge les artefacts, calcule un
    `SHA256SUMS` global et publie la GitHub Release avec `softprops/action-gh-release`.

Pour les notes de version, générer la liste des commits entre le tag précédent
et `HEAD`, en excluant les commits de maintenance de release si nécessaire.

Règles de sécurité et de robustesse :

- `permissions: contents: read` par défaut ;
- le job de publication seul a `contents: write` ;
- pas de secret en clair ;
- noms d'artefacts explicites avec nom, version et plateforme ;
- `fail_on_unmatched_files: true`.

## 12. Packaging Linux

Distribution par archive `tar.gz` pour le moment. Le fichier `.desktop` est inclus
dans l'archive, mais l'installation manuelle reste nécessaire pour copier les
icônes hicolor.

Pour une installation locale, utiliser `packaging/linux/install-local.sh` après
`cargo install --path crates/cli --features gui`. Le même principe peut être
étendu pour un `.deb` ou une AppImage si le projet le demande.

Points à vérifier pour Linux :

- l'APP_ID correspond au nom d'icône et au `StartupWMClass` ;
- Slint utilise le backend winit avec `renderer-femtovg` et `renderer-software` ;
- les bibliothèques dynamiques disponibles sur le runner (`glibc`) déterminent
  la compatibilité des binaires produits ;
- si la cible embarque une version plus ancienne de glibc, utiliser une image de
  build compatible ou produire depuis un environnement plus ancien.

## 12.1 Installation par Cargo

```sh
cargo install --locked --git https://github.com/<owner>/<repo>.git --branch main --features gui
```

Cette commande installe les deux binaires `<proj>` et `<proj>-gui` (sur Windows,
`<proj>.exe` et `<proj>-gui.exe`) dans `~/.cargo/bin`. Si ce répertoire n'est pas
dans `PATH`, il faut l'ajouter ou invoquer les binaires avec leur chemin
complet.

## 12.2 Fichiers assistant et qualité (`rustfmt`, Qwen Code)

`rustfmt.toml` :

```toml
edition = "2024"
max_width = 100
```

`.qwen/rules/ui-slint.md` (règle conditionnelle applicable à `**/*.slint`) :

```md
# Règles CI Slint

- Retirer les commentaires et conserver un seul composant public par vue.
- Placer un fichier `.slint` par vue ou composant.
- Centraliser les espacements, rayons et tailles dans un global.
- Utiliser les couleurs de `Palette` sauf état sémantique.
- Garder la surface publique de la racine minimale.
- Laisser la logique côté Rust.
- Vérifier chaque widget, élément ou propriété avec la documentation Slint avant
  modification via le serveur MCP `slint-docs`.
```

`QWEN.md` doit au minimum contenir :

- l'architecture et la commande de lancement ;
- les commandes de vérification ;
- le module de liaison UI ;
- le rappel d'utiliser `slint-docs` avec la version résolue dans `Cargo.lock` ;
- si `.qwen/` est ignoré par Git, préciser que les règles sont locales.

## 13. Commandes de vérification

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p <proj>-cli
cargo check -p <proj>-cli --features gui
cargo tree -p slint --depth 0
```

Vérifier aussi la version résolue de Slint dans `Cargo.lock`, puis c'est cette
version qui est utilisée pour interroger `slint-docs`. Si la documentation ne
correspond pas exactement, signaler l'écart.

## 14. Pièges courants

- Oublier que `slint::include_modules!()` génère les types dans le module où il
  est invoqué : le toast glue doit rester dans le même crate et le même module
  que l'inclusion.
- Écrire `ToastOverlay { ... }` sans instance réelle : l'import ne suffit pas à
  afficher quoi que ce soit.
- Reverser l'ordre du modèle Slint sans adapter l'indexation : les delegates
  doivent rester attachés au même toast.
- Confondre versions de Slint : la dernière 1.x de la documentation n'est pas
  forcément celle du projet.
- Sous Windows, oublier `windows_subsystem = "windows"` en release expose une
  console.
- Utiliser un identifiant d'application différent entre fenêtre, desktop et
  icônes : l'icône Linux n'est alors pas retrouvée.
- Publier une release sans artefacts GUI, ou oublier la checksum SHA256.
- Laisser des anciens noms d'icônes après renommage, ce qui fait que GNOME
  associe la mauvaise fenêtre.

## 15. Checklist pour un nouveau projet

- [ ] `cargo init` dans le répertoire courant, pas `cargo new`
- [ ] workspace `core` + `cli`
- [ ] feature `gui` optionnelle
- [ ] deux binaires déclarés
- [ ] `build.rs` avec Slint et ressource Windows
- [ ] `set_xdg_app_id` après création de la fenêtre
- [ ] toasts branchés et référencés vers `doc/toast-integration.md`
- [ ] assets et script de génération d'icônes
- [ ] packaging Linux et scripts install/uninstall
- [ ] `.github/workflows/ci.yml` et `release.yml`
- [ ] `dependabot.yml`
- [ ] commandes de vérification passées
- [ ] README mentionnant la version Slint
- [ ] serveurs MCP Slint configurés si l'agent Slint est utilisé

## 15.1 Hors périmètre et à décider par projet

Ce document ne fixe pas :

- le choix d'un runtime asynchrone (`tokio`, etc.) ;
- la persistance (configuration, cache, base locale) et les chemins utilisateur
  via la crate `directories` ;
- l'internationalisation ;
- la création d'installateurs Windows (MSI, NSIS) au-delà de l'archive `.zip` ;
- la distribution `.deb`/AppImage/paquets natifs ;
- la stratégie de versions et de changelog au-delà de la génération depuis les
  commits ;
- les tests d'intégration et end-to-end.

Ces choix doivent être cadrés en Phase 1 du démarrage, comme dans
`doc/prompt-init-projet-rust-slint.md`.

## 16. Sources Trimbox utiles

- `Cargo.toml`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml`
- `crates/cli/build.rs`, `crates/cli/src/main.rs`, `crates/cli/src/gui_main.rs`
- `crates/cli/src/gui.rs`, `crates/cli/src/gui/toast/mod.rs`, `crates/cli/src/gui/toast/queue.rs`
- `crates/cli/ui/main_window.slint`, `crates/cli/ui/toast/toast.slint`
- `assets/icons/generate.sh`
- `packaging/linux/*.desktop`, `packaging/linux/install-local.sh`, `uninstall-local.sh`
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/dependabot.yml`
- `doc/toast-integration.md`, `doc/gui-reference.md`
- `.qwen/agents/slint-ui.md`, `.qwen/rules/ui-slint.md`, `.qwen/settings.json`

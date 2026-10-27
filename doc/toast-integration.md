# Réutiliser le système de toasts Trimbox

Cette note explique comment porter le système de toasts actuel dans un autre
projet Rust + Slint sans introduire une crate lourde. Les valeurs du
comportement restent identiques : nouveau toast en haut, 3 visibles
maximum, éviction animée, auto-disparition à 4 secondes, transition d'opacité de 1 seconde et de hauteur de 200 ms.

## 1. Fichiers source

A copier dans le projet cible :

- `crates/cli/ui/snackbar.slint`
- `assets/icons/ui/close.svg`
- une base équivalente à `crates/cli/src/toast.rs`

Adapter les chemins relatifs au projet cible si nécessaire.

## 2. Partie Slint

Dans le fichier principal de l'interface :

```slint
import { VerticalBox } from "std-widgets.slint";
import { Snackbar, SnackbarItem, SnackbarOverlay } from "snackbar.slint";

in-out property <[SnackbarItem]> snackbar-items: [];

callback close-snackbar(int);
```

Conserver l'overlay supérieur droit à l'aide du composant réutilisable :

```slint
SnackbarOverlay {
    items: root.snackbar-items;
    available-width: root.width;
    spacing: 8px;
    close-requested(id) => { root.close-snackbar(id); }
}
```

Le composant encapsule la géométrie (largeur maximale 360 px, marges, position en haut à
droite), l'empilement inversé et la transmission de `close-requested`. Il utilise ses
valeurs par défaut si aucun thème n'est fourni ; pour réutiliser les jetons d'un projet,
passer `spacing` (et la largeur disponible de l'hôte via `available-width`).

Points conservés volontairement :

- la couleur sémantique vit dans `Snackbar` et ne dépend pas du thème ;
- `spacing` et `corner-radius` peuvent rester à leurs valeurs par défaut ;
- le badge de fermeture est une `TouchArea` sur une pastille 24x24 ;
- la disparition utilise une animation d'`opacity` de 1000 ms et de `height` de 200 ms.
- le modèle est écrit de façon à ce que les nouveaux toasts apparaissent en haut.

## 3. Partie Rust

Le module `toast.rs` contient `SnackbarQueue`, `enqueue_snackbar`, `sync_snackbar_stack`,
`dismiss_snackbar` et `schedule_auto_dismiss`. Le projet cible doit :

- remplacer `MainWindow` par son type de composant Slint généré ;
- remplacer `SnackbarItem` par le type généré correspondant ;
- conserver l'identifiant interne `u32`, Convertir en `i32` vers Slint ;
- raccorder `close-snackbar` à `dismiss_snackbar` ;
- reconstruire `VecModel<SnackbarItem>` dans `sync_snackbar_stack` ;
- réutiliser `Rc<RefCell<SnackbarQueue>>` dans les fermetures UI.

Le comportement à conserver sans modification involontaire :

- maximum 3 toasts actifs visibles ;
- eviction du plus ancien toast actif en le marquant `closing` ;
- finalisation après la durée d'animation de dismissal ;
- timer auto par toast, à 4 secondes ;
- nouvelles entrées placées à la fin, affichées en haut via ordre inverse.
- identification parallèle pile/IDs.

## 4. Icône

Copier `assets/icons/ui/close.svg` ou adapter `source: @image-url(...)` dans le
projet cible. Le badge utilise un rendu blanc sur pastille colorée.

## 5. Vérification

Dans le projet cible, après adaptation :

```sh
cargo fmt --check
cargo check -p votre-crate --features gui
cargo test -p votre-crate --features gui
```

Si le projet utilise Slint 1.18, vérifier que `slint-build` compile bien le
fichier `.slint` principal qui importe `snackbar.slint`.

## 6. Limites

Ce n'est pas une crate autonome. Le portage est volontairement manuel
(DRY, simple), avec preservation du comportement par copie/adaptation. Toute
absence d'import Slint doit être corrigée avant compilation.

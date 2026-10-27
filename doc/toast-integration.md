# Réutiliser le système de toasts Trimbox

Cette note explique comment porter le système de toasts actuel dans un autre
projet Rust + Slint sans introduire une crate lourde. Les valeurs du
comportement restent identiques : nouveau toast en haut, 3 visibles
maximum, éviction animée, auto-disparition à 4 secondes, transition de 350 ms.

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
import { Snackbar, SnackbarItem } from "snackbar.slint";

in-out property <[SnackbarItem]> snackbar-items: [];

callback close-snackbar(int);
```

Conserver un overlay supérieur droit au moins équivalent à :

```slint
Rectangle {
    visible: root.snackbar-items.length > 0;
    width: min(360px, max(0px, root.width - 16px));
    height: self.preferred-height;
    x: max(8px, root.width - self.width - 8px);
    y: 8px;
    background: transparent;

    VerticalBox {
        spacing: 8px;
        for i in root.snackbar-items.length : Snackbar {
            text: root.snackbar-items[root.snackbar-items.length - 1 - i].text;
            level: root.snackbar-items[root.snackbar-items.length - 1 - i].level;
            id: root.snackbar-items[root.snackbar-items.length - 1 - i].id;
            closing: root.snackbar-items[root.snackbar-items.length - 1 - i].closing;
            close-requested => { root.close-snackbar(self.id); }
        }
    }
}
```

Points conservés volontairement :

- la couleur sémantique vit dans `Snackbar` et ne dépend pas du thème ;
- `spacing` et `corner-radius` peuvent rester à leurs valeurs par défaut ;
- le badge de fermeture est une `TouchArea` sur une pastille 24x24 ;
- la disparition utilise `opacity` et `height` animées sur 350 ms.
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

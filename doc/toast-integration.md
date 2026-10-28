# Reusable toast notification system for Rust + Slint

This note explains how to port the toast notification system into another Rust +
Slint project without introducing a heavy crate. The behavior stays identical:
toasts stack at the top right, at most three are visible, eviction is animated,
each toast disappears automatically after four seconds, and exit transitions
animate height, position, and opacity.

## 1. Files to copy

Copy these files into the target project, then adapt relative paths if needed:

- `ui/toast/toast.slint`
- toast Rust code under the target's GUI module:
  - `toast/mod.rs`
  - `toast/queue.rs`
- `assets/icons/ui/close.svg`, or replace the default `close-icon` image

The Slint file owns all markup. The Rust code is kept beside the generated Slint
module because `include_modules!()` creates the Slint types in that module.

## 2. Slint integration

The host component only imports the reusable overlay and exposes the generated
globals. It does not declare its own toast properties or callbacks.

```slint
import { ToastTiming, ToastState, ToastOverlay } from "toast/toast.slint";

export { ToastTiming, ToastState } from "toast/toast.slint";

// Host component, for example a window or root view.
export component HostWindow inherits Window {
    // ... ordinary application UI ...

    // Keep the overlay above the application content so toasts remain visible.
    ToastOverlay {
        available-width: root.width;
        spacing: Theme.spacing;
    }
}
```

If the target project has no `Theme` global, pass a concrete spacing value such
as `8px`. The overlay also has defaults (`available-width: 860px`,
`spacing: 8px`), but a host should pass its real width.

`toast.slint` exports the following Slint API:

- `ToastItem`: `{ text: string, level: int, id: int, closing: bool }`
- `ToastTiming`: global with `enter: 250ms` and `exit: 200ms`
- `ToastState`: global with `in property <[ToastItem]> items` and
  `callback close-requested(int)`
- `Toast`: the visual component used by the overlay
- `ToastOverlay`: the positioned stack container

The host does not read or write `ToastState.items` directly. The Rust side
owns the model. `ToastOverlay` reads `ToastState.items`, renders `Toast` for each
entry, and calls `ToastState.close-requested(id)` when a toast's close badge is
clicked.

## 3. Rust integration

The toast Rust module uses Slint types generated in the GUI module, so it is a
submodule of the GUI module. A typical setup is:

```rust
#![cfg(feature = "gui")]

mod toast;
use toast::{Toaster, ToastLevel};

use anyhow::Result;
use slint::ComponentHandle;

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui = MainWindow::new()
        .map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    let toaster = Toaster::install(&ui);

    ui.on_browse_source(move || {
        // ... application work ...
        toaster.show("Source loaded.".to_owned(), ToastLevel::Success);
    });

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI execution failed: {error}"))?;
    Ok(())
}
```

`Toaster` is cheap to clone. Passing `&Toaster` into helper functions works well:

```rust
fn crop_document(ui: &MainWindow, toaster: &Toaster) {
    if input_is_missing() {
        toaster.show("Choose a source file first.".to_owned(), ToastLevel::Error);
        return;
    }

    // ... document processing ...

    if has_warnings {
        toaster.show("Completed with warnings.".to_owned(), ToastLevel::Warning);
    } else {
        toaster.show("Completed.".to_owned(), ToastLevel::Success);
    }
}
```

`Toaster::install(ui)`:

- creates the private `Rc<RefCell<ToastQueue>>`
- branches `ToastState::on_close_requested`
- initializes `ToastState.items` with an empty model
- returns a `Toaster`

`Toaster::show(text, level)` creates a toast and schedules automatic dismissal
after four seconds. `ToastLevel` maps to the Slint integer expected by
`ToastItem.level`: `Success = 0`, `Warning = 1`, `Error = 2`.

Because the close path is already handled by `Toaster::install`, the host no
longer needs its own close callback or model management.

## 4. Rust module layout

```text
gui.rs                      // include_modules!(), mod toast;, app handlers
gui/toast/mod.rs            // Toaster, ToastLevel, Slint glue
gui/toast/queue.rs          // pure queue logic and unit tests
ui/toast/toast.slint        // all toast Slint markup
ui/host.slint               // imports ToastOverlay
```

`queue.rs` has no Slint dependency. It contains:

- `ToastNotice`: user-facing notice (`text`, `level`, `closing`)
- `ToastQueue`: stack, IDs, eviction, two-phase removal
- constants for three active toasts and four seconds
- unit tests

If the host already has a Cargo feature for the GUI, reuse it. The common
feature shape is:

```toml
[features]
gui = ["dep:slint", "dep:slint-build"]

[dependencies]
slint = { workspace = true, optional = true }

[build-dependencies]
slint-build = { workspace = true, optional = true }
```

The build script compiles the host Slint entry point:

```rust
#[cfg(feature = "gui")]
{
    if let Err(error) = slint_build::compile("ui/host.slint") {
        panic!("Slint compilation failed: {error}");
    }
}
```

## 5. Behavior contract

Preserve these invariants when adapting or extending the system:

- toasts are created with `closing = false`
- at most three active toasts may be visible
- when a fourth active toast is created, the oldest active toast is marked
  `closing`
- closing toasts remain in the model during their exit animation
- final removal happens after the configured exit duration
- auto-dismiss runs independently for each toast after four seconds
- manual dismissal uses the same two-phase path as eviction
- new entries are appended at the end of the queue
- the overlay renders the queue in reverse order, so newer toasts appear higher
- the queue identifies toasts by ID, not by row or position
- `sync`-style model updates compare entries and mutate the existing `VecModel`
  in place instead of replacing it, so toast delegates keep their animation
  identity

The toast colors stay semantic and intentionally vivid rather than theme-derived:
success and error use white text, warning uses dark text.

## 6. Icon

The default close icon is an `in property <image>` on `Toast`. A host can use
the built-in default or override it:

```slint
Toast {
    close-icon: @image-url("assets/icons/ui/close.svg");
}
```

The badge is a 24x24 px circle with a white 14x14 px icon inside it. In the
source Slint file the default path is relative to `toast.slint`; adjust it when
copying the file to another directory.

## 7. Verification

After adaptation, run the host's normal checks:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p host-crate
cargo check -p host-crate --features gui
```

If the host uses Slint 1.18, confirm that `slint-build` can compile the
primary Slint file after it imports `toast.slint`.

## 8. Limitations

This is not a standalone crate. Porting is intentionally manual: copy, adapt
paths and types, then compile. The Rust API is concrete to the host's generated
Slint window type. Genericity over `ComponentHandle` is possible only if the
host keeps the same generated globals and property names.

## Sources in Trimbox

The implementation files referenced by this guide are:

- `crates/cli/ui/toast/toast.slint`
- `crates/cli/src/gui/toast/mod.rs`
- `crates/cli/src/gui/toast/queue.rs`
- `crates/cli/src/gui.rs`
- `crates/cli/ui/main_window.slint`
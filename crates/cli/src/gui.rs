#![cfg(feature = "gui")]
#![allow(unused)]

use slint::ComponentHandle;

slint::include_modules!();

pub fn run() -> anyhow::Result<()> {
    let ui = MainWindow::new()
        .map_err(|error| anyhow::anyhow!("creation de la fenetre impossible : {error}"))?;

    slint::set_xdg_app_id("io.github.Franck-Demongin.Trimbox");

    ui.run()
        .map_err(|error| anyhow::anyhow!("execution de l'interface impossible : {error}"))?;

    Ok(())
}

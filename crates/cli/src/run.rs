use crate::cli::{Cli, Command, PdfCommand};
use anyhow::{Result, bail};
use clap::Parser;
use std::fs;
use trimbox_core::pdf;

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Pdf {
            command: PdfCommand::Info { path },
        }) => {
            let metadata = fs::metadata(&path)?;
            if !metadata.is_file() {
                bail!("chemin non fichier : {}", path.display());
            }
            let bytes = fs::read(&path)?;
            let _document = pdf::load_document(&bytes)?;
            println!("PDF lisible : {}", path.display());
            Ok(())
        }
        #[cfg(feature = "gui")]
        Some(Command::Gui) => crate::gui::run(),
        None => {
            println!("Trimbox : essayez `trimbox --help`");
            Ok(())
        }
    }
}

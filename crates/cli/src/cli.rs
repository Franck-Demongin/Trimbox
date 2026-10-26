use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "trimbox",
    version,
    about = "Lecture et generation de PDFs conformes PDF/X-4"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Inspecte ou transforme un fichier PDF.
    Pdf {
        #[command(subcommand)]
        command: PdfCommand,
    },
    /// Lance l'interface graphique Slint, si le feature `gui` est active.
    #[cfg(feature = "gui")]
    Gui,
}

#[derive(Subcommand, Debug)]
pub enum PdfCommand {
    /// Verifie qu'un PDF peut etre lu.
    Info { path: PathBuf },
}

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
}

#[derive(Subcommand, Debug)]
pub enum PdfCommand {
    /// Affiche les informations et les boites d'un PDF.
    Info { path: PathBuf },
    /// Valide la conformite PDF/X-4 d'un document.
    Validate { path: PathBuf },
    /// Normalise un document vers PDF/X-4.
    Normalize { input: PathBuf, output: PathBuf },
    /// Recadre un PDF au centre aux dimensions indiquees.
    Crop {
        input: PathBuf,
        output: PathBuf,
        #[arg(short, long)]
        width: f64,
        #[arg(short = 'H', long)]
        height: f64,
    },
}

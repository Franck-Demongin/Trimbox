use crate::cli::{Cli, Command, PdfCommand};
use anyhow::{Result, bail};
use clap::Parser;
use std::fs;
use std::path::Path;
use trimbox_core::boxes::read_page_boxes;
use trimbox_core::pdf;

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Pdf {
            command: PdfCommand::Info { path },
        }) => {
            if !path.is_file() {
                bail!("chemin non fichier : {}", path.display());
            }
            let bytes = fs::read(&path)?;
            let document = pdf::load_document(&bytes)?;
            println!("PDF lisible : {}", path.display());
            println!("Pages : {}", pdf::page_count(&document));
            for boxes in read_page_boxes(&document).map_err(|e| anyhow::anyhow!("{e}"))? {
                println!(
                    "page {} : MediaBox={:?} CropBox={:?} TrimBox={:?} BleedBox={:?} ArtBox={:?}",
                    boxes.page, boxes.media, boxes.crop, boxes.trim, boxes.bleed, boxes.art
                );
            }
            let source = trimbox_core::analyze_source(&document);
            println!("Couleur source : {}", source.color.mode().label());
            println!(
                "Profils ICC source : {} (sorties : {})",
                source.color.icc_profile_count(),
                source.color.output_intent_profile_count
            );
            println!("Metadonnees source : {}", source.metadata_note());
            Ok(())
        }
        Some(Command::Pdf {
            command: PdfCommand::Validate { path },
        }) => {
            if !path.is_file() {
                bail!("chemin non fichier : {}", path.display());
            }
            let bytes = fs::read(&path)?;
            let document = pdf::load_document(&bytes)?;
            let report = trimbox_core::validate_pdf_x4(&document, &path.display().to_string());
            println!("{}", report);
            let code = report.exit_code();
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Some(Command::Pdf {
            command: PdfCommand::Normalize { input, output },
        }) => {
            if !input.is_file() {
                bail!("chemin non fichier : {}", input.display());
            }
            if same_path(&input, &output) {
                bail!("le fichier de sortie doit differer du fichier source");
            }
            let bytes = fs::read(&input)?;
            let mut document = pdf::load_document(&bytes)
                .map_err(|e| anyhow::anyhow!("lecture PDF impossible : {e}"))?;

            let report = trimbox_core::normalize_pdf_x4(
                &mut document,
                &input.display().to_string(),
                &output.display().to_string(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            if report.blocked.is_none() {
                let output_bytes = pdf::save_document(&mut document)
                    .map_err(|e| anyhow::anyhow!("ecriture PDF impossible : {e}"))?;
                fs::write(&output, output_bytes)?;
            }

            println!("{}", report);
            let code = if report.blocked.is_some() {
                2
            } else {
                report.after.exit_code()
            };
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
        Some(Command::Pdf {
            command:
                PdfCommand::Crop {
                    input,
                    output,
                    width,
                    height,
                },
        }) => {
            if !input.is_file() {
                bail!("chemin non fichier : {}", input.display());
            }
            if same_path(&input, &output) {
                bail!("le fichier de sortie doit differer du fichier source");
            }
            if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
                bail!("dimensions invalides");
            }
            let bytes = fs::read(&input)?;
            let mut document = pdf::load_document(&bytes)
                .map_err(|e| anyhow::anyhow!("lecture PDF impossible : {e}"))?;

            let report = trimbox_core::crop_document(
                &mut document,
                width,
                height,
                &input.display().to_string(),
                &output.display().to_string(),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;

            let output_bytes = pdf::save_document(&mut document)
                .map_err(|e| anyhow::anyhow!("ecriture PDF impossible : {e}"))?;
            fs::write(&output, output_bytes)?;
            println!("{}", report);
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

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

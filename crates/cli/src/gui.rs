#![cfg(feature = "gui")]
#![allow(unused)]

use anyhow::anyhow;
use anyhow::{Context, Result};
use lopdf::Document;
use slint::ComponentHandle;
use std::fs;
use std::path::{Path, PathBuf};

use trimbox_core::boxes::read_page_boxes;
use trimbox_core::pdf::{load_document, page_count, save_document};
use trimbox_core::{analyze_source, crop_document, normalize_pdf_x4, validate_pdf_x4};

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui = MainWindow::new()
        .map_err(|error| anyhow::anyhow!("creation de la fenetre impossible : {error}"))?;

    slint::set_xdg_app_id("io.github.Franck-Demongin.Trimbox");

    let weak = ui.as_weak();

    let browse_source_weak = weak.clone();
    ui.on_browse_source(move || {
        if let Some(ui) = browse_source_weak.upgrade() {
            let dialog = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_title("Ouvrir un PDF");
            if let Some(path) = dialog.pick_file() {
                let source = path.display().to_string();
                ui.set_source_path(source.clone().into());
                let output = default_output_path(&path, "output");
                ui.set_output_path(output.display().to_string().into());
                set_status(&ui, "Source chargée.".to_owned(), 0);
            }
        }
    });

    let browse_output_weak = weak.clone();
    ui.on_browse_output(move || {
        if let Some(ui) = browse_output_weak.upgrade() {
            let dialog = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_title("Enregistrer le PDF généré");
            if let Some(path) = dialog.save_file() {
                ui.set_output_path(path.display().to_string().into());
            }
        }
    });

    let inspect_weak = weak.clone();
    ui.on_inspect_source(move || {
        if let Some(ui) = inspect_weak.upgrade() {
            inspect_document(&ui);
        }
    });

    let validate_weak = weak.clone();
    ui.on_validate(move || {
        if let Some(ui) = validate_weak.upgrade() {
            validate_document(&ui);
        }
    });

    let normalize_weak = weak.clone();
    ui.on_normalize(move || {
        if let Some(ui) = normalize_weak.upgrade() {
            normalize_document(&ui);
        }
    });

    let crop_weak = weak.clone();
    ui.on_crop(move || {
        if let Some(ui) = crop_weak.upgrade() {
            crop_document_gui(&ui);
        }
    });

    ui.run()
        .map_err(|error| anyhow::anyhow!("execution de l'interface impossible : {error}"))?;
    Ok(())
}

fn inspect_document(ui: &MainWindow) {
    let path = source_path(ui);
    if path.is_empty() {
        set_status(ui, "Choisir d'abord un fichier source.".to_owned(), 2);
        return;
    }
    let path = PathBuf::from(path);
    let source = path.display().to_string();

    let document = match load_document_for_inspection(&path, &source) {
        Ok(document) => document,
        Err(message) => {
            set_report(ui, message.to_string(), "Source non lisible.".to_owned(), 2);
            return;
        }
    };

    let document_summary = match build_source_report(&path, &source, &document) {
        Ok(summary) => summary,
        Err(message) => {
            set_report(
                ui,
                format!("Analyse impossible: {message}"),
                "Analyse échouée.".to_owned(),
                2,
            );
            return;
        }
    };
    set_report(ui, document_summary, "Analyse terminée.".to_owned(), 0);
}

fn validate_document(ui: &MainWindow) {
    let path = source_path(ui);
    if path.is_empty() {
        set_status(ui, "Choisir d'abord un fichier source.".to_owned(), 2);
        return;
    }
    let path = PathBuf::from(path);
    let source = path.display().to_string();
    let bytes =
        match fs::read(&path).with_context(|| format!("lecture impossible: {}", path.display())) {
            Ok(bytes) => bytes,
            Err(error) => {
                set_status(ui, error.to_string(), 2);
                return;
            }
        };
    let document = match load_document(&bytes) {
        Ok(document) => document,
        Err(error) => {
            set_status(ui, format!("PDF illisible : {error}"), 2);
            return;
        }
    };

    let report = validate_pdf_x4(&document, &source);
    let level = report.exit_code().clamp(0, 2);
    set_report(
        ui,
        report.to_string(),
        format!("Validation terminée (code {}).", report.exit_code()),
        level,
    );
}

fn normalize_document(ui: &MainWindow) {
    let input_path = source_path(ui);
    if input_path.is_empty() {
        set_status(ui, "Choisir d'abord un fichier source.".to_owned(), 2);
        return;
    }
    let output = output_path(ui);
    let fallback = default_output_path(Path::new(&input_path), "x4");
    let output = if output.is_empty() {
        fallback
    } else {
        PathBuf::from(output)
    };
    if same_path(Path::new(&input_path), &output) {
        set_status(
            ui,
            "Le fichier de sortie doit differer du fichier source.".to_owned(),
            2,
        );
        return;
    }
    let input = PathBuf::from(input_path);
    let source = input.display().to_string();
    let output_display = output.display().to_string();

    let bytes = match fs::read(&input)
        .with_context(|| format!("lecture impossible: {}", input.display()))
    {
        Ok(bytes) => bytes,
        Err(error) => {
            set_status(ui, error.to_string(), 2);
            return;
        }
    };
    let mut document = match load_document(&bytes) {
        Ok(document) => document,
        Err(error) => {
            set_status(ui, format!("PDF illisible : {error}"), 2);
            return;
        }
    };

    let report = match normalize_pdf_x4(&mut document, &source, &output_display) {
        Ok(report) => report,
        Err(error) => {
            set_status(ui, format!("Normalisation impossible : {error}"), 2);
            return;
        }
    };

    if report.blocked.is_none() {
        match save_document(&mut document) {
            Ok(bytes) => {
                if let Err(error) = fs::write(&output, bytes) {
                    set_status(ui, format!("Ecriture impossible : {error}"), 2);
                    return;
                }
            }
            Err(error) => {
                set_status(ui, format!("Serialisation impossible : {error}"), 2);
                return;
            }
        }
        ui.set_output_path(output_display.clone().into());
        set_report(
            ui,
            report.to_string(),
            format!(
                "PDF normalisé : {} (code {}).",
                output_display,
                report.after.exit_code()
            ),
            report.after.exit_code().clamp(0, 2),
        );
    } else {
        set_report(
            ui,
            report.to_string(),
            "Normalisation bloquée.".to_owned(),
            2,
        );
    }
}

fn crop_document_gui(ui: &MainWindow) {
    let input = source_path(ui);
    if input.is_empty() {
        set_status(ui, "Choisir d'abord un fichier source.".to_owned(), 2);
        return;
    }
    let width = match ui.get_crop_width_mm().as_str().trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        _ => {
            set_status(ui, "Largeur de recadrage invalide.".to_owned(), 2);
            return;
        }
    };
    let height = match ui.get_crop_height_mm().as_str().trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        _ => {
            set_status(ui, "Hauteur de recadrage invalide.".to_owned(), 2);
            return;
        }
    };
    let input_path = PathBuf::from(input);
    let source = input_path.display().to_string();
    let fallback = PathBuf::from(format!(
        "{}-recadre-{}x{}mm.pdf",
        input_path.file_stem().unwrap_or_default().to_string_lossy(),
        width,
        height
    ));
    let output = {
        let current = output_path(ui);
        if current.is_empty() {
            fallback
        } else {
            PathBuf::from(current)
        }
    };
    if same_path(&input_path, &output) {
        set_status(
            ui,
            "Le fichier de sortie doit differer du fichier source.".to_owned(),
            2,
        );
        return;
    }
    let bytes = match fs::read(&input_path)
        .with_context(|| format!("lecture impossible: {}", input_path.display()))
    {
        Ok(bytes) => bytes,
        Err(error) => {
            set_status(ui, error.to_string(), 2);
            return;
        }
    };
    let mut document = match load_document(&bytes) {
        Ok(document) => document,
        Err(error) => {
            set_status(ui, format!("PDF illisible : {error}"), 2);
            return;
        }
    };
    let report = match crop_document(
        &mut document,
        width,
        height,
        &source,
        &output.display().to_string(),
    ) {
        Ok(report) => report,
        Err(error) => {
            set_status(ui, format!("Recadrage impossible : {error}"), 2);
            return;
        }
    };
    match save_document(&mut document) {
        Ok(bytes) => {
            if let Err(error) = fs::write(&output, bytes) {
                set_status(ui, format!("Ecriture impossible : {error}"), 2);
                return;
            }
        }
        Err(error) => {
            set_status(ui, format!("Serialisation impossible : {error}"), 2);
            return;
        }
    }
    ui.set_output_path(output.display().to_string().into());
    set_report(
        ui,
        report.to_string(),
        format!(
            "PDF recadré en {width} x {height} mm : {}.",
            output.display()
        ),
        0,
    );
}

fn load_document_for_inspection(path: &Path, source: &str) -> Result<lopdf::Document> {
    let bytes =
        fs::read(path).with_context(|| format!("lecture impossible: {}", path.display()))?;
    load_document(&bytes).map_err(|error| anyhow!(format!("PDF illisible : {error}")))
}

fn build_source_report(path: &Path, source: &str, document: &Document) -> Result<String> {
    let boxes = read_page_boxes(document)
        .map_err(|error| anyhow!(format!("lecture des boites impossible : {error}")))?;
    let characteristics = analyze_source(document);
    let mut lines = Vec::new();
    lines.push(format!("PDF lisible : {}", source));
    lines.push(format!("Pages : {}", page_count(document)));
    for page in boxes {
        lines.push(format!(
            "page {} : MediaBox={:?} CropBox={:?} TrimBox={:?} BleedBox={:?} ArtBox={:?}",
            page.page, page.media, page.crop, page.trim, page.bleed, page.art
        ));
    }
    lines.push(format!(
        "Couleur source : {}",
        characteristics.color.mode().label()
    ));
    lines.push(format!(
        "Profils ICC source : {} (sorties : {})",
        characteristics.color.icc_profile_count(),
        characteristics.color.output_intent_profile_count
    ));
    lines.push(format!("Métadonnées : {}", characteristics.metadata_note()));
    Ok(lines.join("\n"))
}

fn set_report(ui: &MainWindow, report: String, status: String, level: i32) {
    ui.set_report(report.into());
    set_status(ui, status, level);
}

fn set_status(ui: &MainWindow, status: impl Into<slint::SharedString>, level: i32) {
    ui.set_status(status.into());
    ui.set_status_level(level);
}

fn source_path(ui: &MainWindow) -> String {
    ui.get_source_path().as_str().to_owned()
}

fn output_path(ui: &MainWindow) -> String {
    ui.get_output_path().as_str().to_owned()
}

fn default_output_path(input: &Path, suffix: &str) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("sortie");
    PathBuf::from(parent).join(format!("{stem}-{suffix}.pdf"))
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

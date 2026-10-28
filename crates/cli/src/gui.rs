#![cfg(feature = "gui")]

mod toast;
use toast::{ToastLevel, Toaster};

use anyhow::{Context, Result};
use slint::ComponentHandle;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use trimbox_core::crop::crop_document;
use trimbox_core::pdf::{load_document, save_document};
use trimbox_core::{ColorMode, CropReport, IssueSeverity, PdfXValidationReport, validate_pdf_x4};

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui = MainWindow::new()
        .map_err(|error| anyhow::anyhow!("creation de la fenetre impossible : {error}"))?;

    ui.set_window_title(format!("Trimbox - v{}", env!("CARGO_PKG_VERSION")).into());

    let _ = slint::set_xdg_app_id("io.github.Franck-Demongin.Trimbox");

    let weak = ui.as_weak();

    let toaster = Toaster::install(&ui);

    let browse_source_weak = weak.clone();
    let browse_toaster = toaster.clone();
    ui.on_browse_source(move || {
        if let Some(ui) = browse_source_weak.upgrade() {
            let dialog = rfd::FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_title("Ouvrir un PDF");
            if let Some(path) = dialog.pick_file() {
                let source = path.display().to_string();
                let output = default_output_path(&path, "recadre");
                let source_size = match fs::read(&path) {
                    Ok(bytes) => match trimbox_core::pdf::load_document(&bytes) {
                        Ok(document) => match trimbox_core::pdf::source_page_size(&document) {
                            Ok((width_mm, height_mm, has_other_sizes)) => {
                                let mut text =
                                    format!("Page 1 : {:.1} x {:.1} mm", width_mm, height_mm);
                                if has_other_sizes {
                                    text.push_str(" (pages de tailles differentes)");
                                }
                                text
                            }
                            Err(_) => String::new(),
                        },
                        Err(_) => String::new(),
                    },
                    Err(_) => String::new(),
                };
                ui.set_source_path(source.clone().into());
                ui.set_output_path(output.display().to_string().into());
                ui.set_output_available(false);
                ui.set_report_available(false);
                ui.set_source_size(source_size.into());
                set_status(&ui, &browse_toaster, "Source chargée.".to_owned(), 0);
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
                let normalized = ensure_pdf_extension(path);
                ui.set_output_path(normalized.display().to_string().into());
            }
        }
    });

    let crop_weak = weak.clone();
    let crop_toaster = toaster.clone();
    ui.on_crop(move || {
        if let Some(ui) = crop_weak.upgrade() {
            crop_document_gui(&ui, &crop_toaster);
        }
    });

    let open_weak = weak.clone();
    let open_toaster = toaster.clone();
    ui.on_open_output(move || {
        if let Some(ui) = open_weak.upgrade() {
            open_output_gui(&ui, false, &open_toaster);
        }
    });

    let open_folder_weak = weak.clone();
    let open_folder_toaster = toaster.clone();
    ui.on_open_output_folder(move || {
        if let Some(ui) = open_folder_weak.upgrade() {
            open_output_gui(&ui, true, &open_folder_toaster);
        }
    });

    ui.run()
        .map_err(|error| anyhow::anyhow!("execution de l'interface impossible : {error}"))?;
    Ok(())
}

fn crop_document_gui(ui: &MainWindow, toaster: &Toaster) {
    ui.set_output_available(false);
    ui.set_report_available(false);

    let input = source_path(ui);
    if input.is_empty() {
        set_status(
            ui,
            toaster,
            "Choisir d'abord un fichier source.".to_owned(),
            2,
        );
        return;
    }
    let width = match ui.get_crop_width_mm().as_str().trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        _ => {
            set_status(ui, toaster, "Largeur de recadrage invalide.".to_owned(), 2);
            return;
        }
    };
    let height = match ui.get_crop_height_mm().as_str().trim().parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => value,
        _ => {
            set_status(ui, toaster, "Hauteur de recadrage invalide.".to_owned(), 2);
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
    let output = ensure_pdf_extension({
        let current = output_path(ui);
        if current.is_empty() {
            fallback
        } else {
            PathBuf::from(current)
        }
    });

    if same_path(&input_path, &output) {
        set_status(
            ui,
            toaster,
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
            set_status(ui, toaster, error.to_string(), 2);
            return;
        }
    };
    let mut document = match load_document(&bytes) {
        Ok(document) => document,
        Err(error) => {
            set_status(ui, toaster, format!("PDF illisible : {error}"), 2);
            return;
        }
    };

    let validation = validate_pdf_x4(&document, &source);

    let report = match crop_document(
        &mut document,
        width,
        height,
        &source,
        &output.display().to_string(),
    ) {
        Ok(report) => report,
        Err(error) => {
            set_report(
                ui,
                "Résultat",
                format!("Recadrage impossible : {error}"),
                "Recadrage impossible.".to_owned(),
                2,
                toaster,
            );
            return;
        }
    };

    let saved_bytes = match save_document(&mut document) {
        Ok(bytes) => bytes,
        Err(error) => {
            set_status(
                ui,
                toaster,
                format!("Serialisation impossible : {error}"),
                2,
            );
            return;
        }
    };

    if let Err(error) = fs::write(&output, saved_bytes) {
        set_status(ui, toaster, format!("Ecriture impossible : {error}"), 2);
        return;
    }

    let output_size = fs::metadata(&output)
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    ui.set_output_path(output.display().to_string().into());
    ui.set_output_available(true);

    let warning_count = warning_count_for_crop(&report, &validation);
    let level = if warning_count > 0 { 1 } else { 0 };
    let status = if warning_count > 0 {
        format!("Recadrage terminé avec {} avertissement(s).", warning_count)
    } else {
        "Recadrage terminé.".to_owned()
    };
    set_report(
        ui,
        "Résultat",
        format_crop_summary(&report, &validation, output_size),
        status,
        level,
        toaster,
    );
}

pub fn format_crop_summary(
    report: &CropReport,
    validation: &PdfXValidationReport,
    output_size: u64,
) -> String {
    let color_mode = report.source_characteristics.color.mode().label();
    let mut warnings = Vec::new();

    let fallback = report.source_characteristics.color.fallback_note();
    if fallback.contains("Repli") && !fallback.contains("Aucun repli couleur") {
        warnings.push(format!("Couleur : {fallback}"));
    }
    if report.source_characteristics.color.mode() == ColorMode::Unknown {
        warnings.push(
            "Couleur source : aucun mode détecté; les flots source restent conservés.".to_owned(),
        );
    }

    let issues: Vec<_> = validation
        .issues
        .iter()
        .filter(|issue| issue.severity >= IssueSeverity::Warning)
        .collect();
    for issue in issues.iter().take(8) {
        warnings.push(format!(
            "[{}] {} : {}",
            issue.severity.label(),
            issue.code,
            issue.message
        ));
    }
    if issues.len() > 8 {
        warnings.push(format!(
            "... {} avertissement(s) supplémentaire(s) non affiché(s).",
            issues.len() - 8
        ));
    }

    let validation_line = if validation.issues.is_empty() {
        "Validation PDF/X-4 : aucune non-conformité détectée.".to_owned()
    } else {
        format!(
            "Validation PDF/X-4 : {} écart(s), dont {} erreur(s) et {} avertissement(s).",
            validation.issues.len(),
            validation
                .issues
                .iter()
                .filter(|issue| issue.severity == IssueSeverity::Error)
                .count(),
            validation
                .issues
                .iter()
                .filter(|issue| issue.severity == IssueSeverity::Warning)
                .count()
        )
    };

    let page_word = if report.page_count <= 1 {
        "page"
    } else {
        "pages"
    };

    let mut lines = Vec::new();
    lines.push("Trimbox — résultat".to_owned());
    lines.push(format!("Fichier source : {}", report.source));
    lines.push(format!("Fichier généré : {}", report.output));
    lines.push("Résultat global : SUCCES".to_owned());
    lines.push(format!(
        "Dimensions recadrées : {:.2} mm x {:.2} mm ({} x {} pt)",
        report.requested_width_mm,
        report.requested_height_mm,
        format_pt(report.requested_width_pt),
        format_pt(report.requested_height_pt)
    ));
    lines.push(format!("Pages : {} {}", report.page_count, page_word));
    lines.push(format!("Couleur source : {}", color_mode));
    lines.push(format!(
        "Profils ICC source : {}",
        report.source_characteristics.color.icc_profile_count()
    ));
    lines.push(format!(
        "Profils de sortie : {}",
        report
            .source_characteristics
            .color
            .output_intent_profile_count
    ));
    lines.push(format!(
        "Taille du fichier produit : {}",
        format_bytes(output_size)
    ));
    lines.push(validation_line);

    let mut output = lines.join("\n");
    if warnings.is_empty() {
        output.push_str("\nAvertissements : aucun.");
    } else {
        output.push_str("\nAvertissements :");
        for warning in warnings {
            output.push('\n');
            output.push_str("- ");
            output.push_str(&warning);
        }
    }

    output
}

#[allow(dead_code)]
pub fn format_crop_report(
    report: &CropReport,
    validation: &PdfXValidationReport,
    output_size: u64,
) -> String {
    format_crop_summary(report, validation, output_size)
}

#[allow(dead_code)]
pub fn format_crop_result(
    report: &CropReport,
    validation: &PdfXValidationReport,
    output_size: u64,
) -> String {
    format_crop_summary(report, validation, output_size)
}

pub fn format_report_body(_kind: &str, raw: &str) -> String {
    let normalized = normalize_blank_lines(raw);
    annotate_json_values(&normalized)
}

pub fn normalize_report_body(raw: &str) -> String {
    normalize_blank_lines(raw)
}

pub fn normalize_blank_lines(raw: &str) -> String {
    let mut output = String::new();
    let mut previous_blank = false;
    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            if previous_blank {
                continue;
            }
            output.push('\n');
            previous_blank = true;
        } else {
            previous_blank = false;
            output.push_str(trimmed);
            output.push('\n');
        }
    }
    while output.ends_with('\n') && output.len() > 1 {
        let _ = output.pop();
    }
    if output.trim().is_empty() {
        String::new()
    } else {
        output
    }
}

pub fn format_json_value(raw: &str) -> String {
    let trimmed = raw.trim();
    let parsed = serde_json::from_str::<serde_json::Value>(trimmed);
    match parsed {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| trimmed.to_owned()),
        Err(_) => trimmed.to_owned(),
    }
}

pub fn annotate_json_values(raw: &str) -> String {
    let mut output = String::new();
    for line in raw.lines() {
        output.push_str(&annotate_json_on_line(line));
        output.push('\n');
    }
    if raw.ends_with('\n') || output.is_empty() {
        output
    } else if output.ends_with('\n') {
        output.pop();
        output
    } else {
        output
    }
}

pub fn annotate_json_on_line(line: &str) -> String {
    let start = line.find('{').into_iter().chain(line.find('[')).min();
    let Some(start) = start else {
        return line.to_owned();
    };

    let mut end = None;
    for index in start + 1..=line.len() {
        if !line.is_char_boundary(index) {
            continue;
        }
        if serde_json::from_str::<serde_json::Value>(&line[start..index]).is_ok() {
            end = Some(index);
            break;
        }
    }
    let Some(end) = end else {
        return line.to_owned();
    };

    let prefix = line[..start].trim_end();
    let pretty = serde_json::to_string_pretty(
        &serde_json::from_str::<serde_json::Value>(&line[start..end]).unwrap(),
    )
    .unwrap_or_else(|_| line[start..end].to_owned());
    let suffix = &line[end..];

    let joined = if prefix.is_empty() {
        pretty
    } else if prefix.ends_with(' ') {
        format!("{prefix}{pretty}")
    } else {
        format!("{prefix} {pretty}")
    };
    format!("{joined}{suffix}")
}

fn set_report(
    ui: &MainWindow,
    title: impl Into<slint::SharedString>,
    report: String,
    status: impl Into<slint::SharedString>,
    level: i32,
    toaster: &Toaster,
) {
    ui.set_report_title(title.into());
    let report = format_report_body("report", &report);
    ui.set_report(report.into());
    ui.set_report_available(true);
    set_status(ui, toaster, status, level);
}

fn set_status(
    _ui: &MainWindow,
    toaster: &Toaster,
    status: impl Into<slint::SharedString>,
    level: i32,
) {
    let level = match level {
        1 => ToastLevel::Warning,
        2 => ToastLevel::Error,
        _ => ToastLevel::Success,
    };
    toaster.show(status.into().as_str().to_owned(), level);
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

fn open_output_gui(ui: &MainWindow, folder: bool, toaster: &Toaster) {
    let path = output_path(ui);
    if path.is_empty() {
        return;
    }
    let path = PathBuf::from(path);
    let target = if folder {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path
    };
    if let Err(error) = open_system_path(&target) {
        set_status(ui, toaster, format!("Ouverture impossible : {error}"), 2);
    }
}

fn open_system_path(path: &Path) -> Result<()> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("start").arg("").arg(path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    command
        .spawn()
        .context("impossible de lancer l'outil d'ouverture système")?;
    Ok(())
}

fn ensure_pdf_extension(path: PathBuf) -> PathBuf {
    let is_pdf = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if is_pdf {
        path
    } else {
        let mut value = path.into_os_string();
        value.push(".pdf");
        PathBuf::from(value)
    }
}

fn format_bytes(size: u64) -> String {
    match size {
        0 => "taille non disponible".to_owned(),
        1..=1023 => format!("{} octets", size),
        1024..=1_048_575 => format!("{:.1} Ko", size as f64 / 1024.0),
        _ => format!("{:.1} Mo", size as f64 / 1_048_576.0),
    }
}

fn warning_count_for_crop(report: &CropReport, validation: &PdfXValidationReport) -> usize {
    let mut count = 0;
    let fallback = report.source_characteristics.color.fallback_note();
    if fallback.contains("Repli") && !fallback.contains("Aucun repli couleur") {
        count += 1;
    }
    if report.source_characteristics.color.mode() == ColorMode::Unknown {
        count += 1;
    }
    let issue_count = validation
        .issues
        .iter()
        .filter(|issue| issue.severity >= IssueSeverity::Warning)
        .count();
    count + issue_count
}

fn format_pt(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{}", value.round() as i64)
    } else {
        format!("{:.2}", value)
    }
}

#[cfg(test)]
mod report_format_tests {
    use super::*;
    use lopdf::Document;
    use trimbox_core::boxes::Rect;
    use trimbox_core::{CropReport, IssueSeverity, PdfXIssue, PdfXValidationReport};

    #[test]
    fn blank_lines_are_normalized() {
        assert_eq!(
            format_report_body("report", "alpha\n\n\nbeta"),
            "alpha\n\nbeta"
        );
    }

    #[test]
    fn plain_report_text_is_preserved() {
        assert_eq!(format_report_body("report", "bonjour"), "bonjour");
    }

    #[test]
    fn json_values_are_pretty_printed() {
        assert_eq!(
            format_json_value(r#"{"b":2,"a":1}"#),
            "{\n  \"a\": 1,\n  \"b\": 2\n}"
        );
    }

    #[test]
    fn invalid_json_falls_back_to_input() {
        assert_eq!(format_json_value("not json"), "not json");
    }

    #[test]
    fn crop_summary_stays_concise_and_essential() {
        let mut report = CropReport::new("source.pdf", "result.pdf", 85.0, 55.0);
        report.add_page(
            1,
            Rect {
                llx: 0.0,
                lly: 0.0,
                urx: 595.0,
                ury: 842.0,
            },
            Rect {
                llx: 0.0,
                lly: 0.0,
                urx: trimbox_core::mm_to_pt(85.0),
                ury: trimbox_core::mm_to_pt(55.0),
            },
        );
        report.finalize();

        let mut validation = PdfXValidationReport::new("source.pdf", &Document::new());
        validation.push(PdfXIssue::new(
            IssueSeverity::Warning,
            "TEST",
            "avertissement de test",
        ));

        let summary = format_crop_summary(&report, &validation, 42000);
        assert!(summary.contains("Fichier source : source.pdf"));
        assert!(summary.contains("Fichier généré : result.pdf"));
        assert!(summary.contains("Résultat global : SUCCES"));
        assert!(summary.contains("Pages : 1 page"));
        assert!(summary.contains("Couleur source :"));
        assert!(summary.contains("Taille du fichier produit : 41.0 Ko"));
        assert!(!summary.contains("Métadonnées"));
        assert!(!summary.contains("Page crop"));
        assert!(summary.contains("Avertissements :"));
        assert!(summary.contains("[AVERTISSEMENT] TEST : avertissement de test"));
    }

    #[test]
    fn crop_summary_can_report_no_warnings() {
        let mut report = CropReport::new("source.pdf", "result.pdf", 85.0, 55.0);
        report.source_characteristics.color.cmyk_present = true;
        report.source_characteristics.color.icc_profile_count = 1;
        report
            .source_characteristics
            .color
            .output_intent_profile_count = 1;
        report.add_page(
            1,
            Rect {
                llx: 0.0,
                lly: 0.0,
                urx: 595.0,
                ury: 842.0,
            },
            Rect {
                llx: 0.0,
                lly: 0.0,
                urx: trimbox_core::mm_to_pt(85.0),
                ury: trimbox_core::mm_to_pt(55.0),
            },
        );
        report.finalize();

        let validation = PdfXValidationReport::new("source.pdf", &Document::new());
        let summary = format_crop_summary(&report, &validation, 2048);
        assert!(summary.contains("Avertissements : aucun."));
    }

    #[test]
    fn output_path_receives_pdf_extension_when_missing() {
        assert_eq!(
            ensure_pdf_extension(PathBuf::from("sortie")),
            PathBuf::from("sortie.pdf")
        );
        assert_eq!(
            ensure_pdf_extension(PathBuf::from("sortie.TXT")),
            PathBuf::from("sortie.TXT.pdf")
        );
        assert_eq!(
            ensure_pdf_extension(PathBuf::from("sortie.pdf")),
            PathBuf::from("sortie.pdf")
        );
        assert_eq!(
            ensure_pdf_extension(PathBuf::from("sortie.PDF")),
            PathBuf::from("sortie.PDF")
        );
    }

    #[test]
    fn format_aliases_match_summary() {
        let mut report = CropReport::new("source.pdf", "result.pdf", 85.0, 55.0);
        report.finalize();
        let validation = PdfXValidationReport::new("source.pdf", &Document::new());
        let size = 1024;
        assert_eq!(
            format_crop_summary(&report, &validation, size),
            format_crop_report(&report, &validation, size)
        );
        assert_eq!(
            format_crop_summary(&report, &validation, size),
            format_crop_result(&report, &validation, size)
        );
    }
}

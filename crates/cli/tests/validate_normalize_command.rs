use std::process::Command;

use lopdf::{Dictionary, Object};
use tempfile::tempdir;
use trimbox_core::pdf;

fn build_pdf(path: &std::path::Path) {
    let mut doc = lopdf::Document::new();
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();

    let mut page = Dictionary::new();
    page.set("Type", Object::Name(b"Page".to_vec()));
    page.set("Parent", Object::Reference(pages_id));
    page.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(595.0),
            Object::Real(842.0),
        ]),
    );
    doc.objects.insert(page_id, Object::Dictionary(page));

    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name(b"Pages".to_vec()));
    pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages.set("Count", Object::Integer(1));
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));
    doc.trailer.set("Root", catalog_id);

    let mut out = Vec::new();
    doc.save_to(&mut out).expect("save");
    std::fs::write(path, out).expect("write input");
}

#[test]
fn validate_generic_pdf_reports_missing_output_intent() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    build_pdf(&input);

    let output = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args(["pdf", "validate", input.to_str().unwrap()])
        .output()
        .expect("run");

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PDFX_MISSING_OUTPUTINTENTS"));
}

#[test]
fn normalize_generic_pdf_creates_output() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    build_pdf(&input);

    let result = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args([
            "pdf",
            "normalize",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    assert!(result.status.success());
    assert!(output.is_file());
    let bytes = std::fs::read(&output).expect("read output");
    let document = pdf::load_document(&bytes).expect("load output");
    assert!(pdf::page_count(&document) != 0);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("OutputIntent PDF/X-4 minimal ajoute"));
}

#[test]
fn validate_normalized_pdf_succeeds() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    build_pdf(&input);

    let normalize = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args([
            "pdf",
            "normalize",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
        ])
        .output()
        .expect("normalize");
    assert!(normalize.status.success());

    let validate = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args(["pdf", "validate", output.to_str().unwrap()])
        .output()
        .expect("validate");
    assert!(validate.status.success());
    let stdout = String::from_utf8_lossy(&validate.stdout);
    assert!(!stdout.contains("PDFX_MISSING_OUTPUTINTENTS"));
}

#[test]
fn normalize_rejects_same_input_output() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    build_pdf(&input);

    let result = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args([
            "pdf",
            "normalize",
            input.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .output()
        .expect("run");

    assert!(!result.status.success());
    assert!(!String::from_utf8_lossy(&result.stderr).is_empty());
}

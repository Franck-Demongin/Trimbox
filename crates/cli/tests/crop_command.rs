use std::process::Command;

use lopdf::{Dictionary, Object};
use tempfile::tempdir;
use trimbox_core::boxes::read_page_boxes;
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
fn crop_command_creates_output() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    build_pdf(&input);

    let status = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args([
            "pdf",
            "crop",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "-w",
            "10",
            "-H",
            "15",
        ])
        .status()
        .expect("run");
    assert!(status.success());

    let bytes = std::fs::read(&output).expect("read output");
    let doc = pdf::load_document(&bytes).expect("load output");
    let boxes = read_page_boxes(&doc).expect("boxes");
    let media = boxes[0].media.expect("media");
    let expected_w = trimbox_core::mm_to_pt(10.0);
    let expected_h = trimbox_core::mm_to_pt(15.0);
    assert!((media.width() - expected_w).abs() < 0.5);
    assert!((media.height() - expected_h).abs() < 0.5);
}

#[test]
fn crop_command_rejects_same_input_output() {
    let dir = tempdir().expect("tempdir");
    let input = dir.path().join("input.pdf");
    build_pdf(&input);

    let status = Command::new(env!("CARGO_BIN_EXE_trimbox"))
        .args([
            "pdf",
            "crop",
            input.to_str().unwrap(),
            input.to_str().unwrap(),
            "-w",
            "10",
            "-H",
            "15",
        ])
        .status()
        .expect("run");
    assert!(!status.success());
}

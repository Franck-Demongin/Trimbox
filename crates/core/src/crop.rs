use lopdf::Document;

use crate::boxes::{self, Rect};
use crate::error::Error;
use crate::report::CropReport;
use crate::source_characteristics::analyze_source;

pub fn crop_document(
    document: &mut Document,
    width_cm: f64,
    height_cm: f64,
    source: &str,
    output: &str,
) -> Result<CropReport, Error> {
    if !width_cm.is_finite() || width_cm <= 0.0 {
        return Err(Error::invalid_dimension(format!(
            "largeur invalide : {width_cm}"
        )));
    }
    if !height_cm.is_finite() || height_cm <= 0.0 {
        return Err(Error::invalid_dimension(format!(
            "hauteur invalide : {height_cm}"
        )));
    }

    let requested_width_pt = boxes::cm_to_pt(width_cm);
    let requested_height_pt = boxes::cm_to_pt(height_cm);

    let mut report = CropReport::new(source, output, width_cm, height_cm);
    report.source_characteristics = analyze_source(document);
    report.color_note = report.source_characteristics.color_note();
    let pages = document.get_pages();
    let mut ordered: Vec<(u32, lopdf::ObjectId)> = pages.into_iter().collect();
    ordered.sort_by_key(|(number, _)| *number);

    if ordered.is_empty() {
        return Err(Error::invalid("document PDF sans page"));
    }

    for (page_number, page_id) in &ordered {
        let media_box = boxes::page_media_box(document, *page_id)?
            .ok_or(Error::MissingMediaBox { page: *page_number })?;

        let page_dict = document
            .get_object_mut(*page_id)
            .map_err(|e| Error::Pdf(e.to_string()))?
            .as_dict_mut()
            .map_err(|e| Error::Pdf(e.to_string()))?;

        let src_width = media_box.width();
        let src_height = media_box.height();

        if requested_width_pt > src_width + 0.01 || requested_height_pt > src_height + 0.01 {
            return Err(Error::CropDoesNotFit {
                page: *page_number,
                width: requested_width_pt,
                height: requested_height_pt,
                available_width: src_width,
                available_height: src_height,
            });
        }

        let offset_x = (src_width - requested_width_pt) / 2.0 + media_box.llx;
        let offset_y = (src_height - requested_height_pt) / 2.0 + media_box.lly;
        let after = Rect {
            llx: offset_x,
            lly: offset_y,
            urx: offset_x + requested_width_pt,
            ury: offset_y + requested_height_pt,
        };

        page_dict.set(
            "MediaBox",
            lopdf::Object::Array(
                after
                    .to_array()
                    .into_iter()
                    .map(lopdf::Object::Real)
                    .collect(),
            ),
        );
        page_dict.set(
            "CropBox",
            lopdf::Object::Array(
                after
                    .to_array()
                    .into_iter()
                    .map(lopdf::Object::Real)
                    .collect(),
            ),
        );

        if page_dict.has(b"TrimBox") {
            page_dict.set(
                "TrimBox",
                lopdf::Object::Array(
                    after
                        .to_array()
                        .into_iter()
                        .map(lopdf::Object::Real)
                        .collect(),
                ),
            );
        }
        if page_dict.has(b"BleedBox") {
            page_dict.set(
                "BleedBox",
                lopdf::Object::Array(
                    after
                        .to_array()
                        .into_iter()
                        .map(lopdf::Object::Real)
                        .collect(),
                ),
            );
        }

        report.add_page(*page_number, media_box, after);
    }

    // Ayant ajouté toutes les pages, finalize confirme le compte.
    report.finalize();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_page_boxes;
    use lopdf::{Dictionary, Object};

    fn build_pdf(width: f32, height: f32) -> Document {
        let mut doc = Document::new();
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let content_id = doc.new_object_id();

        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );
        page.set("Contents", Object::Reference(content_id));
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
        doc
    }

    #[test]
    fn crops_single_page() {
        let mut doc = build_pdf(595.0, 842.0);
        let report = crop_document(&mut doc, 10.0, 10.0, "in.pdf", "out.pdf").expect("crop");
        assert_eq!(report.page_count, 1);
        let boxes = read_page_boxes(&doc).expect("boxes");
        let media = boxes[0].media.expect("media");
        let expected_w = boxes::cm_to_pt(10.0);
        assert!((media.width() - expected_w).abs() < 0.5);
    }

    #[test]
    fn rejects_too_large_crop() {
        let mut doc = build_pdf(100.0, 100.0);
        let result = crop_document(&mut doc, 10.0, 10.0, "in.pdf", "out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_zero_width() {
        let mut doc = build_pdf(595.0, 842.0);
        let result = crop_document(&mut doc, 0.0, 10.0, "in.pdf", "out.pdf");
        assert!(result.is_err());
    }
}

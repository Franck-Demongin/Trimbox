use lopdf::Document;

use crate::boxes::{pt_to_mm, read_page_boxes};
use crate::error::Error;

/// Charge un document PDF depuis un tampon en mémoire.
pub fn load_document(bytes: &[u8]) -> Result<Document, Error> {
    Document::load_mem(bytes).map_err(|err| Error::Pdf(err.to_string()))
}

/// Sérialise un document PDF vers un tampon en mémoire.
pub fn save_document(document: &mut Document) -> Result<Vec<u8>, Error> {
    let mut output = Vec::new();
    document
        .save_to(&mut output)
        .map_err(|err| Error::Pdf(err.to_string()))?;
    Ok(output)
}

/// Crée un document PDF minimal destiné à être enrichi selon la norme PDF/X-4.
pub fn blank_pdf_x4() -> Result<Vec<u8>, Error> {
    let mut document = Document::new();
    save_document(&mut document)
}

/// Nombre de pages du document.
pub fn page_count(document: &Document) -> usize {
    document.get_pages().len()
}

/// Taille de la première page en millimètres, avec une indication si au moins une
/// autre page a une taille différente.
pub fn source_page_size(document: &Document) -> Result<(f64, f64, bool), Error> {
    let pages = read_page_boxes(document)?;
    let first = pages
        .first()
        .ok_or_else(|| Error::Pdf("document sans pages".to_owned()))?;
    let first_media = first
        .media
        .ok_or(Error::MissingMediaBox { page: first.page })?;

    let width_mm = pt_to_mm(first_media.width());
    let height_mm = pt_to_mm(first_media.height());

    let has_other_sizes = if pages.len() <= 1 {
        false
    } else {
        let tolerance = 0.1;
        pages.iter().skip(1).any(|page| {
            page.media.as_ref().is_some_and(|media| {
                (pt_to_mm(media.width()) - width_mm).abs() > tolerance
                    || (pt_to_mm(media.height()) - height_mm).abs() > tolerance
            })
        })
    };

    Ok((width_mm, height_mm, has_other_sizes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::mm_to_pt;
    use lopdf::{Dictionary, Object};

    fn build_pdf(specs: &[(f64, f64)]) -> lopdf::Document {
        let mut document = lopdf::Document::new();
        let pages_id = document.new_object_id();
        let mut kids = Vec::new();

        for &(width, height) in specs {
            let page_id = document.new_object_id();
            let mut page = Dictionary::new();
            page.set("Type", Object::Name(b"Page".to_vec()));
            page.set("Parent", Object::Reference(pages_id));
            page.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Real(mm_to_pt(width) as f32),
                    Object::Real(mm_to_pt(height) as f32),
                ]),
            );
            document.objects.insert(page_id, Object::Dictionary(page));
            kids.push(Object::Reference(page_id));
        }

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(kids));
        pages.set("Count", Object::Integer(specs.len() as i64));
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = document.add_object(Object::Dictionary(catalog));
        document.trailer.set("Root", catalog_id);

        document
    }

    fn empty_pages_pdf() -> lopdf::Document {
        let mut document = lopdf::Document::new();
        let pages_id = document.new_object_id();
        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(Vec::new()));
        pages.set("Count", Object::Integer(0));
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = document.add_object(Object::Dictionary(catalog));
        document.trailer.set("Root", catalog_id);

        document
    }

    #[test]
    fn source_page_size_single_page() {
        let document = build_pdf(&[(100.0, 150.0)]);
        let (width, height, has_other_sizes) =
            source_page_size(&document).expect("taille disponible");

        assert!((width - 100.0).abs() < 0.01);
        assert!((height - 150.0).abs() < 0.01);
        assert!(!has_other_sizes);
    }

    #[test]
    fn source_page_size_identical_pages() {
        let document = build_pdf(&[(100.0, 150.0), (100.0, 150.0)]);
        let (_width, _height, has_other_sizes) =
            source_page_size(&document).expect("taille disponible");

        assert!(!has_other_sizes);
    }

    #[test]
    fn source_page_size_mixed_pages() {
        let document = build_pdf(&[(100.0, 150.0), (210.0, 297.0)]);
        let (width, height, has_other_sizes) =
            source_page_size(&document).expect("taille disponible");

        assert!((width - 100.0).abs() < 0.01);
        assert!((height - 150.0).abs() < 0.01);
        assert!(has_other_sizes);
    }

    #[test]
    fn source_page_size_without_media_box() {
        let mut document = lopdf::Document::new();
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();
        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        document.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = document.add_object(Object::Dictionary(catalog));
        document.trailer.set("Root", catalog_id);

        let result = source_page_size(&document);
        assert!(result.is_err());
    }

    #[test]
    fn source_page_size_without_pages() {
        let document = empty_pages_pdf();
        let result = source_page_size(&document);
        assert!(result.is_err());
    }
}

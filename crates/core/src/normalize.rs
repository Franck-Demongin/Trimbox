use crate::boxes::{Rect, page_media_box};
use crate::error::Error;
use crate::validation::{PdfXValidationReport, validate_pdf_x4};
use lopdf::Document;

const TOL: f64 = 0.5;

#[derive(Debug, Clone)]
pub struct PdfXNormalizationReport {
    pub source: String,
    pub output: String,
    pub blocked: Option<String>,
    pub version_before: String,
    pub version_after: String,
    pub changes: Vec<String>,
    pub before: PdfXValidationReport,
    pub after: PdfXValidationReport,
}

impl PdfXNormalizationReport {
    pub fn new(source: impl Into<String>, output: impl Into<String>, document: &Document) -> Self {
        let before = validate_pdf_x4(document, &source.into());
        Self {
            source: before.source.clone(),
            output: output.into(),
            blocked: None,
            version_before: document.version.clone(),
            version_after: document.version.clone(),
            changes: Vec::new(),
            before,
            after: validate_pdf_x4(document, ""),
        }
    }
}

pub fn normalize_pdf_x4(
    document: &mut Document,
    source: &str,
    output: &str,
) -> Result<PdfXNormalizationReport, Error> {
    let mut report = PdfXNormalizationReport::new(source, output, document);
    report.source = source.to_string();
    report.output = output.to_string();
    report.before = validate_pdf_x4(document, source);

    if document.trailer.get(b"Encrypt").is_ok() {
        report.blocked =
            Some("document chiffre non pris en charge pour la normalisation".to_string());
        report.after = report.before.clone();
        return Ok(report);
    }

    let root_id = match document
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|object| object.as_reference().ok())
    {
        Some(id) => id,
        None => {
            report.blocked = Some("catalogue PDF absent".to_string());
            report.after = report.before.clone();
            return Ok(report);
        }
    };

    if version_less_than(&document.version, "1.6") {
        document.version = "1.6".to_string();
        report.changes.push("Version PDF portee a 1.6".to_string());
    }
    report.version_after = document.version.clone();

    // Normalise les boites de page.
    let page_entries: Vec<(u32, lopdf::ObjectId)> = document.get_pages().into_iter().collect();
    for (page_number, page_id) in page_entries {
        let media = match page_media_box(document, page_id) {
            Ok(Some(rect)) => rect,
            _ => continue,
        };

        let page_dict = match document
            .get_object_mut(page_id)
            .and_then(|object| object.as_dict_mut())
        {
            Ok(dict) => dict,
            Err(_) => continue,
        };

        let mut page_changed = false;
        for key in [b"TrimBox" as &[u8], b"BleedBox", b"CropBox", b"ArtBox"] {
            let name = std::str::from_utf8(key).unwrap_or("Box");
            if page_dict.has(key)
                && let Some(rect) = object_to_rect(page_dict.get(key).ok())
                && !contained(&media, &rect)
            {
                page_dict.set(name, rect_array(&media));
                report.changes.push(format!(
                    "page {} : {} ramene dans MediaBox",
                    page_number, name
                ));
                page_changed = true;
            }
        }

        if !page_dict.has(b"TrimBox") && !page_dict.has(b"ArtBox") {
            page_dict.set("TrimBox", rect_array(&media));
            report
                .changes
                .push(format!("page {} : TrimBox ajoute", page_number));
            page_changed = true;
        }

        let _ = page_changed;
    }

    // Normalise les OutputIntents.
    normalize_output_intents(document, root_id, &mut report.changes);

    report.after = validate_pdf_x4(document, output);
    Ok(report)
}

fn normalize_output_intents(
    document: &mut Document,
    root_id: lopdf::ObjectId,
    changes: &mut Vec<String>,
) {
    let current = document
        .get_object(root_id)
        .ok()
        .and_then(|object| object.as_dict().ok())
        .and_then(|catalog| catalog.get(b"OutputIntents").ok().cloned());

    let mut intent_ids = Vec::new();
    match current {
        Some(lopdf::Object::Reference(id)) => {
            let referenced = document.get_object(id).ok().cloned();
            if let Some(lopdf::Object::Array(items)) = referenced {
                let item_clones = items;
                for item in &item_clones {
                    match item {
                        lopdf::Object::Reference(item_id) => intent_ids.push(*item_id),
                        lopdf::Object::Dictionary(_) => {
                            let cloned = item.clone();
                            let new_id = document.add_object(cloned);
                            normalize_intent_object(document, new_id, changes);
                            intent_ids.push(new_id);
                        }
                        _ => {}
                    }
                }
                for id in &intent_ids {
                    normalize_intent_object(document, *id, changes);
                }
                if intent_ids.is_empty() {
                    let new_id = create_output_intent(document, changes);
                    intent_ids.push(new_id);
                }
            } else if let Some(lopdf::Object::Dictionary(dict)) = referenced {
                let cloned = lopdf::Object::Dictionary(dict.clone());
                let new_id = document.add_object(cloned);
                normalize_intent_object(document, new_id, changes);
                intent_ids.push(new_id);
            } else {
                let new_id = create_output_intent(document, changes);
                intent_ids.push(new_id);
            }
        }
        Some(lopdf::Object::Array(items)) => {
            for item in &items {
                match item {
                    lopdf::Object::Reference(id) => intent_ids.push(*id),
                    lopdf::Object::Dictionary(_) => {
                        let cloned = item.clone();
                        let new_id = document.add_object(cloned);
                        normalize_intent_object(document, new_id, changes);
                        intent_ids.push(new_id);
                    }
                    _ => {}
                }
            }
            for id in &intent_ids {
                normalize_intent_object(document, *id, changes);
            }
            if intent_ids.is_empty() {
                let new_id = create_output_intent(document, changes);
                intent_ids.push(new_id);
            }
        }
        Some(lopdf::Object::Dictionary(dict)) => {
            let new_id = document.add_object(lopdf::Object::Dictionary(dict.clone()));
            normalize_intent_object(document, new_id, changes);
            intent_ids.push(new_id);
        }
        _ => {
            let new_id = create_output_intent(document, changes);
            intent_ids.push(new_id);
        }
    }

    if !intent_ids.is_empty() {
        let array = lopdf::Object::Array(
            intent_ids
                .into_iter()
                .map(lopdf::Object::Reference)
                .collect(),
        );
        if let Ok(catalog) = document
            .get_object_mut(root_id)
            .and_then(|object| object.as_dict_mut())
        {
            catalog.set("OutputIntents", array);
        }
    }
}

fn create_output_intent(document: &mut Document, changes: &mut Vec<String>) -> lopdf::ObjectId {
    let mut intent = lopdf::Dictionary::new();
    intent.set("Type", lopdf::Object::Name(b"OutputIntent".to_vec()));
    intent.set("S", lopdf::Object::Name(b"GTS_PDFX".to_vec()));
    intent.set(
        "OutputConditionIdentifier",
        lopdf::Object::String(b"Custom".to_vec(), lopdf::StringFormat::Literal),
    );
    intent.set(
        "RegistryName",
        lopdf::Object::String(
            b"http://www.color.org".to_vec(),
            lopdf::StringFormat::Literal,
        ),
    );
    intent.set(
        "Info",
        lopdf::Object::String(b"Trimbox PDF/X-4".to_vec(), lopdf::StringFormat::Literal),
    );
    let id = document.add_object(lopdf::Object::Dictionary(intent));
    changes.push("OutputIntent PDF/X-4 minimal ajoute".to_string());
    id
}

fn normalize_intent_object(
    document: &mut Document,
    id: lopdf::ObjectId,
    changes: &mut Vec<String>,
) {
    let dict = match document
        .get_object_mut(id)
        .and_then(|object| object.as_dict_mut())
    {
        Ok(dict) => dict,
        Err(_) => return,
    };

    let mut changed = false;
    if match dict.get(b"S") {
        Ok(lopdf::Object::Name(name)) => name != b"GTS_PDFX",
        _ => true,
    } {
        dict.set("S", lopdf::Object::Name(b"GTS_PDFX".to_vec()));
        changed = true;
    }

    if !dict.has(b"Type") {
        dict.set("Type", lopdf::Object::Name(b"OutputIntent".to_vec()));
        changed = true;
    }

    match dict.get(b"OutputConditionIdentifier") {
        Ok(object) => {
            if object_to_string(object)
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                dict.set(
                    "OutputConditionIdentifier",
                    lopdf::Object::String(b"Custom".to_vec(), lopdf::StringFormat::Literal),
                );
                changed = true;
            }
        }
        Err(_) => {
            if dict.get(b"DestOutputProfile").is_err() {
                dict.set(
                    "OutputConditionIdentifier",
                    lopdf::Object::String(b"Custom".to_vec(), lopdf::StringFormat::Literal),
                );
                changed = true;
            }
        }
    }

    if dict.get(b"RegistryName").is_err() && dict.get(b"OutputConditionIdentifier").is_ok() {
        dict.set(
            "RegistryName",
            lopdf::Object::String(
                b"http://www.color.org".to_vec(),
                lopdf::StringFormat::Literal,
            ),
        );
        changed = true;
    }

    if changed {
        changes.push("OutputIntent normalise".to_string());
    }
}

fn contained(outer: &Rect, inner: &Rect) -> bool {
    inner.llx >= outer.llx - TOL
        && inner.lly >= outer.lly - TOL
        && inner.urx <= outer.urx + TOL
        && inner.ury <= outer.ury + TOL
}

fn object_to_rect(object: Option<&lopdf::Object>) -> Option<Rect> {
    let object = object?;
    if let lopdf::Object::Array(items) = object
        && items.len() == 4
    {
        let mut values = [0.0f32; 4];
        for (index, item) in items.iter().enumerate() {
            match item {
                lopdf::Object::Integer(value) => values[index] = *value as f32,
                lopdf::Object::Real(value) => values[index] = *value,
                _ => return None,
            }
        }
        return Rect::from_array(&values);
    }
    None
}

fn rect_array(rect: &Rect) -> lopdf::Object {
    lopdf::Object::Array(vec![
        lopdf::Object::Real(rect.llx as f32),
        lopdf::Object::Real(rect.lly as f32),
        lopdf::Object::Real(rect.urx as f32),
        lopdf::Object::Real(rect.ury as f32),
    ])
}

fn object_to_string(object: &lopdf::Object) -> Option<String> {
    match object {
        lopdf::Object::String(bytes, _) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

fn version_less_than(current: &str, target: &str) -> bool {
    let parse = |value: &str| {
        let mut parts = value.split('.');
        let major = parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0);
        let minor = parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .unwrap_or(0);
        (major, minor)
    };
    parse(current) < parse(target)
}

impl std::fmt::Display for PdfXNormalizationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Trimbox - rapport de normalisation PDF/X-4")?;
        writeln!(f, "Fichier source : {}", self.source)?;
        writeln!(f, "Fichier genere : {}", self.output)?;
        writeln!(
            f,
            "Version PDF : {} -> {}",
            self.version_before, self.version_after
        )?;
        if let Some(blocked) = &self.blocked {
            writeln!(f, "Bloque : {}", blocked)?;
        }
        writeln!(f, "Ecarts avant :")?;
        if self.before.issues.is_empty() {
            writeln!(f, "  aucun")?;
        } else {
            for issue in &self.before.issues {
                writeln!(
                    f,
                    "  [{}] {} - {}",
                    issue.severity, issue.code, issue.message
                )?;
            }
        }
        writeln!(f, "Modifications appliquees :")?;
        if self.changes.is_empty() {
            writeln!(f, "  aucune")?;
        } else {
            for change in &self.changes {
                writeln!(f, "  - {}", change)?;
            }
        }
        writeln!(f, "Ecarts apres :")?;
        if self.after.issues.is_empty() {
            writeln!(f, "  aucun")?;
        } else {
            for issue in &self.after.issues {
                writeln!(
                    f,
                    "  [{}] {} - {}",
                    issue.severity, issue.code, issue.message
                )?;
            }
        }
        writeln!(
            f,
            "Profils ICC embarques apres : {}",
            self.after.embedded_icc_count
        )?;
        writeln!(
            f,
            "Resultat global : {}",
            if self.after.is_conformant() {
                "SUCCES"
            } else {
                "AVERTISSEMENT"
            }
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object};

    fn build_generic_pdf() -> Document {
        let mut doc = Document::new();
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set(
            "MediaBox",
            Object::Array(vec![
                0.into(),
                0.into(),
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
        doc
    }

    #[test]
    fn normalizes_generic_pdf() {
        let mut doc = build_generic_pdf();
        let report = normalize_pdf_x4(&mut doc, "in.pdf", "out.pdf").expect("normalize");
        assert!(!report.changes.is_empty());
        assert!(report.after.output_intent_count >= 1);
    }

    #[test]
    fn normalizes_box_outside_media() {
        let mut doc = build_generic_pdf();
        let page_id = *doc.get_pages().values().next().unwrap();
        if let Ok(object) = doc.get_object_mut(page_id)
            && let Ok(dict) = object.as_dict_mut()
        {
            dict.set(
                "TrimBox",
                Object::Array(vec![
                    Object::Real(-10.0),
                    Object::Real(-10.0),
                    Object::Real(1000.0),
                    Object::Real(1000.0),
                ]),
            );
        }
        let report = normalize_pdf_x4(&mut doc, "in.pdf", "out.pdf").expect("normalize");
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.contains("ramene"))
        );
    }
}

use crate::boxes::{Rect, read_page_boxes};
use crate::source_characteristics::{SourceCharacteristics, analyze_source};
use lopdf::Document;

const TOL: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

impl IssueSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "AVERTISSEMENT",
            Self::Error => "ERREUR",
        }
    }

    pub fn exit_code(self) -> i32 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Error => 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfXIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
}

impl PdfXIssue {
    pub fn new(
        severity: IssueSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfXValidationReport {
    pub source: String,
    pub version: String,
    pub page_count: usize,
    pub output_intent_count: usize,
    pub embedded_icc_count: usize,
    pub source_characteristics: SourceCharacteristics,
    pub issues: Vec<PdfXIssue>,
}

impl PdfXValidationReport {
    pub fn new(source: impl Into<String>, document: &Document) -> Self {
        Self {
            source: source.into(),
            version: document.version.clone(),
            page_count: document.get_pages().len(),
            output_intent_count: 0,
            embedded_icc_count: 0,
            source_characteristics: analyze_source(document),
            issues: Vec::new(),
        }
    }

    pub fn push(&mut self, issue: PdfXIssue) {
        self.issues.push(issue);
    }

    pub fn worst_severity(&self) -> Option<IssueSeverity> {
        self.issues.iter().map(|issue| issue.severity).max()
    }

    pub fn is_conformant(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    pub fn exit_code(&self) -> i32 {
        self.worst_severity()
            .map(IssueSeverity::exit_code)
            .unwrap_or(0)
    }
}

pub fn validate_pdf_x4(document: &Document, source: &str) -> PdfXValidationReport {
    let mut report = PdfXValidationReport::new(source, document);

    if document.trailer.get(b"Encrypt").is_ok() {
        report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_ENCRYPTED",
            "le document est chiffre; PDF/X-4 interdit le chiffrement".to_string(),
        ));
    }

    if version_less_than(&document.version, "1.6") {
        report.push(PdfXIssue::new(
            IssueSeverity::Warning,
            "PDFX_VERSION",
            format!("version PDF {} inferieure a 1.6", document.version),
        ));
    }

    let catalog = root_catalog(document);
    let Some(catalog) = catalog else {
        report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_MISSING_CATALOG",
            "catalogue PDF absent".to_string(),
        ));
        validate_page_boxes(document, &mut report);
        return report;
    };

    if catalog.get(b"Pages").is_err() {
        report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_MISSING_PAGES",
            "noeud Pages absent du catalogue".to_string(),
        ));
    }

    inspect_output_intents(document, catalog, &mut report);

    validate_page_boxes(document, &mut report);
    validate_page_annotations(document, &mut report);

    if report.page_count == 0 {
        report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_NO_PAGES",
            "le document ne contient aucune page".to_string(),
        ));
    }

    report
}

fn inspect_output_intents(
    document: &Document,
    catalog: &lopdf::Dictionary,
    report: &mut PdfXValidationReport,
) {
    match catalog.get(b"OutputIntents") {
        Ok(lopdf::Object::Array(items)) => {
            if items.is_empty() {
                report.push(PdfXIssue::new(
                    IssueSeverity::Error,
                    "PDFX_EMPTY_OUTPUTINTENTS",
                    "aucun OutputIntent declare".to_string(),
                ));
                return;
            }
            report.output_intent_count = items.len();
            for item in items {
                validate_output_intent(document, item, report);
            }
        }
        Ok(lopdf::Object::Reference(id)) => {
            report.output_intent_count = 1;
            if let Ok(object) = document.get_object(*id) {
                validate_output_intent(document, object, report);
            } else {
                report.push(PdfXIssue::new(
                    IssueSeverity::Error,
                    "PDFX_INVALID_OUTPUTINTENTS",
                    "OutputIntent illisible".to_string(),
                ));
            }
        }
        Ok(lopdf::Object::Dictionary(dict)) => {
            report.output_intent_count = 1;
            validate_output_intent_dict(document, dict, report);
        }
        Ok(_) => {
            report.push(PdfXIssue::new(
                IssueSeverity::Error,
                "PDFX_INVALID_OUTPUTINTENTS",
                "OutputIntents n'est pas un tableau ou un dictionnaire".to_string(),
            ));
        }
        Err(_) => {
            report.push(PdfXIssue::new(
                IssueSeverity::Error,
                "PDFX_MISSING_OUTPUTINTENTS",
                "OutputIntents absent du catalogue".to_string(),
            ));
        }
    }
}

fn validate_output_intent(
    document: &Document,
    object: &lopdf::Object,
    report: &mut PdfXValidationReport,
) {
    if let Ok(id) = object.as_reference()
        && let Ok(object) = document.get_object(id)
        && let Ok(dict) = object.as_dict()
    {
        validate_output_intent_dict(document, dict, report);
        return;
    }
    if let Ok(dict) = object.as_dict() {
        validate_output_intent_dict(document, dict, report);
        return;
    }
    report.push(PdfXIssue::new(
        IssueSeverity::Error,
        "PDFX_INVALID_OUTPUTINTENT",
        "OutputIntent illisible".to_string(),
    ));
}

fn validate_output_intent_dict(
    document: &Document,
    dict: &lopdf::Dictionary,
    report: &mut PdfXValidationReport,
) {
    match dict.get(b"S") {
        Ok(lopdf::Object::Name(name)) if name == b"GTS_PDFX" => {}
        _ => report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_OUTPUTINTENT_SUBTYPE",
            "OutputIntent de type GTS_PDFX attendu".to_string(),
        )),
    }

    let has_condition = dict
        .get(b"OutputConditionIdentifier")
        .ok()
        .and_then(object_to_string)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    let has_profile = dict
        .get(b"DestOutputProfile")
        .ok()
        .map(|object| {
            resolve_object(document, object)
                .and_then(|value| value.as_stream().ok())
                .is_some()
        })
        .unwrap_or(false);

    if has_profile {
        report.embedded_icc_count += 1;
    }

    if !has_condition && !has_profile {
        report.push(PdfXIssue::new(
            IssueSeverity::Error,
            "PDFX_MISSING_OUTPUT_CONDITION",
            "OutputConditionIdentifier ou DestOutputProfile manquant".to_string(),
        ));
    }

    if dict.get(b"OutputConditionIdentifier").is_ok() && dict.get(b"RegistryName").is_err() {
        report.push(PdfXIssue::new(
            IssueSeverity::Warning,
            "PDFX_MISSING_REGISTRY",
            "RegistryName absent pour OutputConditionIdentifier".to_string(),
        ));
    }

    if !has_profile {
        report.push(PdfXIssue::new(
            IssueSeverity::Info,
            "PDFX_NO_EMBEDDED_ICC",
            "aucun profil ICC embarque dans cet OutputIntent".to_string(),
        ));
    }
}

fn validate_page_boxes(document: &Document, report: &mut PdfXValidationReport) {
    match read_page_boxes(document) {
        Ok(pages) => {
            for page in pages {
                let Some(media) = page.media else {
                    report.push(PdfXIssue::new(
                        IssueSeverity::Error,
                        "PDFX_MISSING_MEDIABOX",
                        format!("page {} sans MediaBox", page.page),
                    ));
                    continue;
                };

                for (label, rect) in [
                    ("CropBox", page.crop),
                    ("TrimBox", page.trim),
                    ("BleedBox", page.bleed),
                    ("ArtBox", page.art),
                ] {
                    if let Some(inner) = rect
                        && !contained(&media, &inner)
                    {
                        report.push(PdfXIssue::new(
                            IssueSeverity::Error,
                            "PDFX_BOX_OUTSIDE_MEDIA",
                            format!("{} de la page {} depasse MediaBox", label, page.page),
                        ));
                    }
                }

                if page.trim.is_none() && page.art.is_none() {
                    report.push(PdfXIssue::new(
                        IssueSeverity::Warning,
                        "PDFX_MISSING_TRIM_OR_ART",
                        format!("page {} sans TrimBox ni ArtBox", page.page),
                    ));
                }
            }
        }
        Err(error) => {
            report.push(PdfXIssue::new(
                IssueSeverity::Error,
                "PDFX_BOX_READ_ERROR",
                error.to_string(),
            ));
        }
    }
}

fn validate_page_annotations(document: &Document, report: &mut PdfXValidationReport) {
    for (page_number, page_id) in document.get_pages() {
        if let Ok(object) = document.get_object(page_id)
            && let Ok(dict) = object.as_dict()
            && dict.get(b"Annots").is_ok()
        {
            report.push(PdfXIssue::new(
                IssueSeverity::Warning,
                "PDFX_ANNOTATIONS",
                format!("annotations presentes sur la page {}", page_number),
            ));
        }
    }
}

fn contained(outer: &Rect, inner: &Rect) -> bool {
    inner.llx >= outer.llx - TOL
        && inner.lly >= outer.lly - TOL
        && inner.urx <= outer.urx + TOL
        && inner.ury <= outer.ury + TOL
}

fn root_catalog(document: &Document) -> Option<&lopdf::Dictionary> {
    let root = document.trailer.get(b"Root").ok()?;
    if let Ok(id) = root.as_reference() {
        document.get_object(id).ok()?.as_dict().ok()
    } else {
        root.as_dict().ok()
    }
}

fn resolve_object<'a>(
    document: &'a Document,
    object: &'a lopdf::Object,
) -> Option<&'a lopdf::Object> {
    if let Ok(id) = object.as_reference() {
        document.get_object(id).ok()
    } else {
        Some(object)
    }
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

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl std::fmt::Display for PdfXValidationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Trimbox - rapport de validation PDF/X-4")?;
        writeln!(f, "Fichier : {}", self.source)?;
        writeln!(f, "Version PDF : {}", self.version)?;
        writeln!(f, "Pages : {}", self.page_count)?;
        writeln!(f, "OutputIntents : {}", self.output_intent_count)?;
        writeln!(f, "Profils ICC embarques : {}", self.embedded_icc_count)?;
        writeln!(
            f,
            "Couleur source : {}",
            self.source_characteristics.color_note()
        )?;
        writeln!(
            f,
            "Profils couleur source : {}",
            self.source_characteristics.color.icc_profile_count()
        )?;
        writeln!(
            f,
            "Metadonnees source : {}",
            self.source_characteristics.metadata_note()
        )?;
        writeln!(
            f,
            "Conforme PDF/X-4 : {}",
            if self.is_conformant() { "oui" } else { "non" }
        )?;
        if self.issues.is_empty() {
            writeln!(f, "Ecarts : aucun")?;
        } else {
            writeln!(f, "Ecarts :")?;
            for issue in &self.issues {
                writeln!(
                    f,
                    "  [{}] {} - {}",
                    issue.severity, issue.code, issue.message
                )?;
            }
        }
        let code = self.exit_code();
        writeln!(f, "Code de retour : {}", code)?;
        writeln!(
            f,
            "Resultat global : {}",
            if code == 0 {
                "SUCCES"
            } else if code == 1 {
                "AVERTISSEMENT"
            } else {
                "ECHEC"
            }
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod m2_tests {
    use super::*;
    use lopdf::{Dictionary, Object};

    fn build_pdf() -> Document {
        let mut doc = Document::new();
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
        page.set(
            "TrimBox",
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
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc
    }

    fn add_output_intent(doc: &mut Document) -> lopdf::ObjectId {
        let mut intent = Dictionary::new();
        intent.set("Type", Object::Name(b"OutputIntent".to_vec()));
        intent.set("S", Object::Name(b"GTS_PDFX".to_vec()));
        intent.set(
            "OutputConditionIdentifier",
            Object::String(b"Custom".to_vec(), lopdf::StringFormat::Literal),
        );
        intent.set(
            "RegistryName",
            Object::String(
                b"http://www.color.org".to_vec(),
                lopdf::StringFormat::Literal,
            ),
        );

        let catalog_id = match doc.trailer.get(b"Root") {
            Ok(object) => object.as_reference().expect("root reference"),
            Err(_) => panic!("missing root"),
        };
        let intent_id = doc.add_object(Object::Dictionary(intent));
        let Ok(object) = doc.get_object_mut(catalog_id) else {
            panic!("unreadable catalog")
        };
        let Ok(catalog) = object.as_dict_mut() else {
            panic!("invalid catalog")
        };
        catalog.set(
            "OutputIntents",
            Object::Array(vec![Object::Reference(intent_id)]),
        );
        intent_id
    }

    #[test]
    fn missing_output_intent_is_an_error() {
        let doc = build_pdf();
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(!report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_MISSING_OUTPUTINTENTS")
        );
        assert_eq!(report.exit_code(), 2);
    }

    #[test]
    fn wrong_output_intent_subtype_is_an_error() {
        let mut doc = build_pdf();
        let intent_id = add_output_intent(&mut doc);
        let Ok(object) = doc.get_object_mut(intent_id) else {
            panic!("unreadable intent")
        };
        let Ok(intent) = object.as_dict_mut() else {
            panic!("invalid intent")
        };
        intent.set("S", Object::Name(b"GTS_PDFA".to_vec()));
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(!report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_OUTPUTINTENT_SUBTYPE")
        );
    }

    #[test]
    fn missing_registry_name_is_a_warning() {
        let mut doc = build_pdf();
        let intent_id = add_output_intent(&mut doc);
        let Ok(object) = doc.get_object_mut(intent_id) else {
            panic!("unreadable intent")
        };
        let Ok(intent) = object.as_dict_mut() else {
            panic!("invalid intent")
        };
        intent.remove(b"RegistryName");
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_MISSING_REGISTRY")
        );
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn blank_output_condition_is_an_error() {
        let mut doc = build_pdf();
        let intent_id = add_output_intent(&mut doc);
        let Ok(object) = doc.get_object_mut(intent_id) else {
            panic!("unreadable intent")
        };
        let Ok(intent) = object.as_dict_mut() else {
            panic!("invalid intent")
        };
        intent.set(
            "OutputConditionIdentifier",
            Object::String(b" ".to_vec(), lopdf::StringFormat::Literal),
        );
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(!report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_MISSING_OUTPUT_CONDITION")
        );
    }

    #[test]
    fn out_of_bounds_box_is_an_error() {
        let mut doc = build_pdf();
        add_output_intent(&mut doc);
        let page_id = *doc.get_pages().values().next().unwrap();
        let Ok(object) = doc.get_object_mut(page_id) else {
            panic!("unreadable page")
        };
        let Ok(page) = object.as_dict_mut() else {
            panic!("invalid page")
        };
        page.set(
            "TrimBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(1000.0),
                Object::Real(1000.0),
            ]),
        );
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(!report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_BOX_OUTSIDE_MEDIA")
        );
    }

    #[test]
    fn page_without_trim_or_art_reports_a_warning() {
        let mut doc = build_pdf();
        add_output_intent(&mut doc);
        let page_id = *doc.get_pages().values().next().unwrap();
        let Ok(object) = doc.get_object_mut(page_id) else {
            panic!("unreadable page")
        };
        let Ok(page) = object.as_dict_mut() else {
            panic!("invalid page")
        };
        page.remove(b"TrimBox");
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_MISSING_TRIM_OR_ART")
        );
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn annotations_report_a_warning() {
        let mut doc = build_pdf();
        add_output_intent(&mut doc);
        let page_id = *doc.get_pages().values().next().unwrap();
        let Ok(object) = doc.get_object_mut(page_id) else {
            panic!("unreadable page")
        };
        let Ok(page) = object.as_dict_mut() else {
            panic!("invalid page")
        };
        page.set("Annots", Object::Array(Vec::new()));
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_ANNOTATIONS")
        );
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn missing_media_box_is_an_error() {
        let mut doc = build_pdf();
        add_output_intent(&mut doc);
        let page_id = *doc.get_pages().values().next().unwrap();
        let Ok(object) = doc.get_object_mut(page_id) else {
            panic!("unreadable page")
        };
        let Ok(page) = object.as_dict_mut() else {
            panic!("invalid page")
        };
        page.remove(b"MediaBox");
        let report = validate_pdf_x4(&doc, "in.pdf");
        assert!(!report.is_conformant());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "PDFX_MISSING_MEDIABOX")
        );
    }
}

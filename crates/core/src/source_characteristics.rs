use lopdf::{Dictionary, Document};
use std::collections::BTreeSet;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Unknown,
    Gray,
    Rgb,
    Cmyk,
    Mixed,
}

impl ColorMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "inconnu",
            Self::Gray => "GRIS",
            Self::Rgb => "RGB",
            Self::Cmyk => "CMJN",
            Self::Mixed => "mixte",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColorProfileSummary {
    pub cmyk_present: bool,
    pub rgb_present: bool,
    pub gray_present: bool,
    pub mixed: bool,
    pub icc_profile_count: usize,
    pub output_intent_profile_count: usize,
}

impl ColorProfileSummary {
    pub fn mode(&self) -> ColorMode {
        if self.cmyk_present && self.rgb_present {
            ColorMode::Mixed
        } else if self.cmyk_present {
            ColorMode::Cmyk
        } else if self.rgb_present {
            ColorMode::Rgb
        } else if self.gray_present {
            ColorMode::Gray
        } else {
            ColorMode::Unknown
        }
    }

    pub fn color_mode(&self) -> ColorMode {
        self.mode()
    }

    pub fn icc_profile_count(&self) -> usize {
        self.icc_profile_count
    }

    pub fn source_color_note(&self) -> String {
        match self.mode() {
            ColorMode::Cmyk => {
                "Couleur source : CMJN detecte; conservation structurelle des flots et profils ICC."
            }
            ColorMode::Rgb => {
                "Couleur source : RVB detecte; aucune conversion couleur et aucune transcription decorative."
            }
            ColorMode::Gray => {
                "Couleur source : gris detecte; conservation structurelle, aucun embellissement."
            }
            ColorMode::Mixed => {
                "Couleur source mixte detectee; conservation structurelle avec repli explicite recommande."
            }
            ColorMode::Unknown => {
                "Aucune palette couleur detectee; les flots source restent conserves en l'etat."
            }
        }
        .to_string()
    }

    pub fn fallback_note(&self) -> String {
        if self.cmyk_present {
            "Aucun repli couleur necessaire; caracteristiques sources conservees.".to_string()
        } else if self.rgb_present {
            format!(
                "Repli explicite : source RVB{}; aucune conversion couleur n'a ete effectuee.",
                if self.icc_profile_count == 0 {
                    " ou sans profil ICC"
                } else {
                    ""
                }
            )
        } else if self.icc_profile_count == 0 {
            "Repli explicite : absence de profil ICC detectee; la normalisation PDF/X-4 reste a profil minimal."
                .to_string()
        } else {
            "Repli couleur signale explicitement selon les caracteristiques detectees.".to_string()
        }
    }

    pub fn color_fallback_note(&self) -> String {
        self.fallback_note()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadataSummary {
    pub has_info_dictionary: bool,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub xmp_present: bool,
}

impl DocumentMetadataSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn from_strings(
        title: Option<String>,
        author: Option<String>,
        subject: Option<String>,
        keywords: Option<String>,
        creator: Option<String>,
        producer: Option<String>,
        creation_date: Option<String>,
        modification_date: Option<String>,
        xmp_present: bool,
    ) -> Self {
        Self {
            has_info_dictionary: title.is_some()
                || author.is_some()
                || subject.is_some()
                || keywords.is_some()
                || creator.is_some()
                || producer.is_some()
                || creation_date.is_some()
                || modification_date.is_some(),
            title,
            author,
            subject,
            keywords,
            creator,
            producer,
            creation_date,
            modification_date,
            xmp_present,
        }
    }

    pub fn metadata_note(&self) -> String {
        if !self.has_info_dictionary && !self.xmp_present {
            "Aucune metadonnee source detectee; les metadonnees existantes ont conserve leur place si elles etaient deja presentes."
                .to_string()
        } else {
            let mut parts = Vec::new();
            if let Some(value) = &self.title {
                parts.push(format!("Titre : {value}"));
            }
            if let Some(value) = &self.author {
                parts.push(format!("Auteur : {value}"));
            }
            if let Some(value) = &self.producer {
                parts.push(format!("Producteur : {value}"));
            }
            if self.xmp_present {
                parts.push("XMP embarque : present".to_string());
            }
            if parts.is_empty() {
                "Metadonnees source detectees; conservation structurelle sans transcription."
                    .to_string()
            } else {
                format!("Metadonnees source conservees : {}.", parts.join(" ; "))
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceCharacteristics {
    pub color: ColorProfileSummary,
    pub metadata: DocumentMetadataSummary,
}

impl SourceCharacteristics {
    pub fn color_note(&self) -> String {
        self.color.source_color_note()
    }

    pub fn metadata_note(&self) -> String {
        self.metadata.metadata_note()
    }

    pub fn color_fallback_note(&self) -> String {
        self.color.fallback_note()
    }
}

pub fn analyze_color(document: &Document) -> ColorProfileSummary {
    let mut summary = ColorProfileSummary::default();
    let mut seen_profiles = BTreeSet::new();
    let mut output_profiles = BTreeSet::new();
    let mut seen_objects = HashSet::<lopdf::ObjectId>::new();

    for object in document.objects.values() {
        inspect_object(
            document,
            object,
            &mut seen_objects,
            &mut summary,
            &mut seen_profiles,
            &mut output_profiles,
        );

        if let Ok(stream) = object.as_stream() {
            let text = bytes_to_text(&stream.content);
            if contains(&text, "DeviceCMYK") {
                summary.cmyk_present = true;
            }
            if contains(&text, "DeviceRGB") {
                summary.rgb_present = true;
            }
            if contains(&text, "DeviceGray") {
                summary.gray_present = true;
            }
        }
    }

    summary.mixed = summary.cmyk_present && summary.rgb_present;

    summary
}

pub fn read_metadata(document: &Document) -> DocumentMetadataSummary {
    let mut title = None;
    let mut author = None;
    let mut subject = None;
    let mut keywords = None;
    let mut creator = None;
    let mut producer = None;
    let mut creation_date = None;
    let mut modification_date = None;

    if let Some(info) = document_info(document) {
        for (key, value) in info.iter() {
            match key.as_slice() {
                b"Title" => title = pdf_string(value),
                b"Author" => author = pdf_string(value),
                b"Subject" => subject = pdf_string(value),
                b"Keywords" => keywords = pdf_string(value),
                b"Creator" => creator = pdf_string(value),
                b"Producer" => producer = pdf_string(value),
                b"CreationDate" => creation_date = pdf_string(value),
                b"ModDate" => modification_date = pdf_string(value),
                _ => {}
            }
        }
    }

    let xmp_present = document
        .catalog()
        .ok()
        .map(|catalog| catalog.get(b"Metadata").is_ok())
        .unwrap_or(false);

    DocumentMetadataSummary::from_strings(
        title,
        author,
        subject,
        keywords,
        creator,
        producer,
        creation_date,
        modification_date,
        xmp_present,
    )
}

pub fn analyze_source(document: &Document) -> SourceCharacteristics {
    SourceCharacteristics {
        color: analyze_color(document),
        metadata: read_metadata(document),
    }
}

pub fn inspect_source(document: &Document) -> SourceCharacteristics {
    analyze_source(document)
}

pub fn color_profile_summary(document: &Document) -> ColorProfileSummary {
    analyze_color(document)
}

fn document_info(document: &Document) -> Option<Dictionary> {
    let info = document.trailer.get(b"Info").ok()?;
    if let Ok(id) = info.as_reference() {
        document.get_object(id).ok()?.as_dict().cloned().ok()
    } else {
        info.as_dict().cloned().ok()
    }
}

fn pdf_string(value: &lopdf::Object) -> Option<String> {
    match value {
        lopdf::Object::String(bytes, _) => {
            let text = String::from_utf8_lossy(bytes);
            let text = text.trim().trim_matches('\u{0}').trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        lopdf::Object::Name(bytes) => String::from_utf8(bytes.clone()).ok(),
        _ => None,
    }
}

fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

fn bytes_to_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn inspect_object(
    document: &Document,
    object: &lopdf::Object,
    seen_objects: &mut HashSet<lopdf::ObjectId>,
    summary: &mut ColorProfileSummary,
    profile_ids: &mut BTreeSet<lopdf::ObjectId>,
    output_profile_ids: &mut BTreeSet<lopdf::ObjectId>,
) {
    match object {
        lopdf::Object::Reference(id) => {
            if seen_objects.insert(*id)
                && let Ok(referenced) = document.get_object(*id)
            {
                inspect_object(
                    document,
                    referenced,
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                );
            }
        }
        lopdf::Object::Dictionary(dict) => {
            inspect_dictionary(
                document,
                dict,
                seen_objects,
                summary,
                profile_ids,
                output_profile_ids,
            );
        }
        lopdf::Object::Array(items) => {
            for item in items {
                inspect_object(
                    document,
                    item,
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                );
            }
        }
        lopdf::Object::Stream(stream) => {
            inspect_dictionary(
                document,
                &stream.dict,
                seen_objects,
                summary,
                profile_ids,
                output_profile_ids,
            );
        }
        _ => {}
    }
}

fn inspect_dictionary(
    document: &Document,
    dict: &Dictionary,
    seen_objects: &mut HashSet<lopdf::ObjectId>,
    summary: &mut ColorProfileSummary,
    profile_ids: &mut BTreeSet<lopdf::ObjectId>,
    output_profile_ids: &mut BTreeSet<lopdf::ObjectId>,
) {
    if let Ok(color_space) = dict.get(b"ColorSpace") {
        inspect_color_space_value(
            document,
            color_space,
            seen_objects,
            summary,
            profile_ids,
            output_profile_ids,
        );
    }

    if let Ok(name) = dict.get(b"S").and_then(|object| object.as_name())
        && name == b"GTS_PDFX"
        && let Ok(profile) = dict.get(b"DestOutputProfile")
    {
        observe_profile(
            document,
            profile,
            seen_objects,
            summary,
            profile_ids,
            output_profile_ids,
            true,
        );
    }

    for (_, value) in dict.iter() {
        inspect_object(
            document,
            value,
            seen_objects,
            summary,
            profile_ids,
            output_profile_ids,
        );
    }
}

fn inspect_color_space_value(
    document: &Document,
    value: &lopdf::Object,
    seen_objects: &mut HashSet<lopdf::ObjectId>,
    summary: &mut ColorProfileSummary,
    profile_ids: &mut BTreeSet<lopdf::ObjectId>,
    output_profile_ids: &mut BTreeSet<lopdf::ObjectId>,
) {
    match value {
        lopdf::Object::Name(name) => match name.as_slice() {
            b"DeviceCMYK" => summary.cmyk_present = true,
            b"DeviceRGB" => summary.rgb_present = true,
            b"DeviceGray" => summary.gray_present = true,
            _ => {}
        },
        lopdf::Object::Array(items) => {
            if items.len() >= 2
                && matches!(items[0], lopdf::Object::Name(_))
                && let lopdf::Object::Name(first) = &items[0]
                && first == b"ICCBased"
            {
                let channels = resolve_object(document, &items[1])
                    .and_then(|object| object.as_dict().ok())
                    .and_then(|dict| dict.get(b"N").ok())
                    .and_then(|object| object.as_i64().ok());
                match channels {
                    Some(4) => summary.cmyk_present = true,
                    Some(3) => summary.rgb_present = true,
                    Some(1) => summary.gray_present = true,
                    _ => {}
                }
                observe_profile(
                    document,
                    &items[1],
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                    false,
                );
                return;
            }
            for item in items {
                inspect_color_space_value(
                    document,
                    item,
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                );
            }
        }
        lopdf::Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                inspect_color_space_value(
                    document,
                    value,
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                );
            }
        }
        lopdf::Object::Reference(id) => {
            if seen_objects.insert(*id)
                && let Ok(referenced) = document.get_object(*id)
            {
                inspect_color_space_value(
                    document,
                    referenced,
                    seen_objects,
                    summary,
                    profile_ids,
                    output_profile_ids,
                );
            }
        }
        _ => {}
    }
}

fn observe_profile(
    document: &Document,
    value: &lopdf::Object,
    _seen_objects: &mut HashSet<lopdf::ObjectId>,
    summary: &mut ColorProfileSummary,
    profile_ids: &mut BTreeSet<lopdf::ObjectId>,
    output_profile_ids: &mut BTreeSet<lopdf::ObjectId>,
    output_intent: bool,
) {
    match value {
        lopdf::Object::Reference(id) => {
            if profile_ids.insert(*id) {
                summary.icc_profile_count += 1;
            }
            if output_intent && output_profile_ids.insert(*id) {
                summary.output_intent_profile_count += 1;
            }

            if let Ok(object) = document.get_object(*id)
                && let Ok(stream) = object.as_stream()
                && let Ok(number) = stream.dict.get(b"N").and_then(|object| object.as_i64())
            {
                match number {
                    4 => summary.cmyk_present = true,
                    3 => summary.rgb_present = true,
                    1 => summary.gray_present = true,
                    _ => {}
                }
            }
        }
        lopdf::Object::Stream(stream) => {
            summary.icc_profile_count += 1;
            if output_intent {
                summary.output_intent_profile_count += 1;
            }
            if let Ok(number) = stream.dict.get(b"N").and_then(|object| object.as_i64()) {
                match number {
                    4 => summary.cmyk_present = true,
                    3 => summary.rgb_present = true,
                    1 => summary.gray_present = true,
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn resolve_object<'a>(
    document: &'a Document,
    object: &'a lopdf::Object,
) -> Option<&'a lopdf::Object> {
    match object {
        lopdf::Object::Reference(id) => document.get_object(*id).ok(),
        other => Some(other),
    }
}

#[cfg(test)]
mod m3_tests {
    use super::*;
    use lopdf::{Dictionary, Object, Stream};

    #[test]
    fn detects_cmyk_icc_profile_and_metadata() {
        let mut document = lopdf::Document::new();

        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();

        let mut profile_dict = Dictionary::new();
        profile_dict.set("N", Object::Integer(4));
        let profile_id = document.add_object(Object::Stream(Stream::new(profile_dict, Vec::new())));

        let mut info = Dictionary::new();
        info.set(
            "Title",
            Object::String(b"Titre".to_vec(), lopdf::StringFormat::Literal),
        );
        info.set(
            "Producer",
            Object::String(b"Trimbox test".to_vec(), lopdf::StringFormat::Literal),
        );
        let info_id = document.add_object(Object::Dictionary(info));

        let xmp = Dictionary::new();
        let xmp_id =
            document.add_object(Object::Stream(Stream::new(xmp, b"<x:xmpmeta/>".to_vec())));

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
            "ColorSpace",
            Object::Array(vec![
                Object::Name(b"ICCBased".to_vec()),
                Object::Reference(profile_id),
            ]),
        );
        document.objects.insert(page_id, Object::Dictionary(page));

        let mut pages = Dictionary::new();
        pages.set("Type", Object::Name(b"Pages".to_vec()));
        pages.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages.set("Count", Object::Integer(1));
        document.objects.insert(pages_id, Object::Dictionary(pages));

        let mut intent = Dictionary::new();
        intent.set("Type", Object::Name(b"OutputIntent".to_vec()));
        intent.set("S", Object::Name(b"GTS_PDFX".to_vec()));
        intent.set("DestOutputProfile", Object::Reference(profile_id));
        let intent_id = document.add_object(Object::Dictionary(intent));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        catalog.set(
            "OutputIntents",
            Object::Array(vec![Object::Reference(intent_id)]),
        );
        catalog.set("Metadata", Object::Reference(xmp_id));
        let catalog_id = document.add_object(Object::Dictionary(catalog));
        document.trailer.set("Root", Object::Reference(catalog_id));
        document.trailer.set("Info", Object::Reference(info_id));

        let color = analyze_color(&document);
        assert!(color.cmyk_present);
        assert!(!color.rgb_present);
        assert!(!color.gray_present);
        assert_eq!(color.icc_profile_count, 1);
        assert_eq!(color.output_intent_profile_count, 1);
        assert_eq!(color.mode().label(), "CMJN");

        let metadata = read_metadata(&document);
        assert!(metadata.has_info_dictionary);
        assert_eq!(metadata.title.as_deref(), Some("Titre"));
        assert_eq!(metadata.producer.as_deref(), Some("Trimbox test"));
        assert!(metadata.xmp_present);
        assert!(metadata.metadata_note().contains("Titre : Titre"));
    }

    #[test]
    fn detects_unknown_colors_without_profiles() {
        let mut document = lopdf::Document::new();
        let pages_id = document.new_object_id();
        let page_id = document.new_object_id();

        let mut page = Dictionary::new();
        page.set("Type", Object::Name(b"Page".to_vec()));
        page.set("Parent", Object::Reference(pages_id));
        page.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(100.0),
                Object::Real(200.0),
            ]),
        );
        page.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
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
        document.trailer.set("Root", Object::Reference(catalog_id));

        let source = analyze_source(&document);
        assert!(source.color.rgb_present);
        assert!(!source.color.cmyk_present);
        assert_eq!(source.color.icc_profile_count, 0);
        assert!(source.color.fallback_note().contains("RVB"));
        assert!(!source.metadata.has_info_dictionary);
    }
}

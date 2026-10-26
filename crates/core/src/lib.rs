pub mod boxes;
pub mod config;
pub mod crop;
pub mod error;
pub mod normalize;
pub mod pdf;
pub mod report;
pub mod source_characteristics;
pub mod validation;

pub use boxes::{PageBoxes, Rect, cm_to_pt, pt_to_cm, read_page_boxes};
pub use crop::crop_document;
pub use error::Error;
pub use normalize::{PdfXNormalizationReport, normalize_pdf_x4};
pub use report::CropReport;
pub use source_characteristics::{
    ColorMode, ColorProfileSummary, DocumentMetadataSummary, SourceCharacteristics, analyze_color,
    analyze_source, read_metadata,
};

// Compatibility aliases for earlier milestone-style M3 naming.
pub use source_characteristics::ColorMode as SourceColorMode;
pub use source_characteristics::ColorProfileSummary as SourceColorSummary;
pub use validation::{IssueSeverity, PdfXIssue, PdfXValidationReport, validate_pdf_x4};

// Compatibility aliases for milestone callers that may use simpler wording.
pub use source_characteristics as color;
pub use source_characteristics as metadata;
pub use source_characteristics as source;

/// Placeholder visible dans la bibliothèque pour indiquer sans UI que le noyau compile et se teste.
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

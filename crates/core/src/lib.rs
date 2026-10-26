pub mod boxes;
pub mod config;
pub mod crop;
pub mod error;
pub mod pdf;
pub mod report;

pub use boxes::{PageBoxes, Rect, cm_to_pt, pt_to_cm, read_page_boxes};
pub use crop::crop_document;
pub use error::Error;
pub use report::CropReport;

/// Placeholder visible dans la bibliothèque pour indiquer sans UI que le noyau compile et se teste.
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

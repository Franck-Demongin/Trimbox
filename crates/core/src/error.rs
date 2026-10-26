use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no such PDF path or empty input: {0}")]
    InvalidPdf(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF processing error: {0}")]
    Pdf(String),
    #[error("invalid dimension: {0}")]
    InvalidDimension(String),
    #[error("page {page} has no MediaBox")]
    MissingMediaBox { page: u32 },
    #[error(
        "requested crop does not fit page {page}: {width:.2}x{height:.2} pt > {available_width:.2}x{available_height:.2} pt"
    )]
    CropDoesNotFit {
        page: u32,
        width: f64,
        height: f64,
        available_width: f64,
        available_height: f64,
    },
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidPdf(message.into())
    }

    pub fn invalid_dimension(message: impl Into<String>) -> Self {
        Self::InvalidDimension(message.into())
    }
}

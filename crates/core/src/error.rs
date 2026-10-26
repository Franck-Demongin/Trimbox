use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no such PDF path or empty input: {0}")]
    InvalidPdf(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF processing error: {0}")]
    Pdf(String),
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidPdf(message.into())
    }
}

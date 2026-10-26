use lopdf::Document;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bytes_are_rejected() {
        assert!(load_document(&[]).is_err());
    }

    #[test]
    fn blank_document_is_serializable() {
        let bytes = blank_pdf_x4().expect("blank document");
        assert!(!bytes.is_empty());
    }
}

pub mod config;
pub mod error;
pub mod pdf;

pub use error::Error;

/// Placeholder visible dans la bibliothèque pour indiquer sans UI que le noyau compile et se teste.
pub fn library_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

use std::error::Error;

pub type BoxError = Box<dyn Error + Send + Sync>;
pub use eyre::Result;

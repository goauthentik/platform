use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
pub type BoxError = Box<dyn Error + Send + Sync>;

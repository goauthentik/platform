//! Validate proposed authentik Blueprint content.
//!
//! This module has no side effects. Callers must validate before applying a
//! Blueprint; server-side permissions remain the final authorization boundary.

pub mod duration;
pub mod policy;
pub mod refs;
pub mod tags;
pub mod validate;
pub mod yaml;

pub use validate::{BlueprintValidation, FlagItem, validate_blueprint};

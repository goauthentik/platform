//! authentik Blueprint content validator — the control-plane security boundary.
//!
//! A default-deny gate over a *proposed* blueprint, run before any apply. It is
//! a faithful port of the marketplace code-mode `blueprint/` subsystem; the two
//! escalation findings that motivated it (blueprint-apply is superuser-
//! equivalent; broad read leaks `view_*_key` secrets) are why validation must
//! live here in the agent rather than lean on RBAC. It never panics — hostile
//! or malformed input becomes a violation.
//!
//! NOTE: a client-side validator can be bypassed by a tampered binary, so the
//! server-held apply identity must remain independently bounded — it applies
//! only what its own RBAC allows, never arbitrary blueprint content.

pub mod duration;
pub mod policy;
pub mod refs;
pub mod tags;
pub mod validate;
pub mod yaml;

pub use validate::{BlueprintValidation, FlagItem, validate_blueprint};

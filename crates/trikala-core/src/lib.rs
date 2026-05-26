//! trikala-core — types every other foundation crate builds on.
//!
//! Per axiom F1, this crate must be standalone-useful. Per axiom F2,
//! every public type in any other trikala crate routes through here
//! so that `wgpu`, `winit`, `egui` types never appear in cross-crate
//! signatures.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// The version of the trikala foundation this binary was built against.
pub const TRIKALA_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One of the three phases a game passes through.
///
/// Maps to the Sanskrit / Pali concept of *trikala* — past, present,
/// future. Surfaces in CLI help (`U6`) and in the `kind` field of
/// every public error emitted by the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Born — scaffolding a new project.
    Atita,
    /// Built — iterating with hot reload.
    Vartamana,
    /// Shipped — releasing to the world.
    Anagata,
}

impl Phase {
    /// Three-letter shortcode used in error codes and machine output.
    pub fn code(self) -> &'static str {
        match self {
            Phase::Atita => "ATI",
            Phase::Vartamana => "VAR",
            Phase::Anagata => "ANA",
        }
    }
}

/// Project configuration loaded from `trikala.toml`.
///
/// Per axiom U13, version pinning of the trikala foundation itself
/// lives here so a project compiled today still builds in two years.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project identity.
    pub project: Project,
    /// Pinned trikala version (matches `trikala use <ver>`).
    pub trikala: TrikalaPin,
}

/// `[project]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Slug-style project name. Used in URLs and binary filenames.
    pub name: String,
}

/// `[trikala]` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrikalaPin {
    /// Foundation version this project was scaffolded against.
    pub version: String,
}

/// The error type surfaced to end users.
///
/// Per axiom U10 every error has `code`, `cause`, `hint`, `docs_url`.
#[derive(Debug, thiserror::Error)]
#[error("[{code}] {cause}\n  hint: {hint}\n  docs: {docs_url}")]
pub struct TrikalaError {
    /// Machine-readable error code, e.g. `ATI-001`.
    pub code: String,
    /// One-line human-readable cause.
    pub cause: String,
    /// Actionable next step.
    pub hint: String,
    /// Deep link into trikala docs.
    pub docs_url: String,
}

impl TrikalaError {
    /// Construct an error in the given phase with an auto-coded number.
    pub fn new(phase: Phase, num: u16, cause: impl Into<String>, hint: impl Into<String>) -> Self {
        let code = format!("{}-{:03}", phase.code(), num);
        let docs_url = format!("https://github.com/RoundOnline/trikala/issues?q={}", code);
        Self { code, cause: cause.into(), hint: hint.into(), docs_url }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_carries_all_four_fields() {
        let err = TrikalaError::new(Phase::Anagata, 12, "deploy refused", "run `trikala claim`");
        assert_eq!(err.code, "ANA-012");
        assert!(err.docs_url.contains("ANA-012"));
    }

    #[test]
    fn phase_codes_are_three_letters() {
        for p in [Phase::Atita, Phase::Vartamana, Phase::Anagata] {
            assert_eq!(p.code().len(), 3);
        }
    }
}

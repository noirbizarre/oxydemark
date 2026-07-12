//! First-class, PyO3-independent error type for the OxydeMark Rust surface.
//!
//! Historically the public Rust API leaked `PyResult`; OMEP-0008 tracked a
//! first-class Rust error type as a 1.0 follow-up. That follow-up is pulled
//! forward here so the Rust surface is fully independent of PyO3: fallible
//! operations return [`Result<T, OxydeError>`], and the optional Python
//! binding layer converts `OxydeError` into a `PyErr`.

use std::fmt;

/// Errors that can occur while processing Markdown with OxydeMark.
///
/// This type is intentionally small and opaque: it does not leak the
/// underlying `rushdown` renderer error type, keeping the public surface
/// stable across dependency upgrades. It is `#[non_exhaustive]` so new
/// variants can be added without a breaking change.
#[derive(Debug)]
#[non_exhaustive]
pub enum OxydeError {
    /// Rendering an AST or document to HTML failed.
    ///
    /// Wraps the underlying renderer failure as a human-readable message.
    Render(String),
}

impl fmt::Display for OxydeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OxydeError::Render(msg) => write!(f, "render error: {msg}"),
        }
    }
}

impl std::error::Error for OxydeError {}

#[cfg(feature = "python")]
impl From<OxydeError> for pyo3::PyErr {
    fn from(err: OxydeError) -> Self {
        match err {
            OxydeError::Render(msg) => pyo3::exceptions::PyRuntimeError::new_err(msg),
        }
    }
}

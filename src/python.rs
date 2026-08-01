//! PyO3 binding layer.
//!
//! Thin wrappers over the pure-Rust API in [`crate::api`], plus the
//! `oxydemark._core` extension module. This whole module is compiled only when
//! the `python` cargo feature is enabled, so the crate has no dependency on
//! PyO3 for downstream Rust consumers.

use pyo3::prelude::*;

use crate::api;
use crate::ast::{AstNode, Heading, ParseResult};

/// Parse Markdown input into an AST node tree.
#[pyfunction]
#[pyo3(name = "parse")]
fn py_parse(markdown: &str) -> AstNode {
    api::parse(markdown)
}

/// Parse Markdown and compute structured, typed document metadata.
#[pyfunction]
#[pyo3(name = "parse_document")]
fn py_parse_document(markdown: &str) -> ParseResult {
    api::parse_document(markdown)
}

/// Render an `AstNode` tree to an HTML string.
#[pyfunction]
#[pyo3(name = "render_ast")]
fn py_render_ast(node: &AstNode) -> String {
    api::render_ast(node)
}

/// Convert Markdown directly to HTML (fast path, no AST exposure).
#[pyfunction]
#[pyo3(name = "markdown_to_html")]
fn py_markdown_to_html(markdown: &str) -> PyResult<String> {
    // `?` relies on `From<OxydeError> for PyErr` (see `crate::error`).
    Ok(api::markdown_to_html(markdown)?)
}

/// Generate a URL-friendly anchor slug from `text`.
#[pyfunction]
#[pyo3(name = "slugify", signature = (text, existing=None))]
fn py_slugify(text: &str, existing: Option<Vec<String>>) -> String {
    api::slugify(text, existing)
}

/// Extract the summary (excerpt) preceding a `<!-- more -->` delimiter.
#[pyfunction]
#[pyo3(name = "extract_summary")]
fn py_extract_summary(markdown: &str) -> Option<String> {
    api::extract_summary(markdown)
}

/// The native Python module implemented in Rust.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AstNode>()?;
    m.add_class::<Heading>()?;
    m.add_class::<ParseResult>()?;
    m.add_function(wrap_pyfunction!(py_parse, m)?)?;
    m.add_function(wrap_pyfunction!(py_parse_document, m)?)?;
    m.add_function(wrap_pyfunction!(py_render_ast, m)?)?;
    m.add_function(wrap_pyfunction!(py_markdown_to_html, m)?)?;
    m.add_function(wrap_pyfunction!(py_slugify, m)?)?;
    m.add_function(wrap_pyfunction!(py_extract_summary, m)?)?;
    Ok(())
}

use pyo3::prelude::*;

/// Render Markdown input to HTML.
///
/// This is the core rendering function exposed to Python.
/// It currently returns a minimal placeholder; the full pipeline
/// (parsing, AST transformations, rendering) will be implemented
/// in subsequent milestones.
#[pyfunction]
fn render(markdown: &str) -> String {
    // Placeholder: wrap input in a paragraph tag
    format!("<p>{markdown}</p>")
}

/// The native Python module implemented in Rust.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(render, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_wraps_in_paragraph() {
        let html = render("Hello");
        assert_eq!(html, "<p>Hello</p>");
    }
}

//! OxydeMark: extensible Markdown pipelines powered by Rust.
//!
//! The crate root re-exports the stable public Rust surface (OMEP-0008 /
//! OMEP-0010), including [`Meta`] which originates in `rushdown` and is
//! re-exported so the surface is self-contained. Everything else in the crate
//! (the `extensions`, `html_render`, `slug`, and `ast` internal helpers, the
//! thread-local parser/renderer caches, and the arena-conversion functions) is
//! private and may change without notice.
//!
//! The optional `python` cargo feature additionally builds the PyO3 extension
//! module `oxydemark._core`. Without it, the crate has **no** dependency on
//! PyO3, so downstream Rust consumers can use the API below without requiring
//! Python.
//!
//! # Examples
//!
//! ```
//! let ast = oxydemark::parse("# Hello **world**");
//! assert_eq!(ast.kind, "document");
//!
//! let html = oxydemark::markdown_to_html("# Hello").unwrap();
//! assert!(html.contains("<h1"));
//! ```

mod api;
mod ast;
mod error;
mod extensions;
mod html_render;
mod slug;

#[cfg(feature = "python")]
mod python;

// Frozen public Rust surface (OMEP-0008 + OMEP-0010).
pub use api::{extract_summary, markdown_to_html, parse, parse_document, render_ast, slugify};
pub use ast::{AstNode, Heading, ParseResult};
pub use error::OxydeError;
/// Typed metadata value (YAML scalars, sequences and mappings).
///
/// Re-exported from `rushdown`, where it is defined. It is the value type of
/// both [`ParseResult::frontmatter`] and [`AstNode::props`], so it is
/// re-exported here to keep the public surface self-contained: downstream
/// crates can name it -- in a struct field, a function signature or a `match`
/// -- without adding `rushdown` as a dependency (issue #34).
///
/// Because it is part of OxydeMark's public API, a `rushdown` major bump is a
/// breaking change for OxydeMark too, hence the exact pin in `Cargo.toml`
/// (issue #33).
pub use rushdown::ast::Meta;

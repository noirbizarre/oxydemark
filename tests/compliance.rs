//! Comark compliance suite (OMEP-0007 Phase 3).
//!
//! Data-driven tests over the fixtures in `tests/compliance/`, written either
//! as delimited Markdown (`*.md`) or as JSON (`*.json`). Every case asserts the
//! exact HTML produced by *both* render paths — the rushdown fast path and the
//! standalone [`render_ast`] renderer, mirroring the `assert_both_paths` helper
//! of the unit tests — and, optionally, a *partial* AST shape: keys absent from
//! a fixture are never asserted, so additive AST changes cannot break the
//! suite.
//!
//! The same fixtures drive `tests/test_compliance.py`. See
//! `tests/compliance/README.md` for the schema and for how to add a case.
//!
//! This target deliberately builds without the `python` feature: it exercises
//! only the frozen public surface (OMEP-0008).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use oxydemark::{AstNode, markdown_to_html, parse, render_ast};
use rushdown::ast::Meta;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Fixture model
// ---------------------------------------------------------------------------

/// A fixture file: a description and the cases it declares.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    /// What the file covers, for human readers only.
    #[allow(dead_code)]
    description: String,
    /// Optional provenance, typically a link into OMEP-0007.
    #[serde(default)]
    #[allow(dead_code)]
    reference: Option<String>,
    cases: Vec<Case>,
}

/// A single compliance case.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    /// Unique within the file; forms the second half of the test identifier.
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    description: Option<String>,
    /// The Markdown source.
    markdown: String,
    /// The exact expected HTML, including the trailing newline.
    html: String,
    /// An optional partial expectation on the parsed AST.
    #[serde(default)]
    ast: Option<NodeSpec>,
}

/// A *partial* expectation for a single AST node.
///
/// Every field is optional and a field left out is simply not asserted, which
/// keeps fixtures immune to unrelated AST additions.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeSpec {
    /// Exact match on [`AstNode::kind`].
    #[serde(default)]
    kind: Option<String>,
    /// Exact match on [`AstNode::text`], which must then be present.
    #[serde(default)]
    text: Option<String>,
    /// Subset match: every listed key must exist with that exact value.
    #[serde(default)]
    attributes: Option<BTreeMap<String, String>>,
    /// Keys that must *not* be present in [`AstNode::attributes`].
    #[serde(default)]
    absent_attributes: Vec<String>,
    /// `Some(Value::Null)` requires `props` to be `None`; `Some(object)` is a
    /// subset match; `None` (the key absent from the fixture) asserts nothing.
    #[serde(default, deserialize_with = "explicit_value")]
    props: Option<Value>,
    /// Positional *prefix* match against [`AstNode::children`].
    #[serde(default)]
    children: Option<Vec<NodeSpec>>,
    /// Require the child count to match `children` exactly.
    #[serde(default)]
    exact_children: bool,
    /// `"first:<kind>"` — re-anchor the match on the first pre-order
    /// descendant with that kind, so fixtures need not spell out the
    /// `document` → … chain.
    #[serde(default)]
    descend: Option<String>,
}

/// Deserialize a value while distinguishing an absent key from an explicit
/// `null`.
///
/// `#[serde(default)]` yields `None` for an absent key, and this function is
/// only invoked when the key *is* present, so an explicit `null` deserializes
/// to `Some(Value::Null)`.
fn explicit_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

// ---------------------------------------------------------------------------
// Matcher
// ---------------------------------------------------------------------------

/// Check `node` against the partial expectation `spec`.
///
/// `path` is a breadcrumb such as `root.children[0]`, reported on mismatch.
fn match_node(node: &AstNode, spec: &NodeSpec, path: &str) -> Result<(), String> {
    // `descend` re-anchors the match before any other assertion is evaluated.
    if let Some(selector) = &spec.descend {
        let kind = selector
            .strip_prefix("first:")
            .ok_or_else(|| format!("{path}: unsupported `descend` selector {selector:?}"))?;
        let anchored = node
            .walk()
            .into_iter()
            .find(|candidate| candidate.kind == kind)
            .ok_or_else(|| format!("{path}: no descendant with kind {kind:?}"))?;
        let inner = NodeSpec {
            descend: None,
            ..spec.clone()
        };
        return match_node(&anchored, &inner, &format!("{path}/first:{kind}"));
    }

    if let Some(expected) = &spec.kind
        && &node.kind != expected
    {
        return Err(format!(
            "{path}: kind is {:?}, expected {expected:?}",
            node.kind
        ));
    }

    if let Some(expected) = &spec.text
        && node.text.as_deref() != Some(expected.as_str())
    {
        return Err(format!(
            "{path}: text is {:?}, expected {expected:?}",
            node.text
        ));
    }

    if let Some(expected) = &spec.attributes {
        for (key, value) in expected {
            match node.attributes.get(key) {
                Some(actual) if actual == value => {}
                Some(actual) => {
                    return Err(format!(
                        "{path}: attribute {key:?} is {actual:?}, expected {value:?}"
                    ));
                }
                None => {
                    return Err(format!(
                        "{path}: attribute {key:?} is missing (present: {:?})",
                        sorted_keys(&node.attributes)
                    ));
                }
            }
        }
    }

    for key in &spec.absent_attributes {
        if let Some(actual) = node.attributes.get(key) {
            return Err(format!(
                "{path}: attribute {key:?} should be absent but is {actual:?}"
            ));
        }
    }

    match_props(node, spec, path)?;
    match_children(node, spec, path)
}

/// Check the `props` expectation of `spec` against `node`.
fn match_props(node: &AstNode, spec: &NodeSpec, path: &str) -> Result<(), String> {
    match (&spec.props, &node.props) {
        (None, _) => Ok(()),
        (Some(Value::Null), None) => Ok(()),
        (Some(Value::Null), Some(actual)) => {
            Err(format!("{path}: props should be null, got {actual:?}"))
        }
        (Some(expected), None) => Err(format!("{path}: props is null, expected {expected}")),
        (Some(Value::Object(expected)), Some(Meta::Mapping(actual))) => {
            for (key, value) in expected {
                let found = actual
                    .get(key.as_str())
                    .ok_or_else(|| format!("{path}: prop {key:?} is missing"))?;
                if !meta_matches(found, value) {
                    return Err(format!(
                        "{path}: prop {key:?} is {found:?}, expected {value}"
                    ));
                }
            }
            Ok(())
        }
        (Some(expected), Some(actual)) => Err(format!(
            "{path}: props shape mismatch: expected {expected}, got {actual:?}"
        )),
    }
}

/// Check the `children`/`exact_children` expectations of `spec` against `node`.
fn match_children(node: &AstNode, spec: &NodeSpec, path: &str) -> Result<(), String> {
    let Some(expected) = &spec.children else {
        return Ok(());
    };

    if spec.exact_children && node.children.len() != expected.len() {
        return Err(format!(
            "{path}: expected exactly {} children, got {} ({:?})",
            expected.len(),
            node.children.len(),
            child_kinds(node)
        ));
    }
    if node.children.len() < expected.len() {
        return Err(format!(
            "{path}: expected at least {} children, got {} ({:?})",
            expected.len(),
            node.children.len(),
            child_kinds(node)
        ));
    }

    for (index, child_spec) in expected.iter().enumerate() {
        match_node(
            &node.children[index],
            child_spec,
            &format!("{path}.children[{index}]"),
        )?;
    }
    Ok(())
}

/// Compare a typed [`Meta`] value against its JSON counterpart.
///
/// Mappings and sequences are compared recursively; mappings use subset
/// semantics, sequences require an exact element-wise match.
fn meta_matches(meta: &Meta, value: &Value) -> bool {
    match (meta, value) {
        (Meta::Null, Value::Null) => true,
        (Meta::Bool(actual), Value::Bool(expected)) => actual == expected,
        (Meta::Int(actual), Value::Number(expected)) => expected.as_i64() == Some(*actual),
        (Meta::Float(actual), Value::Number(expected)) => expected
            .as_f64()
            .is_some_and(|expected| (actual - expected).abs() < f64::EPSILON),
        (Meta::String(actual), Value::String(expected)) => actual == expected,
        (Meta::Sequence(items), Value::Array(values)) => {
            items.len() == values.len()
                && items
                    .iter()
                    .zip(values)
                    .all(|(item, value)| meta_matches(item, value))
        }
        (Meta::Mapping(map), Value::Object(object)) => object.iter().all(|(key, value)| {
            map.get(key.as_str())
                .is_some_and(|item| meta_matches(item, value))
        }),
        _ => false,
    }
}

/// The attribute keys of `map`, sorted, for deterministic failure messages.
fn sorted_keys(map: &HashMap<String, String>) -> Vec<&str> {
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

/// The kinds of the direct children of `node`, for failure messages.
fn child_kinds(node: &AstNode) -> Vec<&str> {
    node.children
        .iter()
        .map(|child| child.kind.as_str())
        .collect()
}

// ---------------------------------------------------------------------------
// Delimited-markdown fixture parser
// ---------------------------------------------------------------------------

/// A fence opener: the number of backticks and the info string.
struct Fence<'a> {
    backticks: usize,
    info: &'a str,
}

/// Recognise a fence line, i.e. a run of at least three backticks optionally
/// followed by an info string.
fn fence(line: &str) -> Option<Fence<'_>> {
    let trimmed = line.trim_start();
    let backticks = trimmed.chars().take_while(|c| *c == '`').count();
    if backticks < 3 {
        return None;
    }
    let info = trimmed[backticks..].trim();
    // An info string may not itself contain a backtick (CommonMark).
    if info.contains('`') {
        return None;
    }
    Some(Fence { backticks, info })
}

/// Parse a delimited-markdown fixture file.
///
/// See `tests/compliance/README.md` for the grammar. Returns a human-readable
/// message on the first structural error; the caller prefixes it with the file
/// name.
fn parse_markdown_fixture(source: &str) -> Result<Fixture, String> {
    let mut description = String::new();
    let mut reference: Option<String> = None;
    let mut cases: Vec<Case> = Vec::new();
    let mut current: Option<Draft> = None;

    let lines: Vec<&str> = source.lines().collect();
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];

        if let Some(name) = line.strip_prefix("## ") {
            if let Some(draft) = current.take() {
                cases.push(draft.finish()?);
            }
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(format!("line {}: empty case name", index + 1));
            }
            current = Some(Draft {
                name,
                prose: Vec::new(),
                markdown: None,
                html: None,
                ast: None,
            });
            index += 1;
            continue;
        }

        if let Some(opener) = fence(line) {
            let (body, next) = read_block(&lines, index, &opener)?;
            let Some(draft) = current.as_mut() else {
                return Err(format!(
                    "line {}: fenced block {:?} outside of a case",
                    index + 1,
                    opener.info
                ));
            };
            let name = &draft.name;
            let slot = match opener.info {
                "comark" => &mut draft.markdown,
                "html" => &mut draft.html,
                "json ast" => &mut draft.ast,
                other => {
                    return Err(format!(
                        "{name}: unsupported fence info string {other:?} \
                         (expected `comark`, `html` or `json ast`)"
                    ));
                }
            };
            if slot.is_some() {
                return Err(format!("{}: duplicate `{}` block", draft.name, opener.info));
            }
            *slot = Some(body);
            index = next;
            continue;
        }

        match current.as_mut() {
            // Prose inside a case, before its first fence, is its description.
            Some(draft) if draft.markdown.is_none() => draft.prose.push(line.to_string()),
            Some(_) => {}
            None => {
                if let Some(rest) = line.strip_prefix("Reference:") {
                    reference = Some(rest.trim().to_string());
                } else if !line.starts_with('#') {
                    description.push_str(line);
                    description.push('\n');
                }
            }
        }
        index += 1;
    }

    if let Some(draft) = current.take() {
        cases.push(draft.finish()?);
    }
    if cases.is_empty() {
        return Err("no case found (expected at least one `## <name>` heading)".to_string());
    }

    Ok(Fixture {
        description: description.trim().to_string(),
        reference,
        cases,
    })
}

/// Read the body of the block opened at `start`, returning it and the index of
/// the line following the closing fence.
fn read_block(lines: &[&str], start: usize, opener: &Fence<'_>) -> Result<(String, usize), String> {
    let mut body = String::new();
    let mut index = start + 1;
    while index < lines.len() {
        if let Some(closer) = fence(lines[index])
            && closer.info.is_empty()
            && closer.backticks >= opener.backticks
        {
            return Ok((body, index + 1));
        }
        body.push_str(lines[index]);
        body.push('\n');
        index += 1;
    }
    Err(format!(
        "line {}: unterminated `{}` block",
        start + 1,
        opener.info
    ))
}

/// A case under construction: its name, the prose collected before the first
/// fence, and the raw bodies of its blocks.
struct Draft {
    name: String,
    prose: Vec<String>,
    markdown: Option<String>,
    html: Option<String>,
    ast: Option<String>,
}

impl Draft {
    /// Validate the draft and turn it into a [`Case`].
    fn finish(self) -> Result<Case, String> {
        let Draft {
            name,
            prose,
            markdown,
            html,
            ast,
        } = self;
        let markdown = markdown.ok_or_else(|| format!("{name}: missing `comark` block"))?;
        let html = html.ok_or_else(|| format!("{name}: missing `html` block"))?;
        let ast = ast
            .map(|raw| {
                serde_json::from_str::<NodeSpec>(&raw)
                    .map_err(|error| format!("{name}: invalid `json ast` block: {error}"))
            })
            .transpose()?;

        let description = prose.join("\n").trim().to_string();
        Ok(Case {
            name,
            description: (!description.is_empty()).then_some(description),
            markdown,
            html,
            ast,
        })
    }
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// The fixture directory, resolved from the crate manifest so the test does not
/// depend on the current working directory.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compliance")
}

/// Run every case of every fixture file, reporting all failures at once.
#[test]
fn comark_compliance_suite() {
    let dir = fixtures_dir();
    if !dir.exists() {
        // The fixtures are excluded from the published tarball on purpose (see
        // the `include` allow-list in `Cargo.toml`), so a build from an sdist
        // has nothing to run.
        eprintln!(
            "compliance fixtures not shipped in this tarball ({}); skipping",
            dir.display()
        );
        return;
    }

    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("cannot read directory entry").path())
        .filter(|path| {
            path.file_name().is_some_and(|name| name != "README.md")
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json" || extension == "md")
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no compliance fixtures found in {}",
        dir.display()
    );

    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut stems: Vec<String> = Vec::new();

    for file in &files {
        let stem = file
            .file_stem()
            .expect("fixture path has a file name")
            .to_string_lossy()
            .into_owned();
        assert!(
            !stems.contains(&stem),
            "duplicate fixture name {stem:?}: test ids would collide"
        );
        stems.push(stem.clone());
        let raw = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let fixture: Fixture = if file.extension().is_some_and(|extension| extension == "md") {
            parse_markdown_fixture(&raw)
                .unwrap_or_else(|error| panic!("invalid fixture {}: {error}", file.display()))
        } else {
            serde_json::from_str(&raw)
                .unwrap_or_else(|error| panic!("invalid fixture {}: {error}", file.display()))
        };

        let mut seen: Vec<&str> = Vec::new();
        for case in &fixture.cases {
            assert!(
                !seen.contains(&case.name.as_str()),
                "{stem}: duplicate case name {:?}",
                case.name
            );
            seen.push(&case.name);
            total += 1;

            let id = format!("{stem}::{}", case.name);
            failures.extend(run_case(&id, case));
        }
    }

    assert!(
        failures.is_empty(),
        "{} assertion(s) failed across {total} compliance case(s):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// Evaluate a single case, returning one message per failed assertion.
fn run_case(id: &str, case: &Case) -> Vec<String> {
    let mut failures = Vec::new();

    match markdown_to_html(&case.markdown) {
        Ok(html) if html == case.html => {}
        Ok(html) => failures.push(format!(
            "{id}: fast path HTML mismatch\n  input:    {:?}\n  expected: {:?}\n  actual:   {html:?}",
            case.markdown, case.html
        )),
        Err(error) => failures.push(format!("{id}: fast path failed: {error}")),
    }

    let ast = parse(&case.markdown);
    let round_trip = render_ast(&ast);
    if round_trip != case.html {
        failures.push(format!(
            "{id}: AST round-trip HTML mismatch\n  input:    {:?}\n  expected: {:?}\n  actual:   {round_trip:?}",
            case.markdown, case.html
        ));
    }

    if let Some(spec) = &case.ast
        && let Err(message) = match_node(&ast, spec, "root")
    {
        failures.push(format!(
            "{id}: AST mismatch: {message}\n  input: {:?}",
            case.markdown
        ));
    }

    failures
}

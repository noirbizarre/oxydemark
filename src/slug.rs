//! Deterministic, collision-safe heading slug generation.
//!
//! This module implements the anchor algorithm specified in OMEP-0010: text is
//! Unicode NFKD-normalized, lowercased, and reduced to a `[a-z0-9]`-and-`-`
//! slug, with `-N` disambiguation against a per-document set of already-assigned
//! slugs. The logic is intentionally free of any PyO3 or rushdown dependency so
//! it can be unit-tested in isolation and reused by both the Rust and Python
//! surfaces.

use std::collections::HashSet;

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Fallback slug used when the input reduces to an empty string.
const FALLBACK: &str = "section";

/// Compute the base slug for `text` without collision handling.
///
/// The algorithm:
/// 1. Apply Unicode NFKD normalization and lowercase the result.
/// 2. Replace every run of characters that is not `[a-z0-9]` with a single
///    hyphen (`-`); combining marks left by normalization are dropped.
/// 3. Trim leading and trailing hyphens.
/// 4. If the result is empty, fall back to [`FALLBACK`] (`"section"`).
pub(crate) fn slugify_base(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut pending_hyphen = false;

    for ch in text.nfkd().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(ch);
        } else if is_combining_mark(ch) {
            // Combining marks left by NFKD decomposition are stripped entirely
            // (e.g. the accent in "é" -> "e"), not treated as separators.
            continue;
        } else {
            // Any other non-alphanumeric character collapses into a single
            // separating hyphen.
            pending_hyphen = true;
        }
    }

    if slug.is_empty() {
        FALLBACK.to_string()
    } else {
        slug
    }
}

/// Compute a unique slug for `text`, disambiguating against `existing`.
///
/// The base slug is computed with [`slugify_base`]. If it is already present in
/// `existing`, `-N` is appended where `N` is the smallest integer `>= 1` that
/// yields an unused slug; the suffixed candidate is itself checked against the
/// set. The returned slug is inserted into `existing` before being returned so
/// subsequent calls observe it.
pub(crate) fn slugify_unique(text: &str, existing: &mut HashSet<String>) -> String {
    let base = slugify_base(text);
    let slug = disambiguate(base, existing);
    existing.insert(slug.clone());
    slug
}

/// Return `base` if unused, otherwise the first `base-N` (`N >= 1`) not in
/// `existing`. Does not mutate `existing`.
fn disambiguate(base: String, existing: &HashSet<String>) -> String {
    if !existing.contains(&base) {
        return base;
    }
    let mut n: usize = 1;
    loop {
        let candidate = format!("{base}-{n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_hyphenates() {
        assert_eq!(slugify_base("Hello World"), "hello-world");
    }

    #[test]
    fn collapses_runs_of_separators() {
        assert_eq!(slugify_base("Hello   ---  World!!!"), "hello-world");
    }

    #[test]
    fn trims_leading_and_trailing_hyphens() {
        assert_eq!(slugify_base("  Overview  "), "overview");
        assert_eq!(slugify_base("!!!edges!!!"), "edges");
    }

    #[test]
    fn unicode_is_normalized_to_ascii() {
        // NFKD decomposes accented letters; combining marks are stripped.
        assert_eq!(slugify_base("Café"), "cafe");
        assert_eq!(slugify_base("naïve résumé"), "naive-resume");
    }

    #[test]
    fn punctuation_only_falls_back_to_section() {
        assert_eq!(slugify_base("..."), "section");
        assert_eq!(slugify_base("!!!"), "section");
        assert_eq!(slugify_base(""), "section");
    }

    #[test]
    fn digits_are_preserved() {
        assert_eq!(slugify_base("Version 1.2.3"), "version-1-2-3");
    }

    #[test]
    fn collision_appends_incrementing_suffix() {
        let mut seen = HashSet::new();
        assert_eq!(slugify_unique("Overview", &mut seen), "overview");
        assert_eq!(slugify_unique("Overview", &mut seen), "overview-1");
        assert_eq!(slugify_unique("Overview", &mut seen), "overview-2");
    }

    #[test]
    fn suffixed_candidate_is_itself_checked() {
        // A document containing "Overview 1" then two "Overview" headings must
        // still get distinct ids.
        let mut seen = HashSet::new();
        assert_eq!(slugify_unique("Overview 1", &mut seen), "overview-1");
        assert_eq!(slugify_unique("Overview", &mut seen), "overview");
        assert_eq!(slugify_unique("Overview", &mut seen), "overview-2");
    }

    #[test]
    fn honours_preseeded_existing_set() {
        let mut seen = HashSet::new();
        seen.insert("overview".to_string());
        assert_eq!(slugify_unique("Overview", &mut seen), "overview-1");
    }
}

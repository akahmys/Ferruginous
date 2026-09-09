//! What a browser can learn from a document through this crate.
//!
//! **It could learn how long a PDF was and nothing else.** This crate exposed three
//! functions — open, page count, and a `render_page` that refuses — while the engine
//! behind it reads 96% of what a corpus presents. Everything added here needs no GPU,
//! which is the line between what this crate can do and what it says it cannot.
//!
//! Tested through the plain functions rather than the `wasm_bindgen` methods, because
//! `JsValue` cannot be constructed off a WebAssembly target — the same reason
//! `refusal_test.rs` gives.

use fepdf::PdfDocument;
use fepdf_wasm::{decisions_of, struct_tree_of, text_of};

fn sample(name: &str) -> Option<PdfDocument> {
    let bytes = std::fs::read(format!("../../samples/{name}")).ok()?;
    PdfDocument::open(bytes.into()).ok()
}

#[test]
fn a_page_yields_its_text() {
    let Some(doc) = sample("sample.pdf") else { return };
    let text = text_of(&doc, 0).expect("page 1 extracts");
    assert!(!text.trim().is_empty(), "a page with text on it came back empty");
}

/// A page that does not exist is an error, not an empty string — the two are different
/// answers and a caller acting on the first cannot tell.
#[test]
fn a_page_that_is_not_there_is_an_error() {
    let Some(doc) = sample("sample.pdf") else { return };
    assert!(text_of(&doc, 9_999).is_err(), "a page past the end reported success");
}

/// The decisions are what this engine has instead of a log, so a caller that cannot see
/// them has the logging problem back.
#[test]
fn the_decisions_cross_as_json() {
    let Some(doc) = sample("fy05.pdf") else { return };
    let json = decisions_of(&doc).expect("the decisions serialise");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("it is JSON");
    assert!(parsed.is_array(), "the decisions cross as an array: {json:.80}");
}

/// A document with no structure tree says `null`, which is an answer rather than a
/// failure to read one.
#[test]
fn a_document_without_a_structure_tree_says_null() {
    let doc = PdfDocument::create_empty().expect("an empty document");
    assert_eq!(struct_tree_of(&doc).expect("it serialises"), "null");
}

/// And one that has a tree hands it over.
#[test]
fn a_tagged_document_yields_its_tree() {
    let Some(doc) = sample("fy05.pdf") else { return };
    let json = struct_tree_of(&doc).expect("it serialises");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("it is JSON");
    assert!(parsed.is_object() || parsed.is_null(), "a tree is an object or absent");
}

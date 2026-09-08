//! What an operation could not read is reported, not passed off as nothing to report.
//!
//! RR-15 Rule 13 forbids swallowing an error in silence, and its checker looks for
//! `filter_map(Result::ok)`. A `let _ = <call returning Result>;` is the same swallow
//! written as a binding, and the checker cannot see it. A sweep with
//! `clippy::let_underscore_must_use` found 97 of them; 86 discard a `write!` into a
//! `String`, which cannot fail, or an `mpsc` send whose receiver has hung up, which
//! means the GUI is closing. These are the ones that were neither.

use fepdf_doc::remediation::HeuristicEngine;
use fepdf_model::Document;
use fepdf_model::ingest::IngestionOptions;

use fepdf_fixtures::assemble;

fn open(objects: &[&str]) -> Document {
    Document::open(bytes::Bytes::from(assemble(objects)), &IngestionOptions::default())
        .expect("the fixture reads")
}

/// A page whose content stream will not run says so, instead of yielding no candidates.
///
/// `HeuristicEngine::infer_structure` ran each page's content stream to collect its text
/// spans and then inferred headings, tables and paragraphs from them. The run was
/// `let _ = interpreter.execute_raw(&data);`, so a stream that failed produced an empty
/// span list and the page contributed nothing — **reported identically to a page the
/// engine read perfectly and found no structure in.** A caller asking "what structure
/// could be added to this document" was told "none here" about a page nobody had read.
///
/// The fixture shows text with no font selected, which `Tj` refuses (9.4.3 makes the
/// current font a precondition of a text-showing operator). Eight malformed streams were
/// tried to find one the interpreter actually rejects rather than tolerates; most of
/// what a file can get wrong here is repaired or ignored, which is exactly why the one
/// case that *is* an error must not vanish.
#[test]
fn a_page_whose_content_will_not_run_is_reported() {
    let doc = open(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>",
        "<< /Length 12 >>\nstream\nBT (x) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]);

    let before = doc.decisions.len();
    let engine = HeuristicEngine::new();
    let candidates = engine.infer_structure(&doc).expect("inference runs over the whole document");

    assert!(candidates.is_empty(), "nothing could be read, so nothing is inferred");
    assert!(
        doc.decisions.len() > before,
        "and the document says which page could not be read: {:?}",
        doc.decisions.entries()
    );
    assert!(
        doc.decisions.entries().iter().any(|d| d.clause == "7.8.2"),
        "named by the clause that defines a content stream: {:?}",
        doc.decisions.entries()
    );
}

/// A page that reads cleanly records nothing.
///
/// The other half: a log that fires on every document is a log nobody reads.
#[test]
fn a_page_that_reads_cleanly_records_nothing() {
    let doc = open(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
          /Resources << /Font << /F1 5 0 R >> >> >>",
        "<< /Length 42 >>\nstream\nBT /F1 24 Tf 72 700 Td (Hello) Tj ET\nendstream",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]);

    let engine = HeuristicEngine::new();
    engine.infer_structure(&doc).expect("inference runs");

    assert!(
        !doc.decisions.entries().iter().any(|d| d.clause == "7.8.2"),
        "a stream that ran records no failure: {:?}",
        doc.decisions.entries()
    );
}

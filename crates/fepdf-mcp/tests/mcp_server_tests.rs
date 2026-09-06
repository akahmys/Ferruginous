//! What the MCP tools do to a document, asserted against the document.
//!
//! **This suite asserted `is_ok()` and nothing else until 2026-09-06**, and it skipped
//! itself in silence when `samples/sample.pdf` was absent — which `.gitignore` makes it
//! on every machine but the one that generated it. So on a fresh clone every test here
//! passed without running, and on this machine they passed without checking: the
//! redaction tool was covered by one `assert!(res.is_ok())` and removed the wrong text
//! for as long as it had existed ([ADR-0064]).
//!
//! Two things follow, and both are the point of the rewrite:
//!
//! * **The fixtures are built here.** Nothing reads `samples/`, so nothing can skip, and
//!   what each test needs is visible in the test.
//! * **Every assertion is about the output document**, opened and read back. A tool that
//!   returned `Ok` having done nothing fails here.
//!
//! [ADR-0064]: ../../../docs/adr/0064-redaction-removed-the-second-run-of-a-page-and-no-other.md

use fepdf_mcp::McpError;
use fepdf_mcp::prompts::{prompt_audit_accessibility, prompt_remediate_pdf_ua};
use fepdf_mcp::resources::{
    read_audit_resource, read_metadata_resource, read_struct_tree_resource,
};
use fepdf_mcp::tools::{
    AddAnnotationArgs, AddPageDecorationArgs, ApplyBatesNumberingArgs, AuditArgs, ExtractTextArgs,
    OutlineNodeArg, RedactDocumentArgs, RedactionTarget, RemovePagesArgs, ReorderPagesArgs,
    RotatePagesArgs, UpdateOutlinesArgs, VerifySignaturesArgs, add_annotation_impl,
    add_page_decoration_impl, apply_bates_numbering_impl, apply_redaction_impl,
    audit_document_impl, extract_text_impl, remove_pages_impl, reorder_pages_impl,
    rotate_pages_impl, update_outlines_impl, verify_signatures_impl,
};
use std::fmt::Write as _;
use std::path::PathBuf;

// --- fixtures -------------------------------------------------------------------------

fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let table_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n");
    out.into_bytes()
}

/// `n` pages, page *i* carrying the text `Pi` at a known place, each 612x792.
fn pages(n: usize) -> Vec<u8> {
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")),
    ];
    for i in 0..n {
        let content = format!("BT /F1 24 Tf 72 700 Td (P{i}) Tj ET");
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 {} 0 R >> >> /Contents {} 0 R >>",
            3 + n * 2,
            4 + i * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string());
    assemble(&objects)
}

/// A path under the temporary directory, holding `bytes`.
fn written(name: &str, bytes: &[u8]) -> String {
    let mut p: PathBuf = std::env::temp_dir();
    p.push(format!("fepdf_mcp_{name}.pdf"));
    std::fs::write(&p, bytes).expect("the fixture is written");
    p.to_string_lossy().to_string()
}

fn out(name: &str) -> String {
    let mut p: PathBuf = std::env::temp_dir();
    p.push(format!("fepdf_mcp_out_{name}.pdf"));
    let _ = std::fs::remove_file(&p);
    p.to_string_lossy().to_string()
}

/// The text of one page of a document on disk — how these tests see what a tool did.
fn text_of(path: &str, page: usize) -> String {
    extract_text_impl(ExtractTextArgs {
        path: path.to_string(),
        page_range: Some(page.to_string()),
    })
    .expect("the output document reads back")
}

// --- tests ----------------------------------------------------------------------------

#[test]
fn an_io_error_keeps_its_message() {
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "PDF file missing");
    let mcp: McpError = io.into();
    assert!(format!("{mcp}").contains("PDF file missing"));
}

/// Extraction returns the page's own text, not merely a well-formed report.
#[test]
fn extract_text_returns_what_is_on_the_page() {
    let path = written("extract", &pages(2));
    let json = text_of(&path, 0);
    assert!(json.contains("total_pages"), "the report is shaped as documented: {json}");
    assert!(json.contains("P0"), "and carries page 0's text: {json}");
    assert!(!json.contains("P1"), "and only the page asked for: {json}");
}

/// Removing a page removes that page, and the others keep their order.
///
/// **`pages` counts from 1 and `page_range` counts from 0**, on the same surface, and
/// only the second used to say so. `pages: "2"` is the middle page; `page_range: "2"` is
/// the last. This test names both bases in one place so neither can drift into the other
/// unnoticed — which is the only reason the mixture is safe to keep.
///
/// The version this replaces asserted `is_ok()` on a chain of three operations and
/// re-opened none of them.
#[test]
fn remove_pages_counts_from_one_where_extraction_counts_from_zero() {
    let path = written("remove", &pages(3));
    let dest = out("remove");
    remove_pages_impl(RemovePagesArgs {
        input_path: path,
        output_path: dest.clone(),
        pages: "2".into(),
    })
    .expect("the tool runs");

    // "2" removed the middle page, so P0 and P2 are what is left — and `text_of` reaches
    // them by 0-based index, which is the other convention.
    assert!(text_of(&dest, 0).contains("P0"), "the first page stays");
    assert!(text_of(&dest, 1).contains("P2"), "and the last moves up into the gap");
}

/// Reordering moves the page it names.
#[test]
fn reorder_pages_moves_the_page_named() {
    let path = written("reorder", &pages(3));
    let dest = out("reorder");
    reorder_pages_impl(ReorderPagesArgs {
        input_path: path,
        output_path: dest.clone(),
        from: 2,
        to: 0,
    })
    .expect("the tool runs");

    assert!(text_of(&dest, 0).contains("P2"), "the page from the end is now first");
    assert!(text_of(&dest, 1).contains("P0"), "and the one that was first follows it");
}

/// Rotating writes the rotation into the page.
#[test]
fn rotate_pages_writes_the_rotation() {
    let path = written("rotate", &pages(1));
    let dest = out("rotate");
    rotate_pages_impl(RotatePagesArgs {
        input_path: path,
        output_path: dest.clone(),
        selection: Some("all".into()),
        angle: 90,
        relative: Some(true),
    })
    .expect("the tool runs");

    // Not a byte search: objects are packed into 7.5.7 streams by default (ADR-0016), so
    // the names are inside a compressed container. The engine's own reader is what sees
    // them, and a test that greps the file is testing the writer's compression setting.
    // A 90-degree rotation swaps the page's reported width and height, which is the
    // reader's own answer to "was it rotated" and needs no byte search.
    let doc = fepdf::PdfDocument::open(bytes::Bytes::from(
        std::fs::read(&dest).expect("the output exists"),
    ))
    .expect("the output opens");
    let (w, h) = doc.get_page_size(0).expect("the page has a size");
    assert!(w > h, "612x792 rotated a quarter turn reads as landscape, not {w}x{h}");
}

/// A decoration puts its text on the page.
#[test]
fn a_decoration_reaches_the_page() {
    let path = written("decorate", &pages(1));
    let dest = out("decorate");
    add_page_decoration_impl(AddPageDecorationArgs {
        input_path: path,
        output_path: dest.clone(),
        pages: Some("all".into()),
        text: "CONFIDENTIAL".into(),
        position: "top_center".into(),
        layer: None,
    })
    .expect("the tool runs");

    let text = text_of(&dest, 0);
    assert!(text.contains("CONFIDENTIAL"), "the decoration is on the page: {text}");
    assert!(text.contains("P0"), "and the page's own text is still there");
}

/// Bates numbering puts its number on the page.
#[test]
fn bates_numbering_reaches_the_page() {
    let path = written("bates", &pages(2));
    let dest = out("bates");
    apply_bates_numbering_impl(ApplyBatesNumberingArgs {
        input_path: path,
        output_path: dest.clone(),
        pages: None,
        prefix: Some("TEST-".into()),
        start_number: Some(100),
        digits: Some(6),
        position: Some("bottom_right".into()),
    })
    .expect("the tool runs");

    assert!(text_of(&dest, 0).contains("TEST-000100"), "page 0 carries the first number");
    assert!(text_of(&dest, 1).contains("TEST-000101"), "and page 1 the next");
}

/// An annotation reaches the page's `/Annots`.
#[test]
fn an_annotation_reaches_the_page() {
    let path = written("annot", &pages(1));
    let dest = out("annot");
    add_annotation_impl(AddAnnotationArgs {
        input_path: path,
        output_path: dest.clone(),
        page: 0,
        rect: [100.0, 100.0, 200.0, 150.0],
        contents: "Test Comment".into(),
        kind: Some("text".into()),
    })
    .expect("the tool runs");

    let report =
        fepdf::InteractiveReport::survey(&std::fs::read(&dest).expect("the output exists"))
            .expect("the output surveys");
    assert!(!report.is_empty(), "the document is no longer free of interactive features");
    assert_eq!(report.annotations.total, 1, "one annotation, the one asked for");
    assert_eq!(report.annotations.pages_with, 1, "on the one page asked for");
}

/// An outline reaches the catalogue.
#[test]
fn an_outline_reaches_the_catalogue() {
    let path = written("outline", &pages(2));
    let dest = out("outline");
    update_outlines_impl(UpdateOutlinesArgs {
        input_path: path,
        output_path: dest.clone(),
        roots: vec![OutlineNodeArg {
            title: "Chapter 1".into(),
            dest_page: Some(0),
            children: None,
        }],
    })
    .expect("the tool runs");

    let report =
        fepdf::InteractiveReport::survey(&std::fs::read(&dest).expect("the output exists"))
            .expect("the output surveys");
    assert!(report.outline.present, "the catalogue names an outline");
    assert_eq!(report.outline.total, 1, "carrying the one item asked for");
}

/// **Redaction removes the text under the rectangle, and says how many it removed.**
///
/// The version this replaces was one `assert!(res.is_ok())`, and the tool removed the
/// wrong run of a page for as long as it had existed. The count is the second half: it
/// reported `args.targets.len()` under a field documented "Number of redactions
/// successfully scrubbed", so a rectangle over empty space was reported to the caller as
/// a redaction that had happened.
#[test]
fn redaction_removes_the_text_and_reports_what_it_removed() {
    let path = written("redact", &pages(1));

    let dest = out("redact_hit");
    let report = apply_redaction_impl(RedactDocumentArgs {
        input_path: path.clone(),
        output_path: dest.clone(),
        targets: vec![RedactionTarget { page: 0, rect: [0.0, 0.0, 612.0, 792.0] }],
    })
    .expect("the tool runs");

    assert!(!text_of(&dest, 0).contains("P0"), "the page's text is gone");
    assert!(report.contains("\"redacted_count\": 1"), "and the count is what went: {report}");

    let missed = out("redact_miss");
    let report = apply_redaction_impl(RedactDocumentArgs {
        input_path: path,
        output_path: missed.clone(),
        targets: vec![RedactionTarget { page: 0, rect: [0.0, 0.0, 1.0, 1.0] }],
    })
    .expect("the tool runs");

    assert!(text_of(&missed, 0).contains("P0"), "a rectangle over nothing removes nothing");
    assert!(
        report.contains("\"redacted_count\": 0"),
        "and says so, rather than reporting the rectangle it was given: {report}"
    );
}

/// The read-only tools answer about the document rather than merely succeeding.
#[test]
fn the_reporting_tools_answer_about_the_document() {
    let path = written("report", &pages(2));

    let audit = audit_document_impl(AuditArgs { path: path.clone() }).expect("audit runs");
    assert!(audit.contains("\"status\""), "the report carries a verdict: {audit}");
    assert!(
        audit.contains("Structural Tree Root"),
        "and names what an untagged document is missing: {audit}"
    );

    let signatures =
        verify_signatures_impl(VerifySignaturesArgs { path: path.clone(), allow_network: false })
            .expect("verification runs");
    assert!(
        signatures.to_lowercase().contains("no digital signatures"),
        "an unsigned document says so in words rather than returning an empty report: \
         {signatures}"
    );

    assert!(read_metadata_resource(&path).expect("metadata reads").contains('{'));
    assert!(read_audit_resource(&path).expect("the audit resource reads").contains('{'));
    // An untagged document has no structure tree, and the resource says `null` rather
    // than an empty object. Asserted as `null` and not merely "it returned", because
    // "there is no tree" and "I could not read the tree" must not look the same to an
    // agent — and here they would.
    assert_eq!(
        read_struct_tree_resource(&path).expect("the struct tree resource reads").trim(),
        "null",
        "an untagged document has no tree, and says so"
    );
}

/// The prompts name what they are for.
#[test]
fn the_prompts_name_their_subject() {
    let path = written("prompts", &pages(1));
    assert!(prompt_audit_accessibility(&path).contains("PDF/UA-2"));
    assert!(prompt_remediate_pdf_ua(&path, "output.pdf").contains("remediation"));
}

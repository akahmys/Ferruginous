//! What adding a decoration must not cost the page it is added to.
//!
//! Overlaying a header, a footer or a Bates number needs Helvetica in the page's
//! resources, and the code that put it there reached for `/Resources` on the page
//! dictionary alone — building a fresh empty one when it was not found. Two shapes the
//! standard allows break under that:
//!
//! - `/Resources` written as an indirect reference. `as_dict_handle` does not resolve
//!   one, so the page's resources were **replaced** by the new empty dictionary.
//! - `/Resources` inherited from the page tree (7.7.3.4), which a document with uniform
//!   pages normally does. The fresh dictionary on the page **shadowed** it.
//!
//! Either way the fonts and XObjects the page's own content stream names stopped
//! resolving, and the page came out blank with a header on it. These fixtures are the two
//! shapes; what they assert is that the text that was there is still drawn.

use fepdf::{IngestionOptions, PdfDocument};
use fepdf_doc::operation::{DecorationPosition, Operation, PageSelection};
use kurbo::Affine;

use fepdf_fixtures::recorder::Recorder;

use fepdf_fixtures::assemble;

/// A one-page file drawing `ORIGINAL`, with its resources placed by `resources_on_page`
/// and `resources_on_tree` — the two ways 7.7.3.4 lets a page reach them.
fn page_drawing_original(resources_on_page: &str, resources_on_tree: &str) -> Vec<u8> {
    let content = "BT /F1 24 Tf 1 0 0 1 40 700 Tm (ORIGINAL) Tj ET";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [3 0 R] /Count 1 {resources_on_tree} >>"),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] {resources_on_page} \
             /Contents 4 0 R >>"
        ),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Font << /F1 6 0 R >> >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    assemble(&bodies)
}

/// Decorates page 1 and reports the text the page then draws.
fn text_after_decorating(file: Vec<u8>) -> String {
    let mut doc = PdfDocument::open_with_options(file.into(), &IngestionOptions::default())
        .expect("the fixture opens");
    doc.apply(Operation::AddPageDecoration {
        pages: PageSelection::All,
        text: "HEADER".to_string(),
        position: DecorationPosition::TopCenter,
        layer: None,
    })
    .expect("the decoration applies");
    let mut text = Recorder::new();
    doc.render_page(0, &mut text, Affine::IDENTITY).expect("the page interprets");
    text.text()
}

#[test]
fn a_page_whose_resources_are_indirect_keeps_them() {
    let drawn = text_after_decorating(page_drawing_original("/Resources 5 0 R", ""));
    assert!(drawn.contains("ORIGINAL"), "the page's own text was lost: {drawn:?}");
    assert!(drawn.contains("HEADER"), "the decoration was not added: {drawn:?}");
}

#[test]
fn a_page_that_inherits_its_resources_keeps_them() {
    let drawn = text_after_decorating(page_drawing_original("", "/Resources 5 0 R"));
    assert!(drawn.contains("ORIGINAL"), "the inherited resources were shadowed: {drawn:?}");
    assert!(drawn.contains("HEADER"), "the decoration was not added: {drawn:?}");
}

/// The shape that already worked, kept so a fix to the two above cannot quietly break it.
#[test]
fn a_page_carrying_its_own_resources_keeps_them() {
    let drawn =
        text_after_decorating(page_drawing_original("/Resources << /Font << /F1 6 0 R >> >>", ""));
    assert!(drawn.contains("ORIGINAL"), "the page's own text was lost: {drawn:?}");
    assert!(drawn.contains("HEADER"), "the decoration was not added: {drawn:?}");
}

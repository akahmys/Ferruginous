//! The clauses an audit reports having met are the clauses the document carries.
//!
//! **`fepdf inspect audit` reported none of the font clauses until 2026-09-06.**
//! `samples/constitution.pdf` read `Validated Clauses: 14.3.3, 7.7.2, 7.7.3.3, 9.2, 9.8`
//! for a document whose every font is a TrueType: 9.6.3 was absent from a list of what
//! the document met.
//!
//! `font/schema.rs` declares `PdfType1Font`, `PdfTrueTypeFont` and `PdfType0Font`,
//! carrying clauses 9.6.2, 9.6.3 and 9.7. All three were **referenced by nothing** —
//! found in a sweep for public items no code names — while `audit_specific_types` beside
//! them dispatched on `/Subtype` for OpenType, CIDFontType0 and CIDFontType2. The three
//! most common font types were the three it did not ask about.
//!
//! Deleting them was the other option. They are the shape ADR-0017 warns of — a
//! declaration that reads nothing is not modelling — but here the reader was one match
//! arm away, and a clause the engine parses and does not report is a coverage figure
//! that understates itself.

mod common;
use common::assemble;
use fepdf::PdfDocument;

fn clauses_for(font: &str) -> Vec<String> {
    let doc = PdfDocument::open(bytes::Bytes::from(assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
          /Resources << /Font << /F1 4 0 R >> >> >>"
            .to_string(),
        font.to_string(),
    ])))
    .expect("the fixture opens");
    doc.get_summary().expect("the audit runs").compliance.iso_clauses
}

/// A Type 1 font makes 9.6.2 a clause the document presented.
#[test]
fn a_type1_font_reports_its_clause() {
    let clauses = clauses_for(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    assert!(clauses.contains(&"9.6.2".to_string()), "{clauses:?}");
}

/// So does a TrueType font, under 9.6.3.
#[test]
fn a_truetype_font_reports_its_clause() {
    let clauses = clauses_for(
        "<< /Type /Font /Subtype /TrueType /BaseFont /Arial /Encoding /WinAnsiEncoding >>",
    );
    assert!(clauses.contains(&"9.6.3".to_string()), "{clauses:?}");
}

/// And a Type 0 font under 9.7 — the one every CJK document in the corpus carries.
#[test]
fn a_type0_font_reports_its_clause() {
    let clauses = clauses_for(
        "<< /Type /Font /Subtype /Type0 /BaseFont /X /Encoding /Identity-H \
          /DescendantFonts [] >>",
    );
    assert!(clauses.contains(&"9.7".to_string()), "{clauses:?}");
}

/// A document with no font reports none of them.
///
/// Without this, an audit that inserted all three unconditionally would pass the three
/// above and make the clause list a constant.
#[test]
fn a_document_with_no_font_reports_no_font_clause() {
    let doc = PdfDocument::open(bytes::Bytes::from(assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_string(),
    ])))
    .expect("the fixture opens");
    let clauses = doc.get_summary().expect("the audit runs").compliance.iso_clauses;
    for font_clause in ["9.6.2", "9.6.3", "9.7"] {
        assert!(!clauses.contains(&font_clause.to_string()), "{font_clause} in {clauses:?}");
    }
}

//! One PDF assembler for this crate's integration tests.
//!
//! **It was eighteen copies across five crates on 2026-09-06**, twelve of them byte for
//! byte the same eighteen lines, and this session added two more before counting. Ten of
//! them are in this directory and are now this one.
//!
//! Written by hand rather than by `PdfWriter`, deliberately: a fixture the writer
//! produces cannot catch a writer defect, and a test that builds its own bytes says in
//! its own body what the reader is being given.

use std::fmt::Write as _;

/// A one-revision PDF whose objects are `bodies`, numbered from 1, with `/Root 1 0 R`.
///
/// The caller writes the objects; this writes the cross-reference table and the trailer,
/// which is the part every copy had identically and the part that is tedious to get
/// right.
pub fn assemble(bodies: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let table_at = out.len();
    let size = bodies.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n");
    out.into_bytes()
}

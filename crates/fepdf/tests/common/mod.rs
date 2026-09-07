//! One PDF assembler for this crate's integration tests.
//!
//! **It was eighteen copies across five crates on 2026-09-06**, twelve of them byte for
//! byte the same eighteen lines, and this session added two more before counting. Ten of
//! them are in this directory and are now this one; twelve more followed on 2026-09-08,
//! which had differed only in the header version they wrote and in whether `/Root` was
//! hard-coded.
//!
//! Three files still assemble their own, and correctly. `sdk_tests.rs` asserts
//! `arena().version() == 1.7`, so its `%PDF-1.7` header is the subject of a test rather
//! than a leftover, and it needs an `/ID` in the trailer for the encryption tests.
//! `image_sample_count_test.rs` and `smask_in_data_test.rs` put raw sample and codestream
//! bytes in their objects, which a `String` cannot carry — the bound this signature draws.
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
    assemble_with_root(bodies, 1)
}

/// The same, for a document whose catalogue is not object 1.
pub fn assemble_with_root(bodies: &[String], root: usize) -> Vec<u8> {
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
    let _ =
        write!(out, "trailer\n<< /Size {size} /Root {root} 0 R >>\nstartxref\n{table_at}\n%%EOF\n");
    out.into_bytes()
}

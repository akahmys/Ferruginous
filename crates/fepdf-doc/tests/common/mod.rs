//! One PDF assembler for this crate's integration tests.
//!
//! **The consolidation that recorded eighteen copies across five crates on 2026-09-06 did
//! not reach this one.** `discarded_results_test` and `recursion_bounds_test` carried the
//! same seventeen lines. `fepdf` and `fepdf-script` have their own copy of this module for
//! the same reason: a `tests/common/` cannot be shared across crates without a
//! dev-dependency crate, so each consolidates its own.
//!
//! Written by hand rather than by `PdfWriter`, deliberately: a fixture the writer produces
//! cannot catch a writer defect.
//!
//! Takes `&[&str]` where the other two take `&[String]`, which is what this crate's
//! callers pass.

pub fn assemble(objects: &[&str]) -> Vec<u8> {
    use std::fmt::Write as _;
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

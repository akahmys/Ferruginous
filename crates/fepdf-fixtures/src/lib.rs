//! PDFs assembled by hand, for tests and examples across the workspace.
//!
//! **It was thirty-two hand-written assemblers on 2026-09-06**, eighteen of them across
//! five crates and twelve byte for byte the same eighteen lines. `crates/fepdf/tests/`
//! took ten of them into one on that day and eleven more on 2026-09-08; this crate is
//! where that one goes so the rest of the workspace can reach it.
//!
//! Two constraints shape what is here, and both were paid for:
//!
//! - **This crate does not depend on `fepdf`.** A fixture the writer produces cannot
//!   catch a writer defect, and a test that builds its own bytes says in its own body
//!   what the reader is being given.
//! - **An object body is bytes, not a `String`.** The `String` signature the first
//!   version had is why two image tests could not use it: raw sample bytes and a JPX
//!   codestream do not survive a `String`.
//!
//! **What is deliberately not here** is anything that writes a cross-reference table for
//! a test whose subject is the cross-reference table. `fepdf-syntax/src/xref.rs` and
//! `fepdf-model/src/reader.rs` write their own subsections, hybrid entries and malformed
//! tables, and an assembler that got those right for them would remove what they check.

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::fmt::Write as _;

#[cfg(feature = "backend")]
pub mod recorder;

/// A one-revision PDF whose objects are `bodies`, numbered from 1, with `/Root 1 0 R`.
///
/// The caller writes the objects; this writes the cross-reference table and the trailer,
/// which is the part every copy had identically and the part that is tedious to get
/// right.
///
/// ```text
/// let pdf = fepdf_fixtures::assemble(&[
///     "<< /Type /Catalog /Pages 2 0 R >>",
///     "<< /Type /Pages /Kids [] /Count 0 >>",
/// ]);
/// ```
pub fn assemble<B: AsRef<[u8]>>(bodies: &[B]) -> Vec<u8> {
    Pdf::new().assemble(bodies)
}

/// How the trailer and the header are written, for the fixtures that need them said
/// differently.
///
/// Defaults to what [`assemble`] writes: `%PDF-2.0`, `/Root 1 0 R`, and no other trailer
/// entry.
#[derive(Debug, Clone)]
pub struct Pdf {
    version: String,
    root: usize,
    trailer_entries: String,
}

impl Default for Pdf {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdf {
    /// A one-revision PDF 2.0 whose catalogue is object 1.
    pub fn new() -> Self {
        Self { version: "2.0".to_string(), root: 1, trailer_entries: String::new() }
    }

    /// The version in the header line, as it is written there — `"1.7"`, `"2.0"`.
    ///
    /// **A fixture that says 1.7 is usually saying nothing**, so set this only where the
    /// version is the subject: `crates/fepdf/tests/sdk_tests.rs` asserts
    /// `arena().version() == 1.7` and is the reason this exists.
    #[must_use]
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Which object the trailer's `/Root` points at. Object 1 unless said otherwise.
    #[must_use]
    pub const fn root(mut self, root: usize) -> Self {
        self.root = root;
        self
    }

    /// Entries added to the trailer dictionary verbatim, as they would be written in the
    /// file: `"/Encrypt 5 0 R"`, `"/ID [<0123…> <0123…>]"`.
    ///
    /// Raw rather than typed because every caller of this is testing what the reader does
    /// with particular bytes, and a typed entry would decide the bytes for them.
    #[must_use]
    pub fn trailer_entries(mut self, entries: &str) -> Self {
        self.trailer_entries = entries.to_string();
        self
    }

    /// The file, with `bodies` as objects 1 to *n*.
    pub fn assemble<B: AsRef<[u8]>>(&self, bodies: &[B]) -> Vec<u8> {
        let mut out = format!("%PDF-{}\n", self.version).into_bytes();
        let mut offsets = Vec::with_capacity(bodies.len());
        for (index, body) in bodies.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            out.extend_from_slice(body.as_ref());
            out.extend_from_slice(b"\nendobj\n");
        }
        let table_at = out.len();
        out.extend_from_slice(&self.table_and_trailer(&offsets, table_at));
        out
    }

    /// The `xref` table for objects 1..n, and the trailer after it.
    fn table_and_trailer(&self, offsets: &[usize], table_at: usize) -> Vec<u8> {
        let size = offsets.len() + 1;
        let mut out = format!("xref\n0 {size}\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        out.push_str(&self.trailer(size, table_at));
        out.into_bytes()
    }

    fn trailer(&self, size: usize, table_at: usize) -> String {
        let mut out = format!("trailer\n<< /Size {size} /Root {} 0 R", self.root);
        if !self.trailer_entries.is_empty() {
            let _ = write!(out, " {}", self.trailer_entries);
        }
        let _ = write!(out, " >>\nstartxref\n{table_at}\n%%EOF\n");
        out
    }
}

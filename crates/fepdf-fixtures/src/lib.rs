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

/// One text field of an interactive form (12.7.4.3).
#[derive(Debug, Clone)]
pub struct FormField {
    /// `/T`, the field's partial name.
    pub name: String,
    /// `/V`, its value as the file has it — which for a calculated field is whatever it
    /// was saved with, and need not be what the script would compute.
    pub value: String,
    /// The ECMAScript of `/AA` `/C`, when the field is calculated (12.6.3).
    ///
    /// Written as it appears inside the PDF string, so `(` and `)` need escaping:
    /// `r"event.value = this.getField\('a'\).value;"`.
    pub calculate: Option<String>,
}

impl FormField {
    /// A field holding `value` and calculating nothing.
    pub fn new(name: &str, value: &str) -> Self {
        Self { name: name.to_string(), value: value.to_string(), calculate: None }
    }

    /// The same, computing its value from `script` when the calculation order runs it.
    #[must_use]
    pub fn calculating(mut self, script: &str) -> Self {
        self.calculate = Some(script.to_string());
        self
    }
}

/// A one-page document whose `/AcroForm` carries `fields`, with `calculated` naming by
/// index which of them `/CO` lists.
///
/// **The tedious part is the numbering**, and it is why this is here rather than in five
/// test files: the fields are objects 5 onward, `/Fields` and `/Annots` must both list
/// them, and `/CO` names the same objects in the order 12.6.3 runs them. Getting one of
/// those three wrong makes a form that opens and quietly calculates nothing.
///
/// ```text
/// let file = acroform(
///     &[
///         FormField::new("a", "2"),
///         FormField::new("total", "0").calculating(r"event.value = 2;"),
///     ],
///     &[1],
/// );
/// ```
pub fn acroform(fields: &[FormField], calculated: &[usize]) -> Vec<u8> {
    const FIRST: usize = 5;
    let refs = |indices: &[usize]| {
        indices.iter().map(|i| format!("{} 0 R", FIRST + i)).collect::<Vec<_>>().join(" ")
    };
    let all: Vec<usize> = (0..fields.len()).collect();
    let order =
        if calculated.is_empty() { String::new() } else { format!("/CO [{}] ", refs(calculated)) };

    let mut bodies = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{}] \
             {order}/DA (/Helv 9 Tf 0 g) >> >>",
            refs(&all)
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [{}] \
             /Contents 4 0 R >>",
            refs(&all)
        ),
        "<< /Length 0 >>\nstream\n\nendstream".to_string(),
    ];
    for field in fields {
        let calculate = field
            .calculate
            .as_ref()
            .map_or_else(String::new, |js| format!("/AA << /C << /S /JavaScript /JS ({js}) >> >>"));
        bodies.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({}) /V ({}) \
             /Rect [0 0 100 20] /F 4 /DA (/Helv 9 Tf 0 g) {calculate} >>",
            field.name, field.value
        ));
    }
    assemble(&bodies)
}

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

use crate::document::PdfCatalog;
use crate::document::page::{PdfAnnotation, PdfPageDict};
use crate::font::schema::{
    PdfCIDFont, PdfFont, PdfFontDescriptor, PdfOpenTypeFont, PdfTrueTypeFont, PdfType0Font,
    PdfType1Font, PdfType3Font,
};
use crate::graphics::schema::PdfExtGState;
use crate::metadata::PdfInfo;
use crate::{Document, FromPdfObject, Object, PdfSchema};
use std::collections::BTreeSet;

#[derive(Debug, Default)]
/// What a compliance pass observed.
pub struct ComplianceReport {
    /// ISO clauses the document exercised.
    pub clauses_encountered: BTreeSet<&'static str>,
    /// Problems found.
    pub issues: Vec<String>,
}

/// Walks a document recording clause coverage and violations.
pub struct ComplianceAuditor<'a> {
    doc: &'a Document,
    report: ComplianceReport,
}

impl<'a> ComplianceAuditor<'a> {
    /// Prepares an audit of `doc`.
    pub fn new(doc: &'a Document) -> Self {
        Self { doc, report: ComplianceReport::default() }
    }

    /// Runs the audit and returns its report.
    pub fn audit(mut self) -> ComplianceReport {
        let arena = self.doc.arena();
        let root_handle = *self.doc.root_handle();

        // Decisions taken while *reading* are deliberately not folded in here. They are
        // a different category from an audit finding (ARCHITECTURE.md §4.3), and
        // stringifying them cost their severity: every one arrived at the CLI as
        // `IssueSeverity::Warning`, so a `Violation` and a `Repaired` were reported
        // identically, and JSON consumers were told "Warning" about something the
        // engine had classified as `Repaired`. `DocumentSummary::decisions` now carries
        // the log itself, with its severities intact.

        // 1. Audit Catalog
        if let Some(obj) = arena.get_object(root_handle) {
            if let Err(e) = PdfCatalog::from_pdf_object(obj, arena) {
                self.report.issues.push(format!(
                    "Catalog Error ({}): {:?}",
                    PdfCatalog::iso_clause(),
                    e
                ));
            } else {
                self.report.clauses_encountered.insert(PdfCatalog::iso_clause());
            }
        }

        // 2. Audit Info
        if let Some(info_handle) = self.doc.info_handle()
            && let Some(obj) = arena.get_object(info_handle)
        {
            if let Err(e) = PdfInfo::from_pdf_object(obj, arena) {
                self.report.issues.push(format!("Info Error ({}): {:?}", PdfInfo::iso_clause(), e));
            } else {
                self.report.clauses_encountered.insert(PdfInfo::iso_clause());
            }
        }

        // 3. Scan Arena for Fonts and ExtGState
        for i in 0..arena.object_count() {
            let handle = crate::handle::Handle::new(i);
            self.audit_object(handle, i == root_handle.index());
        }

        self.report
    }

    fn audit_object(&mut self, handle: crate::handle::Handle<Object>, is_root: bool) {
        let arena = self.doc.arena();
        let Some(obj) = arena.get_object(handle) else { return };
        let resolved = obj.resolve(arena);
        let Object::Dictionary(dh) = resolved else { return };
        let dict = arena.get_dict(dh).unwrap_or_default();

        // Try parsing as Font
        if dict.contains_key(&arena.name("BaseFont"))
            && PdfFont::from_pdf_object(obj.clone(), arena).is_ok()
        {
            self.report.clauses_encountered.insert(PdfFont::iso_clause());
        }

        // Try parsing as FontDescriptor
        if dict.contains_key(&arena.name("FontName"))
            && dict.contains_key(&arena.name("Flags"))
            && PdfFontDescriptor::from_pdf_object(obj.clone(), arena).is_ok()
        {
            self.report.clauses_encountered.insert(PdfFontDescriptor::iso_clause());
        }

        self.audit_specific_types(&dict, &obj, arena);

        // Check for interactive root keys in Catalog
        if is_root {
            if dict.contains_key(&arena.name("AcroForm")) {
                self.report.clauses_encountered.insert("12.7");
            }
            if dict.contains_key(&arena.name("Names")) {
                self.report.clauses_encountered.insert("7.7.4");
            }
            if dict.contains_key(&arena.name("Outlines")) {
                self.report.clauses_encountered.insert("12.3.3");
            }
        }
    }

    fn audit_specific_types(
        &mut self,
        dict: &std::collections::BTreeMap<crate::handle::Handle<crate::PdfName>, Object>,
        obj: &Object,
        arena: &crate::PdfArena,
    ) {
        self.audit_font_subtypes(dict, obj, arena);
        self.audit_structural_types(dict, obj, arena);
    }

    /// Records the clause for whichever font subtype `/Subtype` declares.
    ///
    /// A font's subtype decides which clause of 9.6/9.7 defines it, so the clause
    /// recorded is the one the document itself named. A dictionary that declares a
    /// subtype but does not parse as it records nothing: the clause list says what
    /// the document met, not what it attempted.
    fn audit_font_subtypes(
        &mut self,
        dict: &std::collections::BTreeMap<crate::handle::Handle<crate::PdfName>, Object>,
        obj: &Object,
        arena: &crate::PdfArena,
    ) {
        let Some(subtype) = name_at(dict, arena, "Subtype") else { return };
        let met = match subtype.as_str() {
            "Type1" | "MMType1" => PdfType1Font::from_pdf_object(obj.clone(), arena)
                .is_ok()
                .then(PdfType1Font::iso_clause),
            "TrueType" => PdfTrueTypeFont::from_pdf_object(obj.clone(), arena)
                .is_ok()
                .then(PdfTrueTypeFont::iso_clause),
            "Type3" => PdfType3Font::from_pdf_object(obj.clone(), arena)
                .is_ok()
                .then(PdfType3Font::iso_clause),
            "Type0" => PdfType0Font::from_pdf_object(obj.clone(), arena)
                .is_ok()
                .then(PdfType0Font::iso_clause),
            "OpenType" => PdfOpenTypeFont::from_pdf_object(obj.clone(), arena)
                .is_ok()
                .then(PdfOpenTypeFont::iso_clause),
            "CIDFontType0" | "CIDFontType2" => {
                PdfCIDFont::from_pdf_object(obj.clone(), arena).is_ok().then(PdfCIDFont::iso_clause)
            }
            _ => None,
        };
        if let Some(clause) = met {
            self.report.clauses_encountered.insert(clause);
        }
    }

    /// Records the clauses for the non-font types a dictionary may declare.
    fn audit_structural_types(
        &mut self,
        dict: &std::collections::BTreeMap<crate::handle::Handle<crate::PdfName>, Object>,
        obj: &Object,
        arena: &crate::PdfArena,
    ) {
        if let Some(type_name) = name_at(dict, arena, "Type") {
            if type_name == "ExtGState" && PdfExtGState::from_pdf_object(obj.clone(), arena).is_ok()
            {
                self.report.clauses_encountered.insert(PdfExtGState::iso_clause());
            }
            if type_name == "Page" && PdfPageDict::from_pdf_object(obj.clone(), arena).is_ok() {
                self.report.clauses_encountered.insert(PdfPageDict::iso_clause());
            }
        }

        // An annotation is known by carrying a `/Subtype` at all: 12.5.2 makes the
        // entry required, and the subtypes of Table 171 are open-ended.
        if name_at(dict, arena, "Subtype").is_some()
            && PdfAnnotation::from_pdf_object(obj.clone(), arena).is_ok()
        {
            self.report.clauses_encountered.insert(PdfAnnotation::iso_clause());
        }
    }
}

/// Reads `key` from `dict` as a name, if it is one.
fn name_at(
    dict: &std::collections::BTreeMap<crate::handle::Handle<crate::PdfName>, Object>,
    arena: &crate::PdfArena,
    key: &str,
) -> Option<String> {
    let handle = dict.get(&arena.name(key))?.as_name()?;
    Some(arena.get_name(handle)?.as_str().to_string())
}

//! A document that opens encrypted asks for its password rather than showing blank pages.
//!
//! **The engine does not refuse a locked document**, and should not: 7.6 leaves the file
//! structure outside the encryption, so the page count and the catalogue are readable
//! while the content is not. It records a 7.6.1 `Violation` saying so. Until 2026-09-09
//! this crate passed no password and read no such decision, so an encrypted PDF opened as
//! a document with the right number of blank pages and nothing to say otherwise.
//!
//! What is checked here is the decision the prompt is built on, which is the part that
//! can go quiet without anyone noticing.

use fepdf::{IngestionOptions, PdfDocument};

/// What `worker::still_locked` reads. Kept in step by this test rather than by memory.
fn locked_reason(doc: &PdfDocument) -> Option<String> {
    doc.decisions().iter().find_map(|d| {
        (d.clause == "7.6.1" && d.found.contains("could not be unlocked")).then(|| d.found.clone())
    })
}

fn open(password: Option<&str>) -> PdfDocument {
    let data = std::fs::read(LOCKED).expect("the fixture reads");
    let options =
        IngestionOptions { password: password.map(str::to_string), ..IngestionOptions::default() };
    PdfDocument::open_with_options(data.into(), &options).expect("a locked document still opens")
}

/// `aes256_owner.pdf` needs `userpw`, which `crosscheck_roundtrip.sh` also knows. Built by
/// `scripts/test/make_encrypted.py`; absent until someone runs it, hence the guard.
const LOCKED: &str = "../../target/encrypted/aes256_owner.pdf";

fn corpus_present() -> bool {
    std::path::Path::new(LOCKED).exists()
}

#[test]
fn a_locked_document_says_so_and_names_its_handler() {
    if !corpus_present() {
        return;
    }
    let reason = locked_reason(&open(None)).expect("a locked document reports 7.6.1");
    assert!(
        reason.starts_with("Password Security"),
        "the handler's name is what the prompt puts above the field: {reason}"
    );
}

/// The right password clears it, which is what makes the prompt worth showing.
#[test]
fn the_password_unlocks_it_and_the_decision_goes() {
    if !corpus_present() {
        return;
    }
    let doc = open(Some("userpw"));
    assert!(locked_reason(&doc).is_none(), "it unlocked, so nothing should still be asking");
    assert!(doc.page_count().expect("pages") > 0, "and its pages are readable");
}

/// A wrong password is not silence: it comes back locked, which is what the second
/// attempt's message depends on.
#[test]
fn a_wrong_password_still_reports_locked() {
    if !corpus_present() {
        return;
    }
    assert!(
        locked_reason(&open(Some("not the password"))).is_some(),
        "a refused password leaves the document locked and saying so"
    );
}

/// The other half: this crate could sign a document and not protect one.
///
/// `SaveOptions::password` was never set from the GUI until 2026-09-09, so the engine's
/// encryption path — every scheme it writes — was unreachable from the only frontend a
/// person clicks. These check the contract the export wizard now depends on.
mod writing {
    use super::{IngestionOptions, PdfDocument};
    use fepdf::SaveOptions;

    fn saved_with(options: &SaveOptions) -> Vec<u8> {
        let doc = PdfDocument::create_empty().expect("an empty document");
        let out = std::env::temp_dir().join("fepdf_gui_encryption_test.pdf");
        doc.save_with_options(&out, "2.0", options).expect("it saves");
        std::fs::read(&out).expect("the output reads")
    }

    /// Whether the document is still locked, by either of the two routes.
    ///
    /// **A locked document reaches a reader two ways**, and this crate only handled one
    /// until the second test below failed. With a plain cross-reference table, 7.6 leaves
    /// the structure outside the encryption and the document opens carrying a 7.6.1
    /// `Violation`. With the structure in object streams (7.5.7) — which is what this
    /// engine writes by default — there is nothing to read and the open returns an error.
    fn locked(bytes: &[u8], password: Option<&str>) -> bool {
        let options = IngestionOptions {
            password: password.map(str::to_string),
            ..IngestionOptions::default()
        };
        match PdfDocument::open_with_options(bytes.to_vec().into(), &options) {
            Ok(doc) => doc
                .decisions()
                .iter()
                .any(|d| d.clause == "7.6.1" && d.found.contains("could not be unlocked")),
            Err(e) => format!("{e:?}").contains("was not unlocked"),
        }
    }

    #[test]
    fn a_password_reaches_the_written_file() {
        let bytes = saved_with(&SaveOptions {
            password: Some("secret".to_string()),
            ..SaveOptions::default()
        });
        assert!(locked(&bytes, None), "the output opened without the password it was saved with");
        assert!(!locked(&bytes, Some("secret")), "the password it was saved with did not open it");
    }

    /// The control, and the reason the checkbox drops the password rather than hiding it.
    #[test]
    fn no_password_writes_an_unprotected_document() {
        let bytes = saved_with(&SaveOptions::default());
        assert!(!locked(&bytes, None), "an unasked-for encryption was written");
    }
}

/// The phrase both routes hand the prompt, checked against the engine rather than quoted.
///
/// The dialog puts the handler's name above the password field — "Password Security
/// (AES-256)" — and takes it from two different places depending on how the document was
/// packed. If either wording moves, the prompt says "This document" instead, which is
/// worse and silent.
#[test]
fn both_routes_name_the_handler_the_same_way() {
    let doc = PdfDocument::create_empty().expect("an empty document");
    let out = std::env::temp_dir().join("fepdf_gui_handler_name.pdf");
    doc.save_with_options(
        &out,
        "2.0",
        &fepdf::SaveOptions { password: Some("x".to_string()), ..fepdf::SaveOptions::default() },
    )
    .expect("it saves");

    let bytes = std::fs::read(&out).expect("the output reads");
    let error = PdfDocument::open_with_options(bytes.into(), &IngestionOptions::default())
        .err()
        .expect("a packed encrypted document does not open without its key");
    let text = format!("{error:?}");
    assert!(
        text.contains("Password Security (AES-256) was not unlocked"),
        "the phrase the prompt splits on has moved: {text}"
    );

    if corpus_present() {
        let found = locked_reason(&open(None)).expect("the other route reports 7.6.1");
        assert!(
            found.starts_with("Password Security (AES-256)"),
            "the two routes name the handler differently: {found}"
        );
    }
}

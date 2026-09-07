//! What `AddLtvInfo` writes, which nothing checked.
//!
//! The operation's own documentation said so — "**Untested, and the only piece of `/DSS`
//! that exists**" — and it stayed true while the operation became an `fepdf-mcp` tool a
//! client can call. `operation_json_tests` covers the variant's JSON round trip, which is
//! the vocabulary's shape and not what applying it does; between them, no test opened the
//! catalogue afterwards.
//!
//! `/DSS` (12.8.4.3) is the Document Security Store: the certificates a reader needs to
//! validate a signature after the signing certificate has expired. Writing it wrong is not
//! visible in the document — a viewer shows the same page either way, and the cost lands
//! years later on whoever tries to validate.

use fepdf_doc::{Operation, apply_operation};
use fepdf_model::{Document, Object, ingest::IngestionOptions};

mod common;
use common::assemble;

fn document() -> Document {
    Document::open(
        bytes::Bytes::from(assemble(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
        ])),
        &IngestionOptions::default(),
    )
    .expect("the fixture reads")
}

/// The certificates arrive as streams, in the order they were given.
#[test]
fn every_certificate_reaches_the_store_as_its_own_stream() {
    let mut doc = document();
    let first = vec![0x30u8, 0x82, 0x01, 0x0a];
    let second = vec![0x30u8, 0x82, 0x02, 0x0b];
    apply_operation(
        &mut doc,
        Operation::AddLtvInfo { certificates: vec![first.clone(), second.clone()] },
    )
    .expect("the store is written");

    let arena = doc.arena();
    let catalog_h = doc.catalog_handle().expect("the fixture has a catalogue");
    let catalog = arena.get_dict(doc.resolve_to_dict(catalog_h).expect("resolves")).expect("read");

    let Some(Object::Dictionary(dss_h)) = catalog.get(&arena.name("DSS")) else {
        panic!("the catalogue carries no /DSS after AddLtvInfo");
    };
    let dss = arena.get_dict(*dss_h).expect("the store reads");
    let Some(Object::Array(certs_h)) = dss.get(&arena.name("Certs")) else {
        panic!("/DSS carries no /Certs: {dss:?}");
    };
    let certs = arena.get_array(*certs_h).expect("the array reads");
    assert_eq!(certs.len(), 2, "one stream per certificate");

    for (n, expected) in [first, second].into_iter().enumerate() {
        let Some(Object::Reference(h)) = certs.get(n) else {
            panic!("certificate {n} is not an indirect reference: {:?}", certs.get(n));
        };
        let Some(Object::Stream(_, data)) = arena.get_object(*h) else {
            panic!("certificate {n} is not a stream");
        };
        let fepdf_model::object::SublimatedData::Raw(bytes) = &*data else {
            panic!("certificate {n} was stored as something other than raw bytes");
        };
        assert_eq!(
            bytes.as_ref(),
            expected.as_slice(),
            "certificate {n} came back as different bytes"
        );
    }
}

/// The half that keeps the first honest: a store written on every call regardless of what
/// it was given would pass the assertions above while carrying nothing.
#[test]
fn a_store_with_no_certificates_names_no_certs_array() {
    let mut doc = document();
    apply_operation(&mut doc, Operation::AddLtvInfo { certificates: Vec::new() })
        .expect("the store is written");

    let arena = doc.arena();
    let catalog_h = doc.catalog_handle().expect("the fixture has a catalogue");
    let catalog = arena.get_dict(doc.resolve_to_dict(catalog_h).expect("resolves")).expect("read");
    let Some(Object::Dictionary(dss_h)) = catalog.get(&arena.name("DSS")) else {
        panic!("the catalogue carries no /DSS");
    };
    let dss = arena.get_dict(*dss_h).expect("the store reads");
    assert!(
        !dss.contains_key(&arena.name("Certs")),
        "an empty /Certs array states there are no certificates, which is not the same as \
         not having been given any: {dss:?}"
    );
}

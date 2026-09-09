//! A `/ColorSpace` resource resolves once per document, not once per `cs`.
//!
//! **Measured 2026-09-10.** One run of `pattern_color_test` resolved 1,514 colour spaces
//! for 2,488 colours and built 1,514 ICC transforms out of them — the same handful of
//! resources, once per `cs` operator, on every page. An ICC transform costs about 2.6ms
//! to build, which put 3.9s of that test's 15.3s into work already done; with the cache
//! the test takes 9.8s, under the 11.3s it took before colour management existed at all.
//!
//! The cache is keyed by the resource entry's arena handle rather than by the operand
//! name, because a name is unique only within one resource dictionary and the lookup
//! walks a stack of them.

use fepdf_model::color::{SpacePool, space_key};
use fepdf_model::object::{Object, PdfName};
use fepdf_model::{Document, PdfArena};

/// A document with nothing in it but the arena the entries are built in.
fn empty_document() -> Document {
    let arena = PdfArena::new();
    let root = arena.alloc_object(Object::Null);
    Document::new(arena, root, None)
}

/// `[/ICCBased <stream with an sRGB profile and /N 3>]`, as an object in `doc`'s arena.
fn icc_entry(arena: &PdfArena) -> Object {
    let profile = moxcms::ColorProfile::new_srgb().encode().expect("sRGB encodes");
    let mut dict = std::collections::BTreeMap::new();
    dict.insert(arena.intern_name(PdfName::new("N")), Object::Integer(3));
    dict.insert(arena.intern_name(PdfName::new("Length")), Object::Integer(profile.len() as i64));
    let stream = Object::Stream(
        arena.alloc_dict(dict),
        std::sync::Arc::new(fepdf_model::object::SublimatedData::Raw(bytes::Bytes::from(profile))),
    );
    Object::Array(
        arena.alloc_array(vec![Object::Name(arena.intern_name(PdfName::new("ICCBased"))), stream]),
    )
}

/// The same entry twice is the same `Arc`, which is the whole claim: the second `cs` did
/// not read the profile again.
#[test]
fn the_same_entry_resolves_once() {
    let doc = empty_document();
    let entry = icc_entry(doc.arena());
    let first = doc.resolved_color_space(&entry).expect("the space resolves");
    let second = doc.resolved_color_space(&entry).expect("the space resolves");
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "a second lookup of one entry parsed it a second time"
    );
}

/// **The check that this cache can be wrong.** `PdfArena::set_object` writes a handle in
/// place, so an edit could put a different colour space behind a handle the cache has
/// already answered for. Nothing that ships does that today — this is what
/// `forget_color_spaces` is called at every edit for, and breaking it here is how that
/// call is known to do anything.
#[test]
fn forgetting_makes_the_next_lookup_read_again() {
    let doc = empty_document();
    let entry = icc_entry(doc.arena());
    let first = doc.resolved_color_space(&entry).expect("the space resolves");
    doc.forget_color_spaces();
    let second = doc.resolved_color_space(&entry).expect("the space resolves");
    assert!(
        !std::sync::Arc::ptr_eq(&first, &second),
        "forget_color_spaces left the old answer in place"
    );
}

/// Two handles can share an index and mean different things, because the arena's pools
/// are separate. The key carries which pool, and this is why.
#[test]
fn the_key_separates_the_pools() {
    let arena = PdfArena::new();
    let array = arena.alloc_array(vec![Object::Null]);
    let dict = arena.alloc_dict(std::collections::BTreeMap::new());
    assert_eq!(space_key(&Object::Array(array)), Some((SpacePool::Array, array.index())));
    assert_eq!(space_key(&Object::Dictionary(dict)), Some((SpacePool::Dictionary, dict.index())));
    assert_eq!(
        space_key(&Object::Integer(7)),
        None,
        "a direct object carries no arena identity to key on"
    );
}

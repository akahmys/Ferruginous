//! `find_object` answers the same way whether or not anything has asked before.
//!
//! The reverse index it reads — object value to the handles holding it — used to be
//! maintained on **every** `alloc_object` and `set_object`, which is every object in the
//! file, at load. Two call sites query it: an annotation looking up its appearance
//! stream, and a page looking up its content stream. Measured on
//! `samples/intel_sdm.pdf` (332,386 entries in its first cross-reference section),
//! maintaining it eagerly cost **3.6 s of a 6.2 s read** — 58% of the time, for a
//! question nothing in `inspect info` ever asks.
//!
//! It is built on the first query now, and kept up to date only once it exists. These
//! tests are what say the answers did not change: the lowest handle wins, later writes
//! are seen, and a value nothing holds is still absent.

use fepdf_model::{Object, PdfArena};

#[test]
fn a_value_is_found_by_the_first_handle_that_holds_it() {
    let arena = PdfArena::new();
    let first = arena.alloc_object(Object::Integer(7));
    let _second = arena.alloc_object(Object::Integer(7));

    assert_eq!(arena.find_object(&Object::Integer(7)), Some(first), "the lowest handle wins");
}

#[test]
fn a_value_nothing_holds_is_not_found() {
    let arena = PdfArena::new();
    arena.alloc_object(Object::Integer(7));

    assert_eq!(arena.find_object(&Object::Integer(9)), None);
}

/// A write after the index exists is seen by the next query.
///
/// This is the half a lazily built index can get wrong: build once, then answer from a
/// snapshot that stopped being true.
#[test]
fn a_write_after_the_first_query_is_seen() {
    let arena = PdfArena::new();
    let handle = arena.alloc_object(Object::Integer(1));

    // Force the index into existence.
    assert_eq!(arena.find_object(&Object::Integer(1)), Some(handle));

    arena.set_object(handle, Object::Integer(2));

    assert_eq!(arena.find_object(&Object::Integer(2)), Some(handle), "the new value is found");
    assert_eq!(arena.find_object(&Object::Integer(1)), None, "and the old one is not");
}

/// An object allocated after the first query is found too.
#[test]
fn an_allocation_after_the_first_query_is_seen() {
    let arena = PdfArena::new();
    arena.alloc_object(Object::Integer(1));
    assert_eq!(arena.find_object(&Object::Integer(1)).map(|h| h.index()), Some(0));

    let later = arena.alloc_object(Object::Integer(5));
    assert_eq!(arena.find_object(&Object::Integer(5)), Some(later));
}

/// Querying twice gives the same answer, which a build-once-per-query design would not
/// guarantee cheaply.
#[test]
fn two_queries_agree() {
    let arena = PdfArena::new();
    let handle = arena.alloc_object(Object::Boolean(true));

    assert_eq!(arena.find_object(&Object::Boolean(true)), Some(handle));
    assert_eq!(arena.find_object(&Object::Boolean(true)), Some(handle));
}

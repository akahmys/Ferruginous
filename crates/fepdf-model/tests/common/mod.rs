//! The two arena builders this crate's colour, function and mesh tests share.
//!
//! Both stood in all three of `calibrated_colour_tests`, `function_tests` and
//! `mesh_tests`, identically. They build *arena* objects rather than PDF bytes, which is
//! the other kind of fixture this crate needs: a shading or a function is reached as a
//! resolved dictionary, and a test that writes one out as a file would be testing the
//! reader on its way to the thing it means to test.
//!
//! Only what all three use lives here. `mesh_tests`'s stream `push` is its own.

use fepdf_model::{Handle, Object, PdfArena, PdfName};
use std::collections::BTreeMap;

/// A dictionary in `arena`, from `(name, value)` pairs.
pub fn dict(
    arena: &PdfArena,
    entries: Vec<(&str, Object)>,
) -> Handle<BTreeMap<Handle<PdfName>, Object>> {
    let mut map = BTreeMap::new();
    for (key, value) in entries {
        map.insert(arena.intern_name(PdfName::new(key)), value);
    }
    arena.alloc_dict(map)
}

/// An array of reals in `arena`, which every one of these fixtures needs.
pub fn nums(arena: &PdfArena, values: &[f64]) -> Object {
    Object::Array(arena.alloc_array(values.iter().map(|v| Object::Real(*v)).collect()))
}

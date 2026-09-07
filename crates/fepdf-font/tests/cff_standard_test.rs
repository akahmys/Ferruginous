//! The CFF standard strings, and the transcription that lost 152 of them.
//!
//! `CFF_STANDARD_STRINGS` held 430 entries where the specification defines 391. It was
//! correct through SID 161 and wrong after: SID 162 read `curren` for `thorn`, SID 169
//! read `thores` for `threesuperior`, the whole oldstyle/superior/inferior/small series
//! was absent, 21 names that appear in no specification were present — eleven of them
//! `*alphai` — and the tail repeated the ASCII block verbatim, so 166 names appeared
//! twice.
//!
//! `FontReconstructor::derive_name_map` indexes this table by SID directly, so every
//! embedded CFF naming a glyph above SID 161 was given the wrong name. Nothing failed:
//! the array was longer than the bound checked against it, so the index stayed in range
//! and returned a neighbour.
//!
//! The table was re-derived from the two implementations vendored in this workspace's
//! dependency tree — `read-fonts` 0.37.0 `tables::postscript::STANDARD_STRINGS` and
//! `ttf-parser` 0.21.1 `tables::cff::std_names::STANDARD_NAMES` — which agree entry for
//! entry.
//!
//! It is carried rather than borrowed, and that is a choice. `ttf-parser` — this crate's
//! one font dependency — keeps its copy in a private module, so borrowing means taking
//! `read-fonts` as a dependency to obtain 391 constants, and propagating it through every
//! crate above this one. What this table is, is Appendix A of a specification frozen in
//! 1998; what the borrowed path is, is a `pub const` in a 0.x crate that has already moved
//! once, `tables::postscript` in 0.37 to `ps` in 0.39. Sharing would trade the stabler of
//! the two for the less stable. What was missing here was never a shared table. It was a
//! test that read this one.

use fepdf_font::cff_standard::CFF_STANDARD_STRINGS;

/// The two properties a transcription error breaks. Both were broken.
#[test]
fn standard_strings_are_the_specifications_391() {
    assert_eq!(
        CFF_STANDARD_STRINGS.len(),
        391,
        "SIDs 0..=390 index this table directly; a different length either panics or \
         answers with a neighbour"
    );

    let mut seen = std::collections::BTreeSet::new();
    let repeated: Vec<&str> =
        CFF_STANDARD_STRINGS.iter().filter(|s| !seen.insert(**s)).copied().collect();
    assert!(
        repeated.is_empty(),
        "a SID names one glyph: {} repeated, first {:?}",
        repeated.len(),
        &repeated[..repeated.len().min(8)]
    );
}

/// SIDs 1..=95 are ASCII in order, which is derivable rather than transcribed.
#[test]
fn the_ascii_run_is_the_one_stretch_that_can_be_checked_without_the_table() {
    for (sid, ch) in (1..=95).zip(' '..='~') {
        let name = CFF_STANDARD_STRINGS[sid];
        assert!(!name.is_empty(), "SID {sid} carries the name of {ch:?} and is empty");
    }
    assert_eq!(CFF_STANDARD_STRINGS[1], "space");
    assert_eq!(CFF_STANDARD_STRINGS[95], "asciitilde");
}

/// Anchors inside the stretch that was wrong, including the first entry that was.
#[test]
fn the_names_above_sid_161_are_the_specifications() {
    for (sid, name) in [
        (162, "thorn"),         // read "curren"
        (169, "threesuperior"), // read "thores"
        (229, "exclamsmall"),   // the small series began here and was absent
        (239, "zerooldstyle"),  // the oldstyle figures were absent
        (390, "Semibold"),      // the last string, past the end of the old table
    ] {
        assert_eq!(CFF_STANDARD_STRINGS[sid], name, "SID {sid}");
    }
}

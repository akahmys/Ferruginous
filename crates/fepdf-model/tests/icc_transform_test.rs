//! `[/ICCBased …]` colours go through the profile (10.3).
//!
//! **The profile was read and thrown away.** `ResolvedColorSpace::from_icc` took the
//! stream's `/N` for the component count and discarded the stream, so every colour in an
//! ICC space — 438 of the corpus's 1,053 images are in one — reached the backend through
//! 10.4.2.1's device formula, the conversion that clause offers a processor which is *not*
//! ICC-enabled. `moxcms` had been a dependency throughout.
//!
//! **The first attempt at this implemented it on the wrong type.** `color::ColorSpace`
//! has an `ICCBased(Arc<ColorProfile>)` variant and a `transform` method, and nothing
//! outside its own module refers to either: the tests passed and no page changed. The
//! question that caught it was "what calls this", which no test asks.

use fepdf_model::PdfArena;
use fepdf_model::color::ResolvedColorSpace;
use fepdf_model::graphics::Color;
use fepdf_model::object::{Object, PdfName};

/// `[/ICCBased <stream with an sRGB profile and /N 3>]`, built in an arena.
fn icc_space(arena: &PdfArena, profile: &[u8], n: i64) -> ResolvedColorSpace {
    let mut dict = std::collections::BTreeMap::new();
    dict.insert(arena.intern_name(PdfName::new("N")), Object::Integer(n));
    dict.insert(arena.intern_name(PdfName::new("Length")), Object::Integer(profile.len() as i64));
    let stream = Object::Stream(
        arena.alloc_dict(dict),
        std::sync::Arc::new(fepdf_model::object::SublimatedData::Raw(
            bytes::Bytes::copy_from_slice(profile),
        )),
    );
    let array =
        arena.alloc_array(vec![Object::Name(arena.intern_name(PdfName::new("ICCBased"))), stream]);
    ResolvedColorSpace::parse(&Object::Array(array), arena).expect("the space resolves")
}

fn srgb_bytes() -> Vec<u8> {
    moxcms::ColorProfile::new_srgb().encode().expect("sRGB encodes")
}

/// Through sRGB and out again a colour is itself, which says the transform ran and is not
/// a destructive one.
#[test]
fn an_icc_space_puts_its_components_through_the_profile() {
    let arena = PdfArena::new();
    let space = icc_space(&arena, &srgb_bytes(), 3);
    let Some(Color::Rgb(r, g, b)) = space.to_color(&[0.2, 0.4, 0.6]) else {
        panic!("an sRGB space produced something other than RGB");
    };
    for (out, want) in [(r, 0.2), (g, 0.4), (b, 0.6)] {
        assert!((out - want).abs() < 0.02, "sRGB through sRGB moved a channel: {out} for {want}");
    }
}

/// A profile that will not decode leaves the space usable: `/N` still says how many
/// components there are, and the device reading is what a processor without a profile
/// does. Losing the colour would be worse than not managing it.
#[test]
fn an_unreadable_profile_falls_back_to_the_device_reading() {
    let arena = PdfArena::new();
    let space = icc_space(&arena, b"not an ICC profile", 4);
    assert_eq!(
        space.to_color(&[0.1, 0.2, 0.3, 0.4]),
        Some(Color::Cmyk(0.1, 0.2, 0.3, 0.4)),
        "an unusable profile has to leave the components readable"
    );
}

/// 8.6.5.5: `/N` is the authority on the component count even when the profile disagrees.
/// Carrying the profile does not change that, and this is the guard.
#[test]
fn n_remains_the_authority_on_the_component_count() {
    let arena = PdfArena::new();
    let space = icc_space(&arena, &srgb_bytes(), 4);
    assert_eq!(space.components, 4, "/N said four, whatever the sRGB profile says");
}

//! What the arena compresses, it gives back unchanged.
//!
//! `SublimatedData::Compressed` is a memory trade and never reaches a writer: a stream
//! over 4 KB is deflated while the document is read and expanded again by
//! `PdfArena::get_stream_bytes`. **Nothing tested the pair.** `deflate` and `inflate` had
//! no round-trip test at all, which for the one place a document's bytes are transformed
//! and restored in memory is the test that has to exist.

use fepdf_model::filters::flate::{deflate, inflate};

fn round_trip(input: &[u8]) {
    let compressed = deflate(input).expect("compressing an in-memory buffer cannot fail");
    let restored = inflate(&compressed).expect("what deflate wrote, inflate reads");
    assert_eq!(restored, input, "the bytes came back changed");
}

#[test]
fn a_content_stream_survives_the_round_trip() {
    round_trip(b"BT /F1 24 Tf 72 700 Td (Hello, world) Tj ET\n");
}

#[test]
fn an_empty_buffer_survives_the_round_trip() {
    round_trip(b"");
}

/// Every byte value, so a codec that mangled one range would be caught.
#[test]
fn all_byte_values_survive_the_round_trip() {
    let all: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
    round_trip(&all);
}

/// Incompressible input comes back too, even though it grows.
///
/// The arena stores whatever `deflate` returns; a buffer that does not compress is still
/// expanded correctly, which is what stops "it got bigger" from becoming "it got lost".
#[test]
fn incompressible_bytes_survive_the_round_trip() {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let noise: Vec<u8> = (0..40_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state & 0xFF) as u8
        })
        .collect();
    round_trip(&noise);
}

/// A stream large enough that `commit_stream_to_arena` would compress it.
#[test]
fn a_buffer_past_the_four_kilobyte_threshold_survives() {
    let page: Vec<u8> = b"BT /F1 12 Tf 0 0 Td (line) Tj ET\n".repeat(400);
    assert!(page.len() > 4096, "the fixture is past the threshold the arena uses");
    round_trip(&page);
}

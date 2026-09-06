//! How much the ingest-time deflate actually holds back.
//!
//! `commit_stream_to_arena` compresses every non-image, non-font stream over 4 KB while
//! the document is being read. Peak RSS over a short-lived `inspect info` cannot show
//! what that buys, because the process exits before sustained footprint matters. This
//! counts the bytes directly: how many streams took the compressed form, what they were,
//! and what they became.

use fepdf_model::Document;
use fepdf_model::ingest::IngestionOptions;
use fepdf_model::object::SublimatedData;

fn main() {
    let path = std::env::args().nth(1).expect("usage: sublimation_probe <file.pdf>");
    let data = std::fs::read(&path).expect("the file reads");
    let doc = Document::open(bytes::Bytes::from(data), &IngestionOptions::default())
        .expect("the document opens");

    let arena = doc.arena();
    let (mut raw_n, mut raw_bytes) = (0usize, 0usize);
    let (mut comp_n, mut comp_before, mut comp_after) = (0usize, 0usize, 0usize);
    let (mut cmd_n, mut img_n) = (0usize, 0usize);

    for i in 0..arena.object_count() {
        let Some(fepdf_model::Object::Stream(_, data)) =
            arena.get_object(fepdf_model::Handle::new(i))
        else {
            continue;
        };
        match &*data {
            SublimatedData::Raw(b) => {
                raw_n += 1;
                raw_bytes += b.len();
            }
            SublimatedData::Compressed { original_len, data } => {
                comp_n += 1;
                comp_before += *original_len;
                comp_after += data.len();
            }
            // Named rather than waved through: Rule 5 forbids a wildcard over a domain
            // enum, and the two pre-parsed forms are the ones that would silently stop
            // being counted if a stream started arriving as one of them.
            SublimatedData::Commands { .. } => cmd_n += 1,
            SublimatedData::Image { .. } => img_n += 1,
        }
    }

    println!("{path}");
    println!("  raw        {raw_n:>7} streams  {:>10} KB", raw_bytes / 1024);
    println!(
        "  compressed {comp_n:>7} streams  {:>10} KB -> {} KB  (saves {} KB)",
        comp_before / 1024,
        comp_after / 1024,
        comp_before.saturating_sub(comp_after) / 1024
    );
    println!("  commands   {cmd_n:>7} streams");
    println!("  images     {img_n:>7} streams");
}

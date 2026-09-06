//! FlateDecode Filter (ISO 32000-2:2020 Clause 7.4.4)

use crate::PdfResult;
use crate::error::PdfError;
use crate::filters::{DecodingFilter, FilterContext, predictor};
use bytes::Bytes;
use flate2::read::ZlibDecoder;
use std::io::Read;

const MAX_DECOMPRESSED_SIZE: u64 = 128 * 1024 * 1024; // 128 MB

/// The `FlateDecode` stream filter.
pub struct FlateFilter;

impl DecodingFilter for FlateFilter {
    fn decode(&self, input: &[u8], cx: &FilterContext<'_>) -> PdfResult<Bytes> {
        let (params, arena) = (cx.params, cx.arena);
        let mut decoder = ZlibDecoder::new(input).take(MAX_DECOMPRESSED_SIZE + 1);
        let mut decoded = Vec::new();

        decoder.read_to_end(&mut decoded).map_err(|e| PdfError::Filter {
            filter: "FlateDecode".into(),
            message: format!("Flate decompression failed: {e}").into(),
        })?;

        if decoded.len() as u64 > MAX_DECOMPRESSED_SIZE {
            return Err(PdfError::Filter {
                filter: "FlateDecode".into(),
                message: format!(
                    "Decompressed stream size exceeded limit of {MAX_DECOMPRESSED_SIZE} bytes"
                )
                .into(),
            });
        }

        // Apply predictors if present in DecodeParms
        if let Some(p) = params {
            decoded = predictor::apply_predictor(&decoded, p, arena)?;
        }

        Ok(Bytes::from(decoded))
    }
}

/// Compresses bytes for the arena's in-memory form, not for a file.
///
/// `SublimatedData::Compressed` never reaches a writer — `PdfArena::get_stream_bytes`
/// expands it first — so this is a memory trade and not a `/FlateDecode` stream. It lives
/// beside the filter because the codec is the same one, and because a second compression
/// crate is what made the engine depend on C.
///
/// **Level 1, not the default 6, and the difference was measured.** Because this runs
/// during `Document::open` on every stream over 4 KB, its cost is paid by every read
/// whether or not the memory is ever wanted. On `samples/intel_sdm.pdf` (24 MB, 5,057
/// pages, 3,154 streams compressed), `inspect info` steady-state:
///
/// | level | time | held back |
/// | --- | ---: | ---: |
/// | 6 (`default`) | 1.97 s | 18,563 KB |
/// | 3 | 1.76 s | 18,256 KB |
/// | **1 (`fast`)** | **1.67 s** | **17,916 KB** |
/// | not compressing at all | 1.62 s | 0 |
///
/// Level 1 keeps 96.5% of what level 6 holds back for 14% of what it costs. The trade
/// this makes is worth having — the last row is what removing it would buy, and 18 MB of
/// a document's streams is not nothing — but paying six-level compression for the last
/// 3.5% of it, on every open, was not.
///
/// # Errors
/// Fails only when the encoder does, which for an in-memory writer means allocation.
pub fn deflate(input: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Write as _;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(input)?;
    encoder.finish()
}

/// Expands what [`deflate`] produced.
///
/// # Errors
/// Fails when the bytes are not what `deflate` wrote.
pub fn inflate(input: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoded = Vec::new();
    ZlibDecoder::new(input).take(MAX_DECOMPRESSED_SIZE + 1).read_to_end(&mut decoded)?;
    Ok(decoded)
}

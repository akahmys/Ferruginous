//! fepdf WASM: WebAssembly bridge for the fepdf PDF engine.
//!
//! Provides a JavaScript-friendly interface for document loading and rendering.

use bytes::Bytes;
use fepdf::PdfDocument as SdkDocument;
use wasm_bindgen::prelude::*;

/// A JavaScript-friendly wrapper for a PDF document.
#[wasm_bindgen]
pub struct PdfDocument {
    inner: SdkDocument,
}

#[wasm_bindgen]
impl PdfDocument {
    /// Opens a PDF document from a byte array.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Result<PdfDocument, JsValue> {
        console_error_panic_hook::set_once();
        let bytes = Bytes::copy_from_slice(data);
        let inner = SdkDocument::open(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(PdfDocument { inner })
    }

    /// Returns the total number of pages in the document.
    #[wasm_bindgen(getter)]
    pub fn page_count(&self) -> Result<usize, JsValue> {
        self.inner.page_count().map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Renders a specific page to a canvas — or rather, says that it does not.
    ///
    /// # Errors
    /// Always. See [`render_page_refusal`].
    pub fn render_page(&self, index: usize, canvas_id: &str) -> Result<(), JsValue> {
        Err(JsValue::from_str(&render_page_refusal(index, canvas_id)))
    }

    /// The text of one page, in reading order.
    ///
    /// **Everything below needs no GPU**, which is why it is here and rendering is not.
    /// This crate opened a document and counted its pages and did nothing else, so a
    /// browser could learn how long a PDF was and not what it said — while the engine
    /// behind it reads 96% of what a corpus presents.
    ///
    /// # Errors
    /// When the page does not exist, or its content stream cannot be read to the end.
    pub fn extract_text(&self, index: usize) -> Result<String, JsValue> {
        text_of(&self.inner, index).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// What the engine had to decide to read this document, as JSON.
    ///
    /// The array is empty for a file that needed no decision, which is the answer rather
    /// than the absence of one: `ARCHITECTURE.md` §4.3 is the reason this engine records
    /// rather than logs, and a caller that cannot see the record has the logging problem
    /// back.
    ///
    /// # Errors
    /// When the decisions cannot be serialised, which would be a defect here rather than
    /// in the document.
    pub fn decisions_json(&self) -> Result<String, JsValue> {
        decisions_of(&self.inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// What protects the document (7.6), or `"None"`.
    #[wasm_bindgen(getter)]
    pub fn security_method(&self) -> String {
        self.inner.security_method()
    }

    /// The logical structure tree as JSON, or `null` where the document has none.
    ///
    /// # Errors
    /// When the tree cannot be serialised.
    pub fn struct_tree_json(&self) -> Result<String, JsValue> {
        struct_tree_of(&self.inner).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Why `render_page` refuses, in terms a caller can act on.
///
/// **It used to return `Ok(())` having drawn nothing**, which is worse than being
/// unimplemented: a caller was told the page had been rendered and got a blank canvas,
/// with nothing anywhere to say otherwise. Not being able to do something is a fact about
/// this crate; reporting success for it is a fact about the caller's next hour.
///
/// Rendering here needs a WebGPU surface through `web-sys` and the facade's `render`
/// feature, and neither is present. It is not written as a stub that might one day fill
/// in, because a stub is what this was: the comment saying implementation "will involve"
/// a WebGPU surface had been there long enough for the roadmap to find it by measurement
/// rather than by memory.
///
/// Separate from the `wasm_bindgen` method so it can be tested on the host, which
/// `JsValue` cannot be — the same shape `fepdf-mcp` uses for its tools.
#[must_use]
pub fn render_page_refusal(index: usize, canvas_id: &str) -> String {
    format!(
        "fepdf-wasm cannot render: page {index} was not drawn to {canvas_id:?}. \
         This build has no WebGPU surface and does not enable the facade's `render` \
         feature. Use page_count and the text APIs, or render outside the browser."
    )
}

/// What a reading method could not do.
///
/// **A typed error even though it becomes a string a line later** (RR-15 Rule 11). The
/// `JsValue` boundary flattens everything to text and that is JavaScript's shape, not a
/// reason for the crate's own functions to lose which of two things went wrong: a page
/// that will not read and a report that will not serialise are the document's problem and
/// this crate's, and only the second is a defect here.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The engine could not read that part of the document.
    #[error("{0}")]
    Document(String),
    /// The report could not be turned into JSON.
    #[error("the report could not be serialised: {0}")]
    Serialisation(#[from] serde_json::Error),
}

/// The bodies of the reading methods above, off the `wasm_bindgen` boundary.
///
/// **`JsValue` cannot be constructed off a WebAssembly target**, so a method returning one
/// is a method this suite cannot run. `render_page_refusal` already had this shape and
/// says why; these follow it, which is also how `fepdf-mcp` separates a tool from its
/// implementation.
///
/// # Errors
/// The engine's error, rendered — a caller in a browser gets a string either way.
pub fn text_of(doc: &SdkDocument, index: usize) -> Result<String, ReadError> {
    doc.extract_text(index).map_err(|e| ReadError::Document(format!("{e:?}")))
}

/// Every decision the engine took reading this document, as a JSON array.
///
/// # Errors
/// When the decisions cannot be serialised, which is a defect here rather than in the
/// document.
pub fn decisions_of(doc: &SdkDocument) -> Result<String, ReadError> {
    Ok(serde_json::to_string(&doc.decisions())?)
}

/// The logical structure tree as JSON, or the string `null` for a document with none.
///
/// **`null` and not an error**: a document without a structure tree is a very common
/// document, not a failure to read one.
///
/// # Errors
/// When the tree cannot be serialised.
pub fn struct_tree_of(doc: &SdkDocument) -> Result<String, ReadError> {
    match doc.extract_struct_tree() {
        Some(tree) => Ok(serde_json::to_string(&tree)?),
        None => Ok("null".to_string()),
    }
}

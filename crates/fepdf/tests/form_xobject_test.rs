//! What a Form XObject's ceremony does, and that both implementations of it agree.
//!
//! `/Do` on a form pushes the graphics state, concatenates `/Matrix`, clips to `/BBox`
//! (8.10.1), runs the form's content, unwinds the clip and restores the state. That
//! sequence stands twice in `fepdf-content`: `execute_form_commands` takes the content as
//! parsed `Command`s and `render_form_xobject` takes it as raw bytes, and 66 of their 70
//! lines are identical. Which one runs is decided by whether ingestion refined the stream,
//! not by anything about the form.
//!
//! **Nothing checked either, and nothing checked that they agree.** These five hold the
//! contract before it is written once; the fifth is the premise the consolidation rests
//! on, and it is the one the crate had no way to state.

use fepdf::PdfDocument;
use fepdf_content::{
    Color, FallbackFontType, PixelFormat, RenderBackend, SMaskData, StrokeStyle, TextGlyph,
    TextState, WindingRule,
};
use fepdf_model::ingest::IngestionOptions;
use kurbo::{Affine, BezPath, Shape};
use std::sync::Arc;

mod common;
use common::assemble;

/// What the interpreter asked the backend to do, in the order it asked.
#[derive(Default)]
struct Recorder {
    events: Vec<String>,
}

impl Recorder {
    fn fills(&self) -> Vec<&str> {
        self.events.iter().filter(|e| e.starts_with("fill")).map(String::as_str).collect()
    }
    fn count(&self, prefix: &str) -> usize {
        self.events.iter().filter(|e| e.starts_with(prefix)).count()
    }
}

impl RenderBackend for Recorder {
    fn fill_path(&mut self, path: &BezPath, color: &Color, _rule: WindingRule) {
        let b = path.bounding_box();
        let shade = match *color {
            Color::Rgb(red, green, blue) => format!("rgb {red:.1},{green:.1},{blue:.1}"),
            Color::Gray(level) => format!("gray {level:.1}"),
            Color::Cmyk(cyan, magenta, yellow, black) => {
                format!("cmyk {cyan:.1},{magenta:.1},{yellow:.1},{black:.1}")
            }
            Color::Lab(lightness, green_red, blue_yellow) => {
                format!("lab {lightness:.1},{green_red:.1},{blue_yellow:.1}")
            }
        };
        self.events.push(format!("fill[{shade}]({:.1},{:.1},{:.1},{:.1})", b.x0, b.y0, b.x1, b.y1));
    }
    fn push_clip(&mut self, path: &BezPath, _rule: WindingRule) {
        let b = path.bounding_box();
        self.events.push(format!("push_clip({:.1},{:.1},{:.1},{:.1})", b.x0, b.y0, b.x1, b.y1));
    }
    fn pop_clip(&mut self) {
        self.events.push("pop_clip".to_string());
    }
    fn push_state(&mut self) {
        self.events.push("push_state".to_string());
    }
    fn pop_state(&mut self) {
        self.events.push("pop_state".to_string());
    }
    fn transform(&mut self, t: Affine) {
        let c = t.as_coeffs();
        self.events.push(format!(
            "transform({:.1},{:.1},{:.1},{:.1},{:.1},{:.1})",
            c[0], c[1], c[2], c[3], c[4], c[5]
        ));
    }
    fn set_transform(&mut self, _t: Affine) {}
    fn stroke_path(&mut self, _p: &BezPath, _c: &Color, _s: &StrokeStyle) {}
    fn draw_image(&mut self, _d: &[u8], _w: u32, _h: u32, _f: PixelFormat, _s: Option<SMaskData>) {}
    fn show_text(&mut self, _g: &[TextGlyph], _s: f64, _t: Affine, _ts: TextState, _o: usize) {}
    fn set_fill_color(&mut self, _c: Color) {}
    fn set_stroke_color(&mut self, _c: Color) {}
    fn set_fill_alpha(&mut self, _a: f64) {}
    fn set_stroke_alpha(&mut self, _a: f64) {}
    fn set_blend_mode(&mut self, _m: fepdf_content::BlendMode) {}
    fn set_font(&mut self, _n: &str) {}
    fn set_text_render_mode(&mut self, _m: fepdf_model::graphics::TextRenderingMode) {}
    fn set_char_spacing(&mut self, _s: f64) {}
    fn set_word_spacing(&mut self, _s: f64) {}
    #[allow(clippy::too_many_arguments)]
    fn define_font(
        &mut self,
        _n: &str,
        _b: Option<&str>,
        _d: Option<Arc<Vec<u8>>>,
        _i: Option<usize>,
        _m: Option<std::collections::BTreeMap<u32, u32>>,
        _f: FallbackFontType,
        _c: bool,
    ) {
    }
}

/// A page drawing one form through `/Do`, and one rectangle after it.
///
/// The form paints a 10x10 red square at the origin and sets the fill colour to red; the
/// page paints a 20x20 square in blue afterwards, which is what says whether the form's
/// state escaped.
fn page(matrix: &str, bbox: &str) -> Vec<u8> {
    let form = "1 0 0 rg 0 0 10 10 re f\n";
    let content = "q 1 0 0 1 50 50 cm /Fx Do Q\n0 0 1 rg 0 0 20 20 re f\n";
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
         /Resources << /XObject << /Fx 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}endstream", content.len()),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox {bbox} {matrix} /Length {} >>\n\
             stream\n{form}endstream",
            form.len()
        ),
    ])
}

fn draw(bytes: Vec<u8>, refine: bool) -> Recorder {
    let options = IngestionOptions { active_refinement: refine, ..IngestionOptions::default() };
    let doc = PdfDocument::open_with_options(bytes.into(), &options).expect("the fixture opens");
    let mut recorder = Recorder::default();
    doc.render_page(0, &mut recorder, Affine::IDENTITY).expect("the page interprets");
    recorder
}

/// `/Matrix` is concatenated onto the CTM before the form's content draws.
///
/// The backend composes it; the path arrives in the form's own space either way, so the
/// fill's bounding box is the same with and without a `/Matrix` and says nothing. What
/// says it is the `transform` call the interpreter makes — which is where the first
/// version of this test looked in the wrong place and passed a wrong reading as a defect.
#[test]
fn the_forms_matrix_reaches_the_backend() {
    let plain = draw(page("", "[0 0 10 10]"), true);
    let scaled = draw(page("/Matrix [2 0 0 2 0 0]", "[0 0 10 10]"), true);
    assert!(
        !plain.events.iter().any(|e| e.starts_with("transform(2.0")),
        "a form with no /Matrix concatenates none: {:?}",
        plain.events
    );
    assert!(
        scaled.events.iter().any(|e| e == "transform(2.0,0.0,0.0,2.0,0.0,0.0)"),
        "the /Matrix the form declares has to reach the backend: {:?}",
        scaled.events
    );
}

/// `/BBox` clips the form's content (8.10.1).
#[test]
fn the_forms_bbox_becomes_a_clip() {
    let r = draw(page("", "[0 0 4 4]"), true);
    let clips: Vec<&String> = r.events.iter().filter(|e| e.starts_with("push_clip")).collect();
    assert!(!clips.is_empty(), "a form with a /BBox has to clip to it: {:?}", r.events);
    assert!(
        clips.iter().any(|c| c.contains("4.0")),
        "the clip has to be the /BBox the form declared, not another rectangle: {clips:?}"
    );
}

/// The state the form changed does not survive it.
#[test]
fn the_forms_state_does_not_escape_it() {
    let r = draw(page("", "[0 0 10 10]"), true);
    let after = r.fills().last().copied().unwrap_or_default().to_string();
    assert!(
        after.contains("0.0,0.0,1.0"),
        "the square after the form is blue; the form set red and must not have kept it: {after}"
    );
}

/// The clip the form pushed is unwound with it.
#[test]
fn the_forms_clip_is_unwound_with_it() {
    let r = draw(page("", "[0 0 4 4]"), true);
    assert_eq!(
        r.count("push_clip"),
        r.count("pop_clip"),
        "every clip the form pushes is popped, or what follows draws inside it: {:?}",
        r.events
    );
}

/// **The two implementations of the ceremony agree.**
///
/// `execute_form_commands` runs when ingestion refined the form's stream into `Command`s
/// and `render_form_xobject` when it did not, so the same document reaches a different
/// implementation depending only on `active_refinement`. They are 66 identical lines out
/// of 70, and nothing said so.
#[test]
fn both_form_implementations_produce_the_same_calls() {
    let bytes = page("/Matrix [2 0 0 2 0 0]", "[0 0 10 10]");
    let refined = draw(bytes.clone(), true);
    let raw = draw(bytes, false);
    assert_eq!(
        refined.events, raw.events,
        "the parsed-command path and the raw-byte path have to draw the same form"
    );
}

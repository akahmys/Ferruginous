//! `gs` sets the line parameters of Table 57, and `w`/`J`/`j`/`M`/`d` are not the only
//! way to reach them.
//!
//! **Measured 2026-09-06, before this existed**: a page that set its stroke width through
//! an `/ExtGState` drew every stroke at the initial width, and was indistinguishable from
//! a page that set nothing at all.
//!
//! | how the parameters were set | width | cap / join |
//! | :--- | ---: | :--- |
//! | `10 w 1 J 1 j` | 10 | Round / Round |
//! | `/G1 gs` with `/LW 10 /LC 1 /LJ 1` | **1** | **Butt / Miter** |
//! | nothing | 1 | Butt / Miter |
//!
//! The struct knew: `StrokeStyle`'s fields are documented as `/LC`, `/LJ`, `/ML` and
//! `/D` — the ExtGState keys — and `handle_gs_operator` read `/ca`, `/CA`, `/BM`,
//! `/SMask` and `/Font` and stopped.

use fepdf::PdfDocument;
use fepdf_content::{
    BlendMode, Color, FallbackFontType, Paint, PixelFormat, RenderBackend, SMaskData, ShadingSpec,
    StrokeStyle, TextGlyph, TextState, WindingRule,
};
use fepdf_model::graphics::TextRenderingMode;
use kurbo::{Affine, BezPath};
use std::sync::Arc;

/// Every stroke the interpreter asked for, with the style it asked for it in.
#[derive(Default)]
struct Strokes(Vec<StrokeStyle>);

impl RenderBackend for Strokes {
    fn stroke_path(&mut self, _p: &BezPath, _c: &Color, style: &StrokeStyle) {
        self.0.push(style.clone());
    }
    fn fill_path(&mut self, _p: &BezPath, _c: &Color, _r: WindingRule) {}
    fn transform(&mut self, _t: Affine) {}
    fn set_transform(&mut self, _t: Affine) {}
    fn push_state(&mut self) {}
    fn pop_state(&mut self) {}
    fn set_fill_color(&mut self, _c: Color) {}
    fn set_stroke_color(&mut self, _c: Color) {}
    fn set_fill_paint(&mut self, _p: &Paint) {}
    fn set_stroke_paint(&mut self, _p: &Paint) {}
    fn set_fill_alpha(&mut self, _a: f64) {}
    fn set_stroke_alpha(&mut self, _a: f64) {}
    fn paint_shading(&mut self, _s: &ShadingSpec) {}
    fn set_blend_mode(&mut self, _m: BlendMode) {}
    fn draw_image(&mut self, _d: &[u8], _w: u32, _h: u32, _f: PixelFormat, _s: Option<SMaskData>) {}
    fn push_clip(&mut self, _p: &BezPath, _r: WindingRule) {}
    fn pop_clip(&mut self) {}
    fn show_text(&mut self, _g: &[TextGlyph], _s: f64, _t: Affine, _ts: TextState, _o: usize) {}
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
    fn set_font(&mut self, _n: &str) {}
    fn set_text_render_mode(&mut self, _m: TextRenderingMode) {}
    fn set_char_spacing(&mut self, _s: f64) {}
    fn set_word_spacing(&mut self, _s: f64) {}
}

fn assemble(bodies: &[String]) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let table_at = out.len();
    let size = bodies.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{table_at}\n%%EOF\n");
    out.into_bytes()
}

/// The strokes a page draws, given a content stream and one `/ExtGState` named `/G1`.
fn strokes(content: &str, ext_g_state: &str) -> (Vec<StrokeStyle>, Vec<fepdf::Decision>) {
    let bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
          /Resources << /ExtGState << /G1 5 0 R >> >> /Contents 4 0 R >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        ext_g_state.to_string(),
    ];
    let doc = PdfDocument::open(bytes::Bytes::from(assemble(&bodies))).expect("the fixture opens");
    let mut backend = Strokes::default();
    doc.render_page(0, &mut backend, Affine::IDENTITY).expect("the page interprets");
    (backend.0, doc.decisions())
}

const LINE: &str = "10 10 m 100 100 l S";
const FULL: &str = "<< /Type /ExtGState /LW 10 /LC 1 /LJ 1 /ML 7 /D [[3 2] 1] >>";

/// `gs` reaches the same state the operators do, parameter for parameter.
///
/// Asserted against the operator form rather than against literals, because the question
/// is whether the two routes agree — not what either of them happens to produce.
#[test]
fn gs_sets_what_the_operators_set() {
    let (by_operator, _) = strokes(&format!("10 w 1 J 1 j 7 M [3 2] 1 d {LINE}"), FULL);
    let (by_ext_g_state, _) = strokes(&format!("/G1 gs {LINE}"), FULL);

    assert_eq!(by_operator.len(), 1, "the operator form draws one stroke");
    assert_eq!(by_ext_g_state.len(), 1, "and so does the ExtGState form");
    assert_eq!(
        format!("{:?}", by_ext_g_state[0]),
        format!("{:?}", by_operator[0]),
        "an ExtGState must set the same state the operators do"
    );
}

/// And it is not simply that both routes leave the initial state alone.
///
/// Without this, an implementation that ignored `gs` *and* the operators would pass the
/// test above.
#[test]
fn the_state_the_two_routes_reach_is_not_the_initial_one() {
    let (set, _) = strokes(&format!("/G1 gs {LINE}"), FULL);
    let (untouched, _) = strokes(LINE, FULL);

    assert_ne!(format!("{:?}", set[0]), format!("{:?}", untouched[0]));
    assert!((set[0].width - 10.0).abs() < 1e-9, "width came from /LW: {:?}", set[0]);
    assert!(set[0].dash_pattern.is_some(), "dash came from /D: {:?}", set[0]);
}

/// A `/LC` an ExtGState may not carry is recorded, as `J` with the same value is.
///
/// Rule 20's ground: an enumerant the standard does not define gets a substitute, and the
/// substitution has to be said. `J 7` has recorded since 2026-08-30; `/LC 7` did not
/// exist as a path at all.
#[test]
fn an_undefined_enumerant_in_an_ext_g_state_is_recorded() {
    let (_, decisions) = strokes(&format!("/G1 gs {LINE}"), "<< /Type /ExtGState /LC 7 >>");
    let found = decisions.iter().find(|d| d.clause == "8.4.3.3").unwrap_or_else(|| {
        panic!("an /LC of 7 must be recorded against Table 53; got {decisions:?}")
    });
    assert!(found.found.contains('7'), "{}", found.found);
}

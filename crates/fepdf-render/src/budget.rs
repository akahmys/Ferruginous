//! What a scene costs against the one vello buffer that is a fixed size.
//!
//! **`binning_size` is a subtraction with no floor.** `vello_encoding::RenderConfig::new`
//! computes `bin_data.len() - layout.bin_data_start`, where `bin_data` is the constant
//! below and `bin_data_start` is the sum of every draw tag's `info_size()` — which grows
//! with the scene and is not bounded anywhere. Cross the two and the subtraction
//! underflows: a **panic** in a debug build, and in release a wrap to something near
//! `u32::MAX` that is then used to size and dispatch GPU work.
//!
//! vello says as much about the constant: *"The following buffer sizes have been hand
//! picked to accommodate the vello test scenes as well as paris-30k. These should instead
//! get derived from the scene layout using reasonable heuristics."*
//!
//! **Measured, the margin is thinner than it looks.** No single page of the nine samples
//! reaches 3% of the budget — `volvo_xc90.pdf`'s worst is 6,318 words of 262,144 — but the
//! viewer composes every *visible* page into one scene, and at its minimum zoom of 10% that
//! file puts 138 pages on screen and reaches **74.5%**. `examples/scene_budget` reports the
//! per-page figures.
//!
//! This does not say where the panic seen once in the viewer came from: that happened at
//! the default zoom, with one page visible and 0.8% of the budget used, and is unexplained.
//! What this module does is stop a scene crossing the line silently, which is worth having
//! whether or not that particular event is ever pinned down.

use vello::Scene;

/// The size `vello_encoding` fixes for the bin-data buffer, and nothing checks against it.
pub const BIN_DATA_BUDGET: u32 = 1 << 18;

/// What `layout.bin_data_start` will be for this scene, computed the same way vello does.
#[must_use]
pub fn bin_data_cost(scene: &Scene) -> u32 {
    scene.encoding().draw_tags.iter().map(|tag| tag.info_size()).sum()
}

/// The cost of one solid-colour fill, measured rather than assumed.
///
/// A caller composing pages draws a background rectangle per page before appending the
/// page itself, and must count it *before* deciding: `Scene` has no way to remove what has
/// been appended, so a composer that discovers the overrun afterwards has nothing to do
/// about it.
#[must_use]
pub fn solid_fill_cost() -> u32 {
    use kurbo::{Affine, Rect};
    use vello::peniko::{Fill, color::palette::css::WHITE};

    let mut scene = Scene::new();
    scene.fill(Fill::NonZero, Affine::IDENTITY, WHITE, None, &Rect::new(0.0, 0.0, 1.0, 1.0));
    bin_data_cost(&scene)
}

/// The reason a scene cannot be submitted, or `None` when it can.
#[must_use]
pub fn over_budget(scene: &Scene) -> Option<String> {
    let cost = bin_data_cost(scene);
    (cost > BIN_DATA_BUDGET).then(|| {
        format!(
            "the scene needs {cost} words of bin data and vello allocates a fixed \
             {BIN_DATA_BUDGET}; submitting it underflows `binning_size` and dispatches \
             GPU work sized from the wrap"
        )
    })
}

/// How many pixels of target one of vello's bins covers, each way.
///
/// A bin is 16 tiles by 16 tiles and a tile is 16 pixels square (`vello_shaders::cpu`).
const BIN_PIXELS: u32 = 256;

/// How many bins the CPU coarse pass can address.
///
/// `N_TILE_X * N_TILE_Y`, and `coarse.rs` indexes `bin_headers[part * N_TILE + bin]` with
/// nothing checking that `bin` is inside it.
const CPU_BINS: u32 = 256;

/// Why the CPU rasteriser cannot draw a target this size, or `None` when it can.
///
/// **A target larger than 256 bins walks off the end of `info_bin_data`.**
/// `vello_shaders::cpu::coarse` reads `info_bin_data[(start + i) as usize]` and panics with
/// *"index out of bounds: the len is 256 but the index is 256"*. The GPU path draws the
/// same scene; only the CPU one is bounded this way, and `panic = "abort"` in the release
/// profile means it cannot be caught — it has to be refused before it is asked.
///
/// **The bound is computed, not guessed.** Measured on `samples/volvo_xc90.pdf` at six
/// scales: 140, 204 and 247 bins drew, and 266, 280 and 441 aborted. The scene's own cost
/// was 2,073 words of bin data at every one of them, so the size of the target is the whole
/// of it.
#[must_use]
pub fn cpu_target_too_large(width: u32, height: u32) -> Option<String> {
    let bins = width.div_ceil(BIN_PIXELS) * height.div_ceil(BIN_PIXELS);
    (bins > CPU_BINS).then(|| {
        format!(
            "the CPU rasteriser addresses {CPU_BINS} bins of {BIN_PIXELS} pixels and \
             {width}x{height} needs {bins}; vello indexes past its own buffer there. \
             Render this on the GPU."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Affine, Rect};
    use vello::peniko::{Fill, color::palette::css::WHITE};

    /// A scene with `n` filled rectangles, which is `n` draw tags.
    fn scene_of(n: usize) -> Scene {
        let mut scene = Scene::new();
        for i in 0..n {
            let x = f64::from(u16::try_from(i % 100).unwrap_or(0));
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                WHITE,
                None,
                &Rect::new(x, 0.0, x + 1.0, 1.0),
            );
        }
        scene
    }

    /// An empty scene costs nothing and a drawn one costs something: without this, a cost
    /// function that always answered zero would pass every test below.
    #[test]
    fn a_scene_costs_more_than_an_empty_one() {
        assert_eq!(bin_data_cost(&Scene::new()), 0);
        assert!(bin_data_cost(&scene_of(1)) > 0, "one fill should cost something");
        assert!(bin_data_cost(&scene_of(100)) > bin_data_cost(&scene_of(10)));
    }

    /// The line is where vello's constant is, and it is reachable: this builds a scene past
    /// it rather than asserting on a number nothing produces.
    #[test]
    fn a_scene_past_the_budget_is_refused_and_says_by_how_much() {
        let per_fill = bin_data_cost(&scene_of(1000)) / 1000;
        assert!(per_fill > 0, "a fill must have a measurable cost for this test to mean anything");
        let needed = (BIN_DATA_BUDGET / per_fill) as usize + 1000;
        let big = scene_of(needed);

        assert!(bin_data_cost(&big) > BIN_DATA_BUDGET, "the scene should be over the line");
        let refusal = over_budget(&big).expect("a scene past the budget is refused");
        assert!(refusal.contains(&bin_data_cost(&big).to_string()), "say the cost: {refusal}");
        assert!(over_budget(&scene_of(10)).is_none(), "a small scene is not refused");
    }

    /// The per-fill cost a composer adds for each page background is the measured one, not
    /// a number written down here that a vello release could quietly falsify.
    #[test]
    fn one_fill_costs_what_a_scene_of_fills_costs_each() {
        let measured = solid_fill_cost();
        assert!(measured > 0, "a fill must cost something to be worth counting");
        assert_eq!(
            bin_data_cost(&scene_of(64)),
            measured * 64,
            "the composer adds `solid_fill_cost` per page and must not under-count"
        );
    }
}

#[cfg(test)]
mod cpu_bins {
    use super::cpu_target_too_large;

    /// **The measured boundary.** `samples/volvo_xc90.pdf` at six scales: 2979x4209 (204
    /// bins), 3277x4630 (247) and smaller drew; 3393x4798 (266), 3512x4967 (280) and
    /// 5333x5333 (441) aborted. The scene cost 2,073 words of bin data at every one, so the
    /// target's size is the whole of what decides it.
    #[test]
    fn the_boundary_is_where_it_was_measured() {
        for (w, h) in [(792_u32, 1119_u32), (2383, 3367), (2979, 4209), (3277, 4630)] {
            assert!(cpu_target_too_large(w, h).is_none(), "{w}x{h} drew when it was measured");
        }
        for (w, h) in [(3393_u32, 4798_u32), (3512, 4967), (3575, 5050), (5333, 5333)] {
            assert!(cpu_target_too_large(w, h).is_some(), "{w}x{h} aborted when it was measured");
        }
    }

    /// Exactly 256 bins is the last size that draws, and one bin more is the first that
    /// does not: a boundary off by one is a boundary that either aborts or refuses a page
    /// it could have drawn.
    #[test]
    fn the_last_size_that_fits_is_exactly_two_hundred_and_fifty_six_bins() {
        assert!(cpu_target_too_large(4096, 4096).is_none(), "16 by 16 bins is the limit");
        assert!(cpu_target_too_large(4097, 4096).is_some(), "one pixel over is 17 by 16");
        assert!(cpu_target_too_large(4096, 4097).is_some(), "and the other way round");
    }

    /// The refusal says what to do about it. A message that only says no leaves a caller
    /// with a page it cannot draw and no idea that the GPU would.
    #[test]
    fn the_refusal_names_the_way_out() {
        let refusal = cpu_target_too_large(5333, 5333).expect("refused");
        assert!(refusal.contains("5333x5333"), "say the size: {refusal}");
        assert!(refusal.contains("441"), "say how many bins that needs: {refusal}");
        assert!(refusal.contains("GPU"), "and say the GPU can: {refusal}");
    }
}

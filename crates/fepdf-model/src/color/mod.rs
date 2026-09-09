//! Color Space Management (ISO 32000-2 Clause 8.6)
//!
//! **A device space consults no profile, and an `/ICCBased` space consults its own.**
//! The two halves of that sentence arrived a year apart and the split is the thing to
//! hold on to: `Color::to_rgb` does 10.4.2.5, the conversion 10.4.2.1 offers a processor
//! that is *not* ICC-enabled, and it applies to `/DeviceCMYK` and its siblings because
//! 8.6.4.4 leaves them device-dependent. `moxcms` parses the profile and builds the
//! transform in [`ResolvedColorSpace`], which is where an `/ICCBased` colour goes.
//!
//! **This header used to claim the module gave "high-fidelity CMYK -> RGB conversion",
//! and it did not**: what ran was the naive `(1 − c)(1 − k)`, which is neither 10.4.2.5
//! nor colour management. It was corrected rather than deleted, because the claim is the
//! reason nobody looked — a module that says it is colour managed is not somewhere you go
//! looking for a naive formula. `AGENTS.md`, Hierarchy of Truth: measurement outranks
//! documentation.
//!
//! On `target/colour/separation.pdf` this engine's raster reads `0 0 0` where PDFKit's
//! reads `26 25 25`, and **both are conformant**: 8.6.4.4 leaves DeviceCMYK
//! device-dependent, PDFKit is ICC-enabled and follows 10.3. `ROADMAP.md` Phase P carries
//! the entry and `crosscheck_image.sh` pins the divergence.
//!
//! [`ResolvedColorSpace`] is the other half of this clause: the spaces whose components
//! are not a colour until something runs — a tint transform (8.6.6), an ICC transform
//! (8.6.5.5) or, for `/CalRGB`, a gamma and a matrix into XYZ (8.6.5.3). `/CalGray`
//! (8.6.5.2) is still read as `/DeviceGray`, which is the same shortcut `/CalRGB` had
//! until Phase P.

mod space;

pub use space::{ResolvedColorSpace, SpaceKey, SpacePool, space_key};

use serde::{Deserialize, Serialize};

/// Lightweight representation of a PDF Color Space type for IR and GraphicsState.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpaceKind {
    /// DeviceGray.
    DeviceGray,
    /// DeviceRGB.
    DeviceRGB,
    /// DeviceCMYK.
    DeviceCMYK,
    /// CIE-based CalGray.
    CalGray,
    /// CIE-based CalRGB.
    CalRGB,
    /// CIE-based L*a*b*.
    Lab,
    /// ICC-profile based.
    ICCBased,
    /// Pattern space; colour comes from a pattern.
    Pattern,
    /// Indexed palette over a base space.
    Indexed,
    /// Separation (single colorant).
    Separation,
    /// DeviceN (multiple colorants).
    DeviceN,
    /// Unrecognised or absent.
    Unknown,
}

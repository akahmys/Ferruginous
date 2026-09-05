# ADR-0065: `gs` reaches Table 57's line parameters, and PDFKit says which output was right

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

`w`, `J`, `j`, `M` and `d` set the stroke width, cap, join, mitre limit and dash. Table 57
lets an `/ExtGState` set the same five as `/LW`, `/LC`, `/LJ`, `/ML` and `/D`, and `gs`
applies it. **`handle_gs_operator` read `/ca`, `/CA`, `/BM`, `/SMask` and `/Font` and
stopped.** Measured 2026-09-06:

| how the parameters were set | width | cap / join |
| :--- | ---: | :--- |
| `10 w 1 J 1 j` | 10 | Round / Round |
| `/G1 gs` with `/LW 10 /LC 1 /LJ 1` | **1** | **Butt / Miter** |
| nothing at all | 1 | Butt / Miter |

A page setting its stroke width through an `/ExtGState` was indistinguishable from one
setting nothing.

**The struct already knew.** `StrokeStyle`'s five fields are documented by these key
names — "How stroke ends are terminated (`/LC`)" — so the type said where the values come
from while the only code that could put them there did not.

Found by a sweep that asked a different question from the usual one: **of the 277
`#[pdf_key]` declarations in the schemas, which name a key that no acting code reads?**
Eleven, in three clusters, of which this is one. `silent_branches.py` cannot see this
class — it reads wildcard match arms, and a key simply not looked up is not an arm.

## Decision

`apply_gs_stroke_style` applies all five, beside the existing `apply_gs_font`.

**An undefined enumerant is recorded, as `J` and `j` already do for the same values.**
`/LC 7` and `/LJ 7` take the initial state's value and raise a `Decision` against Table 53
or 54 — the Rule 20 treatment those operators gained on 2026-08-30, now reachable by the
route that had no path at all. `/LW` and `/ML` take any number, so there is nothing in
them to refuse.

`/D` is `[dashArray dashPhase]` where `d` takes the two as separate operands, so it is
unpacked rather than shared.

## Consequences

- **PDFKit says the new output is the right one.** `PDF-versions3.pdf` is the one file in
  515 external documents that carries these keys — `/LW 4 /LJ 1` — and its first page,
  rendered at 133×133:

  | | non-white pixels of 17,689 |
  | :--- | ---: |
  | fepdf before | 11,449 |
  | **fepdf after** | **12,318** |
  | **PDFKit** | **12,317** |

  One pixel apart, against 868 before. This is the evidence `TESTING.md` asks for before a
  render is called better, and it comes from a second implementation rather than from the
  clause alone.
- **Corpus prevalence is 1 of 515 external files and 1 of 9 samples**
  (`volvo_xc90.pdf`, `/LW 2 /ML 4`, on pages that turn out not to stroke — its first eight
  render identically either way). Zero occurrences would not have argued against building
  this and one does not argue for it; the standard does, and the measurement says what to
  expect rather than whether to act.
- **Three tests, and the second is what makes the first mean anything.** The `gs` form is
  asserted equal to the operator form rather than to literals — the question is whether
  the two routes agree — and the second test then asserts that the state they reach is not
  the initial one, without which an engine ignoring *both* routes would pass.
- **`/Type /ExtGState` is optional**, which cost two passes of the corpus scan: the first
  search for these keys required it and found nothing in `volvo_xc90.pdf`, whose two
  dictionaries omit it. Recorded because the same mistake is available to anyone counting
  ExtGStates.
- **The visual regression suite is red on `constitution.pdf`, and was before this work.**
  Verified by running it at `bae7be0`, the commit this session began from: 65 pixels at a
  maximum channel delta of 201, against a suite whose tolerance is 1. It is not caused by
  this change — `constitution.pdf` carries none of these keys — and it is not diagnosed
  here. It is worth knowing that the suite is not run by `verify_compliance.sh` or by
  `cargo test`, so a red result blocks nothing and had not been looked at.

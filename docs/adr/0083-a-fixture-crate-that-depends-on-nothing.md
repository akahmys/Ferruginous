# ADR-0083: The hand-written PDFs move into a crate that depends on nothing

- **Status**: Accepted.
- **Date**: 2026-09-08
- **Commit**: (see the commit that adds this file)

## Context

A test in this workspace reads bytes it wrote itself. Every such test needed a
cross-reference table and a trailer, and every one of them wrote its own: **thirty-two
hand-written assemblers on 2026-09-06**, of which twelve were byte for byte the same
eighteen lines.

Two consolidations had already happened and both stopped at a crate boundary.
`crates/fepdf/tests/common/mod.rs` took ten copies on 2026-09-06 and eleven more on
2026-09-08; `crates/fepdf-doc/tests/common/mod.rs` and
`crates/fepdf-script/tests/common/mod.rs` each took their own. The doc crate's copy said
why in its own header: *a `tests/common/` cannot be shared across crates without a
dev-dependency crate, so each consolidates its own.*

That is the decision this record takes.

## What the earlier consolidations paid for

Two constraints were not obvious in advance and both were learned by hitting them.

**A fixture the writer produces cannot catch a writer defect.** The bytes are written by
hand deliberately. A crate that assembled them through `PdfWriter` would pass every test
the writer broke, and a test that builds its own bytes says in its own body what the
reader is being given. So the crate does not depend on `fepdf`, and by default it depends
on nothing at all.

**An object body is bytes, not a `String`.** The first consolidated signature took
`&[String]`, and that is why two tests could not use it: `image_sample_count_test.rs`
carries raw one-bit sample data and `smask_in_data_test.rs` a JPX codestream, neither of
which survives a `String`. Both were recorded as legitimate exceptions on 2026-09-08 and
neither was one — the signature was wrong. `crates/fepdf-model/examples/make_scan_fixtures.rs`
and `make_mesh_fixtures.rs` had the same shape and had solved it by hand, each with its own
loop that interleaves text bodies and binary streams.

## Decision

`crates/fepdf-fixtures` is a workspace member with `publish = false`, declared as a
dev-dependency by the crates that need it.

- `assemble(&bodies)` writes a one-revision PDF 2.0 with `/Root 1 0 R`. Bodies are
  `AsRef<[u8]>`, so `&str`, `String` and `Vec<u8>` all pass.
- `Pdf` says the rest: `version`, `root`, and `trailer_entries` — raw, because every
  caller of it is testing what a reader does with particular bytes, and a typed entry
  would decide those bytes for them.
- The `backend` feature carries `recorder::Recorder`, the one `RenderBackend` the
  workspace's tests share. It reaches `fepdf-content`, which defines that trait and
  implements none of it, and no further.

**Default features pull in nothing**, which is what keeps `fepdf-model` — the crate with
the most fixtures — from dev-depending on a stack that sits above it.

## What is deliberately still hand-written

A test whose subject *is* the cross-reference table must write its own, because an
assembler that got the table right for it would remove what it checks. Four places:

| | why |
| :--- | :--- |
| `fepdf-syntax/src/xref.rs` | multiple subsections, malformed tables — the parser's own tests |
| `fepdf-model/src/reader.rs` | object-stream fixtures with hand-written subsections |
| `fepdf-model/src/file_structure.rs` | a table spanning two revisions |
| `fepdf-model/tests/xref_recovery_tests.rs` | recovery from a table that is wrong |

Two more are production code and were miscounted as fixtures before this: `fepdf/src/lib.rs`
holds the static empty document `create_empty` returns, and `fepdf-model/src/writer.rs`
writes tables because writing tables is its job.

## An `append` that was built and deleted unused

The first version carried `Pdf::append`, for a second revision whose trailer names
`/Prev`. It was written for the three fixtures that have one — and all three turned out to
be tests *of* the cross-reference table, which are exactly the ones that keep writing
their own. It had no caller, and RR-15 Rule 2 caught it: the `expect` it needed to find
the base revision's `startxref` failed the audit, which is the check doing its job on code
that should not have existed.

## The first dev-dependency found a hole in Rule A's checker

`scripts/audit/layering.py` failed the audit on `fepdf-mcp declares fepdf-fixtures, which
is neither the facade nor above it`. That is Rule A firing on a dev-dependency, and Rule A
is not about dev-dependencies: a frontend that declares `fepdf-doc` can decide what the
facade decides, which is the objection, and a crate its tests use to write a fixture
reaches nothing at run time.

The checker did not have a view on this. It matched every line of a `Cargo.toml` beginning
`fepdf`, with no idea which table the line sat in, and no frontend had a workspace
dev-dependency for it to be wrong about until this one. It reads the shipping tables now —
`[dependencies]`, `[build-dependencies]` and the per-target forms — and leaves
`[dev-dependencies]` alone. Proved still to fire by declaring `fepdf-doc` under
`[dependencies]` in `fepdf-mcp` and watching the audit fail.

**This is the second defect that check has surfaced in itself since it was written on
2026-09-07**, and both were found by legitimate work rather than by review: the rule it
enforces was right and the thing enforcing it was narrower than the rule.

## Consequences

`crates/fepdf/examples/glyph_loss.rs` was the thirteenth hand-written `RenderBackend` and
the one the `tests/`-local recorder could not reach, an example being unable to declare a
module under `tests/`. It replays the recorder's events now, and it reports the same
**1,137 glyphs lost of 16,321,270** that `ROADMAP.md` quotes — which is what says the
rewrite changed nothing.

`Event::Text` carries the run's glyphs rather than their text joined into a string. Joining
throws away the glyph that reached no character, and that glyph is the whole subject of
`glyph_loss`.

The fixture-generating examples were checked by running them and comparing the files byte
for byte against what they produced before.

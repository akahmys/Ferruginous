# ADR-0070: A command that located the structure tree instead of printing it

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

`fepdf inspect tree` is documented **"Dump hierarchical logical structure tree"**. On
`samples/fugaku.pdf`, which is tagged, it printed:

```
--- [ DOCUMENT STRUCTURE ] ---
Structure Tree Root found: Handle<Object>(93)
```

`print_structure` asked the document for the root handle and formatted it. Nothing walked.
It was two lines of output for every tagged document in the corpus.

**The engine already had the walk.** `StructureTreeVisitor::extract` builds the tree the
GUI's node registry and `fepdf-mcp`'s struct-tree resource both read. Only the CLI called
the stub — which is why nothing noticed: two of the three frontends were right.

**And the one line it printed was a handle's `Debug`.** `Handle<Object>(93)` is the
storage model, which Rule A keeps out of frontends; it reached a user through a printed
report, a door that rule does not watch.

## Decision

`print_structure` walks with `StructureTreeVisitor` and renders one element per line,
indented by depth, with the page and `/Alt` where an element carries them — those two
because they are what someone reading an accessibility tree is looking for, and a tree
showing neither would be a list of tag names.

Two other things went in the same pass:

**`ctm.rs` is deleted.** Eighty-eight lines with a `#[test]` in them, never declared as a
module, and so **not compiled since 2026-05-01** — four months. Checked before deleting,
because "no caller" is not by itself a reason:

| What it held | Where that lives now |
| :--- | :--- |
| clip-depth tracking | `VelloBackend.clip_count` and `GraphicsState.clip_count`, both live |
| a CTM stack | the interpreter's `state.ctm` |
| the y-flip for display | `fepdf/src/lib.rs`, which also handles rotation |

Nothing in it was unique, and `cargo` says nothing about a `.rs` file no `mod` names.

**One PDF assembler per crate, instead of fourteen.** Twelve were byte for byte the same
eighteen lines and this session added two more before counting them. Ten are now
`crates/fepdf/tests/common/mod.rs` and two `crates/fepdf-script/tests/common/mod.rs`; the
remaining two are alone in their crates and have nothing to share with.

## Consequences

- `fugaku.pdf` goes from 2 lines to **130**, `print_sample.pdf` to **1,252**,
  `volvo_xc90.pdf` to **23,420**. The six untagged samples say "No logical structure
  found." as before.
- **Verified by putting the stub back**: three of the four tests fail. One of them asserts
  that no storage vocabulary — `Handle<`, `PdfArena` — appears in the output, which is the
  half a test about content would not have caught.
- **The `RenderBackend` stubs are not consolidated, and that is a decision.** Twelve test
  backends carry 652 lines between them, and in eight of them **22 of 23 methods are pure
  no-ops** — each test overrides one. A macro to fill in the rest would have to compute a
  set difference in `macro_rules!`, and it would hide which methods the trait has. The
  trait deliberately gives no defaults, so that a real backend cannot forget `fill_path`
  and silently draw nothing; a tool that papers over the same surface for tests points the
  other way. What the 22-of-23 figure really says is that these tests want a narrower
  observation trait than the one the renderer needs, and that is a design change rather
  than a cleanup.

# ADR-0073: Rule 13's other half, and a type that was a check all along

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

Four items were left open by [ADR-0071](0071-three-declarations-that-read-nothing-and-one-that-wrote-nothing.md).
Three are settled here, and the shape they share is that **the sweep's usual verdict was
wrong for all of them**: what looked like dead declarations was a check, and what looked
like a lint preference was a defect.

### Type 3 fonts had no schema type, and a 72-font document reported nothing

`samples/fugaku.pdf` — 25 pages of Japanese, 72 fonts, every one Type 3 — read
`Validated Clauses: 14.3.3, 7.7.2, 7.7.3.3`. Not one font clause, not even 9.2. The cause
is correct behaviour meeting a gap: Arlington's `FontType3` has **no `/BaseFont`**, and
`PdfFont`'s gate requires one, so the general font type rightly never matched and there
was nothing else to match instead.

`PdfType3Font` (9.6.5 — the clause `font/mod.rs`, `interpreter/ops/text.rs` and
`interpreter/font.rs` already name for Type 3) carries Arlington's seven required entries.
`/CharProcs` and `/Encoding` are typed `Object` rather than `Handle<Object>` because both
are written directly as often as by reference, and a type accepting only the reference
form would report the clause unmet for half of them.

Across the corpus, 9.6.5 goes from 0 files to 7.

### `PdfFontDescriptor`'s eight metrics read nothing — and must stay

`/FontBBox`, `/ItalicAngle`, `/Ascent`, `/Descent`, `/CapHeight`, `/StemV` and the two
1.5 additions are read by no code, by any route. Measured: 0 sites each, including the
raw-dictionary reads that [ADR-0067](0067-a-substitute-face-is-declared-not-guessed.md)
uses for `/Flags`. That is ADR-0017's shape exactly, and deleting them would have been
wrong.

**The parse is the check.** `audit_specific_types` reports 9.8 when
`PdfFontDescriptor::from_pdf_object` succeeds, so which fields are required decides what
"this document met 9.8" means. The fields are not unread; they are read by the derive.

Checking them against Arlington found one divergence: `/StemV` is `Required = TRUE` in
`FontDescriptorType1`, `FontDescriptorTrueType`, `FontDescriptorCIDType0` and
`FontDescriptorCIDType2`, and `Option` here. It stays `Option`, measured rather than
assumed: requiring it changes the corpus not at all — **232 of 524 files report 9.8 either
way** — while `FontDescriptorType3` has it `FALSE`, so requiring it would stop reporting
9.8 for the one case the standard says may legitimately omit it. One schema type covers
five Arlington types with different required sets, and it takes the loosest.

Two tests now hold this, because without them the next reader sees eight unread fields
and takes the usual course.

### `let _ = f();` is Rule 13, and the checker could not see it

Rule 13 forbids swallowing an error in silence; its check greps for
`filter_map(Result::ok)`. The same swallow written as a binding was invisible.

`HeuristicEngine::infer_structure` carried one. It ran each page's content stream to
collect text spans, then inferred headings, tables and paragraphs from them — and the run
was `let _ = interpreter.execute_raw(&data);`. A page whose stream failed produced an
empty span list and contributed no candidates, **reported identically to a page read
perfectly and found to have no structure.** A caller asking what structure a document
could gain was told "none here" about a page nobody had read.

Two more were found by the same sweep and are worse for being invisible in a GUI:

* `handle_update_node` discarded `doc.apply(UpdateStructElem)` and then ran the Matterhorn
  audit on the tree as it stood. A failed edit sent the user a fresh set of findings for
  the *unchanged* document — the one screen that could have told them the edit did not
  take was the screen showing the old tree as if it were new.
* Both `render_to_texture` calls discarded their failure, leaving the previous frame in
  the texture with nothing to say the page on screen was stale.

Four `Command::spawn()` discards now log: a viewer that will not start is a fact about the
host, so a `log::warn!` and not a `Decision` (AGENTS.md §4.3).

`execute_raw` itself had the same shape one level down — `while let Ok(token) =
parser.peek()` ended the loop on a lexer failure and returned `Ok(())`, so a stream that
stopped being readable was reported as one that ran to the end. It now records a 7.8.2
violation and stops. **Eight malformed streams were tried and none reaches it**; this is a
shape fixed on inspection, not a defect demonstrated, and the record says so.

## Decision

Add `PdfType3Font` and dispatch on it. Keep `PdfFontDescriptor`'s metrics, with the
`/StemV` looseness recorded and two tests pinning the required set. Fix the four real
`let _ =` discards, and add `scripts/audit/discarded_results.py` as a second Rule 13 step.

## Consequences

- **`clippy::let_underscore_must_use` was tried first and rejected, measured.** It has the
  type information the script lacks and reports 97 sites — but 86 are a `write!` into a
  `String`, which cannot fail, or an `mpsc` `send` whose receiver has hung up, meaning the
  GUI is closing. A check that is 89% noise is a check nobody reads. The script names
  those two shapes benign and demands a written reason for the rest.
- **Six discards were fixed by changing their shape rather than exempting them.**
  `let _ = x.map(|v| { side effect });` over an `Option` or a `Result` became `if let`,
  which is what it meant; `let _ = set.insert(h);` lost a binding it never needed.
- **Seven sites remain, each with its reason in `ACCOUNTED_FOR`** — consume-what-peek-showed
  in three parsers, an effect checked instead of a `Result` in `perform_reconstruction`,
  and a `Vec` return that is not a `Result` at all. An entry naming a line that no longer
  discards anything fails as stale, so the list cannot become a permanent exemption.
- **Verified by breaking, and two of those breaks were no-ops that had to be caught.**
  A `sed` written against pre-`cargo fmt` text missed its target twice, and the tests
  passing where they should have failed is what revealed it. The first version of
  `an_incomplete_font_descriptor_does_not_report_its_clause` pointed at an object absent
  from the file, so it passed on a dangling reference and proved nothing.
- **The arena's clone-on-read was the wrong suspect**, which a symbolised profile settled
  the same day — see [ADR-0074](0074-the-reader-copied-the-file-once-per-object.md). The
  33.7 s was `parse_indirect_at` copying the rest of the file once per object.

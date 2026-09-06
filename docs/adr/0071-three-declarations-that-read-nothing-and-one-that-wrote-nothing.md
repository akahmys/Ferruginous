# ADR-0071: What the unreferenced items turned out to be

- **Status**: Accepted; its open items are closed by [ADR-0073](0073-rule-13s-other-half-and-three-declarations-that-were-checks.md) (Type 3, the descriptor's metrics, Rule 13's `let _ =`) and [ADR-0074](0074-the-reader-copied-the-file-once-per-object.md) (efficiency — and the hypothesis this record named, that the arena clones on every read, was wrong)
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

A sweep for public items that no other code names returned 35. This record settles 24 of
them, and they do not all go the same way, because **unreferenced is a symptom and not a
diagnosis.** Three of them were a coverage gap, one was a divergence between four copies
of the same rule, one was a test that stopped one accessor short — and the rest were
containers for jobs nobody does.

### `PdfType1Font`, `PdfTrueTypeFont`, `PdfType0Font` — used, not deleted

`font/schema.rs` declares the three, carrying clauses 9.6.2, 9.6.3 and 9.7. Nothing named
them. Beside them, `audit/compliance.rs::audit_specific_types` dispatched on `/Subtype`
for `OpenType`, `CIDFontType0` and `CIDFontType2` — **the three most common font types
were the three it did not ask about.**

The measurement:

```
samples/constitution.pdf   (24 fonts, every one a TrueType)
before   Validated Clauses: 14.3.3, 7.7.2, 7.7.3.3, 9.2, 9.8
after    Validated Clauses: 14.3.3, 7.7.2, 7.7.3.3, 9.2, 9.6.3, 9.8
```

Across the whole corpus — 9 samples plus 515 external files, 524 in all — the three
clauses were reported on **0** files before and on 273 after:

| clause | before | after |
| --- | ---: | ---: |
| 9.6.3 TrueType | 0 | 162 |
| 9.7 Type 0 | 0 | 65 |
| 9.6.2 Type 1 | 0 | 46 |
| 9.6.4 OpenType | 0 | 0 |

This is the shape [ADR-0017](0017-declaring-a-catalogue-key-is-not-modelling-it.md)
warns of, and the honest reading of it is not always deletion. `PdfFont` (9.2) was already
reported on 256 files, so the engine was parsing these fonts and declining to say which
clause defined them; the reader was one match arm away. **A coverage figure that
understates itself is worse than one that is missing**, because a reader takes the list
for what the engine checked.

The three arms were folded into the existing `/Subtype` dispatch rather than appended, so
one `match` now names every font subtype the schema models. `audit_specific_types` split
into `audit_font_subtypes` and `audit_structural_types` to stay inside RR-15 Rule 1.

### `write_incremental_update` — deleted

[ADR-0014](0014-the-faithful-copy-path-is-not-built.md) already recorded the decision:
"It should be deleted when someone is confident nothing else wants it." Nothing does. The
sweep confirms no caller in any crate, test or script.

It would also have been wrong if wired. It takes `_root_handle` and discards it, writing
`/Root 2 0 R` as a literal; and it calls `generate_file_id(None)`, minting a fresh
identifier for both halves of `/ID`, where 7.5.5 requires an update to carry the original
file's first element forward. A function that cannot be called correctly is not a
half-built feature, it is a trap with a plan attached.

### Two GUI panels that fabricated their contents — deleted

`sidebar/bookmarks.rs` listed `🔖 Chapter {i}: Page {i}` for `i` in `1..=5` — invented
text, not the document's outline, which the engine extracts correctly elsewhere.
`sidebar/attachments.rs` always said "No embedded attachments found." and offered an
"Add Attachment..." button whose click body was a comment. Neither module was reachable:
`ActiveDrawer` has no variant for either, both carried `#[allow(dead_code)]`, and nothing
outside the files named them.

`layers::show_layers` went with them — a wrapper carrying `#[allow(dead_code)]` and an
`// RR-15 Limit:` marker, over a `show_rows` that `document_info` calls directly and an
empty-list branch `document_info` already handles by omitting the section.

### Four copies of "is this font subsetted", in two disagreeing forms — unified

`FontResource::is_subsetted` was unreferenced. Looking for who *should* have called it
found that nobody needed to, because three other places had written the test themselves:

| site | rule |
| --- | --- |
| `font/mod.rs` `is_subsetted` | `base_font.contains('+')` |
| `font/mod.rs` `FontSummary` | `len() > 7 && bytes[6] == b'+'` |
| `ingest/mod.rs` registry | `len() > 7 && bytes[6] == b'+'` |
| `fepdf-font/reconstruction.rs` | `!base_font().contains('+')` |

The loose form calls `Arial+Bold` a subset; the positional form accepts `abc123+Arial`.
Neither is the rule, which is exactly six **uppercase** letters and a `+`.

**Measured before changing anything**: across the 524-file corpus there are 348 distinct
`/BaseFont` names, 316 contain a `+`, and 316 match `^[A-Z]{6}\+`. **The two forms
separate no file that exists here.** So this is a latent divergence, not an observed
defect, and the record says so rather than claiming a bug it cannot show.

`fepdf_font::subset::subset_tag` is now the one reader, with four unit tests for the cases
the corpus does not contain. `is_subsetted` itself was deleted: with the rule shared, an
accessor nothing calls is what ADR-0017 describes.

### `is_editable_combo` — tested, not deleted

Its two siblings `is_combo` and `is_multiselect` were both asserted in
`choice_field_test.rs`; bit 19 (Edit) was the one the suite stopped short of. Two cases
were added — an editable combo, and Edit set without Combo, which Table 232 makes
meaningless. The same reading as the font clauses: the code was right and the reach was
missing.

### Ten containers for jobs nobody does — deleted

- `refine::text::restructure_content_stream` and six private helpers (183 lines). It
  rewrites a page's text bytes back through `unicode_to_gid` / `unified_map` without
  touching `/Encoding` or `/Widths`, and truncates a simple font's code with
  `*code as u8` under an `#[allow(clippy::cast_possible_truncation)]`. Not a half-built
  feature — a rewriter that corrupts the page it is given.
- `Document::compliance_info` and `document/conformance.rs` (36 + 51 lines). A second
  compliance surface beside `audit/compliance.rs`: `issues`, `output_intents`,
  `pdf_a_part` and `pdf_x_version` were never filled by anything, and `pdf_ua_part` was
  set to 2 for any tagged PDF 2.0 — asserting PDF/UA-2 without one Matterhorn check,
  while `audit_ua2()` runs them.
- `get_page_tree_view`, `build_virtual_balanced_view` and `document/strategy.rs`
  (21 + 30 lines). A virtual balanced page tree computed for no consumer; nothing writes
  it to a file or reads it back. Its exemption in `scripts/audit/unbounded_recursion.py`
  went with it — a stale exemption exits 1, which is how that script says so.
- `serialize_image`, which takes `_width`, `_height` and `_format` and discards all
  three, emitting a Flate stream no `/Image` XObject could use.
- `refine::font::normalize_cmap`, whose doc says "Normalizes a CMap stream to a canonical
  PDF 2.0 form" over a body that returns its input unchanged.
- `info_to_xmp`, a wrapper that calls `info_to_xmp_derived` with a default `Provenance` —
  silently dropping the `xmpMM:DerivedFrom` record ADR-0012 exists to write.
- `Matrix::pre_concat`, `concat` with its arguments swapped. `annotation.rs:740` already
  carries a comment about getting that order wrong once.
- `glyph_name_to_sid` (the inverse direction of CFF strings, needed only by a CFF writer
  that does not exist), `CMap::decode_next_strict` (an alias for
  `decode_next_with_min_len(data, None)`, which every caller uses directly),
  `PdfArena::get_object_entry`/`set_object_entry`, `Object::is_stream`,
  `ColorSpace::is_tinted`, `PdfDocument::get_embedded_fonts`,
  `ObjHandle`/`NameHandle`, and `duplicate_selected_pages`.
- Eight colours from `theme::colors`, **and the `#[allow(dead_code)]` over the module**.
  `CARD_BG` was `PANEL_BG` under a second name; `RUST_BADGE_BG`/`TEXT`/`BORDER` styled a
  badge that is never drawn. With the allow gone the compiler now holds the other
  eighteen — it reports clean, which is the proof they are all painted with.

The `ObjHandle`/`NameHandle` pair had also been inserted *into the middle of*
`Handle<T>`'s doc comment, so the struct's documentation began at
"- **Zero-Cost Abstraction**". Removing them repairs it.

### Twelve names for two types — collapsed to two

`type Dict = BTreeMap<Handle<PdfName>, Object>` was written out **ten times inside
`fepdf-model` alone**, and `DictHandle` three times — twice `pub` in the same crate, so
`fepdf_model` exported one type under two paths (`document::DictHandle` and
`reader::DictHandle`), and once privately in `fepdf-doc`'s `cloning.rs`, which also
imported `BTreeMap` twice under two names in the same file.

Nothing was wrong with any of them: the compiler proves aliases identical, so this cost
no behaviour. It cost the reader. `access.rs` already owned the one `Dict` — it was
created for exactly this reason when the encryption accessors were unified
([ADR-0063](0063-one-set-of-accessors-because-two-disagreed-about-one-dictionary.md)) —
and nine files were still declaring their own beside it. `Dict` now lives once in
`access.rs`, `DictHandle` once in `handle.rs` beside `Handle`, re-exported at the crate
root. `fepdf-doc`'s `appearance.rs` keeps a local `Dict`: `access::Dict` is
`pub(crate)`, and widening the public API to save one line is the wrong trade.

### One heading over three different answers — scoped

`fepdf inspect info samples/fy05.pdf` printed:

```
--- [ DECISIONS TAKEN READING (5.3) ] ---
  1 ambiguities, 0 repairs, 0 violations
  [AMBIGUITY] ISO 14.3.3 : /Info /ModDate is "D:20241114200008+09'00'" and the metadata
  stream says "2024-11-08T09:08:18+09:00" -> took the metadata stream …
```

`fepdf inspect structure` on **the same file**, under **the same heading**, printed
`none — the file was read without departing from the standard`. That sentence is a claim
about the file, and for `fy05.pdf` it is false.

The cause is legitimate and the wording was not. `FileStructure::survey` calls
`reader::load_document` and stops at the layout; `Document::open` goes on to refine, and
refinement takes decisions of its own — reconciling `/Info` against the metadata stream
under 14.3.3 among them. A layout survey has no business reading metadata, so the fix is
not to make it. **Measured across the 524-file corpus, the two disagree on 41 files.**

`survey`'s own comment asserted the opposite — "the decisions reported are the ones a
caller would get from `Document::open`" — so the false statement existed twice, once for
the user and once for the next maintainer.

`render_decisions_text` and `render_decisions_markdown` now take the scope they are
reporting on, and the empty case names the reading that found nothing: *the file's
layout*, *the catalogue*, *the encryption dictionary*, *this document's interactive
features*, *reading this document*. `interpretation.rs` gains a test that opens a file
whose `/Info` and XMP dates disagree and asserts `Document::open` records 14.3.3 while
`FileStructure::survey` does not — so the wording cannot drift back to a claim the code
does not support.

## Decision

Wire the three font schema types into the compliance audit; give `subset_tag` one reader
and route all four sites through it; test `is_editable_combo`. Delete
`write_incremental_update`, the ten containers above, `sidebar/bookmarks.rs`,
`sidebar/attachments.rs`, `layers::show_layers`, and the four locale keys the deleted
panels alone used.

## Consequences

- **Three `#[allow(dead_code)]` sites and one `// RR-15 Limit:` marker go with the code
  they covered.** Category 7 of the audit brief: the allow was not hiding a lint quirk,
  it was holding a door open for something nobody was coming through.
- **`crates/fepdf/tests/compliance_clauses_test.rs` guards the clause list in both
  directions.** Three tests assert a font's clause is reported; a fourth asserts a
  document with no font reports none of the three, so an unconditional insert — which
  would pass the first three — fails. Both breakages were performed and observed.
- **9.6.4 is reported on no file in the corpus and stays.** OpenType fonts (9.6.4) are
  parsed and dispatched; the corpus simply has none. Per RR-15 Rule 20 this is "no
  example", not "not needed", and the arm is kept.
- **Type 3 fonts have no schema type, and `samples/fugaku.pdf` shows what that costs.**
  Its 72 fonts are all Type 3, and it reports no font clause at all — not even 9.2, since
  Arlington confirms `FontType3` has no `/BaseFont` and `PdfFont`'s gate requires one.
  Recorded here as the next gap, not closed in this change.
- **ADR-0014's open consequence is closed.** ADR-0012's item was moved to *Not planned*
  by 0014; the code that item referred to is now gone.
- **Every change was verified by breaking it.** `subset_tag` forced to `None` turns
  `constitution.pdf`'s `Sub` column from `✅` to `−` and fails its unit tests;
  `is_editable_combo` reduced to `is_combo()` fails the new pair; dropping the `Type0`
  arm fails one clause test and inserting all three unconditionally fails the control.
- **The two locale files are still key-for-key identical** (218 each), checked after the
  four deletions rather than assumed.
- **`document.rs`'s module header lost a doc comment to the module below it** when
  `pub mod strategy;` went, and `reader.rs`'s module doc was briefly cut in half by a
  scripted import insertion. Both were caught by the compiler and repaired; noted because
  a doc comment silently re-attaching to the next item is a way a deletion goes wrong that
  a build does *not* catch, and the first one only failed loudly by luck.
- **The sweep went from 35 unreferenced public items to one.** The one left is
  `fepdf_macros::derive_from_pdf_object`, a **false positive**: a proc-macro entry point
  is named by `#[derive(FromPdfObject)]`, not by a call, so a sweep that counts
  identifiers cannot see its users. `FontResource::has_any_mapping` joined the list
  *because* of these deletions — it had one caller, inside `restructure_string` — and was
  removed with it.
- **43 files, +196 / −501 lines**, of which the additions are one test file, six unit
  tests, one shared function, one shared alias and this record.

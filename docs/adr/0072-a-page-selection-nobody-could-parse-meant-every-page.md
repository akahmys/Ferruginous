# ADR-0072: A page selection nobody could parse meant every page

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

The MCP surface takes page selections as strings — `"all"`, `"2"`, `"1-3"`, counting from
1. Three separate parsers read them, in `operations/page.rs`, `operations/vocabulary.rs`
and `operations/decoration.rs`. All three ended the same way:

```rust
_ => PageSelection::All,
```

**So every string they could not read selected the whole document.** Measured by running
the parser over probe inputs:

| input | `page.rs` | `vocabulary.rs` |
| --- | --- | --- |
| `"foo"` | All | All |
| `""` | All | All |
| `"2,3"` | All | All |
| `"0"` | page 1 | page 1 |
| `"-1"` | page 1 | page 1 |
| `"3-"` | *nothing* | page 3 |
| `"5-x"` | *nothing* | page 3 |
| `"2-4"` | pages 2–4 | pages 2–4 |

Two failures compound here.

**The `All` fallback is destructive on the tool that removes pages.** `remove_pages` with
`pages: "foo"` deleted every page of the document and returned SUCCESS. `"2,3"` is not an
exotic input — a comma-separated list is the first thing a caller writes when a schema
says "1-3" — and it deleted the file's contents just as thoroughly. Guessing is bad; of
the available guesses `All` is the worst one, because on three of these five tools it is
the maximally destructive reading.

**The two copies disagreed.** `unwrap_or(first)` in one and `unwrap_or(1)` in the other
made `"3-"` mean page 3 to `duplicate_pages` and mean nothing at all to `remove_pages` —
the second producing the range `4..=0`, which is empty. One string, two tools, two
answers.

A fourth site was worse than divergent. **`apply_bates_numbering` never read its `pages`
field**: the body opened `let pages = PageSelection::All;` and the argument went nowhere,
while the schema told every caller "Selection of pages, **counting from 1**: `"all"`,
`"1-5"`. Default: `"all"`." A test confirmed it — asking for page 1 of a three-page
document stamped all three.

## Decision

One `parse_selection` in `operations/mod.rs`, returning `Result<PageSelection, String>`.
`None` (for the tools whose field is optional) and `"all"` mean every page; a bare integer
≥ 1 means that page; `a-b` with both ≥ 1 and `a ≤ b` means that inclusive range. Anything
else is an error naming what was not understood. All five call sites go through it,
`apply_bates_numbering` included.

Refusing rather than extending: `"2,3"` could have been supported instead, but the schemas
document three forms and an error is what tells the caller the difference. A tool that
guesses cannot be corrected by its caller, because the caller never learns it guessed.

## Consequences

- **This is RR-15 Rule 13 in a place the rule's checker cannot see.** Rule 13 forbids
  silently swallowing errors; a `_ =>` arm that turns an unparsed string into the largest
  possible selection is exactly that, written as control flow rather than as a discarded
  `Result`. The checker looks for `filter_map(Result::ok)`, not for this.
- **Five tests, verified by breaking all of them.** Restoring the `All` fallback fails
  `an_unparsable_selection_is_refused_rather_than_meaning_all`; restoring the
  `unwrap_or` divergence fails `both_page_tools_read_one_selection_the_same_way`;
  restoring `let pages = PageSelection::All` in the Bates tool fails
  `bates_numbering_stamps_the_pages_it_was_given`. Each was performed and observed.
- **`the_documented_selection_forms_still_name_their_pages` is the other half.** Refusing
  is only right if the documented forms still work; `"all"` on `remove_pages` still
  removes every page, because that is what it was asked for.
- **The two page bases are still two.** `page` integers are 0-based and `pages` /
  `selection` / `page_range` strings are 1-based, on one surface; that remains documented
  rather than unified, for the reason [ADR-0071](0071-three-declarations-that-read-nothing-and-one-that-wrote-nothing.md)
  records — unifying moves every existing caller's pages by one, silently. What is fixed
  here is that the 1-based strings now agree with each other.

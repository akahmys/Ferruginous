# ADR-0067: A substitute face is declared, not guessed

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

A font the file does not embed has to be drawn with something else. `infer_fallback_type`
chose that something by **substring**: `contains("century")` meant serif,
`contains("gothic")` meant Japanese sans, and anything unrecognised fell to `Default`.

Meanwhile `/FontDescriptor` `/Flags` — the entry the standard provides for precisely this
— was declared in the schema, checked for *presence* by the compliance audit, and **read
by no code at all**. `FallbackFontType::Default`'s own doc comment says "the loader picks
by descriptor flags". It did not.

**This is [ADR-0041](0041-a-character-collection-is-declared-not-guessed.md)'s finding,
one field over.** That record is "a character collection is declared, not guessed", after
the engine decided a CID font's collection from `/BaseFont` substrings while
`/CIDSystemInfo` said so outright. The *shape* of a face was still being guessed the same
way, from the same string, eight weeks later.

**Arlington settles the order.** The machine-readable model in `external/arlington` gives:

```
FontDescriptor   required = fn:IsRequired(fn:SinceVersion(2.0) || fn:NotStandard14Font())
Flags            required = TRUE    type = bitmask
FontFamily / FontStretch / FontWeight   since = 1.5
```

So in **PDF 2.0 a descriptor is required on every simple font** — the standard-14
exception was removed — and `/Flags` is required within it. A reader that asks the name
first is asking the guess before the declaration.

## Decision

Four questions, in the order the file answers them:

| Question | What answers it | Clause |
| :--- | :--- | :--- |
| Is it CJK, and which collection? | `/CIDSystemInfo` `/Registry` and `/Ordering` | 9.7.3, Table 114 |
| Is it fixed-pitch? Is it serif? | `/FontDescriptor` `/Flags`, bits 1, 2, and 6 by elimination | 9.8.2, Table 121 |
| There is no descriptor at all | Then it is one of the standard 14 and its name is its identity | 9.6.2.2 |
| Nothing above answered | The name, and failing that a sans | **not decided by ISO** |

Script before shape, because a Latin face substituted for a CJK one draws nothing while
the wrong weight draws the right characters.

`Symbol` and `ZapfDingbats` are in the fourteen and deliberately get no stand-in: putting
their code points through a text face draws the *wrong* glyphs rather than similar ones.

The last row is the only guess, and it is named one in the code. Which concrete file
stands in for each category is also not something ISO decides; that is `fallback_fonts`.

## Consequences

- **The visual regression suite is green for the first time since 2026-09-05.**
  `constitution.pdf` had been failing on 42 pixels — the page number `1`, drawn in a
  non-embedded `Century`. Bisected to `bae7be0`, which unified three disagreeing fallback
  assemblies; rendered at 72 pt the old glyph has a full-width horizontal slab where a `1`
  has a diagonal flag, and **PDFKit draws the diagonal flag**. The baseline predated the
  repair, and is refreshed on that evidence rather than on the count.
- **The suite is run by neither `verify_compliance.sh` nor `cargo test`**, so it was red
  for a day without blocking anything. Not changed here, and worth knowing.
- **Verified by putting the order back.** Asking the name before the descriptor returns
  `Some(SansSerif)` where the file declares `Serif`; the test that catches it is an
  `/BaseFont /Arial` whose descriptor says serif, because that is where a name and a
  declaration disagree and only one of them is the file's own statement.
- **Weight and slant are read by nothing and cannot be.** `/FontWeight` (1–1000),
  `/FontStretch` (nine names) and `/ItalicAngle` are all in the descriptor and all in the
  standard for this purpose, and the five buckets — serif, sans, mono, and two Japanese —
  have nowhere to put them. Using them means carrying more faces, which is a different
  decision from this one.
- **Six more descriptor entries stay unread**: `/FontBBox`, `/ItalicAngle`, `/Ascent`,
  `/Descent`, `/CapHeight`, `/StemV`. All are *required* by the standard and all are what
  a reader would use to size a substitute to the space the original occupied. This
  decision picks a face; it does not fit one.

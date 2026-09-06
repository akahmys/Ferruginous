# ADR-0069: Two things the file said that nothing read

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

Both came out of the same sweep: **of the 277 `#[pdf_key]` declarations in the schemas,
which name a key that no acting code reads?** Eleven, in three clusters.
[ADR-0065](0065-gs-reaches-table-57s-line-parameters.md) took the first — `gs` and Table
57's line parameters. These are the other two.

**An unknown colour space was guessed from the operand count.** Measured 2026-09-06:

```
/DeviceRGB  cs 1 0 0 sc   ->  red
/Frobnicate cs 1 0 0 sc   ->  red, byte for byte the same image
/DeviceCMYK cs 0 1 1 0 sc ->  red
/Frobnicate cs 0 1 1 0 sc ->  red, the same again
```

decisions recorded: **0**. `CODING.md`'s Rule 5 section names this exact failure as the
one a catch-all must not produce — "a catch-all turns 'unsupported colour space' into
'silently renders black'" — and `silent_branches.py` reads 0 because it looks for wildcard
arms over a numeric value from a file, and a colour space that resolves to nothing is not
an arm.

**`/DW2` was hard-coded.** `glyph_vertical_metrics` returned `(-1000, w0/2, 880)` written
into the function. Those are 9.7.4.3's *defaults*, so a font that declares nothing was
laid out correctly and a font that declares anything else was laid out as though it had
not. `/DW` beside it had been read from the file all along, and `/W2` too.

## Decision

**The colour space records and does not refuse.** A `cs` operand that is neither a device
family nor a `/ColorSpace` entry raises a `Decision` naming 8.6.3, saying what it was
given and that the model came from the operand count. What is drawn does not change: the
guess is often right, and *often right* is exactly what a silent acceptance looks like.
Changing the colour is a separate decision, and only the recording is taken here.

`/Indexed` reaches the same path and is deliberately **not** recorded — its operand is an
index into a palette rather than a colour, it takes the operand-count route on purpose,
and a line per indexed image would bury the case this is for.

**`/DW2` is read into `FontMetrics` as `(position_y, displacement_y)`**, defaulting to
`[880 -1000]` when absent. A malformed entry leaves the default *whole*: a font cannot
declare one number and inherit the other, because a declared position vector against a
default displacement is a layout nobody asked for.

## Consequences

- **Neither changes the corpus.** The colour recording fires on **0 of 524** files and
  every colour probe renders byte-identically before and after. `/DW2` moves no sample's
  extracted text, and `crosscheck_reading_order.sh` exits 0 with no file below its floor
  — which is the check that matters, because vertical Japanese is where `/DW2` decides
  anything at all.
- **Verified in both directions.** Removing the colour recording fails the test; recording
  on *every* `cs` fails two others, which is what stops a recorder that buries its own
  signal. Removing the `/DW2` read returns `(880, -1000)` where the font said
  `(760, -880)`.
- **A test helper that hands over the arena.** Two tests in this session were written with
  a value built in one `PdfArena` and the dictionary in another; the handles collide and
  the reader silently sees something else, with no error anywhere. Both helpers now take
  the arena as an argument so the shape is not available.
- **Six descriptor entries and three substitution entries remain unread**, listed in
  [ADR-0067](0067-a-substitute-face-is-declared-not-guessed.md). This closes the sweep's
  third cluster; the first two clusters are one ADR each and the font-metric one is open.

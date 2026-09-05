# ADR-0064: Redaction removed the second run of a page and no other, because two counters counted differently

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

Redaction is reachable from `fepdf-mcp` and the viewer and not from the CLI. Its only
test asserted `is_ok()`. **The operation had therefore never been shown to remove
anything**, and measured on 2026-09-06 against one page carrying four text runs it did
not:

| Rectangle | Text afterwards |
| :--- | :--- |
| over the first run | `AAA BBB CCC DDD` — unchanged, and `Ok(())` |
| over the second run | `AAA [REDACTED] CCC DDD` |
| over the third or fourth | unchanged, and `Ok(())` |
| **the whole page** | `AAA [REDACTED] CCC DDD` — **three of four survived** |

A caller who redacts `SECRET` gets a file where `PUBLIC` is gone and `SECRET` is still
there. That is worse than doing nothing, because the file looks redacted.

**Two counters, and nothing compared them.** `Interpreter::op_index` counts operator
tokens and is incremented *before* the operator runs, so a page of four `Tj`s reports
**4, 9, 14, 19**. `rewrite_redacted_tokens` re-lexed the stream and counted operators
**and strings**, testing a string before incrementing, so it offered **3, 9, 15, 21**.
The two sets meet at 9 and nowhere else, which is the whole of the observed behaviour —
the arithmetic predicts every row of the table above.

**And a second defect underneath it.** `let _ = interpreter.execute_raw(&data);` dropped
the failure of the pass that collects the spans. A page that will not interpret yields no
spans, so nothing intersects the rectangles, so the scrub finds nothing to do — and the
caller is told `Ok(())` about a page whose text is still there. RR-15 Rule 13 forbids
silent error swallowing and is checked by a grep for `filter_map(Result::ok)`, which does
not see `let _ =` on a `Result`. This is the second such site found by reading in two
days ([ADR-0062](0062-a-page-tree-that-is-not-a-tree-is-reported-not-expanded.md) is the
first).

## Decision

**A string is scrubbed because of the operator that shows it.** The rewriter holds
operands until a keyword arrives, counts only keywords — the interpreter's rule — and
scrubs the held strings when that operator's index is in the set *and* the operator is
one of 9.4.3's four. `TJ`'s array arrives as its own tokens, so every string in it is
scrubbed and the kerning numbers between them are not; `"` keeps its two leading numbers
for the same reason.

**A page that will not interpret is refused.** For an operation whose whole purpose is
that something is *gone*, "I could not read it" and "there was nothing there" must not
arrive as the same answer.

## Consequences

- **Four tests, and the third is the one that keeps the fix honest.** Each run in turn
  including the first; the whole page removing all four; a rectangle over empty space
  removing nothing — without which a rewriter that scrubbed every string would pass the
  other two. Each verified by putting the defect back: the counter one returns
  `left: ["AAA","BBB","CCC","DDD"], right: []`, the swallow one fails on
  "a page that cannot be read cannot be redacted".
- **A document with a broken resource can no longer be redacted at all.** `Do` naming an
  XObject the file does not contain now refuses where it used to return `Ok`. That is the
  intended trade: you cannot redact what you cannot read.
- **The refusal test cannot read the text back to prove it survived**, because
  `extract_text` interprets the same stream and fails the same way. That is the shape of
  the defect rather than a gap in the test: on that page, the only operation that ever
  *succeeded* was the redaction that did nothing.
- **`[REDACTED]` is a substitution, not a removal of the drawing.** The string in the
  content stream is replaced, so the text is gone from extraction and from the page; the
  glyph widths change, so the line reflows. Whether a redaction should also cover the area
  it emptied is a separate question this does not answer.
- **The MCP test that covered this still asserts only `is_ok()`**, and would still pass
  against the defect. It is not changed here — a suite that cannot see this is its own
  item — but it is the reason the defect lasted: the operation had a test, and the test
  had no way to fail.
- **`infer_structure` has the same `let _ = interpreter.execute_raw(&data)`**, one screen
  up in the same file. It feeds `edit tag`, where the cost of a dropped failure is an
  accessibility pass that reports no candidates rather than a redaction that reports
  success. Not fixed here.

# ADR-0078: What sweeping 752 tests found, and what the sweeps got wrong

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0068](0068-a-suite-that-skipped-itself-and-asserted-nothing.md) found the MCP suite
asserting `is_ok()` and skipping itself in silence, and rewrote it. It was found by
accident, while looking at redaction. This is the same question asked deliberately of the
whole suite: **where else does a test pass without checking anything?**

Four sweeps, and the first three were mostly wrong.

| sweep | first count | after checking | real |
| --- | ---: | ---: | ---: |
| no `assert!` in the test body | 35 | 4 | **0** |
| every assertion is only a shape (`is_ok`, `is_err`, `is_empty`) | 61 | 3 | **1** |
| mutates a document and never reads it back | 25 | 0 | **0** |
| passes without running when `samples/` is absent | — | 7 | **5** |

The first sweep's 31 false positives were tests whose assertions live in a helper —
`assert_hidden` and `assert_drawn` in `optional_content_test.rs` carry two each, and 26
tests call them. The second's were `is_err()` and `is_empty()`, where the shape *is* the
claim: `an_offset_past_the_end_is_refused` asserting `is_err()` is asserting the thing it
is named for. The third's were tests that read the arena directly rather than reopening a
file — `test_tier1_operations_execution` checks that the catalogue gained `/PageLabels`,
`/Collection`, `/AF` and `/OutputIntents`, which does catch an operation that did nothing.

The four with no assertion at all are the recursion-bound tests, where a stack overflow
aborts the test binary and that is the failure; both files say so.

## What the sweeps actually found

**One hollow test.** `test_heuristic_retag_execution` applied `Operation::Retag` to a
document with **no pages** and asserted `is_ok()`. It could not have failed for any reason
to do with retagging, and it never checked that `Retag` produces a structure tree at all.
Rewritten against a page the heuristic is built for, asserting that the document starts
untagged, ends tagged, and that the tags include a heading and a paragraph.

**Five tests that passed without running on any machine but the one that generated the
corpus.** `.gitignore` excludes `/samples/`; hiding the directory and running the suite
gives the same 752 passes. Three in `encrypted_objstm_test.rs` and two in
`pattern_color_test.rs` returned early, undeclared. Both files' module docs describe, at
length, real defects the tests exist to catch — an engine that could not read its own
encrypted output, and six pages of `fy05.pdf` that failed on `/P1 scn` and took the 718
pages after them down with them. Those defects were unguarded everywhere else.

`rasteriser_determinism_test.rs` also skips, and declares it: the test costs 18 seconds
and needs a real page.

## Decision

Two different repairs, because the tests are two different things.

`encrypted_objstm_test` builds its fixture and skips nothing. The sample was only ever "a
document with text on page 1", and the subject is the round trip through this engine's own
writer.

`pattern_color_test` keeps its corpus tests — 846 real pages and six named page numbers
are what they measure, and a fixture cannot stand in for that — and gains a test of the
*defect*, which needs no corpus. Disabling the pattern branch reproduces the original
failure exactly: `operator scn [Name("P1")]: Expected number`.

## Consequences

- **Effective coverage on a fresh clone, for those two files: 0 tests to 4.**
- **A detector's output is a list of candidates, not a measurement.** Three sweeps here
  produced 31, 58 and 25 false positives, and every one was opened and read before being
  reported. [ADR-0066](0066-rule-6-gets-a-check-and-the-check-finds-a-sixth-walk.md)
  recorded the same lesson about the recursion detector, which called `extract_name` 579
  lines long and `Object::cmp` recursive. The pattern is now twice-established: a sweep
  finds where to look.
- **The remaining skips are declared in the module doc that carries them**, which is the
  difference between a test that is honest about needing a corpus and one that quietly
  reports success.
- **`calculate_font_size_stats` treats the largest size as the body on a tie.** Found while
  building the retag fixture: it takes the most common font size as the body text, and
  `max_by_key` over a `BTreeMap` returns the last maximum, which is the largest key. A page
  of one 24pt line and one 10pt line therefore has 24pt "body" and no heading. Real pages
  do not look like that, and the fixture was made to look like a real page rather than the
  heuristic changed — but the tie-break is written down here because nothing else says it.

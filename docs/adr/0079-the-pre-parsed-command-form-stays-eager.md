# ADR-0079: The pre-parsed command form stays eager

- **Status**: Accepted
- **Date**: 2026-09-07
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0076](0076-what-the-arena-compresses-and-what-that-was-costing.md) measured what a
read holds — 1,346 MB live for a 24 MB document — and found that
`SublimatedData::Commands`, the pre-parsed form of every content stream, is **596 MB** of
it. [ADR-0077](0077-a-command-was-seventy-two-bytes-because-of-one-rare-variant.md) took
8 bytes off each command and left the larger question open, calling it a decision.

The obvious move is the one that worked twice on this branch: build it when something asks
for it, as `PdfArena::object_index` now does
([ADR-0075](0075-two-costs-a-caller-never-asked-for.md)). Two things appear to support it.
The interpreter already falls back for any stream that is not pre-parsed —

```rust
SublimatedData::Image { .. } | Compressed { .. } | Raw(_) => {
    let data = self.doc.arena().get_stream_bytes(&sublimated)?;
    self.execute_raw(&data)
}
```

— so the form is an optimisation and not a requirement, and disabling it leaves
`inspect text` byte-identical by MD5. And a GUI drawing one page of a 5,057-page document
has no use for the other 5,056.

**It is still wrong, and the reason is not about performance.**

## What normalisation-at-load guarantees

`ARCHITECTURE.md` §4.4 names three things true of every `Document` the moment `open`
returns: the revision chain is merged, the ciphertext is gone, and the metadata has one
answer. "Nothing later can put those back"
([ADR-0013](0013-a-document-is-one-normalised-state.md)). Pass 2 — semantic sublimation —
is a stage of that pipeline, not a cache in front of it.

And sublimation does not only produce commands. It **records `Decision`s**:

```rust
let commands = sublimator.sublimate(content);
issues.append(&mut sublimator.take_decisions());
```

Four kinds, and none of them is cosmetic:

| clause | what it says |
| --- | --- |
| 8.4.3.3, 8.4.3.4, 9.3.6 | `operator {op} was given {val}, which {table} does not define` — **Violation**, for `J`, `j` and `Tr` against Tables 53, 54 and 106 |
| 7.8.2 | `content stream stopped lexing {n} bytes in, of {total}` — **Violation** |
| 7.8.2 | `unknown content operator {op}` — **Ambiguity** |
| 9.6.2 | `the content stream selects /{name}, which its resources do not define` — **Repaired** |

Make the parse lazy and those decisions are not in the log when `open` returns.
`inspect info` would report a document as read without departure, and a violation would
appear later, when something happened to draw a page. That is worse than the defect fixed
earlier on this branch, where `inspect structure` and `inspect info` gave different answers
because they had read different amounts: there the scope differed and the fix was to name
it. Here **the same scope would give different answers at different times**.

## The precedent is already recorded as a defect

§4.4 continues:

> **Fonts are the exception, and it is a live one.** … A document's font state therefore
> settles at first draw, not at load, and the resource that decodes text is not the one
> whose decisions the reading log carries.

Measured in [ADR-0045](0045-normalisation-at-load-does-not-reach-fonts.md), owed by
ROADMAP Phase T, and it cost
[ADR-0041](0041-a-character-collection-is-declared-not-guessed.md) an attempt that fixed
the load-time copy and moved nothing, because the copy that mattered was rebuilt at draw
time.

Making `Commands` lazy would produce a second instance of that exact shape, in the same
words. The repository already owes a debt for the first one.

## Decision

`Commands` stays eager. The 596 MB is the price of a decision log that is complete when
`open` returns.

The middle option — parse per page, keep the result — has the same defect: the decisions
still arrive at draw time. The only shape that preserves the guarantee is "parse for the
decisions at load, keep the commands lazily", which parses twice: slower than today and no
smaller once a page is drawn.

## Consequences

- **Recorded because it was nearly done.** The measurement was written up as a trade —
  596 MB against 12% of a text extraction (`inspect text samples/fy05.pdf`, 1.38 s eager
  against 1.57 s lazy, because pre-parsing runs inside the parallel refinery and on-demand
  parsing does not) — and presented as a decision awaiting an owner. It is not a trade.
  The question "does this conflict with normalisation-at-load" was asked from outside and
  the answer was in `ARCHITECTURE.md` §4.4 the whole time.
- **A measurement is not an argument.** Three changes on this branch came from profiling
  and each was right because the cost bought nothing. This one buys the guarantee the
  architecture is built on, and no profile says so. Where a number and a recorded
  invariant disagree, the invariant is the one that was written down on purpose.
- **The 596 MB stays on the books.** It is the largest single item in a document's
  footprint, and anything that reduces it has to keep the decision log complete at `open`.
  A representation that is cheaper per command would qualify; deferring the parse does not.
- **`ARCHITECTURE.md` §4.4 now has a second dependant.** The fonts exception was the only
  worked example of what the guarantee costs when it is broken; this is the record of it
  holding.

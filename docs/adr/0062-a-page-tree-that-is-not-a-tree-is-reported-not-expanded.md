# ADR-0062: A page tree that is not a tree is reported, not expanded

- **Status**: Accepted
- **Date**: 2026-09-05
- **Commit**: (see the commit that adds this file)

## Context

Found while establishing why `PdfWriter::collect_pages_recursive` could not be reached
from a file ([ADR-0061](0061-four-walks-bounded-and-two-that-were-not-what-the-sweep-said.md)).
The answer was that `Document::open` gets there first, and what it does is worse than
crashing.

Four objects, one of them a page:

```
2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>
3 0 obj << /Type /Page  /Parent 2 0 R /MediaBox [0 0 612 792] >>
4 0 obj << /Type /Pages /Parent 2 0 R /Kids [2 0 R] /Count 1 >>   % back to 2
```

```
$ fepdf inspect info
Pages:      16
--- [ DECISIONS TAKEN READING (5.3) ] ---
  none — the file was read without departing from the standard
```

`walk_pages_recursive` had a depth limit of 32 and no memory. One page is pushed every two
levels of the loop, so the limit produced exactly sixteen. Saving the document wrote
`/Kids [6 0 R 6 0 R … x16] /Count 16` — the invention became the file.

**The depth limit is what made this silent rather than fatal.** It is the reason nothing
crashed, and it is not a guard against this at all: no number stops a walk inventing
pages, only remembering does.

**Two more things were wrong in the same twenty lines.**

The kid loop read `let _ = self.walk_pages_recursive(h, out, depth + 1);`, so the depth
limit's error went nowhere. `find_all_pages`'s own doc comment says **"A page tree that
will not walk is recorded, not swallowed. Both failures here were `if let Ok(..)` and
`let _ =`"** — a previous fix removed those at the top level and this one, one scope
deeper, outlived the sentence describing its removal.

RR-15 Rule 13 forbids silent error swallowing and is checked by a grep for
`filter_map(Result::ok)`. `let _ =` on a `Result` is the same defect and is not checked.

## Decision

**`walk_pages_recursive` carries a visited set as well as its depth**, and the two are
kept because they answer different questions:

| | Guards against | Why the other one cannot |
| :--- | :--- | :--- |
| `seen` | A node reached twice — 7.7.3.2 makes the page tree a tree, every node with one `/Parent`, so this is non-conforming and expanding it invents pages | A depth limit turns an infinite walk into a wrong number |
| `depth > 32` | A tree that is deep and *legitimate* | A visited set does not bound stack use; each level is a separate object, so the parser's nesting limit does not reach across them |

This is the case ADR-0060's table did not cover: there, a tree got a number and a chain
got a set. A page tree gets both, because the malformation and the stack are two problems.

**Reaching a node twice is recorded where it is detected and returns `Ok`**, not an error.
It is not a failure of the subtree — it is a node already counted — and routing it through
the kid loop's error path described it as "the page tree does not walk below object 2",
which says the wrong thing about it.

**The kid loop records instead of discarding.** A failing branch still does not take the
rest of the document with it; it is now visible that it happened.

## Consequences

- The fixture above reports **1 page** and one `[VIOLATION] ISO 7.7.3.2`. A file whose
  page tree has no `/Page` at all reports 0 pages and the same violation, where before it
  reported 0 and said the file was conforming.
- **Measured against both corpora, before and after: 524 files, 18,277 pages, no file
  changed by a single page.** The new violation fires on **0 of 524**. That is what a
  conformance check on a corpus of mostly-valid files should read, and per this project's
  standing rule it measures the corpus rather than the world.
- **Verified by removing the visited set**: the loop case returns to `left: 16, right: 1`
  and the duplicate-kid case to `left: 2, right: 1`. Three tests, one of which — a nested
  three-page tree that records nothing — exists so that a `seen` which pruned too eagerly
  cannot pass by finding nothing.
- **`collect_pages_recursive`'s bound in ADR-0061 stops depending on this defect.** It was
  bounded on the reasoning that relying on the expansion was relying on a defect; the
  defect is now gone and the bound is still right.
- **Two commands print a block headed `DECISIONS TAKEN READING (5.3)` and they are not the
  same set.** `inspect info` shows the `Document`'s log, which is where this lands;
  `inspect structure` shows `FileStructure`'s, which is a separate survey of the bytes.
  Nothing says so at either site. Not changed here, and worth knowing before quoting
  either as "what the engine decided about this file".
- **`let _ =` on a `Result` is Rule 13's defect and Rule 13 does not look for it.** This
  one was found by reading, twice, after a doc comment had already claimed it was gone.

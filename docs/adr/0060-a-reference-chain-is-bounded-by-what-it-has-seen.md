# ADR-0060: A reference chain is bounded by what it has already seen, not by a number

- **Status**: Accepted; open item closed by [ADR-0061](0061-four-walks-bounded-and-two-that-were-not-what-the-sweep-said.md)
- **Date**: 2026-09-05
- **Commit**: (see the commit that adds this file)

## Context

`inspect catalog`'s last column describes each catalogue value — "enough to tell a
dictionary from a name from an array". For an indirect reference it followed the
reference and described what it found, by calling itself.

It had no bound of any kind. Four objects are enough:

```
1 0 obj << /Type /Catalog /Pages 2 0 R /Loop 4 0 R >> endobj
...
4 0 obj 4 0 R endobj
```

```
$ fepdf inspect catalog target/selfref.pdf
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

**RR-15 Rule 6 forbids unbounded recursion, and its enforcement column says "Code
review".** Twelve other walks in `fepdf-model` do carry a bound, so this was a hole in a
practice rather than an absent practice — which is the harder kind to see, because
reading almost any neighbouring function suggests the rule is being kept.

`Object::resolve` follows the same chains and answers `Null` past depth 64. So the engine
already had a value it would not crash on and a report that crashed on it.

## Decision

**`describe` is a loop that remembers the handles it has followed**, and stops when one
repeats:

```
Loop   untyped   —   no   4 0 R -> 4 0 R -> (already in this chain)
```

A visited set rather than a depth limit, which is the opposite of what the other twelve
walks do, because they are walking a different shape:

| | Guard | Why |
| :--- | :--- | :--- |
| A tree — `/Kids`, `/K`, `/Next` | Depth | A node legitimately appears under two parents, so a visited set prunes what it should keep. A number is the right approximation. |
| A reference chain | Visited set | Each step has exactly one successor, so a chain that repeats no handle must end: the arena is finite. The set is **exact** where a number is a guess, and it lets the phrase say why it stopped instead of printing sixty-four identical hops into a table cell. |

**No bound here is taken from the corpus.** The deepest chain any catalogue value carries
is **one hop, in all 524 files of both corpora** — 9 samples and 515 external, measured
2026-09-05. A limit derived from that would have been 1 and would have refused a
conforming file: 7.3.10 lets an indirect object hold an indirect reference, however few
do. The measurement is recorded because it is the argument for *not* using it, and
because it says what the two-hop test below is protecting.

## Consequences

- **Verified by putting the recursion back**, and the failure is worth recording: a stack
  overflow **aborts the test binary** with `SIGABRT`. The defect does not appear as a red
  test — the run stops, and the tests after it in the same binary never report. A bound is
  what turns this class of defect into an assertion at all, which is a reason to prefer
  finding them by construction over waiting for a suite to go red.
- **Two tests, and the second one is the point.** One asserts the whole phrase for a
  self-referencing file; the other asserts that a conforming two-hop chain is still
  followed to its end. Without the second, tightening the bound to 1 — which the corpus
  would have suggested — passes.
- **Six other unbounded walks remain**, found by the same sweep and not fixed here:
  `struct_tree::delete_struct_node`, `apply::annotations::update_form_field_value_in_dict`,
  `apply::metadata::build_outline_level`, `Object::cmp`,
  `PdfWriter::collect_pages_recursive`, and `ObjectCloner::walk`. Each is reachable from a
  frontend; none is known to crash, and `collect_pages_recursive` is protected only by
  normalisation happening first, which nothing states. They are an open item and the table
  above is the rule they should be read against.
- **Rule 6 still names no tool.** This record does not change that, and a rule whose
  enforcement column says "Code review" is one that a four-object file can find first.

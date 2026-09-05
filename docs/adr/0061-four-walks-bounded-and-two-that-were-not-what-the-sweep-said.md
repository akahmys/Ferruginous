# ADR-0061: Four more walks are bounded, and two of the six were not what the sweep said

- **Status**: Accepted; closes the open item in [ADR-0060](0060-a-reference-chain-is-bounded-by-what-it-has-seen.md)
- **Date**: 2026-09-05
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0060](0060-a-reference-chain-is-bounded-by-what-it-has-seen.md) bounded
`catalog::describe` after a four-object file aborted the process, and left six walks named
and unfixed. Each was reached from the side a caller reaches it from — an `Operation`, or
the writer — before anything was changed.

**Two of the six were not defects, and finding that out took measuring rather than
reading.**

| Named in ADR-0060 | Measured |
| :--- | :--- |
| `Object::cmp` | **Not recursive at all.** Every arm compares *handles* — `Array(a).cmp(b)` orders the two array handles, not their contents — so it is O(1). The sweep's detector had matched the method name `cmp` inside `cmp`. |
| `apply::metadata::build_outline_level` | Recursion was real; **the crash was not in it**. Ten thousand levels does abort, in `OutlineNode`'s derived `Drop` releasing the value. The walk itself is never reached that deep: `serde_json` refuses an `OutlineNode` past **62** levels, so `fepdf-mcp` — the only caller that deserialises one — cannot get there, and `Drop` overflows between **5,000** and **10,000**. The recursion sat between the two numbers. |

Two crashed exactly as claimed, both through the operation vocabulary, both on a file a
frontend would accept:

```
Operation::DeleteStructElem   + a /K that names an ancestor   -> SIGABRT
Operation::SetFormFieldValue  + a /Kids that names an ancestor -> SIGABRT
```

Both need a target that is *not* present, so the search has to exhaust the tree rather
than stop at the first match — which is the ordinary case for "this field is not in this
document".

**The sixth, `PdfWriter::collect_pages_recursive`, is reachable from no file, and the
reason is a second defect.** `Document::open` meets a cyclic page tree first and *expands*
it: a two-node loop around a single page is read as **16 pages**, written back out as
`/Kids [6 0 R ×16]`, and `DECISIONS TAKEN READING` says *"none — the file was read without
departing from the standard"*. The reader's depth bound stops the crash and produces a
wrong answer silently.

## Decision

**Four walks take a depth of 64** — `delete_struct_node`, `update_form_field_value_in_dict`,
`build_outline_level` and `collect_pages_recursive` — the number the field-tree and
`/Next` walks already use, and a depth rather than a visited set on ADR-0060's grounds:
these are trees, and a set prunes what a tree may legitimately present twice.

`build_outline_level` **refuses** past the bound where the other three stop quietly. An
outline silently missing the levels a caller asked for is worse than one the caller is
told was not built; a page or a field that is not found is already an answer.

**`ObjectCloner::walk` gets no bound, and the invariant that makes it safe is written down
instead.** It queues references rather than following them, so only *direct* nesting
recurses, and nothing can put deep direct nesting in the arena: `Parser` refuses past 512
levels and records a `[VIOLATION] ISO 7.5.4`. A bound inside `walk` would be a second
guard for a job the parser does, and would have to answer what a truncated clone is. Two
tests hold the invariant from both sides — a page nested 500 deep clones, and one nested
600 deep never reaches the arena.

## Consequences

- **Each bound was verified by removing it.** Three abort with `SIGABRT`
  (`delete_struct_node`, `update_form_field_value_in_dict`, `collect_pages_recursive`);
  `build_outline_level` goes red, because 200 levels is past the bound and nowhere near
  the stack. Each has a companion test that a too-tight bound fails: 60 outline levels
  build, a page 62 deep is still collected, 500 levels of nesting still clone.
- **`collect_pages_recursive` is bounded although nothing can reach it**, because what
  stops it being reached is the page-tree expansion above. Depending on that is depending
  on a defect.
- **`writer.rs` had no unit tests at all** before this — 3,000 lines reached only through
  integration tests. The two added here are the first, and they exist because the walk
  they cover cannot be driven from a file.
- **`OutlineNode` still aborts on `Drop` past ~5,000 levels**, for any caller that builds
  one in Rust. No bound in this crate moves that; a manual `Drop` on the type in
  `fepdf-model` would, and that is a change to a public type rather than to a walk.
- **The page-tree expansion is not fixed here.** Reporting 16 pages for a document with
  one, and recording nothing, is a Rule 20 matter and a change to what `page_count`
  answers — it needs its own decision, not a rider on this one.
- **Rule 6 still names no tool.** Two ADRs have now been written about walks it did not
  catch. The detector used for both sweeps is a throwaway script, and its two false
  positives here are the argument for either building the check or saying in `CODING.md`
  that the rule is held by review and nothing else.

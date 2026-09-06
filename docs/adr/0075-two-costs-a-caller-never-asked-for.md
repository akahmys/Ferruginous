# ADR-0075: Two costs a caller never asked for

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0074](0074-the-reader-copied-the-file-once-per-object.md) took `inspect info` on
`samples/intel_sdm.pdf` (24 MB, 5,057 pages) from 17.1 s to 5.5 s and said the arena was
"not exonerated, only displaced". The next profile answers it, and the answer is again
not the one the standing hypothesis named.

### The reverse index was 58% of the read, for a question nothing asked

A symbolised profile put **1,795 of the main thread's 2,548 samples — 70% — in
`PdfArena::set_object`**, all of them `_platform_memmove`. Not `get_dict` cloning on
read, which is what ADR-0071 recorded as the suspect and what 214 call sites made look
likely. The write path, maintaining `object_index`: a `BTreeMap<Object, Vec<Handle>>` from
an object's *value* to the handles holding it, updated on every `alloc_object` and every
`set_object` — which is every object in the document.

Two call sites query it: an annotation resolving its appearance stream and a page
resolving its content stream. `inspect info` queries it never.

Measured by removing the maintenance outright, the index accounted for most of what was
left: the read went from 5.50 s to 1.99 s once it was made lazy.

It is now built on the first `find_object` and kept current after that, so the cost is
paid once by whoever asks and never by anyone who does not. Building it **before** taking
its own lock matters: `alloc_object` and `set_object` lock `objects` then `object_index`,
and building inside the index's write guard would take them the other way round — a
lock-order inversion that could deadlock against either. Two threads racing to build both
succeed and the second's work is dropped.

### `OutlineNode` aborted the process on `Drop`

[ADR-0061](0061-four-walks-bounded-and-two-that-were-not-what-the-sweep-said.md) measured
this and left it open, with the fix named: "a manual `Drop` on the type in `fepdf-model`
would [move that], and that is a change to a public type rather than to a walk." The
derived destructor released `children`, which released theirs, one stack frame per level;
the abort sits between 5,000 and 10,000 levels, while the walk that *builds* an outline is
bounded well above it. No bound on any walk reaches it — it is the type's own destructor,
and every caller that builds one in Rust has it.

The manual `Drop` moves each node's children out before that node dies, so the node the
compiler drops always has an empty `Vec`. Nothing destructures `OutlineNode`; every use in
the workspace constructs one, so the restriction a `Drop` impl adds costs nothing here.

## Decision

Build `object_index` lazily. Give `OutlineNode` an iterative `Drop`.

## Consequences

| `inspect info` | before both | after ADR-0074 | now |
| --- | ---: | ---: | ---: |
| `samples/intel_sdm.pdf` | 17.05 s | 5.50 s | **1.99 s** |
| `samples/fy05.pdf` | 1.86 s | 1.05 s | **1.04 s** |

Four consecutive runs of each build, the first discarded, nothing else running; each
commit checked out and rebuilt to measure it. **The figures first published in this record
and in ADR-0074 were wrong** — 33.73 s / 6.17 s / 2.67 s — because the baseline was taken
while the audit and the test suite were running, and every figure was a single first run
after a rebuild, which costs about 0.5 s of cold start here. The improvement is real and
is 8.6×, not the 12.6× those numbers implied.

- **Output is unchanged, compared by MD5 over all nine samples** against the values
  recorded before either change.
- **Five tests pin `find_object`'s answers**, because a lazily built index is exactly the
  shape that answers correctly once and then goes stale: a write after the first query is
  seen, an allocation after it is seen, the lowest handle still wins, and a value nothing
  holds is still absent.
- **`a_deep_outline_is_released_without_recursing` fails by aborting the test binary**,
  not by reddening a line. That is what the defect does, and the file says so.
- **What is left is not the parallel section.** Of 1,504 samples, 709 are the main thread
  in `_pthread_cond_wait` under `ParallelRefinery::refine_all` — but sampling the worker
  threads finds every one of them in `_pthread_cond_wait` too, idle. Splitting the command
  by phase puts `inspect structure` (the reader alone) at 0.58 s against `inspect info`'s
  1.99 s, so the remaining second and a half is in ingestion ahead of the parallel work,
  not in the work itself.
- **Measure with nothing else running.** This record had to be corrected once for
  publishing a baseline taken under load; the method is now stated with the numbers so the
  next comparison is made the same way.
- **The standing hypothesis was wrong twice.** ADR-0071 recorded "the arena clones on
  every read" with `get_dict`'s 214 call sites as the evidence; the cost was first the
  reader copying the file, then the arena's *write* path maintaining an index. Both times
  the count of call sites pointed one way and the profile another. `get_dict` cloning may
  yet cost something — it is now simply not what any measurement has shown.

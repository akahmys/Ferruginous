# ADR-0076: What the arena compresses, and what that was costing

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0075](0075-two-costs-a-caller-never-asked-for.md) left `inspect info` on
`samples/intel_sdm.pdf` at 1.99 s and said what remained sat in ingestion ahead of the
parallel work. Profiling ingestion finds **265 of the main thread's 946 samples — 28% —
in `miniz_oxide`'s `compress_inner`**, under `refine::commit_to_arena`.

Reading a document was spending a quarter of its time compressing.

That is deliberate, and `flate::deflate`'s own doc comment says so: `SublimatedData::
Compressed` never reaches a writer, so this is a memory trade, not a `/FlateDecode`
stream. `commit_stream_to_arena` deflates every non-image, non-font stream over 4 KB while
the document is read, and `PdfArena::get_stream_bytes` expands it again on the way out.

**The first measurement of the benefit was worthless and had to be redone.** Peak RSS over
`inspect info` moves by about 6 MB of 1,956 MB with the compression on or off — noise —
because peak is dominated by transient allocation during refinement and the process exits
before sustained footprint means anything. A probe that counts the bytes directly
(`examples/sublimation_probe.rs`) says what it actually holds back:

| document | streams compressed | before | after | held back |
| --- | ---: | ---: | ---: | ---: |
| `intel_sdm.pdf` | 3,154 | 23,742 KB | 5,178 KB | **18,563 KB** |
| `fy05.pdf` | 311 | 15,554 KB | 4,855 KB | **10,698 KB** |
| `unicode_16.pdf` | 2 | 490 KB | 99 KB | 390 KB |
| `bokutokitan.pdf` | 6 | 1,035 KB | 836 KB | 199 KB |
| `volvo_xc90.pdf` | 0 | — | — | 0 |

So the trade is real and worth keeping: 18 MB of a 24 MB document's streams is not
nothing. What was not measured is the *level*.

## Decision

`flate2::Compression::fast()` — level 1 — instead of `default()`, which is level 6.
Measured on `intel_sdm.pdf`, steady state:

| level | `inspect info` | held back |
| --- | ---: | ---: |
| 6 (`default`) | 1.97 s | 18,563 KB |
| 3 | 1.76 s | 18,256 KB |
| **1 (`fast`)** | **1.67 s** | **17,916 KB** |
| not compressing at all | 1.62 s | 0 |

Level 1 keeps **96.5% of what level 6 holds back for 14% of what it costs**. The last row
is what deleting the trade would buy — 0.05 s — which is the argument for keeping it as
much as the first row was the argument against paying six-level compression on every open.

## Consequences

| `inspect info` | before | after |
| --- | ---: | ---: |
| `samples/intel_sdm.pdf` | 1.97 s | **1.69 s** |
| `samples/fy05.pdf` | 1.05 s | **0.94 s** |

Measured as ADR-0074's correction requires: four consecutive runs, the first discarded,
nothing else running.

- **`deflate` and `inflate` had no round-trip test.** For the one place a document's bytes
  are transformed and restored in memory, that is the test that has to exist, and five now
  do: a content stream, an empty buffer, all 256 byte values, incompressible noise that
  grows rather than shrinks, and a buffer past the 4 KB threshold the arena uses. Verified
  by breaking both halves — a `deflate` that returns its input and an `inflate` that drops
  a byte each fail three of the five.
- **`examples/sublimation_probe.rs` is kept**, because the number it produces is the one
  that justifies the trade, and the next person to question the compression should be able
  to re-measure rather than re-derive.
- **Peak RSS is not the measurement for a memory trade in a short-lived process.** Noting
  it because the first attempt here reached for `/usr/bin/time -l`, got noise, and would
  have concluded the compression bought nothing.
- **Reading a 24 MB document holds 1,346 MB live.** Measured with a counting global
  allocator in a throwaway crate outside the workspace, since `unsafe_code = "forbid"`
  here and a `GlobalAlloc` needs it: 1,346 MB live while the document is held, 1,646 MB
  peak, 8,635 MB allocated in total over the read, and 0 MB live after the document is
  dropped — no leak, just a large resident document.

  **Peak RSS could not have told us this and nearly told us the opposite.** Dropping the
  whole document returns 30 MB of 1,935 MB to the OS, which reads like "the memory was
  never live" and is instead the allocator keeping its pages. That misreading was made
  here before the allocator was counted.

  Two things dominate, both measured by disabling them: the pre-parsed
  `SublimatedData::Commands` form costs **596 MB** (1,346 → 750 MB without it), and the
  arena's 341,424 dictionaries cost **169 MB** — a `BTreeMap` holding two entries occupies
  519 bytes, because it allocates a node sized for eleven. Neither is addressed here.
  Dropping the command form trades page-display latency for memory and changing the
  dictionary representation moves the arena's core API; both are decisions, not cleanups.

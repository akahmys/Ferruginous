# ADR-0077: A command was seventy-two bytes because of one rare variant

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

[ADR-0076](0076-what-the-arena-compresses-and-what-that-was-costing.md) measured what a
read holds: 1,346 MB live for a 24 MB document, with the pre-parsed
`SublimatedData::Commands` form costing 596 MB of it and the arena's dictionaries 169 MB.
It named both as decisions rather than cleanups. This is the third thing, which is
neither.

`Command` was **72 bytes**, and `samples/intel_sdm.pdf` holds 2,272,769 of them.

Sizing the parts says why:

| | bytes |
| --- | ---: |
| `StrokeStyle` | 56 |
| `kurbo::Affine` | 48 |
| `Color` | 40 |
| `kurbo::Rect` | 32 |
| `kurbo::Point` | 16 |

`Stroke(StrokeStyle)` and `FillStroke(WindingRule, StrokeStyle)` set the size of the whole
enum, and every other variant paid for them. How often they occur, counted over the
corpus:

| document | `Stroke` |
| --- | ---: |
| `intel_sdm.pdf` | below 0.1% — outside the twelve most common |
| `unicode_16.pdf` | 1.1% |
| `fy05.pdf` | 2.4% |
| `volvo_xc90.pdf` | none |

## Decision

`Box` the `StrokeStyle` in both variants. One allocation on a variant that is at most
2.4% of a page's commands, against 8 bytes off every one of them.

## Consequences

| | before | after |
| --- | ---: | ---: |
| `size_of::<Command>()` | 72 B | **64 B** |
| live holding `intel_sdm.pdf` | 1,346 MB | **1,321 MB** |
| live holding `fy05.pdf` | 257 MB | **250 MB** |
| `inspect text samples/fy05.pdf` | 1.45–1.47 s | **1.41–1.42 s** |
| `inspect text samples/intel_sdm.pdf` | 8.56–8.81 s | 8.61–8.85 s |

- **It is not slower, and on `fy05.pdf` it is slightly faster.** A single timing pair
  suggested a 4% cost and that reading was wrong; five consecutive runs of each build,
  A/B on the same machine with nothing else running, say the opposite. Smaller commands
  fit more of a page in cache, and the allocation lands on 2% of them.
- **`inspect text` output is byte-identical**, compared by MD5 on `fy05.pdf`.
- **The next 8 bytes are not worth taking.** What holds `Command` at 64 is a 56-byte
  payload that looks like `Type3SetMetrics`, whose `Option<kurbo::Rect>` is 40 of it.
  Boxing that would give 56 bytes and another 18 MB — 1.4% of 1,321 MB — and byte-chasing
  past the variant that dominated is how a change stops being worth reading.
- **The two larger costs stay open, and both are decisions.** Making `Commands` lazy would
  save 596 MB and cost 12% of a text extraction: measured on `fy05.pdf`, `inspect text`
  goes from 1.38 s to 1.57 s without the eager pass, because pre-parsing happens inside
  the parallel refinery and on-demand parsing does not. The interpreter already falls back
  to `execute_raw` for any stream that is not pre-parsed, so the form is an optimisation
  and not a requirement — which is what makes it a trade rather than a defect. A GUI
  drawing one page at a time would win; a CLI extracting every page would lose.

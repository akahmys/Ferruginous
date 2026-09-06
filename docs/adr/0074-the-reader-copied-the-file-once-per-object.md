# ADR-0074: The reader copied the rest of the file once per object

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

The last open item from [ADR-0071](0071-three-declarations-that-read-nothing-and-one-that-wrote-nothing.md)
was efficiency, recorded as a hypothesis: the arena clones on every read — `get_dict` at
214 call sites returns a whole `BTreeMap` by value, `get_array` at 125 returns a `Vec` —
and the note said the time cost was **unmeasured, so profile before touching anything**.

Profiling says the hypothesis was wrong.

`fepdf inspect info samples/intel_sdm.pdf` (24 MB, 5,057 pages) took **33.7 s**. A
sampling profile of the main thread:

```
9,988 / 10,207   handle_info
  9,047            Document::open → reader::load_document
    3,705            populate_arena → place_direct
    3,455            parse_indirect_at   ← all of it _platform_memmove
```

Two lines, both in `parse_indirect_at`:

```rust
let mut lexer  = Lexer::new(Bytes::copy_from_slice(&bytes[offset..]));
let mut parser = Parser::new(Bytes::copy_from_slice(&bytes[body_at..]), arena);
```

`parse_indirect_at` runs **once per indirect object**, and each run copied the whole
remainder of the file — twice. `intel_sdm.pdf` has 332,386 entries in its first
cross-reference section alone, so the reader was moving bytes on the order of the file
size times the object count to read a few tokens at a known offset.

`Bytes` exists precisely to make that unnecessary: `slice` is a refcount bump and two
pointers. What stopped it being used is that `parse_indirect_at` took `&[u8]`, from which
`copy_from_slice` is the only route to a `Bytes`.

**The release profile's `strip = true` is why this took two attempts.** The first profile
resolved no names at all and showed only `_platform_memmove` under unnamed frames — enough
to suspect copying, not enough to name the copier, and an earlier version of this record
would have blamed the arena on that evidence.

## Decision

Thread `&Bytes` from `load_document` down to `parse_indirect_at` and slice instead of
copying. The five public entry points that receive a `&[u8]` — `CatalogReport::survey`,
`InteractiveReport::survey`, `SignatureReport::survey`, `FileStructure::survey` and the
encryption probe — copy the file **once**, not once per object. `Document::open` already
holds a `Bytes` and now passes it through untouched.

## Consequences

| | before | after |
| --- | ---: | ---: |
| `inspect info samples/intel_sdm.pdf` (5,057 pages) | 33.73 s | **6.17 s** |
| `inspect info samples/fy05.pdf` (846 pages) | 2.71 s | **1.05 s** |
| `inspect info samples/constitution.pdf` | 0.02 s | 0.01 s |

- **The output is unchanged, checked rather than assumed.** `inspect info` over all nine
  samples produces byte-identical output before and after, compared by MD5 against a
  binary built from the stashed tree.
- **`FileStructure::survey` shares one copy between its two readers.** It walked the
  cross-reference chain and then called `load_document`; both now take the same `Bytes`.
- **The arena's clone-on-read is not exonerated, only displaced.** It was 3,705 samples
  under `populate_arena` that this change does not touch, and the next profile after this
  one is where that question gets answered. What is settled is that it was not the
  headline: a hypothesis held for a whole session was worth eleven seconds of profiling.
- **Three copies of the same shape remain and are left alone**: the stream body at
  `reader.rs:136` is bounded by `/Length` rather than by the file's end, the trailer parse
  runs once per section, and the object-stream parse works on a decoded buffer that has no
  shared owner to slice.

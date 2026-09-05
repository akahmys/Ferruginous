# ADR-0063: One set of accessors, because two of them disagreed about one dictionary

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

`decrypt.rs` unlocks a document and `encryption.rs` reports on what protects it. Both read
the same `/Encrypt` dictionary, each through its own private helpers — **twelve functions
spelling six different behaviours** for four value types. The one that mattered:
`integer` followed one level of indirection in `decrypt.rs` and none in `encryption.rs`.

7.3.10 lets any object in a file be written as an indirect reference, so `/V 4 0 R` and
`/V 4` say the same thing. Measured 2026-09-06 against a real RC4-128 document with its
entries written as references — a document this engine decrypts perfectly, extracting its
Japanese text in the same run:

| Written as a reference | Reported | Truth |
| :--- | :--- | :--- |
| `/Filter` | `(absent)` | `/Standard` |
| `/V` | `—` | 2 |
| `/R` | `—` | 3 |
| `/Length` | `—` | 128 |
| `/EncryptMetadata false` | **`true`** | `false` |
| verdict | **`UNSUPPORTED — no handler is implemented for this /V and /R`** | implemented |
| `DECISIONS TAKEN READING` | *none — the file was read without departing from the standard* | |

**The engine reported that it could not handle a file it had just fully read**, and
recorded nothing about doing so.

`/EncryptMetadata` is the worst of them and not because it is the largest. `boolean` could
not read the value, answered `None`, and the caller reads
`boolean_at(..).unwrap_or(true)` — so the report stated **the opposite of what the
document said** about whether its metadata is encrypted. An accessor that cannot
distinguish "absent" from "I could not read it" turns every caller's default into a
misstatement.

**Text had a second divergence.** `text_in` used `String::from_utf8_lossy` where every
other text reader in the engine uses the 7.9.2.2 decoder. A `/Desc` and `/UF` written as
UTF-16BE — which is what a producer writing Japanese must do — came back as:

```
payload file      ��f�S�S0U0�0_m�N� . p d f
```

## Decision

**One module, `access`, and every accessor in it resolves.** `entry_at` is
`dict.get(key)?.resolve(arena)`, and each typed accessor is that plus a type test, so
following a chain is not a thing a reader can forget to do. `text_at` decodes with
`refine::text::recover_string`, which is 7.9.2.2 and nothing else.

**A chain, not one hop.** `Object::resolve` follows to depth 64 and answers `Null` past
it. One hop is what `decrypt.rs` did and is the shape that looks correct until a second
reference is added; the test asserts depth 0, 1, 2 and 5, and neither original passes it.

**A stream is read as the dictionary it is** (7.3.8). The three `as_dict` copies this
replaces accepted `Dictionary` only.

`entry_at` and `array_of` are private, because nothing outside the module has asked for
the untyped forms. Widen them when something does.

## Consequences

- **Every entry now reads what the file declares**, and the verdict follows from it. The
  same fixture reports `/V 2 /R 3`, 128 bits, `/Standard`, `EncryptMetadata false`, and
  `implemented`.
- **Regression, measured by comparing full `inspect encryption` output from binaries
  built either side of the change, over 542 files**: 7 reports changed, and all seven are
  the indirect-reference and UTF-16BE fixtures written for this. **0 of the 524 corpus
  files changed.** The change can only turn a `None` into a `Some` — it never returns a
  different value for a direct one — so a corpus of files with direct entries cannot move,
  and this measures that rather than assuming it.
- **The decryption path changed too, not only the report.** `/O`, `/U`, `/UE`, `/OE` and
  `/Perms` are now read through a resolving accessor, so a file that writes them as
  references unlocks where it previously could not. `/Filter` likewise, which is what
  routes Standard against `Adobe.PubSec`: an indirect `/Filter` used to send a public-key
  document down the standard handler.
- **Verified by putting each half back.** Stopping the resolution returns `(None, None)`
  for `/V` and `/R` and `true` for a `/EncryptMetadata` that says `false`; restoring
  `from_utf8_lossy` returns `Some("��f�S�S\u{16}0U0…")`.
- **Seven more `dict_of` and five more `array_of` remain in other modules** (measured
  2026-09-05), with the same three-way split of behaviour — `catalog.rs` still cannot read
  a `Ref → Ref → Dict` that `document/entries.rs` can. `access` is the destination for
  those; this moves two files, not twelve.
- **What an unsupported `/V` should record is left open.** `judge`'s wildcard arm answers
  with a verdict rather than a `Decision`, which was invisible while the arm was also
  being reached by conforming files. It no longer is, and whether the verdict is enough
  for Rule 20 is now a question on its own rather than a symptom of this one.

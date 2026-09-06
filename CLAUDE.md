# fepdf

**Read [AGENTS.md](AGENTS.md) before writing documentation.** It is 73 lines and it is the
only place that says which document answers what. This file exists because an agent worked
through this repository for a day without opening it, and broke three of its five writing
rules in the process — one of them by adding two paragraphs of dated narrative to
`ARCHITECTURE.md`, nine days after
[ADR-0039](docs/adr/0039-the-design-document-was-narrating-its-own-corrections.md) had cut
2,200 words of exactly that out of the same file.

The rules most easily broken, in the order they were broken:

- **Present tense in `ARCHITECTURE.md` and `AGENTS.md`; past tense in `docs/adr/`.** A
  design document says what the design *is*. Why it came to be that way, what it was
  before, and what a measurement showed on a given day all belong in a record under
  `docs/adr/`. `scripts/audit/documents.py` fails the audit on a line carrying both a date
  and a past-tense verb in either document.
- **A quoted figure carries its date, or is re-derived before quoting.** Nothing checks
  this. `TESTING.md` stood eight days claiming a test run took 1m 57s when it took 30s;
  `ARCHITECTURE.md`'s crate sizes were stale by 1,956 lines. Put the command that derives
  a number beside the number, or leave the number out.
- **One ADR, one decision.** ADR-0071 carries eight subjects and is the longest record in
  the directory by half again. Split at the second subject.

## Before you start

`./scripts/dev/status.sh` re-derives the figures these documents lean on, so a stale one
reads as a disagreement rather than as current.

`./scripts/audit/verify_compliance.sh` must end `=== AUDIT PASSED ===`. Read that line,
not the first. It is the gate, with `cargo test --workspace`.

## Measurement outranks documentation

A claim about this codebase is established by running something. That includes claims in
this file: if it disagrees with a measurement, the measurement wins and this file is
corrected. `AGENTS.md` states the full hierarchy.

# ADR-0080: A design document does not carry a number that moves

- **Status**: Accepted
- **Date**: 2026-09-07
- **Commit**: (see the commit that adds this file)

## Context

`ARCHITECTURE.md`'s crate table carried a `~Lines` column. It was re-derived on
2026-08-18, again on 2026-08-22, and would have been again on 2026-09-07: ten of its
thirteen rows had drifted in sixteen days, `fepdf-model` by 1,956 lines.

The column was never a constraint. The document said so — "nothing here should be read as
a budget", every crate having outgrown the row that described it — and nothing else in the
repository read the figures. What they were for was noticing drift, and there is one
worked example: `fepdf-mcp` read 330 while it held 1,902, and the gap was how anyone learnt
it had become the frontend constructing every operation of the day.

**That signal is only visible to whoever re-derives, and re-deriving spends it.** Between
one measurement and the next the column says what the crate *was*, which is the failure the
document warned about four paragraphs above the table while the table was doing it.

## Decision

The column goes. The command that derives it stays, and the weights that do not move are
stated in words: `fepdf-model` is the bulk of the engine by an order of magnitude,
`fepdf-gui` the largest frontend, `fepdf-wasm` and `fepdf-macros` thin enough to read in a
sitting.

## Consequences

- **The first attempt put this reasoning in `ARCHITECTURE.md` itself**, in two paragraphs
  of dated narrative — which is the same error one level up. A design document says what
  the architecture *is*; why it changed is what this directory is for. Of what was written
  there, three sentences were design and the rest is above.
- **`ARCHITECTURE.md` carries eight dates and nine past-tense constructions**, more than
  any other document in the repository (`CODING.md` 8 and 0, `TESTING.md` 6 and 3,
  `PLANNING.md` 0 and 0). Not all of it is misplaced — "since Phase A the reader lives
  here" describes the shape — but the boundary is worth naming, and this record does not
  move the rest.
- **Numbers of other kinds stay.** "Lost 167 lines when ten methods left for the
  vocabulary" records a move that happened and cannot rot; Rule 1's 50/200/500 are
  thresholds the audit enforces; "~220 lines" is a hypothetical. What went is the present
  tense.
- **`TESTING.md` is the same shape, unresolved.** Its timing table is a measurement with a
  date on it, re-derived on 2026-09-07 after standing eight days and being wrong by 4×.
  There the number *is* the subject — the file exists to say where the time goes — so it
  stays, with the command beside it.

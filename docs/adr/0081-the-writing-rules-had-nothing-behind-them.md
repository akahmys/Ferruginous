# ADR-0081: The writing rules had nothing behind them

- **Status**: Accepted
- **Date**: 2026-09-07
- **Commit**: (see the commit that adds this file)

## Context

`AGENTS.md` states five rules for writing the documentation, and its fourth says **a rule
that is not checked is a comment**. None of the five named an enforcer.

Three were broken between 2026-08-29 and 2026-09-07, and the sequence is the argument:

* **2026-08-29.** [ADR-0039](0039-the-design-document-was-narrating-its-own-corrections.md)
  measures `ARCHITECTURE.md` at 41% past-tense prose and cuts 2,200 words of dated
  self-correction out of it. It quotes one of them — *"this listing was fiction until
  2026-08-22"* — as an example of the shape being removed.
* **2026-09-07.** That sentence is still in the file, along with four more of the same
  kind. An agent works through the repository for a day without opening `AGENTS.md`, then
  adds two fresh paragraphs of dated narrative to `ARCHITECTURE.md` — while cleaning up
  stale figures, which is rule 3.
* The same day: seven of the twenty-two relative ADR links in source point nowhere, all by
  miscounting `../`. `TESTING.md` has stood eight days at 4× wrong about its own subject.
  ADR-0071 carries eight subjects where the house median is one.

The agent's account of why is worth keeping, because it decides what the fix has to be:
**it does not read documents, it greps them**, and a rule it does not know exists is a rule
it cannot grep for. `AGENTS.md` is 73 lines. Length was never the obstacle; nothing in the
working loop opened it.

## Decision

`scripts/audit/documents.py`, run by `verify_compliance.sh` as step 15, checks the three
rules that are checkable:

| | what it catches | what it caught the day it was written |
| :--- | :--- | ---: |
| tense | a line carrying both a date and a past-tense verb in `ARCHITECTURE.md` or `AGENTS.md` | 8 |
| links | a relative link in any document or source doc comment that does not resolve | 7 |
| ADR index | duplicate or missing numbers, index and directory disagreeing, a record with no Status or Date | 0 |

A `CLAUDE.md` is added, which the agent's tooling loads without being asked. It carries the
three rules broken here and points at `AGENTS.md` for the rest; it does not restate the
hierarchy, because rule 1 forbids the second copy.

## Consequences

- **The check caught its own author twice.** The paragraph introducing it into `AGENTS.md`
  was written in the past tense and failed the tense rule; the sentence citing this record
  failed the link rule before the record existed. Both are in the commit that adds them.
- **Rules 3 and 5 stay unchecked and the reason is in the code.** Whether a quoted figure
  is current is not a property of its text — `TESTING.md`'s "1m 57s" was well-formed and
  four times wrong. What rule 3 asks for is a *derivation beside the number*, which is a
  habit. Rule 5 — prove a check fires by breaking the thing it checks — is a procedure, and
  a procedure cannot be grepped for.
- **Reading was the weaker of the two fixes and is the one that would have been chosen.**
  `CLAUDE.md` raises the chance the rules are read; the audit stops the commit whether they
  were or not. This is the third time this shape has been cleaned up, so the first alone
  would have bought a fourth.
- **Two documents lost content to this.** `ARCHITECTURE.md` goes from 5,532 words to 5,129
  with five dated self-corrections moved into records, and its crate line counts are gone
  entirely ([ADR-0080](0080-a-design-document-does-not-carry-a-number-that-moves.md)).

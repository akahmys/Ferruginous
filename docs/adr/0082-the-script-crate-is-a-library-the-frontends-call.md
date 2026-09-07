# ADR-0082: `fepdf-script` is a library the frontends call, not a fifth frontend

- **Status**: Accepted. Amends [ADR-0025](0025-a-script-processor-is-a-frontend-not-a-subsystem.md) a fourth time, after [ADR-0032](0032-running-scripts-is-a-frontend-verb-not-an-operation.md). The judgements in [ADR-0031](0031-a-script-frontend-cannot-be-a-facade-feature.md) and ADR-0032 are unchanged.
- **Date**: 2026-09-07
- **Commit**: (see the commit that adds this file)

## Context

ADR-0025 called the script processor a **fifth frontend**, and two later records tested
that shape against the compiler. ADR-0031 found that a facade feature is a dependency
cycle and that moving the crate below the facade would hand it `fepdf-doc` and
`fepdf-model` directly — more privilege than a frontend has, and the opposite of
ADR-0025's central argument. ADR-0032 found that `Operation::RunDocumentScripts` costs
more than it buys and settled on a function the caller calls. Both are right and neither
is reopened here.

What neither tested is the word *frontend* itself.

**The other four have an entry point. This one has none.** `fepdf-cli` and `fepdf-gui`
are binaries; `fepdf-mcp` is a library its own `main` drives; `fepdf-wasm` is a `cdylib`
whose caller is JavaScript. `fepdf-script` is a plain library with no binary and, until
today, no dependent. Measured by narrowing every `pub` in it to `pub(crate)` and asking
the compiler: `run_calculations`, `ScriptError`, `collect_scripts`, `install`, `run_one`,
`AFORM_JS` and fourteen more report `never used`. Its own crate documentation says so in
as many words — "is not on the path of any other caller".

**That classification cost four phases of a defect standing while its fix sat finished.**
`run_calculations` was reached only by this crate's own tests, so every write through
`Operation::SetFormFieldValue` went on recording the 12.6.3 `Violation` — *"wrote the
value and did not run the scripts; fields computed from it are now stale"* — that Phase R
exists to remove, while `ROADMAP.md` marked the item `[x]` resolved. Both halves existed
and nothing joined them.

**The join was the thing the classification forbade.** ADR-0032's own consequences read:
"A caller that wants scripts depends on `fepdf-script`, exactly as one that wants
rendering enables `render`." When `fepdf-mcp` became that caller, `status.sh` reported
what Rule A asks of a frontend:

```text
Rule A: frontend deps that are not the facade (expect 0)   1
```

A frontend had declared a frontend. The ADR that named the dependency and the check that
counts them disagreed, and the disagreement could not surface while the crate had no
callers.

## Decision

**`fepdf-script` is a library that sits above the facade, and the frontends call it.** It
is not one of them.

Rule A's Cargo half becomes: *a frontend declares the facade, and may declare a library
that stands above the facade.* One crate is such a library today. Everything else Rule A
asks is untouched — no frontend reaches `fepdf-doc` or `fepdf-model`, and no arena type
appears above the facade, `fepdf-script` included.

**The partition `status.sh` draws is unchanged in membership.** Above the facade there
are still five crates; four are frontends and one is a library they call. Both halves of
Rule A that read the partition — the log-site count and the arena-leak grep — go on
covering all five, because both are properties of standing above the facade rather than
of being a frontend.

**Rule A moves into the gate.** `scripts/audit/layering.py` performs both counts and
exits non-zero on either, `verify_compliance.sh` runs it, and `status.sh` reports the
numbers it returns rather than deriving them a second time.

## Consequences

**A check that names its places got the answer wrong for the fourth time this month.**
ADR-0031 counted three — `verify_compliance.sh`'s eleven crate directories, the `Decision`
row, and two in `status.sh`. `FRONTEND_CRATES` is the fourth, and it failed differently
from the others: the list was accurate about the crates that existed and wrong about what
one of them was. Deriving the list from the workspace, which fixed the previous three,
would not have caught this.

**The gate passed while a documented expectation of 0 read 1.** `status.sh` measured it
and `verify_compliance.sh` never asked. That is `CODING.md`'s "a rule that is not checked
is a comment" reached from the other side: the rule was measured, and the measurement was
not a gate.

**`fepdf-mcp` links boa.** A JSON-over-stdio server carries an ECMAScript engine so that
writing a form field runs the form's calculations. This is Rule B's shape — an
implementation's dependency tree reaching a consumer — taken deliberately, because the
alternative measured worse: the field write was silently producing documents whose
computed fields disagreed with their inputs. The same server already links a GPU stack
through `fepdf = { features = ["render"] }`, which is Rule B's own example sentence and is
not addressed here.

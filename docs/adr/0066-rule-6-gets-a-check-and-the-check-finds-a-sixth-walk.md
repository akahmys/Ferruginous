# ADR-0066: Rule 6 gets a check, and the check finds a sixth walk on its first run

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

Rule 6 forbids unbounded recursion. Its enforcement column said **Code review** from the
day it was written, and `AUDITING.md` listed it among the rules "nothing automated checks"
— which was honest, and was also the whole problem. In one week review missed five walks:

| | Found by |
| :--- | :--- |
| `catalog::describe` — a four-object file aborted `inspect catalog` | reading, after a crash |
| `struct_tree::delete_struct_node`, `apply::update_form_field_value_in_dict` | a hand sweep, then driven from an `Operation` |
| `PdfWriter::collect_pages_recursive` | the same sweep; reachable from no file only because [ADR-0062](0062-a-page-tree-that-is-not-a-tree-is-reported-not-expanded.md)'s defect got there first |
| `apply::build_outline_level` | the same sweep; bounded on Rule 6's terms, no reachable crash |

The detector used for those sweeps was a throwaway regex script, and it produced **two
false positives** — it read the `{` inside `b'{'` as a brace and called a twelve-line
function 579 lines, and it read `a.cmp(b)` inside `fn cmp` as self-recursion.
[ADR-0061](0061-four-walks-bounded-and-two-that-were-not-what-the-sweep-said.md) closed
with that as an open item: build the check, or write in `CODING.md` that there is none.

## Decision

**`scripts/audit/unbounded_recursion.py`, in `verify_compliance.sh` as step 13.** It
strips comments and literals, builds a per-file call graph, and reports cycles where no
participant carries a depth, a visited set or a worklist.

**It reports one class and counts two.** A walk over a document's own graph — an arena, a
`Handle`, a `Dict` — is reported, because a file can make one into a loop. A walk over an
owned Rust structure is counted and not reported: a `Vec<Node>` the program built cannot
be cyclic, because Rust will not allow it without an `Rc`. Eight cycles are exempt with
one guard between them: they follow *direct* nesting and not `Object::Reference`, and
`Parser` refuses past 512 levels.

**It found a sixth walk on the run that first listed anything.** `check_font_embedding`
follows `/DescendantFonts` with neither a depth nor a set, and a Type0 font naming itself
as its own descendant aborted `fepdf inspect audit` on **five objects**. None of the hand
sweeps had it: the throwaway detector looked for `/Kids`, `/K` and `/Next` in a body, and
this walk names none of them. It now carries the same 64 the other tree walks use.

## Consequences

- **Verified in both directions by breaking it**: removing the new bound makes the script
  exit 1 naming `check_font_embedding_at`, and an exemption pointing at a function that is
  no longer recursive exits 1 as a stale exemption.
- **The first version reported eighty cycles and was useless.** It accepted any
  capitalised path as a receiver, so `Vec::new()` inside `fn new` read as recursion.
  Narrowing to `self.`, `Self::` and bare calls took it to thirty-nine; separating a
  method that calls a free function of the same name — `impl CcittFilter { fn decode(&self)
  { decode(..) } }` — took it to the eight that are real. **Each narrowing had to be
  checked for what it lost**: requiring a bare call for non-methods lost
  `Self::build_virtual_balanced_view`, and a false negative in a rule about stack safety
  is worse than noise, so non-methods accept both forms.
- **What it cannot see** is written into the script and into `CODING.md`: recursion
  through a trait object or a closure, and mutual recursion split across two files. The
  exemption list is where those go when they exist; none do today.
- **`AUDITING.md`'s derivation command was reading comments.** It said to derive the step
  list with `grep -oE '\[Rule [0-9]+\]'`, which reported a `[Rule 17]` that is a comment
  recording what the clippy step used to be called. Deriving from `^echo "\[…\]` gives the
  sixteen steps that exist. A derivation that reads comments is not a derivation, and this
  one had been the documented way to answer "what does the audit check".
- **Three of the six were reachable from a frontend and aborted the process.** A stack
  overflow aborts the test binary rather than reddening one test, so none of them could
  have been caught by adding a case to a suite; the check is what makes the class visible
  before a file arrives.

#!/usr/bin/env python3
"""Rule A, the half Cargo can decide: what stands above the facade, and what it declares.

`CODING.md`'s Rule A says storage abstractions stop at the facade, and its enforcement row
says a frontend declares `fepdf` and nothing else. `status.sh` measured both and nothing
gated on either, so on 2026-09-07 a frontend gained a dependency on another crate above the
facade, the row read 1 where it expects 0, and `verify_compliance.sh` reported
`=== AUDIT PASSED ===`.

Two counts, both expecting 0:

* **Declarations.** A frontend may declare the facade and a library that stands above it
  ([ADR-0082](../../docs/adr/0082-the-script-crate-is-a-library-the-frontends-call.md)).
  Anything else is a frontend reaching around the facade, which is what `fepdf-gui` did to
  `fepdf-render` before ADR-0004's opt-in existed.
* **Arena leaks.** `PdfArena` and `Handle<T>` are how the object graph is stored and are
  not part of the caller's vocabulary. Above the facade they may not appear at all, in a
  frontend or in a library the frontends call.

The lists are here rather than in `status.sh`, which reports what this returns: written
twice, the two are free to disagree, and the one that is wrong is the one nobody re-reads.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The frontends: each is something a person or another program drives, and each has an
# entry point. `fepdf-wasm` is a `cdylib` whose caller is JavaScript.
FRONTENDS = ["fepdf-cli", "fepdf-gui", "fepdf-mcp", "fepdf-wasm"]

# Above the facade and not a frontend: a library the frontends call. It has no entry point
# of its own, which is what ADR-0082 settled after four phases of it having no caller.
ABOVE_FACADE_LIBRARIES = ["fepdf-script"]

ABOVE_FACADE = FRONTENDS + ABOVE_FACADE_LIBRARIES
FACADE = "fepdf"
ARENA = re.compile(r"PdfArena|Handle<")


def named_crates_exist() -> list[str]:
    """A name here that is not a crate silently drops that crate out of both counts."""
    return [
        f"  {name} is named above the facade and is not a crate"
        for name in ABOVE_FACADE
        if not (ROOT / "crates" / name / "src").is_dir()
    ]


def stray_declarations() -> list[str]:
    """Workspace crates a frontend declares that are neither the facade nor a library."""
    allowed = {FACADE, *ABOVE_FACADE_LIBRARIES}
    out = []
    for name in FRONTENDS:
        manifest = ROOT / "crates" / name / "Cargo.toml"
        declared = {
            m.group(0)
            for m in re.finditer(r"^fepdf[a-z-]*", manifest.read_text(), re.M)
        }
        for stray in sorted(declared - allowed - {name}):
            out.append(f"  {name} declares {stray}, which is neither the facade nor above it")
    return out


def arena_leaks() -> list[str]:
    """`PdfArena` or `Handle<T>` reaching a crate that stands above the facade."""
    out = []
    for name in ABOVE_FACADE:
        for file in sorted((ROOT / "crates" / name / "src").rglob("*.rs")):
            for number, line in enumerate(file.read_text(errors="ignore").splitlines(), 1):
                if ARENA.search(line):
                    where = file.relative_to(ROOT)
                    out.append(f"  {where}:{number} names an arena type above the facade")
    return out


def main() -> int:
    broken = named_crates_exist()
    if broken:
        print("Rule A: the list of crates above the facade is stale")
        print("\n".join(broken))
        return 1

    strays, leaks = stray_declarations(), arena_leaks()
    print(f"declarations={len(strays)} leaks={len(leaks)}")
    for line in strays + leaks:
        print(line)
    if strays or leaks:
        print("Rule A: a frontend declares the facade, and a library that stands above it")
        return 1
    print("  PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())

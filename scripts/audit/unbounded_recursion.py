#!/usr/bin/env python3
"""Recursive walks with no bound (RR-15 Rule 6).

Rule 6 forbids unbounded recursion and its enforcement column said "Code review" until
2026-09-06. Review missed five of them. Between ADR-0060 and ADR-0062 a four-object file
crashed `inspect catalog`, `Operation::DeleteStructElem` and `SetFormFieldValue` aborted
the process on a cyclic `/K` or `/Kids`, and the writer's page walk was safe only because
the reader expanded a cyclic page tree before it got there.

The throwaway detector used for those sweeps produced two false positives, and both came
from not tokenising: it counted the `{` inside `b'{'` as a brace and read `extract_name`
as 579 lines, and it read `a.cmp(b)` inside `fn cmp` as self-recursion. This one strips
comments and literals first, and distinguishes a call on `self` from a method call on
some other value.

**What it sees**: a cycle in the call graph of one file, where no participant carries a
guard — a depth compared against something, a visited set whose `insert` is tested, or an
explicit worklist.

**What it does not see**, and so what the exemptions are for: recursion through a trait
object or a closure, and mutual recursion split across files. Both are listed by hand
below when they exist.

It exits non-zero on a cycle that is neither guarded nor exempt, and on an exemption
naming a function that is no longer recursive — an exemption for a walk that has gone
reads as a check still being made.
"""

import re
import sys
from pathlib import Path

# Cycles whose guard this script cannot see, with the guard named so the claim can be
# checked by reading the line it points at.
#
# Every entry below is the same guard: `Parser` refuses nesting past 512 levels and
# records a `[VIOLATION] ISO 7.5.4`, and the arena is filled by the parser or by these
# walks copying what the parser produced. So a *directly* nested value cannot be deep and
# cannot be cyclic — a dictionary cannot contain itself, because the parser builds the
# inner one first. What these walks do not do is follow `Object::Reference`, which is the
# edge a file can make into a loop; each queues or ignores references instead.
#
# ADR-0061 states this for `ObjectCloner::walk` and the tests in
# `crates/fepdf-doc/tests/recursion_bounds_test.rs` hold it from both sides: 500 levels
# clone, 600 never reach the arena.
_PARSER_BOUNDED = (
    "direct nesting only; references are not followed, and Parser refuses past 512 "
    "levels (ADR-0061)"
)

GUARDED_ELSEWHERE: dict[tuple[str, str], str] = {
    ("crates/fepdf-doc/src/cloning.rs", "walk"): _PARSER_BOUNDED,
    ("crates/fepdf-model/src/decrypt.rs", "decrypt_value"): _PARSER_BOUNDED,
    ("crates/fepdf-model/src/writer.rs", "write_object"): _PARSER_BOUNDED,
    ("crates/fepdf-model/src/document.rs", "build_virtual_balanced_view"): (
        "each level divides the slice by max_kids, so the depth is logarithmic in the "
        "page count; at max_kids 1 every chunk is already Flat"
    ),
}

KEYWORDS = {
    "if", "while", "for", "match", "return", "fn", "let", "else", "loop", "move",
    "unsafe", "impl", "struct", "enum", "mod", "use", "as", "in", "where", "assert",
}

GUARD = re.compile(
    # a depth compared against something, or a named ceiling
    r"\bdepth\b\s*[<>=]|[<>=]\s*\bdepth\b|\bMAX_[A-Z_]*\b|"
    # a visited set whose insert decides. `self.` and `out.` sit between the `!` and the
    # name often enough that leaving them out cost two false reports in the first run.
    r"!\s*[\w.]*(?:seen|visited|reachable|handle_map)[\w]*\.insert\(|"
    r"(?:seen|visited|reachable)\w*\s*:\s*&\s*mut|"
    # an explicit worklist drained by a loop
    r"\bwhile\s+let\s+Some\(.*\)\s*=\s*[\w.]*(?:stack|queue|worklist)\w*\.pop\("
)


def strip(src: str) -> str:
    """The source with comments, strings and char literals blanked, lengths preserved.

    Blanked rather than removed so every offset still lines up with the original, which
    is what lets a brace scan and a line number come from the same string.
    """
    out = list(src)
    i, n = 0, len(src)
    while i < n:
        two = src[i : i + 2]
        if two == "//":
            j = src.find("\n", i)
            j = n if j < 0 else j
            for k in range(i, j):
                out[k] = " "
            i = j
        elif two == "/*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src[j : j + 2] == "/*":
                    depth, j = depth + 1, j + 2
                elif src[j : j + 2] == "*/":
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            for k in range(i, min(j, n)):
                if src[k] != "\n":
                    out[k] = " "
            i = j
        elif src[i] == "r" and i + 1 < n and src[i + 1] in '"#':
            m = re.match(r'r(#*)"', src[i:])
            if not m:
                i += 1
                continue
            close = '"' + m.group(1)
            j = src.find(close, i + m.end())
            j = n if j < 0 else j + len(close)
            for k in range(i, j):
                if src[k] != "\n":
                    out[k] = " "
            i = j
        elif src[i] == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            for k in range(i, min(j + 1, n)):
                if src[k] != "\n":
                    out[k] = " "
            i = j + 1
        elif src[i] == "'":
            # A char or byte literal, or a lifetime. `'a` is a lifetime; `'{'` is not.
            m = re.match(r"'(?:\\.|[^\\'])'", src[i:])
            if m:
                for k in range(i, i + m.end()):
                    out[k] = " "
                i += m.end()
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def body_of(text: str, brace: int) -> tuple[int, int]:
    """The span of the block opening at `brace`."""
    depth, j = 0, brace
    while j < len(text):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                return brace, j
        j += 1
    return brace, len(text)


def functions(text: str) -> list[tuple[str, int, int, int, bool]]:
    """Every `fn`, as (name, line, body start, body end, takes self)."""
    found = []
    for m in re.finditer(r"\bfn\s+([A-Za-z_]\w*)", text):
        brace = text.find("{", m.end())
        if brace < 0:
            continue
        semi = text.find(";", m.end())
        if 0 <= semi < brace:
            continue  # a trait method with no body
        start, end = body_of(text, brace)
        params = text[m.end() : brace]
        takes_self = re.search(r"\(\s*&?\s*(?:mut\s+)?self\b", params) is not None
        found.append((m.group(1), text.count("\n", 0, m.start()) + 1, start, end, takes_self))
    return found


def calls(body: str) -> set[str]:
    """Names this body calls in a way that could reach the function it is in.

    `self.f(`, `Self::f(` and a bare `f(` count. Two forms deliberately do not:

    * `value.f(` — a method on something else. `a.cmp(b)` inside `fn cmp` is not
      recursion, and reading it as such is one of the two false positives that made the
      throwaway detector unusable.
    * `Type::f(` — `Vec::new()` inside `fn new` is not recursion either, and accepting
      any capitalised path put eighty cycles in the first run of this script, almost all
      of them `new`, `default` and `parse`. `Self::` is the idiomatic form for the case
      that *is* recursion, so nothing real is lost; a genuine `Document::open` inside
      `Document::open` would be missed, and no such call exists.
    """
    out = set()
    for m in re.finditer(r"(?:(self|Self)\s*(?:\.|::)\s*)?\b([a-z_]\w*)\s*\(", body):
        receiver, name = m.group(1), m.group(2)
        if name in KEYWORDS:
            continue
        before = body[max(0, m.start(2) - 3) : m.start(2)]
        if receiver is None and (before.rstrip().endswith(".") or before.rstrip().endswith("::")):
            continue
        out.add(name)
    return out


# A cycle can only meet a loop that came out of a *file* if it walks one. These are the
# types that carry a document's own graph; a `Vec<Node>` the program built cannot be
# cyclic, because Rust will not let it be without an `Rc`. So a recursion over an owned
# tree is bounded by however deep that tree was built, and a recursion over an arena is
# bounded by nothing until it says so.
FILE_DRIVEN = re.compile(r"\bPdfArena\b|\bHandle<|\bDictHandle\b|&\s*Object\b|\bDict\b")


def cycles_in(path: Path) -> list[tuple[list[str], int, bool, bool]]:
    """Every recursive cycle in one file: names, line, guarded, walks a file's graph."""
    text = strip(path.read_text(encoding="utf8", errors="replace"))
    fns = functions(text)
    graph: dict[str, set[str]] = {}
    lines: dict[str, int] = {}
    bodies: dict[str, list[str]] = {}
    signatures: dict[str, str] = {}
    methods: dict[str, bool] = {}
    for name, line, start, end, takes_self in fns:
        body = text[start:end]
        # Two `fn is_cjk` in one file — a trait impl and an inherent one — are different
        # functions. Merging their bodies reads a call in one as recursion in the other,
        # so each body is kept apart and self-recursion needs one body to call itself.
        bodies.setdefault(name, []).append(body)
        graph.setdefault(name, set()).update(calls(body))
        lines.setdefault(name, line)
        head = text[max(0, start - 400) : start]
        signatures[name] = signatures.get(name, "") + head
        methods[name] = methods.get(name, False) or takes_self

    found, seen_cycles = [], set()
    for root in graph:
        stack = [(root, [root])]
        while stack:
            node, path_so_far = stack.pop()
            for nxt in graph.get(node, ()):
                if nxt not in graph:
                    continue
                if nxt == root:
                    if len(path_so_far) == 1:
                        # A method calls itself as `self.f(` or `Self::f(`. A bare `f(`
                        # inside a method reaches the *free* function of that name — which
                        # is what `impl CcittFilter { fn decode(&self, ..) { decode(..) } }`
                        # does, and reading it as recursion was a false positive.
                        pattern = (
                            r"(?:self\s*\.|Self\s*::)\s*" + re.escape(root) + r"\s*\("
                            if methods.get(root)
                            # A free function or an associated one without `self` calls
                            # itself bare *or* through `Self::`. Requiring the bare form
                            # alone lost `Self::build_virtual_balanced_view`, and a false
                            # negative in a rule about stack safety is worse than noise.
                            else r"(?:(?<![.\w:])|Self\s*::\s*)" + re.escape(root) + r"\s*\("
                        )
                        if not any(re.search(pattern, b) for b in bodies[root]):
                            continue
                    key = frozenset(path_so_far)
                    if key not in seen_cycles:
                        seen_cycles.add(key)
                        guarded = any(
                            GUARD.search(signatures[f] + b)
                            for f in path_so_far
                            for b in bodies[f]
                        )
                        walks_a_file = any(
                            FILE_DRIVEN.search(signatures[f]) for f in path_so_far
                        )
                        found.append((sorted(path_so_far), lines[root], guarded, walks_a_file))
                elif nxt not in path_so_far and len(path_so_far) < 4:
                    stack.append((nxt, path_so_far + [nxt]))
    return found


def main() -> int:
    unguarded, guarded, exempt, in_memory = [], [], [], []
    for path in sorted(Path("crates").glob("*/src/**/*.rs")):
        for names, line, is_guarded, walks_a_file in cycles_in(path):
            why = next(
                (GUARDED_ELSEWHERE[(str(path), n)] for n in names
                 if (str(path), n) in GUARDED_ELSEWHERE),
                None,
            )
            row = (path, line, names, why)
            if why:
                exempt.append(row)
            elif is_guarded:
                guarded.append(row)
            elif not walks_a_file:
                in_memory.append(row)
            else:
                unguarded.append(row)

    for path, line, names, _ in unguarded:
        print(f"{path}:{line}  {' -> '.join(names)}")
    print(f"{len(unguarded)} recursive walks over a document's own graph, with no bound (Rule 6)")
    print(f"{len(guarded)} guarded by a depth, a visited set or a worklist")
    print(f"{len(in_memory)} over an owned Rust structure, which cannot be cyclic")
    if exempt:
        print(f"\n{len(exempt)} exempt, with the guard named:")
        for path, line, names, why in exempt:
            print(f"  {path}:{line}  {' -> '.join(names)}  -> {why}")

    live = {(str(p), n) for p, _l, names, _w in exempt + guarded + unguarded + in_memory for n in names}
    stale = [k for k in GUARDED_ELSEWHERE if k not in live]
    for path, name in sorted(stale):
        print(f"\nSTALE EXEMPTION: {path} {name} is no longer recursive")
    return 1 if (unguarded or stale) else 0


if __name__ == "__main__":
    sys.exit(main())

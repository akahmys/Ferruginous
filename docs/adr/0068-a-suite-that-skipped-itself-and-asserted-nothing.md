# ADR-0068: A suite that skipped itself in silence and asserted nothing when it ran

- **Status**: Accepted
- **Date**: 2026-09-06
- **Commit**: (see the commit that adds this file)

## Context

`fepdf-mcp`'s tests were the only coverage the MCP tools had. They did two things:

**They skipped themselves.** Every test began `if !Path::new(&sample).exists() { return; }`
against `samples/sample.pdf`, and `.gitignore` excludes `/samples/` outright. On any
machine but the one that generated the corpus, **the whole file passed without running**.
Seven such returns.

**When they ran, they asserted `is_ok()`.** Five tests, twenty-odd operations, and not one
of them re-opened the output. `test_redaction_tool` was a single
`assert!(redact_res.is_ok())` — and redaction removed the *wrong* run of a page for as
long as it had existed ([ADR-0064](0064-redaction-removed-the-second-run-of-a-page-and-no-other.md)).
The operation had a test. The test had no way to fail.

## Decision

**The fixtures are built in the test file**, so nothing reads `samples/` and nothing can
skip. Each test says in its own body what document it needs.

**Every assertion is about the output document, read back.** A rotation is checked by the
page's reported size, an annotation and an outline through `InteractiveReport`, a
decoration and a Bates number through the extracted text, a removal by which pages remain.

**Not by searching the output's bytes.** The writer packs objects into 7.5.7 streams by
default ([ADR-0016](0016-objects-are-packed-by-default.md)), so `/Annots` and `/Outlines`
are inside a compressed container and a grep finds neither. The first version of three of
these tests did exactly that and failed; a test that greps the file is testing the
writer's compression setting.

## Consequences

- **Verified by putting ADR-0064's defect back**: the redaction test fails. Putting the
  count back to `args.targets.len()` fails it too. Both are what the suite it replaces
  passed over.
- **`redacted_count` was reporting the request, not the result.** The field is documented
  "Number of redactions successfully scrubbed" and read `args.targets.len()`, so a
  rectangle over empty space was reported to the caller — an agent — as a redaction that
  had happened. `apply_physical_redaction_to_page` now answers how many strings it
  scrubbed, and `affected_pages` lists only pages where something went.
- **The MCP surface counts pages two ways, and only one of them said so.** Eight `page`
  integers and `extract_text`'s `page_range` are zero-based and document it; five
  `pages`/`selection` strings are one-based and documented it as an example — so
  `page_range: "1"` and `pages: "1"` name different pages in the same tool set. Found by
  a test that assumed the wrong one. The five are now explicit, and one test names both
  bases together so neither drifts into the other unseen.
- **Unifying the two bases is not done here.** Changing a string's meaning would move
  every existing caller's pages by one without any error, which is the worst shape of
  change for a machine interface. The CLI is one-based throughout, deliberately, because
  a person counts from one; whether an agent surface should follow the CLI or its own
  integers is a decision, not a cleanup.
- **Three tools lost their only coverage** and did not regain it: `attach_associated_file`,
  `set_output_intent`, `update_layers`, `set_measurement_scale`, `set_form_field_value`
  and `update_struct_elem` were exercised by the old chain and are not in the new file.
  They were exercised by `is_ok()` against a document that had no forms, no layers and no
  structure tree, so what is lost is the appearance of coverage — but the appearance was
  all six had, and this record is where that is written down rather than left to be
  rediscovered.

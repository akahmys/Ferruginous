//! What the server puts in front of a client, checked rather than read.
//!
//! **`prompts.rs` named `get_structure_tree` for four phases and no such tool has ever
//! existed**, which is the kind of thing only a check catches: a prompt is prose until a
//! client tries to follow it. This walks the router the server actually serves.

use fepdf_mcp::FepdfServer;

/// Every tool this build registers, by name.
fn served() -> Vec<String> {
    FepdfServer::all_tools().list_all().into_iter().map(|t| t.name.to_string()).collect()
}

/// The `render_page` tool is present exactly when the feature that rasterises is.
///
/// It is the only tool that needs a GPU stack — 285 crates in the dependency tree with
/// it, 246 without, measured 2026-09-08 — so a server that answers over stdio and never
/// draws can be built without either.
#[test]
fn render_page_follows_its_feature() {
    let has = served().iter().any(|name| name == "render_page");
    assert_eq!(
        has,
        cfg!(feature = "render"),
        "the tool list and the feature disagree: {:?}",
        served()
    );
}

/// A tool a prompt names has to be one the server registers.
#[test]
fn every_tool_a_prompt_names_is_served() {
    let served = served();
    let prompts = [
        ("audit_accessibility", fepdf_mcp::prompts::prompt_audit_accessibility("f.pdf")),
        ("remediate_pdf_ua", fepdf_mcp::prompts::prompt_remediate_pdf_ua("in.pdf", "out.pdf")),
    ];
    let mut missing = Vec::new();
    for (name, text) in &prompts {
        for word in text.split('`') {
            if word.contains(' ') || word.is_empty() {
                continue;
            }
            if word.ends_with("_tool") || !word.contains('_') {
                continue;
            }
            if !served.contains(&word.to_string()) && looks_like_a_tool_name(word) {
                missing.push(format!("{name}: {word}"));
            }
        }
    }
    assert!(missing.is_empty(), "a prompt names a tool the server does not serve: {missing:?}");
}

/// A backticked word is taken for a tool name when it is `snake_case` and its first word
/// is a verb this server's tools begin with.
fn looks_like_a_tool_name(word: &str) -> bool {
    const VERBS: [&str; 8] = ["get", "set", "list", "read", "add", "remove", "render", "extract"];
    VERBS.iter().any(|v| word.starts_with(&format!("{v}_")))
}

//! Pre-configured MCP system prompts for PDF accessibility auditing and remediation.

/// Returns the system prompt template for evaluating PDF accessibility and Matterhorn compliance.
pub fn prompt_audit_accessibility(path: &str) -> String {
    format!(
        r"You are an expert accessibility auditor specialising in PDF/UA-2 (ISO 14289-2) and WCAG 2.2.
Please audit the PDF document at path: `{path}`.

Instructions:
1. Use the `audit_document` tool to check for structural compliance and Matterhorn failures.
2. Read the logical structure tree via the `get_structure_tree` tool or `pdf://local/{path}/struct_tree` resource.
3. Review headings hierarchy (H1 -> H2 -> H3), tables structure (TH, TD headers), and missing alternative texts (Alt) on Figures.
4. Provide a structured audit report with actionable remediation steps."
    )
}

/// Returns the system prompt template for performing guided remediation of PDF/UA-2 issues.
pub fn prompt_remediate_pdf_ua(input_path: &str, output_path: &str) -> String {
    format!(
        r"You are an autonomous document remediation assistant for PDF/UA-2 compliance.
Target input: `{input_path}`
Target output: `{output_path}`

Workflow:
1. Run `audit_document` on `{input_path}` to collect all compliance issues.
2. Use `update_struct_elem` to set proper tag roles (e.g. converting generic P to H1/H2 where appropriate) and attach missing Alt texts.
3. Use `add_page_decoration` or `set_page_labels` if page numbering or headers are inconsistent.
4. Verify the final result by auditing `{output_path}`."
    )
}

/// The prompts a client lists.
///
/// **These are what makes the module reachable.** Both templates above read correctly and
/// no MCP client could ask for one: nothing registered a prompt, so `prompts/list`
/// returned an empty array and `prompts/get` returned method-not-found.
pub(crate) fn catalogue() -> Vec<rmcp::model::Prompt> {
    vec![
        rmcp::model::Prompt::new(
            "audit_accessibility",
            Some("Audit a PDF against PDF/UA-2 and WCAG 2.2, and report what to remediate."),
            Some(vec![
                rmcp::model::PromptArgument::new("path")
                    .with_description("Path to the PDF to audit.")
                    .with_required(true),
            ]),
        ),
        rmcp::model::Prompt::new(
            "remediate_pdf_ua",
            Some("Walk a PDF to PDF/UA-2 compliance and write the result to a new file."),
            Some(vec![
                rmcp::model::PromptArgument::new("input_path")
                    .with_description("Path to the PDF to remediate.")
                    .with_required(true),
                rmcp::model::PromptArgument::new("output_path")
                    .with_description("Where to write the remediated PDF.")
                    .with_required(true),
            ]),
        ),
    ]
}

/// Builds the prompt `name` asks for.
///
/// # Errors
/// Fails when no prompt carries that name, or when a required argument is absent.
pub(crate) fn get(
    name: &str,
    arguments: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<rmcp::model::GetPromptResult, rmcp::ErrorData> {
    let argument = |key: &str| -> Result<String, rmcp::ErrorData> {
        arguments
            .and_then(|a| a.get(key))
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("`{name}` needs a string argument `{key}`"),
                    None,
                )
            })
    };
    let text = match name {
        "audit_accessibility" => prompt_audit_accessibility(&argument("path")?),
        "remediate_pdf_ua" => {
            prompt_remediate_pdf_ua(&argument("input_path")?, &argument("output_path")?)
        }
        other => {
            return Err(rmcp::ErrorData::invalid_params(
                format!("`{other}` is not a prompt this server carries"),
                None,
            ));
        }
    };
    Ok(rmcp::model::GetPromptResult::new(vec![rmcp::model::PromptMessage::new(
        rmcp::model::PromptMessageRole::User,
        rmcp::model::PromptMessageContent::text(text),
    )]))
}

#[cfg(test)]
mod reachable {
    use super::{catalogue, get};

    /// A prompt nobody can list is a prompt nobody has.
    #[test]
    fn every_prompt_this_module_writes_is_listed() {
        let listed = catalogue();
        let names: Vec<&str> = listed.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["audit_accessibility", "remediate_pdf_ua"]);
    }

    #[test]
    fn a_listed_prompt_can_be_fetched_by_the_name_it_is_listed_under() {
        for prompt in catalogue() {
            let mut args = serde_json::Map::new();
            for argument in prompt.arguments.iter().flatten() {
                args.insert(argument.name.clone(), serde_json::json!("some.pdf"));
            }
            let result = get(&prompt.name, Some(&args)).unwrap_or_else(|e| {
                panic!("`{}` is listed and cannot be fetched: {e}", prompt.name)
            });
            assert!(!result.messages.is_empty(), "`{}` carried no message", prompt.name);
        }
    }

    #[test]
    fn a_prompt_missing_a_required_argument_says_which() {
        let error = get("audit_accessibility", None).expect_err("`path` is required");
        assert!(format!("{error}").contains("path"), "{error}");
    }

    #[test]
    fn a_name_no_prompt_carries_is_refused() {
        assert!(get("no_such_prompt", None).is_err());
    }
}

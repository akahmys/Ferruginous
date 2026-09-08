//! Which operations run the form's calculation order, and which leave it alone (12.6.3).
//!
//! **The cascade ran after every operation until 2026-09-09.** The argument was that a
//! form with no `/CO` returns from `run_calculations` before building a context, so
//! restricting it would cost a test and buy nothing. That is true about the cost. What it
//! left out is what happens to a form that *does* declare `/CO`, and the second test here
//! is that: a page rotation overwrote a real date with the fixed instant the deterministic
//! script environment uses.

use fepdf_fixtures::assemble;

/// A one-page form whose single field is in `/CO` and computes `calc`.
fn form_that_calculates(name: &str, value: &str, calc: &str) -> Vec<u8> {
    assemble(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /CO [5 0 R] \
         /DA (/Helv 9 Tf 0 g) >> >>"
            .to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [5 0 R] \
         /Contents 4 0 R >>"
            .to_string(),
        "<< /Length 0 >>\nstream\n\nendstream".to_string(),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /V ({value}) \
             /Rect [0 0 100 20] /F 4 /DA (/Helv 9 Tf 0 g) \
             /AA << /C << /S /JavaScript /JS ({calc}) >> >> >>"
        ),
    ])
}

/// The value of `field` after the file at `path` is read back.
fn field_value(bytes: &[u8], field: &str) -> Option<String> {
    fepdf::InteractiveReport::survey(bytes)
        .expect("the output surveys")
        .form
        .terminal
        .iter()
        .find(|f| f.qualified_name.as_deref() == Some(field))
        .and_then(|f| f.value.clone())
}

/// Writes `file`, runs `op` over it through the server, and hands back the output bytes.
fn through_the_server(file: &[u8], name: &str, run: impl FnOnce(&str, &str)) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("fepdf_calculation_scope_{name}"));
    let _ = std::fs::create_dir_all(&dir);
    let input = dir.join("in.pdf");
    let output = dir.join("out.pdf");
    std::fs::write(&input, file).expect("the fixture writes");
    run(input.to_str().expect("utf-8 path"), output.to_str().expect("utf-8 path"));
    std::fs::read(&output).expect("the server wrote an output")
}

fn rotate(input: &str, output: &str) {
    let args = serde_json::from_value(serde_json::json!({
        "input_path": input,
        "output_path": output,
        "selection": "all",
        "angle": 90
    }))
    .expect("rotate args parse");
    fepdf_mcp::tools::rotate_pages_impl(args).expect("the rotation succeeds");
}

/// Rotating a page does not touch a field value, so it does not start the cascade.
///
/// `total` is stale in the fixture on purpose: it says `0` where the script would compute
/// `2`. Refreshing it here would be the engine deciding to, on a call that asked to rotate
/// a page.
#[test]
fn a_page_rotation_leaves_the_calculated_value_alone() {
    let file = form_that_calculates("total", "0", r"event.value = 2;");
    let after = through_the_server(&file, "total", rotate);
    assert_eq!(
        field_value(&after, "total").as_deref(),
        Some("0"),
        "rotating a page recalculated a field it was not asked to touch"
    );
}

/// The case that settled it: the calculated value is a date, and the clock is fixed.
///
/// `ScriptEnvironment::default()` pins the instant to 2020-01-01 so that two runs of the
/// same document agree, which is right. It also means a cascade run on an unrelated
/// operation replaces a real date with `2020`.
#[test]
fn a_page_rotation_does_not_stamp_the_fixed_clock_into_a_date_field() {
    let file = form_that_calculates(
        "signed_on",
        "2026-09-09",
        r"event.value = new Date\(\).getFullYear\(\);",
    );
    let after = through_the_server(&file, "date", rotate);
    assert_eq!(
        field_value(&after, "signed_on").as_deref(),
        Some("2026-09-09"),
        "rotating a page overwrote a date with the deterministic clock's year"
    );
}

/// Setting a field value *is* 12.6.3's trigger, and the cascade still runs for it.
#[test]
fn setting_a_field_value_still_runs_the_cascade() {
    let file = form_that_calculates("total", "0", r"event.value = 2;");
    let after = through_the_server(&file, "set", |input, output| {
        let args = serde_json::from_value(serde_json::json!({
            "input_path": input,
            "output_path": output,
            "field_name": "total",
            "value": "1"
        }))
        .expect("set args parse");
        fepdf_mcp::tools::set_form_field_value_impl(args).expect("the write succeeds");
    });
    assert_eq!(
        field_value(&after, "total").as_deref(),
        Some("2"),
        "the calculation order did not run after a field write"
    );
}

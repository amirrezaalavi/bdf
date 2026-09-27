//! TDD: the CLI contract. These tests are what make the product agent-drivable,
//! so they are stricter than the library tests: exit codes and envelope shape only.

use std::process::Command;

const FIXTURE: &str = "../../corpus/raw/synthetic/minimal-ltr.pdf";

fn pdfrtl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pdfrtl"))
}

#[test]
fn inspect_emits_one_json_envelope_and_exits_zero() {
    let out = pdfrtl()
        .args(["--json", "inspect", FIXTURE])
        .output()
        .expect("binary runs");

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("stdout is utf-8");
    assert_eq!(
        stdout.lines().count(),
        1,
        "exactly one JSON line on stdout, got: {stdout}"
    );

    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["pages"], serde_json::json!(1));
    assert_eq!(v["data"]["producer"], serde_json::json!("pdfrtl-gen"));
    assert_eq!(v["reasons"], serde_json::json!([]));
}

#[test]
fn missing_file_is_exit_4_with_ok_false_and_names_the_file() {
    let out = pdfrtl()
        .args(["--json", "inspect", "../../corpus/raw/synthetic/nope.pdf"])
        .output()
        .expect("binary runs");

    assert_eq!(out.status.code(), Some(4));

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout is JSON even on failure");
    assert_eq!(v["ok"], serde_json::json!(false));
    assert!(
        v["data"]["error"]
            .as_str()
            .unwrap_or_default()
            .contains("nope.pdf"),
        "error payload must name the file: {v}"
    );
}

#[test]
fn unknown_subcommand_is_exit_2() {
    let out = pdfrtl().args(["nonsense"]).output().expect("binary runs");
    assert_eq!(out.status.code(), Some(2), "usage errors are exit 2");
}

#[test]
fn exit_codes_are_pinned_in_the_library() {
    // Callers branch on these numbers instead of parsing text, so they are public API.
    // Pinned here (not in the binary) so the contract is assertable from outside.
    assert_eq!(pdfrtl_cli::exit::EXIT_OK, 0);
    assert_eq!(pdfrtl_cli::exit::EXIT_UNSUPPORTED, 3);
    assert_eq!(pdfrtl_cli::exit::EXIT_IO, 4);
}

#[test]
fn human_mode_prints_no_json_braces_on_stdout() {
    // Human mode exists so a person (or the future GUI) can read output, but stdout must
    // never mix modes: an agent that forgets --json must see parseable failure, not prose.
    let out = pdfrtl()
        .args(["inspect", FIXTURE])
        .output()
        .expect("binary runs");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains('{'),
        "human mode must not print JSON: {stdout}"
    );
    assert!(
        stdout.contains("pages:     1"),
        "human mode shows facts: {stdout}"
    );
}

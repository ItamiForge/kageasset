use assert_cmd::prelude::*;
use predicates::str::contains;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn shows_help() {
    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Kat (kat) is a compact command-line tool"));
}

#[test]
fn scan_placeholder_runs() {
    let temp = tempdir().expect("temp dir");

    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .arg("scan")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(contains("Wrote assets.md"));

    let report_path = temp.path().join("assets.md");
    assert!(report_path.exists(), "markdown report should be created");

    let contents = fs::read_to_string(report_path).expect("read report");
    assert!(contents.contains("# Asset Report"));
}

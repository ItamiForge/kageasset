use assert_cmd::prelude::*;
use predicates::str::contains;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn shows_help() {
    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("asset inventory"));
}

#[test]
fn scan_creates_report() {
    let temp = tempdir().expect("temp dir");

    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .arg("scan")
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(contains("Wrote assets.md"));

    assert!(temp.path().join("assets.md").exists());
}

use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const PNG_PIXEL: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 153, 99, 96, 0, 0, 0, 2, 0, 1, 229, 33,
    177, 120, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn write_png(path: &std::path::Path) {
    fs::write(path, PNG_PIXEL).expect("write png");
}

#[test]
fn finds_duplicate_groups() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    write_png(&dir.join("a.png"));
    write_png(&dir.join("b.png"));
    write_png(&dir.join("unique.png"));

    let output = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .arg("duplicates")
        .current_dir(dir)
        .output()
        .expect("run duplicates");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Found 1 duplicate group(s)."));
    assert!(stdout.contains("a.png"));
    assert!(stdout.contains("b.png"));
}

#[test]
fn duplicates_json_format() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    write_png(&dir.join("x.png"));
    write_png(&dir.join("y.png"));

    let output = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["duplicates", "--json"])
        .current_dir(dir)
        .output()
        .expect("run duplicates");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(value["total_groups"].as_u64(), Some(1));
    assert_eq!(value["groups"][0]["assets"].as_array().expect("assets").len(), 2);
}

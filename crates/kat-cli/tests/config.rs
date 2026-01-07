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
fn default_ignores_skip_node_modules() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    write_png(&dir.join("root.png"));
    fs::create_dir_all(dir.join("node_modules")).expect("create node_modules");
    write_png(&dir.join("node_modules/ignored.png"));

    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["scan", "--json"])
        .current_dir(dir)
        .assert()
        .success();

    let payload: Value = serde_json::from_slice(&fs::read(dir.join("assets.json")).expect("read"))
        .expect("parse json");
    let assets = payload["assets"].as_array().expect("assets");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["relative_path"].as_str(), Some("root.png"));
}

#[test]
fn config_file_ignores_patterns() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    write_png(&dir.join("keep.png"));
    fs::create_dir_all(dir.join("custom/sub")).expect("create custom");
    write_png(&dir.join("custom/sub/skip.png"));

    fs::write(
        dir.join(".kat.toml"),
        "[scan]\nignore = [\"**/custom/**\"]\n",
    )
    .expect("write config");

    Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["scan", "--json"])
        .current_dir(dir)
        .assert()
        .success();

    let payload: Value = serde_json::from_slice(&fs::read(dir.join("assets.json")).expect("read"))
        .expect("parse json");
    let assets = payload["assets"].as_array().expect("assets");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["relative_path"].as_str(), Some("keep.png"));
}

#[test]
fn info_shows_metadata() {
    let temp = tempdir().expect("temp dir");
    let image_path = temp.path().join("test.png");
    write_png(&image_path);

    let output = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["info", image_path.to_str().expect("path")])
        .output()
        .expect("run info");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("size:"));
    assert!(stdout.contains("hash:"));
    assert!(stdout.contains("resolution: 1x1"));
    assert!(stdout.contains("format:"));
}

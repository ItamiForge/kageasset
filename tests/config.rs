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
fn scan_uses_default_ignores() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    write_png(&dir.join("root.png"));
    fs::create_dir_all(dir.join("node_modules")).expect("create node_modules");
    write_png(&dir.join("node_modules/ignored.png"));

    let status = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["scan", "--json"])
        .current_dir(dir)
        .status()
        .expect("run scan");
    assert!(status.success(), "scan command should succeed");

    let json_path = dir.join("assets.json");
    assert!(json_path.exists(), "assets.json should be created");

    let payload: Value =
        serde_json::from_slice(&fs::read(json_path).expect("read json")).expect("parse json");

    let assets = payload
        .get("assets")
        .and_then(|v| v.as_array())
        .expect("assets array");
    assert_eq!(assets.len(), 1, "only root asset should be listed");
    assert_eq!(
        assets[0]
            .get("relative_path")
            .and_then(|v| v.as_str())
            .expect("relative path"),
        "root.png"
    );

    let ignored = payload.get("config_path").and_then(|v| v.as_str());
    assert!(
        ignored.is_none(),
        "config path should be absent when defaults only"
    );
}

#[test]
fn scan_respects_config_ignore() {
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

    let status = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["scan", "--json"])
        .current_dir(dir)
        .status()
        .expect("run scan");
    assert!(status.success(), "scan command should succeed");

    let json_path = dir.join("assets.json");
    let payload: Value =
        serde_json::from_slice(&fs::read(json_path).expect("read json")).expect("parse json");

    let assets = payload
        .get("assets")
        .and_then(|v| v.as_array())
        .expect("assets array");
    assert_eq!(assets.len(), 1, "only keep.png should be indexed");
    assert_eq!(
        assets[0]
            .get("relative_path")
            .and_then(|v| v.as_str())
            .expect("relative path"),
        "keep.png"
    );
}

#[test]
fn info_command_outputs_metadata() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();

    let image_path = dir.join("info.png");
    write_png(&image_path);

    let output = Command::new(assert_cmd::cargo::cargo_bin!("kat"))
        .args(["info", image_path.to_str().expect("path str")])
        .output()
        .expect("run info");

    assert!(output.status.success(), "info command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("size:"), "should include file size");
    assert!(stdout.contains("hash:"), "should include hash");
    assert!(
        stdout.contains("resolution: 1x1"),
        "should include resolution"
    );
    assert!(stdout.contains("format:"), "should include format");
}

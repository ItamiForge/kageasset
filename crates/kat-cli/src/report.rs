
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use kat_scanner::{Asset, ScanResult};

#[derive(Serialize)]
struct JsonReport<'a> {
    schema_version: &'static str,
    total_assets: usize,
    assets: &'a [Asset],
    warnings: &'a [String],
    config_path: Option<&'a str>,
}

pub fn write_markdown(path: &Path, result: &ScanResult) -> Result<()> {
    ensure_parent_exists(path)?;

    let file = File::create(path).with_context(|| format!("failed to create report file {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "# Asset Report")?;
    writeln!(writer)?;
    writeln!(writer, "Total assets: {}", result.assets.len())?;
    writeln!(writer)?;

    if result.assets.is_empty() {
        writeln!(writer, "(No assets found)")?;
    } else {
        writeln!(
            writer,
            "| Path | Size | Resolution | Format | Transparency |"
        )?;
        writeln!(
            writer,
            "|------|------|------------|--------|---------------|"
        )?;

        for asset in &result.assets {
            let path = escape_markdown(&asset.relative_path);
            let size = format_size(asset.size_bytes);
            let resolution = format_resolution(asset.width, asset.height);
            let format = asset
                .format
                .as_deref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string());
            let transparency = match asset.has_alpha {
                Some(true) => "has alpha".to_string(),
                Some(false) => "opaque".to_string(),
                None => "-".to_string(),
            };

            writeln!(
                writer,
                "| {path} | {size} | {resolution} | {format} | {transparency} |"
            )?;

            if let Some(error) = &asset.error {
                let escaped_error = escape_markdown(error);
                writeln!(writer, "| ↳ error | {escaped_error} |  |  |  |")?;
            }
        }
    }
    writeln!(writer)?;

    if !result.warnings.is_empty() {
        writeln!(writer, "## Warnings")?;
        for warning in &result.warnings {
            writeln!(writer, "- {warning}")?;
        }
    }

    writer.flush()?;
    Ok(())
}

pub fn write_json(path: &Path, result: &ScanResult) -> Result<()> {
    ensure_parent_exists(path)?;

    let payload = JsonReport {
        schema_version: "0.1.0",
        total_assets: result.assets.len(),
        assets: &result.assets,
        warnings: &result.warnings,
        config_path: result.config_path.as_deref(),
    };

    let file = File::create(path).with_context(|| format!("failed to create json report {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &payload)?;
    Ok(())
}

pub fn derive_json_path(markdown_path: &Path) -> PathBuf {
    match markdown_path.file_stem() {
        Some(stem) => {
            let mut file_name = stem.to_os_string();
            file_name.push(".json");
            if let Some(parent) = markdown_path.parent() {
                parent.join(file_name)
            } else {
                PathBuf::from(file_name)
            }
        }
        None => markdown_path.with_extension("json"),
    }
}

fn ensure_parent_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory for {}", path.display())
            })?;
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

fn format_resolution(width: Option<u32>, height: Option<u32>) -> String {
    match (width, height) {
        (Some(w), Some(h)) => format!("{w}x{h}"),
        _ => "-".to_string(),
    }
}

fn escape_markdown(text: &str) -> String {
    text.replace('|', "\\|")
}

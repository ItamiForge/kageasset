use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::cli::{
    ConfigCommand, ConfigInitArgs, ConfigShowArgs, DuplicateArgs, InfoArgs, ScanArgs, ScanOptions,
    SfsymbolArgs,
};
use crate::config::{self, ConfigFile, ScanSection};
use crate::report;
use kat_pipeline::external::{bgone, imagemagick, potrace, svgo, swiftdraw};
use kat_scanner::{self as scanner, inspect as metadata_inspect, MetadataOptions};
use kat_sfsymbol::{sfsymbol_pipeline, sfsymbol_pipeline_with_bg_removal};

pub fn run_scan(args: ScanArgs) -> Result<()> {
    if args.options.paths.is_empty() {
        bail!("no scan paths provided");
    }

    let plan = build_scan_plan(&args.options, args.meta)?;
    let mut result = scanner::scan(&plan)?;
    result
        .assets
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    report::write_markdown(&args.output, &result)?;
    if args.json {
        let json_path = report::derive_json_path(&args.output);
        report::write_json(&json_path, &result)?;
        println!(
            "Wrote {} and {}",
            args.output.display(),
            json_path.display()
        );
    } else {
        println!("Wrote {}", args.output.display());
    }

    if !result.warnings.is_empty() {
        eprintln!("Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            eprintln!("  - {warning}");
        }
    }

    Ok(())
}

pub fn run_info(args: InfoArgs) -> Result<()> {
    let path = args.path;
    let metadata = fs::metadata(&path)
        .with_context(|| format!("failed to read file metadata for {}", path.display()))?;

    println!("Asset: {}", path.display());
    println!("  size: {} bytes", metadata.len());

    match metadata.modified() {
        Ok(modified) => {
            if let Ok(delta) = modified.duration_since(std::time::UNIX_EPOCH) {
                println!(
                    "  modified: {}",
                    humantime::format_rfc3339_seconds(std::time::UNIX_EPOCH + delta)
                );
            }
        }
        Err(_) => {
            println!("  modified: (unknown)");
        }
    }

    let mut bytes = Vec::new();
    let info_len = metadata.len();
    if info_len <= 20 * 1024 * 1024 {
        if let Ok(content) = fs::read(&path) {
            bytes = content;
        }
    } else {
        if let Ok(f) = std::fs::File::open(&path) {
            use std::io::Read;
            // 2MB prefix
            let _ = f.take(2 * 1024 * 1024).read_to_end(&mut bytes);
        }
    }

    match metadata_inspect(&path, &bytes, MetadataOptions { deep: args.meta }) {
        Ok(meta) => {
            if let (Some(width), Some(height)) = (meta.width, meta.height) {
                println!("  resolution: {}x{}", width, height);
            }
            if let Some(format) = meta.format {
                println!("  format: {format}");
            }
            if let Some(color_type) = meta.color_type {
                println!("  color type: {color_type}");
            }
            if let Some(has_alpha) = meta.has_alpha {
                println!(
                    "  transparency: {}",
                    if has_alpha { "has alpha" } else { "opaque" }
                );
            }
            if let Some(orientation) = meta.orientation {
                println!("  orientation: {orientation}");
            }
            if let Some(profile) = meta.color_profile_name {
                println!("  color profile: {profile}");
            }
            if let Some(total) = meta.metadata_total_bytes {
                println!("  embedded metadata: {} bytes", total);
                for (kind, size) in meta.metadata_sizes {
                    println!("    - {kind}: {size} bytes");
                }
            }
            if !meta.flags.is_empty() {
                println!("  flags:");
                for flag in meta.flags {
                    println!("    - {flag}");
                }
            }
            if args.meta && !meta.metadata.is_empty() {
                println!("  metadata entries:");
                for (key, value) in meta.metadata {
                    println!("    {key}: {value}");
                }
            }
        }
        Err(err) => {
            eprintln!("warning: failed to inspect image metadata: {err}");
        }
    }

    match scanner::compute_hash(&path) {
        Ok(hash) => println!("  hash: {hash}"),
        Err(err) => eprintln!("warning: failed to hash file: {err}"),
    }

    Ok(())
}

pub fn run_duplicates(args: DuplicateArgs) -> Result<()> {
    let plan = build_scan_plan(&args.options, false)?;
    let mut result = scanner::scan(&plan)?;
    result
        .assets
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for asset in &result.assets {
        if let Some(hash) = &asset.hash {
            map.entry(hash.clone())
                .or_default()
                .push(asset.relative_path.clone());
        }
    }

    let groups: Vec<DuplicateGroup> = map
        .into_iter()
        .filter_map(|(hash, assets)| {
            if assets.len() > 1 {
                Some(DuplicateGroup::new(hash, assets))
            } else {
                None
            }
        })
        .collect();

    if args.json {
        let payload = DuplicateReport::new(groups.clone(), &result.warnings);
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if groups.is_empty() {
        println!("No duplicates found.");
    } else {
        println!("Found {} duplicate group(s).", groups.len());
        for group in &groups {
            println!("hash: {} ({} files)", group.hash, group.count);
            for path in &group.assets {
                println!("  - {path}");
            }
        }
    }

    if !result.warnings.is_empty() {
        eprintln!("Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            eprintln!("  - {warning}");
        }
    }

    Ok(())
}

#[derive(Clone, Serialize)]
struct DuplicateGroup {
    hash: String,
    count: usize,
    assets: Vec<String>,
}

impl DuplicateGroup {
    fn new(hash: String, mut assets: Vec<String>) -> Self {
        assets.sort();
        let count = assets.len();
        Self {
            hash,
            count,
            assets,
        }
    }
}

#[derive(Serialize)]
struct DuplicateReport {
    schema_version: &'static str,
    total_groups: usize,
    groups: Vec<DuplicateGroup>,
    warnings: Vec<String>,
}

impl DuplicateReport {
    fn new(groups: Vec<DuplicateGroup>, warnings: &[String]) -> Self {
        Self {
            schema_version: "0.1.0",
            total_groups: groups.len(),
            groups,
            warnings: warnings.to_vec(),
        }
    }
}

pub fn run_config(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show(args) => run_config_show(args),
        ConfigCommand::Init(args) => run_config_init(args),
    }
}

fn build_scan_plan(options: &ScanOptions, deep_metadata: bool) -> Result<scanner::ScanPlan> {
    let config = config::load(options.config_path.clone())?;

    let use_default_ignores = if options.no_default_ignores {
        false
    } else {
        config.scan.use_default_ignores.unwrap_or(true)
    };

    let mut ignore_patterns: Vec<String> = Vec::new();
    if use_default_ignores {
        ignore_patterns.extend(
            config::default_ignore_patterns()
                .iter()
                .map(|pattern| (*pattern).to_string()),
        );
    }
    ignore_patterns.extend(config.scan.ignore.clone());
    ignore_patterns.extend(options.ignore.clone());
    ignore_patterns.retain(|pattern| !pattern.trim().is_empty());
    ignore_patterns.sort();
    ignore_patterns.dedup();

    let extensions = if !options.extensions.is_empty() {
        options.extensions.clone()
    } else if !config.scan.extensions.is_empty() {
        config.scan.extensions.clone()
    } else {
        Vec::new()
    };

    let paths = if options.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        options.paths.clone()
    };

    Ok(scanner::ScanPlan {
        paths,
        extensions,
        ignore_patterns,
        config_path: config.source_path,
        deep_metadata,
    })
}

fn run_config_show(args: ConfigShowArgs) -> Result<()> {
    let config = config::load(args.path.clone())?;

    println!(
        "Configuration: {}",
        args.path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(auto)".to_string())
    );
    if let Some(source) = &config.source_path {
        println!("Resolved file: {}", source.display());
    } else {
        println!("Resolved file: (none, using defaults)");
    }

    let use_defaults = config.scan.use_default_ignores.unwrap_or(true);
    println!("Default ignores enabled: {use_defaults}");

    if use_defaults {
        let defaults = config::default_ignore_patterns()
            .iter()
            .map(|pattern| pattern.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "Built-in ignores ({}): {}",
            config::default_ignore_patterns().len(),
            defaults
        );
    }

    if config.scan.ignore.is_empty() {
        println!("Custom ignores: (none)");
    } else {
        println!(
            "Custom ignores ({}): {}",
            config.scan.ignore.len(),
            config.scan.ignore.join(", ")
        );
    }

    if config.scan.extensions.is_empty() {
        println!("Extensions: (defaults)");
    } else {
        println!("Extensions: {}", config.scan.extensions.join(", "));
    }

    Ok(())
}

fn run_config_init(args: ConfigInitArgs) -> Result<()> {
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(config::default_config_file_name()));

    if output_path.exists() && !args.force {
        bail!(
            "configuration file '{}' already exists (use --force to overwrite)",
            output_path.display()
        );
    }

    let mut ignore = args.ignore.clone();
    ignore.retain(|pattern| !pattern.trim().is_empty());
    ignore.sort();
    ignore.dedup();

    let mut extensions = args.extensions.clone();
    extensions.retain(|ext| !ext.trim().is_empty());
    extensions.sort();
    extensions.dedup();

    let config_file = ConfigFile {
        scan: ScanSection {
            ignore,
            extensions,
            use_default_ignores: Some(!args.no_default_ignores),
        },
    };

    config::write_config_file(&output_path, &config_file)?;

    println!("Wrote {}", output_path.display());

    Ok(())
}

/// Supported image extensions for SF Symbol conversion.
const SFSYMBOL_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn run_sfsymbol(args: SfsymbolArgs, quiet: bool) -> Result<()> {
    let files = collect_input_files(&args.input)?;

    if files.is_empty() {
        bail!("no supported image files found in '{}'", args.input.display());
    }

    if let Some(ref output) = args.output {
        fs::create_dir_all(output).with_context(|| {
            format!("failed to create output directory '{}'", output.display())
        })?;
    }

    let pipeline = if args.remove_bg {
        sfsymbol_pipeline_with_bg_removal(args.size).keep_temp(args.keep_temp)
    } else {
        sfsymbol_pipeline(args.size).keep_temp(args.keep_temp)
    };

    if let Err(e) = pipeline.validate_tools() {
        eprintln!("{}: {}", "Error".red().bold(), e);
        eprintln!("\nRun 'kat doctor' to check tool availability.");
        std::process::exit(1);
    }

    let progress = if !quiet && files.len() > 1 {
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut successes = Vec::new();
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        if let Some(ref pb) = progress {
            pb.set_message(file.file_name().unwrap_or_default().to_string_lossy().to_string());
        }

        let output_dir = args
            .output
            .clone()
            .or_else(|| file.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        fs::create_dir_all(&output_dir).with_context(|| {
            format!("failed to create output directory '{}'", output_dir.display())
        })?;

        match pipeline.run(file, &output_dir) {
            Ok(output_path) => {
                successes.push((file.clone(), output_path));
            }
            Err(e) => {
                failures.push((file.clone(), e.to_string()));
            }
        }

        if let Some(ref pb) = progress {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    if !quiet {
        println!("\n{}", "SF Symbol Generation Complete".green().bold());
        println!("  {} processed", format!("{} file(s)", successes.len()).cyan());
        
        if !failures.is_empty() {
            println!("  {} failed", format!("{} file(s)", failures.len()).red());
        }

        if !successes.is_empty() {
            if let Some(output) = &args.output {
                println!("\nOutput directory: {}", output.display());
            } else {
                println!("\nOutput files saved next to the source images.");
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("\n{}", "Failures:".red().bold());
        for (path, error) in &failures {
            eprintln!("  {} {}", path.display().to_string().yellow(), error);
        }
    }

    Ok(())
}

fn collect_input_files(input: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if input.is_file() {
        // Single file
        if is_supported_image(input) {
            files.push(input.clone());
        } else {
            bail!(
                "unsupported file format: '{}'. Supported formats: {}",
                input.display(),
                SFSYMBOL_EXTENSIONS.join(", ")
            );
        }
    } else if input.is_dir() {
        // Directory - collect all supported images
        for entry in fs::read_dir(input)
            .with_context(|| format!("failed to read directory '{}'", input.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && is_supported_image(&path) {
                files.push(path);
            }
        }
        files.sort();
    } else {
        bail!("input path '{}' does not exist", input.display());
    }

    Ok(files)
}

fn is_supported_image(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SFSYMBOL_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn run_doctor() -> Result<()> {
    println!("{}", "SF Symbol Pipeline Tool Check".cyan().bold());
    println!();

    let tools = vec![
        bgone(),
        imagemagick(),
        potrace(),
        svgo(),
        swiftdraw(),
    ];

    let mut all_available = true;

    println!("{:<15} {:<12} {}", "Tool", "Status", "Install Command");
    println!("{}", "-".repeat(60));

    for tool in &tools {
        let available = tool.is_available();
        let status = if available {
            "✓ installed".green().to_string()
        } else {
            all_available = false;
            "✗ missing".red().to_string()
        };

        println!(
            "{:<15} {:<22} {}",
            tool.name,
            status,
            tool.install_hint.dimmed()
        );
    }

    println!();

    if all_available {
        println!("{}", "All tools are installed and ready!".green().bold());
        Ok(())
    } else {
        println!(
            "{}",
            "Some tools are missing. Install them to use the SF Symbol pipeline.".yellow()
        );
        std::process::exit(1);
    }
}

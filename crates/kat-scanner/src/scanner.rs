use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use pathdiff::diff_paths;
use rayon::prelude::*;
use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use indicatif::ParallelProgressIterator;

use crate::metadata::{self, MetadataOptions};

/// Represents an image file with associated metadata.
#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub size_bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_type: Option<String>,
    pub has_alpha: Option<bool>,
    pub format: Option<String>,
    pub hash: Option<String>,
    pub error: Option<String>,
    pub orientation: Option<String>,
    pub color_profile_name: Option<String>,
    pub metadata_total_bytes: Option<u64>,
    pub metadata_sizes: BTreeMap<String, u64>,
    pub metadata: BTreeMap<String, String>,
    pub flags: Vec<String>,
}

impl Asset {
    pub fn new(relative_path: String, absolute_path: PathBuf, size_bytes: u64) -> Self {
        Self {
            relative_path,
            absolute_path,
            size_bytes,
            width: None,
            height: None,
            color_type: None,
            has_alpha: None,
            format: None,
            hash: None,
            error: None,
            orientation: None,
            color_profile_name: None,
            metadata_total_bytes: None,
            metadata_sizes: BTreeMap::new(),
            metadata: BTreeMap::new(),
            flags: Vec::new(),
        }
    }
}

/// Result of a scan operation.
#[derive(Debug, Default)]
pub struct ScanResult {
    pub assets: Vec<Asset>,
    pub warnings: Vec<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanPlan {
    pub paths: Vec<PathBuf>,
    pub extensions: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub config_path: Option<PathBuf>,
    pub deep_metadata: bool,
}

const DEFAULT_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg"];
const MAX_MEMORY_READ_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

pub fn scan(plan: &ScanPlan) -> Result<ScanResult> {
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    let ignore_set = build_ignore_set(&plan.ignore_patterns)?;
    let extensions = normalize_extensions(&plan.extensions);

    // Phase 1: Collect files
    let mut files_to_scan = Vec::new();
    let mut warnings = Vec::new();

    for input_path in &plan.paths {
        let resolved = match fs::canonicalize(input_path) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(format!("unable to resolve path '{}': {}", input_path.display(), e));
                continue;
            }
        };

        if resolved.is_dir() {
            collect_files(&resolved, &ignore_set, &extensions, &mut files_to_scan, &mut warnings);
        } else if resolved.is_file() {
            if should_include(&resolved, &extensions) {
                files_to_scan.push(resolved);
            }
        } else {
            warnings.push(format!("skipping unsupported path type: {}", resolved.display()));
        }
    }

    // Phase 2: Parallel process
    use indicatif::{ProgressBar, ProgressStyle};
    
    let pb = ProgressBar::new(files_to_scan.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let results: Vec<(Option<Asset>, Vec<String>)> = files_to_scan
        .par_iter()
        .progress_with(pb)
        .map(|path| process_path(path, &cwd, plan.deep_metadata))
        .collect();

    let mut final_assets = Vec::with_capacity(results.len());
    
    for (asset_opt, w) in results {
        if let Some(asset) = asset_opt {
            final_assets.push(asset);
        }
        warnings.extend(w);
    }

    Ok(ScanResult {
        assets: final_assets,
        warnings,
        config_path: plan.config_path.as_ref().map(|p| p.display().to_string()),
    })
}

fn collect_files(
    root: &Path,
    ignore_set: &Option<GlobSet>,
    extensions: &HashSet<String>,
    collector: &mut Vec<PathBuf>,
    warnings: &mut Vec<String>,
) {
    let iter = WalkDir::new(root).follow_links(false).into_iter();
    for entry in iter {
        match entry {
            Ok(entry) => {
                if should_skip(&entry, ignore_set) {
                    continue;
                }
                if entry.file_type().is_file() && should_include(entry.path(), extensions) {
                    collector.push(entry.path().to_path_buf());
                }
            }
            Err(e) => warnings.push(e.to_string()),
        }
    }
}

fn should_skip(entry: &DirEntry, ignore_set: &Option<GlobSet>) -> bool {
    ignore_set
        .as_ref()
        .map(|set| set.is_match(entry.path()))
        .unwrap_or(false)
}

fn process_path(path: &Path, cwd: &Path, deep_metadata: bool) -> (Option<Asset>, Vec<String>) {
    let mut warnings = Vec::new();
    
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warnings.push(format!("failed to read metadata for {}: {}", path.display(), e));
            return (None, warnings);
        }
    };

    let size_bytes = metadata.len();
    let absolute_path = path.to_path_buf();
    
    let relative_path = diff_paths(&absolute_path, cwd)
        .unwrap_or_else(|| PathBuf::from(path.file_name().unwrap_or_default()))
        .to_string_lossy()
        .replace('\\', "/");

    let mut asset = Asset::new(relative_path, absolute_path.clone(), size_bytes);

    if size_bytes <= MAX_MEMORY_READ_BYTES {
        // Small file: read all
        match fs::read(path) {
            Ok(bytes) => {
                let hash = blake3::hash(&bytes).to_hex().to_string();
                asset.hash = Some(hash);
                
                match metadata::inspect(path, &bytes, MetadataOptions { deep: deep_metadata }) {
                    Ok(meta) => apply_metadata(&mut asset, meta),
                    Err(e) => asset.error = Some(e.to_string()),
                }
            }
            Err(e) => {
                warnings.push(format!("failed to read file {}: {}", path.display(), e));
                return (None, warnings);
            }
        }
    } else {
        // Large file logic
        // 1. Prefix read
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                warnings.push(format!("failed to open file {}: {}", path.display(), e));
                return (None, warnings);
            }
        };
        
        // 2MB prefix
        let mut buffer = Vec::with_capacity(2 * 1024 * 1024);
        if let Err(e) = file.by_ref().take(2 * 1024 * 1024).read_to_end(&mut buffer) {
             warnings.push(format!("failed to read prefix {}: {}", path.display(), e));
        } else {
             match metadata::inspect(path, &buffer, MetadataOptions { deep: deep_metadata }) {
                Ok(meta) => apply_metadata(&mut asset, meta),
                Err(e) => asset.error = Some(e.to_string()),
            }
        }
        
        // 2. Hash (stream entire file)
        if let Err(e) = file.seek(SeekFrom::Start(0)) {
             warnings.push(format!("failed to seek file {}: {}", path.display(), e));
        } else {
             match compute_hash_stream(file) {
                 Ok(h) => asset.hash = Some(h),
                 Err(e) => warnings.push(format!("failed to hash {}: {}", path.display(), e)),
             }
        }
    }

    (Some(asset), warnings)
}

fn apply_metadata(asset: &mut Asset, meta: metadata::ImageMetadata) {
    asset.width = meta.width;
    asset.height = meta.height;
    asset.color_type = meta.color_type;
    asset.has_alpha = meta.has_alpha;
    asset.format = meta.format;
    asset.orientation = meta.orientation;
    asset.color_profile_name = meta.color_profile_name;
    asset.metadata_total_bytes = meta.metadata_total_bytes;
    asset.metadata_sizes = meta.metadata_sizes;
    asset.metadata = meta.metadata;
    asset.flags = meta.flags;
}

fn compute_hash_stream(file: File) -> Result<String> {
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

pub fn compute_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    compute_hash_stream(file)
}

fn should_include(path: &Path, extensions: &HashSet<String>) -> bool {
    if extensions.is_empty() {
        return true;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.trim_start_matches('.').to_lowercase())
        .unwrap_or_default();

    extensions.contains(&ext)
}

fn normalize_extensions(input: &[String]) -> HashSet<String> {
    if input.is_empty() {
        DEFAULT_EXTENSIONS
            .iter()
            .map(|ext| (*ext).to_string())
            .collect()
    } else {
        input
            .iter()
            .map(|ext| ext.trim_start_matches('.').to_lowercase())
            .collect()
    }
}

fn build_ignore_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }

    Ok(Some(builder.build()?))
}

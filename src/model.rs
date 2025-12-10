use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

#[derive(Debug, Default)]
pub struct ScanResult {
    pub assets: Vec<Asset>,
    pub warnings: Vec<String>,
    pub config_path: Option<String>,
}

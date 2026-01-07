//! Asset and ImageInfo types for representing image files with metadata.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Represents an image file with associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub size_bytes: u64,
    pub info: Option<ImageInfo>,
    pub hash: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub error: Option<String>,
}

/// Contains dimensions, format, and color information for an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub color_type: Option<String>,
    pub has_alpha: bool,
}

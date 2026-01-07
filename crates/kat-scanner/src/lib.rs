//! kat-scanner: File discovery and metadata extraction for Kat.

pub mod metadata;
pub mod scanner;

pub use scanner::{scan, compute_hash, ScanPlan, ScanResult, Asset};
pub use metadata::{inspect, ImageMetadata, MetadataOptions};

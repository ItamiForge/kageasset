//! Stage trait and types for pipeline processing.

use crate::external::ExternalTool;
use kat_core::Result;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// Input to a pipeline stage.
#[derive(Debug, Clone)]
pub struct StageInput {
    /// Path to the input file for this stage.
    pub path: PathBuf,
    /// Original source file path (for reference).
    pub source_path: PathBuf,
    /// Metadata carried through the pipeline.
    pub metadata: HashMap<String, String>,
}

/// Output from a pipeline stage.
#[derive(Debug, Clone)]
pub struct StageOutput {
    /// Path to the output file from this stage.
    pub path: PathBuf,
    /// Updated metadata.
    pub metadata: HashMap<String, String>,
}

/// A single processing stage in a pipeline.
pub trait Stage: Send + Sync {
    /// Human-readable name of this stage.
    fn name(&self) -> &str;

    /// Process the input and produce output.
    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput>;

    /// List of external tools required by this stage.
    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![]
    }
}

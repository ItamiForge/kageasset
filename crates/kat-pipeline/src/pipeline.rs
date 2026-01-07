//! Pipeline orchestrator for executing stages in sequence.

use crate::stage::{Stage, StageInput};
use kat_core::{KatError, Result};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A pipeline that holds an ordered sequence of stages.
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
    keep_temp: bool,
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Self {
            stages: vec![],
            keep_temp: false,
        }
    }

    /// Add a stage to the pipeline (fluent builder pattern).
    pub fn add<S: Stage + 'static>(mut self, stage: S) -> Self {
        self.stages.push(Box::new(stage));
        self
    }

    /// Configure whether to keep temporary files.
    pub fn keep_temp(mut self, keep: bool) -> Self {
        self.keep_temp = keep;
        self
    }

    /// Validate all required tools are available.
    pub fn validate_tools(&self) -> Result<()> {
        let mut missing = vec![];
        for stage in &self.stages {
            for tool in stage.required_tools() {
                if !tool.is_available() {
                    missing.push(tool);
                }
            }
        }
        if !missing.is_empty() {
            return Err(KatError::ToolNotFound {
                tool: missing
                    .iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                install_hint: missing
                    .iter()
                    .map(|t| t.install_hint.clone())
                    .collect::<Vec<_>>()
                    .join("\n"),
            });
        }
        Ok(())
    }

    /// Execute the pipeline on a single input file.
    ///
    /// Temp directory behavior:
    /// - If `keep_temp` is true: temp directory is preserved regardless of outcome
    /// - If `keep_temp` is false and pipeline succeeds: temp directory is deleted
    /// - If `keep_temp` is false and pipeline fails: temp directory is preserved for debugging
    pub fn run(&self, input_path: &Path, output_dir: &Path) -> Result<PathBuf> {
        self.validate_tools()?;

        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().to_path_buf();

        let result = self.run_stages(input_path, output_dir, &temp_path);

        // Handle temp directory cleanup based on outcome
        match (&result, self.keep_temp) {
            // Always keep if keep_temp is set
            (_, true) => {
                let _ = temp_dir.keep(); // Prevent cleanup
            }
            // Keep on failure for debugging
            (Err(_), false) => {
                let _ = temp_dir.keep(); // Prevent cleanup
            }
            // Delete on success when keep_temp is false
            (Ok(_), false) => {
                // TempDir will be dropped and cleaned up automatically
            }
        }

        result
    }

    fn run_stages(&self, input_path: &Path, output_dir: &Path, temp_path: &Path) -> Result<PathBuf> {
        let mut current = StageInput {
            path: input_path.to_path_buf(),
            source_path: input_path.to_path_buf(),
            metadata: Default::default(),
        };

        for stage in &self.stages {
            let output = stage
                .process(current.clone(), temp_path)
                .map_err(|e| KatError::StageFailed {
                    stage: stage.name().to_string(),
                    message: e.to_string(),
                })?;
            current = StageInput {
                path: output.path,
                source_path: current.source_path,
                metadata: output.metadata,
            };
        }

        // Copy final output to destination
        let file_name = current
            .source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let output_path = output_dir.join(format!("{}.svg", file_name));
        std::fs::copy(&current.path, &output_path)?;

        Ok(output_path)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

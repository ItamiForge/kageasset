//! Vectorization stage using potrace.

use kat_core::Result;
use kat_pipeline::external::{imagemagick, potrace, ExternalTool};
use kat_pipeline::{Stage, StageInput, StageOutput};
use std::path::Path;

pub struct Vectorize;

impl Stage for Vectorize {
    fn name(&self) -> &str {
        "Vectorize"
    }

    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![imagemagick(), potrace()]
    }

    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput> {
        let stem = input
            .source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        let pbm_path = temp_dir.join(format!("{}_alpha.pbm", stem));
        let output_path = temp_dir.join(format!("{}_raw.svg", stem));

        imagemagick().execute(&[
            input.path.to_str().unwrap(),
            "-alpha",
            "extract",
            "-negate",
            "-morphology",
            "Close",
            "Disk:1",
            pbm_path.to_str().unwrap(),
        ])?;

        potrace().execute(&[
            "-s",
            "-t",
            "2",
            "-a",
            "1.34",
            "-O",
            "0.2",
            "-o",
            output_path.to_str().unwrap(),
            pbm_path.to_str().unwrap(),
        ])?;

        let _ = std::fs::remove_file(&pbm_path);

        Ok(StageOutput {
            path: output_path,
            metadata: input.metadata,
        })
    }
}

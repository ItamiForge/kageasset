//! SVG optimization stage using svgo.

use kat_core::Result;
use kat_pipeline::external::{svgo, ExternalTool};
use kat_pipeline::{Stage, StageInput, StageOutput};
use std::path::Path;

pub struct OptimizeSvg;

impl Stage for OptimizeSvg {
    fn name(&self) -> &str {
        "Optimize SVG"
    }

    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![svgo()]
    }

    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput> {
        let output_path = temp_dir.join(format!(
            "{}_clean.svg",
            input
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        svgo().execute(&[
            "-i",
            input.path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "--multipass",
        ])?;

        Ok(StageOutput {
            path: output_path,
            metadata: input.metadata,
        })
    }
}

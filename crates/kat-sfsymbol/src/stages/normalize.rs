//! Image normalization stage using ImageMagick.

use kat_core::Result;
use kat_pipeline::external::{imagemagick, ExternalTool};
use kat_pipeline::{Stage, StageInput, StageOutput};
use std::path::Path;

pub struct Normalize {
    pub size: u32,
}

impl Normalize {
    pub fn new(size: u32) -> Self {
        Self { size }
    }
}

impl Stage for Normalize {
    fn name(&self) -> &str {
        "Normalize"
    }

    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![imagemagick()]
    }

    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput> {
        let output_path = temp_dir.join(format!(
            "{}_ready.png",
            input
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        let extent = format!("{}x{}", self.size, self.size);

        imagemagick().execute(&[
            input.path.to_str().unwrap(),
            "-background",
            "none",
            "-gravity",
            "center",
            "-extent",
            &extent,
            output_path.to_str().unwrap(),
        ])?;

        Ok(StageOutput {
            path: output_path,
            metadata: input.metadata,
        })
    }
}

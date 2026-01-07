//! Background removal stage using bgone.

use kat_core::Result;
use kat_pipeline::external::{bgone, ExternalTool};
use kat_pipeline::{Stage, StageInput, StageOutput};
use std::path::Path;

pub struct RemoveBackground;

impl Stage for RemoveBackground {
    fn name(&self) -> &str {
        "Remove Background"
    }

    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![bgone()]
    }

    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput> {
        let output_path = temp_dir.join(format!(
            "{}_nobg.png",
            input
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        bgone().execute(&[
            input.path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "--trim",
        ])?;

        Ok(StageOutput {
            path: output_path,
            metadata: input.metadata,
        })
    }
}

//! SF Symbol packaging stage using swiftdraw.

use kat_core::Result;
use kat_pipeline::external::{swiftdraw, ExternalTool};
use kat_pipeline::{Stage, StageInput, StageOutput};
use std::path::Path;

pub struct PackageSfSymbol;

impl Stage for PackageSfSymbol {
    fn name(&self) -> &str {
        "Package SF Symbol"
    }

    fn required_tools(&self) -> Vec<ExternalTool> {
        vec![swiftdraw()]
    }

    fn process(&self, input: StageInput, temp_dir: &Path) -> Result<StageOutput> {
        let output_path = temp_dir.join(format!(
            "{}.svg",
            input
                .source_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
        ));

        swiftdraw().execute(&[
            input.path.to_str().unwrap(),
            "--format",
            "sfsymbol",
            "--output",
            output_path.to_str().unwrap(),
        ])?;

        Ok(StageOutput {
            path: output_path,
            metadata: input.metadata,
        })
    }
}

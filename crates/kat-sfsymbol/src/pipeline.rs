//! SF Symbol pipeline factory.

use crate::stages::*;
use kat_pipeline::Pipeline;

/// Create SF Symbol pipeline for images with transparent background.
pub fn sfsymbol_pipeline(canvas_size: u32) -> Pipeline {
    Pipeline::new()
        .add(Normalize::new(canvas_size))
        .add(Vectorize)
        .add(OptimizeSvg)
        .add(PackageSfSymbol)
}

/// Create SF Symbol pipeline for images that need background removal first.
pub fn sfsymbol_pipeline_with_bg_removal(canvas_size: u32) -> Pipeline {
    Pipeline::new()
        .add(RemoveBackground)
        .add(Normalize::new(canvas_size))
        .add(Vectorize)
        .add(OptimizeSvg)
        .add(PackageSfSymbol)
}

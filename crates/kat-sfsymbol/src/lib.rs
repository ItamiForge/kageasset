//! kat-sfsymbol: SF Symbol generation pipeline for Kat.

pub mod pipeline;
pub mod stages;

pub use pipeline::{sfsymbol_pipeline, sfsymbol_pipeline_with_bg_removal};

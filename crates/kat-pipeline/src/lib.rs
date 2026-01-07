//! kat-pipeline: Pipeline execution engine for Kat.
//!
//! This crate provides the Stage trait and Pipeline orchestrator for
//! composable image processing pipelines.

pub mod external;
pub mod pipeline;
pub mod stage;

pub use external::ExternalTool;
pub use pipeline::Pipeline;
pub use stage::{Stage, StageInput, StageOutput};

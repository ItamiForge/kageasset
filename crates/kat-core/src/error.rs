//! Error types for the Kat workspace.

use thiserror::Error;

/// All error types across the Kat workspace.
#[derive(Error, Debug)]
pub enum KatError {
    // IO and filesystem errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // External tool errors
    #[error("External tool '{tool}' not found. Install with: {install_hint}")]
    ToolNotFound { tool: String, install_hint: String },

    #[error("External tool '{tool}' failed (exit {code}): {stderr}")]
    ToolFailed { tool: String, code: i32, stderr: String },

    #[error("External tool '{tool}' timed out after {seconds}s")]
    ToolTimeout { tool: String, seconds: u64 },

    // Pipeline errors
    #[error("Pipeline stage '{stage}' failed: {message}")]
    StageFailed { stage: String, message: String },

    #[error("Pipeline validation failed: {0}")]
    PipelineValidation(String),

    // Input validation
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    // Configuration
    #[error("Configuration error: {0}")]
    Config(String),

    // Catch-all
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias using KatError.
pub type Result<T> = std::result::Result<T, KatError>;

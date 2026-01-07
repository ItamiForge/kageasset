//! kat-core: Shared types, errors, and utilities for the Kat workspace.

mod asset;
mod error;

pub use asset::{Asset, ImageInfo};
pub use error::{KatError, Result};

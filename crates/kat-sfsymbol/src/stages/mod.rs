//! SF Symbol pipeline stages.

mod normalize;
mod optimize_svg;
mod package_sfsymbol;
mod remove_background;
mod vectorize;

pub use normalize::Normalize;
pub use optimize_svg::OptimizeSvg;
pub use package_sfsymbol::PackageSfSymbol;
pub use remove_background::RemoveBackground;
pub use vectorize::Vectorize;

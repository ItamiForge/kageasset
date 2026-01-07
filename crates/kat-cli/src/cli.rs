use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueHint};

/// Command line interface definition for Kat.
#[derive(Debug, Parser)]
#[command(
    version,
    about = "kat — fast asset inventory CLI",
    long_about = "Kat (kat) is a compact command-line tool that inventories image assets, captures metadata, and surfaces duplicates.",
    before_help = r"
   /\_/\  
  ( o.o )  kat v0.1.0
   > ^ <   asset inventory
"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output.
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan directories and emit an asset report.
    Scan(ScanArgs),
    /// Inspect a single asset file.
    Info(InfoArgs),
    /// Report on duplicate assets.
    Duplicates(DuplicateArgs),
    /// Configuration helpers.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Convert images to SF Symbols.
    Sfsymbol(SfsymbolArgs),
    /// Check external tool dependencies.
    Doctor,
}

#[derive(Debug, Args)]
pub struct SfsymbolArgs {
    /// Input file or directory.
    #[arg(required = true, value_hint = ValueHint::AnyPath)]
    pub input: PathBuf,

    /// Output directory for generated symbols. Defaults to the input file's folder.
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    pub output: Option<PathBuf>,

    /// Canvas size for normalization (square).
    #[arg(long, default_value = "1000")]
    pub size: u32,

    /// Remove background before processing (for images without transparency).
    #[arg(long)]
    pub remove_bg: bool,

    /// Keep temporary files for debugging.
    #[arg(long)]
    pub keep_temp: bool,

    /// Custom temporary directory.
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub temp_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ScanOptions {
    /// One or more paths to scan. Defaults to current directory.
    #[arg(value_hint = ValueHint::DirPath, num_args = 0.., default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Limit scanning to specific file extensions (case-insensitive).
    #[arg(short = 'e', long = "extensions", value_delimiter = ',', value_hint = ValueHint::Other)]
    pub extensions: Vec<String>,

    /// Glob patterns to ignore.
    #[arg(short, long, value_delimiter = ',', value_hint = ValueHint::AnyPath)]
    pub ignore: Vec<String>,

    /// Optional path to a custom configuration file.
    #[arg(long = "config", value_hint = ValueHint::FilePath)]
    pub config_path: Option<PathBuf>,

    /// Disable built-in default ignore patterns.
    #[arg(long)]
    pub no_default_ignores: bool,
}

#[derive(Debug, Args)]
pub struct ScanArgs {
    #[command(flatten)]
    pub options: ScanOptions,

    /// Write JSON output alongside Markdown.
    #[arg(long, default_value_t = true)]
    pub json: bool,

    /// Output path for the Markdown report.
    #[arg(short, long, value_hint = ValueHint::FilePath, default_value = "assets.md")]
    pub output: PathBuf,

    /// Perform a deep metadata scan (EXIF, XMP, ICC, etc.).
    #[arg(long, default_value_t = true)]
    pub meta: bool,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Path to the asset to inspect.
    #[arg(value_hint = ValueHint::FilePath)]
    pub path: PathBuf,

    /// Include extended metadata in the inspection output.
    #[arg(long)]
    pub meta: bool,
}

#[derive(Debug, Args)]
pub struct DuplicateArgs {
    #[command(flatten)]
    pub options: ScanOptions,

    /// Optional JSON output for duplicate report.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show the resolved configuration and defaults.
    Show(ConfigShowArgs),
    /// Create a configuration file with optional overrides.
    Init(ConfigInitArgs),
}

#[derive(Debug, Args)]
pub struct ConfigShowArgs {
    /// Optional path to the configuration file to inspect.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ConfigInitArgs {
    /// Output path for the generated configuration file. Defaults to ./.kat.toml
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Additional ignore globs to seed in the config file.
    #[arg(long, value_delimiter = ',', value_hint = ValueHint::AnyPath)]
    pub ignore: Vec<String>,

    /// Override the default extension list.
    #[arg(long, value_delimiter = ',', value_hint = ValueHint::Other)]
    pub extensions: Vec<String>,

    /// Do not include built-in default ignore patterns in the generated config.
    #[arg(long)]
    pub no_default_ignores: bool,

    /// Overwrite the configuration file if it already exists.
    #[arg(long)]
    pub force: bool,
}

mod cli;
mod commands;
mod config;
mod metadata;
mod model;
mod report;
mod scanner;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use commands::{run_config, run_duplicates, run_info, run_scan};

fn main() -> Result<()> {
    // Print banner
    eprintln!("{}", r"
   /\_/\  
  ( o.o )  kat v0.1.0
   > ^ <   asset inventory
");

    let cli = Cli::parse();

    match cli.command {
        Command::Scan(args) => run_scan(args)?,
        Command::Info(args) => run_info(args)?,
        Command::Duplicates(args) => run_duplicates(args)?,
        Command::Config { command } => run_config(command)?,
    }

    Ok(())
}

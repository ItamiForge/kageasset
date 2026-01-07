mod cli;
mod commands;
mod config;
mod report;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use commands::{run_config, run_doctor, run_duplicates, run_info, run_scan, run_sfsymbol};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Print banner unless quiet mode
    if !cli.quiet {
        eprintln!(
            "{}",
            r#"
                                      /\
             ,-.       _,---._ __    //\\
             /` )    .-'       `./  //  \\
            (  (   ,'            ` //    /|
             \  `-"      '      \' \\   / |
              `.              ,  \  \\ /  |
               /`.          ,'-`------Y   |
              (       .    ;          |   |
              |  ,-.    ,-'           |  /
              |  ) (   (  kat v0.1.0  | /
             (  (   \  `._____________|/
              `--'   `--'
"#
        );
    }

    match cli.command {
        Command::Scan(args) => run_scan(args)?,
        Command::Info(args) => run_info(args)?,
        Command::Duplicates(args) => run_duplicates(args)?,
        Command::Config { command } => run_config(command)?,
        Command::Sfsymbol(args) => run_sfsymbol(args, cli.quiet)?,
        Command::Doctor => run_doctor()?,
    }

    Ok(())
}

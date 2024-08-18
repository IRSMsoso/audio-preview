use crate::cli::Cli;
use crate::interactive_mode::run_interactive_mode;
use crate::single_mode::run_single_mode;
use clap::Parser;

mod cli;
mod interactive_mode;
mod single_mode;

fn main() {
    let cli = Cli::parse();

    match cli.path {
        Some(path) => run_single_mode(&path, cli.should_loop),
        None => run_interactive_mode(),
    }
}

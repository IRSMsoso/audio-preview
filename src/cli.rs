use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Path of file to play
    pub(crate) path: Option<String>,

    /// Whether to loop
    #[arg(short='l',long=None, default_value_t = false)]
    pub(crate) should_loop: bool,
}

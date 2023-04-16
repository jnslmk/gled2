use clap::Parser;
use once_cell::sync::Lazy;
use std::path::PathBuf;

pub static OPTS: Lazy<Opts> = Lazy::new(Opts::parse);

#[derive(Parser, Debug)]
#[command(author, version, name = "gled")]
pub struct Opts {
    /// Project file to open
    #[arg(name = "PROJECT")]
    pub project_path: Option<PathBuf>,

    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

//! CLI parsing utilities for gathering input.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the person to greet
    #[arg(short, long = "entries-file-path")]
    pub entries_file_path: String,
}

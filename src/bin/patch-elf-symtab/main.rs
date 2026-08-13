//! ELF symbol table patching utility.

mod cli;
mod entries;

use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::Parser;

use cli::Args;
use entries::Entries;
use patch_elf_symtab::{ElfSymtabPatcher, PatchFailureReason};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

/// Main run function
fn run() -> Result<()> {
    let args = Args::parse();

    let entries_json = std::fs::read_to_string(&args.entries_file_path)
        .with_context(|| format!("failed to read entries file {:?}", args.entries_file_path))?;
    let entries: Entries = serde_json::from_str(&entries_json)
        .with_context(|| format!("failed to parse entries JSON {:?}", args.entries_file_path))?;

    let mut input_buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input_buffer)
        .context("failed to read ELF data from stdin")?;

    let mut patcher = ElfSymtabPatcher::new(&input_buffer)?;
    for (name, patch) in entries.0.iter() {
        patcher.patch_symbol(name, patch.0.as_slice());
    }

    let (output, failures) = patcher.finish();

    // Preserve prior CLI semantics: patch problems are fatal; unknown keys are reported
    // only when every named patch that matched a symbol succeeded.
    let mut unknown_keys = Vec::new();
    for failure in &failures {
        match failure.reason {
            PatchFailureReason::NotFound => unknown_keys.push(failure.symbol_name.clone()),
            _ => anyhow::bail!("{failure}"),
        }
    }
    if !unknown_keys.is_empty() {
        unknown_keys.sort();
        anyhow::bail!(
            "unknown JSON key(s) do not match any object symbol in the ELF: {:?}",
            unknown_keys
        );
    }

    std::io::stdout().write_all(&output)?;
    Ok(())
}

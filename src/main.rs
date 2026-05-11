//! ELF symbol table patching utility.

mod cli;
mod entries;

use std::io::Read;

use anyhow::{Context, Result};
use clap::Parser;

use cli::Args;
use entries::{Entries, HexBytes};

use elf::ElfBytes;
use elf::endian::AnyEndian;
use elf::symbol::Symbol;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn patch_symbol(sym: Symbol, patch: &HexBytes) -> Result<()> {
    Ok(())
}

/// Main run function
fn run() -> Result<()> {
    let args = Args::parse();

    // Parse CLI arg
    let entries_json = std::fs::read_to_string(&args.entries_file_path)
        .with_context(|| format!("failed to read entries file {:?}", args.entries_file_path))?;
    let entries: Entries = serde_json::from_str(&entries_json)
        .with_context(|| format!("failed to parse entries JSON {:?}", args.entries_file_path))?;

    // Create the buffer for the ELF file
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .context("failed to read ELF data from stdin")?;

    // Read ELF
    let elf = ElfBytes::<AnyEndian>::minimal_parse(&buffer).context("failed to parse ELF data")?;
    let common_sections = elf
        .find_common_data()
        .context("unable to find common ELF data -- likely malformed ELF file")?;
    let symtab_strs = common_sections
        .symtab_strs
        .context("no .symtab_strs section")?;
    let symtab = common_sections.symtab.context("no .symtab section")?;

    for (sym_idx, sym) in symtab.iter().enumerate() {
        if sym.st_symtype() != elf::abi::STT_OBJECT {
            continue;
        }

        let name: &str = match symtab_strs.get(sym.st_name as usize) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let patch: &HexBytes = match (entries.get(name)) {
            Some(result) => result,
            None => continue,
        };

        patch_symbol(sym, patch);

        // let data = symbol_defined_bytes(&elf, &sym).context("resolve symbol bytes in file")?;
        // let _patch: Option<&HexBytes> = entries.get(name);

        // match data {
        //     Some(bytes) => eprintln!("{sym_idx}\t{name}\t{}", hex::encode(bytes)),
        //     None => eprintln!("{sym_idx}\t{name}"),
        // }
    }

    Ok(())
}

//! ELF symbol table patching utility.

mod cli;
mod entries;

use std::io::Read;

use anyhow::{Context, Result};
use clap::Parser;

use cli::Args;
use entries::{Entries, HexBytes};

use elf::ElfBytes;
use elf::abi::{ET_DYN, ET_EXEC, PT_LOAD, SHN_UNDEF};
use elf::endian::AnyEndian;
use elf::symbol::Symbol;
use std::io::Write;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

/// Resolve the `[start, end)` range in the file that may hold `sym`'s bytes (`end` is exclusive).
fn symbol_data_file_range(elf: &ElfBytes<AnyEndian>, sym: Symbol) -> Result<(usize, usize)> {
    if sym.st_shndx == SHN_UNDEF {
        anyhow::bail!("cannot patch undefined symbol");
    }

    match elf.ehdr.e_type {
        // ET_EXEC: classic non-PIE executable. ET_DYN: PIE executable or shared object — symbol
        // virtual addresses still map to file offsets through PT_LOAD the same way.
        ET_EXEC | ET_DYN => {
            let virtual_address = sym.st_value;
            let segments = elf.segments().ok_or_else(|| {
                anyhow::anyhow!("executable/shared object has no program headers")
            })?;
            for phdr in segments.iter() {
                if phdr.p_type != PT_LOAD {
                    continue;
                }
                if virtual_address < phdr.p_vaddr {
                    continue;
                }
                let seg_end_va = phdr.p_vaddr.saturating_add(phdr.p_memsz);
                if virtual_address >= seg_end_va {
                    continue;
                }
                let delta = virtual_address - phdr.p_vaddr;
                let file_offset = phdr
                    .p_offset
                    .checked_add(delta)
                    .ok_or_else(|| anyhow::anyhow!("file offset overflow"))?;
                let file_end = phdr
                    .p_offset
                    .checked_add(phdr.p_filesz)
                    .ok_or_else(|| anyhow::anyhow!("segment file range overflow"))?;
                if file_offset >= file_end {
                    continue;
                }
                let start: usize = file_offset
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("symbol file offset does not fit in usize"))?;
                let container_end: usize = file_end
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("segment file end does not fit in usize"))?;
                return Ok((start, container_end));
            }
            anyhow::bail!(
                "virtual address {virtual_address:#x} is not mapped to a file-backed PT_LOAD region"
            );
        }
        other => anyhow::bail!("unsupported ELF type {other} (e_type)"),
    }
}

/// Overwrites the symbol's allocation in `buffer` with `patch` bytes.
///
/// `elf` must have been parsed from a **different** byte slice than `buffer` (e.g. a parse-only
/// copy) so this can borrow `buffer` mutably while the parser holds an immutable view.
fn patch_symbol(
    elf: &ElfBytes<AnyEndian>,
    buffer: &mut [u8],
    sym: Symbol,
    patch: &HexBytes,
) -> Result<()> {
    let payload = patch.0.as_slice();
    if payload.is_empty() {
        return Ok(());
    }

    if sym.st_size > 0 && payload.len() as u64 > sym.st_size {
        anyhow::bail!(
            "patch is {} bytes but symbol size is {}",
            payload.len(),
            sym.st_size
        );
    }

    let (start, container_end) =
        symbol_data_file_range(elf, sym).context("resolve symbol offset in file")?;

    let end = start
        .checked_add(payload.len())
        .ok_or_else(|| anyhow::anyhow!("patch end overflow"))?;
    if end > container_end {
        anyhow::bail!(
            "patch [{start}, {end}) extends past symbol container [{start}, {container_end})"
        );
    }
    if end > buffer.len() {
        anyhow::bail!(
            "patch [{start}, {end}) extends past buffer length {}",
            buffer.len()
        );
    }

    buffer[start..end].copy_from_slice(payload);
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
    let mut input_buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input_buffer)
        .context("failed to read ELF data from stdin")?;
    let mut output_buffer = input_buffer.clone();

    let elf =
        ElfBytes::<AnyEndian>::minimal_parse(&input_buffer).context("failed to parse ELF data")?;
    let common_sections = elf
        .find_common_data()
        .context("unable to find common ELF data -- likely malformed ELF file")?;
    let symtab_strs = common_sections
        .symtab_strs
        .context("no .symtab_strs section")?;
    let symtab = common_sections.symtab.context("no .symtab section")?;

    for sym in symtab.iter() {
        if sym.st_symtype() != elf::abi::STT_OBJECT {
            continue;
        }

        let name: &str = match symtab_strs.get(sym.st_name as usize) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let patch: &HexBytes = match entries.get(name) {
            Some(result) => result,
            None => continue,
        };

        patch_symbol(&elf, &mut output_buffer, sym, patch)
            .with_context(|| format!("failed to patch symbol {:?}", name))?;
    }

    std::io::stdout().write_all(&output_buffer)?;

    Ok(())
}

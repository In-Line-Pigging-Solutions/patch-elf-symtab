//! ELF symbol table patching library.
//!
//! Parse an ELF once with [`ElfSymtabPatcher::new`], apply one or more
//! [`ElfSymtabPatcher::patch_symbol`] updates (failures are recorded, not fatal),
//! then take the result with [`ElfSymtabPatcher::finish`].

use std::collections::HashMap;
use std::fmt;

use anyhow::{Context, Result};
use elf::ElfBytes;
use elf::abi::{ET_DYN, ET_EXEC, PT_LOAD, SHN_UNDEF, STT_OBJECT};
use elf::endian::AnyEndian;
use elf::symbol::Symbol;

/// Why a [`patch_symbol`](ElfSymtabPatcher::patch_symbol) attempt did not apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchFailureReason {
    /// No defined `STT_OBJECT` symbol with that name was present at construction.
    NotFound,
    /// Payload is longer than the symbol's non-zero `st_size`.
    TooLarge { patch_len: usize, symbol_size: u64 },
    /// Payload would write past the file-backed PT_LOAD container for the symbol.
    ExtendsPastContainer {
        start: usize,
        end: usize,
        container_end: usize,
    },
    /// Payload would write past the end of the working buffer.
    ExtendsPastBuffer {
        start: usize,
        end: usize,
        buffer_len: usize,
    },
    /// Arithmetic overflow computing the patch end offset.
    EndOverflow { start: usize, patch_len: usize },
}

impl fmt::Display for PatchFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no object symbol with that name"),
            Self::TooLarge {
                patch_len,
                symbol_size,
            } => write!(
                f,
                "patch is {patch_len} bytes but symbol size is {symbol_size}"
            ),
            Self::ExtendsPastContainer {
                start,
                end,
                container_end,
            } => write!(
                f,
                "patch [{start}, {end}) extends past symbol container [{start}, {container_end})"
            ),
            Self::ExtendsPastBuffer {
                start,
                end,
                buffer_len,
            } => write!(
                f,
                "patch [{start}, {end}) extends past buffer length {buffer_len}"
            ),
            Self::EndOverflow { start, patch_len } => {
                write!(f, "patch end overflow at start {start} len {patch_len}")
            }
        }
    }
}

/// A failed patch attempt, with the symbol name and reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchFailure {
    pub symbol_name: String,
    pub reason: PatchFailureReason,
}

impl fmt::Display for PatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to patch symbol {:?}: {}",
            self.symbol_name, self.reason
        )
    }
}

/// File-backed range where a defined object symbol's bytes may be written.
#[derive(Clone, Debug)]
struct PatchSite {
    /// Inclusive start offset in the ELF file / working buffer.
    start: usize,
    /// Exclusive end of the file-backed PT_LOAD container for this symbol.
    container_end: usize,
    /// Symbol `st_size` (0 means size is unknown / unconstrained by st_size).
    st_size: u64,
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

fn apply_patch(
    buffer: &mut [u8],
    site: &PatchSite,
    payload: &[u8],
) -> Result<(), PatchFailureReason> {
    if payload.is_empty() {
        return Ok(());
    }

    if site.st_size > 0 && payload.len() as u64 > site.st_size {
        return Err(PatchFailureReason::TooLarge {
            patch_len: payload.len(),
            symbol_size: site.st_size,
        });
    }

    let end = site
        .start
        .checked_add(payload.len())
        .ok_or(PatchFailureReason::EndOverflow {
            start: site.start,
            patch_len: payload.len(),
        })?;
    if end > site.container_end {
        return Err(PatchFailureReason::ExtendsPastContainer {
            start: site.start,
            end,
            container_end: site.container_end,
        });
    }
    if end > buffer.len() {
        return Err(PatchFailureReason::ExtendsPastBuffer {
            start: site.start,
            end,
            buffer_len: buffer.len(),
        });
    }

    buffer[site.start..end].copy_from_slice(payload);
    Ok(())
}

/// Owns a working copy of an ELF image and pre-resolved object-symbol patch sites.
///
/// Construct with [`ElfSymtabPatcher::new`] (parses once), apply updates with
/// [`patch_symbol`](Self::patch_symbol) (failures are recorded, not fatal), then take the
/// bytes and failure list with [`finish`](Self::finish).
pub struct ElfSymtabPatcher {
    buffer: Vec<u8>,
    sites: HashMap<String, Vec<PatchSite>>,
    failures: Vec<PatchFailure>,
}

impl ElfSymtabPatcher {
    /// Parse `input` once and build a patcher over a cloned working buffer.
    pub fn new(input: &[u8]) -> Result<Self> {
        let elf =
            ElfBytes::<AnyEndian>::minimal_parse(input).context("failed to parse ELF data")?;
        let common_sections = elf
            .find_common_data()
            .context("unable to find common ELF data -- likely malformed ELF file")?;
        let symtab_strs = common_sections
            .symtab_strs
            .context("no .symtab_strs section")?;
        let symtab = common_sections.symtab.context("no .symtab section")?;

        let mut sites: HashMap<String, Vec<PatchSite>> = HashMap::new();
        for sym in symtab.iter() {
            if sym.st_symtype() != STT_OBJECT {
                continue;
            }
            if sym.st_shndx == SHN_UNDEF {
                continue;
            }

            let name: &str = match symtab_strs.get(sym.st_name as usize) {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Only file-backed object symbols are patchable (e.g. skip BSS).
            let st_size = sym.st_size;
            let Ok((start, container_end)) = symbol_data_file_range(&elf, sym) else {
                continue;
            };

            sites.entry(name.to_string()).or_default().push(PatchSite {
                start,
                container_end,
                st_size,
            });
        }

        Ok(Self {
            buffer: input.to_vec(),
            sites,
            failures: Vec::new(),
        })
    }

    /// Attempt to overwrite the named object symbol's bytes with `replacement`.
    ///
    /// Failures (missing symbol, payload too large, out of range, etc.) are **non-fatal**:
    /// they are appended to the failure list (see [`failures`](Self::failures) /
    /// [`finish`](Self::finish)) and chaining continues. Prior successful patches remain.
    pub fn patch_symbol(&mut self, symbol_name: &str, replacement: &[u8]) -> &mut Self {
        let Some(sites) = self.sites.get(symbol_name).cloned() else {
            self.failures.push(PatchFailure {
                symbol_name: symbol_name.to_string(),
                reason: PatchFailureReason::NotFound,
            });
            return self;
        };

        for site in &sites {
            if let Err(reason) = apply_patch(&mut self.buffer, site, replacement) {
                self.failures.push(PatchFailure {
                    symbol_name: symbol_name.to_string(),
                    reason,
                });
                // Stop applying this symbol's remaining duplicate sites on first failure.
                break;
            }
        }
        self
    }

    /// Failures recorded so far from [`patch_symbol`](Self::patch_symbol).
    pub fn failures(&self) -> &[PatchFailure] {
        &self.failures
    }

    /// Consume the patcher and return the (possibly partially) patched ELF bytes plus failures.
    pub fn finish(self) -> (Vec<u8>, Vec<PatchFailure>) {
        (self.buffer, self.failures)
    }
}

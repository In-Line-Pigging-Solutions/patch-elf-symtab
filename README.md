# patch-elf-symtab

CLI utility that overwrites **defined object symbols** in an ELF **executable** (`ET_EXEC`) by writing new bytes into the file-backed region that backs each symbol. The typical motivator is firmware or embedded images that embed blobs (for example certificates) as globals in `.symtab`, where you want to substitute bytes without rebuilding the whole toolchain.

The tool reads the ELF from **stdin**, writes the patched ELF to **stdout**, and takes a JSON **entries** file that maps symbol names to hex-encoded payloads.

## Behavior and limits

- Only **`ET_EXEC`** binaries are supported today (not shared objects / relocatables).
- Only symbols with type **`STT_OBJECT`** are considered; others are skipped.
- Patches are keyed by the symbol name as it appears in the symbol table.
- Each patch must fit within the symbol’s `st_size` when it is non-zero.
- The input ELF must contain a **`.symtab`** section (and associated string table).

## Entries file

The entries file is JSON: an object whose keys are symbol names and whose values are **hex strings** (two hex digits per byte). Serialization uses lowercase hex; deserialization accepts mixed case and an optional `0x` / `0X` prefix.

Example:

```json
{
  "greeting": "70617463686564206f6b210000000000"
}
```

That payload is raw bytes for the ASCII string `patched ok!` followed by NUL padding—matching a 16-byte symbol in the bundled test fixture.

## Usage

```console
patch-elf-symtab --entries-file-path entries.json < input.elf > output.elf
```

Short flag: `-e` / `--entries-file-path`.

## Installation

### Nix (flake)

Build the package:

```console
nix build .#patch-elf-symtab
```

Resulting binary: `./result/bin/patch-elf-symtab`.

Development shell (Rust toolchain, `pre-commit`, and so on):

```console
nix develop
```

This flake currently declares **`x86_64-linux`** only.

### Overlay

Expose the package as `pkgs.patch-elf-symtab` in another flake:

```nix
{
  inputs.patch-elf-symtab.url = "github:<owner>/patch-elf-symtab";

  outputs = { self, nixpkgs, patch-elf-symtab, ... }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ patch-elf-symtab.overlays.default ];
      };
    in
    {
      # pkgs.patch-elf-symtab
    };
}
```

## Testing

Integration checks live under `testsuites/` (Nix derivations). Run the full flake checks with:

```console
nix flake check
```

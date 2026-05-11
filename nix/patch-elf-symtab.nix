{ rustPlatform
, src
,
}:

rustPlatform.buildRustPackage {
  pname = "patch-elf-symtab";
  version = "0.1.0";
  inherit src;

  cargoLock = {
    lockFile = src + "/Cargo.lock";
  };

  meta = {
    description = "ELF symbol table patching utility";
    mainProgram = "patch-elf-symtab";
  };
}

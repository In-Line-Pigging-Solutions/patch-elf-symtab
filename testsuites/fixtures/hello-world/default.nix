{ stdenv, lib }:

stdenv.mkDerivation {
  pname = "hello-world-fixture";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  # ET_EXEC is required: patch-elf-symtab only handles classic executables (not PIE / ET_DYN).
  dontStrip = true;

  buildPhase = ''
    runHook preBuild
    $CC -fno-pie -no-pie -Wall -Wextra -O2 -o hello-world main.c
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/bin"
    cp hello-world "$out/bin/hello-world"
    runHook postInstall
  '';

  meta = {
    description = "Minimal ELF fixture: prints a volatile global char[16]";
    mainProgram = "hello-world";
  };
}

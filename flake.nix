{
  description = "patch-elf-symtab";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=25.11";
    flake-parts.url = "github:hercules-ci/flake-parts";

    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    git-hooks-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ ./nix/build.nix ];
      systems = [ "x86_64-linux" ];

  perSystem = { config, pkgs, ... }:
    let
      pname = "patch-elf-symtab";
      src = pkgs.lib.cleanSource ../.;
    in
    {
      packages.${pname} = pkgs.rustPlatform.buildRustPackage {
        pname = pname;
        version = "0.1.0";
        inherit src;

        cargoLock = {
          lockFile = src + "/Cargo.lock";
        };

        meta = {
          description = "ELF symbol table patching utility";
          mainProgram = pname;
        };
      };

      devShells.default = pkgs.mkShell {
        packages = [
          pkgs.rustfmt
          pkgs.clippy
          pkgs.rustc
          pkgs.cargo
          pkgs.rust-analyzer
        ];
        inputsFrom = [ config.packages.${pname} ];
        env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        shellHook = ''
          ${config.pre-commit.installationScript}
        '';
      };
    };
    };
}

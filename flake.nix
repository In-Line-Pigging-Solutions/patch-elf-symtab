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
      imports = [ ./nix/build.nix ./testsuites/build.nix ];
      systems = [ "x86_64-linux" ];

      flake.overlays.default = final: _prev: {
        patch-elf-symtab = final.callPackage ./nix/patch-elf-symtab.nix {
          src = final.lib.cleanSource inputs.self;
        };
      };

      perSystem = { config, pkgs, ... }:
        let
          pname = "patch-elf-symtab";
          src = pkgs.lib.cleanSource ./.;
        in
        {
          packages.${pname} = pkgs.callPackage ./nix/patch-elf-symtab.nix { inherit src; };

          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.pre-commit
              pkgs.rustfmt
              pkgs.clippy
              pkgs.rustc
              pkgs.cargo
              pkgs.rust-analyzer
              pkgs.ctags
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

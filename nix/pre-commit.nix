# Git hooks (aligned with RAPTOR_firmware patterns).
{ inputs, ... }:
{
  imports = [ inputs.git-hooks-nix.flakeModule ];

  perSystem = { config, pkgs, ... }: {
    pre-commit.settings.hooks = {
      nixpkgs-fmt.enable = true;

      typos.enable = true;
      typos.settings.configPath = builtins.toString ../.typos.toml;

      rustfmt-monorepo = {
        enable = true;
        name = "rustfmt";
        description = "Format Rust code.";
        entry = "${config.pre-commit.settings.hooks.rustfmt.package}/bin/rustfmt --color always --edition 2024";
        files = "\\.rs$";
      };
    };
  };
}

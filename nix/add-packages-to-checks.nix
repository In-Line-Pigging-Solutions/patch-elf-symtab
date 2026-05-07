# Copy `.packages` into `.checks` so `nix flake check` builds every package.
{ inputs, ... }:
{
  imports = [ inputs.git-hooks-nix.flakeModule ];

  perSystem = { config, ... }: {
    checks = config.packages;
  };
}

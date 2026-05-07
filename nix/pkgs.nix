# Plain nixpkgs (no firmware overlays).
{ inputs, ... }:
{
  perSystem = { system, ... }: {
    _module.args.pkgs = import inputs.nixpkgs {
      inherit system;
      config = { };
    };
  };
}

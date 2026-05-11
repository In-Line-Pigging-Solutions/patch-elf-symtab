{ ... }:
{
  imports = [ ./integration/build.nix ];

  perSystem = { pkgs, ... }: {
    packages.hello-world-fixture = pkgs.callPackage ./fixtures/hello-world { };
  };
}

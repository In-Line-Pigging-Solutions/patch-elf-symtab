{ ... }:
{
  perSystem = { config, pkgs, ... }:
    let
      # Repo copy of patch map: symbol `greeting` → "patched ok!\0" + zero padding (16 bytes).
      entriesJson = ./entries.json;
    in
    {
      checks.integration = pkgs.runCommand "patch-elf-symtab-integration"
        {
          nativeBuildInputs = [ config.packages.patch-elf-symtab ];
          inherit entriesJson;
        }
        ''
          set -euo pipefail

          patch_elf_symtab=${config.packages.patch-elf-symtab}/bin/patch-elf-symtab
          hello_world=${config.packages.hello-world-fixture}/bin/hello-world

          "$patch_elf_symtab" --entries-file-path "$entriesJson" < "$hello_world" > patched
          chmod +x patched

          ./patched >actual.out
          # Fixture uses `printf("%s", greeting)` (no trailing newline from format).
          if ! printf '%s' 'patched ok!' | cmp -s - actual.out; then
            echo "integration: patched binary stdout mismatch" >&2
            printf 'expected (hex): '
            printf '%s' 'patched ok!' | od -An -tx1 | tr -d '\n'
            printf '\nactual   (hex): '
            od -An -tx1 actual.out | tr -d '\n'
            printf '\n' >&2
            exit 1
          fi

          mkdir -p "$out"
          echo ok >"$out"/success
        '';
    };
}

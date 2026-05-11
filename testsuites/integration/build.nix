{ ... }:
{
  perSystem = { config, pkgs, ... }:
    {
      checks.integration-pass = pkgs.runCommand "patch-elf-symtab-integration-pass"
        {
          nativeBuildInputs = [ config.packages.patch-elf-symtab ];
          entriesJson = ./patched-entries.json;
        }
        ''
          set -euo pipefail

          hello_world=${config.packages.hello-world-fixture}/bin/hello-world

          patch-elf-symtab --entries-file-path "$entriesJson" < "$hello_world" > patched
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

      checks.integration-fail = pkgs.runCommand "patch-elf-symtab-integration-fail"
        {
          nativeBuildInputs = [ config.packages.patch-elf-symtab ];
          entriesJson = ./bad-patched-entries.json;
        }
        ''
          set -euo pipefail

          hello_world=${config.packages.hello-world-fixture}/bin/hello-world

          set +e
          stderr=$(patch-elf-symtab --entries-file-path "$entriesJson" < "$hello_world" 2>&1)
          status=$?
          set -e

          if [[ "$status" -eq 0 ]]; then
            echo "integration-fail: expected patch-elf-symtab to exit non-zero" >&2
            exit 1
          fi

          expected='error: failed to patch symbol "greeting": patch is 37 bytes but symbol size is 16'
          if [[ "$stderr" != "$expected" ]]; then
            echo "integration-fail: unexpected stderr from patch-elf-symtab" >&2
            printf 'expected: %s\n' "$expected" >&2
            printf 'got:      %s\n' "$stderr" >&2
            exit 1
          fi

          mkdir -p "$out"
          echo ok >"$out"/success
        '';
    };
}

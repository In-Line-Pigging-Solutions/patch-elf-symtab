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

      checks.integration-identity = pkgs.runCommand "patch-elf-symtab-integration-identity"
        {
          nativeBuildInputs = [ config.packages.patch-elf-symtab ];
          entriesJson = ./identity-entries.json;
        }
        ''
          set -euo pipefail

          hello_world=${config.packages.hello-world-fixture}/bin/hello-world

          patch-elf-symtab --entries-file-path "$entriesJson" < "$hello_world" > patched

          if ! cmp -s "$hello_world" patched; then
            echo "integration-identity: patched binary differs from input (expected byte-identical identity patch)" >&2
            cmp -l "$hello_world" patched | head -n 20 >&2 || true
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

      checks.integration-unknown-json-key = pkgs.runCommand "patch-elf-symtab-integration-unknown-json-key"
        {
          nativeBuildInputs = [ config.packages.patch-elf-symtab ];
          entriesJson = ./entries-unknown-key.json;
        }
        ''
          set -euo pipefail

          hello_world=${config.packages.hello-world-fixture}/bin/hello-world

          set +e
          stderr=$(patch-elf-symtab --entries-file-path "$entriesJson" < "$hello_world" 2>&1)
          status=$?
          set -e

          if [[ "$status" -eq 0 ]]; then
            echo "integration-unknown-json-key: expected patch-elf-symtab to exit non-zero" >&2
            exit 1
          fi

          # JSON patches greeting and also names a key that is not an object symbol in the ELF.
          if [[ "$stderr" != *'unknown JSON key'* ]] || [[ "$stderr" != *'___not_a_symbol_in_this_elf___'* ]]; then
            echo "integration-unknown-json-key: unexpected stderr from patch-elf-symtab" >&2
            printf 'got: %s\n' "$stderr" >&2
            exit 1
          fi

          mkdir -p "$out"
          echo ok >"$out"/success
        '';
    };
}

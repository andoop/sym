#!/usr/bin/env bash
# 17：树解释器与 --vm 对同一 .sym 的 stdout 一致，并与 golden 对齐
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYM="${ROOT}/target/debug/sym"
CASE="${ROOT}/tests/conformance/cases/triple.sym"
GOLD="${ROOT}/tests/conformance/cases/triple.stdout.expected"
if [[ ! -x "$SYM" ]]; then
  echo "run: cargo build -p symc" >&2
  exit 1
fi
out_tree=$("$SYM" run "$CASE")
out_vm=$("$SYM" run --vm "$CASE")
diff -q "$GOLD" <(printf '%s\n' "$out_tree") >/dev/null || {
  echo "tree stdout != golden" >&2
  printf '%s\n' "$out_tree" | diff -u "$GOLD" - || true
  exit 1
}
if [[ "$out_tree" != "$out_vm" ]]; then
  echo "tree vs --vm stdout differ" >&2
  diff -u <(printf '%s\n' "$out_tree") <(printf '%s\n' "$out_vm") || true
  exit 1
fi
echo "conformance: triple.sym OK (tree == vm == golden)"

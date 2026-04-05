#!/usr/bin/env bash
# 树解释器与 --vm 对同一 .sym 的 stdout 一致，并与各用例 golden 对齐
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SYM="${ROOT}/target/debug/sym"
if [[ ! -x "$SYM" ]]; then
  echo "run: cargo build -p symc" >&2
  exit 1
fi

cd "$ROOT"

failures=0
for name in triple json_parity; do
  CASE="${ROOT}/tests/conformance/cases/${name}.sym"
  GOLD="${ROOT}/tests/conformance/cases/${name}.stdout.expected"
  out_tree=$("$SYM" run "$CASE")
  out_vm=$("$SYM" run --vm "$CASE")
  if ! diff -q "$GOLD" <(printf '%s\n' "$out_tree") >/dev/null; then
    echo "tree stdout != golden ($name)" >&2
    printf '%s\n' "$out_tree" | diff -u "$GOLD" - || true
    failures=$((failures + 1))
    continue
  fi
  if [[ "$out_tree" != "$out_vm" ]]; then
    echo "tree vs --vm stdout differ ($name)" >&2
    diff -u <(printf '%s\n' "$out_tree") <(printf '%s\n' "$out_vm") || true
    failures=$((failures + 1))
    continue
  fi
  echo "conformance: ${name}.sym OK (tree == vm == golden)"
done

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

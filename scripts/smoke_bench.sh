#!/usr/bin/env bash
# 19：简易计时（需已构建 release sym）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/target/release/sym"
SYM="${ROOT}/examples/vm_fib.sym"
if [[ ! -x "$BIN" ]]; then
  echo "run: cargo build -p symc --release" >&2
  exit 1
fi
for _ in $(seq 1 5); do
  /usr/bin/time -p "$BIN" run --vm "$SYM" >/dev/null
done

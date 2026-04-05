# Sym

[![CI](https://github.com/andoop/sym/actions/workflows/ci.yml/badge.svg)](https://github.com/andoop/sym/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust: 1.74+](https://img.shields.io/badge/rust-1.74%2B-orange.svg)](https://www.rust-lang.org)

**Sym** is a small programming language focused on **clear structure, low ambiguity, and tooling/LLM-friendly** code. This repository ships a **lexer, parser, type checker, tree-walking interpreter, and an optional stack bytecode VM**, exposed as the **`sym`** CLI (`crates/symc`).

[中文说明 / 完整中文文档索引 → README.zh.md](README.zh.md)

**Status:** early-stage / **experimental** — semantics and CLI may change; pin a git SHA for serious experiments.

## Features

- **Types**: `Int`, `Bool`, `String`, `Unit`, algebraic data types (`type` / `match`), prelude `Option[T]` and `List[T]`
- **Execution**: `sym run` (interpreter) or `sym run --vm` for a **compileable subset** (see [docs/spec/VM_SUBSET.md](docs/spec/VM_SUBSET.md))
- **Modules**: `import` with stitched sources and `# sym:file` markers for diagnostics
- **Built-ins**: I/O, strings, JSON helpers, HTTPS client (`http_post`, SSE fold), env/files/shell (see [docs/industrial/builtins-contracts.md](docs/industrial/builtins-contracts.md))
- **Diagnostics**: human-readable or **single-line JSON** (`--message-format json`) with logical file/line hints for stitched sources

## Requirements

- [Rust](https://rustup.rs/) (stable), **MSRV 1.74** (see workspace `Cargo.toml`)

## Build

```bash
cargo build --release
```

Binary: `target/release/sym`, or install the crate CLI:

```bash
cargo install --path crates/symc
```

## Quick start

```bash
# Typecheck (loads import chain + default prelude)
sym check examples/hello.sym

# Run entrypoint `main` (no arguments)
sym run examples/hello.sym

# Bytecode VM (subset only; unsupported programs error with `VM: …`)
sym run --vm examples/vm_fib.sym

# Multi-file example (library + entry)
sym run examples/call_lib.sym

# Lexer debug: tokens and byte spans
sym tokens examples/hello.sym

# Optional: no prelude; custom stdlib root for `import a.b` fallback
sym --no-prelude --stdlib ./stdlib check examples/hello.sym

# Machine-readable errors (CI / tooling)
sym check --message-format json path/to/file.sym
```

## Repository layout

| Path | Purpose |
|------|---------|
| `crates/symc/` | Compiler, library API, `sym` binary |
| `stdlib/` | Default prelude (`prelude.sym`) |
| `examples/` | Sample programs; see [examples/README.md](examples/README.md) |
| `examples/ccode/` | Agent-style demo (OpenAI-compatible / Anthropic); needs env vars (see `main.sym` header) |
| `tests/conformance/` | VM vs tree golden checks |
| `scripts/` | CI helpers (`conformance_vm_tree.sh`, `smoke_bench.sh`) |
| `docs/spec/` | Language & semantics notes (mostly **Chinese**; code is the source of truth) |
| `docs/industrial/` | Maturity checklist, contracts, CI notes |

## Documentation

- **English (this file)**: overview and links  
- **Chinese**: [README.zh.md](README.zh.md), [docs/README.md](docs/README.md)  
- **Developer setup**: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)  
- **Spec index**: [docs/spec/README.md](docs/spec/README.md)  
- **Specs (zh)**: [LANGUAGE.md](docs/spec/LANGUAGE.md), [SEMANTICS.md](docs/spec/SEMANTICS.md), [SYNTAX.md](docs/spec/SYNTAX.md), [SOUL.md](docs/spec/SOUL.md) (design intent)  
- **VM**: [VM_SUBSET.md](docs/spec/VM_SUBSET.md), [VM_TREE_PARITY.md](docs/spec/VM_TREE_PARITY.md)  
- **Roadmap (37-step index, zh)**: [docs/industrial/CHECKLIST.md](docs/industrial/CHECKLIST.md)

## Testing

```bash
cargo test -p symc
cargo fmt -p symc --all -- --check
cargo clippy -p symc --all-targets -- -D warnings
bash scripts/conformance_vm_tree.sh   # after `cargo build -p symc`
```

CI runs the above on **Ubuntu** and **macOS** (see [.github/workflows/ci.yml](.github/workflows/ci.yml)).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md), and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Support

See [SUPPORT.md](SUPPORT.md) for where to ask questions and what to include in bug reports.

## Security

See [SECURITY.md](SECURITY.md). Please **do not** file public issues for unfixed vulnerabilities.

## License

This project is licensed under the **MIT License** — see [LICENSE](LICENSE).

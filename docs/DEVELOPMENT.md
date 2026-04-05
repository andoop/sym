# Development guide

How to work on **Sym** / **`symc`** from a checkout.

## Prerequisites

- **Rust** stable, **MSRV 1.74** (see root `Cargo.toml`).
- On macOS/Linux, usual build tools for Rust are enough.

## Clone and build

```bash
git clone https://github.com/andoop/sym.git
cd sym
cargo build -p symc
cargo run -p symc -- --help
```

Release binary:

```bash
cargo build -p symc --release
./target/release/sym --help
```

## Running tests and checks (same as CI)

```bash
cargo fmt -p symc --all -- --check
cargo clippy -p symc --all-targets -- -D warnings
cargo test --workspace
cargo build -p symc
bash scripts/conformance_vm_tree.sh
```

## Crate layout (`crates/symc/src`)

| Module (approx.) | Role |
|------------------|------|
| `lexer.rs` | Tokenization |
| `parser.rs` | AST |
| `ast.rs` | AST types |
| `typeck.rs` | Type checking |
| `interp.rs` | Tree interpreter |
| `bytecode.rs` / `vm.rs` | VM compile + run |
| `load.rs` | Multi-file stitch + prelude |
| `sourcemap.rs` | Logical file/line for diagnostics |
| `main.rs` | CLI (`clap`) |
| `lib.rs` | Public API + tests |

## MSRV policy

The workspace declares `rust-version` in `Cargo.toml`. When bumping MSRV, update **CI**, **badges in README**, and `docs/industrial/quality-and-ci.md` if referenced.

## Fuzzing

See [fuzz/README.md](../fuzz/README.md).

---

# 开发说明（中文）

从源码参与 **symc** 开发：

1. 安装 **Rust**（≥ MSRV，见根目录 `Cargo.toml`）。
2. `cargo build -p symc`、`cargo test --workspace`。
3. 提交前与 CI 对齐：`fmt`、`clippy -D warnings`、一致性脚本 `scripts/conformance_vm_tree.sh`（需先 `cargo build -p symc`）。

模块职责见上表。模糊测试见 `fuzz/README.md`。

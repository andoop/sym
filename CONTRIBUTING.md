# Contributing to Sym

Thanks for your interest in improving Sym and `symc`.

## Quick checklist (before opening a PR)

1. Format and lint:
   ```bash
   cargo fmt -p symc --all
   cargo clippy -p symc --all-targets -- -D warnings
   ```
2. Tests:
   ```bash
   cargo test --workspace
   bash scripts/conformance_vm_tree.sh   # requires `cargo build -p symc` first
   ```
3. For **language or stdlib behavior changes**, open an issue or RFC first (see `docs/process/rfc-template.md`).

## How to contribute

- **Bugs**: open an issue with a minimal `.sym` reproducer and the exact `sym` command line.
- **Features**: propose in an issue; larger changes should go through the RFC template when they affect semantics or public surfaces.
- **Docs**: improvements to English (`README.md`, this file) or Chinese (`README.zh.md`, `docs/`) are welcome.

## Code of conduct

All participants must follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

---

# 贡献指南（中文）

感谢参与 Sym / `symc` 的改进。

## 提交 PR 前

- 执行：`cargo fmt -p symc --all`、`cargo clippy -p symc --all-targets -- -D warnings`、`cargo test --workspace`；并视情况运行 `scripts/conformance_vm_tree.sh`（需先 `cargo build -p symc`）。
- **语言或标准库行为变更**：请先开 issue 或按 [`docs/process/rfc-template.md`](docs/process/rfc-template.md) 写 RFC 讨论。
- **行为准则**：[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 参与方式

- **缺陷**：请附最小复现 `.sym` 与完整 `sym` 命令行。
- **功能**：建议先开 issue；影响语义或对外约定的大改动走 RFC。
- **文档**：欢迎同时改进英文与中文文档。

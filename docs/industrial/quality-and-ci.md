# 7、18、20、31：诊断、测试、内存、CI

## 1. 结构化诊断（进行中）

- **已实现**：`symc::format_error_json`、`symc::error_kind`；CLI 全局 `--message-format json|human`（`check` / `run` / `tokens` 的错误路径）。
- **后续**：稳定错误码字段 `code`（如 `E0123`）、`related` 二级 span、多错误数组。
- **拼接源**：已含 `logical_file`、`logical_line`、`logical_column`（片段内行列）。

## 2. 模糊与属性测试

- **Fuzz**：`cargo fuzz` 目标 `fuzz_lex`、`fuzz_parse`；见仓库 `fuzz/README.md`（若已添加）。
- **属性**：`proptest` 对 `lex → parse → typeck` 不 panic。

## 3. 内存基线

- 使用 `dhat`/`heaptrack` 或定期 `massif` 对 `sym run` 大脚本采样；在发布说明中记录趋势。

## 4. CI 与 MSRV

- 见 `.github/workflows/ci.yml`；`rust-version` 建议在根 `Cargo.toml` 的 `[workspace.package]` 声明并在此文档记录。

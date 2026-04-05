# Fuzzing (placeholder)

**English:** Sym’s fuzz harness is a **placeholder**. Once targets exist under `crates/symc/fuzz/fuzz_targets/`, run from `crates/symc`:

```bash
cargo install cargo-fuzz
cd crates/symc
cargo fuzz run fuzz_lexer   # example name; add real targets first
```

Tracked as maturity item **#18** in [docs/industrial/CHECKLIST.md](../docs/industrial/CHECKLIST.md).

---

# 模糊测试（占位）

待在 `crates/symc/fuzz/fuzz_targets/` 添加目标后更新命令与说明；工业清单见 [docs/industrial/CHECKLIST.md](../docs/industrial/CHECKLIST.md) 第 18 步。

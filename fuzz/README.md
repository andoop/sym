# 18：模糊测试（占位）

```bash
cargo install cargo-fuzz
cd crates/symc
cargo fuzz run fuzz_lexer   # 待添加 fuzz 目标
```

在 `crates/symc/fuzz/fuzz_targets/` 添加目标后更新本说明。

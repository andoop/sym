# 9、34：ABI 与嵌入

## 1. FFI / ABI（规划）

- 若对外暴露 C API：定义 `sym_value` 标签联合体、字符串 UTF-8、错误码枚举、线程安全边界。

## 2. 嵌入宿主

- 使用 `symc` 库：`parse_and_check`、`run_module`、`run_module_vm`；同一 `Module` 不可跨线程共享除非加锁。
- 初始化：设置 stdlib 根目录、`LoadOptions::no_prelude` 等行为需在宿主文档中写明。

# 1–5 & 8：规范、语义、字节码 VM、树 VM 对齐、源映射

## 1. 语言规范（大纲）

- **完整初稿**：[../spec/LANGUAGE.md](../spec/LANGUAGE.md)。
- 词法：`//`、`#` 注释、数字分隔符、保留字与内建名见 `crates/symc/src/builtins.rs`。
- 语法：`fn`、`let`、`if`/`else`、`while`、`do`/`end`、`match`、类型标注、枚举定义。
- 类型：`Int`、`Bool`、`String`、`Unit`、命名 ADT、`Never`、`Option[T]`（prelude）、函数类型。
- 程序入口：`main`，无参；`sym run --vm` 走字节码路径（未下沉构造编译期报错，见 [VM_SUBSET.md](../spec/VM_SUBSET.md)）。

## 2. 操作语义（笔记）

- **成文规范**：[../spec/SEMANTICS.md](../spec/SEMANTICS.md)。
- 摘要：调用按值；`while` 每次重算条件；`&&`/`||` 短路；`/` 向零、`%` 欧几里得；字符串序为 Unicode 标量序。

## 3. 字节码 VM 说明与对照入口

- **能力与指令**：[../spec/VM_SUBSET.md](../spec/VM_SUBSET.md)；**树↔VM 矩阵**：[../spec/VM_TREE_PARITY.md](../spec/VM_TREE_PARITY.md)。
- 摘要：在可编译前提下，覆盖字面量与局部、`if`/`let`/块/`while`、算术与比较（含 `CompareVal`）、短路逻辑、用户 `fn` 与 `Call`/`CallIndirect`、`match` 与枚举、`HostBuiltin` 宿主内建等；具体以 `bytecode.rs` / `vm.rs` 为准。

## 4. VM 与树解释器对齐

- **功能矩阵与边界**：[../spec/VM_TREE_PARITY.md](../spec/VM_TREE_PARITY.md)。
- 一致性测试：`crates/symc/src/lib.rs` 中 `vm_parity_*`、`run_module` vs `run_module_vm`。
- 扩展新内建或指令时：同步增加 parity 或 golden 用例。

## 5. 源映射（stitched）

- 加载器将 prelude 与 import 拼成单一源；每段前有 `# sym:file <路径>` 标记（见 `load.rs`）。
- **JSON 诊断**：`format_error_json` 附加 `logical_file`、`logical_line`、`logical_column`（片段内坐标，见 `sourcemap.rs`）；`line`/`column`/`span` 仍为整段拼接缓冲坐标。
- 精确「磁盘文件行号」需后续记录 strip 前行数偏移。

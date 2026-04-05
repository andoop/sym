# Sym 语言规范（初稿）

本文档描述当前 `symc` 实现的行为，供实现与测试对齐；未定稿处会标明。操作语义（求值顺序、短路等）见 [SEMANTICS.md](./SEMANTICS.md)。

## 1. 程序结构

- 一个 **模块** 由若干 **项** 组成：`type`（代数类型）、`fn`（函数）。
- **入口**：名为 `main` 的函数，无参数；由 CLI `sym run` 调用。
- **多文件**：`import a.b.c` 在加载阶段解析为文件路径并 **拼接** 成单一源字符串（见 §8）；拼接后仅一个模块。

## 2. 词法

- **注释**：`//` 行注释；`#` 行注释（整行以 `#` 开头有效）。
- **标识符**：字母、数字、下划线；不得以数字开头。
- **整数字面量**：十进制；允许 `_` 分隔（如 `1_000`）。
- **字符串字面量**：`"..."`，支持常见转义（与实现一致）。
- **关键字**（节选）：`fn`、`end`、`let`、`in`、`if`、`then`、`else`、`while`、`do`、`match`、`type`、`import`、`module` 等。
- **内建调用名**：保留，不可定义为普通 `fn`，列表见 `crates/symc/src/builtins.rs`。

## 3. 类型

- **标量**：`Int`、`Bool`、`String`、`Unit`。
- **命名类型**：`type Name ...` 定义的枚举和单变体和类型别名式用法（以检查器为准）。
- **`Option[T]`**：依赖 prelude 中的 ADT 定义。
- **函数类型**：`fn(T1,...) -> U` 仅出现在内建/高阶相关检查中；用户定义函数为顶层 `fn`。
- **`Never`**：发散表达式（如 `exit`）与类型合一规则由 `typeck` 定义。

## 4. 表达式与语句

- **块**：`do <stmt>... <tail-expr> end`；语句可为 `let x = e;` 或表达式语句 `e;`。
- **条件**：`if cond then a else b`；`cond` 为 `Bool`，两分支类型需一致或可合一（含 `Never`）。
- **循环**：`while cond do body end`；`cond: Bool`，`body: Unit`，整体 `Unit`。
- **局部绑定**：`let name = value in body`。
- **逻辑**：`&&`、`||` 短路；`!` 一元。
- **比较**：同类型 `==` / `!=`；`Int`/`String` 可有序比较 `<` 等（字符串为 Unicode 标量序）。
- **算术**：`+ - * / %` 作用于 `Int`；`/ ` 向零截断；`%` 为欧几里得余数（与 Rust `rem_euclid` 一致）。

## 5. `match`

- 模式：通配、绑定、构造子及字段；须 **穷尽**（检查器验证）。
- 语义：按顺序匹配首个成功模式。

## 6. 内建与 I/O

- 内建由编译器特殊处理：类型在 `typeck` 中注册；求值在 `interp`（及部分 VM 指令）中实现。
- 具副作用的内建（文件、网络、子进程等）与宿主进程权限一致；工业部署需另加沙箱策略（见 `docs/industrial/runtime-and-security.md`）。

## 7. 字节码 VM（`sym run --vm`）

- **子集**：不支持或未实现的语言特性会 **编译期** 报错，需用树解释器运行；清单见 [VM_SUBSET.md](./VM_SUBSET.md)。
- **语义**：与树解释器在支持的子集上应一致；参见 `crates/symc/src/lib.rs` 中 `vm_parity_*` 测试。
- **细节**：指令与内建映射见 `bytecode.rs`、`vm.rs`。

## 8. 拼接源与诊断

- 加载器在每个拼接片段前插入标记行：`# sym:file <路径>`（实现细节，非语言语法）。
- **类型检查/解析** 所见的 `Span` 基于 **拼接后** 缓冲区。
- **JSON 诊断**（`--message-format json`）在可解析时附带 `logical_file`、`logical_line`、`logical_column`：错误位置在 **该片段正文**（`# sym:file` 行之后、下一文件标记之前）内的路径与行列；全局的 `line`/`column`/`span` 仍相对于整段拼接缓冲。与磁盘上原始 `.sym` 行号可能不一致（剔除 `import`/`module` 行与 `trim` 等），见 [SEMANTICS.md §6](./SEMANTICS.md#6-拼接源与诊断坐标)。

## 9. 版本与变更

- 语言与 `symc` 版本绑定发布；破坏性变更遵循项目 changelog 与（未来）SemVer。

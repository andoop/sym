# `sym run --vm` 字节码路径

与实现源文件 [`crates/symc/src/bytecode.rs`](../../crates/symc/src/bytecode.rs)、[`vm.rs`](../../crates/symc/src/vm.rs) 对齐；类型检查仍走完整 `typeck`。不满足时 `compile_module` 返回 `VmCompile` 错误。与树解释器的对照见 [VM_TREE_PARITY.md](./VM_TREE_PARITY.md)。

## 模块形态

- 编译 **顶层 `fn`**；`type` 项不生成代码，但 **`type` 定义的 ADT** 会通过 `collect_variants` 参与枚举构造与 `match` 的字段顺序。
- 必须存在 **`main`**，且 **无参数**。
- 函数索引按模块内 **`fn` 出现顺序**；互相递归时顺序即定义顺序。

## 支持的表达式

| 类别 | 说明 |
|------|------|
| 字面量 | `Int`、`Bool`、`String`、`Unit` |
| 局部 | `Var`（含参数、`let`、块内绑定）；未遮蔽的全局 `fn` 名为 `PushFn` |
| 一元 | `-`（Int）、`!`（Bool）、`+`（Int，无操作） |
| 二元算术 | `+ - * / %`（Int） |
| 二元比较 | `== !=`：两操作数静态为 Int 或 Bool 时走快速指令；否则 `CompareVal` |
|  | `< <= > >=`：两操作数静态为 Int 时 `LtI`…；否则 `CompareVal`（含 String） |
| 逻辑 | `&&` `||` 短路（条件跳转） |
| 控制 | `if`、`let … in`、`do … end`、`while` |
| 调用 | 标识符 `f(args…)`：内建或本模块 `fn` 为 `Call`；否则先求值 `callee` 再 `CallIndirect` |
| `match` | 枚举子模式、`Some(x: v)` 等（见编译器 `compile_match`） |
| 枚举构造 | `Type.Variant(...)`（`BuildEnum`） |

## 内建

与 [`builtins.rs`](../../crates/symc/src/builtins.rs) 中保留名一致：在 **未被局部同名遮蔽** 时，由 `HostBuiltin` 或专用指令实现（`println`、`parse_int`、`assert` 等仍可有专用码以兼顾历史与优化）。

包含但不限于：`println` / `eprintln`、`exit`、`concat`、`string_from_int`、`strlen`、`read_line`、`assert`、`parse_int`、`env_get`、`read_file`、`write_file`、`write_file_ok`、`list_dir`、`glob_files`、`shell_exec`、`trim`、`starts_with`、`substring`、`index_of`、`http_post`、`http_post_sse_fold`、`stdout_print`、`json_string`、`json_extract`、`json_value`。

`http_post_sse_fold` 在 VM 内通过 **嵌套 `run_with_entry`** 调用 reducer 函数，语义与树解释器一致。

## 运行时限制

- 每函数局部槽 ≤ **255**（`u8` 索引）。
- 除零、`exit`、栈下溢、`assert` 失败等由 VM 报错为 `VmRuntime`。

## 回归

- 单元测试：`crates/symc/src/lib.rs` 中 `vm_parity_*`、`vm_short_circuit_*`、`vm_fib_12` 等；宿主 I/O 对拍含 `vm_parity_read_file`、`vm_parity_write_file_ok_and_read_file`、`vm_parity_env_get`（拼接 `stdlib/prelude.sym` 以使用 `Option`）。
- Golden：`tests/conformance/cases/`（如 `triple.sym`、`json_parity.sym`）+ `scripts/conformance_vm_tree.sh`（对应用例逐一校验 tree == vm == golden）。

# `sym run --vm` 可编译子集

与实现源文件 [`crates/symc/src/bytecode.rs`](../../crates/symc/src/bytecode.rs)、[`vm.rs`](../../crates/symc/src/vm.rs) 对齐；类型检查仍走完整 `typeck`。不满足时 `compile_module` 返回 `VmCompile` 错误。功能矩阵与已知边界见 [VM_TREE_PARITY.md](./VM_TREE_PARITY.md)。

## 模块形态

- 仅编译 **顶层 `fn`**；`type` 等项被跳过，但若被用户 `fn` 依赖则须在树解释器下使用。
- 必须存在 **`main`**，且 **无参数**。
- 函数索引按模块内 **`fn` 出现顺序**；互相递归调用时顺序即定义顺序。

## 支持的表达式

| 类别 | 说明 |
|------|------|
| 字面量 | `Int`、`Bool`、`String`、`Unit` |
| 局部 | `Var`（含参数、`let`、块内绑定） |
| 一元 | `-`（Int）、`!`（Bool）、`+`（Int，无操作） |
| 二元算术 | `+ - * / %`（Int） |
| 二元比较 | `== !=`：字面量全为 Int 或全为 Bool 时走快速指令；否则 `CompareVal`（`String`、局部、`Call` 等） |
|  | `< <= > >=`：两操作数静态为 Int 时 `LtI`…；否则 `CompareVal`（含 String） |
| 逻辑 | `&&` `||` 短路（跳转，非栈上 `And`/`Or`） |
| 控制 | `if`、`let … in`、`do … end`、`while` |
| 调用 | **仅** `callee` 为标识符的直接调用：`f(args…)` |

## 内建（专用指令，非普通 `Call`）

| 名称 | 约束 |
|------|------|
| `println` / `eprintln` | 参数个数 ≤ 255；求值后 `PrintLn` + `Unit` |
| `concat` | 恰好 2 个 `String` |
| `string_from_int` | 1 个 `Int` |
| `strlen` | 1 个 `String` |
| `exit` | 1 个 `Int`，进程退出 |

## 用户函数调用

- `Call { fn_idx, argc }`：`argc` 与形参个数由类型检查保证；被调函数体编译为独立 `Chunk`，末尾 `Ret`。

## 显式不支持（须树解释器）

- `match`
- 枚举构造 `Type.Variant(...)`
- **间接调用**：`callee` 非 `Var`（如变量存函数引用后调用）

## 运行时限制（实现细节）

- 每函数局部槽 ≤ **255**（`u8` 索引）。
- 除零、`exit`、栈下溢等由 VM 报错为 `VmRuntime`。

## 与树解释器一致性

- 子集行为应与 `Interpreter` 一致；回归见 `crates/symc/src/lib.rs` 中 `vm_parity_*`、`vm_short_circuit_*`、`vm_fib_12` 等。
- 仓库内 golden：`tests/conformance/cases/` + `scripts/conformance_vm_tree.sh`。

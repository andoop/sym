# VM 与树解释器行为对照（第 4 步）

在 **VM 可编译子集** 内，两种运行方式应对同一程序给出 **相同结果**（`main` 的 `Value` 相等；含 `println` 时 stdout 亦应对齐）。详见 [VM_SUBSET.md](./VM_SUBSET.md)。

## 功能矩阵

| 能力 | 树解释器 | `--vm` | 说明 |
|------|----------|--------|------|
| 顶层 `fn`、相互调用 | ✓ | ✓ | `main` 须无参 |
| `type` 定义 | 参与类型检查 | 不参与字节码 | ADT 用于类型检查；VM 程序若不用 `match`/构造体则仍可能可跑 |
| 字面量、局部、`let`、块 | ✓ | ✓ | |
| `if` / `while` | ✓ | ✓ | `while` 体须 `Unit` |
| `&&` `||` | ✓ 短路 | ✓ 短路 | 字节码用条件跳转 |
| 算术与比较 | ✓ | ✓ | `CompareVal` 与 `interp` 规则一致 |
| `match`、枚举构造 | ✓ | ✗ | VM 编译报错 |
| 间接调用 | ✓（若类型允许） | ✗ | VM 仅 `f(...)` 直接调用 |
| 内建 | 全部 | [子集](./VM_SUBSET.md#内建专用指令非普通-call) | 见 [builtins-contracts.md](../industrial/builtins-contracts.md) |

## 回归手段

- **单元测试**：`crates/symc/src/lib.rs` 中 `vm_parity_*`、`vm_short_circuit_*`、`vm_fib_12`、`vm_while_*` 等。
- **Golden**：`tests/conformance/` + `scripts/conformance_vm_tree.sh`。

## 已知边界

- 含 **未下沉内建**（如 `read_file`）的程序无法用 VM 跑通，不要求 parity。
- **拼接源** 下诊断坐标：全局 `line`/`column` 相对 stitched；`logical_*` 相对片段正文（见 [SEMANTICS.md §6](./SEMANTICS.md#6-拼接源与诊断坐标)）。

# VM 与树解释器行为对照（第 4 步）

对 **同一已类型检查的模块**，在 VM 可编译的前提下，`sym run` 与 `sym run --vm` 应对 **`main` 的返回值** 一致（含 `println` 时 stdout 亦应对齐）。VM 编译失败时返回 `VmCompile`，与树路径无关。

## 功能矩阵

| 能力 | 树解释器 | `--vm` | 说明 |
|------|----------|--------|------|
| 顶层 `fn`、相互调用 | ✓ | ✓ | `main` 须无参 |
| `type` 定义 | 参与类型检查 | 参与 ADT 元数据（构造 / `match`） | |
| 字面量、局部、`let`、块 | ✓ | ✓ | |
| `if` / `while` | ✓ | ✓ | `while` 体须 `Unit` |
| `&&` `||` | ✓ 短路 | ✓ 短路 | |
| 算术与比较 | ✓ | ✓ | `CompareVal` 与 `interp` 规则一致 |
| `match`、枚举构造 | ✓ | ✓ | |
| 间接调用（`FnRef`） | ✓ | ✓ | `CallIndirect` |
| 保留内建 | ✓ | ✓ | 共享 `interp::host_builtin_apply` 与 SSE fold 逻辑；见 [VM_SUBSET.md](./VM_SUBSET.md) |

## 回归手段

- **单元测试**：`crates/symc/src/lib.rs` 中 `vm_parity_*`、`vm_short_circuit_*`、`vm_fib_12`、`vm_while_*` 等。
- **Golden**：`tests/conformance/` + `scripts/conformance_vm_tree.sh`。

## 已知边界

- **拼接源** 下诊断坐标：全局 `line`/`column` 相对 stitched；`logical_*` 相对片段正文（见 [SEMANTICS.md §6](./SEMANTICS.md#6-拼接源与诊断坐标)）。

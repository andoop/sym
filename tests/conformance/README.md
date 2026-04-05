# 17：一致性测试

- **树 vs VM**：在 `symc` 单元测试中通过 `assert_vm_matches_tree` 与专用用例扩展。
- **Golden**：`cases/*.sym` + `cases/*.stdout.expected`（期望的 `sym run` 标准输出，末尾换行与 golden 一致）。
- **脚本**：仓库根目录 `scripts/conformance_vm_tree.sh`（需已 `cargo build -p symc`）：校验 `triple.sym` 树/vm 输出一致且等于 `triple.stdout.expected`。
- **约定**：新增 VM 行为须先增加 parity 或 golden，再合并。

# 工业成熟度 — 37 步交付索引

状态：`[ ]` 未验收 · `[~]` 已有文档/脚手架 · `[x]` 已落实（可持续维护）

| # | 交付物 | 路径 / 说明 |
|---|--------|----------------|
| 1 | 语言规范大纲 | [../spec/LANGUAGE.md](../spec/LANGUAGE.md) |
| 2 | 操作语义笔记 | [../spec/SEMANTICS.md](../spec/SEMANTICS.md) |
| 3 | 核心子集定义 | [../spec/VM_SUBSET.md](../spec/VM_SUBSET.md) |
| 4 | VM / 树解释器对齐说明 | [../spec/VM_TREE_PARITY.md](../spec/VM_TREE_PARITY.md)、[spec-and-semantics.md](./spec-and-semantics.md) §4 |
| 5 | VM 特性扩展路线图 | [vm-and-performance.md](./vm-and-performance.md) §1 |
| 6 | 运行时模型与上限 | [runtime-and-security.md](./runtime-and-security.md) §1 |
| 7 | 诊断与结构化错误方案 | [quality-and-ci.md](./quality-and-ci.md) §1（CLI `json` 已部分落地） |
| 8 | 源映射 / stitched 定位 | `sourcemap.rs` + JSON `logical_*`；[spec-and-semantics.md](./spec-and-semantics.md) §5 |
| 9 | FFI / ABI 说明 | [embedding-and-abi.md](./embedding-and-abi.md) |
| 10 | 内建契约表 | [builtins-contracts.md](./builtins-contracts.md) |
| 11 | 沙箱与安全模型 | [runtime-and-security.md](./runtime-and-security.md) §2 |
| 12 | 供应链与审计流程 | [supply-chain.md](./supply-chain.md) |
| 13 | 包与模块系统设计 | [ecosystem-and-process.md](./ecosystem-and-process.md) §1 |
| 14 | 包管理器 CLI 规划 | [ecosystem-and-process.md](./ecosystem-and-process.md) §2 |
| 15 | 标准库分层 | [ecosystem-and-process.md](./ecosystem-and-process.md) §3 |
| 16 | 标准库稳定性分级 | [ecosystem-and-process.md](./ecosystem-and-process.md) §4 |
| 17 | 一致性测试说明 | [../../tests/conformance/README.md](../../tests/conformance/README.md) + `scripts/conformance_vm_tree.sh` |
| 18 | 模糊 / 属性测试指引 | [quality-and-ci.md](./quality-and-ci.md) §2 |
| 19 | 性能基准规划 | [vm-and-performance.md](./vm-and-performance.md) §2 |
| 20 | 内存基线指引 | [quality-and-ci.md](./quality-and-ci.md) §3 |
| 21 | VM 后端演进 | [vm-and-performance.md](./vm-and-performance.md) §3 |
| 22 | 增量编译 / 缓存 | [vm-and-performance.md](./vm-and-performance.md) §4 |
| 23 | LSP 规划 | [tooling-roadmap.md](./tooling-roadmap.md) §1 |
| 24 | 格式化与风格 | [tooling-roadmap.md](./tooling-roadmap.md) §2 |
| 25 | Linter 规划 | [tooling-roadmap.md](./tooling-roadmap.md) §3 |
| 26 | 调试 / 跟踪 | [tooling-roadmap.md](./tooling-roadmap.md) §4 |
| 27 | 测试运行器 | [tooling-roadmap.md](./tooling-roadmap.md) §5 |
| 28 | 文档生成 | [tooling-roadmap.md](./tooling-roadmap.md) §6 |
| 29 | 迁移与版本策略 | [ecosystem-and-process.md](./ecosystem-and-process.md) §5 |
| 30 | RFC 模板与流程 | [../process/rfc-template.md](../process/rfc-template.md) |
| 31 | CI 矩阵与 MSRV | [../../.github/workflows/ci.yml](../../.github/workflows/ci.yml)、[quality-and-ci.md](./quality-and-ci.md) §4 |
| 32 | 发布与 changelog | [../process/release-checklist.md](../process/release-checklist.md)、[../../CHANGELOG.md](../../CHANGELOG.md) |
| 33 | LTS 策略草案 | [../process/lts-policy.md](../process/lts-policy.md) |
| 34 | 嵌入宿主指南 | [embedding-and-abi.md](./embedding-and-abi.md) §2 |
| 35 | 治理与贡献 | [../../CONTRIBUTING.md](../../CONTRIBUTING.md)、[../../CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md) |
| 36 | 诊断国际化 | [i18n-diagnostics.md](./i18n-diagnostics.md) |
| 37 | 合规与审计日志 | [runtime-and-security.md](./runtime-and-security.md) §3 |

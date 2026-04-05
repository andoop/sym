# 12：供应链

- 锁定：`Cargo.lock` 提交仓库；升级依赖走 PR + changelog。
- 审计：本地/CI 运行 `cargo audit`（需安装 `cargo-audit`）；高危项阻塞发布。
- 许可证：`cargo deny check licenses`（可选 `deny.toml`）。
- SBOM：发布流程中 `cargo cyclonedx` 或平台等价物；见 `docs/process/release-checklist.md`。

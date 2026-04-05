# 13–16、29：包管理、标准库、迁移

## 1. 包与模块系统（设计）

- 包 ID、版本、入口文件、`sym.toml` 清单格式（待定义）。

## 2. 包管理器 CLI（设计）

- 子命令：`add`、`update`、`vendor`、lockfile 路径规范。

## 3. 标准库分层

- `prelude.sym`：当前单体；未来拆 `core`（无 IO）与 `std`。

## 4. 稳定性分级

- `stable` / `experimental` / `internal` 标注规则与 semver 联动。

## 5. 版本与迁移

- 主版本：不兼容语法/类型/内建行为；提供迁移说明与可选 `sym migrate`。

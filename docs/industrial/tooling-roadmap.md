# 23–28：语言服务与工具链

## 1. LSP

- 能力：诊断、补全、跳转、`textDocument/hover` 类型、重命名；crate `symc` 复用 typeck。

## 2. 格式化

- `sym fmt`：基于语法树或官方风格指南；与 `clippy`-style 规则分离。

## 3. Linter

- `sym lint`：弃用 API、非可移植内建、性能反模式。

## 4. 调试

- DAP 或自定义：`--trace` 逐步指令 / AST；断点绑定源映射。

## 5. 测试运行器

- `sym test`：约定目录、`expect` 宏或快照文件。

## 6. 文档生成

- 从 `fn` 签名与注释生成静态站点；链接到语言规范。

# Sym（中文）

[![CI](https://github.com/andoop/sym/actions/workflows/ci.yml/badge.svg)](https://github.com/andoop/sym/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

面向「结构清晰、歧义少、便于工具与模型协同」的小型语言；本仓库提供 **词法 / 语法 / 类型检查 / 树解释器** 与可选 **字节码 VM**，CLI 名为 **`sym`**。

**English overview:** [README.md](README.md)

**状态：** 早期 / **实验性** — 语义与 CLI 可能变更；严肃实验请固定 git 提交。

**GitHub 主页右侧 About**（描述、网站、Topics）须在网页上填写，仓库内备有可复制文案：[.github/ABOUT.md](.github/ABOUT.md)。

## 环境

需要安装 [Rust](https://rustup.rs/)（stable，**MSRV 1.74**，见根目录 `Cargo.toml`）。

## 构建

```bash
cargo build --release
```

可执行文件：`target/release/sym`；也可 `cargo install --path crates/symc`。

## 用法

```bash
# 类型检查（会加载入口文件的 import 链，并默认前置 stdlib/prelude.sym）
sym check examples/hello.sym

# 解析、检查并运行入口函数 main（无参数）
sym run examples/hello.sym

# 栈式字节码 VM（仅支持语言子集；不支持的程序会报错并需去掉 `--vm`）
sym run --vm examples/vm_fib.sym

# 多文件示例
sym run examples/call_lib.sym

# 词法调试：打印 token 与字节 span
sym tokens examples/hello.sym

# 可选：不加载 prelude；自定义 stdlib 根目录（用于 `import a.b` 回退到 DIR/a/b.sym）
sym --no-prelude --stdlib ./stdlib check examples/hello.sym

# 错误输出为单行 JSON（便于 CI / 工具解析）
sym check --message-format json bad.sym
```

## 工业成熟度路线图

37 步交付索引与分组文档：[docs/industrial/CHECKLIST.md](docs/industrial/CHECKLIST.md)。

**规范与文档**：语言初稿 [docs/spec/LANGUAGE.md](docs/spec/LANGUAGE.md)；操作语义 [docs/spec/SEMANTICS.md](docs/spec/SEMANTICS.md)；语法全览与蓝图 [docs/spec/SYNTAX.md](docs/spec/SYNTAX.md)；设计理念 [docs/spec/SOUL.md](docs/spec/SOUL.md)；VM 子集 [docs/spec/VM_SUBSET.md](docs/spec/VM_SUBSET.md)；VM↔树对照 [docs/spec/VM_TREE_PARITY.md](docs/spec/VM_TREE_PARITY.md)。示例索引见 [examples/README.md](examples/README.md)。

## 验证

```bash
cargo test -p symc
cargo run -p symc -- run examples/hello.sym
cargo run -p symc -- run examples/fib.sym
cargo run -p symc -- run --vm examples/vm_fib.sym
cargo run -p symc -- run examples/option.sym
```

`examples/math_lib.sym` 为被 `import` 的库文件，无 `main`，请用 `sym run examples/call_lib.sym` 等入口验证多文件。需 stdin 的 `examples/while_input.sym` 可：`printf 'ok\n' | cargo run -p symc -- run examples/while_input.sym`。

**OpenAI 兼容 / DeepSeek / Anthropic 示例**：见 `examples/ccode/main.sym` 文件头环境变量说明；运行前配置 `DEEPSEEK_API_KEY` 或 `OPENAI_API_KEY` 等；若同时存在 `ANTHROPIC_API_KEY`，默认可能走 Anthropic，可用 `CCODE_USE_OPENAI=1` 强制 OpenAI 线。

## 语言要点（摘要）

- **注释**：`#` 与 `//` 均为行注释；`/` 仍为除法。
- **字面量**：整数可用 `_` 分组；枚举、`fn`、`do`、调用等支持尾随逗号。
- **枚举与 `match`**：`type` 定义变体；`match` 穷尽检查。
- **函数**：`fn f(a: T) -> U = ... end`；省略 `-> U` 时为 `Unit`。
- **循环**：`while cond do body end`；`&&` / `||` **短路**。
- **运算**：整数 `+ - * / %`（`%` 为欧几里得余数，与 Rust `rem_euclid` 一致）；`String` 可比较 `== != < <= > >=`（UTF-8 字节序与 Unicode 标量字典序一致）。
- **内建**：`println` / `eprintln`、`exit`、`concat`、字符串与文件、HTTP、`json_*` 等（不可再用 `fn` 重名）；详见 [docs/industrial/builtins-contracts.md](docs/industrial/builtins-contracts.md)。
- **VM 子集**：`sym run --vm`；与树解释器对照见 [docs/spec/VM_TREE_PARITY.md](docs/spec/VM_TREE_PARITY.md)。
- **多文件**：`import` 解析与同目录 / `--stdlib` 回退。

更完整的语义与示例见 `examples/` 与 `stdlib/prelude.sym`。**`examples/library_mgmt.sym`**：基于 `List[Book]` 的简易交互图书管理（内存数据）。

## 开发与文档索引

- 开发者搭建与模块说明：[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)  
- 规范目录：[docs/spec/README.md](docs/spec/README.md)  
- 提问与缺陷反馈方式：[SUPPORT.md](SUPPORT.md)

## 参与与许可

贡献见 [CONTRIBUTING.md](CONTRIBUTING.md)；行为准则 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)；安全 [SECURITY.md](SECURITY.md)。**MIT License**，见 [LICENSE](LICENSE)。

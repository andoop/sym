# Sym

面向「结构清晰、歧义少、便于工具与模型协同」的小型语言；本仓库提供 **词法/语法/类型检查/解释器** 与 CLI `sym`。

## 环境

需要安装 [Rust](https://rustup.rs/)（`cargo`）。

## 构建

```bash
cargo build --release
```

可执行文件：`target/release/sym`（或将 `crates/symc` 单独 `cargo install --path crates/symc`）。

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
cargo run -p symc -- check examples/deepseek_agent.sym
```

`examples/math_lib.sym` 为被 `import` 的库文件，无 `main`，请用 `sym run examples/call_lib.sym` 等入口验证多文件。需 stdin 的 `examples/while_input.sym` 可：`printf 'ok\n' | cargo run -p symc -- run examples/while_input.sym`。`examples/deepseek_agent.sym` 需设置 `DEEPSEEK_API_KEY` 后再 `sym run`。

## 语言要点

- **注释**：`#` 与 `//` 均为**行注释**（从记号起到行尾）。除法运算符仍是单个 `/`；`//` 不会与 `/` 冲突。
- **字面量**：十进制整数允许用 `_` 分组，如 `1_000_000`；`_` 两侧须为数字，禁止尾部悬空的 `_`。
- **分支链**：`if c0 then e0 else if c1 then e1 else e2` 按嵌套 `if` 解析，各分支类型须一致。
- **尾随逗号**：`fn f(a: Int,)`、`f(1, 2,)`、`Option[Int].Some(1,)`、`type T = | C(x: Int,) |`、`[Int, Bool,]` 等列表处均可选尾随逗号。
- **一元 `+`**：仅对 `Int` 有效，语义为恒等。
- **枚举**：`type Name[T] = | Var(fields...) | ... end`，构造：`Name[Int].Some(1)`、`Color.Red`。
- **函数**：`fn f(a: T) -> U = ... end`；省略 `-> U` 时返回 `Unit`。
- **序列**：`do stmt; ... tail end`，其中 `let` 语句为 `let x = e;`。
- **循环**：`while cond do body end`，`cond` 每轮重新求值，整体类型为 `Unit`；`body` 须为 `Unit`（常用 `do ... end` 块）。示例见 `examples/while_input.sym`（读 stdin 直到输入 `ok`）。
- **运算**：整数 `+ - * / %`；`%` 为欧几里得取余（与 Rust `rem_euclid` 一致，对负数结果非负）。`String` 支持 `==`、`!=` 与字典序比较 `< <= > >=`（按 Unicode 标量值序列比较，与 Rust `str::chars().cmp` 一致）。
- **内建**（不可再用 `fn` 重定义同名）：`println` / `eprintln`（参数规则同 `println`，后者输出到 stderr）；`exit(Int)` 以给定状态码结束进程（表达式类型为 `Never`，可与 `Unit` 等兼容）；`concat`、`string_from_int`、`strlen`、`read_line`、`assert`、`parse_int`；字符串工具 `trim`、`starts_with`、`substring(s, start, len)`（`len < 0` 表示一直到末尾；`start` 为从 0 起的 Unicode 标量索引）；`index_of(hay, needle)`（子串首现位置，按标量索引，找不到为 `-1`）；`json_string(s)`（把 `s` 编成带引号的 JSON 字符串字面量，供在 Sym 里手写 JSON）；`json_extract(json, path)`（按点分路径取 JSON 字符串值，路径段为纯数字时表示数组下标，如 `choices.0.message.content`，失败为 `None`）；`http_post(url, headers, body) -> Option[String]`（HTTPS POST，`headers` 为多行 `Name: Value`；能读到响应体则为 `Some`（含非 2xx，便于解析错误 JSON），请求或读体失败为 `None`）；环境变量 `env_get`；文件 `read_file` / `write_file`。`println` / `eprintln` 参数可为 `Int` / `Bool` / `String` / `Unit` / 任意枚举值。
- **示例**：`examples/deepseek_agent.sym` 用 Sym 拼 OpenAI 兼容请求体并调 DeepSeek（URL/模型名在源码中，非语言内置）；运行前 `export DEEPSEEK_API_KEY`，可选 `DEEPSEEK_BASE_URL`（默认 `https://api.deepseek.com/v1`，与 `concat(..., "/chat/completions")` 拼接）。写入任意路径有风险，仅用于本地实验。
- **Prelude**：默认前置 `stdlib/prelude.sym`（含泛型 `Option[T]`、`List[T]`）。多文件拼接后可用行首注释 `# sym:file <路径>` 对照报错行所属源文件。
- **逻辑运算**：`&&` 与 `||` 在解释器中**短路**求值（右侧可为不执行的副作用或如 `1/0` 等危险表达式，只要左侧已决定结果）。
- **字节码 VM**：`sym run --vm FILE.sym` 走栈式虚拟机（[`crates/symc/src/bytecode.rs`](crates/symc/src/bytecode.rs) / [`vm.rs`](crates/symc/src/vm.rs)）。子集与树解释器对照见 [docs/spec/VM_SUBSET.md](docs/spec/VM_SUBSET.md)、[docs/spec/VM_TREE_PARITY.md](docs/spec/VM_TREE_PARITY.md)。摘要：支持 `String` 字面量与比较、`while`、短路 `&&`/`||`；内建仅 `println`/`eprintln`、`concat`、`string_from_int`、`strlen`、`exit`；不支持 `match`、枚举构造、间接调用等——遇不支持构造会报 `VM: …` 并需改用默认解释器。
- **多文件**：`import math_lib` 先在同目录找 `math_lib.sym`，找不到再在 `--stdlib`（默认 `./stdlib`）下按路径查找。各文件里的 `import` / `module` 行在拼接时会被去掉，再整体类型检查。

更完整的语义与示例见 `examples/` 与 `stdlib/prelude.sym`。另有 **`examples/library_mgmt.sym`**：基于 `List[Book]` 与递归 `repl` 的简易图书管理（列出 / 入库 / 借出 / 归还，内存数据）。

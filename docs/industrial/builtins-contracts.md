# 10：内建契约（全表）

来源：`crates/symc/src/builtins.rs`、`typeck.rs`（签名）、`interp.rs`（语义）、`bytecode.rs`（VM 专用分支）。

**VM** 列：`专用` = 有独立指令、可在 `--vm` 子集中使用（见 [../spec/VM_SUBSET.md](../spec/VM_SUBSET.md)）；`—` = 仅树解释器（程序若调用则整体不可 VM）。对照表见 [../spec/VM_TREE_PARITY.md](../spec/VM_TREE_PARITY.md)。

**Option†**：返回类型含 `Option[…]` 时，类型检查要求 ADT `Option` 在作用域内（默认 CLI 会加载 `stdlib/prelude.sym`；`--no-prelude` 时需自行定义或导入）。

| 名称 | 参数（类型检查） | 返回 | 主要副作用 / 备注 | VM |
|------|------------------|------|-------------------|-----|
| `println` | 0..n，`ensure_printable`：`Int`/`Bool`/`String`/`Unit`/任意枚举 | `Unit` | stdout，参数间无分隔，末尾 `\n` | 专用 |
| `eprintln` | 同上 | `Unit` | stderr，同上 | 专用 |
| `exit` | 1×`Int` | `Never` | `process::exit` | 专用 |
| `concat` | 2×`String` | `String` | 无 | 专用 |
| `string_from_int` | 1×`Int` | `String` | 无 | 专用 |
| `strlen` | 1×`String` | `Int` | Unicode 标量个数（与 `substring`/`index_of` 索引一致） | 专用 |
| `read_line` | 0 | `String` | stdin 一行；去尾 `\n`/`\r\n`；EOF 得空串；IO 错为 `RuntimeError` | — |
| `assert` | `Bool`, `String` | `Unit` | 条件为假 → `RuntimeError`（`assertion failed: …`） | — |
| `parse_int` | 1×`String` | `Option[Int]`† | 先 `trim` 再解析；失败 `None` | — |
| `env_get` | 1×`String` | `Option[String]`† | `std::env::var` 失败（未设置或无效 Unicode 等）→ `None` | — |
| `read_file` | 1×`String` | `Option[String]`† | 读盘失败 → `None` | — |
| `write_file` | 2×`String`（path, content） | `Unit` | 写失败 → `RuntimeError` | — |
| `write_file_ok` | 2×`String` | `Bool` | 成功与否，不抛错 | — |
| `list_dir` | 1×`String` | `Option[String]`† | 成功：`Some`，多行文件名，排序；目录名带后缀 `/`；跳过隐藏项；失败 `None` | — |
| `glob_files` | 2×`String`（base, pattern） | `Option[String]`† | `base` 与 `pattern` 拼接后走 `glob`；成功为排序后的 `\n` 分隔路径 | — |
| `shell_exec` | 2×`String`（cwd, command） | `Option[String]`† | POSIX：`sh -lc`；Windows：`cmd /C`；`CCODE_DISABLE_SHELL=1|true` 时返回说明性 `Some`；正常情况 `Some` 为 `[exit n]` + stderr/stdout 块（合计约 512KiB 上限）；仅启动子进程失败等为 `None`（错误可打 stderr） | — |
| `trim` | 1×`String` | `String` | Unicode trim | — |
| `starts_with` | 2×`String` | `Bool` | 无 | — |
| `substring` | (`String`, `Int`, `Int`) | `String` | `start`、`len` 为标量索引；`len < 0` 表示直到末尾；负 `start` 按 0 处理 | — |
| `index_of` | 2×`String` | `Int` | 子串首现标量索引；未找到 `-1` | — |
| `http_post` | 3×`String`（url, headers, body） | `Option[String]`† | HTTPS POST；`headers` 多行 `Name: Value`；**能读到响应体则为 `Some`（含非 2xx，便于解析错误 JSON）**；请求/读体失败 `None`；代理见 `interp` 中 `sym_http_agent`（读常见 `*_PROXY`） | — |
| `http_post_sse_fold` | 4×`String` + `fn(String, String) -> String` | `Option[String]`† | SSE 流式折叠； reducer 在解释器里回调；需 `Option`；网络错误等 → `None` | — |
| `stdout_print` | 1×`String` | `Unit` | stdout 无换行 | — |
| `json_string` | 1×`String` | `String` | JSON 字符串转义（供拼接 JSON 文本） | — |
| `json_extract` | 2×`String`（json, path） | `Option[String]`† | 点分路径（数字段为数组下标）；**仅当叶节点为 JSON string 时 `Some`** | — |
| `json_value` | 2×`String` | `Option[String]`† | 同路径导航；叶节点任意类型 → `serde_json::Value` 的 `to_string()` | — |

## 与实现核对时看哪里

- 类型与 arity：`typeck.rs` 中 `ExprKind::Call` 对保留名的 `match` 分支。
- 运行时：`interp.rs` 同名分支；`http_post` / `shell_exec` / `json_*` 的辅助函数在同文件后部。
- VM 可编译性：`bytecode.rs` 中 `compile_expr` 对 `Call` 的内建特判。

## 仍待成文（工业深化）

- 可重入性、超时上限一览（HTTP 已有约 120s，见 `sym_http_agent`）。
- 国际化诊断与稳定错误码（若 CLI 要机器可读）。

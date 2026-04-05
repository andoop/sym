# Examples / 示例程序索引

**English:** Single-file `.sym` samples live in this folder; subfolders are multi-file demos. Run from repo root: `sym run <entry>.sym`.

根目录下的 `.sym` 多为单文件小例子；子目录为多文件或应用场景。

| 路径 | 说明 |
|------|------|
| `hello.sym` | 最小入口 |
| `fib.sym` | 递归 |
| `vm_fib.sym` | 字节码 VM 子集 |
| `option.sym` | `Option` / prelude |
| `list.sym` | `List` |
| `while_input.sym` | `while` + stdin |
| `syntax_showcase.sym` | 语法片段演示 |
| `library_mgmt.sym` | 交互式图书管理（内存） |
| `math_lib.sym` | 被 `import` 的库（无 `main`） |
| `call_lib.sym` | 多文件：`import math_lib` |
| `ccode/main.sym` | OpenAI 兼容 / Anthropic 双路径 agent（需环境变量，见文件头注释） |
| `ccode/agent/*.sym` | ccode 依赖模块 |
| `ccode/web-playground/` | 浏览器侧实验页（HTML），与 Sym 运行无关 |

运行：`sym run <入口.sym>`（项目根目录下执行）。

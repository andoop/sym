# Sym 语法全览与蓝图

本文档描述 **Sym** 的语法：**§2–§11** 与当前 `symc` 实现一致；**§12** 为规划中的扩展蓝图（尚未承诺排期）。与设计理念对照见 [`SOUL.md`](./SOUL.md)。

**约定**：已实现语法用 `sym` 代码块；蓝图用注释标明「（蓝图）」或「当前不可用」。

---

## 1. 记法约定

- **`|`**：在 `type` 与 `match` 中引入一条分支（非按位或）。
- **`end`**：闭合 `type`、`fn`、`match`、`while`、`do` 等块（**不**用于闭合 `if`）。
- **`if`**：`if cond then e1 else e2` — **无**尾随 `end`。
- **标识符**：字母/下划线起始，后续可为字母、数字、`_`（与实现一致）。
- **大小写惯例**：类型与枚举构造子常以大写开头（惯例，非词法强制）。

**示例（`|`、`end`、`if` 无 `end`）：**

```sym
type Color =
| Red
| Green
end

fn demo(x: Int) -> Int =
  if x < 0 then 0 else if x > 10 then 10 else x
end

fn use_match(c: Color) -> Int =
  match c
  | Red => 1
  | Green => 2
  end
end

fn main() -> Unit =
  println(use_match(Color.Red))
end
```

---

## 2. 词法（Lexical）

### 2.1 空白与注释

- 空白：` `、`\t`、`\r`、`\n`（无特殊缩进语义，缩进不参与语法）。
- **`#`**：行注释，从 `#` 到行尾。
- **`//`**：行注释，从 `//` 到行尾（与除法运算符 **`/`** 区分：`//` 整段为注释）。

**示例：**

```sym
# 哈希注释
fn f() -> Int = 1 end // 行尾也可写 //

fn g() -> Int =
  2 + 3  // 除法要写单个 /，不能写成 //
end

fn main() -> Unit =
  do
    println(f());
    println(g());
    ()
  end
end
```

### 2.2 字面量

| 形式 | 说明 |
|------|------|
| **整数** | 十进制；可在数字间插入 `_` 分组（两侧须为数字，无悬空 `_`）。 |
| **字符串** | `"..."`，支持转义 `\"`、`\\`、`\n`、`\r`、`\t`；字符串内不可裸写换行。 |
| **`true` / `false`** | 布尔。 |
| **`()`** | 单元值（`Unit`）。 |

**示例：**

```sym
fn literals() -> Unit =
  do
    println(1_000_000);
    println("line\nbreak");
    println(true);
    println(());
    ()
  end
end

fn main() -> Unit = literals() end
```

### 2.3 关键字（保留为记号，不可作普通标识符绑定名）

`module`、`import`、`as`、`type`、`fn`、`let`、`in`、`if`、`then`、`else`、`match`、`while`、`do`、`end`、`true`、`false`。

**说明**：下列名字若用作 `fn f(let: Int)` 这类参数名会与词法冲突；应避开。

### 2.4 运算符与分隔符（已实现）

| 记号 | 用途 |
|------|------|
| `=` | 定义处绑定（`fn` 体、`type` 等）；**非**比较。 |
| `==` `!=` | 相等、不等。 |
| `<` `<=` `>` `>=` | 有序比较（`Int` / `String`）。 |
| `+` `-` `*` `/` `%` | 算术；`%` 为欧几里得取余。 |
| `-`（一元） | 整数取负。 |
| `+`（一元） | 整数恒等。 |
| `!` | 逻辑非（`Bool`）。 |
| `&&` `||` | 逻辑与、或（解释器短路）。 |
| `->` | 函数类型、函数结果类型。 |
| `=>` | `match` 分支体引导。 |
| `\|` | `type` 变体列表、`match` 分支、`||`。 |
| `,` `;` `:` `.` `( ) [ ]` | 列表、序列、类型标注、成员访问等。 |

**示例：**

```sym
fn ops(n: Int, s: String) -> Bool =
  (n % 3 == 1) && (s != "") || !false
end

fn main() -> Unit =
  println(ops(1, "a"))
end
```

---

## 3. 模块与拼接标记

### 3.1 可选模块头

```text
module a.b.c
```

- 可选；若出现，须置于文件最前（在 `import` 之前）。
- 多文件加载时，非入口文件的 `module` / `import` 行会在拼接时剔除。

**示例：**

```sym
module examples.math_stub

fn triple(x: Int) -> Int = x * 3 end
```

### 3.2 导入

```text
import path.to.file
import x.y as alias
```

- `path` 为点分标识符序列，解析为相对文件或 `--stdlib` 下路径（实现细节见加载器）。

**示例（见仓库 `examples/call_lib.sym`）：**

```sym
import math_lib

fn main() -> Unit =
  do
    println(triple(4));
    ()
  end
end
```

### 3.3 拼接溯源注释（由加载器注入，也可手写对照）

```text
# sym:file <路径>
```

- 非语言语义；用于在拼接大源码中定位原始文件。

**示例：**

```sym
# sym:file /project/examples/hello.sym
fn main() -> Unit = println("hi") end
```

---

## 4. 顶层条目

程序由若干 **`type`** 与 **`fn`** 组成（顺序任意，相互可前向引用由类型检查器处理）。

```text
Module ::= [ModuleDecl] Import* Item*
Item   ::= TypeDef | FnDef
```

**示例（最小文件骨架）：**

```sym
type Bit =
| Zero
| One
end

fn as_int(b: Bit) -> Int =
  match b
  | Zero => 0
  | One => 1
  end
end

fn main() -> Unit = println(as_int(Bit.One)) end
```

---

## 5. 类型定义（代数类型）

```text
type Name [ [ TParam (, TParam)* ,? ] ] =
  | VariantName ( Field (, Field)* ,? )?
  | ...
end

Field ::= ident : TypeExpr
```

- **泛型参数**：方括号内逗号分隔的类型参数名（当前实现为 ADT 与字段替换用）。
- **变体**：`| VariantName` 后可选字段列表 `(...)`；无括号表示无字段。
- **尾随逗号**：字段列表、`[]` 内参数列表等允许可选尾随逗号。

**示例（无字段变体 + 有字段变体 + 尾随逗号）：**

```sym
type Status =
| Ok
| Err(msg: String,)
end

type Pair[A, B] =
| P(first: A, second: B,)
end

fn main() -> Unit =
  println(Status.Ok)
end
```

---

## 6. 函数定义

```text
fn name ( Param (, Param)* ,? ) [ -> TypeExpr ] = Expr end

Param ::= ident : TypeExpr
```

- 省略 **`-> TypeExpr`** 时，返回类型为 **`Unit`**。
- 无参数：`fn name () = ... end`。
- **入口**：`fn main() -> ... = ... end`，无参数，由运行时调用。

**示例：**

```sym
fn nop() -> Unit = () end

fn add(a: Int, b: Int,) -> Int = a + b end

fn greet(name: String) =
  println(concat("hello, ", name))
end

fn main() -> Unit =
  do
    greet("Sym");
    ()
  end
end
```

---

## 7. 类型表达式 `TypeExpr`

```text
TypeExpr ::=
  | NamedType
  | FunType

NamedType ::= ident [ [ TypeExpr (, TypeExpr)* ,? ] ]
FunType   ::= ( TypeExpr (, TypeExpr)* ) -> TypeExpr
            | TypeExpr -> TypeExpr          (* 右结合，单参可省略括号 *)
```

- **构造类型应用**：`Option[Int]`、`List[Book]`。
- **函数类型**：`(Int, Int) -> Int` 或 `Int -> Int`（单参）。

**示例（高阶函数类型需显式标注参数类型时）：**

```sym
fn apply_twice(f: Int -> Int, x: Int) -> Int =
  f(f(x))
end

fn bump(n: Int) -> Int =
  n + 1
end

fn main() -> Unit =
  println(apply_twice(bump, 5))
end
```

---

## 8. 模式 `Pattern`（用于 `match`）

```text
Pattern ::=
  | _
  | ident
  | CtorName ( PatternField (, PatternField)* ,? )?

PatternField ::=
  | ident : Pattern
  | Pattern
```

- **`_`**：通配，不匹配绑定。
- **小写标识符**：将整个值绑定到该名。
- **`CtorName(...)`**：枚举构造；字段可 **命名**（`field: pat`）或 **位置**（与变体字段顺序一致）。
- **构造子命名**：以大写开头的标识符按构造子解析（与解析器规则一致）。

**示例（依赖 prelude 的 `Option` / `List`）：**

```sym
fn opt_u(x: Option[Int]) -> Int =
  match x
  | None => 0
  | Some(value: n) => n
  end
end

fn len(xs: List[Int]) -> Int =
  match xs
  | Nil => 0
  | Cons(head: _, tail: rest) => 1 + len(rest)
  end
end

fn main() -> Unit =
  do
    println(opt_u(Option[Int].Some(42)));
    println(len(List[Int].Cons(1, List[Int].Nil)));
    ()
  end
end
```

---

## 9. 表达式 `Expr`

### 9.1 优先级（从高到低，同级左结合除非说明）

1. **后缀**：函数调用 `f(a,b)`、构造 `Type.Var(args)` / `Type[T].Var(args)`（`args` 可空）。
2. **一元**：`!`、`+`、`-`。
3. **乘除模**：`*`、`/`、`%`。
4. **加减**：`+`、`-`。
5. **比较**：`<` `<=` `>` `>=`。
6. **相等**：`==`、`!=`。
7. **逻辑与**：`&&`（短路）。
8. **逻辑或**：`||`（短路）。

**示例（括号改变结合顺序）：**

```sym
fn prec() -> Int =
  1 + 2 * 3
end

fn prec2() -> Int =
  (1 + 2) * 3
end

fn main() -> Unit =
  do
    println(prec());
    println(prec2());
    ()
  end
end
```

### 9.2 形式一览（已实现）

本节中每个 **`sym` 代码块都是完整可编译单元**（均含 `main`），可单独存成 `.sym` 后用 `sym check` / `sym run` 验证。

| 语法 | 说明 |
|------|------|
| 字面量 | 整数、字符串、`true`/`false`、`()` |
| `ident` | 变量或函数名（由上下文与类型检查区分） |
| `Expr ( Expr (, Expr)* ,? )` | 调用 |
| `Expr BinOp Expr` | 二元运算 |
| `UnOp Expr` | 一元运算 |
| `if Expr then Expr else Expr` | 条件；**必须**含 `else`；可嵌套为 `else if` |
| `match Expr \| Arm \| Arm ... end` | `Arm ::= Pattern => Expr` |
| `let ident = Expr in Expr` | 表达式级 `let` |
| `do Stmt* Expr end` | 序列 |
| `while Expr do Expr end` | 循环；整体类型 **`Unit`** |
| `TypeName ... . Variant ( ... )` | 枚举构造 |
| `( Expr )` | 分组；**`()`** 单独为单元 |

**示例 — 调用与构造：**

```sym
fn calls() -> Unit =
  do
    println(string_from_int(strlen("ab")));
    let o = Option[Int].Some(7);
    println(o);
    ()
  end
end

fn main() -> Unit = calls() end
```

**示例 — `let ... in ...`（与 `do` 内 `let` 区分）：**

```sym
fn square_plus_one(n: Int) -> Int =
  let x = n * n in x + 1
end

fn main() -> Unit =
  println(square_plus_one(3))
end
```

**示例 — `do` / `while`（`while` 体须为 `Unit`，常用内层 `do`）：**

```sym
fn loop_demo() -> Unit =
  do
    while false do
      do
        println("skip");
        ()
      end
    end;
    ()
  end
end

fn main() -> Unit = loop_demo() end
```

**示例 — `match`：**

```sym
fn sign(n: Int) -> String =
  match n < 0
  | true => "neg"
  | false => "non-neg"
  end
end

fn main() -> Unit =
  println(sign(-1))
end
```

### 9.3 内建调用 `ident(...)`（保留名，不可 `fn` 重定义）

当前实现包括但不限于：`println`、`eprintln`、`exit`、`concat`、`string_from_int`、`strlen`、`read_line`、`assert`、`parse_int`、`env_get`、`read_file`、`write_file`、`trim`、`starts_with`、`substring`、`index_of`、`http_post`、`json_string`、`json_extract`（签名与语义见类型检查器与解释器）。

**示例：**

```sym
fn use_io() -> Unit =
  do
    assert(1 == 1, "ok");
    println(trim("  hi  "));
    ()
  end
end

fn main() -> Unit = use_io() end
```

---

## 10. `do` 块中的语句

```text
Stmt ::= let ident = Expr ;
       | Expr ;
```

- 尾部必须是 **表达式**，后跟 **`end`**，**无**分号。

**示例：**

```sym
fn block() -> Int =
  do
    let a = 1;
    let b = 2;
    println("side");
    a + b
  end
end

fn main() -> Unit =
  println(block())
end
```

---

## 11. 与实现相关的限制（当前）

- **`main`**：须存在、无参数。
- **`let ... in ...`** 与 **`do ... let ...;`** 两种绑定形式并存。
- **无** 可变赋值、无 `struct`/`class` 独立语法（数据用 ADT）。
- **无** 用户可见的 `module` 命名空间边界（多文件拼接为单模块类型检查）。

**示例（错误示范 — 不要写）：**

```text
# 不可用：x = x + 1
# 不可用：缺少 main
```

**合法最小入口：**

```sym
fn main() -> Unit = () end
```

---

## 12. 蓝图：未来可能扩展的语法（未实现）

下列条目 **仅作方向**，与 [`SOUL.md`](./SOUL.md) 一致者优先：**任何糖须能脱糖到显式语义，不引入隐藏运行时模型**。代码块均为 **设想语法**，当前 `sym` **不能**编译。

### 12.1 字面量与数据

- **列表字面量**：脱糖为 `Cons`/`Nil`。
- **记录 / 单变体结构体**。
- **更多标量**：`Float`、`Char` 等。

**设想示例：**

```sym
# （蓝图，当前不可用）
# let xs = [1, 2, 3]
# type Point = struct { x: Float, y: Float }
```

### 12.2 模式与 `match`

- **守卫**、**或模式**、**`as` 绑定**。

**设想示例：**

```sym
# （蓝图）
# match n
# | x if x > 0 => 1
# | Nil | Zero => 0
# end
```

### 12.3 控制流

- **`break` / `continue`**、**`for` 糖**。

**设想示例：**

```sym
# （蓝图）
# while cond do
#   if done then break else ()
# end
```

### 12.4 函数与抽象

- **泛型函数**、**trait / impl**。

**设想示例：**

```sym
# （蓝图）
# fn id[T](x: T) -> T = x end
```

### 12.5 模块与可见性

- **`pub` / `private`**、限定导入。

**设想示例：**

```sym
# （蓝图）
# import lib.core.{foo, bar}
# pub fn exported() -> Int = 0 end
```

### 12.6 注释与元数据

- **文档注释**、**属性**。

**设想示例：**

```sym
# （蓝图）
# ## 返回单位值
# @inline
# fn f() -> Unit = () end
```

### 12.7 错误与效应（类型层）

- **`Result[T, E]`**、**`?` 糖**。

**设想示例：**

```sym
# （蓝图）
# let y = may_fail()? + 1
```

### 12.8 字符串与文本

- **多行字符串**、**插值**。

**设想示例：**

```sym
# （蓝图）
# let s = "hello \(name)"
```

### 12.9 运算符

- **用户运算符**、**范围**。

**设想示例：**

```sym
# （蓝图）
# let r = 0..10
```

### 12.10 与工具的协定

- **AST JSON**、**格式化规范**等（无单独语法，属工具链）。

---

## 13. 版本与维护

- **实现源真理**：`crates/symc/src/lexer.rs`、`parser.rs`、`ast.rs`、`typeck.rs`。
- 本文 **§12** 随讨论演进；**已实现**部分应以仓库代码为准，若有出入以代码修正文档。

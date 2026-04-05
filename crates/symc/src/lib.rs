pub mod ast;
pub mod builtins;
pub mod bytecode;
pub mod interp;
pub mod lexer;
pub mod load;
pub mod parser;
pub mod sourcemap;
pub mod span;
pub mod typeck;

mod vm;

pub use load::{load_and_check, LoadOptions};

use serde_json::json;

use crate::ast::Module;
use crate::bytecode::CompileError;
use crate::interp::RuntimeError;
use crate::lexer::LexError;
use crate::parser::ParseError;
use crate::typeck::TypeError;

#[derive(Debug)]
pub enum SymError {
    Lex(LexError),
    Parse(ParseError),
    Type(TypeError),
    Runtime(RuntimeError),
    Io(String),
    ImportNotFound {
        parent: String,
        segments: String,
    },
    /// Bytecode compiler: unsupported construct or internal error.
    VmCompile(CompileError),
    /// VM execution error (stack, division by zero, etc.).
    VmRuntime(String),
}

fn line_col(source: &str, byte: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, c) in source.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

pub fn format_error(path: &str, source: &str, err: &SymError) -> String {
    match err {
        SymError::Lex(e) => {
            let (l, c) = line_col(source, e.offset);
            format!("{}:{}:{}: {}", path, l, c, e.message)
        }
        SymError::Parse(e) => {
            let (l, c) = line_col(source, e.span.start);
            format!("{}:{}:{}: {}", path, l, c, e.message)
        }
        SymError::Type(e) => {
            let (l, c) = line_col(source, e.span.start);
            format!("{}:{}:{}: {}", path, l, c, e.message)
        }
        SymError::Runtime(e) => {
            let (l, c) = line_col(source, e.span.start);
            format!("{}:{}:{}: {}", path, l, c, e.message)
        }
        SymError::Io(msg) => format!("{path}: io: {msg}"),
        SymError::ImportNotFound { parent, segments } => {
            format!("import not found: `{segments}` (from `{parent}`)")
        }
        SymError::VmCompile(e) => {
            let (l, c) = line_col(source, e.span.start);
            format!("{}:{}:{}: {}", path, l, c, e.message)
        }
        SymError::VmRuntime(msg) => format!("{path}: vm: {msg}"),
    }
}

/// Stable `kind` string for tooling (step 7: structured diagnostics).
pub fn error_kind(err: &SymError) -> &'static str {
    match err {
        SymError::Lex(_) => "lex",
        SymError::Parse(_) => "parse",
        SymError::Type(_) => "type",
        SymError::Runtime(_) => "runtime",
        SymError::Io(_) => "io",
        SymError::ImportNotFound { .. } => "import_not_found",
        SymError::VmCompile(_) => "vm_compile",
        SymError::VmRuntime(_) => "vm_runtime",
    }
}

/// One-line JSON diagnostic for CI / editors (stderr). Schema may grow; `kind` is stable.
fn attach_logical_fields(v: &mut serde_json::Value, source: &str, byte: usize) {
    if let Some((file, line, col)) = crate::sourcemap::logical_position(source, byte) {
        if let Some(m) = v.as_object_mut() {
            m.insert("logical_file".to_string(), json!(file));
            m.insert("logical_line".to_string(), json!(line));
            m.insert("logical_column".to_string(), json!(col));
        }
    }
}

pub fn format_error_json(path: &str, source: &str, err: &SymError) -> String {
    let v = match err {
        SymError::Lex(e) => {
            let (line, column) = line_col(source, e.offset);
            let mut o = json!({
                "path": path,
                "kind": error_kind(err),
                "message": e.message,
                "line": line,
                "column": column,
                "byte_offset": e.offset,
            });
            attach_logical_fields(&mut o, source, e.offset);
            o
        }
        SymError::Parse(e) => {
            let (line, column) = line_col(source, e.span.start);
            let mut o = json!({
                "path": path,
                "kind": error_kind(err),
                "message": e.message,
                "line": line,
                "column": column,
                "span": { "start": e.span.start, "end": e.span.end },
            });
            attach_logical_fields(&mut o, source, e.span.start);
            o
        }
        SymError::Type(e) => {
            let (line, column) = line_col(source, e.span.start);
            let mut o = json!({
                "path": path,
                "kind": error_kind(err),
                "message": e.message,
                "line": line,
                "column": column,
                "span": { "start": e.span.start, "end": e.span.end },
            });
            attach_logical_fields(&mut o, source, e.span.start);
            o
        }
        SymError::Runtime(e) => {
            let (line, column) = line_col(source, e.span.start);
            let mut o = json!({
                "path": path,
                "kind": error_kind(err),
                "message": e.message,
                "line": line,
                "column": column,
                "span": { "start": e.span.start, "end": e.span.end },
            });
            attach_logical_fields(&mut o, source, e.span.start);
            o
        }
        SymError::Io(msg) => json!({
            "path": path,
            "kind": error_kind(err),
            "message": msg,
        }),
        SymError::ImportNotFound { parent, segments } => json!({
            "path": path,
            "kind": error_kind(err),
            "message": format!("import not found: `{segments}` (from `{parent}`)"),
            "import": { "parent": parent, "segments": segments },
        }),
        SymError::VmCompile(e) => {
            let (line, column) = line_col(source, e.span.start);
            let mut o = json!({
                "path": path,
                "kind": error_kind(err),
                "message": e.message,
                "line": line,
                "column": column,
                "span": { "start": e.span.start, "end": e.span.end },
            });
            attach_logical_fields(&mut o, source, e.span.start);
            o
        }
        SymError::VmRuntime(msg) => json!({
            "path": path,
            "kind": error_kind(err),
            "message": msg,
        }),
    };
    serde_json::to_string(&v).unwrap_or_else(|_| {
        format!(r#"{{"path":{path:?},"kind":"internal","message":"json encoding failed"}}"#)
    })
}

pub fn parse_and_check(source: &str) -> Result<Module, SymError> {
    let tokens = lexer::lex(source).map_err(SymError::Lex)?;
    let mut p = parser::Parser::new(source, tokens);
    let module = p.parse_module().map_err(SymError::Parse)?;
    let mut tc = typeck::TypeChecker::new();
    tc.check_module(&module).map_err(SymError::Type)?;
    Ok(module)
}

pub fn run_module(module: &Module) -> Result<interp::Value, SymError> {
    let i = interp::Interpreter::new(module);
    i.run_main().map_err(SymError::Runtime)
}

/// Run `main` via stack bytecode when the program is VM-eligible; otherwise returns [`SymError::VmCompile`].
pub fn run_module_vm(module: &Module) -> Result<interp::Value, SymError> {
    let prog = bytecode::compile_module(module).map_err(SymError::VmCompile)?;
    vm::run(&prog).map_err(|e| SymError::VmRuntime(e.message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_string_ops() {
        let src = r#"
fn main() -> Unit =
  do
    println(concat("x", string_from_int(7)));
    ()
  end
end
"#;
        parse_and_check(src).expect("builtins");
    }

    #[test]
    fn short_circuit_and_skips_rhs() {
        let m = parse_and_check(r#"fn main() -> Bool = false && (1 / 0 > 0) end"#).unwrap();
        let i = crate::interp::Interpreter::new(&m);
        assert_eq!(i.run_main().unwrap(), crate::interp::Value::Bool(false));
    }

    #[test]
    fn short_circuit_or_skips_rhs() {
        let m = parse_and_check(r#"fn main() -> Bool = true || (1 / 0 > 0) end"#).unwrap();
        let i = crate::interp::Interpreter::new(&m);
        assert_eq!(i.run_main().unwrap(), crate::interp::Value::Bool(true));
    }

    #[test]
    fn syntax_comments_underscore_else_if_trailing_comma() {
        let src = r#"
// line comment
# hash comment
fn f(a: Int,) -> Int = +a + 1_000 end
fn main() -> Int =
  if false then 0 else if false then 1 else 2
end
"#;
        parse_and_check(src).expect("syntax extensions");
    }

    #[test]
    fn zero_arg_call_returns_value_type() {
        parse_and_check(
            r#"
fn pi() -> Int = 3 end
fn main() -> Int = pi() + 1 end
"#,
        )
        .expect("zero-arg call type");
    }

    #[test]
    fn string_ordering() {
        parse_and_check(r#"fn main() -> Bool = "ab" < "ac" end"#).expect("str cmp");
    }

    #[test]
    fn exit_returns_never_unifies_with_unit() {
        parse_and_check(r#"fn main() -> Unit = exit(0) end"#).expect("exit");
    }

    #[test]
    fn while_loop() {
        let src = r#"
fn main() -> Unit = while false do do (); () end end end
"#;
        parse_and_check(src).expect("while");
    }

    #[test]
    fn modulo_op() {
        let src = r#"
fn main() -> Int = 17 % 5 end
"#;
        parse_and_check(src).expect("mod");
    }

    #[test]
    fn parse_int_and_assert() {
        let prelude = include_str!("../../../stdlib/prelude.sym");
        let src = r#"
fn main() -> Unit =
  do
    assert(10 % 3 == 1, "mod");
    match parse_int(" -2 ")
    | None => assert(false, "parse_int")
    | Some(value: n) => assert(n == -2, "value")
    end;
    ()
  end
end
"#;
        parse_and_check(&format!("{prelude}\n\n{src}")).expect("parse_int assert");
    }

    #[test]
    fn vm_fib_12() {
        let src = r#"
fn fib(n: Int) -> Int =
  if n < 2 then n else fib(n - 1) + fib(n - 2)
end
fn main() -> Int = fib(12) end
"#;
        let m = parse_and_check(src).expect("vm fib parse");
        let v = run_module_vm(&m).expect("vm run");
        assert_eq!(v, crate::interp::Value::Int(144));
    }

    /// Bytecode `&&` / `||` must short-circuit (same as tree interpreter).
    #[test]
    fn vm_short_circuit_and_skips_rhs() {
        let m = parse_and_check(r#"fn main() -> Bool = false && (1 / 0 > 0) end"#).expect("parse");
        let v = run_module_vm(&m).expect("vm must not divide by zero");
        assert_eq!(v, crate::interp::Value::Bool(false));
    }

    #[test]
    fn vm_short_circuit_or_skips_rhs() {
        let m = parse_and_check(r#"fn main() -> Bool = true || (1 / 0 > 0) end"#).expect("parse");
        let v = run_module_vm(&m).expect("vm must not divide by zero");
        assert_eq!(v, crate::interp::Value::Bool(true));
    }

    #[test]
    fn vm_while_false_exits() {
        let src = r#"
fn main() -> Int =
  do
    while false do do (); () end end;
    42
  end
end
"#;
        let m = parse_and_check(src).expect("while parse");
        let v = run_module_vm(&m).expect("vm while");
        assert_eq!(v, crate::interp::Value::Int(42));
    }

    #[test]
    fn vm_while_call_cond() {
        let src = r#"
fn done() -> Bool = false end
fn main() -> Int =
  do
    while done() do do (); () end end;
    7
  end
end
"#;
        let m = parse_and_check(src).expect("parse");
        let v = run_module_vm(&m).expect("vm");
        assert_eq!(v, crate::interp::Value::Int(7));
    }

    fn assert_vm_matches_tree(src: &str) {
        let m = parse_and_check(src).expect("parse");
        let tree = run_module(&m).expect("tree");
        let bytecode = run_module_vm(&m).expect("vm");
        assert_eq!(tree, bytecode, "tree vs --vm disagree");
    }

    #[test]
    fn vm_parity_arith_if_let_call() {
        assert_vm_matches_tree(
            r#"
fn dbl(x: Int) -> Int = x * 2 end
fn main() -> Int =
  let a = 3 in
  if a < 10 then dbl(a) + 1 else 0
end
"#,
        );
    }

    #[test]
    fn vm_parity_string_main() {
        assert_vm_matches_tree(r#"fn main() -> String = "hello" end"#);
    }

    #[test]
    fn vm_parity_bool_ops() {
        assert_vm_matches_tree(r#"fn main() -> Bool = !false && (true || false) end"#);
    }

    #[test]
    fn vm_parity_string_ordering() {
        assert_vm_matches_tree(r#"fn main() -> Bool = "ab" < "ac" end"#);
    }

    #[test]
    fn vm_parity_eq_string_locals() {
        assert_vm_matches_tree(r#"fn main() -> Bool = let a = "hi" in let b = "hi" in a == b end"#);
    }

    #[test]
    fn vm_parity_eq_int_locals() {
        assert_vm_matches_tree(r#"fn main() -> Bool = let x = 7 in let y = 7 in x == y end"#);
    }

    #[test]
    fn vm_parity_concat_strlen_string_from_int() {
        assert_vm_matches_tree(
            r#"fn main() -> Bool = strlen(concat("x", string_from_int(7))) == 2 end"#,
        );
    }

    #[test]
    fn substring_index_of_unicode() {
        let src = r#"
fn main() -> Unit =
  do
    assert(substring("αβγδ", 1, 2) == "βγ", "substring mid");
    assert(substring("αβγδ", 10, 1) == "", "substring past end");
    assert(substring("αβ", 0, -1) == "αβ", "substring len neg");
    assert(index_of("αβγ", "β") == 1, "index_of single scalar");
    assert(index_of("hello", "ll") == 2, "index_of multi");
    assert(index_of("a", "bc") == -1, "index_of missing");
    ()
  end
end
"#;
        let m = parse_and_check(src).expect("substring index_of");
        let i = crate::interp::Interpreter::new(&m);
        i.run_main().expect("run");
    }

    /// Simulates stitched output of `math_lib` + `call_lib` (import line removed).
    #[test]
    fn stitched_import_order() {
        let stitched = concat!(
            "fn triple(x: Int) -> Int = x * 3 end\n\n",
            "fn main() -> Unit =\n",
            "  do\n",
            "    println(triple(4));\n",
            "    ()\n",
            "  end\n",
            "end\n",
        );
        parse_and_check(stitched).expect("stitched multi-file");
    }

    #[test]
    fn format_error_json_stable_kind() {
        let src = "fn main() -> Int = true end";
        let err = parse_and_check(src).expect_err("type error");
        let j = format_error_json("t.sym", src, &err);
        assert!(j.contains("\"kind\":\"type\""), "{j}");
        assert!(j.contains("\"path\":\"t.sym\""), "{j}");
    }

    #[test]
    fn format_error_json_logical_file_stitched() {
        let stitched = concat!(
            "# sym:file /x/a.sym\n",
            "fn main() -> Int = 1 end\n\n",
            "# sym:file /x/b.sym\n",
            "fn bad() -> Int = true end\n\n",
        );
        let err = parse_and_check(stitched).expect_err("type error");
        let j = format_error_json("entry.sym", stitched, &err);
        assert!(j.contains("\"logical_file\":\"/x/b.sym\""), "{j}");
        assert!(j.contains("\"logical_line\":1"), "{j}");
    }
}

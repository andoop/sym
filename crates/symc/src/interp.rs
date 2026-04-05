use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::ast::{
    BinOp, Expr, ExprKind, FnDef, Item, Module, Pattern, PatternField, Stmt, TypeExpr, UnaryOp,
};
use crate::bytecode::HostBuiltin;
use crate::span::Span;

/// Upper bound on `bind` calls for this pattern (used to pre-size the match arm frame map).
fn pattern_bind_count(pat: &Pattern) -> usize {
    match pat {
        Pattern::Wildcard => 0,
        Pattern::Bind(_) => 1,
        Pattern::Ctor { fields, .. } => fields
            .iter()
            .map(|f| match f {
                PatternField::Named(_, p) => pattern_bind_count(p.as_ref()),
                PatternField::Pos(p) => pattern_bind_count(p.as_ref()),
            })
            .sum(),
    }
}

/// `parse_int` result for the bytecode VM (mirrors `parse_int` builtin).
pub(crate) fn value_parse_int_string(s: &str) -> Value {
    match s.trim().parse::<i64>() {
        Ok(n) => value_option_some_int(n),
        Err(_) => value_option_none_int(),
    }
}

#[inline]
pub(crate) fn value_option_none_int() -> Value {
    Value::Enum {
        typ: "Option".into(),
        variant: "None".into(),
        fields: vec![],
    }
}

#[inline]
pub(crate) fn value_option_some_int(n: i64) -> Value {
    Value::Enum {
        typ: "Option".into(),
        variant: "Some".into(),
        fields: vec![("value".into(), Value::Int(n))],
    }
}

#[inline]
pub(crate) fn value_option_none_string() -> Value {
    value_option_none_int()
}

#[inline]
pub(crate) fn value_option_some_string(s: String) -> Value {
    Value::Enum {
        typ: "Option".into(),
        variant: "Some".into(),
        fields: vec![("value".into(), Value::String(s))],
    }
}

/// Run a host builtin for the stack VM (`args` = left-to-right parameters).
pub(crate) fn host_builtin_apply(b: HostBuiltin, args: &[Value]) -> Result<Value, String> {
    match b {
        HostBuiltin::ReadLine => {
            if !args.is_empty() {
                return Err("`read_line` expects no arguments".into());
            }
            let mut line = String::new();
            let stdin = std::io::stdin();
            let mut lock = stdin.lock();
            lock.read_line(&mut line)
                .map_err(|_| "read_line: io error".to_string())?;
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(Value::String(line))
        }
        HostBuiltin::EnvGet => {
            let Value::String(name) = &args[0] else {
                return Err("`env_get` expects String".into());
            };
            Ok(match std::env::var(name) {
                Ok(s) => value_option_some_string(s),
                Err(_) => value_option_none_string(),
            })
        }
        HostBuiltin::ReadFile => {
            let Value::String(path) = &args[0] else {
                return Err("`read_file` expects String".into());
            };
            Ok(match std::fs::read_to_string(path) {
                Ok(s) => value_option_some_string(s),
                Err(_) => value_option_none_string(),
            })
        }
        HostBuiltin::WriteFile => {
            let (Value::String(path), Value::String(content)) = (&args[0], &args[1]) else {
                return Err("`write_file` expects (String, String)".into());
            };
            std::fs::write(path, content.as_bytes())
                .map_err(|e| format!("write_file: {e}"))?;
            Ok(Value::Unit)
        }
        HostBuiltin::WriteFileOk => {
            let (Value::String(path), Value::String(content)) = (&args[0], &args[1]) else {
                return Err("`write_file_ok` expects (String, String)".into());
            };
            let ok = std::fs::write(path, content.as_bytes()).is_ok();
            Ok(Value::Bool(ok))
        }
        HostBuiltin::ListDir => {
            let Value::String(path) = &args[0] else {
                return Err("`list_dir` expects String".into());
            };
            Ok(match std::fs::read_dir(path) {
                Ok(rd) => {
                    let mut names: Vec<String> = rd
                        .filter_map(|e| e.ok())
                        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                        .filter_map(|e| {
                            let n = e.file_name().to_string_lossy().into_owned();
                            let is_dir = e.file_type().map(|t| t.is_dir()).ok()?;
                            Some(if is_dir { format!("{n}/") } else { n })
                        })
                        .collect();
                    names.sort();
                    value_option_some_string(names.join("\n"))
                }
                Err(_) => value_option_none_string(),
            })
        }
        HostBuiltin::GlobFiles => {
            let (Value::String(base), Value::String(pattern)) = (&args[0], &args[1]) else {
                return Err("`glob_files` expects (String, String)".into());
            };
            let path_pattern = std::path::Path::new(base).join(pattern);
            let pat = path_pattern.to_string_lossy().replace('\\', "/");
            Ok(match glob::glob(&pat) {
                Ok(paths) => {
                    let mut acc: Vec<String> = paths
                        .flatten()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                        .collect();
                    acc.sort();
                    value_option_some_string(acc.join("\n"))
                }
                Err(_) => value_option_none_string(),
            })
        }
        HostBuiltin::ShellExec => {
            let (Value::String(cwd), Value::String(cmd)) = (&args[0], &args[1]) else {
                return Err("`shell_exec` expects (cwd: String, command: String)".into());
            };
            Ok(shell_exec(cwd, cmd))
        }
        HostBuiltin::Trim => {
            let Value::String(s) = &args[0] else {
                return Err("`trim` expects String".into());
            };
            Ok(Value::String(s.trim().to_string()))
        }
        HostBuiltin::StartsWith => {
            let (Value::String(s), Value::String(pfx)) = (&args[0], &args[1]) else {
                return Err("`starts_with` expects (String, String)".into());
            };
            Ok(Value::Bool(s.starts_with(pfx.as_str())))
        }
        HostBuiltin::Substring => {
            let (Value::String(text), Value::Int(start), Value::Int(len)) =
                (&args[0], &args[1], &args[2])
            else {
                return Err("`substring` expects (String, Int, Int)".into());
            };
            let start = (*start).max(0) as usize;
            let it = text.chars().skip(start);
            let out: String = if *len < 0 {
                it.collect()
            } else {
                it.take((*len).max(0) as usize).collect()
            };
            Ok(Value::String(out))
        }
        HostBuiltin::IndexOf => {
            let (Value::String(hay), Value::String(needle)) = (&args[0], &args[1]) else {
                return Err("`index_of` expects (String, String)".into());
            };
            Ok(Value::Int(index_of_chars(hay, needle)))
        }
        HostBuiltin::HttpPost => {
            let (Value::String(url), Value::String(headers), Value::String(body)) =
                (&args[0], &args[1], &args[2])
            else {
                return Err("`http_post` expects (String, String, String)".into());
            };
            Ok(http_post(url, headers, body))
        }
        HostBuiltin::StdoutPrint => {
            let Value::String(s) = &args[0] else {
                return Err("`stdout_print` expects String".into());
            };
            print!("{s}");
            let _ = std::io::stdout().flush();
            Ok(Value::Unit)
        }
        HostBuiltin::JsonString => {
            let Value::String(s) = &args[0] else {
                return Err("`json_string` expects String".into());
            };
            Ok(Value::String(json_string_escape(s)))
        }
        HostBuiltin::JsonExtract => {
            let (Value::String(json), Value::String(path)) = (&args[0], &args[1]) else {
                return Err("`json_extract` expects (String, String)".into());
            };
            Ok(match json_extract_path(json, path) {
                Some(s) => value_option_some_string(s),
                None => value_option_none_string(),
            })
        }
        HostBuiltin::JsonValue => {
            let (Value::String(json), Value::String(path)) = (&args[0], &args[1]) else {
                return Err("`json_value` expects (String, String)".into());
            };
            Ok(match json_value_path(json, path) {
                Some(s) => value_option_some_string(s),
                None => value_option_none_string(),
            })
        }
        HostBuiltin::HttpPostSseFold => Err(
            "internal: HttpPostSseFold must be handled by the VM with nested calls".into(),
        ),
    }
}

pub(crate) fn http_post_sse_fold_with_reducer(
    url: &str,
    headers: &str,
    body: &str,
    state0: &str,
    mut reduce: impl FnMut(String, String) -> Result<String, String>,
) -> Result<Value, String> {
    let agent = sym_http_agent();
    let mut req = agent.post(url);
    for line in headers.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        if k.is_empty() {
            continue;
        }
        req = req.set(k, v);
    }
    let resp = match req.send_string(body) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return Ok(value_option_none_string());
        }
    };
    let status = resp.status();
    if !(200..300).contains(&status) {
        return match resp.into_string() {
            Ok(t) => Ok(value_option_some_string(t)),
            Err(e) => {
                eprintln!("{e}");
                Ok(value_option_none_string())
            }
        };
    }
    let ct = resp.header("content-type").unwrap_or("").to_lowercase();
    if !ct.contains("event-stream") && !ct.contains("text/event-stream") {
        return match resp.into_string() {
            Ok(t) => Ok(value_option_some_string(t)),
            Err(e) => {
                eprintln!("{e}");
                Ok(value_option_none_string())
            }
        };
    }
    let reader = std::io::BufReader::new(resp.into_reader());
    let mut state = state0.to_string();
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| format!("SSE read: {e}"))?;
        let line = line.trim_end();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let payload = if let Some(rest) = line.strip_prefix("data:") {
            rest.trim()
        } else {
            continue;
        };
        if payload == "[DONE]" {
            break;
        }
        if payload.is_empty() {
            continue;
        }
        state = reduce(state, payload.to_string())?;
    }
    Ok(value_option_some_string(state))
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Unit,
    Enum {
        typ: String,
        variant: String,
        fields: Vec<(String, Value)>,
    },
    FnRef(String),
}

#[derive(Debug)]
pub struct RuntimeError {
    pub span: Span,
    pub message: String,
}

/// Lexical environment for evaluation: all `fn` names live in `globals`; locals use a stack of
/// per-frame maps so we avoid cloning the whole environment on every `let` / `match` arm / call,
/// and variable lookup is O(1) average per frame instead of scanning the whole binding list.
pub struct EvalEnv {
    globals: HashMap<String, Value>,
    frames: Vec<HashMap<String, Value>>,
}

impl EvalEnv {
    fn with_globals_capacity(globals_cap: usize) -> Self {
        Self {
            globals: HashMap::with_capacity(globals_cap),
            frames: Vec::new(),
        }
    }

    fn insert_global(&mut self, name: String, v: Value) {
        self.globals.insert(name, v);
    }

    #[inline]
    fn get(&self, name: &str) -> Option<Value> {
        for fr in self.frames.iter().rev() {
            if let Some(v) = fr.get(name) {
                return Some(v.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    /// `bindings` = number of locals expected in this frame (`HashMap::with_capacity`).
    #[inline]
    fn push_frame(&mut self, bindings: usize) {
        self.frames.push(if bindings == 0 {
            HashMap::new()
        } else {
            HashMap::with_capacity(bindings)
        });
    }

    #[inline]
    fn pop_frame(&mut self) {
        self.frames.pop();
    }

    #[inline]
    fn bind(&mut self, name: String, v: Value) {
        self.frames
            .last_mut()
            .expect("EvalEnv::bind without active frame")
            .insert(name, v);
    }
}

pub struct Interpreter<'a> {
    pub fns: HashMap<String, &'a FnDef>,
    variants: HashMap<(String, String), Vec<String>>,
}

impl<'a> Interpreter<'a> {
    fn write_print_args(
        &self,
        out: &mut impl Write,
        args: &[Expr],
        env: &mut EvalEnv,
        span: Span,
    ) -> Result<(), RuntimeError> {
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                write!(out, " ").map_err(|_| RuntimeError {
                    span,
                    message: "io error".into(),
                })?;
            }
            let v = self.eval_expr(a, env)?;
            write!(out, "{v}").map_err(|_| RuntimeError {
                span,
                message: "io error".into(),
            })?;
        }
        writeln!(out).map_err(|_| RuntimeError {
            span,
            message: "io error".into(),
        })?;
        Ok(())
    }

    pub fn new(module: &'a Module) -> Self {
        let mut fns = HashMap::new();
        let mut variants = HashMap::new();
        for item in &module.items {
            match item {
                Item::Fn(f) => {
                    fns.insert(f.name.clone(), f);
                }
                Item::Type(td) => {
                    for v in &td.variants {
                        let names: Vec<_> = v.fields.iter().map(|f| f.name.clone()).collect();
                        variants.insert((td.name.clone(), v.name.clone()), names);
                    }
                }
            }
        }
        Self { fns, variants }
    }

    pub fn run_main(&self) -> Result<Value, RuntimeError> {
        let main = self.fns.get("main").ok_or_else(|| RuntimeError {
            span: Span::new(0, 0),
            message: "no `main` function defined".into(),
        })?;
        if !main.params.is_empty() {
            return Err(RuntimeError {
                span: main.span,
                message: "`main` must take no parameters".into(),
            });
        }
        let mut env = EvalEnv::with_globals_capacity(self.fns.len());
        for n in self.fns.keys() {
            env.insert_global(n.clone(), Value::FnRef(n.clone()));
        }
        self.eval_expr(&main.body, &mut env)
    }

    fn eval_expr(&self, e: &Expr, env: &mut EvalEnv) -> Result<Value, RuntimeError> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::String(s) => Ok(Value::String(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Unit => Ok(Value::Unit),
            ExprKind::Var(name) => env.get(name).ok_or_else(|| RuntimeError {
                span: e.span,
                message: format!("unknown variable `{name}`"),
            }),
            ExprKind::Binary { op, left, right } => match op {
                BinOp::And => {
                    let lv = self.eval_expr(left, env)?;
                    match lv {
                        Value::Bool(false) => Ok(Value::Bool(false)),
                        Value::Bool(true) => {
                            let rv = self.eval_expr(right, env)?;
                            match rv {
                                Value::Bool(b) => Ok(Value::Bool(b)),
                                _ => Err(RuntimeError {
                                    span: right.span,
                                    message: "`&&` right side must be Bool".into(),
                                }),
                            }
                        }
                        _ => Err(RuntimeError {
                            span: left.span,
                            message: "`&&` left side must be Bool".into(),
                        }),
                    }
                }
                BinOp::Or => {
                    let lv = self.eval_expr(left, env)?;
                    match lv {
                        Value::Bool(true) => Ok(Value::Bool(true)),
                        Value::Bool(false) => {
                            let rv = self.eval_expr(right, env)?;
                            match rv {
                                Value::Bool(b) => Ok(Value::Bool(b)),
                                _ => Err(RuntimeError {
                                    span: right.span,
                                    message: "`||` right side must be Bool".into(),
                                }),
                            }
                        }
                        _ => Err(RuntimeError {
                            span: left.span,
                            message: "`||` left side must be Bool".into(),
                        }),
                    }
                }
                _ => {
                    let lv = self.eval_expr(left, env)?;
                    let rv = self.eval_expr(right, env)?;
                    self.eval_binop(*op, lv, rv, e.span)
                }
            },
            ExprKind::Unary { op, expr } => {
                let v = self.eval_expr(expr, env)?;
                self.eval_unary(*op, v, e.span)
            }
            ExprKind::If {
                cond,
                then_arm,
                else_arm,
            } => {
                let c = self.eval_expr(cond, env)?;
                match c {
                    Value::Bool(true) => self.eval_expr(then_arm, env),
                    Value::Bool(false) => self.eval_expr(else_arm, env),
                    _ => Err(RuntimeError {
                        span: cond.span,
                        message: "`if` condition must be Bool".into(),
                    }),
                }
            }
            ExprKind::While { cond, body } => {
                loop {
                    let c = self.eval_expr(cond, env)?;
                    match c {
                        Value::Bool(false) => break,
                        Value::Bool(true) => {
                            self.eval_expr(body, env)?;
                        }
                        _ => {
                            return Err(RuntimeError {
                                span: cond.span,
                                message: "`while` condition must be Bool".into(),
                            });
                        }
                    }
                }
                Ok(Value::Unit)
            }
            ExprKind::Let { name, value, body } => {
                let v = self.eval_expr(value, env)?;
                env.push_frame(1);
                env.bind(name.clone(), v);
                let r = self.eval_expr(body, env);
                env.pop_frame();
                r
            }
            ExprKind::Block { stmts, tail } => {
                let let_count = stmts
                    .iter()
                    .filter(|s| matches!(s, Stmt::Let { .. }))
                    .count();
                env.push_frame(let_count);
                for s in stmts {
                    match s {
                        Stmt::Let { name, value } => {
                            let v = self.eval_expr(value, env)?;
                            env.bind(name.clone(), v);
                        }
                        Stmt::Expr(ex) => {
                            self.eval_expr(ex, env)?;
                        }
                    }
                }
                let r = self.eval_expr(tail, env);
                env.pop_frame();
                r
            }
            ExprKind::Match { scrutinee, arms } => {
                let sv = self.eval_expr(scrutinee, env)?;
                for arm in arms {
                    env.push_frame(pattern_bind_count(&arm.pattern));
                    match self.bind_pattern(&arm.pattern, &sv, env, arm.span) {
                        Ok(true) => {
                            let r = self.eval_expr(&arm.body, env);
                            env.pop_frame();
                            return r;
                        }
                        Ok(false) => env.pop_frame(),
                        Err(err) => {
                            env.pop_frame();
                            return Err(err);
                        }
                    }
                }
                Err(RuntimeError {
                    span: e.span,
                    message: "`match` fell through (interpreter)".into(),
                })
            }
            ExprKind::Construct { typ, variant, args } => {
                let TypeExpr::Named { name: tn, args: _ } = typ else {
                    return Err(RuntimeError {
                        span: e.span,
                        message: "invalid constructed type".into(),
                    });
                };
                let key = (tn.clone(), variant.clone());
                let field_names = self.variants.get(&key).ok_or_else(|| RuntimeError {
                    span: e.span,
                    message: format!("unknown variant `{variant}` on `{tn}`"),
                })?;
                if field_names.len() != args.len() {
                    return Err(RuntimeError {
                        span: e.span,
                        message: "constructor argument count mismatch".into(),
                    });
                }
                let mut fields = Vec::new();
                for (fname, ae) in field_names.iter().zip(args.iter()) {
                    let v = self.eval_expr(ae, env)?;
                    fields.push((fname.clone(), v));
                }
                Ok(Value::Enum {
                    typ: tn.clone(),
                    variant: variant.clone(),
                    fields,
                })
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Var(name) = &callee.kind {
                    match name.as_str() {
                        "println" => {
                            let mut out = std::io::stdout().lock();
                            self.write_print_args(&mut out, args, env, e.span)?;
                            return Ok(Value::Unit);
                        }
                        "eprintln" => {
                            let mut out = std::io::stderr().lock();
                            self.write_print_args(&mut out, args, env, e.span)?;
                            return Ok(Value::Unit);
                        }
                        "exit" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::Int(n) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`exit` expects Int".into(),
                                });
                            };
                            std::process::exit(n as i32);
                        }
                        "concat" => {
                            let a0 = self.eval_expr(&args[0], env)?;
                            let a1 = self.eval_expr(&args[1], env)?;
                            return match (a0, a1) {
                                (Value::String(s0), Value::String(s1)) => {
                                    Ok(Value::String(format!("{s0}{s1}")))
                                }
                                _ => Err(RuntimeError {
                                    span: e.span,
                                    message: "`concat` expects two strings".into(),
                                }),
                            };
                        }
                        "string_from_int" => {
                            let v = self.eval_expr(&args[0], env)?;
                            return match v {
                                Value::Int(n) => Ok(Value::String(n.to_string())),
                                _ => Err(RuntimeError {
                                    span: e.span,
                                    message: "`string_from_int` expects Int".into(),
                                }),
                            };
                        }
                        "strlen" => {
                            let v = self.eval_expr(&args[0], env)?;
                            return match v {
                                Value::String(s) => Ok(Value::Int(s.chars().count() as i64)),
                                _ => Err(RuntimeError {
                                    span: e.span,
                                    message: "`strlen` expects String".into(),
                                }),
                            };
                        }
                        "read_line" => {
                            let mut line = String::new();
                            let stdin = std::io::stdin();
                            let mut lock = stdin.lock();
                            lock.read_line(&mut line).map_err(|_| RuntimeError {
                                span: e.span,
                                message: "read_line: io error".into(),
                            })?;
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            }
                            return Ok(Value::String(line));
                        }
                        "assert" => {
                            let c = self.eval_expr(&args[0], env)?;
                            let m = self.eval_expr(&args[1], env)?;
                            let Value::Bool(ok) = c else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`assert` condition must be Bool".into(),
                                });
                            };
                            let Value::String(msg) = m else {
                                return Err(RuntimeError {
                                    span: args[1].span,
                                    message: "`assert` message must be String".into(),
                                });
                            };
                            if !ok {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: format!("assertion failed: {msg}"),
                                });
                            }
                            return Ok(Value::Unit);
                        }
                        "parse_int" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(s) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`parse_int` expects String".into(),
                                });
                            };
                            return Ok(match s.trim().parse::<i64>() {
                                Ok(n) => value_option_some_int(n),
                                Err(_) => value_option_none_int(),
                            });
                        }
                        "env_get" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(name) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`env_get` expects String".into(),
                                });
                            };
                            return Ok(match std::env::var(&name) {
                                Ok(s) => value_option_some_string(s),
                                Err(_) => value_option_none_string(),
                            });
                        }
                        "read_file" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(path) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`read_file` expects String".into(),
                                });
                            };
                            return Ok(match std::fs::read_to_string(&path) {
                                Ok(s) => value_option_some_string(s),
                                Err(_) => value_option_none_string(),
                            });
                        }
                        "write_file" => {
                            let p = self.eval_expr(&args[0], env)?;
                            let c = self.eval_expr(&args[1], env)?;
                            let (Value::String(path), Value::String(content)) = (p, c) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`write_file` expects (String, String)".into(),
                                });
                            };
                            std::fs::write(&path, content.as_bytes()).map_err(|err| {
                                RuntimeError {
                                    span: e.span,
                                    message: format!("write_file: {err}"),
                                }
                            })?;
                            return Ok(Value::Unit);
                        }
                        "write_file_ok" => {
                            let p = self.eval_expr(&args[0], env)?;
                            let c = self.eval_expr(&args[1], env)?;
                            let (Value::String(path), Value::String(content)) = (p, c) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`write_file_ok` expects (String, String)".into(),
                                });
                            };
                            let ok = std::fs::write(&path, content.as_bytes()).is_ok();
                            return Ok(Value::Bool(ok));
                        }
                        "list_dir" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(path) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`list_dir` expects String".into(),
                                });
                            };
                            return Ok(match std::fs::read_dir(&path) {
                                Ok(rd) => {
                                    let mut names: Vec<String> = rd
                                        .filter_map(|e| e.ok())
                                        .filter(|e| {
                                            !e.file_name().to_string_lossy().starts_with('.')
                                        })
                                        .filter_map(|e| {
                                            let n = e.file_name().to_string_lossy().into_owned();
                                            let is_dir = e.file_type().map(|t| t.is_dir()).ok()?;
                                            Some(if is_dir { format!("{n}/") } else { n })
                                        })
                                        .collect();
                                    names.sort();
                                    value_option_some_string(names.join("\n"))
                                }
                                Err(_) => value_option_none_string(),
                            });
                        }
                        "glob_files" => {
                            let a0 = self.eval_expr(&args[0], env)?;
                            let a1 = self.eval_expr(&args[1], env)?;
                            let (Value::String(base), Value::String(pattern)) = (a0, a1) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`glob_files` expects (String, String)".into(),
                                });
                            };
                            let path_pattern = std::path::Path::new(&base).join(&pattern);
                            let pat = path_pattern.to_string_lossy().replace('\\', "/");
                            return Ok(match glob::glob(&pat) {
                                Ok(paths) => {
                                    let mut acc: Vec<String> = paths
                                        .flatten()
                                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                                        .collect();
                                    acc.sort();
                                    value_option_some_string(acc.join("\n"))
                                }
                                Err(_) => value_option_none_string(),
                            });
                        }
                        "shell_exec" => {
                            let a0 = self.eval_expr(&args[0], env)?;
                            let a1 = self.eval_expr(&args[1], env)?;
                            let (Value::String(cwd), Value::String(cmd)) = (a0, a1) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`shell_exec` expects (cwd: String, command: String)"
                                        .into(),
                                });
                            };
                            return Ok(shell_exec(&cwd, &cmd));
                        }
                        "trim" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(s) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`trim` expects String".into(),
                                });
                            };
                            return Ok(Value::String(s.trim().to_string()));
                        }
                        "starts_with" => {
                            let a = self.eval_expr(&args[0], env)?;
                            let b = self.eval_expr(&args[1], env)?;
                            let (Value::String(s), Value::String(pfx)) = (a, b) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`starts_with` expects (String, String)".into(),
                                });
                            };
                            return Ok(Value::Bool(s.starts_with(&pfx)));
                        }
                        "substring" => {
                            let s = self.eval_expr(&args[0], env)?;
                            let st = self.eval_expr(&args[1], env)?;
                            let ln = self.eval_expr(&args[2], env)?;
                            let (Value::String(text), Value::Int(start), Value::Int(len)) =
                                (s, st, ln)
                            else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`substring` expects (String, Int, Int)".into(),
                                });
                            };
                            let start = start.max(0) as usize;
                            let it = text.chars().skip(start);
                            let out: String = if len < 0 {
                                it.collect()
                            } else {
                                it.take(len.max(0) as usize).collect()
                            };
                            return Ok(Value::String(out));
                        }
                        "index_of" => {
                            let a = self.eval_expr(&args[0], env)?;
                            let b = self.eval_expr(&args[1], env)?;
                            let (Value::String(hay), Value::String(needle)) = (a, b) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`index_of` expects (String, String)".into(),
                                });
                            };
                            let idx = index_of_chars(&hay, &needle);
                            return Ok(Value::Int(idx));
                        }
                        "http_post" => {
                            let u = self.eval_expr(&args[0], env)?;
                            let h = self.eval_expr(&args[1], env)?;
                            let b = self.eval_expr(&args[2], env)?;
                            let (Value::String(url), Value::String(headers), Value::String(body)) =
                                (u, h, b)
                            else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`http_post` expects (String, String, String)".into(),
                                });
                            };
                            return Ok(http_post(&url, &headers, &body));
                        }
                        "stdout_print" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(s) = v else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`stdout_print` expects String".into(),
                                });
                            };
                            print!("{s}");
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Unit);
                        }
                        "http_post_sse_fold" => {
                            let u = self.eval_expr(&args[0], env)?;
                            let h = self.eval_expr(&args[1], env)?;
                            let b = self.eval_expr(&args[2], env)?;
                            let s0 = self.eval_expr(&args[3], env)?;
                            let red = self.eval_expr(&args[4], env)?;
                            let (
                                Value::String(url),
                                Value::String(headers),
                                Value::String(body),
                                Value::String(state0),
                                Value::FnRef(reducer),
                            ) = (u, h, b, s0, red)
                            else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`http_post_sse_fold` expects (url: String, headers: String, body: String, state0: String, reducer: fn(String, String) -> String)".into(),
                                });
                            };
                            return self.http_post_sse_fold(
                                &url, &headers, &body, &state0, &reducer, env, e.span,
                            );
                        }
                        "json_string" => {
                            let v = self.eval_expr(&args[0], env)?;
                            let Value::String(s) = v else {
                                return Err(RuntimeError {
                                    span: args[0].span,
                                    message: "`json_string` expects String".into(),
                                });
                            };
                            return Ok(Value::String(json_string_escape(&s)));
                        }
                        "json_extract" => {
                            let j = self.eval_expr(&args[0], env)?;
                            let p = self.eval_expr(&args[1], env)?;
                            let (Value::String(json), Value::String(path)) = (j, p) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`json_extract` expects (String, String)".into(),
                                });
                            };
                            return Ok(match json_extract_path(&json, &path) {
                                Some(s) => value_option_some_string(s),
                                None => value_option_none_string(),
                            });
                        }
                        "json_value" => {
                            let j = self.eval_expr(&args[0], env)?;
                            let p = self.eval_expr(&args[1], env)?;
                            let (Value::String(json), Value::String(path)) = (j, p) else {
                                return Err(RuntimeError {
                                    span: e.span,
                                    message: "`json_value` expects (String, String)".into(),
                                });
                            };
                            return Ok(match json_value_path(&json, &path) {
                                Some(s) => value_option_some_string(s),
                                None => value_option_none_string(),
                            });
                        }
                        _ => {}
                    }
                }
                let fv = self.eval_expr(callee, env)?;
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(a, env)?);
                }
                match fv {
                    Value::FnRef(fname) => {
                        let fd = self.fns.get(&fname).ok_or_else(|| RuntimeError {
                            span: callee.span,
                            message: format!("unknown function `{fname}`"),
                        })?;
                        if fd.params.len() != arg_vals.len() {
                            return Err(RuntimeError {
                                span: e.span,
                                message: format!(
                                    "`{}` expects {} argument(s), got {}",
                                    fname,
                                    fd.params.len(),
                                    arg_vals.len()
                                ),
                            });
                        }
                        env.push_frame(fd.params.len());
                        for (p, v) in fd.params.iter().zip(arg_vals.into_iter()) {
                            env.bind(p.name.clone(), v);
                        }
                        let r = self.eval_expr(&fd.body, env);
                        env.pop_frame();
                        r
                    }
                    _ => Err(RuntimeError {
                        span: callee.span,
                        message: "call target is not a function".into(),
                    }),
                }
            }
        }
    }

    fn call_named_with_env(
        &self,
        fname: &str,
        arg_vals: Vec<Value>,
        env: &mut EvalEnv,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let fd = self.fns.get(fname).ok_or_else(|| RuntimeError {
            span,
            message: format!("unknown function `{fname}`"),
        })?;
        if fd.params.len() != arg_vals.len() {
            return Err(RuntimeError {
                span,
                message: format!(
                    "`{fname}` expects {} argument(s), got {}",
                    fd.params.len(),
                    arg_vals.len()
                ),
            });
        }
        env.push_frame(fd.params.len());
        for (p, v) in fd.params.iter().zip(arg_vals.into_iter()) {
            env.bind(p.name.clone(), v);
        }
        let r = self.eval_expr(&fd.body, env);
        env.pop_frame();
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn http_post_sse_fold(
        &self,
        url: &str,
        headers: &str,
        body: &str,
        state0: &str,
        reducer_name: &str,
        env: &mut EvalEnv,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let rname = reducer_name.to_string();
        http_post_sse_fold_with_reducer(url, headers, body, state0, |state, payload| {
            match self.call_named_with_env(
                &rname,
                vec![Value::String(state), Value::String(payload)],
                env,
                span,
            ) {
                Ok(Value::String(s)) => Ok(s),
                Ok(_) => Err("SSE fold reducer must return String".into()),
                Err(e) => Err(e.message),
            }
        })
        .map_err(|msg| RuntimeError { span, message: msg })
    }

    fn bind_pattern(
        &self,
        pat: &Pattern,
        value: &Value,
        env: &mut EvalEnv,
        span: Span,
    ) -> Result<bool, RuntimeError> {
        Ok(match pat {
            Pattern::Wildcard => true,
            Pattern::Bind(n) => {
                env.bind(n.clone(), value.clone());
                true
            }
            Pattern::Ctor { name, fields } => {
                let Value::Enum {
                    typ: _,
                    variant,
                    fields: vals,
                } = value
                else {
                    return Err(RuntimeError {
                        span,
                        message: "pattern mismatch: expected enum value".into(),
                    });
                };
                if variant != name {
                    return Ok(false);
                }
                let has_named = fields
                    .iter()
                    .any(|f| matches!(f, PatternField::Named(_, _)));
                let has_pos = fields.iter().any(|f| matches!(f, PatternField::Pos(_)));
                if has_named && has_pos {
                    return Err(RuntimeError {
                        span,
                        message: "invalid pattern (mixed named/positional)".into(),
                    });
                }
                if has_named {
                    for f in fields {
                        if let PatternField::Named(n, p) = f {
                            let (_, fv) =
                                vals.iter().find(|(fnm, _)| fnm == n).ok_or_else(|| {
                                    RuntimeError {
                                        span,
                                        message: format!("missing field `{n}` in value"),
                                    }
                                })?;
                            if !self.bind_pattern(p, fv, env, span)? {
                                return Ok(false);
                            }
                        }
                    }
                } else {
                    if fields.len() != vals.len() {
                        return Err(RuntimeError {
                            span,
                            message: "pattern field count mismatch".into(),
                        });
                    }
                    for (pf, (_, fv)) in fields.iter().zip(vals.iter()) {
                        let p = match pf {
                            PatternField::Pos(p) => p.as_ref(),
                            PatternField::Named(_, _) => {
                                return Err(RuntimeError {
                                    span,
                                    message: "internal pattern error".into(),
                                });
                            }
                        };
                        if !self.bind_pattern(p, fv, env, span)? {
                            return Ok(false);
                        }
                    }
                }
                true
            }
        })
    }

    fn eval_binop(&self, op: BinOp, l: Value, r: Value, span: Span) -> Result<Value, RuntimeError> {
        match op {
            BinOp::Add => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                _ => Err(RuntimeError {
                    span,
                    message: "`+` expects Int".into(),
                }),
            },
            BinOp::Sub => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                _ => Err(RuntimeError {
                    span,
                    message: "`-` expects Int".into(),
                }),
            },
            BinOp::Mul => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                _ => Err(RuntimeError {
                    span,
                    message: "`*` expects Int".into(),
                }),
            },
            BinOp::Div => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(RuntimeError {
                            span,
                            message: "division by zero".into(),
                        })
                    } else {
                        Ok(Value::Int(a / b))
                    }
                }
                _ => Err(RuntimeError {
                    span,
                    message: "`/` expects Int".into(),
                }),
            },
            BinOp::Mod => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(RuntimeError {
                            span,
                            message: "modulo by zero".into(),
                        })
                    } else {
                        Ok(Value::Int(a.rem_euclid(b)))
                    }
                }
                _ => Err(RuntimeError {
                    span,
                    message: "`%` expects Int".into(),
                }),
            },
            BinOp::Eq => Ok(Value::Bool(l == r)),
            BinOp::Ne => Ok(Value::Bool(l != r)),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(match op {
                    BinOp::Lt => a < b,
                    BinOp::Le => a <= b,
                    BinOp::Gt => a > b,
                    BinOp::Ge => a >= b,
                    _ => false,
                })),
                (Value::String(a), Value::String(b)) => {
                    let o = a.cmp(&b);
                    Ok(Value::Bool(match op {
                        BinOp::Lt => o == Ordering::Less,
                        BinOp::Le => matches!(o, Ordering::Less | Ordering::Equal),
                        BinOp::Gt => o == Ordering::Greater,
                        BinOp::Ge => matches!(o, Ordering::Greater | Ordering::Equal),
                        _ => false,
                    }))
                }
                _ => Err(RuntimeError {
                    span,
                    message: "comparison expects two Int or two String".into(),
                }),
            },
            BinOp::And | BinOp::Or => Err(RuntimeError {
                span,
                message: "internal: `&&`/`||` must be evaluated in eval_expr".into(),
            }),
        }
    }

    fn eval_unary(&self, op: UnaryOp, v: Value, span: Span) -> Result<Value, RuntimeError> {
        match op {
            UnaryOp::Not => match v {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(RuntimeError {
                    span,
                    message: "`!` expects Bool".into(),
                }),
            },
            UnaryOp::Neg => match v {
                Value::Int(n) => Ok(Value::Int(-n)),
                _ => Err(RuntimeError {
                    span,
                    message: "unary `-` expects Int".into(),
                }),
            },
            UnaryOp::Pos => match v {
                Value::Int(n) => Ok(Value::Int(n)),
                _ => Err(RuntimeError {
                    span,
                    message: "unary `+` expects Int".into(),
                }),
            },
        }
    }
}

fn index_of_chars(hay: &str, needle: &str) -> i64 {
    if needle.is_empty() {
        return 0;
    }
    let mut nc_iter = needle.chars();
    let Some(n0) = nc_iter.next() else {
        return 0;
    };
    let n_rest: Vec<char> = nc_iter.collect();
    if n_rest.is_empty() {
        // Single scalar: one pass, no hay allocation.
        for (i, c) in hay.chars().enumerate() {
            if c == n0 {
                return i as i64;
            }
        }
        return -1;
    }
    let mut nc: Vec<char> = Vec::with_capacity(1 + n_rest.len());
    nc.push(n0);
    nc.extend(n_rest);
    let hc: Vec<char> = hay.chars().collect();
    let max = hc.len().saturating_sub(nc.len());
    'outer: for i in 0..=max {
        for j in 0..nc.len() {
            if hc[i + j] != nc[j] {
                continue 'outer;
            }
        }
        return i as i64;
    }
    -1
}

fn json_string_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(2));
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_navigate<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut cur = v;
    for seg in path.split('.').filter(|s| !s.is_empty()) {
        cur = if let Ok(i) = seg.parse::<usize>() {
            cur.get(i)?
        } else {
            cur.get(seg)?
        };
    }
    Some(cur)
}

fn json_extract_path(json: &str, path: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let cur = json_navigate(&v, path)?;
    match cur {
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// JSON 子树序列化（`json_extract` 仅字符串叶；本内建用于数组/对象等任意类型）。
fn json_value_path(json: &str, path: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let cur = json_navigate(&v, path)?;
    Some(cur.to_string())
}

/// 在 `cwd` 下执行一条 shell 命令（POSIX 用 `sh -lc`，Windows 用 `cmd /C`）。返回合并后的文本报告（含退出码）。
/// 若环境变量 `CCODE_DISABLE_SHELL=1` 则拒绝执行。输出截断约 512KiB。
fn shell_exec(cwd: &str, command: &str) -> Value {
    const OUT_CAP: usize = 512 * 1024;
    if std::env::var("CCODE_DISABLE_SHELL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return value_option_some_string(
            "[exit -1]\n--- stderr ---\nshell disabled: unset CCODE_DISABLE_SHELL or set it to 0 to allow shell_exec\n".into(),
        );
    }
    let cmd = command.trim();
    if cmd.is_empty() {
        return value_option_some_string(
            "[exit -1]\n--- stderr ---\nempty command\n".into(),
        );
    }
    let root = cwd.trim();
    let root = if root.is_empty() { "." } else { root };

    let mut child = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", cmd])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    } else {
        let mut c = Command::new("/bin/sh");
        c.args(["-lc", cmd])
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    };

    let output = match child.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return value_option_none_string();
        }
    };
    let code = output.status.code().unwrap_or(-1);
    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let mut note = String::new();
    let total = stdout.len() + stderr.len();
    if total > OUT_CAP {
        let take = OUT_CAP.saturating_sub(256);
        if stdout.len() >= take {
            stdout.truncate(take);
            stdout.push_str("\n... (stdout truncated)\n");
            stderr.clear();
            note = "combined output exceeded cap; stderr omitted after stdout truncation\n".into();
        } else {
            let rest = take - stdout.len();
            stderr.truncate(rest);
            stderr.push_str("\n... (stderr truncated)\n");
        }
    }
    let mut s = format!("[exit {code}]\n");
    if !note.is_empty() {
        s.push_str("--- note ---\n");
        s.push_str(&note);
    }
    if !stderr.is_empty() {
        s.push_str("--- stderr ---\n");
        s.push_str(&stderr);
        if !stderr.ends_with('\n') {
            s.push('\n');
        }
    }
    if !stdout.is_empty() {
        s.push_str("--- stdout ---\n");
        s.push_str(&stdout);
        if !stdout.ends_with('\n') {
            s.push('\n');
        }
    }
    if stdout.is_empty() && stderr.is_empty() {
        s.push_str("(no output)\n");
    }
    value_option_some_string(s)
}

/// 与 curl 类似：识别常见代理环境变量（ureq 默认 Agent 不会自动读它们）。
fn sym_http_agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new().timeout(Duration::from_secs(120));
    for key in [
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ] {
        if let Ok(px) = std::env::var(key) {
            let px = px.trim();
            if px.is_empty() {
                continue;
            }
            match ureq::Proxy::new(px) {
                Ok(p) => {
                    builder = builder.proxy(p);
                    break;
                }
                Err(e) => eprintln!("{e}"),
            }
        }
    }
    builder.build()
}

fn http_post(url: &str, headers: &str, body: &str) -> Value {
    let agent = sym_http_agent();
    let mut req = agent.post(url);
    for line in headers.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        if k.is_empty() {
            continue;
        }
        req = req.set(k, v);
    }
    let resp = match req.send_string(body) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return value_option_none_string();
        }
    };
    // 非 2xx 仍返回响应体，便于上层解析 JSON 错误字段。
    match resp.into_string() {
        Ok(t) => value_option_some_string(t),
        Err(e) => {
            eprintln!("{e}");
            value_option_none_string()
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Unit => write!(f, "()"),
            Value::Enum {
                typ,
                variant,
                fields,
            } => {
                if fields.is_empty() {
                    write!(f, "{typ}::{variant}")
                } else {
                    write!(f, "{typ}::{variant}(")?;
                    for (i, (_, v)) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{v}")?;
                    }
                    write!(f, ")")
                }
            }
            Value::FnRef(n) => write!(f, "<fn {n}>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (
                Value::Enum {
                    typ: t1,
                    variant: v1,
                    fields: f1,
                },
                Value::Enum {
                    typ: t2,
                    variant: v2,
                    fields: f2,
                },
            ) => t1 == t2 && v1 == v2 && f1 == f2,
            (Value::FnRef(a), Value::FnRef(b)) => a == b,
            _ => false,
        }
    }
}

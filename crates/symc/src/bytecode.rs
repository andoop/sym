//! Stack bytecode for Sym. Unsupported constructs return [`CompileError`].

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, ExprKind, FnDef, Item, Module, Param, Stmt, TypeExpr, UnaryOp};
use crate::span::Span;

#[derive(Debug)]
pub struct CompileError {
    pub span: Span,
    pub message: String,
}

/// Host (I/O, FS, HTTP, JSON, …) builtins shared by the tree interpreter and the VM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostBuiltin {
    ReadLine,
    EnvGet,
    ReadFile,
    WriteFile,
    WriteFileOk,
    ListDir,
    GlobFiles,
    ShellExec,
    Trim,
    StartsWith,
    Substring,
    IndexOf,
    HttpPost,
    StdoutPrint,
    JsonString,
    JsonExtract,
    JsonValue,
    HttpPostSseFold,
}

impl HostBuiltin {
    pub const fn argc(self) -> u8 {
        match self {
            HostBuiltin::ReadLine => 0,
            HostBuiltin::EnvGet
            | HostBuiltin::ReadFile
            | HostBuiltin::ListDir
            | HostBuiltin::Trim
            | HostBuiltin::JsonString
            | HostBuiltin::StdoutPrint => 1,
            HostBuiltin::WriteFile
            | HostBuiltin::WriteFileOk
            | HostBuiltin::GlobFiles
            | HostBuiltin::ShellExec
            | HostBuiltin::StartsWith
            | HostBuiltin::IndexOf
            | HostBuiltin::JsonExtract
            | HostBuiltin::JsonValue => 2,
            HostBuiltin::Substring | HostBuiltin::HttpPost => 3,
            HostBuiltin::HttpPostSseFold => 5,
        }
    }
}

/// Relational / equality on arbitrary [`crate::interp::Value`] (Int, String, Bool, Unit, …).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValCmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug)]
pub enum Instr {
    PushInt(i64),
    PushBool(bool),
    PushUnit,
    /// Index into current chunk's string pool.
    PushStr(usize),
    LoadLocal(u8),
    StoreLocal(u8),
    /// Drop one stack value (e.g. after expression statement).
    Pop,
    AddI,
    SubI,
    MulI,
    DivI,
    ModI,
    EqI,
    NeI,
    LtI,
    LeI,
    GtI,
    GeI,
    EqB,
    NeB,
    NotB,
    NegI,
    /// Absolute instruction index.
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    Call {
        fn_idx: usize,
        argc: u8,
    },
    /// Pop `argc` values (top = last arg), print space-separated + newline; push `Unit`.
    PrintLn {
        stderr: bool,
        argc: u8,
    },
    /// Pop `right`, `left`; push `Bool` (same rules as tree interpreter for `==` / ordering).
    CompareVal(ValCmpOp),
    /// Pop two strings (top = second arg), push concatenation.
    ConcatStr,
    /// Pop `Int`, push decimal string.
    IntToStr,
    /// Pop `String`, push scalar count as `Int` (same as tree `strlen`).
    StrLen,
    /// Pop `Int` and `std::process::exit` (no return).
    Exit,
    /// Duplicate top stack value.
    Dup,
    /// Push `Value::FnRef` for `prog.fn_names[idx]`.
    PushFn(usize),
    /// Pop `argc` arguments then `FnRef` callee; dispatch like `Call`.
    CallIndirect {
        argc: u8,
    },
    /// Pop `arity` values (top = last field in source order); push `Value::Enum`.
    BuildEnum {
        typ_idx: usize,
        variant_idx: usize,
        field_name_indices: Vec<usize>,
    },
    /// Pop scrutinee: if `Enum` with matching `variant` and field count, push fields (first field on top); else restore value and jump to `fail_pc`.
    MatchEnumUnpack {
        variant_idx: usize,
        arity: u8,
        fail_pc: usize,
    },
    /// `match` fell through (runtime error).
    MatchFail,
    /// Pop `String`, push `Option[Int]` as enum value (like `parse_int`).
    ParseInt,
    /// Pop `String` message then `Bool` condition; if false, VM error.
    Assert,
    /// OS / HTTP / JSON builtin (see [`HostBuiltin::argc`]).
    HostBuiltin(HostBuiltin),
    Ret,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub strings: Vec<String>,
    pub local_count: usize,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub chunks: Vec<Chunk>,
    pub fn_names: Vec<String>,
    pub main_idx: usize,
}

struct CompileCtx<'a> {
    fn_idx: &'a HashMap<String, usize>,
    /// `(type_name, variant_name) -> field names in source order`
    variants: &'a HashMap<(String, String), Vec<String>>,
    scopes: Vec<HashMap<String, u8>>,
    next_slot: u8,
}

impl<'a> CompileCtx<'a> {
    fn new(
        fn_idx: &'a HashMap<String, usize>,
        variants: &'a HashMap<(String, String), Vec<String>>,
        params: &[Param],
    ) -> Self {
        let mut m = HashMap::new();
        for (i, p) in params.iter().enumerate() {
            m.insert(p.name.clone(), i as u8);
        }
        let next_slot = params.len().min(255) as u8;
        Self {
            fn_idx,
            variants,
            scopes: vec![m],
            next_slot,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn lookup(&self, name: &str) -> Option<u8> {
        self.scopes.iter().rev().find_map(|m| m.get(name).copied())
    }

    fn alloc_slot(&mut self, name: String) -> Result<u8, CompileError> {
        let s = self.next_slot;
        self.next_slot = self.next_slot.checked_add(1).ok_or_else(|| CompileError {
            span: Span::new(0, 0),
            message: "VM compile: too many locals (max 255)".into(),
        })?;
        self.scopes.last_mut().expect("scope").insert(name, s);
        Ok(s)
    }
}

fn host_builtin_for_name(name: &str) -> Option<HostBuiltin> {
    Some(match name {
        "read_line" => HostBuiltin::ReadLine,
        "env_get" => HostBuiltin::EnvGet,
        "read_file" => HostBuiltin::ReadFile,
        "write_file" => HostBuiltin::WriteFile,
        "write_file_ok" => HostBuiltin::WriteFileOk,
        "list_dir" => HostBuiltin::ListDir,
        "glob_files" => HostBuiltin::GlobFiles,
        "shell_exec" => HostBuiltin::ShellExec,
        "trim" => HostBuiltin::Trim,
        "starts_with" => HostBuiltin::StartsWith,
        "substring" => HostBuiltin::Substring,
        "index_of" => HostBuiltin::IndexOf,
        "http_post" => HostBuiltin::HttpPost,
        "stdout_print" => HostBuiltin::StdoutPrint,
        "json_string" => HostBuiltin::JsonString,
        "json_extract" => HostBuiltin::JsonExtract,
        "json_value" => HostBuiltin::JsonValue,
        "http_post_sse_fold" => HostBuiltin::HttpPostSseFold,
        _ => return None,
    })
}

fn unsupported(span: Span, msg: &str) -> CompileError {
    CompileError {
        span,
        message: msg.into(),
    }
}

fn patch_jump(code: &mut [Instr], at: usize, target: usize) {
    match &mut code[at] {
        Instr::Jump(ref mut t) | Instr::JumpIfFalse(ref mut t) | Instr::JumpIfTrue(ref mut t) => {
            *t = target
        }
        _ => panic!("patch_jump: wrong instr at {at}"),
    }
}

fn patch_match_unpack(code: &mut [Instr], at: usize, target: usize) {
    if let Instr::MatchEnumUnpack { fail_pc, .. } = &mut code[at] {
        *fail_pc = target;
    } else {
        panic!("patch_match_unpack: wrong instr at {at}");
    }
}

fn collect_variants(module: &Module) -> HashMap<(String, String), Vec<String>> {
    let mut m = HashMap::new();
    for item in &module.items {
        if let Item::Type(td) = item {
            for v in &td.variants {
                let names: Vec<String> = v.fields.iter().map(|f| f.name.clone()).collect();
                m.insert((td.name.clone(), v.name.clone()), names);
            }
        }
    }
    m
}

/// If `variant` is unique across all ADTs, return its `(type_name, field names)`.
fn unique_variant_def<'a>(
    variants: &'a HashMap<(String, String), Vec<String>>,
    variant: &str,
) -> Option<(&'a String, &'a Vec<String>)> {
    let hits: Vec<_> = variants
        .iter()
        .filter(|((_, vn), _)| vn == variant)
        .collect();
    if hits.len() == 1 {
        let ((tn, _), fields) = hits[0];
        Some((tn, fields))
    } else {
        None
    }
}

fn intern_string(pool: &mut Vec<String>, s: &str) -> usize {
    if let Some(i) = pool.iter().position(|x| x == s) {
        return i;
    }
    let i = pool.len();
    pool.push(s.to_string());
    i
}

/// Whether expression is definitely Int-typed (conservative: no `Var` / `Call`).
fn expr_yields_int(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Neg | UnaryOp::Pos,
            expr,
        } => expr_yields_int(expr),
        ExprKind::Unary {
            op: UnaryOp::Not, ..
        } => false,
        ExprKind::Binary {
            op: BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod,
            left,
            right,
        } => expr_yields_int(left) && expr_yields_int(right),
        ExprKind::If {
            then_arm, else_arm, ..
        } => expr_yields_int(then_arm) && expr_yields_int(else_arm),
        ExprKind::Let { body, .. } => expr_yields_int(body),
        ExprKind::Block { tail, .. } => expr_yields_int(tail),
        ExprKind::While { .. } => false,
        _ => false,
    }
}

/// Whether expression is definitely Bool-typed (conservative).
fn expr_yields_bool(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Bool(_) => true,
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => expr_yields_bool(expr),
        ExprKind::Unary {
            op: UnaryOp::Neg | UnaryOp::Pos,
            ..
        } => false,
        ExprKind::Binary {
            op: BinOp::And | BinOp::Or,
            left,
            right,
        } => expr_yields_bool(left) && expr_yields_bool(right),
        ExprKind::Binary {
            op: BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge,
            left,
            right,
        } => {
            (expr_yields_int(left) && expr_yields_int(right))
                || (expr_yields_bool(left) && expr_yields_bool(right))
        }
        ExprKind::Binary {
            op: BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod,
            ..
        } => false,
        ExprKind::If {
            then_arm, else_arm, ..
        } => expr_yields_bool(then_arm) && expr_yields_bool(else_arm),
        ExprKind::Let { body, .. } => expr_yields_bool(body),
        ExprKind::Block { tail, .. } => expr_yields_bool(tail),
        ExprKind::While { .. } => false,
        _ => false,
    }
}

fn compile_expr(
    e: &Expr,
    ctx: &mut CompileCtx<'_>,
    strings: &mut Vec<String>,
    out: &mut Vec<Instr>,
) -> Result<(), CompileError> {
    match &e.kind {
        ExprKind::Int(n) => out.push(Instr::PushInt(*n)),
        ExprKind::Bool(b) => out.push(Instr::PushBool(*b)),
        ExprKind::Unit => out.push(Instr::PushUnit),
        ExprKind::String(s) => {
            let i = intern_string(strings, s);
            out.push(Instr::PushStr(i));
        }
        ExprKind::Var(name) => {
            if let Some(slot) = ctx.lookup(name) {
                out.push(Instr::LoadLocal(slot));
            } else if let Some(&idx) = ctx.fn_idx.get(name) {
                out.push(Instr::PushFn(idx));
            } else {
                return Err(unsupported(
                    e.span,
                    &format!("VM: unknown variable `{name}`"),
                ));
            }
        }
        ExprKind::Unary { op, expr } => {
            compile_expr(expr, ctx, strings, out)?;
            match op {
                UnaryOp::Neg => out.push(Instr::NegI),
                UnaryOp::Not => out.push(Instr::NotB),
                UnaryOp::Pos => {}
            }
        }
        ExprKind::Binary { op, left, right } => match op {
            BinOp::And => {
                // L && R : if L false -> false; else if R false -> false; else true
                compile_expr(left, ctx, strings, out)?;
                let j_l_false = out.len();
                out.push(Instr::JumpIfFalse(0));
                compile_expr(right, ctx, strings, out)?;
                let j_r_false = out.len();
                out.push(Instr::JumpIfFalse(0));
                out.push(Instr::PushBool(true));
                let j_merge = out.len();
                out.push(Instr::Jump(0));
                let fail = out.len();
                patch_jump(out, j_l_false, fail);
                patch_jump(out, j_r_false, fail);
                out.push(Instr::PushBool(false));
                let merge = out.len();
                patch_jump(out, j_merge, merge);
            }
            BinOp::Or => {
                // L || R : if L true -> true; else if R true -> true; else false
                compile_expr(left, ctx, strings, out)?;
                let j_l_true = out.len();
                out.push(Instr::JumpIfTrue(0));
                compile_expr(right, ctx, strings, out)?;
                let j_r_true = out.len();
                out.push(Instr::JumpIfTrue(0));
                out.push(Instr::PushBool(false));
                let j_merge = out.len();
                out.push(Instr::Jump(0));
                let ok = out.len();
                patch_jump(out, j_l_true, ok);
                patch_jump(out, j_r_true, ok);
                out.push(Instr::PushBool(true));
                let merge = out.len();
                patch_jump(out, j_merge, merge);
            }
            BinOp::Eq | BinOp::Ne => {
                if expr_yields_bool(left) && expr_yields_bool(right) {
                    compile_expr(left, ctx, strings, out)?;
                    compile_expr(right, ctx, strings, out)?;
                    out.push(if *op == BinOp::Eq {
                        Instr::EqB
                    } else {
                        Instr::NeB
                    });
                } else if expr_yields_int(left) && expr_yields_int(right) {
                    compile_expr(left, ctx, strings, out)?;
                    compile_expr(right, ctx, strings, out)?;
                    out.push(if *op == BinOp::Eq {
                        Instr::EqI
                    } else {
                        Instr::NeI
                    });
                } else {
                    compile_expr(left, ctx, strings, out)?;
                    compile_expr(right, ctx, strings, out)?;
                    out.push(Instr::CompareVal(if *op == BinOp::Eq {
                        ValCmpOp::Eq
                    } else {
                        ValCmpOp::Ne
                    }));
                }
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let int_fast = expr_yields_int(left) && expr_yields_int(right);
                compile_expr(left, ctx, strings, out)?;
                compile_expr(right, ctx, strings, out)?;
                out.push(if int_fast {
                    match op {
                        BinOp::Lt => Instr::LtI,
                        BinOp::Le => Instr::LeI,
                        BinOp::Gt => Instr::GtI,
                        BinOp::Ge => Instr::GeI,
                        _ => unreachable!(),
                    }
                } else {
                    Instr::CompareVal(match op {
                        BinOp::Lt => ValCmpOp::Lt,
                        BinOp::Le => ValCmpOp::Le,
                        BinOp::Gt => ValCmpOp::Gt,
                        BinOp::Ge => ValCmpOp::Ge,
                        _ => unreachable!(),
                    })
                });
            }
            _ => {
                compile_expr(left, ctx, strings, out)?;
                compile_expr(right, ctx, strings, out)?;
                match op {
                    BinOp::Add => out.push(Instr::AddI),
                    BinOp::Sub => out.push(Instr::SubI),
                    BinOp::Mul => out.push(Instr::MulI),
                    BinOp::Div => out.push(Instr::DivI),
                    BinOp::Mod => out.push(Instr::ModI),
                    BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::And
                    | BinOp::Or => unreachable!(),
                }
            }
        },
        ExprKind::If {
            cond,
            then_arm,
            else_arm,
        } => {
            compile_expr(cond, ctx, strings, out)?;
            let j_false = out.len();
            out.push(Instr::JumpIfFalse(0));
            compile_expr(then_arm, ctx, strings, out)?;
            let j_end = out.len();
            out.push(Instr::Jump(0));
            let else_start = out.len();
            patch_jump(out, j_false, else_start);
            compile_expr(else_arm, ctx, strings, out)?;
            let end = out.len();
            patch_jump(out, j_end, end);
        }
        ExprKind::Let { name, value, body } => {
            ctx.push_scope();
            let slot = ctx.alloc_slot(name.clone())?;
            compile_expr(value, ctx, strings, out)?;
            out.push(Instr::StoreLocal(slot));
            compile_expr(body, ctx, strings, out)?;
            ctx.pop_scope();
        }
        ExprKind::Block { stmts, tail } => {
            ctx.push_scope();
            for s in stmts {
                match s {
                    Stmt::Let { name, value } => {
                        let slot = ctx.alloc_slot(name.clone())?;
                        compile_expr(value, ctx, strings, out)?;
                        out.push(Instr::StoreLocal(slot));
                    }
                    Stmt::Expr(ex) => {
                        compile_expr(ex, ctx, strings, out)?;
                        out.push(Instr::Pop);
                    }
                }
            }
            compile_expr(tail, ctx, strings, out)?;
            ctx.pop_scope();
        }
        ExprKind::Call { callee, args } => {
            if let ExprKind::Var(fname) = &callee.kind {
                if fname == "println" || fname == "eprintln" {
                    if args.len() > 255 {
                        return Err(unsupported(e.span, "VM: too many arguments"));
                    }
                    for a in args {
                        compile_expr(a, ctx, strings, out)?;
                    }
                    out.push(Instr::PrintLn {
                        stderr: fname == "eprintln",
                        argc: args.len() as u8,
                    });
                    out.push(Instr::PushUnit);
                    return Ok(());
                }
                if fname == "concat" {
                    if args.len() != 2 {
                        return Err(unsupported(
                            e.span,
                            "VM: `concat` expects exactly two arguments",
                        ));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    compile_expr(&args[1], ctx, strings, out)?;
                    out.push(Instr::ConcatStr);
                    return Ok(());
                }
                if fname == "string_from_int" {
                    if args.len() != 1 {
                        return Err(unsupported(
                            e.span,
                            "VM: `string_from_int` expects one argument",
                        ));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    out.push(Instr::IntToStr);
                    return Ok(());
                }
                if fname == "strlen" {
                    if args.len() != 1 {
                        return Err(unsupported(e.span, "VM: `strlen` expects one argument"));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    out.push(Instr::StrLen);
                    return Ok(());
                }
                if fname == "exit" {
                    if args.len() != 1 {
                        return Err(unsupported(e.span, "VM: `exit` expects one argument"));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    out.push(Instr::Exit);
                    return Ok(());
                }
                if fname == "parse_int" {
                    if args.len() != 1 {
                        return Err(unsupported(
                            e.span,
                            "VM: `parse_int` expects one String argument",
                        ));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    out.push(Instr::ParseInt);
                    return Ok(());
                }
                if fname == "assert" {
                    if args.len() != 2 {
                        return Err(unsupported(e.span, "VM: `assert` expects (Bool, String)"));
                    }
                    compile_expr(&args[0], ctx, strings, out)?;
                    compile_expr(&args[1], ctx, strings, out)?;
                    out.push(Instr::Assert);
                    out.push(Instr::PushUnit);
                    return Ok(());
                }
                if ctx.lookup(fname).is_none() {
                    if let Some(b) = host_builtin_for_name(fname) {
                        let argc = b.argc() as usize;
                        if args.len() != argc {
                            return Err(unsupported(
                                e.span,
                                &format!(
                                    "VM: `{fname}` expects {argc} argument(s), got {}",
                                    args.len()
                                ),
                            ));
                        }
                        if args.len() > 255 {
                            return Err(unsupported(e.span, "VM: too many arguments"));
                        }
                        for a in args {
                            compile_expr(a, ctx, strings, out)?;
                        }
                        out.push(Instr::HostBuiltin(b));
                        return Ok(());
                    }
                    if let Some(&idx) = ctx.fn_idx.get(fname) {
                        for a in args {
                            compile_expr(a, ctx, strings, out)?;
                        }
                        out.push(Instr::Call {
                            fn_idx: idx,
                            argc: args.len() as u8,
                        });
                        return Ok(());
                    }
                }
            }
            compile_expr(callee, ctx, strings, out)?;
            for a in args {
                compile_expr(a, ctx, strings, out)?;
            }
            if args.len() > 255 {
                return Err(unsupported(e.span, "VM: too many arguments"));
            }
            out.push(Instr::CallIndirect {
                argc: args.len() as u8,
            });
        }
        ExprKind::While { cond, body } => {
            // Condition type is already `Bool` after typecheck; allow `Var`, `Call`, comparisons, etc.
            let loop_head = out.len();
            compile_expr(cond, ctx, strings, out)?;
            let j_exit = out.len();
            out.push(Instr::JumpIfFalse(0));
            compile_expr(body, ctx, strings, out)?;
            out.push(Instr::Pop);
            out.push(Instr::Jump(loop_head));
            let loop_exit = out.len();
            patch_jump(out, j_exit, loop_exit);
            out.push(Instr::PushUnit);
        }
        ExprKind::Match { scrutinee, arms } => {
            compile_match(scrutinee, arms, e.span, ctx, strings, out)?;
        }
        ExprKind::Construct { typ, variant, args } => {
            let TypeExpr::Named { name: tname, .. } = typ else {
                return Err(unsupported(e.span, "VM: enum constructor needs named type"));
            };
            let field_names = ctx
                .variants
                .get(&(tname.clone(), variant.clone()))
                .ok_or_else(|| {
                    unsupported(e.span, &format!("VM: unknown enum `{tname}::{variant}`"))
                })?;
            if field_names.len() != args.len() {
                return Err(unsupported(
                    e.span,
                    "VM: constructor argument count mismatch",
                ));
            }
            for a in args {
                compile_expr(a, ctx, strings, out)?;
            }
            let typ_idx = intern_string(strings, tname);
            let variant_idx = intern_string(strings, variant);
            let field_name_indices: Vec<usize> = field_names
                .iter()
                .map(|n| intern_string(strings, n))
                .collect();
            out.push(Instr::BuildEnum {
                typ_idx,
                variant_idx,
                field_name_indices,
            });
        }
    }
    Ok(())
}

fn compile_pattern_from_stack(
    pat: &crate::ast::Pattern,
    fail_arm: Option<usize>,
    unpack_fixups: &mut Vec<(usize, Option<usize>)>,
    ctx: &mut CompileCtx<'_>,
    strings: &mut Vec<String>,
    out: &mut Vec<Instr>,
) -> Result<(), CompileError> {
    use crate::ast::Pattern;
    use crate::ast::PatternField;
    match pat {
        Pattern::Wildcard => {
            out.push(Instr::Pop);
        }
        Pattern::Bind(n) => {
            let slot = ctx.alloc_slot(n.clone())?;
            out.push(Instr::StoreLocal(slot));
        }
        Pattern::Ctor { name, fields } => {
            let has_named = fields
                .iter()
                .any(|f| matches!(f, PatternField::Named(_, _)));
            let has_pos = fields.iter().any(|f| matches!(f, PatternField::Pos(_)));
            if has_named && has_pos {
                return Err(CompileError {
                    span: Span::new(0, 0),
                    message: "VM: mixed named/positional match pattern".into(),
                });
            }
            let vidx = intern_string(strings, name);
            let arity = fields.len() as u8;
            let unpack_at = out.len();
            out.push(Instr::MatchEnumUnpack {
                variant_idx: vidx,
                arity,
                fail_pc: 0,
            });
            unpack_fixups.push((unpack_at, fail_arm));
            if has_named {
                let Some((_, def_fields)) = unique_variant_def(ctx.variants, name) else {
                    return Err(CompileError {
                        span: Span::new(0, 0),
                        message: format!("VM: ambiguous or unknown variant `{name}` in match"),
                    });
                };
                if def_fields.len() != fields.len() {
                    return Err(CompileError {
                        span: Span::new(0, 0),
                        message: "VM: pattern field count mismatch".into(),
                    });
                }
                let mut temps: Vec<u8> = Vec::new();
                for _ in 0..def_fields.len() {
                    let t = ctx.alloc_slot(format!("__m{}", temps.len()))?;
                    temps.push(t);
                    out.push(Instr::StoreLocal(t));
                }
                for f in fields {
                    if let PatternField::Named(n, p) = f {
                        let idx = def_fields.iter().position(|dn| dn == n).ok_or_else(|| {
                            CompileError {
                                span: Span::new(0, 0),
                                message: format!("VM: unknown field `{n}` in pattern"),
                            }
                        })?;
                        out.push(Instr::LoadLocal(temps[idx]));
                        compile_pattern_from_stack(
                            p.as_ref(),
                            fail_arm,
                            unpack_fixups,
                            ctx,
                            strings,
                            out,
                        )?;
                    }
                }
            } else {
                for f in fields {
                    if let PatternField::Pos(p) = f {
                        compile_pattern_from_stack(
                            p.as_ref(),
                            fail_arm,
                            unpack_fixups,
                            ctx,
                            strings,
                            out,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn compile_match(
    scrutinee: &Expr,
    arms: &[crate::ast::MatchArm],
    span: Span,
    ctx: &mut CompileCtx<'_>,
    strings: &mut Vec<String>,
    out: &mut Vec<Instr>,
) -> Result<(), CompileError> {
    if arms.is_empty() {
        return Err(unsupported(span, "VM: empty match"));
    }
    compile_expr(scrutinee, ctx, strings, out)?;
    let mut start_of_arm: Vec<usize> = vec![0; arms.len()];
    let mut end_jumps: Vec<usize> = Vec::new();
    let mut unpack_fixups: Vec<(usize, Option<usize>)> = Vec::new();

    for (i, arm) in arms.iter().enumerate() {
        start_of_arm[i] = out.len();
        ctx.push_scope();
        let fail_arm = if i + 1 < arms.len() {
            Some(i + 1)
        } else {
            None
        };
        compile_pattern_from_stack(
            &arm.pattern,
            fail_arm,
            &mut unpack_fixups,
            ctx,
            strings,
            out,
        )?;
        compile_expr(&arm.body, ctx, strings, out)?;
        ctx.pop_scope();
        // Every arm must jump past `MatchFail`; the last arm is not special.
        end_jumps.push(out.len());
        out.push(Instr::Jump(0));
    }

    let match_fail = out.len();
    out.push(Instr::MatchFail);
    let after_match = out.len();

    for j in end_jumps {
        patch_jump(out, j, after_match);
    }
    for (unpack_at, dest_arm) in unpack_fixups {
        let target = match dest_arm {
            Some(ai) => start_of_arm[ai],
            None => match_fail,
        };
        patch_match_unpack(out, unpack_at, target);
    }

    Ok(())
}

/// Build bytecode for every `fn` in module order (types skipped). `main` must exist.
pub fn compile_module(module: &Module) -> Result<Program, CompileError> {
    let mut fns: Vec<&FnDef> = Vec::new();
    for item in &module.items {
        if let Item::Fn(f) = item {
            fns.push(f);
        }
    }
    let main_pos = fns
        .iter()
        .position(|f| f.name == "main")
        .ok_or_else(|| CompileError {
            span: Span::new(0, 0),
            message: "VM compile: no `main`".into(),
        })?;
    if !fns[main_pos].params.is_empty() {
        return Err(CompileError {
            span: fns[main_pos].span,
            message: "VM compile: `main` must take no parameters".into(),
        });
    }

    let fn_idx: HashMap<String, usize> = fns
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.clone(), i))
        .collect();

    let variants = collect_variants(module);

    let mut chunks = Vec::with_capacity(fns.len());
    let fn_names: Vec<String> = fns.iter().map(|f| f.name.clone()).collect();

    for f in &fns {
        let mut ctx = CompileCtx::new(&fn_idx, &variants, &f.params);
        let mut strings = Vec::new();
        let mut code = Vec::new();
        compile_expr(&f.body, &mut ctx, &mut strings, &mut code)?;
        code.push(Instr::Ret);
        let local_count = ctx.next_slot as usize;
        chunks.push(Chunk {
            code,
            strings,
            local_count,
        });
    }

    Ok(Program {
        chunks,
        fn_names,
        main_idx: main_pos,
    })
}

/// Format a value like the tree interpreter's `println`.
pub fn format_value_line(v: &crate::interp::Value) -> String {
    let mut line = String::new();
    write_value(&mut line, v);
    line
}

fn write_value(out: &mut String, v: &crate::interp::Value) {
    use crate::interp::Value;
    match v {
        Value::Int(n) => {
            use std::fmt::Write;
            let _ = write!(out, "{n}");
        }
        Value::Bool(b) => {
            use std::fmt::Write;
            let _ = write!(out, "{b}");
        }
        Value::String(s) => out.push_str(s),
        Value::Unit => out.push_str("()"),
        Value::Enum {
            typ,
            variant,
            fields,
        } => {
            use std::fmt::Write;
            if fields.is_empty() {
                let _ = write!(out, "{typ}::{variant}");
            } else {
                let _ = write!(out, "{typ}::{variant}(");
                for (i, (_, fv)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_value(out, fv);
                }
                out.push(')');
            }
        }
        Value::FnRef(n) => {
            use std::fmt::Write;
            let _ = write!(out, "<fn {n}>");
        }
    }
}

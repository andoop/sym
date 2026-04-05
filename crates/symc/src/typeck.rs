use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinOp, Expr, ExprKind, FnDef, Item, MatchArm, Module, Pattern, PatternField, Stmt, TypeDef,
    TypeExpr, UnaryOp,
};
use crate::builtins;
use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Int,
    Bool,
    String,
    Unit,
    Never,
    Enum { name: String, args: Vec<Ty> },
    Fun(Vec<Ty>, Box<Ty>),
}

#[derive(Debug)]
pub struct TypeError {
    pub span: Span,
    pub message: String,
}

struct AdtInfo {
    params: Vec<String>,
    variants: HashMap<String, Vec<(String, TypeExpr)>>,
}

pub struct TypeChecker {
    adts: HashMap<String, AdtInfo>,
    fns: HashMap<String, (Vec<Ty>, Ty)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            adts: HashMap::new(),
            fns: HashMap::new(),
        }
    }

    pub fn check_module(&mut self, m: &Module) -> Result<(), TypeError> {
        for item in &m.items {
            if let Item::Type(td) = item {
                self.register_adt(td)?;
            }
        }
        for item in &m.items {
            if let Item::Fn(f) = item {
                if self.fns.contains_key(&f.name) {
                    return Err(TypeError {
                        span: f.span,
                        message: format!("duplicate function `{}`", f.name),
                    });
                }
                if builtins::is_reserved(&f.name) {
                    return Err(TypeError {
                        span: f.span,
                        message: format!("name `{}` is reserved for a compiler builtin", f.name),
                    });
                }
                let param_tys: Vec<Ty> = f
                    .params
                    .iter()
                    .map(|p| self.resolve_type(&p.ty, &HashMap::new()))
                    .collect::<Result<_, _>>()?;
                let ret = match &f.ret {
                    Some(t) => self.resolve_type(t, &HashMap::new())?,
                    None => Ty::Unit,
                };
                self.fns.insert(f.name.clone(), (param_tys, ret));
            }
        }
        for item in &m.items {
            if let Item::Fn(f) = item {
                self.check_fn(f)?;
            }
        }
        Ok(())
    }

    fn register_adt(&mut self, td: &TypeDef) -> Result<(), TypeError> {
        if self.adts.contains_key(&td.name) {
            return Err(TypeError {
                span: td.span,
                message: format!("duplicate type `{}`", td.name),
            });
        }
        let mut variants = HashMap::new();
        for v in &td.variants {
            if variants.contains_key(&v.name) {
                return Err(TypeError {
                    span: td.span,
                    message: format!("duplicate variant `{}` in `{}`", v.name, td.name),
                });
            }
            let fields: Vec<(String, TypeExpr)> = v
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.ty.clone()))
                .collect();
            variants.insert(v.name.clone(), fields);
        }
        self.adts.insert(
            td.name.clone(),
            AdtInfo {
                params: td.params.clone(),
                variants,
            },
        );
        Ok(())
    }

    fn resolve_type(&self, t: &TypeExpr, subst: &HashMap<String, Ty>) -> Result<Ty, TypeError> {
        match t {
            TypeExpr::Named { name, args } => {
                if let Some(ty) = subst.get(name) {
                    if !args.is_empty() {
                        return Err(TypeError {
                            span: Span::new(0, 0),
                            message: format!("type parameter `{name}` cannot take arguments"),
                        });
                    }
                    return Ok(ty.clone());
                }
                match name.as_str() {
                    "Int" => {
                        if !args.is_empty() {
                            return Err(TypeError {
                                span: Span::new(0, 0),
                                message: "`Int` is not generic".into(),
                            });
                        }
                        Ok(Ty::Int)
                    }
                    "Bool" => {
                        if !args.is_empty() {
                            return Err(TypeError {
                                span: Span::new(0, 0),
                                message: "`Bool` is not generic".into(),
                            });
                        }
                        Ok(Ty::Bool)
                    }
                    "String" => {
                        if !args.is_empty() {
                            return Err(TypeError {
                                span: Span::new(0, 0),
                                message: "`String` is not generic".into(),
                            });
                        }
                        Ok(Ty::String)
                    }
                    "Unit" => {
                        if !args.is_empty() {
                            return Err(TypeError {
                                span: Span::new(0, 0),
                                message: "`Unit` is not generic".into(),
                            });
                        }
                        Ok(Ty::Unit)
                    }
                    _ => {
                        let info = self.adts.get(name).ok_or_else(|| TypeError {
                            span: Span::new(0, 0),
                            message: format!("unknown type `{name}`"),
                        })?;
                        if info.params.len() != args.len() {
                            return Err(TypeError {
                                span: Span::new(0, 0),
                                message: format!(
                                    "type `{}` expects {} parameter(s), got {}",
                                    name,
                                    info.params.len(),
                                    args.len()
                                ),
                            });
                        }
                        let mut arg_tys = Vec::new();
                        for a in args {
                            arg_tys.push(self.resolve_type(a, subst)?);
                        }
                        Ok(Ty::Enum {
                            name: name.clone(),
                            args: arg_tys,
                        })
                    }
                }
            }
            TypeExpr::Fun { params, ret } => {
                let mut ps = Vec::new();
                for p in params {
                    ps.push(self.resolve_type(p, subst)?);
                }
                let r = self.resolve_type(ret, subst)?;
                Ok(Ty::Fun(ps, Box::new(r)))
            }
        }
    }

    fn subst_field_type(
        &self,
        field_ty: &TypeExpr,
        adt_params: &[String],
        adt_args: &[Ty],
    ) -> Result<Ty, TypeError> {
        let mut map = HashMap::new();
        for (p, a) in adt_params.iter().zip(adt_args.iter()) {
            map.insert(p.clone(), a.clone());
        }
        self.resolve_type(field_ty, &map)
    }

    fn variant_field_tys(
        &self,
        enum_name: &str,
        variant: &str,
        enum_args: &[Ty],
    ) -> Result<Vec<(String, Ty)>, TypeError> {
        let info = self.adts.get(enum_name).ok_or_else(|| TypeError {
            span: Span::new(0, 0),
            message: format!("unknown enum `{enum_name}`"),
        })?;
        let fields = info.variants.get(variant).ok_or_else(|| TypeError {
            span: Span::new(0, 0),
            message: format!("unknown variant `{variant}` on `{enum_name}`"),
        })?;
        let mut out = Vec::new();
        for (fname, fty) in fields {
            out.push((
                fname.clone(),
                self.subst_field_type(fty, &info.params, enum_args)?,
            ));
        }
        Ok(out)
    }

    fn check_fn(&self, f: &FnDef) -> Result<(), TypeError> {
        let (param_tys, ret_ty) = self.fns.get(&f.name).unwrap().clone();
        let mut env: HashMap<String, Ty> = HashMap::new();
        for (name, (pts, rt)) in &self.fns {
            env.insert(name.clone(), Ty::Fun(pts.clone(), Box::new(rt.clone())));
        }
        for (p, t) in f.params.iter().zip(param_tys.iter()) {
            env.insert(p.name.clone(), t.clone());
        }
        let body_ty = self.check_expr(&f.body, &env)?;
        self.unify_span(&body_ty, &ret_ty, f.body.span, "function body")?;
        Ok(())
    }

    fn unify_span(&self, got: &Ty, expect: &Ty, span: Span, ctx: &str) -> Result<(), TypeError> {
        if self.compatible(got, expect) {
            Ok(())
        } else {
            Err(TypeError {
                span,
                message: format!(
                    "{ctx}: expected `{}`, found `{}`",
                    self.ty_display(expect),
                    self.ty_display(got)
                ),
            })
        }
    }

    fn compatible(&self, a: &Ty, b: &Ty) -> bool {
        match (a, b) {
            (Ty::Never, _) => true,
            (_, Ty::Never) => true,
            _ => a == b,
        }
    }

    fn ty_display(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "Int".into(),
            Ty::Bool => "Bool".into(),
            Ty::String => "String".into(),
            Ty::Unit => "Unit".into(),
            Ty::Never => "Never".into(),
            Ty::Enum { name, args } if args.is_empty() => name.clone(),
            Ty::Enum { name, args } => {
                let inner: Vec<_> = args.iter().map(|x| self.ty_display(x)).collect();
                format!("{}[{}]", name, inner.join(", "))
            }
            Ty::Fun(ps, r) => {
                let ps: Vec<_> = ps.iter().map(|x| self.ty_display(x)).collect();
                format!("fn({}) -> {}", ps.join(", "), self.ty_display(r))
            }
        }
    }

    fn check_expr(&self, e: &Expr, env: &HashMap<String, Ty>) -> Result<Ty, TypeError> {
        match &e.kind {
            ExprKind::Int(_) => Ok(Ty::Int),
            ExprKind::String(_) => Ok(Ty::String),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Unit => Ok(Ty::Unit),
            ExprKind::Var(name) => env.get(name).cloned().ok_or_else(|| TypeError {
                span: e.span,
                message: format!("unknown variable `{name}`"),
            }),
            ExprKind::Binary { op, left, right } => {
                let lt = self.check_expr(left, env)?;
                let rt = self.check_expr(right, env)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        self.unify_span(&lt, &Ty::Int, left.span, "binary left")?;
                        self.unify_span(&rt, &Ty::Int, right.span, "binary right")?;
                        Ok(Ty::Int)
                    }
                    BinOp::Eq | BinOp::Ne => {
                        if lt != rt && !(matches!((&lt, &rt), (Ty::Never, _) | (_, Ty::Never))) {
                            return Err(TypeError {
                                span: e.span,
                                message: format!(
                                    "`{}` needs equal operand types, got `{}` and `{}`",
                                    match op {
                                        BinOp::Eq => "==",
                                        BinOp::Ne => "!=",
                                        _ => "?",
                                    },
                                    self.ty_display(&lt),
                                    self.ty_display(&rt)
                                ),
                            });
                        }
                        Ok(Ty::Bool)
                    }
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match (&lt, &rt) {
                        (Ty::Int, Ty::Int)
                        | (Ty::Never, Ty::Int)
                        | (Ty::Int, Ty::Never)
                        | (Ty::Never, Ty::Never) => {
                            self.unify_span(&lt, &Ty::Int, left.span, "compare left")?;
                            self.unify_span(&rt, &Ty::Int, right.span, "compare right")?;
                            Ok(Ty::Bool)
                        }
                        (Ty::String, Ty::String)
                        | (Ty::Never, Ty::String)
                        | (Ty::String, Ty::Never) => {
                            self.unify_span(&lt, &Ty::String, left.span, "compare left")?;
                            self.unify_span(&rt, &Ty::String, right.span, "compare right")?;
                            Ok(Ty::Bool)
                        }
                        _ => Err(TypeError {
                            span: e.span,
                            message: format!(
                                "`{}` expects two Int or two String, got `{}` and `{}`",
                                match op {
                                    BinOp::Lt => "<",
                                    BinOp::Le => "<=",
                                    BinOp::Gt => ">",
                                    BinOp::Ge => ">=",
                                    _ => "?",
                                },
                                self.ty_display(&lt),
                                self.ty_display(&rt)
                            ),
                        }),
                    },
                    BinOp::And | BinOp::Or => {
                        self.unify_span(&lt, &Ty::Bool, left.span, "logic left")?;
                        self.unify_span(&rt, &Ty::Bool, right.span, "logic right")?;
                        Ok(Ty::Bool)
                    }
                }
            }
            ExprKind::Unary { op, expr } => {
                let t = self.check_expr(expr, env)?;
                match op {
                    UnaryOp::Not => {
                        self.unify_span(&t, &Ty::Bool, expr.span, "`!`")?;
                        Ok(Ty::Bool)
                    }
                    UnaryOp::Neg => {
                        self.unify_span(&t, &Ty::Int, expr.span, "unary `-`")?;
                        Ok(Ty::Int)
                    }
                    UnaryOp::Pos => {
                        self.unify_span(&t, &Ty::Int, expr.span, "unary `+`")?;
                        Ok(Ty::Int)
                    }
                }
            }
            ExprKind::If {
                cond,
                then_arm,
                else_arm,
            } => {
                self.unify_span(
                    &self.check_expr(cond, env)?,
                    &Ty::Bool,
                    cond.span,
                    "if condition",
                )?;
                let tt = self.check_expr(then_arm, env)?;
                let et = self.check_expr(else_arm, env)?;
                if tt == et || matches!((&tt, &et), (Ty::Never, _) | (_, Ty::Never)) {
                    Ok(if tt == Ty::Never { et } else { tt })
                } else {
                    Err(TypeError {
                        span: e.span,
                        message: format!(
                            "`if` branches disagree: `{}` vs `{}`",
                            self.ty_display(&tt),
                            self.ty_display(&et)
                        ),
                    })
                }
            }
            ExprKind::While { cond, body } => {
                self.unify_span(
                    &self.check_expr(cond, env)?,
                    &Ty::Bool,
                    cond.span,
                    "while condition",
                )?;
                self.unify_span(
                    &self.check_expr(body, env)?,
                    &Ty::Unit,
                    body.span,
                    "while body",
                )?;
                Ok(Ty::Unit)
            }
            ExprKind::Let { name, value, body } => {
                let vt = self.check_expr(value, env)?;
                let mut env2 = env.clone();
                env2.insert(name.clone(), vt);
                self.check_expr(body, &env2)
            }
            ExprKind::Block { stmts, tail } => {
                let mut env2 = env.clone();
                for s in stmts {
                    match s {
                        Stmt::Let { name, value } => {
                            let t = self.check_expr(value, &env2)?;
                            env2.insert(name.clone(), t);
                        }
                        Stmt::Expr(ex) => {
                            self.check_expr(ex, &env2)?;
                        }
                    }
                }
                self.check_expr(tail, &env2)
            }
            ExprKind::Match { scrutinee, arms } => {
                let st = self.check_expr(scrutinee, env)?;
                let enum_name = match &st {
                    Ty::Enum { name, .. } => name.clone(),
                    _ => {
                        return Err(TypeError {
                            span: scrutinee.span,
                            message: "`match` scrutinee must be an enum".into(),
                        });
                    }
                };
                let variant_names: HashSet<_> = self
                    .adts
                    .get(&enum_name)
                    .unwrap()
                    .variants
                    .keys()
                    .cloned()
                    .collect();
                if !self.match_exhaustive(&st, arms, &variant_names) {
                    return Err(TypeError {
                        span: e.span,
                        message: "non-exhaustive `match`".into(),
                    });
                }
                let mut result_ty: Option<Ty> = None;
                for arm in arms {
                    let mut arm_env = env.clone();
                    self.bind_pattern(&arm.pattern, &st, &mut arm_env, arm.span)?;
                    let bt = self.check_expr(&arm.body, &arm_env)?;
                    result_ty = Some(match &result_ty {
                        None => bt,
                        Some(prev) => {
                            if prev == &bt || *prev == Ty::Never || bt == Ty::Never {
                                if *prev == Ty::Never {
                                    bt
                                } else if bt == Ty::Never {
                                    prev.clone()
                                } else {
                                    bt
                                }
                            } else {
                                return Err(TypeError {
                                    span: arm.span,
                                    message: format!(
                                        "`match` arm types disagree: `{}` vs `{}`",
                                        self.ty_display(prev),
                                        self.ty_display(&bt)
                                    ),
                                });
                            }
                        }
                    });
                }
                Ok(result_ty.unwrap_or(Ty::Never))
            }
            ExprKind::Construct { typ, variant, args } => {
                let sty = self.resolve_type(typ, &HashMap::new())?;
                let Ty::Enum {
                    name: en,
                    args: eargs,
                } = sty.clone()
                else {
                    return Err(TypeError {
                        span: e.span,
                        message: "constructor base must be an enum type".into(),
                    });
                };
                let field_tys = self.variant_field_tys(&en, variant, &eargs)?;
                if field_tys.len() != args.len() {
                    return Err(TypeError {
                        span: e.span,
                        message: format!(
                            "variant `{variant}` expects {} argument(s), got {}",
                            field_tys.len(),
                            args.len()
                        ),
                    });
                }
                for ((_, ft), ae) in field_tys.iter().zip(args.iter()) {
                    self.unify_span(
                        &self.check_expr(ae, env)?,
                        ft,
                        ae.span,
                        "constructor argument",
                    )?;
                }
                Ok(sty)
            }
            ExprKind::Call { callee, args } => {
                if let ExprKind::Var(name) = &callee.kind {
                    match name.as_str() {
                        "println" | "eprintln" => {
                            for a in args {
                                let t = self.check_expr(a, env)?;
                                self.ensure_printable(&t, a.span)?;
                            }
                            return Ok(Ty::Unit);
                        }
                        "exit" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`exit` expects one Int (exit code)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::Int,
                                args[0].span,
                                "`exit`",
                            )?;
                            return Ok(Ty::Never);
                        }
                        "concat" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`concat` expects two arguments".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`concat` first arg",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`concat` second arg",
                            )?;
                            return Ok(Ty::String);
                        }
                        "string_from_int" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`string_from_int` expects one Int argument".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::Int,
                                args[0].span,
                                "`string_from_int`",
                            )?;
                            return Ok(Ty::String);
                        }
                        "strlen" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`strlen` expects one String argument".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`strlen`",
                            )?;
                            return Ok(Ty::Int);
                        }
                        "read_line" => {
                            if !args.is_empty() {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`read_line` expects no arguments".into(),
                                });
                            }
                            return Ok(Ty::String);
                        }
                        "assert" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`assert` expects (Bool, String)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::Bool,
                                args[0].span,
                                "`assert` condition",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`assert` message",
                            )?;
                            return Ok(Ty::Unit);
                        }
                        "parse_int" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`parse_int` expects one String".into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`parse_int` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`parse_int`",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::Int],
                            });
                        }
                        "env_get" | "read_file" | "list_dir" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: format!(
                                        "`{}` expects one String argument",
                                        name.as_str()
                                    ),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: format!(
                                        "`{}` needs type `Option` in scope (load stdlib/prelude.sym)",
                                        name.as_str()
                                    ),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                name.as_str(),
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "write_file" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`write_file` expects (String, String)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`write_file` path",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`write_file` content",
                            )?;
                            return Ok(Ty::Unit);
                        }
                        "write_file_ok" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`write_file_ok` expects (String, String)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`write_file_ok` path",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`write_file_ok` content",
                            )?;
                            return Ok(Ty::Bool);
                        }
                        "glob_files" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`glob_files` expects (String, String)".into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`glob_files` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`glob_files` base directory",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`glob_files` glob pattern",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "shell_exec" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`shell_exec` expects (cwd: String, command: String)"
                                        .into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`shell_exec` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`shell_exec` cwd",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`shell_exec` command",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "trim" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`trim` expects one String".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`trim`",
                            )?;
                            return Ok(Ty::String);
                        }
                        "starts_with" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`starts_with` expects (String, String)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`starts_with` haystack",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`starts_with` prefix",
                            )?;
                            return Ok(Ty::Bool);
                        }
                        "substring" => {
                            if args.len() != 3 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`substring` expects (String, Int, Int)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`substring` string",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::Int,
                                args[1].span,
                                "`substring` start",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[2], env)?,
                                &Ty::Int,
                                args[2].span,
                                "`substring` length",
                            )?;
                            return Ok(Ty::String);
                        }
                        "index_of" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`index_of` expects (String, String)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`index_of` haystack",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`index_of` needle",
                            )?;
                            return Ok(Ty::Int);
                        }
                        "http_post" => {
                            if args.len() != 3 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`http_post` expects (url: String, headers: String, body: String)".into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`http_post` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            for (i, label) in ["url", "headers", "body"].iter().enumerate() {
                                self.unify_span(
                                    &self.check_expr(&args[i], env)?,
                                    &Ty::String,
                                    args[i].span,
                                    label,
                                )?;
                            }
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "stdout_print" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message:
                                        "`stdout_print` expects one String (no trailing newline)"
                                            .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`stdout_print`",
                            )?;
                            return Ok(Ty::Unit);
                        }
                        "http_post_sse_fold" => {
                            if args.len() != 5 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`http_post_sse_fold` expects (url: String, headers: String, body: String, state0: String, reducer: fn(String, String) -> String)".into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`http_post_sse_fold` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            for (i, label) in
                                ["url", "headers", "body", "state0"].iter().enumerate()
                            {
                                self.unify_span(
                                    &self.check_expr(&args[i], env)?,
                                    &Ty::String,
                                    args[i].span,
                                    label,
                                )?;
                            }
                            let red_ty =
                                Ty::Fun(vec![Ty::String, Ty::String], Box::new(Ty::String));
                            self.unify_span(
                                &self.check_expr(&args[4], env)?,
                                &red_ty,
                                args[4].span,
                                "`http_post_sse_fold` reducer",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "json_string" => {
                            if args.len() != 1 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`json_string` expects one String (returns a JSON string literal)".into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`json_string`",
                            )?;
                            return Ok(Ty::String);
                        }
                        "json_extract" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`json_extract` expects (json: String, path: String)"
                                        .into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`json_extract` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`json_extract` json",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`json_extract` path",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        "json_value" => {
                            if args.len() != 2 {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`json_value` expects (json: String, path: String)"
                                        .into(),
                                });
                            }
                            if !self.adts.contains_key("Option") {
                                return Err(TypeError {
                                    span: e.span,
                                    message: "`json_value` needs type `Option` in scope (load stdlib/prelude.sym)"
                                        .into(),
                                });
                            }
                            self.unify_span(
                                &self.check_expr(&args[0], env)?,
                                &Ty::String,
                                args[0].span,
                                "`json_value` json",
                            )?;
                            self.unify_span(
                                &self.check_expr(&args[1], env)?,
                                &Ty::String,
                                args[1].span,
                                "`json_value` path",
                            )?;
                            return Ok(Ty::Enum {
                                name: "Option".into(),
                                args: vec![Ty::String],
                            });
                        }
                        _ => {}
                    }
                }
                let mut ct = self.check_expr(callee, env)?;
                let mut remaining = args.as_slice();
                while !remaining.is_empty() {
                    let Ty::Fun(ps, ret) = ct else {
                        return Err(TypeError {
                            span: callee.span,
                            message: format!(
                                "expected function type, got `{}`",
                                self.ty_display(&ct)
                            ),
                        });
                    };
                    if ps.is_empty() {
                        return Err(TypeError {
                            span: callee.span,
                            message: "mis-applied function type".into(),
                        });
                    }
                    let (first, rest) = ps.split_first().unwrap();
                    self.unify_span(
                        &self.check_expr(&remaining[0], env)?,
                        first,
                        remaining[0].span,
                        "call argument",
                    )?;
                    remaining = &remaining[1..];
                    if rest.is_empty() {
                        ct = (*ret).clone();
                    } else {
                        ct = Ty::Fun(rest.to_vec(), ret.clone());
                    }
                }
                while let Ty::Fun(ps, ret) = &ct {
                    if !ps.is_empty() {
                        break;
                    }
                    ct = (**ret).clone();
                }
                Ok(ct)
            }
        }
    }

    fn ensure_printable(&self, t: &Ty, span: Span) -> Result<(), TypeError> {
        match t {
            Ty::Int | Ty::Bool | Ty::String | Ty::Unit => Ok(()),
            Ty::Enum { .. } => Ok(()),
            _ => Err(TypeError {
                span,
                message: format!("`println` cannot print `{}`", self.ty_display(t)),
            }),
        }
    }

    fn match_exhaustive(
        &self,
        scrutinee: &Ty,
        arms: &[MatchArm],
        all_variants: &HashSet<String>,
    ) -> bool {
        for arm in arms {
            match &arm.pattern {
                Pattern::Wildcard | Pattern::Bind(_) => return true,
                Pattern::Ctor { name, .. } => {
                    let _ = name;
                }
            }
        }
        let Ty::Enum { name, args: _ } = scrutinee else {
            return true;
        };
        let _ = name;
        let mut covered = HashSet::new();
        for arm in arms {
            if let Pattern::Ctor { name, .. } = &arm.pattern {
                covered.insert(name.clone());
            }
        }
        all_variants.iter().all(|v| covered.contains(v))
    }

    fn bind_pattern(
        &self,
        pat: &Pattern,
        scrutinee: &Ty,
        env: &mut HashMap<String, Ty>,
        span: Span,
    ) -> Result<(), TypeError> {
        match pat {
            Pattern::Wildcard => Ok(()),
            Pattern::Bind(n) => {
                env.insert(n.clone(), scrutinee.clone());
                Ok(())
            }
            Pattern::Ctor { name, fields } => {
                let Ty::Enum {
                    name: en,
                    args: eargs,
                } = scrutinee
                else {
                    return Err(TypeError {
                        span,
                        message: "internal: ctor pattern on non-enum".into(),
                    });
                };
                let expected = self.variant_field_tys(en, name, eargs)?;
                let has_named = fields
                    .iter()
                    .any(|f| matches!(f, PatternField::Named(_, _)));
                let has_pos = fields.iter().any(|f| matches!(f, PatternField::Pos(_)));
                if has_named && has_pos {
                    return Err(TypeError {
                        span,
                        message:
                            "mixing named and positional fields in the same pattern is not allowed"
                                .into(),
                    });
                }
                if has_named {
                    for f in fields {
                        if let PatternField::Named(n, p) = f {
                            let (_, ty) =
                                expected.iter().find(|(fnm, _)| fnm == n).ok_or_else(|| {
                                    TypeError {
                                        span,
                                        message: format!("unknown field `{n}` in pattern"),
                                    }
                                })?;
                            self.bind_pattern(p, ty, env, span)?;
                        }
                    }
                } else {
                    if fields.len() != expected.len() {
                        return Err(TypeError {
                            span,
                            message: format!(
                                "pattern `{}` expects {} field(s), got {}",
                                name,
                                expected.len(),
                                fields.len()
                            ),
                        });
                    }
                    for (pf, (_, ty)) in fields.iter().zip(expected.iter()) {
                        let p = match pf {
                            PatternField::Pos(p) => p.as_ref(),
                            PatternField::Named(_, _) => {
                                return Err(TypeError {
                                    span,
                                    message: "cannot mix named and positional pattern fields"
                                        .into(),
                                });
                            }
                        };
                        self.bind_pattern(p, ty, env, span)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

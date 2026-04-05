use crate::span::Span;

#[derive(Clone, Debug)]
pub struct Module {
    pub span: Span,
    pub path: Vec<String>,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub span: Span,
    pub path: Vec<String>,
    pub alias: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Item {
    Type(TypeDef),
    Fn(FnDef),
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub span: Span,
    pub name: String,
    pub params: Vec<String>,
    pub variants: Vec<Variant>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug)]
pub struct FnDef {
    pub span: Span,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpr {
    Named {
        name: String,
        args: Vec<TypeExpr>,
    },
    Fun {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub span: Span,
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    String(String),
    Bool(bool),
    Unit,
    Var(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_arm: Box<Expr>,
        else_arm: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    Block {
        stmts: Vec<Stmt>,
        tail: Box<Expr>,
    },
    /// `while cond do body end` — condition re-evaluated each iteration; whole expression is `Unit`.
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    /// `Option[Int].Some(1)` or `Color.Red`
    Construct {
        typ: TypeExpr,
        variant: String,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    /// Unary `+` on `Int` (no-op).
    Pos,
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Wildcard,
    Bind(String),
    Ctor {
        name: String,
        fields: Vec<PatternField>,
    },
}

#[derive(Clone, Debug)]
pub enum PatternField {
    Named(String, Box<Pattern>),
    Pos(Box<Pattern>),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Expr(Expr),
}

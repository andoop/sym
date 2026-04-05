use crate::ast::{
    BinOp, Expr, ExprKind, Field, FnDef, Import, Item, MatchArm, Module, Param, Pattern,
    PatternField, Stmt, TypeDef, TypeExpr, UnaryOp, Variant,
};
use crate::lexer::Token;
use crate::span::Span;

pub struct Parser<'a> {
    pub source: &'a str,
    tokens: Vec<(Token, Span)>,
    pos: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, tokens: Vec<(Token, Span)>) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].0
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].1
    }

    fn bump(&mut self) -> (Token, Span) {
        let t = self.tokens[self.pos].0.clone();
        let s = self.tokens[self.pos].1;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        (t, s)
    }

    fn expect(&mut self, want: &str) -> Result<Span, ParseError> {
        let span = self.peek_span();
        let t = self.peek().clone();
        let ok = match want {
            "module" => matches!(t, Token::Module),
            "import" => matches!(t, Token::Import),
            "as" => matches!(t, Token::As),
            "type" => matches!(t, Token::TypeKw),
            "fn" => matches!(t, Token::Fn),
            "let" => matches!(t, Token::Let),
            "in" => matches!(t, Token::In),
            "if" => matches!(t, Token::If),
            "then" => matches!(t, Token::Then),
            "else" => matches!(t, Token::Else),
            "match" => matches!(t, Token::Match),
            "while" => matches!(t, Token::While),
            "do" => matches!(t, Token::Do),
            "end" => matches!(t, Token::End),
            "=" => matches!(t, Token::Eq),
            "->" => matches!(t, Token::Arrow),
            "=>" => matches!(t, Token::FatArrow),
            "|" => matches!(t, Token::Pipe),
            "(" => matches!(t, Token::LParen),
            ")" => matches!(t, Token::RParen),
            "[" => matches!(t, Token::LBracket),
            "]" => matches!(t, Token::RBracket),
            "," => matches!(t, Token::Comma),
            ";" => matches!(t, Token::Semi),
            ":" => matches!(t, Token::Colon),
            "." => matches!(t, Token::Dot),
            _ => false,
        };
        if ok {
            self.bump();
            Ok(span)
        } else {
            Err(ParseError {
                span,
                message: format!("expected `{want}`"),
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Ident(s) => {
                self.bump();
                Ok((s, span))
            }
            _ => Err(ParseError {
                span,
                message: "expected identifier".into(),
            }),
        }
    }

    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let start = self.peek_span().start;
        let mut path = Vec::new();
        if matches!(self.peek(), Token::Module) {
            self.expect("module")?;
            path.push(self.expect_ident()?.0);
            while matches!(self.peek(), Token::Dot) {
                self.bump();
                path.push(self.expect_ident()?.0);
            }
        }

        let mut imports = Vec::new();
        while matches!(self.peek(), Token::Import) {
            imports.push(self.parse_import()?);
        }

        let mut items = Vec::new();
        while !matches!(self.peek(), Token::Eof) {
            match self.peek() {
                Token::TypeKw => items.push(Item::Type(self.parse_type_def()?)),
                Token::Fn => items.push(Item::Fn(self.parse_fn_def()?)),
                _ => {
                    return Err(ParseError {
                        span: self.peek_span(),
                        message: "expected `type` or `fn`".into(),
                    });
                }
            }
        }

        let end = self.peek_span().end;
        Ok(Module {
            span: Span::new(start, end),
            path,
            imports,
            items,
        })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        let span = self.peek_span();
        self.expect("import")?;
        let mut path = vec![self.expect_ident()?.0];
        while matches!(self.peek(), Token::Dot) {
            self.bump();
            path.push(self.expect_ident()?.0);
        }
        let mut alias = None;
        if matches!(self.peek(), Token::As) {
            self.expect("as")?;
            alias = Some(self.expect_ident()?.0);
        }
        let end = self.peek_span().start;
        Ok(Import {
            span: Span::new(span.start, end),
            path,
            alias,
        })
    }

    fn parse_type_def(&mut self) -> Result<TypeDef, ParseError> {
        let start = self.peek_span().start;
        self.expect("type")?;
        let name = self.expect_ident()?.0;
        let mut params = Vec::new();
        if matches!(self.peek(), Token::LBracket) {
            self.expect("[")?;
            params.push(self.expect_ident()?.0);
            while matches!(self.peek(), Token::Comma) {
                self.bump();
                if matches!(self.peek(), Token::RBracket) {
                    break;
                }
                params.push(self.expect_ident()?.0);
            }
            self.expect("]")?;
        }
        self.expect("=")?;
        let mut variants = Vec::new();
        while matches!(self.peek(), Token::Pipe) {
            self.bump();
            let vname = self.expect_ident()?.0;
            let fields = if matches!(self.peek(), Token::LParen) {
                self.expect("(")?;
                let mut fs = Vec::new();
                if !matches!(self.peek(), Token::RParen) {
                    fs.push(self.parse_field()?);
                    while matches!(self.peek(), Token::Comma) {
                        self.bump();
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        fs.push(self.parse_field()?);
                    }
                }
                self.expect(")")?;
                fs
            } else {
                Vec::new()
            };
            variants.push(Variant {
                name: vname,
                fields,
            });
        }
        self.expect("end")?;
        let end = self.peek_span().start;
        Ok(TypeDef {
            span: Span::new(start, end),
            name,
            params,
            variants,
        })
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        let name = self.expect_ident()?.0;
        self.expect(":")?;
        let ty = self.parse_type_expr()?;
        Ok(Field { name, ty })
    }

    fn parse_fn_def(&mut self) -> Result<FnDef, ParseError> {
        let start = self.peek_span().start;
        self.expect("fn")?;
        let name = self.expect_ident()?.0;
        self.expect("(")?;
        let mut params = Vec::new();
        if !matches!(self.peek(), Token::RParen) {
            params.push(self.parse_param()?);
            while matches!(self.peek(), Token::Comma) {
                self.bump();
                if matches!(self.peek(), Token::RParen) {
                    break;
                }
                params.push(self.parse_param()?);
            }
        }
        self.expect(")")?;
        let mut ret = None;
        if matches!(self.peek(), Token::Arrow) {
            self.bump();
            ret = Some(self.parse_type_expr()?);
        }
        self.expect("=")?;
        let body = self.parse_expr()?;
        self.expect("end")?;
        let end = self.peek_span().start;
        Ok(FnDef {
            span: Span::new(start, end),
            name,
            params,
            ret,
            body,
        })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let name = self.expect_ident()?.0;
        self.expect(":")?;
        let ty = self.parse_type_expr()?;
        Ok(Param { name, ty })
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        self.parse_type_arrow()
    }

    fn parse_type_arrow(&mut self) -> Result<TypeExpr, ParseError> {
        let left = self.parse_type_app()?;
        if matches!(self.peek(), Token::Arrow) {
            self.bump();
            let ret = self.parse_type_arrow()?;
            Ok(TypeExpr::Fun {
                params: vec![left],
                ret: Box::new(ret),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_type_app(&mut self) -> Result<TypeExpr, ParseError> {
        if matches!(self.peek(), Token::LParen) {
            self.expect("(")?;
            let inner = self.parse_type_expr()?;
            self.expect(")")?;
            return Ok(inner);
        }
        let (name, _) = self.expect_ident()?;
        let mut args = Vec::new();
        if matches!(self.peek(), Token::LBracket) {
            self.expect("[")?;
            args.push(self.parse_type_expr()?);
            while matches!(self.peek(), Token::Comma) {
                self.bump();
                if matches!(self.peek(), Token::RBracket) {
                    break;
                }
                args.push(self.parse_type_expr()?);
            }
            self.expect("]")?;
        }
        Ok(TypeExpr::Named { name, args })
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::PipePipe) {
            let op_span = self.peek_span();
            self.bump();
            let right = self.parse_and()?;
            let span = Span::merge(left.span, right.span);
            left = Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
            let _ = op_span;
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_cmp()?;
        while matches!(self.peek(), Token::AmpAmp) {
            self.bump();
            let right = self.parse_cmp()?;
            let span = Span::merge(left.span, right.span);
            left = Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_cmp(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => Some(BinOp::Eq),
                Token::Ne => Some(BinOp::Ne),
                Token::Lt => Some(BinOp::Lt),
                Token::Le => Some(BinOp::Le),
                Token::Gt => Some(BinOp::Gt),
                Token::Ge => Some(BinOp::Ge),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let right = self.parse_add()?;
            let span = Span::merge(left.span, right.span);
            left = Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus => Some(BinOp::Add),
                Token::Minus => Some(BinOp::Sub),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let right = self.parse_mul()?;
            let span = Span::merge(left.span, right.span);
            left = Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => Some(BinOp::Mul),
                Token::Slash => Some(BinOp::Div),
                Token::Percent => Some(BinOp::Mod),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let right = self.parse_unary()?;
            let span = Span::merge(left.span, right.span);
            left = Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Token::Bang => {
                let s = self.peek_span();
                self.bump();
                let e = self.parse_unary()?;
                let span = Span::new(s.start, e.span.end);
                Ok(Expr {
                    span,
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(e),
                    },
                })
            }
            Token::Plus => {
                let s = self.peek_span();
                self.bump();
                let e = self.parse_unary()?;
                let span = Span::new(s.start, e.span.end);
                Ok(Expr {
                    span,
                    kind: ExprKind::Unary {
                        op: UnaryOp::Pos,
                        expr: Box::new(e),
                    },
                })
            }
            Token::Minus => {
                let s = self.peek_span();
                self.bump();
                let e = self.parse_unary()?;
                let span = Span::new(s.start, e.span.end);
                Ok(Expr {
                    span,
                    kind: ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(e),
                    },
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.parse_primary()?;
        while matches!(self.peek(), Token::LParen) {
            let lparen = self.peek_span();
            self.expect("(")?;
            let mut args = Vec::new();
            if !matches!(self.peek(), Token::RParen) {
                args.push(self.parse_expr()?);
                while matches!(self.peek(), Token::Comma) {
                    self.bump();
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    args.push(self.parse_expr()?);
                }
            }
            let close = self.expect(")")?;
            let span = Span::merge(e.span, close);
            let _ = lparen;
            e = Expr {
                span,
                kind: ExprKind::Call {
                    callee: Box::new(e),
                    args,
                },
            };
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.peek_span();
        match self.peek().clone() {
            Token::Int(n) => {
                let s = self.peek_span();
                self.bump();
                Ok(Expr {
                    span: s,
                    kind: ExprKind::Int(n),
                })
            }
            Token::String(st) => {
                let s = self.peek_span();
                self.bump();
                Ok(Expr {
                    span: s,
                    kind: ExprKind::String(st),
                })
            }
            Token::True => {
                self.bump();
                Ok(Expr {
                    span,
                    kind: ExprKind::Bool(true),
                })
            }
            Token::False => {
                self.bump();
                Ok(Expr {
                    span,
                    kind: ExprKind::Bool(false),
                })
            }
            Token::LParen => {
                self.bump();
                if matches!(self.peek(), Token::RParen) {
                    self.bump();
                    return Ok(Expr {
                        span,
                        kind: ExprKind::Unit,
                    });
                }
                let e = self.parse_expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::Match => self.parse_match(),
            Token::Let => self.parse_let(),
            Token::Do => self.parse_do(),
            Token::Ident(name) => {
                let name = name.clone();
                let s = self.peek_span();
                let is_upper = name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);
                self.bump();
                if is_upper
                    && (matches!(self.peek(), Token::LBracket) || matches!(self.peek(), Token::Dot))
                {
                    self.parse_ctor_after_type_name(name, s)
                } else {
                    Ok(Expr {
                        span: s,
                        kind: ExprKind::Var(name),
                    })
                }
            }
            _ => Err(ParseError {
                span,
                message: "unexpected token in expression".into(),
            }),
        }
    }

    fn parse_while(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect("while")?;
        let cond = self.parse_expr()?;
        self.expect("do")?;
        let body = self.parse_expr()?;
        let close = self.expect("end")?;
        Ok(Expr {
            span: Span::new(start, close.end),
            kind: ExprKind::While {
                cond: Box::new(cond),
                body: Box::new(body),
            },
        })
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect("if")?;
        let cond = self.parse_expr()?;
        self.expect("then")?;
        let then_arm = self.parse_expr()?;
        self.expect("else")?;
        let else_arm = if matches!(self.peek(), Token::If) {
            self.parse_if()?
        } else {
            self.parse_expr()?
        };
        let end = else_arm.span.end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::If {
                cond: Box::new(cond),
                then_arm: Box::new(then_arm),
                else_arm: Box::new(else_arm),
            },
        })
    }

    fn parse_match(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect("match")?;
        let scrutinee = self.parse_expr()?;
        let mut arms = Vec::new();
        while matches!(self.peek(), Token::Pipe) {
            let arm_span = self.peek_span();
            self.bump();
            let pattern = self.parse_pattern()?;
            self.expect("=>")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm {
                span: Span::merge(arm_span, body.span),
                pattern,
                body,
            });
        }
        self.expect("end")?;
        let end = self.peek_span().start;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let (name, _) = self.expect_ident()?;
        self.pattern_from_ident(name)
    }

    fn pattern_from_ident(&mut self, name: String) -> Result<Pattern, ParseError> {
        if name == "_" {
            return Ok(Pattern::Wildcard);
        }
        let is_ctor = name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        if !is_ctor {
            return Ok(Pattern::Bind(name));
        }
        if matches!(self.peek(), Token::LParen) {
            self.expect("(")?;
            let mut fields = Vec::new();
            if !matches!(self.peek(), Token::RParen) {
                fields.push(self.parse_pattern_field()?);
                while matches!(self.peek(), Token::Comma) {
                    self.bump();
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    fields.push(self.parse_pattern_field()?);
                }
            }
            self.expect(")")?;
            Ok(Pattern::Ctor { name, fields })
        } else {
            Ok(Pattern::Ctor {
                name,
                fields: Vec::new(),
            })
        }
    }

    fn parse_pattern_field(&mut self) -> Result<PatternField, ParseError> {
        let (n, _) = self.expect_ident()?;
        if matches!(self.peek(), Token::Colon) {
            self.bump();
            let p = self.parse_pattern()?;
            Ok(PatternField::Named(n, Box::new(p)))
        } else {
            let pat = self.pattern_from_ident(n)?;
            Ok(PatternField::Pos(Box::new(pat)))
        }
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect("let")?;
        let name = self.expect_ident()?.0;
        self.expect("=")?;
        let value = self.parse_expr()?;
        self.expect("in")?;
        let body = self.parse_expr()?;
        let end = body.span.end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Let {
                name,
                value: Box::new(value),
                body: Box::new(body),
            },
        })
    }

    fn parse_ctor_after_type_name(
        &mut self,
        type_name: String,
        start: Span,
    ) -> Result<Expr, ParseError> {
        let mut type_args = Vec::new();
        if matches!(self.peek(), Token::LBracket) {
            self.bump();
            type_args.push(self.parse_type_expr()?);
            while matches!(self.peek(), Token::Comma) {
                self.bump();
                if matches!(self.peek(), Token::RBracket) {
                    break;
                }
                type_args.push(self.parse_type_expr()?);
            }
            self.expect("]")?;
        }
        let ty = TypeExpr::Named {
            name: type_name,
            args: type_args,
        };
        self.expect(".")?;
        let variant = self.expect_ident()?.0;
        let mut args = Vec::new();
        let end = if matches!(self.peek(), Token::LParen) {
            self.expect("(")?;
            if !matches!(self.peek(), Token::RParen) {
                args.push(self.parse_expr()?);
                while matches!(self.peek(), Token::Comma) {
                    self.bump();
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    args.push(self.parse_expr()?);
                }
            }
            let close = self.expect(")")?;
            close.end
        } else {
            self.peek_span().start
        };
        Ok(Expr {
            span: Span::new(start.start, end),
            kind: ExprKind::Construct {
                typ: ty,
                variant,
                args,
            },
        })
    }

    fn parse_do(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span().start;
        self.expect("do")?;
        let mut stmts = Vec::new();
        loop {
            if matches!(self.peek(), Token::End) {
                return Err(ParseError {
                    span: self.peek_span(),
                    message: "`do` block needs a final expression before `end`".into(),
                });
            }
            if matches!(self.peek(), Token::Let) {
                self.expect("let")?;
                let name = self.expect_ident()?.0;
                self.expect("=")?;
                let value = self.parse_expr()?;
                self.expect(";")?;
                stmts.push(Stmt::Let { name, value });
                continue;
            }
            let e = self.parse_expr()?;
            if matches!(self.peek(), Token::Semi) {
                self.bump();
                stmts.push(Stmt::Expr(e));
                continue;
            }
            let close = self.peek_span();
            self.expect("end")?;
            return Ok(Expr {
                span: Span::new(start, close.end),
                kind: ExprKind::Block {
                    stmts,
                    tail: Box::new(e),
                },
            });
        }
    }
}

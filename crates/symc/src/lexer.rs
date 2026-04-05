use crate::span::Span;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Module,
    Import,
    As,
    TypeKw,
    Fn,
    Let,
    In,
    If,
    Then,
    Else,
    Match,
    While,
    Do,
    End,
    True,
    False,
    Arrow,    // ->
    FatArrow, // =>
    Eq,       // =
    EqEq,     // ==
    Pipe,     // |
    Comma,
    Semi,
    Colon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Lt,
    Le,
    Gt,
    Ge,
    AmpAmp,
    PipePipe,
    Ne,
    Ident(String),
    Int(i64),
    String(String),
    Eof,
}

pub fn lex(source: &str) -> Result<Vec<(Token, Span)>, LexError> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
            }
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                let s = i;
                i += 1;
                out.push((Token::LParen, Span::new(s, i)));
            }
            b')' => {
                let s = i;
                i += 1;
                out.push((Token::RParen, Span::new(s, i)));
            }
            b'[' => {
                let s = i;
                i += 1;
                out.push((Token::LBracket, Span::new(s, i)));
            }
            b']' => {
                let s = i;
                i += 1;
                out.push((Token::RBracket, Span::new(s, i)));
            }
            b',' => {
                let s = i;
                i += 1;
                out.push((Token::Comma, Span::new(s, i)));
            }
            b';' => {
                let s = i;
                i += 1;
                out.push((Token::Semi, Span::new(s, i)));
            }
            b':' => {
                let s = i;
                i += 1;
                out.push((Token::Colon, Span::new(s, i)));
            }
            b'.' => {
                let s = i;
                i += 1;
                out.push((Token::Dot, Span::new(s, i)));
            }
            b'|' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'|' {
                    i += 1;
                    out.push((Token::PipePipe, Span::new(s, i)));
                } else {
                    out.push((Token::Pipe, Span::new(s, i)));
                }
            }
            b'+' => {
                let s = i;
                i += 1;
                out.push((Token::Plus, Span::new(s, i)));
            }
            b'*' => {
                let s = i;
                i += 1;
                out.push((Token::Star, Span::new(s, i)));
            }
            b'/' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    i += 2;
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    let s = i;
                    i += 1;
                    out.push((Token::Slash, Span::new(s, i)));
                }
            }
            b'%' => {
                let s = i;
                i += 1;
                out.push((Token::Percent, Span::new(s, i)));
            }
            b'!' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    out.push((Token::Ne, Span::new(s, i)));
                } else {
                    out.push((Token::Bang, Span::new(s, i)));
                }
            }
            b'=' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'>' {
                    i += 1;
                    out.push((Token::FatArrow, Span::new(s, i)));
                } else if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    out.push((Token::EqEq, Span::new(s, i)));
                } else {
                    out.push((Token::Eq, Span::new(s, i)));
                }
            }
            b'<' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    out.push((Token::Le, Span::new(s, i)));
                } else {
                    out.push((Token::Lt, Span::new(s, i)));
                }
            }
            b'>' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                    out.push((Token::Ge, Span::new(s, i)));
                } else {
                    out.push((Token::Gt, Span::new(s, i)));
                }
            }
            b'&' => {
                let s = i;
                if i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                    i += 2;
                    out.push((Token::AmpAmp, Span::new(s, i)));
                } else {
                    return Err(LexError {
                        offset: i,
                        message: "expected '&' after '&'",
                    });
                }
            }
            b'-' => {
                let s = i;
                i += 1;
                if i < bytes.len() && bytes[i] == b'>' {
                    i += 1;
                    out.push((Token::Arrow, Span::new(s, i)));
                } else {
                    out.push((Token::Minus, Span::new(s, i)));
                }
            }
            b'"' => {
                let s = i;
                i += 1;
                let mut buf = String::new();
                let mut closed = false;
                while i < bytes.len() {
                    match bytes[i] {
                        b'"' => {
                            i += 1;
                            closed = true;
                            break;
                        }
                        b'\\' => {
                            i += 1;
                            if i >= bytes.len() {
                                return Err(LexError {
                                    offset: i,
                                    message: "unterminated string escape",
                                });
                            }
                            match bytes[i] {
                                b'n' => buf.push('\n'),
                                b'r' => buf.push('\r'),
                                b't' => buf.push('\t'),
                                b'\\' => buf.push('\\'),
                                b'"' => buf.push('"'),
                                _ => {
                                    return Err(LexError {
                                        offset: i,
                                        message: "unknown escape in string",
                                    });
                                }
                            }
                            i += 1;
                        }
                        b'\n' => {
                            return Err(LexError {
                                offset: i,
                                message: "newline in string",
                            });
                        }
                        _ => {
                            let tail = &source[i..];
                            let c = tail.chars().next().unwrap();
                            i += c.len_utf8();
                            buf.push(c);
                        }
                    }
                }
                if !closed {
                    return Err(LexError {
                        offset: s,
                        message: "unterminated string",
                    });
                }
                out.push((Token::String(buf), Span::new(s, i)));
            }
            b'0'..=b'9' => {
                let s = i;
                let mut n: i64 = 0;
                while i < bytes.len() {
                    match bytes[i] {
                        d @ b'0'..=b'9' => {
                            n = n.saturating_mul(10).saturating_add(i64::from(d - b'0'));
                            i += 1;
                        }
                        b'_' => {
                            i += 1;
                            if i >= bytes.len() || !bytes[i].is_ascii_digit() {
                                return Err(LexError {
                                    offset: i.saturating_sub(1),
                                    message: "`_` in number must be followed by a digit",
                                });
                            }
                        }
                        _ => break,
                    }
                }
                out.push((Token::Int(n), Span::new(s, i)));
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let s = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let ident = source[s..i].to_string();
                let tok = match ident.as_str() {
                    "module" => Token::Module,
                    "import" => Token::Import,
                    "as" => Token::As,
                    "type" => Token::TypeKw,
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "in" => Token::In,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "while" => Token::While,
                    "do" => Token::Do,
                    "end" => Token::End,
                    "true" => Token::True,
                    "false" => Token::False,
                    _ => Token::Ident(ident),
                };
                out.push((tok, Span::new(s, i)));
            }
            _ => {
                return Err(LexError {
                    offset: i,
                    message: "unexpected character",
                });
            }
        }
    }

    out.push((Token::Eof, Span::new(bytes.len(), bytes.len())));
    Ok(out)
}

#[derive(Debug)]
pub struct LexError {
    pub offset: usize,
    pub message: &'static str,
}

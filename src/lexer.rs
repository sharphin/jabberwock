use crate::diag::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i128),
    Str(String),
    Ident(String),
    // キーワード
    Fn,
    Let,
    If,
    Else,
    While,
    Loop,
    For,
    In,
    Break,
    Continue,
    Match,
    Case,
    Default,
    Return,
    Struct,
    Enum,
    Own,
    As,
    Ref,
    Mut,
    True,
    False,
    // 記号
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Arrow,
    Dot,
    Colon,
    Semi,
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    PlusPlus,
    MinusMinus,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Question,
    AndAnd,
    OrOr,
    Eof,
}

impl Tok {
    /// エラーメッセージ用の表示
    pub fn describe(&self) -> String {
        match self {
            Tok::Int(n) => format!("整数 `{}`", n),
            Tok::Str(_) => "文字列".to_string(),
            Tok::Ident(s) => format!("名前 `{}`", s),
            Tok::Eof => "ファイルの終わり".to_string(),
            other => format!("`{}`", other.symbol()),
        }
    }

    fn symbol(&self) -> &'static str {
        match self {
            Tok::Fn => "fn",
            Tok::Let => "let",
            Tok::If => "if",
            Tok::Else => "else",
            Tok::While => "while",
            Tok::Loop => "loop",
            Tok::For => "for",
            Tok::In => "in",
            Tok::Break => "break",
            Tok::Continue => "continue",
            Tok::Match => "match",
            Tok::Case => "case",
            Tok::Default => "default",
            Tok::Return => "return",
            Tok::Struct => "struct",
            Tok::Enum => "enum",
            Tok::Own => "own",
            Tok::As => "as",
            Tok::Ref => "ref",
            Tok::Mut => "mut",
            Tok::True => "true",
            Tok::False => "false",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::Comma => ",",
            Tok::Arrow => "->",
            Tok::Dot => ".",
            Tok::Colon => ":",
            Tok::Semi => ";",
            Tok::Assign => "=",
            Tok::PlusEq => "+=",
            Tok::MinusEq => "-=",
            Tok::StarEq => "*=",
            Tok::SlashEq => "/=",
            Tok::PercentEq => "%=",
            Tok::PlusPlus => "++",
            Tok::MinusMinus => "--",
            Tok::EqEq => "==",
            Tok::NotEq => "!=",
            Tok::Lt => "<",
            Tok::Le => "<=",
            Tok::Gt => ">",
            Tok::Ge => ">=",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Star => "*",
            Tok::Slash => "/",
            Tok::Percent => "%",
            Tok::Bang => "!",
            Tok::Question => "?",
            Tok::AndAnd => "&&",
            Tok::OrOr => "||",
            _ => "?",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while i < chars.len() {
        let c = chars[i];
        let start_col = col;

        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
                col += 1;
            }
            '\n' => {
                // 文の区切りは `;`。改行はただの空白
                i += 1;
                line += 1;
                col = 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
            }
            '0'..='9' => {
                let s = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    col += 1;
                }
                let text: String = chars[s..i].iter().collect();
                let span = Span { line, col: start_col, len: i - s };
                // 型は型検査が決める（相手の型に合わせる）。ここでは u64 に収まるかだけ見る
                let n = text
                    .parse::<u64>()
                    .map(i128::from)
                    .map_err(|_| Diagnostic::new("整数が大きすぎます").primary(span, "どの整数型にも収まりません"))?;
                out.push(Token { tok: Tok::Int(n), span });
            }
            c if c.is_alphabetic() || c == '_' => {
                let s = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                    col += 1;
                }
                let text: String = chars[s..i].iter().collect();
                let tok = match text.as_str() {
                    "fn" => Tok::Fn,
                    "let" => Tok::Let,
                    "if" => Tok::If,
                    "else" => Tok::Else,
                    "while" => Tok::While,
                    "loop" => Tok::Loop,
                    "for" => Tok::For,
                    "in" => Tok::In,
                    "break" => Tok::Break,
                    "continue" => Tok::Continue,
                    "match" => Tok::Match,
                    "case" => Tok::Case,
                    "default" => Tok::Default,
                    "return" => Tok::Return,
                    "struct" => Tok::Struct,
                    "enum" => Tok::Enum,
                    "own" => Tok::Own,
                    "as" => Tok::As,
                    "ref" => Tok::Ref,
                    "mut" => Tok::Mut,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    _ => Tok::Ident(text),
                };
                out.push(Token {
                    tok,
                    span: Span { line, col: start_col, len: i - s },
                });
            }
            '"' => {
                i += 1;
                col += 1;
                let mut text = String::new();
                loop {
                    if i >= chars.len() || chars[i] == '\n' {
                        return Err(Diagnostic::new("文字列が閉じていません").primary(
                            Span { line, col: start_col, len: 1 },
                            "この `\"` に対応する閉じ `\"` が同じ行にありません",
                        ));
                    }
                    let ch = chars[i];
                    if ch == '"' {
                        i += 1;
                        col += 1;
                        break;
                    }
                    if ch == '\\' {
                        let esc = chars.get(i + 1).copied();
                        let mapped = match esc {
                            Some('n') => '\n',
                            Some('t') => '\t',
                            Some('"') => '"',
                            Some('\\') => '\\',
                            _ => {
                                return Err(Diagnostic::new("未知のエスケープです").primary(
                                    Span { line, col, len: 2 },
                                    "使えるのは \\n \\t \\\" \\\\ です",
                                ));
                            }
                        };
                        text.push(mapped);
                        i += 2;
                        col += 2;
                    } else {
                        text.push(ch);
                        i += 1;
                        col += 1;
                    }
                }
                out.push(Token {
                    tok: Tok::Str(text),
                    span: Span { line, col: start_col, len: col - start_col },
                });
            }
            _ => {
                let next = chars.get(i + 1).copied();
                let (tok, n) = match (c, next) {
                    ('=', Some('=')) => (Tok::EqEq, 2),
                    ('!', Some('=')) => (Tok::NotEq, 2),
                    ('<', Some('=')) => (Tok::Le, 2),
                    ('>', Some('=')) => (Tok::Ge, 2),
                    ('&', Some('&')) => (Tok::AndAnd, 2),
                    ('|', Some('|')) => (Tok::OrOr, 2),
                    ('-', Some('>')) => (Tok::Arrow, 2),
                    ('+', Some('=')) => (Tok::PlusEq, 2),
                    ('-', Some('=')) => (Tok::MinusEq, 2),
                    ('*', Some('=')) => (Tok::StarEq, 2),
                    ('/', Some('=')) => (Tok::SlashEq, 2),
                    ('%', Some('=')) => (Tok::PercentEq, 2),
                    ('+', Some('+')) => (Tok::PlusPlus, 2),
                    ('-', Some('-')) => (Tok::MinusMinus, 2),
                    ('=', _) => (Tok::Assign, 1),
                    ('<', _) => (Tok::Lt, 1),
                    ('>', _) => (Tok::Gt, 1),
                    ('(', _) => (Tok::LParen, 1),
                    (')', _) => (Tok::RParen, 1),
                    ('{', _) => (Tok::LBrace, 1),
                    ('}', _) => (Tok::RBrace, 1),
                    (',', _) => (Tok::Comma, 1),
                    ('.', _) => (Tok::Dot, 1),
                    (':', _) => (Tok::Colon, 1),
                    (';', _) => (Tok::Semi, 1),
                    ('+', _) => (Tok::Plus, 1),
                    ('-', _) => (Tok::Minus, 1),
                    ('*', _) => (Tok::Star, 1),
                    ('/', _) => (Tok::Slash, 1),
                    ('%', _) => (Tok::Percent, 1),
                    ('!', _) => (Tok::Bang, 1),
                    ('?', _) => (Tok::Question, 1),
                    _ => {
                        return Err(Diagnostic::new("使えない文字です").primary(
                            Span { line, col, len: 1 },
                            format!("`{}` はこの言語にありません", c),
                        ));
                    }
                };
                out.push(Token {
                    tok,
                    span: Span { line, col: start_col, len: n },
                });
                i += n;
                col += n;
            }
        }
    }

    out.push(Token {
        tok: Tok::Eof,
        span: Span { line, col, len: 1 },
    });
    Ok(out)
}

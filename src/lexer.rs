use crate::diag::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i128),
    Str(String),
    /// `{...}` の埋め込みがある文字列 `"x = {x}"`
    Interp(Vec<StrPart>),
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
    Static,
    Interface,
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
            Tok::Str(_) | Tok::Interp(_) => "文字列".to_string(),
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
            Tok::Static => "static",
            Tok::Interface => "interface",
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

/// 埋め込みのある文字列の部品
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Lit(String),
    /// `{` と `}` の中の式のトークン（最後は Eof）
    Code(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
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
                    "static" => Tok::Static,
                    "interface" => Tok::Interface,
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
                let mut parts: Vec<StrPart> = Vec::new();
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
                    if (ch == '{' || ch == '}') && chars.get(i + 1) == Some(&ch) {
                        // `{{` / `}}` は `{` / `}` そのもの
                        text.push(ch);
                        i += 2;
                        col += 2;
                        continue;
                    }
                    if ch == '}' {
                        return Err(Diagnostic::new("文字列の中の `}` に対応する `{` がありません")
                            .primary(Span { line, col, len: 1 }, "`}` そのものを書くなら `}}` と書いてください"));
                    }
                    if ch == '{' {
                        // 埋め込み: 対応する `}` までを式として字句解析する
                        let open_col = col;
                        let mut depth = 1;
                        let mut j = i + 1;
                        while j < chars.len() && chars[j] != '\n' {
                            match chars[j] {
                                '{' => depth += 1,
                                '}' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                '"' => {
                                    return Err(Diagnostic::new("埋め込みの `{` が閉じていません")
                                        .primary(Span { line, col: open_col, len: 1 }, "この `{` の `}` より先に `\"` がある")
                                        .note("`{` そのものは `{{` と書きます。`{ }` の中には文字列を書けないので、先に変数に入れてください"))
                                }
                                _ => {}
                            }
                            j += 1;
                        }
                        if j >= chars.len() || chars[j] != '}' {
                            return Err(Diagnostic::new("埋め込みの `{` が閉じていません").primary(
                                Span { line, col: open_col, len: 1 },
                                "`{` そのものを書くなら `{{` と書いてください",
                            ));
                        }
                        let code: String = chars[i + 1..j].iter().collect();
                        if code.trim().is_empty() {
                            return Err(Diagnostic::new("埋め込みの `{ }` が空です")
                                .primary(Span { line, col: open_col, len: j - i + 1 }, "中に式を書いてください"));
                        }
                        let mut toks = lex(&code)?;
                        for t in &mut toks {
                            // 部分の字句解析の位置を、元のファイルでの位置に直す
                            t.span = Span { line, col: t.span.col + open_col, len: t.span.len };
                        }
                        if !text.is_empty() {
                            parts.push(StrPart::Lit(std::mem::take(&mut text)));
                        }
                        parts.push(StrPart::Code(toks));
                        col += j - i + 1;
                        i = j + 1;
                        continue;
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
                let tok = if parts.is_empty() {
                    Tok::Str(text)
                } else {
                    if !text.is_empty() {
                        parts.push(StrPart::Lit(text));
                    }
                    Tok::Interp(parts)
                };
                out.push(Token {
                    tok,
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

//! Lexing for the parser.

use std::hash::Hash;

use regex_automata::{Anchored, Input, PatternID};

pub use regex_automata::meta::Regex;

use crate::LexIt;

/// A span in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The start of the span, inclusive
    pub start: usize,
    /// The end of the span, exclusive
    pub end: usize,
}

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a, T> {
    /// A token carrying a literal value.
    Literal(LiteralToken<'a>),
    /// End of file token.
    Eof,
    /// User-defined token.
    Custom(T),
}

macro_rules! impl_token_from_literal {
    ($($type:ty),+$(,)?) => {
        $(
            impl<'a, T> From<$type> for Token<'a, T> {
                fn from(value: $type) -> Self {
                    Token::Literal(LiteralToken::from(value))
                }
            }
        )+
    };
}

impl_token_from_literal! {
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128,
    f32, f64,
    char, String,
    bool,
}

impl<'a, T> From<&'a str> for Token<'a, T> {
    fn from(value: &'a str) -> Self {
        Token::Literal(LiteralToken::Str(value))
    }
}

/// A literal token type.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralToken<'a> {
    /// A token carrying an i8 value.
    I8(i8),
    /// A token carrying an i16 value.
    I16(i16),
    /// A token carrying an i32 value.
    I32(i32),
    /// A token carrying an i64 value.
    I64(i64),
    /// A token carrying an i128 value.
    I128(i128),
    /// A token carrying an u8 value.
    U8(u8),
    /// A token carrying an u16 value.
    U16(u16),
    /// A token carrying an u32 value.
    U32(u32),
    /// A token carrying an u64 value.
    U64(u64),
    /// A token carrying an u128 value.
    U128(u128),
    /// A token carrying a f32 value.
    F32(f32),
    /// A token carrying a f64 value.
    F64(f64),
    /// A token carrying a char value.
    Char(char),
    /// A token carrying a str value.
    Str(&'a str),
    /// A token carrying a string value.
    String(String),
    /// A token carrying a boolean value.
    Bool(bool),
}

macro_rules! impl_literal_token_from {
    ($($name:ident => $type:ty),+$(,)?) => {
        $(
            impl<'a> From<$type> for LiteralToken<'a> {
                fn from(value: $type) -> Self {
                    LiteralToken::$name(value)
                }
            }
        )+
    };
}

impl_literal_token_from! {
    I8 => i8,
    I16 => i16,
    I32 => i32,
    I64 => i64,
    I128 => i128,
    U8 => u8,
    U16 => u16,
    U32 => u32,
    U64 => u64,
    U128 => u128,
    F32 => f32,
    F64 => f64,
    Char => char,
    String => String,
    Bool => bool,
}

macro_rules! impl_literal_token_integer {
    ($($function:ident -> $type:ident),+$(,)?) => {
        $(
            impl<'a> LiteralToken<'a> {
                #[doc = concat!("Try converting the token to an ", stringify!($type), " value.")]
                pub fn $function(&self) -> Option<$type> {
                    match *self {
                        LiteralToken::I8(v) => v.try_into().ok(),
                        LiteralToken::I16(v) => v.try_into().ok(),
                        LiteralToken::I32(v) => v.try_into().ok(),
                        LiteralToken::I64(v) => v.try_into().ok(),
                        LiteralToken::I128(v) => v.try_into().ok(),
                        LiteralToken::U8(v) => v.try_into().ok(),
                        LiteralToken::U16(v) => v.try_into().ok(),
                        LiteralToken::U32(v) => v.try_into().ok(),
                        LiteralToken::U64(v) => v.try_into().ok(),
                        LiteralToken::U128(v) => v.try_into().ok(),
                        _ => None,
                    }
                }
            }
        )+
    };
}

impl_literal_token_integer! {
    as_i8 -> i8,
    as_i16 -> i16,
    as_i32 -> i32,
    as_i64 -> i64,
    as_i128 -> i128,
    as_u8 -> u8,
    as_u16 -> u16,
    as_u32 -> u32,
    as_u64 -> u64,
    as_u128 -> u128,
}

impl<'a> LiteralToken<'a> {
    /// Try converting the token to a `char` value.
    pub fn as_char(&self) -> Option<char> {
        match *self {
            LiteralToken::Char(c) => Some(c),
            _ => None,
        }
    }

    /// Try converting the token to a `String` value.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            LiteralToken::String(ref s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Try converting the token to a `bool` value.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            LiteralToken::Bool(b) => Some(b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cursor {
    cursor: usize,
    start: usize,
}

/// TODO
#[derive(Clone)]
pub struct LexerState<'a> {
    start: usize,
    cursor: usize,
    input: &'a str,
}

impl<'a> LexerState<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            start: 0,
            cursor: 0,
            input,
        }
    }

    /// TODO
    pub fn run(&mut self, regex: &Regex) -> Option<PatternID> {
        let input = Input::new(self.input)
            .range(self.cursor..)
            .anchored(Anchored::Yes);
        let end = regex.search_half(&input)?;
        self.start = self.cursor;
        self.cursor = end.offset();
        Some(end.pattern())
    }

    /// TODO
    pub fn lexeme(&self) -> &'a str {
        &self.input[self.start..self.cursor]
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> Cursor {
        Cursor {
            start: self.start,
            cursor: self.cursor,
        }
    }

    pub fn span(&self) -> Span {
        Span {
            start: self.start,
            end: self.cursor,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cursor >= self.input.len()
    }

    pub fn advance_to_cursor(&mut self, cursor: Cursor) {
        self.start = cursor.start;
        self.cursor = cursor.cursor;
    }
}

/// A lexer for a single character.
#[derive(Clone)]
pub struct CharLexer;

impl LexIt for CharLexer {
    type Token<'a> = char;

    fn new() -> Self {
        Self
    }

    fn next<'a>(&self, lexbuf: &mut LexerState<'a>) -> Option<Self::Token<'a>> {
        thread_local! {
            static REGEX: Regex = Regex::new(r".").unwrap();
        }
        REGEX.with(|regex| {
            if lexbuf.run(regex).is_some() {
                let lexeme = lexbuf.lexeme();
                Some(lexeme.chars().next().unwrap())
            } else {
                None
            }
        })
    }
}

//! Lexing for the parser.

use std::hash::Hash;

use regex_automata::{Anchored, Input, PatternID};

pub use regex_automata::meta::Regex;

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
pub enum Token<T> {
    /// A token carrying a literal value.
    Literal(LiteralToken),
    /// End of file token.
    Eof,
    /// User-defined token.
    Custom(T),
}

macro_rules! impl_token_from_literal {
    ($($type:ty),+$(,)?) => {
        $(
            impl<T> From<$type> for Token<T> {
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

impl<T> From<&str> for Token<T> {
    fn from(value: &str) -> Self {
        Token::Literal(LiteralToken::String(value.to_string()))
    }
}

/// A literal token type.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralToken {
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
    /// A token carrying a string value.
    String(String),
    /// A token carrying a boolean value.
    Bool(bool),
}

macro_rules! impl_literal_token_from {
    ($($name:ident => $type:ty),+$(,)?) => {
        $(
            impl From<$type> for LiteralToken {
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
            impl LiteralToken {
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

impl LiteralToken {
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

/// A lexer for the parser.
pub trait Lexer<'a> {
    /// The lexed token type.
    type Token: PartialEq;
    /// The cursor type used for tracking positions.
    type Cursor: Clone + Hash + Eq + PartialOrd;

    /// Create a new lexer from the given input.
    fn new(input: &'a str) -> Self;

    /// Get the current lexeme.
    fn lexeme(&self) -> &str;

    /// Get the current span.
    fn span(&self) -> Span;

    /// Get the current position.
    fn cursor(&self) -> &Self::Cursor;

    /// Consume the next token.
    fn next(&mut self) -> Option<Self::Token>;

    /// Whether the lexer is at the end of the input.
    fn is_empty(&self) -> bool;

    /// Advance the lexer to the given cursor.
    fn advance_to_cursor(&mut self, cursor: &Self::Cursor);

    /// Fork the lexer.
    fn fork(&self) -> Self;
}

/// A lexer for a single character.
#[derive(Clone)]
pub struct CharLexer<'a> {
    cursor: usize,
    current: char,
    remaining: &'a str,
}

impl<'a> Lexer<'a> for CharLexer<'a> {
    type Token = char;
    type Cursor = usize;

    fn new(input: &'a str) -> Self {
        Self {
            cursor: 0,
            current: input.chars().next().unwrap_or_default(),
            remaining: input,
        }
    }

    fn lexeme(&self) -> &str {
        self.remaining
    }

    fn span(&self) -> Span {
        Span {
            start: self.cursor,
            end: self.cursor + self.current.len_utf8(),
        }
    }

    fn cursor(&self) -> &Self::Cursor {
        &self.cursor
    }

    fn next(&mut self) -> Option<Self::Token> {
        let start = self.cursor;
        let mut chars = self.remaining.chars();
        if let Some(c) = chars.next() {
            let advance = c.len_utf8();
            let remaining = chars.as_str();

            self.cursor = start + advance;
            self.current = c;
            self.remaining = remaining;

            Some(c)
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }

    fn advance_to_cursor(&mut self, cursor: &usize) {
        self.cursor = *cursor;
        self.remaining = &self.remaining[self.cursor..];
        self.current = self.remaining.chars().next().unwrap_or_default();
    }

    fn fork(&self) -> Self {
        self.clone()
    }
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
    pub fn lexeme(&self) -> &str {
        &self.input[self.start..self.cursor]
    }
}

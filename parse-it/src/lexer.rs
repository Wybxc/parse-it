//! Lexing for the parser.

use std::{hash::Hash, rc::Rc};

use regex_automata::{Anchored, Input, PatternID};

pub use regex_automata::meta::Regex;

use crate::{LexIt, Memo};

/// A span in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The start of the span, inclusive
    pub start: usize,
    /// The end of the span, exclusive
    pub end: usize,
}

/// A trait for types that can be converted to another type.
pub trait TryConvert<T> {
    /// Try to convert the value to the target type.
    fn try_convert(&self) -> Option<T>;
}

impl<T: Copy> TryConvert<T> for T {
    fn try_convert(&self) -> Option<T> {
        Some(*self)
    }
}

/// Cursor position in the input.
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
    memo: Rc<Memo<Cursor, (PatternID, *const Regex)>>,
}

impl<'a> LexerState<'a> {
    /// Create a new lexer state.
    pub fn new(input: &'a str) -> Self {
        Self {
            start: 0,
            cursor: 0,
            input,
            memo: Default::default(),
        }
    }

    /// Run the lexer against the given regex.
    pub fn run(&mut self, regex: &Regex) -> Option<PatternID> {
        let cursor = self.cursor();
        if let Some(((pattern, re), end)) = self.memo.get(&cursor) {
            if std::ptr::addr_eq(re, regex) {
                self.advance_to_cursor(end);
                return Some(pattern);
            }
        }
        let input = Input::new(self.input)
            .range(self.cursor..)
            .anchored(Anchored::Yes);
        let end = regex.search_half(&input)?;
        self.start = self.cursor;
        self.cursor = end.offset();
        let pattern = end.pattern();
        
        self.memo.insert(cursor, ((pattern, regex), self.cursor()));
        Some(pattern)
    }

    /// Get the lexeme of the current token.
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

    /// Get the span of the current token.
    pub fn span(&self) -> Span {
        Span {
            start: self.start,
            end: self.cursor,
        }
    }

    /// Check if the lexer is at the end of the input.
    pub fn is_empty(&self) -> bool {
        self.cursor >= self.input.len()
    }

    /// Advance the lexer to the given cursor position.
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

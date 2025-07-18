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

pub trait TryConvert<T> {
    fn try_convert(&self) -> Option<T>;
}

impl<T: Copy> TryConvert<T> for T {
    fn try_convert(&self) -> Option<T> {
        Some(*self)
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

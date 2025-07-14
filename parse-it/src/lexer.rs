//! Lexing for the parser.

use std::hash::Hash;

/// A span in the source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The start of the span, inclusive
    pub start: usize,
    /// The end of the span, exclusive
    pub end: usize,
}

/// A lexer for the parser.
pub trait Lexer<'a> {
    /// The lexed token type.
    type Token: Eq;
    /// The lexeme type.
    type Position: Clone + Hash + Eq + PartialOrd;

    /// Create a new lexer from the given input.
    fn new(input: &'a str) -> Self;

    /// Get the current lexeme.
    fn lexeme(&self) -> &str;

    /// Get the current span.
    fn span(&self) -> Span;

    /// Get the current position.
    fn pos(&self) -> &Self::Position;

    /// Consume the next token.
    fn next(&mut self) -> Option<Self::Token>;

    /// Whether the lexer is at the end of the input.
    fn is_empty(&self) -> bool;

    /// Advance the lexer to the given lexeme.
    fn advance_to_pos(&mut self, pos: &Self::Position);

    /// Fork the lexer.
    fn fork(&self) -> Self;
}

/// A lexer for a single character.
#[derive(Clone)]
pub struct CharLexer<'a> {
    pos: usize,
    current: char,
    remaining: &'a str,
}

impl<'a> Lexer<'a> for CharLexer<'a> {
    type Token = char;
    type Position = usize;

    fn new(input: &'a str) -> Self {
        Self {
            pos: 0,
            current: input.chars().next().unwrap_or_default(),
            remaining: input,
        }
    }

    fn lexeme(&self) -> &str {
        self.remaining
    }

    fn span(&self) -> Span {
        Span {
            start: self.pos,
            end: self.pos + self.current.len_utf8(),
        }
    }

    fn pos(&self) -> &Self::Position {
        &self.pos
    }

    fn next(&mut self) -> Option<Self::Token> {
        let start = self.pos;
        let mut chars = self.remaining.chars();
        if let Some(c) = chars.next() {
            let advance = c.len_utf8();
            let remaining = chars.as_str();

            self.pos = start + advance;
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

    fn advance_to_pos(&mut self, pos: &usize) {
        self.pos = *pos;
        self.remaining = &self.remaining[self.pos..];
        self.current = self.remaining.chars().next().unwrap_or_default();
    }

    fn fork(&self) -> Self {
        self.clone()
    }
}

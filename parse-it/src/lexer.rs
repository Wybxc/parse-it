//! Lexing for the parser.

use std::hash::Hash;

/// A lexer for the parser.
pub trait Lexer<'a> {
    /// The lexed token type.
    type Token: Eq;
    /// The position type.
    type Position: Copy + Eq + Ord + Hash + 'static;

    /// Create a new lexer from the given input.
    fn new(input: &'a str) -> Self;

    /// Get the current parsing position.
    fn pos(&self) -> Self::Position;

    /// Consume the next token.
    fn next(&mut self) -> (Option<Self::Token>, usize);

    /// Whether the lexer is at the end of the input.
    fn is_empty(&self) -> bool;

    /// Advance the lexer to the given position.
    fn advance_to_pos(&mut self, pos: Self::Position);

    /// Fork the lexer.
    fn fork(&self) -> Self;
}

/// A lexer for a single character.
pub struct CharLexer<'a> {
    pos: usize,
    remaining: &'a str,
}

impl<'a> Lexer<'a> for CharLexer<'a> {
    type Token = char;
    type Position = usize;

    fn new(input: &'a str) -> Self {
        Self {
            pos: 0,
            remaining: input,
        }
    }

    fn pos(&self) -> Self::Position {
        self.pos
    }

    fn next(&mut self) -> (Option<Self::Token>, usize) {
        let start = self.pos;
        let mut chars = self.remaining.chars();
        if let Some(c) = chars.next() {
            let advance = c.len_utf8();
            let remaining = chars.as_str();

            self.pos = start + advance;
            self.remaining = remaining;

            (Some(c), advance)
        } else {
            (None, 0)
        }
    }

    fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }

    fn advance_to_pos(&mut self, pos: Self::Position) {
        let advance = pos - self.pos;
        self.pos = pos;
        self.remaining = &self.remaining[advance..];
    }

    fn fork(&self) -> Self {
        Self {
            pos: self.pos,
            remaining: self.remaining,
        }
    }
}

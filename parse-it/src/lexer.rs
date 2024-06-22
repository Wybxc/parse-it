//! Lexing for the parser.

use std::cell::Cell;
use std::hash::Hash;

/// A lexer for the parser.
pub trait Lexer<'a> {
    /// The lexed token type.
    type Token: Copy + Eq;
    /// The position type.
    type Position: Copy + Eq + Ord + Hash;

    /// Create a new lexer from the given input.
    fn new(input: &'a str) -> Self;

    /// Get the current parsing position.
    fn pos(&self) -> Self::Position;

    /// Consume the next token.
    fn next(&self) -> (Option<Self::Token>, usize);

    /// Whether the lexer is at the end of the input.
    fn is_empty(&self) -> bool;

    /// Advance the lexer to the given position.
    fn advance_to_pos(&self, pos: Self::Position);

    /// Fork the lexer.
    fn fork(&self) -> Self;
}

/// A lexer for a single character.
pub struct CharLexer<'a> {
    pos: Cell<usize>,
    remaining: Cell<&'a str>,
}

impl<'a> Lexer<'a> for CharLexer<'a> {
    type Token = char;
    type Position = usize;

    fn new(input: &'a str) -> Self {
        Self {
            pos: Cell::new(0),
            remaining: Cell::new(input),
        }
    }

    fn pos(&self) -> Self::Position {
        self.pos.get()
    }

    fn next(&self) -> (Option<Self::Token>, usize) {
        let start = self.pos.get();
        let mut chars = self.remaining.get().chars();
        if let Some(c) = chars.next() {
            let advance = c.len_utf8();
            let remaining = chars.as_str();

            self.pos.set(start + advance);
            self.remaining.set(remaining);

            (Some(c), advance)
        } else {
            (None, 0)
        }
    }

    fn is_empty(&self) -> bool {
        self.remaining.get().is_empty()
    }

    fn advance_to_pos(&self, pos: Self::Position) {
        let advance = pos - self.pos.get();
        self.pos.set(pos);
        self.remaining.set(&self.remaining.get()[advance..]);
    }

    fn fork(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            remaining: self.remaining.clone(),
        }
    }
}

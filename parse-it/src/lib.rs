//! # Parse It
//!
//! *A user-friendly, opinionated parser generator for Rust.*
//!
//! ## Example
//!
//! ```rust
//! use parse_it::{ParseIt, parse_it};
//!
//! #[derive(Debug, Clone)]
//! pub enum Instr {
//!     Left,
//!     Right,
//!     Incr,
//!     Decr,
//!     Read,
//!     Write,
//!     Loop(Vec<Self>),
//! }
//!
//! parse_it! {
//!     #[parser]
//!     mod parse {
//!         use super::Instr;
//!
//!         type Lexer = parse_it::CharLexer;
//!
//!         pub Brainfuck -> Vec<Instr> {
//!             Primitive* => self,
//!         }
//!
//!         Primitive -> Instr {
//!             '<' => Instr::Left,
//!             '>' => Instr::Right,
//!             '+' => Instr::Incr,
//!             '-' => Instr::Decr,
//!             ',' => Instr::Read,
//!             '.' => Instr::Write,
//!             '[' Primitive+ ']' => Instr::Loop(self)
//!         }
//!     }
//! }
//!
//! fn main() {
//!     let parser = parse::Brainfuck::default();
//!     let src = "--[>--->->->++>-<<<<<-------]>--.>---------.>--..+++.>----.>+++++++++.<<.+++.------.<-.>>+";
//!     let instrs = parser.parse(src).unwrap();
//!     println!("{:?}", instrs);
//! }
//! ```
#![warn(missing_docs)]
#![allow(clippy::needless_doctest_main)]

pub mod lexer;
pub mod memo;
pub mod parser;

pub use parse_it_macros::parse_it;

pub use crate::{
    lexer::{CharLexer, Cursor, LexerState},
    memo::{left_rec, memorize, Memo},
    parser::{Error, ParserState},
};

/// A lexer.
pub trait LexIt {
    /// The token type.
    type Token<'a>;

    /// Create a new lexer instance.
    fn new() -> Self;

    /// Get the next token from the lexer.
    fn next<'a>(&self, lexbuf: &mut LexerState<'a>) -> Option<Self::Token<'a>>;
}

/// A parser.
pub trait ParseIt {
    /// The lexer type.
    type Lexer: LexIt + Clone;
    /// The parser output type.
    type Output;

    /// Parse from a [`ParserState`].
    fn parse_stream<'a>(
        &self,
        state: &mut ParserState<'a, Self::Lexer>,
    ) -> Result<Self::Output, Error>;

    /// Parse from a string.
    fn parse(&self, input: &str) -> Result<Self::Output, Error> {
        let mut state = ParserState::new(input);
        self.parse_stream(&mut state)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! identity {
    ($expr:expr) => {
        $expr
    };
}

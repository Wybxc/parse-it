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
// #![warn(missing_docs)]
#![allow(clippy::needless_doctest_main)]

pub mod lexer;
pub mod memo;
pub mod parser;

pub use parse_it_macros::parse_it;

pub use crate::{
    lexer::{AsLiteral, CharLexer, Cursor, LexerState},
    memo::{left_rec, memorize, Memo},
    parser::{Error, ParserState},
};

/// A lexer for the parser.
pub trait LexIt {
    type Token<'a>;

    fn new() -> Self;

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
    ) -> Result<Self::Output, Error>
    where
        <Self::Lexer as LexIt>::Token<'a>: AsLiteral;

    /// Parse from a string.
    fn parse<'a>(&self, input: &'a str) -> Result<Self::Output, Error>
    where
        <Self::Lexer as LexIt>::Token<'a>: AsLiteral,
    {
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

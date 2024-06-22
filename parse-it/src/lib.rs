#![warn(missing_docs)]
#![doc=include_str!("../../README.md")]

pub mod lexer;
pub mod memo;
pub mod parser;

pub use parse_it_macros::parse_it;

pub use crate::lexer::{CharLexer, Lexer};
pub use crate::memo::{left_rec, memorize, Memo};
pub use crate::parser::{Error, ParserState};

/// A parser.
pub trait ParseIt {
    /// The lexer type.
    type Lexer<'a>: Lexer<'a>;
    /// The parser output type.
    type Output;

    /// Parse from a [`ParserState`].
    fn parse_stream(&self, state: &ParserState<Self::Lexer<'_>>) -> Result<Self::Output, Error>;

    /// Parse from a string.
    fn parse(&self, input: &str) -> Result<Self::Output, Error> {
        let state = ParserState::new(Self::Lexer::new(input));
        self.parse_stream(&state)
    }
}

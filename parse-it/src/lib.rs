#![doc=include_str!("../../README.md")]

pub mod lexer;
pub mod memo;
pub mod parser;

pub use parse_it_macros::parse_it;

pub use crate::lexer::{CharLexer, Lexer};
pub use crate::memo::{left_rec, memorize, Memo};
pub use crate::parser::{Error, ParserState};

pub trait ParseIt {
    type Lexer<'a>: Lexer<'a>;
    type Output;

    fn parse_stream(&self, state: &ParserState<Self::Lexer<'_>>) -> Result<Self::Output, Error>;

    fn parse(&self, input: &str) -> Result<Self::Output, Error> {
        let state = ParserState::new(Self::Lexer::new(input));
        self.parse_stream(&state)
    }
}

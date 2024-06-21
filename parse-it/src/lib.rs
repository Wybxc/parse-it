#![doc=include_str!("../../README.md")]

mod arena;
pub mod combinator;

pub mod parser;
pub mod primitive;
pub mod recursive;

pub use parse_it_macros::parse_it;

use crate::parser::Token;
pub use crate::parser::{left_rec, memorize, Error, Memo, ParserState};

pub trait ParseIt {
    type Output;

    fn parse_stream(&self, state: &ParserState<char>) -> Result<Self::Output, Error>;

    fn parse(&self, input: &str) -> Result<Self::Output, Error> {
        let state = ParserState::new(
            input
                .char_indices()
                .map(|(i, c)| Token {
                    kind: c,
                    span: (i, i + c.len_utf8()),
                })
                .collect(),
        );
        self.parse_stream(&state)
    }
}

#![doc=include_str!("../../README.md")]

pub mod memo;
pub mod parser;

pub use parse_it_macros::parse_it;

pub use crate::memo::{left_rec, memorize, Memo};
use crate::parser::Token;
pub use crate::parser::{Error, ParserState};

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

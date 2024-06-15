use chumsky::error::Simple;
pub use parse_it_macros::parse_it;

pub type Parser<'src, T> = chumsky::BoxedParser<'src, char, T, Simple<char>>;

pub trait Grammar {
    type Parser;
}

#[doc(hidden)]
pub mod __internal {
    use chumsky::prelude::*;

    pub use chumsky::primitive::choice;

    pub fn define_parser<T, P>(parser: P) -> P
    where
        P: Parser<char, T, Error = Simple<char>>,
    {
        parser
    }

    pub fn declare_recursive<'a, O>() -> Recursive<'a, char, O, Simple<char>> {
        Recursive::declare()
    }
}

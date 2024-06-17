#![doc=include_str!("../../README.md")]

use chumsky::{error::Simple, Parser as _};
pub use parse_it_macros::parse_it;

pub struct Parser<'src, T> {
    parser: chumsky::BoxedParser<'src, char, T, Simple<char>>,
}

impl<'src, T> Parser<'src, T> {
    pub fn parse(&self, src: &'src str) -> Result<T, Vec<Simple<char>>> {
        self.parser
            .clone()
            .then_ignore(chumsky::primitive::end())
            .parse(src)
    }
}

impl<'src, T> From<chumsky::BoxedParser<'src, char, T, Simple<char>>> for Parser<'src, T> {
    fn from(parser: chumsky::BoxedParser<'src, char, T, Simple<char>>) -> Self {
        Self { parser }
    }
}

#[doc(hidden)]
pub mod __internal {
    use chumsky::prelude::*;

    pub use chumsky::primitive::choice;

    #[inline(always)]
    pub fn declare_recursive<'a, O>() -> Recursive<'a, char, O, Simple<char>> {
        Recursive::declare()
    }

    #[inline(always)]
    pub fn define_recursive<'a, O>(
        decl: &mut Recursive<'a, char, O, Simple<char>>,
        value: impl Parser<char, O, Error = Simple<char>> + 'a,
    ) {
        decl.define(value)
    }

    #[inline(always)]
    pub fn just_parser(c: char) -> chumsky::primitive::Just<char, char, Simple<char>> {
        just(c)
    }

    #[inline(always)]
    pub fn map_parser<U, T>(
        parser: impl Parser<char, T, Error = Simple<char>> + Clone,
        f: impl Fn(T) -> U + Clone,
    ) -> chumsky::combinator::Map<
        impl Parser<char, T, Error = Simple<char>> + Clone,
        impl Fn(T) -> U + Clone,
        T,
    > {
        parser.map(f)
    }

    #[inline(always)]
    pub fn then_parser<T, U>(
        parser1: impl Parser<char, T, Error = Simple<char>> + Clone,
        parser2: impl Parser<char, U, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::Then<
        impl Parser<char, T, Error = Simple<char>> + Clone,
        impl Parser<char, U, Error = Simple<char>> + Clone,
    > {
        parser1.then(parser2)
    }

    #[inline(always)]
    pub fn then_ignore_parser<T, U>(
        parser1: impl Parser<char, T, Error = Simple<char>> + Clone,
        parser2: impl Parser<char, U, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::ThenIgnore<
        impl Parser<char, T, Error = Simple<char>> + Clone,
        impl Parser<char, U, Error = Simple<char>> + Clone,
        T,
        U,
    > {
        parser1.then_ignore(parser2)
    }

    #[inline(always)]
    pub fn ignore_then_parser<T, U>(
        parser1: impl Parser<char, T, Error = Simple<char>> + Clone,
        parser2: impl Parser<char, U, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::IgnoreThen<
        impl Parser<char, T, Error = Simple<char>> + Clone,
        impl Parser<char, U, Error = Simple<char>> + Clone,
        T,
        U,
    > {
        parser1.ignore_then(parser2)
    }

    pub use chumsky::primitive::choice as choice_parser;

    #[inline(always)]
    pub fn repeat_parser<T>(
        parser: impl Parser<char, T, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::Repeated<impl Parser<char, T, Error = Simple<char>> + Clone> {
        parser.repeated()
    }

    #[inline(always)]
    pub fn repeat1_parser<T>(
        parser: impl Parser<char, T, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::Repeated<impl Parser<char, T, Error = Simple<char>> + Clone> {
        parser.repeated().at_least(1)
    }

    #[inline(always)]
    pub fn or_not_parser<T>(
        parser: impl Parser<char, T, Error = Simple<char>> + Clone,
    ) -> chumsky::combinator::OrNot<impl Parser<char, T, Error = Simple<char>> + Clone> {
        parser.or_not()
    }

    pub fn into_parser<'src, T>(
        parser: impl Parser<char, T, Error = Simple<char>> + 'src,
    ) -> super::Parser<'src, T> {
        parser.boxed().into()
    }
}

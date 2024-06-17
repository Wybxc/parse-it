#![doc=include_str!("../../README.md")]

mod arena;
pub mod combinator;
pub mod parser;
pub mod primitive;
pub mod recursive;

use std::rc::Rc;

// use chumsky::{error::Simple, Parser as _};
pub use parse_it_macros::parse_it;

use crate::{
    arena::Arena,
    parser::{Error, ParserState, Token},
};

pub struct Parser<T> {
    _arena: Rc<Arena>,
    parser: Box<dyn parser::Parser<char, Output = T>>,
}

impl<T> Parser<T> {
    pub fn parse(&self, src: &str) -> Result<T, Error> {
        let state = ParserState::new(
            src.char_indices()
                .map(|(i, c)| Token {
                    kind: c,
                    span: (i, i + c.len_utf8()),
                })
                .collect(),
        );
        self.parser.parse(&state)
    }
}

#[doc(hidden)]
pub mod __internal {

    use crate::{arena::Arena, combinator::*, parser::Parser, primitive::*, recursive::*};

    #[inline(always)]
    pub fn new_arena() -> std::rc::Rc<Arena> {
        Arena::new()
    }

    #[inline(always)]
    pub fn declare_recursive<K, T>(arena: std::rc::Rc<Arena>) -> Recursive<K, T> {
        Recursive::declare(arena)
    }

    #[inline(always)]
    pub fn define_recursive<K: 'static, T: 'static>(
        decl: Recursive<K, T>,
        value: impl Parser<K, Output = T> + 'static,
    ) -> Recursive<K, T> {
        decl.define(value)
    }

    #[inline(always)]
    pub fn just_parser(c: char) -> Just<char> {
        Just { value: c }
    }

    #[inline(always)]
    pub fn map_parser<U, T>(
        parser: impl Parser<char, Output = T> + Clone,
        f: impl Fn(T) -> U + Clone,
    ) -> Map<impl Parser<char, Output = T> + Clone, impl Fn(T) -> U + Clone> {
        Map { parser, f }
    }

    #[inline(always)]
    pub fn then_parser<T, U>(
        parser1: impl Parser<char, Output = T> + Clone,
        parser2: impl Parser<char, Output = U> + Clone,
    ) -> Then<impl Parser<char, Output = T> + Clone, impl Parser<char, Output = U> + Clone> {
        Then { parser1, parser2 }
    }

    #[inline(always)]
    pub fn then_ignore_parser<T, U>(
        parser1: impl Parser<char, Output = T> + Clone,
        parser2: impl Parser<char, Output = U> + Clone,
    ) -> ThenIgnore<impl Parser<char, Output = T> + Clone, impl Parser<char, Output = U> + Clone>
    {
        ThenIgnore { parser1, parser2 }
    }

    #[inline(always)]
    pub fn ignore_then_parser<T, U>(
        parser1: impl Parser<char, Output = T> + Clone,
        parser2: impl Parser<char, Output = U> + Clone,
    ) -> IgnoreThen<impl Parser<char, Output = T> + Clone, impl Parser<char, Output = U> + Clone>
    {
        IgnoreThen { parser1, parser2 }
    }

    #[inline(always)]
    pub fn choice_parser<P>(parsers: P) -> Choice<P> {
        Choice { parsers }
    }

    #[inline(always)]
    pub fn repeat_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> Repeat<impl Parser<char, Output = T> + Clone> {
        Repeat {
            parser,
            at_least: 0,
        }
    }

    #[inline(always)]
    pub fn repeat1_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> Repeat<impl Parser<char, Output = T> + Clone> {
        Repeat {
            parser,
            at_least: 1,
        }
    }

    #[inline(always)]
    pub fn or_not_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> OrNot<impl Parser<char, Output = T> + Clone> {
        OrNot { parser }
    }

    #[inline(always)]
    pub fn into_parser<T>(
        parser: impl Parser<char, Output = T> + Clone + 'static,
        arena: std::rc::Rc<Arena>,
    ) -> super::Parser<T> {
        super::Parser {
            parser: Box::new(parser),
            _arena: arena.clone(),
        }
    }
}

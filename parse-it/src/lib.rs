#![doc=include_str!("../../README.md")]

mod arena;
pub mod combinator;

pub mod parser;
pub mod primitive;
pub mod recursive;

pub use parse_it_macros::parse_it;

use crate::parser::{Error, ParserState, Token};

pub struct Parser<T> {
    _arena: std::rc::Rc<dyn std::any::Any>,
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

    use crate::arena::Arena;
    use crate::combinator::*;
    use crate::parser::Parser;
    use crate::primitive::*;
    use crate::recursive::*;

    #[inline(always)]
    pub fn new_arena<const N: usize>() -> Arena<N> {
        Arena::new()
    }

    #[inline(always)]
    pub fn declare_recursive<const N: usize, K, T>(arena: &Arena<N>) -> Recursive<N, K, T> {
        Recursive::declare(arena)
    }

    #[inline(always)]
    pub fn define_recursive<const N: usize, K: 'static, T: 'static>(
        decl: Recursive<N, K, T>,
        value: impl Parser<K, Output = T> + 'static,
    ) -> Recursive<N, K, T> {
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
    pub fn choice_parser<P>(parsers: P) -> Choice<P> {
        Choice { parsers }
    }

    #[inline(always)]
    pub fn memorize_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> Memorize<T, impl Parser<char, Output = T> + Clone> {
        Memorize::new(parser)
    }

    #[inline(always)]
    pub fn left_rec_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> LeftRec<T, impl Parser<char, Output = T> + Clone> {
        LeftRec::new(parser)
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
    pub fn look_ahead_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> LookAhead<impl Parser<char, Output = T> + Clone> {
        LookAhead { parser }
    }

    #[inline(always)]
    pub fn look_ahead_not_parser<T>(
        parser: impl Parser<char, Output = T> + Clone,
    ) -> LookAheadNot<impl Parser<char, Output = T> + Clone> {
        LookAheadNot { parser }
    }

    #[inline(always)]
    pub fn into_parser<const N: usize, T>(
        parser: impl Parser<char, Output = T> + Clone + 'static,
        arena: &Arena<N>,
    ) -> super::Parser<T> {
        super::Parser {
            parser: Box::new(parser),
            _arena: arena.inner(),
        }
    }
}

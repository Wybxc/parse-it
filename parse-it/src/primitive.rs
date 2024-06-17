use crate::parser::{Error, Parser, ParserState};

#[derive(Clone, Copy)]
pub struct Just<K> {
    pub value: K,
}

impl<K: Copy + Eq> Parser<K> for Just<K> {
    type Output = K;

    fn parse(&self, state: &ParserState<K>) -> Result<K, Error> {
        if state.peek(|token| token.kind == self.value) {
            Ok(state.next().unwrap().kind)
        } else {
            Err(Error { span: state.span() })
        }
    }
}

#[derive(Clone, Copy)]
pub struct Choice<P> {
    pub parsers: P,
}

macro_rules! impl_choice {
    ($(($parser:ident, $idx:tt)),*) => {
        impl<K, T, $($parser),*> Parser<K> for Choice<($($parser,)*)>
        where
            K: Copy,
            $($parser: Parser<K, Output = T>,)*
        {
            type Output = T;

            fn parse(&self, state: &ParserState<K>) -> Result<T, Error> {
                let fork = state.fork();
                $(
                    match self.parsers.$idx.parse(&fork) {
                        Ok(value) => {
                            state.advance_to(&fork);
                            return Ok(value);
                        }
                        Err(_) => {}
                    }
                )*
                Err(Error { span: state.span() })
            }
        }
    };
}

#[rustfmt::skip]
mod choice {
    use super::*;
    impl_choice!((P0, 0), (P1, 1));
    impl_choice!((P0, 0), (P1, 1), (P2, 2));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10), (P11, 11));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10), (P11, 11), (P12, 12));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10), (P11, 11), (P12, 12), (P13, 13));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10), (P11, 11), (P12, 12), (P13, 13), (P14, 14));
    impl_choice!((P0, 0), (P1, 1), (P2, 2), (P3, 3), (P4, 4), (P5, 5), (P6, 6), (P7, 7), (P8, 8), (P9, 9), (P10, 10), (P11, 11), (P12, 12), (P13, 13), (P14, 14), (P15, 15));
}

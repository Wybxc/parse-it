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
            Err(Error::new(state.span()))
        }
    }
}

#[derive(Clone, Copy)]
pub struct Choice<P> {
    pub parsers: P,
}

#[typle::typle(Tuple for 2..=32)]
impl<K, T, P: Tuple> Parser<K> for Choice<P>
where
    K: Copy,
    P<_>: Parser<K, Output = T>,
{
    type Output = T;

    fn parse(&self, state: &ParserState<K>) -> Result<T, Error> {
        for typle_index!(i) in 0..P::LEN {
            let fork = state.fork();
            if let Ok(value) = self.parsers[[i]].parse(&fork) {
                state.advance_to(&fork);
                return Ok(value);
            }
        }
        Err(Error::new(state.span()))
    }
}

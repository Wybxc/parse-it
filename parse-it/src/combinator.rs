use crate::parser::{Error, Parser, ParserState};

#[derive(Clone, Copy)]
pub struct Map<P, F> {
    pub parser: P,
    pub f: F,
}

impl<K, T, U, P, F> Parser<K> for Map<P, F>
where
    P: Parser<K, Output = T>,
    F: Fn(T) -> U,
{
    type Output = U;

    fn parse(&self, state: &ParserState<K>) -> Result<U, Error> {
        self.parser.parse(state).map(&self.f)
    }
}

#[derive(Clone, Copy)]
pub struct Then<P1, P2> {
    pub parser1: P1,
    pub parser2: P2,
}

impl<K, T, U, P1, P2> Parser<K> for Then<P1, P2>
where
    P1: Parser<K, Output = T>,
    P2: Parser<K, Output = U>,
{
    type Output = (T, U);

    fn parse(&self, state: &ParserState<K>) -> Result<(T, U), Error> {
        let value1 = self.parser1.parse(state)?;
        let value2 = self.parser2.parse(state)?;
        Ok((value1, value2))
    }
}

#[derive(Clone, Copy)]
pub struct ThenIgnore<P1, P2> {
    pub parser1: P1,
    pub parser2: P2,
}

impl<K, T, U, P1, P2> Parser<K> for ThenIgnore<P1, P2>
where
    P1: Parser<K, Output = T>,
    P2: Parser<K, Output = U>,
{
    type Output = T;

    fn parse(&self, state: &ParserState<K>) -> Result<T, Error> {
        let value1 = self.parser1.parse(state)?;
        self.parser2.parse(state)?;
        Ok(value1)
    }
}

#[derive(Clone, Copy)]
pub struct IgnoreThen<P1, P2> {
    pub parser1: P1,
    pub parser2: P2,
}

impl<K, T, U, P1, P2> Parser<K> for IgnoreThen<P1, P2>
where
    P1: Parser<K, Output = T>,
    P2: Parser<K, Output = U>,
{
    type Output = U;

    fn parse(&self, state: &ParserState<K>) -> Result<U, Error> {
        self.parser1.parse(state)?;
        self.parser2.parse(state)
    }
}

#[derive(Clone, Copy)]
pub struct Repeat<P> {
    pub parser: P,
    pub at_least: usize,
}

impl<K, T, P> Parser<K> for Repeat<P>
where
    K: Copy,
    P: Parser<K, Output = T>,
{
    type Output = Vec<T>;

    fn parse(&self, state: &ParserState<K>) -> Result<Vec<T>, Error> {
        let mut values = Vec::new();
        let fork = state.fork();
        let mut count = 0;
        while let Ok(value) = self.parser.parse(&fork) {
            values.push(value);
            count += 1;
        }
        if count < self.at_least {
            return Err(Error { span: state.span() });
        }
        state.advance_to(&fork);
        Ok(values)
    }
}

#[derive(Clone, Copy)]
pub struct OrNot<P> {
    pub parser: P,
}

impl<K, T, P> Parser<K> for OrNot<P>
where
    K: Copy,
    P: Parser<K, Output = T>,
{
    type Output = Option<T>;

    fn parse(&self, state: &ParserState<K>) -> Result<Option<T>, Error> {
        let fork = state.fork();
        match self.parser.parse(&fork) {
            Ok(value) => {
                state.advance_to(&fork);
                Ok(Some(value))
            }
            Err(_) => Ok(None),
        }
    }
}

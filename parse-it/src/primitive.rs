use std::cell::RefCell;
use std::rc::Rc;

use ahash::HashMap;

use crate::parser::{Error, Parser, ParserState};

#[derive(Clone, Copy)]
pub struct Just<K> {
    pub value: K,
}

impl<K: Copy + Eq> Parser<K> for Just<K> {
    type Output = K;

    #[inline(always)]
    fn parse(&self, state: &ParserState<K>) -> Result<K, Error> {
        match state.next() {
            Some(token) if token.kind == self.value => Ok(self.value),
            _ => Err(Error::new(state.span())),
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

    #[inline(always)]
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

type Memo<T> = Rc<RefCell<HashMap<usize, T>>>;

/// Prackrat memorization parser.
#[derive(Clone)]
pub struct Memorize<T, P> {
    memo: Memo<(T, usize)>,
    parser: P,
}

impl<T, P> Memorize<T, P> {
    pub fn new(parser: P) -> Self {
        Self {
            memo: Default::default(),
            parser,
        }
    }
}

impl<K, T, P> Parser<K> for Memorize<T, P>
where
    K: Copy,
    T: Clone,
    P: Parser<K, Output = T>,
{
    type Output = T;

    #[inline(always)]
    fn parse(&self, state: &ParserState<K>) -> Result<Self::Output, Error> {
        let pos = state.pos();
        if let Some((value, end)) = self.memo.borrow().get(&pos) {
            state.advance_to_pos(*end);
            return Ok(value.clone());
        }

        let value = self.parser.parse(state)?;
        let end = state.pos();
        self.memo.borrow_mut().insert(pos, (value.clone(), end));

        Ok(value)
    }
}

/// Left recursive memoization parser.
///
/// ref: https://medium.com/@gvanrossum_83706/left-recursive-peg-grammars-65dab3c580e1
#[derive(Clone)]
pub struct LeftRec<T, P> {
    memo: Memo<(Option<T>, usize)>,
    parser: P,
}

impl<T, P> LeftRec<T, P> {
    pub fn new(parser: P) -> Self {
        Self {
            memo: Default::default(),
            parser,
        }
    }
}

impl<K, T, P> Parser<K> for LeftRec<T, P>
where
    K: Copy,
    T: Clone,
    P: Parser<K, Output = T>,
{
    type Output = T;

    #[inline(always)]
    fn parse(&self, state: &ParserState<K>) -> Result<Self::Output, Error> {
        let pos = state.pos();
        if let Some((value, end)) = self.memo.borrow().get(&pos) {
            state.advance_to_pos(*end);
            if let Some(value) = value {
                return Ok(value.clone());
            } else {
                return Err(Error::new(state.span()));
            }
        }

        self.memo.borrow_mut().insert(pos, (None, pos));
        let mut last = pos;
        let end = loop {
            let fork = state.fork();
            let value = self.parser.parse(&fork)?;
            let end = fork.pos();
            if end <= last {
                break end;
            }
            last = end;
            self.memo.borrow_mut().insert(pos, (Some(value), end));
        };
        state.advance_to_pos(end);
        self.memo.borrow()[&pos]
            .0
            .clone()
            .ok_or_else(|| Error::new(state.span()))
    }
}

use crate::{
    arena::{Arena, Slot},
    parser::{Error, Parser, ParserState},
};

#[derive(Clone)]
pub struct Recursive<const N: usize, K, T> {
    inner: Slot<N, Box<dyn Parser<K, Output = T>>>,
}

impl<const N: usize, K, T> Recursive<N, K, T> {
    pub fn declare(arena: &Arena<N>) -> Self {
        Recursive {
            inner: arena.alloc(),
        }
    }

    pub fn define(self, parser: impl Parser<K, Output = T> + 'static) -> Self
    where
        K: 'static,
        T: 'static,
    {
        self.inner.store(Box::new(parser));

        Recursive { inner: self.inner }
    }
}

impl<const N: usize, K, T> Parser<K> for Recursive<N, K, T>
where
    K: 'static,
    T: 'static,
{
    type Output = T;

    #[inline(always)]
    fn parse(&self, state: &ParserState<K>) -> Result<Self::Output, Error> {
        self.inner.with(|parser| parser.parse(state))
    }
}

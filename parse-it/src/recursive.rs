use std::rc::Rc;

use crate::{
    arena::{Arena, Slot},
    parser::{Error, Parser, ParserState},
};

#[derive(Clone)]
pub struct Recursive<K, T> {
    inner: Slot<Box<dyn Parser<K, Output = T>>>,
}

impl<K, T> Recursive<K, T> {
    pub fn declare(arena: Rc<Arena>) -> Self {
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

impl<K, T> Parser<K> for Recursive<K, T>
where
    K: 'static,
    T: 'static,
{
    type Output = T;

    fn parse(&self, state: &ParserState<K>) -> Result<Self::Output, Error> {
        self.inner.get().parse(state)
    }
}

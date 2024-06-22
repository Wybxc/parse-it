use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;

use rustc_hash::FxHashMap;

use crate::lexer::Lexer;
use crate::{Error, ParserState};

pub struct Memo<P: Clone + Eq + Hash, T: Clone> {
    map: RefCell<FxHashMap<P, (T, P)>>,
}

impl<P: Clone + Eq + Hash, T: Clone> Default for Memo<P, T> {
    fn default() -> Self {
        Self {
            map: RefCell::new(FxHashMap::default()),
        }
    }
}

impl<P: Clone + Eq + Hash + Debug, T: Clone + Debug> Debug for Memo<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.borrow().fmt(f)
    }
}

impl<P: Clone + Eq + Hash, T: Clone> Memo<P, T> {
    pub fn get(&self, pos: &P) -> Option<(T, P)> {
        self.map.borrow().get(pos).cloned()
    }

    pub fn insert(&self, pos: P, value: (T, P)) {
        self.map.borrow_mut().insert(pos, value);
    }
}

#[inline]
pub fn memorize<'a, L: Lexer<'a>, T: Clone>(
    state: &ParserState<L>,
    memo: &Memo<L::Position, T>,
    parser: impl FnOnce(&ParserState<L>) -> Result<T, Error>,
) -> Result<T, Error> {
    let pos = state.pos();
    if let Some((value, end)) = memo.get(&pos) {
        state.advance_to_pos(end);
        Ok(value.clone())
    } else {
        let value = parser(state)?;
        let end = state.pos();
        memo.insert(pos, (value.clone(), end));
        Ok(value)
    }
}

#[inline]
pub fn left_rec<'a, L: Lexer<'a>, T: Clone>(
    state: &ParserState<L>,
    memo: &Memo<L::Position, Option<T>>,
    mut parser: impl FnMut(&ParserState<L>) -> Result<T, Error>,
) -> Result<T, Error> {
    let pos = state.pos();
    if let Some((value, end)) = memo.get(&pos) {
        state.advance_to_pos(end);
        if let Some(value) = value {
            Ok(value.clone())
        } else {
            Err(state.error())
        }
    } else {
        memo.insert(pos, (None, pos));
        let mut last = (None, pos);
        loop {
            let fork = state.fork();
            let Ok(value) = parser(&fork) else { break };
            let end = fork.pos();
            if end <= last.1 {
                break;
            }
            last = (Some(value), end);
            memo.insert(pos, last.clone());
        }
        state.advance_to_pos(last.1);
        last.0.ok_or_else(|| state.error())
    }
}

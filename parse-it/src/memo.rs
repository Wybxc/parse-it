use std::cell::RefCell;
use std::fmt::Debug;

use rustc_hash::FxHashMap;

use crate::parser::Position;
use crate::{Error, ParserState};

pub struct Memo<T: Clone> {
    map: RefCell<FxHashMap<Position, (T, Position)>>,
}

impl<T: Clone> Default for Memo<T> {
    fn default() -> Self {
        Self {
            map: RefCell::new(FxHashMap::default()),
        }
    }
}

impl<T: Clone + Debug> Debug for Memo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.borrow().fmt(f)
    }
}

impl<T: Clone> Memo<T> {
    pub fn get(&self, pos: &Position) -> Option<(T, Position)> {
        self.map.borrow().get(pos).cloned()
    }

    pub fn insert(&self, pos: Position, value: (T, Position)) {
        self.map.borrow_mut().insert(pos, value);
    }
}

#[inline]
pub fn memorize<K: Copy, T: Clone>(
    state: &ParserState<K>,
    memo: &Memo<T>,
    parser: impl FnOnce(&ParserState<K>) -> Result<T, Error>,
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
pub fn left_rec<K: Copy, T: Clone>(
    state: &ParserState<K>,
    memo: &Memo<Option<T>>,
    mut parser: impl FnMut(&ParserState<K>) -> Result<T, Error>,
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

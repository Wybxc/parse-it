use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustc_hash::FxHashMap as HashMap;

#[derive(Clone, Copy)]
pub struct Token<K> {
    pub kind: K,
    /// `[start, end)` of the token in the source.
    pub span: (usize, usize),
}

pub trait Parser<K> {
    type Output;

    fn parse(&self, state: &ParserState<K>) -> Result<Self::Output, Error>;
}

#[derive(Debug)]
pub struct Error {
    pub span: (usize, usize),
    pub backtrace: std::backtrace::Backtrace,
}

impl Error {
    pub fn new(span: (usize, usize)) -> Self {
        Self {
            span,
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

/// A opaque wrapper around a position in a sequence of tokens.
///
/// The position means the index of token in the sequence.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position(usize);

impl Position {
    fn incr(self) -> Self {
        Self(self.0 + 1)
    }
}

pub struct ParserState<K> {
    pos: Cell<Position>,
    items: Rc<Vec<Token<K>>>,
}

impl<K: Copy> ParserState<K> {
    pub fn new(items: Vec<Token<K>>) -> Self {
        Self {
            pos: Cell::new(Position(0)),
            items: Rc::new(items),
        }
    }

    pub fn pos(&self) -> Position {
        self.pos.get()
    }

    pub fn span(&self) -> (usize, usize) {
        if self.is_empty() {
            (self.items.len(), self.items.len())
        } else {
            self.items[self.pos.get().0].span
        }
    }

    pub fn next(&self) -> Option<Token<K>> {
        let pos = self.pos.get();
        if pos.0 < self.items.len() {
            self.pos.set(pos.incr());
            Some(self.items[pos.0])
        } else {
            None
        }
    }

    pub fn parse(&self, token: K) -> Result<K, Error>
    where
        K: Eq,
    {
        match self.next() {
            Some(Token { kind, .. }) if kind == token => Ok(kind),
            _ => Err(self.error()),
        }
    }

    pub fn error(&self) -> Error {
        Error::new(self.span())
    }

    pub fn is_empty(&self) -> bool {
        self.pos.get().0 >= self.items.len()
    }

    /// Advance the state to the given state.
    ///
    /// # Panics
    /// Panics if the given state is before the current state.
    pub fn advance_to(&self, other: &Self) {
        self.advance_to_pos(other.pos.get())
    }

    /// Advance the state to the given position.
    ///
    /// # Panics
    /// Panics if the given position is before the current position.
    pub fn advance_to_pos(&self, pos: Position) {
        assert!(pos >= self.pos.get() && pos.0 <= self.items.len());
        self.pos.set(pos)
    }

    pub fn fork(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            items: self.items.clone(),
        }
    }
}

pub struct Memo<T: Clone> {
    map: RefCell<HashMap<Position, (T, Position)>>,
}

impl<T: Clone> Default for Memo<T> {
    fn default() -> Self {
        Self {
            map: RefCell::new(HashMap::default()),
        }
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
        if let Some(value) = value {
            state.advance_to_pos(end);
            Ok(value.clone())
        } else {
            Err(state.error())
        }
    } else {
        memo.insert(pos, (None, pos));
        let mut last = (None, pos);
        loop {
            let fork = state.fork();
            let value = parser(&fork)?;
            let end = fork.pos();
            if end <= last.1 {
                state.advance_to_pos(end);
                break last.0.ok_or_else(|| state.error());
            }
            last = (Some(value), end);
            memo.insert(pos, last.clone());
        }
    }
}

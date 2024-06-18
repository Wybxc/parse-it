use std::cell::Cell;
use std::rc::Rc;

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

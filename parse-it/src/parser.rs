use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

use crate::lexer::Lexer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Error {
    pub span: Span,
}

impl Error {
    pub fn new(span: Span) -> Self {
        Self { span }
    }
}

pub struct ParserState<L> {
    span: Cell<Span>,
    lexer: L,
    stack: Rc<RefCell<Vec<(&'static str, usize)>>>,
}

impl<'a, L: Lexer<'a>> ParserState<L> {
    pub fn new(lexer: L) -> Self {
        Self {
            span: Cell::new(Span { start: 0, end: 0 }),
            lexer,
            stack: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn pos(&self) -> L::Position {
        self.lexer.pos()
    }

    pub fn next(&self) -> Option<L::Token> {
        match self.lexer.next() {
            (Some(token), advance) => {
                let Span { end, .. } = self.span.get();
                self.span.set(Span {
                    start: end,
                    end: end + advance,
                });
                Some(token)
            }
            _ => None,
        }
    }

    pub fn parse(&self, token: L::Token) -> Result<L::Token, Error> {
        match self.next() {
            Some(tt) if tt == token => Ok(tt),
            _ => Err(self.error()),
        }
    }

    pub fn error(&self) -> Error {
        Error::new(self.span.get())
    }

    pub fn is_empty(&self) -> bool {
        self.lexer.is_empty()
    }

    /// Advance the state to the given state.
    ///
    /// # Panics
    /// Panics if the given state is before the current state.
    pub fn advance_to(&self, other: &Self) {
        self.advance_to_pos(other.lexer.pos())
    }

    /// Advance the state to the given position.
    ///
    /// # Panics
    /// Panics if the given position is before the current position.
    pub fn advance_to_pos(&self, pos: L::Position) {
        assert!(pos >= self.lexer.pos(), "you cannot rewind");
        self.lexer.advance_to_pos(pos);
    }

    pub fn fork(&self) -> Self {
        Self {
            span: self.span.clone(),
            lexer: self.lexer.fork(),
            stack: self.stack.clone(),
        }
    }

    pub fn push(&self, name: &'static str) {
        self.stack.borrow_mut().push((name, self.span.get().end));
    }

    pub fn pop(&self) {
        self.stack.borrow_mut().pop();
    }

    pub fn debug(&self) -> String {
        format!("{:?}", self.stack.borrow())
    }
}

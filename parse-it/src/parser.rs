//! Basic definitions for working with the parser.
//!
//! If you're looking for a convenient way to parse data, you don't need to dive into
//! the details of the parser. The [`ParseIt::parse`] method abstracts away all the
//! complexity, making it easy to use.
//!
//! However, if you're interested in learning more about how the parser works under the
//! hood, you can refer to the [`ParserState`] documentation.
//!
//! [`ParseIt::parse`]: crate::ParseIt::parse

use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::lexer::{Lexer, Span};

/// An error that occurred during parsing.
#[derive(Debug)]
pub struct Error {
    /// The span in the source code where the error occurred.
    pub span: Span,
}

impl Error {
    /// Create a new error from the given span.
    pub fn new(span: Span) -> Self {
        Self { span }
    }
}

/// The inner state of a parser.
///
/// `ParserState` is a cursor over the lexer and keeps track of the current position
/// in the source code. It is used to drive the parsing process.
///
/// # Writing a Parser
///
/// A parser is a function `Fn(&ParserState) -> Result<T, Error>`, that takes a
/// `&ParserState` as input and returns the parsed result or an error.
///
/// The common use case is to call the [`parse`](ParserState::parse) method to
/// read a token from the lexer and advance the state by one token.
///
/// ```
/// # use parse_it::*;
/// fn parse_abc(state: &mut ParserState<CharLexer>) -> Result<char, Error> {
///     state.parse('a')?;
///     state.parse('b')?;
///     state.parse('c')?;
///     Ok('c')
/// }
///
/// let mut state = ParserState::new(CharLexer::new("abc"));
/// parse_abc(&mut state).unwrap();
/// assert!(state.is_empty());
/// ```
///
/// Please note that `ParserState` uses interior mutability to share its state
/// between parsers. This means that even if a parser takes a `&ParserState`,
/// the state can still be mutated.
///
/// # Speculative Parsing
///
/// `ParserState` allows you to create a fork of the current state via the
/// [`fork`](ParserState::fork) method, and join it back to the original state
/// later via the [`advance_to`](ParserState::advance_to) method. This is useful
/// for speculative parsing.
///
/// It's important to note that `ParserState` can only move forward and not
/// backward. When joining a fork back to the original state, it must be
/// ensured that the fork is at a position beyond or equal to the original
/// state.
///
/// ```
/// # use parse_it::*;
/// fn parse_option(
///     state: &mut ParserState<CharLexer>,
///     parser: impl Fn(&mut ParserState<CharLexer>) -> Result<char, Error>
/// ) -> Result<Option<char>, Error> {
///     let fork = &mut state.fork();
///     match parser(fork) {
///         Ok(c) => {
///             state.advance_to(fork);
///             Ok(Some(c))
///         }
///         Err(_) => Ok(None),
///     }
/// }
///
/// let mut state = ParserState::new(CharLexer::new("aaa"));
/// assert_eq!(parse_option(&mut state, |state| state.parse('a')).unwrap(), Some('a'));
/// assert_eq!(parse_option(&mut state, |state| state.parse('b')).unwrap(), None);
/// ```
pub struct ParserState<L> {
    lexer: L,
    stack: Rc<RefCell<Vec<(&'static str, usize)>>>,
}

impl<'a, L: Lexer<'a>> ParserState<L> {
    /// Create a new parser state from the given lexer.
    pub fn new(lexer: L) -> Self {
        Self {
            lexer,
            stack: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Get the current parsing position.
    pub fn cursor(&self) -> &L::Cursor {
        self.lexer.cursor()
    }

    /// Advance to the next token.
    fn next(&mut self) -> Option<L::Token> {
        self.lexer.next()
    }

    /// Consume the next token if it matches the given token.
    pub fn parse_with<T>(
        &mut self,
        matches: impl FnOnce(L::Token) -> Option<T>,
    ) -> Result<T, Error> {
        self.next()
            .and_then(matches)
            .ok_or_else(|| Error::new(self.lexer.span()))
    }

    /// Consume the next token if it matches the given token via [`PartialEq`].
    pub fn parse<T>(&mut self, terminal: T) -> Result<L::Token, Error>
    where
        L::Token: PartialEq<T>,
    {
        self.parse_with(|tt| tt.eq(&terminal).then_some(tt))
    }

    /// Report an error at the current position.
    pub fn error(&self) -> Error {
        Error::new(self.lexer.span())
    }

    /// Whether the parser is at the end of the input.
    pub fn is_empty(&self) -> bool {
        self.lexer.is_empty()
    }

    /// Advance the state to the given state.
    ///
    /// # Panics
    /// Panics if the given state is before the current state.
    pub fn advance_to(&mut self, other: &Self) {
        self.advance_to_cursor(other.lexer.cursor())
    }

    /// Advance the state to the given position.
    ///
    /// # Panics
    /// Panics if the given position is before the current position.
    pub fn advance_to_cursor(&mut self, cursor: &L::Cursor) {
        assert!(cursor >= self.lexer.cursor(), "you cannot rewind");
        self.lexer.advance_to_cursor(cursor);
    }

    /// Create a fork of the current state for speculative parsing.
    pub fn fork(&self) -> Self {
        Self {
            lexer: self.lexer.fork(),
            stack: self.stack.clone(),
        }
    }

    /// Push the given name onto the stack (for debugging purposes).
    pub fn push(&self, name: &'static str) {
        self.stack.borrow_mut().push((name, self.lexer.span().end));
    }

    /// Pop the last name from the stack (for debugging purposes).
    pub fn pop(&self) {
        self.stack.borrow_mut().pop();
    }

    /// Get the current stack (for debugging purposes).
    pub fn debug(&self) -> String {
        format!("{:?}", self.stack.borrow())
    }
}

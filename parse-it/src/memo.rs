//! Memoization and left recursion support.

use std::{cell::RefCell, fmt::Debug, hash::Hash};

use rustc_hash::FxHashMap;

use crate::{lexer::Cursor, Error, LexIt, ParserState};

/// Memorization for a parser.
///
/// It records the results of parsing a given position in the source code, including
/// the parsed value and the position to which the parser was advanced.
#[derive(Clone)]
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
    /// Get a memoized value.
    pub fn get(&self, pos: &P) -> Option<(T, P)> {
        self.map.borrow().get(pos).cloned()
    }

    /// Insert a memoized value.
    pub fn insert(&self, pos: P, value: (T, P)) {
        self.map.borrow_mut().insert(pos, value);
    }
}

/// The ["Packrat"] memoization for a parser.
///
/// It ensures that parsing the same position in the source code only occurs once,
/// by recording the results of parsing. The memoization is distinguished by the
/// position itself, so different parsing processes should have their own memos.
///
/// ["Packrat"]: https://en.wikipedia.org/wiki/Packrat_parser
#[inline]
pub fn memorize<L: LexIt + Clone, T: Clone>(
    state: &mut ParserState<L>,
    memo: &Memo<Cursor, T>,
    parser: impl FnOnce(&mut ParserState<L>) -> Result<T, Error>,
) -> Result<T, Error> {
    let pos = state.cursor();
    if let Some((value, end)) = memo.get(&pos) {
        state.advance_to_cursor(end);
        Ok(value.clone())
    } else {
        let value = parser(state)?;
        let end = state.cursor();
        memo.insert(pos, (value.clone(), end));
        Ok(value)
    }
}

/// Left recursion support.
///
/// Wrapping a parser in `left_rec` allows it to be left-recursive. This is
/// crucial for parsing left-recursive grammars, as recursive descent
/// parsers often fail to handle them.
///
/// The `left_rec` function solves this problem by employing memoization.
/// The algorithm used is based on this [blog post].
///
/// ```
/// # use parse_it::*;
/// fn parse(
///     state: &mut ParserState<CharLexer>,
///     memo: &Memo<Cursor, Option<String>>,
/// ) -> Result<String, Error> {
///     left_rec(state, memo, |state| {
///         let fork = &mut state.fork();
///         if let Ok(mut s) = parse(fork, memo) {
///             state.advance_to(fork);
///             s.push(state.parse_char('b')?);
///             Ok(s)
///         } else {
///             state.parse_char('a').map(|_| String::from("a"))
///         }
///     })
/// }
///
/// let mut state = ParserState::new("abbbb");
/// assert_eq!(parse(&mut state, &Memo::default()).unwrap(), "abbbb");
/// ```
///
/// [blog post]:https://medium.com/@gvanrossum_83706/left-recursive-peg-grammars-65dab3c580e1
#[inline]
pub fn left_rec<L: LexIt + Clone, T: Clone>(
    state: &mut ParserState<L>,
    memo: &Memo<Cursor, Option<T>>,
    mut parser: impl FnMut(&mut ParserState<L>) -> Result<T, Error>,
) -> Result<T, Error> {
    let pos = state.cursor();
    if let Some((value, end)) = memo.get(&pos) {
        state.advance_to_cursor(end);
        if let Some(value) = value {
            Ok(value.clone())
        } else {
            Err(state.error())
        }
    } else {
        memo.insert(pos, (None, pos));
        let mut last = (None, pos);
        loop {
            let mut fork = state.fork();
            let Ok(value) = parser(&mut fork) else { break };
            let end = fork.cursor();
            if end <= last.1 {
                break;
            }
            last = (Some(value), end);
            memo.insert(pos, last.clone());
        }
        state.advance_to_cursor(last.1);
        last.0.ok_or_else(|| state.error())
    }
}

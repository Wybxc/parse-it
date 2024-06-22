//! Memoization and left recursion support.

use std::cell::RefCell;
use std::fmt::Debug;
use std::hash::Hash;

use rustc_hash::FxHashMap;

use crate::lexer::Lexer;
use crate::{Error, ParserState};

/// Memorization for a parser.
///
/// It records the results of parsing a given position in the source code, including
/// the parsed value and the position to which the parser was advanced.
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
/// fn parse(
///     state: &ParserState<CharLexer>, 
///     memo: &Memo<usize, Option<String>>,
/// ) -> Result<String, Error> {
///     left_rec(state, memo, |state| {
///         let fork = state.fork();
///         if let Ok(mut s) = parse(fork) {
///             state.advance_to(&fork);
///             s.push(state.parse('b')?);
///             Ok(s)
///         } else {
///             state.parse('a').map(|_| String::from("a"))
///         }
///     })
/// }
/// 
/// let state = ParserState::new(CharLexer::new("abbbb"));
/// assert_eq!(parse(&state, &Memo::default()).unwrap(), "abbbb");
/// ```
/// 
/// [blog post]:https://medium.com/@gvanrossum_83706/left-recursive-peg-grammars-65dab3c580e1
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

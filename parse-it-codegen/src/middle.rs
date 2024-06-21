use hashlink::LinkedHashMap;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::hash::OrderedMap;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Value {
    pub id: u32,
    _non_send: std::marker::PhantomData<*const ()>,
}

impl Value {
    pub fn next() -> Self {
        thread_local! {
            static NEXT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        NEXT.with(|next| {
            let value = next.get();
            next.set(value + 1);
            Self {
                id: value,
                _non_send: std::marker::PhantomData,
            }
        })
    }
}

#[derive(Clone)]
pub enum Capture {
    Loud,
    Slient,
    Named(Box<syn::Pat>, Box<Capture>),
    Tuple(Box<Capture>, Box<Capture>),
}

impl Capture {
    pub fn is_loud(&self) -> bool {
        match self {
            Capture::Loud => true,
            Capture::Slient => false,
            Capture::Named(_, _) => true,
            Capture::Tuple(_, n) => n.is_loud(),
        }
    }

    pub fn to_anoymous(&self) -> Capture {
        if self.is_loud() {
            Capture::Loud
        } else {
            Capture::Slient
        }
    }

    pub fn unify(self, cap: &Capture) -> Result<Capture, TokenStream> {
        match (self, cap) {
            (Capture::Named(p1, c1), Capture::Named(p2, c2)) => {
                if &p1 == p2 {
                    if let Ok(c) = c1.unify(c2) {
                        Ok(Capture::Named(p1, Box::new(c)))
                    } else {
                        Ok(Capture::Named(p1, Box::new(Capture::Loud)))
                    }
                } else {
                    Err(quote_spanned! {
                        p1.span() => compile_error!("pattern mismatch")
                    })
                }
            }
            (Capture::Tuple(c1, c2), Capture::Tuple(c3, c4)) => {
                let c1 = c1.unify(c3)?;
                let c2 = c2.unify(c4)?;
                Ok(Capture::Tuple(Box::new(c1), Box::new(c2)))
            }
            (Capture::Loud, _) => Ok(Capture::Loud),
            (_, Capture::Loud) => Ok(Capture::Loud),
            (Capture::Slient, Capture::Slient) => Ok(Capture::Slient),
            _ => Err(quote! {
                compile_error!("capture mismatch")
            }),
        }
    }
}

pub struct Parsing {
    values: OrderedMap<Value, ParseOp>,
    pub capture: Capture,
}

impl Parsing {
    pub fn into_iter(self) -> impl Iterator<Item = (Value, ParseOp)> {
        self.values.into_iter()
    }

    fn from_op(op: ParseOp, capture: Capture) -> Self {
        let mut values = LinkedHashMap::default();
        values.insert(Value::next(), op);
        Self { values, capture }
    }

    pub fn result(&self) -> Value {
        self.values
            .back()
            .map(|(k, _)| *k)
            .expect("parser is empty")
    }

    fn push(mut self, op: ParseOp) -> Self {
        self.values.insert(Value::next(), op);
        self
    }

    pub fn just(c: char) -> Self {
        Self::from_op(ParseOp::Just(c), Capture::Slient)
    }

    pub fn call(name: syn::Ident, depends: impl Iterator<Item = ParserRef>) -> Self {
        Self::from_op(
            ParseOp::Call {
                parser: ParserRef::new(&name),
                depends: depends.collect(),
            },
            Capture::Loud,
        )
    }

    pub fn map(self, f: syn::Expr) -> Self {
        let parser = self.result();
        let capture = self.capture.clone();
        self.push(ParseOp::Map {
            parser,
            cap: capture,
            expr: f,
        })
    }

    pub fn then(mut self, next: Box<Parsing>) -> Self {
        let prev = self.result();
        let op = match (self.capture.is_loud(), next.capture.is_loud()) {
            (true, false) => ParseOp::ThenIgnore { prev, next },
            (false, true) => {
                self.capture = next.capture.clone();
                ParseOp::IgnoreThen { prev, next }
            }
            _ => {
                self.capture =
                    Capture::Tuple(Box::new(self.capture), Box::new(next.capture.clone()));
                ParseOp::Then { prev, next }
            }
        };
        self.push(op)
    }

    pub fn choice(
        mut self,
        rest: impl Iterator<Item = Result<Parsing, TokenStream>>,
    ) -> Result<Self, TokenStream> {
        let first = self.result();
        let mut parsers = vec![];

        for item in rest {
            let parser = item?;
            self.capture = self.capture.unify(&parser.capture)?;
            parsers.push(parser);
        }

        let op = ParseOp::Choice {
            parsers: (first, parsers),
        };

        Ok(self.push(op))
    }

    pub fn choice_nocap(
        self,
        rest: impl Iterator<Item = Result<Parsing, TokenStream>>,
    ) -> Result<Self, TokenStream> {
        let first = self.result();
        let parsers = (first, rest.collect::<Result<_, _>>()?);
        Ok(self.push(ParseOp::Choice { parsers }))
    }

    fn recovery(self, capture: Capture) -> Self {
        let op = ParseOp::Recovery {
            parser: Box::new(self),
        };
        Self::from_op(op, capture)
    }

    pub fn repeat(self, at_least: usize) -> Self {
        let cap = self.capture.to_anoymous();
        let parser = Box::new(self);
        Self::from_op(ParseOp::Repeat { parser, at_least }, cap.clone()).recovery(cap)
    }

    pub fn optional(self) -> Self {
        let cap = self.capture.to_anoymous();
        let parser = Box::new(self);
        Self::from_op(ParseOp::Optional { parser }, cap.clone()).recovery(cap)
    }

    pub fn look_ahead(self) -> Self {
        let parser = self.result();
        self.push(ParseOp::Ignore { parser })
            .recovery(Capture::Slient)
    }

    pub fn look_ahead_not(self) -> Self {
        let parser = self.result();
        self.push(ParseOp::Not { parser }).recovery(Capture::Slient)
    }
}

pub enum ParseOp {
    /// ```
    /// {state}.char({0})
    /// ```
    Just(char),
    /// ```
    /// {parser}.parse_memo({state}, {..depends})
    /// ```
    Call {
        parser: ParserRef,
        depends: Vec<ParserRef>,
    },
    /// ```
    /// {parser}.map(|{cap}| {f})
    /// ```
    Map {
        parser: Value,
        cap: Capture,
        expr: syn::Expr,
    },
    /// ```
    /// match {prev} {
    ///     Ok(v1) => {next}.map(|v2| (v1, v2)),
    ///     Err(e) => Err(e),
    /// }
    /// ```
    Then { prev: Value, next: Box<Parsing> },
    /// ```
    /// match {prev} {
    ///     Ok(v1) => {next}.map(|_| v1),
    ///     Err(e) => Err(e),
    /// }
    /// ```
    ThenIgnore { prev: Value, next: Box<Parsing> },
    /// ```
    /// match {prev} {
    ///     Ok(_) => {next},
    ///     Err(e) => Err(e),
    /// }
    /// ```
    IgnoreThen { prev: Value, next: Box<Parsing> },
    /// ```
    /// let mut results = vec![];
    /// while let Ok(value) = {parser} {
    ///     results.push(value);
    /// }
    /// if results.len() >= {at_least} {
    ///     Ok(results)
    /// } else {
    ///     Err(state.error())
    /// }
    /// ```
    Repeat {
        parser: Box<Parsing>,
        at_least: usize,
    },
    /// ```
    /// {parser}.ok()
    /// ```
    Optional { parser: Box<Parsing> },
    /// ```
    /// let fork = {state}.fork();
    /// {push_state(&fork)}
    /// let value = {parser};
    /// {pop_state(&fork)}
    /// value.inspect(|_| {state}.advance_to(&fork))
    /// ```
    Recovery { parser: Box<Parsing> },
    /// ```
    /// {parser}.map(|_| ())
    /// ```
    Ignore { parser: Value },
    /// ```
    /// if let Ok(value) = {parser} {
    ///     Err(state.error())
    /// } else {
    ///     Ok(())
    /// }
    /// ```
    Not { parser: Value },
    /// ```
    /// if let Ok(value) = {parser.0} {
    ///     Ok(value)
    /// } else if let Ok(value) = {parser.1[0]} {
    ///     Ok(value)
    /// } else if let Ok(value) = {parser.1[1]} {
    ///     Ok(value)
    /// } ... else {
    ///     Err(state.error())
    /// }
    /// ```
    Choice { parsers: (Value, Vec<Parsing>) },
}

pub enum MemoKind {
    None,
    Memorize,
    LeftRec,
}

pub struct ParserImpl {
    pub name: syn::Ident,
    pub curr: ParserRef,
    pub parser: Parsing,
    pub memo: MemoKind,
    pub vis: syn::Visibility,
    pub ret_ty: syn::Type,
    pub depends: Vec<(ParserRef, syn::Ident)>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ParserRef(syn::Ident);

impl ParserRef {
    pub fn new(name: &syn::Ident) -> Self {
        Self(format_ident!(
            "__parser_{}",
            name,
            span = Span::mixed_site()
        ))
    }

    pub fn curr() -> Self {
        Self(format_ident!("self"))
    }

    pub fn as_ident(&self) -> &syn::Ident {
        &self.0
    }
}

pub struct Middle {
    pub crate_name: TokenStream,
    pub mod_name: syn::Ident,
    pub parsers: Vec<ParserImpl>,
}

impl Middle {
    pub fn new(crate_name: TokenStream, mod_name: syn::Ident) -> Self {
        Self {
            crate_name,
            mod_name,
            parsers: vec![],
        }
    }
}

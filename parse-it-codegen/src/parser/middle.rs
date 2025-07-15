use hashlink::{LinkedHashMap, LinkedHashSet};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, visit::Visit};

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

pub struct PatVistor {
    pub captures: LinkedHashSet<syn::Ident>,
}

impl PatVistor {
    pub fn new() -> Self {
        Self {
            captures: LinkedHashSet::default(),
        }
    }

    pub fn collect_captures(pat: &syn::Pat) -> LinkedHashSet<syn::Ident> {
        let mut visitor = Self::new();
        visitor.visit_pat(pat);
        visitor.captures
    }
}

impl Visit<'_> for PatVistor {
    fn visit_pat_ident(&mut self, i: &syn::PatIdent) {
        self.captures.insert(i.ident.clone());
    }
}

#[derive(Clone)]
pub enum Capture {
    Loud,
    Slient,
    Named(Box<syn::Pat>, Box<Capture>),
    Tuple(Box<Capture>, Box<Capture>),
    TupleVec(Vec<syn::Ident>),
}

impl Capture {
    pub fn is_loud(&self) -> bool {
        match self {
            Capture::Loud => true,
            Capture::Slient => false,
            Capture::Named(_, _) => true,
            Capture::Tuple(_, n) => n.is_loud(),
            Capture::TupleVec(_) => true,
        }
    }

    pub fn to_anonymous(&self) -> Capture {
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

    pub fn just(c: syn::Lit) -> Self {
        Self::from_op(ParseOp::Just(c), Capture::Slient)
    }

    pub fn just_pat(p: syn::Pat) -> Self {
        let captures = PatVistor::collect_captures(&p);
        let captures: Vec<syn::Ident> = captures.into_iter().collect();
        Self::from_op(
            ParseOp::Pat(p.clone(), captures.clone()),
            Capture::TupleVec(captures),
        )
    }

    pub fn call(name: syn::Ident, depends: Vec<ParserRef>) -> Self {
        Self::from_op(
            ParseOp::Call {
                parser: ParserRef::new(&name),
                depends,
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
        self,
        rest: impl Iterator<Item = Result<Parsing, TokenStream>>,
    ) -> Result<Self, TokenStream> {
        let mut capture = self.capture.clone();
        let mut parsers = vec![self];

        for item in rest {
            let parser = item?;
            capture = capture.unify(&parser.capture)?;
            parsers.push(parser);
        }

        let op = ParseOp::Choice { parsers };
        Ok(Self::from_op(op, capture))
    }

    pub fn choice_nocap(
        self,
        rest: impl Iterator<Item = Result<Parsing, TokenStream>>,
    ) -> Result<Self, TokenStream> {
        let parsers = std::iter::once(Ok(self)).chain(rest);
        let parsers = parsers.collect::<Result<Vec<_>, _>>()?;
        let op = ParseOp::Choice { parsers };
        Ok(Self::from_op(op, Capture::Loud))
    }

    pub fn repeat(self, at_least: usize) -> Self {
        let cap = self.capture.to_anonymous();
        let parser = Box::new(self);
        Self::from_op(ParseOp::Repeat { parser, at_least }, cap)
    }

    pub fn optional(self) -> Self {
        let cap = self.capture.to_anonymous();
        let parser = Box::new(self);
        Self::from_op(ParseOp::Optional { parser }, cap)
    }

    pub fn look_ahead(self) -> Self {
        Self::from_op(
            ParseOp::LookAhead {
                parser: Box::new(self),
            },
            Capture::Slient,
        )
    }

    pub fn look_ahead_not(self) -> Self {
        Self::from_op(
            ParseOp::LookAheadNot {
                parser: Box::new(self),
            },
            Capture::Slient,
        )
    }
}

pub enum ParseOp {
    /// ```ignore
    /// {state}.parse({lit})
    /// ```
    Just(syn::Lit),
    /// ```ignore
    /// {state}.parse(|tt| match tt {
    ///     {pat} => Some(({..cap})),
    ///     _ => None,
    /// })
    /// ```
    Pat(syn::Pat, Vec<syn::Ident>),
    /// ```ignore
    /// {parser}.parse_memo({state}, {..depends})
    /// ```
    Call {
        parser: ParserRef,
        depends: Vec<ParserRef>,
    },
    /// ```ignore
    /// {parser}.map(|{cap}| {f})
    /// ```
    Map {
        parser: Value,
        cap: Capture,
        expr: syn::Expr,
    },
    /// ```ignore
    /// match {prev} {
    ///     Ok(v1) => {next}.map(|v2| (v1, v2)),
    ///     Err(e) => Err(e),
    /// }
    /// ```
    Then { prev: Value, next: Box<Parsing> },
    /// ```ignore
    /// match {prev} {
    ///     Ok(v1) => {next}.map(|_| v1),
    ///     Err(e) => Err(e),
    /// }
    /// ```
    ThenIgnore { prev: Value, next: Box<Parsing> },
    /// ```ignore
    /// match {prev} {
    ///     Ok(_) => {next},
    ///     Err(e) => Err(e),
    /// }
    /// ```
    IgnoreThen { prev: Value, next: Box<Parsing> },
    /// ```ignore
    /// let fork = &{state}.fork();
    /// let mut results = vec![];
    /// while let Ok(value) = {parser/fork} {
    ///     {state}.advance_to(fork);
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
    /// ```ignore
    /// {parser}.ok()
    /// ```
    Optional { parser: Box<Parsing> },
    /// ```ignore
    /// let fork = &{state}.fork();
    /// {parser/fork}.map(|_| ())
    /// ```
    LookAhead { parser: Box<Parsing> },
    /// ```ignore
    /// let fork = &{state}.fork();
    /// if let Ok(value) = {parser/fork} {
    ///     Err(state.error())
    /// } else {
    ///     Ok(())
    /// }
    /// ```
    LookAheadNot { parser: Box<Parsing> },
    /// ```ignore
    /// let mut fork = &{state}.fork();
    /// if let Ok(value) = {parser[0]/fork} {
    ///     {state}.advance_to(fork);
    ///     Ok(value)
    /// } else if let Ok(value) = {
    ///     fork = &{state}.fork();
    ///     {parser[1]/fork}
    /// } {
    ///     {state}.advance_to(fork);
    ///     Ok(value)
    /// } ... else {
    ///     Err(state.error())
    /// }
    /// ```
    Choice { parsers: Vec<Parsing> },
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
    pub attrs: Vec<syn::Attribute>,
    pub crate_name: TokenStream,
    pub mod_name: syn::Ident,
    pub items: Vec<syn::Item>,
    pub parsers: Vec<ParserImpl>,
    pub debug: bool,
}

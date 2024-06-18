use std::collections::{HashMap, HashSet};

use proc_macro2::TokenStream;
use quote::{format_ident, quote_spanned};
use syn::visit_mut::VisitMut;

use crate::middle::{Capture, Middle, Value, ValueData};
use crate::syntax::{Atom, ParseIt, Parser, Part, Production, Rule};
use crate::Hasher;

#[derive(Default)]
struct Context {
    pub symbols: HashMap<String, Symbol, Hasher>,
    pub left_calls: HashMap<String, HashSet<String, Hasher>, Hasher>,
    pub left_recursion: HashSet<String, Hasher>,
}

enum Symbol {
    Decl(Value, syn::Ident),
    Defined(Value),
}

impl Symbol {
    fn value(&self) -> Value {
        match self {
            Symbol::Decl(value, _) | Symbol::Defined(value) => *value,
        }
    }

    fn initialized(&self) -> bool {
        matches!(self, Symbol::Defined(_))
    }
}

impl Context {
    fn new() -> Self {
        Self::default()
    }
}

struct ExprVisitor {
    /// replace `self` with this ident
    pub self_ident: syn::Ident,
    /// whether `self` is referred
    pub referred_self: bool,
}

impl ExprVisitor {
    pub fn new() -> Self {
        Self {
            self_ident: format_ident!("r#__self"),
            referred_self: false,
        }
    }
}

impl VisitMut for ExprVisitor {
    fn visit_ident_mut(&mut self, i: &mut proc_macro2::Ident) {
        if i == "self" {
            let span = i.span();
            *i = self.self_ident.clone();
            i.set_span(span);
            self.referred_self = true;
        }
    }
}

impl ParseIt {
    pub fn compile(self) -> Result<Middle, TokenStream> {
        let mut lang = Middle::new();
        let mut ctx = Context::new();

        self.analyze_left_recursion(&mut ctx);

        for parser in self.parsers {
            parser.compile(&mut lang, &mut ctx)?;
        }

        for symbol in ctx.symbols.values() {
            if let Symbol::Decl(_, id) = symbol {
                return Err(quote_spanned! {
                    id.span() => compile_error!("undefined parser")
                });
            }
        }

        for name in self.results {
            if let Some(sym) = ctx.symbols.get(&name.to_string()) {
                lang.results.push(sym.value());
            } else {
                return Err(quote_spanned! {
                    name.span() => compile_error!("undefined parser")
                });
            }
        }

        Ok(lang)
    }

    fn analyze_left_recursion(&self, ctx: &mut Context) {
        fn dfs(
            name: &str,
            stack: &mut HashSet<String, Hasher>,
            left_calls: &HashMap<String, HashSet<String, Hasher>, Hasher>,
            left_recursion: &mut HashSet<String, Hasher>,
        ) {
            stack.insert(name.to_string());

            for leftcall in &left_calls[name] {
                if stack.contains(leftcall) {
                    left_recursion.extend(stack.iter().cloned());
                    return;
                } else {
                    dfs(leftcall, stack, left_calls, left_recursion);
                }
            }

            stack.remove(name);
        }

        for parser in &self.parsers {
            parser.analyze_left_calls(ctx);
        }

        for parser in &self.parsers {
            let name = parser.name.to_string();
            if !ctx.left_recursion.contains(&name) {
                dfs(
                    &name,
                    &mut HashSet::default(),
                    &ctx.left_calls,
                    &mut ctx.left_recursion,
                );
            }
        }
    }
}

impl Parser {
    fn compile(self, lang: &mut Middle, ctx: &mut Context) -> Result<(), TokenStream> {
        if self.rules.is_empty() {
            return Err(quote_spanned! {
                self.name.span() => compile_error!("empty parser")
            });
        }
        let rules = self
            .rules
            .into_iter()
            .map(|rule| rule.compile(lang, ctx, self.ty.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let value = if rules.len() == 1 {
            rules.into_iter().next().unwrap()
        } else {
            let rules = ValueData::choice_nocap(rules);
            lang.push_back(rules)
        };
        let value = lang.push_back(ValueData::memorize(
            value,
            ctx.left_recursion.contains(&self.name.to_string()),
        ));

        let name = self.name.to_string();
        if let Some(symbol) = ctx.symbols.get_mut(&name) {
            if symbol.initialized() {
                return Err(quote_spanned! {
                    self.name.span() => compile_error!("redefinition of parser")
                });
            }
            let value = lang.push_back(ValueData::define(symbol.value(), value));
            *symbol = Symbol::Defined(value);
        } else {
            ctx.symbols.insert(name, Symbol::Defined(value));
        }

        Ok(())
    }

    fn analyze_left_calls<'a>(&self, ctx: &'a mut Context) -> &'a HashSet<String, Hasher> {
        ctx.left_calls
            .entry(self.name.to_string())
            .or_insert_with(move || {
                let mut set = HashSet::default();
                for rule in &self.rules {
                    for part in rule.production.first_progress() {
                        if let Atom::NonTerminal(p) = &part.part {
                            set.insert(p.to_string());
                        }
                    }
                }
                set
            })
    }
}

impl Rule {
    fn compile(
        mut self,
        lang: &mut Middle,
        ctx: &mut Context,
        ty: syn::Type,
    ) -> Result<Value, TokenStream> {
        let (value, mut capture) = self.production.compile(lang, ctx)?;

        let mut visitor = ExprVisitor::new();
        visitor.visit_expr_mut(&mut self.action);
        if visitor.referred_self {
            capture = Capture::Named(
                Box::new(syn::Pat::Ident(syn::PatIdent {
                    attrs: Vec::new(),
                    by_ref: None,
                    mutability: None,
                    ident: visitor.self_ident,
                    subpat: None,
                })),
                Box::new(capture),
            );
        }

        let value = ValueData::map(value, capture, ty, self.action);
        let value = lang.push_back(value);
        Ok(value)
    }
}

impl Production {
    fn compile(
        self,
        lang: &mut Middle,
        ctx: &mut Context,
    ) -> Result<(Value, Capture), TokenStream> {
        let mut result = self.parts.0.compile(lang, ctx)?;
        for part in self.parts.1 {
            let (value, cap) = ValueData::then(result, part.compile(lang, ctx)?);
            let value = lang.push_back(value);
            result = (value, cap);
        }
        Ok(result)
    }

    /// Iterate over the parts that may "make first progress" when parsing.
    fn first_progress(&self) -> impl Iterator<Item = &Part> {
        let mut iter = std::iter::once(&self.parts.0).chain(self.parts.1.iter());
        let mut finished = false;
        std::iter::from_fn(move || {
            if finished {
                return None;
            }
            for part in iter.by_ref() {
                if part.part.must_progress() {
                    finished = true;
                    return Some(part);
                } else if part.part.may_progress() {
                    return Some(part);
                }
            }
            finished = true;
            None
        })
    }

    /// Whether this production must make progress when parsing.
    fn must_progress(&self) -> bool {
        self.first_progress().any(|p| p.part.must_progress())
    }

    /// Whether this production may make progress when parsing.
    fn may_progress(&self) -> bool {
        self.first_progress().any(|p| p.part.may_progress())
    }
}

impl Part {
    fn compile(
        self,
        lang: &mut Middle,
        ctx: &mut Context,
    ) -> Result<(Value, Capture), TokenStream> {
        let (value, capture) = self.part.compile(lang, ctx)?;
        let capture = match self.capture {
            crate::syntax::Capture::Named(name) => Capture::Named(name, Box::new(capture)),
            crate::syntax::Capture::Loud => {
                if capture.is_loud() {
                    capture
                } else {
                    Capture::Loud
                }
            }
            crate::syntax::Capture::NotSpecified => capture,
        };
        Ok((value, capture))
    }
}

impl Atom {
    fn compile(
        self,
        lang: &mut Middle,
        ctx: &mut Context,
    ) -> Result<(Value, Capture), TokenStream> {
        match self {
            Atom::Terminal(lit) => {
                let (value, capture) = match lit {
                    syn::Lit::Char(c) => ValueData::just(c.value()),
                    _ => {
                        Err(quote_spanned! { lit.span() => compile_error!("unsupported literal") })?
                    }
                };
                let value = lang.push_back(value);
                Ok((value, capture))
            }
            Atom::NonTerminal(id) => {
                let name = id.to_string();
                let value = ctx
                    .symbols
                    .entry(name)
                    .or_insert_with(|| {
                        let value = lang.push_back(ValueData::declare());
                        Symbol::Decl(value, id)
                    })
                    .value();
                Ok((value, Capture::Loud))
            }
            Atom::Sub(p) => p.compile(lang, ctx),
            Atom::Choice(choices) => {
                let (choices, capture) =
                    ValueData::choice(choices.into_iter().map(|p| p.compile(lang, ctx)))?;
                let value = lang.push_back(choices);
                Ok((value, capture))
            }
            Atom::Repeat(p) => {
                let (value, capture) = ValueData::repeat(p.compile(lang, ctx)?);
                let value = lang.push_back(value);
                Ok((value, capture))
            }
            Atom::Repeat1(p) => {
                let (value, capture) = ValueData::repeat1(p.compile(lang, ctx)?);
                let value = lang.push_back(value);
                Ok((value, capture))
            }
            Atom::Optional(p) => {
                let (value, capture) = ValueData::or_not(p.compile(lang, ctx)?);
                let value = lang.push_back(value);
                Ok((value, capture))
            }
            Atom::LookAhead(p) => {
                let (value, capture) = ValueData::look_ahead(p.compile(lang, ctx)?.0);
                let value = lang.push_back(value);
                Ok((value, capture))
            }
            Atom::LookAheadNot(p) => {
                let (value, capture) = ValueData::look_ahead_not(p.compile(lang, ctx)?.0);
                let value = lang.push_back(value);
                Ok((value, capture))
            }
        }
    }

    /// Whether this atom must make progress when parsing.
    fn must_progress(&self) -> bool {
        match self {
            Atom::Terminal(_) | Atom::NonTerminal(_) => true,
            Atom::Repeat(_) | Atom::Optional(_) | Atom::LookAhead(_) | Atom::LookAheadNot(_) => {
                false
            }
            Atom::Sub(p) => p.must_progress(),
            Atom::Choice(choices) => choices.iter().all(|p| p.must_progress()),

            Atom::Repeat1(p) => p.must_progress(),
        }
    }

    /// Whether this atom may make progress when parsing.
    fn may_progress(&self) -> bool {
        match self {
            Atom::Terminal(_) | Atom::NonTerminal(_) => true,
            Atom::LookAhead(_) | Atom::LookAheadNot(_) => false,
            Atom::Sub(p) => p.may_progress(),
            Atom::Choice(choices) => choices.iter().any(|p| p.may_progress()),
            Atom::Repeat(p) | Atom::Repeat1(p) | Atom::Optional(p) => p.may_progress(),
        }
    }
}

use std::vec;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::visit_mut::VisitMut;

use crate::hash::{HashMap, HashSet, OrderedMap, OrderedSet};
use crate::middle::{Capture, MemoKind, Middle, ParserImpl, ParserRef, Parsing};
use crate::syntax::{Atom, ParseIt, Parser, Part, Production, Rule};

#[derive(Default)]
struct Context {
    pub left_calls: HashMap<syn::Ident, HashSet<syn::Ident>>,
    pub left_recursion: HashSet<syn::Ident>,
    pub direct_depends: HashMap<syn::Ident, OrderedMap<syn::Ident, ParserRef>>,
    pub depends: HashMap<syn::Ident, OrderedMap<syn::Ident, ParserRef>>,
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
            self_ident: format_ident!("r#__self", span = Span::call_site()),
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
        let mut ctx = Context::default();

        self.analyze_left_recursion(&mut ctx);
        self.analyze_depends(&mut ctx);

        let crate_name = match &self.crate_name {
            Some(crate_name) => quote! { #crate_name },
            None => quote! { ::parse_it },
        };
        let mut middle = Middle::new(crate_name, self.mod_name);
        for parser in self.parsers {
            let parser = parser.compile(&mut ctx)?;
            middle.parsers.push(parser);
        }

        Ok(middle)
    }

    fn analyze_left_recursion(&self, ctx: &mut Context) {
        for parser in &self.parsers {
            parser.analyze_left_calls(ctx);
        }

        // left recursion is a FVS in the left_calls graph
        for name in ctx.left_calls.keys() {
            if ctx.left_recursion.contains(name) {
                continue;
            }
            let mut stack = OrderedSet::default();
            stack.insert(name);
            while let Some(name) = stack.pop_back() {
                for dep in &ctx.left_calls[name] {
                    if ctx.left_recursion.contains(dep) {
                        continue;
                    }
                    if !stack.insert(dep) || dep == name {
                        ctx.left_recursion.insert(name.clone());
                        break;
                    }
                }
            }
        }
    }

    fn analyze_depends(&self, ctx: &mut Context) {
        for parser in &self.parsers {
            parser.analyze_direct_depends(ctx);
        }

        // full dependencies are transitive closure of direct dependencies
        for name in ctx.direct_depends.keys() {
            let mut depends = OrderedMap::default();
            let mut stack = vec![name];
            while let Some(name) = stack.pop() {
                if depends.contains_key(name) {
                    continue;
                }
                depends.insert(name.clone(), ParserRef::new(name));
                stack.extend(ctx.direct_depends[name].keys());
            }
            depends.remove(name);
            ctx.depends.insert(name.clone(), depends);
        }
    }
}

impl Parser {
    fn compile(self, ctx: &mut Context) -> Result<ParserImpl, TokenStream> {
        let curr = ParserRef::new(&self.name);
        let depends = ctx.depends[&self.name]
            .iter()
            .map(|(p, i)| (i.clone(), p.clone()))
            .collect();
        let mut parser = self.rules.0.compile(ctx)?;
        if !self.rules.1.is_empty() {
            parser = parser.choice_nocap(self.rules.1.into_iter().map(|rule| rule.compile(ctx)))?;
        }

        let memo = if ctx.left_recursion.contains(&self.name) {
            MemoKind::LeftRec
        } else {
            MemoKind::Memorize
        };

        Ok(ParserImpl {
            name: self.name,
            curr,
            parser,
            memo,
            vis: self.vis,
            ret_ty: self.ty,
            depends,
        })
    }

    fn analyze_left_calls<'a>(&self, ctx: &'a mut Context) -> &'a HashSet<syn::Ident> {
        ctx.left_calls
            .entry(self.name.clone())
            .or_insert_with(move || {
                let mut set = HashSet::default();
                for rule in self.rules() {
                    set.extend(rule.left_calls());
                }
                set
            })
    }

    fn analyze_direct_depends<'a>(
        &self,
        ctx: &'a mut Context,
    ) -> &'a OrderedMap<syn::Ident, ParserRef> {
        ctx.direct_depends
            .entry(self.name.clone())
            .or_insert_with(move || {
                let mut depends = OrderedMap::default();
                for rule in self.rules() {
                    rule.production
                        .analyze_direct_depends(&mut depends, &self.name);
                }
                depends
            })
    }
}

impl Rule {
    fn compile(mut self, ctx: &mut Context) -> Result<Parsing, TokenStream> {
        let mut parser = self.production.compile(ctx)?;

        let mut visitor = ExprVisitor::new();
        visitor.visit_expr_mut(&mut self.action);
        if visitor.referred_self {
            parser.capture = Capture::Named(
                Box::new(syn::Pat::Ident(syn::PatIdent {
                    attrs: Vec::new(),
                    by_ref: None,
                    mutability: None,
                    ident: visitor.self_ident,
                    subpat: None,
                })),
                Box::new(parser.capture),
            );
        }

        Ok(parser.map(self.action))
    }

    fn left_calls(&self) -> impl Iterator<Item = syn::Ident> + '_ {
        self.production
            .first_progress()
            .filter_map(|part| match &part.part {
                Atom::NonTerminal(p) => Some(p.clone()),
                _ => None,
            })
    }
}

impl Production {
    fn compile(self, ctx: &mut Context) -> Result<Parsing, TokenStream> {
        let mut result = self.parts.0.compile(ctx)?;
        for part in self.parts.1 {
            let part = part.compile(ctx)?;
            result = result.then(Box::new(part));
        }
        Ok(result)
    }

    /// Iterate over the parts that may "make first progress" when parsing.
    fn first_progress(&self) -> impl Iterator<Item = &Part> {
        let mut iter = self.parts();
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

    fn analyze_direct_depends(
        &self,
        depends: &mut OrderedMap<syn::Ident, ParserRef>,
        curr: &syn::Ident,
    ) {
        for part in self.parts() {
            part.part.analyze_direct_depends(depends, curr);
        }
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
    fn compile(self, ctx: &mut Context) -> Result<Parsing, TokenStream> {
        let mut parser = self.part.compile(ctx)?;
        match self.capture {
            crate::syntax::Capture::Named(name) => {
                parser.capture = Capture::Named(name, Box::new(parser.capture));
            }
            crate::syntax::Capture::Loud => {
                if !parser.capture.is_loud() {
                    parser.capture = Capture::Loud;
                }
            }
            crate::syntax::Capture::NotSpecified => {}
        }
        Ok(parser)
    }
}

impl Atom {
    fn compile(self, ctx: &mut Context) -> Result<Parsing, TokenStream> {
        match self {
            Atom::Terminal(lit) => match lit {
                syn::Lit::Char(c) => Ok(Parsing::just(c.value())),
                _ => Err(quote_spanned! { lit.span() => compile_error!("unsupported literal") })?,
            },
            Atom::NonTerminal(name) => {
                let depends = ctx.depends.get(&name).ok_or_else(|| {
                    quote_spanned! { name.span() => compile_error!("use of undeclared parser") }
                })?;
                let depends = depends.iter().map(|(_, p)| p.clone());
                Ok(Parsing::call(name, depends))
            }
            Atom::Sub(p) => p.compile(ctx),
            Atom::Choice(first, rest) => first
                .compile(ctx)?
                .choice(rest.into_iter().map(|p| p.compile(ctx))),
            Atom::Repeat(p) => Ok(p.compile(ctx)?.repeat(0)),
            Atom::Repeat1(p) => Ok(p.compile(ctx)?.repeat(1)),
            Atom::Optional(p) => Ok(p.compile(ctx)?.optional()),
            Atom::LookAhead(p) => Ok(p.compile(ctx)?.look_ahead()),
            Atom::LookAheadNot(p) => Ok(p.compile(ctx)?.look_ahead_not()),
        }
    }

    fn analyze_direct_depends(
        &self,
        depends: &mut OrderedMap<syn::Ident, ParserRef>,
        curr: &syn::Ident,
    ) {
        match self {
            Atom::NonTerminal(name) if name != curr => {
                depends.insert(name.clone(), ParserRef::new(name));
            }
            Atom::Sub(p) => p.analyze_direct_depends(depends, curr),
            Atom::Choice(first, rest) => {
                first.analyze_direct_depends(depends, curr);
                for p in rest {
                    p.analyze_direct_depends(depends, curr);
                }
            }
            Atom::Repeat(p)
            | Atom::Repeat1(p)
            | Atom::Optional(p)
            | Atom::LookAhead(p)
            | Atom::LookAheadNot(p) => p.analyze_direct_depends(depends, curr),
            _ => {}
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
            Atom::Choice(first, rest) => {
                first.must_progress() && rest.iter().all(|p| p.must_progress())
            }
            Atom::Repeat1(p) => p.must_progress(),
        }
    }

    /// Whether this atom may make progress when parsing.
    fn may_progress(&self) -> bool {
        match self {
            Atom::Terminal(_) | Atom::NonTerminal(_) => true,
            Atom::LookAhead(_) | Atom::LookAheadNot(_) => false,
            Atom::Sub(p) => p.may_progress(),
            Atom::Choice(first, rest) => {
                first.may_progress() || rest.iter().any(|p| p.may_progress())
            }
            Atom::Repeat(p) | Atom::Repeat1(p) | Atom::Optional(p) => p.may_progress(),
        }
    }
}

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote_spanned;

use crate::{
    backend::{Capture, Middle, Value, ValueData},
    syntax::{Atom, ParseIt, Parser, Part, Production, Rule},
};

struct Context {
    pub symbols: HashMap<String, Symbol>,
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
        Self {
            symbols: HashMap::new(),
        }
    }
}

impl ParseIt {
    pub fn compile(self) -> Result<Middle, TokenStream> {
        let mut lang = Middle::new();
        let mut ctx = Context::new();

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
}

impl Parser {
    fn compile(self, lang: &mut Middle, ctx: &mut Context) -> Result<(), TokenStream> {
        let rules = self
            .rules
            .into_iter()
            .map(|rule| rule.compile(lang, ctx))
            .collect::<Result<Vec<_>, _>>()?;
        let rules = ValueData::choice_nocap(rules);
        let value = lang.push_back(rules);

        let name = self.name.to_string();
        if let Some(symbol) = ctx.symbols.get_mut(&name) {
            if symbol.initialized() {
                return Err(quote_spanned! {
                    self.name.span() => compile_error!("redefinition of parser")
                });
            }
            lang.push_back(ValueData::define(symbol.value(), value));
            *symbol = Symbol::Defined(value);
        } else {
            ctx.symbols.insert(name, Symbol::Defined(value));
        }

        Ok(())
    }
}

impl Rule {
    fn compile(self, lang: &mut Middle, ctx: &mut Context) -> Result<Value, TokenStream> {
        let (value, _) = self.production.compile(lang, ctx)?;
        let value = ValueData::map(value, self.action);
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
        }
    }
}

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::{
    backend::{Capture, Middle, Value, ValueData},
    syntax::{ParseIt, Parser, Production, Rule},
};

struct Context {
    pub symbols: HashMap<String, Symbol>,
}

struct Symbol {
    pub value: Value,
    pub initialized: bool,
}

impl Symbol {
    fn new_decl(value: Value) -> Self {
        Self {
            value,
            initialized: false,
        }
    }

    fn new_defined(value: Value) -> Self {
        Self {
            value,
            initialized: true,
        }
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
            if symbol.initialized {
                return Err(quote_spanned! {
                    self.name.span() => compile_error!("redefinition of parser")
                });
            }
            lang.push_back(ValueData::define(symbol.value, value));
            symbol.initialized = true;
        } else {
            ctx.symbols.insert(name, Symbol::new_defined(value));
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
        todo!()
    }
}

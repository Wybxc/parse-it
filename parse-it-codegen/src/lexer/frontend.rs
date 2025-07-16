use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};

use crate::{
    hash::HashMap,
    lexer::middle::{LexerImpl, Middle, Rule},
    syntax::{Lexer, LexerMod, LexerPattern},
};

impl LexerMod {
    pub fn compile(self) -> Result<Middle, TokenStream> {
        let crate_name = match &self.config.crate_name {
            Some(crate_name) => quote! { #crate_name },
            None => quote! { ::parse_it },
        };

        let lexers = self
            .lexers
            .iter()
            .map(|lexer| (lexer.name.clone(), lexer))
            .collect::<HashMap<_, _>>();
        let lexers = self
            .lexers
            .iter()
            .map(|lexer| lexer.compile(&lexers))
            .collect::<Result<Vec<_>, _>>()?;

        let middle = Middle {
            attrs: self.attrs,
            crate_name,
            mod_name: self.mod_name,
            items: self.items,
            lexers,
            debug: self.config.debug,
        };
        Ok(middle)
    }
}

impl Lexer {
    pub fn full_rules(
        &self,
        lexers: &HashMap<syn::Ident, &Lexer>,
        stack: &mut Vec<syn::Ident>,
    ) -> Result<Vec<Rule>, TokenStream> {
        stack.push(self.name.clone());
        let mut rules = vec![];
        for rule in &self.rules {
            match &rule.pattern {
                LexerPattern::Regex(lit_str) => {
                    if let Err(e) = regex_syntax::parse(&lit_str.value()) {
                        let e = format!("Invalid regex pattern: {e}");
                        return Err(quote_spanned! { lit_str.span() => compile_error!(#e) });
                    }
                    rules.push(Rule {
                        pattern: lit_str.clone(),
                        action: vec![(rule.action.clone(), self.ty.clone())],
                    });
                }
                LexerPattern::Name(ident) => {
                    if stack.contains(ident) {
                        let e = format!("Recursive inclusion of lexer `{ident}`");
                        return Err(quote_spanned! { ident.span() => compile_error!(#e) });
                    }
                    let lexer = lexers.get(ident).ok_or_else(|| {
                        let e = format!("Lexer `{ident}` not found");
                        quote_spanned! { ident.span() => compile_error!(#e) }
                    })?;
                    if !lexer.inputs.is_empty() {
                        let e = format!("Cannot include lexer `{ident}` in another lexer, it has inputs defined");
                        return Err(quote_spanned! { ident.span() => compile_error!(#e) });
                    }
                    let action = rule.action.clone();
                    rules.extend(
                        lexer
                            .full_rules(lexers, stack)?
                            .into_iter()
                            .map(|mut rule| {
                                rule.action.push((action.clone(), self.ty.clone()));
                                rule
                            }),
                    );
                }
            }
        }
        stack.pop();
        Ok(rules)
    }

    pub fn compile(&self, lexers: &HashMap<syn::Ident, &Lexer>) -> Result<LexerImpl, TokenStream> {
        let rules = self.full_rules(lexers, &mut vec![])?;
        Ok(LexerImpl {
            name: self.name.clone(),
            rules,
            vis: self.vis.clone(),
            ret_ty: self.ty.clone(),
        })
    }
}

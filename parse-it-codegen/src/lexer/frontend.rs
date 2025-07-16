use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::visit_mut::VisitMut;

use crate::{
    hash::HashMap,
    lexer::middle::{Action, LexerImpl, Middle, Rule},
    syntax::{Lexer, LexerMod, LexerPattern, LexerRule},
    utils::RewriteSelfVisitor,
};

#[derive(Default)]
struct Context {
    pub parse_macros: Rc<Vec<syn::Path>>,
}

impl LexerMod {
    pub fn compile(self) -> Result<Middle, TokenStream> {
        let ctx = Context {
            parse_macros: self.config.parse_macros.clone(),
        };
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
            .map(|lexer| lexer.compile(&lexers, &ctx))
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
    fn full_rules(
        &self,
        lexers: &HashMap<syn::Ident, &Lexer>,
        stack: &mut Vec<syn::Ident>,
        ctx: &Context,
    ) -> Result<Vec<Rule>, TokenStream> {
        stack.push(self.name.clone());
        let mut rules = vec![];
        for rule in &self.rules {
            match &rule.pattern {
                LexerPattern::Regex(lit_str) => {
                    if let Err(e) = regex_syntax::parse(&lit_str.value()) {
                        let e = format!("Invalid regex pattern: {e}");
                        return Err(quote_spanned! { lit_str.span() => compile_error!(#e); });
                    }
                    rules.push(Rule {
                        pattern: lit_str.clone(),
                        actions: (rule.compile(self.ty.clone(), ctx), vec![]),
                    });
                }
                LexerPattern::Name(ident) => {
                    if stack.contains(ident) {
                        let e = format!("Recursive inclusion of lexer `{ident}`");
                        return Err(quote_spanned! { ident.span() => compile_error!(#e); });
                    }
                    let lexer = lexers.get(ident).ok_or_else(|| {
                        let e = format!("Lexer `{ident}` not found");
                        quote_spanned! { ident.span() => compile_error!(#e); }
                    })?;
                    if !lexer.inputs.is_empty() {
                        let e = format!("Cannot include lexer `{ident}` in another lexer, it has inputs defined");
                        return Err(quote_spanned! { ident.span() => compile_error!(#e); });
                    }
                    let action = rule.compile(self.ty.clone(), ctx);
                    rules.extend(lexer.full_rules(lexers, stack, ctx)?.into_iter().map(
                        |mut rule| {
                            rule.actions.1.push(action.clone());
                            rule
                        },
                    ));
                }
            }
        }
        stack.pop();
        Ok(rules)
    }

    fn compile(
        &self,
        lexers: &HashMap<syn::Ident, &Lexer>,
        ctx: &Context,
    ) -> Result<LexerImpl, TokenStream> {
        if self.rules.is_empty() {
            let e = format!("Lexer `{}` has no rules defined", self.name);
            return Err(quote_spanned! { self.name.span() => compile_error!(#e); });
        }
        let rules = self.full_rules(lexers, &mut vec![], ctx)?;
        let inputs = self.inputs.iter().cloned().collect();
        Ok(LexerImpl {
            name: self.name.clone(),
            rules,
            vis: self.vis.clone(),
            inputs,
            ret_ty: self.ty.clone(),
        })
    }
}

impl LexerRule {
    fn compile(&self, ret_ty: Option<syn::Type>, ctx: &Context) -> Action {
        let mut action = self.action.clone();

        let mut visitor = RewriteSelfVisitor::new(ctx.parse_macros.clone());
        visitor.visit_expr_mut(&mut action);
        let self_ident = visitor.self_ident;

        Action {
            action,
            ret_ty,
            self_ident,
        }
    }
}

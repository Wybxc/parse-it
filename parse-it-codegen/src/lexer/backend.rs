use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, visit_mut::VisitMut};

use crate::lexer::middle::{Action, LexerImpl, Middle};

pub struct Context {
    crate_name: TokenStream,
    lexbuf: syn::Ident,
    _debug: bool,
}

impl Middle {
    pub fn expand(self) -> Result<TokenStream, TokenStream> {
        let mut result = TokenStream::new();
        let ctx = Context {
            crate_name: self.crate_name,
            lexbuf: format_ident!("r#__lexbuf", span = Span::call_site()),
            _debug: self.debug,
        };

        for lexer in self.lexers {
            result.extend(lexer.expand(&ctx)?);
        }

        let mod_name = self.mod_name;
        let attrs = self.attrs;
        let items = self.items;
        Ok(quote! {
            #[allow(non_snake_case)]
            #(#attrs)*
            mod #mod_name {
                #(#items)*
                #result
            }
        })
    }
}

impl LexerImpl {
    pub fn expand(self, ctx: &Context) -> Result<TokenStream, TokenStream> {
        let name = self.name;
        let vis = self.vis;
        let inputs = self.inputs;
        let ret_ty = if let Some(ref ret_ty) = self.ret_ty {
            quote! { #ret_ty }
        } else {
            quote! { () }
        };

        let mut regexes = vec![];
        let mut actions = vec![];
        for (i, rule) in self.rules.into_iter().enumerate() {
            regexes.push(rule.pattern);
            let (action, _) = rule.actions.1.into_iter().try_fold(
                rule.actions.0.expand(ctx)?,
                |(inner, inner_ty), it| -> Result<_, TokenStream> {
                    let (action, ret_ty) = it.expand(ctx)?;
                    Ok((
                        quote! {{
                            let __self: #inner_ty = #inner;
                            #action
                        }},
                        ret_ty,
                    ))
                },
            )?;
            actions.push(quote! {
                #i => #action
            });
        }

        let crate_name = &ctx.crate_name;
        let lexbuf = &ctx.lexbuf;

        let lexer_impl = if inputs.is_empty() {
            quote! {
                impl #crate_name::LexIt for #name {
                    type Token<'lex> = #ret_ty;

                    fn new() -> Self {
                        Self
                    }

                    fn next<'lex>(&self, #lexbuf: &mut #crate_name::LexerState<'lex>) -> Option<Self::Token<'lex>> {
                        Self::run(#lexbuf).ok().flatten()
                    }
                }
            }
        } else {
            quote! {}
        };

        Ok(quote! {
            #[derive(Clone, Copy, Debug)]
            #vis struct #name;

            impl #name {
                thread_local! {
                    static REGEX: #crate_name::lexer::Regex = #crate_name::lexer::Regex::new_many(
                        &[#(#regexes),*]
                    ).unwrap();
                }

                #[allow(
                    dead_code,
                    unreachable_code,
                    clippy::never_loop,
                    clippy::let_unit_value,
                    clippy::unit_arg,
                    clippy::useless_conversion
                )]
                pub fn run<'lex>(
                    #lexbuf: &mut #crate_name::lexer::LexerState<'lex>,
                    #(#inputs),*
                ) -> Result<Option<#ret_ty>, ()> {
                    Self::REGEX.with(|regex| {
                        'lex: loop {
                            if let Some(pat) = #lexbuf.run(regex) {
                                let __self = #lexbuf.lexeme();
                                let value = match pat.as_u32() as usize {
                                    #(#actions,)*
                                    _ => unreachable!(),
                                };
                                return Ok(Some(value));
                            } else {
                                return Err(());
                            }
                        }
                        Ok(None)
                    })
                }
            }

            #lexer_impl
        })
    }
}

struct ExpandLexMacroVisitor {
    crate_name: TokenStream,
    lexbuf: syn::Ident,
    failure: Vec<TokenStream>,
}

impl ExpandLexMacroVisitor {
    pub fn new(crate_name: TokenStream, lexbuf: syn::Ident) -> Self {
        Self {
            crate_name,
            lexbuf,
            failure: vec![],
        }
    }

    pub fn failure(self) -> Option<TokenStream> {
        self.failure.into_iter().reduce(|mut a, b| {
            a.extend(b);
            a
        })
    }
}

impl VisitMut for ExpandLexMacroVisitor {
    fn visit_macro_mut(&mut self, i: &mut syn::Macro) {
        if i.path.is_ident("lex") {
            struct LexMacro {
                pub lexer: syn::Ident,
                pub args: Vec<syn::Expr>,
            }
            impl syn::parse::Parse for LexMacro {
                fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                    let lexer = input.parse()?;
                    let args = if input.peek(syn::token::Paren) {
                        let content;
                        syn::parenthesized!(content in input);

                        let args =
                            Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated(&content)?;
                        args.into_iter().collect()
                    } else {
                        vec![]
                    };
                    Ok(Self { lexer, args })
                }
            }

            let crate_name = &self.crate_name;
            let lexbuf = &self.lexbuf;
            match syn::parse2::<LexMacro>(i.tokens.clone()) {
                Ok(lex_macro) => {
                    let LexMacro { lexer, args } = lex_macro;
                    i.path = syn::parse_quote!(#crate_name::identity);
                    i.tokens = quote! { #lexer::run(#lexbuf, #(#args),*)? };
                }
                Err(e) => self.failure.push(e.to_compile_error()),
            }
        }
    }
}

impl Action {
    pub fn expand(&self, ctx: &Context) -> Result<(TokenStream, TokenStream), TokenStream> {
        let mut action = self.action.clone();

        let mut visitor = ExpandLexMacroVisitor::new(ctx.crate_name.clone(), ctx.lexbuf.clone());
        visitor.visit_expr_mut(&mut action);
        if let Some(failure) = visitor.failure() {
            return Err(failure);
        }

        let ret_ty = self.ret_ty();
        Ok((
            quote! {
                #action
            },
            ret_ty,
        ))
    }

    pub fn ret_ty(&self) -> TokenStream {
        if let Some(ref ret_ty) = self.ret_ty {
            quote! { #ret_ty }
        } else {
            quote! { () }
        }
    }
}

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::hash::HashMap;
use crate::middle::{Capture, MemoKind, Middle, ParseOp, ParserImpl, Parsing, Value};

pub struct Context {
    crate_name: TokenStream,
}

impl Value {
    pub fn to_ident(self) -> syn::Ident {
        let val = self.id;
        format_ident!("r#__{}", val, span = Span::mixed_site())
    }
}

impl Capture {
    pub fn to_pat(&self) -> Result<TokenStream, TokenStream> {
        match self {
            Capture::Loud | Capture::Slient => Ok(quote! { _ }),
            Capture::Named(p, c) => match p.as_ref() {
                syn::Pat::Ident(_) => {
                    let c = c.to_pat()?;
                    Ok(quote! { #p @ #c })
                }
                _ => match c.as_ref() {
                    Capture::Loud | Capture::Slient => Ok(quote! { #p }),
                    _ => {
                        Err(quote_spanned! { p.span() => compile_error!("must be an ident here") })
                    }
                },
            },
            Capture::Tuple(c1, c2) => {
                let c1 = c1.to_pat()?;
                let c2 = c2.to_pat()?;
                Ok(quote! { (#c1, #c2) })
            }
        }
    }
}

impl Middle {
    pub fn expand(self) -> Result<TokenStream, TokenStream> {
        let mut result = TokenStream::new();
        let ctx = Context {
            crate_name: self.crate_name,
        };
        let mut ret_ty = HashMap::default();
        let mut depends = HashMap::default();

        for parser in self.parsers {
            ret_ty.insert(parser.name.clone(), parser.ret_ty.clone());
            depends.insert(parser.name.clone(), parser.depends.clone());
            result.extend(parser.expand(&ctx)?);
        }

        let crate_name = &ctx.crate_name;
        for id in &self.results {
            let ret_ty = ret_ty.get(id).ok_or_else(|| {
                quote_spanned! { id.span() => compile_error!("use of undeclared parser") }
            })?;
            let depends = depends.get(id).ok_or_else(|| {
                quote_spanned! { id.span() => compile_error!("use of undeclared parser") }
            })?;
            let depends_def = depends.iter().map(|(d, ty)| {
                let d = d.as_ident();
                quote! { let #d = &#ty::default(); }
            });
            let depends_use = depends.iter().map(|(d, _)| d.as_ident());

            result.extend(quote! {
                impl #crate_name::ParseIt for #id {
                    type Output = #ret_ty;

                    fn parse_stream(&self, state: &#crate_name::ParserState<char>) -> Result<#ret_ty, ::parse_it::Error> {
                        #(#depends_def)*
                        self.parse_memo(state, #(#depends_use),*)
                    }
                }
            });
        }

        let results = self.results;
        Ok(quote! {
            #[allow(non_snake_case)]
            mod __parse_it {
                use super::*;
                #result
            }
            use __parse_it::{#(#results),*};
        })
    }
}

#[derive(Default, Clone, Copy)]
pub struct StateToken(u32);

impl StateToken {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn fork(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn to_ident(self) -> syn::Ident {
        format_ident!("__state{}", self.0, span = Span::mixed_site())
    }
}

impl ParserImpl {
    pub fn expand(self, ctx: &Context) -> Result<TokenStream, TokenStream> {
        let name = self.name;
        let curr = self.curr.as_ident();
        let ret_ty = self.ret_ty;

        let crate_name = &ctx.crate_name;

        let depends_decl = self.depends.iter().map(|(d, ty)| {
            let name = d.as_ident();
            quote! { #name: &#ty }
        });
        let depends_decl = quote! { #(#depends_decl),* };

        let state_token = StateToken::new();
        let state = state_token.to_ident();
        let parser = self.parser.expand(state_token)?;
        let parse_impl = quote! {
            pub fn parse_impl(
                &self,
                #state: &#crate_name::ParserState<char>,
                #depends_decl
            ) -> Result<#ret_ty, ::parse_it::Error> {
                let #curr = self;
                #parser
            }
        };

        let depends_use = self.depends.iter().map(|(d, _)| d.as_ident());
        let memo_decl = match self.memo {
            MemoKind::None => quote! {},
            MemoKind::Memorize => quote! { memo: #crate_name::Memo<#ret_ty> },
            MemoKind::LeftRec => quote! { memo: #crate_name::Memo<::std::option::Option<#ret_ty>> },
        };
        let memo_func = match self.memo {
            MemoKind::None => quote! { self.parse_impl(#state, #(#depends_use),*)},
            MemoKind::Memorize => {
                quote! { #crate_name::memorize(#state, &self.memo, |state| self.parse_impl(state, #(#depends_use),*)) }
            }
            MemoKind::LeftRec => {
                quote! { #crate_name::left_rec(#state, &self.memo, |state| self.parse_impl(state, #(#depends_use),*)) }
            }
        };
        let parse_memo = quote! {
            pub fn parse_memo(
                &self,
                #state: &#crate_name::ParserState<char>,
                #depends_decl
            ) -> Result<#ret_ty, ::parse_it::Error> {
                #memo_func
            }
        };

        Ok(quote! {
            #[derive(Default)]
            pub struct #name {
                #memo_decl
            }

            impl #name {
                #parse_impl
                #parse_memo
            }
        })
    }
}

impl Parsing {
    pub fn expand(self, state_token: StateToken) -> Result<TokenStream, TokenStream> {
        let mut result = TokenStream::new();
        let state = state_token.to_ident();
        let value = self.result();
        for (value, op) in self.into_iter() {
            let value = value.to_ident();
            let op = match op {
                ParseOp::Just(c) => quote! { let #value = #state.parse(#c); },
                ParseOp::Call { parser, depends } => {
                    let parser = parser.as_ident();
                    let depends = depends.iter().map(|d| d.as_ident());
                    quote! { let #value = #parser.parse_memo(#state, #(#depends),*); }
                }
                ParseOp::Map { parser, cap, expr } => {
                    let parser = parser.to_ident();
                    let capture = cap.to_pat()?;
                    quote! { let #value = #parser.map(|#capture| #expr); }
                }
                ParseOp::Then { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token)?;
                    quote! {
                        let #value = match #prev {
                            Ok(v1) => #next.map(|v2| (v1, v2)),
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::ThenIgnore { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token)?;
                    quote! {
                        let #value = match #prev {
                            Ok(v) => #next.map(|_| v),
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::IgnoreThen { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token)?;
                    quote! {
                        let #value = match #prev {
                            Ok(_) => #next,
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::Repeat { parser, at_least } => {
                    let parser = parser.expand(state_token)?;
                    quote! {
                        let mut results = vec![];
                        while let Ok(value) = #parser {
                            results.push(value);
                        }
                        let #value = if results.len() >= #at_least {
                            Ok(results)
                        } else {
                            Err(#state.error())
                        };
                    }
                }
                ParseOp::Optional { parser } => {
                    let parser = parser.expand(state_token)?;
                    quote! { let #value = #parser.ok(); }
                }
                ParseOp::Recovery { parser } => {
                    let fork_token = state_token.fork();
                    let fork = fork_token.to_ident();
                    let parser = parser.expand(fork_token)?;
                    quote! {
                        let #fork = &#state.fork();
                        let value = #parser;
                        let #value = value.inspect(|_| #state.advance_to(&#fork));
                    }
                }
                ParseOp::Ignore { parser } => {
                    let parser = parser.to_ident();
                    quote! { let #value = #parser.map(|_| ()); }
                }
                ParseOp::Not { parser } => {
                    let parser = parser.to_ident();
                    quote! {
                        let #value = if let Ok(value) = #parser {
                            Err(#state.error())
                        } else {
                            Ok(())
                        };
                    }
                }
                ParseOp::Choice { parsers } => {
                    let first = parsers.0.to_ident();
                    let rest = parsers
                        .1
                        .into_iter()
                        .map(|p| p.expand(state_token))
                        .collect::<Result<Vec<_>, _>>()?;
                    quote! {
                        let #value = if let Ok(value) = #first {
                            Ok(value)
                        } #(
                            else if let Ok(value) = #rest {
                                Ok(value)
                            }
                        )* else {
                            Err(#state.error())
                        };
                    }
                }
            };
            result.extend(op);
        }
        let value = value.to_ident();
        Ok(quote! {{
            #result
            #value
        }})
    }
}

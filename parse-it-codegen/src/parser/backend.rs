use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::parser::middle::{Capture, MemoKind, Middle, ParseOp, ParserImpl, Parsing, Value};

pub struct Context {
    crate_name: TokenStream,
    debug: bool,
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
                        Err(quote_spanned! { p.span() => compile_error!("must be an ident here"); })
                    }
                },
            },
            Capture::Tuple(c1, c2) => {
                let c1 = c1.to_pat()?;
                let c2 = c2.to_pat()?;
                Ok(quote! { (#c1, #c2) })
            }
            Capture::TupleVec(c) => Ok(quote! { (#(#c),*) }),
        }
    }
}

impl Middle {
    pub fn expand(self) -> Result<TokenStream, TokenStream> {
        let mut result = TokenStream::new();
        let ctx = Context {
            crate_name: self.crate_name,
            debug: self.debug,
        };

        for parser in self.parsers {
            result.extend(parser.expand(&ctx)?);
        }

        let mod_name = self.mod_name;
        let attrs = self.attrs;
        let items = self.items;
        Ok(quote! {
            #[allow(non_snake_case, unused_parens, clippy::double_parens, clippy::redundant_closure)]
            #(#attrs)*
            mod #mod_name {
                #(#items)*
                #result
            }
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
        let depends_use = self.depends.iter().map(|(d, _)| d.as_ident());
        let depends_use = quote! { #(#depends_use),* };
        let depends_def = self.depends.iter().map(|(d, ty)| {
            let d = d.as_ident();
            quote! { let #d = &#ty::default(); }
        });
        let depends_def = quote! { #(#depends_def)* };

        let state_token = StateToken::new();
        let state = state_token.to_ident();
        let parser = self.parser.expand(state_token, ctx)?;
        let parse_impl = quote! {
            fn parse_impl(
                &self,
                #state: &mut #crate_name::ParserState<Lexer>,
                #depends_decl
            ) -> Result<#ret_ty, ::parse_it::Error> {
                let #curr = self;
                #parser
            }
        };

        let cursor_ty = quote! { #crate_name::Cursor };
        let memo_decl = match self.memo {
            MemoKind::None => quote! {},
            MemoKind::Memorize => quote! { memo: #crate_name::Memo<#cursor_ty, #ret_ty> },
            MemoKind::LeftRec => {
                quote! { memo: #crate_name::Memo<#cursor_ty, ::std::option::Option<#ret_ty>> }
            }
        };
        let memo_func = match self.memo {
            MemoKind::None => quote! { self.parse_impl(#state, #depends_use)},
            MemoKind::Memorize => {
                quote! { #crate_name::memorize(#state, &self.memo, |state| self.parse_impl(state, #depends_use)) }
            }
            MemoKind::LeftRec => {
                quote! { #crate_name::left_rec(#state, &self.memo, |state| self.parse_impl(state, #depends_use)) }
            }
        };
        let debug_push = if ctx.debug {
            quote! { #state.push(Self::NAME); }
        } else {
            quote! {}
        };
        let debug_print = if ctx.debug {
            quote! { eprintln!("{}: {:?}", Self::NAME, result); }
        } else {
            quote! {}
        };
        let debug_pop = if ctx.debug {
            quote! { #state.pop(); }
        } else {
            quote! {}
        };
        let parse_memo = quote! {
            fn parse_memo(
                &self,
                #state: &mut #crate_name::ParserState<Lexer>,
                #depends_decl
            ) -> Result<#ret_ty, ::parse_it::Error> {
                #debug_push
                let result = #memo_func;
                #debug_print
                #debug_pop
                result
            }
        };

        let name_str = name.to_string();
        let vis = self.vis;

        Ok(quote! {
            #[derive(Debug, Default)]
            #vis struct #name {
                #memo_decl
            }

            impl #name {
                const NAME: &'static str = #name_str;

                #parse_impl
                #parse_memo
            }

            impl #crate_name::ParseIt for #name {
                type Lexer = Lexer;
                type Output = #ret_ty;

                fn parse_stream<'a>(
                    &self,
                    state: &mut #crate_name::ParserState<'a, Lexer>
                ) -> Result<#ret_ty, ::parse_it::Error> {
                    #depends_def
                    let result = self.parse_memo(state, #depends_use);
                    result
                }
            }
        })
    }
}

impl Parsing {
    pub fn expand(
        self,
        state_token: StateToken,
        ctx: &Context,
    ) -> Result<TokenStream, TokenStream> {
        let mut result = TokenStream::new();
        let span = self.span;
        let state = state_token.to_ident();
        let value = self.result();
        let crate_name = &ctx.crate_name;
        for (value, op) in self.into_iter() {
            let value = value.to_ident();
            let op = match op {
                ParseOp::Just(c) => {
                    let result = match c {
                        syn::Lit::Str(lit_str) => {
                            quote_spanned! { span => #state.parse_str(#lit_str) }
                        }
                        syn::Lit::Char(lit_char) => {
                            quote_spanned! { span => #state.parse_char(#lit_char) }
                        }
                        _ => {
                            let e = "Unsupported literal";
                            return Err(quote_spanned! { c.span() => compile_error!(#e); });
                        }
                    };
                    quote_spanned! { span => let #value = #result; }
                }
                ParseOp::JustType(ty) => quote_spanned! { span =>
                    let #value = #state.parse_literal_type::<#ty>();
                },
                ParseOp::Pat(p, caps) => quote_spanned! { span =>
                    let #value = #state.parse_with(|tt| match tt {
                        #p => Some((#(#caps),*)),
                        _ => None,
                    });
                },
                ParseOp::Call { parser, depends } => {
                    let parser = parser.as_ident();
                    let depends = depends.iter().map(|d| d.as_ident());
                    quote_spanned! { span => let #value = #parser.parse_memo(#state, #(#depends),*); }
                }
                ParseOp::Map { parser, cap, expr } => {
                    let parser = parser.to_ident();
                    let capture = cap.to_pat()?;
                    quote_spanned! { span => let #value = #parser.map(|#capture| #expr); }
                }
                ParseOp::Then { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token, ctx)?;
                    quote_spanned! { span =>
                        let #value = match #prev {
                            Ok(v1) => #next.map(|v2| (v1, v2)),
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::ThenIgnore { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token, ctx)?;
                    quote_spanned! { span =>
                        let #value = match #prev {
                            Ok(v) => #next.map(|_| v),
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::IgnoreThen { prev, next } => {
                    let prev = prev.to_ident();
                    let next = next.expand(state_token, ctx)?;
                    quote_spanned! { span =>
                        let #value = match #prev {
                            Ok(_) => #next,
                            Err(e) => Err(e),
                        };
                    }
                }
                ParseOp::Repeat { parser, at_least } => {
                    let fork_token = state_token.fork();
                    let fork = fork_token.to_ident();
                    let parser = parser.expand(fork_token, ctx)?;
                    let repeat = quote_spanned! { span =>
                        let #fork = &mut #state.fork();
                        let mut results = vec![];
                        while let Ok(value) = #parser {
                            #state.advance_to(&#fork);
                            results.push(value);
                        }
                    };
                    if at_least == 0 {
                        quote_spanned! { span =>
                            #repeat
                            let #value: ::std::result::Result<_, #crate_name::Error> = Ok(results);
                        }
                    } else {
                        quote_spanned! { span =>
                            #repeat
                            let #value = if results.len() >= #at_least {
                                Ok(results)
                            } else {
                                Err(#state.error())
                            };
                        }
                    }
                }
                ParseOp::Optional { parser } => {
                    let parser = parser.expand(state_token, ctx)?;
                    quote_spanned! { span => let #value = #parser.ok(); }
                }
                ParseOp::LookAhead { parser } => {
                    let fork_token = state_token.fork();
                    let fork = fork_token.to_ident();
                    let parser = parser.expand(fork_token, ctx)?;
                    quote_spanned! { span =>
                        let #fork = &mut #state.fork();
                        let #value = #parser.map(|_| ());
                    }
                }
                ParseOp::LookAheadNot { parser } => {
                    let fork_token = state_token.fork();
                    let fork = fork_token.to_ident();
                    let parser = parser.expand(fork_token, ctx)?;
                    quote_spanned! { span =>
                        let #fork = &mut #state.fork();
                        let #value = if let Ok(value) = #parser {
                            Err(#state.error())
                        } else {
                            Ok(())
                        };
                    }
                }
                ParseOp::Choice { parsers } => {
                    let fork_token = state_token.fork();
                    let fork = fork_token.to_ident();
                    let parsers = parsers
                        .into_iter()
                        .map(|p| p.expand(fork_token, ctx))
                        .collect::<Result<Vec<_>, _>>()?;
                    quote_spanned! { span =>
                        let mut fork;
                        let mut #fork;
                        let #value = #(if let Ok(value) = {
                            fork = #state.fork();
                            #fork = &mut fork;
                            #parsers
                        } {
                            #state.advance_to(#fork);
                            Ok(value)
                        } else)*{
                            Err(#state.error())
                        };
                    }
                }
            };
            result.extend(op);
        }
        let value = value.to_ident();
        Ok(quote_spanned! { span => {
            #result
            #value
        }})
    }
}

use std::rc::Rc;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{punctuated::Punctuated, visit_mut::VisitMut, Token};

pub struct RewriteSelfVisitor {
    pub parse_macros: Rc<Vec<syn::Path>>,
    /// replace `self` with this ident
    pub self_ident: syn::Ident,
    /// whether `self` is referred
    pub referred_self: bool,
}

impl RewriteSelfVisitor {
    pub fn new(parse_macros: Rc<Vec<syn::Path>>) -> Self {
        Self {
            parse_macros,
            self_ident: format_ident!("r#__self", span = Span::call_site()),
            referred_self: false,
        }
    }
}

impl VisitMut for RewriteSelfVisitor {
    fn visit_ident_mut(&mut self, i: &mut proc_macro2::Ident) {
        if i == "self" {
            let span = i.span();
            *i = self.self_ident.clone();
            i.set_span(span);
            self.referred_self = true;
        }
    }

    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        if self.parse_macros.contains(&m.path) {
            struct MacroArgs(pub Vec<syn::Expr>);
            impl syn::parse::Parse for MacroArgs {
                fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                    let args = Punctuated::<syn::Expr, Token![,]>::parse_terminated(input)?;
                    Ok(Self(args.into_iter().collect()))
                }
            }

            if let Ok(MacroArgs(mut args)) = syn::parse2::<MacroArgs>(m.tokens.clone()) {
                for expr in args.iter_mut() {
                    self.visit_expr_mut(expr);
                }
                m.tokens = TokenStream::new();
                for expr in args {
                    match expr {
                        syn::Expr::Lit(syn::ExprLit { attrs, lit }) if attrs.is_empty() => {
                            m.tokens.extend(lit.to_token_stream());
                        }
                        _ => {
                            m.tokens.extend(quote! { #expr });
                        }
                    }
                    m.tokens.extend(quote! {,});
                }
            }
        }
    }
}

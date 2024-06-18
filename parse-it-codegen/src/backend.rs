use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;

use crate::{
    middle::{Capture, Middle, Value},
    Hasher,
};

impl Value {
    pub fn to_ident(self) -> syn::Ident {
        let val = self.0;
        format_ident!("r#__{}", val)
    }
}

impl Capture {
    pub fn to_pat(&self) -> TokenStream {
        match self {
            Capture::Loud | Capture::Slient => quote! { _ },
            Capture::Named(p, c) => match p.as_ref() {
                syn::Pat::Ident(_) => {
                    let c = c.to_pat();
                    quote! { #p @ #c }
                }
                _ => match c.as_ref() {
                    Capture::Loud | Capture::Slient => quote! { #p },
                    _ => quote_spanned! { p.span() => compile_error!("must be an ident here") },
                },
            },
            Capture::Tuple(c1, c2) => {
                let c1 = c1.to_pat();
                let c2 = c2.to_pat();
                quote! { (#c1, #c2) }
            }
        }
    }
}

impl Middle {
    pub fn expand(self) -> TokenStream {
        let arena = format_ident!("r#__arena");
        let arena_size = self
            .values()
            .filter(|(_, v)| matches!(v.kind(), crate::middle::ValueKind::Declare))
            .count();
        let mut result = quote! {
            let #arena = ::parse_it::__internal::new_arena::<#arena_size>();
        };
        let last_use = self.analyze_last_use();

        for (val, data) in self.values() {
            let is_last_use = |v: Value| last_use.get(&v) == Some(val);
            let use_ident = |v: Value| {
                if is_last_use(v) {
                    let v = v.to_ident();
                    quote! { #v }
                } else {
                    let v = v.to_ident();
                    quote! { #v.clone() }
                }
            };
            let val = val.to_ident();
            match data.kind() {
                crate::middle::ValueKind::Declare => result.extend(quote! {
                    let #val = ::parse_it::__internal::declare_recursive(&#arena);
                }),
                crate::middle::ValueKind::Define { decl, value } => {
                    let decl = use_ident(*decl);
                    let value = use_ident(*value);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::define_recursive(#decl, #value);
                    })
                }
                crate::middle::ValueKind::Just(c) => result.extend(quote! {
                    let #val = ::parse_it::__internal::just_parser(#c);
                }),
                crate::middle::ValueKind::Map(v, c, t, e) => {
                    let v = use_ident(*v);
                    let c = c.to_pat();
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::map_parser::<#t, _>(#v, |#c| #e);
                    })
                }
                crate::middle::ValueKind::Memorize(v) => {
                    let v = use_ident(*v);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::memorize_parser(#v);
                    })
                }
                crate::middle::ValueKind::LeftRec(v) => {
                    let v = use_ident(*v);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::left_rec_parser(#v);
                    })
                }
                crate::middle::ValueKind::Then(v1, v2) => {
                    let v1 = use_ident(*v1);
                    let v2 = use_ident(*v2);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::then_parser(#v1, #v2);
                    })
                }
                crate::middle::ValueKind::ThenIgnore(v1, v2) => {
                    let v1 = use_ident(*v1);
                    let v2 = use_ident(*v2);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::then_ignore_parser(#v1, #v2);
                    })
                }
                crate::middle::ValueKind::IgnoreThen(v1, v2) => {
                    let v1 = use_ident(*v1);
                    let v2 = use_ident(*v2);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::ignore_then_parser(#v1, #v2);
                    })
                }
                crate::middle::ValueKind::Choice(vs) => {
                    let vs = vs.iter().copied().map(use_ident);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::choice_parser((#(#vs),*));
                    })
                }
                crate::middle::ValueKind::Repeat(v) => {
                    let v = use_ident(*v);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::repeat_parser(#v);
                    })
                }
                crate::middle::ValueKind::Repeat1(v) => {
                    let v = use_ident(*v);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::repeat1_parser(#v);
                    })
                }
                crate::middle::ValueKind::OrNot(v) => {
                    let v = use_ident(*v);
                    result.extend(quote! {
                        let #val = ::parse_it::__internal::or_not_parser(#v);
                    })
                }
            }
        }

        let results = self.results.into_iter().map(|v| {
            let v = v.to_ident();
            quote! { ::parse_it::__internal::into_parser(#v, &#arena) }
        });
        result.extend(quote! {
            (#(#results),*)
        });

        quote! {{ #result }}
    }

    fn analyze_last_use(&self) -> HashMap<Value, Value, Hasher> {
        let mut last_use = HashMap::default();

        for (&val, data) in self.values() {
            for v in data.kind().uses() {
                last_use.insert(v, val);
            }
        }

        // last use is after all values
        for res in &self.results {
            last_use.remove(res);
        }

        last_use
    }
}

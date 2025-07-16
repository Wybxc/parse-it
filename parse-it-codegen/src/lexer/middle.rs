use proc_macro2::TokenStream;

pub struct Rule {
    pub pattern: syn::LitStr,
    pub action: Vec<(syn::Expr, Option<syn::Type>)>,
}

pub struct LexerImpl {
    pub name: syn::Ident,
    pub rules: Vec<Rule>,
    pub vis: syn::Visibility,
    pub ret_ty: Option<syn::Type>,
}

pub struct Middle {
    pub attrs: Vec<syn::Attribute>,
    pub crate_name: TokenStream,
    pub mod_name: syn::Ident,
    pub items: Vec<syn::Item>,
    pub lexers: Vec<LexerImpl>,
    pub debug: bool,
}

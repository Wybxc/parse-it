use proc_macro2::TokenStream;

#[derive(Debug, Clone)]
pub struct Action {
    pub action: syn::Expr,
    pub ret_ty: Option<syn::Type>,
    /// replace `self` with this ident
    pub self_ident: syn::Ident,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: syn::LitStr,
    pub actions: (Action, Vec<Action>),
}

#[derive(Debug, Clone)]
pub struct LexerImpl {
    pub name: syn::Ident,
    pub rules: Vec<Rule>,
    pub vis: syn::Visibility,
    pub inputs: Vec<syn::PatType>,
    pub ret_ty: Option<syn::Type>,
}

#[derive(Debug, Clone)]
pub struct Middle {
    pub attrs: Vec<syn::Attribute>,
    pub crate_name: TokenStream,
    pub mod_name: syn::Ident,
    pub items: Vec<syn::Item>,
    pub lexers: Vec<LexerImpl>,
    pub debug: bool,
}

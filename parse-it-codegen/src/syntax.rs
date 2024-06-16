#[derive(Debug)]
pub struct ParseIt {
    pub parsers: Vec<Parser>,
    pub results: Vec<syn::Ident>,
}

/// ```text
/// Parser ::= Name '->' Type '{' Rule* '}'
/// ```
#[derive(Debug)]
pub struct Parser {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub rules: Vec<Rule>,
}

/// ```text
/// Rule ::= Production '=>' Expr
/// ```
#[derive(Debug)]
pub struct Rule {
    pub production: Production,
    pub action: syn::Expr,
}

/// ```text
/// Production ::= Part+
/// Part ::= (Pat ':')? '@'? Atom ('*' | '+' | '?')?
/// Atom ::= '(' Production ')'
///        | '[' Production ('|' Production)* ']'
///        | Terminal
///        | NonTerminal
/// ```
#[derive(Debug)]
pub struct Production {
    /// non-empty: (first, rest)
    pub parts: (Part, Vec<Part>),
}

#[derive(Debug)]
pub struct Part {
    pub capture: Capture,
    pub part: Atom,
}

#[derive(Debug)]
pub enum Capture {
    Named(Box<syn::Pat>),
    Loud,
    NotSpecified,
}

#[derive(Debug)]
pub enum Atom {
    Terminal(syn::Lit),
    NonTerminal(syn::Ident),
    Sub(Box<Production>),
    Choice(Vec<Production>),
    Repeat(Box<Atom>),
    Repeat1(Box<Atom>),
    Optional(Box<Atom>),
}

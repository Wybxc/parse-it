#[derive(Debug)]
pub struct ParseIt {
    pub crate_name: Option<syn::Path>,
    pub mod_name: syn::Ident,
    pub parsers: Vec<Parser>,
}

/// ```text
/// Parser ::= Vis Name '->' Type '{' Rule+ '}'
/// ```
#[derive(Debug)]
pub struct Parser {
    pub vis: syn::Visibility,
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub rules: (Rule, Vec<Rule>),
}

impl Parser {
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        std::iter::once(&self.rules.0).chain(self.rules.1.iter())
    }
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
/// Part ::= (Pat ':')? '@'? ('&' | '!')? Atom ('*' | '+' | '?')?
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

impl Production {
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        std::iter::once(&self.parts.0).chain(self.parts.1.iter())
    }
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
    Choice(Box<Production>, Vec<Production>),
    Repeat(Box<Atom>),
    Repeat1(Box<Atom>),
    Optional(Box<Atom>),
    LookAhead(Box<Atom>),
    LookAheadNot(Box<Atom>),
}

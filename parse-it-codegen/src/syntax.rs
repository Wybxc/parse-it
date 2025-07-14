use std::rc::Rc;

use syn::{parse::discouraged::Speculative, punctuated::Punctuated, Attribute, Token};

#[derive(Debug)]
pub struct ParserConfig {
    pub crate_name: Option<syn::Path>,
    pub parse_macros: Rc<Vec<syn::Path>>,
    pub debug: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            crate_name: None,
            parse_macros: Rc::new(vec![
                syn::parse_quote! { print },
                syn::parse_quote! { println },
                syn::parse_quote! { eprint },
                syn::parse_quote! { eprintln },
                syn::parse_quote! { format },
                syn::parse_quote! { dbg },
            ]),
            debug: false,
        }
    }
}

#[derive(Debug)]
pub struct ParseIt {
    pub mods: Vec<Mod>,
}

impl syn::parse::Parse for ParseIt {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut mods = vec![];
        while !input.is_empty() {
            let fork = input.fork();
            let mut attrs = input.call(syn::Attribute::parse_outer)?;

            input.parse::<Token![mod]>()?;
            let mod_name = input.parse::<syn::Ident>()?;

            let content;
            syn::braced!(content in input);

            #[derive(Clone, Copy, PartialEq, Eq)]
            enum ModType {
                Parser,
                Lexer,
                Common,
            }
            let mut mod_types = vec![];
            attrs.retain(|attr| {
                if attr.path().is_ident("parser") {
                    mod_types.push(ModType::Parser);
                    return false;
                } else if attr.path().is_ident("lexer") {
                    mod_types.push(ModType::Lexer);
                    return false;
                }
                true
            });
            let mod_type = if mod_types.is_empty() {
                ModType::Common
            } else if mod_types.len() == 1 {
                mod_types[0]
            } else {
                return Err(syn::Error::new_spanned(
                    mod_name,
                    "module can only be marked as parser or lexer, not both",
                ));
            };
            match mod_type {
                ModType::Parser => {
                    let parser_mod = ParserMod::parse(attrs, mod_name, &content)?;
                    mods.push(Mod::Parser(parser_mod));
                }
                ModType::Lexer => todo!(),
                ModType::Common => {
                    mods.push(Mod::Common(fork.parse::<syn::ItemMod>()?));
                    input.advance_to(&fork);
                }
            }
        }
        Ok(Self { mods })
    }
}

#[derive(Debug)]
pub enum Mod {
    Parser(ParserMod),
    Common(syn::ItemMod),
}

#[derive(Debug)]
pub struct ParserMod {
    pub attrs: Vec<syn::Attribute>,
    pub mod_name: syn::Ident,
    pub items: Vec<syn::Item>,
    pub parsers: Vec<Parser>,
    pub config: ParserConfig,
}

impl ParserMod {
    fn parse(
        attrs: Vec<Attribute>,
        mod_name: syn::Ident,
        content: syn::parse::ParseStream,
    ) -> syn::Result<Self> {
        let mut config = ParserConfig::default();
        let mut common_attrs = vec![];
        for attr in attrs {
            if attr.path().is_ident("parse_it") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("crate") {
                        let value = meta.value()?;
                        let value = value.parse::<syn::LitStr>()?;
                        config.crate_name = Some(value.parse().map_err(|_| {
                            syn::Error::new_spanned(value, "expected a valid path")
                        })?);
                    } else if meta.path.is_ident("parse_macros") {
                        let value = meta.value()?;
                        let value = value.parse::<syn::LitStr>()?;
                        config.parse_macros = Rc::new(
                            value
                                .parse_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                                .map_err(|_| {
                                    syn::Error::new_spanned(
                                        value,
                                        "expected a list of paths separated by commas",
                                    )
                                })?
                                .into_iter()
                                .collect(),
                        );
                    } else if meta.path.is_ident("debug") {
                        let value = meta.value()?;
                        let value = value.parse::<syn::LitBool>()?;
                        config.debug = value.value;
                    } else {
                        Err(syn::Error::new_spanned(meta.path, "unknown attribute"))?
                    }
                    Ok(())
                })?;
            } else {
                common_attrs.push(attr);
            }
        }

        let mut parsers = vec![];
        let mut items = vec![];
        while !content.is_empty() {
            let fork = content.fork();
            if let Ok(parser) = fork.parse::<Parser>() {
                content.advance_to(&fork);
                parsers.push(parser);
            } else {
                let item = content.parse::<syn::Item>()?;
                items.push(item);
            }
        }
        Ok(Self {
            attrs: common_attrs,
            items,
            mod_name,
            parsers,
            config,
        })
    }
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

impl syn::parse::Parse for Parser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let vis = input.parse::<syn::Visibility>()?;
        let name = input.parse::<syn::Ident>()?;
        input.parse::<Token![->]>()?;
        let ty = input.parse::<syn::Type>()?;

        let content;
        syn::braced!(content in input);

        let first_rule = content.parse::<Rule>()?;
        let mut rules = vec![];
        while !content.is_empty() {
            let rule = content.parse::<Rule>()?;
            rules.push(rule);
        }
        let rules = (first_rule, rules);

        Ok(Parser {
            vis,
            name,
            ty,
            rules,
        })
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

impl syn::parse::Parse for Rule {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let production = input.parse::<Production>()?;
        input.parse::<Token![=>]>()?;
        let action = input.parse::<syn::Expr>()?;
        if (requires_comma_to_be_match_arm(&action) && !input.is_empty()) || input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Rule { production, action })
    }
}

/// ```text
/// Production ::= Part+
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

impl syn::parse::Parse for Production {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let first_part = input.parse::<Part>()?;
        let mut rest_parts = Vec::new();
        while !input.peek(Token![=>]) && !input.peek(Token![|]) && !input.is_empty() {
            // Production ::= Part+
            rest_parts.push(input.parse::<Part>()?);
        }

        let parts = (first_part, rest_parts);
        Ok(Production { parts })
    }
}

#[derive(Debug)]
pub enum Capture {
    Named(Box<syn::Pat>),
    Loud,
    NotSpecified,
}

/// ```text
/// Part ::= (Pat ':')? '@'? ('&' | '!')? Atom ('*' | '+' | '?')?
/// ```
#[derive(Debug)]
pub struct Part {
    pub capture: Capture,
    pub part: Atom,
}

impl syn::parse::Parse for Part {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fork = input.fork();
        let capture = if let Ok(pat) = fork
            .call(syn::Pat::parse_single)
            .and_then(|pat| fork.parse::<Token![:]>().map(|_| pat))
        {
            // Choice ::= Pat ':' Atom ...
            input.advance_to(&fork);
            Some(pat)
        } else {
            None
        };

        let non_slient = if input.peek(Token![@]) {
            // Choice ::= ... '@' ...
            input.parse::<Token![@]>()?;
            true
        } else {
            false
        };

        let atom = input.parse::<Atom>()?;
        let part = if input.peek(Token![*]) {
            // Choice ::= ... Atom '*'
            input.parse::<Token![*]>()?;
            Atom::Repeat(Box::new(atom))
        } else if input.peek(Token![+]) {
            // Choice ::= ... Atom '+'
            input.parse::<Token![+]>()?;
            Atom::Repeat1(Box::new(atom))
        } else if input.peek(Token![?]) {
            // Choice ::= ... Atom '?'
            input.parse::<Token![?]>()?;
            Atom::Optional(Box::new(atom))
        } else {
            atom
        };

        let capture = if let Some(capture) = capture {
            Capture::Named(Box::new(capture))
        } else if non_slient {
            Capture::Loud
        } else {
            Capture::NotSpecified
        };

        Ok(Part { capture, part })
    }
}

/// ```text
/// Atom ::= '(' Production ')'
///        | '[' Production ('|' Production)* ']'
///        | Terminal
///        | NonTerminal
/// ```
#[derive(Debug)]
pub enum Atom {
    Terminal(syn::Lit),
    PatTerminal(syn::Pat),
    NonTerminal(syn::Ident),
    Sub(Box<Production>),
    Choice(Box<Production>, Vec<Production>),
    Repeat(Box<Atom>),
    Repeat1(Box<Atom>),
    Optional(Box<Atom>),
    LookAhead(Box<Atom>),
    LookAheadNot(Box<Atom>),
}

impl syn::parse::Parse for Atom {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let atom = if lookahead.peek(syn::token::Paren) {
            // Atom ::= '(' Production ')'
            let content;
            syn::parenthesized!(content in input);
            Atom::Sub(Box::new(content.parse()?))
        } else if lookahead.peek(syn::token::Bracket) {
            // Atom ::= '[' Production ('|' Production)* ']'
            let content;
            syn::bracketed!(content in input);
            let mut choices = content
                .parse_terminated(Production::parse, Token![|])?
                .into_iter();
            let first_choice = choices
                .next()
                .ok_or_else(|| content.error("expected at least one choice"))?;
            Atom::Choice(Box::new(first_choice), choices.collect())
        } else if lookahead.peek(syn::Lit) {
            // Atom ::= Terminal
            Atom::Terminal(input.parse()?)
        } else if lookahead.peek(syn::Ident) {
            let fork = input.fork();
            if let Ok(pat) = fork.call(syn::Pat::parse_single) {
                if matches!(&pat, syn::Pat::Ident(_)) {
                    // Atom ::= NonTerminal
                    Atom::NonTerminal(input.parse()?)
                } else {
                    // Atom ::= PatTerminal
                    input.advance_to(&fork);
                    Atom::PatTerminal(pat)
                }
            } else {
                Err(lookahead.error())?
            }
        } else {
            Err(lookahead.error())?
        };

        Ok(atom)
    }
}

fn requires_comma_to_be_match_arm(expr: &syn::Expr) -> bool {
    use syn::Expr;
    !matches!(
        expr,
        Expr::If(_)
            | Expr::Match(_)
            | Expr::Block(_)
            | Expr::Unsafe(_)
            | Expr::While(_)
            | Expr::Loop(_)
            | Expr::ForLoop(_)
            | Expr::TryBlock(_)
            | Expr::Const(_)
    )
}

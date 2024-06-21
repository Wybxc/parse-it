use parse_it_codegen::syntax::{Atom, Capture, ParseIt, Parser, Part, Production, Rule};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::{Result, Token};

fn parse(input: ParseStream) -> Result<ParseIt> {
    let mut parsers = vec![];
    let mut results = vec![];
    while !input.is_empty() {
        if input.peek(Token![return]) {
            input.parse::<Token![return]>()?;
            if input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in input);
                results.extend(content.parse_terminated(syn::Ident::parse, Token![,])?);
            } else {
                results.push(input.parse::<syn::Ident>()?);
            }
            input.parse::<Token![;]>()?;
            continue;
        }
        parsers.push(input.call(parse_parser)?);
    }
    Ok(ParseIt {
        crate_name: None, // TODO: parse crate name
        parsers,
        results,
    })
}

fn parse_parser(input: ParseStream) -> Result<Parser> {
    let name = input.parse::<syn::Ident>()?;
    input.parse::<Token![->]>()?;
    let ty = input.parse::<syn::Type>()?;

    let content;
    syn::braced!(content in input);

    let first_rule = content.call(parse_rule)?;
    let mut rules = vec![];
    while !content.is_empty() {
        let rule = content.call(parse_rule)?;
        rules.push(rule);
    }
    let rules = (first_rule, rules);

    Ok(Parser { name, ty, rules })
}

fn parse_rule(input: ParseStream) -> Result<Rule> {
    let production = input.call(parse_production)?;
    input.parse::<Token![=>]>()?;
    let action = input.parse::<syn::Expr>()?;
    if (requires_comma_to_be_match_arm(&action) && !input.is_empty()) || input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }
    Ok(Rule { production, action })
}

fn parse_production(input: ParseStream) -> Result<Production> {
    let first_part = input.call(parse_part)?;
    let mut rest_parts = Vec::new();
    while !input.peek(Token![=>]) && !input.peek(Token![|]) && !input.is_empty() {
        // Production ::= Part+
        rest_parts.push(input.call(parse_part)?);
    }

    let parts = (first_part, rest_parts);
    Ok(Production { parts })
}

fn parse_part(input: ParseStream) -> Result<Part> {
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

    let atom = input.call(parse_atom)?;
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

fn parse_atom(input: ParseStream) -> Result<Atom> {
    let lookahead = input.lookahead1();
    let atom = if lookahead.peek(syn::token::Paren) {
        // Atom ::= '(' Production ')'
        let content;
        syn::parenthesized!(content in input);
        Atom::Sub(Box::new(content.call(parse_production)?))
    } else if lookahead.peek(syn::token::Bracket) {
        // Atom ::= '[' Production ('|' Production)* ']'
        let content;
        syn::bracketed!(content in input);
        let mut choices = content
            .parse_terminated(parse_production, Token![|])?
            .into_iter();
        let first_choice = choices
            .next()
            .ok_or_else(|| content.error("expected at least one choice"))?;
        Atom::Choice(Box::new(first_choice), choices.collect())
    } else if lookahead.peek(syn::Lit) {
        // Atom ::= Terminal
        Atom::Terminal(input.parse()?)
    } else if lookahead.peek(syn::Ident) {
        // Atom ::= NonTerminal
        Atom::NonTerminal(input.parse()?)
    } else {
        return Err(lookahead.error());
    };

    Ok(atom)
}

#[proc_macro]
pub fn parse_it(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input with parse);
    let middle = match input.compile() {
        Ok(middle) => middle,
        Err(msg) => return msg.into(),
    };
    match middle.expand() {
        Ok(expanded) => expanded.into(),
        Err(msg) => msg.into(),
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

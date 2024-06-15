use parse_it_codegen::syntax::{Atom, Capture, ParseIt, Parser, Part, Production, Rule};
use syn::{
    parse::{discouraged::Speculative, Parse, ParseStream},
    Result, Token,
};

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
    Ok(ParseIt { parsers, results })
}

fn parse_parser(input: ParseStream) -> Result<Parser> {
    let name = input.parse::<syn::Ident>()?;
    input.parse::<Token![->]>()?;
    let ty = input.parse::<syn::Type>()?;

    let content;
    syn::braced!(content in input);
    let rules = content.parse_terminated(parse_rule, Token![,])?;

    Ok(Parser {
        name,
        ty,
        rules: rules.into_iter().collect(),
    })
}

fn parse_rule(input: ParseStream) -> Result<Rule> {
    let production = input.call(parse_production)?;
    input.parse::<Token![=>]>()?;
    let action = input.parse::<syn::Expr>()?;
    Ok(Rule { production, action })
}

fn parse_production(input: ParseStream) -> Result<Production> {
    let mut parts = Vec::new();
    parts.push(input.call(parse_part)?);
    while !input.peek(Token![=>]) && !input.is_empty() {
        // Production ::= Part+
        parts.push(input.call(parse_part)?);
    }

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
        Capture::Named(capture)
    } else if non_slient {
        Capture::Loud
    } else {
        Capture::Slient
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
        let choices = content.parse_terminated(parse_production, Token![|])?;
        Atom::Choice(choices.into_iter().collect())
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
    println!("{:?}", input);
    proc_macro::TokenStream::new()
}

// impl syntax::ParseIt {
//     /// Expand the macro invocation into a token stream.
//     pub fn expand(&self) -> TokenStream {
//         // collect all the parsers that are referenced before they are defined
//         let mut rec_names = BTreeSet::new();
//         let mut defined = BTreeSet::new();
//         for parser in self.parsers.iter() {
//             let references = parser.references();
//             references.difference(&defined).for_each(|reference| {
//                 rec_names.insert(reference.clone());
//             });
//             defined.insert(parser.name.to_string());
//             defined.extend(rec_names.iter().cloned());
//         }

//         // expand each parser
//         let mut recursives = vec![];
//         let mut defines = vec![];
//         for parser in self.parsers.iter() {
//             let name = &parser.name;
//             if rec_names.contains(&name.to_string()) {
//                 recursives.push(quote! {
//                     let mut #name = ::parse_it::__internal::declare_recursive();
//                 });
//                 defines.push(parser.expand_rec());
//             } else {
//                 defines.push(parser.expand_norec());
//             }
//         }

//         // expand the results
//         let results = if self.results.is_empty() {
//             quote! { () }
//         } else if self.results.len() == 1 {
//             let result = &self.results[0];
//             quote! { #result }
//         } else {
//             let results = self.results.iter();
//             quote! { (#(#results),*) }
//         };

//         quote! {{
//             #(#recursives)*
//             #(#defines)*
//             #results
//         }}
//     }
// }

// impl syntax::Parser {
//     /// Collect other parsers that are referenced by this parser.
//     pub fn references(&self) -> BTreeSet<String> {
//         let mut references = BTreeSet::new();
//         for rule in &self.rules {
//             rule.collect_references(&mut references);
//         }
//         references
//     }

//     /// Expand the parser into a token stream, for a recursive parser.
//     pub fn expand_rec(&self) -> TokenStream {
//         let name = &self.name;
//         let ty = &self.ty;
//         let rules = self.expand_rules();
//         quote! {
//             #name.define(::parse_it::__internal::define_parser::<#ty, _>(#rules));
//         }
//     }

//     /// Expand the parser into a token stream, for a non-recursive parser.
//     pub fn expand_norec(&self) -> TokenStream {
//         let name = &self.name;
//         let ty = &self.ty;
//         let rules = self.expand_rules();
//         quote! {
//             let #name = ::parse_it::__internal::define_parser::<#ty, _>(#rules);
//         }
//     }

//     fn expand_rules(&self) -> TokenStream {
//         if self.rules.is_empty() {
//             quote_spanned! { self.name.span() =>
//                 compile_error!("parser must have at least one rule");
//             }
//         } else if self.rules.len() == 1 {
//             let rule = self.rules[0].expand();
//             quote! { #rule }
//         } else {
//             let rules = self.rules.iter().map(|rule| rule.expand());
//             quote! {
//                 ::parse_it::__internal::choice((#(#rules),*))
//             }
//         }
//     }
// }

// impl syntax::Rule {
//     /// Collect other parsers that are referenced by this rule.
//     pub fn collect_references(&self, refs: &mut BTreeSet<String>) {
//         self.production.collect_references(refs);
//     }

//     pub fn expand(&self) -> TokenStream {
//         quote! {}
//     }
// }

// impl syntax::Production {
//     /// Collect other parsers that are referenced by this production.
//     pub fn collect_references(&self, refs: &mut BTreeSet<String>) {
//         // match self {
//         //     syntax::Production::Terminal(_) => {}
//         //     syntax::Production::NonTerminal(ident) => {
//         //         refs.insert(ident.to_string());
//         //     }
//         //     syntax::Production::NonSlient(production)
//         //     | syntax::Production::Repeat(production)
//         //     | syntax::Production::Repeat1(production)
//         //     | syntax::Production::Optional(production) => {
//         //         production.collect_references(refs);
//         //     }
//         //     syntax::Production::Sequence(productions) | syntax::Production::Choice(productions) => {
//         //         for production in productions {
//         //             production.collect_references(refs);
//         //         }
//         //     }
//         //     syntax::Production::Capture { production, .. } => {
//         //         production.collect_references(refs);
//         //     }
//         // }
//         todo!()
//     }
// }

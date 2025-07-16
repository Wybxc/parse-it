use parse_it_codegen::syntax::{Mod, ParseIt};

#[proc_macro]
pub fn parse_it(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as ParseIt);
    let mut result = proc_macro::TokenStream::new();
    for submod in input.mods {
        match submod {
            Mod::Parser(parser_mod) => {
                let middle = match parser_mod.compile() {
                    Ok(middle) => middle,
                    Err(msg) => return msg.into(),
                };
                let tokens: proc_macro::TokenStream = match middle.expand() {
                    Ok(expanded) => expanded.into(),
                    Err(msg) => return msg.into(),
                };
                result.extend(tokens);
            }
            Mod::Lexer(lexer_mod) => {
                let middle = match lexer_mod.compile() {
                    Ok(middle) => middle,
                    Err(msg) => return msg.into(),
                };
                let tokens: proc_macro::TokenStream = match middle.expand() {
                    Ok(expanded) => expanded.into(),
                    Err(msg) => return msg.into(),
                };
                result.extend(tokens);
            }
        }
    }
    result
}

use parse_it_codegen::syntax::ParseIt;

#[proc_macro]
pub fn parse_it(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as ParseIt);
    let middle = match input.compile() {
        Ok(middle) => middle,
        Err(msg) => return msg.into(),
    };
    match middle.expand() {
        Ok(expanded) => expanded.into(),
        Err(msg) => msg.into(),
    }
}

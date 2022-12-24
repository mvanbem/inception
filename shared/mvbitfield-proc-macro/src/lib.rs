use crate::ast::parse;
use crate::decl::StructDecl;

mod ast;
mod decl;
mod types;

#[proc_macro]
pub fn mvbitfield(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    StructDecl::from_ast(parse(input.into()))
        .into_token_stream()
        .into()
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::ast::parse;
    use crate::decl::StructDecl;

    #[test]
    fn success() {
        let input = quote! {
            pub struct MyBitfield: u32 {
                pub foo: 6,
                pub flag: 1 as bool,
            }
        };

        let ast = parse(input);
        let _ = StructDecl::from_ast(ast).into_token_stream();
    }
}

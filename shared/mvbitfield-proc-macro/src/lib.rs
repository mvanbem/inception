use combine::Parser;
use combine_proc_macro::{Incomplete, Input};

use crate::ast::struct_ast;
use crate::decl::StructDecl;

mod ast;
mod decl;
mod types;

#[proc_macro]
pub fn mvbitfield(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let (ast, trailing) = struct_ast()
        .easy_parse(Input::from(input).with_lookahead(1))
        .map_err(|e| panic!("parse error: {e:#?}"))
        .unwrap();
    if let Some(e) = Incomplete::from_stream(trailing) {
        panic!("Unexpected trailing input:\n{e}");
    }

    StructDecl::from_ast(ast).into_token_stream().into()
}

#[cfg(test)]
mod tests {
    use combine::Parser;
    use combine_proc_macro::{Incomplete, Input};
    use quote::quote;

    use crate::ast::struct_ast;
    use crate::decl::StructDecl;

    #[test]
    fn success() {
        let input = quote! {
            pub struct MyBitfield: u32 {
                pub foo: 6,
                pub flag: 1 as bool,
            }
        };

        let (ast, trailing) = struct_ast()
            .easy_parse(Input::from(input).with_lookahead(1))
            .map_err(|e| panic!("parse error: {e:#?}"))
            .unwrap();
        assert!(Incomplete::from_stream(trailing).is_none());

        let _ = StructDecl::from_ast(ast).into_token_stream();
    }
}

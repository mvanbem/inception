use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::decl::{Config, StructDecl};
use crate::input::Input;

mod decl;
mod input;
mod types;

#[proc_macro]
pub fn bitfield(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitfield_impl(parse_macro_input!(tokens)).into()
}

fn bitfield_impl(input: Input) -> TokenStream {
    let cfg = Config::from_ast(&input);

    let mut results = Vec::new();
    for ast_struct in input.structs {
        results.push(
            match StructDecl::from_ast(&cfg, &ast_struct).and_then(StructDecl::into_token_stream) {
                Ok(result) => result,
                Err(e) => {
                    let name = &ast_struct.name;
                    let compile_error = e.into_compile_error();
                    quote! {
                        #compile_error
                        struct #name {}
                    }
                }
            },
        );
    }
    quote! { #(#results)* }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::bitfield_impl;

    #[test]
    fn success_zero_structs() {
        let tokens = quote! { (some::path,) };

        let _ = bitfield_impl(syn::parse2(tokens).unwrap());
    }

    #[test]
    fn success_multiple_structs() {
        let tokens = quote! {(
            ::mvbitfield,

            #[msb_first]
            pub struct MyBitfield: u32 {
                pub foo: 6,
                pub flag: 1 as bool,
                ..
            }

            #[lsb_first]
            pub struct AnotherBitfield: u16 {
                bar: 3,
                qux: 13,
            }
        )};

        let _ = bitfield_impl(syn::parse2(tokens).unwrap());
    }
}

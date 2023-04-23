use syn::parse_macro_input;

use crate::gen::bitfield_impl;

mod ast;
mod gen;
mod pack;

#[proc_macro]
pub fn bitfield(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitfield_impl(parse_macro_input!(tokens)).into()
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
            pub struct MyBitfield: 32 {
                pub foo: 6,
                pub flag: 1 as bool,
                ..
            }

            #[lsb_first]
            pub struct AnotherBitfield: 16 {
                bar: 3,
                qux: 13,
            }
        )};

        let _ = bitfield_impl(syn::parse2(tokens).unwrap());
    }
}

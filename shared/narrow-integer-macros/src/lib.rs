use proc_macro::TokenStream;

mod literal;

#[proc_macro]
pub fn lit(tokens: TokenStream) -> TokenStream {
    literal::lit(tokens.into()).into()
}

#[proc_macro_attribute]
pub fn narrow_integer_literals(attr: TokenStream, item: TokenStream) -> TokenStream {
    literal::narrow_integer_literals(attr.into(), item.into()).into()
}

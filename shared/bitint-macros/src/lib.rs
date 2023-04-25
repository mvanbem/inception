use litrs::IntegerLit;
use proc_macro2::{Group, Literal, TokenStream, TokenTree};
use quote::{format_ident, quote_spanned, ToTokens};
use syn::parse::{Parse, ParseBuffer, Parser};
use syn::{parenthesized, parse_quote, token, Error, LitInt, Path, Result, Token};

#[proc_macro]
pub fn bitint(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bitint_impl(tokens.into()).into()
}

#[proc_macro_attribute]
pub fn bitint_literals(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    bitint_literals_impl(attr.into(), item.into()).into()
}

struct BitintInput {
    _paren_token: token::Paren,
    crate_path: Path,
    _comma_token: Token![,],
    lit: LitInt,
}

impl Parse for BitintInput {
    fn parse(input: &ParseBuffer) -> Result<Self> {
        let content;
        Ok(Self {
            _paren_token: parenthesized!(content in input),
            crate_path: content.parse()?,
            _comma_token: content.parse()?,
            lit: content.parse()?,
        })
    }
}

fn bitint_impl(tokens: TokenStream) -> TokenStream {
    let input: BitintInput = match syn::parse2(tokens) {
        Ok(input) => input,
        Err(e) => return e.into_compile_error(),
    };
    match rewrite_literal(&input.crate_path, input.lit.token()) {
        RewriteResult::Rewritten(tokens) => tokens,
        RewriteResult::UnrecognizedSuffix(literal) => Error::new(
            literal.span(),
            "literal must have a suffix: 'U' followed by an integer in 1..=128",
        )
        .into_compile_error(),
        RewriteResult::ValueError(e) => e.into_compile_error(),
    }
}

enum RewriteResult {
    Rewritten(TokenStream),
    UnrecognizedSuffix(Literal),
    ValueError(Error),
}

fn rewrite_literal(crate_path: &Path, literal: Literal) -> RewriteResult {
    // Only rewrite integer literals with a recognized suffix.
    let Ok(integer_lit) = IntegerLit::try_from(literal.clone()) else {
        return RewriteResult::UnrecognizedSuffix(literal);
    };
    let Some(width) = parse_suffix(integer_lit.suffix()) else {
        return RewriteResult::UnrecognizedSuffix(literal);
    };

    // Parse the value and enforce bounds.
    let span = literal.span();
    let Some(value) = integer_lit.value::<u128>() else {
        return RewriteResult::ValueError(
            Error::new(span, "could not parse integer literal")
        );
    };
    if width < 128 {
        let max: u128 = (1 << width) - 1;
        if value > max {
            return RewriteResult::ValueError(Error::new(
                span,
                format!("integer literal value {value} out of range for U{width}"),
            ));
        }
    }

    // Build the rewritten literal.
    let type_name = format_ident!("U{width}", span = span);
    let mut new_literal = Literal::u128_unsuffixed(value);
    new_literal.set_span(span);
    RewriteResult::Rewritten(
        quote_spanned! {span=> #crate_path::types::#type_name::new_masked(#new_literal) },
    )
}

fn parse_suffix(suffix: &str) -> Option<u8> {
    if !suffix.starts_with('U') {
        return None;
    }
    let width: u8 = suffix[1..].parse().ok()?;
    if width < 1 || width > 128 {
        return None;
    }
    Some(width)
}

fn map_token_stream_literals(
    stream: TokenStream,
    f: &mut impl FnMut(Literal) -> TokenStream,
) -> TokenStream {
    stream
        .into_iter()
        .map(|tt| map_token_tree_literals(tt, f))
        .flatten()
        .collect()
}

fn map_token_tree_literals(
    tt: TokenTree,
    f: &mut impl FnMut(Literal) -> TokenStream,
) -> TokenStream {
    match tt {
        TokenTree::Group(group) => {
            let mut new_group = Group::new(
                group.delimiter(),
                map_token_stream_literals(group.stream(), f),
            );
            new_group.set_span(group.span());
            TokenTree::Group(new_group).into()
        }
        TokenTree::Ident(_) => tt.into(),
        TokenTree::Punct(_) => tt.into(),
        TokenTree::Literal(lit) => f(lit),
    }
}

#[derive(Default)]
struct ConfigBuilder {
    crate_path: Option<Path>,
}

impl ConfigBuilder {
    fn parser(&mut self) -> impl Parser<Output = ()> + '_ {
        syn::meta::parser(|meta| {
            if meta.path.is_ident("crate_path") {
                self.crate_path = Some(meta.value()?.parse()?);
                Ok(())
            } else {
                Err(meta.error("unsupported property"))
            }
        })
    }

    fn build(self) -> Config {
        Config {
            crate_path: self.crate_path.unwrap_or_else(|| parse_quote! { ::bitint }),
        }
    }
}

struct Config {
    crate_path: Path,
}

impl Config {
    fn new(attr: TokenStream) -> (Self, Errors) {
        let mut errors = Errors::new();
        let mut builder = ConfigBuilder::default();
        if !attr.is_empty() {
            errors.record(builder.parser().parse2(attr));
        }
        (builder.build(), errors)
    }
}

#[derive(Default)]
struct Errors {
    error: Option<Error>,
}

impl Errors {
    fn new() -> Self {
        Default::default()
    }

    fn push(&mut self, e: Error) {
        match &mut self.error {
            None => self.error = Some(e),
            Some(error) => error.combine(e),
        }
    }

    fn record(&mut self, result: Result<()>) {
        if let Err(e) = result {
            self.push(e);
        }
    }
}

impl ToTokens for Errors {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(error) = &self.error {
            tokens.extend(error.to_compile_error());
        }
    }
}

fn bitint_literals_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (cfg, cfg_errors) = Config::new(attr);
    let mut result = cfg_errors.into_token_stream();

    result.extend(map_token_stream_literals(
        item,
        &mut |literal| match rewrite_literal(&cfg.crate_path, literal) {
            RewriteResult::Rewritten(tokens) => tokens,
            RewriteResult::UnrecognizedSuffix(literal) => TokenTree::Literal(literal).into(),
            RewriteResult::ValueError(e) => e.into_compile_error(),
        },
    ));

    result
}

#[cfg(test)]
mod tests {
    use quote::{quote, ToTokens};
    use std::fmt::{self, Debug, Formatter};
    use syn::parse::{Parse, ParseStream};
    use syn::{Expr, Item, Result};

    use super::{bitint_impl, bitint_literals_impl};

    #[test]
    fn bitint_simple() {
        assert_eq!(
            syn::parse2::<Expr>(bitint_impl(quote! { (some::path::to, 7_U3) })).unwrap(),
            syn::parse2::<Expr>(quote! { some::path::to::types::U3::new_masked(7) }).unwrap(),
        );
    }

    #[derive(PartialEq, Eq)]
    struct ParseItems(Vec<Item>);

    impl Parse for ParseItems {
        fn parse(input: ParseStream) -> Result<Self> {
            let mut items = Vec::new();
            while !input.is_empty() {
                items.push(input.parse()?);
            }
            Ok(Self(items))
        }
    }

    impl Debug for ParseItems {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            let mut delim = "[";
            for item in &self.0 {
                write!(f, "{delim}")?;
                delim = ", ";
                write!(f, "{:?}", item.to_token_stream().to_string())?;
            }
            write!(f, "]")
        }
    }

    #[test]
    fn bitint_literals_simple() {
        assert_eq!(
            syn::parse2::<ParseItems>(bitint_literals_impl(
                quote! {},
                quote! { fn foo() { 1234567_U24 } },
            ))
            .unwrap(),
            syn::parse2::<ParseItems>(quote! {
                fn foo() { ::bitint::types::U24::new_masked(1234567) }
            })
            .unwrap(),
        );
    }

    #[test]
    fn bitint_literals_with_crate_path() {
        assert_eq!(
            syn::parse2::<ParseItems>(bitint_literals_impl(
                quote! { crate_path = path::to::bitint_crate },
                quote! { fn foo() { 1234567_U24 } },
            ))
            .unwrap(),
            syn::parse2::<ParseItems>(quote! {
                fn foo() { path::to::bitint_crate::types::U24::new_masked(1234567) }
            })
            .unwrap(),
        );
    }
}

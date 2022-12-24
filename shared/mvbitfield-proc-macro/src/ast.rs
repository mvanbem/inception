use nom::combinator::{opt, recognize};
use nom::multi::separated_list1;
use nom::sequence::{terminated, tuple};
use nom::{Finish, Parser};
use proc_macro2::{Delimiter, Ident, Literal, TokenStream};
use proc_macro2_nom::{
    delim, descend, ident, keyword, literal, punct, FlatTokenStream, FlatTokenStreamSlice,
};

pub(crate) struct StructAst {
    pub header: StructAstHeader,
    pub fields: Vec<FieldAst>,
}

pub(crate) struct StructAstHeader {
    pub visibility: TokenStream,
    pub name: Ident,
    pub repr: Ident,
}

pub(crate) struct FieldAst {
    pub visibility: TokenStream,
    pub name: Ident,
    pub width: Literal,
    pub io_type: Option<Ident>,
}

fn visibility<'input>(
) -> impl Parser<FlatTokenStreamSlice<'input>, TokenStream, proc_macro2_nom::Error> {
    recognize(opt(tuple((
        keyword("pub"),
        opt(descend(delim(Delimiter::Parenthesis), keyword("crate"))),
    ))))
    .map(FlatTokenStreamSlice::to_token_stream)
}

fn header<'input>(
) -> impl Parser<FlatTokenStreamSlice<'input>, StructAstHeader, proc_macro2_nom::Error> {
    tuple((visibility(), keyword("struct"), ident, punct(':'), ident)).map(
        |(visibility, _, name, _, repr)| StructAstHeader {
            visibility,
            name: name.clone(),
            repr: repr.clone(),
        },
    )
}

fn field<'input>() -> impl Parser<FlatTokenStreamSlice<'input>, FieldAst, proc_macro2_nom::Error> {
    tuple((
        visibility(),
        ident,
        punct(':'),
        literal,
        opt(tuple((keyword("as"), ident)).map(|(_, io_type)| io_type)),
    ))
    .map(|(visibility, name, _, width, io_type)| FieldAst {
        visibility,
        name: name.clone(),
        width: width.clone(),
        io_type: io_type.cloned(),
    })
}

fn struct_decl<'input>(
) -> impl Parser<FlatTokenStreamSlice<'input>, StructAst, proc_macro2_nom::Error> {
    tuple((
        header(),
        descend(
            delim(Delimiter::Brace),
            terminated(separated_list1(punct(','), field()), opt(punct(','))),
        ),
    ))
    .map(|(header, fields)| StructAst { header, fields })
}

pub(crate) fn parse(input: TokenStream) -> StructAst {
    let mut parser = struct_decl();
    parser
        .parse(FlatTokenStream::new(input).slice())
        .finish()
        .unwrap()
        .1
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    fn apply<'input, T>(
        mut parser: impl Parser<FlatTokenStreamSlice<'input>, T, proc_macro2_nom::Error>,
        input: &'input FlatTokenStream,
    ) -> T {
        parser.parse(input.slice()).finish().unwrap().1
    }

    #[test]
    fn visibility_empty() {
        let input = quote! {};
        let parser = visibility();
        let result = apply(parser, &FlatTokenStream::new(input));
        assert_eq!(result.to_string(), "");
    }

    #[test]
    fn visibility_pub() {
        let input = quote! { pub };
        let parser = visibility();
        let result = apply(parser, &FlatTokenStream::new(input.clone()));
        assert_eq!(result.to_string(), input.to_string());
    }

    #[test]
    fn visibility_pub_crate() {
        let input = quote! { pub(crate) };
        let parser = visibility();
        let result = apply(parser, &FlatTokenStream::new(input.clone()));
        assert_eq!(result.to_string(), input.to_string());
    }

    #[test]
    fn header_default_visibility() {
        let input = quote! { struct Foo: u32 };
        let parser = header();
        let result = apply(parser, &FlatTokenStream::new(input));
        assert!(result.visibility.is_empty());
        assert_eq!(result.name.to_string(), "Foo");
        assert_eq!(result.repr.to_string(), "u32");
    }

    #[test]
    fn header_pub() {
        let input = quote! { pub struct Foo: U24 };
        let parser = header();
        let result = apply(parser, &FlatTokenStream::new(input));
        assert_eq!(result.visibility.to_string(), "pub");
        assert_eq!(result.name.to_string(), "Foo");
        assert_eq!(result.repr.to_string(), "U24");
    }

    #[test]
    fn field_default() {
        let input = quote! { my_field: 5 };
        let parser = field();
        let result = apply(parser, &FlatTokenStream::new(input));
        assert_eq!(result.visibility.to_string(), "");
        assert_eq!(result.name.to_string(), "my_field");
        assert_eq!(result.width.to_string(), "5");
        assert!(result.io_type.is_none());
    }

    #[test]
    fn field_everything() {
        let input = quote! { pub(crate) my_field: 5 as Bar };
        let parser = field();
        let result = apply(parser, &FlatTokenStream::new(input));
        assert_eq!(
            result.visibility.to_string(),
            quote! { pub(crate) }.to_string(),
        );
        assert_eq!(result.name.to_string(), "my_field");
        assert_eq!(result.width.to_string(), "5");
        assert_eq!(result.io_type.unwrap().to_string(), "Bar");
    }
}

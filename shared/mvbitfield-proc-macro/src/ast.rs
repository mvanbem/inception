use combine::combinator::recognize;
use combine::{between, choice, optional, sep_end_by1, ParseError, Parser, Stream};
use combine_proc_macro::parser::{delim, ident, keyword, literal, punct};
use combine_proc_macro::Token;
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};

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

pub(crate) fn struct_ast<I>() -> impl Parser<Input = I, Output = StructAst>
where
    I: Stream<Item = Token>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    struct Body {
        fields: Vec<FieldAst>,
    }

    let visibility = || {
        recognize::<Vec<_>, _>(optional(choice((
            keyword("pub").map(drop),
            (keyword("pub"), delim('('), keyword("crate"), delim(')')).map(drop),
        ))))
        .map(|vec| {
            vec.into_iter()
                .map(|x: Token| TokenTree::try_from(x).unwrap())
                .collect()
        })
    };

    let header = (
        visibility(),
        keyword("struct"),
        ident(),
        punct(':'),
        ident(),
    )
        .map(|(visibility, _, name, _, repr)| StructAstHeader {
            visibility,
            name,
            repr,
        });

    let field = (
        visibility(),
        ident(),
        punct(':'),
        literal(),
        optional((keyword("as"), ident()).map(|(_, io_type)| io_type)),
    )
        .map(|(visibility, name, _, width, io_type)| FieldAst {
            visibility,
            name,
            width,
            io_type,
        });

    let body = between(delim('{'), delim('}'), sep_end_by1(field, punct(',')))
        .map(|fields| Body { fields });

    (header, body).map(|(header, Body { fields })| StructAst { header, fields })
}

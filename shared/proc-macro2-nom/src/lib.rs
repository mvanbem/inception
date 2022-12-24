//! Provides [`nom`] parsers recognizing [`proc_macro2::TokenStream`]s.

use std::ops::RangeTo;

use fix_hidden_lifetime_bug::Captures;
use nom::combinator::{all_consuming, map_parser, verify};
use nom::error::{ErrorKind, ParseError};
use nom::{IResult, InputLength, Offset, Parser, Slice};
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, TokenStream, TokenTree};

/// A processed token stream that is contiguous and offers random access.
#[derive(Clone, Debug)]
pub struct FlatTokenStream {
    items: Vec<FlatTokenTree>,
}

impl FlatTokenStream {
    pub fn new(token_stream: TokenStream) -> Self {
        Self {
            items: Vec::from_iter(token_stream.into_iter().map(Into::into)),
        }
    }

    pub fn slice(&self) -> FlatTokenStreamSlice {
        FlatTokenStreamSlice {
            stream: self,
            start: 0,
            end: self.items.len(),
        }
    }
}

/// A token tree where groups have been preprocessed into [`FlatTokenStream`]s.
#[derive(Clone, Debug)]
enum FlatTokenTree {
    Group {
        inner: Group,
        stream: FlatTokenStream,
    },
    Ident(Ident),
    Punct(Punct),
    Literal(Literal),
}

impl FlatTokenTree {
    fn to_token_tree(&self) -> TokenTree {
        match self {
            FlatTokenTree::Group { inner, .. } => TokenTree::Group(inner.clone()),
            FlatTokenTree::Ident(inner) => TokenTree::Ident(inner.clone()),
            FlatTokenTree::Punct(inner) => TokenTree::Punct(inner.clone()),
            FlatTokenTree::Literal(inner) => TokenTree::Literal(inner.clone()),
        }
    }
}

impl From<TokenTree> for FlatTokenTree {
    fn from(value: TokenTree) -> Self {
        match value {
            TokenTree::Group(inner) => {
                let stream = FlatTokenStream::new(inner.stream());
                Self::Group { inner, stream }
            }
            TokenTree::Ident(inner) => Self::Ident(inner),
            TokenTree::Punct(inner) => Self::Punct(inner),
            TokenTree::Literal(inner) => Self::Literal(inner),
        }
    }
}

/// A group that has had its span preprocessed into a [`FlatTokenStream`].
#[derive(Clone, Debug)]
pub struct FlatGroup<'input> {
    inner: &'input Group,
    stream: &'input FlatTokenStream,
}

impl<'input> FlatGroup<'input> {
    pub fn inner(&self) -> &'input Group {
        self.inner
    }

    pub fn delimiter(&self) -> Delimiter {
        self.inner.delimiter()
    }

    pub fn slice(&self) -> FlatTokenStreamSlice<'input> {
        FlatTokenStreamSlice {
            stream: self.stream,
            start: 0,
            end: self.stream.items.len(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlatTokenStreamSlice<'input> {
    stream: &'input FlatTokenStream,
    start: usize,
    end: usize,
}

impl<'input> FlatTokenStreamSlice<'input> {
    fn peek(&mut self) -> Option<&'input FlatTokenTree> {
        if self.start >= self.end {
            None
        } else {
            Some(&self.stream.items[self.start])
        }
    }

    fn next(&mut self) -> Option<&'input FlatTokenTree> {
        let result = self.peek();
        if result.is_some() {
            self.start += 1;
        }
        result
    }

    pub fn to_token_stream(self) -> TokenStream {
        self.stream.items[self.start..self.end]
            .iter()
            .map(FlatTokenTree::to_token_tree)
            .collect()
    }
}

impl<'input> InputLength for FlatTokenStreamSlice<'input> {
    fn input_len(&self) -> usize {
        self.end - self.start
    }
}

impl<'input> Offset for FlatTokenStreamSlice<'input> {
    fn offset(&self, second: &Self) -> usize {
        assert!(std::ptr::eq(self.stream, second.stream));
        second.start - self.start
    }
}

impl<'input> Slice<RangeTo<usize>> for FlatTokenStreamSlice<'input> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        Self {
            stream: self.stream,
            start: self.start,
            end: self.end.min(self.start + range.end),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    _priv: (),
}

impl<'input> ParseError<FlatTokenStreamSlice<'input>> for Error {
    fn from_error_kind(_input: FlatTokenStreamSlice<'input>, _kind: ErrorKind) -> Self {
        Self { _priv: () }
    }

    fn append(_input: FlatTokenStreamSlice<'input>, _kind: ErrorKind, _other: Self) -> Self {
        Self { _priv: () }
    }
}

fn error<'input, T>() -> Result<'input, T> {
    Err(nom::Err::Error(Error { _priv: () }))
}

pub type Result<'input, T> = IResult<FlatTokenStreamSlice<'input>, T, Error>;
pub type RefResult<'input, T> = Result<'input, &'input T>;

macro_rules! consume_if_match_and_extract_else_error {
    (
        $input:ident,
        $pat:pat $(if $guard:expr)? => $x:expr $(,)?
    ) => {
        match $input.peek() {
            #[allow(unused_variables)]
            $pat $(if $guard)? => {
                if let $pat = $input.next() {
                    Ok(($input, $x))
                } else {
                    unreachable!()
                }
            }
            _ => error(),
        }
    };
}

/// Matches any ident token tree.
pub fn ident<'input>(mut input: FlatTokenStreamSlice<'input>) -> RefResult<'input, Ident> {
    consume_if_match_and_extract_else_error!(input, Some(FlatTokenTree::Ident(id)) => id)
}

/// Matches an ident token tree with the given text.
pub fn keyword<'input: 'text, 'text>(
    text: &'text str,
) -> impl Parser<FlatTokenStreamSlice<'input>, &'input Ident, Error> + 'text {
    verify(ident, move |id: &&'input Ident| id.to_string() == text)
}

/// Matches any group token tree.
pub fn group<'input>(mut input: FlatTokenStreamSlice<'input>) -> Result<'input, FlatGroup<'input>> {
    consume_if_match_and_extract_else_error!(
        input,
        Some(FlatTokenTree::Group { inner, stream }) => FlatGroup {
            inner, stream: stream,
        },
    )
}

/// Matches a group token tree with the given delimiter.
pub fn delim<'input>(
    delim: Delimiter,
) -> impl Parser<FlatTokenStreamSlice<'input>, FlatGroup<'input>, Error> {
    verify(group, move |group: &FlatGroup<'input>| {
        group.delimiter() == delim
    })
}

/// Applies a parser over the token stream inside a parsed group.
pub fn descend<'input, I, O, E>(
    group_parser: impl Parser<I, FlatGroup<'input>, E>,
    parser: impl Parser<FlatTokenStreamSlice<'input>, O, E>,
) -> impl Parser<I, O, E> + Captures<'input>
where
    E: ParseError<I>,
    E: ParseError<FlatTokenStreamSlice<'input>>,
{
    map_parser(
        group_parser.map(|group| group.slice()),
        all_consuming(parser),
    )
}

/// Matches any punct token tree.
pub fn any_punct<'input>(mut input: FlatTokenStreamSlice<'input>) -> RefResult<'input, Punct> {
    consume_if_match_and_extract_else_error!(input, Some(FlatTokenTree::Punct(punct)) => punct)
}

/// Matches a punct token tree with the given character.
pub fn punct<'input>(c: char) -> impl Parser<FlatTokenStreamSlice<'input>, &'input Punct, Error> {
    verify(any_punct, move |punct: &&'input Punct| punct.as_char() == c)
}

/// Matches any literal token tree.
pub fn literal<'input>(mut input: FlatTokenStreamSlice<'input>) -> RefResult<'input, Literal> {
    consume_if_match_and_extract_else_error!(input, Some(FlatTokenTree::Literal(literal)) => literal)
}

#[cfg(test)]
mod tests {
    use nom::Parser;
    use quote::quote;

    use crate::*;

    #[test]
    fn ident_success() {
        let input = FlatTokenStream::new(quote! { foo });
        let parser = ident;

        let (mut rest, result) = parser(input.slice()).unwrap();
        assert_eq!(result.to_string(), "foo");
        assert!(rest.peek().is_none());
    }

    #[test]
    fn ident_eof() {
        let input = FlatTokenStream::new(quote! {});
        let parser = ident;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }

    #[test]
    fn ident_unexpected_token() {
        let input = FlatTokenStream::new(quote! { ! });
        let parser = ident;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }

    #[test]
    fn keyword_success() {
        let input = FlatTokenStream::new(quote! { foo });
        let mut parser = keyword("foo");

        let (mut rest, result) = parser.parse(input.slice()).unwrap();
        assert_eq!(result.to_string(), "foo");
        assert!(rest.peek().is_none());
    }

    #[test]
    fn keyword_eof() {
        let input = FlatTokenStream::new(quote! {});
        let mut parser = keyword("foo");

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn keyword_unexpected_token() {
        let input = FlatTokenStream::new(quote! { ! });
        let mut parser = keyword("foo");

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn keyword_unexpected_text() {
        let input = FlatTokenStream::new(quote! { bar });
        let mut parser = keyword("foo");

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn group_success() {
        let input = FlatTokenStream::new(quote! { (foo) });
        let mut parser = descend(group, keyword("foo"));

        let (mut rest, result) = parser.parse(input.slice()).unwrap();
        assert_eq!(result.to_string(), "foo");
        assert!(rest.peek().is_none());
    }

    #[test]
    fn group_eof() {
        let input = FlatTokenStream::new(quote! {});
        let mut parser = descend(group, keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn group_unexpected_token() {
        let input = FlatTokenStream::new(quote! { ! });
        let mut parser = descend(group, keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn delim_success() {
        let input = FlatTokenStream::new(quote! { (foo) });
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        let (mut rest, result) = parser.parse(input.slice()).unwrap();
        assert_eq!(result.to_string(), "foo");
        assert!(rest.peek().is_none());
    }

    #[test]
    fn delim_eof() {
        let input = FlatTokenStream::new(quote! {});
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn delim_unexpected_token() {
        let input = FlatTokenStream::new(quote! { ! });
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn delim_unexpected_delim() {
        let input = FlatTokenStream::new(quote! { [foo] });
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn delim_inner_not_all_consuming() {
        let input = FlatTokenStream::new(quote! { (foo bar) });
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn delim_inner_eof() {
        let input = FlatTokenStream::new(quote! { () });
        let mut parser = descend(delim(Delimiter::Parenthesis), keyword("foo"));

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn any_punct_success() {
        let input = FlatTokenStream::new(quote! { ! });
        let parser = any_punct;

        let (mut rest, result) = parser(input.slice()).unwrap();
        assert_eq!(result.as_char(), '!');
        assert!(rest.peek().is_none());
    }

    #[test]
    fn any_punct_eof() {
        let input = FlatTokenStream::new(quote! {});
        let parser = any_punct;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }

    #[test]
    fn any_punct_unexpected_token() {
        let input = FlatTokenStream::new(quote! { foo });
        let parser = any_punct;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }

    #[test]
    fn punct_success() {
        let input = FlatTokenStream::new(quote! { ! });
        let mut parser = punct('!');

        let (mut rest, result) = parser.parse(input.slice()).unwrap();
        assert_eq!(result.as_char(), '!');
        assert!(rest.peek().is_none());
    }

    #[test]
    fn punct_eof() {
        let input = FlatTokenStream::new(quote! {});
        let mut parser = punct('!');

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn punct_unexpected_token() {
        let input = FlatTokenStream::new(quote! { foo });
        let mut parser = punct('!');

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn punct_unexpected_char() {
        let input = FlatTokenStream::new(quote! { , });
        let mut parser = punct('!');

        assert!(matches!(
            parser.parse(input.slice()),
            Err(nom::Err::Error(_)),
        ));
    }

    #[test]
    fn literal_success() {
        let input = FlatTokenStream::new(quote! { 5 });
        let parser = literal;

        let (mut rest, result) = parser(input.slice()).unwrap();
        assert_eq!(result.to_string(), "5");
        assert!(rest.peek().is_none());
    }

    #[test]
    fn literal_eof() {
        let input = FlatTokenStream::new(quote! {});
        let parser = literal;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }

    #[test]
    fn literal_unexpected_token() {
        let input = FlatTokenStream::new(quote! { foo });
        let parser = literal;

        assert!(matches!(parser(input.slice()), Err(nom::Err::Error(_))));
    }
}

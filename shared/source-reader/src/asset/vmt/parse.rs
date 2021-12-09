use nalgebra_glm::{vec3, Vec3};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::*;
use nom::multi::{fold_many0, many0};
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;
use nom::{combinator::*, Parser};

pub fn vmt(input: &str) -> Result<Object, nom::Err<nom::error::Error<&str>>> {
    let (input, root) = object(input)?;
    let (input, _) = whitespace0(input)?;
    assert_eq!(input, "");

    Ok(root)
}

fn whitespace_char(input: &str) -> IResult<&str, char> {
    satisfy(|c| c.is_whitespace())(input)
}

fn non_whitespace_char(input: &str) -> IResult<&str, char> {
    satisfy(|c| !c.is_whitespace())(input)
}

fn line_comment(input: &str) -> IResult<&str, &str> {
    recognize(tuple((tag("//"), many0(none_of("\r\n")), line_ending)))(input)
}

fn whitespace0(input: &str) -> IResult<&str, &str> {
    recognize(many0(alt((
        recognize(whitespace_char),
        recognize(line_comment),
    ))))(input)
}

fn string(input: &str) -> IResult<&str, &str> {
    preceded(
        whitespace0,
        alt((
            delimited(char('"'), recognize(many0(none_of("\""))), char('"')),
            recognize(tuple((none_of("{}"), many0(non_whitespace_char)))),
        )),
    )(input)
}

fn operator(c: char) -> impl Fn(&str) -> IResult<&str, char> {
    move |input| preceded(whitespace0, char(c))(input)
}

#[derive(Debug)]
pub enum Entry<'a> {
    KeyValue(KeyValue<'a>),
    Object(Object<'a>),
}

#[derive(Debug)]
pub struct KeyValue<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

#[derive(Debug)]
pub struct Object<'a> {
    pub name: &'a str,
    pub entries: Vec<Entry<'a>>,
}

fn entry(input: &str) -> IResult<&str, Entry> {
    alt((
        key_value.map(|key_value| Entry::KeyValue(key_value)),
        object.map(|object| Entry::Object(object)),
    ))(input)
}

fn key_value(input: &str) -> IResult<&str, KeyValue> {
    tuple((string, string))
        .map(|(key, value)| KeyValue { key, value })
        .parse(input)
}

fn object(input: &str) -> IResult<&str, Object> {
    let (input, name) = string(input)?;
    let (input, _) = operator('{')(input)?;
    let (input, entries) = fold_many0(
        entry,
        || Vec::new(),
        |mut entries, entry| {
            entries.push(entry);
            entries
        },
    )(input)?;
    let (input, _) = operator('}')(input)?;

    IResult::Ok((input, Object { name, entries }))
}

pub fn material_vector(input: &str) -> Result<Vec3, nom::Err<nom::error::Error<&str>>> {
    let (input, value) = alt((
        delimited(
            operator('['),
            tuple((number, number, number)),
            operator(']'),
        )
        .map(|(x, y, z)| vec3(x.parse().unwrap(), y.parse().unwrap(), z.parse().unwrap())),
        delimited(
            operator('{'),
            tuple((number, number, number)),
            operator('}'),
        )
        .map(|(x, y, z)| vec3(x.parse().unwrap(), y.parse().unwrap(), z.parse().unwrap()) / 255.0),
    ))(input)?;
    let (input, _) = whitespace0(input)?;
    assert_eq!(input, "");

    Ok(value)
}

fn digit0(input: &str) -> IResult<&str, &str> {
    recognize(many0(satisfy(|c| c.is_ascii_digit())))(input)
}

fn number(input: &str) -> IResult<&str, &str> {
    preceded(
        whitespace0,
        recognize(tuple((digit0, opt(tuple((char('.'), digit0)))))),
    )(input)
}

#[cfg(test)]
mod tests {
    #[test]
    fn line_comment() {
        assert_eq!(
            super::line_comment("// comment \nabc"),
            Ok(("abc", "// comment \n")),
        );
    }

    #[test]
    fn string() {
        assert_eq!(super::string("\"abc\"def"), Ok(("def", "abc")));
        assert_eq!(super::string("abc def"), Ok((" def", "abc")));
        assert_eq!(super::string("abc\ndef"), Ok(("\ndef", "abc")));
    }

    #[test]
    fn number() {
        assert_eq!(super::number("1"), Ok(("", "1")));
        assert_eq!(super::number("1."), Ok(("", "1.")));
        assert_eq!(super::number("1.2"), Ok(("", "1.2")));
        assert_eq!(super::number(".3"), Ok(("", ".3")));
    }
}

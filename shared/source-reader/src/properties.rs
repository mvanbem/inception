use std::collections::HashMap;

use nalgebra_glm::{vec3, Mat2x3, Mat3, Vec3};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::*;
use nom::multi::{fold_many0, many0, many1};
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;
use nom::{combinator::*, Parser};

pub fn vmt(input: &str) -> Result<Object, nom::Err<nom::error::Error<&str>>> {
    let (input, root) = object(input)?;
    let (input, _) = whitespace0(input)?;
    assert_eq!(input, "");

    Ok(root)
}

pub fn flat_objects(
    input: &str,
) -> Result<Vec<HashMap<String, String>>, nom::Err<nom::error::Error<&str>>> {
    let (input, objects) = many1(flat_object)(input)?;
    let (input, _) = whitespace0(input)?;
    assert_eq!(input, "");

    Ok(objects)
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

fn flat_object(input: &str) -> IResult<&str, HashMap<String, String>> {
    let (input, _) = whitespace0(input)?;
    let (input, _) = operator('{')(input)?;
    let (input, key_values) = fold_many0(
        key_value,
        || HashMap::new(),
        |mut key_values, key_value| {
            key_values.insert(key_value.key.to_lowercase(), key_value.value.to_lowercase());
            key_values
        },
    )(input)?;
    let (input, _) = operator('}')(input)?;

    IResult::Ok((input, key_values))
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

pub fn texture_transform(input: &str) -> Result<Mat2x3, nom::Err<nom::error::Error<&str>>> {
    let (input, ((cx, cy), (sx, sy), r, (tx, ty))) = tuple((
        preceded(tuple((whitespace0, tag("center"))), tuple((number, number)))
            .map(|(x, y)| -> (f32, f32) { (x.parse().unwrap(), y.parse().unwrap()) }),
        preceded(tuple((whitespace0, tag("scale"))), tuple((number, number)))
            .map(|(x, y)| -> (f32, f32) { (x.parse().unwrap(), y.parse().unwrap()) }),
        preceded(tuple((whitespace0, tag("rotate"))), number)
            .map(|a| -> f32 { a.parse().unwrap() }),
        preceded(
            tuple((whitespace0, tag("translate"))),
            tuple((number, number)),
        )
        .map(|(x, y)| -> (f32, f32) { (x.parse().unwrap(), y.parse().unwrap()) }),
    ))(input)?;
    let (input, _) = whitespace0(input)?;
    assert_eq!(input, "");

    let shift_before_rotate = Mat3::from_rows(&[
        vec3(1.0, 0.0, -cx).transpose(),
        vec3(0.0, 1.0, -cy).transpose(),
        vec3(0.0, 0.0, 1.0).transpose(),
    ]);
    let angle = core::f32::consts::PI / 180.0 * r;
    let cos = angle.cos();
    let sin = angle.sin();
    let rotate_shift_back_and_translate = Mat3::from_rows(&[
        vec3(cos, -sin, cx + tx).transpose(),
        vec3(sin, cos, cy + ty).transpose(),
        vec3(0.0, 0.0, 1.0).transpose(),
    ]);

    let scale = Mat3::from_diagonal(&vec3(sx, sy, 1.0));

    Ok(Mat2x3::identity() * scale * rotate_shift_back_and_translate * shift_before_rotate)
}

#[cfg(test)]
mod tests {
    use approx::AbsDiffEq;
    use nalgebra_glm::{vec3, Mat2x3};

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

    #[test]
    fn texture_transform() {
        assert_eq!(
            super::texture_transform("center .5 .5 scale 1 1 rotate 0 translate 0 0"),
            Ok(Mat2x3::identity()),
        );
        assert!(
            super::texture_transform("center 0 0 scale 1 1 rotate 90 translate 0 0")
                .unwrap()
                .abs_diff_eq(
                    &Mat2x3::from_rows(&[
                        vec3(0.0, -1.0, 0.0).transpose(),
                        vec3(1.0, 0.0, 0.0).transpose(),
                    ]),
                    1e-6,
                )
        );
        assert!(
            super::texture_transform("center .5 .5 scale 1 1 rotate 90 translate 0 0")
                .unwrap()
                .abs_diff_eq(
                    &Mat2x3::from_rows(&[
                        vec3(0.0, -1.0, 1.0).transpose(),
                        vec3(1.0, 0.0, 0.0).transpose(),
                    ]),
                    1e-6,
                )
        );
        assert!(
            super::texture_transform("center .5 .5 scale 2 0.5 rotate 0 translate 0 0")
                .unwrap()
                .abs_diff_eq(
                    &Mat2x3::from_rows(&[
                        vec3(2.0, 0.0, 0.0).transpose(),
                        vec3(0.0, 0.5, 0.0).transpose(),
                    ]),
                    1e-6,
                )
        );
        assert!(
            super::texture_transform("center .5 .5 scale 1 1 rotate 0 translate 3 5")
                .unwrap()
                .abs_diff_eq(
                    &Mat2x3::from_rows(&[
                        vec3(1.0, 0.0, 3.0).transpose(),
                        vec3(0.0, 1.0, 5.0).transpose(),
                    ]),
                    1e-6,
                )
        );
    }
}

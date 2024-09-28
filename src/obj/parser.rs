use std::str::FromStr;

use nom::{
    self,
    branch::alt,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::char,
    combinator::{cut, map, success},
    error::{context, ContextError, ErrorKind, ParseError},
    multi::{many0, many1, many_m_n},
    number::complete::float,
    sequence::{preceded, terminated, tuple},
    IResult,
};

#[derive(Clone)]
pub enum ObjDirective<'a> {
    Comment(&'a str),
    Preprocess(Vec<&'a str>),
    Mtllib(&'a str),
    O(&'a str),
    V(f32, f32, f32),
    Vt(f32, f32),
    Vn(f32, f32, f32),
    Usemtl(&'a str),
    F(Vec<[usize; 3]>),
}

#[derive(Clone)]
pub enum MtlDirective<'a> {
    Comment(&'a str),
    Newmtl(&'a str),
    MapKd(&'a str),
}

pub fn sp<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| matches!(c, '\t' | '\x0C' | ' '))(input)
}

pub fn term<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    match input.is_empty() {
        false => take_while1(|c| matches!(c, '\t' | '\n' | '\r'))(input),
        true => success("")(input),
    }
}

pub fn index<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, usize, E> {
    context(
        "non-zero index",
        map(take_while(|c| matches!(c, '0'..='9')), |input| usize::from_str(input)),
    )(input)
    .and_then(|(input, output)| {
        Ok((
            input,
            output
                .ok()
                .and_then(|output| output.checked_sub(1))
                .ok_or_else(|| nom::Err::Error(E::from_error_kind(input, ErrorKind::Digit)))?,
        ))
    })
}

pub fn id<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while(|c| matches!(c, '-' | '_' | '.') || c.is_alphanumeric())(input)
}

pub fn obj_comment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "comment",
        preceded(
            tag("#"),
            cut(preceded(
                sp,
                alt((
                    map(preceded(tag(">>>"), many1(preceded(sp, id))), ObjDirective::Preprocess),
                    map(
                        |input: &'a str| match input.is_empty() {
                            false => take_while(|c| !matches!(c, '\t' | '\n' | '\r'))(input),
                            true => success("")(input),
                        },
                        ObjDirective::Comment,
                    ),
                )),
            )),
        ),
    )(input)
}

pub fn mtllib<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "mtllib",
        preceded(tag("mtllib"), cut(preceded(sp, map(id, ObjDirective::Mtllib)))),
    )(input)
}

pub fn o<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context("o", preceded(tag("o"), cut(preceded(sp, map(id, ObjDirective::O)))))(input)
}

pub fn v<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "v",
        preceded(
            tag("v"),
            map(
                // We don't `cut()` here because `v` might actually be `vt` or `vn`.
                tuple((preceded(sp, float), preceded(sp, float), preceded(sp, float))),
                |(x, y, z)| ObjDirective::V(x, y, z),
            ),
        ),
    )(input)
}

pub fn vt<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "vt",
        preceded(
            tag("vt"),
            cut(map(tuple((preceded(sp, float), preceded(sp, float))), |(u, v)| {
                ObjDirective::Vt(u, v)
            })),
        ),
    )(input)
}

pub fn vn<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "vn",
        preceded(
            tag("vn"),
            cut(map(
                tuple((preceded(sp, float), preceded(sp, float), preceded(sp, float))),
                |(x, y, z)| ObjDirective::Vn(x, y, z),
            )),
        ),
    )(input)
}

pub fn usemtl<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "usemtl`",
        preceded(tag("usemtl"), cut(preceded(sp, map(id, ObjDirective::Usemtl)))),
    )(input)
}

pub fn f<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, ObjDirective<'a>, E> {
    context(
        "f",
        preceded(
            tag("f"),
            cut(map(
                many_m_n(
                    3,
                    usize::MAX,
                    preceded(
                        sp,
                        map(tuple((index, char('/'), index, char('/'), index)), |(v, _, vt, _, vn)| {
                            [v, vt, vn]
                        }),
                    ),
                ),
                ObjDirective::F,
            )),
        ),
    )(input)
}

pub fn parse_obj<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, Vec<ObjDirective<'a>>, E> {
    many0(terminated(
        alt((obj_comment, mtllib, o, v, vt, vn, usemtl, f)),
        preceded(sp, term),
    ))(input)
}

pub fn mtl_comment<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, MtlDirective<'a>, E> {
    context(
        "comment",
        preceded(
            tag("#"),
            cut(preceded(
                sp,
                map(
                    |input: &'a str| match input.is_empty() {
                        false => take_while(|c| !matches!(c, '\t' | '\n' | '\r'))(input),
                        true => success("")(input),
                    },
                    MtlDirective::Comment,
                ),
            )),
        ),
    )(input)
}

pub fn newmtl<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, MtlDirective<'a>, E> {
    context(
        "newmtl",
        preceded(tag("newmtl"), cut(preceded(sp, map(id, MtlDirective::Newmtl)))),
    )(input)
}

pub fn map_kd<'a, E: ParseError<&'a str> + ContextError<&'a str>>(input: &'a str) -> IResult<&'a str, MtlDirective<'a>, E> {
    context(
        "map_Kd",
        preceded(tag("map_Kd"), cut(preceded(sp, map(id, MtlDirective::MapKd)))),
    )(input)
}

pub fn parse_mtl<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, Vec<MtlDirective<'a>>, E> {
    many0(terminated(alt((mtl_comment, newmtl, map_kd)), preceded(sp, term)))(input)
}

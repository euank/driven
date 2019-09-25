extern crate nom;

use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_not, tag, take, take_while1},
    character::complete::{char, multispace0, one_of, space0},
    combinator::{cut, map, map_parser, map_res},
    error::{context, ErrorKind, ParseError, VerboseError},
    multi,
    sequence::{preceded, terminated},
    IResult,
};

// parses the escape character(s) after a '\' in quotes
fn str_escape<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let take1 = take(1usize);
    let unescape_char = |c: char| -> Result<&str, E> {
        match c {
            'n' => Ok("\n"),
            '"' => Ok("\""),
            '$' => Ok("$"),
            '\\' => Ok("\\"),
            _ => Err(ParseError::add_context(
                i,
                "expected a valid backslash escape character",
                ParseError::from_error_kind(i, ErrorKind::OneOf),
            )),
        }
    };
    let unescape = map_res(one_of("n\\\""), unescape_char);
    map_parser(take1, unescape)(i)
}

fn str_part<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StringPart, E> {
    // TODO: variables
    // "${foo}" must interact well with escaping, such as "\${foo}", too
    let string_lit = escaped_transform(is_not("\\\""), '\\', str_escape);
    map(string_lit, |s: String| StringPart::Literal(s))(i)
}

fn is_variable_char<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while1(move |c: char| c.is_alphanumeric() || c == '_')(i)
}

fn unquoted_variable_name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StringRef, E> {
    let varname = is_variable_char;
    map(varname, |s: &str| StringRef {
        parts: vec![StringPart::Literal(s.to_string())],
    })(i)
}

fn quoted_string<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StringRef, E> {
    let refparser = context(
        "string",
        preceded(char('\"'), cut(terminated(str_part, char('\"')))),
    );
    // TODO: parse multiple parts, only relevant when we support variable references
    map(refparser, |s: StringPart| StringRef { parts: vec![s] })(i)
}

fn variable_name<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StringRef, E> {
    alt((unquoted_variable_name, quoted_string))(i)
}

fn wrapped<I, O1, O2, E: ParseError<I>, F, G>(first: F, wrapper: G) -> impl Fn(I) -> IResult<I, O1, E>
where
    F: Fn(I) -> IResult<I, O1, E>,
    G: Fn(I) -> IResult<I, O2, E>,
    G: Copy,
{
    terminated(preceded(wrapper, first), wrapper)
}

fn variable_assignment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, DrivenVar, E> {
    let (i, _) = multispace0(i)?;
    let whitespace_var_name = wrapped(variable_name, space0);
    let internal = wrapped(tag("internal"), space0);
    let internal_var = map(terminated(preceded(internal, whitespace_var_name), tag("=")), |res| (true, res));

    let whitespace_var_name = wrapped(variable_name, space0);
    let regular_var = map(terminated(whitespace_var_name, tag("=")), |res| (false, res));

    let (i, (internal, var)) = alt((internal_var, regular_var))(i)?;
    let (i, val) = terminated(preceded(space0, quoted_string), multispace0)(i)?;

    Ok((
        i,
        DrivenVar {
            internal: internal,
            name: var,
            value: val,
        },
    ))
}

fn driven_file<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, DrivenFile, E> {
    // TODO: ignore_parents and allow_shell_exec
    let (i, vars) = multi::many0(variable_assignment)(i)?;

    Ok((
        i,
        DrivenFile {
            ignore_parents: false,
            allow_shell_exec: false,
            variables: vars,
        },
    ))
}

pub fn drivenfile(i: &str) -> Result<DrivenFile, String> {
    match driven_file::<VerboseError<&str>>(i) {
        Ok((_, f)) => Ok(f),
        Err(nom::Err::Error(e)) => {
            Err(nom::error::convert_error(i, e))
        },
        Err(nom::Err::Incomplete(_)) => {
            // we're not using the streaming api
            panic!("unreachable");
        }
        Err(nom::Err::Failure(f)) => {
            Err(format!("Failure parsing drivenfile: {:?}", f))
        }
    }
}

#[derive(Debug, PartialEq)]
// DrivenFile is a type representing a single `.driven` file that has been parsed, but has not had
// all variables resolved.
// Variable resolution is defered because a .driven file may reference variables defined in other
// files and in the environment itself, so we cannot yet evaluate those references.
pub struct DrivenFile<'a> {
    ignore_parents: bool,
    allow_shell_exec: bool,
    variables: Vec<DrivenVar<'a>>,
}

#[derive(Debug, PartialEq)]
struct DrivenVar<'a> {
    internal: bool,
    name: StringRef<'a>,
    value: StringRef<'a>,
}

#[derive(Debug, PartialEq)]
enum StringPart<'a> {
    Literal(String),
    Variable(&'a str),
}

#[derive(Debug, PartialEq)]
struct StringRef<'a> {
    parts: Vec<StringPart<'a>>,
}

mod test {
    use super::*;

    fn litpart(s: &str) -> StringPart {
        StringPart::Literal(s.to_string())
    }
    fn lit(s: &str) -> StringRef {
        StringRef {
            parts: vec![litpart(s)],
        }
    }

    #[test]
    fn str_part_test() {
        assert_eq!(
            str_part::<()>("foo"),
            Ok(("", litpart("foo")))
        );
        assert_eq!(
            str_part::<()>(r#"foo\"a"#),
            Ok(("", litpart(r#"foo"a"#)))
        );
        assert_eq!(
            str_part::<()>(r#"foo\\foo"#),
            Ok(("", litpart(r#"foo\foo"#)))
        );
        assert_eq!(
            str_part::<()>("ご飯"),
            Ok(("", litpart("ご飯")))
        );
        assert_eq!(
            str_part::<(&str, ErrorKind)>(r#"foo\1foo"#),
            Err(nom::Err::Error(("1", ErrorKind::OneOf))),
        );
    }

    #[test]
    fn quoted_str_test() {
        assert_eq!(quoted_string::<(_, _)>(r#""foo""#), Ok(("", lit("foo"))),)
    }

    #[test]
    fn variable_assignment_test() {
        assert_eq!(
            variable_assignment::<(_, _)>(r#"foo="bar""#),
            Ok((
                "",
                DrivenVar {
                    internal: false,
                    name: lit("foo"),
                    value: lit("bar"),
                }
            ))
        );
        assert_eq!(
            variable_assignment::<(_, _)>(r#""foo"="bar""#),
            Ok((
                "",
                DrivenVar {
                    internal: false,
                    name: lit("foo"),
                    value: lit("bar"),
                }
            ))
        );
        assert_eq!(
            variable_assignment::<(_, _)>(r#" "foo" = "bar" "#),
            Ok((
                "",
                DrivenVar {
                    internal: false,
                    name: lit("foo"),
                    value: lit("bar"),
                }
            ))
        );
        assert_eq!(
            variable_assignment::<(_, _)>(r#" internal "foo" = "bar" "#),
            Ok((
                "",
                DrivenVar {
                    internal: true,
                    name: lit("foo"),
                    value: lit("bar"),
                }
            ))
        );
        assert!(variable_assignment::<(_, _)>(" foo = \n \"bar\" ").is_err(),);
    }
}

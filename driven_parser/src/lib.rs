#[macro_use]
extern crate nom;

use nom::{
    bytes::complete::{escaped_transform, is_not, take},
    character::complete::{char, one_of},
    combinator::{cut, map, map_res, map_parser},
    error::{context,  ParseError, ErrorKind},
    sequence::{preceded, terminated},
    IResult,
};

// assumes the '\' has been parsed already, parses the character proceeding it.
fn parse_str_escape_char<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let take1 = take(1usize);
    let unescape_char = |c: char| -> Result<&str, E> {
        match c {
            'n' => Ok("\n"),
            '"' => Ok("\""),
            '$' => Ok("$"),
            '\\' => Ok("\\"),
            _ => Err(ParseError::add_context(i, "expected a valid backslash escape character", ParseError::from_error_kind(i, ErrorKind::OneOf))),
        }
    };
    let unescape = map_res(one_of("n\\\""), unescape_char);
    map_parser(take1, unescape)(i)
}

fn parse_str_part<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, StringPart, E> {
    // TODO: variables
    // "${foo}" must interact well with escaping, such as "\${foo}", too
    let string_lit = escaped_transform(is_not("\\\""), '\\', parse_str_escape_char);
    map(string_lit, |s: String| StringPart::Literal(s))(i)
}

fn quoted_string(i: &str) -> IResult<&str, StringRef> {
    let refparser = context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str_part, char('\"')))),
    );
    // TODO: parse multiple parts
    map(refparser, |s: StringPart| StringRef { parts: vec![s] })(i)
}

#[derive(Debug, PartialEq)]
// DrivenFile is a type representing a single `.driven` file that has been parsed, but has not had
// all variables resolved.
// Variable resolution is defered because a .driven file may reference variables defined in other
// files and in the environment itself, so we cannot yet evaluate those references.
struct DrivenFile<'a> {
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
    use nom::Err;

    #[test]
    fn parse_str_part_test() {
        assert_eq!(
            parse_str_part::<()>("foo"),
            Ok(("", StringPart::Literal("foo".to_string())))
        );
        assert_eq!(
            parse_str_part::<()>(r#"foo\"a"#),
            Ok(("", StringPart::Literal(r#"foo"a"#.to_string())))
        );
        assert_eq!(
            parse_str_part::<()>(r#"foo\\foo"#),
            Ok(("", StringPart::Literal(r#"foo\foo"#.to_string())))
        );
        assert_eq!(
            parse_str_part::<(&str, ErrorKind)>(r#"foo\1foo"#),
            Err(Err::Error(("1", ErrorKind::OneOf))),
        );
    }

    #[test]
    fn quoted_str_test() {
        assert_eq!(
            quoted_string(r#""foo""#),
            Ok(("", StringRef{
                parts: vec![StringPart::Literal("foo".to_string())],
            }))
        )
    }
}

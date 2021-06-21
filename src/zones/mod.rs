use crate::resource::*;
use crate::zones::tokens::tokenise;
use crate::zones::tokens::TokenType;
use crate::zones::tokens::Tokens;
use crate::Class;
use crate::Resource;
use nom::combinator::map_res;
use nom::combinator::value;

use nom::branch::alt;
use nom::branch::permutation;
use nom::bytes::complete::*;
use nom::combinator::all_consuming;
use nom::combinator::cut;
use nom::combinator::map;
use nom::combinator::success;
use nom::combinator::verify;
use nom::error::context;
use nom::error::ContextError;
use nom::error::FromExternalError;
use nom::error::ParseError;
use nom::error::VerboseError;
use nom::error::VerboseErrorKind;
use nom::multi::many0;
use nom::sequence::delimited;
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::sequence::tuple;
use nom::IResult;
use nom_locate::LocatedSpan;

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;
use std::time::Duration;

mod tokens;

/// Parse a IPv4 address, we use the Ipv4Addr::from_str implementation.
fn ipv4_addr<
    'a,
    E: ParseError<Tokens<'a>>
        + ContextError<Tokens<'a>>
        + FromExternalError<Tokens<'a>, std::net::AddrParseError>,
>(
    input: Tokens<'a>,
) -> IResult<Tokens<'a>, Ipv4Addr, E> {
    context(
        "IPv4 address",
        map_res(tag(TokenType::Word), |t: Tokens| {
            Ipv4Addr::from_str(t[0].as_str())
        }),
    )(input)
}

// https://datatracker.ietf.org/doc/html/rfc3596#section-2.4
// https://datatracker.ietf.org/doc/html/rfc3513
fn ipv6_addr<
    'a,
    E: ParseError<Tokens<'a>>
        + ContextError<Tokens<'a>>
        + FromExternalError<Tokens<'a>, std::net::AddrParseError>,
>(
    input: Tokens<'a>,
) -> IResult<Tokens<'a>, Ipv6Addr, E> {
    context(
        "IPv6 address",
        map_res(tag(TokenType::Word), |t: Tokens| {
            Ipv6Addr::from_str(t[0].as_str())
        }),
    )(input)
}

/// Consumes and discards the prefix, then returns the result of the parser.
fn prefixed<'a, O, E: ParseError<Tokens<'a>>, F>(
    prefix: &'a str,
    f: F,
) -> impl FnMut(Tokens<'a>) -> IResult<Tokens<'a>, O, E>
where
    F: nom::Parser<Tokens<'a>, O, E>,
{
    preceded(pair(keyword(prefix), tag(TokenType::Whitespace)), f)
}

/// Matches a token with this word.
fn keyword<'a, E: ParseError<Tokens<'a>>>(
    word: &'a str,
) -> impl FnMut(Tokens<'a>) -> IResult<Tokens<'a>, Tokens<'a>, E> {
    verify(tag(TokenType::Word), move |tokens: &Tokens| {
        tokens[0].as_str() == word
    })
}

/// Runs the parser and if successful returns the result a [`Option::Some`].
fn some<I: Clone, O, E: ParseError<I>, F>(mut f: F) -> impl FnMut(I) -> IResult<I, Option<O>, E>
where
    F: nom::Parser<I, O, E>,
{
    // Based on num::opt()
    move |input: I| match f.parse(input) {
        Ok((i, o)) => Ok((i, Some(o))),
        Err(e) => Err(e),
    }
}

fn string<'a, E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>>(
    input: Tokens<'a>,
) -> IResult<Tokens, &'a str, E> {
    map(tag(TokenType::Word), |t: Tokens| t[0].as_str())(input)
}

fn space<'a, E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>>(
    s: Tokens<'a>,
) -> IResult<Tokens, (), E> {
    value((), tag(TokenType::Whitespace))(s)
}

fn digit1<'a, O, E>(s: Tokens<'a>) -> IResult<Tokens, O, E>
where
    O: std::str::FromStr<Err = std::num::ParseIntError>,
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    // TODO Perhaps turn this into its own type!
    context(
        "number",
        map_res(tag(TokenType::Word), |tokens: Tokens| {
            tokens[0].as_str().parse::<O>()
        }),
    )(s)
}

fn duration<'a, E>(s: Tokens<'a>) -> IResult<Tokens, Duration, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    // TODO Bind supports different formats of TTL, such as "1d"
    context(
        "Duration",
        terminated(map(digit1, |i: u64| Duration::new(i, 0)), space),
    )(s)
}

/// Does this Token look like a domain name?
fn is_domain(s: &str) -> bool {
    // TODO We should tighten up the defintion.
    s.is_ascii()
}

/// Parses a domain name.
fn domain<'a, E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>>(
    input: Tokens<'a>,
) -> IResult<Tokens, &'a str, E> {
    context(
        "Domain name",
        map(
            verify(tag(TokenType::Word), |t: &Tokens| is_domain(t[0].as_str())),
            |t: Tokens| t[0].as_str(),
        ),
    )(input)
}

fn domain_space<'a, E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>>(
    input: Tokens<'a>,
) -> IResult<Tokens, &'a str, E> {
    terminated(domain, space)(input)
}

/// Parses a [`Class`], and one more whitespace.
fn class_space<'a, E>(
    input: Tokens<'a>,
) -> IResult<Tokens, Class, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, strum::ParseError>
{
    context(
        "Class",
        terminated(
            map_res(
                alt((keyword("IN"), keyword("CS"), keyword("CH"), keyword("HS"))),
                |t: Tokens| {
                    Class::from_str(t[0].as_str())
                },
            ),
            space,
        ),
    )(input)
}

/// Parses a TTL, and one more whitespace.
fn ttl_space<'a, E>(s: Tokens<'a>) -> IResult<Tokens, Duration, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    // TODO Bind supports different formats of TTL, such as "1d"
    context("TTL", terminated(duration, space))(s)
}

/// Internal struct for capturing each row.
///
/// It is very similar to a [`Record`] but allows for
/// optional values. When parsing a full zone file
/// those options can be derived from previous rows.
#[derive(Debug, PartialEq)]
struct Row<'a> {
    name: Option<&'a str>,
    ttl: Option<Duration>,
    class: Option<Class>,
    resource: Resource,
}

fn mx_record<'a, E>(s: Tokens<'a>) -> IResult<Tokens, MX, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    map(tuple((digit1, space, domain)), |x| MX {
        preference: x.0,
        exchange: x.2.to_string(),
    })(s)
}

fn soa_record<'a, E>(s: Tokens<'a>) -> IResult<Tokens, SOA, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    map(
        tuple((
            domain, space, string, space, digit1, space, duration, space, duration, space,
            duration, space, duration,
        )),
        |x| SOA {
            mname: x.0.to_string(),
            rname: x.2.to_string(),
            serial: x.4,
            refresh: x.6,
            retry: x.8,
            expire: x.10,
            minimum: x.12,
        },
    )(s)
}

fn rdata<'a, E>(input: Tokens<'a>) -> IResult<Tokens, Resource, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::net::AddrParseError>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
{
    context(
        "Resource Data",
        alt((
            // TODO Add other type
            prefixed("A", map(ipv4_addr, Resource::A)),
            prefixed("AAAA", map(ipv6_addr, Resource::AAAA)),
            prefixed("NS", map(domain, |x| Resource::NS(x.to_string()))),
            prefixed("CNAME", map(domain, |x| Resource::CNAME(x.to_string()))),
            prefixed("PTR", map(domain, |x| Resource::PTR(x.to_string()))),
            prefixed("MX", map(mx_record, Resource::MX)),
            prefixed("SOA", map(soa_record, Resource::SOA)),
        )),
    )(input)
}

/// Parses a single row from the zone file.
///
/// ```text
/// <blank>[<comment>]
/// $ORIGIN <domain-name> [<comment>]
/// <domain-name><rr> [<comment>]
/// <blank><rr> [<comment>]
/// ```
///
/// Not supported:
/// ```text
/// $INCLUDE <file-name> [<domain-name>] [<comment>]
/// $TTL integer_value ; Sets the default value of TTL for following RRs in file (RFC2308, bind>8.1)
/// ```
///
/// <rr> contents take one of the following forms:
/// ```text
/// [<TTL>] [<class>] <type> <RDATA>
/// [<class>] [<TTL>] <type> <RDATA>
/// ```
/// See https://datatracker.ietf.org/doc/html/rfc1035#section-5
/// More examples: https://datatracker.ietf.org/doc/html/rfc2308#section-10
/// https://web.mit.edu/rhel-doc/5/RHEL-5-manual/Deployment_Guide-en-US/s1-bind-zone.html
fn parse_row<'a, E>(input: Tokens<'a>) -> IResult<Tokens<'a>, Row, E>
where
    E: ParseError<Tokens<'a>> + ContextError<Tokens<'a>>,
    E: FromExternalError<Tokens<'a>, std::net::AddrParseError>,
    E: FromExternalError<Tokens<'a>, std::num::ParseIntError>,
    E: FromExternalError<Tokens<'a>, strum::ParseError>,
{
    // TODO Check if the first field is a special field
    // TODO Make sure this is case insensitive (AAAA is the same as aaaa)

    // Nom is a greedy parser, and doesn't seem to back-track
    // when there is ambiguity. For example, the record:
    //
    //  <blank>   A   1.2.3.4
    //
    // It greedily consumes "A" thinking it is the domain name
    // which causes parsing to fail. To resolve this, we use
    // a table of `alt` paths, each one a different possible
    // combination.  This may not be the best, but it forces
    // Nom to backtrack to the `alt` and keep trying branches.
    // This also allows us to express how to resolve the ambiguity
    // based on the order of the list.
    //
    // The order is as such:
    //   domain, ttl, class    // permutation (of ttl and class)
    //   domain, ttl,
    //   domain,    , class
    //   domain,    ,
    //         , ttl, class    // permutation (of ttl and class)
    //         , ttl,
    //         ,    , class
    //         ,    ,
    //
    // Since some of them it's unambiguious for where the rdata starts
    // we use cut(rdata) to enforce it successfully parse.
    //
    // TODO Since TTL is digits, and would never be confused with a domain/class, I think
    // we can refactor the below to make ttl a opt(ttl) thus halfing the number of branches.

    all_consuming(delimited(
        many0(space),
        map(
            alt((
                tuple((
                    some(domain_space),
                    permutation((some(ttl_space), some(class_space))),
                    // Use `cut` because if we have all three (domain, ttl, class), then the next thing must be a RR Data
                    cut(rdata),
                )),
                tuple((
                    some(domain_space),
                    pair(some(ttl_space), success(None)),
                    rdata,
                )),
                tuple((
                    some(domain_space),
                    pair(success(None), some(class_space)),
                    rdata,
                )),
                tuple((
                    some(domain_space),
                    pair(success(None), success(None)),
                    rdata,
                )),
                tuple((
                    success(None),
                    permutation((some(ttl_space), some(class_space))),
                    rdata,
                )),
                tuple((
                    success(None),
                    pair(some(ttl_space), success(None)),
                    cut(rdata),
                )),
                tuple((success(None), pair(success(None), some(class_space)), rdata)),
                tuple((success(None), pair(success(None), success(None)), rdata)),
            )),
            |ret| Row {
                name: ret.0,
                ttl: ret.1 .0,
                class: ret.1 .1,
                resource: ret.2,
            },
        ),
        many0(space),
    ))(input)
}

fn my_convert_error(input: Tokens, e: VerboseError<Tokens>) -> String {
    use std::fmt::Write;

    let mut result = String::new();

    for (i, (substring, kind)) in e.errors.iter().enumerate() {
        if input.is_empty() {
            match kind {
                VerboseErrorKind::Char(c) => {
                    write!(&mut result, "{}: expected '{}', got empty input\n\n", i, c)
                }
                VerboseErrorKind::Context(s) => {
                    write!(&mut result, "{}: in {}, got empty input\n\n", i, s)
                }
                VerboseErrorKind::Nom(e) => {
                    write!(&mut result, "{}: in {:?}, got empty input\n\n", i, e)
                }
            }
        } else {
            let t = substring[0]; // TODO will this panic?
            let line_number = t.pos.location_line();

            // The (1-indexed) column number is the offset of our substring into that line
            let column_number = t.pos.get_utf8_column();

            //let substring = substring.to_string();
            let substring = t.pos.fragment();

            // TODO This only gets the line up to the token. We should keep looking though the tokens appending as needed.
            let line = std::str::from_utf8(t.pos.get_line_beginning()).unwrap();

            match kind {
                VerboseErrorKind::Char(c) => {
                    if let Some(actual) = substring.chars().next() {
                        write!(
                            &mut result,
                            "{i}: at line {line_number}:\n\
               {line}\n\
               {caret:>column$}\n\
               expected '{expected}', found {actual}\n\n",
                            i = i,
                            line_number = line_number,
                            line = line,
                            caret = '^',
                            column = column_number,
                            expected = c,
                            actual = actual,
                        )
                    } else {
                        write!(
                            &mut result,
                            "{i}: at line {line_number}:\n\
               {line}\n\
               {caret:>column$}\n\
               expected '{expected}', got end of input\n\n",
                            i = i,
                            line_number = line_number,
                            line = line,
                            caret = '^',
                            column = column_number,
                            expected = c,
                        )
                    }
                }
                VerboseErrorKind::Context(s) => write!(
                    &mut result,
                    "{i}: at line {line_number}, in {context}:\n\
             {line}\n\
             {caret:>column$}\n\n",
                    i = i,
                    line_number = line_number,
                    context = s,
                    line = line,
                    caret = '^',
                    column = column_number,
                ),
                VerboseErrorKind::Nom(e) => write!(
                    &mut result,
                    "{i}: at line {line_number}, in {nom_err:?}:\n\
             {line}\n\
             {caret:>column$}\n\n",
                    i = i,
                    line_number = line_number,
                    nom_err = e, // TODO If this is EOF print a different error
                    line = line,
                    caret = '^',
                    column = column_number,
                ),
            }
        }
        // Because `write!` to a `String` is infallible, this `unwrap` is fine.
        .unwrap();
    }

    result
}

fn parse<'a>(input: &'a str) -> Result<Row, ()> {
    let (remaining, tokens) = tokenise::<VerboseError<LocatedSpan<&str>>>(input.into()).unwrap(); // TODO Fix
    assert!(remaining.is_empty());

    // TODO Return a full zone file
    // TODO Make pretty error messages
    println!("Tokens:\n{}", tokens);

    let ret = parse_row::<VerboseError<Tokens<'a>>>(tokens.clone()); // TODO remove clone
                                                                     //println!("parsed verbose: {:#?}", ret);
    match ret {
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            println!("{}", my_convert_error(tokens, e));
            Err(())
        }

        Err(nom::Err::Incomplete(_e)) => {
            println!(
                "incomplete input" // TODO!
            );

            Err(())
        }

        Ok((remaining, result)) => {
            assert!(remaining.is_empty(), "all input should have been consumed.");
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::convert_error;
    use nom::Err;

    impl Row<'_> {
        fn new(
            name: Option<&str>,
            class: Option<Class>,
            ttl: Option<Duration>,
            resource: Resource,
        ) -> Row {
            Row {
                name,
                ttl,
                class,
                resource,
            }
        }
    }

    fn print_err<I: std::fmt::Debug + std::ops::Deref<Target = str>, O: std::fmt::Debug>(
        input: I,
        err: IResult<I, O, VerboseError<I>>,
    ) {
        // If there is a error print a nice set of debugging.
        println!("VerboseError: {:#?}", err);

        match err {
            Err(Err::Error(e)) | Err(Err::Failure(e)) => {
                println!(
                    "VerboseError - `root::<VerboseError>(data)`:\n{}",
                    convert_error(input, e)
                );
            }
            _ => {}
        }
    }

    #[test]
    fn test_parse_row() {
        let tests = vec![
            (
                "A       A       26.3.0.103",
                Row::new(
                    Some("A"),
                    None,
                    None,
                    Resource::A("26.3.0.103".parse().unwrap()),
                ),
            ),
            (
                "VENERA  A       10.1.0.52",
                Row::new(
                    Some("VENERA"),
                    None,
                    None,
                    Resource::A("10.1.0.52".parse().unwrap()),
                ),
            ),
            (
                "        A       128.9.0.32",
                Row::new(None, None, None, Resource::A("128.9.0.32".parse().unwrap())),
            ),
            (
                "        NS      VAXA",
                Row::new(None, None, None, Resource::NS("VAXA".to_string())),
            ),
            (
                "        MX      20      VAXA",
                Row::new(
                    None,
                    None,
                    None,
                    Resource::MX(MX {
                        preference: 20,
                        exchange: "VAXA".to_string(),
                    }),
                ),
            ),
            (
                "        AAAA    2400:cb00:2049:1::a29f:1804",
                Row::new(
                    None,
                    None,
                    None,
                    Resource::AAAA("2400:cb00:2049:1::a29f:1804".parse().unwrap()),
                ),
            ),
        ];

        for (input, want) in tests {
            /*
            let ret = parse_row::<VerboseError<&str>>(input);
            if ret.is_err() {
                print_err(input, ret);
                panic!("failed '{}'", input)
            }

            let (remaining, got) = ret.unwrap();
            assert_eq!(got, want);
            assert_eq!(remaining, "", "all input should have been consumed.");
            */

            // TODO
            let ret = parse(input);
            if ret.is_err() {
                panic!("failed '{}'", input)
            }
            let got = ret.unwrap();
            assert_eq!(got, want);
        }

        // TODO add some bad data examples
    }

    // Test Full files
    #[test]
    fn test_parse() {
        let tests = vec![
            // Examples from https://www.nlnetlabs.nl/documentation/nsd/grammar-for-dns-zone-files/
            "$ORIGIN example.org.
            SOA    soa    soa    ( 1 2 3 4 5 6 )",

            "$ORIGIN example.org.
            SOA    soa    soa    ( 1 2 ) ( 3 4 ) ( 5 ) ( 6 )",

            // Examples from https://datatracker.ietf.org/doc/html/rfc1035#section-5.3
            "$ORIGIN ISI.EDU.
            @   IN  SOA     VENERA      Action\\.domains (
                                             20     ; SERIAL
                                             7200   ; REFRESH
                                             600    ; RETRY
                                             3600000; EXPIRE
                                             60)    ; MINIMUM

                    NS      A.ISI.EDU.
                    NS      VENERA
                    NS      VAXA
                    MX      10      VENERA
                    MX      20      VAXA

            A       A       26.3.0.103

            VENERA  A       10.1.0.52
                    A       128.9.0.32

            VAXA    A       10.2.0.27
                    A       128.9.0.33
            ",

            // Examples from https://en.wikipedia.org/wiki/Zone_file
            "
            $ORIGIN example.com.     ; designates the start of this zone file in the namespace
            $TTL 3600                ; default expiration time (in seconds) of all RRs without their own TTL value
            example.com.  IN  SOA   ns.example.com. username.example.com. ( 2020091025 7200 3600 1209600 3600 )
            example.com.  IN  NS    ns                    ; ns.example.com is a nameserver for example.com
            example.com.  IN  NS    ns.somewhere.example. ; ns.somewhere.example is a backup nameserver for example.com
            example.com.  IN  MX    10 mail.example.com.  ; mail.example.com is the mailserver for example.com
            @             IN  MX    20 mail2.example.com. ; equivalent to above line, '@' represents zone origin
            @             IN  MX    50 mail3              ; equivalent to above line, but using a relative host name
            example.com.  IN  A     192.0.2.1             ; IPv4 address for example.com
                          IN  AAAA  2001:db8:10::1        ; IPv6 address for example.com
            ns            IN  A     192.0.2.2             ; IPv4 address for ns.example.com
                          IN  AAAA  2001:db8:10::2        ; IPv6 address for ns.example.com
            www           IN  CNAME example.com.          ; www.example.com is an alias for example.com
            wwwtest       IN  CNAME www                   ; wwwtest.example.com is another alias for www.example.com
            mail          IN  A     192.0.2.3             ; IPv4 address for mail.example.com
            mail2         IN  A     192.0.2.4             ; IPv4 address for mail2.example.com
            mail3         IN  A     192.0.2.5             ; IPv4 address for mail3.example.com
        "];

        for input in tests {
            let ret = parse(input);
            if ret.is_err() {
                panic!("failed '{}'", input)
            }
        }
    }
}

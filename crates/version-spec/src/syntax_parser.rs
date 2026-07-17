use crate::syntax::*;
use compact_str::CompactString;
use pest::error::*;
use pest::{Parser, Span, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct SyntaxParser;

fn is_wildcard(input: &str) -> bool {
    matches!(input, "" | "*" | "x" | "X")
}

pub(crate) fn calendar_year(year: u32) -> u32 {
    if year.to_string().len() < 4 {
        year + 2000
    } else {
        year
    }
}

#[doc(hidden)]
pub fn parse_semver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::parse_semver, input.as_ref().trim())?;
    let mut version = Version::default();

    for pair in pairs {
        handle_version(pair, &mut version)?;
    }

    Ok(version)
}

#[doc(hidden)]
pub fn parse_semver_req<T: AsRef<str>>(input: T) -> Result<Requirement, pest::error::Error<Rule>> {
    let input = input.as_ref().trim();
    let mut req = Requirement::default();

    if is_wildcard(input) {
        req.op = Op::Wildcard;

        return Ok(req);
    }

    let pairs = SyntaxParser::parse(Rule::parse_semver_req, input)?;

    for pair in pairs {
        handle_requirement(pair, &mut req)?;
    }

    Ok(req)
}

#[doc(hidden)]
pub fn parse_semver_range<T: AsRef<str>>(input: T) -> Result<Range, pest::error::Error<Rule>> {
    let input = input.as_ref().trim();
    let mut range = Range::default();

    if is_wildcard(input) {
        return Ok(range);
    }

    let pairs = SyntaxParser::parse(Rule::parse_semver_range, input)?;

    for pair in pairs {
        handle_range(pair, &mut range)?;
    }

    Ok(range)
}

#[doc(hidden)]
pub fn parse_calver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::parse_calver, input.as_ref().trim())?;
    let mut version = Version::default();

    for pair in pairs {
        handle_version(pair, &mut version)?;
    }

    Ok(version)
}

#[doc(hidden)]
pub fn parse_calver_req<T: AsRef<str>>(input: T) -> Result<Requirement, pest::error::Error<Rule>> {
    let input = input.as_ref().trim();
    let mut req = Requirement::default();

    if is_wildcard(input) {
        req.kind = VersionKind::Calendar;
        req.op = Op::Wildcard;

        return Ok(req);
    }

    let pairs = SyntaxParser::parse(Rule::parse_calver_req, input)?;

    for pair in pairs {
        handle_requirement(pair, &mut req)?;
    }

    Ok(req)
}

#[doc(hidden)]
pub fn parse_calver_range<T: AsRef<str>>(input: T) -> Result<Range, pest::error::Error<Rule>> {
    let input = input.as_ref().trim();
    let mut range = Range::default();

    if is_wildcard(input) {
        return Ok(range);
    }

    let pairs = SyntaxParser::parse(Rule::parse_calver_range, input)?;

    for pair in pairs {
        handle_range(pair, &mut range)?;
    }

    Ok(range)
}

#[doc(hidden)]
pub fn parse_alias<T: AsRef<str>>(input: T) -> Result<CompactString, pest::error::Error<Rule>> {
    let input = input.as_ref().trim();

    SyntaxParser::parse(Rule::parse_alias, input)?;

    Ok(CompactString::new(input))
}

fn parse_int(pair: Pair<Rule>, message: &str) -> Result<u32, Error<Rule>> {
    pair.as_str().parse::<u32>().map_err(|error| {
        Error::new_from_span(
            ErrorVariant::CustomError {
                message: format!("{message}: {error}"),
            },
            pair.as_span(),
        )
    })
}

fn parse_int_opt(pair: Pair<Rule>, message: &str) -> Result<Option<u32>, Error<Rule>> {
    match pair.as_str() {
        "*" | "x" | "X" => Ok(None),
        _ => parse_int(pair, message).map(Some),
    }
}

// Mirror the semver crate, where a numeric part cannot follow
// a wildcard part, for example "*.1" or "1.*.1"
fn verify_wildcard_order(
    previous: Option<u32>,
    current: Option<u32>,
    span: Span,
) -> Result<(), Error<Rule>> {
    if previous.is_none() && current.is_some() {
        return Err(Error::new_from_span(
            ErrorVariant::CustomError {
                message: "a version part cannot follow a wildcard part".to_owned(),
            },
            span,
        ));
    }

    Ok(())
}

fn handle_version(pair: Pair<Rule>, version: &mut Version) -> Result<(), pest::error::Error<Rule>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::scope => version.scope = Some(CompactString::new(inner.as_str())),

            Rule::pre => version.prerelease = Some(CompactString::new(inner.as_str())),

            Rule::build => version.build = Some(CompactString::new(inner.as_str())),

            Rule::major => {
                version.kind = VersionKind::Semantic;
                version.major = parse_int(inner, "failed to parse major version")?;
            }

            Rule::minor => {
                version.minor = parse_int(inner, "failed to parse minor version")?;
            }

            Rule::patch => {
                version.patch = parse_int(inner, "failed to parse patch version")?;
            }

            Rule::year => {
                version.kind = VersionKind::Calendar;
                version.major = parse_int(inner, "failed to parse year").map(calendar_year)?;
            }

            Rule::month => {
                version.minor = parse_int(inner, "failed to parse month")?;
            }

            Rule::day => {
                version.patch = parse_int(inner, "failed to parse day")?;
            }

            // Continue parsing
            Rule::parse_semver | Rule::parse_calver | Rule::semver | Rule::calver => {
                handle_version(inner, version)?;
            }

            // End of input
            Rule::EOI => {}

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    Ok(())
}

fn handle_requirement(
    pair: Pair<Rule>,
    req: &mut Requirement,
) -> Result<(), pest::error::Error<Rule>> {
    let mut has_op = false;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::req_scope => req.scope = Some(CompactString::new(inner.as_str())),

            Rule::pre => req.prerelease = Some(CompactString::new(inner.as_str())),

            // Build metadata is accepted for compatibility, but ignored
            Rule::build => {}

            Rule::op => {
                has_op = true;
                req.op = match inner.as_str() {
                    "=" | "==" => Op::Exact,
                    ">" => Op::Greater,
                    ">=" => Op::GreaterEq,
                    "<" => Op::Less,
                    "<=" => Op::LessEq,
                    "~" => Op::Tilde,
                    "^" => Op::Caret,
                    "*" | "x" | "X" => Op::Wildcard,
                    _ => unreachable!(),
                };
            }

            Rule::major_req => {
                req.kind = VersionKind::Semantic;
                req.major = parse_int_opt(inner, "failed to parse major version")?;

                // A wildcard part, like "*" or "1.*", is a wildcard match,
                // unless an operator was explicitly defined
                if !has_op && req.major.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::minor_req => {
                let span = inner.as_span();

                req.minor = parse_int_opt(inner, "failed to parse minor version")?;

                verify_wildcard_order(req.major, req.minor, span)?;

                if !has_op && req.minor.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::patch_req => {
                let span = inner.as_span();

                req.patch = parse_int_opt(inner, "failed to parse patch version")?;

                verify_wildcard_order(req.minor, req.patch, span)?;

                if !has_op && req.patch.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::year_req => {
                req.kind = VersionKind::Calendar;
                req.major = parse_int_opt(inner, "failed to parse year")
                    .map(|year| year.map(calendar_year))?;

                // A wildcard part, like "*" or "2000-*", is a wildcard match,
                // unless an operator was explicitly defined
                if !has_op && req.major.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::month_req => {
                let span = inner.as_span();

                req.minor = parse_int_opt(inner, "failed to parse month")?;

                verify_wildcard_order(req.major, req.minor, span)?;

                if !has_op && req.minor.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::day_req => {
                let span = inner.as_span();

                req.patch = parse_int_opt(inner, "failed to parse day")?;

                verify_wildcard_order(req.minor, req.patch, span)?;

                if !has_op && req.patch.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            // Continue parsing
            Rule::parse_semver_req
            | Rule::parse_calver_req
            | Rule::semver_req
            | Rule::calver_req => {
                handle_requirement(inner, req)?;
            }

            // End of input
            Rule::EOI => {}

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    Ok(())
}

fn handle_between(pair: Pair<Rule>) -> Result<Clause, pest::error::Error<Rule>> {
    let mut left = None;
    let mut right = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::semver | Rule::calver => {
                let mut version = Version::default();

                handle_version(inner, &mut version)?;

                if left.is_none() {
                    left = Some(version);
                } else {
                    right = Some(version);
                }
            }

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    // The grammar requires both versions
    match (left, right) {
        (Some(left), Some(right)) => Ok(Clause::Between(Box::new(left), Box::new(right))),
        _ => unreachable!(),
    }
}

fn handle_clause(pair: Pair<Rule>) -> Result<Clause, pest::error::Error<Rule>> {
    let mut reqs = vec![];

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::semver_between | Rule::calver_between => {
                return handle_between(inner);
            }

            Rule::semver_req | Rule::calver_req => {
                let mut req = Requirement::default();

                handle_requirement(inner, &mut req)?;

                reqs.push(req);
            }

            Rule::and => {}

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    // The grammar requires at least one requirement
    Ok(if reqs.len() == 1 {
        Clause::Only(reqs.remove(0))
    } else {
        Clause::All(reqs)
    })
}

fn handle_range(pair: Pair<Rule>, range: &mut Range) -> Result<(), pest::error::Error<Rule>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::semver_clause | Rule::calver_clause => {
                range.clauses.push(handle_clause(inner)?);
            }

            Rule::or => {}

            // Continue parsing
            Rule::parse_semver_range
            | Rule::parse_calver_range
            | Rule::semver_range
            | Rule::calver_range => {
                handle_range(inner, range)?;
            }

            // End of input
            Rule::EOI => {}

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    Ok(())
}

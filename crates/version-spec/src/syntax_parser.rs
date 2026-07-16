use crate::syntax::*;
use pest::error::*;
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct SyntaxParser;

fn is_wildcard(input: &str) -> bool {
    matches!(input, "" | "*" | "x" | "X")
}

pub(crate) fn calendar_year(year: u64) -> u64 {
    if year.to_string().len() < 4 {
        year + 2000
    } else {
        year
    }
}

pub fn parse_semver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::parse_semver, input.as_ref().trim())?;
    let mut version = Version::default();

    for pair in pairs {
        handle_version(pair, &mut version)?;
    }

    Ok(version)
}

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

pub fn parse_calver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::parse_calver, input.as_ref().trim())?;
    let mut version = Version::default();

    for pair in pairs {
        handle_version(pair, &mut version)?;
    }

    Ok(version)
}

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

fn parse_int(pair: Pair<Rule>, message: &str) -> Result<u64, Error<Rule>> {
    pair.as_str().parse::<u64>().map_err(|error| {
        Error::new_from_span(
            ErrorVariant::CustomError {
                message: format!("{message}: {error}"),
            },
            pair.as_span(),
        )
    })
}

fn parse_int_opt(pair: Pair<Rule>, message: &str) -> Result<Option<u64>, Error<Rule>> {
    match pair.as_str() {
        "*" | "x" | "X" => Ok(None),
        _ => parse_int(pair, message).map(Some),
    }
}

fn handle_version(pair: Pair<Rule>, version: &mut Version) -> Result<(), pest::error::Error<Rule>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::scope => version.scope = Some(inner.as_str().to_string()),

            Rule::pre => version.prerelease = Some(inner.as_str().to_string()),

            Rule::build => version.build = Some(inner.as_str().to_string()),

            Rule::major => {
                version.kind = VersionKind::Semantic;
                version.major = parse_int(inner, "failed to parse major version")?;
            }

            Rule::minor => {
                version.minor = parse_int(inner, "failed to parse minor version")?;
            }

            Rule::patch => {
                version.micro = parse_int(inner, "failed to parse patch version")?;
            }

            Rule::year => {
                version.kind = VersionKind::Calendar;
                version.major = parse_int(inner, "failed to parse year").map(calendar_year)?;
            }

            Rule::month => {
                version.minor = parse_int(inner, "failed to parse month")?;
            }

            Rule::day => {
                version.micro = parse_int(inner, "failed to parse day")?;
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
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::req_scope => req.scope = Some(inner.as_str().to_string()),

            Rule::pre => req.prerelease = Some(inner.as_str().to_string()),

            Rule::op => {
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

                // A wildcard-only version, like "node-*", is a wildcard match
                if req.major.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::minor_req => {
                req.minor = parse_int_opt(inner, "failed to parse minor version")?;
            }

            Rule::patch_req => {
                req.micro = parse_int_opt(inner, "failed to parse patch version")?;
            }

            Rule::year_req => {
                req.kind = VersionKind::Calendar;
                req.major = parse_int_opt(inner, "failed to parse year")
                    .map(|year| year.map(calendar_year))?;

                // A wildcard-only version, like "node-*", is a wildcard match
                if req.major.is_none() {
                    req.op = Op::Wildcard;
                }
            }

            Rule::month_req => {
                req.minor = parse_int_opt(inner, "failed to parse month")?;
            }

            Rule::day_req => {
                req.micro = parse_int_opt(inner, "failed to parse day")?;
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
        (Some(left), Some(right)) => Ok(Clause::Between(left, right)),
        _ => unreachable!(),
    }
}

fn handle_clause(pair: Pair<Rule>) -> Result<Clause, pest::error::Error<Rule>> {
    let mut left = None;
    let mut right = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::semver_between | Rule::calver_between => {
                return handle_between(inner);
            }

            Rule::semver_req | Rule::calver_req => {
                let mut req = Requirement::default();

                handle_requirement(inner, &mut req)?;

                if left.is_none() {
                    left = Some(req);
                } else {
                    right = Some(req);
                }
            }

            Rule::and => {}

            // Error for unhandled rules
            _ => {
                unreachable!();
            }
        }
    }

    // The grammar requires a left value, with an optional right value
    Ok(match (left, right) {
        (Some(left), Some(right)) => Clause::And(left, right),
        (Some(left), None) => Clause::Only(left),
        _ => unreachable!(),
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

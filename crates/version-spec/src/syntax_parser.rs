use crate::syntax::*;
use pest::error::*;
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct SyntaxParser;

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

    if matches!(input, "" | "*" | "x" | "X") {
        req.op = Op::Wildcard;

        return Ok(req);
    }

    let pairs = SyntaxParser::parse(Rule::parse_semver_req, input)?;

    for pair in pairs {
        handle_requirement(pair, &mut req)?;
    }

    Ok(req)
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

    if matches!(input, "" | "*" | "x" | "X") {
        req.op = Op::Wildcard;

        return Ok(req);
    }

    let pairs = SyntaxParser::parse(Rule::parse_calver_req, input)?;

    for pair in pairs {
        handle_requirement(pair, &mut req)?;
    }

    Ok(req)
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
                version.major = parse_int(inner, "failed to parse major version")?;
            }

            Rule::minor => {
                version.minor = parse_int(inner, "failed to parse minor version")?;
            }

            Rule::patch => {
                version.micro = parse_int(inner, "failed to parse patch version")?;
            }

            Rule::year => {
                version.major = parse_int(inner, "failed to parse year")?;
            }

            Rule::month => {
                version.minor = parse_int(inner, "failed to parse month")?;
            }

            Rule::day => {
                version.micro = parse_int(inner, "failed to parse day")?;
            }

            // Continue parsing
            Rule::parse_semver | Rule::parse_calver => {
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
            Rule::scope => req.scope = Some(inner.as_str().to_string()),

            Rule::pre => req.prerelease = Some(inner.as_str().to_string()),

            Rule::build => req.build = Some(inner.as_str().to_string()),

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
                req.major = parse_int_opt(inner, "failed to parse major version")?;
            }

            Rule::minor_req => {
                req.minor = parse_int_opt(inner, "failed to parse minor version")?;
            }

            Rule::patch_req => {
                req.micro = parse_int_opt(inner, "failed to parse patch version")?;
            }

            Rule::year_req => {
                req.major = parse_int_opt(inner, "failed to parse year")?;
            }

            Rule::month_req => {
                req.minor = parse_int_opt(inner, "failed to parse month")?;
            }

            Rule::day_req => {
                req.micro = parse_int_opt(inner, "failed to parse day")?;
            }

            // Continue parsing
            Rule::parse_semver_req | Rule::parse_calver_req => {
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

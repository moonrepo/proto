use pest::error::*;
use pest::{Parser, iterators::Pair};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct SyntaxParser;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Version {
    pub scope: Option<String>,
    pub major: u64, // year
    pub minor: u64, // month
    pub micro: u64, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

fn handle_semver(pair: Pair<Rule>, version: &mut Version) -> Result<(), pest::error::Error<Rule>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::scope => version.scope = Some(inner.as_str().to_string()),

            Rule::pre => version.prerelease = Some(inner.as_str().to_string()),

            Rule::build => version.build = Some(inner.as_str().to_string()),

            Rule::major => {
                version.major = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse major version: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            Rule::minor => {
                version.minor = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse minor version: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            Rule::patch => {
                version.micro = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse patch version: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            // Continue parsing
            Rule::semver | Rule::semver_version => {
                handle_semver(inner, version)?;
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

fn handle_calver(pair: Pair<Rule>, version: &mut Version) -> Result<(), pest::error::Error<Rule>> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            // Extract information
            Rule::scope => version.scope = Some(inner.as_str().to_string()),

            Rule::pre => version.prerelease = Some(inner.as_str().to_string()),

            Rule::build => version.build = Some(inner.as_str().to_string()),

            Rule::year => {
                version.major = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse year: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            Rule::month => {
                version.minor = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse month: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            Rule::day => {
                version.micro = inner.as_str().parse::<u64>().map_err(|e| {
                    Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("failed to parse day: {e}"),
                        },
                        inner.as_span(),
                    )
                })?
            }

            // Continue parsing
            Rule::calver | Rule::calver_version => {
                handle_calver(inner, version)?;
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

pub fn parse_calver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::calver, input.as_ref().trim())?;
    let mut version = Version::default();

    for pair in pairs {
        handle_calver(pair, &mut version)?;
    }

    Ok(version)
}

pub fn parse_semver<T: AsRef<str>>(input: T) -> Result<Version, pest::error::Error<Rule>> {
    let pairs = SyntaxParser::parse(Rule::semver, input.as_ref().trim())?;
    let mut version = Version::default();

    dbg!(&pairs);

    for pair in pairs {
        handle_semver(pair, &mut version)?;
    }

    dbg!(&version);

    Ok(version)
}
